//! Phase 1 — session lifecycle.

use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use korangar_networking::{DEFAULT_HAIR_STYLE, NetworkEvent, NetworkingSystem};
use ragnarok_packets::Sex;

use crate::context::{Config, PACKET_VERSION, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("smoke", 1, smoke),
        Scenario::new("bad-password", 1, bad_password),
        Scenario::new("account-registration", 1, account_registration),
        Scenario::new("connection-state", 1, connection_state),
        Scenario::new("character-select-invalid", 1, character_select_invalid),
        Scenario::new("character-create-delete", 1, character_create_delete),
        Scenario::new("character-delete-after-play", 1, character_delete_after_play),
        Scenario::new("character-starting-stats", 1, character_starting_stats),
        Scenario::new("party-job-level-refresh", 8, party_job_level_refresh),
        Scenario::new("character-slot-switch-rejected", 1, character_slot_switch_rejected),
        Scenario::new("character-slot-switch", 1, character_slot_switch),
        Scenario::new("kick-explains-itself", 1, kick_explains_itself),
        Scenario::new("kick-confirms-to-the-kicker", 1, kick_confirms_to_the_kicker),
        Scenario::new("logout-relogin", 1, logout_relogin),
        Scenario::new("respawn", 1, respawn),
    ]
}

/// A character with zero `slotchange` entitlement must receive an explicit
/// rejection when it attempts to exchange two occupied slots.
fn character_slot_switch_rejected(config: &Config) -> Result<(), String> {
    let (mut session, characters) = connect_to_character_select(config)?;
    let primary = match config.character.as_deref() {
        Some(name) => characters.iter().find(|character| character.name == name),
        None => characters.first(),
    }
    .ok_or("no primary character available for slot switch")?
    .clone();
    let temporary = if let Some(character) = characters.iter().find(|character| character.name.starts_with("HlSwap")) {
        character.clone()
    } else {
        let used_slots: Vec<usize> = characters.iter().map(|character| character.character_number as usize).collect();
        let free_slot = (0..9usize)
            .find(|slot| !used_slots.contains(slot))
            .ok_or("no free character slot")?;
        let name = format!("HlSwap{}", std::process::id() % 100000);
        session
            .0
            .create_character(free_slot, name, Sex::Male, DEFAULT_HAIR_STYLE)
            .map_err(|_| "disconnected")?;
        wait_char_event(&mut session, config.timeout, &mut |event| match event {
            NetworkEvent::CharacterCreated { character_information } => Some(Ok(character_information.clone())),
            NetworkEvent::CharacterCreationFailed { message, .. } => Some(Err(format!("creation failed: {message}"))),
            _ => None,
        })??
    };

    let origin_slot = primary.character_number as usize;
    session
        .0
        .switch_character_slot(origin_slot, temporary.character_number as usize)
        .map_err(|_| "disconnected")?;
    wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterSlotSwitchFailed => Some(Ok(())),
        NetworkEvent::CharacterSlotSwitched => Some(Err("slot switch unexpectedly succeeded without entitlement".to_owned())),
        _ => None,
    })
    .map_err(|error| format!("slot rejection: {error}"))??;
    session.0.delete_character(temporary.character_id).map_err(|_| "disconnected")?;
    wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterDeleted => Some(Ok(())),
        NetworkEvent::CharacterDeletionFailed { message, .. } => Some(Err(format!("cleanup deletion failed: {message}"))),
        _ => None,
    })
    .map_err(|error| format!("temporary-character cleanup: {error}"))??;
    Ok(())
}

/// Slot-switch success and persistence on the entitled partner character.
///
/// Slot moves are gated per character by the `slotchange` column of the
/// `char` table (this Hercules build has no config to enable them globally),
/// so the partner character needs the one-time SQL fixture in
/// `tools/testing/fixtures/grant-slotchange.sql`. The GM character keeps
/// `slotchange = 0` so `character-slot-switch-rejected` stays valid.
fn character_slot_switch(config: &Config) -> Result<(), String> {
    const PARTNER_CHARACTER: &str = "HeadlessTwo";

    let connect = |config: &Config| connect_to_character_select_as(config, &config.partner_username, &config.partner_password);

    let (mut session, characters) =
        connect(config).map_err(|error| format!("partner account unavailable (run a phase 8 scenario once to create it): {error}"))?;
    let character = characters
        .iter()
        .find(|character| character.name == PARTNER_CHARACTER)
        .ok_or_else(|| format!("partner character {PARTNER_CHARACTER} not found (run a phase 8 scenario once to create it)"))?
        .clone();

    let origin_slot = character.character_number as usize;
    let used_slots: Vec<usize> = characters.iter().map(|entry| entry.character_number as usize).collect();
    let target_slot = (0..9usize)
        .find(|slot| !used_slots.contains(slot))
        .ok_or("no free character slot on the partner account")?;

    let switch = |session: &mut CharSession, from: usize, to: usize, timeout| {
        session.0.switch_character_slot(from, to).map_err(|_| "disconnected")?;
        wait_char_event(session, timeout, &mut |event| match event {
            NetworkEvent::CharacterSlotSwitched => Some(Ok(())),
            NetworkEvent::CharacterSlotSwitchFailed => Some(Err("slot switch rejected — grant the entitlement fixture with \
                                                                 tools/testing/fixtures/grant-slotchange.sql"
                .to_owned())),
            _ => None,
        })?
    };

    switch(&mut session, origin_slot, target_slot, config.timeout)?;
    drop(session);
    sleep(Duration::from_millis(700));

    // The move must persist across a fresh character-server session.
    let (mut session, characters) = connect(config)?;
    let moved = characters
        .iter()
        .find(|entry| entry.character_id == character.character_id)
        .ok_or("partner character vanished after slot switch")?;
    if moved.character_number as usize != target_slot {
        return Err(format!(
            "character sits in slot {} after relogin, expected {target_slot}",
            moved.character_number
        ));
    }

    // Restore the original slot and verify that persists too.
    switch(&mut session, target_slot, origin_slot, config.timeout)?;
    drop(session);
    sleep(Duration::from_millis(700));

    let (_, characters) = connect(config)?;
    let restored = characters
        .iter()
        .find(|entry| entry.character_id == character.character_id)
        .ok_or("partner character vanished after restoring its slot")?;
    if restored.character_number as usize != origin_slot {
        return Err(format!(
            "character sits in slot {} after restore, expected {origin_slot}",
            restored.character_number
        ));
    }
    Ok(())
}

/// Login → char select → map load → chat echo round trip.
fn smoke(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    const MARKER: &str = "[headless-tester] smoke marker";
    context.flush();
    context.say(MARKER)?;
    context.wait_for("chat echo", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains(MARKER) => Some(()),
        _ => None,
    })?;

    context.net.disconnect_from_map_server();
    Ok(())
}

/// Connection getters track map login and explicit disconnects without
/// panicking. Login/character sockets are often already closed after map
/// handoff on this PACKETVER — only map connectivity is required post-connect.
fn connection_state(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    if !context.net.is_map_server_connected() {
        return Err("map server should be connected after TestContext::connect".to_owned());
    }
    // Safe to call; may already be false after map login.
    let _ = context.net.is_character_server_connected();
    let _ = context.net.is_login_server_connected();

    context.net.disconnect_from_map_server();
    context.pump(Duration::from_millis(300));
    if context.net.is_map_server_connected() {
        return Err("map server still reports connected after disconnect_from_map_server".to_owned());
    }

    // Explicit disconnects must not panic even when already closed.
    context.net.disconnect_from_character_server();
    context.net.disconnect_from_login_server();
    context.pump(Duration::from_millis(200));
    let _ = context.net.is_character_server_connected();
    let _ = context.net.is_login_server_connected();
    Ok(())
}

/// Selecting an empty / out-of-range character slot must fail typed, then a
/// valid select must still succeed without restarting the process.
fn character_select_invalid(config: &Config) -> Result<(), String> {
    let (mut session, characters) = connect_to_character_select(config)?;
    if characters.is_empty() {
        return Err("no characters available for select-failure test".to_owned());
    }
    let valid_slot = characters[0].character_number as usize;

    // Empty slot (assuming not all 9 are filled — pick a free one).
    let used: Vec<usize> = characters.iter().map(|c| c.character_number as usize).collect();
    let empty = (0..9usize).find(|slot| !used.contains(slot));
    if let Some(empty_slot) = empty {
        session.0.select_character(empty_slot).map_err(|_| "disconnected")?;
        wait_char_event(&mut session, config.timeout, &mut |event| match event {
            NetworkEvent::CharacterSelectionFailed { .. } => Some(Ok(())),
            NetworkEvent::CharacterSelected { .. } => Some(Err("empty slot selection unexpectedly succeeded".to_owned())),
            _ => None,
        })
        .map_err(|error| format!("empty-slot select: {error}"))??;
    }

    // Valid retry on the same session.
    session.0.select_character(valid_slot).map_err(|_| "disconnected")?;
    wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterSelected { .. } => Some(Ok(())),
        NetworkEvent::CharacterSelectionFailed { message, .. } => Some(Err(format!("valid select failed: {message}"))),
        _ => None,
    })
    .map_err(|error| format!("valid select retry: {error}"))??;
    Ok(())
}

/// Wrong credentials must produce a clean failure event, not a hang.
fn bad_password(config: &Config) -> Result<(), String> {
    let (mut net, mut buffer) = NetworkingSystem::spawn_with_callback(config.ledger.clone());
    net.connect_to_login_server(PACKET_VERSION, config.server, "definitely_wrong".to_owned(), "nope".to_owned());

    let deadline = Instant::now() + config.timeout;
    loop {
        net.get_events(&mut buffer);
        for event in buffer.drain() {
            match event {
                NetworkEvent::LoginServerConnectionFailed { .. } => return Ok(()),
                NetworkEvent::LoginServerConnected { .. } => {
                    return Err("login with bogus credentials unexpectedly succeeded".to_owned());
                }
                _ => {}
            }
        }
        if Instant::now() > deadline {
            return Err("no failure event for bad credentials".to_owned());
        }
        sleep(Duration::from_millis(30));
    }
}

/// Registration is how every account on this server comes into existence, and
/// nothing else in the suite touches it: every other scenario logs in as a row
/// that `run-integration-tests.sh` inserted with SQL. That gap mattered little
/// while upstream's `_M`/`_F` sat behind our `_create` as a fallback. `_M`/`_F`
/// was removed on 2026-09-05, so `_create` is now the only way in and a
/// regression here locks out every new player with nothing to fall back on.
///
/// The bare name is checked *before* registering as well as after. Without the
/// "before", a `_create` that stored the account under its full suffixed name
/// would still pass step two, and a name that somehow already existed would
/// make the whole scenario vacuous.
///
/// **This leaves a real account behind.** Harmless under
/// `run-integration-tests.sh`, which drops its whole database afterwards, but
/// pointed at a live server it registers into the actual `login` table with a
/// known weak password. Verified against the live server on 2026-09-05 (the
/// only way to exercise `use_MD5_passwords: true`, which the harness leaves at
/// its default of false) and the row was deleted by hand afterwards. If you run
/// it that way again, clean up: the accounts are the ones named `reg<digits>`.
fn account_registration(config: &Config) -> Result<(), String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the unix epoch".to_owned())?
        .as_nanos();

    // NAME_LENGTH is 24 on the server and `_create` costs 7 of it, so the base
    // name has to stay at 16 characters or under for the suffixed form to
    // survive `safestrncpy` intact.
    let account = format!("reg{:013}", stamp % 10_000_000_000_000);
    let legacy = format!("leg{:013}", stamp % 10_000_000_000_000);
    let password = "regpass";

    if login_is_accepted(config, &account, password)? {
        return Err(format!("`{account}` already exists, so this run proves nothing"));
    }
    if !login_is_accepted(config, &format!("{account}_create"), password)? {
        return Err(format!("`{account}_create` was rejected; registration is broken"));
    }
    if !login_is_accepted(config, &account, password)? {
        return Err(format!(
            "`{account}` does not exist after registering `{account}_create`; the suffix is being stored instead of stripped"
        ));
    }
    if login_is_accepted(config, &format!("{legacy}_m"), password)? {
        return Err(format!(
            "`{legacy}_m` created an account; upstream's _M/_F registration was supposed to be gone"
        ));
    }

    Ok(())
}

/// One login-server attempt. `Ok(true)` means the server accepted it. Reaching
/// the character server is deliberately not part of this: the question is
/// whether the account exists, and `login_auth_ok` wipes a previous
/// login-only session (`char_server == -1`) rather than rejecting the next
/// attempt, which is what makes checking the same account twice safe.
fn login_is_accepted(config: &Config, username: &str, password: &str) -> Result<bool, String> {
    let (mut net, mut buffer) = NetworkingSystem::spawn_with_callback(config.ledger.clone());
    net.connect_to_login_server(PACKET_VERSION, config.server, username.to_owned(), password.to_owned());

    let deadline = Instant::now() + config.timeout;
    loop {
        net.get_events(&mut buffer);
        for event in buffer.drain() {
            match event {
                NetworkEvent::LoginServerConnected { .. } => {
                    net.disconnect_from_login_server();
                    return Ok(true);
                }
                NetworkEvent::LoginServerConnectionFailed { .. } => return Ok(false),
                _ => {}
            }
        }
        if Instant::now() > deadline {
            return Err(format!("no login result for `{username}`"));
        }
        sleep(Duration::from_millis(30));
    }
}

/// Create a character in a free slot, verify it persists across a fresh
/// character-server session, verify duplicate names are rejected without
/// list mutation, then delete it and verify absence after reconnecting.
fn character_create_delete(config: &Config) -> Result<(), String> {
    let temp_name = format!("HlTmp{}", std::process::id() % 100000);

    let (created, baseline) = create_temporary_character(config, &temp_name)?;

    // Even if a mid-scenario assertion fails, the temporary character must be
    // removed; run the checks first, the deletion unconditionally, and report
    // the first error.
    let checks = character_create_checks(config, &created, &temp_name);
    let cleanup = delete_and_verify_absent(config, &created, &temp_name, &baseline);
    checks.and(cleanup)
}

/// Create a disposable character, enter the map with it once, then delete it
/// from character select and verify absence across two reconnects without
/// disturbing any other character.
fn character_delete_after_play(config: &Config) -> Result<(), String> {
    let temp_name = format!("HlDel{}", std::process::id() % 100000);

    let (created, baseline) = create_temporary_character(config, &temp_name)?;

    // Enter the map once with the disposable character, then log out cleanly
    // (the context's Drop performs the server-acknowledged logout).
    let play = TestContext::connect_as(config, &config.username, &config.password, Some(&temp_name), None).map(drop);
    sleep(Duration::from_millis(700));

    let cleanup = delete_and_verify_absent(config, &created, &temp_name, &baseline);
    play.map_err(|error| format!("map session with disposable character failed: {error}"))
        .and(cleanup)?;

    // Second reconnect: absence must hold across another fresh session.
    let (_, characters) = connect_to_character_select(config)?;
    if characters
        .iter()
        .any(|c| c.character_id == created.character_id || c.name == temp_name)
    {
        return Err(format!("deleted character {temp_name} reappeared on the second reconnect"));
    }
    assert_baseline_unchanged(&characters, &Some(created.clone()), &baseline)?;
    Ok(())
}

/// The Acolyte preset in the creation window, applied at exactly the same
/// protocol boundary as the GUI and checked after a fresh login.
fn character_starting_stats(config: &Config) -> Result<(), String> {
    use ragnarok_packets::StatUpType;
    let name = format!("HlStats{}", std::process::id() % 100000);
    let (created, baseline) = create_temporary_character(config, &name)?;
    let check = (|| {
        if created.stat_points != 48 {
            return Err(format!("expected 48 starting points, got {}", created.stat_points));
        }
        let context = TestContext::connect_with_starting_stats(config, &name, &[
            StatUpType::Agility { amount: 2 },
            StatUpType::Vitality { amount: 8 },
            StatUpType::Intelligence { amount: 8 },
            StatUpType::Dexterity { amount: 6 },
        ])?;
        drop(context);
        sleep(Duration::from_millis(700));
        let (_, characters) = connect_to_character_select(config)?;
        let actual = characters
            .iter()
            .find(|c| c.character_id == created.character_id)
            .ok_or("created character missing")?;
        let stats = [
            actual.strength,
            actual.agility,
            actual.vitality,
            actual.intelligence,
            actual.dexterity,
            actual.luck,
        ];
        println!("         persisted starting stats: {stats:?}, unspent={}", actual.stat_points);
        if stats != [1, 3, 9, 9, 7, 1] || actual.stat_points != 0 {
            return Err(format!(
                "starting allocation was not persisted: {stats:?}, unspent={}",
                actual.stat_points
            ));
        }
        Ok(())
    })();
    let cleanup = delete_and_verify_absent(config, &created, &name, &baseline);
    check.and(cleanup)
}

/// Observe the actual job/level broadcasts after both kinds of change.
/// No shared test character's job, stats or existing party is changed.
fn party_job_level_refresh(config: &Config) -> Result<(), String> {
    let name = format!("HlParty{}", std::process::id() % 100000);
    let (created, baseline) = create_temporary_character(config, &name)?;
    let partner_config = Config {
        username: config.partner_username.clone(),
        password: config.partner_password.clone(),
        ..config.clone()
    };
    let partner_name = format!("HlRoster{}", std::process::id() % 100000);
    let partner_created = create_temporary_character(&partner_config, &partner_name);
    let (partner_created, partner_baseline) = match partner_created {
        Ok(value) => value,
        Err(error) => {
            let _ = delete_and_verify_absent(config, &created, &name, &baseline);
            return Err(error);
        }
    };
    let check = (|| {
        let mut context = TestContext::connect_as(config, &config.username, &config.password, Some(&name), None)?;
        let mut partner = TestContext::connect_with_starting_stats(&partner_config, &partner_name, &[])?;
        super::social::form_party(&mut context, &mut partner)?;
        context.flush();
        context.say("@jobchange 4")?;
        let subject = context.account_id;
        context.wait_for("0x0ABD acolyte update", |event| match event {
            NetworkEvent::PartyMemberJobAndLevel {
                account_id,
                job_id,
                base_level,
            } if *account_id == subject && job_id.0 == 4 => {
                println!("         0x0ABD job={} level={base_level}", job_id.0);
                Some(())
            }
            _ => None,
        })?;
        // Changing leader broadcasts the full cached roster without a level
        // update/relogin first repairing the cached job through the char server.
        context.flush();
        context.net.change_party_leader(partner.account_id).map_err(|_| "disconnected")?;
        let roster_job = context.wait_for("full roster after job change", |event| match event {
            NetworkEvent::PartyList { members, .. } => members.iter().find(|m| m.account_id == subject).map(|m| m.job_id.0),
            _ => None,
        })?;
        println!("         full roster job={roster_job} (expected 4)");
        if roster_job != 4 {
            super::social::leave_party_both(&mut context, &mut partner);
            return Err(format!("full party roster reverted Acolyte to job {roster_job}"));
        }
        context.flush();
        context.say("@baselevel 1")?;
        context.wait_for("0x0ABD level update", |event| match event {
            NetworkEvent::PartyMemberJobAndLevel {
                account_id,
                job_id,
                base_level,
            } if *account_id == subject && job_id.0 == 4 && *base_level == 2 => Some(()),
            _ => None,
        })?;
        super::social::leave_party_both(&mut context, &mut partner);
        Ok(())
    })();
    let cleanup = delete_and_verify_absent(config, &created, &name, &baseline);
    let partner_cleanup = delete_and_verify_absent(&partner_config, &partner_created, &partner_name, &partner_baseline);
    check.and(cleanup).and(partner_cleanup)
}

/// Connect to character select, pick a free slot, and create `temp_name`
/// there. Returns the created character plus the pre-existing character list
/// (the baseline for no-collateral-mutation assertions).
fn create_temporary_character(
    config: &Config,
    temp_name: &str,
) -> Result<
    (
        ragnarok_packets::CharacterInformation,
        Vec<ragnarok_packets::CharacterInformation>,
    ),
    String,
> {
    let (mut session, characters) = connect_to_character_select(config)?;

    if characters.iter().any(|c| c.name == temp_name) {
        return Err(format!("leftover temp character {temp_name} exists — delete manually"));
    }

    let used_slots: Vec<u8> = characters.iter().map(|c| c.character_number as u8).collect();
    let free_slot = (0..9u8).find(|slot| !used_slots.contains(slot)).ok_or("no free character slot")?;

    session
        .0
        .create_character(free_slot as usize, temp_name.to_owned(), Sex::Male, DEFAULT_HAIR_STYLE)
        .map_err(|_| "disconnected")?;
    let created = wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterCreated { character_information } => Some(Ok(character_information.clone())),
        NetworkEvent::CharacterCreationFailed { message, .. } => Some(Err(format!("creation failed: {message}"))),
        _ => None,
    })??;

    if created.name != temp_name {
        return Err(format!("created character has name {:?}, expected {temp_name:?}", created.name));
    }
    if created.character_number as u8 != free_slot {
        return Err(format!(
            "created character landed in slot {}, expected the requested slot {free_slot}",
            created.character_number
        ));
    }

    drop(session);
    sleep(Duration::from_millis(700));
    Ok((created, characters))
}

/// Post-creation checks: persistence across a fresh session and duplicate-name
/// rejection without character-list mutation.
fn character_create_checks(config: &Config, created: &ragnarok_packets::CharacterInformation, temp_name: &str) -> Result<(), String> {
    let (mut session, characters) = connect_to_character_select(config)?;

    let persisted = characters
        .iter()
        .find(|c| c.character_id == created.character_id)
        .ok_or("created character missing after reconnect")?;
    if persisted.character_number != created.character_number {
        return Err(format!(
            "created character moved from slot {} to slot {} across sessions",
            created.character_number, persisted.character_number
        ));
    }
    if persisted.name != temp_name {
        return Err(format!("created character renamed to {:?} across sessions", persisted.name));
    }

    // A second character with the same name must be rejected.
    let used_slots: Vec<u8> = characters.iter().map(|c| c.character_number as u8).collect();
    let second_slot = (0..9u8)
        .find(|slot| !used_slots.contains(slot))
        .ok_or("no free character slot for the duplicate-name attempt")?;
    session
        .0
        .create_character(second_slot as usize, temp_name.to_owned(), Sex::Male, DEFAULT_HAIR_STYLE)
        .map_err(|_| "disconnected")?;
    wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterCreationFailed { .. } => Some(Ok(())),
        NetworkEvent::CharacterCreated { .. } => Some(Err("duplicate character name was accepted".to_owned())),
        _ => None,
    })??;

    // The failed creation must not have mutated the character list.
    session.0.request_character_list().map_err(|_| "disconnected")?;
    let after = wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterList { characters } => Some(characters.clone()),
        _ => None,
    })?;
    if after.len() != characters.len() {
        return Err(format!(
            "character list changed after rejected duplicate creation: {} entries, expected {}",
            after.len(),
            characters.len()
        ));
    }
    for before in &characters {
        let unchanged = after
            .iter()
            .any(|c| c.character_id == before.character_id && c.character_number == before.character_number && c.name == before.name);
        if !unchanged {
            return Err(format!(
                "character {:?} (id {}) changed after rejected duplicate creation",
                before.name, before.character_id.0
            ));
        }
    }
    Ok(())
}

/// Delete `created` (if present), reconnect, and assert it is gone while every
/// baseline character kept its identity and slot.
fn delete_and_verify_absent(
    config: &Config,
    created: &ragnarok_packets::CharacterInformation,
    temp_name: &str,
    baseline: &[ragnarok_packets::CharacterInformation],
) -> Result<(), String> {
    let (mut session, characters) = connect_to_character_select(config)?;

    if characters.iter().any(|c| c.character_id == created.character_id) {
        session.0.delete_character(created.character_id).map_err(|_| "disconnected")?;
        wait_char_event(&mut session, config.timeout, &mut |event| match event {
            NetworkEvent::CharacterDeleted => Some(Ok(())),
            NetworkEvent::CharacterDeletionFailed { message, .. } => Some(Err(format!("deletion failed: {message}"))),
            _ => None,
        })??;
    }
    drop(session);
    sleep(Duration::from_millis(700));

    let (_, characters) = connect_to_character_select(config)?;
    if characters
        .iter()
        .any(|c| c.character_id == created.character_id || c.name == temp_name)
    {
        return Err(format!("character {temp_name} still present after deletion and reconnect"));
    }
    assert_baseline_unchanged(&characters, &Some(created.clone()), baseline)
}

/// Every baseline character (minus the disposable one) must still exist with
/// the same ID, slot, and name.
fn assert_baseline_unchanged(
    characters: &[ragnarok_packets::CharacterInformation],
    disposable: &Option<ragnarok_packets::CharacterInformation>,
    baseline: &[ragnarok_packets::CharacterInformation],
) -> Result<(), String> {
    for before in baseline {
        if disposable.as_ref().is_some_and(|d| d.character_id == before.character_id) {
            continue;
        }
        let unchanged = characters
            .iter()
            .any(|c| c.character_id == before.character_id && c.character_number == before.character_number && c.name == before.name);
        if !unchanged {
            return Err(format!(
                "collateral change: character {:?} (id {}, slot {}) no longer matches",
                before.name, before.character_id.0, before.character_number
            ));
        }
    }
    Ok(())
}

/// Log out from the map server and log back in cleanly, proving both
/// sessions are actionable with chat markers.
/// Guards the **map-server `SC_NOTIFY_BAN` (0x0081)** model — the reason a
/// player is given when the server throws them out.
///
/// The find this protects: `0x0081` is sent by the login, character **and map**
/// servers, and only the first two had it modelled
/// (`LoginFailedPacket` derives `LoginServer, CharacterServer`). On the map
/// connection it is the **only** explanation a player ever gets for a kick, a
/// ban, a shutdown, or someone else logging into their account — and the client
/// dropped it and bounced to character select in silence. **The same header can
/// mean different things on different connections.**
///
/// It took four rounds to fix because each round failed at a different stage
/// (wire → handler → surface lifetime → draw order), and until now it was
/// guarded by nothing at all: it was verified by eye once, in a GUI pass.
///
/// This covers the **wire half only**, which is the half a merge can silently
/// break — that the packet is modelled on the map connection and produces
/// `MapDisconnectReason` before the socket closes. Whether the popup then
/// survives the trip to character select is a draw-order question in
/// `korangar/src`, which no headless scenario can reach.
///
/// The ordering is the delicate part and it is why this asserts on collected
/// events rather than waiting: `clif_authfail_fd` calls `sockt->eof(fd)` in the
/// same breath as the send, so the reason and the disconnect arrive together.
/// This context marks the disconnect as expected before the kick. Ordinary
/// disconnects remain fatal; this one keeps the reason and disconnect events
/// available so their ordering can be asserted together.
fn kick_explains_itself(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;

    let target = partner.character_name.clone();
    partner.expect_disconnect();
    partner.flush();

    primary.say(&format!("@kick {target}"))?;

    let events = partner.collect_for(Duration::from_secs(8));
    let message = events
        .iter()
        .find_map(|event| match event {
            NetworkEvent::MapDisconnectReason { message } => Some(message.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            let disconnected = events
                .iter()
                .any(|event| matches!(event, NetworkEvent::MapServerDisconnected { .. }));
            format!(
                "@kick dropped the session with no MapDisconnectReason (disconnect seen: {disconnected}). The map-server half of 0x0081 \
                 is unmodelled again — check that the packet is registered on the MAP connection and not only for \
                 LoginServer/CharacterServer, since the same header carries a different reason table there"
            )
        })?;

    if message.trim().is_empty() {
        return Err(
            "the kick reason arrived empty — the packet is registered but its reason did not resolve to any text, which reaches the \
             player as a blank popup"
                .to_owned(),
        );
    }

    Ok(())
}

/// The **kicking DM's** half of the same interaction — `kick-explains-itself`
/// covers only the target's.
///
/// `GmKickResponsePacket` (0x00CD) is the sole acknowledgement a successful
/// `@kick` produces anywhere: `ACMD(kick)` returns immediately after
/// `clif->GM_kick` and prints nothing (`atcommand.c:3450`), and that function's
/// entire feedback path is `clif->GM_kickack(sd, 1)` (`clif.c:9410`). While the
/// client registered it as a no-op, a DM typed the command, watched the target
/// vanish, and was told nothing — indistinguishable from a command that never
/// arrived.
///
/// **The assertion is the text, not "a message arrived".** The kicker's
/// connection is not quiet: `@kick`'s own failure paths (no name, character not
/// found, outranked) all print through `clif->message` and land in the same
/// place, so "some chat appeared" would pass with the handler deleted, on a
/// kick that failed. The literal is repeated here rather than shared with the
/// handler on purpose — a constant imported from both sides would survive its
/// own rename and stop testing anything.
fn kick_confirms_to_the_kicker(config: &Config) -> Result<(), String> {
    const CONFIRMATION: &str = "The player has been disconnected.";

    let (mut primary, mut partner) = TestContext::connect_pair(config)?;

    let target = partner.character_name.clone();
    partner.expect_disconnect();
    primary.flush();

    primary.say(&format!("@kick {target}"))?;

    let events = primary.collect_for(Duration::from_secs(8));
    let chat: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            NetworkEvent::ChatMessage { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();

    if chat.iter().any(|text| text.contains(CONFIRMATION)) {
        return Ok(());
    }

    Err(format!(
        "`@kick {target}` told the kicker nothing — no chat line contained {CONFIRMATION:?}. The kicker saw: {chat:?}. Either 0x00CD is \
         registered as a no-op again (it is the only acknowledgement a successful kick produces; the target's own 0x0081 goes to the \
         target, not here), or the kick did not succeed at all — a failure prints its own `clif->message` line, which would appear in \
         that list"
    ))
}

fn logout_relogin(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    let marker = format!("[headless-tester] pre-logout {}", std::process::id());
    context.flush();
    context.say(&marker)?;
    context.wait_for("pre-logout chat echo", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains(&marker) => Some(()),
        _ => None,
    })?;

    context.net.log_out().map_err(|_| "disconnected")?;
    context.wait_for("LoggedOut", |event| match event {
        NetworkEvent::LoggedOut => Some(()),
        _ => None,
    })?;
    context.net.disconnect_from_map_server();
    drop(context);
    sleep(Duration::from_millis(700));

    // Second full session proves the first logout was clean (no lingering
    // "already online" terminal failure) and is itself actionable.
    let mut second = TestContext::connect(config)?;
    let marker = format!("[headless-tester] post-relogin {}", std::process::id());
    second.flush();
    second.say(&marker)?;
    second.wait_for("post-relogin chat echo", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains(&marker) => Some(()),
        _ => None,
    })?;
    second.net.disconnect_from_map_server();
    Ok(())
}

/// `@die`, then respawn at the save point: the destination map must match the
/// save point, HP must recover above zero, and the session must accept a
/// movement request afterwards.
fn respawn(config: &Config) -> Result<(), String> {
    const SAVE_MAP: &str = "prontera";

    let mut context = TestContext::connect(config)?;

    // Pin the save point to a known location so the respawn destination is
    // deterministic instead of whatever the character last saved.
    context.warp(SAVE_MAP, 155, 180)?;
    context.say("@save")?;
    context.pump(Duration::from_millis(300));

    // Death is signaled by the removal of our own entity with reason `Died`
    // (there is no HealthPoints(0) stat update — clients must key off this).
    let player_id = context.player_id;
    context.flush();
    context.say("@die")?;
    context.wait_for("own-entity RemoveEntity (Died)", |event| match event {
        NetworkEvent::RemoveEntity {
            entity_id,
            reason: ragnarok_packets::DisappearanceReason::Died,
        } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.respawn().map_err(|_| "disconnected")?;
    let map_name = context.wait_for("ChangeMap to save point", |event| match event {
        NetworkEvent::ChangeMap { map_name, .. } => Some(map_name.clone()),
        _ => None,
    })?;
    if map_name != SAVE_MAP {
        return Err(format!("respawned on {map_name:?}, expected save point {SAVE_MAP:?}"));
    }
    context.net.map_loaded().map_err(|_| "disconnected")?;

    // HP must come back above zero with the respawn stat burst.
    context.wait_for("HealthPoints > 0 after respawn", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::HealthPoints(value),
        } if *value > 0 => Some(()),
        _ => None,
    })?;

    // A movement round trip proves the revived session is actionable.
    let position = context.position;
    context.walk_to(position.x + 2, position.y)?;

    // Heal back up so later scenarios start healthy.
    context.say("@heal")?;
    context.pump(Duration::from_millis(300));
    context.net.disconnect_from_map_server();
    Ok(())
}

// --- character-server-only helpers -----------------------------------------

type CharSession = (NetworkingSystem<crate::ledger::Ledger>, korangar_networking::NetworkEventBuffer);

/// Connect up to (and including) the character list, without selecting.
fn connect_to_character_select(config: &Config) -> Result<(CharSession, Vec<ragnarok_packets::CharacterInformation>), String> {
    connect_to_character_select_as(config, &config.username, &config.password)
}

fn connect_to_character_select_as(
    config: &Config,
    username: &str,
    password: &str,
) -> Result<(CharSession, Vec<ragnarok_packets::CharacterInformation>), String> {
    let mut last_error = String::new();
    for attempt in 0..4 {
        if attempt > 0 {
            sleep(Duration::from_secs(4));
        }
        match try_connect_to_character_select(config, username, password) {
            Ok(session) => return Ok(session),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

fn try_connect_to_character_select(
    config: &Config,
    username: &str,
    password: &str,
) -> Result<(CharSession, Vec<ragnarok_packets::CharacterInformation>), String> {
    let (mut net, buffer) = NetworkingSystem::spawn_with_callback(config.ledger.clone());
    net.connect_to_login_server(PACKET_VERSION, config.server, username.to_owned(), password.to_owned());

    let mut session = (net, buffer);

    let (character_servers, login_data) = wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::LoginServerConnected {
            character_servers,
            login_data,
        } => Some(Ok((character_servers.clone(), login_data.clone()))),
        NetworkEvent::LoginServerConnectionFailed { message, .. } => Some(Err(format!("login failed: {message}"))),
        _ => None,
    })??;

    session.0.disconnect_from_login_server();
    let server = character_servers.first().ok_or("no character servers")?.clone();
    session.0.connect_to_character_server(PACKET_VERSION, &login_data, server);

    wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterServerConnected { .. } => Some(Ok(())),
        NetworkEvent::CharacterServerConnectionFailed { message, .. } => Some(Err(format!("char server failed: {message}"))),
        _ => None,
    })??;

    session.0.request_character_list().map_err(|_| "disconnected")?;
    let characters = wait_char_event(&mut session, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterList { characters } => Some(Ok::<_, String>(characters.clone())),
        _ => None,
    })??;

    Ok((session, characters))
}

fn wait_char_event<T>(
    session: &mut CharSession,
    timeout: Duration,
    matcher: &mut impl FnMut(&NetworkEvent) -> Option<T>,
) -> Result<T, String> {
    let deadline = Instant::now() + timeout;
    loop {
        session.0.get_events(&mut session.1);
        for event in session.1.drain() {
            if let Some(value) = matcher(&event) {
                return Ok(value);
            }
        }
        if Instant::now() > deadline {
            return Err("timed out waiting for character server event".to_owned());
        }
        sleep(Duration::from_millis(30));
    }
}
