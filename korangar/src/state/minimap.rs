use std::sync::Arc;

use korangar_interface::element::StateElement;
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

/// Client-side minimap data for the current map.
///
/// Official RO loads `data\texture\유저인터페이스\map\{map}.bmp` (path relative
/// to the texture root is `유저인터페이스\map\{map}.bmp`).
/// Facility icons come from `System/Towninfo*.lub` + `information\*.bmp`.
/// The local player uses `minimap\player_*.bmp` so the marker is drawn as a
/// texture (rectangles are rendered under all UI textures and would be covered
/// by the map bitmap).
#[derive(Default, RustState, StateElement)]
pub struct MinimapState {
    /// Base map name without extension (e.g. `izlude`).
    map_name: String,
    /// GAT dimensions in tiles.
    map_width: u16,
    map_height: u16,
    /// Minimap bitmap when available.
    #[hidden_element]
    texture: Option<Arc<Texture>>,
    /// Official player blip (`minimap\player_1.bmp`).
    #[hidden_element]
    player_marker: Option<Arc<Texture>>,
    /// Towninfo facility markers for the current map.
    #[hidden_element]
    pois: Vec<MinimapPoi>,
}

impl MinimapState {
    pub fn clear(&mut self) {
        self.map_name.clear();
        self.map_width = 0;
        self.map_height = 0;
        self.texture = None;
        self.player_marker = None;
        self.pois.clear();
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
}
