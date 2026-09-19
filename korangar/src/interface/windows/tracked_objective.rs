use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::*;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::ClientState;
use crate::state::breadcrumb::BreadcrumbState;
use crate::state::inventory::Inventory;
use crate::state::navigation::NavigationState;
use crate::state::quests::QuestLogState;
use crate::state::theme::InterfaceThemeType;
use crate::this_entity;

struct TrackedObjectiveElement<Q, A, B, N> {
    breadcrumb_path: Q,
    quest_log_path: A,
    inventory_path: B,
    navigation_path: N,
    elements: Vec<(u64, ElementBox<ClientState>)>,
}

impl<Q, A, B, N> Element<ClientState> for TrackedObjectiveElement<Q, A, B, N>
where
    Q: Path<ClientState, BreadcrumbState>,
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
    N: Path<ClientState, NavigationState>,
{
    type LayoutInfo = ();

    fn create_layout_info(&mut self, state: &State<ClientState>, mut store: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) {
        use korangar_interface::prelude::*;
        with_single_resolver(resolvers, |resolver| {
            self.elements.clear();
            let log = state.get(&self.quest_log_path);
            let inventory = state.get(&self.inventory_path);
            let breadcrumb = state.get(&self.breadcrumb_path);
            let navigation = state.get(&self.navigation_path);
            let player_tile = state.try_follow(this_entity()).map(|player| player.get_tile_position());
            let ui_scale = f32::from(breadcrumb.scale) / 100.0;
            let opacity = f32::from(breadcrumb.opacity) / 100.0;

            if !breadcrumb.hidden
                && let Some(tracked_id) = log.tracked()
                && let Some(quest) = log.quests().iter().find(|q| q.quest_id == tracked_id)
            {
                self.elements.push((
                    1,
                    ErasedElement::new(text! {
                        text: format!("★ {}", quest.name()),
                        font_size: FontSize(15.0 * ui_scale),
                        color: Color::rgb_u8(255, 220, 130).multiply_alpha(opacity),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    }),
                ));

                if !breadcrumb.collapsed {
                    let ready = quest.objectives_ready(|id| inventory.count_of(id));
                    let summary = if ready {
                        "✔ Objectives ready for the next step!".to_string()
                    } else if !quest.kill_objectives().is_empty() {
                        quest
                            .kill_objectives()
                            .iter()
                            .map(|objective| {
                                let name = if objective.mob_id != 0 {
                                    crate::world::display_monster(objective.mob_id, "normal").to_string()
                                } else {
                                    format!("objective {}", objective.objective_id)
                                };
                                format!("Defeat {name} ({}/{})", objective.current, objective.total)
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    } else if !quest.requirements().is_empty() {
                        let mut parts = Vec::new();
                        for req in quest.requirements() {
                            let have = inventory.count_of(req.item_id);
                            parts.push(format!("{} ({}/{})", req.item_name, have, req.needed));
                        }
                        parts.join(", ")
                    } else if !quest.location.is_empty() {
                        quest.location.clone()
                    } else {
                        "Follow quest instructions".to_string()
                    };

                    let summary_color = if ready {
                        Color::rgb_u8(150, 230, 170)
                    } else {
                        Color::monochrome_u8(235)
                    };

                    self.elements.push((
                        2,
                        ErasedElement::new(text! {
                            text: summary,
                            font_size: FontSize(13.0 * ui_scale),
                            color: summary_color.multiply_alpha(opacity),
                            overflow_behavior: OverflowBehavior::LineBreak,
                        }),
                    ));

                    if !quest.location.is_empty() && !ready {
                        self.elements.push((
                            3,
                            ErasedElement::new(text! {
                                text: quest.location.clone(),
                                font_size: FontSize(12.0 * ui_scale),
                                color: Color::rgb_u8(180, 210, 255).multiply_alpha(opacity),
                                overflow_behavior: OverflowBehavior::LineBreak,
                            }),
                        ));
                    }

                    if let Some(player) = player_tile
                        && let Some((target_x, target_y)) = breadcrumb.target(&breadcrumb.map)
                    {
                        let distance = breadcrumb.distance(&breadcrumb.map, player.x, player.y).unwrap_or_default();
                        let direction = breadcrumb.direction(&breadcrumb.map, player.x, player.y).unwrap_or("•");
                        self.elements.push((
                            5,
                            ErasedElement::new(text! {
                                text: format!("{} {} tiles · {},{}", direction, distance, target_x, target_y),
                                font_size: FontSize(12.0 * ui_scale),
                                color: Color::rgb_u8(255, 210, 120).multiply_alpha(opacity),
                                overflow_behavior: OverflowBehavior::Shrink,
                            }),
                        ));
                    }

                    if navigation.available && !navigation.next_map.is_empty() {
                        self.elements.push((
                            13,
                            ErasedElement::new(text! {
                                text: format!(
                                    "Portal: {},{} → {} · {} map{}",
                                    navigation.next_portal_x.unwrap_or_default(),
                                    navigation.next_portal_y.unwrap_or_default(),
                                    navigation.next_map,
                                    navigation.route_maps.len().saturating_sub(1),
                                    if navigation.route_maps.len() == 2 { "" } else { "s" },
                                ),
                                font_size: FontSize(11.0 * ui_scale),
                                color: Color::rgb_u8(180, 230, 210).multiply_alpha(opacity),
                                overflow_behavior: OverflowBehavior::LineBreak,
                            }),
                        ));
                    }
                }

                self.elements.push((
                    4,
                    ErasedElement::new(button! {
                        text: "Journal (Ctrl+Q)",
                        tooltip: "Open Quest Journal [^000001Ctrl+Q^000000]",
                        height: 24.0,
                        font_size: FontSize(13.0 * ui_scale),
                        event: InputEvent::ToggleQuestLogWindow,
                    }),
                ));
            } else {
                self.elements.push((
                    1,
                    ErasedElement::new(text! {
                        text: "No objective tracked",
                        font_size: FontSize(14.0),
                        color: Color::monochrome_u8(180),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    }),
                ));
                self.elements.push((
                    2,
                    ErasedElement::new(button! {
                        text: if breadcrumb.hidden { "Show Breadcrumb" } else { "Open Journal (Ctrl+Q)" },
                        tooltip: "Show the tracked objective HUD",
                        height: 24.0,
                        font_size: FontSize(13.0 * ui_scale),
                        event: if breadcrumb.hidden { InputEvent::ToggleBreadcrumbHidden } else { InputEvent::ToggleQuestLogWindow },
                    }),
                ));
            }

            if !breadcrumb.hidden {
                self.elements.push((
                    6,
                    ErasedElement::new(button! {
                        text: if breadcrumb.collapsed { "Expand" } else { "Collapse" },
                        tooltip: "Collapse or expand objective details",
                        height: 22.0,
                        font_size: FontSize(12.0 * ui_scale),
                        event: InputEvent::ToggleBreadcrumbCollapsed,
                    }),
                ));
                self.elements.push((
                    7,
                    ErasedElement::new(button! {
                        text: format!("Size {}% +", breadcrumb.scale),
                        tooltip: "Increase breadcrumb size",
                        height: 22.0,
                        font_size: FontSize(12.0 * ui_scale),
                        event: InputEvent::BreadcrumbScale { delta: 10 },
                    }),
                ));
                self.elements.push((
                    8,
                    ErasedElement::new(button! {
                        text: format!("Size {}% −", breadcrumb.scale),
                        tooltip: "Decrease breadcrumb size",
                        height: 22.0,
                        font_size: FontSize(12.0 * ui_scale),
                        event: InputEvent::BreadcrumbScale { delta: -10 },
                    }),
                ));
                self.elements.push((
                    9,
                    ErasedElement::new(button! {
                        text: format!("Opacity {}% −", breadcrumb.opacity),
                        tooltip: "Decrease breadcrumb opacity",
                        height: 22.0,
                        font_size: FontSize(12.0 * ui_scale),
                        event: InputEvent::BreadcrumbOpacity { delta: -10 },
                    }),
                ));
                self.elements.push((
                    10,
                    ErasedElement::new(button! {
                        text: format!("Opacity {}% +", breadcrumb.opacity),
                        tooltip: "Increase breadcrumb opacity",
                        height: 22.0,
                        font_size: FontSize(12.0 * ui_scale),
                        event: InputEvent::BreadcrumbOpacity { delta: 10 },
                    }),
                ));
                self.elements.push((
                    11,
                    ErasedElement::new(button! {
                        text: if breadcrumb.guidance_enabled { "Guidance: On" } else { "Guidance: Off" },
                        tooltip: "Toggle same-map direction, distance, and minimap marker",
                        height: 22.0,
                        font_size: FontSize(12.0 * ui_scale),
                        event: InputEvent::ToggleBreadcrumbGuidance,
                    }),
                ));
                self.elements.push((
                    12,
                    ErasedElement::new(button! {
                        text: "Hide",
                        tooltip: "Hide the tracked objective HUD",
                        height: 22.0,
                        font_size: FontSize(12.0 * ui_scale),
                        event: InputEvent::ToggleBreadcrumbHidden,
                    }),
                ));
            }

            for (id, element) in &mut self.elements {
                element.create_layout_info(state, store.child_store(*id), resolver);
            }
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a (),
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (id, element) in &self.elements {
            element.lay_out(state, store.child_store(*id), &(), layout);
        }
    }
}

pub struct TrackedObjectiveWindow<A, B, C, D> {
    breadcrumb_path: A,
    quest_log_path: B,
    inventory_path: C,
    navigation_path: D,
}

impl<A, B, C, D> TrackedObjectiveWindow<A, B, C, D> {
    pub fn new(breadcrumb_path: A, quest_log_path: B, inventory_path: C, navigation_path: D) -> Self {
        Self {
            breadcrumb_path,
            quest_log_path,
            inventory_path,
            navigation_path,
        }
    }
}

impl<A, B, C, D> CustomWindow<ClientState> for TrackedObjectiveWindow<A, B, C, D>
where
    A: Path<ClientState, BreadcrumbState> + 'static,
    B: Path<ClientState, QuestLogState> + 'static,
    C: Path<ClientState, Inventory> + 'static,
    D: Path<ClientState, NavigationState> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::TrackedObjective)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        window! {
            title: "Tracked Objective",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 180.0,
            maximum_width: 280.0,
            elements: (
                TrackedObjectiveElement {
                    breadcrumb_path: self.breadcrumb_path,
                    quest_log_path: self.quest_log_path,
                    inventory_path: self.inventory_path,
                    navigation_path: self.navigation_path,
                    elements: Vec::new(),
                },
            )
        }
    }
}
