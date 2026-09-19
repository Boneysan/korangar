use std::cell::{Cell, UnsafeCell};

use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector};

use crate::graphics::Color;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::skill_cooldowns::{SkillCooldowns, SkillCooldownsPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::world::{CommonPathExt, Player, PlayerPathExt};

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
                        // HP and SP first: they change fastest and are what the
                        // player checks under pressure. Both were missing entirely
                        // — the client drew no numeric vitals anywhere, so the only
                        // way to read your own health was the sprite's bar.
                        split! {
                            children: (
                                text! { text: "HP", overflow_behavior: OverflowBehavior::Shrink },
                                text! {
                                    text: VitalPairSelector::new(
                                        self.player_path.common().health_points(),
                                        self.player_path.common().maximum_health_points(),
                                    ),
                                    color: Color::rgb_u8(255, 120, 120),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 2.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        split! {
                            children: (
                                text! { text: "SP", overflow_behavior: OverflowBehavior::Shrink },
                                text! {
                                    text: VitalPairSelector::new(
                                        self.player_path.spell_points(),
                                        self.player_path.maximum_spell_points(),
                                    ),
                                    color: Color::rgb_u8(120, 170, 255),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 2.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
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
                        text! {
                            text: self.player_path.recovery_status(),
                            color: Color::rgb_u8(200, 220, 160),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
            )
        }
    }
}

/// Formats `current / maximum (pct%)` for HP and SP.
///
/// Separate from [`ExpPairSelector`] because these are `usize` rather than
/// `u64`, and because a maximum of zero means "not known yet" here rather than
/// "already at the cap" — showing `(MAX)` for an unpopulated maximum would be
/// an outright lie about the player's health.
struct VitalPairSelector<A, B> {
    current: A,
    maximum: B,
    last: Cell<Option<(usize, usize)>>,
    text: UnsafeCell<String>,
}

impl<A, B> VitalPairSelector<A, B> {
    fn new(current: A, maximum: B) -> Self {
        Self {
            current,
            maximum,
            last: Cell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A, B> Selector<ClientState, String> for VitalPairSelector<A, B>
where
    A: Path<ClientState, usize>,
    B: Path<ClientState, usize>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let current = *self.current.follow_safe(state);
        let maximum = *self.maximum.follow_safe(state);
        let last = self.last.get();

        if last != Some((current, maximum)) {
            // SAFETY: text is only written here and never aliased while we hold &self.
            unsafe {
                *self.text.get() = match maximum {
                    0 => format!("{current}"),
                    maximum => {
                        let percent = (current as f64 / maximum as f64 * 100.0).clamp(0.0, 100.0);
                        format!("{current} / {maximum} ({percent:.0}%)")
                    }
                };
            }
            self.last.set(Some((current, maximum)));
        }

        // SAFETY: see above; returns the stable string buffer for this selector
        // instance.
        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

pub fn format_exp_gain_toast(amount: u64, kind: &str, quest: bool) -> String {
    if quest {
        format!("Gained {amount} {kind} EXP (quest)")
    } else {
        format!("Gained {amount} {kind} EXP")
    }
}

pub fn format_exp_hud_pair(current: u64, next: u64) -> String {
    if next == 0 {
        format!("{current} (MAX)")
    } else {
        let pct = (current as f64 / next as f64 * 100.0).clamp(0.0, 100.0);
        format!("{current} / {next} ({pct:.1}%)")
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
                *self.text.get() = format_exp_hud_pair(current, next);
            }
            self.last.set(Some((current, next)));
        }

        // SAFETY: see above; returns the stable string buffer for this selector
        // instance.
        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

#[cfg(test)]
mod recovery_status_tests {
    use super::{format_exp_gain_toast, format_exp_hud_pair};
    use crate::world::format_recovery_status;

    #[test]
    fn recovery_status_names_active_and_blocked_states() {
        assert_eq!(format_recovery_status(1, 0), "Standing recovery");
        assert_eq!(format_recovery_status(2, 0), "Sitting recovery: 25% HP/SP every 10s");
        assert_eq!(format_recovery_status(3, 0), "Respawn recovery: filling remaining HP/SP");
        assert_eq!(format_recovery_status(0, 0), "Recovery idle");
        assert_eq!(format_recovery_status(1, 3), "Recovery blocked: overweight");
        assert_eq!(format_recovery_status(1, 2), "Recovery blocked: status");
        assert_eq!(format_recovery_status(1, 1), "Recovery blocked: dead");
        assert_eq!(format_recovery_status(1, 4), "Standing recovery paused: in combat");
    }

    #[test]
    fn weight_bands_match_approved_70_and_90() {
        use crate::world::{weight_is_soft, weight_is_warn};

        assert!(!weight_is_warn(69, 100));
        assert!(weight_is_warn(70, 100));
        assert!(weight_is_warn(89, 100));
        assert!(!weight_is_soft(89, 100));
        assert!(weight_is_soft(90, 100));
        assert!(weight_is_soft(99, 100));
        assert!(weight_is_soft(100, 100));
    }

    #[test]
    fn quest_award_toast_and_hud_rollover_match_1000_and_500() {
        assert_eq!(format_exp_gain_toast(1000, "Base", true), "Gained 1000 Base EXP (quest)");
        assert_eq!(format_exp_gain_toast(500, "Job", true), "Gained 500 Job EXP (quest)");
        assert_eq!(format_exp_hud_pair(1000, 2000), "1000 / 2000 (50.0%)");
        assert_eq!(format_exp_hud_pair(1500, 2000), "1500 / 2000 (75.0%)");
        assert_eq!(format_exp_hud_pair(2000, 0), "2000 (MAX)");
    }
}
