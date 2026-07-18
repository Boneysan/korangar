use std::cell::{Cell, UnsafeCell};

use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector};

use crate::graphics::Color;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::skill_cooldowns::{SkillCooldowns, SkillCooldownsPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::world::{Player, PlayerPathExt};

/// Compact zeny / base-exp / job-exp / skill-cooldown readout.
pub struct HudWindow<P, C> {
    player_path: P,
    cooldowns_path: C,
}

impl<P, C> HudWindow<P, C> {
    pub fn new(player_path: P, cooldowns_path: C) -> Self {
        Self {
            player_path,
            cooldowns_path,
        }
    }
}

impl<P, C> CustomWindow<ClientState> for HudWindow<P, C>
where
    P: Path<ClientState, Player>,
    C: Path<ClientState, SkillCooldowns>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Hud)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let cooldown_text = self.cooldowns_path.display_text();

        window! {
            title: "HUD",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 220.0,
            maximum_width: 360.0,
            elements: (
                fragment! {
                    gaps: 2.0,
                    children: (
                        split! {
                            children: (
                                text! { text: "Zeny", overflow_behavior: OverflowBehavior::Shrink },
                                text! {
                                    text: PartialEqDisplaySelector::new(self.player_path.zeny()),
                                    color: Color::rgb_u8(250, 230, 130),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 2.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        split! {
                            children: (
                                text! { text: "Base EXP", overflow_behavior: OverflowBehavior::Shrink },
                                text! {
                                    text: ExpPairSelector::new(
                                        self.player_path.base_experience(),
                                        self.player_path.next_base_experience(),
                                    ),
                                    color: Color::rgb_u8(120, 220, 255),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 2.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        split! {
                            children: (
                                text! { text: "Job EXP", overflow_behavior: OverflowBehavior::Shrink },
                                text! {
                                    text: ExpPairSelector::new(
                                        self.player_path.job_experience(),
                                        self.player_path.next_job_experience(),
                                    ),
                                    color: Color::rgb_u8(180, 255, 140),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 2.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        text! {
                            text: cooldown_text,
                            color: Color::rgb_u8(255, 180, 120),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
            )
        }
    }
}

/// Formats `current / next (pct%)` experience for the HUD.
struct ExpPairSelector<A, B> {
    current: A,
    next: B,
    last: Cell<Option<(u64, u64)>>,
    text: UnsafeCell<String>,
}

impl<A, B> ExpPairSelector<A, B> {
    fn new(current: A, next: B) -> Self {
        Self {
            current,
            next,
            last: Cell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A, B> Selector<ClientState, String> for ExpPairSelector<A, B>
where
    A: Path<ClientState, u64>,
    B: Path<ClientState, u64>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let current = *self.current.follow_safe(state);
        let next = *self.next.follow_safe(state);
        let last = self.last.get();

        if last != Some((current, next)) {
            // SAFETY: text is only written here and never aliased while we hold &self.
            unsafe {
                *self.text.get() = if next == 0 {
                    format!("{current}")
                } else {
                    let pct = (current as f64 / next as f64 * 100.0).clamp(0.0, 100.0);
                    format!("{current} / {next} ({pct:.1}%)")
                };
            }
            self.last.set(Some((current, next)));
        }

        // SAFETY: see above; returns the stable string buffer for this selector
        // instance.
        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}
