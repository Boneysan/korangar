//! Tracked-objective HUD breadcrumb.

use korangar_interface::element::StateElement;
use rust_state::RustState;

#[derive(Clone, Debug, Default, PartialEq, Eq, RustState, StateElement)]
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
    pub scale: u8,
    pub opacity: u8,
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

        self.remaining = total_remaining;

        if let Some(guidance) = crate::world::bundled_guidance().get(&quest.quest_id) {
            self.destination = format!("{} ({})", guidance.npc, guidance.area);
        } else if !quest.location.is_empty() {
            self.destination = quest.location.clone();
        }

        if total_needed > 0 && total_remaining == 0 {
            self.objective_summary = "All items collected — turn in".to_string();
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
        if self.hidden || self.map != map {
            return None;
        }
        let (tx, ty) = (self.tile_x?, self.tile_y?);
        Some(x.abs_diff(tx).max(y.abs_diff(ty)))
    }

    #[allow(dead_code)]
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
            location: "Prontera West Field".into(),
        };

        let mut b = BreadcrumbState::default();
        b.update_from_quest(&quest, |id| if id == ItemId(940) { 7 } else { 4 });
        assert_eq!(b.quest_id, Some(20003));
        assert_eq!(b.quest_title, "Field Contract: Rockers and Rumors");
        assert_eq!(b.remaining, 9);
        assert!(b.objective_summary.contains("Grasshopper's Leg (7/10)"));
        assert!(b.destination.contains("Wynne"));

        b.update_from_quest(&quest, |_| 10);
        assert_eq!(b.remaining, 0);
        assert!(b.objective_summary.contains("All items collected"));

        b.clear();
        assert_eq!(b.quest_id, None);
        assert!(b.quest_title.is_empty());
    }
}
