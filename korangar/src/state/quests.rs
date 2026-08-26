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
}

impl QuestLogState {
    pub fn quests(&self) -> &[QuestEntry] {
        &self.quests
    }

    pub fn is_empty(&self) -> bool {
        self.quests.is_empty()
    }

    /// Replace the whole log, as `ZC_ALL_QUEST_LIST` does on map login.
    pub fn replace(&mut self, quests: Vec<QuestEntry>) {
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
    }

    /// Drop everything, for a logout or a character switch.
    pub fn clear(&mut self) {
        self.quests.clear();
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
}
