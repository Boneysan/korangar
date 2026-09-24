//! Player-facing, open-search reference guide. This intentionally does not
//! reuse the DM Bestiary: campaign reveal/spawn controls and unlocks are not
//! part of the player's mechanical reference.

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox, StateElement};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, RustState, State, VecIndexExt};

use crate::dm::reference_data::{ReferenceItem, ReferenceJobBonuses, ReferenceMonster, ReferenceSkill, reference_data};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

const MAX_QUERY: usize = 48;
const MAX_RESULTS: usize = 60;
const JOB_NAMES: &str = include_str!("../../world/library/hercules_job_names.tsv");

#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct GuideResult {
    pub label: String,
    pub kind: String,
    pub id: u32,
}

#[derive(RustState, StateElement)]
pub struct AdventureGuideWindowState {
    query: String,
    category: String,
    results: Vec<GuideResult>,
    detail: Vec<String>,
}

impl Default for AdventureGuideWindowState {
    fn default() -> Self {
        Self {
            query: String::new(),
            category: "Monsters".to_owned(),
            results: Vec::new(),
            detail: vec!["Choose a category and search, then select an entry.".to_owned()],
        }
    }
}

pub struct AdventureGuideWindow<A> {
    state_path: A,
}

impl<A> AdventureGuideWindow<A> {
    pub fn new(state_path: A) -> Self {
        Self { state_path }
    }
}

fn monster_details(monster: &ReferenceMonster) -> Vec<String> {
    let mut lines = vec![
        format!("{}  (ID {})", display_name(&monster.name, &monster.sprite_name), monster.id),
        format!("Level {}   HP {}", monster.level, monster.hp),
    ];
    if let Some(element) = &monster.element {
        lines.push(format!("Element: {} {}", element.r#type, element.level));
    }
    if let Some(race) = &monster.race {
        lines.push(format!("Race: {race}"));
    }
    if let Some(size) = &monster.size {
        lines.push(format!("Size: {size}"));
    }
    lines.push(format!("Skills: {}   Drops: {}", monster.skills.len(), monster.drops.len()));
    for drop in monster.drops.iter().take(8) {
        lines.push(format!(
            "@guide:item:{}|{} — {:.2}%",
            drop.item_id,
            display_name(&drop.name, &drop.aegis_name),
            drop.rate_per_10000 as f32 / 100.0
        ));
    }
    if let Some(source) = &monster.source {
        lines.push(format!("Source: {} ({})", source.path, source.record));
    }
    let routeable_regions = monster
        .spawn_regions
        .iter()
        .filter(|region| is_graph_map(&region.map))
        .collect::<Vec<_>>();
    if routeable_regions.is_empty() {
        lines.push("No graph-known static spawn-map route is available; conditional/scripted spawns may also be omitted.".to_owned());
    } else {
        lines.push("Known static spawn maps (broad map references only; no spawn cells are exposed):".to_owned());
        for region in routeable_regions.iter().take(12) {
            lines.push(format!("{} — {} spawn records", region.map, region.spawn_records));
            lines.push(format!("@route:{}", region.map));
        }
        if routeable_regions.len() > 12 {
            lines.push(format!(
                "{} additional spawn maps omitted from this detail view.",
                routeable_regions.len() - 12
            ));
        }
    }
    lines
}

fn item_details(item: &ReferenceItem, card: bool) -> Vec<String> {
    let kind = if card { "Card" } else { item.item_type.as_str() };
    let mut lines = vec![
        format!("{}  (ID {})", display_name(&item.name, &item.aegis_name), item.id),
        format!("Type: {kind}   Weight: {}", item.weight),
    ];
    if item.buy > 0 {
        lines.push(format!("Buy price: {}z", item.buy));
    }
    if let Some(atk) = item.atk {
        lines.push(format!("ATK: {atk}"));
    }
    if let Some(matk) = item.matk {
        lines.push(format!("MATK: {matk}"));
    }
    if let Some(defense) = item.defense {
        lines.push(format!("DEF: {defense}"));
    }
    if let Some(slots) = item.slots {
        lines.push(format!("Slots: {slots}"));
    }
    if !item.effect_status.is_empty() {
        lines.push(format!("Script status: {} (not translated)", item.effect_status));
    } else {
        lines.push("Detailed script effect: not documented yet.".to_owned());
    }
    if !item.drops_from.is_empty() {
        lines.push("Dropped by:".to_owned());
        for source in item.drops_from.iter().take(8) {
            lines.push(format!(
                "@guide:monster:{}|{} (ID {}) — {:.2}%",
                source.monster_id,
                source.sprite_name,
                source.monster_id,
                source.rate_per_10000 as f32 / 100.0
            ));
            if let Some(monster) = reference_data().monster_by_id(source.monster_id) {
                for region in monster.spawn_regions.iter().take(3) {
                    if !is_graph_map(&region.map) {
                        continue;
                    }
                    lines.push(format!("@route:{}", region.map));
                }
            }
        }
    }
    if let Some(source) = &item.source {
        lines.push(format!("Source: {} ({})", source.path, source.record));
    }
    lines
}

fn is_graph_map(map_name: &str) -> bool {
    crate::world::navigation_graph()
        .maps
        .iter()
        .any(|known| known.eq_ignore_ascii_case(map_name))
}

fn display_name(name: &str, fallback: &str) -> String {
    if name.is_empty() { fallback.to_owned() } else { name.to_owned() }
}

fn item_matches_query(item: &ReferenceItem, query: &str) -> bool {
    query.is_empty()
        || query.parse::<u32>().ok() == Some(item.id)
        || item.name.to_lowercase().contains(query)
        || item.aegis_name.to_lowercase().contains(query)
}

fn quest_details(quest: &crate::state::quests::QuestEntry) -> Vec<String> {
    let data = reference_data();
    let mut lines = vec![format!("{}  (Quest ID {})", quest.name(), quest.quest_id)];
    if quest.hunt_objectives().is_empty() && quest.requirements().is_empty() {
        lines.push("No objective details are available from the server or bundled campaign data.".to_owned());
    }
    for objective in quest.hunt_objectives() {
        lines.push(format!(
            "@guide:monster:{}|Hunt {} — {}/{}",
            objective.monster_id,
            display_name(&objective.monster_name, &format!("Monster {}", objective.monster_id)),
            objective.current_count,
            objective.total_count,
        ));
        if let Some(monster) = data.monster_by_id(objective.monster_id) {
            let mut route_count = 0;
            for region in monster.spawn_regions.iter().filter(|region| is_graph_map(&region.map)).take(8) {
                lines.push(format!("@route:{}", region.map));
                route_count += 1;
            }
            if route_count == 0 {
                lines.push("No loaded static spawn-map route is known for this objective.".to_owned());
            }
        }
    }
    for requirement in quest.requirements() {
        lines.push(format!(
            "@guide:item:{}|Collect {} — {}/?",
            requirement.item_id.0, requirement.item_name, requirement.needed
        ));
    }
    if let Some(reference) = data.quest_by_id(quest.quest_id) {
        lines.extend(quest_reference_details(reference));
    }
    lines
}

fn quest_reference_details(quest: &crate::dm::reference_data::ReferenceQuest) -> Vec<String> {
    let mut lines = vec![format!("{} — bundled hunt reference", quest.name)];
    if quest.targets.is_empty() {
        lines.push("No explicit hunt targets are recorded for this quest.".to_owned());
    }
    for target in &quest.targets {
        let title = format!("{} × {}", target.monster_name, target.count);
        lines.push(format!("Objective: {title}"));
        if let Some(level) = target.level_range {
            lines.push(format!("Target level bounds (raw server values): {} / {}", level[0], level[1]));
        }
        let map_has_route = target.map_name.as_ref().is_some_and(|map| is_graph_map(map));
        if let Some(map) = &target.map_name {
            if map_has_route {
                lines.push(format!("@route:{map}"));
            } else {
                lines.push(format!("Target map {map} has no loaded navigation route."));
            }
        }
        if !map_has_route && let Some(mob_id) = target.mob_id {
            if target.monster_data_known == Some(true) {
                lines.push(format!(
                    "@guide:monster:{mob_id}|View {} and known spawn routes",
                    target.monster_name
                ));
            } else {
                lines.push(format!("Monster ID {mob_id} has no matching bundled bestiary entry."));
            }
        }
    }
    lines.push("Quest giver, scripted story steps, prerequisites, and rewards are not included in this static hunt reference.".to_owned());
    lines
}

fn resolve_details(result: &GuideResult) -> Vec<String> {
    let data = reference_data();
    match result.kind.as_str() {
        "monster" => data
            .monster_by_id(result.id)
            .map(monster_details)
            .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        "item" => data
            .item_by_id(result.id)
            .or_else(|| data.card_by_id(result.id))
            .map(|item| item_details(item, data.card_by_id(result.id).is_some()))
            .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        "card" => data
            .card_by_id(result.id)
            .map(|item| item_details(item, true))
            .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        "skill" => data
            .skill_by_id(result.id)
            .map(skill_details)
            .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        "status" => data
            .statuses
            .iter()
            .find(|status| status.id == result.id)
            .map(status_details)
            .unwrap_or_else(|| vec!["Status reference entry unavailable.".to_owned()]),
        "quest" => data
            .quest_by_id(result.id)
            .map(quest_reference_details)
            .unwrap_or_else(|| vec!["Quest reference entry unavailable.".to_owned()]),
        "map" => crate::world::navigation_graph()
            .maps
            .get(result.id as usize)
            .map(|map_name| vec![format!("Map: {map_name}"), "@route:".to_owned() + map_name])
            .unwrap_or_else(|| vec!["Map entry unavailable.".to_owned()]),
        "job" => job_names()
            .find(|(id, _)| *id as u32 == result.id)
            .map(|(_, name)| job_details(result.id as u16, name))
            .unwrap_or_else(|| vec!["Job entry unavailable.".to_owned()]),
        _ => vec!["Unsupported category.".to_owned()],
    }
}

fn job_details(job_id: u16, name: &str) -> Vec<String> {
    let mut lines = vec![format!("{name}  (Job ID {job_id})")];
    let data = reference_data();
    if let Some(bonuses) = data.job_bonuses_by_id(job_id) {
        append_job_bonus_details(&mut lines, bonuses);
    } else {
        lines.push("No job-level stat bonus schedule is present for this job ID in Hercules job_db2.txt.".to_owned());
    }
    let Some(tree) = data.job_skill_tree_by_id(job_id) else {
        lines.push("No matching skill tree is present in the bundled Hercules job-skill export.".to_owned());
        lines.push("Job bonus source: bundled Hercules job_db2.txt export; conditional-script effects are not inferred.".to_owned());
        return lines;
    };

    lines.push(format!(
        "Skill tree: {} — {} skills including inherited skills",
        tree.tree_name,
        tree.skills.len()
    ));
    for skill in &tree.skills {
        let mut label = format!("{} — max level {}", skill.name, skill.max_level);
        if skill.minimum_job_level > 0 {
            label.push_str(&format!(", job level {}", skill.minimum_job_level));
        }
        if !skill.prerequisites.is_empty() {
            let prerequisites = skill
                .prerequisites
                .iter()
                .map(|prerequisite| format!("{} Lv {}", prerequisite.name, prerequisite.level))
                .collect::<Vec<_>>()
                .join(", ");
            label.push_str(&format!(" — requires {prerequisites}"));
        }
        lines.push(format!("@guide:skill:{}|{label}", skill.skill_id));
    }
    lines.push(
        "Sources: bundled Hercules renewal skill_tree.conf and skill_db.conf for skills; job_db2.txt for job-level stat bonuses. \
         Conditional-script effects are not inferred."
            .to_owned(),
    );
    lines
}

fn append_job_bonus_details(lines: &mut Vec<String>, bonuses: &ReferenceJobBonuses) {
    lines.push(format!("Job-level stat bonuses through job level {}:", bonuses.max_job_level));
    for stat in ["STR", "AGI", "VIT", "INT", "DEX", "LUK"] {
        let levels = bonuses
            .bonus_levels
            .iter()
            .filter(|bonus| bonus.stat == stat)
            .map(|bonus| bonus.job_level.to_string())
            .collect::<Vec<_>>();
        if levels.is_empty() {
            continue;
        }
        let total = bonuses.total_bonuses.get(stat).copied().unwrap_or(levels.len() as u16);
        lines.push(format!("{stat} +{total}: job levels {}", levels.join(", ")));
    }
}

fn skill_details(skill: &ReferenceSkill) -> Vec<String> {
    let mut lines = crate::world::skill_tooltip_text(skill.id, &skill.description, 1, skill.maximum_level)
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    lines.push(format!("Skill identifier: {} (ID {})", skill.name, skill.id));
    let linked_statuses: Vec<_> = reference_data()
        .statuses
        .iter()
        .filter(|status| {
            status
                .statuses
                .iter()
                .any(|mechanic| mechanic.associated_skill.as_ref().is_some_and(|source| source.id == skill.id))
        })
        .collect();
    if !linked_statuses.is_empty() {
        lines.push("Associated status references:".to_owned());
        for status in linked_statuses {
            lines.push(format!("@guide:status:{}|{} (icon {})", status.id, status.name, status.id));
        }
    }
    lines.push("Source: bundled Hercules skill database export.".to_owned());
    lines
}

fn status_details(status: &crate::dm::reference_data::ReferenceStatus) -> Vec<String> {
    let mut lines = vec![format!("{}  (Status icon ID {})", status.name, status.id)];
    if status.statuses.is_empty() {
        lines.push("Verified reference: server status-icon name only; no matching sc_config record.".to_owned());
    } else {
        lines.push("Verified server metadata from renewal sc_config.conf:".to_owned());
        for mechanic in &status.statuses {
            lines.push(format!("{} (status ID {})", mechanic.constant, mechanic.id));
            if mechanic.flags.is_empty() {
                lines.push("  Server flags: none listed".to_owned());
            } else {
                lines.push(format!("  Server flags: {}", mechanic.flags.join(", ")));
            }
            if !mechanic.calculation_flags.is_empty() {
                lines.push(format!("  Recalculation flags: {}", mechanic.calculation_flags.join(", ")));
            }
            if let Some(skill) = &mechanic.associated_skill {
                let label = if skill.description.is_empty() {
                    skill.name.clone()
                } else {
                    format!("{} ({})", skill.description, skill.name)
                };
                lines.push(format!("@guide:skill:{}|Associated skill: {label}", skill.id));
            }
        }
    }
    lines.push("Exact effect, duration, per-level odds, all sources, interactions, and cures: not documented yet.".to_owned());
    lines
}

fn parse_guide_link(line: &str) -> Option<GuideResult> {
    let link = line.strip_prefix("@guide:")?;
    let (target, label) = link.split_once('|')?;
    let (kind, id) = target.split_once(':')?;
    if !matches!(kind, "item" | "monster" | "skill" | "status" | "quest") {
        return None;
    }
    Some(GuideResult {
        label: label.to_owned(),
        kind: kind.to_owned(),
        id: id.parse().ok()?,
    })
}

struct GuideResultList<A> {
    state_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> Element<ClientState> for GuideResultList<A>
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy + 'static,
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
            let count = state.get(&self.state_path.results()).len();
            self.elements.truncate(count);
            for index in self.elements.len()..count {
                let row_path = self.state_path.results().index(index).manually_asserted();
                let detail_path = self.state_path.detail();
                self.elements.push(ErasedElement::new(button! {
                    text: row_path.label(),
                    event: move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                        let result = state.get(&row_path).clone();
                        let detail = if result.kind == "quest" {
                            state
                                .get(&client_state().quest_log())
                                .quests()
                                .iter()
                                .find(|quest| quest.quest_id == result.id)
                                .map(quest_details)
                                .unwrap_or_else(|| vec!["This quest is no longer active. Refresh the search to update the list.".to_owned()])
                        } else {
                            resolve_details(&result)
                        };
                        state.update_value(detail_path, detail);
                    },
                }));
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

struct GuideLines<A> {
    path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> GuideLines<A> {
    fn new(path: A) -> Self {
        Self {
            path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for GuideLines<A>
where
    A: Path<ClientState, Vec<String>> + Copy,
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
            let count = state.get(&self.path).len();
            self.elements.truncate(count);
            for index in self.elements.len()..count {
                let line = self.path.index(index).manually_asserted();
                let value = state.get(&line).clone();
                if let Some(map_name) = value.strip_prefix("@route:") {
                    let map_name = map_name.to_owned();
                    self.elements.push(ErasedElement::new(button! {
                        text: format!("Route to {map_name}"),
                        event: InputEvent::SetNavigationMapDestination { map_name },
                    }));
                } else if let Some(result) = parse_guide_link(&value) {
                    let detail_path = self.path;
                    self.elements.push(ErasedElement::new(button! {
                        text: result.label.clone(),
                        event: move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                            state.update_value(detail_path, resolve_details(&result));
                        },
                    }));
                } else {
                    self.elements.push(ErasedElement::new(
                        text! { text: line, overflow_behavior: OverflowBehavior::Shrink },
                    ));
                }
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

fn run_search<A>(state: &State<ClientState>, path: A)
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy,
{
    let query = state.get(&path.query()).to_lowercase();
    let category = state.get(&path.category()).clone();
    let data = reference_data();
    let discovery_path = client_state().discovery();
    let discovery = state.get(&discovery_path);
    let mut rows = Vec::new();
    if category == "Monsters" {
        rows.extend(
            data.search_monsters(&query, MAX_RESULTS).into_iter().map(|monster| GuideResult {
                label: format!(
                    "{}{}  Lv {}",
                    match (discovery.snapshot_complete(), discovery.milestone(monster.id as u16).is_some()) {
                        (_, true) => "[Discovered] ",
                        (true, false) => "[Not encountered] ",
                        (false, false) => "[Sync pending] ",
                    },
                    display_name(&monster.name, &monster.sprite_name),
                    monster.level,
                ),
                kind: "monster".to_owned(),
                id: monster.id,
            }),
        );
    } else if category == "Skills" {
        rows.extend(data.search_skills(&query, MAX_RESULTS).into_iter().map(|skill| GuideResult {
            label: format!("{}  (ID {})", display_name(&skill.description, &skill.name), skill.id),
            kind: "skill".to_owned(),
            id: skill.id as u32,
        }));
    } else if category == "Status Effects" {
        rows.extend(data.search_statuses(&query, MAX_RESULTS).into_iter().map(|status| GuideResult {
            label: format!("{}  (status icon {})", status.name, status.id),
            kind: "status".to_owned(),
            id: status.id,
        }));
    } else if category == "Maps" {
        rows.extend(
            crate::world::navigation_graph()
                .maps
                .iter()
                .enumerate()
                .filter(|(_, map)| query.is_empty() || map.to_lowercase().contains(&query))
                .take(MAX_RESULTS)
                .map(|(index, map)| GuideResult {
                    label: format!(
                        "{}{}  (map)",
                        match (discovery.map_snapshot_complete(), discovery.visited_map(map)) {
                            (_, true) => "[Visited] ",
                            (true, false) => "[Not visited] ",
                            (false, false) => "[Sync pending] ",
                        },
                        map,
                    ),
                    kind: "map".to_owned(),
                    id: index as u32,
                }),
        );
    } else if category == "Jobs" {
        rows.extend(
            job_names()
                .filter(|(_, name)| query.is_empty() || name.to_lowercase().contains(&query))
                .take(MAX_RESULTS)
                .map(|(id, name)| GuideResult {
                    label: format!("{name}  (job)"),
                    kind: "job".to_owned(),
                    id: id as u32,
                }),
        );
    } else if category == "Quests" {
        let quest_log_path = client_state().quest_log();
        let active_quests = state.get(&quest_log_path).quests();
        let active_ids = active_quests
            .iter()
            .map(|quest| quest.quest_id)
            .collect::<std::collections::HashSet<_>>();
        rows.extend(data.search_quests(&query, MAX_RESULTS).into_iter().map(|quest| GuideResult {
            label: format!(
                "{}  (Quest {}){}",
                quest.name,
                quest.id,
                if active_ids.contains(&quest.id) { " — Active" } else { "" }
            ),
            kind: "quest".to_owned(),
            id: quest.id,
        }));
        let listed_ids = rows.iter().map(|row| row.id).collect::<std::collections::HashSet<_>>();
        let active = active_quests
            .iter()
            .filter(|quest| query.is_empty() || quest.name().to_lowercase().contains(&query) || quest.quest_id.to_string().contains(&query))
            .filter(|quest| !listed_ids.contains(&quest.quest_id))
            .take(MAX_RESULTS.saturating_sub(rows.len()))
            .map(|quest| GuideResult {
                label: format!("{}  (Quest {})", quest.name(), quest.quest_id),
                kind: "quest".to_owned(),
                id: quest.quest_id,
            });
        rows.extend(active);
    } else {
        let cards_only = category == "Cards";
        let source = if cards_only { &data.cards } else { &data.items };
        rows.extend(
            source
                .iter()
                .filter(|item| item_matches_query(item, &query))
                .take(MAX_RESULTS)
                .map(|item| GuideResult {
                    label: format!("{}  (ID {})", display_name(&item.name, &item.aegis_name), item.id),
                    kind: if cards_only { "card" } else { "item" }.to_owned(),
                    id: item.id,
                }),
        );
    }
    state.update_value(path.results(), rows);
    state.update_value(path.detail(), vec![
        "Select an entry to see its verified fields and source.".to_owned(),
    ]);
}

/// Select an item from another in-game surface, populate the Guide search and
/// detail panes, and leave the guide ready to continue browsing.
pub fn open_item_entry<A>(state: &State<ClientState>, path: A, item_id: u32)
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy,
{
    state.update_value(path.category(), "Items".to_owned());
    state.update_value(path.query(), item_id.to_string());
    run_search(state, path);
    let data = reference_data();
    let detail = data
        .item_by_id(item_id)
        .map(|item| item_details(item, false))
        .or_else(|| data.card_by_id(item_id).map(|item| item_details(item, true)))
        .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]);
    state.update_value(path.detail(), detail);
}

impl<A> CustomWindow<ClientState> for AdventureGuideWindow<A>
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::AdventureGuide)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;
        struct GuideSearchBox;
        let path = self.state_path;
        let search = move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| run_search(state, path);
        let set_category = |category: &'static str| {
            move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                state.update_value(path.category(), category.to_owned());
                run_search(state, path);
            }
        };
        let revision = reference_data().source_revision.clone();
        window! {
            title: "Adventure Guide",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! { text: format!("Open reference • data revision {} • discovery badges sync per account; mechanics remain open • untranslated scripts and missing spawn data are labeled", revision), overflow_behavior: OverflowBehavior::Shrink },
                text_box! { ghost_text: "Search monsters, items, cards, skills, status effects, maps, jobs, or quests…", state: path.query(), input_handler: DefaultHandler::<_, _, MAX_QUERY>::new(path.query(), search), focus_id: GuideSearchBox, overflow_behavior: OverflowBehavior::Shrink },
                split! { gaps: theme().window().gaps(), children: (
                    button! { text: "Monsters", event: set_category("Monsters") },
                    button! { text: "Items", event: set_category("Items") },
                    button! { text: "Cards", event: set_category("Cards") },
                    button! { text: "Skills", event: set_category("Skills") },
                    button! { text: "Status Effects", event: set_category("Status Effects") },
                    button! { text: "Maps", event: set_category("Maps") },
                    button! { text: "Jobs", event: set_category("Jobs") },
                    button! { text: "Quests", event: set_category("Quests") },
                    button! { text: "Search", event: search },
                ) },
                scroll_view! { children: GuideResultList { state_path: path, elements: Vec::new() } },
                scroll_view! { children: GuideLines::new(path.detail()) },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GuideResult, ReferenceItem, display_name, item_details, item_matches_query, job_names, monster_details, parse_guide_link,
        quest_details, quest_reference_details, reference_data, resolve_details, skill_details,
    };
    use crate::dm::reference_data::{ReferenceQuest, ReferenceQuestTarget};
    use crate::state::quests::{QuestEntry, QuestHuntObjectiveEntry, QuestRequirementEntry};

    #[test]
    fn guide_labels_untranslated_script_effects_and_missing_fields() {
        let item = ReferenceItem {
            id: 501,
            aegis_name: "RED_POTION".to_owned(),
            name: "Red Potion".to_owned(),
            item_type: "IT_HEALING".to_owned(),
            buy: 0,
            weight: 0,
            atk: None,
            matk: None,
            defense: None,
            slots: None,
            effect_status: "<script>".to_owned(),
            source: None,
            drops_from: Vec::new(),
        };
        let lines = item_details(&item, false).join("\n");
        assert!(lines.contains("not translated"));
        assert!(!lines.contains("not documented yet"));
        let mut undocumented = item;
        undocumented.effect_status.clear();
        assert!(item_details(&undocumented, false).join("\n").contains("not documented yet"));
        assert_eq!(display_name("", "PORING"), "PORING");
    }

    #[test]
    fn item_search_accepts_exact_numeric_ids_without_matching_other_rows() {
        let item = ReferenceItem {
            id: 501,
            aegis_name: "RED_POTION".to_owned(),
            name: "Red Potion".to_owned(),
            item_type: "IT_HEALING".to_owned(),
            buy: 0,
            weight: 0,
            atk: None,
            matk: None,
            defense: None,
            slots: None,
            effect_status: String::new(),
            source: None,
            drops_from: Vec::new(),
        };
        assert!(item_matches_query(&item, "501"));
        assert!(!item_matches_query(&item, "502"));
        assert!(item_matches_query(&item, "red pot"));
    }

    #[test]
    fn guide_drop_lists_are_clickable_cross_references() {
        let data = reference_data();
        let monster = data
            .monsters
            .iter()
            .find(|monster| !monster.drops.is_empty())
            .expect("monster drop data");
        let item_link = monster_details(monster)
            .into_iter()
            .find_map(|line| line.strip_prefix("@guide:item:").map(str::to_owned))
            .expect("monster drop cross-link");
        let (target, _) = item_link.split_once('|').expect("well-formed item cross-link");
        let item_id = target.parse::<u32>().expect("numeric item id");
        assert!(data.item_by_id(item_id).is_some() || data.card_by_id(item_id).is_some());

        let item = data
            .items
            .iter()
            .find(|item| {
                item.drops_from.iter().any(|source| {
                    data.monster_by_id(source.monster_id)
                        .is_some_and(|monster| !monster.spawn_regions.is_empty())
                })
            })
            .expect("item drop source data");
        let item_detail = item_details(item, false);
        let monster_link = item_detail
            .iter()
            .cloned()
            .find_map(|line| line.strip_prefix("@guide:monster:").map(str::to_owned))
            .expect("item source cross-link");
        let (target, _) = monster_link.split_once('|').expect("well-formed monster cross-link");
        let monster_id = target.parse::<u32>().expect("numeric monster id");
        assert!(data.monster_by_id(monster_id).is_some());
        let source_map = data
            .monster_by_id(monster_id)
            .and_then(|monster| monster.spawn_regions.first())
            .map(|region| region.map.as_str())
            .expect("drop-source monster map");
        assert!(
            crate::world::navigation_graph()
                .maps
                .iter()
                .any(|known| known.eq_ignore_ascii_case(source_map))
        );
        assert!(item_detail.iter().any(|line| line == &format!("@route:{source_map}")));

        let detail = resolve_details(&GuideResult {
            label: String::new(),
            kind: "monster".to_owned(),
            id: monster_id,
        });
        assert!(!detail.is_empty());
    }

    #[test]
    fn guide_searches_existing_skill_export_and_shows_verified_tooltip_fields() {
        let matches = reference_data().search_skills("Fire Bolt", 10);
        let skill = matches.iter().find(|skill| skill.id == 19).expect("Fire Bolt skill row");
        let detail = skill_details(skill).join("\n");
        assert!(detail.contains("Fire Bolt"));
        assert!(detail.contains("SP "));
        assert!(detail.contains("Source: bundled Hercules skill database export."));
    }

    #[test]
    fn status_and_skill_reference_links_are_navigable_in_both_directions() {
        let blessing = reference_data()
            .search_skills("AL_BLESSING", 1)
            .into_iter()
            .next()
            .expect("Blessing skill row");
        let skill_lines = skill_details(blessing);
        let status_link = skill_lines
            .iter()
            .find(|line| line.starts_with("@guide:status:"))
            .expect("skill links to its status icon record");
        let target = parse_guide_link(status_link).expect("status link is an actionable Guide link");
        assert_eq!(target.kind, "status");
        let status_lines = resolve_details(&target).join("\n");
        assert!(status_lines.contains("SC_BLESSING (status ID 30)"));
        assert!(status_lines.contains("@guide:skill:34|Associated skill: Blessing (AL_BLESSING)"));
    }

    #[test]
    fn guide_map_details_offer_routing_for_graph_maps() {
        let maps = &crate::world::navigation_graph().maps;
        let index = maps.iter().position(|map| map == "prt_fild08").expect("known map");
        let result = GuideResult {
            label: "prt_fild08".to_owned(),
            kind: "map".to_owned(),
            id: index as u32,
        };
        let detail = resolve_details(&result);
        assert!(detail.iter().any(|line| line == "Map: prt_fild08"));
        assert!(detail.iter().any(|line| line == "@route:prt_fild08"));
    }

    #[test]
    fn status_effect_search_shows_verified_server_metadata_and_marks_gaps() {
        let data = reference_data();
        let blessing = data
            .search_statuses("blessing", 10)
            .into_iter()
            .next()
            .expect("Blessing status name");
        let details = super::status_details(blessing).join("\n");
        assert!(details.contains("Verified server metadata from renewal sc_config.conf"));
        assert!(details.contains("SC_BLESSING (status ID 30)"));
        assert!(details.contains("Server flags: Buff, NoBoss, NoMadoReset, NoMagicBlocked"));
        assert!(details.contains("Recalculation flags: Dex, Hit, Int, Str"));
        assert!(details.contains("@guide:skill:34|Associated skill: Blessing (AL_BLESSING)"));
        assert!(details.contains("Exact effect, duration, per-level odds, all sources, interactions, and cures: not documented yet."));
        assert_eq!(data.statuses.len(), 700);
        assert!(
            data.search_statuses(&blessing.id.to_string(), 1)
                .iter()
                .any(|status| status.id == blessing.id)
        );
        assert_eq!(data.search_statuses("AL_BLESSING", 1)[0].id, blessing.id);

        let name_only = data.search_statuses("Stormkick Ready", 1).remove(0);
        assert!(
            super::status_details(name_only)
                .join("\n")
                .contains("server status-icon name only; no matching sc_config record")
        );
    }

    #[test]
    fn guide_job_entries_cross_link_to_verified_skill_trees() {
        let (id, name) = job_names().find(|(_, name)| *name == "Knight").expect("Knight job");
        let detail = resolve_details(&GuideResult {
            label: name.to_owned(),
            kind: "job".to_owned(),
            id: id as u32,
        });
        assert!(detail[0].contains("Knight"));
        assert!(detail.iter().any(|line| line.contains("inherited skills")));
        assert!(detail.iter().any(|line| line == "STR +8: job levels 4, 10, 15, 21, 27, 33, 46, 47"));
        assert!(
            detail
                .iter()
                .any(|line| line.starts_with("Job-level stat bonuses through job level 50:"))
        );
        assert!(detail.iter().any(|line| line.starts_with("@guide:skill:")));
        assert!(detail.iter().any(|line| line.contains("requires")));
        let bash_id = reference_data()
            .job_skill_tree_by_id(id)
            .and_then(|tree| tree.skills.iter().find(|skill| skill.name == "SM_BASH"))
            .map(|skill| skill.skill_id as u32)
            .expect("Knight tree includes inherited Swordsman-line skill");
        assert!(detail.iter().any(|line| line.starts_with(&format!("@guide:skill:{bash_id}|"))));
        let linked_skill = detail
            .iter()
            .find_map(|line| parse_guide_link(line))
            .expect("job skill must be clickable");
        assert_eq!(linked_skill.kind, "skill");
        assert!(resolve_details(&linked_skill).iter().any(|line| line.contains("Skill identifier:")));
    }

    #[test]
    fn guide_quest_details_show_server_hunt_progress_and_known_routes() {
        let quest = QuestEntry {
            quest_id: 42,
            name: "Poring hunt".to_owned(),
            requirements: vec![QuestRequirementEntry {
                item_id: ragnarok_packets::ItemId(501),
                item_name: "Red Potion".to_owned(),
                needed: 2,
            }],
            hunt_objectives: vec![QuestHuntObjectiveEntry {
                monster_id: 1002,
                monster_name: "Poring".to_owned(),
                total_count: 10,
                current_count: 3,
            }],
        };
        let detail = quest_details(&quest);
        assert!(detail.iter().any(|line| line.contains("Poring — 3/10")));
        assert!(detail.iter().any(|line| line.starts_with("@route:")));
        assert!(detail.iter().any(|line| line.starts_with("@guide:monster:")));
        assert!(detail.iter().any(|line| line.starts_with("@guide:item:501|")));
    }

    #[test]
    fn guide_static_quest_details_include_searchable_targets_and_route_links() {
        let data = reference_data();
        let quest = data.quest_by_id(1100).expect("tracked reference quest");
        let detail = quest_reference_details(quest);
        assert!(detail.iter().any(|line| line.contains("bundled hunt reference")));
        assert!(detail.iter().any(|line| line.starts_with("@guide:monster:")) || detail.iter().any(|line| line.starts_with("@route:")));
        assert!(detail.iter().any(|line| line.contains("Quest giver")));

        let quest_link = parse_guide_link("@guide:quest:1100|Open quest").expect("quest links are supported");
        assert_eq!(quest_link.kind, "quest");
        assert!(!resolve_details(&quest_link).is_empty());
    }

    #[test]
    fn guide_quest_with_unsupported_map_falls_back_to_monster_routes() {
        let quest = ReferenceQuest {
            id: 1,
            name: "Test quest".to_owned(),
            targets: vec![ReferenceQuestTarget {
                mob_id: Some(1002),
                monster_name: "Poring".to_owned(),
                monster_data_known: Some(true),
                count: 3,
                level_range: None,
                map_name: Some("not_in_navigation_graph".to_owned()),
            }],
        };
        let detail = quest_reference_details(&quest);
        assert!(detail.iter().any(|line| line.contains("no loaded navigation route")));
        assert!(detail.iter().any(|line| line.starts_with("@guide:monster:1002|")));
    }
}

fn job_names() -> impl Iterator<Item = (u16, &'static str)> {
    JOB_NAMES.lines().filter_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (id, name) = line.split_once('\t')?;
        Some((id.parse().ok()?, name.trim()))
    })
}
