#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
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
    pub protected_items_by_character: Vec<(u32, Vec<u32>)>,
    /// Player-selected tracked quest IDs, keyed by character ID.
    #[serde(default)]
    pub tracked_quests_by_character: Vec<(u32, Vec<u32>)>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            auto_attack: true,
            show_minimap: true,
            wasd_movement: true,
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
                &mut self.protected_items_by_character.last_mut().expect("inserted character protection list").1
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
