#[cfg(feature = "debug")]
use cgmath::Vector2;
#[cfg(feature = "debug")]
use korangar_debug::profiling::FrameMeasurement;
use korangar_interface::event::{ClickHandler, Event, EventQueue};
use korangar_networking::{InventoryItem, ShopItem};
use ragnarok_packets::{
    AccountId, AttackRange, BuyOrSellOption, CharacterId, CharacterServerInformation, EntityId, HotbarSlot, RepairableItemInformation,
    ShopId, SkillId, SkillLevel, SoldItemInformation, StatUpType, TilePosition,
};
use rust_state::State;

use crate::interface::resource::{ItemSource, SkillSource};
use crate::loaders::ServiceId;
use crate::state::ClientState;
use crate::state::skills::LearnableSkill;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::ResourceMetadata;

/// An event triggered by the user through mouse or keyboard input.
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// Log in to the login server.
    LogIn {
        /// Id of the selected service.
        service_id: ServiceId,
        /// Account username.
        username: String,
        /// Account password.
        password: String,
    },
    /// Select a character server.
    SelectServer {
        /// Selected character server.
        character_server_information: CharacterServerInformation,
    },
    /// Respawn the player.
    Respawn,
    /// Log out of the map server.
    LogOut,
    /// Log out of the character server.
    LogOutCharacter,
    /// Exit Korangar.
    Exit,
    /// Zoom the player camera.
    ZoomCamera {
        /// Amount to zoom.
        zoom_factor: f32,
    },
    /// Rotate the player camera.
    RotateCamera {
        /// Amount of rotation.
        rotation: f32,
    },
    /// Reset the player camera rotation.
    ResetCameraRotation,
    /// Open or close the menu window. Only works while playing.
    ToggleMenuWindow,
    /// Open or close the basic character information window.
    ToggleCharacterOverviewWindow,
    /// Open or close the inventory window. Only works while playing.
    ToggleInventoryWindow,
    /// Open or close the equipment window. Only works while playing.
    ToggleEquipmentWindow,
    /// Open or close the skill tree window. Only works while playing.
    ToggleSkillTreeWindow,
    /// Open or close the stats window. Only works while playing.
    ToggleStatsWindow,
    /// Open or close the game settings window.
    ToggleGameSettingsWindow,
    /// Open or close the interface settings window.
    ToggleInterfaceSettingsWindow,
    /// Open or close the graphics settings window.
    ToggleGraphicsSettingsWindow,
    /// Open or close the audio settings window.
    ToggleAudioSettingsWindow,
    /// Open or close the friend list window. Only works while playing.
    ToggleFriendListWindow,
    /// Open or close the party roster window. Only works while playing.
    TogglePartyWindow,
    /// Open or close the zeny/exp HUD. Only works while playing.
    ToggleHudWindow,
    /// Close the most recently opened or clicked closable window.
    CloseTopWindow,
    /// Close all ordinary windows while retaining basic info and chat (F11).
    CloseAllOrdinaryWindows,
    /// Toggle if the user interface should be rendered or not.
    ToggleShowInterface,
    /// Select a character to start playing.
    SelectCharacter {
        /// Slot that the selected character is in.
        slot: usize,
    },
    /// Open a window to create a new character.
    OpenCharacterCreationWindow {
        /// Slot in which to create the new character.
        slot: usize,
    },
    /// Create a new character.
    CreateCharacter {
        /// Slot in which to create the new character.
        slot: usize,
        /// Name of the new character.
        name: String,
    },
    /// Delete a character.
    DeleteCharacter {
        /// Id of the character to be deleted.
        character_id: CharacterId,
    },
    /// Switch the characters of two slots.
    SwitchCharacterSlot {
        /// First slot.
        origin_slot: usize,
        /// Second slot.
        destination_slot: usize,
    },
    /// Start moving the player.
    PlayerMove {
        /// Destination of the move.
        destination: TilePosition,
    },
    /// Interact with an entity. The type of interaction depends on the entity
    /// type.
    PlayerInteract {
        /// Id of the entity to interact with.
        entity_id: EntityId,
    },
    /// Pick up an item from the ground.
    PickUpItem {
        /// Id of the item entity to pick up.
        entity_id: EntityId,
    },
    /// Send a chat message.
    SendMessage {
        /// Text of the message.
        text: String,
    },
    /// Request that the map server broadcast an emote over our character.
    UseEmotion {
        emotion: u8,
    },
    /// Toggle sit / stand (official client: Insert).
    ToggleSit,
    /// Toggle the in-game minimap window (official-style map corner).
    ToggleMinimapWindow,
    /// Grow the minimap square (button / scroll).
    MinimapZoomIn,
    /// Shrink the minimap square (button / scroll).
    MinimapZoomOut,
    /// Use a consumable / trigger item use (`CZ_USE_ITEM2`).
    UseItem {
        inventory_index: ragnarok_packets::InventoryIndex,
    },
    /// Drop an inventory item onto the ground (`CZ_ITEM_THROW2`).
    DropItem {
        inventory_index: ragnarok_packets::InventoryIndex,
        amount: u16,
    },
    /// Reorder inventory display by dragging an item onto another grid slot.
    /// Server inventory indices are unchanged; this is client layout only.
    ReorderInventory {
        from_index: ragnarok_packets::InventoryIndex,
        to_slot: usize,
    },
    /// Open the inventory item actions popup (right-click menu).
    OpenItemActions {
        item: InventoryItem<ResourceMetadata>,
    },
    /// Close the inventory item actions popup.
    CloseItemActions,
    /// One-click identify with a magnifier.
    IdentifyItem {
        inventory_index: ragnarok_packets::InventoryIndex,
    },
    /// Cancel identify dialog.
    IdentifyCancel,
    /// Choose one of the destinations supplied by a warp skill.
    SelectWarpDestination {
        skill_id: SkillId,
        map_name: String,
    },
    CancelWarpSelection {
        skill_id: SkillId,
    },
    /// Choose a weapon supplied by the Weapon Refine skill.
    RefineWeapon {
        inventory_index: ragnarok_packets::InventoryIndex,
    },
    CancelWeaponRefine,
    /// Choose broken equipment supplied by Repair Weapon.
    RepairItem {
        item: RepairableItemInformation,
    },
    CancelItemRepair,
    /// Accept pending trade request.
    TradeAccept,
    /// Reject pending trade request.
    TradeReject,
    /// Put an inventory item into the open trade.
    TradeAddItem {
        inventory_index: ragnarok_packets::InventoryIndex,
        amount: u32,
    },
    /// Put zeny into the open trade.
    TradeAddZeny {
        amount: u32,
    },
    /// Lock our trade offer.
    TradeOk,
    /// Commit trade (both sides must have locked).
    TradeCommit,
    /// Cancel active trade.
    TradeCancel,
    /// Close kafra storage.
    CloseStorage,
    /// Action for the "Next"-button in a dialog.
    NextDialog {
        /// Id of the NPC the player is in a dialog with.
        npc_id: EntityId,
    },
    /// Action for the "Close"-button in a dialog.
    CloseDialog {
        /// Id of the NPC the player is in a dialog with.
        npc_id: EntityId,
    },
    /// Choose an option in a dialog.
    ChooseDialogOption {
        /// Id of the NPC the player is in a dialog with.
        npc_id: EntityId,
        /// Id of the option.
        option: i8,
    },
    /// Submit a number from the NPC input dialog.
    SubmitDialogNumber {
        npc_id: EntityId,
        value: i32,
    },
    /// Submit a string from the NPC input dialog.
    SubmitDialogString {
        npc_id: EntityId,
        text: String,
    },
    /// Move an item in the user interface.
    MoveItem {
        /// Source of the move.
        source: ItemSource,
        /// Destination of the move.
        destination: ItemSource,
        /// Item to move.
        item: InventoryItem<ResourceMetadata>,
    },
    /// Move a skill in the user interface.
    MoveSkill {
        /// Source of the move.
        source: SkillSource,
        /// Destination of the move.
        destination: SkillSource,
        /// Skill to move.
        skill: LearnableSkill,
    },
    /// Assign a skill to the first free hotbar slot (skill-tree right click).
    AssignSkillToHotbar {
        skill: LearnableSkill,
    },
    /// Cast a skill.
    CastSkill {
        /// Slot of the hotbar that the skill is bound to.
        slot: HotbarSlot,
    },
    /// Cast an entity-targeted skill, walking into its range first.
    CastSkillAtEntity {
        skill_id: SkillId,
        skill_level: SkillLevel,
        attack_range: AttackRange,
        entity_id: EntityId,
    },
    /// Cast a ground-targeted skill at a cell, walking into its range first.
    CastSkillAtTile {
        skill_id: SkillId,
        skill_level: SkillLevel,
        attack_range: AttackRange,
        tile: TilePosition,
    },
    /// Stop a skill.
    StopSkill {
        /// Slot of the hotbar that the skill is bound to.
        slot: HotbarSlot,
    },
    /// Add a new friend.
    AddFriend {
        /// Name of the character to befriend.
        character_name: String,
    },
    /// Remove a current friend.
    RemoveFriend {
        /// Account id of the friend.
        account_id: AccountId,
        /// Character id of the friend.
        character_id: CharacterId,
    },
    /// Reject a pending friend request.
    RejectFriendRequest {
        /// Account id of the requestor.
        account_id: AccountId,
        /// Character id of the requestor.
        character_id: CharacterId,
    },
    /// Accept a pending friend request.
    AcceptFriendRequest {
        /// Account id of the requestor.
        account_id: AccountId,
        /// Character id of the requestor.
        character_id: CharacterId,
    },
    /// Create a new party.
    CreateParty {
        /// Name of the party to create.
        party_name: String,
    },
    /// Invite a character to the current party.
    InviteToParty {
        /// Name of the character to invite.
        character_name: String,
    },
    /// Accept the pending party invite.
    AcceptPartyInvite,
    /// Reject the pending party invite.
    RejectPartyInvite,
    /// Leave the current party.
    LeaveParty,
    /// Choose one of the skills Auto Spell offered.
    SelectAutoSpell {
        skill_id: ragnarok_packets::SkillId,
    },
    /// Point the chat window at a character for whispering.
    StartWhisper {
        /// Character to whisper to.
        character_name: String,
    },
    /// Ask a character to trade.
    RequestTrade {
        /// Account id of the character to trade with.
        account_id: AccountId,
    },
    /// Kick a member from the party (leader only).
    KickPartyMember {
        account_id: AccountId,
        character_name: String,
    },
    /// Hand party leadership to another member (leader only).
    PromotePartyLeader {
        account_id: AccountId,
    },
    /// Set the party's three share rules. All three are sent together because
    /// the packet has no "unchanged" encoding.
    SetPartyShare {
        experience: bool,
        pickup: bool,
        division: bool,
    },
    /// Add or remove a character from the whisper ignore list.
    SetPlayerIgnored {
        character_name: String,
        ignored: bool,
    },
    /// Allow or block incoming party invites.
    SetPartyInvitationBlock {
        /// `true` refuses every invite server-side.
        blocked: bool,
    },
    /// Buy items from a shop.
    BuyItems {
        /// Items to buy.
        items: Vec<ShopItem<u32>>,
    },
    /// Close the shop.
    CloseShop,
    /// Choose whether to buy or sell items at a shop.
    BuyOrSell {
        /// Id of the open shop.
        shop_id: ShopId,
        /// Whether to sell or buy items.
        buy_or_sell: BuyOrSellOption,
    },
    /// Sell items to a shop.
    SellItems {
        /// Items to sell.
        items: Vec<SoldItemInformation>,
    },
    /// Up a stat.
    StatUp {
        stat_type: StatUpType,
    },
    /// Distribute skill points to meet all requirements for a given skill and
    /// put a single point into the provided skill. If the player does not
    /// have enough skill points, this will skill as much of the
    /// dependencies as possible.
    DistributePointsForSkill {
        /// Id of the skill to level up.
        skill_id: SkillId,
    },
    /// Level up a skill.
    LevelUpSkills {
        /// List of skills to level up by one. This list is allowed to contain
        /// the same skill id multiple times and they will be applied
        /// sequentially from start to end.
        skill_ids: Vec<SkillId>,
    },
    /// Reload the language from disk.
    #[cfg(feature = "debug")]
    ReloadLanguage,
    /// Save the language to disk.
    #[cfg(feature = "debug")]
    SaveLanguage,
    /// Warp the player.
    #[cfg(feature = "debug")]
    WarpToMap {
        /// Map name. Can be the same as the current map.
        map_name: String,
        /// Position on the new map after the warp.
        position: TilePosition,
    },
    /// Open a window with the details for a marker.
    #[cfg(feature = "debug")]
    OpenMarkerDetails {
        /// Id of the marker to inspect.
        marker_identifier: MarkerIdentifier,
    },
    /// Open or close the render options window.
    #[cfg(feature = "debug")]
    ToggleRenderOptionsWindow,
    /// Open the map data window.
    #[cfg(feature = "debug")]
    OpenMapDataWindow,
    /// Open or close the client state inspector window.
    #[cfg(feature = "debug")]
    ToggleClientStateInspectorWindow,
    /// Open or close the maps window. Only works while playing.
    #[cfg(feature = "debug")]
    ToggleMapsWindow,
    /// Open or close the GM/DM commands window. Only works while playing.
    ToggleCommandsWindow,
    ToggleDiceWindow,
    /// Open or close the player emote palette. Only works while playing.
    ToggleEmoteWindow,
    /// Open or close the bestiary journal. Only works while playing.
    ToggleBestiaryWindow,
    /// Open or close the DM loot generator. Only works while playing.
    ToggleLootWindow,
    /// Open the theme inspector window.
    #[cfg(feature = "debug")]
    ToggleThemeInspectorWindow,
    /// Open or close the profiler window.
    #[cfg(feature = "debug")]
    ToggleProfilerWindow,
    /// Open or close the packet inspector window.
    #[cfg(feature = "debug")]
    TogglePacketInspectorWindow,
    /// Open the cache statistics window.
    #[cfg(feature = "debug")]
    ToggleCacheStatisticsWindow,
    /// Move the view direction of the debug camera.
    #[cfg(feature = "debug")]
    CameraLookAround {
        /// Offset of the view direction.
        offset: Vector2<f32>,
    },
    /// Move the debug camera forward.
    #[cfg(feature = "debug")]
    CameraMoveForward,
    /// Move the debug camera backward.
    #[cfg(feature = "debug")]
    CameraMoveBackward,
    /// Move the debug camera left.
    #[cfg(feature = "debug")]
    CameraMoveLeft,
    /// Move the debug camera right.
    #[cfg(feature = "debug")]
    CameraMoveRight,
    /// Move the debug camera up.
    #[cfg(feature = "debug")]
    CameraMoveUp,
    /// Set the debug camera speed to its higher value.
    #[cfg(feature = "debug")]
    CameraAccelerate,
    /// Set the debug camera speed to its lower value.
    #[cfg(feature = "debug")]
    CameraDecelerate,
    /// Open a window to inspect a frame.
    #[cfg(feature = "debug")]
    InspectFrame {
        measurement: FrameMeasurement,
    },
}

impl From<InputEvent> for Event<ClientState> {
    fn from(custom_event: InputEvent) -> Self {
        Event::Application { custom_event }
    }
}

impl ClickHandler<ClientState> for InputEvent {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(self.clone());
    }
}
