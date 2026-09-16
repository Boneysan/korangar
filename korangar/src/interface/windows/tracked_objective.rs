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
use crate::state::quests::QuestLogState;
use crate::state::theme::InterfaceThemeType;

struct TrackedObjectiveElement<A, B> {
    quest_log_path: A,
    inventory_path: B,
    elements: Vec<(u64, ElementBox<ClientState>)>,
}

impl<A, B> Element<ClientState> for TrackedObjectiveElement<A, B>
where
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
{
    type LayoutInfo = ();

    fn create_layout_info(&mut self, state: &State<ClientState>, mut store: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) {
        use korangar_interface::prelude::*;
        with_single_resolver(resolvers, |resolver| {
            self.elements.clear();
            let log = state.get(&self.quest_log_path);
            let inventory = state.get(&self.inventory_path);

            if let Some(tracked_id) = log.tracked()
                && let Some(quest) = log.quests().iter().find(|q| q.quest_id == tracked_id)
            {
                let ready = quest.items_ready(|id| inventory.count_of(id));

                self.elements.push((
                    1,
                    ErasedElement::new(text! {
                        text: format!("★ {}", quest.name()),
                        font_size: FontSize(15.0),
                        color: Color::rgb_u8(255, 220, 130),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    }),
                ));

                let summary = if ready {
                    "✔ Items ready to turn in!".to_string()
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
                        font_size: FontSize(13.0),
                        color: summary_color,
                        overflow_behavior: OverflowBehavior::LineBreak,
                    }),
                ));

                if !quest.location.is_empty() && !ready {
                    self.elements.push((
                        3,
                        ErasedElement::new(text! {
                            text: quest.location.clone(),
                            font_size: FontSize(12.0),
                            color: Color::rgb_u8(180, 210, 255),
                            overflow_behavior: OverflowBehavior::LineBreak,
                        }),
                    ));
                }

                self.elements.push((
                    4,
                    ErasedElement::new(button! {
                        text: "Journal (Ctrl+Q)",
                        tooltip: "Open Quest Journal [^000001Ctrl+Q^000000]",
                        height: 24.0,
                        font_size: FontSize(13.0),
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
                        text: "Open Journal (Ctrl+Q)",
                        tooltip: "Choose an objective to track in the Quest Journal [^000001Ctrl+Q^000000]",
                        height: 24.0,
                        font_size: FontSize(13.0),
                        event: InputEvent::ToggleQuestLogWindow,
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

pub struct TrackedObjectiveWindow<A, B, C> {
    _breadcrumb_path: A,
    quest_log_path: B,
    inventory_path: C,
}

impl<A, B, C> TrackedObjectiveWindow<A, B, C> {
    pub fn new(breadcrumb_path: A, quest_log_path: B, inventory_path: C) -> Self {
        Self {
            _breadcrumb_path: breadcrumb_path,
            quest_log_path,
            inventory_path,
        }
    }
}

impl<A, B, C> CustomWindow<ClientState> for TrackedObjectiveWindow<A, B, C>
where
    A: Path<ClientState, BreadcrumbState> + 'static,
    B: Path<ClientState, QuestLogState> + 'static,
    C: Path<ClientState, Inventory> + 'static,
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
                    quest_log_path: self.quest_log_path,
                    inventory_path: self.inventory_path,
                    elements: Vec::new(),
                },
            )
        }
    }
}
