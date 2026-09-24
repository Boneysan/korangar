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

/// One ephemeral party location ping; it is deliberately map-local and never
/// saved or synchronized beyond the party chat transport.
#[derive(Clone, Debug)]
pub struct PartyPing {
    pub sender: String,
    pub kind: String,
    pub map_name: String,
    pub x: u16,
    pub y: u16,
    pub expires_at: ClientTick,
}

const PARTY_PING_TTL_MS: u32 = 15_000;

/// Decode the human-readable versioned chat fallback emitted by Korangar.
/// The server still owns party membership and relays the message; this is only
/// an ephemeral presentation hint, never an authority-bearing command.
pub fn parse_party_ping(text: &str) -> Option<PartyPing> {
    let (sender, body) = text.split_once(" : ")?;
    let (kind, payload) = if let Some(payload) = body.strip_prefix("[KORANGAR-PING:v1] ") {
        ("location", payload)
    } else if let Some(payload) = body.strip_prefix("[KORANGAR-PING:v2] ") {
        let (kind, payload) = payload.split_once(' ')?;
        if !matches!(kind, "location" | "assist" | "danger" | "retreat" | "ready" | "on-my-way") {
            return None;
        }
        (kind, payload)
    } else {
        return None;
    };
    let mut fields = payload.split_whitespace();
    let map_name = fields.next()?;
    let x = fields.next()?.parse::<u16>().ok()?;
    let y = fields.next()?.parse::<u16>().ok()?;
    if fields.next().is_some()
        || sender.is_empty()
        || sender.len() > 24
        || map_name.is_empty()
        || map_name.len() > 24
        || !map_name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || x == 0
        || y == 0
    {
        return None;
    }
    Some(PartyPing {
        sender: sender.to_owned(),
        kind: kind.to_owned(),
        map_name: map_name.to_owned(),
        x,
        y,
        expires_at: ClientTick(0),
    })
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
pub const MAX_MINIMAP_SIDE: f32 = 640.0;

#[derive(Clone, Debug, PartialEq, RustState)]
pub struct NavigationTarget {
    pub map_name: String,
    /// `None` represents a map-level destination with no exact cell exposed.
    pub position: Option<(u16, u16)>,
}

#[derive(RustState, StateElement)]
pub struct MinimapState {
    /// Base map name without extension (e.g. `izlude`).
    map_name: String,
    /// GAT dimensions in tiles.
    map_width: u16,
    map_height: u16,
    /// User-controlled square map size (window width tracks this;
    /// scroll/buttons zoom).
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
    /// Map-level navigation target and the next route exit to draw.
    #[hidden_element]
    navigation_target: Option<NavigationTarget>,
    #[hidden_element]
    navigation_marker: Option<(u16, u16)>,
    /// Sampled walkable tiles from the player to the next route waypoint.
    #[hidden_element]
    navigation_breadcrumbs: Vec<(u16, u16)>,
    /// Latest map-local party ping (15 s, cleared on map change).
    #[hidden_element]
    party_ping: Option<PartyPing>,
    #[hidden_element]
    party_ping_sent_at: Option<ClientTick>,
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
            navigation_target: None,
            navigation_marker: None,
            navigation_breadcrumbs: Vec::new(),
            party_ping: None,
            party_ping_sent_at: None,
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
        self.party_ping = None;
        self.navigation_marker = None;
        self.navigation_breadcrumbs.clear();
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
        self.party_ping = None;
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

    pub fn navigation_target(&self) -> Option<&NavigationTarget> {
        self.navigation_target.as_ref()
    }

    pub fn set_navigation_target(&mut self, target: Option<NavigationTarget>) {
        self.navigation_target = target;
    }

    pub fn set_navigation_marker(&mut self, marker: Option<(u16, u16)>) {
        self.navigation_marker = marker;
    }

    pub fn navigation_marker(&self) -> Option<(u16, u16)> {
        self.navigation_marker
    }

    pub fn set_navigation_breadcrumbs(&mut self, breadcrumbs: Vec<(u16, u16)>) {
        self.navigation_breadcrumbs = breadcrumbs;
    }

    pub fn navigation_breadcrumbs(&self) -> &[(u16, u16)] {
        &self.navigation_breadcrumbs
    }

    pub fn set_party_ping(&mut self, mut ping: PartyPing, now: ClientTick) -> bool {
        if !ping.map_name.eq_ignore_ascii_case(&self.map_name) || ping.x >= self.map_width || ping.y >= self.map_height {
            return false;
        }
        ping.expires_at = ClientTick(now.0.saturating_add(PARTY_PING_TTL_MS));
        self.party_ping = Some(ping);
        true
    }

    pub fn party_ping(&self) -> Option<&PartyPing> {
        self.party_ping.as_ref()
    }

    /// Bound client-generated ephemeral party-chat traffic to one update per
    /// second.
    pub fn allow_party_session_message_send(&mut self, now: ClientTick) -> bool {
        if self.party_ping_sent_at.is_some_and(|last| now.0.wrapping_sub(last.0) < 1_000) {
            return false;
        }
        self.party_ping_sent_at = Some(now);
        true
    }

    /// Apply a `MarkMinimapPosition` (0x0144) packet.
    pub fn apply_mark(&mut self, marker_type: MarkerType, position: (u32, u32), id: u8, color: ColorRGBA, now: ClientTick) {
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
        if self.party_ping.as_ref().is_some_and(|ping| ping.expires_at.0 <= now.0) {
            self.party_ping = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn party_ping_parser_is_bounded_and_versioned() {
        let ping = parse_party_ping("BigZ : [KORANGAR-PING:v1] prt_fild08 120 154").unwrap();
        assert_eq!(ping.sender, "BigZ");
        assert_eq!(ping.kind, "location");
        assert_eq!(ping.map_name, "prt_fild08");
        assert_eq!((ping.x, ping.y), (120, 154));
        assert!(parse_party_ping("BigZ : [KORANGAR-PING:v2] prt_fild08 120 154").is_none());
        assert!(parse_party_ping("BigZ : [KORANGAR-PING:v1] prt_fild08 -1 154").is_none());
        assert!(parse_party_ping("BigZ : [KORANGAR-PING:v1] prt_fild08 120 154 extra").is_none());
        assert!(parse_party_ping("BigZ : [KORANGAR-PING:v1] prt_fild08;@warp 120 154").is_none());
        assert_eq!(
            parse_party_ping("BigZ : [KORANGAR-PING:v2] danger prt_fild08 120 154")
                .unwrap()
                .kind,
            "danger"
        );
        assert!(parse_party_ping("BigZ : [KORANGAR-PING:v2] arbitrary prt_fild08 120 154").is_none());
    }

    #[test]
    fn party_ping_expires_and_is_cleared_on_map_change() {
        let mut state = MinimapState::default();
        state.set_map("prt_fild08".into(), 200, 200, None, None, Vec::new());
        assert!(state.set_party_ping(
            PartyPing {
                sender: "BigZ".to_owned(),
                kind: "location".to_owned(),
                map_name: "prt_fild08".to_owned(),
                x: 120,
                y: 154,
                expires_at: ClientTick(0),
            },
            ClientTick(1_000),
        ));
        assert!(state.party_ping().is_some());
        state.tick_markers(ClientTick(15_999));
        assert!(state.party_ping().is_some());
        state.tick_markers(ClientTick(16_000));
        assert!(state.party_ping().is_none());

        state.set_party_ping(
            PartyPing {
                sender: "BigZ".to_owned(),
                kind: "location".to_owned(),
                map_name: "prt_fild08".to_owned(),
                x: 120,
                y: 154,
                expires_at: ClientTick(0),
            },
            ClientTick(20_000),
        );
        state.set_map("izlude".into(), 200, 200, None, None, Vec::new());
        assert!(state.party_ping().is_none());
    }

    #[test]
    fn party_ping_outside_the_loaded_map_is_rejected() {
        let mut state = MinimapState::default();
        state.set_map("prt_fild08".into(), 200, 200, None, None, Vec::new());
        assert!(!state.set_party_ping(
            PartyPing {
                sender: "BigZ".to_owned(),
                kind: "danger".to_owned(),
                map_name: "prt_fild08".to_owned(),
                x: 20_000,
                y: 154,
                expires_at: ClientTick(0),
            },
            ClientTick(1_000),
        ));
        assert!(state.party_ping().is_none());
    }

    #[test]
    fn party_session_sender_has_a_one_second_cooldown() {
        let mut state = MinimapState::default();
        assert!(state.allow_party_session_message_send(ClientTick(10_000)));
        assert!(!state.allow_party_session_message_send(ClientTick(10_999)));
        assert!(state.allow_party_session_message_send(ClientTick(11_000)));
    }

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
