//! Versioned `[DMJ]` chat transport for authoritative DM campaign state.
//!
//! DMJ is deliberately narrow: it accepts only structured server echoes. A
//! normal player chat line, malformed JSON, unsupported versions, or an
//! unknown message type is ignored and remains ordinary chat.

use serde::Deserialize;

const DMJ_PREFIX: &str = "[DMJ]";
const DMJ_VERSION: u8 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "t")]
pub enum DmjMessage {
    #[serde(rename = "checkpoint")]
    Checkpoint {
        v: u8,
        seq: u64,
        arc: u16,
        step: u32,
        carrier: Option<u32>,
    },
    #[serde(rename = "flag")]
    Flag { v: u8, seq: u64, name: String, value: u32 },
    #[serde(rename = "objective")]
    Objective {
        v: u8,
        seq: u64,
        quest_id: u32,
        objective_id: u32,
        kind: String,
        current: u16,
        total: u16,
        required: bool,
        completed: bool,
        party_shared: bool,
        dm_triggered: bool,
    },
    #[serde(rename = "reconcile")]
    Reconcile {
        v: u8,
        seq: u64,
        party_id: u32,
        arc: u16,
        step: u32,
        mode: String,
        eligible: u32,
        ahead: u32,
        offline: u32,
        #[serde(default)]
        unavailable: u32,
        changed: u32,
    },
    #[serde(rename = "reset")]
    Reset { v: u8, seq: u64 },
}

impl DmjMessage {
    pub fn sequence(&self) -> u64 {
        match self {
            Self::Checkpoint { seq, .. }
            | Self::Flag { seq, .. }
            | Self::Objective { seq, .. }
            | Self::Reconcile { seq, .. }
            | Self::Reset { seq, .. } => *seq,
        }
    }

    pub(crate) fn is_supported(&self) -> bool {
        match self {
            Self::Checkpoint { v, .. }
            | Self::Flag { v, .. }
            | Self::Objective { v, .. }
            | Self::Reconcile { v, .. }
            | Self::Reset { v, .. } => *v == DMJ_VERSION,
        }
    }
}

pub fn parse_dmj(text: &str) -> Option<DmjMessage> {
    let payload = text.strip_prefix(DMJ_PREFIX)?.trim();
    let message = serde_json::from_str::<DmjMessage>(payload).ok()?;
    message.is_supported().then_some(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versioned_checkpoint() {
        let message = parse_dmj(r#"[DMJ]{"t":"checkpoint","v":1,"seq":7,"arc":1,"step":4,"carrier":42}"#).expect("checkpoint should parse");
        assert_eq!(message.sequence(), 7);
        assert!(matches!(message, DmjMessage::Checkpoint {
            arc: 1,
            step: 4,
            carrier: Some(42),
            ..
        }));
    }

    #[test]
    fn parses_typed_dm_objective_without_local_inference() {
        let message = parse_dmj(
            r#"[DMJ]{"t":"objective","v":1,"seq":8,"quest_id":20006,"objective_id":2,"kind":"DM","current":1,"total":1,"required":true,"completed":true,"party_shared":true,"dm_triggered":true}"#,
        )
        .expect("objective should parse");
        assert!(matches!(message, DmjMessage::Objective { kind, completed: true, dm_triggered: true, .. } if kind == "DM"));
    }

    #[test]
    fn rejects_malformed_unknown_version_and_plain_chat() {
        assert!(parse_dmj("[DMJ]not json").is_none());
        assert!(parse_dmj(r#"[DMJ]{"t":"flag","v":2,"seq":1,"name":"dm_arc01_started","value":1}"#).is_none());
        assert!(parse_dmj("A player says [DMJ]{...}").is_none());
    }

    #[test]
    fn parses_explicit_reset_message() {
        let message = parse_dmj(r#"[DMJ]{"t":"reset","v":1,"seq":99}"#).expect("reset should parse");
        assert!(matches!(message, DmjMessage::Reset { seq: 99, .. }));
    }

    #[test]
    fn parses_reconciliation_result() {
        let message = parse_dmj(r#"[DMJ]{"t":"reconcile","v":1,"seq":100004,"party_id":7,"arc":1,"step":4,"mode":"preview","eligible":2,"ahead":1,"offline":1,"unavailable":0,"changed":0}"#).expect("reconciliation should parse");
        assert!(matches!(message, DmjMessage::Reconcile { party_id: 7, mode, eligible: 2, changed: 0, .. } if mode == "preview"));
    }

    #[test]
    fn accepts_older_v1_reconciliation_without_unavailable_count() {
        let message = parse_dmj(r#"[DMJ]{"t":"reconcile","v":1,"seq":100004,"party_id":7,"arc":1,"step":4,"mode":"preview","eligible":2,"ahead":1,"offline":1,"changed":0}"#).expect("legacy v1 reconciliation should parse");
        assert!(matches!(message, DmjMessage::Reconcile { unavailable: 0, .. }));
    }
}
