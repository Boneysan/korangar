//! Directed warp graph parsed from Hercules warp NPC lines.
//! Never hand-author a player route; edges come from script text.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet, VecDeque};

pub const WARP_GRAPH_SCHEMA: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WarpEdge {
    pub from_map: String,
    pub from_x: u16,
    pub from_y: u16,
    pub to_map: String,
    pub to_x: u16,
    pub to_y: u16,
    pub one_way: bool,
    pub gated: bool,
    pub source: String,
}

#[derive(Clone, Debug, Default)]
pub struct WarpGraph {
    edges: Vec<WarpEdge>,
    dynamic_edges: Vec<DynamicWarpEdge>,
    known_maps: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicWarpEdge {
    pub source: String,
    pub kind: String,
    pub gated: bool,
    pub available: bool,
    pub expression: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteLeg {
    pub from_map: String,
    pub from_x: u16,
    pub from_y: u16,
    pub to_map: String,
    pub to_x: u16,
    pub to_y: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationRoute {
    pub legs: Vec<RouteLeg>,
    pub total_cost: u32,
}

impl WarpGraph {
    pub fn load() -> Result<Self, String> {
        parse_graph_pack(BUNDLED_WARP_GRAPH)
    }

    pub fn edges(&self) -> &[WarpEdge] {
        &self.edges
    }

    pub fn known_maps(&self) -> &HashSet<String> {
        &self.known_maps
    }

    pub fn dynamic_edges(&self) -> &[DynamicWarpEdge] {
        &self.dynamic_edges
    }

    pub fn route_to_objective(
        &self,
        start_map: &str,
        start_x: u16,
        start_y: u16,
        goal_map: &str,
        goal_x: u16,
        goal_y: u16,
    ) -> Option<NavigationRoute> {
        route_to_objective(&self.edges, start_map, start_x, start_y, goal_map, goal_x, goal_y)
    }
}

pub fn parse_warp_line(line: &str) -> Option<WarpEdge> {
    // map,x,y,0<sep>warp<sep>name<sep>sx,sy,dest,dx,dy
    let line = line.trim();
    if line.is_empty() || line.starts_with("//") || !line.contains("warp") {
        return None;
    }
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let loc = tokens.first()?;
    if !loc.contains(',') {
        return None;
    }
    let mut loc_parts = loc.split(',');
    let from_map = loc_parts.next()?.to_owned();
    let from_x = loc_parts.next()?.parse().ok()?;
    let from_y = loc_parts.next()?.parse().ok()?;
    let dest = tokens.last()?;
    let d: Vec<&str> = dest.split(',').collect();
    if d.len() < 5 {
        return None;
    }
    let to_map = d[2].to_owned();
    let to_x = d[3].parse().ok()?;
    let to_y = d[4].parse().ok()?;
    Some(WarpEdge {
        from_map,
        from_x,
        from_y,
        to_map,
        to_x,
        to_y,
        one_way: false,
        gated: false,
        source: String::new(),
    })
}

const BUNDLED_WARP_GRAPH: &str = include_str!("warp_graph.tsv");

pub fn parse_graph_pack(source: &str) -> Result<WarpGraph, String> {
    let mut lines = source.lines();
    let header = lines.next().ok_or("empty warp graph")?;
    let schema = header
        .strip_prefix("# schema=")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or("warp graph missing schema")?;
    if schema != WARP_GRAPH_SCHEMA {
        return Err(format!("incompatible warp graph schema {schema}"));
    }

    let mut edges = Vec::new();
    let mut dynamic_edges = Vec::new();
    let mut known_maps = HashSet::new();
    for (line_number, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.first() == Some(&"@dynamic") {
            if fields.len() != 6 || fields[1].is_empty() || fields[2].is_empty() || fields[5].is_empty() {
                return Err(format!("invalid dynamic warp on line {}", line_number + 2));
            }
            let parse_flag = |field: &str, name: &str| match field {
                "0" => Ok(false),
                "1" => Ok(true),
                _ => Err(format!("bad {name} on line {}", line_number + 2)),
            };
            dynamic_edges.push(DynamicWarpEdge {
                source: fields[1].to_owned(),
                kind: fields[2].to_owned(),
                gated: parse_flag(fields[3], "dynamic gated flag")?,
                available: parse_flag(fields[4], "dynamic availability flag")?,
                expression: fields[5].to_owned(),
            });
            continue;
        }
        if fields.len() != 10 {
            return Err(format!("warp graph line {} has {} fields", line_number + 2, fields.len()));
        }
        let parse_u16 = |field: &str, name: &str| field.parse::<u16>().map_err(|_| format!("bad {name} on line {}", line_number + 2));
        let parse_bool = |field: &str, name: &str| match field {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(format!("bad {name} on line {}", line_number + 2)),
        };
        let edge = WarpEdge {
            from_map: fields[0].to_owned(),
            from_x: parse_u16(fields[1], "source x")?,
            from_y: parse_u16(fields[2], "source y")?,
            to_map: fields[3].to_owned(),
            to_x: parse_u16(fields[4], "destination x")?,
            to_y: parse_u16(fields[5], "destination y")?,
            one_way: parse_bool(fields[6], "one-way flag")?,
            gated: parse_bool(fields[7], "gated flag")?,
            source: fields[9].to_owned(),
        };
        if edge.from_map.is_empty() || edge.to_map.is_empty() || fields[8] != "0" {
            return Err(format!("invalid warp graph edge on line {}", line_number + 2));
        }
        known_maps.insert(edge.from_map.clone());
        known_maps.insert(edge.to_map.clone());
        edges.push(edge);
    }
    validate_graph(&edges, &known_maps)?;
    Ok(WarpGraph {
        edges,
        dynamic_edges,
        known_maps,
    })
}

pub fn parse_warps(script: &str) -> Result<Vec<WarpEdge>, String> {
    let mut edges = Vec::new();
    for line in script.lines() {
        if let Some(edge) = parse_warp_line(line) {
            if edge.from_map.is_empty() || edge.to_map.is_empty() {
                return Err("dangling warp map".into());
            }
            edges.push(edge);
        }
    }
    Ok(edges)
}

pub fn validate_graph(edges: &[WarpEdge], known_maps: &HashSet<String>) -> Result<(), String> {
    for e in edges {
        if !known_maps.is_empty() && (!known_maps.contains(&e.from_map) || !known_maps.contains(&e.to_map)) {
            return Err(format!("dangling map on {} -> {}", e.from_map, e.to_map));
        }
    }
    Ok(())
}

/// BFS route; stable tie-break by destination map name then coordinates.
pub fn route(edges: &[WarpEdge], start: &str, goal: &str) -> Option<Vec<String>> {
    if start == goal {
        return Some(vec![start.to_owned()]);
    }
    let mut adj: HashMap<&str, Vec<&WarpEdge>> = HashMap::new();
    for e in edges {
        adj.entry(e.from_map.as_str()).or_default().push(e);
    }
    for list in adj.values_mut() {
        list.sort_by_key(|e| (e.to_map.as_str(), e.to_x, e.to_y));
    }
    let mut q = VecDeque::new();
    let mut prev: HashMap<String, String> = HashMap::new();
    q.push_back(start.to_owned());
    prev.insert(start.to_owned(), start.to_owned());
    while let Some(map) = q.pop_front() {
        if map == goal {
            break;
        }
        if let Some(list) = adj.get(map.as_str()) {
            for e in list {
                if e.gated {
                    continue;
                }
                prev.entry(e.to_map.clone()).or_insert_with(|| {
                    q.push_back(e.to_map.clone());
                    map.clone()
                });
            }
        }
    }
    if !prev.contains_key(goal) {
        return None;
    }
    let mut path = vec![goal.to_owned()];
    let mut cur = goal.to_owned();
    while cur != start {
        cur = prev.get(&cur)?.clone();
        path.push(cur.clone());
    }
    path.reverse();
    Some(path)
}

/// Compute a deterministic route to an objective coordinate.
///
/// Moving to a portal costs its Chebyshev distance from the current position;
/// crossing a portal costs one; the final leg also includes the distance from
/// the arrival portal to the objective. Unavailable/gated edges are excluded.
pub fn route_to_objective(
    edges: &[WarpEdge],
    start_map: &str,
    start_x: u16,
    start_y: u16,
    goal_map: &str,
    goal_x: u16,
    goal_y: u16,
) -> Option<NavigationRoute> {
    if start_map == goal_map {
        return Some(NavigationRoute {
            legs: Vec::new(),
            total_cost: start_x.abs_diff(goal_x).max(start_y.abs_diff(goal_y)) as u32,
        });
    }

    let mut adjacency: HashMap<&str, Vec<&WarpEdge>> = HashMap::new();
    for edge in edges.iter().filter(|edge| !edge.gated) {
        adjacency.entry(edge.from_map.as_str()).or_default().push(edge);
    }
    for list in adjacency.values_mut() {
        list.sort_by_key(|edge| (edge.to_map.as_str(), edge.to_x, edge.to_y, edge.from_x, edge.from_y));
    }

    let mut queue: Vec<(u32, Vec<String>, String)> = Vec::new();
    let mut best: HashMap<String, (u32, Vec<String>)> = HashMap::new();
    let mut previous: HashMap<String, (String, WarpEdge)> = HashMap::new();
    let start_path = vec![start_map.to_owned()];
    best.insert(start_map.to_owned(), (0, start_path.clone()));
    queue.push((0, start_path, start_map.to_owned()));

    while !queue.is_empty() {
        let index = queue
            .iter()
            .enumerate()
            .min_by_key(|(_, (cost, path, map))| (*cost, path.clone(), map.clone()))
            .map(|(index, _)| index)?;
        let (cost, path, map) = queue.swap_remove(index);
        if best.get(&map) != Some(&(cost, path.clone())) {
            continue;
        }
        let Some(edges_here) = adjacency.get(map.as_str()) else {
            continue;
        };
        for edge in edges_here {
            let approach = if map == start_map {
                start_x.abs_diff(edge.from_x).max(start_y.abs_diff(edge.from_y)) as u32
            } else {
                0
            };
            let arrival = if edge.to_map == goal_map {
                edge.to_x.abs_diff(goal_x).max(edge.to_y.abs_diff(goal_y)) as u32
            } else {
                0
            };
            let next_cost = cost + approach + 1 + arrival;
            let mut next_path = path.clone();
            next_path.push(edge.to_map.clone());
            let should_replace = best
                .get(&edge.to_map)
                .is_none_or(|(old_cost, old_path)| (next_cost, &next_path) < (*old_cost, old_path));
            if should_replace {
                best.insert(edge.to_map.clone(), (next_cost, next_path.clone()));
                previous.insert(edge.to_map.clone(), (map.clone(), (*edge).clone()));
                queue.push((next_cost, next_path, edge.to_map.clone()));
            }
        }
    }

    let (total_cost, _) = best.get(goal_map)?.clone();
    let mut legs = Vec::new();
    let mut current = goal_map.to_owned();
    while current != start_map {
        let (previous_map, edge) = previous.get(&current)?.clone();
        legs.push(RouteLeg {
            from_map: edge.from_map,
            from_x: edge.from_x,
            from_y: edge.from_y,
            to_map: edge.to_map,
            to_x: edge.to_x,
            to_y: edge.to_y,
        });
        current = previous_map;
    }
    legs.reverse();
    Some(NavigationRoute { legs, total_cost })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
prt_fild01,136,373,0	warp	prtf030	1,1,prt_maze01,99,31
prt_maze01,18,21,0	warp	maze1	2,2,prt_maze02,15,20
prontera,156,191,0	warp	prt_west	4,2,prt_fild05,367,205
";

    #[test]
    fn parses_real_warp_syntax() {
        let edges = parse_warps(FIXTURE).unwrap();
        assert!(edges.iter().any(|e| e.from_map == "prt_fild01" && e.to_map == "prt_maze01"));
        assert!(edges.iter().any(|e| e.to_map == "prt_maze02"));
    }

    #[test]
    fn rejects_dangling_maps() {
        let edges = parse_warps(FIXTURE).unwrap();
        let mut known = HashSet::new();
        known.insert("prontera".into());
        assert!(validate_graph(&edges, &known).is_err());
    }

    #[test]
    fn route_prontera_field_to_maze() {
        let extra = "\
prt_fild05,1,1,0	warp	a	1,1,prt_fild01,2,2
prt_fild01,136,373,0	warp	prtf030	1,1,prt_maze01,99,31
prt_maze01,18,21,0	warp	maze1	2,2,prt_maze02,15,20
";
        let edges = parse_warps(extra).unwrap();
        let path = route(&edges, "prt_fild05", "prt_maze02").unwrap();
        assert_eq!(path, vec!["prt_fild05", "prt_fild01", "prt_maze01", "prt_maze02"]);
        assert!(route(&edges, "prt_maze02", "prontera").is_none());
    }

    #[test]
    fn one_way_and_gated() {
        let mut edges = parse_warps("a,1,1,0	warp	x	1,1,b,2,2\nb,2,2,0	warp	y	1,1,c,3,3\n").unwrap();
        edges[0].one_way = true;
        edges[1].gated = true;
        assert_eq!(route(&edges, "a", "b").unwrap(), vec!["a", "b"]);
        assert!(route(&edges, "a", "c").is_none());
    }

    #[test]
    fn graph_pack_rejects_bad_schema_and_preserves_metadata() {
        let pack = "# schema=1\na\t1\t2\tb\t3\t4\t1\t1\t0\tnpc.txt:7";
        let graph = parse_graph_pack(pack).unwrap();
        assert_eq!(graph.edges()[0].source, "npc.txt:7");
        assert!(graph.edges()[0].one_way);
        assert!(graph.edges()[0].gated);
        assert!(parse_graph_pack("# schema=9\n").is_err());
    }

    #[test]
    fn graph_pack_preserves_dynamic_edges_without_faking_coordinates() {
        let pack = "# schema=1\n# dynamic\n# \
                    tag\tsource\tkind\tgated\tavailable\texpression\n@dynamic\tnpc.txt:9\tinstance\t1\t0\twarp(.@map$, .@x, .@y)";
        let graph = parse_graph_pack(pack).unwrap();
        assert_eq!(graph.dynamic_edges()[0].kind, "instance");
        assert!(!graph.dynamic_edges()[0].available);
        assert!(graph.edges().is_empty());
    }

    #[test]
    fn objective_route_chooses_stable_cheapest_route_and_excludes_gates() {
        let edges = vec![
            edge("a", 5, 5, "b", 10, 10, false),
            edge("b", 10, 10, "goal", 2, 2, false),
            edge("a", 1, 1, "c", 3, 3, false),
            edge("c", 3, 3, "goal", 2, 2, false),
            edge("a", 1, 1, "blocked", 1, 1, true),
        ];
        let route = route_to_objective(&edges, "a", 1, 1, "goal", 5, 5).unwrap();
        assert_eq!(route.legs.iter().map(|leg| leg.to_map.as_str()).collect::<Vec<_>>(), vec![
            "c", "goal"
        ]);
        assert!(route_to_objective(&edges, "missing", 1, 1, "goal", 5, 5).is_none());
    }

    #[test]
    fn labyrinth_acceptance_recomputes_after_wrong_portal_teleport_and_respawn() {
        let edges = vec![
            edge("prontera", 156, 191, "prt_fild05", 367, 205, false),
            edge("prt_fild05", 1, 1, "prt_fild01", 136, 373, false),
            edge("prt_fild05", 300, 300, "prt_fild02", 20, 20, false),
            edge("prt_fild02", 20, 20, "prt_fild01", 136, 373, false),
            edge("prt_fild01", 136, 373, "prt_maze01", 99, 31, false),
            edge("prt_maze01", 18, 21, "prt_maze02", 15, 20, false),
        ];

        let intended = route_to_objective(&edges, "prontera", 150, 190, "prt_maze02", 15, 20).unwrap();
        assert_eq!(intended.legs.iter().map(|leg| leg.to_map.as_str()).collect::<Vec<_>>(), [
            "prt_fild05",
            "prt_fild01",
            "prt_maze01",
            "prt_maze02",
        ]);
        assert_eq!(
            intended
                .legs
                .iter()
                .map(|leg| (
                    leg.from_map.as_str(),
                    leg.from_x,
                    leg.from_y,
                    leg.to_map.as_str(),
                    leg.to_x,
                    leg.to_y
                ))
                .collect::<Vec<_>>(),
            vec![
                ("prontera", 156, 191, "prt_fild05", 367, 205),
                ("prt_fild05", 1, 1, "prt_fild01", 136, 373),
                ("prt_fild01", 136, 373, "prt_maze01", 99, 31),
                ("prt_maze01", 18, 21, "prt_maze02", 15, 20),
            ]
        );

        let wrong_portal = route_to_objective(&edges, "prt_fild02", 20, 20, "prt_maze02", 15, 20).unwrap();
        assert_eq!(wrong_portal.legs[0].from_map, "prt_fild02");
        assert_eq!(wrong_portal.legs.last().unwrap().to_map, "prt_maze02");

        let teleported = route_to_objective(&edges, "prt_maze01", 40, 40, "prt_maze02", 15, 20).unwrap();
        assert_eq!(teleported.legs.len(), 1);
        assert_eq!(teleported.legs[0].from_map, "prt_maze01");
        assert_eq!((teleported.legs[0].from_x, teleported.legs[0].from_y), (18, 21));
        let respawned = route_to_objective(&edges, "prt_fild01", 136, 373, "prt_maze02", 15, 20).unwrap();
        assert_eq!(respawned.legs[0].to_map, "prt_maze01");
        assert_eq!((respawned.legs[0].from_x, respawned.legs[0].from_y), (136, 373));
    }

    fn edge(from_map: &str, from_x: u16, from_y: u16, to_map: &str, to_x: u16, to_y: u16, gated: bool) -> WarpEdge {
        WarpEdge {
            from_map: from_map.into(),
            from_x,
            from_y,
            to_map: to_map.into(),
            to_x,
            to_y,
            one_way: true,
            gated,
            source: "fixture".into(),
        }
    }
}
