use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::TilePosition;
use rust_state::State;

use crate::PlayerPathExt;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state, this_player};
use crate::world::{Library, NavigationEdge, TownPoi, WorldRegion, is_dangerous_map_level, map_region, navigation_graph, route_edges};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoadKind {
    Walk,
    Transport,
}

#[derive(Clone, Copy, Debug)]
pub struct AtlasRoad {
    pub from: usize,
    pub to: usize,
    pub kind: RoadKind,
}

#[derive(Clone, Copy)]
pub struct AtlasLocation {
    pub map: &'static str,
    pub label: &'static str,
    #[allow(dead_code)]
    pub region: WorldRegion,
    pub x: f32,
    pub y: f32,
    pub tile: TilePosition,
}

// Positions are a readable regional overview rather than in-game coordinates.
pub const LOCATIONS: &[AtlasLocation] = &[
    AtlasLocation {
        map: "yuno",
        label: "Juno",
        region: WorldRegion::Schwarzwald,
        x: 0.48,
        y: 0.12,
        tile: TilePosition { x: 157, y: 123 },
    },
    AtlasLocation {
        map: "rachel",
        label: "Rachel",
        region: WorldRegion::Arunafeltz,
        x: 0.69,
        y: 0.12,
        tile: TilePosition { x: 120, y: 120 },
    },
    AtlasLocation {
        map: "hugel",
        label: "Hugel",
        region: WorldRegion::Schwarzwald,
        x: 0.84,
        y: 0.27,
        tile: TilePosition { x: 96, y: 145 },
    },
    AtlasLocation {
        map: "aldebaran",
        label: "Al De Baran",
        region: WorldRegion::RuneMidgarts,
        x: 0.39,
        y: 0.27,
        tile: TilePosition { x: 140, y: 131 },
    },
    AtlasLocation {
        map: "geffen",
        label: "Geffen",
        region: WorldRegion::RuneMidgarts,
        x: 0.20,
        y: 0.34,
        tile: TilePosition { x: 119, y: 59 },
    },
    AtlasLocation {
        map: "prontera",
        label: "Prontera",
        region: WorldRegion::RuneMidgarts,
        x: 0.48,
        y: 0.43,
        tile: TilePosition { x: 155, y: 183 },
    },
    AtlasLocation {
        map: "payon",
        label: "Payon",
        region: WorldRegion::RuneMidgarts,
        x: 0.76,
        y: 0.39,
        tile: TilePosition { x: 160, y: 120 },
    },
    AtlasLocation {
        map: "morocc",
        label: "Morroc",
        region: WorldRegion::RuneMidgarts,
        x: 0.31,
        y: 0.57,
        tile: TilePosition { x: 156, y: 97 },
    },
    AtlasLocation {
        map: "izlude",
        label: "Izlude",
        region: WorldRegion::RuneMidgarts,
        x: 0.55,
        y: 0.60,
        tile: TilePosition { x: 128, y: 146 },
    },
    AtlasLocation {
        map: "alberta",
        label: "Alberta",
        region: WorldRegion::RuneMidgarts,
        x: 0.68,
        y: 0.58,
        tile: TilePosition { x: 28, y: 234 },
    },
    AtlasLocation {
        map: "amatsu",
        label: "Amatsu",
        region: WorldRegion::GlobalProject,
        x: 0.88,
        y: 0.52,
        tile: TilePosition { x: 198, y: 84 },
    },
    AtlasLocation {
        map: "comodo",
        label: "Comodo",
        region: WorldRegion::RuneMidgarts,
        x: 0.17,
        y: 0.72,
        tile: TilePosition { x: 184, y: 151 },
    },
    AtlasLocation {
        map: "umbala",
        label: "Umbala",
        region: WorldRegion::RuneMidgarts,
        x: 0.31,
        y: 0.78,
        tile: TilePosition { x: 97, y: 153 },
    },
    AtlasLocation {
        map: "jawaii",
        label: "Jawaii",
        region: WorldRegion::GlobalProject,
        x: 0.49,
        y: 0.82,
        tile: TilePosition { x: 251, y: 132 },
    },
    AtlasLocation {
        map: "louyang",
        label: "Louyang",
        region: WorldRegion::GlobalProject,
        x: 0.71,
        y: 0.76,
        tile: TilePosition { x: 217, y: 100 },
    },
    AtlasLocation {
        map: "gonryun",
        label: "Gonryun",
        region: WorldRegion::GlobalProject,
        x: 0.84,
        y: 0.68,
        tile: TilePosition { x: 160, y: 120 },
    },
    AtlasLocation {
        map: "ayothaya",
        label: "Ayothaya",
        region: WorldRegion::GlobalProject,
        x: 0.88,
        y: 0.84,
        tile: TilePosition { x: 208, y: 166 },
    },
    AtlasLocation {
        map: "brasilis",
        label: "Brasilis",
        region: WorldRegion::GlobalProject,
        x: 0.10,
        y: 0.88,
        tile: TilePosition { x: 196, y: 217 },
    },
    AtlasLocation {
        map: "einbroch",
        label: "Einbroch",
        region: WorldRegion::Schwarzwald,
        x: 0.57,
        y: 0.22,
        tile: TilePosition { x: 64, y: 200 },
    },
    AtlasLocation {
        map: "einbech",
        label: "Einbech",
        region: WorldRegion::Schwarzwald,
        x: 0.66,
        y: 0.29,
        tile: TilePosition { x: 63, y: 35 },
    },
    AtlasLocation {
        map: "lighthalzen",
        label: "Lighthalzen",
        region: WorldRegion::Schwarzwald,
        x: 0.55,
        y: 0.34,
        tile: TilePosition { x: 158, y: 92 },
    },
    AtlasLocation {
        map: "lasagna",
        label: "Lasagna",
        region: WorldRegion::GlobalProject,
        x: 0.96,
        y: 0.36,
        tile: TilePosition { x: 193, y: 182 },
    },
    AtlasLocation {
        map: "dicastes01",
        label: "El Dicastes",
        region: WorldRegion::DimensionalGorge,
        x: 0.80,
        y: 0.16,
        tile: TilePosition { x: 198, y: 187 },
    },
    AtlasLocation {
        map: "xmas",
        label: "Lutie",
        region: WorldRegion::RuneMidgarts,
        x: 0.31,
        y: 0.19,
        tile: TilePosition { x: 147, y: 134 },
    },
    AtlasLocation {
        map: "mid_camp",
        label: "Midgard Camp",
        region: WorldRegion::DimensionalGorge,
        x: 0.39,
        y: 0.69,
        tile: TilePosition { x: 180, y: 240 },
    },
    AtlasLocation {
        map: "c_tower1",
        label: "Clock Tower",
        region: WorldRegion::DungeonLandmark,
        x: 0.28,
        y: 0.27,
        tile: TilePosition { x: 235, y: 218 },
    },
    AtlasLocation {
        map: "ama_dun01",
        label: "Amatsu Cave",
        region: WorldRegion::DungeonLandmark,
        x: 0.93,
        y: 0.59,
        tile: TilePosition { x: 54, y: 107 },
    },
];

// Authored regional roads keep the overview legible, explicitly classifying
// overland walk roads and in-world transport connections (ferries, airships,
// sleighs, dimensional rifts). Actual route highlighting and the minimap
// guidance use the generated, verified warp graph.
pub const ROADS: &[AtlasRoad] = &[
    AtlasRoad {
        from: 0,
        to: 18,
        kind: RoadKind::Transport,
    }, // Juno <-> Einbroch (airship)
    AtlasRoad {
        from: 0,
        to: 22,
        kind: RoadKind::Transport,
    }, // Juno <-> El Dicastes
    AtlasRoad {
        from: 1,
        to: 22,
        kind: RoadKind::Transport,
    }, // Rachel <-> El Dicastes
    AtlasRoad {
        from: 2,
        to: 22,
        kind: RoadKind::Transport,
    }, // Hugel <-> El Dicastes
    AtlasRoad {
        from: 18,
        to: 19,
        kind: RoadKind::Walk,
    }, // Einbroch <-> Einbech
    AtlasRoad {
        from: 18,
        to: 9,
        kind: RoadKind::Transport,
    }, // Einbroch <-> Alberta (airship)
    AtlasRoad {
        from: 19,
        to: 20,
        kind: RoadKind::Walk,
    }, // Einbech <-> Lighthalzen
    AtlasRoad {
        from: 20,
        to: 9,
        kind: RoadKind::Transport,
    }, // Lighthalzen <-> Alberta (airship)
    AtlasRoad {
        from: 3,
        to: 4,
        kind: RoadKind::Walk,
    }, // Al De Baran <-> Geffen
    AtlasRoad {
        from: 3,
        to: 5,
        kind: RoadKind::Walk,
    }, // Al De Baran <-> Prontera
    AtlasRoad {
        from: 3,
        to: 23,
        kind: RoadKind::Transport,
    }, // Al De Baran <-> Lutie (Santa sleigh)
    AtlasRoad {
        from: 4,
        to: 5,
        kind: RoadKind::Walk,
    }, // Geffen <-> Prontera
    AtlasRoad {
        from: 4,
        to: 7,
        kind: RoadKind::Walk,
    }, // Geffen <-> Morroc
    AtlasRoad {
        from: 4,
        to: 23,
        kind: RoadKind::Transport,
    }, // Geffen <-> Lutie
    AtlasRoad {
        from: 5,
        to: 6,
        kind: RoadKind::Walk,
    }, // Prontera <-> Payon
    AtlasRoad {
        from: 5,
        to: 7,
        kind: RoadKind::Walk,
    }, // Prontera <-> Morroc
    AtlasRoad {
        from: 5,
        to: 8,
        kind: RoadKind::Walk,
    }, // Prontera <-> Izlude
    AtlasRoad {
        from: 5,
        to: 24,
        kind: RoadKind::Transport,
    }, // Prontera <-> Midgard Camp (gorge)
    AtlasRoad {
        from: 6,
        to: 8,
        kind: RoadKind::Walk,
    }, // Payon <-> Izlude
    AtlasRoad {
        from: 6,
        to: 9,
        kind: RoadKind::Walk,
    }, // Payon <-> Alberta
    AtlasRoad {
        from: 6,
        to: 15,
        kind: RoadKind::Transport,
    }, // Payon <-> Gonryun
    AtlasRoad {
        from: 7,
        to: 12,
        kind: RoadKind::Walk,
    }, // Morroc <-> Umbala
    AtlasRoad {
        from: 7,
        to: 24,
        kind: RoadKind::Transport,
    }, // Morroc <-> Midgard Camp
    AtlasRoad {
        from: 8,
        to: 9,
        kind: RoadKind::Walk,
    }, // Izlude <-> Alberta
    AtlasRoad {
        from: 8,
        to: 13,
        kind: RoadKind::Transport,
    }, // Izlude <-> Jawaii (ferry)
    AtlasRoad {
        from: 9,
        to: 10,
        kind: RoadKind::Transport,
    }, // Alberta <-> Amatsu (voyage)
    AtlasRoad {
        from: 9,
        to: 13,
        kind: RoadKind::Transport,
    }, // Alberta <-> Jawaii (ferry)
    AtlasRoad {
        from: 10,
        to: 15,
        kind: RoadKind::Transport,
    }, // Amatsu <-> Gonryun
    AtlasRoad {
        from: 10,
        to: 26,
        kind: RoadKind::Walk,
    }, // Amatsu <-> Amatsu Cave
    AtlasRoad {
        from: 11,
        to: 12,
        kind: RoadKind::Walk,
    }, // Comodo <-> Umbala
    AtlasRoad {
        from: 12,
        to: 17,
        kind: RoadKind::Transport,
    }, // Umbala <-> Brasilis
    AtlasRoad {
        from: 13,
        to: 14,
        kind: RoadKind::Transport,
    }, // Jawaii <-> Louyang
    AtlasRoad {
        from: 14,
        to: 15,
        kind: RoadKind::Transport,
    }, // Louyang <-> Gonryun
    AtlasRoad {
        from: 14,
        to: 16,
        kind: RoadKind::Transport,
    }, // Louyang <-> Ayothaya
];

pub(crate) struct RouteClick {
    pub(crate) map: &'static str,
    pub(crate) position: TilePosition,
}

impl RouteClick {
    pub(crate) fn event(&self) -> InputEvent {
        InputEvent::SetNavigationDestination {
            map_name: self.map.to_owned(),
            x: self.position.x,
            y: self.position.y,
        }
    }
}

pub(crate) struct FacilityRouteClick {
    pub(crate) map_name: String,
    pub(crate) position: TilePosition,
}

impl FacilityRouteClick {
    pub(crate) fn event(&self) -> InputEvent {
        InputEvent::SetNavigationDestination {
            map_name: self.map_name.clone(),
            x: self.position.x,
            y: self.position.y,
        }
    }
}

impl ClickHandler<ClientState> for FacilityRouteClick {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(self.event());
    }
}

struct FacilityRoute {
    name: String,
    click: FacilityRouteClick,
}

fn town_poi_tile(poi: &TownPoi) -> Option<TilePosition> {
    Some(TilePosition {
        x: poi.x.try_into().ok()?,
        y: poi.y.try_into().ok()?,
    })
}

impl ClickHandler<ClientState> for RouteClick {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(self.event());
    }
}

struct AtlasNode {
    location: AtlasLocation,
    click: RouteClick,
}

struct AtlasView {
    library: Arc<Library>,
    nodes: Vec<AtlasNode>,
    facility_routes: Vec<FacilityRoute>,
    details: [String; 6],
    dangerous_maps: HashSet<String>,
}

impl AtlasView {
    fn new(library: Arc<Library>) -> Self {
        Self {
            library,
            nodes: LOCATIONS
                .iter()
                .copied()
                .map(|location| AtlasNode {
                    location,
                    click: RouteClick {
                        map: location.map,
                        position: location.tile,
                    },
                })
                .collect(),
            facility_routes: Vec::new(),
            details: destination_detail_lines("", None, None, 0, None, &[], None, &[]),
            dangerous_maps: HashSet::new(),
        }
    }
}

impl Element<ClientState> for AtlasView {
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        let minimap_path = client_state().minimap();
        let minimap = state.get(&minimap_path);
        let current_map = minimap.map_name();
        let target = minimap.navigation_target().map(|target| target.map_name.as_str());
        let discovery_path = client_state().discovery();
        let discovery = state.get(&discovery_path);
        let party_path = client_state().party_state();
        let party = state.get(&party_path);
        let route = target.and_then(|destination| route_edges(current_map, destination));
        let selected_map = target.unwrap_or(current_map);
        let outgoing_exits = navigation_graph()
            .edges
            .iter()
            .filter(|edge| edge.from.map.eq_ignore_ascii_case(selected_map))
            .count();
        let visit_state = if discovery.visited_map(selected_map) {
            "visited"
        } else if discovery.map_snapshot_complete() {
            "not yet visited"
        } else {
            "discovery sync pending"
        };
        let party_members_here: Vec<_> = party
            .members()
            .iter()
            .filter(|member| member.online() && normalized_map_name(member.map_name()).eq_ignore_ascii_case(selected_map))
            .map(|member| member.name().to_owned())
            .collect();
        let pois = self.library.town_pois(selected_map);
        self.facility_routes = pois
            .iter()
            .filter_map(|poi| {
                let position = town_poi_tile(poi)?;
                Some(FacilityRoute {
                    name: format!("Route: {}", poi.name),
                    click: FacilityRouteClick {
                        map_name: selected_map.to_owned(),
                        position,
                    },
                })
            })
            .take(3)
            .collect();
        let town_pois = pois.iter().map(|poi| poi.name.clone()).collect::<Vec<_>>();
        let reference = crate::dm::reference_data::reference_data();
        self.details = destination_detail_lines(
            current_map,
            target,
            route.as_deref(),
            outgoing_exits,
            Some(visit_state),
            &party_members_here,
            reference.map_spawn_details(selected_map),
            &town_pois,
        );
        let player_level = state
            .try_get(&this_player().base_level())
            .copied()
            .map(|level| level.min(u16::MAX as usize) as u16);
        self.dangerous_maps = player_level.map_or_else(HashSet::new, |player_level| {
            LOCATIONS
                .iter()
                .filter(|location| {
                    reference
                        .map_spawn_summary(location.map)
                        .is_some_and(|(_, mean_level, _)| is_dangerous_map_level(mean_level, player_level))
                })
                .map(|location| location.map.to_ascii_lowercase())
                .collect()
        });
        with_single_resolver(resolvers, |resolver| resolver.with_height(520.0))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        area: &Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let minimap_path = client_state().minimap();
        let minimap = state.get(&minimap_path);
        let discovery_path = client_state().discovery();
        let discovery = state.get(&discovery_path);
        let current_map = minimap.map_name();
        let target = minimap.navigation_target().map(|target| target.map_name.as_str());
        let route = target.and_then(|destination| route_edges(current_map, destination));
        let mut route_maps = Vec::new();
        if let Some(edges) = &route {
            for edge in edges {
                route_maps.push(edge.from.map.as_str());
                route_maps.push(edge.to.map.as_str());
            }
        }

        layout.add_rectangle(
            *area,
            CornerDiameter::uniform(10.0),
            Color::rgb_u8(15, 27, 42),
            Color::rgba_u8(0, 0, 0, 120),
            ShadowPadding::uniform(3.0),
        );

        let node_width = 94.0;
        let node_height = 28.0;
        let point = |location: AtlasLocation| {
            (
                area.left + location.x * (area.width - node_width - 8.0),
                area.top + location.y * (area.height - node_height - 160.0),
            )
        };
        for road in available_roads() {
            let (Some(left), Some(right)) = (self.nodes.get(road.from), self.nodes.get(road.to)) else {
                continue;
            };
            let (x1, y1) = point(left.location);
            let (x2, y2) = point(right.location);
            let color = match road.kind {
                RoadKind::Walk => Color::rgb_u8(67, 100, 121),
                RoadKind::Transport => Color::rgb_u8(82, 134, 184),
            };
            draw_dotted_connection(
                layout,
                x1 + node_width / 2.0,
                y1 + node_height / 2.0,
                x2 + node_width / 2.0,
                y2 + node_height / 2.0,
                color,
            );
        }

        // Mark route legs by linking the visible atlas locations found in the
        // computed route. The exact walkable line for the current map appears
        // as breadcrumbs on its minimap.
        let mut route_nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| route_maps.iter().any(|map| map.eq_ignore_ascii_case(node.location.map)))
            .collect();
        route_nodes.sort_by_key(|node| {
            route_maps
                .iter()
                .position(|map| map.eq_ignore_ascii_case(node.location.map))
                .unwrap_or(usize::MAX)
        });
        for pair in route_nodes.windows(2) {
            let (x1, y1) = point(pair[0].location);
            let (x2, y2) = point(pair[1].location);
            draw_dotted_connection(
                layout,
                x1 + node_width / 2.0,
                y1 + node_height / 2.0,
                x2 + node_width / 2.0,
                y2 + node_height / 2.0,
                Color::rgb_u8(255, 204, 84),
            );
        }

        for node in &self.nodes {
            let (center_x, center_y) = point(node.location);
            let node_area = Area {
                left: center_x,
                top: center_y,
                width: node_width,
                height: node_height,
            };
            let is_current = current_map.eq_ignore_ascii_case(node.location.map);
            let is_target = target.is_some_and(|map| map.eq_ignore_ascii_case(node.location.map));
            let in_route = route_maps.iter().any(|map| map.eq_ignore_ascii_case(node.location.map));
            let is_dangerous = self.dangerous_maps.contains(node.location.map);
            let is_hovered = node_area.check().run(layout);
            let color = if is_target {
                Color::rgb_u8(122, 79, 25)
            } else if is_current {
                Color::rgb_u8(32, 94, 108)
            } else if in_route {
                Color::rgb_u8(77, 71, 42)
            } else if is_hovered {
                Color::rgb_u8(58, 78, 96)
            } else {
                Color::rgb_u8(34, 49, 66)
            };
            layout.add_rectangle(
                node_area,
                CornerDiameter::uniform(7.0),
                color,
                if is_dangerous {
                    Color::rgb_u8(255, 126, 92)
                } else {
                    Color::rgba_u8(0, 0, 0, 150)
                },
                ShadowPadding::uniform(2.0),
            );
            let visit_marker = if is_dangerous {
                "!"
            } else if discovery.visited_map(node.location.map) {
                "✓"
            } else if discovery.map_snapshot_complete() {
                "·"
            } else {
                "?"
            };
            layout.add_text(
                node_area,
                visit_marker,
                FontSize(12.0),
                Color::rgb_u8(145, 218, 173),
                Color::WHITE,
                HorizontalAlignment::Left { offset: 4.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
            layout.add_text(
                Area {
                    left: node_area.left + 15.0,
                    width: node_area.width - 15.0,
                    ..node_area
                },
                node.location.label,
                FontSize(12.0),
                Color::rgb_u8(239, 235, 215),
                Color::WHITE,
                HorizontalAlignment::Center { offset: -4.0, border: 3.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
            if is_hovered {
                layout.register_click_handler(MouseButton::Left, &node.click);
            }
        }

        for (index, line) in self.details.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            if index == 4 && !self.facility_routes.is_empty() {
                let row = Area {
                    left: area.left + 12.0,
                    top: area.top + area.height - 144.0 + index as f32 * 18.0,
                    width: area.width - 24.0,
                    height: 18.0,
                };
                layout.add_text(
                    Area { width: 78.0, ..row },
                    "Facilities:",
                    FontSize(12.0),
                    Color::rgb_u8(195, 205, 214),
                    Color::WHITE,
                    HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
                let route_width = ((row.width - 82.0) / self.facility_routes.len() as f32).min(150.0);
                for (route_index, facility) in self.facility_routes.iter().enumerate() {
                    let route_area = Area {
                        left: row.left + 80.0 + route_width * route_index as f32,
                        top: row.top,
                        width: route_width,
                        height: row.height,
                    };
                    let hovered = route_area.check().run(layout);
                    layout.add_rectangle(
                        route_area,
                        CornerDiameter::uniform(4.0),
                        if hovered {
                            Color::rgb_u8(58, 78, 96)
                        } else {
                            Color::rgb_u8(34, 49, 66)
                        },
                        Color::rgba_u8(0, 0, 0, 80),
                        ShadowPadding::uniform(0.0),
                    );
                    layout.add_text(
                        route_area,
                        &facility.name,
                        FontSize(11.0),
                        Color::rgb_u8(239, 235, 215),
                        Color::WHITE,
                        HorizontalAlignment::Center { offset: 0.0, border: 3.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );
                    if hovered {
                        layout.register_click_handler(MouseButton::Left, &facility.click);
                    }
                }
                continue;
            }
            layout.add_text(
                Area {
                    left: area.left + 12.0,
                    top: area.top + area.height - 144.0 + index as f32 * 18.0,
                    width: area.width - 24.0,
                    height: 18.0,
                },
                line,
                FontSize(12.0),
                Color::rgb_u8(195, 205, 214),
                Color::WHITE,
                HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        let status = if target.is_some() && route.is_none() {
            "No verified portal route from this map. Choose a reachable location."
        } else if let Some(target) = target {
            // target only borrows minimap state, so display a static prompt here;
            // the exact destination is also shown beneath the map window.
            let _ = target;
            "Gold: route  •  Cyan: overland  •  Blue: in-world transport  •  Cyan dots: walkable trail"
        } else {
            "Select a town or destination to plot a route. Click Clear Route to cancel."
        };
        layout.add_text(
            Area {
                left: area.left + 12.0,
                top: area.top + area.height - 27.0,
                width: area.width - 24.0,
                height: 20.0,
            },
            status,
            FontSize(12.0),
            Color::rgb_u8(195, 205, 214),
            Color::WHITE,
            HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );
    }
}

fn available_roads() -> &'static [AtlasRoad] {
    static AVAILABLE: OnceLock<Vec<AtlasRoad>> = OnceLock::new();
    AVAILABLE.get_or_init(|| {
        ROADS
            .iter()
            .copied()
            .filter(|road| route_edges(LOCATIONS[road.from].map, LOCATIONS[road.to].map).is_some())
            .collect()
    })
}

#[allow(clippy::too_many_arguments)] // one input per displayed fact
fn destination_detail_lines(
    current_map: &str,
    target_map: Option<&str>,
    route: Option<&[&'static NavigationEdge]>,
    outgoing_exits: usize,
    visit_state: Option<&str>,
    party_members_here: &[String],
    population: Option<crate::dm::reference_data::MapSpawnDetails>,
    town_pois: &[String],
) -> [String; 6] {
    let selected_map = target_map.unwrap_or(current_map);
    let visited = visit_state.unwrap_or("discovery sync pending");
    let region_info = map_region(selected_map)
        .map(|region| format!(" • {}", region.name()))
        .unwrap_or_default();
    let map_heading = match target_map {
        Some(target) => format!("Destination: {target} • {visited}{region_info}"),
        None => format!("Current map: {selected_map} • {visited}{region_info}"),
    };
    let (level_line, population_line) = population.map_or_else(
        || {
            (
                "Suggested level: unavailable (low-coverage or non-combat map)".to_owned(),
                "Static population: no verified spawn records (low coverage)".to_owned(),
            )
        },
        |details| {
            let range_str = if details.min_level != details.max_level {
                format!(" (range: {}–{})", details.min_level, details.max_level)
            } else {
                String::new()
            };
            (
                format!("Suggested level: ~{}{range_str} (static-spawn mean)", details.mean_level),
                format!(
                    "Static population: {} spawn records • {} species",
                    details.records, details.species
                ),
            )
        },
    );
    let party_line = if party_members_here.is_empty() {
        "Party here: no online members reporting this map".to_owned()
    } else {
        format!("Party here: {}", party_members_here.join(", "))
    };
    let poi_line = if town_pois.is_empty() {
        "Towninfo facilities: none listed for this map".to_owned()
    } else {
        let shown = town_pois.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
        if town_pois.len() > 3 {
            format!("Towninfo facilities: {shown}, +{} more", town_pois.len() - 3)
        } else {
            format!("Towninfo facilities: {shown}")
        }
    };
    let (route_summary, next_exit) = match (target_map, route) {
        (None, _) => (
            format!("Verified outgoing connections: {outgoing_exits}"),
            "choose a destination to plot a route".to_owned(),
        ),
        (Some(target), None) => (
            "No verified route".to_owned(),
            format!("No portal route from {current_map} to {target}"),
        ),
        (Some(target), Some([])) => ("Already at destination".to_owned(), format!("you are on {target}")),
        (Some(_), Some(edges)) => {
            let edge = edges[0];
            let transport_count = edges.iter().filter(|e| e.kind == "npc_service").count();
            let walk_count = edges.len() - transport_count;
            let leg_composition = if transport_count > 0 {
                format!(
                    "{} travel legs ({} walk, {} transport)",
                    edges.len(),
                    walk_count,
                    transport_count
                )
            } else {
                format!("{} travel legs", edges.len())
            };
            let edge_action = if edge.kind == "npc_service" {
                let action = edge.action.as_deref().unwrap_or("use the listed travel service");
                let requirements = edge
                    .requirements
                    .as_deref()
                    .map(|requirements| format!(" • {requirements}"))
                    .unwrap_or_default();
                let availability = if edge.availability == "conditional" { " (conditional)" } else { "" };
                format!(
                    "next: service at {} ({}, {}) — {action}{availability}{requirements} → {}",
                    edge.from.map, edge.from.x, edge.from.y, edge.to.map
                )
            } else {
                format!(
                    "next: portal at {} ({}, {}) → {}",
                    edge.from.map, edge.from.x, edge.from.y, edge.to.map
                )
            };
            (format!("{leg_composition} • {outgoing_exits} connections"), edge_action)
        }
    };
    [
        map_heading,
        level_line,
        population_line,
        party_line,
        poi_line,
        format!("Connections: {route_summary} • {next_exit}"),
    ]
}

fn normalized_map_name(map_name: &str) -> &str {
    map_name.strip_suffix(".gat").unwrap_or(map_name)
}

fn draw_dotted_connection(layout: &mut WindowLayout<'_, ClientState>, x1: f32, y1: f32, x2: f32, y2: f32, color: Color) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let distance = (dx * dx + dy * dy).sqrt();
    let count = (distance / 9.0).ceil() as usize;
    for index in 1..count {
        let ratio = index as f32 / count as f32;
        let x = x1 + dx * ratio;
        let y = y1 + dy * ratio;
        layout.add_rectangle(
            Area {
                left: x - 2.0,
                top: y - 2.0,
                width: 4.0,
                height: 4.0,
            },
            CornerDiameter::uniform(2.0),
            color,
            Color::rgba_u8(0, 0, 0, 0),
            ShadowPadding::uniform(0.0),
        );
    }
}

#[allow(dead_code)]
impl AtlasLocation {
    pub const fn region(&self) -> WorldRegion {
        self.region
    }
}

pub struct MapsWindow {
    library: Arc<Library>,
}

impl MapsWindow {
    pub fn new(library: Arc<Library>) -> Self {
        Self { library }
    }
}

impl CustomWindow<ClientState> for MapsWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Maps)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "World Map & Route Finder",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                AtlasView::new(self.library),
                button! {
                    text: "Clear Route",
                    tooltip: "Clear the current destination and breadcrumb trail",
                    event: InputEvent::ClearNavigationDestination,
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use korangar_interface::event::{Event, EventQueue};
    use ragnarok_packets::TilePosition;

    use super::{
        FacilityRouteClick, LOCATIONS, ROADS, RoadKind, RouteClick, available_roads, destination_detail_lines, normalized_map_name,
        town_poi_tile,
    };
    use crate::input::InputEvent;
    use crate::state::discovery::DiscoveryState;
    use crate::world::{TownPoi, TownPoiKind, is_dangerous_map_level, map_region, navigation_graph, route_edges};

    #[test]
    fn atlas_poi_routes_accept_only_nonnegative_client_coordinates() {
        let poi = TownPoi {
            name: "Kafra Employee".to_owned(),
            x: 156,
            y: 191,
            kind: TownPoiKind::Kafra,
        };
        assert_eq!(town_poi_tile(&poi), Some(TilePosition { x: 156, y: 191 }));

        let invalid = TownPoi { x: -1, ..poi };
        assert_eq!(town_poi_tile(&invalid), None);
    }

    #[test]
    fn atlas_destination_details_use_verified_route_edges_and_visit_state() {
        let route = route_edges("prontera", "izlude").expect("known route");
        let graph = navigation_graph();
        let outgoing = graph
            .edges
            .iter()
            .filter(|edge| edge.from.map.eq_ignore_ascii_case("izlude"))
            .count();
        let party = vec!["Alice".to_owned()];
        let pois = vec!["Kafra Employee".to_owned(), "Tool Dealer".to_owned()];
        let population = crate::dm::reference_data::reference_data().map_spawn_details("izlude");
        let lines = destination_detail_lines(
            "prontera",
            Some("izlude"),
            Some(&route),
            outgoing,
            Some("visited"),
            &party,
            population,
            &pois,
        );

        assert!(lines[0].contains("Destination: izlude • visited"));
        assert!(lines[0].contains("Rune-Midgarts Kingdom"));
        assert!(lines[1].contains("Suggested level:"));
        assert!(lines[2].contains("Static population:"));
        assert!(lines[3].contains("Alice"));
        assert!(lines[4].contains("Kafra Employee"));
        assert!(lines[5].contains(&format!("{} travel legs", route.len())));
        assert!(lines[5].contains(&route[0].from.map));
        assert!(lines[5].contains(&route[0].to.map));
    }

    #[test]
    fn atlas_destination_details_explain_conditional_npc_service_legs() {
        let route = route_edges("izlude", "iz_dun00").expect("authored conditional ferry route");
        let lines = destination_detail_lines("izlude", Some("iz_dun00"), Some(&route), 2, Some("visited"), &[], None, &[]);

        assert!(lines[5].contains("2 travel legs (1 walk, 1 transport)"));
        assert!(lines[5].contains("service at izlude (197, 205)"));
        assert!(lines[5].contains("choose Byalan Island"));
        assert!(lines[5].contains("conditional"));
        assert!(lines[5].contains("150 zeny"));
        assert!(lines[5].contains("izlu2dun"));
    }

    #[test]
    fn atlas_destination_details_do_not_claim_routes_when_graph_has_none() {
        let lines = destination_detail_lines("unknown_map", Some("unknown_destination"), None, 0, None, &[], None, &[]);
        assert!(lines[0].contains("discovery sync pending"));
        assert!(lines[1].contains("unavailable"));
        assert!(lines[2].contains("no verified spawn records"));
        assert!(lines[4].contains("none listed for this map"));
        assert!(lines[5].contains("No verified route"));

        let empty = destination_detail_lines("prontera", None, None, 3, Some("visited"), &[], None, &[]);
        assert!(empty[0].contains("Current map: prontera • visited"));
        assert!(empty[0].contains("Rune-Midgarts Kingdom"));
        assert!(empty[4].contains("none listed"));
        assert!(empty[5].contains("3"));
        assert_eq!(normalized_map_name("izlude.gat"), "izlude");
    }

    #[test]
    fn visited_account_vs_unvisited_account() {
        let mut discovery = DiscoveryState::default();
        discovery.set_account_id(1001);

        // Before server delta or snapshot sync, status is unvisited and sync pending.
        assert!(!discovery.visited_map("prontera"));
        assert!(!discovery.map_snapshot_complete());
        let sync_pending = destination_detail_lines("geffen", Some("prontera"), None, 4, None, &[], None, &[]);
        assert!(sync_pending[0].contains("discovery sync pending"));

        // Receive private server discovery line marking prontera visited.
        let recognized = discovery.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:1001:prontera]", Some(1001));
        assert!(recognized, "server map discovery protocol line must be recognized");
        assert!(discovery.visited_map("prontera"));
        assert!(!discovery.visited_map("geffen"));

        let visited_details = destination_detail_lines(
            "geffen",
            Some("prontera"),
            None,
            4,
            Some(if discovery.visited_map("prontera") {
                "visited"
            } else {
                "not yet visited"
            }),
            &[],
            None,
            &[],
        );
        assert!(visited_details[0].contains("Destination: prontera • visited • Rune-Midgarts Kingdom"));

        let unvisited_details = destination_detail_lines(
            "prontera",
            Some("geffen"),
            None,
            4,
            Some(if discovery.visited_map("geffen") {
                "visited"
            } else {
                "not yet visited"
            }),
            &[],
            None,
            &[],
        );
        assert!(unvisited_details[0].contains("Destination: geffen • not yet visited • Rune-Midgarts Kingdom"));
    }

    #[test]
    fn two_characters_on_same_account_share_visit_history() {
        let mut discovery = DiscoveryState::default();
        discovery.set_account_id(1001);

        // Character 1 (e.g. Swordsman) visits Prontera, Izlude, and Geffen.
        assert!(discovery.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:1001:prontera]", Some(1001)));
        assert!(discovery.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:1001:izlude]", Some(1001)));
        assert!(discovery.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:1001:geffen]", Some(1001)));
        assert_eq!(discovery.visited_map_count(), 3);

        // Player switches to Character 2 (e.g. Mage) on the same account (account_id
        // remains 1001).
        discovery.set_account_id(1001);
        assert_eq!(discovery.visited_map_count(), 3, "same account must retain all visited maps");
        assert!(discovery.visited_map("prontera"));
        assert!(discovery.visited_map("izlude"));
        assert!(discovery.visited_map("geffen"));

        // Player switches to an alternate account (account_id: 1002).
        discovery.set_account_id(1002);
        assert_eq!(discovery.visited_map_count(), 0, "different account must reset visited maps");
        assert!(!discovery.visited_map("prontera"));
    }

    #[test]
    fn conditional_in_world_transport_routes() {
        let graph = navigation_graph();

        // 1. Izlude -> Byalan Island: conditional ferry service
        let byalan_route = route_edges("izlude", "iz_dun00").expect("ferry route to Byalan");
        assert_eq!(byalan_route.len(), 2);
        assert_eq!(byalan_route[0].id, "service-izlude-byalan-ferry");
        assert_eq!(byalan_route[0].kind, "npc_service");
        assert_eq!(byalan_route[0].availability, "conditional");
        assert_eq!(byalan_route[0].requirements.as_deref(), Some("Costs 150 zeny."));
        assert_eq!(byalan_route[1].kind, "walk_warp");

        // 2. Return from Byalan Island: free sailor
        let return_route = route_edges("izlu2dun", "izlude").expect("return sailor route");
        assert_eq!(return_route.len(), 1);
        assert_eq!(return_route[0].id, "service-byalan-return-sailor");
        assert_eq!(return_route[0].kind, "npc_service");
        assert_eq!(return_route[0].availability, "always");
        assert_eq!(return_route[0].requirements.as_deref(), Some("No fare."));

        // 3. Cat Fleet to Malangdo
        let cat_route = route_edges("izlude", "malangdo").expect("cat fleet route");
        assert_eq!(cat_route.len(), 1);
        assert_eq!(cat_route[0].kind, "npc_service");
        assert_eq!(cat_route[0].availability, "conditional");
        assert!(cat_route[0].requirements.as_deref().unwrap().contains("1000 zeny"));

        // Verify graph maps exist
        assert!(graph.maps.iter().any(|m| m == "izlude"));
        assert!(graph.maps.iter().any(|m| m == "izlu2dun"));
        assert!(graph.maps.iter().any(|m| m == "malangdo"));
    }

    #[test]
    fn unreachable_routes_return_none_and_never_hallucinate() {
        assert!(route_edges("prontera", "nonexistent_map").is_none());
        assert!(route_edges("nonexistent_map", "prontera").is_none());

        let lines = destination_detail_lines(
            "prontera",
            Some("nonexistent_map"),
            None,
            4,
            Some("not yet visited"),
            &[],
            None,
            &[],
        );
        assert!(lines[5].contains("No verified route"));
        assert!(lines[5].contains("No portal route from prontera to nonexistent_map"));
    }

    #[test]
    fn route_selection_safety_guarantee_no_teleportation() {
        let node_click = RouteClick {
            map: "prontera",
            position: TilePosition { x: 155, y: 183 },
        };
        let event = node_click.event();

        let facility_click = FacilityRouteClick {
            map_name: "prontera".to_owned(),
            position: TilePosition { x: 156, y: 191 },
        };
        let facility_event = facility_click.event();

        let mut queue = EventQueue::default();
        queue.queue(event);
        queue.queue(facility_event);

        let events: Vec<_> = queue.drain().collect();
        assert_eq!(events.len(), 2);

        // Destination click dispatches ONLY client-side SetNavigationDestination event
        // (never teleports)
        match &events[0] {
            Event::Application {
                custom_event: InputEvent::SetNavigationDestination { map_name, x, y },
            } => {
                assert_eq!(map_name, "prontera");
                assert_eq!(*x, 155);
                assert_eq!(*y, 183);
            }
            _ => panic!("expected SetNavigationDestination"),
        }

        // Facility click dispatches ONLY client-side SetNavigationDestination event
        match &events[1] {
            Event::Application {
                custom_event: InputEvent::SetNavigationDestination { map_name, x, y },
            } => {
                assert_eq!(map_name, "prontera");
                assert_eq!(*x, 156);
                assert_eq!(*y, 191);
            }
            _ => panic!("expected SetNavigationDestination"),
        }
    }

    #[test]
    fn atlas_roads_classification_and_region_coverage() {
        // Every atlas location must have a valid regional assignment matching
        // map_region
        for location in LOCATIONS {
            assert_eq!(
                location.region(),
                map_region(location.map).expect("atlas location must have a known region"),
                "location {} mismatch",
                location.map
            );
        }

        // Roads must contain both overland walk roads and in-world transport
        // connections
        let walk_roads = ROADS.iter().filter(|r| r.kind == RoadKind::Walk).count();
        let transport_roads = ROADS.iter().filter(|r| r.kind == RoadKind::Transport).count();
        assert!(walk_roads >= 10, "expected at least 10 overland walk roads, got {walk_roads}");
        assert!(
            transport_roads >= 10,
            "expected at least 10 in-world transport routes, got {transport_roads}"
        );

        // Every road that is marked available must have a verified route in the
        // navigation graph
        let available = available_roads();
        assert!(!available.is_empty(), "available roads must not be empty");
        for road in available {
            let from_map = LOCATIONS[road.from].map;
            let to_map = LOCATIONS[road.to].map;
            assert!(
                route_edges(from_map, to_map).is_some(),
                "road between {from_map} and {to_map} must have a verified route"
            );
        }
    }

    #[test]
    fn map_information_spawn_range_and_population_summary() {
        let reference = crate::dm::reference_data::reference_data();
        let details = reference
            .map_spawn_details("prt_fild08")
            .expect("prt_fild08 must have static spawn records");
        assert!(details.records > 0);
        assert!(details.species > 0);
        assert!(details.min_level <= details.max_level);

        let lines = destination_detail_lines("prontera", Some("prt_fild08"), None, 4, Some("visited"), &[], Some(details), &[
        ]);
        let expected_range = if details.min_level != details.max_level {
            format!("(range: {}–{})", details.min_level, details.max_level)
        } else {
            String::new()
        };
        assert!(lines[1].contains(&format!("Suggested level: ~{}", details.mean_level)));
        if !expected_range.is_empty() {
            assert!(lines[1].contains(&expected_range));
        }
        assert!(lines[1].contains("(static-spawn mean)"));
        assert!(lines[2].contains(&format!(
            "Static population: {} spawn records • {} species",
            details.records, details.species
        )));
    }

    #[test]
    fn map_information_low_coverage_labeling() {
        let lines = destination_detail_lines("prontera", Some("unknown_map"), None, 0, None, &[], None, &[]);
        assert_eq!(lines[1], "Suggested level: unavailable (low-coverage or non-combat map)");
        assert_eq!(lines[2], "Static population: no verified spawn records (low coverage)");
    }

    #[test]
    fn map_information_danger_guidance_and_entry_warning() {
        assert!(is_dangerous_map_level(35, 20));
        assert!(!is_dangerous_map_level(34, 20));
        assert!(!is_dangerous_map_level(20, 20));

        let warning = crate::map_difficulty_warning("orcsdun02", Some(35), Some(20), true).expect("expected danger warning");
        assert!(warning.contains("orcsdun02 averages level 35"));
        assert!(warning.contains("15+ above your level (20)"));
        assert!(warning.contains("Warning only; travel is unrestricted."));

        assert!(
            crate::map_difficulty_warning("orcsdun02", Some(35), Some(20), false).is_none(),
            "opt-out toggle false must suppress danger warning"
        );
        assert!(
            crate::map_difficulty_warning("unknown_map", None, Some(20), true).is_none(),
            "missing spawn records must not guess or trigger warning"
        );
    }
}
