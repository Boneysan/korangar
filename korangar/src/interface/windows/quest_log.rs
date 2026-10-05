use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox, StateElement};
use korangar_interface::layout::{Resolvers, WindowLayout, with_nth_resolver};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::ItemId;
use rust_state::{Path, PathExt, RustState, State};

use super::WindowClass;
use crate::graphics::Color;
use crate::input::InputEvent;
use crate::loaders::OverflowBehavior;
use crate::state::dm_journal::DmJournalState;
use crate::state::inventory::Inventory;
use crate::state::quests::{QuestEntry, QuestLocationEntry, QuestLogState};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// The quest log's tabs. Story quests and hunts are told apart by data, not
/// by name: only the campaign's hunting-contract table supplies item
/// turn-ins (`resolve_quest_entry`), and only hunting quests carry kill
/// counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub enum QuestLogTab {
    #[default]
    Quests,
    Hunts,
    Goals,
    Clues,
}

impl QuestLogTab {
    const ALL: [QuestLogTab; 4] = [QuestLogTab::Quests, QuestLogTab::Hunts, QuestLogTab::Goals, QuestLogTab::Clues];

    fn label(self) -> &'static str {
        match self {
            QuestLogTab::Quests => "Quests",
            QuestLogTab::Hunts => "Hunts",
            QuestLogTab::Goals => "My goals",
            QuestLogTab::Clues => "Clues",
        }
    }
}

#[derive(Default, RustState, StateElement)]
pub struct QuestLogWindowState {
    selected_tab: QuestLogTab,
}

const DONE: Color = Color::rgb_u8(120, 220, 120);
const OPEN: Color = Color::rgb_u8(210, 210, 210);
const QUIET: Color = Color::rgb_u8(150, 150, 150);
const ACCENT: Color = Color::rgb_u8(140, 200, 255);

/// Where a route button sends the player: the exact cell of an NPC, from the
/// campaign scripts. Map-wide routes (spawn and drop regions) live in
/// [`Line::Sources`].
#[derive(Clone, Debug, PartialEq, Eq)]
enum RouteTarget {
    Cell { map: String, x: u16, y: u16 },
}

/// One line inside a quest card.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Line {
    /// Secondary text: guidance, explanations.
    Note(String),
    /// An item or kill objective, green when done.
    Objective {
        text: String,
        done: bool,
        guide_item: Option<u32>,
    },
    Route {
        label: String,
        target: RouteTarget,
    },
    /// Drop or spawn sources, folded away: `(button label, map)`.
    Sources {
        title: String,
        routes: Vec<(String, String)>,
    },
    Track {
        quest_id: u32,
        tracked: bool,
    },
    RemoveGoal {
        monster_id: u32,
    },
}

/// One row of the log under the selected tab.
#[derive(Clone, Debug, PartialEq)]
enum Row {
    Note(String),
    Text { text: String, color: Color },
    Card { title: String, expanded: bool, lines: Vec<Line> },
}

fn is_hunt(quest: &QuestEntry) -> bool {
    !quest.requirements().is_empty() || !quest.hunt_objectives().is_empty()
}

/// `(done, total)` objectives for a hunt.
fn hunt_progress(quest: &QuestEntry, carried: &dyn Fn(ItemId) -> u32) -> (usize, usize) {
    let items = quest
        .requirements()
        .iter()
        .map(|requirement| carried(requirement.item_id) >= requirement.needed);
    let kills = quest
        .hunt_objectives()
        .iter()
        .map(|objective| objective.current_count >= objective.total_count);
    let all: Vec<bool> = items.chain(kills).collect();
    (all.iter().filter(|done| **done).count(), all.len())
}

fn route_line(location: &QuestLocationEntry) -> Line {
    let role = if location.is_turn_in { "Turn in to" } else { "Go to" };
    match is_graph_known_map(&location.map_name) {
        true => Line::Route {
            label: format!("{role} {} ({})", location.npc, location.map_name),
            target: RouteTarget::Cell {
                map: location.map_name.clone(),
                x: location.x,
                y: location.y,
            },
        },
        false => Line::Note(quest_location_label(location)),
    }
}

fn quest_card(quest: &QuestEntry, tracked: bool, carried: &dyn Fn(ItemId) -> u32) -> Row {
    let mut lines = Vec::new();
    let mut title = format!("{}{}", if tracked { "• " } else { "" }, quest.name());

    if is_hunt(quest) {
        let (done, total) = hunt_progress(quest, carried);
        title += &match done == total {
            true => "  — ready to turn in".to_owned(),
            false => format!("  — {done}/{total}"),
        };
        for requirement in quest.requirements() {
            let have = carried(requirement.item_id);
            lines.push(Line::Objective {
                text: format!(
                    "{}   {} / {}",
                    requirement.item_name,
                    have.min(requirement.needed),
                    requirement.needed
                ),
                done: have >= requirement.needed,
                guide_item: Some(requirement.item_id.0),
            });
        }
        for objective in quest.hunt_objectives() {
            lines.push(Line::Objective {
                text: format!(
                    "Defeat {}   {} / {}",
                    objective.monster_name, objective.current_count, objective.total_count
                ),
                done: objective.current_count >= objective.total_count,
                guide_item: None,
            });
        }
    }

    if let Some(location) = quest.location() {
        lines.push(route_line(location));
    }
    lines.extend(quest.guidance().iter().cloned().map(Line::Note));

    let mut sources: Vec<(String, String)> = Vec::new();
    for requirement in quest.requirements() {
        for (monster_name, map_name) in requirement_source_routes(requirement.item_id.0) {
            sources.push((format!("{}: {monster_name}, {map_name}", requirement.item_name), map_name));
        }
    }
    for objective in quest.hunt_objectives() {
        for map_name in hunt_spawn_maps(objective.monster_id) {
            sources.push((format!("{}: {map_name}", objective.monster_name), map_name));
        }
    }
    if !sources.is_empty() {
        lines.push(Line::Sources {
            title: format!("Where to find ({})", sources.len()),
            routes: sources,
        });
    }

    if lines.is_empty() {
        lines.push(Line::Note("No details yet.".to_owned()));
    }
    lines.push(Line::Track {
        quest_id: quest.quest_id,
        tracked,
    });

    Row::Card {
        title,
        expanded: tracked,
        lines,
    }
}

/// The rows the log shows under `tab`. Pure, so the layout can be tested
/// without a window; the element turns each row into widgets.
fn plan_rows(log: &QuestLogState, carried: &dyn Fn(ItemId) -> u32, clues: &[String], tab: QuestLogTab) -> Vec<Row> {
    let mut rows = Vec::new();
    match tab {
        QuestLogTab::Quests | QuestLogTab::Hunts => {
            let want_hunts = tab == QuestLogTab::Hunts;
            let mut quests: Vec<&QuestEntry> = log.quests().iter().filter(|quest| is_hunt(quest) == want_hunts).collect();
            // Tracked quests first, otherwise the server's order.
            quests.sort_by_key(|quest| !log.is_tracked(quest.quest_id));
            if quests.is_empty() {
                rows.push(Row::Note(
                    match want_hunts {
                        true => "No hunting contracts right now.",
                        false => "No story quests right now.",
                    }
                    .to_owned(),
                ));
            } else {
                if want_hunts {
                    rows.push(Row::Note(
                        "Counts are what you carry. The turn-in NPC also counts party members who are offline.".to_owned(),
                    ));
                }
                if quests.iter().any(|quest| log.is_tracked(quest.quest_id)) {
                    rows.push(Row::Note("• tracked on your HUD".to_owned()));
                }
                rows.extend(
                    quests
                        .into_iter()
                        .map(|quest| quest_card(quest, log.is_tracked(quest.quest_id), carried)),
                );
            }
        }
        QuestLogTab::Goals => {
            let goals = log.client_hunting_goals();
            if goals.is_empty() {
                rows.push(Row::Note(
                    "No personal goals. Add one from a monster's page in the Adventure Guide.".to_owned(),
                ));
            } else {
                rows.push(Row::Note(
                    "Your own targets, kept on this computer. The server does not count these kills.".to_owned(),
                ));
                for goal in goals {
                    let maps = hunt_spawn_maps(goal.monster_id);
                    let mut lines = Vec::new();
                    match maps.is_empty() {
                        true => lines.push(Line::Note("No routable spawn map is known for this monster.".to_owned())),
                        false => lines.push(Line::Sources {
                            title: format!("Where to find ({})", maps.len()),
                            routes: maps.into_iter().map(|map| (map.clone(), map)).collect(),
                        }),
                    }
                    lines.push(Line::RemoveGoal {
                        monster_id: goal.monster_id,
                    });
                    rows.push(Row::Card {
                        title: goal.monster_name.clone(),
                        expanded: false,
                        lines,
                    });
                }
            }
        }
        QuestLogTab::Clues => {
            // `clue_lines` starts with its own "Clues" header; the tab says that.
            match clues.split_first() {
                None => rows.push(Row::Note(
                    "No clues yet. They appear here as the campaign reveals them.".to_owned(),
                )),
                Some((_, lines)) => {
                    for line in lines {
                        rows.push(match line.strip_prefix("  ") {
                            Some(next) => Row::Text {
                                text: next.to_owned(),
                                color: ACCENT,
                            },
                            None => Row::Text {
                                text: line.clone(),
                                color: OPEN,
                            },
                        });
                    }
                }
            }
        }
    }
    rows
}

/// A fixed list of already-built widgets, laid out top to bottom.
struct Rows {
    elements: Vec<ElementBox<ClientState>>,
}

impl Element<ClientState> for Rows {
    type LayoutInfo = ();

    fn get_element_count(&self, _: &State<ClientState>) -> usize {
        self.elements.len()
    }

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        for (index, element) in self.elements.iter_mut().enumerate() {
            with_nth_resolver(resolvers, index, |resolver| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        }
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

fn line_element(line: &Line) -> ElementBox<ClientState> {
    use korangar_interface::prelude::*;

    match line {
        Line::Note(text) => ErasedElement::new(text! {
            text: text.clone(),
            color: QUIET,
            overflow_behavior: OverflowBehavior::Shrink,
        }),
        Line::Objective { text, done, guide_item } => {
            let label = text! {
                text: text.clone(),
                color: if *done { DONE } else { OPEN },
                overflow_behavior: OverflowBehavior::Shrink,
            };
            match guide_item {
                Some(item_id) => ErasedElement::new(split! {
                    children: (
                        label,
                        button! {
                            text: "Guide",
                            tooltip: "Open this item in the Adventure Guide: drop sources and maps.",
                            event: InputEvent::OpenAdventureGuideItem { item_id: *item_id },
                        },
                    ),
                }),
                None => ErasedElement::new(label),
            }
        }
        Line::Route { label, target } => {
            let RouteTarget::Cell { map, x, y } = target.clone();
            let event = InputEvent::SetNavigationDestination { map_name: map, x, y };
            ErasedElement::new(button! {
                text: label.clone(),
                tooltip: "Show the route on the map.",
                event: event,
            })
        }
        Line::Sources { title, routes } => {
            let buttons = routes
                .iter()
                .map(|(label, map_name)| -> ElementBox<ClientState> {
                    ErasedElement::new(button! {
                        text: label.clone(),
                        tooltip: "Route to this map. Exact spawn cells are not shown.",
                        event: InputEvent::SetNavigationMapDestination { map_name: map_name.clone() },
                    })
                })
                .collect();
            ErasedElement::new(collapsible! {
                text: title.clone(),
                initially_expanded: false,
                children: (Rows { elements: buttons },),
            })
        }
        Line::Track { quest_id, tracked } => ErasedElement::new(button! {
            text: if *tracked { "Stop tracking" } else { "Track on HUD" },
            event: InputEvent::ToggleQuestTracking(*quest_id),
        }),
        Line::RemoveGoal { monster_id } => ErasedElement::new(button! {
            text: "Remove goal",
            event: InputEvent::RemoveClientHuntingGoal { monster_id: *monster_id },
        }),
    }
}

fn row_element(row: &Row) -> ElementBox<ClientState> {
    use korangar_interface::prelude::*;

    match row {
        Row::Note(text) => ErasedElement::new(text! {
            text: text.clone(),
            color: QUIET,
            overflow_behavior: OverflowBehavior::Shrink,
        }),
        Row::Text { text, color } => ErasedElement::new(text! {
            text: text.clone(),
            color: *color,
            overflow_behavior: OverflowBehavior::Shrink,
        }),
        Row::Card { title, expanded, lines } => ErasedElement::new(collapsible! {
            text: title.clone(),
            initially_expanded: *expanded,
            children: (Rows {
                elements: lines.iter().map(line_element).collect(),
            },),
        }),
    }
}

/// The selected tab's rows, rebuilt from state every layout.
struct QuestLogElement<A, B, C> {
    quest_log_path: A,
    inventory_path: B,
    journal_path: C,
    rows: Vec<ElementBox<ClientState>>,
}

impl<A, B, C> QuestLogElement<A, B, C>
where
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
    C: Path<ClientState, DmJournalState>,
{
    fn new(quest_log_path: A, inventory_path: B, journal_path: C) -> Self {
        Self {
            quest_log_path,
            inventory_path,
            journal_path,
            rows: Vec::new(),
        }
    }

    fn plan(&self, state: &State<ClientState>) -> Vec<Row> {
        let inventory = state.get(&self.inventory_path);
        let tab = *state.get(&client_state().quest_log_window().selected_tab());
        plan_rows(
            state.get(&self.quest_log_path),
            &|item_id| inventory.count_of(item_id),
            &clue_lines(state.get(&self.journal_path)),
            tab,
        )
    }
}

/// The Clues section: every investigation step the server has revealed, as the
/// text rows shown under the quests. One list feeds both the row count and the
/// rows, so the two cannot drift apart. Empty (and no header) until the server
/// reports a flag that unlocks a step.
fn clue_lines(journal: &DmJournalState) -> Vec<String> {
    let steps = crate::world::visible_story_steps(|flag| journal.flag(flag));
    if steps.is_empty() {
        return Vec::new();
    }
    let mut lines = vec!["Clues".to_owned()];
    for step in steps {
        lines.push(format!("{}: {}", step.speaker, step.clue));
        if !step.remaining_action.is_empty() {
            lines.push(format!("  Next: {}", step.remaining_action));
        }
    }
    lines
}

/// A map the navigation graph knows, so a route to it can actually be drawn.
fn is_graph_known_map(map_name: &str) -> bool {
    crate::world::navigation_graph()
        .maps
        .iter()
        .any(|known| known.eq_ignore_ascii_case(map_name))
}

/// The label and, when the map is routable, the exact-cell route for a quest's
/// NPC.
fn quest_location_label(location: &QuestLocationEntry) -> String {
    let role = if location.is_turn_in { "Turn in to" } else { "Quest contact" };
    format!("{role} {} — {} {},{}", location.npc, location.map_name, location.x, location.y)
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

impl<A, B, C> Element<ClientState> for QuestLogElement<A, B, C>
where
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
    C: Path<ClientState, DmJournalState>,
{
    type LayoutInfo = ();

    fn get_element_count(&self, state: &State<ClientState>) -> usize {
        self.plan(state).len()
    }

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        // One plan drives both the count above and the rows here, so the two
        // cannot drift apart (they were separate code before 2026-10-05).
        self.rows = self.plan(state).iter().map(row_element).collect();
        for (index, row) in self.rows.iter_mut().enumerate() {
            with_nth_resolver(resolvers, index, |resolver| {
                row.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        }
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

/// The quest log: tabs for story quests, hunting contracts, personal goals
/// and clues. Each quest is one collapsible card whose title carries its
/// progress; tracked quests open expanded.
pub struct QuestLogWindow<A, B, C> {
    quest_log_path: A,
    inventory_path: B,
    journal_path: C,
}

impl<A, B, C> QuestLogWindow<A, B, C> {
    pub fn new(quest_log_path: A, inventory_path: B, journal_path: C) -> Self {
        Self {
            quest_log_path,
            inventory_path,
            journal_path,
        }
    }
}

fn tab_button(tab: QuestLogTab) -> impl Element<ClientState> {
    use korangar_interface::prelude::*;

    let path = client_state().quest_log_window().selected_tab();
    button! {
        text: tab.label(),
        event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
            state.update_value(path, tab);
        },
        // The open tab reads as pressed, the way the skill tree's tabs do.
        disabled: ComputedSelector::new_default(move |state: &ClientState| *path.follow_safe(state) == tab),
    }
}

impl<A, B, C> CustomWindow<ClientState> for QuestLogWindow<A, B, C>
where
    A: Path<ClientState, QuestLogState> + 'static,
    B: Path<ClientState, Inventory> + 'static,
    C: Path<ClientState, DmJournalState> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::QuestLog)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let [quests, hunts, goals, clues] = QuestLogTab::ALL;
        window! {
            title: "Quest Log",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (tab_button(quests), tab_button(hunts), tab_button(goals), tab_button(clues)),
                },
                scroll_view! {
                    children: QuestLogElement::new(self.quest_log_path, self.inventory_path, self.journal_path),
                },
            ),
        }
    }
}

#[cfg(test)]
mod layout_tests {
    use ragnarok_packets::ItemId;

    use super::{Line, QuestLogTab, Row, plan_rows};
    use crate::state::quests::{QuestEntry, QuestHuntObjectiveEntry, QuestLogState, QuestRequirementEntry};

    fn story(quest_id: u32, name: &str) -> QuestEntry {
        QuestEntry {
            quest_id,
            name: name.to_owned(),
            requirements: Vec::new(),
            hunt_objectives: Vec::new(),
            location: None,
            guidance: vec!["Ask around the fountain.".to_owned()],
        }
    }

    fn contract(quest_id: u32) -> QuestEntry {
        QuestEntry {
            quest_id,
            name: "Bone Tag Turn-In".to_owned(),
            requirements: vec![
                QuestRequirementEntry {
                    item_id: ItemId(932),
                    item_name: "Skel-Bone".to_owned(),
                    needed: 4,
                },
                QuestRequirementEntry {
                    item_id: ItemId(938),
                    item_name: "Sticky Mucus".to_owned(),
                    needed: 5,
                },
            ],
            hunt_objectives: Vec::new(),
            location: None,
            guidance: Vec::new(),
        }
    }

    fn log(quests: Vec<QuestEntry>, tracked: &[u32]) -> QuestLogState {
        let mut log = QuestLogState::default();
        log.replace(quests);
        log.set_tracked_quests(tracked);
        log
    }

    fn cards(rows: &[Row]) -> Vec<(&str, bool, &[Line])> {
        rows.iter()
            .filter_map(|row| match row {
                Row::Card { title, expanded, lines } => Some((title.as_str(), *expanded, lines.as_slice())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn story_quests_and_hunts_go_to_their_own_tabs() {
        let log = log(vec![story(20004, "The Trembling Ground"), contract(20009)], &[]);
        let none = |_| 0;
        let quests = plan_rows(&log, &none, &[], QuestLogTab::Quests);
        let hunts = plan_rows(&log, &none, &[], QuestLogTab::Hunts);
        assert_eq!(cards(&quests).len(), 1);
        assert!(cards(&quests)[0].0.starts_with("The Trembling Ground"));
        assert_eq!(cards(&hunts).len(), 1);
        assert!(cards(&hunts)[0].0.starts_with("Bone Tag Turn-In"));
    }

    #[test]
    fn a_hunt_title_summarises_progress_and_says_when_it_is_ready() {
        let log = log(vec![contract(20009)], &[]);
        let some = |item: ItemId| if item.0 == 938 { 5 } else { 1 };
        let rows = plan_rows(&log, &some, &[], QuestLogTab::Hunts);
        assert_eq!(cards(&rows)[0].0, "Bone Tag Turn-In  — 1/2");
        let all = |_| 99;
        let rows = plan_rows(&log, &all, &[], QuestLogTab::Hunts);
        assert_eq!(cards(&rows)[0].0, "Bone Tag Turn-In  — ready to turn in");
        // Carrying more than needed never reads as "99 / 5".
        assert!(
            cards(&rows)[0]
                .2
                .iter()
                .any(|line| matches!(line, Line::Objective { text, done: true, .. } if text.ends_with("5 / 5")))
        );
    }

    #[test]
    fn cards_start_collapsed_unless_tracked_and_tracked_ones_come_first() {
        let log = log(vec![story(1, "First"), story(2, "Second")], &[2]);
        let rows = plan_rows(&log, &|_| 0, &[], QuestLogTab::Quests);
        let cards = cards(&rows);
        assert_eq!(cards[0].0, "• Second");
        assert!(cards[0].1, "a tracked quest opens expanded");
        assert_eq!(cards[1].0, "First");
        assert!(!cards[1].1, "an untracked quest starts collapsed");
    }

    #[test]
    fn sources_are_folded_into_one_section_not_a_button_per_map() {
        let mut quest = contract(20009);
        quest.hunt_objectives = vec![QuestHuntObjectiveEntry {
            monster_id: 1002,
            monster_name: "Poring".to_owned(),
            total_count: 10,
            current_count: 3,
        }];
        let log = log(vec![quest], &[]);
        let rows = plan_rows(&log, &|_| 0, &[], QuestLogTab::Hunts);
        let lines = cards(&rows)[0].2;
        let routes_outside = lines.iter().filter(|line| matches!(line, Line::Route { .. })).count();
        assert_eq!(routes_outside, 0, "no loose per-map buttons: {lines:?}");
        assert!(lines.iter().filter(|line| matches!(line, Line::Sources { .. })).count() <= 1);
        assert!(
            lines
                .iter()
                .any(|line| matches!(line, Line::Objective { text, done: false, .. } if text == "Defeat Poring   3 / 10"))
        );
        assert!(matches!(
            lines.last(),
            Some(Line::Track {
                quest_id: 20009,
                tracked: false
            })
        ));
    }

    #[test]
    fn story_quests_are_named_from_the_quest_database() {
        // resolve_quest_entry names a non-contract quest from this lookup;
        // it used to print "Quest 20004".
        let data = crate::dm::reference_data::reference_data();
        assert_eq!(
            data.quest_by_id(20004).map(|quest| quest.name.as_str()),
            Some("The Trembling Ground")
        );
    }

    #[test]
    fn empty_tabs_explain_themselves() {
        let empty = QuestLogState::default();
        for tab in QuestLogTab::ALL {
            let rows = plan_rows(&empty, &|_| 0, &[], tab);
            assert!(matches!(rows.as_slice(), [Row::Note(_)]), "{tab:?}: {rows:?}");
        }
    }

    #[test]
    fn clues_drop_their_section_header_and_highlight_next_steps() {
        let clues = vec![
            "Clues".to_owned(),
            "Mira: the bell rang twice".to_owned(),
            "  Next: ask the priest".to_owned(),
        ];
        let rows = plan_rows(&QuestLogState::default(), &|_| 0, &clues, QuestLogTab::Clues);
        assert_eq!(rows.len(), 2);
        assert!(matches!(&rows[1], Row::Text { text, .. } if text == "Next: ask the priest"));
    }
}

#[cfg(test)]
mod tests {
    use super::{clue_lines, hunt_spawn_maps, requirement_source_routes};
    use crate::state::dm_journal::DmJournalState;

    fn journal(lines: &[&str]) -> DmJournalState {
        let mut journal = DmJournalState::default();
        for body in lines {
            journal.receive_server_line(&format!("[DMJ]{body}"));
        }
        journal
    }

    /// No flag from the server, no Clues section: not even a header.
    #[test]
    fn the_clues_section_is_absent_until_the_server_reveals_something() {
        assert!(clue_lines(&DmJournalState::default()).is_empty());
    }

    #[test]
    fn a_revealed_clue_shows_with_its_next_step_under_a_header() {
        let lines = clue_lines(&journal(&[r#"{"t":"flag","v":1,"seq":0,"name":"dm_arc01_started","value":1}"#]));
        assert_eq!(lines[0], "Clues");
        assert!(lines[1].starts_with("Quartermaster Wynne: "), "{lines:?}");
        assert!(lines[2].starts_with("  Next: "), "{lines:?}");
        assert_eq!(lines.len(), 3);
    }

    /// A clue the server has not unlocked must not appear just because a
    /// neighbouring one did.
    #[test]
    fn only_the_unlocked_clues_appear() {
        let lines = clue_lines(&journal(&[
            r#"{"t":"flag","v":1,"seq":0,"name":"dm_arc01_clue_mask","value":4}"#,
        ]))
        .join("\n");
        assert!(lines.contains("Tibbets the Keeper"), "{lines}");
        assert!(!lines.contains("Painted Sluice"), "{lines}");
        assert!(!lines.contains("Mira"), "{lines}");
    }

    /// A flag the server clears takes its clue away again.
    #[test]
    fn a_flag_going_back_to_zero_hides_the_clue() {
        let journal = journal(&[
            r#"{"t":"flag","v":1,"seq":0,"name":"dm_arc01_started","value":1}"#,
            r#"{"t":"flag","v":1,"seq":0,"name":"dm_arc01_started","value":0}"#,
        ]);
        assert!(clue_lines(&journal).is_empty());
    }

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
