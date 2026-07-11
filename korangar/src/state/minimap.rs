use std::sync::Arc;

use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, ColorRGBA, MarkerType};
use rust_state::RustState;

use crate::graphics::Texture;
use crate::world::TownPoiKind;

/// A facility marker drawn on the minimap (from Towninfo).
#[derive(Clone)]
pub struct MinimapPoi {
    pub x: i16,
    pub y: i16,
    pub kind: TownPoiKind,
    /// Official display name (e.g. "Kafra Employee"); reserved for tooltips.
    #[allow(dead_code)]
    pub name: String,
    pub texture: Option<Arc<Texture>>,
}

/// Compass / NPC mark from `ZC_COMPASS` / `MarkMinimapPosition` (0x0144).
#[derive(Clone, Debug)]
pub struct DynamicMinimapMarker {
    pub id: u8,
    pub x: f32,
    pub y: f32,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
    /// When set, the marker is removed once `client_tick` reaches this value.
    pub expires_at: Option<ClientTick>,
}

/// Client-side minimap data for the current map.
///
/// Official RO loads `data\texture\유저인터페이스\map\{map}.bmp` (path relative
/// to the texture root is `유저인터페이스\map\{map}.bmp`).
/// Facility icons come from `System/Towninfo*.lub` + `information\*.bmp`.
/// The local player uses `minimap\player_*.bmp` so the marker is drawn as a
/// texture (rectangles are rendered under all UI textures and would be covered
/// by the map bitmap).
/// Default square map size (classic RO corner minimap).
pub const DEFAULT_MINIMAP_SIDE: f32 = 160.0;
pub const MIN_MINIMAP_SIDE: f32 = 96.0;
pub const MAX_MINIMAP_SIDE: f32 = 400.0;

#[derive(RustState, StateElement)]
pub struct MinimapState {
    /// Base map name without extension (e.g. `izlude`).
    map_name: String,
    /// GAT dimensions in tiles.
    map_width: u16,
    map_height: u16,
    /// User-controlled square map size (window width tracks this; scroll/buttons zoom).
    display_side: f32,
    /// Minimap bitmap when available.
    #[hidden_element]
    texture: Option<Arc<Texture>>,
    /// Official player blip (`minimap\player_1.bmp`).
    #[hidden_element]
    player_marker: Option<Arc<Texture>>,
    /// Towninfo facility markers for the current map.
    #[hidden_element]
    pois: Vec<MinimapPoi>,
    /// Compass / quest-style dynamic markers (0x0144).
    #[hidden_element]
    dynamic_markers: Vec<DynamicMinimapMarker>,
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            map_name: String::new(),
            map_width: 0,
            map_height: 0,
            display_side: DEFAULT_MINIMAP_SIDE,
            texture: None,
            player_marker: None,
            pois: Vec::new(),
            dynamic_markers: Vec::new(),
        }
    }
}

impl MinimapState {
    pub fn clear(&mut self) {
        self.map_name.clear();
        self.map_width = 0;
        self.map_height = 0;
        // Keep display_side — user zoom preference across map changes.
        self.texture = None;
        self.player_marker = None;
        self.pois.clear();
        self.dynamic_markers.clear();
    }

    pub fn set_map(
        &mut self,
        map_name: String,
        map_width: u16,
        map_height: u16,
        texture: Option<Arc<Texture>>,
        player_marker: Option<Arc<Texture>>,
        pois: Vec<MinimapPoi>,
    ) {
        self.map_name = map_name;
        self.map_width = map_width;
        self.map_height = map_height;
        self.texture = texture;
        self.player_marker = player_marker;
        self.pois = pois;
        // Compass marks are map-local.
        self.dynamic_markers.clear();
    }

    pub fn display_side(&self) -> f32 {
        self.display_side.clamp(MIN_MINIMAP_SIDE, MAX_MINIMAP_SIDE)
    }

    pub fn set_display_side(&mut self, side: f32) {
        self.display_side = side.clamp(MIN_MINIMAP_SIDE, MAX_MINIMAP_SIDE);
    }

    /// Zoom by a multiplicative factor (e.g. 1.1 / 0.9) or additive pixels.
    pub fn zoom_by(&mut self, delta_pixels: f32) {
        self.set_display_side(self.display_side + delta_pixels);
    }

    pub fn map_name(&self) -> &str {
        &self.map_name
    }

    pub fn map_width(&self) -> u16 {
        self.map_width
    }

    pub fn map_height(&self) -> u16 {
        self.map_height
    }

    pub fn texture(&self) -> Option<&Arc<Texture>> {
        self.texture.as_ref()
    }

    pub fn player_marker(&self) -> Option<&Arc<Texture>> {
        self.player_marker.as_ref()
    }

    pub fn pois(&self) -> &[MinimapPoi] {
        &self.pois
    }

    pub fn dynamic_markers(&self) -> &[DynamicMinimapMarker] {
        &self.dynamic_markers
    }

    /// Apply a `MarkMinimapPosition` (0x0144) packet.
    pub fn apply_mark(
        &mut self,
        marker_type: MarkerType,
        position: (u32, u32),
        id: u8,
        color: ColorRGBA,
        now: ClientTick,
    ) {
        match marker_type {
            MarkerType::RemoveMark => {
                self.dynamic_markers.retain(|m| m.id != id);
            }
            MarkerType::DisplayFor15Seconds => {
                self.upsert_marker(DynamicMinimapMarker {
                    id,
                    x: position.0 as f32,
                    y: position.1 as f32,
                    red: color.red,
                    green: color.green,
                    blue: color.blue,
                    alpha: color.alpha,
                    expires_at: Some(ClientTick(now.0.saturating_add(15_000))),
                });
            }
            MarkerType::DisplayUntilLeave => {
                self.upsert_marker(DynamicMinimapMarker {
                    id,
                    x: position.0 as f32,
                    y: position.1 as f32,
                    red: color.red,
                    green: color.green,
                    blue: color.blue,
                    alpha: color.alpha,
                    expires_at: None,
                });
            }
        }
    }

    fn upsert_marker(&mut self, marker: DynamicMinimapMarker) {
        if let Some(existing) = self.dynamic_markers.iter_mut().find(|m| m.id == marker.id) {
            *existing = marker;
        } else {
            self.dynamic_markers.push(marker);
        }
    }

    /// Drop timed compass markers that have expired.
    pub fn tick_markers(&mut self, now: ClientTick) {
        self.dynamic_markers
            .retain(|m| m.expires_at.map(|until| until.0 > now.0).unwrap_or(true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_mark_expires() {
        let mut state = MinimapState::default();
        state.apply_mark(
            MarkerType::DisplayFor15Seconds,
            (10, 20),
            1,
            ColorRGBA {
                red: 255,
                green: 0,
                blue: 0,
                alpha: 255,
            },
            ClientTick(0),
        );
        assert_eq!(state.dynamic_markers().len(), 1);
        state.tick_markers(ClientTick(14_999));
        assert_eq!(state.dynamic_markers().len(), 1);
        state.tick_markers(ClientTick(15_000));
        assert!(state.dynamic_markers().is_empty());
    }

    #[test]
    fn remove_mark_by_id() {
        let mut state = MinimapState::default();
        state.apply_mark(
            MarkerType::DisplayUntilLeave,
            (5, 5),
            3,
            ColorRGBA {
                red: 0,
                green: 255,
                blue: 0,
                alpha: 255,
            },
            ClientTick(0),
        );
        state.apply_mark(
            MarkerType::RemoveMark,
            (0, 0),
            3,
            ColorRGBA {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            },
            ClientTick(0),
        );
        assert!(state.dynamic_markers().is_empty());
    }

    #[test]
    fn set_map_clears_dynamic_markers() {
        let mut state = MinimapState::default();
        state.apply_mark(
            MarkerType::DisplayUntilLeave,
            (1, 1),
            1,
            ColorRGBA {
                red: 1,
                green: 1,
                blue: 1,
                alpha: 255,
            },
            ClientTick(0),
        );
        state.set_map("izlude".into(), 100, 100, None, None, Vec::new());
        assert!(state.dynamic_markers().is_empty());
        assert_eq!(state.map_name(), "izlude");
    }

    #[test]
    fn zoom_clamps_display_side() {
        let mut state = MinimapState::default();
        state.zoom_by(10_000.0);
        assert!((state.display_side() - MAX_MINIMAP_SIDE).abs() < 0.01);
        state.zoom_by(-10_000.0);
        assert!((state.display_side() - MIN_MINIMAP_SIDE).abs() < 0.01);
    }
}
