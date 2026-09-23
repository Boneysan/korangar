use std::sync::OnceLock;

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::Element;
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::TilePosition;
use rust_state::State;

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::route_edges;

#[derive(Clone, Copy)]
struct AtlasLocation {
    map: &'static str,
    label: &'static str,
    x: f32,
    y: f32,
    tile: TilePosition,
}

// Positions are a readable regional overview rather than in-game coordinates.
const LOCATIONS: &[AtlasLocation] = &[
    AtlasLocation { map: "yuno", label: "Juno", x: 0.48, y: 0.12, tile: TilePosition { x: 157, y: 123 } },
    AtlasLocation { map: "rachel", label: "Rachel", x: 0.69, y: 0.12, tile: TilePosition { x: 120, y: 120 } },
    AtlasLocation { map: "hugel", label: "Hugel", x: 0.84, y: 0.27, tile: TilePosition { x: 96, y: 145 } },
    AtlasLocation { map: "aldebaran", label: "Al De Baran", x: 0.39, y: 0.27, tile: TilePosition { x: 140, y: 131 } },
    AtlasLocation { map: "geffen", label: "Geffen", x: 0.20, y: 0.34, tile: TilePosition { x: 119, y: 59 } },
    AtlasLocation { map: "prontera", label: "Prontera", x: 0.48, y: 0.43, tile: TilePosition { x: 155, y: 183 } },
    AtlasLocation { map: "payon", label: "Payon", x: 0.76, y: 0.39, tile: TilePosition { x: 160, y: 120 } },
    AtlasLocation { map: "morocc", label: "Morroc", x: 0.31, y: 0.57, tile: TilePosition { x: 156, y: 97 } },
    AtlasLocation { map: "izlude", label: "Izlude", x: 0.55, y: 0.60, tile: TilePosition { x: 128, y: 146 } },
    AtlasLocation { map: "alberta", label: "Alberta", x: 0.68, y: 0.58, tile: TilePosition { x: 28, y: 234 } },
    AtlasLocation { map: "amatsu", label: "Amatsu", x: 0.88, y: 0.52, tile: TilePosition { x: 198, y: 84 } },
    AtlasLocation { map: "comodo", label: "Comodo", x: 0.17, y: 0.72, tile: TilePosition { x: 184, y: 151 } },
    AtlasLocation { map: "umbala", label: "Umbala", x: 0.31, y: 0.78, tile: TilePosition { x: 97, y: 153 } },
    AtlasLocation { map: "jawaii", label: "Jawaii", x: 0.49, y: 0.82, tile: TilePosition { x: 251, y: 132 } },
    AtlasLocation { map: "louyang", label: "Louyang", x: 0.71, y: 0.76, tile: TilePosition { x: 217, y: 100 } },
    AtlasLocation { map: "gonryun", label: "Gonryun", x: 0.84, y: 0.68, tile: TilePosition { x: 160, y: 120 } },
    AtlasLocation { map: "ayothaya", label: "Ayothaya", x: 0.88, y: 0.84, tile: TilePosition { x: 208, y: 166 } },
    AtlasLocation { map: "brasilis", label: "Brasilis", x: 0.10, y: 0.88, tile: TilePosition { x: 196, y: 217 } },
    AtlasLocation { map: "einbroch", label: "Einbroch", x: 0.57, y: 0.22, tile: TilePosition { x: 64, y: 200 } },
    AtlasLocation { map: "einbech", label: "Einbech", x: 0.66, y: 0.29, tile: TilePosition { x: 63, y: 35 } },
    AtlasLocation { map: "lighthalzen", label: "Lighthalzen", x: 0.55, y: 0.34, tile: TilePosition { x: 158, y: 92 } },
    AtlasLocation { map: "lasagna", label: "Lasagna", x: 0.96, y: 0.36, tile: TilePosition { x: 193, y: 182 } },
    AtlasLocation { map: "dicastes01", label: "El Dicastes", x: 0.80, y: 0.16, tile: TilePosition { x: 198, y: 187 } },
    AtlasLocation { map: "xmas", label: "Lutie", x: 0.31, y: 0.19, tile: TilePosition { x: 147, y: 134 } },
    AtlasLocation { map: "mid_camp", label: "Midgard Camp", x: 0.39, y: 0.69, tile: TilePosition { x: 180, y: 240 } },
    AtlasLocation { map: "c_tower1", label: "Clock Tower", x: 0.28, y: 0.27, tile: TilePosition { x: 235, y: 218 } },
    AtlasLocation { map: "ama_dun01", label: "Amatsu Cave", x: 0.93, y: 0.59, tile: TilePosition { x: 54, y: 107 } },
];

// Authored regional roads keep the overview legible. Actual route highlighting
// and the minimap guidance use the generated, verified warp graph.
const ROADS: &[(usize, usize)] = &[
    (0, 18), (0, 22), (1, 22), (2, 22), (18, 19), (18, 9), (19, 20), (20, 9),
    (3, 4), (3, 5), (3, 23), (4, 5), (4, 7), (4, 23), (5, 6), (5, 7), (5, 8),
    (5, 24), (6, 8), (6, 9), (6, 15), (7, 12), (7, 24), (8, 9), (8, 13), (9, 10),
    (9, 13), (10, 15), (10, 26), (11, 12), (12, 17), (13, 14), (14, 15), (14, 16),
];

struct RouteClick {
    map: &'static str,
    position: TilePosition,
}

impl ClickHandler<ClientState> for RouteClick {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(InputEvent::SetNavigationDestination {
            map_name: self.map.to_owned(),
            x: self.position.x,
            y: self.position.y,
        });
    }
}

struct AtlasNode {
    location: AtlasLocation,
    click: RouteClick,
}

struct AtlasView {
    nodes: Vec<AtlasNode>,
}

impl AtlasView {
    fn new() -> Self {
        Self {
            nodes: LOCATIONS
                .iter()
                .copied()
                .map(|location| AtlasNode {
                    location,
                    click: RouteClick { map: location.map, position: location.tile },
                })
                .collect(),
        }
    }
}

impl Element<ClientState> for AtlasView {
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        _: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| resolver.with_height(520.0))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        area: &Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let minimap = state.get(&client_state().minimap());
        let current_map = minimap.map_name();
        let target = minimap.navigation_target().map(|(map, _, _)| map.as_str());
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
                area.top + location.y * (area.height - node_height - 42.0),
            )
        };
        for &(from, to) in available_roads() {
            let (Some(left), Some(right)) = (self.nodes.get(from), self.nodes.get(to)) else {
                continue;
            };
            let (x1, y1) = point(left.location);
            let (x2, y2) = point(right.location);
            draw_dotted_connection(layout, x1 + node_width / 2.0, y1 + node_height / 2.0, x2 + node_width / 2.0, y2 + node_height / 2.0, Color::rgb_u8(67, 100, 121));
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
            draw_dotted_connection(layout, x1 + node_width / 2.0, y1 + node_height / 2.0, x2 + node_width / 2.0, y2 + node_height / 2.0, Color::rgb_u8(255, 204, 84));
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
            layout.add_rectangle(node_area, CornerDiameter::uniform(7.0), color, Color::rgba_u8(0, 0, 0, 150), ShadowPadding::uniform(2.0));
            layout.add_text(
                node_area,
                node.location.label,
                FontSize(12.0),
                Color::rgb_u8(239, 235, 215),
                Color::WHITE,
                HorizontalAlignment::Center { offset: 2.0, border: 3.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
            if is_hovered {
                layout.register_click_handler(MouseButton::Left, &node.click);
            }
        }

        let status = if target.is_some() && route.is_none() {
            "No verified portal route from this map. Choose a reachable location."
        } else if let Some(target) = target {
            // target only borrows minimap state, so display a static prompt here;
            // the exact destination is also shown beneath the map window.
            let _ = target;
            "Gold path: selected route  •  Cyan dots: walkable trail on current map"
        } else {
            "Select a town or destination to plot a route. Click Clear Route to cancel."
        };
        layout.add_text(
            Area { left: area.left + 12.0, top: area.top + area.height - 27.0, width: area.width - 24.0, height: 20.0 },
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

fn available_roads() -> &'static [(usize, usize)] {
    static AVAILABLE: OnceLock<Vec<(usize, usize)>> = OnceLock::new();
    AVAILABLE.get_or_init(|| {
        ROADS
            .iter()
            .copied()
            .filter(|&(from, to)| route_edges(LOCATIONS[from].map, LOCATIONS[to].map).is_some())
            .collect()
    })
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
            Area { left: x - 2.0, top: y - 2.0, width: 4.0, height: 4.0 },
            CornerDiameter::uniform(2.0),
            color,
            Color::rgba_u8(0, 0, 0, 0),
            ShadowPadding::uniform(0.0),
        );
    }
}

pub struct MapsWindow;

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
            elements: [
                AtlasView::new(),
                button! {
                    text: "Clear Route",
                    tooltip: "Clear the current destination and breadcrumb trail",
                    event: InputEvent::ClearNavigationDestination,
                },
            ],
        }
    }
}
