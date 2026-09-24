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

const MAX_CLIENT_HUNTING_GOALS: usize = 5;

/// One item a contract asks for, with its display name already resolved.
#[derive(Clone, Debug, RustState, StateElement)]
pub struct QuestRequirementEntry {
    pub item_id: ItemId,
    pub item_name: String,
    pub needed: u32,
}

/// A server-reported kill objective. Locations are resolved from verified
/// static monster spawn records only; no destination is guessed from text.
#[derive(Clone, Debug, RustState, StateElement)]
pub struct QuestHuntObjectiveEntry {
    pub monster_id: u32,
    pub monster_name: String,
    pub total_count: u16,
    pub current_count: u16,
}

/// A player-authored, local-only target selected from the Adventure Guide.
/// It deliberately has no progress counter: ordinary monster death packets
/// do not prove that the local player earned the kill.
#[derive(Clone, Debug, Eq, PartialEq, RustState, StateElement)]
pub struct ClientHuntingGoalEntry {
    pub monster_id: u32,
    pub monster_name: String,
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
    /// Explicit monster objectives from Hercules hunting-quest packets.
    pub hunt_objectives: Vec<QuestHuntObjectiveEntry>,
}

impl QuestEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn requirements(&self) -> &[QuestRequirementEntry] {
        &self.requirements
    }

    pub fn hunt_objectives(&self) -> &[QuestHuntObjectiveEntry] {
        &self.hunt_objectives
    }
}

/// Active quests, in the order the server listed them.
#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct QuestLogState {
    quests: Vec<QuestEntry>,
    tracked_quests: Vec<u32>,
    client_hunting_goals: Vec<ClientHuntingGoalEntry>,
    /// HUD tracker text. Rebuilt whenever the log changes.
    display_text: String,
}

impl QuestLogState {
    pub fn quests(&self) -> &[QuestEntry] {
        &self.quests
    }

    pub fn is_empty(&self) -> bool {
        self.quests.is_empty() && self.client_hunting_goals.is_empty()
    }

    pub fn is_tracked(&self, quest_id: u32) -> bool {
        self.tracked_quests.contains(&quest_id)
    }

    pub fn tracked_quest_ids(&self) -> &[u32] {
        &self.tracked_quests
    }

    pub fn client_hunting_goals(&self) -> &[ClientHuntingGoalEntry] {
        &self.client_hunting_goals
    }

    pub fn set_client_hunting_goals(&mut self, goals: Vec<ClientHuntingGoalEntry>) {
        self.client_hunting_goals = goals;
        self.client_hunting_goals.sort_by_key(|goal| goal.monster_id);
        self.client_hunting_goals.dedup_by_key(|goal| goal.monster_id);
        self.client_hunting_goals.truncate(MAX_CLIENT_HUNTING_GOALS);
        self.rebuild_display();
    }

    pub fn add_client_hunting_goal(&mut self, goal: ClientHuntingGoalEntry) -> bool {
        if self.client_hunting_goals.len() >= MAX_CLIENT_HUNTING_GOALS
            || self
                .client_hunting_goals
                .iter()
                .any(|existing| existing.monster_id == goal.monster_id)
        {
            return false;
        }
        self.client_hunting_goals.push(goal);
        self.client_hunting_goals.sort_by_key(|goal| goal.monster_id);
        self.rebuild_display();
        true
    }

    pub fn remove_client_hunting_goal(&mut self, monster_id: u32) -> bool {
        let old_len = self.client_hunting_goals.len();
        self.client_hunting_goals.retain(|goal| goal.monster_id != monster_id);
        let removed = old_len != self.client_hunting_goals.len();
        if removed {
            self.rebuild_display();
        }
        removed
    }

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    pub fn set_tracked_quests(&mut self, quest_ids: &[u32]) {
        self.tracked_quests = quest_ids
            .iter()
            .copied()
            .filter(|quest_id| self.quests.iter().any(|quest| quest.quest_id == *quest_id))
            .collect();
        self.tracked_quests.sort_unstable();
        self.tracked_quests.dedup();
        self.rebuild_display();
    }

    pub fn toggle_tracking(&mut self, quest_id: u32) {
        if self.tracked_quests.contains(&quest_id) {
            self.tracked_quests.retain(|id| *id != quest_id);
        } else if self.quests.iter().any(|quest| quest.quest_id == quest_id) {
            self.tracked_quests.push(quest_id);
        }
        self.rebuild_display();
    }

    /// Replace the whole log, as `ZC_ALL_QUEST_LIST` does on map login.
    pub fn replace(&mut self, mut quests: Vec<QuestEntry>) {
        let was_empty = self.quests.is_empty();
        for quest in &mut quests {
            if let Some(existing) = self.quests.iter().find(|entry| entry.quest_id == quest.quest_id) {
                quest.hunt_objectives.clone_from(&existing.hunt_objectives);
            }
        }
        self.quests = quests;
        self.tracked_quests
            .retain(|id| self.quests.iter().any(|quest| quest.quest_id == *id));
        if was_empty && self.tracked_quests.is_empty() {
            self.tracked_quests = self.quests.iter().map(|quest| quest.quest_id).collect();
        }
        self.rebuild_display();
    }

    /// Add a quest, or refresh one already listed.
    ///
    /// The server re-sends `ZC_ADD_QUEST` for a quest it has already told us
    /// about (activating a paused quest does it), so this must not duplicate.
    pub fn add(&mut self, quest: QuestEntry) {
        match self.quests.iter_mut().find(|entry| entry.quest_id == quest.quest_id) {
            Some(existing) => {
                let mut quest = quest;
                quest.hunt_objectives.clone_from(&existing.hunt_objectives);
                *existing = quest;
            }
            None => {
                self.tracked_quests.push(quest.quest_id);
                self.quests.push(quest);
            }
        }
        self.rebuild_display();
    }

    pub fn set_hunt_objectives(&mut self, quest_id: u32, objectives: Vec<QuestHuntObjectiveEntry>) {
        let Some(quest) = self.quests.iter_mut().find(|entry| entry.quest_id == quest_id) else {
            self.add(QuestEntry {
                quest_id,
                name: format!("Quest {quest_id}"),
                requirements: Vec::new(),
                hunt_objectives: objectives,
            });
            return;
        };
        quest.hunt_objectives = objectives;
        self.rebuild_display();
    }

    pub fn update_hunt_progress(&mut self, quest_id: u32, objective_index: u32, current_count: u16) {
        if let Some(objective) = self
            .quests
            .iter_mut()
            .find(|entry| entry.quest_id == quest_id)
            .and_then(|quest| quest.hunt_objectives.get_mut(objective_index as usize))
        {
            objective.current_count = current_count.min(objective.total_count);
            self.rebuild_display();
        }
    }

    pub fn remove(&mut self, quest_id: u32) {
        self.quests.retain(|entry| entry.quest_id != quest_id);
        self.tracked_quests.retain(|id| *id != quest_id);
        self.rebuild_display();
    }

    fn rebuild_display(&mut self) {
        let mut entries = self
            .quests
            .iter()
            .filter(|quest| self.tracked_quests.contains(&quest.quest_id))
            .map(|quest| {
                let mut text = if quest.requirements.is_empty() {
                    quest.name.clone()
                } else {
                    let items = quest
                        .requirements
                        .iter()
                        .map(|requirement| format!("need {} x{}", requirement.item_name, requirement.needed))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{} ({items})", quest.name)
                };
                for objective in &quest.hunt_objectives {
                    text.push_str(&format!(
                        "\n  {}: {} / {}",
                        objective.monster_name, objective.current_count, objective.total_count
                    ));
                }
                text
            })
            .collect::<Vec<_>>();
        if !self.client_hunting_goals.is_empty() {
            entries.push("Personal hunting goals (client-only):".to_owned());
            entries.extend(
                self.client_hunting_goals
                    .iter()
                    .map(|goal| format!("  {} — no server-tracked progress", goal.monster_name)),
            );
        }
        self.display_text = entries.join("\n");
    }

    /// Drop everything, for a logout or a character switch.
    pub fn clear(&mut self) {
        self.quests.clear();
        self.tracked_quests.clear();
        self.client_hunting_goals.clear();
        self.rebuild_display();
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::ItemId;

    use super::{ClientHuntingGoalEntry, QuestEntry, QuestHuntObjectiveEntry, QuestLogState, QuestRequirementEntry};

    fn entry(quest_id: u32, name: &str) -> QuestEntry {
        QuestEntry {
            quest_id,
            name: name.to_owned(),
            requirements: vec![QuestRequirementEntry {
                item_id: ItemId(1016),
                item_name: "Rat Tail".to_owned(),
                needed: 7,
            }],
            hunt_objectives: Vec::new(),
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
    fn hunting_objectives_survive_server_list_refresh_and_update_progress() {
        let mut log = QuestLogState::default();
        log.add(entry(20002, "Cellar Vermin"));
        log.set_hunt_objectives(20002, vec![QuestHuntObjectiveEntry {
            monster_id: 1002,
            monster_name: "Poring".to_owned(),
            total_count: 10,
            current_count: 3,
        }]);
        log.update_hunt_progress(20002, 0, 5);
        log.replace(vec![entry(20002, "Cellar Vermin"), entry(20003, "Other active quest")]);

        assert_eq!(log.quests()[0].hunt_objectives()[0].current_count, 5);
        assert!(log.display_text().contains("Poring: 5 / 10"));
        log.remove(20002);
        assert_eq!(log.quests().len(), 1);
    }

    #[test]
    fn client_hunting_goals_are_deduplicated_local_and_cleared_on_character_logout() {
        let mut log = QuestLogState::default();
        let poring = ClientHuntingGoalEntry {
            monster_id: 1002,
            monster_name: "Poring".to_owned(),
        };
        assert!(log.add_client_hunting_goal(poring.clone()));
        assert!(!log.add_client_hunting_goal(poring));
        assert!(log.display_text().contains("Personal hunting goals (client-only):"));
        assert!(log.display_text().contains("Poring — no server-tracked progress"));
        assert!(!log.is_empty());

        log.clear();
        assert!(log.client_hunting_goals().is_empty());
        assert!(log.is_empty());
    }

    #[test]
    fn personal_hunting_goal_count_is_bounded() {
        let mut log = QuestLogState::default();
        for monster_id in 1..=5 {
            assert!(log.add_client_hunting_goal(ClientHuntingGoalEntry {
                monster_id,
                monster_name: format!("Monster {monster_id}"),
            }));
        }
        assert!(!log.add_client_hunting_goal(ClientHuntingGoalEntry {
            monster_id: 6,
            monster_name: "Monster 6".to_owned(),
        }));
        assert_eq!(log.client_hunting_goals().len(), 5);
    }
}
