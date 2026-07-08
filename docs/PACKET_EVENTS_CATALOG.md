# Packet Events Catalogue

**Purpose**: This is the authoritative technical map of how raw Ragnarok Online wire packets become high-level `NetworkEvent`s in Korangar, and how those events drive the rest of the client (world entities, particles, state, UI, audio cues). It is the companion to:

- `docs/protocol/packet-length-fallbacks.md`
- `docs/protocol/hercules-20220406.md`
- `docs/WORLD_MAPS_ENTITIES.md` (entity + quest effect flows)
- `docs/DM_SERVER_FUNCTIONS.md` (chat + quest effect usage for DM campaign)
- `docs/GRAPHICS_PIPELINE.md`, `docs/CLIENT_SYSTEMS_OVERVIEW.md`

Korangar uses **framing-by-deserialization**. The `PacketHandler` reads a 2-byte header, dispatches to a registered closure, and that closure consumes exactly the bytes for that packet. Unknown headers previously caused full buffer loss; `register_length_fallbacks` (installed last on the map connection) now guarantees framing stability while still surfacing unknowns via the `PacketCallback`.

The **single surface** between networking and the rest of the engine is the `NetworkEvent` enum. Almost every gameplay reaction (AddEntity, chat, quest markers, damage numbers, dialog buttons, inventory deltas, skill tree, hotbar, shop, status effects, etc.) is expressed as one of these variants.

Packet version in use: `SupportedPacketVersion::_20220406` (maps to server `PACKETVER 20190605` main). The length table is auto-generated from Hercules.

## Packet Handler Machinery

Defined in `ragnarok-packets/src/handler.rs` and used from `korangar-networking`.

Key types:

```rust
pub enum HandlerResult<Output> {
    Ok(Output),
    UnhandledPacket,   // no handler for header -> callback + cut buffer (pre-fallbacks)
    PacketCutOff,
    InternalError(Box<ConversionError>),
}

pub trait PacketCallback: Clone + 'static {
    fn incoming_packet<Packet>(&self, packet: &Packet) where Packet: ragnarok_packets::Packet { ... }
    fn outgoing_packet<Packet>(&self, packet: &Packet) where Packet: ragnarok_packets::Packet { ... }
    fn unknown_packet(&self, bytes: Vec<u8>) { ... }
    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ConversionError>) { ... }
}
```

`PacketHandler<Output, Callback>` holds a `HashMap<PacketHeader, HandlerFunction<Output>>`.

Registration APIs (used in `version_20220406.rs`):

- `register::<P, R>(|p: P| -> R)` — normal semantic handler. The closure receives the fully deserialized packet. `packet_callback.incoming_packet` is called inside.
- `register_noop::<P>()` — consume + callback only, produce `Output::default()` (i.e. no `NetworkEvent`).
- `register_length_fallbacks(&[(u16, i32)])` — for every header not yet present, install a length-aware consumer that calls `unknown_packet` with the payload (header already consumed by `process_one`). Must be called **last**.

`process_one` is the core loop entry:

```rust
pub fn process_one(&mut self, byte_reader: &mut ByteReader) -> HandlerResult<Output>
```

It peeks the header, looks up, runs the handler (which does `payload_from_bytes`), and handles cutoff vs. real error.

See `docs/protocol/packet-length-fallbacks.md` for generation (`tools/generate_packet_lengths.sh`) and why this design exists.

## Registration Entry Points

In `korangar-networking/src/packet_versions/version_20220406.rs`:

- `register_login_server_packets`
- `register_character_server_packets`
- `register_map_server_packets`

Called from `korangar-networking/src/lib.rs`:

```rust
fn create_map_server_packet_handler(...) {
    ...
    version_20220406::register_map_server_packets(&mut packet_handler)?;
    ...
}
```

`register_map_server_packets` ends with:

```rust
// Consume any remaining server packet whose length is known (from Hercules'
// own tables) but that has no dedicated handler yet...
packet_handler.register_length_fallbacks(super::lengths_20220406::PACKET_LENGTHS);
```

Login and character flows are small and fully modeled (no fallbacks registered there today).

## The NetworkEvent Enum (the catalogue surface)

Full definition from `korangar-networking/src/event.rs`:

```rust
pub enum NetworkEvent {
    // Connection lifecycle
    LoginServerConnected { character_servers: Vec<CharacterServerInformation>, login_data: LoginServerLoginData },
    LoginServerConnectionFailed { reason: UnifiedLoginFailedReason, message: &'static str },
    LoginServerDisconnected { reason: DisconnectReason },
    CharacterServerConnected { normal_slot_count: usize },
    CharacterServerConnectionFailed { reason: LoginFailedReason, message: &'static str },
    CharacterServerDisconnected { reason: DisconnectReason },
    MapServerDisconnected { reason: DisconnectReason },

    // Early account id (special case before any packet framing on map login)
    AccountId { account_id: AccountId },

    // Character selection / management
    CharacterList { characters: Vec<CharacterInformation> },
    CharacterSelected { login_data: CharacterServerLoginData },
    CharacterSelectionFailed { reason: UnifiedCharacterSelectionFailedReason, message: &'static str },
    CharacterCreated { character_information: CharacterInformation },
    CharacterCreationFailed { reason: CharacterCreationFailedReason, message: &'static str },
    CharacterDeleted,
    CharacterDeletionFailed { reason: CharacterDeletionFailedReason, message: &'static str },
    CharacterSlotSwitched,
    CharacterSlotSwitchFailed,

    // Initial player numbers
    InitialStats { strength_stat_points_cost: u8, ... luck_stat_points_cost: u8 },

    // World / map
    ResurrectPlayer { entity_id: EntityId },
    PlayerStandUp { entity_id: EntityId },
    ChangeMap { map_name: String, position: TilePosition },
    UpdateClientTick { client_tick: ClientTick, received_at: Instant },

    // Entities
    AddEntity { entity_data: EntityData },
    RemoveEntity { entity_id: EntityId, reason: DisappearanceReason },
    EntityMove { entity_id, origin, destination, starting_timestamp },
    PlayerMove { origin, destination, starting_timestamp },
    UpdateEntityDetails { entity_id, name: String },
    UpdateEntityHealth { entity_id, health_points: usize, maximum_health_points: usize },
    ChangeJob { account_id: AccountId, job_id: JobId },
    ChangeHair { account_id: AccountId, hair_id: u32 },

    // Ground items
    AddGroundItem { entity_id, item_id, is_identified, quantity, position, x_offset, y_offset },
    RemoveGroundItem { entity_id },

    // Chat / messages (all colors collapsed here)
    ChatMessage { text: String, color: MessageColor },

    // Combat / effects
    DamageEffect { source_entity_id, destination_entity_id, damage_amount: Option<usize>, attack_duration: u32, is_critical: bool },
    EntityPickUpItem { entity_id, item_entity_id },
    HealEffect { entity_id, heal_amount: usize },
    StatusChange { entity_id, index: u16, gained: bool, duration_ms: u32, remaining_ms: u32 },
    VisualEffect { effect_path: &'static str, entity_id: EntityId },
    AddSkillUnit { entity_id, unit_id: UnitId, position: TilePosition },
    RemoveSkillUnit { entity_id },

    // Quest markers (the "!" and colored indicators)
    AddQuestEffect { quest_effect: QuestEffectPacket },
    RemoveQuestEffect { entity_id: EntityId },

    // NPC dialogs
    OpenDialog { text: String, npc_id: EntityId },
    AddNextButton { npc_id: EntityId },
    AddCloseButton { npc_id: EntityId },
    AddChoiceButtons { choices: Vec<String>, npc_id: EntityId },

    // Inventory + acquisition
    SetInventory { items: Vec<InventoryItem<NoMetadata>> },
    IventoryItemAdded { item: InventoryItem<NoMetadata> },
    ItemObtained { item_id: ItemId, quantity: u16, is_identified: bool },
    InventoryItemRemoved { reason: RemoveItemReason, index: InventoryIndex, amount: u16 },

    // Equipment
    UpdateEquippedPosition { index: InventoryIndex, equipped_position: EquipPosition },

    // Stats / skills / hotbar
    UpdateStat { stat_type: StatType },
    SkillTree { skill_information: Vec<SkillInformation> },
    UpdateSkill { skill_id, skill_level, spell_point_cost: u16, attack_range: AttackRange, upgradable: bool },
    RemoveSkill { skill_id: SkillId },
    SetHotkeyData { tab: HotbarTab, hotkeys: Vec<HotkeyState> },

    // Shops
    OpenShop { items: Vec<ShopItem<NoMetadata>> },
    AskBuyOrSell { shop_id: ShopId },
    BuyingCompleted { result: BuyShopItemsResult },
    SellItemList { items: Vec<SellItemInformation> },
    SellingCompleted { result: SellItemsResult },

    // Social
    FriendRequest { requestee: Friend },
    FriendAdded { friend: Friend },
    FriendRemoved { account_id: AccountId, character_id: CharacterId },
    SetFriendList { friend_list: Vec<Friend> },

    // Session
    LoggedOut,

    // Failure feedback
    AttackFailed { target_entity_id, target_position, player_position, attack_range },
}
```

`NetworkEventList` / `NoNetworkEvents` are tiny newtype wrappers so handlers can return `impl Into<NetworkEventList>` (single event, vec, option, or nothing).

Disconnect reasons are turned into the corresponding `*Disconnected` events by the connection manager.

## Event Sources — Map Server (the rich set)

From `register_map_server_packets` (verbatim excerpts):

### Core movement / presence

```rust
packet_handler.register(|packet: ChangeMapPacket| { ... NetworkEvent::ChangeMap ... })?;
packet_handler.register(|packet: EntityMovePacket| NetworkEvent::EntityMove { ... })?;
packet_handler.register(|packet: PlayerMovePacket| NetworkEvent::PlayerMove { ... })?;
packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;
packet_handler.register(|packet: MapServerLoginSuccessPacket| NetworkEvent::UpdateClientTick { ... })?;
packet_handler.register(|packet: ServerTickPacket| NetworkEvent::UpdateClientTick { ... })?;
```

`ChangeMapPacket` (0x0091):

```rust
#[header(0x0091)]
pub struct ChangeMapPacket {
    #[length(16)] pub map_name: String,
    pub position: TilePosition,
}
```

### Entity appearance (multiple packets → single event + conversion)

```rust
packet_handler.register(|packet: EntityAppearPacket| NetworkEvent::AddEntity { entity_data: packet.into() })?;
packet_handler.register(|packet: EntityAppear2Packet| NetworkEvent::AddEntity { entity_data: packet.into() })?;
packet_handler.register(|packet: MovingEntityAppearPacket| NetworkEvent::AddEntity { entity_data: packet.into() })?;
packet_handler.register(|packet: EntityDisAppearPacket| NetworkEvent::RemoveEntity { ... })?;
packet_handler.register(|packet: ResurrectionPacket| NetworkEvent::ResurrectPlayer { ... })?;
```

`EntityData` (in `korangar-networking/src/entity.rs`) has `From` impls for the three appear packets. It normalizes position vs. (origin, destination) for moving spawns.

Example appear packet (0x09FE / 0x09FF family, variable length):

```rust
pub struct EntityAppearPacket { /* ... object_type, entity_id, job_id, sex, position, c_level, name, hp/maxhp, ... */ }
pub struct EntityAppear2Packet { /* similar modern layout */ }
pub struct MovingEntityAppearPacket { /* includes move_start_time + from/to encoded in position */ }
```

`RemoveEntity` carries `DisappearanceReason` (OutOfSight, Died, LoggedOut, Teleported, TrickDead). Died triggers special dead-entity list handling for monsters/players.

### Ground items

Multiple `GroundItemAppear*` packets (1-4) all map to `AddGroundItem`. `ItemDisappearPacket` → `RemoveGroundItem`.

### Chat / message family (all collapse to `ChatMessage` + color)

```rust
packet_handler.register(|packet: BroadcastMessagePacket| NetworkEvent::ChatMessage { text: ..., color: MessageColor::Broadcast })?;
packet_handler.register(|packet: Broadcast2MessagePacket| { /* RGB color from packet */ ChatMessage { ... } })?;
packet_handler.register(|packet: ServerMessagePacket| ChatMessage { ..., color: MessageColor::Server })?;
packet_handler.register(|packet: EntityMessagePacket| { color from BGRA; ChatMessage })?;
packet_handler.register(|packet: OverheadMessagePacket| ChatMessage { ..., Broadcast })?;  // NOTE: treated as broadcast today
```

Sending chat uses `GlobalMessagePacket` (client → server):

```rust
#[header(0x00F3)]
#[variable_length]
pub struct GlobalMessagePacket {
    #[length_remaining_off_by_one] pub message: String,  // "name : text"
}
```

`MessageColor` variants: `Broadcast`, `Server`, `Error`, `Information`, `Rgb {..}`.

DM usage: All `@dm*` commands and responses flow as ordinary chat text today. Structured `[DMJ]{...}` echoes (planned) will arrive as `ChatMessage` and can be parsed by UI/state without new packets.

### Quest effects (the visual "!" / markers)

```rust
packet_handler.register(|packet: QuestEffectPacket| match packet.effect {
    QuestEffect::None => NetworkEvent::RemoveQuestEffect { entity_id: packet.entity_id },
    _ => NetworkEvent::AddQuestEffect { quest_effect: packet },
})?;
```

Definition:

```rust
#[header(0x0446)]
pub struct QuestEffectPacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
    pub effect: QuestEffect,
    pub color: QuestColor,
}

#[numeric_type(u16)]
pub enum QuestEffect {
    Quest, Quest2, Job, Job2, Event, Event2, ClickMe, DailyQuest, Event3, JobQuest, JumpingPoring,
    #[numeric_value(9999)] None,
}

#[numeric_type(u16)]
pub enum QuestColor { Yellow, Orange, Green, Purple }
```

Downstream (`korangar/src/lib.rs`):

```rust
NetworkEvent::AddQuestEffect { quest_effect } => {
    if let Some(map) = &self.map {
        self.particle_holder.add_quest_icon(&self.texture_loader, map, quest_effect)
    }
}
NetworkEvent::RemoveQuestEffect { entity_id } => self.particle_holder.remove_quest_icon(entity_id),
```

See `src/world/particles/mod.rs` (QuestIcon uses `quest_{effect}.bmp` textures, position from packet or entity, rendered decoupled from entity list) and `WORLD_MAPS_ENTITIES.md`.

DM usage: Server scripts (`@dmhazard`, scene setup, etc.) can emit `QuestEffectPacket` (or equivalent) to place persistent or temporary markers for hazards, clues, beats, etc. Client does not require the base entity to still be present.

### NPC dialogs

```rust
packet_handler.register(|p: NpcDialogPacket| NetworkEvent::OpenDialog { text: p.text, npc_id: p.npc_id })?;
packet_handler.register(|p: NextButtonPacket| NetworkEvent::AddNextButton { npc_id })?;
packet_handler.register(|p: CloseButtonPacket| NetworkEvent::AddCloseButton { npc_id })?;
packet_handler.register(|p: DialogMenuPacket| { split on ':' → AddChoiceButtons })?;
```

`NpcDialogPacket` (0x00B4) is variable length with `#[length_remaining]`.

These drive `DialogWindow` + `client_state().dialog_window()`.

### Inventory (accumulation pattern)

A trio of packets + transient `Rc<RefCell<Option<Vec<...>>>>`:

- `InventoyStartPacket` → reset vec
- `RegularItemListPacket` / `EquippableItemListPacket` → extend
- `InventoyEndPacket` → emit `SetInventory { items }`

Deltas use `ItemPickupPacket` (complex result + details) → `IventoryItemAdded` + `ItemObtained`, and `RemoveItemFromInventoryPacket` → `InventoryItemRemoved`.

### Skills, hotkeys, stats

- `UpdateSkillTreePacket` → `SkillTree`
- `UpdateHotkeysPacket` → `SetHotkeyData`
- `InitialStatsPacket` → `InitialStats`
- `UpdateSkillPacket` / `RemoveSkillPacket` → per-skill updates
- `UpdateStat*` family (4 variants) → `UpdateStat`

Hotkey binding uses `SetHotkeyData2Packet` (client).

### Combat / effects

`DamagePacket1` (0x008A) and `DamagePacket3` (0x08C8) are pattern-matched on `DamageType`:

- Damage / CriticalHit → `DamageEffect` (particles + attack anim)
- PickUpItem → `EntityPickUpItem`
- StandUp → `PlayerStandUp`

Other:

- `DisplaySkillEffectNoDamagePacket` → `HealEffect`
- `StatusChangePacket` → `StatusChange`
- `VisualEffectPacket` → `VisualEffect` (maps enum to `.str` effect paths)
- `NotifySkillUnitPacket` / `SkillUnitDisappearPacket` → skill ground units (effect_holder)

`RequestPlayerAttackFailedPacket` → `AttackFailed`.

### Shops

- `BuyOrSellPacket` → `AskBuyOrSell`
- `ShopItemListPacket` → `OpenShop`
- `SellListPacket` → `SellItemList`
- Result packets → `BuyingCompleted` / `SellingCompleted`

### Friends

Full set: list, request, result (with side effects), removed.

### Other modeled

`FriendListPacket`, `NotifyFriendRemovedPacket`, `UpdateEntityHealthPointsPacket`, `RequestPlayerDetailsSuccessPacket` / `RequestEntityDetailsSuccessPacket` (name lookup after hover), `RestartResponsePacket`, `DisconnectResponsePacket`, etc.

Many packets are `register_noop` today (e.g. most quest list/notification variants, `DisplaySpecialEffectPacket`, `Achievement*`, `NotifyActorInitPacket`, `ParameterChangePacket`, party packets, etc.). They still get framed correctly thanks to fallbacks and appear in the packet history UI.

## Consumption — `handle_network_events` (korangar/src/lib.rs)

The main loop drains `network_event_buffer` and has one giant match. Representative reactions:

- `AddEntity` → construct `Npc` (or later Player/Monster), request animation data, push to `client_state().entities()`, inherit fade if re-spawn.
- `RemoveEntity` (Died) → move monster to dead list or mark player dead + open respawn window.
- `ChatMessage` → push to `client_state().chat_messages()`.
- `OpenDialog` / buttons → mutate `client_state().dialog_window()` and open `DialogWindow`.
- `AddQuestEffect` → `particle_holder.add_quest_icon(...)`.
- `SetInventory` / item deltas → `client_state().inventory()`.
- `SkillTree` → populate `client_state().skill_tree()`.
- `DamageEffect` → rotate attacker, start attack anim, spawn `DamageNumber`/`Miss` particle.
- `ChangeMap` → close dialogs, `async_loader.request_map_load(...)`.
- Map disconnect → clear entities/ground/particles/effects/lights, force character server reconnect + default map load.
- Many others mutate hotbar, shop windows, friend list, equipped positions, etc.

See full match for edge cases (auto-attack buffering, job change side effects, etc.).

## Outgoing Packets (client → server)

All go through `NetworkingSystem` methods that format the packet and hand it to the per-connection `action_sender`. Examples:

- `player_move`, `player_attack`, `pick_up_item`
- `send_chat_message` (builds `GlobalMessagePacket`)
- `start_dialog` / `next_dialog` / `close_dialog` / `choose_dialog_option`
- `cast_skill`, `cast_ground_skill`, `cast_channeling_skill`
- `request_item_equip` / `unequip`
- `purchase_items`, `sell_items`, `select_buy_or_sell`
- `add_friend`, `remove_friend`, `accept/reject_friend_request`
- `set_hotkey_data`, `request_stat_up`, `level_up_skill`
- `log_out`, `respawn`, `map_loaded`
- `warp_to_map`, `entity_details`

The map keepalive is `RequestServerTickPacket`; character keepalive is simpler.

All sent packets also go through the `PacketCallback::outgoing_packet` for history.

## DM Campaign Integration Points

- **Command surface**: Client code (or future DM windows) builds strings like `"Player : @dmhazard foo"` and sends via `GlobalMessagePacket`. Server NPC scripts (bound via `bindatcmd`) execute and may reply with normal chat or planned structured `[DMJ]` messages.
- **Feedback surface**: All replies (success, rolls, hazard announcements, story beats) arrive as `ChatMessage`. The chat window + future DM chrome parse or display them. See `DM_SERVER_FUNCTIONS.md` and `DM_INTERFACE.md`.
- **In-world visuals**: `@dmhazard` / scene / beat markers can be realized by the server emitting `QuestEffectPacket` (or AddEntity + effect). Client already renders these via the particle system without requiring a live entity (see WORLD_MAPS_ENTITIES.md fix for `quest_icons.values()`).
- **Party context**: Campaign is party-locked on the server (`$dm_active_party`, `DM_RequireDM`). See dedicated targeted spec `docs/specs/party-packets.md`. Party roster, member add/remove, HP, position, job/level, party chat, invite feedback, and whisper now promote to `NetworkEvent`s and hidden `party_state`; visible party frames still need live validation and UI work.
- **No custom packets required for Phase A**: Everything above works with stock 20190605 traffic + chat.

Future: when `[DMJ]` JSON echoes or richer quest/effect data are added server-side, promote the relevant packets from noop → real handler + new or extended `NetworkEvent` variants.

## Adding / Promoting a Packet

1. Ensure the struct exists in `ragnarok-packets/src/lib.rs` (use `#[derive(Packet, ServerPacket, MapServer)]`, `#[header(0x....)]`, field attributes for variable/repeating lengths, `ByteConvertable` where needed).
2. In `version_20220406.rs` (appropriate `register_*` function):
   - `register(|p| NetworkEvent::Foo { ... })` when you need the contents, or
   - `register_noop::<ThePacket>()?` if you only need framing + debug visibility for now.
3. Add a variant to `NetworkEvent` (and the `From` glue if using the list wrappers) when promoting.
4. Handle the variant in `lib.rs` (and/or world/particles/state as appropriate).
5. If the length wasn't known, it will have been a desync risk; after adding a real handler the fallback is ignored automatically.
6. Add a round-trip or layout test if it's a new wire layout.
7. For length table changes: rerun the generator against the exact Hercules `PACKETVER` tree.

Never place a dedicated handler before the length fallbacks registration.

## Debugging & Observability

- `KORANGAR_PACKET_LOG=1` — prints hex of keepalives + on send, plus some add-entity details.
- Packet history window (debug builds) — shows every incoming/outgoing/unknown/error packet as a collapsible with fields (powered by `Packet::to_element` + `PacketCallback`).
- `unknown_packet` path in the callback and fallback handler always receives payload bytes (header stripped in fallbacks).
- `failed_packet` on conversion errors inside a handler.

The history UI lives in `korangar/src/networking/mod.rs` (`PacketHistory`, `PacketHistoryCallback`, `PacketEntry`, `UnknownPacket`/`ErrorPacket` shims).

## Packet Struct Notes & Gotchas

- Many strings use `#[length(N)]` or `#[length_remaining]` / `#[length_remaining_off_by_one]`.
- Position types: `WorldPosition` (with direction), `TilePosition`, `LargeTilePosition`.
- Several modern packets (post-2018) use different layouts (`DamagePacket3`, newer quest notifications, `0x0AE*` UI/actor packets).
- `QuestEffectPacket` (0x0446) is small and fixed — very reliable for DM markers.
- Variable-length chat packets are the most common source of "off by one" issues (hence the special attribute on `GlobalMessagePacket`).

See `ragnarok-packets/src/lib.rs` for the full set of derived packets and `ragnarok-bytes` for the low-level reader/writer.

## Cross References

- **Entities & quest markers**: `WORLD_MAPS_ENTITIES.md`, `src/world/entity/*`, `src/world/particles/mod.rs`, `AddEntity`/`AddQuestEffect` handling in `lib.rs`.
- **DM server driving the client**: `DM_SERVER_FUNCTIONS.md` (how `@dm*` and vars turn into chat + quest effects).
- **Framing safety**: `docs/protocol/packet-length-fallbacks.md`.
- **Hercules source map**: `docs/protocol/hercules-20220406.md`.
- **Overall data flow**: `CLIENT_SYSTEMS_OVERVIEW.md`.
- **Graphics that react to some events** (visual effects, lights on map load): `GRAPHICS_PIPELINE.md`.

## Appendix: Lengths Table

`korangar-networking/src/packet_versions/lengths_20220406.rs` is `@generated`. It currently contains ~1470 entries. Regenerate only against the exact server tree + `PACKETVER` you are running:

```
tools/generate_packet_lengths.sh ~/GitHub/Hercules_RO 20190605 main
```

The table is direction-agnostic; client→server entries are harmless on the receive side.

---

This catalogue is intended to be living documentation. When you promote a noop, add a DM-specific event, or discover a new packet shape from live traffic, update the relevant sections here with verbatim snippets and the downstream effect.
