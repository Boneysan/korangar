//! Tracked-objective HUD breadcrumb.

use korangar_interface::element::StateElement;
use rust_state::RustState;

#[derive(Clone, Debug, PartialEq, Eq, RustState, StateElement)]
pub struct BreadcrumbState {
    pub quest_id: Option<u32>,
    pub quest_title: String,
    pub objective_summary: String,
    pub remaining: u32,
    pub destination: String,
    pub map: String,
    pub tile_x: Option<u16>,
    pub tile_y: Option<u16>,
    pub collapsed: bool,
    pub hidden: bool,
    pub guidance_enabled: bool,
    pub scale: u8,
    pub opacity: u8,
}

impl Default for BreadcrumbState {
    fn default() -> Self {
        Self {
            quest_id: None,
            quest_title: String::new(),
            objective_summary: String::new(),
            remaining: 0,
            destination: String::new(),
            map: String::new(),
            tile_x: None,
            tile_y: None,
            collapsed: false,
            hidden: false,
            guidance_enabled: true,
            scale: 100,
            opacity: 100,
        }
    }
}

impl BreadcrumbState {
    #[allow(dead_code)]
    pub fn from_objective(quest_id: u32, remaining: u32, dest: &str, map: &str, x: Option<u16>, y: Option<u16>) -> Self {
        Self {
            quest_id: Some(quest_id),
            quest_title: String::new(),
            objective_summary: String::new(),
            remaining,
            destination: dest.to_owned(),
            map: map.to_owned(),
            tile_x: x,
            tile_y: y,
            collapsed: false,
            hidden: false,
            guidance_enabled: true,
            scale: 100,
            opacity: 100,
        }
    }

    pub fn update_from_quest(&mut self, quest: &crate::state::quests::QuestEntry, count_item: impl Fn(ragnarok_packets::ItemId) -> u32) {
        self.quest_id = Some(quest.quest_id);
        self.quest_title = quest.name.clone();

        let mut total_remaining = 0;
        let mut total_needed = 0;
        let mut first_incomplete_desc = String::new();

        for req in quest.requirements() {
            let have = count_item(req.item_id);
            let rem = req.needed.saturating_sub(have);
            total_needed += req.needed;
            total_remaining += rem;
            if rem > 0 && first_incomplete_desc.is_empty() {
                first_incomplete_desc = format!("{} ({}/{})", req.item_name, have, req.needed);
            }
        }

        for objective in quest.kill_objectives() {
            let remaining = u32::from(objective.total.saturating_sub(objective.current));
            total_remaining += remaining;
            if remaining > 0 && first_incomplete_desc.is_empty() {
                let name = if objective.mob_id != 0 {
                    crate::world::display_monster(objective.mob_id, "normal").to_string()
                } else {
                    format!("objective {}", objective.objective_id)
                };
                first_incomplete_desc = format!("Defeat {name} ({}/{})", objective.current, objective.total);
            }
        }

        self.remaining = total_remaining;
        self.map = quest.destination_map.clone();
        self.tile_x = quest.destination_x;
        self.tile_y = quest.destination_y;

        if let Some(guidance) = crate::world::bundled_guidance().get(&quest.quest_id) {
            self.destination = format!("{} ({})", guidance.npc, guidance.area);
        } else if !quest.location.is_empty() {
            self.destination = quest.location.clone();
        }

        if (total_needed > 0 || !quest.kill_objectives().is_empty()) && total_remaining == 0 {
            self.objective_summary = if quest.kill_objectives().is_empty() {
                "All items collected — turn in".to_string()
            } else {
                "Objectives complete — resolve next step".to_string()
            };
        } else if !first_incomplete_desc.is_empty() {
            self.objective_summary = first_incomplete_desc;
        } else if !quest.location.is_empty() {
            self.objective_summary = quest.location.clone();
        } else {
            self.objective_summary = "Follow instructions".to_string();
        }
    }

    pub fn clear(&mut self) {
        self.quest_id = None;
        self.quest_title.clear();
        self.objective_summary.clear();
        self.remaining = 0;
        self.destination.clear();
        self.map.clear();
        self.tile_x = None;
        self.tile_y = None;
    }

    #[allow(dead_code)]
    pub fn distance(&self, map: &str, x: u16, y: u16) -> Option<u16> {
        if !self.is_visible_on(map) {
            return None;
        }
        let (tx, ty) = (self.tile_x?, self.tile_y?);
        Some(x.abs_diff(tx).max(y.abs_diff(ty)))
    }

    /// Returns the arrow from the player to the revealed target.
    pub fn direction(&self, map: &str, x: u16, y: u16) -> Option<&'static str> {
        if !self.is_visible_on(map) {
            return None;
        }
        let (tx, ty) = (self.tile_x?, self.tile_y?);
        let dx = tx.cmp(&x);
        let dy = ty.cmp(&y);
        Some(match (dx, dy) {
            (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => "•",
            (std::cmp::Ordering::Greater, std::cmp::Ordering::Equal) => "→",
            (std::cmp::Ordering::Less, std::cmp::Ordering::Equal) => "←",
            (std::cmp::Ordering::Equal, std::cmp::Ordering::Greater) => "↑",
            (std::cmp::Ordering::Equal, std::cmp::Ordering::Less) => "↓",
            (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => "↗",
            (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => "↖",
            (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => "↘",
            (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => "↙",
        })
    }

    pub fn is_visible_on(&self, map: &str) -> bool {
        self.quest_id.is_some()
            && self.remaining > 0
            && !self.hidden
            && self.guidance_enabled
            && self.tile_x.is_some()
            && self.tile_y.is_some()
            && normalize_map(&self.map) == normalize_map(map)
    }

    pub fn target(&self, map: &str) -> Option<(u16, u16)> {
        if !self.is_visible_on(map) {
            return None;
        }
        Some((self.tile_x?, self.tile_y?))
    }

    pub fn set_scale(&mut self, scale: u8) {
        self.scale = scale.clamp(75, 150);
    }

    pub fn set_opacity(&mut self, opacity: u8) {
        self.opacity = opacity.clamp(40, 100);
    }

    #[allow(dead_code)]
    pub fn captures_world_click(&self, over_control: bool) -> bool {
        !self.hidden && over_control
    }
}

fn normalize_map(map: &str) -> &str {
    map.trim().trim_end_matches(".gat").trim_end_matches(".rsw")
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
        b.guidance_enabled = false;
        let clone = b.clone();
        assert_eq!(clone.collapsed, true);
        assert_eq!(clone.scale, 80);
        assert!(!clone.guidance_enabled);
    }

    #[test]
    fn update_from_quest_populates_title_and_summary() {
        use ragnarok_packets::ItemId;

        use crate::state::quests::{QuestEntry, QuestRequirementEntry};

        let quest = QuestEntry {
            quest_id: 20003,
            name: "Field Contract: Rockers and Rumors".into(),
            requirements: vec![
                QuestRequirementEntry {
                    item_id: ItemId(940),
                    item_name: "Grasshopper's Leg".into(),
                    needed: 10,
                },
                QuestRequirementEntry {
                    item_id: ItemId(919),
                    item_name: "Animal Skin".into(),
                    needed: 10,
                },
            ],
            kill_objectives: Vec::new(),
            location: "Prontera West Field".into(),
            destination_map: "prontera".into(),
            destination_x: Some(156),
            destination_y: Some(191),
        };

        let mut b = BreadcrumbState::default();
        b.update_from_quest(&quest, |id| if id == ItemId(940) { 7 } else { 4 });
        assert_eq!(b.quest_id, Some(20003));
        assert_eq!(b.quest_title, "Field Contract: Rockers and Rumors");
        assert_eq!(b.remaining, 9);
        assert!(b.objective_summary.contains("Grasshopper's Leg (7/10)"));
        assert!(b.destination.contains("Wynne"));
        assert_eq!(b.map, "prontera");
        assert_eq!(b.distance("prontera.gat", 156, 191), Some(0));
        assert_eq!(b.direction("prontera", 150, 190), Some("↗"));

        b.update_from_quest(&quest, |_| 10);
        assert_eq!(b.remaining, 0);
        assert!(b.objective_summary.contains("All items collected"));

        b.clear();
        assert_eq!(b.quest_id, None);
        assert!(b.quest_title.is_empty());
    }

    #[test]
    fn guidance_is_hidden_for_missing_coordinate_map_mismatch_completion_and_hidden_target() {
        let mut b = BreadcrumbState::from_objective(20003, 1, "Wynne", "prontera", Some(156), Some(191));
        assert_eq!(b.distance("prontera", 150, 190), Some(6));
        assert_eq!(b.direction("prontera", 150, 190), Some("↗"));
        b.tile_x = None;
        assert_eq!(b.distance("prontera", 150, 190), None);
        b.tile_x = Some(156);
        assert_eq!(b.distance("prt_fild07", 150, 190), None);
        b.remaining = 0;
        assert_eq!(b.direction("prontera", 150, 190), None);
        b.remaining = 1;
        b.hidden = true;
        assert_eq!(b.target("prontera"), None);
    }
}
