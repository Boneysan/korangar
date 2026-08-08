mod audio_settings;
mod buy;
mod buy_cart;
mod buy_or_sell;
mod cache;
mod character_creation;
mod character_overview;
mod character_selection;
mod chat;
mod commands;
mod dialog;
mod dice;
mod dm;
mod emote;
mod equipment;
mod disconnect_notice;
mod error;
#[cfg(feature = "debug")]
mod frame_inspector;
mod friend_list;
mod friend_request;
mod game_settings;
mod graphics_settings;
mod hotbar;
mod hud;
mod identify;
mod interface_settings;
mod inventory;
mod item_actions;
mod login;
#[cfg(feature = "debug")]
mod maps;
mod menu;
mod minimap;
#[cfg(feature = "debug")]
mod packet_inspector;
mod auto_spell;
mod instance;
mod party;
mod party_invite;
mod player_target;
#[cfg(feature = "debug")]
mod profiler;
#[cfg(feature = "debug")]
mod render_options;
mod repair_weapon;
mod respawn;
mod selection_list;
mod sell;
mod sell_cart;
mod server_selection;
mod skill_tree;
mod stats;
mod status_bar;
mod storage;
#[cfg(feature = "debug")]
mod theme_inspector;
mod trade;
mod warp_selection;
mod weapon_refine;

use serde::{Deserialize, Serialize};

pub use self::audio_settings::AudioSettingsWindow;
pub use self::buy::BuyWindow;
pub use self::buy_cart::BuyCartWindow;
pub use self::buy_or_sell::BuyOrSellWindow;
pub use self::cache::WindowCache;
pub use self::character_creation::CharacterCreationWindow;
pub use self::character_overview::CharacterOverviewWindow;
pub use self::character_selection::CharacterSelectionWindow;
pub use self::chat::{ChatTextBox, ChatWindow, ChatWindowState};
pub use self::commands::{CommandsWindow, CommandsWindowState};
pub use self::dialog::{DialogWindow, DialogWindowState};
pub use self::dice::{DiceWindow, DiceWindowState};
pub use self::dm::{BestiaryWindow, BestiaryWindowState, LootGeneratorWindow, LootWindowState};
pub use self::emote::EmoteWindow;
pub use self::equipment::EquipmentWindow;
pub use self::disconnect_notice::DisconnectNoticeWindow;
pub use self::error::ErrorWindow;
#[cfg(feature = "debug")]
pub use self::frame_inspector::FrameInspectorWindow;
pub use self::friend_list::{FriendListWindow, FriendListWindowState};
pub use self::friend_request::FriendRequestWindow;
pub use self::game_settings::GameSettingsWindow;
pub use self::graphics_settings::GraphicsSettingsWindow;
pub use self::hotbar::HotbarWindow;
pub use self::hud::HudWindow;
pub use self::identify::IdentifyWindow;
pub use self::interface_settings::InterfaceSettingsWindow;
pub use self::inventory::InventoryWindow;
pub use self::item_actions::{ItemActionsWindow, inventory_item_amount};
pub use self::login::{LoginWindow, LoginWindowState, LoginWindowStatePathExt};
#[cfg(feature = "debug")]
pub use self::maps::MapsWindow;
pub use self::menu::MenuWindow;
pub use self::minimap::MinimapWindow;
#[cfg(feature = "debug")]
pub use self::packet_inspector::PacketInspectorWindow;
pub use self::auto_spell::AutoSpellWindow;
pub use self::instance::InstanceWindow;
pub use self::party::{PartyWindow, PartyWindowState};
pub use self::party_invite::PartyInviteWindow;
pub use self::player_target::PlayerTargetWindow;
#[cfg(feature = "debug")]
pub use self::profiler::{ProfilerWindow, ProfilerWindowState};
#[cfg(feature = "debug")]
pub use self::render_options::RenderOptionsWindow;
pub use self::repair_weapon::RepairWeaponWindow;
pub use self::respawn::RespawnWindow;
pub use self::sell::SellWindow;
pub use self::sell_cart::SellCartWindow;
pub use self::server_selection::ServerSelectionWindow;
pub use self::skill_tree::{SkillTreeWindow, SkillTreeWindowState, SkillTreeWindowStatePathExt};
pub use self::stats::StatsWindow;
pub use self::status_bar::StatusBarWindow;
pub use self::storage::StorageWindow;
#[cfg(feature = "debug")]
pub use self::theme_inspector::{ThemeInspectorWindow, ThemeInspectorWindowState};
pub use self::trade::{TradeRequestWindow, TradeWindow, TradeWindowState};
pub use self::warp_selection::WarpSelectionWindow;
pub use self::weapon_refine::WeaponRefineWindow;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowClass {
    AudioSettings,
    Buy,
    BuyCart,
    BuyOrSell,
    Chat,
    CharacterCreation,
    CharacterOverview,
    CharacterSelection,
    Dialog,
    GameSettings,
    InterfaceSettings,
    GraphicsSettings,
    Hotbar,
    Hud,
    Inventory,
    ItemActions,
    Equipment,
    Emotes,
    /// Login / network error popup (wrong password, disconnect, …).
    Error,
    DisconnectNotice,
    StatusBar,
    SkillTree,
    Stats,
    FriendList,
    FriendRequest,
    Login,
    Menu,
    Minimap,
    Party,
    /// Incoming party invite popup (Accept / Decline).
    PartyInvite,
    /// Auto Spell skill chooser.
    AutoSpell,
    /// Memorial dungeon / instance information.
    Instance,
    /// Target frame for a clicked player (whisper / invite / trade / befriend).
    PlayerTarget,
    Storage,
    Trade,
    TradeRequest,
    Identify,
    Respawn,
    SelectServer,
    Sell,
    SellCart,
    WarpSelection,
    WeaponRefine,
    RepairWeapon,
    /// GM / DM command panel (levels, zeny, heal, campaign mode). Available in
    /// all builds.
    Commands,
    /// Dice roller (sends `@roll`). Available to all players in all builds.
    Dice,
    /// Bestiary journal (Seal Cascade campaign, unlock-on-kill).
    Bestiary,
    /// DM loot / rewards generator (Seal Cascade campaign).
    DmLoot,
    #[cfg(feature = "debug")]
    Maps,
    #[cfg(feature = "debug")]
    ClientStateInspector,
    #[cfg(feature = "debug")]
    PacketInspector,
    #[cfg(feature = "debug")]
    RenderOptions,
    #[cfg(feature = "debug")]
    ThemeInspector,
    #[cfg(feature = "debug")]
    Profiler,
    #[cfg(feature = "debug")]
    CacheStatistics,
}
