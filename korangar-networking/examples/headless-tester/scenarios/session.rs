//! Phase 1 — session lifecycle.

use std::thread::sleep;
use std::time::{Duration, Instant};

use korangar_networking::{NetworkEvent, NetworkingSystem};

use crate::context::{Config, PACKET_VERSION, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("smoke", 1, smoke),
        Scenario::new("bad-password", 1, bad_password),
        Scenario::new("character-create-delete", 1, character_create_delete),
        Scenario::new("character-delete-after-play", 1, character_delete_after_play),
        Scenario::new("character-slot-switch-rejected", 1, character_slot_switch_rejected),
        Scenario::new("character-slot-switch", 1, character_slot_switch),
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
        session.0.create_character(free_slot, name).map_err(|_| "disconnected")?;
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
        .create_character(free_slot as usize, temp_name.to_owned())
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
        .create_character(second_slot as usize, temp_name.to_owned())
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
