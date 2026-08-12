//! Session context for scenarios: connection bootstrap, event waiting with
//! diagnostics, GM command helpers, and lightweight client-state tracking.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::{Duration, Instant};

use korangar_networking::{
    DisconnectReason, EntityData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem, SupportedPacketVersion,
};
use ragnarok_packets::{
    AccountId, CharacterId, CharacterInformation, Direction, EntityId, InventoryIndex, ItemId, JobId, SkillInformation, SpriteChangeType,
    TilePosition, WorldPosition,
};

use crate::ledger::Ledger;

pub const PACKET_VERSION: SupportedPacketVersion = SupportedPacketVersion::_20220406;

/// A connection error seen by a context even when the immediate caller used a
/// best-effort pump/cleanup API. The runner consumes this after every scenario,
/// so a dropped error can never turn into PASS (or allowlisted skill silence).
static RECORDED_CONNECTION_ERROR: Mutex<Option<String>> = Mutex::new(None);

pub fn clear_recorded_connection_error() {
    if let Ok(mut error) = RECORDED_CONNECTION_ERROR.lock() {
        *error = None;
    }
}

pub fn take_recorded_connection_error() -> Option<String> {
    RECORDED_CONNECTION_ERROR.lock().ok().and_then(|mut error| error.take())
}

fn record_connection_error(error: &str) {
    if let Ok(mut recorded) = RECORDED_CONNECTION_ERROR.lock() {
        recorded.get_or_insert_with(|| error.to_owned());
    }
}

/// Where [`TestContext::connect_pair`] convenes both seats.
///
/// Open field, well clear of NPCs and warps, and — the property that actually
/// matters — **the same cells every run**, so a scenario that places a ground
/// unit gets ground it can place on. See `connect_pair` for what inheriting
/// this from the previous scenario used to cost.
pub const PAIR_VENUE: (&str, u16, u16) = ("prt_fild08", 286, 338);

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
    /// Sticky because a best-effort `pump` or `flush` may be the first code to
    /// observe the disconnect. The next checked wait must still see it.
    connection_error: Option<String>,
    /// One scenario (`kick-explains-itself`) deliberately asks the server to
    /// disconnect this context after sending a reason packet.
    expected_disconnect: bool,
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
    /// Equipped ammunition of everyone in view, keyed by account id.
    ///
    /// Deliberately *not* on `EntityData`, mirroring `ClientState` in the real
    /// client: ammunition is the one broadcast attribute with no spawn-packet
    /// slot, so it has no rebuild-safe home on the entity. Everything else in
    /// this file lives on the tracked `EntityData` for exactly the opposite
    /// reason.
    pub remote_ammunition: HashMap<AccountId, ItemId>,
    pub inventory: Vec<korangar_networking::InventoryItem<korangar_networking::NoMetadata>>,
    pub skills: Vec<SkillInformation>,
}

/// One observable attribute of another character, as this session currently
/// believes it to be.
///
/// The point of naming them is that a parity assertion reads the *observer's*
/// belief, never the actor's — the axis that made all six of the 2026-07-29
/// bugs invisible to single-client testing.
///
/// Several variants have no scenario yet. They are declared anyway because the
/// matrix is the deliverable — the remaining rows are "write a scenario", not
/// "extend the vocabulary". Headgear and robe need an item equipped rather than
/// a GM command, and body style needs a third-job costume (`@bodystyle` refuses
/// otherwise).
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    HairStyle,
    HairColor,
    ClothesColor,
    HeadTop,
    HeadMid,
    HeadBottom,
    Robe,
    BodyStyle,
    Weapon,
    Shield,
    JobId,
    /// Read from [`TestContext::remote_ammunition`], not from `EntityData`.
    Ammunition,
}

impl Appearance {
    fn read(self, entity: Option<&EntityData>, ammunition: Option<ItemId>) -> Option<u32> {
        match self {
            Self::Ammunition => Some(ammunition.map(|item| item.0).unwrap_or(0)),
            other => entity.map(|entity| match other {
                Self::HairStyle => entity.head as u32,
                Self::HairColor => entity.head_palette as u32,
                Self::ClothesColor => entity.body_palette as u32,
                Self::HeadTop => entity.accessory2 as u32,
                Self::HeadMid => entity.accessory3 as u32,
                Self::HeadBottom => entity.accessory as u32,
                Self::Robe => entity.robe as u32,
                Self::BodyStyle => entity.body as u32,
                Self::Weapon => entity.weapon,
                Self::Shield => entity.shield,
                Self::JobId => entity.job_id.0 as u32,
                Self::Ammunition => unreachable!("handled above"),
            }),
        }
    }
}

/// Message a scenario fails with when the map server drops the session
/// mid-run. Matched by the runner, which retries such a scenario once — see the
/// note there for why this is a mitigation rather than a fix.
pub const CONNECTION_ERROR: &str = "map server connection error";

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
                Ok(mut context) => {
                    context.normalize();
                    return Ok(context);
                }
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    /// Put the shared character into a known state, at the *start* of every
    /// scenario.
    ///
    /// All 114 scenarios share one character and nothing resets it, so each one
    /// inherits whatever the last left behind. Every order-dependence bug found
    /// on 2026-07-30 was a variant of that, and each was fixed by patching the
    /// scenario that leaked. This is the general cure instead.
    ///
    /// **At the start, deliberately, not the end.** End-of-scenario cleanup is
    /// exactly what gets skipped when a scenario fails early or returns on the
    /// `?` operator — which is when it matters most. The ammunition leak
    /// survived for weeks because its cleanup ran only on the happy path.
    ///
    /// Best-effort throughout: this must not turn a working scenario into a
    /// failing one, and the partner account is not guaranteed to hold GM
    /// rights.
    ///
    /// Deliberately **not** normalised here:
    /// - **Job** — every scenario that cares calls `ensure_job`, which is
    ///   idempotent. Forcing one here would add a second job change to all 114.
    /// - **Position** — scenarios warp where they need, and the pair-meeting
    ///   hazard is fixed properly in `far_map_from`.
    /// - **Base level and stats** — only ever raised, and they clamp.
    fn normalize(&mut self) {
        // Full HP/SP. `skill-fail-rejection` drains SP to zero, and the next
        // scenario to need any inherited an unusable character.
        let _ = self.say("@heal");

        // Items that scenarios hand out and that *stack*, so accumulation is
        // invisible: ammunition, traps, grenades. Left alone, ~16 runs of 100
        // rounds each pushed the character past its weight limit, at which
        // point `@item` fails with "Failed to pick up item." and every
        // item-dependent scenario breaks at once, far from the cause.
        for item_id in [13200, 13201, 1065, 7135] {
            let _ = self.say(&format!("@delitem {item_id} 30000"));
        }
        // Arrow ids are contiguous (1750-1770).
        for item_id in 1750..=1770 {
            let _ = self.say(&format!("@delitem {item_id} 30000"));
        }

        // Pump so the commands land, but **do NOT flush**. `flush` clears
        // pending events, and the server delivers real state at login —
        // `QuestList` among it. Flushing here ate that before the scenario
        // could see it, breaking `dm-quest-lifecycle`, which reconnects
        // specifically to assert the quest survives a fresh map login.
        //
        // The cost is that this leaves `@delitem` chat noise in the queue.
        // That is the safer side of the trade: scenarios that need a clean
        // slate already call `flush` themselves, and losing genuine login data
        // is not something a scenario can defend against.
        self.pump(Duration::from_millis(500));
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
            connection_error: None,
            expected_disconnect: false,
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
            remote_ammunition: HashMap::new(),
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
                // Prefer a free slot; slot 0 is usually occupied by HeadlessOne.
                let free_slot = (0..9usize)
                    .find(|candidate| !context.characters.iter().any(|info| info.character_number as usize == *candidate))
                    .ok_or("no free character slot for auto-create")?;
                context
                    .net
                    .create_character(free_slot, new_name.to_owned())
                    .map_err(|_| "disconnected")?;
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

        // Baseline state: a GUI DM session may have left the GM character
        // hidden (`@hide` persists via char.option), which makes it invisible
        // to other clients and untargetable by mobs — silently breaking every
        // proximity-dependent scenario. Clear all option flags best-effort.
        // Only the primary is touched because only a GUI DM session leaves this
        // state behind — both accounts are in fact GM 99, so this is a scoping
        // choice, not a permission one.
        if username == config.username {
            let _ = context.say("@option 0 0 0");
            context.pump(Duration::from_millis(200));
        }

        Ok(context)
    }

    /// Two sessions on the same map, close enough to see each other.
    ///
    /// Every observer-parity assertion needs both seats, because the whole bug
    /// class is invisible from the acting session.
    ///
    /// **The venue is chosen ([`PAIR_VENUE`]), not inherited.** It used to be
    /// "wherever the partner was last left", on the stated grounds that the
    /// partner was non-GM and could not warp itself. That was false — both
    /// accounts are group 99 — and the false premise cost real time: the
    /// meeting place was shared mutable state, and it broke `observer.rs`
    /// twice.
    ///
    /// The failure is worth knowing because it names nothing. The partner's
    /// save point is `int_land`, so anything that sends it home (an
    /// instance closing, a death) moved every later paired scenario to the
    /// beginner island; the rows that merely look at each other did not
    /// care, and the rows that placed a ground unit died silently, because
    /// Hercules drops an unplaceable ground cast with a bare `return 0` and
    /// no `clif->skill_fail`.
    ///
    /// Both seats are warped explicitly, so a scenario that needs particular
    /// ground gets the same ground every run, whatever ran before it.
    pub fn connect_pair(config: &Config) -> Result<(Self, Self), String> {
        let mut primary = Self::connect(config)?;
        let mut partner = Self::connect_partner(config)?;
        let (venue_map, venue_x, venue_y) = PAIR_VENUE;
        // Partner first, then the primary one cell east of it. `warp` already
        // tolerates "you are already at your destination", so the common case of
        // the pair being here from the previous scenario costs nothing.
        partner.warp(venue_map, venue_x, venue_y)?;
        primary.warp(venue_map, venue_x + 1, venue_y)?;
        primary.pump(Duration::from_millis(500));
        partner.pump(Duration::from_millis(500));
        eprintln!(
            "    [connect_pair] partner at {}({}, {}), primary at {}({}, {}), partner sees primary: {}",
            partner.map_name,
            partner.position.x,
            partner.position.y,
            primary.map_name,
            primary.position.x,
            primary.position.y,
            partner.entities.contains_key(&primary.player_id)
        );
        Ok((primary, partner))
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
        if let Some(error) = &self.connection_error {
            return Err(error.clone());
        }

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
            let connection_error = if let NetworkEvent::MapServerDisconnected {
                reason: DisconnectReason::ConnectionError,
            } = &event
            {
                Some(format!(
                    "{CONNECTION_ERROR} (possible desync — check ledger for failed packets)"
                ))
            } else {
                None
            };
            self.pending.push(event);

            if let Some(error) = connection_error {
                if self.expected_disconnect {
                    continue;
                }
                self.connection_error = Some(error.clone());
                record_connection_error(&error);
                return Err(error);
            }
        }
        Ok(())
    }

    /// Mark the next map-server disconnect as part of the scenario contract.
    /// This is intentionally narrow: ordinary disconnects remain fatal.
    pub fn expect_disconnect(&mut self) {
        self.expected_disconnect = true;
    }

    pub fn check_connection(&self) -> Result<(), String> {
        match &self.connection_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
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
            self.check_connection()?;
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

    /// Pump for `duration`, then classify everything still pending **without
    /// consuming any of it**.
    ///
    /// This is the "keep looking" half of an observation window.
    /// `wait_for_within` removes the one event it matched and returns; a
    /// caller that wants to know what *else* the action produced has to
    /// keep reading, and must not eat the events that later waits in the
    /// same scenario are relying on. Pair it with a `flush()` before the
    /// action, so what is pending afterwards is that action's own output
    /// and nothing older.
    pub fn scan_pending<T>(&mut self, duration: Duration, classify: &mut impl FnMut(&NetworkEvent) -> Option<T>) -> Result<Vec<T>, String> {
        self.pump(duration);
        self.check_connection()?;
        Ok(self.pending.iter().filter_map(|event| classify(event)).collect())
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
                // Cleared on map change, never on entity removal — the server
                // re-sends on enter-view, so a stale entry is overwritten, while
                // evicting on removal reopens the hole this map exists to close.
                self.remote_ammunition.clear();
                let _ = self.net.map_loaded();
            }
            // --- appearance ------------------------------------------------
            //
            // Without these the tracked map holds *spawn data only*, and a
            // parity assertion cannot see a look change at all — the gap that
            // made this harness impossible before the wire→event boundary was
            // fixed.
            NetworkEvent::ChangeHair { account_id, hair_id } => {
                if let Some(entity) = self.entities.get_mut(&EntityId(account_id.0)) {
                    entity.head = *hair_id as u16;
                }
            }
            NetworkEvent::ChangeWeapon { account_id, weapon_id } => {
                // Mirrors the client: a weapon change invalidates cached
                // ammunition, because the server force-unequips it.
                self.remote_ammunition.remove(account_id);
                if let Some(entity) = self.entities.get_mut(&EntityId(account_id.0)) {
                    entity.weapon = *weapon_id;
                }
            }
            NetworkEvent::ChangeShield { account_id, shield_id } => {
                if let Some(entity) = self.entities.get_mut(&EntityId(account_id.0)) {
                    entity.shield = *shield_id;
                }
            }
            NetworkEvent::ChangeJob { account_id, job_id } if account_id.0 != self.account_id.0 => {
                if let Some(entity) = self.entities.get_mut(&EntityId(account_id.0)) {
                    entity.job_id = *job_id;
                }
            }
            NetworkEvent::ChangeAmmunition { account_id, item_id } => {
                self.remote_ammunition.insert(*account_id, *item_id);
            }
            NetworkEvent::ChangeLook {
                account_id,
                look_type,
                value,
            } => {
                if let Some(entity) = self.entities.get_mut(&EntityId(account_id.0)) {
                    let short = *value as u16;
                    match look_type {
                        SpriteChangeType::HeadBottom => entity.accessory = short,
                        SpriteChangeType::HeadTop => entity.accessory2 = short,
                        SpriteChangeType::HeadMiddle => entity.accessory3 = short,
                        SpriteChangeType::HairCollor => entity.head_palette = short,
                        SpriteChangeType::ClothesColor => entity.body_palette = short,
                        SpriteChangeType::Robe => entity.robe = short,
                        SpriteChangeType::Body2 => entity.body = short,
                        // No view slot; the original client ignores them too.
                        SpriteChangeType::Shoes | SpriteChangeType::Body => {}
                        // Delivered as their own events, never as ChangeLook.
                        SpriteChangeType::Base
                        | SpriteChangeType::Hair
                        | SpriteChangeType::Weapon
                        | SpriteChangeType::Shield
                        | SpriteChangeType::Ammunition => {}
                    }
                }
            }
            NetworkEvent::EntityDirection {
                entity_id, head_direction, ..
            } => {
                if let Some(entity) = self.entities.get_mut(entity_id) {
                    entity.head_direction = *head_direction as usize;
                }
            }
            NetworkEvent::EntityStopMove { entity_id, position } => {
                if let Some(entity) = self.entities.get_mut(entity_id) {
                    entity.position = WorldPosition::new(position.x, position.y, entity.position.direction);
                    entity.destination = None;
                }
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
            // A skill granted mid-session (`ZC_ADD_SKILL`) — quest rewards, the
            // `skill` script command, `@questskill`, Plagiarism. The full tree
            // only arrives on login and job change, so without this a scenario
            // that grants a skill cannot see it and reads as "the command did
            // nothing", which is how the missing 0x0111 handler was found.
            NetworkEvent::SkillAdded { skill_information } => {
                match self.skills.iter_mut().find(|skill| skill.skill_id == skill_information.skill_id) {
                    Some(existing) => *existing = skill_information.clone(),
                    None => self.skills.push(skill_information.clone()),
                }
            }
            NetworkEvent::UpdateSkill { skill_id, skill_level, .. } => {
                if let Some(skill) = self.skills.iter_mut().find(|skill| skill.skill_id == *skill_id) {
                    skill.skill_level = *skill_level;
                }
            }
            NetworkEvent::RemoveSkill { skill_id } => {
                self.skills.retain(|skill| skill.skill_id != *skill_id);
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

    /// Walk to a tile and wait out the travel time. Long walks are split into
    /// short hops: the server silently ignores move requests beyond
    /// `max_walk_path` (17 cells stock), and with the enlarged `area_size`
    /// view radius entities are visible well past that.
    pub fn walk_to(&mut self, x: u16, y: u16) -> Result<(), String> {
        use ragnarok_packets::{Direction, WorldPosition};

        const MAX_HOP: i32 = 10;

        for _hop in 0..12 {
            let current = self.position;
            let dx = x as i32 - current.x as i32;
            let dy = y as i32 - current.y as i32;
            if dx == 0 && dy == 0 {
                return Ok(());
            }
            let step_x = (current.x as i32 + dx.clamp(-MAX_HOP, MAX_HOP)) as u16;
            let step_y = (current.y as i32 + dy.clamp(-MAX_HOP, MAX_HOP)) as u16;

            self.flush();
            self.net
                .player_move(WorldPosition::new(step_x, step_y, Direction::North))
                .map_err(|_| "disconnected")?;
            let (origin, destination) = self.wait_for("PlayerMove ack", |event| match event {
                NetworkEvent::PlayerMove { origin, destination, .. } => Some((origin.tile_position(), destination.tile_position())),
                _ => None,
            })?;

            let distance = (origin.x.abs_diff(destination.x)).max(origin.y.abs_diff(destination.y)) as u64;
            // Base walk speed is ~150ms/cell at speed 150; pad generously.
            self.pump(Duration::from_millis((distance * 200 + 400).min(4000)));
            self.position = destination;

            // Close enough: the server may route around small obstacles.
            if self.position.x.abs_diff(x) <= 1 && self.position.y.abs_diff(y) <= 1 {
                return Ok(());
            }
            if self.position.x == current.x && self.position.y == current.y {
                return Err(format!(
                    "walk made no progress toward ({x}, {y}) from ({}, {})",
                    current.x, current.y
                ));
            }
        }
        Err(format!("did not reach ({x}, {y}) within the hop budget"))
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

    // --- observer parity ---------------------------------------------------

    /// What this session currently believes `subject`'s `attribute` to be.
    /// `None` means the entity is not in view at all.
    pub fn observed(&self, subject: AccountId, attribute: Appearance) -> Option<u32> {
        attribute.read(
            self.entities.get(&EntityId(subject.0)),
            self.remote_ammunition.get(&subject).copied(),
        )
    }

    /// Poll until this session's view of `subject` converges on `expected`.
    ///
    /// Convergence, not equality-right-now, is the property worth asserting:
    /// broadcasts and enter-view re-sends race the spawn packet, and a client
    /// is allowed to be briefly wrong. It is *staying* wrong that is the bug.
    ///
    /// Always call this on the OBSERVER, never on the actor. Every one of the
    /// six bugs found on 2026-07-29 looked correct from the acting session.
    pub fn assert_converges(&mut self, subject: AccountId, attribute: Appearance, expected: u32) -> Result<(), String> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let actual = self.observed(subject, attribute);
            if actual == Some(expected) {
                return Ok(());
            }

            if Instant::now() > deadline {
                let in_view = self.entities.contains_key(&EntityId(subject.0));
                return Err(format!(
                    "{:?} of account {} did not converge on {expected}: observer sees {}{}",
                    attribute,
                    subject.0,
                    match actual {
                        Some(value) => value.to_string(),
                        None => "nothing".to_owned(),
                    },
                    match in_view {
                        true => "",
                        false => " (entity not in view — the spawn never arrived, which is a different bug)",
                    },
                ));
            }

            sleep(POLL_INTERVAL);
            self.poll()?;
        }
    }

    /// Wait until `subject` is (or is no longer) in this session's view, so a
    /// timing scenario can be sure it actually left before changing something.
    pub fn assert_in_view(&mut self, subject: AccountId, expected: bool) -> Result<(), String> {
        let deadline = Instant::now() + self.timeout;
        loop {
            if self.entities.contains_key(&EntityId(subject.0)) == expected {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err(format!(
                    "account {} was still {} view after {:?}",
                    subject.0,
                    match expected {
                        true => "out of",
                        false => "in",
                    },
                    self.timeout
                ));
            }
            sleep(POLL_INTERVAL);
            self.poll()?;
        }
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
