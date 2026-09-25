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
    use super::{is_dangerous_map_level, navigation_graph, portal_label, route_edges};

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
}
