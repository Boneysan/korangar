use std::cell::UnsafeCell;

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::event::{EventQueue, ScrollHandler};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::State;

use super::WindowClass;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
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

            let mut extra_blips = Vec::new();

            // Party members on the same map with a known tile position.
            let party_path = client_state().party_state();
            let party = state.get(&party_path);
            for member in party.members() {
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

            // Compass / NPC marks (0x0144).
            for mark in minimap.dynamic_markers() {
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

            MinimapViewLayout {
                area,
                player_tile,
                extra_blips,
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
        if area.check().run(layout) {
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
        let poi_size = (side / DEFAULT_MINIMAP_SIDE * POI_ICON_SIZE).clamp(8.0, 20.0);
        let player_size = (side / DEFAULT_MINIMAP_SIDE * PLAYER_MARKER_SIZE).clamp(10.0, 22.0);

        // Towninfo facility POIs (shops, kafra, guides, …).
        // Must be textures — rectangles flush under the map bitmap and disappear.
        for poi in minimap.pois() {
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
        const CHROME_H: f32 = 56.0;

        window! {
            title: "Map",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            // Drag the right edge or bottom-right corner, or use − / + / scroll.
            minimum_width: MIN_MINIMAP_SIDE + CHROME_W,
            maximum_width: MAX_MINIMAP_SIDE + CHROME_W,
            minimum_height: MIN_MINIMAP_SIDE + COORDS_HEIGHT + ZOOM_ROW_HEIGHT + CHROME_H,
            maximum_height: MAX_MINIMAP_SIDE + COORDS_HEIGHT + ZOOM_ROW_HEIGHT + CHROME_H,
            elements: (
                MinimapView {
                    hover_tip: UnsafeCell::new(String::new()),
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
            ),
        }
    }
}
