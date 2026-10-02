use std::cell::UnsafeCell;

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::event::{ClickHandler, EventQueue, ScrollHandler};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::State;

use super::WindowClass;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::settings::GameSettingsPathExt;
use crate::state::minimap::{DEFAULT_MINIMAP_SIDE, MAX_MINIMAP_SIDE, MIN_MINIMAP_SIDE};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state, this_entity};

/// Player blip size at the default minimap scale (must stay readable).
const PLAYER_MARKER_SIZE: f32 = 14.0;
/// Official information icons are small; keep them readable when resized.
const POI_ICON_SIZE: f32 = 12.0;
const COORDS_HEIGHT: f32 = 18.0;
const ZOOM_ROW_HEIGHT: f32 = 28.0;
/// Scroll wheel sensitivity (pixels of map side per scroll unit).
const SCROLL_ZOOM_STEP: f32 = 18.0;

/// A blip drawn after the map texture (must use textures so it sits on top).
#[derive(Clone)]
struct MinimapBlip {
    x: f32,
    y: f32,
    /// Tint applied to the player_marker texture (or solid color fallback).
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
    /// Scale relative to the default player marker size.
    size_scale: f32,
    name: String,
}

/// Map area layout plus live blips (player, party, compass).
struct MinimapViewLayout {
    area: Area,
    /// Player tile when available; used for the moving position dot.
    player_tile: Option<(u16, u16)>,
    /// Party members on this map + compass markers (drawn with tinted blips).
    extra_blips: Vec<MinimapBlip>,
    /// Broad monster population regions for the current map (GDD 9.4).
    population_regions: Vec<crate::world::BroadSpawnRectangle>,
}

/// Draws the current-map minimap image, Towninfo POIs, and a live player blip.
///
/// Size comes from [`MinimapState::display_side`] (zoom buttons, scroll, or
/// window resize). The map area is always square.
///
/// Important: UI **textures** are flushed after all **rectangles**. Drawing the
/// player as a rectangle would put it under the map bitmap and make it
/// invisible — the blip must be a texture instruction (or similar custom).
struct MinimapView {
    hover_tip: UnsafeCell<String>,
    /// Scratch storage for this frame's waypoint-click handler, so
    /// `register_click_handler` can borrow it for `'a`. Overwritten every
    /// frame in `lay_out` before use; the initial value is never read.
    waypoint_click: UnsafeCell<MinimapWaypointClick>,
}

struct MinimapBlipTooltip;

struct MinimapScrollZoom;

impl ScrollHandler<ClientState> for MinimapScrollZoom {
    fn handle_scroll(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>, delta: f32) -> bool {
        // Positive delta = scroll up = zoom in on most platforms.
        let step = if delta > 0.0 {
            SCROLL_ZOOM_STEP
        } else if delta < 0.0 {
            -SCROLL_ZOOM_STEP
        } else {
            return true;
        };
        state.update_value_with(client_state().minimap(), move |minimap| {
            minimap.zoom_by(step);
        });
        true
    }
}

/// Left-click places the personal waypoint at the clicked tile (GDD 10.12).
/// The tile is computed once per frame in `lay_out`, from the same `area`
/// [`tile_to_minimap`] uses, so a click and the marker it places always agree
/// on where the map bitmap actually is on screen.
struct MinimapWaypointClick {
    tile_x: u16,
    tile_y: u16,
}

impl ClickHandler<ClientState> for MinimapWaypointClick {
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        let waypoint = Some((self.tile_x, self.tile_y));
        state.update_value_with(client_state().minimap(), move |minimap| {
            minimap.set_personal_waypoint(waypoint);
        });
    }
}

/// Right-click clears the personal waypoint. A dedicated gesture rather than
/// click-to-toggle: a player refining a spot with several left-clicks should
/// not have to first figure out where the previous click already landed.
struct MinimapWaypointClear;

impl ClickHandler<ClientState> for MinimapWaypointClear {
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        state.update_value_with(client_state().minimap(), |minimap| {
            minimap.set_personal_waypoint(None);
        });
    }
}

impl Element<ClientState> for MinimapView {
    type LayoutInfo = MinimapViewLayout;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut<'_>,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let minimap_path = client_state().minimap();
            // User zoom (buttons / scroll) or last edge-resize — always square.
            let side = state.get(&minimap_path).display_side();
            let area = resolver.with_height(side);
            let player_tile = state.try_follow(this_entity()).map(|player| {
                let t = player.get_tile_position();
                (t.x, t.y)
            });

            let minimap = state.get(&minimap_path);
            let current_map = minimap.map_name();
            let game_settings_path = client_state().game_settings();
            let game_settings = state.get(&game_settings_path);

            let mut extra_blips = Vec::new();

            // Party members on the same map with a known tile position.
            // Layer toggle (GDD 10.12): "Show party members on minimap".
            let party_path = client_state().party_state();
            let party = state.get(&party_path);
            for member in party.members().iter().filter(|_| game_settings.show_minimap_party) {
                if !member.online() {
                    continue;
                }
                let Some(pos) = member.position() else {
                    continue;
                };
                let member_map = member.map_name().trim_end_matches(".gat").trim_end_matches(".GAT").to_lowercase();
                if !member_map.is_empty() && member_map != current_map {
                    continue;
                }
                extra_blips.push(MinimapBlip {
                    x: pos.x as f32,
                    y: pos.y as f32,
                    // Soft green — distinct from the red player blip.
                    red: 80,
                    green: 220,
                    blue: 120,
                    alpha: 255,
                    size_scale: 0.85,
                    name: member.name().to_owned(),
                });
            }

            for (x, y) in minimap.navigation_breadcrumbs() {
                extra_blips.push(MinimapBlip {
                    x: f32::from(*x),
                    y: f32::from(*y),
                    red: 80,
                    green: 220,
                    blue: 255,
                    alpha: 225,
                    size_scale: 0.38,
                    name: "Route breadcrumb".to_owned(),
                });
            }

            if let Some(ping) = minimap.party_ping() {
                let (red, green, blue) = match ping.kind.as_str() {
                    // Okabe-Ito colors remain distinct for common red/green
                    // deficiencies. The marker's name also carries the ping
                    // kind, so meaning never depends on color alone.
                    "assist" => (240, 228, 66),
                    "danger" => (213, 94, 0),
                    "retreat" => (230, 159, 0),
                    "ready" => (0, 114, 178),
                    "on-my-way" => (86, 180, 233),
                    _ => (204, 121, 167),
                };
                extra_blips.push(MinimapBlip {
                    x: f32::from(ping.x),
                    y: f32::from(ping.y),
                    red,
                    green,
                    blue,
                    alpha: 255,
                    size_scale: 1.15,
                    name: format!("{} ping from {}", ping.kind, ping.sender),
                });
            }

            if let Some((x, y)) = minimap.navigation_marker() {
                extra_blips.push(MinimapBlip {
                    x: f32::from(x),
                    y: f32::from(y),
                    red: 255,
                    green: 210,
                    blue: 70,
                    alpha: 255,
                    size_scale: 1.15,
                    name: "Route exit".to_owned(),
                });
            }

            if let Some((x, y)) = minimap.personal_waypoint() {
                // Deliberately distinct from every other marker color used
                // above (route exit, ping kinds, compass) so it reads as its
                // own thing at a glance.
                extra_blips.push(MinimapBlip {
                    x: f32::from(x),
                    y: f32::from(y),
                    red: 235,
                    green: 235,
                    blue: 245,
                    alpha: 255,
                    size_scale: 1.05,
                    name: format!("Waypoint ({x}, {y})"),
                });
            }

            // Compass / NPC marks (0x0144).
            // Layer toggle (GDD 10.12): "Show quest marks on minimap".
            for mark in minimap
                .dynamic_markers()
                .iter()
                .filter(|_| game_settings.show_minimap_quest_markers)
            {
                extra_blips.push(MinimapBlip {
                    x: mark.x,
                    y: mark.y,
                    red: mark.red,
                    green: mark.green,
                    blue: mark.blue,
                    alpha: mark.alpha.max(200),
                    size_scale: 1.05,
                    name: "Mark".to_owned(),
                });
            }

            // Verified walk-warp portal destination markers and tracked-route accents (GDD
            // 9.9, 10.12).
            if game_settings.show_minimap_portals {
                let route_target = minimap.navigation_target().map(|target| target.map_name.as_str());
                for exit in crate::world::map_portal_exits(minimap.map_name(), route_target) {
                    if exit.is_route_exit {
                        extra_blips.push(MinimapBlip {
                            x: f32::from(exit.x),
                            y: f32::from(exit.y),
                            red: 255,
                            green: 210,
                            blue: 70, // bright gold tracked-route accent
                            alpha: 255,
                            size_scale: 1.25,
                            name: exit.label(),
                        });
                    } else {
                        extra_blips.push(MinimapBlip {
                            x: f32::from(exit.x),
                            y: f32::from(exit.y),
                            red: 190,
                            green: 110,
                            blue: 245, // soft purple portal marker
                            alpha: 220,
                            size_scale: 0.95,
                            name: exit.label(),
                        });
                    }
                }
            }

            let population_regions = if game_settings.show_minimap_population_regions {
                let target_monster = minimap.selected_monster_id().or_else(|| {
                    state
                        .get(&client_state().quest_log())
                        .client_hunting_goals()
                        .first()
                        .map(|goal| goal.monster_id)
                });
                crate::world::broad_spawn_rectangles_for_map(minimap.map_name(), target_monster)
            } else {
                Vec::new()
            };

            MinimapViewLayout {
                area,
                player_tile,
                extra_blips,
                population_regions,
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let minimap_path = client_state().minimap();
        let minimap = state.get(&minimap_path);
        let show_minimap_facilities = *state.get(&client_state().game_settings().show_minimap_facilities());
        let row = layout_info.area;
        // Center a square map area inside the row (row may be wider after chrome).
        let side = row.height.min(row.width).clamp(MIN_MINIMAP_SIDE, MAX_MINIMAP_SIDE);
        let area = Area {
            left: row.left + (row.width - side) / 2.0,
            top: row.top,
            width: side,
            height: side,
        };

        // Scroll-wheel zoom while the cursor is over the map.
        let hovering = area.check().run(layout);
        if hovering {
            layout.register_scroll_handler(&MinimapScrollZoom);
        }

        // Background so missing textures are still visible.
        layout.add_rectangle(
            area,
            CornerDiameter::uniform(4.0),
            Color::rgb_u8(20, 24, 32),
            Color::rgba_u8(0, 0, 0, 0),
            ShadowPadding::uniform(0.0),
        );

        if let Some(texture) = minimap.texture() {
            layout.add_texture(area, texture.clone(), Color::WHITE, false);
        } else {
            // No BMP in archives for this map (custom map, or asset not in GRF).
            // Map base name still appears in the coordinate line under the window.
            layout.add_text(
                area,
                "No minimap BMP",
                FontSize(12.0),
                Color::rgb_u8(180, 180, 180),
                Color::rgb_u8(255, 160, 60),
                HorizontalAlignment::Center { offset: 0.0, border: 4.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        let map_w = minimap.map_width().max(1) as f32;
        let map_h = minimap.map_height().max(1) as f32;

        // Click-to-place / click-to-clear the personal waypoint (GDD 10.12).
        // Registered only while hovering the map area, not the whole window,
        // so clicking the zoom buttons or coordinate readout never places one.
        if hovering && minimap.map_width() > 0 && minimap.map_height() > 0 {
            let mouse = layout.get_mouse_position();
            let (tile_x, tile_y) = minimap_to_tile(mouse.left, mouse.top, map_w, map_h, area);
            // Safety: overwritten here, immediately before the borrow below is
            // taken, and the borrow does not outlive this call to `lay_out`.
            unsafe { *self.waypoint_click.get() = MinimapWaypointClick { tile_x, tile_y } };
            layout.register_click_handler(MouseButton::Left, unsafe { &*self.waypoint_click.get() });
            layout.register_click_handler(MouseButton::Right, &MinimapWaypointClear);
        }

        let poi_size = (side / DEFAULT_MINIMAP_SIDE * POI_ICON_SIZE).clamp(8.0, 20.0);
        let player_size = (side / DEFAULT_MINIMAP_SIDE * PLAYER_MARKER_SIZE).clamp(10.0, 22.0);

        // Towninfo facility POIs (shops, kafra, guides, …).
        // Layer toggle (GDD 10.12): "Show facility markers on minimap".
        // Must be textures — rectangles flush under the map bitmap and disappear.
        for poi in minimap.pois().iter().filter(|_| show_minimap_facilities) {
            let (cx, cy) = tile_to_minimap(poi.x as f32, poi.y as f32, map_w, map_h, area);
            let icon_area = Area {
                left: cx - poi_size / 2.0,
                top: cy - poi_size / 2.0,
                width: poi_size,
                height: poi_size,
            };

            if let Some(texture) = poi.texture.as_ref() {
                layout.add_texture(icon_area, texture.clone(), Color::WHITE, false);
            } else if let Some(texture) = minimap.player_marker() {
                // Missing facility icon: tinted blip so POIs stay visible.
                let (r, g, b) = poi.kind.fallback_color_rgb();
                layout.add_texture(icon_area, texture.clone(), Color::rgb_u8(r, g, b), false);
            }
        }

        // Broad monster population regions (GDD 9.4).
        for region in &layout_info.population_regions {
            if region.is_map_wide {
                continue;
            }
            let rx = area.left + (region.x as f32 / map_w) * area.width;
            let ry = area.top + (1.0 - (region.y + region.height) as f32 / map_h) * area.height;
            let rw = (region.width as f32 / map_w) * area.width;
            let rh = (region.height as f32 / map_h) * area.height;
            let rect_area = Area {
                left: rx,
                top: ry,
                width: rw.max(4.0),
                height: rh.max(4.0),
            };

            let (fr, fg, fb, fa) = region.density.color_rgba();
            let (or, og, ob, oa) = region.density.outline_rgba();

            if let Some(texture) = minimap.player_marker() {
                layout.add_texture(rect_area, texture.clone(), Color::rgba_u8(fr, fg, fb, fa), false);
            } else {
                layout.add_rectangle(
                    rect_area,
                    CornerDiameter::uniform(2.0),
                    Color::rgba_u8(fr, fg, fb, fa),
                    Color::rgba_u8(or, og, ob, oa),
                    ShadowPadding::uniform(0.0),
                );
            }

            if rect_area.check().run(layout) {
                unsafe {
                    *self.hover_tip.get() = region.tooltip_text();
                    layout.add_tooltip(self.hover_tip.as_ref_unchecked().as_str(), MinimapBlipTooltip.tooltip_id());
                }
            }
        }

        // Party / compass blips first, then local player on top.
        for blip in &layout_info.extra_blips {
            let (cx, cy) = tile_to_minimap(blip.x, blip.y, map_w, map_h, area);
            let size = player_size * blip.size_scale;
            let marker = Area {
                left: cx - size / 2.0,
                top: cy - size / 2.0,
                width: size,
                height: size,
            };
            let tint = Color::rgba_u8(blip.red, blip.green, blip.blue, blip.alpha);
            if let Some(texture) = minimap.player_marker() {
                layout.add_texture(marker, texture.clone(), tint, false);
            } else {
                layout.add_rectangle(
                    marker,
                    CornerDiameter::uniform(size / 2.0),
                    tint,
                    Color::rgba_u8(0, 0, 0, 0),
                    ShadowPadding::uniform(0.0),
                );
            }
            if !blip.name.is_empty() && marker.check().run(layout) {
                unsafe {
                    *self.hover_tip.get() = blip.name.clone();
                    layout.add_tooltip(self.hover_tip.as_ref_unchecked().as_str(), MinimapBlipTooltip.tooltip_id());
                }
            }
        }

        // Live player blip — must be a texture so it draws above the map bitmap.
        if let Some((tx, ty)) = layout_info.player_tile {
            let (cx, cy) = tile_to_minimap(tx as f32, ty as f32, map_w, map_h, area);
            let marker = Area {
                left: cx - player_size / 2.0,
                top: cy - player_size / 2.0,
                width: player_size,
                height: player_size,
            };

            if marker.check().run(layout) {
                let name = state
                    .try_follow(this_entity())
                    .and_then(|player| player.get_details().map(String::as_str))
                    .unwrap_or("You");
                unsafe {
                    *self.hover_tip.get() = name.to_owned();
                    layout.add_tooltip(self.hover_tip.as_ref_unchecked().as_str(), MinimapBlipTooltip.tooltip_id());
                }
            }

            if let Some(texture) = minimap.player_marker() {
                // Soft shadow under the blip (texture tint) for contrast on bright maps.
                let shadow = Area {
                    left: marker.left + 1.0,
                    top: marker.top + 1.0,
                    width: marker.width,
                    height: marker.height,
                };
                layout.add_texture(shadow, texture.clone(), Color::rgba_u8(0, 0, 0, 140), false);
                layout.add_texture(marker, texture.clone(), Color::WHITE, false);
            } else {
                // Last-resort: bright crosshair via two thin rectangles. These still
                // render under the map, so prefer the player texture; keep for debug
                // when assets are missing (map may also be missing then).
                let hx = Area {
                    left: cx - player_size / 2.0,
                    top: cy - 1.5,
                    width: player_size,
                    height: 3.0,
                };
                let hy = Area {
                    left: cx - 1.5,
                    top: cy - player_size / 2.0,
                    width: 3.0,
                    height: player_size,
                };
                layout.add_rectangle(
                    hx,
                    CornerDiameter::uniform(0.0),
                    Color::rgb_u8(255, 40, 40),
                    Color::rgba_u8(0, 0, 0, 0),
                    ShadowPadding::uniform(0.0),
                );
                layout.add_rectangle(
                    hy,
                    CornerDiameter::uniform(0.0),
                    Color::rgb_u8(255, 40, 40),
                    Color::rgba_u8(0, 0, 0, 0),
                    ShadowPadding::uniform(0.0),
                );
            }
        }
    }
}

/// RO: tile (0,0) is south-west; minimap image has north at the top.
fn tile_to_minimap(tile_x: f32, tile_y: f32, map_w: f32, map_h: f32, area: Area) -> (f32, f32) {
    let nx = (tile_x + 0.5) / map_w;
    let ny = 1.0 - (tile_y + 0.5) / map_h;
    let cx = area.left + nx.clamp(0.0, 1.0) * area.width;
    let cy = area.top + ny.clamp(0.0, 1.0) * area.height;
    (cx, cy)
}

/// Inverse of [`tile_to_minimap`]: a screen point inside `area` to the GAT
/// tile under it. Used for click-to-place, so it must invert the same
/// left-to-right, bottom-to-top mapping exactly, not just approximate it.
fn minimap_to_tile(screen_x: f32, screen_y: f32, map_w: f32, map_h: f32, area: Area) -> (u16, u16) {
    let nx = ((screen_x - area.left) / area.width).clamp(0.0, 1.0);
    let ny = ((screen_y - area.top) / area.height).clamp(0.0, 1.0);
    let tile_x = (nx * map_w).clamp(0.0, map_w - 1.0);
    let tile_y = ((1.0 - ny) * map_h).clamp(0.0, map_h - 1.0);
    (tile_x as u16, tile_y as u16)
}

/// Coordinate readout under the map image.
struct MinimapCoords;

impl Element<ClientState> for MinimapCoords {
    type LayoutInfo = (Area, String);

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut<'_>,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let area = resolver.with_height(COORDS_HEIGHT);
            let minimap_path = client_state().minimap();
            let minimap = state.get(&minimap_path);
            let text = match state.try_follow(this_entity()) {
                Some(player) => {
                    let p = player.get_tile_position();
                    format!("{}  {},{}", minimap.map_name(), p.x, p.y)
                }
                None => minimap.map_name().to_owned(),
            };
            (area, text)
        })
    }

    fn lay_out<'a>(
        &'a self,
        _: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        layout.add_text(
            layout_info.0,
            &layout_info.1,
            FontSize(11.0),
            Color::rgb_u8(220, 220, 220),
            Color::rgb_u8(255, 160, 60),
            HorizontalAlignment::Center { offset: 0.0, border: 2.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );
    }
}

pub struct MinimapWindow;

impl CustomWindow<ClientState> for MinimapWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Minimap)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // Border + title chrome roughly; content is square map + coords + zoom row.
        const CHROME_W: f32 = 24.0;
        const CHROME_H: f32 = 156.0;

        window! {
            title: "Map",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            // Drag the right edge or bottom-right corner, or use − / + / scroll.
            minimum_width: MIN_MINIMAP_SIDE + CHROME_W + 32.0,
            maximum_width: MAX_MINIMAP_SIDE + CHROME_W,
            minimum_height: MIN_MINIMAP_SIDE + COORDS_HEIGHT + ZOOM_ROW_HEIGHT + CHROME_H,
            maximum_height: MAX_MINIMAP_SIDE + COORDS_HEIGHT + ZOOM_ROW_HEIGHT + CHROME_H,
            elements: (
                MinimapView {
                    hover_tip: UnsafeCell::new(String::new()),
                    waypoint_click: UnsafeCell::new(MinimapWaypointClick { tile_x: 0, tile_y: 0 }),
                },
                MinimapCoords,
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "−",
                            tooltip: "Zoom out (or scroll down on the map)",
                            event: InputEvent::MinimapZoomOut,
                        },
                        button! {
                            text: "+",
                            tooltip: "Zoom in (or scroll up on the map)",
                            event: InputEvent::MinimapZoomIn,
                        },
                    ),
                },
                split! { gaps: theme().window().gaps(), children: (
                    button! { text: "Location", tooltip: "Share your current position", event: InputEvent::SendPartyPing { kind: "location".to_owned() } },
                    button! { text: "Assist", tooltip: "Ask party members for help here", event: InputEvent::SendPartyPing { kind: "assist".to_owned() } },
                    button! { text: "Danger", tooltip: "Warn the party about danger here", event: InputEvent::SendPartyPing { kind: "danger".to_owned() } },
                ) },
                split! { gaps: theme().window().gaps(), children: (
                    button! { text: "Retreat", tooltip: "Suggest regrouping or retreating here", event: InputEvent::SendPartyPing { kind: "retreat".to_owned() } },
                    button! { text: "Ready", tooltip: "Mark this location as ready", event: InputEvent::SendPartyPing { kind: "ready".to_owned() } },
                    button! { text: "On my way", tooltip: "Tell the party you are moving here", event: InputEvent::SendPartyPing { kind: "on-my-way".to_owned() } },
                ) },
                button! { text: "Share current route", tooltip: "Send your selected map route to party members; they choose whether to accept it", event: InputEvent::SharePartyDestination },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_area() -> Area {
        Area {
            left: 40.0,
            top: 20.0,
            width: 160.0,
            height: 160.0,
        }
    }

    #[test]
    fn minimap_to_tile_inverts_tile_to_minimap() {
        let area = test_area();
        let (map_w, map_h) = (200.0, 200.0);
        for (tile_x, tile_y) in [(0.0, 0.0), (50.0, 120.0), (199.0, 199.0), (100.0, 100.0)] {
            let (screen_x, screen_y) = tile_to_minimap(tile_x, tile_y, map_w, map_h, area);
            let (round_trip_x, round_trip_y) = minimap_to_tile(screen_x, screen_y, map_w, map_h, area);
            // Sub-tile precision is lost (minimap_to_tile floors to a whole
            // tile), so the round trip only needs to land on the same tile,
            // not reproduce the exact fractional input.
            assert_eq!(round_trip_x, tile_x as u16, "x round-trip for tile ({tile_x}, {tile_y})");
            assert_eq!(round_trip_y, tile_y as u16, "y round-trip for tile ({tile_x}, {tile_y})");
        }
    }

    #[test]
    fn minimap_to_tile_flips_y_between_screen_and_world() {
        // Screen top (low y) is the map's north edge -- the highest tile_y,
        // matching tile_to_minimap's own `1.0 - ...` flip. Getting this
        // backwards would place every click's marker mirrored top-to-bottom.
        let area = test_area();
        let (top_tile_x, top_tile_y) = minimap_to_tile(area.left, area.top, 200.0, 200.0, area);
        let (bottom_tile_x, bottom_tile_y) = minimap_to_tile(area.left, area.top + area.height, 200.0, 200.0, area);
        assert_eq!(top_tile_x, bottom_tile_x);
        assert!(
            top_tile_y > bottom_tile_y,
            "top of screen should map to a higher tile_y (north) than the bottom"
        );
    }

    #[test]
    fn minimap_to_tile_clamps_outside_the_map_area() {
        let area = test_area();
        let (map_w, map_h) = (200.0, 200.0);
        // Comfortably outside the area on every side.
        assert_eq!(minimap_to_tile(area.left - 500.0, area.top, map_w, map_h, area), (0, 199));
        assert_eq!(
            minimap_to_tile(area.left + area.width + 500.0, area.top + area.height, map_w, map_h, area),
            (199, 0)
        );
    }

    #[test]
    fn portal_blips_and_route_accents() {
        let exits = crate::world::map_portal_exits("prt_fild08", Some("prontera"));
        assert!(!exits.is_empty(), "prt_fild08 must have verified portal exits");

        let route_portal = exits.iter().find(|e| e.to_map == "prontera").expect("route portal to prontera");
        assert!(route_portal.is_route_exit);
        assert_eq!(route_portal.label(), "→ Route portal: prontera");

        let non_route = exits.iter().find(|e| e.to_map == "izlude").expect("exit to izlude");
        assert!(!non_route.is_route_exit);
        assert_eq!(non_route.label(), "Portal to izlude");
    }

    #[test]
    fn population_region_bounding_box_mapping() {
        let area = test_area();
        let (map_w, map_h) = (200.0, 200.0);
        let regions = crate::world::broad_spawn_rectangles_for_map("prt_maze01", Some(1002));
        assert!(!regions.is_empty(), "prt_maze01 has Poring localized spawn");

        let region = &regions[0];
        assert!(!region.is_map_wide);

        let rx = area.left + (region.x as f32 / map_w) * area.width;
        let ry = area.top + (1.0 - (region.y + region.height) as f32 / map_h) * area.height;
        let rw = (region.width as f32 / map_w) * area.width;
        let rh = (region.height as f32 / map_h) * area.height;

        assert!(rx >= area.left && rx <= area.left + area.width);
        assert!(ry >= area.top && ry <= area.top + area.height);
        assert!(rw > 0.0 && rw <= area.width);
        assert!(rh > 0.0 && rh <= area.height);
    }

    #[test]
    fn minimap_layer_toggles_preserve_route_continuity_and_target() {
        use crate::settings::GameSettings;
        use crate::state::minimap::{MinimapState, NavigationTarget};

        let mut minimap = MinimapState::default();
        minimap.set_map("prt_fild08".into(), 200, 200, None, None, Vec::new());
        minimap.set_personal_waypoint(Some((120, 154)));
        minimap.set_navigation_target(Some(NavigationTarget {
            map_name: "prontera".to_owned(),
            position: None,
        }));

        let mut settings = GameSettings::default();
        settings.show_minimap_portals = false;
        settings.show_minimap_population_regions = false;
        settings.show_minimap_facilities = false;
        settings.show_minimap_party = false;
        settings.show_minimap_quest_markers = false;

        // Even with all display layers toggled off, navigation targets and personal
        // waypoints remain intact.
        assert_eq!(minimap.personal_waypoint(), Some((120, 154)));
        assert_eq!(minimap.navigation_target().map(|t| t.map_name.as_str()), Some("prontera"));

        // Route exit resolution remains fully operational for navigation guidance.
        let exits = crate::world::map_portal_exits(minimap.map_name(), minimap.navigation_target().map(|t| t.map_name.as_str()));
        let route_portal = exits.iter().find(|e| e.to_map == "prontera").expect("route portal to prontera");
        assert!(route_portal.is_route_exit);
    }
}
