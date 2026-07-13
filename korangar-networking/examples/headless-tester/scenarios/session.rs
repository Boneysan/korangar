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

    let (mut session, characters) = connect(config).map_err(|error| {
        format!("partner account unavailable (run a phase 8 scenario once to create it): {error}")
    })?;
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
            NetworkEvent::CharacterSlotSwitchFailed => Some(Err(
                "slot switch rejected — grant the entitlement fixture with tools/testing/fixtures/grant-slotchange.sql".to_owned(),
            )),
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

/// Create a character in a free slot, then delete it again.
fn character_create_delete(config: &Config) -> Result<(), String> {
    // Connect only as far as the character server.
    let mut context = TestContext::connect(config)?;
    // The main context is already in-game; use a second connection for
    // character management so we exercise a fresh character-server session.
    context.net.disconnect_from_map_server();
    drop(context);
    sleep(Duration::from_millis(700));

    let temp_name = format!("HlTmp{}", std::process::id() % 100000);

    let (mut char_context, characters) = connect_to_character_select(config)?;

    if characters.iter().any(|c| c.name == temp_name) {
        return Err(format!("leftover temp character {temp_name} exists — delete manually"));
    }

    let used_slots: Vec<u8> = characters.iter().map(|c| c.character_number as u8).collect();
    let free_slot = (0..9u8).find(|slot| !used_slots.contains(slot)).ok_or("no free character slot")?;

    char_context
        .0
        .create_character(free_slot as usize, temp_name.clone())
        .map_err(|_| "disconnected")?;
    let created = wait_char_event(&mut char_context, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterCreated { character_information } => Some(Ok(character_information.clone())),
        NetworkEvent::CharacterCreationFailed { message, .. } => Some(Err(format!("creation failed: {message}"))),
        _ => None,
    })??;

    if created.name != temp_name {
        return Err(format!("created character has name {:?}, expected {temp_name:?}", created.name));
    }

    char_context.0.delete_character(created.character_id).map_err(|_| "disconnected")?;
    wait_char_event(&mut char_context, config.timeout, &mut |event| match event {
        NetworkEvent::CharacterDeleted => Some(Ok(())),
        NetworkEvent::CharacterDeletionFailed { message, .. } => Some(Err(format!("deletion failed: {message}"))),
        _ => None,
    })??;

    Ok(())
}

/// Log out from the map server and log back in cleanly.
fn logout_relogin(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.net.log_out().map_err(|_| "disconnected")?;
    context.wait_for("LoggedOut", |event| match event {
        NetworkEvent::LoggedOut => Some(()),
        _ => None,
    })?;
    context.net.disconnect_from_map_server();
    drop(context);
    sleep(Duration::from_millis(700));

    // Second full session proves the first logout was clean.
    let mut second = TestContext::connect(config)?;
    second.net.disconnect_from_map_server();
    Ok(())
}

/// `@die`, then respawn at the save point.
fn respawn(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

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
    context.wait_for("ChangeMap to save point", |event| match event {
        NetworkEvent::ChangeMap { .. } => Some(()),
        _ => None,
    })?;
    context.net.map_loaded().map_err(|_| "disconnected")?;

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
