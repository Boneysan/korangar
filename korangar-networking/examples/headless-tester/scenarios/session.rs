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
        Scenario::new("logout-relogin", 1, logout_relogin),
        Scenario::new("respawn", 1, respawn),
    ]
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
    let (mut net, buffer) = NetworkingSystem::spawn_with_callback(config.ledger.clone());
    net.connect_to_login_server(PACKET_VERSION, config.server, config.username.clone(), config.password.clone());

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
