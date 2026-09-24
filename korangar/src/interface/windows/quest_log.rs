use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
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
        (if quest_log.quests().iter().any(|quest| !quest.requirements().is_empty()) {
            1
        } else {
            0
        }) + quest_log
            .quests()
            .iter()
            .map(|quest| {
                let fallback = usize::from(quest.requirements().is_empty() && quest.hunt_objectives().is_empty());
                1 + quest
                    .requirements()
                    .iter()
                    .map(|requirement| 1 + requirement_source_routes(requirement.item_id.0).len())
                    .sum::<usize>()
                    + fallback
                    + quest
                        .hunt_objectives()
                        .iter()
                        .map(|objective| 1 + hunt_spawn_maps(objective.monster_id).len())
                        .sum::<usize>()
            })
            .sum::<usize>()
    }
}

fn hunt_spawn_maps(monster_id: u32) -> Vec<String> {
    crate::dm::reference_data::reference_data()
        .monster_by_id(monster_id)
        .map(|monster| {
            monster
                .spawn_regions
                .iter()
                .filter(|region| {
                    crate::world::navigation_graph()
                        .maps
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(&region.map))
                })
                .take(3)
                .map(|region| region.map.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Broad routes inferred only through exported item-drop and static spawn
/// records. No route is shown when either link is unavailable.
fn requirement_source_routes(item_id: u32) -> Vec<(String, String)> {
    let data = crate::dm::reference_data::reference_data();
    let Some(item) = data.item_by_id(item_id).or_else(|| data.card_by_id(item_id)) else {
        return Vec::new();
    };
    let mut routes: Vec<(String, String)> = Vec::new();
    for source in &item.drops_from {
        let Some(monster) = data.monster_by_id(source.monster_id) else {
            continue;
        };
        let monster_name = if monster.name.is_empty() {
            monster.sprite_name.clone()
        } else {
            monster.name.clone()
        };
        for region in &monster.spawn_regions {
            if !crate::world::navigation_graph()
                .maps
                .iter()
                .any(|known| known.eq_ignore_ascii_case(&region.map))
            {
                continue;
            }
            if routes.iter().any(|(_, map)| map.eq_ignore_ascii_case(&region.map)) {
                continue;
            }
            routes.push((monster_name.clone(), region.map.clone()));
            if routes.len() == 3 {
                return routes;
            }
        }
    }
    routes
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
                if quest.requirements().is_empty() && quest.hunt_objectives().is_empty() {
                    self.rows
                        .push(ErasedElement::new(text! { text: "No objective details are available yet." }));
                } else {
                    for requirement in quest.requirements() {
                        let carried = inventory.count_of(requirement.item_id);
                        let done = carried >= requirement.needed;
                        let color = if done {
                            Color::rgb_u8(120, 220, 120)
                        } else {
                            Color::monochrome_u8(210)
                        };
                        self.rows.push(ErasedElement::new(split! {
                            children: (
                                text! {
                                    text: format!("{}  you {} / need {}", requirement.item_name, carried, requirement.needed),
                                    color,
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                button! {
                                    text: "Guide",
                                    tooltip: "Open this item’s verified drop sources and known maps.",
                                    event: InputEvent::OpenAdventureGuideItem { item_id: requirement.item_id.0 },
                                },
                            ),
                        }));
                        for (monster_name, map_name) in requirement_source_routes(requirement.item_id.0) {
                            self.rows.push(ErasedElement::new(button! {
                                text: format!("Route to {map_name} ({monster_name})"),
                                tooltip: "Broad static source map for an item drop; exact spawn cells are not shown.",
                                event: InputEvent::SetNavigationMapDestination { map_name },
                            }));
                        }
                    }
                }
                for objective in quest.hunt_objectives() {
                    self.rows.push(ErasedElement::new(text! {
                        text: format!(
                            "Hunt {}  {} / {}",
                            objective.monster_name, objective.current_count, objective.total_count
                        ),
                        overflow_behavior: OverflowBehavior::Shrink,
                    }));
                    for map_name in hunt_spawn_maps(objective.monster_id) {
                        self.rows.push(ErasedElement::new(button! {
                            text: format!("Route to {map_name}"),
                            tooltip: "Broad static spawn region; exact spawn cells are not shown.",
                            event: InputEvent::SetNavigationMapDestination { map_name },
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

#[cfg(test)]
mod tests {
    use super::{hunt_spawn_maps, requirement_source_routes};

    #[test]
    fn item_turn_in_routes_use_only_verified_drop_sources_and_graph_maps() {
        let data = crate::dm::reference_data::reference_data();
        let item = data
            .items
            .iter()
            .find(|item| {
                item.drops_from.iter().any(|source| {
                    data.monster_by_id(source.monster_id)
                        .is_some_and(|monster| !monster.spawn_regions.is_empty())
                })
            })
            .expect("item with a verified drop-source map");
        let routes = requirement_source_routes(item.id);
        assert!(!routes.is_empty());
        assert!(routes.len() <= 3);
        assert!(routes.iter().all(|(monster, map)| {
            !monster.is_empty()
                && crate::world::navigation_graph()
                    .maps
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(map))
        }));
        assert!(requirement_source_routes(u32::MAX).is_empty());
    }

    #[test]
    fn hunt_objective_routes_exclude_maps_outside_the_navigation_graph() {
        let data = crate::dm::reference_data::reference_data();
        for monster in data.monsters.iter().filter(|monster| !monster.spawn_regions.is_empty()).take(100) {
            assert!(hunt_spawn_maps(monster.id).iter().all(|map| {
                crate::world::navigation_graph()
                    .maps
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(map))
            }));
        }
    }
}
