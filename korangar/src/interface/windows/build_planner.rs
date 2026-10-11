use std::cmp::Ordering;

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::build_planner::{BuildPlannerStatePathExt, PlannerSkillRow, PlannerSkillRowPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::StatKind;

/// One row per skill of the job's tree, built the first time the list sees
/// that many rows. The skill id is read when the row is built, so a window
/// always shows the tree of the job it was opened for.
struct SkillList<A> {
    rows_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> SkillList<A> {
    fn new(rows_path: A) -> Self {
        Self {
            rows_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for SkillList<A>
where
    A: Path<ClientState, Vec<PlannerSkillRow>>,
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

            let rows = state.get(&self.rows_path);

            match rows.len().cmp(&self.elements.len()) {
                Ordering::Less => self.elements.truncate(rows.len()),
                Ordering::Equal => {}
                Ordering::Greater => {
                    for (index, planned_row) in rows.iter().enumerate().skip(self.elements.len()) {
                        let row_path = self.rows_path.index(index).manually_asserted();
                        let skill_id = planned_row.skill_id;

                        let row = split! {
                            children: (
                                text! {
                                    text: row_path.text(),
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                button! { text: "−", event: InputEvent::BuildPlannerSkill { skill_id, change: -1 } },
                                button! { text: "+", event: InputEvent::BuildPlannerSkill { skill_id, change: 1 } },
                            ),
                        };

                        self.elements.push(ErasedElement::new(row));
                    }
                }
            }

            self.elements.iter_mut().enumerate().for_each(|(index, element)| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}

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
            title: "Build Planner",
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
                collapsible! {
                    text: "Results",
                    initially_expanded: true,
                    children: (
                        line!(hp_sp_text),
                        line!(hit_flee_text),
                        line!(def_text),
                        line!(atk_text),
                        line!(crit_text),
                        line!(cast_weight_text),
                    ),
                },
                collapsible! {
                    text: "Skills",
                    initially_expanded: false,
                    children: (
                        scroll_view! {
                    children: SkillList::new(client_state().build_planner().skill_rows()),
                },
                    ),
                },
                collapsible! {
                    text: "Saved builds",
                    initially_expanded: false,
                    children: (
                        split! {
                    children: (
                        button! { text: "Save 1", event: InputEvent::BuildPlannerSave { slot: 1 } },
                        button! { text: "Save 2", event: InputEvent::BuildPlannerSave { slot: 2 } },
                        button! { text: "Save 3", event: InputEvent::BuildPlannerSave { slot: 3 } },
                    ),
                },
                        split! {
                    children: (
                        button! { text: "Load 1", event: InputEvent::BuildPlannerLoad { slot: 1 } },
                        button! { text: "Load 2", event: InputEvent::BuildPlannerLoad { slot: 2 } },
                        button! { text: "Load 3", event: InputEvent::BuildPlannerLoad { slot: 3 } },
                    ),
                },
                        button! { text: "Reset to current character", event: InputEvent::BuildPlannerReset },
                    ),
                },
                line!(status_text),
            ),
        }
    }
}
