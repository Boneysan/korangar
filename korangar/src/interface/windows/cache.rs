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

    /// Is this a real, usable window size, as opposed to the placeholder
    /// written on first open before layout?
    ///
    /// **The single definition, deliberately.** This used to be stated three
    /// times — here for reads, and again inline in `register_window` and
    /// `update_size` for writes — and the writers were looser than the reader,
    /// so a size could be stored that could never be read back. `update_size`
    /// in particular compared with `<` and `>`, and **every comparison against
    /// `NaN` is false**, so a non-finite size passed straight through a guard
    /// that existed to stop it. The stored value was then unusable on read and
    /// the player's window size was silently replaced by the class default.
    /// A predicate that decides what is readable belongs in one place.
    fn is_valid_size(size: ScreenSize) -> bool {
        size.width.is_finite()
            && size.height.is_finite()
            && size.width >= 16.0
            && size.height >= 16.0
            && size.width < 10_000.0
            && size.height < 10_000.0
    }

    fn has_valid_size(&self) -> bool {
        Self::is_valid_size(self.size)
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
            // Quest log sits in the same left column as the skill tree.
            WindowClass::QuestLog => state(AnchorPoint::CenterLeft, MARGIN, -260.0, 340.0, 380.0),
            // Minimap top-right (above inventory).
            WindowClass::Minimap => state(AnchorPoint::TopRight, -(176.0 + MARGIN), MARGIN, 176.0, 210.0),
            // Buff bar top-center.
            // Effects list one per line (up to MAXIMUM_DISPLAYED_EFFECTS = 8), so 48px
            // clipped everything past the first. Sized for the full list.
            WindowClass::StatusBar => state(AnchorPoint::TopCenter, -160.0, MARGIN, 320.0, 160.0),
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
            // Same treatment as a friend request: centred, so an invite cannot
            // arrive off-screen or behind another window.
            WindowClass::PartyInvite => state(AnchorPoint::Center, 0.0, -40.0, 360.0, 160.0),
            WindowClass::AutoSpell => state(AnchorPoint::Center, 0.0, -40.0, 300.0, 320.0),
            WindowClass::Instance => state(AnchorPoint::TopRight, -MARGIN, MARGIN + 60.0, 280.0, 120.0),
            WindowClass::PlayerTarget => state(AnchorPoint::TopLeft, MARGIN, MARGIN + 120.0, 260.0, 200.0),
            // Centered error popup — wrong password / disconnect (must be visible
            // over the login form; class-less windows could open with no size).
            WindowClass::Error => state(AnchorPoint::Center, 0.0, -40.0, 360.0, 140.0),
            WindowClass::DisconnectNotice => state(AnchorPoint::Center, 0.0, -40.0, 380.0, 170.0),
            WindowClass::Commands => state(AnchorPoint::CenterLeft, MARGIN + 40.0, -180.0, 470.0, 520.0),
            // DM campaign tools: bestiary right of center, loot generator left.
            WindowClass::Bestiary => state(AnchorPoint::CenterRight, -(400.0 + MARGIN), -240.0, 380.0, 480.0),
            WindowClass::DmLoot => state(AnchorPoint::Center, -190.0, -180.0, 380.0, 360.0),
            WindowClass::Dice => state(AnchorPoint::CenterRight, -(300.0 + MARGIN), -180.0, 300.0, 360.0),
            WindowClass::Emotes => state(AnchorPoint::CenterRight, -(540.0 + MARGIN), -240.0, 520.0, 480.0),
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
            WindowClass::Emotes,
            // Centered error popup (wrong password / disconnect).
            WindowClass::Error,
            WindowClass::DisconnectNotice,
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
        if !WindowState::is_valid_size(size) {
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
        if !WindowState::is_valid_size(size) {
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

#[cfg(test)]
mod tests {
    // The three methods under test are trait implementations, so the trait has
    // to be in scope to call them at all.
    use korangar_interface::application::WindowCache as _;

    use super::*;

    fn size(width: f32, height: f32) -> ScreenSize {
        ScreenSize { width, height }
    }

    fn settled_anchor() -> Anchor<ClientState> {
        Anchor::with_point(AnchorPoint::TopLeft, ScreenPosition { left: 40.0, top: 40.0 })
    }

    /// Exactly what `save` writes and `load` reads, without touching the disk
    /// those two use.
    fn round_trip(entries: &HashMap<WindowClass, WindowState>) -> Option<HashMap<WindowClass, WindowState>> {
        let data = ron::ser::to_string_pretty(entries, PrettyConfig::new()).unwrap();
        ron::from_str(&data).ok()
    }

    /// A size the *reader* rejects must not be one the *writer* stores.
    ///
    /// "Placeholder" is decided in three places — `has_valid_size` for reads,
    /// and separate inline conditions in `register_window` and `update_size` —
    /// and the writers are looser than the reader. This pins them together from
    /// the reader's side, which is the one that decides what is usable.
    #[test]
    fn the_writers_reject_every_size_the_reader_calls_invalid() {
        let rejected = [
            ("NaN width", size(f32::NAN, 300.0)),
            ("NaN height", size(400.0, f32::NAN)),
            ("infinite height", size(400.0, f32::INFINITY)),
            ("width over the ceiling", size(50_000.0, 300.0)),
            ("below the 16px floor", size(8.0, 8.0)),
        ];

        for (what, bad) in rejected {
            assert!(
                !WindowState::new(settled_anchor(), bad).has_valid_size(),
                "{what} should be invalid"
            );

            let mut cache = WindowCache::default();
            cache
                .entries
                .insert(WindowClass::Chat, WindowState::new(settled_anchor(), size(400.0, 300.0)));
            cache.update_size(WindowClass::Chat, bad);

            let stored = cache.entries.get(&WindowClass::Chat).expect("entry vanished");
            assert!(
                stored.has_valid_size(),
                "update_size stored a {what} that the reader calls invalid: {:?}",
                stored.size
            );
        }
    }

    /// A tripwire on the file staying parseable, not a live bug.
    ///
    /// `save` is `to_string_pretty` and `load` is `from_str` followed by
    /// `.ok()`, so a value that serialises but does not parse would not cost
    /// the one window — `load` returns `None` and **every** window position the
    /// player ever set is gone. That was the theory when this was written and
    /// **it is wrong**: ron 0.12 round-trips non-finite floats, measured here
    /// rather than assumed, which is why the guard above is about a size being
    /// unreadable and not about losing the file. Kept because the blast radius
    /// if a future ron ever stops round-tripping is the whole layout.
    #[test]
    fn a_cache_entry_cannot_be_written_in_a_form_that_wipes_the_whole_file() {
        let mut entries = HashMap::new();
        entries.insert(WindowClass::Chat, WindowState::new(settled_anchor(), size(400.0, 300.0)));
        entries.insert(
            WindowClass::Inventory,
            WindowState::new(settled_anchor(), size(f32::NAN, 300.0)),
        );

        assert!(
            round_trip(&entries).is_some(),
            "a non-finite size survives serialisation but fails to parse, so `load` returns None and the whole window layout is lost"
        );
    }

    /// The first-open placeholder must never be persisted — it re-centres every
    /// window and blows its height to max on the next launch.
    #[test]
    fn the_first_open_placeholder_is_replaced_by_the_class_default() {
        let mut cache = WindowCache::default();
        cache.register_window(WindowClass::Chat, settled_anchor(), size(0.0, f32::MAX));

        let stored = cache.entries.get(&WindowClass::Chat).expect("no entry seeded for the placeholder");
        assert!(
            stored.has_valid_size(),
            "the placeholder itself was persisted: {:?}",
            stored.size
        );
    }

    /// An unsettled anchor means the user has not placed the window, so the
    /// class default wins over whatever was recorded mid-layout.
    #[test]
    fn an_unsettled_anchor_falls_back_to_the_class_default() {
        let mut cache = WindowCache::default();
        cache
            .entries
            .insert(WindowClass::Chat, WindowState::new(Anchor::default(), size(400.0, 300.0)));

        let (_, resolved) = cache.get_window_state(WindowClass::Chat).expect("Chat has a seeded default");
        let default = WindowCache::default_for_class(WindowClass::Chat).expect("Chat has a default");
        assert_eq!(resolved.width, default.size.width);
        assert_eq!(resolved.height, default.size.height);
    }
}
