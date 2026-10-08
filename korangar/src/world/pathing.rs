//! Implements pathfinding algorithms.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use hashbrown::{HashMap, HashSet};
use ragnarok_packets::{AttackRange, TilePosition};

const MOVE_DIAGONAL_COST: usize = 14;
const MOVE_ORTHOGONAL_COST: usize = 10;
/// The maximum size a walkable path can have.
pub const MAX_WALK_PATH_SIZE: usize = 32;
/// Hercules `max_walk_path` in `conf/map/battle/client.conf`. A click whose
/// path is longer than this is dropped by `unit_walk_toxy`.
pub const SERVER_MAX_WALK_PATH: usize = 17;
/// With `OFFICIAL_WALKPATH` (`src/config/core.h`), a click whose straight shot
/// is blocked is dropped above `(max_walk_path / 17) * 14`.
pub const SERVER_OBSTRUCTED_WALK_PATH: usize = 14;

/// Essential trait that is needed to be implements for pathfinding.
pub trait Traversable {
    /// Must return `true` if the position can be walked on.
    fn is_walkable(&self, position: TilePosition) -> bool;

    /// Must return `true` if the position can be shot through.
    fn is_snipeable(&self, position: TilePosition) -> bool;
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct PathNode {
    position: TilePosition,
    f_score: usize,
    g_score: usize,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Pathfinding algorithm for entity map navigation.
#[derive(Default)]
pub struct PathFinder {
    open_set: BinaryHeap<PathNode>,
    closed_set: HashSet<TilePosition>,
    came_from: HashMap<TilePosition, TilePosition>,
    g_scores: HashMap<TilePosition, usize>,
    path: Vec<TilePosition>,
    neighbors: Vec<TilePosition>,
}

impl PathFinder {
    /// Returns the shortest walkable path between start and goal. Uses a simple
    /// A* search algorithm like the legacy client and alternative server
    /// implementations. It must have the same behavior, or else we would
    /// "desync" with our client movement prediction.
    pub fn find_walkable_path(&mut self, map: &impl Traversable, start: TilePosition, goal: TilePosition) -> Option<&[TilePosition]> {
        self.find_walkable_path_in_range(map, start, goal, AttackRange(0))
    }

    /// Returns the shortest walkable path between start and one attack range
    /// away from the goal. Uses a simple A* search algorithm like the legacy
    /// client and alternative server implementations. It must have the same
    /// behavior, or else we would "desync" with our client movement prediction.
    pub fn find_walkable_path_in_range(
        &mut self,
        map: &impl Traversable,
        start: TilePosition,
        goal: TilePosition,
        attack_range: AttackRange,
    ) -> Option<&[TilePosition]> {
        self.find_walkable_path_with_limit(map, start, goal, attack_range, MAX_WALK_PATH_SIZE)
    }

    /// Returns a longer walkable route for visual navigation guidance. This is
    /// not used to issue player movement, so it can retain a full map path.
    pub fn find_navigation_path(&mut self, map: &impl Traversable, start: TilePosition, goal: TilePosition) -> Option<&[TilePosition]> {
        self.find_walkable_path_with_limit(map, start, goal, AttackRange(0), 16_384)
    }

    fn find_walkable_path_with_limit(
        &mut self,
        map: &impl Traversable,
        start: TilePosition,
        goal: TilePosition,
        attack_range: AttackRange,
        max_path_size: usize,
    ) -> Option<&[TilePosition]> {
        self.open_set.clear();
        self.closed_set.clear();
        self.came_from.clear();
        self.g_scores.clear();
        self.path.clear();

        self.open_set.push(PathNode {
            position: start,
            g_score: 0,
            f_score: Self::heuristic(start, goal),
        });
        self.g_scores.insert(start, 0);

        while let Some(current) = self.open_set.pop() {
            if current.position.x.abs_diff(goal.x).max(current.position.y.abs_diff(goal.y)) <= attack_range.0 {
                return match self.reconstruct_path(start, current.position, max_path_size) {
                    true => Some(&self.path),
                    false => None,
                };
            }

            if self.closed_set.contains(&current.position) {
                continue;
            }
            self.closed_set.insert(current.position);

            self.find_neighbors(map, current.position);

            for neighbor in self.neighbors.drain(..) {
                if self.closed_set.contains(&neighbor) {
                    continue;
                }

                let movement_cost = if neighbor.x != current.position.x && neighbor.y != current.position.y {
                    MOVE_DIAGONAL_COST
                } else {
                    MOVE_ORTHOGONAL_COST
                };

                let tentative_g_score = current.g_score + movement_cost;

                if tentative_g_score < self.g_scores.get(&neighbor).copied().unwrap_or(usize::MAX) {
                    self.came_from.insert(neighbor, current.position);
                    self.g_scores.insert(neighbor, tentative_g_score);

                    let h_score = Self::heuristic(neighbor, goal);
                    let f_score = tentative_g_score + h_score;

                    self.open_set.push(PathNode {
                        position: neighbor,
                        g_score: tentative_g_score,
                        f_score,
                    });
                }
            }
        }

        None
    }

    /// Returns the shortest path between start and goal that can be shot
    /// through.
    // TODO: Unused for now.
    #[allow(dead_code)]
    pub fn find_snipable_path(&mut self, map: &impl Traversable, start: TilePosition, goal: TilePosition) -> Option<&[TilePosition]> {
        self.path.clear();

        let mut current_x = start.x as isize;
        let mut current_y = start.y as isize;
        let mut target_x = goal.x as isize;
        let mut target_y = goal.y as isize;

        let mut delta_x = target_x - current_x;
        if delta_x < 0 {
            std::mem::swap(&mut current_x, &mut target_x);
            std::mem::swap(&mut current_y, &mut target_y);
            delta_x = -delta_x;
        }
        let delta_y = target_y - current_y;

        self.path.push(TilePosition {
            x: current_x as u16,
            y: current_y as u16,
        });

        let weight = if delta_x > delta_y.abs() { delta_x } else { delta_y.abs() };

        let mut weight_x = 0;
        let mut weight_y = 0;

        while current_x != target_x || current_y != target_y {
            weight_x += delta_x;
            weight_y += delta_y;

            if weight_x >= weight {
                weight_x -= weight;
                current_x += 1;
            }
            if weight_y >= weight {
                weight_y -= weight;
                current_y += 1;
            } else if weight_y < 0 {
                weight_y += weight;
                current_y -= 1;
            }

            if self.path.len() < MAX_WALK_PATH_SIZE {
                self.path.push(TilePosition {
                    x: current_x as u16,
                    y: current_y as u16,
                });
            } else {
                return None;
            }

            if (current_x != target_x || current_y != target_y)
                && !map.is_snipeable(TilePosition {
                    x: current_x as u16,
                    y: current_y as u16,
                })
            {
                return None;
            }
        }

        Some(&self.path)
    }

    fn heuristic(start: TilePosition, goal: TilePosition) -> usize {
        let dx = (start.x as isize - goal.x as isize).unsigned_abs();
        let dy = (start.y as isize - goal.y as isize).unsigned_abs();
        let manhattan_distance = dx + dy;
        MOVE_ORTHOGONAL_COST * manhattan_distance
    }

    fn find_neighbors(&mut self, map: &impl Traversable, position: TilePosition) {
        let orthogonal_neighbors = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        for (dx, dy) in orthogonal_neighbors {
            let new_x = position.x.wrapping_add_signed(dx);
            let new_y = position.y.wrapping_add_signed(dy);
            let new_position = TilePosition { x: new_x, y: new_y };

            if map.is_walkable(new_position) {
                self.neighbors.push(new_position);
            }
        }

        let diagonal_neighbors = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

        for (dx, dy) in diagonal_neighbors {
            let new_x = position.x.wrapping_add_signed(dx);
            let new_y = position.y.wrapping_add_signed(dy);
            let new_position = TilePosition { x: new_x, y: new_y };

            // Only allow diagonal neighbors when both adjacent orthogonal neighbors are
            // also walkable.
            if map.is_walkable(new_position)
                && map.is_walkable(TilePosition { x: position.x, y: new_y })
                && map.is_walkable(TilePosition { x: new_x, y: position.y })
            {
                self.neighbors.push(new_position);
            }
        }
    }

    fn reconstruct_path(&mut self, start: TilePosition, goal: TilePosition, max_path_size: usize) -> bool {
        let mut current = goal;

        while current != start {
            self.path.push(current);
            current = *self.came_from.get(&current).unwrap();

            if self.path.len() >= max_path_size {
                return false;
            }
        }

        self.path.push(start);
        self.path.reverse();

        true
    }
}

/// Cell a click or a held mouse button may name.
///
/// The cursor cell is what the player asked for. Hercules drops the packet
/// when the walked path is longer than [`SERVER_MAX_WALK_PATH`], and when the
/// straight shot is blocked it drops anything longer than
/// [`SERVER_OBSTRUCTED_WALK_PATH`] (`unit.c`, `OFFICIAL_WALKPATH`). Sending
/// the cursor anyway means a long click never starts. This returns the
/// furthest cell on the walkable route that the server still accepts, so a
/// held button can continue on the next send.
///
/// `None` means there is no step to take: the click is the cell underfoot, or
/// the first step is a wall.
pub fn click_walk_destination(
    finder: &mut PathFinder,
    map: &impl Traversable,
    start: TilePosition,
    clicked: TilePosition,
) -> Option<TilePosition> {
    if start == clicked {
        return None;
    }

    let route = match finder.find_navigation_path(map, start, clicked) {
        Some(path) => path.to_vec(),
        None => straight_walk_prefix(map, start, clicked, SERVER_MAX_WALK_PATH),
    };
    furthest_accepted_step(&route, map)
}

/// Furthest cell of `path` (index 0 is the start) that `unit_walk_toxy` will
/// walk. The cap depends on the straight shot to that cell, matching
/// `path_search_long`: a clear shot may be 17 steps, a blocked shot 14.
fn furthest_accepted_step(path: &[TilePosition], map: &impl Traversable) -> Option<TilePosition> {
    if path.len() < 2 {
        return None;
    }

    let start = path[0];
    let max_step = (path.len() - 1).min(SERVER_MAX_WALK_PATH);
    let mut best = None;
    for step in 1..=max_step {
        let cell = path[step];
        let clear = shot_is_clear(
            |x, y| tile_is_walkable(map, x, y),
            start.x as i32,
            start.y as i32,
            cell.x as i32,
            cell.y as i32,
        );
        let limit = if clear { SERVER_MAX_WALK_PATH } else { SERVER_OBSTRUCTED_WALK_PATH };
        if step <= limit {
            best = Some(cell);
        }
    }
    best
}

/// Hercules `path_search_long` (`path.c`). The start cell is not tested.
fn shot_is_clear(walkable: impl Fn(i32, i32) -> bool, mut x0: i32, mut y0: i32, mut x1: i32, mut y1: i32) -> bool {
    let mut dx = x1 - x0;
    if dx < 0 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
        dx = -dx;
    }
    let dy = y1 - y0;
    let weight = if dx > dy.abs() { dx } else { dy.abs() };
    if weight == 0 {
        return true;
    }

    let mut wx = 0;
    let mut wy = 0;
    while x0 != x1 || y0 != y1 {
        wx += dx;
        wy += dy;
        if wx >= weight {
            wx -= weight;
            x0 += 1;
        }
        if wy >= weight {
            wy -= weight;
            y0 += 1;
        } else if wy < 0 {
            wy += weight;
            y0 -= 1;
        }
        if !walkable(x0, y0) {
            return false;
        }
    }
    true
}

fn tile_is_walkable(map: &impl Traversable, x: i32, y: i32) -> bool {
    let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
        return false;
    };
    map.is_walkable(TilePosition { x, y })
}

/// Walkable steps toward `clicked` when no full route exists, diagonal first.
/// Includes `start`. Stops at a wall or after `max_steps`.
fn straight_walk_prefix(map: &impl Traversable, start: TilePosition, clicked: TilePosition, max_steps: usize) -> Vec<TilePosition> {
    let mut path = vec![start];
    let mut here = start;
    for _ in 0..max_steps {
        let dx = (clicked.x as i32 - here.x as i32).signum();
        let dy = (clicked.y as i32 - here.y as i32).signum();
        if dx == 0 && dy == 0 {
            break;
        }
        let next_x = here.x as i32 + dx;
        let next_y = here.y as i32 + dy;
        let (Ok(x), Ok(y)) = (u16::try_from(next_x), u16::try_from(next_y)) else {
            break;
        };
        let next = TilePosition { x, y };
        let corner_open =
            dx == 0 || dy == 0 || (map.is_walkable(TilePosition { x, y: here.y }) && map.is_walkable(TilePosition { x: here.x, y }));
        if !corner_open || !map.is_walkable(next) {
            break;
        }
        path.push(next);
        here = next;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMap {
        width: u16,
        height: u16,
        not_walkable: HashSet<TilePosition>,
        not_snipable: HashSet<TilePosition>,
    }

    impl TestMap {
        fn new(width: u16, height: u16) -> Self {
            Self {
                width,
                height,
                not_walkable: HashSet::new(),
                not_snipable: HashSet::new(),
            }
        }

        fn set_unwalkable(&mut self, points: &[TilePosition]) {
            for point in points {
                self.not_walkable.insert(*point);
            }
        }

        fn set_unsnipable(&mut self, points: &[TilePosition]) {
            for point in points {
                self.not_snipable.insert(*point);
            }
        }
    }

    impl Traversable for TestMap {
        fn is_walkable(&self, position: TilePosition) -> bool {
            position.x < self.width && position.y < self.height && !self.not_walkable.contains(&position)
        }

        fn is_snipeable(&self, position: TilePosition) -> bool {
            position.x < self.width && position.y < self.height && !self.not_snipable.contains(&position)
        }
    }

    #[test]
    fn test_straight_path() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 3, y: 0 };

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();
        assert_eq!(path, vec![
            TilePosition { x: 0, y: 0 },
            TilePosition { x: 1, y: 0 },
            TilePosition { x: 2, y: 0 },
            TilePosition { x: 3, y: 0 },
        ]);
    }

    #[test]
    fn test_diagonal_path() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 3, y: 3 };

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();
        assert_eq!(path, vec![
            TilePosition { x: 0, y: 0 },
            TilePosition { x: 1, y: 1 },
            TilePosition { x: 2, y: 2 },
            TilePosition { x: 3, y: 3 },
        ]);
    }

    #[test]
    fn test_path_with_obstacle() {
        let mut map = TestMap::new(5, 5);
        map.set_unwalkable(&[TilePosition { x: 1, y: 1 }, TilePosition { x: 1, y: 2 }, TilePosition {
            x: 1,
            y: 3,
        }]);

        let mut pathfinder = PathFinder::default();
        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 2, y: 2 };

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();

        assert_eq!(path, vec![
            TilePosition { x: 0, y: 0 },
            TilePosition { x: 1, y: 0 },
            TilePosition { x: 2, y: 0 },
            TilePosition { x: 2, y: 1 },
            TilePosition { x: 2, y: 2 },
        ]);
    }

    #[test]
    fn test_no_path_possible() {
        let mut map = TestMap::new(5, 5);

        map.set_unwalkable(&[
            TilePosition { x: 1, y: 0 },
            TilePosition { x: 1, y: 1 },
            TilePosition { x: 1, y: 2 },
            TilePosition { x: 1, y: 3 },
            TilePosition { x: 1, y: 4 },
        ]);

        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 2 };
        let goal = TilePosition { x: 2, y: 2 };

        assert!(pathfinder.find_walkable_path(&map, start, goal).is_none());
    }

    #[test]
    fn test_shoot_path_straight() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 3, y: 0 };

        let path = pathfinder.find_snipable_path(&map, start, goal).unwrap();
        assert_eq!(path.len(), 4);

        for (index, step) in path.iter().enumerate() {
            assert_eq!(step.x as usize, index);
            assert_eq!(step.y, 0);
        }
    }

    #[test]
    fn test_shoot_path_diagonal() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 3, y: 3 };

        let path = pathfinder.find_snipable_path(&map, start, goal).unwrap();
        assert_eq!(path.len(), 4);

        for (index, step) in path.iter().enumerate() {
            assert_eq!(step.x as usize, index);
            assert_eq!(step.y as usize, index);
        }
    }

    #[test]
    fn a_long_open_click_stops_at_seventeen_steps() {
        let map = TestMap::new(40, 5);
        let mut finder = PathFinder::default();
        let start = TilePosition { x: 0, y: 0 };
        let clicked = TilePosition { x: 30, y: 0 };

        let destination = click_walk_destination(&mut finder, &map, start, clicked).unwrap();

        assert_eq!(destination, TilePosition { x: 17, y: 0 });
    }

    #[test]
    fn a_short_click_and_a_short_detour_name_the_clicked_cell() {
        let map = TestMap::new(10, 10);
        let mut finder = PathFinder::default();
        let start = TilePosition { x: 0, y: 0 };
        let clicked = TilePosition { x: 5, y: 0 };
        assert_eq!(click_walk_destination(&mut finder, &map, start, clicked), Some(clicked));

        // Same shape as `test_path_with_obstacle`: four steps around a wall.
        // The server still accepts that, so the click is not shortened.
        let mut blocked = TestMap::new(5, 5);
        blocked.set_unwalkable(&[TilePosition { x: 1, y: 1 }, TilePosition { x: 1, y: 2 }, TilePosition {
            x: 1,
            y: 3,
        }]);
        let goal = TilePosition { x: 2, y: 2 };
        assert_eq!(click_walk_destination(&mut finder, &blocked, start, goal), Some(goal));
    }

    #[test]
    fn a_click_on_the_cell_underfoot_or_into_a_solid_wall_sends_nothing() {
        let map = TestMap::new(5, 5);
        let mut finder = PathFinder::default();
        let start = TilePosition { x: 0, y: 2 };
        assert!(click_walk_destination(&mut finder, &map, start, start).is_none());

        let mut wall = TestMap::new(5, 5);
        for y in 0..5 {
            wall.set_unwalkable(&[TilePosition { x: 1, y }]);
        }
        assert!(click_walk_destination(&mut finder, &wall, start, TilePosition { x: 3, y: 2 }).is_none());
    }

    #[test]
    fn a_click_past_a_wall_walks_up_to_the_wall() {
        let mut map = TestMap::new(12, 3);
        for y in 0..3 {
            map.set_unwalkable(&[TilePosition { x: 4, y }]);
        }
        let mut finder = PathFinder::default();

        let destination = click_walk_destination(&mut finder, &map, TilePosition { x: 0, y: 0 }, TilePosition { x: 8, y: 0 }).unwrap();

        assert_eq!(destination, TilePosition { x: 3, y: 0 });
    }

    #[test]
    fn a_route_around_a_wall_does_not_ask_for_a_hidden_step_past_fourteen() {
        // One corridor: ten east, two north, then west back to the goal.
        // Steps 12 through 14 sit past the corner (the straight shot is
        // blocked) and are still legal. Step 15 is not.
        let mut map = TestMap::new(12, 4);
        for x in 0..12 {
            for y in 0..4 {
                map.set_unwalkable(&[TilePosition { x, y }]);
            }
        }
        for x in 0..=10 {
            map.not_walkable.remove(&TilePosition { x, y: 0 });
            map.not_walkable.remove(&TilePosition { x, y: 2 });
        }
        map.not_walkable.remove(&TilePosition { x: 10, y: 1 });

        let mut finder = PathFinder::default();
        let start = TilePosition { x: 0, y: 0 };
        let clicked = TilePosition { x: 0, y: 2 };
        let destination = click_walk_destination(&mut finder, &map, start, clicked).unwrap();

        assert_eq!(destination, TilePosition { x: 8, y: 2 });
        let path = finder.find_navigation_path(&map, start, clicked).unwrap();
        let step = path.iter().position(|cell| *cell == destination).unwrap();
        assert_eq!(step, SERVER_OBSTRUCTED_WALK_PATH);
        assert_ne!(destination, clicked);
    }

    #[test]
    fn test_shoot_path_blocked() {
        let mut map = TestMap::new(5, 5);
        map.set_unsnipable(&[TilePosition { x: 1, y: 1 }]);

        let mut pathfinder = PathFinder::default();
        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 2, y: 2 };

        assert!(pathfinder.find_snipable_path(&map, start, goal).is_none());
    }
}
