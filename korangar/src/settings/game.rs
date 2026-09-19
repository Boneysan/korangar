use std::collections::HashMap;

#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

fn default_true() -> bool {
    true
}

fn default_breadcrumb_scale() -> u8 {
    100
}

fn default_breadcrumb_opacity() -> u8 {
    100
}

/// Configurable key binding for cycling hostile monster targets by distance.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum TargetHostileBinding {
    #[default]
    Tab,
    Tilde,
    KeyQ,
    Disabled,
}

impl TargetHostileBinding {
    pub fn to_key_code(self) -> Option<KeyCode> {
        match self {
            Self::Tab => Some(KeyCode::Tab),
            Self::Tilde => Some(KeyCode::Backquote),
            Self::KeyQ => Some(KeyCode::KeyQ),
            Self::Disabled => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Tab => "Tab",
            Self::Tilde => "~ (Tilde)",
            Self::KeyQ => "Q",
            Self::Disabled => "Disabled",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Tab => Self::Tilde,
            Self::Tilde => Self::KeyQ,
            Self::KeyQ => Self::Disabled,
            Self::Disabled => Self::Tab,
        }
    }
}

/// Configurable key binding to attack the current Tab-cycled hostile target
/// without needing to click or re-hover it. Deliberately a separate,
/// disable-able confirm press rather than firing automatically on Tab-select,
/// so cycling through several monsters to look around does not engage all of
/// them.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum AttackTargetBinding {
    #[default]
    Space,
    KeyF,
    KeyR,
    Disabled,
}

impl AttackTargetBinding {
    pub fn to_key_code(self) -> Option<KeyCode> {
        match self {
            Self::Space => Some(KeyCode::Space),
            Self::KeyF => Some(KeyCode::KeyF),
            Self::KeyR => Some(KeyCode::KeyR),
            Self::Disabled => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Space => "Space",
            Self::KeyF => "F",
            Self::KeyR => "R",
            Self::Disabled => "Disabled",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Space => Self::KeyF,
            Self::KeyF => Self::KeyR,
            Self::KeyR => Self::Disabled,
            Self::Disabled => Self::Space,
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
    /// Key binding to cycle visible, alive, hostile monsters by distance.
    #[serde(default)]
    pub target_hostile_binding: TargetHostileBinding,
    /// Key binding to attack the current Tab-cycled target.
    #[serde(default)]
    pub attack_target_binding: AttackTargetBinding,
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
    /// Filters for the Combat chat channel.
    #[serde(default)]
    pub combat_filters: crate::state::combat_chat::CombatFilters,
    /// Local party-color overrides. Never sent on the wire.
    #[serde(default)]
    #[hidden_element]
    pub party_color_overrides: crate::state::party_colors::PartyColorOverrides,
    /// Locally persisted tracked quest per character name.
    #[serde(default)]
    #[hidden_element]
    pub tracked_quests: HashMap<String, u32>,
    /// Local tracked-objective HUD preferences.
    #[serde(default)]
    #[hidden_element]
    pub breadcrumb_collapsed: bool,
    #[serde(default)]
    #[hidden_element]
    pub breadcrumb_hidden: bool,
    #[serde(default = "default_breadcrumb_scale")]
    #[hidden_element]
    pub breadcrumb_scale: u8,
    #[serde(default = "default_breadcrumb_opacity")]
    #[hidden_element]
    pub breadcrumb_opacity: u8,
    #[serde(default = "default_true")]
    #[hidden_element]
    pub breadcrumb_guidance_enabled: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            auto_attack: true,
            show_minimap: true,
            wasd_movement: true,
            target_hostile_binding: TargetHostileBinding::default(),
            attack_target_binding: AttackTargetBinding::default(),
            window_size: None,
            window_maximized: false,
            combat_filters: crate::state::combat_chat::CombatFilters::default(),
            party_color_overrides: crate::state::party_colors::PartyColorOverrides::default(),
            tracked_quests: HashMap::new(),
            breadcrumb_collapsed: false,
            breadcrumb_hidden: false,
            breadcrumb_scale: 100,
            breadcrumb_opacity: 100,
            breadcrumb_guidance_enabled: true,
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
}

impl Drop for GameSettings {
    fn drop(&mut self) {
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracked_quests_serialization() {
        let mut settings = GameSettings::default();
        settings.tracked_quests.insert("TestPlayer".to_string(), 20003);
        settings.breadcrumb_collapsed = true;
        settings.breadcrumb_hidden = true;
        settings.breadcrumb_scale = 125;
        settings.breadcrumb_opacity = 65;
        settings.breadcrumb_guidance_enabled = false;

        let data = ron::ser::to_string_pretty(&settings, PrettyConfig::new()).unwrap();
        let loaded: GameSettings = ron::from_str(&data).unwrap();

        assert_eq!(loaded.tracked_quests.get("TestPlayer"), Some(&20003));
        assert!(loaded.breadcrumb_collapsed);
        assert!(loaded.breadcrumb_hidden);
        assert_eq!(loaded.breadcrumb_scale, 125);
        assert_eq!(loaded.breadcrumb_opacity, 65);
        assert!(!loaded.breadcrumb_guidance_enabled);
    }
}
