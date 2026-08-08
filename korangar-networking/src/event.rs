use std::time::Instant;

use ragnarok_packets::*;

use crate::hotkey::HotkeyState;
use crate::items::ShopItem;
use crate::{
    CharacterServerLoginData, EntityData, InventoryItem, LoginServerLoginData, MessageColor, NoMetadata,
    UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};

/// An event triggered by one of the Ragnarok Online servers.
#[derive(Debug)]
pub enum NetworkEvent {
    LoginServerConnected {
        character_servers: Vec<CharacterServerInformation>,
        login_data: LoginServerLoginData,
    },
    LoginServerConnectionFailed {
        reason: UnifiedLoginFailedReason,
        message: &'static str,
    },
    LoginServerDisconnected {
        reason: DisconnectReason,
    },
    CharacterServerConnected {
        normal_slot_count: usize,
    },
    CharacterServerConnectionFailed {
        reason: LoginFailedReason,
        message: &'static str,
    },
    CharacterServerDisconnected {
        reason: DisconnectReason,
    },
    AccountId {
        account_id: AccountId,
    },
    CharacterList {
        characters: Vec<CharacterInformation>,
    },
    CharacterSelected {
        login_data: CharacterServerLoginData,
    },
    CharacterSelectionFailed {
        reason: UnifiedCharacterSelectionFailedReason,
        message: &'static str,
    },
    CharacterCreated {
        character_information: CharacterInformation,
    },
    CharacterCreationFailed {
        reason: CharacterCreationFailedReason,
        message: &'static str,
    },
    CharacterDeleted,
    CharacterDeletionFailed {
        reason: CharacterDeletionFailedReason,
        message: &'static str,
    },
    MapServerDisconnected {
        reason: DisconnectReason,
    },
    /// Initial player status.
    InitialStats {
        strength_stat_points_cost: u8,
        agility_stat_points_cost: u8,
        vitality_stat_points_cost: u8,
        intelligence_stat_points_cost: u8,
        dexterity_stat_points_cost: u8,
        luck_stat_points_cost: u8,
    },
    /// Resurrect a player.
    ResurrectPlayer {
        entity_id: EntityId,
    },
    /// Make a player stand up.
    PlayerSitDown {
        entity_id: EntityId,
    },
    PlayerStandUp {
        entity_id: EntityId,
    },
    /// Add an entity to the list of entities that the client is aware of.
    AddEntity {
        entity_data: EntityData,
    },
    /// Remove an entity from the list of entities that the client is aware of
    /// by its id.
    RemoveEntity {
        entity_id: EntityId,
        reason: DisappearanceReason,
    },
    /// Add an item to the ground.
    AddGroundItem {
        entity_id: EntityId,
        item_id: ItemId,
        is_identified: bool,
        quantity: u16,
        position: TilePosition,
        x_offset: u8,
        y_offset: u8,
    },
    /// Remove an item from the ground.
    RemoveGroundItem {
        entity_id: EntityId,
    },
    /// The player is pathing to a new position.
    PlayerMove {
        origin: WorldPosition,
        destination: WorldPosition,
        starting_timestamp: ClientTick,
    },
    /// An Entity nearby is pathing to a new position.
    EntityMove {
        entity_id: EntityId,
        origin: WorldPosition,
        destination: WorldPosition,
        starting_timestamp: ClientTick,
    },
    /// An entity moved instantly on the current map (`ZC_HIGHJUMP`).
    EntitySlide {
        entity_id: EntityId,
        position: TilePosition,
    },
    /// Monster stats returned by Wizard Sense/Estimation.
    MonsterInformation {
        job_id: JobId,
        level: u16,
        size: u16,
        health_points: u32,
        defense: u16,
        race: u16,
        magic_defense: u16,
        element: u16,
        elemental_effectiveness: [u8; 9],
    },
    /// Available destinations for Teleport/Warp Portal.
    WarpList {
        skill_id: SkillId,
        destinations: Vec<String>,
    },
    SkillCooldownList {
        cooldowns: Vec<SkillCooldownInformation>,
    },
    RefinableWeaponList {
        weapons: Vec<RefinableWeaponInformation>,
    },
    WeaponRefineResult {
        result: i32,
        item_id: ItemId,
    },
    RepairableItemList {
        items: Vec<RepairableItemInformation>,
    },
    ItemRepairResult {
        inventory_index: InventoryIndex,
        success: bool,
    },
    /// Player was moved to a new position on a different map or the current map
    ChangeMap {
        map_name: String,
        position: TilePosition,
    },
    /// Update the client side to keep server and client synchronized.
    UpdateClientTick {
        client_tick: ClientTick,
        received_at: Instant,
    },
    /// New chat message for the client.
    ChatMessage {
        text: String,
        color: MessageColor,
    },
    /// Official msgstringtable lookup (`ZC_MSG` / `ZC_MSG_COLOR`). The client
    /// resolves `message_id` via `data\msgstringtable.txt`.
    MessageTable {
        message_id: u16,
        color: MessageColor,
    },
    /// A skill was rejected because a required item is missing
    /// (`ZC_ACK_TOUSESKILL` causes 71 / 72). Hercules only sends the item *id*,
    /// and the item name table lives in the client crate, so the message is
    /// finished there instead of reporting a raw id.
    SkillFailedMissingItem {
        item_id: ItemId,
        /// How many the skill needs. Hercules sends this in `btype`; `0` and
        /// `1` both mean a single item.
        amount: u16,
        /// `true` for cause 72 (a required *equipment* piece) rather than a
        /// consumable.
        equipment: bool,
    },
    /// A message-table line carrying a number (`ZC_MSG_VALUE`). The table lives
    /// in the client crate, and the id's text holds the `%d` this fills.
    MessageTableNumber { message_id: u16, value: u32 },
    /// Result of ignoring or unignoring everyone (`ZC_ACK_WHISPER_LIST`).
    IgnoreAllResult { ignore_type: u8, result: u8 },
    /// A storage deposit was refused (`ZC_MOVE_ITEM_FAILED`). Hercules sends
    /// only the inventory slot, so — as with [`Self::SkillFailedMissingItem`] —
    /// the client finishes the message, since the item table lives there.
    ItemMoveFailed { item_index: InventoryIndex, amount: u16 },
    CharacterSlotSwitched,
    CharacterSlotSwitchFailed,
    /// Update entity details. Mostly received when the client sends
    /// [RequestDetailsPacket] after the player hovered an entity.
    UpdateEntityDetails {
        entity_id: EntityId,
        name: String,
    },
    UpdateEntityHealth {
        entity_id: EntityId,
        health_points: usize,
        maximum_health_points: usize,
    },
    DamageEffect {
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        /// Present when the damage came from `ZC_NOTIFY_SKILL2` rather than a
        /// basic attack. The renderer uses this to select a skill visual.
        skill_id: Option<SkillId>,
        /// Authoritative tick carried by `ZC_NOTIFY_ACT`/`ZC_NOTIFY_SKILL2`.
        /// Ragexe bases the local actor-message delay on dispatch time, but
        /// recipes and occurrence correlation must retain this packet field.
        packet_tick: ClientTick,
        /// Damage amount. [`None`] on miss, [`Some`] otherwise.
        damage_amount: Option<usize>,
        /// Number of hits bundled into this packet (`div`), at least 1. The
        /// bolt spells deliver their whole volley in one packet.
        hit_count: usize,
        /// The source motion field (`sMotion`/amotion). Ragexe does not use it
        /// to stretch source ACT playback; retain it for packet/recipe logic.
        attack_duration: u32,
        /// The target motion field (`dMotion`). Ragexe converts it to reaction
        /// cycles with `dMotion / 288.0`; zero means no flinch (e.g. Endure).
        damage_delay: u32,
        is_critical: bool,
    },
    EntityPickUpItem {
        entity_id: EntityId,
        item_entity_id: EntityId,
    },
    HealEffect {
        entity_id: EntityId,
        heal_amount: usize,
    },
    /// A successful non-damage skill use (0x09CB). This is
    /// emitted even when an area skill such as Frost Nova finds no targets,
    /// making it the correct trigger for caster-centered visuals.
    SkillEffectNoDamage {
        skill_id: SkillId,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        effect_value: u32,
        successful: bool,
    },
    AutoRunSkill {
        skill_id: SkillId,
        skill_type: SkillType,
        skill_level: SkillLevel,
        spell_point_cost: u16,
        attack_range: AttackRange,
        skill_name: String,
        upgradable: bool,
    },
    /// A timed status effect (buff or debuff) changed on an entity.
    StatusChange {
        entity_id: EntityId,
        index: u16,
        gained: bool,
        duration_ms: u32,
        remaining_ms: u32,
        /// Hercules' `val1`/`val2`/`val3` for this status, sent verbatim in
        /// `ZC_MSG_STATE_CHANGE`. These carry the server's own computed
        /// numbers — e.g. `SC_VOLCANO` sends the skill level in `val1` and the
        /// resulting ATK/MATK bonus in `val2` — so the UI can quote real
        /// values instead of re-deriving the server's formulas.
        values: [u32; 3],
    },
    /// An entity's option flags changed (`ZC_STATE_CHANGE` 0x0229).
    ///
    /// Distinct from [`NetworkEvent::StatusChange`]: those are timed
    /// buffs/debuffs, these are the persistent option bitfield (hide,
    /// cloak, riding, …). Hercules sends `sc->option` verbatim in
    /// `effect_state` (`clif_changeoption`).
    StateChange {
        entity_id: EntityId,
        /// `sc->option` — test against the `OPTION_*` masks in
        /// [`crate::EntityOption`].
        option: u32,
        /// `sc->opt1` — stun/freeze/petrify style states.
        body_state: u16,
        /// `sc->opt2` — poison/curse style states.
        health_state: u16,
        /// Packet `isPKModeON`; Ragexe stores this at actor `+0x2C0` and
        /// uses it to keep the neutral player pose combat-ready.
        is_pk_mode_on: bool,
    },
    UpdateStat {
        stat_type: StatType,
    },
    /// Soft overweight threshold percent from `ZC_OVERWEIGHT_PERCENT` (0x0ADE).
    CriticalWeightPercent {
        percent: u32,
    },
    /// Melee attack range from `ZC_ATTACK_RANGE` (0x013A).
    UpdateAttackRange {
        attack_range: AttackRange,
    },
    OpenDialog {
        text: String,
        npc_id: EntityId,
    },
    AddNextButton {
        npc_id: EntityId,
    },
    AddCloseButton {
        npc_id: EntityId,
    },
    AddChoiceButtons {
        choices: Vec<String>,
        npc_id: EntityId,
    },
    /// NPC requested a numeric input box (`ZC_OPEN_EDITDLG` 0x0142).
    NpcRequestNumberInput {
        npc_id: EntityId,
    },
    /// NPC requested a string input box (`ZC_OPEN_EDITDLGSTR` 0x01D4).
    NpcRequestStringInput {
        npc_id: EntityId,
    },
    AddQuestEffect {
        quest_effect: QuestEffectPacket,
    },
    RemoveQuestEffect {
        entity_id: EntityId,
    },
    /// A quest was added to the quest log (`ZC_ADD_QUEST` family).
    QuestAdded {
        quest_id: u32,
        active: bool,
    },
    /// A quest was erased from the quest log (`ZC_DEL_QUEST`).
    QuestRemoved {
        quest_id: u32,
    },
    /// The full quest log, sent after map login (`ZC_ALL_QUEST_LIST` family).
    QuestList {
        quest_ids: Vec<u32>,
    },
    SetInventory {
        items: Vec<InventoryItem<NoMetadata>>,
    },
    IventoryItemAdded {
        item: InventoryItem<NoMetadata>,
    },
    ItemObtained {
        item_id: ItemId,
        quantity: u16,
        is_identified: bool,
    },
    SkillTree {
        skill_information: Vec<SkillInformation>,
    },
    /// Play a sound file (`ZC_SOUND`) — the `soundeffect` script command.
    /// `entity_id` is `None` for a repeating sound, which Hercules sends with no
    /// position so it plays flat rather than in space.
    PlaySoundEffect {
        file_name: String,
        entity_id: Option<EntityId>,
    },
    /// Floating text over an entity (`ZC_SHOWSCRIPT`) — `showscript`.
    ShowScript {
        entity_id: EntityId,
        message: String,
    },
    /// The NPC progress bar started (`ZC_PROGRESS`) or was cancelled
    /// (`ZC_PROGRESS_CANCEL`). `None` cancels.
    ProgressBar {
        duration: Option<std::time::Duration>,
    },
    /// A single skill *added* to the tree (`ZC_ADD_SKILL`), as opposed to
    /// [`Self::UpdateSkill`] raising one already there. Quest rewards, the
    /// `skill` script command and Plagiarism all arrive this way, and the full
    /// tree is only re-sent on login or job change — so without this a newly
    /// granted skill is invisible until relog.
    SkillAdded {
        skill_information: SkillInformation,
    },
    UpdateEquippedPosition {
        index: InventoryIndex,
        equipped_position: EquipPosition,
    },
    ChangeJob {
        account_id: AccountId,
        job_id: JobId,
    },
    ChangeHair {
        account_id: AccountId,
        hair_id: u32,
    },
    ChangeWeapon {
        account_id: AccountId,
        weapon_id: u32,
    },
    ChangeShield {
        account_id: AccountId,
        shield_id: u32,
    },
    /// Another character's equipped ammunition changed, so their arrows can be
    /// drawn as the ammo they actually loaded rather than the generic one.
    ///
    /// A Korangar-fork broadcast — official Ragnarok never reports anyone else's
    /// ammunition. `item_id` is `0` when they unequip.
    ChangeAmmunition {
        account_id: AccountId,
        item_id: ItemId,
    },
    /// Any other sprite change: the look types that have no dedicated event of
    /// their own (headgear, hair colour, clothes colour, shoes, robe, body
    /// style).
    ///
    /// This variant exists so the `SpriteChangeType` match can be **exhaustive**.
    /// It used to end in `_ => None`, which silently discarded nine of the
    /// fourteen look types the server broadcasts — the widest hole on the
    /// wire→event boundary, and one no amount of client-side testing could see,
    /// because the value never crossed the crate boundary at all.
    ///
    /// Do not reintroduce a catch-all arm. The whole point is that adding a look
    /// type to `SpriteChangeType` now fails to compile until somebody decides
    /// what it means.
    ChangeLook {
        account_id: AccountId,
        look_type: SpriteChangeType,
        value: u32,
    },
    /// An entity turned in place (`ZC_CHANGE_DIRECTION` / 0x009C).
    ///
    /// Broadcast `AREA_WOS` from `clif_parse_ChangeDir` and `AREA` from
    /// `unit_setdir`, but the packet was a no-op here, so a remote player's
    /// facing only ever changed as a side effect of movement.
    EntityDirection {
        entity_id: EntityId,
        direction: Direction,
        head_direction: u16,
    },
    /// An entity stopped moving before reaching its destination
    /// (`ZC_STOPMOVE` / 0x0088).
    ///
    /// Previously a no-op, so the client kept animating the entity toward a
    /// destination it had already abandoned.
    EntityStopMove {
        entity_id: EntityId,
        position: TilePosition,
    },
    LoggedOut,
    FriendRequest {
        requestee: Friend,
    },
    VisualEffect {
        effect_path: &'static str,
        entity_id: EntityId,
    },
    /// Native special effect (`ZC_NOTIFY_EFFECT2` / 0x01F3). The client maps
    /// `effect_id` through `special_effect_recipe` (STR or procedural).
    SpecialEffect {
        entity_id: EntityId,
        effect_id: ragnarok_packets::EffectId,
    },
    AddSkillUnit {
        entity_id: EntityId,
        unit_id: UnitId,
        position: TilePosition,
    },
    RemoveSkillUnit {
        entity_id: EntityId,
    },
    SetFriendList {
        friend_list: Vec<Friend>,
    },
    FriendAdded {
        friend: Friend,
    },
    FriendRemoved {
        account_id: AccountId,
        character_id: CharacterId,
    },
    CreatePartyResult {
        result: u8,
    },
    /// Party share rules. The item fields are `None` from the minimal 0x0101
    /// form, which carries the EXP rule alone.
    PartyShareOptions {
        experience_share: bool,
        item_pickup_share: Option<bool>,
        item_division_share: Option<bool>,
    },
    /// NPC blacksmith refine result (0x0188), distinct from the skill-based
    /// `WeaponRefineResult`. `inventory_index` is already corrected for the
    /// server's `index + 2`.
    NpcRefineResult {
        result: u16,
        inventory_index: InventoryIndex,
        refine_level: u16,
    },
    /// A party member died or came back. Never describes the local player —
    /// the packet is sent `PARTY_WOS`.
    PartyMemberAlive {
        account_id: AccountId,
        is_dead: bool,
    },
    /// Skills Auto Spell is offering.
    AutoSpellList {
        skills: Vec<SkillId>,
    },
    /// Spirit spheres orbiting an entity (Monk spheres, Gunslinger coins).
    SpiritSpheres {
        entity_id: EntityId,
        amount: u16,
    },
    /// A trap or other skill unit changed state (Ankle Snare catching something).
    SkillUnitUpdated {
        entity_id: EntityId,
    },
    /// Instant relocation — Snap, Body Relocation, Backslide.
    EntitySnapped {
        entity_id: EntityId,
        position: TilePosition,
    },
    /// Entity effect state / level.
    EntityEffectState {
        entity_id: EntityId,
        effect_state: u32,
        level: u32,
    },
    /// Taekwon star place set or reported.
    StarPlace {
        map_name: String,
        monster_id: u32,
        star: u8,
        result: u8,
    },
    /// Server is asking which target to "feel".
    FeelRequest {
        which: u8,
    },
    /// Instance window should open, with the instance's name.
    InstanceInfo {
        instance_name: String,
    },
    /// Instance entered. Exactly one timer is non-zero: `progress` while it
    /// runs, `idle` while it waits to be entered.
    InstanceJoined {
        instance_name: String,
        progress_remaining: u32,
        idle_remaining: u32,
    },
    /// Instance window should close.
    InstanceLeft,
    /// The server re-typed a map cell (`ZC_UPDATE_MAPINFO`). Ice Wall is the
    /// common case: its cells become impassable while it stands and revert
    /// when it expires.
    MapCellChanged {
        position: TilePosition,
        cell_type: u16,
    },
    /// Result of an ignore request. `result` is 0 on success.
    IgnoreResult {
        ignore_type: u8,
        result: u8,
    },
    /// Party leadership moved to another member.
    PartyLeaderChanged {
        previous_leader_account_id: AccountId,
        new_leader_account_id: AccountId,
    },
    /// Fork packet 0x0EFF: names the sender of the `PartyInvite` that follows.
    /// Arrives first; pair the two by `party_id`.
    PartyInviteSender {
        party_id: PartyId,
        character_name: String,
    },
    PartyInvite {
        party_id: PartyId,
        party_name: String,
    },
    PartyInviteResult {
        character_name: String,
        result: u32,
    },
    PartyInvitationState {
        deny_party_invites: bool,
    },
    PartyList {
        party_name: String,
        members: Vec<PartyMember>,
    },
    PartyMemberAdded {
        member: PartyMemberInfoPacket,
    },
    PartyMemberPosition {
        account_id: AccountId,
        position: TilePosition,
    },
    PartyMemberHealth {
        account_id: AccountId,
        health_points: usize,
        maximum_health_points: usize,
        /// `(current, maximum)` spell points, or `None` from the narrow 0x080E
        /// form, which carries no SP. Our Hercules delta makes the server send
        /// the wide 0x0BAB form, so in practice this is `Some` — but a stock
        /// server would still be handled correctly.
        spell_points: Option<(usize, usize)>,
    },
    PartyMemberJobAndLevel {
        account_id: AccountId,
        job_id: JobId,
        base_level: u16,
    },
    PartyMemberRemoved {
        account_id: AccountId,
        character_name: String,
        result: u8,
    },
    PartyChatMessage {
        account_id: AccountId,
        text: String,
    },
    WhisperReceived {
        sender_character_id: CharacterId,
        sender_name: String,
        is_admin: bool,
        message: String,
    },
    WhisperResult {
        result: u8,
    },
    SetHotkeyData {
        tab: HotbarTab,
        hotkeys: Vec<HotkeyState>,
    },
    OpenShop {
        items: Vec<ShopItem<NoMetadata>>,
    },
    AskBuyOrSell {
        shop_id: ShopId,
    },
    BuyingCompleted {
        result: BuyShopItemsResult,
    },
    SellItemList {
        items: Vec<SellItemInformation>,
    },
    SellingCompleted {
        result: SellItemsResult,
    },
    InventoryItemRemoved {
        reason: RemoveItemReason,
        index: InventoryIndex,
        amount: u16,
    },
    AttackFailed {
        target_entity_id: EntityId,
        target_position: TilePosition,
        player_position: TilePosition,
        attack_range: AttackRange,
    },
    UpdateSkill {
        skill_id: SkillId,
        skill_level: SkillLevel,
        spell_point_cost: u16,
        attack_range: AttackRange,
        upgradable: bool,
    },
    /// Delete a skill from the skill tree.
    RemoveSkill {
        skill_id: SkillId,
    },
    /// Compass / minimap mark (`ZC_COMPASS` / `MarkMinimapPosition` 0x0144).
    MarkMinimap {
        npc_id: EntityId,
        marker_type: MarkerType,
        position: LargeTilePosition,
        id: u8,
        color: ColorRGBA,
    },
    /// Skill post-delay cooldown (`ZC_SKILL_POSTDELAY` 0x043D).
    SkillCooldown {
        skill_id: SkillId,
        until: ClientTick,
    },
    /// Floating experience gain notification (`ZC_NOTIFY_EXP` 0x0ACC).
    GainedExperience {
        account_id: AccountId,
        amount: u64,
        experience_type: ExperienceType,
        experience_source: ExperienceSource,
    },
    /// Friend online / offline presence (`ZC_FRIENDS_STATE` 0x0206).
    FriendOnlineStatus {
        account_id: AccountId,
        character_id: CharacterId,
        online: bool,
        name: String,
    },
    /// Entity emote balloon (`ZC_EMOTION` 0x00C0).
    DisplayEmotion {
        entity_id: EntityId,
        emotion: u8,
    },
    /// A skill was used at a ground position (`ZC_NOTIFY_GROUNDSKILL`
    /// 0x0117). The original client plays ground-cast area effects like
    /// Thunderstorm and Storm Gust from this packet, not from damage.
    GroundSkillEffect {
        skill_id: SkillId,
        source_entity_id: EntityId,
        level: SkillLevel,
        position: TilePosition,
        /// Server-synchronized effect start tick from the packet. Retained for
        /// recipe clocks even while the current STR backend starts on receipt.
        start_tick: ClientTick,
    },
    /// Skill cast bar / wind-up (`ZC_USESKILL_ACK` / success 0x0B1A / 0x07FB).
    SkillCast {
        source_entity_id: EntityId,
        skill_id: SkillId,
        /// Cast duration in milliseconds (`delay_time`).
        cast_ms: u32,
    },
    /// An active cast ended without executing (`ZC_DISPEL`), or the local
    /// skill request failed before the server supplied an actor id.
    SkillCastCancelled {
        source_entity_id: Option<EntityId>,
    },
    /// Magnifier / identify skill: list of inventory indices
    /// (`ZC_ITEMIDENTIFY_LIST`).
    ItemIdentifyList {
        indices: Vec<InventoryIndex>,
    },
    /// Identify result (`ZC_ACK_ITEMIDENTIFY`).
    ItemIdentified {
        inventory_index: InventoryIndex,
        success: bool,
    },
    /// Incoming trade request (`ZC_REQ_EXCHANGE_ITEM2`).
    TradeRequest {
        name: String,
        character_id: CharacterId,
        base_level: u16,
    },
    /// Trade window open / request result (`ZC_ACK_EXCHANGE_ITEM2`).
    TradeStart {
        result: u8,
        character_id: CharacterId,
        base_level: u16,
    },
    /// Partner added an item to the trade.
    TradePartnerItem {
        item_id: ItemId,
        item_type: u8,
        amount: u32,
        identified: bool,
        refine: u8,
    },
    /// Our add-item to trade result.
    TradeAddItemResult {
        inventory_index: InventoryIndex,
        result: u8,
    },
    /// One side locked the trade (`who`: 0 self, 1 partner).
    TradeLocked {
        who: u8,
    },
    TradeCancelled,
    TradeCompleted {
        success: bool,
    },
    /// Full storage item list (via inventory_type = STORAGE).
    SetStorage {
        items: Vec<InventoryItem<NoMetadata>>,
    },
    StorageAmount {
        amount: u16,
        max_amount: u16,
    },
    StorageItemAdded {
        item: InventoryItem<NoMetadata>,
    },
    StorageItemRemoved {
        index: InventoryIndex,
        amount: u32,
    },
    StorageClosed,
}

/// New-type so we can implement some `From` traits. This will help when
/// registering the packet handlers.
#[derive(Default)]
pub(crate) struct NetworkEventList(pub Vec<NetworkEvent>);

pub(crate) struct NoNetworkEvents;

impl From<NetworkEvent> for NetworkEventList {
    fn from(event: NetworkEvent) -> Self {
        Self(vec![event])
    }
}

impl From<Vec<NetworkEvent>> for NetworkEventList {
    fn from(events: Vec<NetworkEvent>) -> Self {
        Self(events)
    }
}

impl From<Option<NetworkEvent>> for NetworkEventList {
    fn from(event: Option<NetworkEvent>) -> Self {
        match event {
            Some(event) => Self(vec![event]),
            None => Self(Vec::new()),
        }
    }
}

impl From<NoNetworkEvents> for NetworkEventList {
    fn from(_: NoNetworkEvents) -> Self {
        Self(Vec::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    ClosedByClient,
    ConnectionError,
}

pub(crate) trait DisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent;
}

pub(crate) struct LoginServerDisconnectedEvent;
pub(crate) struct CharacterServerDisconnectedEvent;
pub(crate) struct MapServerDisconnectedEvent;

impl DisconnectedEvent for LoginServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::LoginServerDisconnected { reason }
    }
}

impl DisconnectedEvent for CharacterServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::CharacterServerDisconnected { reason }
    }
}

impl DisconnectedEvent for MapServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::MapServerDisconnected { reason }
    }
}
