use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

use serde::Deserialize;

pub const DANGER_LEVEL_GAP: u16 = 15;

pub fn is_dangerous_map_level(mean_spawn_level: u16, player_level: u16) -> bool {
    mean_spawn_level.saturating_sub(player_level) >= DANGER_LEVEL_GAP
}

#[derive(Deserialize)]
pub struct NavigationGraph {
    pub maps: Vec<String>,
    pub edges: Vec<NavigationEdge>,
}

#[derive(Deserialize)]
pub struct NavigationEdge {
    pub id: String,
    pub from: NavigationPosition,
    pub to: NavigationPosition,
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

static NAVIGATION_GRAPH: OnceLock<NavigationGraph> = OnceLock::new();

pub fn navigation_graph() -> &'static NavigationGraph {
    NAVIGATION_GRAPH.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/navigation_graph.json"))
            .expect("generated navigation graph must match its embedded schema")
    })
}

/// Return every verified warp on a minimum-hop route to `target_map`.
pub fn route_edges(current_map: &str, target_map: &str) -> Option<Vec<&'static NavigationEdge>> {
    if current_map.eq_ignore_ascii_case(target_map) {
        return Some(Vec::new());
    }

    let graph = navigation_graph();
    let mut outgoing: HashMap<&str, Vec<&NavigationEdge>> = HashMap::new();
    for edge in &graph.edges {
        outgoing.entry(&edge.from.map).or_default().push(edge);
    }

    let mut visited = HashSet::from([current_map.to_ascii_lowercase()]);
    let mut came_from: HashMap<String, &'static NavigationEdge> = HashMap::new();
    let mut queue = VecDeque::from([current_map.to_ascii_lowercase()]);
    let target = target_map.to_ascii_lowercase();

    while let Some(map) = queue.pop_front() {
        let Some(edges) = outgoing.get(map.as_str()) else {
            continue;
        };
        for edge in edges {
            let next = edge.to.map.to_ascii_lowercase();
            if !visited.insert(next.clone()) {
                continue;
            }
            if next == target {
                came_from.insert(next.clone(), edge);
                let mut route = Vec::new();
                let mut cursor = target.clone();
                while cursor != current_map.to_ascii_lowercase() {
                    let route_edge = *came_from.get(&cursor)?;
                    route.push(route_edge);
                    cursor = route_edge.from.map.to_ascii_lowercase();
                }
                route.reverse();
                return Some(route);
            }
            came_from.insert(next.clone(), edge);
            queue.push_back(next);
        }
    }
    None
}

/// Return the first verified warp on a minimum-hop route to `target_map`.
pub fn next_route_edge(current_map: &str, target_map: &str) -> Option<&'static NavigationEdge> {
    route_edges(current_map, target_map)?.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::is_dangerous_map_level;

    #[test]
    fn map_danger_level_uses_the_inclusive_fifteen_level_boundary() {
        assert!(!is_dangerous_map_level(34, 20));
        assert!(is_dangerous_map_level(35, 20));
        assert!(!is_dangerous_map_level(20, 35));
        assert!(!is_dangerous_map_level(u16::MAX, u16::MAX));
    }
}
