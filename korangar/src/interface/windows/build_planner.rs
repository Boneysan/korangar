use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::build_planner::BuildPlannerStatePathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::StatKind;

/// Simulated build planner (GDD F03, §§7.4, 7.6). Every control here edits a
/// local plan; no button sends a stat or skill packet.
pub struct BuildPlannerWindow;

impl CustomWindow<ClientState> for BuildPlannerWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::BuildPlanner)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        macro_rules! level_row {
            ($text:ident, $event:ident) => {
                split! {
                    children: (
                        text! {
                            text: client_state().build_planner().$text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! { text: "−", event: InputEvent::$event { change: -1 } },
                        button! { text: "+", event: InputEvent::$event { change: 1 } },
                    ),
                }
            };
        }

        macro_rules! stat_row {
            ($text:ident, $stat:ident) => {
                split! {
                    children: (
                        text! {
                            text: client_state().build_planner().$text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! { text: "−", event: InputEvent::BuildPlannerStat { stat: StatKind::$stat, change: -1 } },
                        button! { text: "+", event: InputEvent::BuildPlannerStat { stat: StatKind::$stat, change: 1 } },
                    ),
                }
            };
        }

        macro_rules! line {
            ($text:ident) => {
                text! {
                    text: client_state().build_planner().$text(),
                    overflow_behavior: OverflowBehavior::Shrink,
                }
            };
        }

        window! {
            title: "Build Planner (simulation)",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                line!(header_text),
                level_row!(base_level_text, BuildPlannerBaseLevel),
                level_row!(job_level_text, BuildPlannerJobLevel),
                line!(points_text),
                stat_row!(strength_text, Strength),
                stat_row!(agility_text, Agility),
                stat_row!(vitality_text, Vitality),
                stat_row!(intelligence_text, Intelligence),
                stat_row!(dexterity_text, Dexterity),
                stat_row!(luck_text, Luck),
                line!(hp_sp_text),
                line!(hit_flee_text),
                line!(def_text),
                line!(atk_text),
                line!(crit_text),
                line!(cast_weight_text),
                button! { text: "Reset to current character", event: InputEvent::BuildPlannerReset },
                line!(status_text),
            ),
        }
    }
}
