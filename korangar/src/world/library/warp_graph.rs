//! Directed warp graph parsed from Hercules warp NPC lines.
//! Never hand-author a player route; edges come from script text.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet, VecDeque};

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
}
