use std::collections::HashMap;

#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::window::{Anchor, AnchorPoint};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use super::WindowClass;
use crate::graphics::{ScreenPosition, ScreenSize};
use crate::state::ClientState;

const MARGIN: f32 = 12.0;

#[derive(Serialize, Deserialize)]
pub struct WindowState {
    pub anchor: Anchor<ClientState>,
    pub size: ScreenSize,
}

impl WindowState {
    pub fn new(anchor: Anchor<ClientState>, size: ScreenSize) -> Self {
        Self { anchor, size }
    }

    /// Placeholder size written on first open before layout — not a real size.
    fn has_valid_size(&self) -> bool {
        self.size.width.is_finite()
            && self.size.height.is_finite()
            && self.size.width >= 16.0
            && self.size.height >= 16.0
            && self.size.width < 10_000.0
            && self.size.height < 10_000.0
    }

    /// User has settled placement (dragged or seeded with `initializing:
    /// false`).
    fn has_settled_anchor(&self) -> bool {
        !self.anchor.is_initializing()
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct WindowCache {
    entries: HashMap<WindowClass, WindowState>,
}

impl WindowCache {
    // Since `WindowClass` has some variants with debug features enabled, we use a
    // different file to store the window cache. This avoids failing to load and
    // thereby wiping the previous window cache when switching between debug and
    // non-debug builds.
    #[cfg(not(feature = "debug"))]
    const FILE_NAME: &'static str = "client/window_cache.ron";
    #[cfg(feature = "debug")]
    const FILE_NAME: &'static str = "client/window_cache_debug.ron";

    fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading window cache from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .map(|entries| Self { entries })
    }

    fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving window cache to {}", Self::FILE_NAME.magenta());

        if let Err(_error) = std::fs::create_dir_all("client") {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] failed to create client/ for window cache: {:?}",
                "warning".yellow(),
                _error
            );
        }

        let data = ron::ser::to_string_pretty(&self.entries, PrettyConfig::new()).unwrap();
        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] failed to save window cache to {}: {:?}",
                "warning".yellow(),
                Self::FILE_NAME.magenta(),
                _error
            );
        }
    }

    /// RO-style default placement around the screen edge (not center).
    fn default_for_class(class: WindowClass) -> Option<WindowState> {
        let state = |point, left, top, width, height| {
            WindowState::new(Anchor::with_point(point, ScreenPosition { left, top }), ScreenSize {
                width,
                height,
            })
        };

        Some(match class {
            // Bottom-left chat strip (classic RO).
            WindowClass::Chat => state(AnchorPoint::BottomLeft, MARGIN, -(180.0 + MARGIN), 480.0, 180.0),
            // Bottom-center action bar.
            WindowClass::Hotbar => state(AnchorPoint::BottomCenter, -220.0, -(80.0 + MARGIN), 440.0, 80.0),
            // Top-left character sheet / menus.
            WindowClass::CharacterOverview => state(AnchorPoint::TopLeft, MARGIN, MARGIN, 300.0, 280.0),
            WindowClass::Menu => state(AnchorPoint::TopLeft, MARGIN, 300.0, 220.0, 320.0),
            // Right column: inventory, equipment, stats.
            WindowClass::Inventory => state(AnchorPoint::TopRight, -(380.0 + MARGIN), MARGIN, 380.0, 320.0),
            WindowClass::Equipment => state(AnchorPoint::TopRight, -(280.0 + MARGIN), 340.0, 280.0, 420.0),
            WindowClass::Stats => state(AnchorPoint::CenterRight, -(280.0 + MARGIN), -120.0, 280.0, 360.0),
            WindowClass::SkillTree => state(AnchorPoint::CenterLeft, MARGIN, -200.0, 420.0, 400.0),
            WindowClass::FriendList => state(AnchorPoint::CenterLeft, MARGIN, 80.0, 280.0, 360.0),
            // Minimap top-right (above inventory).
            WindowClass::Minimap => state(AnchorPoint::TopRight, -(176.0 + MARGIN), MARGIN, 176.0, 210.0),
            // Buff bar top-center.
            WindowClass::StatusBar => state(AnchorPoint::TopCenter, -160.0, MARGIN, 320.0, 48.0),
            // Zeny / EXP / cooldown strip under the minimap.
            WindowClass::Hud => state(AnchorPoint::TopRight, -(280.0 + MARGIN), 230.0, 260.0, 110.0),
            // Party roster left of center.
            WindowClass::Party => state(AnchorPoint::CenterLeft, MARGIN, -40.0, 320.0, 240.0),
            WindowClass::Storage => state(AnchorPoint::CenterRight, -(380.0 + MARGIN), -80.0, 360.0, 320.0),
            WindowClass::Trade => state(AnchorPoint::Center, -20.0, -80.0, 400.0, 360.0),
            WindowClass::TradeRequest => state(AnchorPoint::Center, 0.0, -40.0, 320.0, 160.0),
            WindowClass::Identify => state(AnchorPoint::Center, 0.0, -40.0, 360.0, 160.0),
            WindowClass::WarpSelection | WindowClass::WeaponRefine | WindowClass::RepairWeapon => {
                state(AnchorPoint::Center, 0.0, -80.0, 360.0, 300.0)
            }
            WindowClass::ItemActions => state(AnchorPoint::Center, 0.0, -40.0, 280.0, 220.0),
            // Dialogs / shops slightly right of center so they don't cover chat.
            WindowClass::Dialog => state(AnchorPoint::CenterRight, -(420.0 + MARGIN), -160.0, 400.0, 280.0),
            WindowClass::Buy => state(AnchorPoint::CenterRight, -(440.0 + MARGIN), -200.0, 420.0, 360.0),
            WindowClass::BuyCart => state(AnchorPoint::CenterRight, -(220.0 + MARGIN), -200.0, 200.0, 360.0),
            WindowClass::Sell => state(AnchorPoint::CenterRight, -(440.0 + MARGIN), -200.0, 420.0, 360.0),
            WindowClass::SellCart => state(AnchorPoint::CenterRight, -(220.0 + MARGIN), -200.0, 200.0, 360.0),
            WindowClass::BuyOrSell => state(AnchorPoint::Center, 0.0, -40.0, 200.0, 120.0),
            // Settings — keep near center-left.
            WindowClass::GameSettings | WindowClass::InterfaceSettings | WindowClass::GraphicsSettings | WindowClass::AudioSettings => {
                state(AnchorPoint::CenterLeft, MARGIN + 40.0, -160.0, 360.0, 400.0)
            }
            WindowClass::Respawn => state(AnchorPoint::Center, 0.0, -40.0, 280.0, 140.0),
            WindowClass::FriendRequest => state(AnchorPoint::Center, 0.0, -40.0, 360.0, 160.0),
            WindowClass::Commands => state(AnchorPoint::CenterLeft, MARGIN + 40.0, -180.0, 470.0, 520.0),
            WindowClass::Dice => state(AnchorPoint::CenterRight, -(300.0 + MARGIN), -180.0, 300.0, 360.0),
            // Login / char select stay centered (menu flow).
            WindowClass::Login | WindowClass::SelectServer | WindowClass::CharacterSelection | WindowClass::CharacterCreation => {
                return None;
            }
            #[cfg(feature = "debug")]
            WindowClass::Maps
            | WindowClass::ClientStateInspector
            | WindowClass::PacketInspector
            | WindowClass::RenderOptions
            | WindowClass::ThemeInspector
            | WindowClass::Profiler
            | WindowClass::CacheStatistics => state(AnchorPoint::TopLeft, MARGIN, 80.0, 400.0, 400.0),
        })
    }

    fn seed_defaults(&mut self) {
        // Drop still-initializing center placeholders so edge defaults apply.
        // Keep any entry the user already dragged (initializing: false), even if
        // size was never written (old bug stored 0 × MAX).
        self.entries.retain(|_, state| state.has_settled_anchor());

        for class in [
            WindowClass::Chat,
            WindowClass::Hotbar,
            WindowClass::CharacterOverview,
            WindowClass::Inventory,
            WindowClass::Equipment,
            WindowClass::Stats,
            WindowClass::SkillTree,
            WindowClass::FriendList,
            WindowClass::Minimap,
            WindowClass::StatusBar,
            WindowClass::Hud,
            WindowClass::Party,
            WindowClass::Storage,
            WindowClass::Trade,
            WindowClass::TradeRequest,
            WindowClass::Identify,
            WindowClass::WarpSelection,
            WindowClass::WeaponRefine,
            WindowClass::RepairWeapon,
            WindowClass::Dialog,
            WindowClass::Buy,
            WindowClass::BuyCart,
            WindowClass::Sell,
            WindowClass::SellCart,
            WindowClass::Menu,
        ] {
            if !self.entries.contains_key(&class)
                && let Some(default) = Self::default_for_class(class)
            {
                self.entries.insert(class, default);
            } else if let Some(entry) = self.entries.get_mut(&class)
                && !entry.has_valid_size()
                && let Some(default) = Self::default_for_class(class)
            {
                // Keep user position; repair missing size from defaults.
                entry.size = default.size;
            }
        }
    }
}

impl korangar_interface::application::WindowCache<ClientState> for WindowCache {
    fn create() -> Self {
        let mut cache = Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to load window cache from {}. creating empty cache",
                Self::FILE_NAME.magenta()
            );

            Default::default()
        });

        cache.seed_defaults();
        cache
    }

    fn get_window_state(&self, class: WindowClass) -> Option<(Anchor<ClientState>, ScreenSize)> {
        if let Some(entry) = self.entries.get(&class).filter(|s| s.has_settled_anchor()) {
            let size = if entry.has_valid_size() {
                entry.size
            } else {
                Self::default_for_class(class).map(|d| d.size).unwrap_or(ScreenSize {
                    width: 200.0,
                    height: 200.0,
                })
            };
            return Some((entry.anchor, size));
        }
        // Prefer seeded edge defaults over Anchor::default() center.
        Self::default_for_class(class).map(|state| (state.anchor, state.size))
    }

    fn register_window(&mut self, class: WindowClass, anchor: Anchor<ClientState>, size: ScreenSize) {
        // Never persist the first-open placeholder (0 × f32::MAX) — it forces re-center
        // next launch and blows window height to max.
        let placeholder = size.width < 1.0 || size.height > 10_000.0 || !size.width.is_finite() || !size.height.is_finite();
        if placeholder {
            if let Some(default) = Self::default_for_class(class) {
                self.entries.entry(class).or_insert(default);
            }
            return;
        }

        if let Some(entry) = self.entries.get_mut(&class) {
            entry.anchor = anchor;
            entry.size = size;
        } else {
            self.entries.insert(class, WindowState::new(anchor, size));
        }
    }

    fn update_anchor(&mut self, class: WindowClass, anchor: Anchor<ClientState>) {
        if let Some(entry) = self.entries.get_mut(&class) {
            entry.anchor = anchor;
        } else if let Some(mut default) = Self::default_for_class(class) {
            default.anchor = anchor;
            self.entries.insert(class, default);
        } else {
            self.entries.insert(
                class,
                WindowState::new(anchor, ScreenSize {
                    width: 200.0,
                    height: 200.0,
                }),
            );
        }
        // Persist immediately so a crash/kill still keeps the layout.
        self.save();
    }

    fn update_size(&mut self, class: WindowClass, size: ScreenSize) {
        if size.width < 1.0 || size.height > 10_000.0 {
            return;
        }
        if let Some(entry) = self.entries.get_mut(&class) {
            entry.size = size;
        } else if let Some(mut default) = Self::default_for_class(class) {
            default.size = size;
            self.entries.insert(class, default);
        } else {
            self.entries.insert(
                class,
                WindowState::new(
                    Anchor::with_point(AnchorPoint::Center, ScreenPosition { left: 0.0, top: 0.0 }),
                    size,
                ),
            );
        }
        self.save();
    }
}

impl Drop for WindowCache {
    fn drop(&mut self) {
        self.save();
    }
}
