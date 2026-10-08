use std::cell::{Cell, UnsafeCell};

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector, State};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::party::PartyStatePathExt;
use crate::state::recovery::RecoveryStatePathExt;
use crate::state::skill_cooldowns::{SkillCooldowns, SkillCooldownsPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::toasts::ToastQueuePathExt;
use crate::world::{CommonPathExt, Player, PlayerPathExt};

/// Compact zeny / base-exp / job-exp / skill-cooldown readout.
pub struct HudWindow<P, C, T, Q, G, R> {
    player_path: P,
    cooldowns_path: C,
    toasts_path: T,
    quests_path: Q,
    party_path: G,
    recovery_path: R,
}

impl<P, C, T, Q, G, R> HudWindow<P, C, T, Q, G, R> {
    pub fn new(player_path: P, cooldowns_path: C, toasts_path: T, quests_path: Q, party_path: G, recovery_path: R) -> Self {
        Self {
            player_path,
            cooldowns_path,
            toasts_path,
            quests_path,
            party_path,
            recovery_path,
        }
    }
}

impl<P, C, T, Q, G, R> CustomWindow<ClientState> for HudWindow<P, C, T, Q, G, R>
where
    P: Path<ClientState, Player>,
    C: Path<ClientState, SkillCooldowns>,
    T: Path<ClientState, crate::state::toasts::ToastQueue>,
    Q: Path<ClientState, crate::state::quests::QuestLogState>,
    G: Path<ClientState, crate::state::party::PartyState>,
    R: Path<ClientState, crate::state::recovery::RecoveryState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Hud)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let cooldown_text = self.cooldowns_path.display_text();
        let toast_text = self.toasts_path.display_text();
        let goals_text = self.party_path.goals_hud_text();
        let recovery_status = self.recovery_path.display_text();

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
                                text! { text: "Level", overflow_behavior: OverflowBehavior::Shrink },
                                text! {
                                    text: PartialEqDisplaySelector::new(self.player_path.base_level()),
                                    color: Color::rgb_u8(230, 230, 230),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 2.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
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
                            text: recovery_status,
                            color: Color::rgb_u8(160, 230, 190),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: cooldown_text,
                            color: Color::rgb_u8(255, 180, 120),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        TrackedQuestHud {
                            quests_path: self.quests_path,
                            fingerprint: Vec::new(),
                            elements: Vec::new(),
                        },
                        text! {
                            text: goals_text,
                            color: Color::rgb_u8(200, 255, 200),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: toast_text,
                            color: Color::rgb_u8(255, 230, 160),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
            )
        }
    }
}

/// Tracked quests on the HUD. The first is marked Primary. Navigate uses the
/// quest's NPC cell when one is known; otherwise the row says there is no map
/// location. Untrack is the same control as the quest log.
struct TrackedQuestHud<Q> {
    quests_path: Q,
    fingerprint: Vec<(u32, String, bool)>,
    elements: Vec<ElementBox<ClientState>>,
}

impl<Q> Element<ClientState> for TrackedQuestHud<Q>
where
    Q: Path<ClientState, crate::state::quests::QuestLogState>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;

            let rows = state.get(&self.quests_path).hud_rows();
            let fingerprint: Vec<_> = rows
                .iter()
                .map(|row| (row.quest_id, row.title.clone(), row.map_name.is_some()))
                .collect();
            if fingerprint != self.fingerprint {
                self.elements.clear();
                if rows.is_empty() {
                    self.elements.push(ErasedElement::new(text! {
                        text: "No tracked quest",
                        color: Color::rgb_u8(140, 200, 255),
                        overflow_behavior: OverflowBehavior::Shrink,
                    }));
                }
                for row in rows {
                    let quest_id = row.quest_id;
                    let title_color = if row.is_primary {
                        Color::rgb_u8(255, 220, 120)
                    } else {
                        Color::rgb_u8(140, 200, 255)
                    };
                    self.elements.push(ErasedElement::new(text! {
                        text: row.title.clone(),
                        color: title_color,
                        overflow_behavior: OverflowBehavior::LineBreak,
                    }));
                    match row.map_name.clone() {
                        Some(map_name) => {
                            let x = row.x;
                            let y = row.y;
                            self.elements.push(ErasedElement::new(button! {
                                text: "Navigate",
                                tooltip: "Show the route to this quest.",
                                event: InputEvent::SetNavigationDestination { map_name, x, y },
                            }));
                        }
                        None => self.elements.push(ErasedElement::new(text! {
                            text: "No map location",
                            color: Color::rgb_u8(160, 160, 160),
                            overflow_behavior: OverflowBehavior::Shrink,
                        })),
                    }
                    self.elements.push(ErasedElement::new(button! {
                        text: "Untrack",
                        tooltip: "Take this quest off the HUD. Track it again from the quest log.",
                        event: InputEvent::ToggleQuestTracking(quest_id),
                    }));
                }
                self.fingerprint = fingerprint;
            }

            for (index, element) in self.elements.iter_mut().enumerate() {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            }
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (index, element) in self.elements.iter().enumerate() {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
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
                    // The server sends a next-level requirement of 0 at max
                    // level, which rendered as a bare number with no total and
                    // no percentage — indistinguishable from the value simply
                    // being missing, which is how it was first reported.
                    format!("{current} (MAX)")
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
