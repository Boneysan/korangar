#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use super::key_bindings::{BindableAction, KeyBindings};

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum CombatTextFrequency {
    /// Show every damage, miss, and healing number.
    #[default]
    All,
    /// Keep critical hits and misses, while hiding routine damage numbers.
    Important,
    /// Hide floating numbers while leaving textual status notifications intact.
    StatusOnly,
}

impl CombatTextFrequency {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Important,
            Self::Important => Self::StatusOnly,
            Self::StatusOnly => Self::All,
        }
    }

    pub fn shows_damage(self, is_critical: bool) -> bool {
        match self {
            Self::All => true,
            Self::Important => is_critical,
            Self::StatusOnly => false,
        }
    }

    pub fn shows_miss(self) -> bool {
        matches!(self, Self::All | Self::Important)
    }

    pub fn shows_healing(self) -> bool {
        matches!(self, Self::All | Self::Important)
    }
}

/// Keep server-reported per-hit damage intact while avoiding one floating
/// label per division in a multi-hit packet.
pub fn format_damage_number(amount: usize, hit_count: usize) -> String {
    let hit_count = hit_count.max(1);
    if hit_count == 1 {
        amount.to_string()
    } else {
        format!("{amount} x {hit_count}")
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum CombatTextSize {
    Small,
    #[default]
    Normal,
    Large,
}

impl CombatTextSize {
    pub fn next(self) -> Self {
        match self {
            Self::Small => Self::Normal,
            Self::Normal => Self::Large,
            Self::Large => Self::Small,
        }
    }

    pub fn scale(self) -> f32 {
        match self {
            Self::Small => 0.8,
            Self::Normal => 1.0,
            Self::Large => 1.3,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GameSettings {
    pub auto_attack: bool,
    /// Whether the in-game minimap should be shown (Alt+M / Map button).
    /// Persisted so closing it stays closed across map changes and restarts.
    #[serde(default = "default_true")]
    pub show_minimap: bool,
    /// Camera-relative WASD movement. Click-to-move stays available either way.
    #[serde(default = "default_true")]
    pub wasd_movement: bool,
    /// Suppress camera shake and other nonessential camera motion.
    #[serde(default)]
    pub reduce_motion: bool,
    /// Reduce the brightness of procedural combat bursts and their point
    /// lights.
    #[serde(default)]
    pub reduce_flashing: bool,
    /// Cast a ground-targeted skill at the current cursor cell when selected,
    /// falling back to the armed aim-and-click flow when the cursor has no map
    /// target.
    #[serde(default)]
    pub quickcast_ground_skills: bool,
    /// Show floating damage, miss, and healing numbers.
    #[serde(default = "default_true")]
    pub show_combat_text: bool,
    /// Select how much floating damage feedback to show while enabled.
    #[serde(default)]
    pub combat_text_frequency: CombatTextFrequency,
    /// Size multiplier for floating combat text.
    #[serde(default)]
    pub combat_text_size: CombatTextSize,
    /// User overrides for keyboard shortcuts; missing entries use shipped
    /// defaults.
    #[serde(default)]
    #[hidden_element]
    pub key_bindings: KeyBindings,
    /// In-memory remap capture request; deliberately not persisted.
    #[serde(skip)]
    #[hidden_element]
    pub pending_key_binding: Option<BindableAction>,
    /// Last window size in **logical** pixels, restored on the next launch.
    ///
    /// Logical rather than physical so moving between monitors of different
    /// scale factors restores the same apparent size rather than the same pixel
    /// count. `None` means "never resized", and the window opens at
    /// `INITIAL_SCREEN_SIZE`.
    #[serde(default)]
    #[hidden_element]
    pub window_size: Option<(u32, u32)>,
    /// Whether the window was maximized when it last closed. Kept separate from
    /// `window_size`, which keeps the size to restore when un-maximized.
    #[serde(default)]
    #[hidden_element]
    pub window_maximized: bool,
    /// When set, the hotbar ignores drags. Number keys still cast.
    #[serde(default)]
    pub hotbar_locked: bool,
    /// Character overview shows only the name line.
    #[serde(default)]
    pub overview_minimized: bool,
    /// Item IDs protected from dropping or NPC sale, keyed by character ID.
    #[serde(default)]
    #[hidden_element]
    pub protected_items_by_character: Vec<(u32, Vec<u32>)>,
    /// Player-selected tracked quest IDs, keyed by character ID.
    #[serde(default)]
    #[hidden_element]
    pub tracked_quests_by_character: Vec<(u32, Vec<u32>)>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            auto_attack: true,
            show_minimap: true,
            wasd_movement: true,
            reduce_motion: false,
            reduce_flashing: false,
            quickcast_ground_skills: false,
            show_combat_text: true,
            combat_text_frequency: CombatTextFrequency::default(),
            combat_text_size: CombatTextSize::default(),
            key_bindings: KeyBindings::default(),
            pending_key_binding: None,
            window_size: None,
            window_maximized: false,
            hotbar_locked: false,
            overview_minimized: false,
            protected_items_by_character: Vec::new(),
            tracked_quests_by_character: Vec::new(),
        }
    }
}

impl GameSettings {
    const FILE_NAME: &'static str = "client/game_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load game settings from {}", Self::FILE_NAME.magenta());
            Default::default()
        })
    }

    /// The saved window geometry, without leaving a `GameSettings` to drop.
    ///
    /// The first window is created before the client state exists, so this is
    /// read straight off disk. It deliberately does **not** hand back a
    /// `GameSettings`: dropping one writes the file (see the `Drop` impl), so a
    /// throwaway instance here would rewrite settings during startup, before
    /// anything has been loaded that could have changed them.
    pub fn saved_window_geometry() -> (Option<(u32, u32)>, bool) {
        let settings = std::mem::ManuallyDrop::new(Self::load().unwrap_or_default());
        (settings.window_size, settings.window_maximized)
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading game settings from {}", Self::FILE_NAME.magenta());
        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving game settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save game settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }

    pub fn is_item_protected(&self, character_id: u32, item_id: u32) -> bool {
        self.protected_items_by_character
            .iter()
            .find(|(id, _)| *id == character_id)
            .is_some_and(|(_, items)| items.contains(&item_id))
    }

    pub fn toggle_item_protection(&mut self, character_id: u32, item_id: u32) {
        let position = self.protected_items_by_character.iter().position(|(id, _)| *id == character_id);
        let items = match position {
            Some(position) => &mut self.protected_items_by_character[position].1,
            None => {
                self.protected_items_by_character.push((character_id, Vec::new()));
                &mut self
                    .protected_items_by_character
                    .last_mut()
                    .expect("inserted character protection list")
                    .1
            }
        };
        if let Some(position) = items.iter().position(|id| *id == item_id) {
            items.remove(position);
        } else {
            items.push(item_id);
            items.sort_unstable();
        }
        self.protected_items_by_character.retain(|(_, items)| !items.is_empty());
    }

    pub fn tracked_quests(&self, character_id: u32) -> Option<&[u32]> {
        self.tracked_quests_by_character
            .iter()
            .find(|(id, _)| *id == character_id)
            .map(|(_, quest_ids)| quest_ids.as_slice())
    }

    pub fn set_tracked_quests(&mut self, character_id: u32, quest_ids: &[u32]) {
        let quest_ids = {
            let mut quest_ids = quest_ids.to_vec();
            quest_ids.sort_unstable();
            quest_ids.dedup();
            quest_ids
        };
        if let Some((_, existing)) = self.tracked_quests_by_character.iter_mut().find(|(id, _)| *id == character_id) {
            *existing = quest_ids;
        } else {
            self.tracked_quests_by_character.push((character_id, quest_ids));
        }
    }
}

impl Drop for GameSettings {
    fn drop(&mut self) {
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use std::mem::ManuallyDrop;

    use super::super::key_bindings::BindableAction;
    use super::{CombatTextFrequency, CombatTextSize, GameSettings, format_damage_number};

    #[test]
    fn accessibility_settings_have_safe_defaults_and_migrate_older_files() {
        let default_settings = ManuallyDrop::new(GameSettings::default());
        assert!(!default_settings.reduce_motion);
        assert!(!default_settings.reduce_flashing);
        assert!(!default_settings.quickcast_ground_skills);
        assert!(default_settings.show_combat_text);
        assert_eq!(default_settings.combat_text_frequency, CombatTextFrequency::All);
        assert_eq!(default_settings.combat_text_size, CombatTextSize::Normal);

        let old_settings: ManuallyDrop<GameSettings> = ManuallyDrop::new(ron::from_str("(auto_attack:true)").unwrap());
        assert!(!old_settings.reduce_motion);
        assert!(!old_settings.reduce_flashing);
        assert!(!old_settings.quickcast_ground_skills);
        assert!(old_settings.show_combat_text);
        assert_eq!(old_settings.combat_text_frequency, CombatTextFrequency::All);
        assert_eq!(old_settings.combat_text_size, CombatTextSize::Normal);
        assert_eq!(old_settings.key_bindings.chord(BindableAction::OpenInventory).key, "KeyI");
        assert_eq!(old_settings.pending_key_binding, None);

        let migrated_user_choices: ManuallyDrop<GameSettings> = ManuallyDrop::new(
            ron::from_str("(auto_attack:true, show_combat_text:false, combat_text_frequency:Important, combat_text_size:Large)").unwrap(),
        );
        assert!(!migrated_user_choices.show_combat_text);
        assert_eq!(migrated_user_choices.combat_text_frequency, CombatTextFrequency::Important);
        assert_eq!(migrated_user_choices.combat_text_size, CombatTextSize::Large);
        let status_only: ManuallyDrop<GameSettings> =
            ManuallyDrop::new(ron::from_str("(auto_attack:true, combat_text_frequency:StatusOnly)").unwrap());
        assert_eq!(status_only.combat_text_frequency, CombatTextFrequency::StatusOnly);
        let quickcast: ManuallyDrop<GameSettings> =
            ManuallyDrop::new(ron::from_str("(auto_attack:true, quickcast_ground_skills:true)").unwrap());
        assert!(quickcast.quickcast_ground_skills);
    }

    #[test]
    fn combat_text_accessibility_modes_are_predictable_and_cyclable() {
        assert!(CombatTextFrequency::All.shows_damage(false));
        assert!(CombatTextFrequency::All.shows_miss());
        assert!(CombatTextFrequency::All.shows_healing());
        assert!(CombatTextFrequency::Important.shows_damage(true));
        assert!(!CombatTextFrequency::Important.shows_damage(false));
        assert!(CombatTextFrequency::Important.shows_miss());
        assert!(CombatTextFrequency::Important.shows_healing());
        assert!(!CombatTextFrequency::StatusOnly.shows_damage(true));
        assert!(!CombatTextFrequency::StatusOnly.shows_miss());
        assert!(!CombatTextFrequency::StatusOnly.shows_healing());
        assert_eq!(CombatTextFrequency::All.next(), CombatTextFrequency::Important);
        assert_eq!(CombatTextFrequency::Important.next(), CombatTextFrequency::StatusOnly);
        assert_eq!(CombatTextFrequency::StatusOnly.next(), CombatTextFrequency::All);

        assert_eq!(CombatTextSize::Small.next(), CombatTextSize::Normal);
        assert_eq!(CombatTextSize::Normal.next(), CombatTextSize::Large);
        assert_eq!(CombatTextSize::Large.next(), CombatTextSize::Small);
        assert!(CombatTextSize::Small.scale() < CombatTextSize::Normal.scale());
        assert!(CombatTextSize::Normal.scale() < CombatTextSize::Large.scale());
    }

    #[test]
    fn multi_hit_damage_text_is_compact_without_aggregating_server_damage() {
        assert_eq!(format_damage_number(123, 1), "123");
        assert_eq!(format_damage_number(123, 0), "123");
        assert_eq!(format_damage_number(123, 5), "123 x 5");
    }
}
