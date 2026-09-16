//! Tracked-objective HUD breadcrumb.

use korangar_interface::element::StateElement;
use rust_state::RustState;

#[derive(Clone, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub struct BreadcrumbState {
    pub quest_id: Option<u32>,
    pub remaining: u32,
    pub destination: String,
    pub map: String,
    pub tile_x: Option<u16>,
    pub tile_y: Option<u16>,
    pub collapsed: bool,
    pub hidden: bool,
    pub scale: u8,
    pub opacity: u8,
}

impl BreadcrumbState {
    pub fn from_objective(quest_id: u32, remaining: u32, dest: &str, map: &str, x: Option<u16>, y: Option<u16>) -> Self {
        Self {
            quest_id: Some(quest_id),
            remaining,
            destination: dest.to_owned(),
            map: map.to_owned(),
            tile_x: x,
            tile_y: y,
            collapsed: false,
            hidden: false,
            scale: 100,
            opacity: 100,
        }
    }

    pub fn distance(&self, map: &str, x: u16, y: u16) -> Option<u16> {
        if self.hidden || self.map != map {
            return None;
        }
        let (tx, ty) = (self.tile_x?, self.tile_y?);
        Some(x.abs_diff(tx).max(y.abs_diff(ty)))
    }

    pub fn captures_world_click(&self, over_control: bool) -> bool {
        !self.hidden && over_control
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_none_when_map_mismatch_or_hidden_or_complete() {
        let mut b = BreadcrumbState::from_objective(20003, 3, "Wynne", "prontera", Some(156), Some(191));
        assert_eq!(b.distance("prontera", 156, 191), Some(0));
        assert_eq!(b.distance("prt_fild07", 156, 191), None);
        b.hidden = true;
        assert_eq!(b.distance("prontera", 156, 191), None);
        b.hidden = false;
        b.tile_x = None;
        assert_eq!(b.distance("prontera", 10, 10), None);
        assert!(!b.captures_world_click(false));
        assert!(b.captures_world_click(true));
    }

    #[test]
    fn settings_round_trip() {
        let mut b = BreadcrumbState::default();
        b.collapsed = true;
        b.scale = 80;
        b.opacity = 70;
        let clone = b.clone();
        assert_eq!(clone.collapsed, true);
        assert_eq!(clone.scale, 80);
    }
}
