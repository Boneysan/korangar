//! Session context for scenarios: connection bootstrap, event waiting with
//! diagnostics, GM command helpers, and lightweight client-state tracking.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::{Duration, Instant};

use korangar_networking::{
    DisconnectReason, EntityData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem, SupportedPacketVersion,
};
use ragnarok_packets::{
    AccountId, CharacterId, CharacterInformation, Direction, EntityId, InventoryIndex, JobId, SkillInformation, TilePosition, WorldPosition,
};

use crate::ledger::Ledger;

pub const PACKET_VERSION: SupportedPacketVersion = SupportedPacketVersion::_20220406;

/// How often the event buffer is polled while waiting.
const POLL_INTERVAL: Duration = Duration::from_millis(30);
/// How many recent events are kept for timeout diagnostics.
const EVENT_LOG_CAPACITY: usize = 40;

#[derive(Clone)]
pub struct Config {
    pub server: SocketAddr,
    pub username: String,
    pub password: String,
    pub character: Option<String>,
    pub partner_username: String,
    pub partner_password: String,
    pub timeout: Duration,
    pub ledger: Ledger,
}

pub struct TestContext {
    pub net: NetworkingSystem<Ledger>,
    buffer: NetworkEventBuffer,
    /// Events drained but not yet consumed by a matcher.
    pending: Vec<NetworkEvent>,
    /// Rolling log of recent events (debug strings) for timeout diagnostics.
    event_log: Vec<String>,
    pub timeout: Duration,

    // --- identity ---
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub player_id: EntityId,
    pub character_name: String,
    pub characters: Vec<CharacterInformation>,

    // --- tracked client state (updated for every drained event) ---
    pub base_level: u32,
    pub job_id: JobId,
    pub zeny: u32,
    pub health_points: u32,
    pub map_name: String,
    pub position: TilePosition,
    pub entities: HashMap<EntityId, EntityData>,
    pub inventory: Vec<korangar_networking::InventoryItem<korangar_networking::NoMetadata>>,
    pub skills: Vec<SkillInformation>,
}

impl TestContext {
    /// Full login → character select → map load flow. Retries once to absorb
    /// a lingering "already online" session from a previous run.
    pub fn connect(config: &Config) -> Result<Self, String> {
        Self::connect_as(config, &config.username, &config.password, config.character.as_deref(), None)
    }

    /// Connect with explicit credentials. `create_character`: name to create
    /// if the account has no characters (used for the partner account).
    pub fn connect_as(
        config: &Config,
        username: &str,
        password: &str,
        character: Option<&str>,
        create_character: Option<&str>,
    ) -> Result<Self, String> {
        let mut last_error = String::new();
        for attempt in 0..4 {
            if attempt > 0 {
                // Two causes need waiting out: a lingering "already online"
                // ghost session, and Hercules' prevent_logout window (~10s
                // after combat) which delays the previous session's release.
                sleep(Duration::from_secs(4));
            }
            match Self::try_connect(config, username, password, character, create_character) {
                Ok(context) => return Ok(context),
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    fn try_connect(
        config: &Config,
        username: &str,
        password: &str,
        character: Option<&str>,
        create_character: Option<&str>,
    ) -> Result<Self, String> {
        let (net, buffer) = NetworkingSystem::spawn_with_callback(config.ledger.clone());

        let mut context = Self {
            net,
            buffer,
            pending: Vec::new(),
            event_log: Vec::new(),
            timeout: config.timeout,
            account_id: AccountId(0),
            character_id: CharacterId(0),
            player_id: EntityId(0),
            character_name: String::new(),
            characters: Vec::new(),
            base_level: 0,
            job_id: JobId(0),
            zeny: 0,
            health_points: 0,
            map_name: String::new(),
            position: TilePosition { x: 0, y: 0 },
            entities: HashMap::new(),
            inventory: Vec::new(),
            skills: Vec::new(),
        };

        context
            .net
            .connect_to_login_server(PACKET_VERSION, config.server, username.to_owned(), password.to_owned());

        let (character_servers, login_data) = context.wait_for("LoginServerConnected", |event| match event {
            NetworkEvent::LoginServerConnected {
                character_servers,
                login_data,
            } => Some(Ok((character_servers.clone(), login_data.clone()))),
            NetworkEvent::LoginServerConnectionFailed { message, .. } => Some_err(format!("login failed: {message}")),
            _ => None,
        })??;

        context.account_id = login_data.account_id;
        context.player_id = EntityId(login_data.account_id.0);

        context.net.disconnect_from_login_server();
        let character_server = character_servers.first().ok_or("no character servers listed")?.clone();
        context
            .net
            .connect_to_character_server(PACKET_VERSION, &login_data, character_server);

        context.wait_for("CharacterServerConnected", |event| match event {
            NetworkEvent::CharacterServerConnected { .. } => Some(Ok(())),
            NetworkEvent::CharacterServerConnectionFailed { message, .. } => Some_err(format!("char server failed: {message}")),
            _ => None,
        })??;

        context.net.request_character_list().map_err(|_| "disconnected")?;
        let characters = context.wait_for("CharacterList", |event| match event {
            NetworkEvent::CharacterList { characters } => Some(characters.clone()),
            _ => None,
        })?;
        context.characters = characters;

        let slot = match character {
            Some(name) => context
                .characters
                .iter()
                .find(|info| info.name == name)
                .map(|info| info.character_number as usize),
            None => context.characters.first().map(|info| info.character_number as usize),
        };

        let slot = match (slot, create_character) {
            (Some(slot), _) => slot,
            (None, Some(new_name)) => {
                context.net.create_character(0, new_name.to_owned()).map_err(|_| "disconnected")?;
                let info = context.wait_for("CharacterCreated", |event| match event {
                    NetworkEvent::CharacterCreated { character_information } => Some(Ok(character_information.clone())),
                    NetworkEvent::CharacterCreationFailed { message, .. } => Some_err(format!("creation failed: {message}")),
                    _ => None,
                })??;
                let slot = info.character_number as usize;
                context.characters.push(info);
                slot
            }
            (None, None) => return Err(format!("character {character:?} not found and creation disabled")),
        };

        let info = context
            .characters
            .iter()
            .find(|info| info.character_number as usize == slot)
            .unwrap()
            .clone();
        context.character_name = info.name.clone();
        context.character_id = info.character_id;
        context.job_id = info.job_id;
        context.base_level = info.base_level as u32;

        context.net.select_character(slot).map_err(|_| "disconnected")?;
        let map_login_data = context.wait_for("CharacterSelected", |event| match event {
            NetworkEvent::CharacterSelected { login_data } => Some(Ok(login_data.clone())),
            NetworkEvent::CharacterSelectionFailed { message, .. } => Some_err(format!("selection failed: {message}")),
            _ => None,
        })??;

        context.net.disconnect_from_character_server();
        context.net.connect_to_map_server(PACKET_VERSION, &login_data, map_login_data);

        context.wait_for("map server login (UpdateClientTick)", |event| match event {
            NetworkEvent::UpdateClientTick { .. } => Some(()),
            _ => None,
        })?;
        context.net.map_loaded().map_err(|_| "disconnected")?;

        // Let the initial burst (inventory, skill tree, stats, entities) land.
        context.pump(Duration::from_millis(800));

        Ok(context)
    }

    /// Connect the configured second account, registering it and creating a
    /// character on first use. Hercules' `_M` suffix is only sent for the
    /// registration attempt; subsequent runs use the stable account name.
    pub fn connect_partner(config: &Config) -> Result<Self, String> {
        const CHARACTER_NAME: &str = "HeadlessTwo";
        match Self::connect_as(
            config,
            &config.partner_username,
            &config.partner_password,
            Some(CHARACTER_NAME),
            Some(CHARACTER_NAME),
        ) {
            Ok(context) => Ok(context),
            Err(first_error) if first_error.contains("login failed") => {
                let registration_name = format!("{}_M", config.partner_username);
                match Self::try_connect(
                    config,
                    &registration_name,
                    &config.partner_password,
                    Some(CHARACTER_NAME),
                    Some(CHARACTER_NAME),
                ) {
                    Ok(context) => Ok(context),
                    Err(registration_result) => {
                        // Hercules may create the account and still close this
                        // registration connection. Retry using the stable name
                        // rather than repeatedly sending the `_M` suffix.
                        sleep(Duration::from_secs(1));
                        Self::connect_as(
                            config,
                            &config.partner_username,
                            &config.partner_password,
                            Some(CHARACTER_NAME),
                            Some(CHARACTER_NAME),
                        )
                        .map_err(|error| {
                            format!(
                                "partner login failed ({first_error}); registration returned ({registration_result}); retry failed \
                                 ({error})"
                            )
                        })
                    }
                }
            }
            Err(error) => Err(error),
        }
    }

    // --- event machinery -------------------------------------------------

    /// Drain the network buffer once, doing state bookkeeping on every event.
    fn poll(&mut self) -> Result<(), String> {
        self.net.get_events(&mut self.buffer);
        let events: Vec<NetworkEvent> = self.buffer.drain().collect();
        for event in events {
            self.track(&event);
            let line = format!("{event:?}");
            self.event_log
                .push(if line.len() > 160 { format!("{}…", &line[..160]) } else { line });
            if self.event_log.len() > EVENT_LOG_CAPACITY {
                self.event_log.remove(0);
            }
            if let NetworkEvent::MapServerDisconnected {
                reason: DisconnectReason::ConnectionError,
            } = &event
            {
                return Err("map server connection error (possible desync — check ledger for failed packets)".to_owned());
            }
            self.pending.push(event);
        }
        Ok(())
    }

    /// Drain events for `duration`, keeping them available to later waits.
    pub fn pump(&mut self, duration: Duration) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            let _ = self.poll();
            sleep(POLL_INTERVAL);
        }
    }

    /// Discard all pending events (call before the action you want to observe).
    pub fn flush(&mut self) {
        let _ = self.poll();
        self.pending.clear();
    }

    /// Wait (default timeout) until `matcher` returns `Some` for an event.
    /// Consumes only the matched event; unmatched ones stay available.
    pub fn wait_for<T>(&mut self, what: &str, mut matcher: impl FnMut(&NetworkEvent) -> Option<T>) -> Result<T, String> {
        self.wait_for_within(what, self.timeout, &mut matcher)
    }

    pub fn wait_for_within<T>(
        &mut self,
        what: &str,
        timeout: Duration,
        matcher: &mut impl FnMut(&NetworkEvent) -> Option<T>,
    ) -> Result<T, String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(index_value) = self
                .pending
                .iter()
                .enumerate()
                .find_map(|(index, event)| matcher(event).map(|value| (index, value)))
            {
                self.pending.remove(index_value.0);
                return Ok(index_value.1);
            }

            if Instant::now() > deadline {
                let mut message = format!("timed out after {:?} waiting for {what}. Recent events:", timeout);
                for line in self.event_log.iter().rev().take(12).rev() {
                    let _ = write!(message, "\n    {line}");
                }
                return Err(message);
            }

            sleep(POLL_INTERVAL);
            self.poll()?;
        }
    }

    /// Collect every event arriving within `duration` (plus already-pending
    /// ones), consuming them.
    pub fn collect_for(&mut self, duration: Duration) -> Vec<NetworkEvent> {
        self.pump(duration);
        std::mem::take(&mut self.pending)
    }

    // --- state bookkeeping ------------------------------------------------

    fn track(&mut self, event: &NetworkEvent) {
        use ragnarok_packets::StatType;

        match event {
            NetworkEvent::UpdateStat { stat_type } => match stat_type {
                StatType::BaseLevel(value) => self.base_level = *value,
                StatType::Zeny(value) => self.zeny = *value,
                StatType::HealthPoints(value) => self.health_points = *value,
                _ => {}
            },
            NetworkEvent::ChangeJob { account_id, job_id } if account_id.0 == self.account_id.0 => {
                self.job_id = *job_id;
            }
            NetworkEvent::ChangeMap { map_name, position } => {
                self.map_name = map_name.clone();
                self.position = *position;
                self.entities.clear();
                let _ = self.net.map_loaded();
            }
            NetworkEvent::PlayerMove { destination, .. } => {
                self.position = destination.tile_position();
            }
            NetworkEvent::AddEntity { entity_data } => {
                self.entities.insert(entity_data.entity_id, entity_data.clone());
            }
            NetworkEvent::EntityMove {
                entity_id, destination, ..
            } => {
                if let Some(entity) = self.entities.get_mut(entity_id) {
                    entity.position = *destination;
                }
            }
            NetworkEvent::EntitySlide { entity_id, position } => {
                if entity_id.0 == self.player_id.0 {
                    self.position = *position;
                } else if let Some(entity) = self.entities.get_mut(entity_id) {
                    entity.position = WorldPosition::new(position.x, position.y, Direction::North);
                }
            }
            NetworkEvent::RemoveEntity { entity_id, .. } => {
                self.entities.remove(entity_id);
            }
            NetworkEvent::SetInventory { items } => {
                self.inventory = items.clone();
            }
            NetworkEvent::IventoryItemAdded { item } => {
                if let Some(existing) = self.inventory.iter_mut().find(|existing| existing.index == item.index) {
                    *existing = item.clone();
                } else {
                    self.inventory.push(item.clone());
                }
            }
            NetworkEvent::InventoryItemRemoved { index, .. } => {
                self.inventory.retain(|item| item.index != *index);
            }
            NetworkEvent::SkillTree { skill_information } => {
                self.skills = skill_information.clone();
            }
            _ => {}
        }
    }

    // --- high-level helpers -----------------------------------------------

    /// Send a chat line / GM command (no feedback expected).
    pub fn say(&mut self, text: &str) -> Result<(), String> {
        let name = self.character_name.clone();
        self.net.send_chat_message(&name, text).map_err(|_| "disconnected".to_owned())
    }

    /// Send a GM command and wait for any server-styled feedback line.
    pub fn gm_expect_feedback(&mut self, command: &str) -> Result<String, String> {
        self.flush();
        self.say(command)?;
        self.wait_for(&format!("server feedback for {command}"), |event| match event {
            NetworkEvent::ChatMessage {
                text,
                color: MessageColor::Server | MessageColor::Information | MessageColor::Error,
            } => Some(text.clone()),
            _ => None,
        })
    }

    /// Ensure the character has the given job (via `@job`), waiting for the
    /// `ChangeJob` round trip.
    pub fn ensure_job(&mut self, job_id: u16) -> Result<(), String> {
        if self.job_id.0 == job_id {
            return Ok(());
        }
        self.flush();
        self.say(&format!("@job {job_id}"))?;
        let account_id = self.account_id;
        self.wait_for(&format!("ChangeJob to {job_id}"), |event| match event {
            NetworkEvent::ChangeJob {
                account_id: event_account,
                job_id: event_job,
            } if event_account.0 == account_id.0 && event_job.0 == job_id => Some(Ok(())),
            NetworkEvent::ChatMessage {
                text,
                color: MessageColor::Server,
            } if text.contains("unable to change") || text.contains("failed") => {
                Some(Err(format!("Job change to {job_id} failed: {text}")))
            }
            _ => None,
        })?
    }

    /// Ensure the character is at the given base level (`@blevel` is relative).
    pub fn ensure_base_level(&mut self, target: u32) -> Result<(), String> {
        if self.base_level == target {
            return Ok(());
        }
        let delta = target as i64 - self.base_level as i64;
        self.flush();
        self.say(&format!("@blevel {delta}"))?;
        self.wait_for(&format!("BaseLevel == {target}"), |event| match event {
            NetworkEvent::UpdateStat {
                stat_type: ragnarok_packets::StatType::BaseLevel(value),
            } if *value == target => Some(()),
            _ => None,
        })
    }

    /// Warp via GM command and wait for the `ChangeMap` round trip. A warp to
    /// the tile we are already standing on is treated as success (the server
    /// rejects it with "You already are at your destination!").
    pub fn warp(&mut self, map: &str, x: u16, y: u16) -> Result<(), String> {
        self.flush();
        self.say(&format!("@warp {map} {x} {y}"))?;
        let moved = self.wait_for(&format!("ChangeMap to {map}"), |event| match event {
            NetworkEvent::ChangeMap { map_name, .. } if map_name == map => Some(Ok(true)),
            NetworkEvent::ChatMessage { text, .. } if text.contains("already are at your destination") => Some(Ok(false)),
            NetworkEvent::ChatMessage { text, .. } if text.contains("@warp failed") => {
                Some(Err(format!("@warp {map} {x} {y} rejected by server")))
            }
            _ => None,
        })??;

        if moved {
            // The server expects a load acknowledgement after every map change
            // before it will process further commands.
            self.net.map_loaded().map_err(|_| "disconnected")?;
            self.pump(Duration::from_millis(500));
        }
        self.map_name = map.to_owned();
        self.position = TilePosition { x, y };
        Ok(())
    }

    /// Warp to a random walkable cell of a map (`@warp <map>` without
    /// coordinates); needed for field maps whose safe cells we don't know.
    pub fn warp_random(&mut self, map: &str) -> Result<(), String> {
        self.flush();
        self.say(&format!("@warp {map}"))?;
        let position = self.wait_for(&format!("ChangeMap to {map}"), |event| match event {
            NetworkEvent::ChangeMap { map_name, position } if map_name == map => Some(*position),
            _ => None,
        })?;
        self.net.map_loaded().map_err(|_| "disconnected")?;
        self.map_name = map.to_owned();
        self.position = position;
        self.pump(Duration::from_millis(500));
        Ok(())
    }

    /// Walk to a tile and wait out the travel time.
    pub fn walk_to(&mut self, x: u16, y: u16) -> Result<(), String> {
        use ragnarok_packets::{Direction, WorldPosition};

        self.flush();
        self.net
            .player_move(WorldPosition::new(x, y, Direction::North))
            .map_err(|_| "disconnected")?;
        let (origin, destination) = self.wait_for("PlayerMove ack", |event| match event {
            NetworkEvent::PlayerMove { origin, destination, .. } => Some((origin.tile_position(), destination.tile_position())),
            _ => None,
        })?;

        let distance = (origin.x.abs_diff(destination.x)).max(origin.y.abs_diff(destination.y)) as u64;
        // Base walk speed is ~150ms/cell at speed 150; pad generously.
        self.pump(Duration::from_millis((distance * 200 + 400).min(4000)));
        self.position = destination;
        Ok(())
    }

    /// `@item` an item and return its inventory index.
    pub fn give_item(&mut self, item_id: u32, amount: u16) -> Result<InventoryIndex, String> {
        self.flush();
        self.say(&format!("@item {item_id} {amount}"))?;
        self.wait_for(&format!("inventory add of item {item_id}"), |event| match event {
            NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id => Some(item.index),
            _ => None,
        })
    }

    /// Spawn a monster next to the player and return its entity id.
    pub fn spawn_monster(&mut self, name: &str, mob_job_id: u16) -> Result<EntityId, String> {
        self.flush();
        self.say(&format!("@monster {name}"))?;
        self.wait_for(&format!("AddEntity for {name}"), |event| match event {
            NetworkEvent::AddEntity { entity_data } if entity_data.job_id.0 == mob_job_id => Some(entity_data.entity_id),
            _ => None,
        })
    }

    /// Kill every spawned monster on the map (cleanup).
    pub fn kill_all_monsters(&mut self) {
        let _ = self.say("@killmonster");
        self.pump(Duration::from_millis(400));
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Request a server-acknowledged logout, then close: without it, the
        // next scenario's login hits "account still flagged online".
        if self.net.log_out().is_ok() {
            let _ = self.wait_for_within("LoggedOut", Duration::from_secs(3), &mut |event| match event {
                NetworkEvent::LoggedOut => Some(()),
                _ => None,
            });
        }
        self.net.disconnect_from_map_server();
        sleep(Duration::from_millis(300));
    }
}

/// Helper to return an error from inside a `wait_for` matcher: wraps it so the
/// outer `??` surfaces it.
#[allow(non_snake_case)]
fn Some_err<T>(message: String) -> Option<Result<T, String>> {
    Some(Err(message))
}
