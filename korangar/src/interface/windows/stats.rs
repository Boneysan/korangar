use std::cell::{Cell, UnsafeCell};

use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::StatUpType;
use rust_state::{Path, PathExt, Selector};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{Player, PlayerPathExt, StatKind, StatPreviewInput, stat_preview_tooltip};

struct StatTextSelector<A> {
    bonus_path: A,
    last_value: Cell<Option<i32>>,
    text: UnsafeCell<String>,
}

impl<A> StatTextSelector<A> {
    pub fn new(bonus_path: A) -> Self {
        Self {
            bonus_path,
            last_value: Cell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A> Selector<ClientState, String> for StatTextSelector<A>
where
    A: Path<ClientState, i32>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let bonus_value = self.bonus_path.follow_safe(state);

        unsafe {
            let last_value = self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| *last != *bonus_value) {
                *self.text.get() = format!("^000001{bonus_value:+}^000000");
                self.last_value.set(Some(*bonus_value));
            }
        }

        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

struct CostTextSelector<A> {
    cost_path: A,
    last_value: UnsafeCell<Option<u8>>,
    text: UnsafeCell<String>,
}

impl<A> CostTextSelector<A> {
    pub fn new(cost_path: A) -> Self {
        Self {
            cost_path,
            last_value: UnsafeCell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A> Selector<ClientState, String> for CostTextSelector<A>
where
    A: Path<ClientState, u8>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let cost = self.cost_path.follow_safe(state);

        unsafe {
            let last_value = &mut *self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| *last != *cost) {
                *self.text.get() = match *cost {
                    0 => "^000001max^000000".to_string(),
                    cost => format!("+1 (^000001{cost}^000000)"),
                };

                *last_value = Some(*cost);
            }
        }

        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct StatSnapshot {
    base_level: usize,
    stat_points: u32,
    stat_cost: u8,
    strength: i32,
    bonus_strength: i32,
    agility: i32,
    bonus_agility: i32,
    vitality: i32,
    bonus_vitality: i32,
    intelligence: i32,
    bonus_intelligence: i32,
    dexterity: i32,
    bonus_dexterity: i32,
    luck: i32,
    bonus_luck: i32,
    max_hp: usize,
    max_sp: usize,
}

struct StatPreviewSelector<A> {
    player_path: A,
    stat: StatKind,
    last_snapshot: Cell<Option<StatSnapshot>>,
    text: UnsafeCell<String>,
}

impl<A> StatPreviewSelector<A> {
    pub fn new(player_path: A, stat: StatKind) -> Self {
        Self {
            player_path,
            stat,
            last_snapshot: Cell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A> Selector<ClientState, String> for StatPreviewSelector<A>
where
    A: Path<ClientState, Player>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let player = self.player_path.follow_safe(state);
        let stat_cost = match self.stat {
            StatKind::Strength => player.strength_stat_points_cost,
            StatKind::Agility => player.agility_stat_points_cost,
            StatKind::Vitality => player.vitality_stat_points_cost,
            StatKind::Intelligence => player.intelligence_stat_points_cost,
            StatKind::Dexterity => player.dexterity_stat_points_cost,
            StatKind::Luck => player.luck_stat_points_cost,
        };

        let snapshot = StatSnapshot {
            base_level: player.base_level,
            stat_points: player.stat_points,
            stat_cost,
            strength: player.strength,
            bonus_strength: player.bonus_strength,
            agility: player.agility,
            bonus_agility: player.bonus_agility,
            vitality: player.vitality,
            bonus_vitality: player.bonus_vitality,
            intelligence: player.intelligence,
            bonus_intelligence: player.bonus_intelligence,
            dexterity: player.dexterity,
            bonus_dexterity: player.bonus_dexterity,
            luck: player.luck,
            bonus_luck: player.bonus_luck,
            max_hp: player.get_common().maximum_health_points,
            max_sp: player.maximum_spell_points,
        };

        unsafe {
            let last_snapshot = self.last_snapshot.get();

            if last_snapshot.is_none() || last_snapshot.as_ref().is_some_and(|last| *last != snapshot) {
                let input = StatPreviewInput {
                    base_level: snapshot.base_level,
                    strength: snapshot.strength,
                    bonus_strength: snapshot.bonus_strength,
                    agility: snapshot.agility,
                    bonus_agility: snapshot.bonus_agility,
                    vitality: snapshot.vitality,
                    bonus_vitality: snapshot.bonus_vitality,
                    intelligence: snapshot.intelligence,
                    bonus_intelligence: snapshot.bonus_intelligence,
                    dexterity: snapshot.dexterity,
                    bonus_dexterity: snapshot.bonus_dexterity,
                    luck: snapshot.luck,
                    bonus_luck: snapshot.bonus_luck,
                    max_hp: snapshot.max_hp,
                    max_sp: snapshot.max_sp,
                };

                *self.text.get() = stat_preview_tooltip(self.stat, &input, stat_cost, player.stat_points);
                self.last_snapshot.set(Some(snapshot));
            }

            Some(self.text.as_ref_unchecked())
        }
    }
}

#[derive(Default)]
pub struct StatsWindow<A> {
    player_path: A,
}

impl<A> StatsWindow<A> {
    pub fn new(player_path: A) -> Self {
        Self { player_path }
    }
}

impl<A> CustomWindow<ClientState> for StatsWindow<A>
where
    A: Path<ClientState, Player> + Copy,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Stats)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        fn disabled_cutoff<A, B>(stat_points_path: A, cost_path: B) -> impl Selector<ClientState, bool>
        where
            A: Path<ClientState, u32>,
            B: Path<ClientState, u8>,
        {
            ComputedSelector::new_default(move |state: &ClientState| {
                let stat_points = stat_points_path.follow_safe(state);
                let cost = cost_path.follow_safe(state);

                // The cost is 0 if the the player is at the maximum level.
                *cost == 0 || *stat_points < *cost as u32
            })
        }

        macro_rules! stat_row {
            ($text_name:expr, $name:ident, $bonus_name:ident, $cost_name:ident, $variant_name:ident) => {
                split! {
                    children: (
                        text! {
                            text: client_state().localization().$text_name(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        split! {
                            children: (
                                text! {
                                    text: PartialEqDisplaySelector::new(self.player_path.$name()),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 5.0, border: 5.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: StatTextSelector::new(self.player_path.$bonus_name()),
                                    horizontal_alignment: HorizontalAlignment::Left { offset: 5.0, border: 5.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        button! {
                            text: CostTextSelector::new(self.player_path.$cost_name()),
                            tooltip: StatPreviewSelector::new(self.player_path, StatKind::$variant_name),
                            disabled: disabled_cutoff(self.player_path.stat_points(), self.player_path.$cost_name()),
                            disabled_tooltip: StatPreviewSelector::new(self.player_path, StatKind::$variant_name),
                            event: InputEvent::StatUp { stat_type: StatUpType::$variant_name { amount: 1 } },
                        },
                    ),
                }
            };
        }

        window! {
            title: client_state().localization().stats_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    children: (
                        text! {
                            text: client_state().localization().available_stat_points_text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: PartialEqDisplaySelector::new(self.player_path.stat_points()),
                            horizontal_alignment: HorizontalAlignment::Right { offset: 5.0, border: 5.0 },
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
                stat_row!(strength_text, strength, bonus_strength, strength_stat_points_cost, Strength),
                stat_row!(agility_text, agility, bonus_agility, agility_stat_points_cost, Agility),
                stat_row!(vitality_text, vitality, bonus_vitality, vitality_stat_points_cost, Vitality),
                stat_row!(intelligence_text, intelligence, bonus_intelligence, intelligence_stat_points_cost, Intelligence),
                stat_row!(dexterity_text, dexterity, bonus_dexterity, dexterity_stat_points_cost, Dexterity),
                stat_row!(luck_text, luck, bonus_luck, luck_stat_points_cost, Luck),
                button! {
                    text: "Build planner",
                    tooltip: "Simulate future levels and stat allocations. Nothing is sent to the server.",
                    event: InputEvent::ToggleBuildPlannerWindow,
                },
            ),
        }
    }
}
