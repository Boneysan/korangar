use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use korangar_interface::element::StateElement;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

pub const DANGER_LEVEL_GAP: u16 = 15;

pub fn is_dangerous_map_level(mean_spawn_level: u16, player_level: u16) -> bool {
    mean_spawn_level.saturating_sub(player_level) >= DANGER_LEVEL_GAP
}

#[derive(Deserialize)]
pub struct NavigationGraph {
    pub maps: Vec<String>,
    pub edges: Vec<NavigationEdge>,
    /// Why a graph map cannot be reached from Prontera, keyed by map name
    /// (`tools/generate_navigation_graph.py` `access_notes`).
    #[serde(default)]
    pub access_notes: HashMap<String, AccessNote>,
}

#[derive(Deserialize)]
pub struct AccessNote {
    /// `unreviewed_script_entrance`, `behind_unreviewed_entrance`, or
    /// `no_known_entrance`.
    pub kind: String,
    /// For `behind_unreviewed_entrance`: the gated map this one is behind.
    #[serde(default)]
    pub via: Option<String>,
    #[serde(default)]
    pub entrances: Vec<ScriptEntrance>,
    #[serde(default)]
    pub entrance_count: usize,
    /// Character variables the entrances check, with the NPCs most likely to
    /// assign a passing value.
    #[serde(default)]
    pub variables: std::collections::BTreeMap<String, Vec<VariableSetter>>,
}

/// A placed NPC that assigns a literal value to a character variable.
#[derive(Deserialize)]
pub struct VariableSetter {
    pub npc: String,
    pub map: String,
    pub x: u16,
    pub y: u16,
    /// The assignment as written, e.g. `= 3` or `|= 8192`.
    pub sets: String,
    pub source: String,
}

/// A literal `warp` call found in a loaded NPC script. Its conditions have
/// not been reviewed; it only says who can send a player there.
#[derive(Deserialize)]
pub struct ScriptEntrance {
    pub npc: String,
    #[serde(default)]
    pub npc_map: Option<String>,
    pub source: String,
    /// Gate-looking `if` conditions before the warp in the same NPC script.
    #[serde(default)]
    pub conditions: Vec<String>,
}

#[derive(Deserialize)]
pub struct NavigationEdge {
    pub id: String,
    #[serde(default = "default_edge_kind")]
    pub kind: String,
    #[serde(default)]
    pub availability: String,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub requirements: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    /// Zeny the hop costs (0 for walk warps and free services).
    #[serde(default)]
    pub fare_zeny: u32,
    /// Why a player may be refused this hop (a quest, an item, marriage).
    #[serde(default)]
    pub locked_by: Option<String>,
    /// Ordered steps that unlock the hop, reviewed from the server script.
    #[serde(default)]
    pub unlock_steps: Vec<String>,
    pub from: NavigationPosition,
    pub to: NavigationPosition,
}

fn default_edge_kind() -> String {
    "walk_warp".to_owned()
}

#[derive(Deserialize)]
pub struct NavigationPosition {
    pub map: String,
    pub x: u16,
    pub y: u16,
    #[serde(default)]
    pub width: u16,
    #[serde(default)]
    pub height: u16,
}

/// Resolve a hovered tile to a verified outgoing walk-warp destination. The
/// optional route target marks only the first walk-warp edge on its route.
pub fn portal_label<'a>(
    edges: &'a [NavigationEdge],
    current_map: &str,
    x: u16,
    y: u16,
    route_target: Option<&str>,
) -> Option<(&'a str, bool)> {
    let route_edge = route_target
        .and_then(|target| route_edges(current_map, target))
        .and_then(|route| route.into_iter().next());

    edges.iter().find_map(|edge| {
        if edge.kind != "walk_warp" {
            return None;
        }
        let from = &edge.from;
        let inside = from.map.eq_ignore_ascii_case(current_map)
            && x >= from.x
            && y >= from.y
            && x < from.x.saturating_add(from.width.max(1))
            && y < from.y.saturating_add(from.height.max(1));
        inside.then(|| {
            (
                edge.to.map.as_str(),
                route_edge.is_some_and(|route_edge| route_edge.id == edge.id),
            )
        })
    })
}

/// A verified walk-warp portal exit on a map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapPortalExit {
    pub from_map: String,
    pub to_map: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub is_route_exit: bool,
}

impl MapPortalExit {
    pub fn label(&self) -> String {
        if self.is_route_exit {
            format!("→ Route portal: {}", self.to_map)
        } else {
            format!("Portal to {}", self.to_map)
        }
    }

    #[allow(dead_code)]
    pub fn center_tile(&self) -> (u16, u16) {
        (self.x.saturating_add(self.width / 2), self.y.saturating_add(self.height / 2))
    }
}

/// Enumerate all verified outgoing walk-warp portal exits on `current_map`.
///
/// If `route_target` is provided, the next exit leading along the tracked route
/// is flagged with `is_route_exit: true`.
///
/// Safety guarantees:
/// - Only verified, active walk-warp edges in `navigation_graph` are returned.
/// - NPC services (e.g. ferries, cat fleet expeditions) are distinct travel
///   legs and are never returned as walk portals.
/// - Inactive, disabled, or unindexed exits are never hallucinated or exposed.
pub fn map_portal_exits(current_map: &str, route_target: Option<&str>) -> Vec<MapPortalExit> {
    let graph = navigation_graph();
    let next_edge = route_target.and_then(|target| next_route_edge(current_map, target));

    let mut exits = Vec::new();
    for edge in &graph.edges {
        if edge.kind != "walk_warp" {
            continue;
        }
        if !edge.from.map.eq_ignore_ascii_case(current_map) {
            continue;
        }
        let is_route_exit = next_edge.is_some_and(|next| next.id == edge.id);
        exits.push(MapPortalExit {
            from_map: edge.from.map.clone(),
            to_map: edge.to.map.clone(),
            x: edge.from.x,
            y: edge.from.y,
            width: edge.from.width,
            height: edge.from.height,
            is_route_exit,
        });
    }

    exits.sort_by(|a, b| a.to_map.cmp(&b.to_map).then_with(|| a.x.cmp(&b.x)));
    exits
}

static NAVIGATION_GRAPH: OnceLock<NavigationGraph> = OnceLock::new();

pub fn navigation_graph() -> &'static NavigationGraph {
    NAVIGATION_GRAPH.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/navigation_graph.json"))
            .expect("generated navigation graph must match its embedded schema")
    })
}

/// What the route finder minimises first, like a GPS route option. Ties are
/// broken by the remaining criteria so a route stays deterministic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, RustState, StateElement)]
pub enum RoutePreference {
    /// Fewest map transitions (then shortest walk, then cheapest).
    #[default]
    FewestMaps,
    /// Least zeny spent on fares (then fewest maps, then shortest walk).
    Cheapest,
    /// Shortest approximate walk; teleports and boats count as no walking
    /// (then fewest maps, then cheapest).
    ShortestWalk,
    /// Fewest locked steps (quest, item, or marriage gates), then fewest maps.
    AvoidLocked,
}

impl RoutePreference {
    pub const ALL: [RoutePreference; 4] = [
        RoutePreference::FewestMaps,
        RoutePreference::Cheapest,
        RoutePreference::ShortestWalk,
        RoutePreference::AvoidLocked,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RoutePreference::FewestMaps => "Fewest maps",
            RoutePreference::Cheapest => "Cheapest",
            RoutePreference::ShortestWalk => "Shortest walk",
            RoutePreference::AvoidLocked => "Avoid locked steps",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|preference| *preference == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    fn index(self) -> u8 {
        Self::ALL.iter().position(|preference| *preference == self).unwrap_or(0) as u8
    }

    fn key(self, totals: RouteSummary) -> (u64, u64, u64, u64) {
        let hops = totals.hops as u64;
        let zeny = u64::from(totals.zeny);
        let walk = u64::from(totals.walk_cells);
        let locked = totals.locked_hops as u64;
        match self {
            RoutePreference::FewestMaps => (hops, walk, zeny, locked),
            RoutePreference::Cheapest => (zeny, hops, walk, locked),
            RoutePreference::ShortestWalk => (walk, hops, zeny, locked),
            RoutePreference::AvoidLocked => (locked, hops, walk, zeny),
        }
    }
}

/// The preference every route query uses. Process-wide like the graph itself:
/// the minimap, world map, Guide, and breadcrumbs must all agree on one route.
static ROUTE_PREFERENCE: AtomicU8 = AtomicU8::new(0);

pub fn set_route_preference(preference: RoutePreference) {
    ROUTE_PREFERENCE.store(preference.index(), Ordering::Relaxed);
}

pub fn route_preference() -> RoutePreference {
    RoutePreference::ALL[usize::from(ROUTE_PREFERENCE.load(Ordering::Relaxed)) % RoutePreference::ALL.len()]
}

/// Totals for one route, for labels like "3 maps · 1,200 z · ~180 cells".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RouteSummary {
    pub hops: usize,
    pub zeny: u32,
    /// Straight-line walking inside each map between where a hop arrives and
    /// where the next one starts, in cells (diagonals count about 1.4). Walls
    /// are ignored, so this is an estimate.
    pub walk_cells: u32,
    pub locked_hops: usize,
}

/// Best arrival on every map reachable from one origin, under one preference.
struct RouteTree {
    /// Map (lowercase) -> index of the graph edge the best route arrives by.
    arrival: HashMap<String, usize>,
    /// Edge index -> the edge before it on its best route (`None` = first hop).
    parent: HashMap<usize, Option<usize>>,
    totals: HashMap<usize, RouteSummary>,
}

fn walk_cells(from: &NavigationPosition, to: &NavigationPosition) -> u32 {
    let dx = u32::from(from.x.abs_diff(to.x));
    let dy = u32::from(from.y.abs_diff(to.y));
    // Octile distance: straight steps cost 1, diagonal steps about 1.4.
    dx.max(dy) + (dx.min(dy) * 4 + 5) / 10
}

/// Dijkstra over hops: a node is "arrived through edge e", so the walk to the
/// next hop starts where `e` lands rather than at an arbitrary map center.
fn route_tree(origin: &str, preference: RoutePreference) -> RouteTree {
    let graph = navigation_graph();
    let mut outgoing: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        outgoing.entry(edge.from.map.to_ascii_lowercase()).or_default().push(index);
    }

    let mut tree = RouteTree {
        arrival: HashMap::new(),
        parent: HashMap::new(),
        totals: HashMap::new(),
    };
    let mut heap = BinaryHeap::new();
    let hop_totals = |previous: RouteSummary, walk: u32, edge: &NavigationEdge| RouteSummary {
        hops: previous.hops + 1,
        zeny: previous.zeny.saturating_add(edge.fare_zeny),
        walk_cells: previous.walk_cells.saturating_add(walk),
        locked_hops: previous.locked_hops + usize::from(edge.locked_by.is_some()),
    };
    for &index in outgoing.get(origin).into_iter().flatten() {
        let totals = hop_totals(RouteSummary::default(), 0, &graph.edges[index]);
        heap.push(Reverse((
            preference.key(totals),
            index,
            None::<usize>,
            totals.hops,
            totals.zeny,
            totals.walk_cells,
            totals.locked_hops,
        )));
    }

    while let Some(Reverse((_, index, parent, hops, zeny, walk, locked))) = heap.pop() {
        if tree.parent.contains_key(&index) {
            continue;
        }
        let totals = RouteSummary {
            hops,
            zeny,
            walk_cells: walk,
            locked_hops: locked,
        };
        tree.parent.insert(index, parent);
        tree.totals.insert(index, totals);
        let edge = &graph.edges[index];
        let arrived = edge.to.map.to_ascii_lowercase();
        if arrived == origin {
            continue;
        }
        tree.arrival.entry(arrived.clone()).or_insert(index);
        for &next in outgoing.get(&arrived).into_iter().flatten() {
            if tree.parent.contains_key(&next) {
                continue;
            }
            let next_edge = &graph.edges[next];
            let next_totals = hop_totals(totals, walk_cells(&edge.to, &next_edge.from), next_edge);
            heap.push(Reverse((
                preference.key(next_totals),
                next,
                Some(index),
                next_totals.hops,
                next_totals.zeny,
                next_totals.walk_cells,
                next_totals.locked_hops,
            )));
        }
    }
    tree
}

fn with_route_tree<T>(origin: &str, preference: RoutePreference, f: impl FnOnce(&RouteTree) -> T) -> T {
    static CACHE: Mutex<Option<HashMap<(String, RoutePreference), RouteTree>>> = Mutex::new(None);
    const MAX_CACHED_TREES: usize = 8;

    let key = (origin.to_ascii_lowercase(), preference);
    let mut cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let trees = cache.get_or_insert_with(HashMap::new);
    if !trees.contains_key(&key) {
        if trees.len() >= MAX_CACHED_TREES {
            trees.clear();
        }
        let tree = route_tree(&key.0, key.1);
        trees.insert(key.clone(), tree);
    }
    f(&trees[&key])
}

/// Every verified hop on the best route to `target_map` under the current
/// [`RoutePreference`].
pub fn route_edges(current_map: &str, target_map: &str) -> Option<Vec<&'static NavigationEdge>> {
    route_edges_with(current_map, target_map, route_preference())
}

/// [`route_edges`] under an explicit preference.
pub fn route_edges_with(current_map: &str, target_map: &str, preference: RoutePreference) -> Option<Vec<&'static NavigationEdge>> {
    if current_map.eq_ignore_ascii_case(target_map) {
        return Some(Vec::new());
    }
    let graph = navigation_graph();
    with_route_tree(current_map, preference, |tree| {
        let mut cursor = Some(*tree.arrival.get(&target_map.to_ascii_lowercase())?);
        let mut route = Vec::new();
        while let Some(index) = cursor {
            route.push(&graph.edges[index]);
            cursor = tree.parent[&index];
        }
        route.reverse();
        Some(route)
    })
}

/// Totals for the best route to `target_map` (`hops == 0` for the same map),
/// or `None` when the graph has no route.
pub fn route_summary(current_map: &str, target_map: &str) -> Option<RouteSummary> {
    route_summary_with(current_map, target_map, route_preference())
}

/// [`route_summary`] under an explicit preference.
pub fn route_summary_with(current_map: &str, target_map: &str, preference: RoutePreference) -> Option<RouteSummary> {
    if current_map.eq_ignore_ascii_case(target_map) {
        return Some(RouteSummary::default());
    }
    with_route_tree(current_map, preference, |tree| {
        let index = tree.arrival.get(&target_map.to_ascii_lowercase())?;
        tree.totals.get(index).copied()
    })
}

/// Return the first verified warp on a minimum-hop route to `target_map`.
pub fn next_route_edge(current_map: &str, target_map: &str) -> Option<&'static NavigationEdge> {
    route_edges(current_map, target_map)?.into_iter().next()
}

/// Player-facing lines for every locked hop on the route from `current_map`
/// to `target_map`: what locks it and the reviewed steps that open it.
pub fn route_lock_notes(current_map: &str, target_map: &str) -> Vec<String> {
    let Some(route) = route_edges(current_map, target_map) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for edge in route {
        let Some(locked_by) = &edge.locked_by else {
            continue;
        };
        lines.push(format!("Locked step {} → {}: {locked_by}", edge.from.map, edge.to.map));
        lines.extend(
            edge.unlock_steps
                .iter()
                .enumerate()
                .map(|(index, step)| format!("  {}. {step}", index + 1)),
        );
    }
    lines
}

/// Why `target_map` has no route from `current_map`, as player-facing lines.
pub fn unreachable_explanation(current_map: &str, target_map: &str) -> Vec<String> {
    if route_hop_count("prontera", target_map).is_some() {
        return vec![format!(
            "{target_map}: no route is known from {current_map}. Return to a town first; {target_map} can be routed to from Prontera."
        )];
    }

    let graph = navigation_graph();
    let describe = |entrance: &ScriptEntrance| match &entrance.npc_map {
        Some(map) => format!("  • {} on {map} ({})", entrance.npc, entrance.source),
        None => format!("  • script {} ({})", entrance.npc, entrance.source),
    };
    let Some(note) = graph.access_notes.get(&target_map.to_ascii_lowercase()) else {
        return vec![format!(
            "{target_map}: no loaded warp, travel NPC, or script names this map as a destination. It may be an instance, guild, event, or \
             job-test map, or a script may choose it at run time."
        )];
    };

    let mut lines = Vec::new();
    match note.kind.as_str() {
        "unreviewed_script_entrance" => {
            lines.push(format!(
                "{target_map}: only reached through an NPC script whose conditions have not been reviewed yet (often a quest or an item \
                 check). Known entrances:"
            ));
        }
        "behind_unreviewed_entrance" => {
            let via = note.via.as_deref().unwrap_or("another map");
            lines.push(format!(
                "{target_map}: reached on foot from {via}, which is only entered through an NPC script whose conditions have not been \
                 reviewed yet. Entrances to {via}:"
            ));
        }
        _ => {
            lines.push(format!(
                "{target_map}: no loaded warp, travel NPC, or script names this map as a destination. It may be an instance, guild, \
                 event, or job-test map, or a script may choose it at run time."
            ));
        }
    }
    for entrance in &note.entrances {
        lines.push(describe(entrance));
        if !entrance.conditions.is_empty() {
            lines.push(format!("      checks: {}", entrance.conditions.join("; ")));
        }
    }
    if note.entrance_count > note.entrances.len() {
        lines.push(format!("  … and {} more", note.entrance_count - note.entrances.len()));
    }
    if !note.variables.is_empty() {
        lines.push("Quest progress these checks read, and NPCs that set it:".to_owned());
        for (variable, setters) in &note.variables {
            let where_set = setters
                .iter()
                .map(|setter| format!("{} ({} {}, {}) {}", setter.npc, setter.map, setter.x, setter.y, setter.sets))
                .collect::<Vec<_>>()
                .join("; ");
            lines.push(format!("  • {variable}: {where_set}"));
        }
    }
    lines
}

/// Number of map transitions on the best route (`Some(0)` for the same map).
pub fn route_hop_count(current_map: &str, target_map: &str) -> Option<usize> {
    route_summary(current_map, target_map).map(|summary| summary.hops)
}

#[cfg(test)]
mod route_preference_tests {
    use super::{RoutePreference, route_edges_with, route_summary_with};

    fn summary(from: &str, to: &str, preference: RoutePreference) -> super::RouteSummary {
        route_summary_with(from, to, preference).unwrap_or_else(|| panic!("{from} reaches {to}"))
    }

    #[test]
    fn each_preference_minimises_its_own_measure() {
        for (from, to) in [("prontera", "payon"), ("prontera", "spl_fild02"), ("izlude", "aldebaran")] {
            let fewest = summary(from, to, RoutePreference::FewestMaps);
            let cheapest = summary(from, to, RoutePreference::Cheapest);
            let walk = summary(from, to, RoutePreference::ShortestWalk);
            let unlocked = summary(from, to, RoutePreference::AvoidLocked);
            for other in [cheapest, walk, unlocked] {
                assert!(fewest.hops <= other.hops, "{from}->{to}: {fewest:?} vs {other:?}");
            }
            for other in [fewest, walk, unlocked] {
                assert!(cheapest.zeny <= other.zeny, "{from}->{to}: {cheapest:?} vs {other:?}");
                assert!(
                    unlocked.locked_hops <= other.locked_hops,
                    "{from}->{to}: {unlocked:?} vs {other:?}"
                );
            }
            for other in [fewest, cheapest, unlocked] {
                assert!(walk.walk_cells <= other.walk_cells, "{from}->{to}: {walk:?} vs {other:?}");
            }
        }
    }

    #[test]
    fn cheapest_walks_where_the_fewest_maps_route_pays_a_kafra() {
        let fewest = summary("prontera", "payon", RoutePreference::FewestMaps);
        let cheapest = summary("prontera", "payon", RoutePreference::Cheapest);
        assert!(fewest.zeny > 0, "the one-hop route is the paid Kafra teleport: {fewest:?}");
        assert_eq!(cheapest.zeny, 0, "Payon is walkable from Prontera: {cheapest:?}");
        assert!(cheapest.hops > fewest.hops);
    }

    #[test]
    fn avoiding_locks_takes_the_paid_cat_instead_of_the_gated_camp_guard() {
        let route = route_edges_with("mid_camp", "spl_fild02", RoutePreference::AvoidLocked).expect("route");
        assert!(
            route.iter().all(|edge| edge.locked_by.is_none()),
            "{:?}",
            route.iter().map(|edge| &edge.id).collect::<Vec<_>>()
        );
        assert_eq!(summary("mid_camp", "spl_fild02", RoutePreference::AvoidLocked).locked_hops, 0);
    }

    #[test]
    fn a_summary_matches_the_route_it_describes() {
        for preference in RoutePreference::ALL {
            let route = route_edges_with("prontera", "eclage", preference).expect("route");
            let totals = summary("prontera", "eclage", preference);
            assert_eq!(totals.hops, route.len());
            assert_eq!(totals.zeny, route.iter().map(|edge| edge.fare_zeny).sum::<u32>());
            assert_eq!(totals.locked_hops, route.iter().filter(|edge| edge.locked_by.is_some()).count());
        }
    }

    #[test]
    fn preferences_cycle_through_all_four() {
        let mut preference = RoutePreference::FewestMaps;
        for expected in [
            RoutePreference::Cheapest,
            RoutePreference::ShortestWalk,
            RoutePreference::AvoidLocked,
            RoutePreference::FewestMaps,
        ] {
            preference = preference.next();
            assert_eq!(preference, expected);
        }
    }
}

#[cfg(test)]
mod route_hop_count_tests {
    use super::{route_edges, route_hop_count, route_lock_notes, unreachable_explanation};

    #[test]
    fn a_route_through_a_locked_hop_lists_the_lock_and_its_reviewed_steps() {
        let notes = route_lock_notes("prontera", "eclage");
        // Eclage itself can only be entered through its registration gate.
        assert!(
            notes
                .iter()
                .any(|line| line.starts_with("Locked step ecl_fild01 → eclage: Eclage entry registration")),
            "{notes:#?}"
        );
        assert!(
            notes.iter().any(|line| line == "  2. Wait 30 seconds (quest 11310's time limit)."),
            "{notes:#?}"
        );

        // An ordinary walk/Kafra route has nothing to unlock.
        assert!(route_lock_notes("prontera", "izlude").is_empty());
    }

    #[test]
    fn every_kind_of_unreachable_map_says_why() {
        let unreviewed = unreachable_explanation("prontera", "tha_t03");
        assert!(
            unreviewed[0].contains("NPC script whose conditions have not been reviewed"),
            "{unreviewed:#?}"
        );
        assert!(
            unreviewed.iter().any(|line| line.contains("3rdf_warp on tha_t02")),
            "{unreviewed:#?}"
        );

        let behind = unreachable_explanation("prontera", "tha_t04");
        assert!(behind[0].starts_with("tha_t04: reached on foot from tha_t03"), "{behind:#?}");
        assert!(
            behind.iter().any(|line| line == "      checks: thana_tower == 0"),
            "{behind:#?}"
        );
        assert!(
            behind
                .iter()
                .any(|line| line.starts_with("  • thana_tower: Guide (tha_t01 149, 78) = 1")),
            "the NPC that starts the tower quest is named: {behind:#?}"
        );

        let unknown = unreachable_explanation("prontera", "gld_dun01");
        assert!(unknown[0].contains("no loaded warp, travel NPC, or script"), "{unknown:#?}");

        // Reachable from Prontera but not from an isolated map.
        let elsewhere = unreachable_explanation("gld_dun01", "izlude");
        assert!(elsewhere[0].contains("can be routed to from Prontera"), "{elsewhere:#?}");
    }

    #[test]
    fn hop_count_matches_the_route_the_navigator_would_follow() {
        let route = route_edges("prontera", "izlude").expect("prontera reaches izlude");
        assert_eq!(route_hop_count("prontera", "izlude"), Some(route.len()));
        assert_eq!(route_hop_count("PRONTERA", "prontera"), Some(0));
    }

    #[test]
    fn a_map_in_another_graph_component_has_no_route() {
        // A guild dungeon: WoE is disabled and no loaded script warps there.
        assert!(route_edges("prontera", "gld_dun01").is_none());
        assert_eq!(route_hop_count("prontera", "gld_dun01"), None);
    }

    #[test]
    fn changing_origin_recomputes_instead_of_reusing_the_old_distances() {
        let from_prontera = route_hop_count("prontera", "izlude");
        let from_izlude = route_hop_count("izlude", "izlude");
        assert_eq!(from_izlude, Some(0));
        assert_eq!(route_hop_count("prontera", "izlude"), from_prontera);
        assert_eq!(
            route_hop_count("izlude", "prontera"),
            route_edges("izlude", "prontera").map(|route| route.len())
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{is_dangerous_map_level, map_portal_exits, navigation_graph, portal_label, route_edges};

    #[test]
    fn map_danger_level_uses_the_inclusive_fifteen_level_boundary() {
        assert!(!is_dangerous_map_level(34, 20));
        assert!(is_dangerous_map_level(35, 20));
        assert!(!is_dangerous_map_level(20, 35));
        assert!(!is_dangerous_map_level(u16::MAX, u16::MAX));
    }

    #[test]
    fn hovered_known_portal_names_destination_and_marks_the_next_route_exit() {
        let graph = navigation_graph();
        let edge = graph
            .edges
            .iter()
            .find(|edge| {
                edge.kind == "walk_warp" && super::next_route_edge(&edge.from.map, &edge.to.map).is_some_and(|next| next.id == edge.id)
            })
            .expect("navigation graph has direct route edges");

        assert_eq!(
            portal_label(&graph.edges, &edge.from.map, edge.from.x, edge.from.y, Some(&edge.to.map)),
            Some((edge.to.map.as_str(), true)),
        );

        let outside = edge.from.x.saturating_add(edge.from.width.max(1));
        assert_eq!(
            portal_label(&graph.edges, &edge.from.map, outside, edge.from.y, Some(&edge.to.map)),
            None,
        );
        assert_eq!(
            portal_label(&graph.edges, &edge.from.map, edge.from.x, edge.from.y, Some(&edge.from.map)),
            Some((edge.to.map.as_str(), false)),
        );
    }

    #[test]
    fn izlude_route_includes_conditional_ferry_service_then_dungeon_warp() {
        let graph = navigation_graph();
        let route = route_edges("izlude", "iz_dun00").expect("authored ferry route");

        assert_eq!(route.len(), 2);
        assert_eq!(route[0].id, "service-izlude-byalan-ferry");
        assert_eq!(route[0].kind, "npc_service");
        assert_eq!(route[0].availability, "conditional");
        assert_eq!(route[0].requirements.as_deref(), Some("Costs 150 zeny."));
        assert_eq!(route[1].kind, "walk_warp");

        assert_eq!(
            graph.maps.iter().find(|map| map.as_str() == "izlu2dun").map(String::as_str),
            Some("izlu2dun")
        );
    }

    #[test]
    fn service_steps_are_not_portals_and_the_post_ferry_warp_is_the_next_exit() {
        let graph = navigation_graph();
        assert_eq!(
            portal_label(&graph.edges, "izlude", 197, 205, Some("iz_dun00")),
            None,
            "an NPC service is a route step, not a walk warp entity"
        );

        let next = super::next_route_edge("izlu2dun", "iz_dun00").expect("dungeon entrance warp");
        assert_eq!(next.kind, "walk_warp");
        assert_eq!(
            portal_label(&graph.edges, "izlu2dun", next.from.x, next.from.y, Some("iz_dun00")),
            Some(("iz_dun00", true))
        );
    }

    #[test]
    fn malangdo_cat_fleet_connects_an_otherwise_unreachable_island() {
        // Before the four service edges below, `malangdo` had no static-warp
        // connection to the overworld at all -- only to its own interiors
        // (mal_in01/02, mal_dun01). Each direction is a single-hop route.
        for (from, to, id) in [
            ("izlude", "malangdo", "service-izlude-malangdo-cat-fleet"),
            ("alberta", "malangdo", "service-alberta-malangdo-cat-fleet"),
            ("malangdo", "izlude", "service-malangdo-izlude-cat-fleet"),
            ("malangdo", "alberta", "service-malangdo-alberta-cat-fleet"),
        ] {
            let route = route_edges(from, to).unwrap_or_else(|| panic!("expected a route from {from} to {to}"));
            assert_eq!(route.len(), 1, "expected a single-hop service route from {from} to {to}");
            assert_eq!(route[0].id, id);
            assert_eq!(route[0].kind, "npc_service");
            assert_eq!(route[0].availability, "conditional");
            assert!(
                route[0].requirements.as_deref().is_some_and(|text| text.contains("1000 zeny")),
                "fare should be documented as up to 1000 zeny, varying with ep13_yong1"
            );
        }
    }

    #[test]
    fn malangdo_service_hops_are_not_portals() {
        let graph = navigation_graph();
        // A service step is an NPC conversation, not a walkable warp entity --
        // it must not be labeled the way a hovered walk-warp tile would be.
        assert_eq!(portal_label(&graph.edges, "izlude", 182, 218, Some("malangdo")), None);
        assert_eq!(portal_label(&graph.edges, "alberta", 200, 151, Some("malangdo")), None);
        assert_eq!(portal_label(&graph.edges, "malangdo", 219, 86, Some("izlude")), None);
        assert_eq!(portal_label(&graph.edges, "malangdo", 219, 86, Some("alberta")), None);
    }

    #[test]
    fn map_portal_exits_pilot_map_prt_fild08_and_tracked_route_accent() {
        let exits = map_portal_exits("prt_fild08", Some("prontera"));
        assert!(!exits.is_empty(), "prt_fild08 must have verified walk-warp portal exits");

        // prt_fild08 connects to prontera, prt_fild07, izlude, moc_fild01
        let to_maps: Vec<&str> = exits.iter().map(|e| e.to_map.as_str()).collect();
        assert!(to_maps.contains(&"prontera"));
        assert!(to_maps.contains(&"prt_fild07"));
        assert!(to_maps.contains(&"izlude"));
        assert!(to_maps.contains(&"moc_fild01"));

        // On route to Prontera, prontera is the next step and receives tracked route
        // accent
        let route_exit = exits.iter().find(|e| e.to_map == "prontera").expect("exit to prontera");
        assert!(route_exit.is_route_exit, "prontera must be accented as route exit");
        assert_eq!(route_exit.label(), "→ Route portal: prontera");

        // Non-route exits are not accented
        let other_exit = exits.iter().find(|e| e.to_map == "izlude").expect("exit to izlude");
        assert!(!other_exit.is_route_exit);
        assert_eq!(other_exit.label(), "Portal to izlude");
    }

    #[test]
    fn map_portal_exits_pilot_map_izlude_excludes_services() {
        let exits = map_portal_exits("izlude", Some("iz_dun00"));

        // izlude has verified walk warps (e.g. to prt_fild08)
        assert!(exits.iter().any(|e| e.to_map == "prt_fild08"));

        // NPC services (ferry to Byalan Island, cat fleet to Malangdo) are distinct
        // travel legs and must never be exposed as walk-warp portal exits
        assert!(!exits.iter().any(|e| e.to_map == "izlu2dun" || e.to_map == "iz_dun00"));
        assert!(!exits.iter().any(|e| e.to_map == "malangdo"));

        // Since the route to iz_dun00 starts with an NPC service, no walk portal is
        // marked as route exit
        assert!(!exits.iter().any(|e| e.is_route_exit));
    }

    #[test]
    fn map_portal_exits_pilot_map_prt_maze01() {
        let exits = map_portal_exits("prt_maze01", None);
        assert!(!exits.is_empty(), "prt_maze01 must have verified portal exits");
        assert!(
            exits.iter().all(|e| !e.is_route_exit),
            "without route target, no portal is route exit"
        );
    }
}
