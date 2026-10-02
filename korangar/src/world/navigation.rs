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
