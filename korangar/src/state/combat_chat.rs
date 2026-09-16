//! Structured combat-log events from authoritative network facts, not parsed
//! chat strings.

use std::collections::VecDeque;

use korangar_interface::element::StateElement;
use korangar_networking::MessageColor;
use ragnarok_packets::{ClientTick, EntityId, ExperienceType, ItemId, SkillFailReason, SkillId};
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::state::status_effects::status_name;
use crate::state::{ChatHistory, ChatMessage};
use crate::world::{ItemName, ItemNameKey, Library, SkillListInformation};

/// Primary category for a combat event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CombatCategory {
    DamageDealt,
    DamageReceived,
    Healing,
    StatusGain,
    StatusLoss,
    SkillFailure,
    Exp,
    Loot,
}

/// Perspective of the local player relative to the combat action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatDirection {
    Outgoing,
    Incoming,
    None,
}

/// A structured combat entry containing raw fields. Names are formatted
/// only at the presentation boundary via [`CombatEntry::format`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatEntry {
    pub category: CombatCategory,
    pub source: Option<EntityId>,
    pub target: Option<EntityId>,
    pub source_name: Option<String>,
    pub target_name: Option<String>,
    pub skill_id: Option<SkillId>,
    pub item_id: Option<ItemId>,
    pub amount: i64,
    pub direction: CombatDirection,
    pub timestamp: ClientTick,
    pub status_index: Option<u16>,
    pub fail_reason: Option<SkillFailReason>,
    pub exp_type: Option<ExperienceType>,
    pub is_zeny: bool,
    pub count: u32,
}

impl CombatEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn from_damage(
        source: Option<EntityId>,
        target: Option<EntityId>,
        source_name: Option<String>,
        target_name: Option<String>,
        skill_id: Option<SkillId>,
        amount: usize,
        outgoing: bool,
        timestamp: ClientTick,
    ) -> Self {
        Self {
            category: if outgoing {
                CombatCategory::DamageDealt
            } else {
                CombatCategory::DamageReceived
            },
            source,
            target,
            source_name,
            target_name,
            skill_id,
            item_id: None,
            amount: amount as i64,
            direction: if outgoing {
                CombatDirection::Outgoing
            } else {
                CombatDirection::Incoming
            },
            timestamp,
            status_index: None,
            fail_reason: None,
            exp_type: None,
            is_zeny: false,
            count: 1,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_heal(
        source: Option<EntityId>,
        target: Option<EntityId>,
        source_name: Option<String>,
        target_name: Option<String>,
        skill_id: Option<SkillId>,
        amount: usize,
        outgoing: bool,
        timestamp: ClientTick,
    ) -> Self {
        Self {
            category: CombatCategory::Healing,
            source,
            target,
            source_name,
            target_name,
            skill_id,
            item_id: None,
            amount: amount as i64,
            direction: if outgoing {
                CombatDirection::Outgoing
            } else {
                CombatDirection::Incoming
            },
            timestamp,
            status_index: None,
            fail_reason: None,
            exp_type: None,
            is_zeny: false,
            count: 1,
        }
    }

    pub fn from_status(entity_id: Option<EntityId>, name: Option<String>, index: u16, gained: bool, timestamp: ClientTick) -> Self {
        Self {
            category: if gained {
                CombatCategory::StatusGain
            } else {
                CombatCategory::StatusLoss
            },
            source: None,
            target: entity_id,
            source_name: None,
            target_name: name,
            skill_id: None,
            item_id: None,
            amount: 0,
            direction: CombatDirection::Incoming,
            timestamp,
            status_index: Some(index),
            fail_reason: None,
            exp_type: None,
            is_zeny: false,
            count: 1,
        }
    }

    pub fn from_skill_fail(
        skill_id: SkillId,
        cause: u8,
        reason: Option<SkillFailReason>,
        item_id: Option<ItemId>,
        timestamp: ClientTick,
    ) -> Self {
        Self {
            category: CombatCategory::SkillFailure,
            source: None,
            target: None,
            source_name: None,
            target_name: None,
            skill_id: Some(skill_id),
            item_id,
            amount: cause as i64,
            direction: CombatDirection::Outgoing,
            timestamp,
            status_index: None,
            fail_reason: reason,
            exp_type: None,
            is_zeny: false,
            count: 1,
        }
    }

    pub fn from_exp(amount: u64, experience_type: ExperienceType, timestamp: ClientTick) -> Self {
        Self {
            category: CombatCategory::Exp,
            source: None,
            target: None,
            source_name: None,
            target_name: None,
            skill_id: None,
            item_id: None,
            amount: amount as i64,
            direction: CombatDirection::Incoming,
            timestamp,
            status_index: None,
            fail_reason: None,
            exp_type: Some(experience_type),
            is_zeny: false,
            count: 1,
        }
    }

    pub fn from_loot(item_id: Option<ItemId>, item_name: Option<String>, amount: u32, is_zeny: bool, timestamp: ClientTick) -> Self {
        Self {
            category: CombatCategory::Loot,
            source: None,
            target: None,
            source_name: None,
            target_name: item_name,
            skill_id: None,
            item_id,
            amount: amount as i64,
            direction: CombatDirection::Incoming,
            timestamp,
            status_index: None,
            fail_reason: None,
            exp_type: None,
            is_zeny,
            count: 1,
        }
    }

    /// Whether this entry can coalesce with another entry.
    /// Coalescing is applied to explicitly approved spam categories (repeated
    /// damage dealt or received).
    pub fn can_coalesce_with(&self, other: &Self) -> bool {
        matches!(self.category, CombatCategory::DamageDealt | CombatCategory::DamageReceived)
            && self.category == other.category
            && self.source == other.source
            && self.target == other.target
            && self.source_name == other.source_name
            && self.target_name == other.target_name
            && self.skill_id == other.skill_id
            && self.amount == other.amount
            && self.timestamp.0.saturating_sub(other.timestamp.0) <= 1000
    }

    /// Formats this entry into a presentation [`ChatMessage`].
    pub fn format(&self, library: &Library) -> ChatMessage {
        let count_str = if self.count > 1 {
            format!(" (x{})", self.count)
        } else {
            String::new()
        };

        match self.category {
            CombatCategory::DamageDealt => {
                let target = self.target_name.as_deref().unwrap_or("target");
                let skill = if let Some(skill_id) = self.skill_id {
                    library
                        .try_get::<SkillListInformation>(skill_id)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| format!("Skill #{}", skill_id.0))
                } else {
                    "Attack".to_string()
                };
                ChatMessage::new(
                    format!("[Dealt] {} damage to {target} ({skill}){count_str}", self.amount),
                    MessageColor::Rgb {
                        red: 255,
                        green: 220,
                        blue: 120,
                    },
                )
            }
            CombatCategory::DamageReceived => {
                let source = self.source_name.as_deref().unwrap_or("enemy");
                let skill = if let Some(skill_id) = self.skill_id {
                    library
                        .try_get::<SkillListInformation>(skill_id)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| format!("Skill #{}", skill_id.0))
                } else {
                    "Attack".to_string()
                };
                ChatMessage::new(
                    format!("[Taken] Took {} damage from {source} ({skill}){count_str}", self.amount),
                    MessageColor::Rgb {
                        red: 255,
                        green: 110,
                        blue: 110,
                    },
                )
            }
            CombatCategory::Healing => {
                let text = if self.direction == CombatDirection::Outgoing {
                    let target = self.target_name.as_deref().unwrap_or("target");
                    format!("[Heal] Healed {target} for {} HP{count_str}", self.amount)
                } else {
                    format!("[Heal] Healed for {} HP{count_str}", self.amount)
                };
                ChatMessage::new(text, MessageColor::Rgb {
                    red: 110,
                    green: 255,
                    blue: 110,
                })
            }
            CombatCategory::StatusGain => {
                let status = self.status_index.map(status_name).unwrap_or_else(|| "Status".to_string());
                let text = if self.direction == CombatDirection::Incoming || self.target_name.is_none() {
                    format!("[Status+] Gained {status}{count_str}")
                } else {
                    let target = self.target_name.as_deref().unwrap_or("target");
                    format!("[Status+] {target} gained {status}{count_str}")
                };
                ChatMessage::new(text, MessageColor::Rgb {
                    red: 130,
                    green: 210,
                    blue: 255,
                })
            }
            CombatCategory::StatusLoss => {
                let status = self.status_index.map(status_name).unwrap_or_else(|| "Status".to_string());
                let text = if self.direction == CombatDirection::Incoming || self.target_name.is_none() {
                    format!("[Status-] Lost {status}{count_str}")
                } else {
                    let target = self.target_name.as_deref().unwrap_or("target");
                    format!("[Status-] {target} lost {status}{count_str}")
                };
                ChatMessage::new(text, MessageColor::Rgb {
                    red: 170,
                    green: 180,
                    blue: 210,
                })
            }
            CombatCategory::SkillFailure => {
                let skill_name = if let Some(skill_id) = self.skill_id {
                    library
                        .try_get::<SkillListInformation>(skill_id)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| format!("Skill #{}", skill_id.0))
                } else {
                    "Skill".to_string()
                };
                let reason_text = if let Some(reason) = self.fail_reason {
                    skill_fail_reason_text(reason).to_string()
                } else if let Some(item_id) = self.item_id {
                    let item_name = library
                        .try_get::<ItemName>(ItemNameKey {
                            item_id,
                            is_identified: true,
                        })
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| format!("item #{}", item_id.0));
                    format!("Missing required item: {item_name}")
                } else if let Some(standard) = standard_skill_fail_cause_text(self.amount as u8) {
                    standard.to_string()
                } else {
                    format!("Refused (cause {})", self.amount)
                };
                ChatMessage::new(
                    format!("[Skill Fail] {skill_name}: {reason_text}{count_str}"),
                    MessageColor::Rgb {
                        red: 255,
                        green: 180,
                        blue: 80,
                    },
                )
            }
            CombatCategory::Exp => {
                let kind = match self.exp_type {
                    Some(ExperienceType::BaseExperience) => "Base",
                    Some(ExperienceType::JobExperience) => "Job",
                    None => "EXP",
                };
                ChatMessage::new(
                    format!("[EXP] Gained {} {kind} EXP{count_str}", self.amount),
                    MessageColor::Rgb {
                        red: 255,
                        green: 240,
                        blue: 130,
                    },
                )
            }
            CombatCategory::Loot => {
                let text = if self.is_zeny {
                    format!("[Loot] Obtained {} Zeny{count_str}", self.amount)
                } else if let Some(name) = &self.target_name {
                    format!("[Loot] Obtained {name} ({}){count_str}", self.amount)
                } else if let Some(item_id) = self.item_id {
                    let name = library
                        .try_get::<ItemName>(ItemNameKey {
                            item_id,
                            is_identified: true,
                        })
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| format!("Item #{}", item_id.0));
                    format!("[Loot] Obtained {name} ({}){count_str}", self.amount)
                } else {
                    format!("[Loot] Obtained item ({}){count_str}", self.amount)
                };
                ChatMessage::new(text, MessageColor::Rgb {
                    red: 255,
                    green: 215,
                    blue: 50,
                })
            }
        }
    }
}

fn skill_fail_reason_text(reason: SkillFailReason) -> &'static str {
    match reason {
        SkillFailReason::EnsemblePartner => "An ensemble skill requires an ensemble partner standing within range.",
        SkillFailReason::BenedictioHelpers => "That needs two Acolyte-class helpers standing to your left and right.",
        SkillFailReason::NoParty => "You have to be in a party to use that.",
        SkillFailReason::NoOneInRange => "No dead party member was in range.",
        SkillFailReason::NotEnoughExperience => "That spends 1% of your base and job experience, and you do not have it.",
        SkillFailReason::TargetResisted => "The target resisted.",
        SkillFailReason::NothingToSteal => "There was nothing to steal.",
        SkillFailReason::SuppressedByKyomu => "Kyomu suppressed the skill.",
        SkillFailReason::TargetImmune => "The target cannot be affected by that.",
        SkillFailReason::NeedsWarpPortal => "That has to be cast beside a warp portal.",
    }
}

fn standard_skill_fail_cause_text(cause: u8) -> Option<&'static str> {
    match cause {
        0 => Some("Skill failed."),
        1 => Some("Not enough SP."),
        2 => Some("Not enough HP."),
        3 => Some("Redundant attempt."),
        4 => Some("Cannot use that skill in this state."),
        71 => Some("Missing required item."),
        72 => Some("Missing required equipment."),
        _ => None,
    }
}

/// Category toggles for filtering which combat rows are created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub struct CombatFilters {
    pub damage_dealt: bool,
    pub damage_received: bool,
    pub healing: bool,
    pub status: bool,
    pub skill_failure: bool,
    pub exp: bool,
    pub loot: bool,
}

impl Default for CombatFilters {
    fn default() -> Self {
        Self {
            damage_dealt: true,
            damage_received: true,
            healing: true,
            status: true,
            skill_failure: true,
            exp: true,
            loot: true,
        }
    }
}

impl CombatFilters {
    pub fn is_enabled(&self, category: CombatCategory) -> bool {
        match category {
            CombatCategory::DamageDealt => self.damage_dealt,
            CombatCategory::DamageReceived => self.damage_received,
            CombatCategory::Healing => self.healing,
            CombatCategory::StatusGain | CombatCategory::StatusLoss => self.status,
            CombatCategory::SkillFailure => self.skill_failure,
            CombatCategory::Exp => self.exp,
            CombatCategory::Loot => self.loot,
        }
    }
}

/// State for the combat channel: bounded log, presentation messages,
/// unread count, and de-duplication metadata.
#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct CombatLogState {
    #[hidden_element]
    entries: VecDeque<CombatEntry>,
    messages: ChatHistory,
    unread_count: usize,
    is_selected: bool,
    #[hidden_element]
    last_heal: Option<(ClientTick, EntityId, usize)>,
}

impl CombatLogState {
    pub const CAPACITY: usize = 500;

    pub fn entries(&self) -> &VecDeque<CombatEntry> {
        &self.entries
    }

    pub fn messages(&self) -> &ChatHistory {
        &self.messages
    }

    pub fn unread_count(&self) -> usize {
        self.unread_count
    }

    pub fn is_selected(&self) -> bool {
        self.is_selected
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
        if selected {
            self.unread_count = 0;
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.messages = ChatHistory::default();
        self.unread_count = 0;
        self.last_heal = None;
    }

    /// Check and record heal de-duplication across packet boundaries.
    fn check_duplicate_heal(&mut self, tick: ClientTick, target: EntityId, amount: usize) -> bool {
        if let Some((last_tick, last_target, last_amt)) = self.last_heal
            && last_target == target
            && last_amt == amount
            && tick.0.abs_diff(last_tick.0) < 300
        {
            return true;
        }
        self.last_heal = Some((tick, target, amount));
        false
    }

    /// Records a combat entry if its category is enabled by `filters`.
    /// Performs coalescing for identical consecutive entries and de-duplicates
    /// redundant heal packets.
    pub fn record(&mut self, entry: CombatEntry, library: &Library, filters: &CombatFilters) {
        if !filters.is_enabled(entry.category) {
            return;
        }

        if entry.category == CombatCategory::Healing {
            let target = entry.target.unwrap_or(EntityId(0));
            if self.check_duplicate_heal(entry.timestamp, target, entry.amount as usize) {
                return;
            }
        }

        if let Some(last) = self.entries.back_mut()
            && entry.can_coalesce_with(last)
        {
            last.count += entry.count;
            last.timestamp = entry.timestamp;
            let updated_msg = last.format(library);
            if let Some(msg) = self.messages.last_mut() {
                *msg = updated_msg;
            }
            return;
        }

        if self.entries.len() >= Self::CAPACITY {
            self.entries.pop_front();
        }

        let chat_msg = entry.format(library);
        self.messages.push(chat_msg);
        self.entries.push_back(entry);

        if !self.is_selected {
            self.unread_count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_entry_per_category_and_direction() {
        let library = Library::empty_for_test();
        let filters = CombatFilters::default();
        let mut log = CombatLogState::default();

        // 1. Damage dealt (outbound)
        let dealt = CombatEntry::from_damage(
            Some(EntityId(1)),
            Some(EntityId(2)),
            Some("You".into()),
            Some("Poring".into()),
            Some(SkillId(19)), // Bash
            150,
            true,
            ClientTick(100),
        );
        log.record(dealt, &library, &filters);
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].category, CombatCategory::DamageDealt);
        assert!(log.messages[0].text.starts_with("[Dealt]"));

        // 2. Damage received (inbound)
        let taken = CombatEntry::from_damage(
            Some(EntityId(2)),
            Some(EntityId(1)),
            Some("Poring".into()),
            Some("You".into()),
            None,
            12,
            false,
            ClientTick(200),
        );
        log.record(taken, &library, &filters);
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.entries[1].category, CombatCategory::DamageReceived);
        assert!(log.messages[1].text.starts_with("[Taken]"));

        // 3. Healing (inbound)
        let heal = CombatEntry::from_heal(
            None,
            Some(EntityId(1)),
            None,
            Some("You".into()),
            None,
            45,
            false,
            ClientTick(300),
        );
        log.record(heal, &library, &filters);
        assert_eq!(log.entries.len(), 3);
        assert_eq!(log.entries[2].category, CombatCategory::Healing);
        assert!(log.messages[2].text.starts_with("[Heal]"));

        // 4. Healing (outbound)
        let heal_out = CombatEntry::from_heal(
            Some(EntityId(1)),
            Some(EntityId(3)),
            Some("You".into()),
            Some("Ally".into()),
            Some(SkillId(28)), // Heal
            120,
            true,
            ClientTick(400),
        );
        log.record(heal_out, &library, &filters);
        assert_eq!(log.entries.len(), 4);
        assert!(log.messages[3].text.contains("Healed Ally"));

        // 5. Status Gain
        let status_gain = CombatEntry::from_status(
            Some(EntityId(1)),
            Some("You".into()),
            6, // Poison
            true,
            ClientTick(500),
        );
        log.record(status_gain, &library, &filters);
        assert_eq!(log.entries.len(), 5);
        assert_eq!(log.entries[4].category, CombatCategory::StatusGain);
        assert!(log.messages[4].text.starts_with("[Status+]"));

        // 6. Status Loss
        let status_loss = CombatEntry::from_status(Some(EntityId(1)), Some("You".into()), 6, false, ClientTick(600));
        log.record(status_loss, &library, &filters);
        assert_eq!(log.entries.len(), 6);
        assert_eq!(log.entries[5].category, CombatCategory::StatusLoss);
        assert!(log.messages[5].text.starts_with("[Status-]"));

        // 7. Skill Failure
        let fail = CombatEntry::from_skill_fail(
            SkillId(19),
            1, // USESKILL_FAIL_SP_INSUFFICIENT
            None,
            None,
            ClientTick(700),
        );
        log.record(fail, &library, &filters);
        assert_eq!(log.entries.len(), 7);
        assert_eq!(log.entries[6].category, CombatCategory::SkillFailure);
        assert!(log.messages[6].text.starts_with("[Skill Fail]"));

        // 8. EXP Gain
        let exp = CombatEntry::from_exp(500, ExperienceType::BaseExperience, ClientTick(800));
        log.record(exp, &library, &filters);
        assert_eq!(log.entries.len(), 8);
        assert_eq!(log.entries[7].category, CombatCategory::Exp);
        assert!(log.messages[7].text.starts_with("[EXP]"));

        // 9. Item Loot
        let loot_item = CombatEntry::from_loot(Some(ItemId(501)), None, 2, false, ClientTick(900));
        log.record(loot_item, &library, &filters);
        assert_eq!(log.entries.len(), 9);
        assert_eq!(log.entries[8].category, CombatCategory::Loot);
        assert!(log.messages[8].text.starts_with("[Loot]"));

        // 10. Zeny Loot
        let loot_zeny = CombatEntry::from_loot(None, None, 1500, true, ClientTick(1000));
        log.record(loot_zeny, &library, &filters);
        assert_eq!(log.entries.len(), 10);
        assert!(log.messages[9].text.contains("1500 Zeny"));
    }

    #[test]
    fn duplicate_heal_is_deduplicated_across_packets() {
        let library = Library::empty_for_test();
        let filters = CombatFilters::default();
        let mut log = CombatLogState::default();

        let heal1 = CombatEntry::from_heal(
            None,
            Some(EntityId(10)),
            None,
            Some("You".into()),
            Some(SkillId(28)),
            250,
            false,
            ClientTick(1000),
        );
        log.record(heal1, &library, &filters);
        assert_eq!(log.entries.len(), 1);

        // A second packet describing the exact same heal 50ms later
        let heal2 = CombatEntry::from_heal(
            None,
            Some(EntityId(10)),
            None,
            Some("You".into()),
            None,
            250,
            false,
            ClientTick(1050),
        );
        log.record(heal2, &library, &filters);
        assert_eq!(log.entries.len(), 1, "duplicate heal must not create a second row");

        // A distinct heal later in time should be accepted
        let heal3 = CombatEntry::from_heal(
            None,
            Some(EntityId(10)),
            None,
            Some("You".into()),
            Some(SkillId(28)),
            250,
            false,
            ClientTick(2000),
        );
        log.record(heal3, &library, &filters);
        assert_eq!(log.entries.len(), 2);
    }

    #[test]
    fn disabled_category_allocates_no_visible_entry() {
        let library = Library::empty_for_test();
        let mut filters = CombatFilters::default();
        let mut log = CombatLogState::default();

        filters.loot = false;
        let loot = CombatEntry::from_loot(Some(ItemId(501)), None, 1, false, ClientTick(100));
        log.record(loot, &library, &filters);

        assert_eq!(log.entries.len(), 0);
        assert_eq!(log.messages.len(), 0);
        assert_eq!(log.unread_count, 0);

        filters.loot = true;
        let loot2 = CombatEntry::from_loot(Some(ItemId(501)), None, 1, false, ClientTick(200));
        log.record(loot2, &library, &filters);
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.messages.len(), 1);
    }

    #[test]
    fn unread_count_and_selection_lifecycle() {
        let library = Library::empty_for_test();
        let filters = CombatFilters::default();
        let mut log = CombatLogState::default();

        assert_eq!(log.unread_count(), 0);
        assert!(!log.is_selected());

        // While not selected, each entry increments unread
        for tick in [100, 3000, 6000] {
            let exp = CombatEntry::from_exp(100, ExperienceType::BaseExperience, ClientTick(tick));
            log.record(exp, &library, &filters);
        }
        assert_eq!(log.unread_count(), 3);

        // Selecting the channel resets unread count to 0
        log.set_selected(true);
        assert!(log.is_selected());
        assert_eq!(log.unread_count(), 0);

        // Entries arriving while selected do not increment unread
        let exp = CombatEntry::from_exp(100, ExperienceType::BaseExperience, ClientTick(9000));
        log.record(exp, &library, &filters);
        assert_eq!(log.unread_count(), 0);

        // Deselecting makes future entries increment unread again
        log.set_selected(false);
        let exp = CombatEntry::from_exp(100, ExperienceType::BaseExperience, ClientTick(12000));
        log.record(exp, &library, &filters);
        assert_eq!(log.unread_count(), 1);
    }

    #[test]
    fn bounded_eviction_and_clear() {
        let library = Library::empty_for_test();
        let filters = CombatFilters::default();
        let mut log = CombatLogState::default();

        for i in 0..600 {
            let entry = CombatEntry::from_exp(i as u64, ExperienceType::BaseExperience, ClientTick((i * 3000) as u32));
            log.record(entry, &library, &filters);
        }

        assert_eq!(log.entries().len(), CombatLogState::CAPACITY);
        assert_eq!(log.messages().len(), CombatLogState::CAPACITY);
        assert_eq!(log.entries().back().unwrap().amount, 599);

        log.clear();
        assert_eq!(log.entries().len(), 0);
        assert_eq!(log.messages().len(), 0);
        assert_eq!(log.unread_count(), 0);
    }

    #[test]
    fn coalescing_repeated_combat_entries() {
        let library = Library::empty_for_test();
        let filters = CombatFilters::default();
        let mut log = CombatLogState::default();

        // 3 consecutive hits of same amount within 500ms
        for tick in [100, 200, 300] {
            let entry = CombatEntry::from_damage(
                Some(EntityId(1)),
                Some(EntityId(2)),
                Some("You".into()),
                Some("Poring".into()),
                None,
                15,
                true,
                ClientTick(tick),
            );
            log.record(entry, &library, &filters);
        }

        assert_eq!(log.entries().len(), 1, "identical consecutive entries must coalesce");
        assert_eq!(log.entries()[0].count, 3);
        assert!(log.messages()[0].text.contains("(x3)"));
    }

    #[test]
    fn unknown_ids_degrade_to_stable_labels_without_panic() {
        let library = Library::empty_for_test();
        let filters = CombatFilters::default();
        let mut log = CombatLogState::default();

        let unknown_skill = CombatEntry::from_damage(
            Some(EntityId(1)),
            Some(EntityId(2)),
            None,
            None,
            Some(SkillId(9999)),
            10,
            true,
            ClientTick(100),
        );
        log.record(unknown_skill, &library, &filters);
        assert!(log.messages()[0].text.contains("Skill #9999"));

        let unknown_item = CombatEntry::from_loot(Some(ItemId(99999)), None, 1, false, ClientTick(200));
        log.record(unknown_item, &library, &filters);
        assert!(log.messages()[1].text.contains("Item #99999"));

        let unknown_status = CombatEntry::from_status(None, None, 9999, true, ClientTick(300));
        log.record(unknown_status, &library, &filters);
        assert!(log.messages()[2].text.contains("#9999"));
    }
}
