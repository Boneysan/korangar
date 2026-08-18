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
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            auto_attack: true,
            show_minimap: true,
            window_size: None,
            window_maximized: false,
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
