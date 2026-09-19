//! Seal Cascade DM campaign systems.
//!
//! Kept isolated (with `interface/windows/dm/`) so the fork stays rebaseable
//! against upstream Korangar — see `CLAUDE.md` and `docs/DM_DATA_GUIDE.md`.

pub mod data;
pub mod loot;
pub mod parser;

pub use data::{BestiaryMonster, dm_data};
use korangar_interface::element::StateElement;
pub use loot::{LootDifficulty, generate_loot};
pub use parser::DmjMessage;
use rust_state::RustState;

/// One server-authoritative campaign flag value.
#[derive(Clone, Debug, PartialEq, Eq, RustState, StateElement)]
pub struct CampaignFlag {
    pub name: String,
    pub value: u32,
}

/// One server-authoritative typed objective update.
#[derive(Clone, Debug, PartialEq, Eq, RustState, StateElement)]
pub struct DmObjectiveProgress {
    pub quest_id: u32,
    pub objective_id: u32,
    pub kind: String,
    pub current: u16,
    pub total: u16,
    pub required: bool,
    pub completed: bool,
    pub party_shared: bool,
    pub dm_triggered: bool,
}

/// Server-authoritative preview/result for reconnect reconciliation.
#[derive(Clone, Debug, PartialEq, Eq, RustState, StateElement)]
pub struct ReconciliationState {
    pub party_id: u32,
    pub arc: u16,
    pub step: u32,
    pub mode: String,
    pub eligible: u32,
    pub ahead: u32,
    pub offline: u32,
    pub unavailable: u32,
    pub changed: u32,
}

/// Campaign-persistent DM state (session-scoped for now).
#[derive(Default, RustState, StateElement)]
pub struct DmCampaignState {
    /// Mob IDs the party has defeated — unlocks bestiary journal entries.
    pub bestiary_unlocked: Vec<u32>,
    /// Latest server checkpoint snapshot.
    pub checkpoint_arc: u16,
    pub checkpoint_step: u32,
    pub checkpoint_carrier: Option<u32>,
    /// Party flags supplied by the server's `[DMJ]` channel.
    pub flags: Vec<CampaignFlag>,
    /// Typed DM objective state supplied by the server.
    pub objectives: Vec<DmObjectiveProgress>,
    /// Latest server-generated preview or confirm result.
    pub reconciliation: Option<ReconciliationState>,
    /// Monotonic server sequence; older chat echoes are ignored.
    pub last_sequence: u64,
}

impl DmCampaignState {
    /// Apply only a version-valid, monotonic server message. This state has no
    /// local action/chat inference path: all campaign progress comes through
    /// this authoritative echo or the existing quest packets.
    pub fn apply_authoritative(&mut self, message: DmjMessage) -> bool {
        if !message.is_supported() {
            return false;
        }
        if let DmjMessage::Reset { seq, .. } = message {
            self.bestiary_unlocked.clear();
            self.checkpoint_arc = 0;
            self.checkpoint_step = 0;
            self.checkpoint_carrier = None;
            self.flags.clear();
            self.objectives.clear();
            self.reconciliation = None;
            self.last_sequence = seq;
            return true;
        }
        if message.sequence() < self.last_sequence {
            return false;
        }
        self.last_sequence = message.sequence();
        match message {
            DmjMessage::Checkpoint { arc, step, carrier, .. } => {
                self.checkpoint_arc = arc;
                self.checkpoint_step = step;
                self.checkpoint_carrier = carrier;
            }
            DmjMessage::Flag { name, value, .. } => {
                if let Some(flag) = self.flags.iter_mut().find(|flag| flag.name == name) {
                    flag.value = value;
                } else {
                    self.flags.push(CampaignFlag { name, value });
                }
            }
            DmjMessage::Objective {
                quest_id,
                objective_id,
                kind,
                current,
                total,
                required,
                completed,
                party_shared,
                dm_triggered,
                ..
            } => {
                let update = DmObjectiveProgress {
                    quest_id,
                    objective_id,
                    kind,
                    current,
                    total,
                    required,
                    completed,
                    party_shared,
                    dm_triggered,
                };
                if let Some(existing) = self
                    .objectives
                    .iter_mut()
                    .find(|objective| objective.quest_id == quest_id && objective.objective_id == objective_id)
                {
                    *existing = update;
                } else {
                    self.objectives.push(update);
                }
            }
            DmjMessage::Reconcile {
                party_id,
                arc,
                step,
                mode,
                eligible,
                ahead,
                offline,
                unavailable,
                changed,
                ..
            } => {
                self.reconciliation = Some(ReconciliationState {
                    party_id,
                    arc,
                    step,
                    mode,
                    eligible,
                    ahead,
                    offline,
                    unavailable,
                    changed,
                });
            }
            DmjMessage::Reset { .. } => unreachable!("reset is handled before sequence validation"),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{DmCampaignState, DmjMessage};

    #[test]
    fn authoritative_messages_update_checkpoint_flags_and_objective() {
        let mut state = DmCampaignState::default();
        assert!(state.apply_authoritative(DmjMessage::Checkpoint {
            v: 1,
            seq: 1,
            arc: 1,
            step: 3,
            carrier: Some(42),
        }));
        assert!(state.apply_authoritative(DmjMessage::Flag {
            v: 1,
            seq: 2,
            name: "dm_arc01_child_found".to_owned(),
            value: 1,
        }));
        assert!(state.apply_authoritative(DmjMessage::Objective {
            v: 1,
            seq: 3,
            quest_id: 20006,
            objective_id: 2,
            kind: "DM".to_owned(),
            current: 1,
            total: 1,
            required: true,
            completed: true,
            party_shared: true,
            dm_triggered: true,
        }));
        assert_eq!(state.checkpoint_step, 3);
        assert_eq!(state.flags[0].value, 1);
        assert!(state.objectives[0].completed);
        assert!(state.objectives[0].dm_triggered);
    }

    #[test]
    fn stale_echoes_cannot_roll_authoritative_state_back() {
        let mut state = DmCampaignState::default();
        assert!(state.apply_authoritative(DmjMessage::Checkpoint {
            v: 1,
            seq: 4,
            arc: 1,
            step: 9,
            carrier: None,
        }));
        assert!(!state.apply_authoritative(DmjMessage::Checkpoint {
            v: 1,
            seq: 3,
            arc: 1,
            step: 2,
            carrier: Some(7),
        }));
        assert_eq!(state.checkpoint_step, 9);
        assert_eq!(state.last_sequence, 4);
    }

    #[test]
    fn reset_clears_authoritative_campaign_state_and_reopens_sequence() {
        let mut state = DmCampaignState::default();
        state.checkpoint_step = 8;
        state.last_sequence = 8;
        state.flags.push(super::CampaignFlag {
            name: "dm_arc01_started".to_owned(),
            value: 1,
        });
        assert!(state.apply_authoritative(DmjMessage::Reset { v: 1, seq: 100_000 }));
        assert_eq!(state.checkpoint_step, 0);
        assert!(state.flags.is_empty());
        assert_eq!(state.last_sequence, 100_000);
        assert!(state.apply_authoritative(DmjMessage::Checkpoint {
            v: 1,
            seq: 100_001,
            arc: 1,
            step: 1,
            carrier: None,
        }));
        assert_eq!(state.checkpoint_step, 1);
    }

    #[test]
    fn reconciliation_preview_and_confirm_are_typed_server_state() {
        let mut state = DmCampaignState::default();
        assert!(state.apply_authoritative(DmjMessage::Reconcile {
            v: 1,
            seq: 100_004,
            party_id: 7,
            arc: 1,
            step: 4,
            mode: "preview".to_owned(),
            eligible: 2,
            ahead: 1,
            offline: 1,
            unavailable: 0,
            changed: 0,
        }));
        let preview = state.reconciliation.as_ref().expect("preview state");
        assert_eq!(preview.eligible, 2);
        assert_eq!(preview.changed, 0);
        assert!(state.apply_authoritative(DmjMessage::Reconcile {
            v: 1,
            seq: 100_005,
            party_id: 7,
            arc: 1,
            step: 4,
            mode: "confirm".to_owned(),
            eligible: 2,
            ahead: 1,
            offline: 1,
            unavailable: 0,
            changed: 2,
        }));
        assert_eq!(state.reconciliation.as_ref().unwrap().changed, 2);
    }

    #[test]
    fn two_clients_converge_on_authoritative_reconciliation_snapshots() {
        let messages = [
            DmjMessage::Checkpoint {
                v: 1,
                seq: 200,
                arc: 1,
                step: 6,
                carrier: Some(42),
            },
            DmjMessage::Flag {
                v: 1,
                seq: 201,
                name: "dm_arc01_binding_applied".to_owned(),
                value: 1,
            },
            DmjMessage::Reconcile {
                v: 1,
                seq: 202,
                party_id: 7,
                arc: 1,
                step: 6,
                mode: "preview".to_owned(),
                eligible: 2,
                ahead: 1,
                offline: 1,
                unavailable: 1,
                changed: 0,
            },
            DmjMessage::Checkpoint {
                v: 1,
                seq: 203,
                arc: 1,
                step: 6,
                carrier: Some(42),
            },
            DmjMessage::Reconcile {
                v: 1,
                seq: 204,
                party_id: 7,
                arc: 1,
                step: 6,
                mode: "confirm".to_owned(),
                eligible: 2,
                ahead: 1,
                offline: 1,
                unavailable: 1,
                changed: 2,
            },
        ];

        let mut primary = DmCampaignState::default();
        let mut late_joiner = DmCampaignState::default();
        for message in messages {
            assert!(primary.apply_authoritative(message.clone()));
            assert!(late_joiner.apply_authoritative(message));
        }

        assert_eq!(primary.checkpoint_arc, late_joiner.checkpoint_arc);
        assert_eq!(primary.checkpoint_step, late_joiner.checkpoint_step);
        assert_eq!(primary.checkpoint_carrier, late_joiner.checkpoint_carrier);
        assert_eq!(primary.flags, late_joiner.flags);
        assert_eq!(primary.objectives, late_joiner.objectives);
        assert_eq!(primary.reconciliation, late_joiner.reconciliation);
        assert_eq!(primary.last_sequence, 204);
        assert_eq!(primary.reconciliation.as_ref().map(|result| result.changed), Some(2));
    }
}
