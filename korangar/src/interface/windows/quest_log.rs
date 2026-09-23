use korangar_interface::element::{Element, ElementBox, ErasedElement};
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::{Resolvers, WindowLayout, with_nth_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use super::WindowClass;
use crate::graphics::Color;
use crate::input::InputEvent;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::inventory::Inventory;
use crate::state::quests::QuestLogState;
use crate::state::theme::InterfaceThemeType;

/// Dynamic quest rows, with a Track/Untrack control beside each quest.
struct QuestLogElement<A, B> {
    quest_log_path: A,
    inventory_path: B,
    rows: Vec<ElementBox<ClientState>>,
}

impl<A, B> QuestLogElement<A, B> {
    fn new(quest_log_path: A, inventory_path: B) -> Self {
        Self {
            quest_log_path,
            inventory_path,
            rows: Vec::new(),
        }
    }
}

impl<A, B> QuestLogElement<A, B>
where
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
{
    fn row_count(&self, state: &State<ClientState>) -> usize {
        let quest_log = state.get(&self.quest_log_path);
        if quest_log.is_empty() {
            return 1;
        }
        (if quest_log.quests().iter().any(|quest| !quest.requirements().is_empty()) { 1 } else { 0 })
            + quest_log.quests().iter().map(|quest| 1 + quest.requirements().len().max(1)).sum::<usize>()
    }
}

impl<A, B> Element<ClientState> for QuestLogElement<A, B>
where
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
{
    type LayoutInfo = ();

    fn get_element_count(&self, state: &State<ClientState>) -> usize {
        self.row_count(state)
    }

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        use korangar_interface::prelude::*;

        self.rows.clear();
        let quest_log = state.get(&self.quest_log_path);
        let inventory = state.get(&self.inventory_path);
        if quest_log.is_empty() {
            self.rows.push(ErasedElement::new(text! { text: "No active quests." }));
        } else {
            if quest_log.quests().iter().any(|quest| !quest.requirements().is_empty()) {
                self.rows.push(ErasedElement::new(text! {
                    text: "Counts show what you carry. The NPC also counts party members who are offline."
                }));
            }
            for quest in quest_log.quests() {
                let action = if quest_log.is_tracked(quest.quest_id) { "Untrack" } else { "Track" };
                self.rows.push(ErasedElement::new(split! {
                    children: (
                        button! {
                            text: action,
                            event: InputEvent::ToggleQuestTracking(quest.quest_id),
                        },
                        text! { text: quest.name().to_owned() },
                    ),
                }));
                if quest.requirements().is_empty() {
                    self.rows.push(ErasedElement::new(text! { text: "Speak to the quest giver." }));
                } else {
                    for requirement in quest.requirements() {
                        let carried = inventory.count_of(requirement.item_id);
                        let done = carried >= requirement.needed;
                        let color = if done { Color::rgb_u8(120, 220, 120) } else { Color::monochrome_u8(210) };
                        self.rows.push(ErasedElement::new(text! {
                            text: format!("{}  you {} / need {}", requirement.item_name, carried, requirement.needed),
                            color,
                            overflow_behavior: OverflowBehavior::Shrink,
                        }));
                    }
                }
            }
        }

        for (index, row) in self.rows.iter_mut().enumerate() {
            with_nth_resolver(resolvers, index, |resolver| {
                row.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        }
        ()
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (index, row) in self.rows.iter().enumerate() {
            row.lay_out(state, store.child_store(index as u64), &(), layout);
        }
    }
}

/// The quest log.
///
/// Campaign hunting contracts show what they want handed in and how much of it
/// the player is carrying. Before this existed the three quest packets were
/// registered and then dropped on the floor, so every quest in the campaign was
/// invisible outside NPC dialogue.
pub struct QuestLogWindow<A, B> {
    quest_log_path: A,
    inventory_path: B,
}

impl<A, B> QuestLogWindow<A, B> {
    pub fn new(quest_log_path: A, inventory_path: B) -> Self {
        Self {
            quest_log_path,
            inventory_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for QuestLogWindow<A, B>
where
    A: Path<ClientState, QuestLogState> + 'static,
    B: Path<ClientState, Inventory> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::QuestLog)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Quest Log",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                scroll_view! {
                    children: QuestLogElement::new(self.quest_log_path, self.inventory_path),
                },
            ),
        }
    }
}
