//! Quest log state.
//!
//! Hercules tells the client which quests are active (`ZC_ADD_QUEST`,
//! `ZC_DEL_QUEST`, `ZC_ALL_QUEST_LIST`) but nothing about what they want:
//! the packets carry kill objectives only, and the Seal Cascade hunting
//! contracts have none — they are filled by handing in items. The item list
//! comes from the bundled campaign table instead, resolved to names at the
//! boundary because the interface layer holds no `Library`.
//!
//! Progress is deliberately *not* stored. How many of an item the player is
//! carrying is already in the inventory, and caching it here would be a second
//! copy to keep in sync on every pickup, drop, trade and vend.

use korangar_interface::element::StateElement;
use ragnarok_packets::ItemId;
use rust_state::RustState;

/// One item a contract asks for, with its display name already resolved.
#[derive(Clone, Debug, RustState, StateElement)]
pub struct QuestRequirementEntry {
    pub item_id: ItemId,
    pub item_name: String,
    pub needed: u32,
}

/// A quest in the log.
#[derive(Clone, Debug, RustState, StateElement)]
pub struct QuestEntry {
    pub quest_id: u32,
    /// The contract's name, or a placeholder for a quest the campaign table
    /// does not describe (a story quest, or any non-campaign quest).
    pub name: String,
    /// Empty for a quest with no item turn-in.
    pub requirements: Vec<QuestRequirementEntry>,
}

impl QuestEntry {
    /// Inventory readiness is not server-confirmed quest completion. Quests
    /// without known item objectives must never appear ready by vacuous truth.
    pub fn items_ready(&self, count: impl Fn(ItemId) -> u32) -> bool {
        !self.requirements.is_empty() && self.requirements.iter().all(|item| count(item.item_id) >= item.needed)
    }

    pub fn matches_search(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        self.name.to_lowercase().contains(&query)
            || self.quest_id.to_string().contains(&query)
            || self.requirements.iter().any(|item| item.item_name.to_lowercase().contains(&query))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn requirements(&self) -> &[QuestRequirementEntry] {
        &self.requirements
    }
}

/// Active quests, in the order the server listed them.
#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct QuestLogState {
    quests: Vec<QuestEntry>,
    /// Journal preferences survive closing the window and map-list refreshes,
    /// but are cleared on character switch. Pins order this journal, not a HUD.
    pub search: String,
    pub ready_only: bool,
    pinned: Vec<u32>,
}

impl QuestLogState {
    pub fn quests(&self) -> &[QuestEntry] {
        &self.quests
    }

    pub fn is_empty(&self) -> bool {
        self.quests.is_empty()
    }

    pub fn is_pinned(&self, quest_id: u32) -> bool {
        self.pinned.contains(&quest_id)
    }

    pub fn toggle_pin(&mut self, quest_id: u32) {
        if self.is_pinned(quest_id) {
            self.pinned.retain(|id| *id != quest_id);
        } else if self.quests.iter().any(|quest| quest.quest_id == quest_id) {
            self.pinned.push(quest_id);
        }
    }

    /// Replace the whole log, as `ZC_ALL_QUEST_LIST` does on map login.
    pub fn replace(&mut self, quests: Vec<QuestEntry>) {
        self.pinned.retain(|id| quests.iter().any(|quest| quest.quest_id == *id));
        self.quests = quests;
    }

    /// Add a quest, or refresh one already listed.
    ///
    /// The server re-sends `ZC_ADD_QUEST` for a quest it has already told us
    /// about (activating a paused quest does it), so this must not duplicate.
    pub fn add(&mut self, quest: QuestEntry) {
        match self.quests.iter_mut().find(|entry| entry.quest_id == quest.quest_id) {
            Some(existing) => *existing = quest,
            None => self.quests.push(quest),
        }
    }

    pub fn remove(&mut self, quest_id: u32) {
        self.quests.retain(|entry| entry.quest_id != quest_id);
        self.pinned.retain(|id| *id != quest_id);
    }

    /// Drop everything, for a logout or a character switch.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::ItemId;

    use super::{QuestEntry, QuestLogState, QuestRequirementEntry};

    fn entry(quest_id: u32, name: &str) -> QuestEntry {
        QuestEntry {
            quest_id,
            name: name.to_owned(),
            requirements: vec![QuestRequirementEntry {
                item_id: ItemId(1016),
                item_name: "Rat Tail".to_owned(),
                needed: 7,
            }],
        }
    }

    #[test]
    fn adding_the_same_quest_twice_refreshes_rather_than_duplicates() {
        let mut log = QuestLogState::default();
        log.add(entry(20002, "Contract: Cellar Vermin"));
        log.add(entry(20002, "Contract: Cellar Vermin"));

        assert_eq!(log.quests().len(), 1);
    }

    #[test]
    fn removing_a_quest_leaves_the_others() {
        let mut log = QuestLogState::default();
        log.add(entry(20002, "Contract: Cellar Vermin"));
        log.add(entry(20003, "Field Contract"));
        log.remove(20002);

        assert_eq!(log.quests().len(), 1);
        assert_eq!(log.quests()[0].quest_id, 20003);
    }

    /// A full list replaces rather than merges: it is the server's statement
    /// of what is active, so a quest missing from it is finished.
    #[test]
    fn a_full_list_replaces_the_log() {
        let mut log = QuestLogState::default();
        log.add(entry(20002, "Contract: Cellar Vermin"));
        log.replace(vec![entry(20008, "Mushroom Ring Patrol")]);

        assert_eq!(log.quests().len(), 1);
        assert_eq!(log.quests()[0].quest_id, 20008);
    }

    #[test]
    fn readiness_tracks_inventory_in_both_directions() {
        let quest = entry(20002, "Contract");
        assert!(!quest.items_ready(|_| 6));
        assert!(quest.items_ready(|_| 7));
        assert!(quest.items_ready(|_| 100));
        assert!(!quest.items_ready(|_| 0));
        let mut story = quest;
        story.requirements.clear();
        assert!(!story.items_ready(|_| 100));
    }

    #[test]
    fn readiness_requires_every_objective() {
        let mut quest = entry(20002, "Contract");
        quest.requirements.push(QuestRequirementEntry {
            item_id: ItemId(1052),
            item_name: "Single Cell".into(),
            needed: 7,
        });
        assert!(!quest.items_ready(|id| if id == ItemId(1016) { 100 } else { 6 }));
        assert!(quest.items_ready(|_| 7));
    }

    #[test]
    fn search_matches_names_items_and_ids_without_case_or_outer_spaces() {
        let quest = entry(20002, "Contract: Cellar Vermin");
        for query in ["", "  CELLAR  ", "rat tail", "20002"] {
            assert!(quest.matches_search(query));
        }
        assert!(!quest.matches_search("Mushroom"));
    }

    #[test]
    fn pins_follow_quest_ids_across_refresh_and_are_removed_with_quests() {
        let mut log = QuestLogState::default();
        log.add(entry(20002, "First"));
        log.add(entry(20003, "Second"));
        log.toggle_pin(20002);
        log.toggle_pin(99999);
        assert!(!log.is_pinned(99999));
        log.search = "First".into();
        log.replace(vec![entry(20003, "Second"), entry(20002, "First")]);
        assert!(log.is_pinned(20002));
        assert_eq!(log.search, "First");
        log.remove(20002);
        assert!(!log.is_pinned(20002));
        log.toggle_pin(20003);
        log.replace(vec![]);
        assert!(!log.is_pinned(20003));
    }

    #[test]
    fn unpin_and_character_switch_reset_journal_preferences() {
        let mut log = QuestLogState::default();
        log.add(entry(20002, "First"));
        log.toggle_pin(20002);
        log.toggle_pin(20002);
        assert!(!log.is_pinned(20002));
        log.toggle_pin(20002);
        log.search = "Rat".into();
        log.ready_only = true;
        log.clear();
        assert!(log.is_empty());
        assert!(!log.is_pinned(20002));
        assert!(log.search.is_empty());
        assert!(!log.ready_only);
    }
}
