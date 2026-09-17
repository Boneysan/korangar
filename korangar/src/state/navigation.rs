//! Advisory navigation state for the tracked quest objective.

use korangar_interface::element::StateElement;
use rust_state::RustState;

use crate::world::NavigationRoute;

#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct NavigationState {
    /// Ordered map names for the current route, including the current map.
    pub route_maps: Vec<String>,
    /// The next portal leg, if the objective is on another map.
    pub next_map: String,
    pub next_portal_x: Option<u16>,
    pub next_portal_y: Option<u16>,
    pub next_x: Option<u16>,
    pub next_y: Option<u16>,
    pub total_cost: Option<u32>,
    pub available: bool,
    /// Monotonic refresh counter, useful to make HUD updates observable.
    pub recompute_count: u64,
    pub last_reason: String,
}

impl NavigationState {
    pub fn apply_route(&mut self, route: Option<&NavigationRoute>, reason: &str) {
        self.route_maps.clear();
        self.next_map.clear();
        self.next_portal_x = None;
        self.next_portal_y = None;
        self.next_x = None;
        self.next_y = None;
        self.total_cost = None;
        self.available = false;
        self.recompute_count = self.recompute_count.saturating_add(1);
        self.last_reason = reason.to_owned();

        let Some(route) = route else { return };

        self.available = true;
        self.total_cost = Some(route.total_cost);
        if let Some(first) = route.legs.first() {
            self.route_maps.push(first.from_map.clone());
        }
        for leg in &route.legs {
            if self.route_maps.last() != Some(&leg.to_map) {
                self.route_maps.push(leg.to_map.clone());
            }
        }
        if let Some(first) = route.legs.first() {
            self.next_map = first.to_map.clone();
            self.next_portal_x = Some(first.from_x);
            self.next_portal_y = Some(first.from_y);
            self.next_x = Some(first.to_x);
            self.next_y = Some(first.to_y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::RouteLeg;

    #[test]
    fn records_route_and_refresh_reason() {
        let route = NavigationRoute {
            legs: vec![RouteLeg {
                from_map: "prt_fild08".into(),
                from_x: 10,
                from_y: 20,
                to_map: "prontera".into(),
                to_x: 150,
                to_y: 180,
            }],
            total_cost: 42,
        };
        let mut state = NavigationState::default();
        state.apply_route(Some(&route), "map change");

        assert_eq!(state.route_maps, ["prt_fild08", "prontera"]);
        assert_eq!(state.next_map, "prontera");
        assert_eq!(state.next_portal_x, Some(10));
        assert_eq!(state.next_portal_y, Some(20));
        assert_eq!(state.next_x, Some(150));
        assert_eq!(state.total_cost, Some(42));
        assert!(state.available);
        assert_eq!(state.recompute_count, 1);
        assert_eq!(state.last_reason, "map change");
    }

    #[test]
    fn clears_unavailable_route_but_keeps_revision() {
        let mut state = NavigationState::default();
        state.apply_route(None, "objective refresh");

        assert!(!state.available);
        assert!(state.route_maps.is_empty());
        assert_eq!(state.recompute_count, 1);
        assert_eq!(state.last_reason, "objective refresh");
    }

    #[test]
    fn final_map_route_keeps_destination_but_has_no_portal_leg() {
        let route = NavigationRoute {
            legs: Vec::new(),
            total_cost: 17,
        };
        let mut state = NavigationState::default();
        state.apply_route(Some(&route), "authoritative position");

        assert!(state.available);
        assert_eq!(state.total_cost, Some(17));
        assert!(state.route_maps.is_empty());
        assert!(state.next_map.is_empty());
        assert_eq!(state.next_portal_x, None);
        assert_eq!(state.next_portal_y, None);
    }

    #[test]
    fn exposes_only_the_current_portal_leg_from_a_multi_map_route() {
        let route = NavigationRoute {
            legs: vec![
                RouteLeg {
                    from_map: "prontera".into(),
                    from_x: 100,
                    from_y: 100,
                    to_map: "prt_fild08".into(),
                    to_x: 20,
                    to_y: 30,
                },
                RouteLeg {
                    from_map: "prt_fild08".into(),
                    from_x: 250,
                    from_y: 250,
                    to_map: "prt_maze01".into(),
                    to_x: 40,
                    to_y: 50,
                },
            ],
            total_cost: 99,
        };
        let mut state = NavigationState::default();
        state.apply_route(Some(&route), "map transition");

        assert_eq!(state.route_maps, ["prontera", "prt_fild08", "prt_maze01"]);
        assert_eq!(state.next_map, "prt_fild08");
        assert_eq!(state.next_portal_x, Some(100));
        assert_eq!(state.next_portal_y, Some(100));
        assert_eq!(state.next_x, Some(20));
        assert_eq!(state.next_y, Some(30));
    }
}
