//! Campaign state the server reports to the client as `[DMJ]{json}` lines.
//!
//! The server (`dm_dmj.txt`, `dm_checkpoint.txt`) sends these with
//! `dispbottom`, so they arrive as ordinary server chat text. This module
//! recognises them, applies the ones it understands, and tells the caller to
//! keep **every** `[DMJ]` line out of the chat history, understood or not: a
//! player must never read raw JSON, and a line from a newer server is not an
//! error.
//!
//! Campaign flags, quests and checkpoints stay authoritative on the server. The
//! client only displays what it is told and never infers progress from chat or
//! local actions. See `docs/specs/dm-flag-channel.md`.

use std::collections::{BTreeMap, HashMap};

use korangar_interface::element::StateElement;
use rust_state::RustState;
use serde_json::Value;

const PREFIX: &str = "[DMJ]";
/// Wire version this client understands. Anything else is dropped.
const VERSION: u64 = 1;
/// Hercules chat lines are at most 255 bytes; anything near 1 KiB is not ours.
const MAX_LINE_BYTES: usize = 1024;
const MAX_FLAGS: usize = 512;
const MAX_FLAG_NAME: usize = 64;
const MAX_OBJECTIVES: usize = 512;
/// A snapshot is a handful of short parts; refuse a claim of more than this.
const MAX_SNAPSHOT_PARTS: u64 = 64;
const MAX_FLAGS_PER_PART: usize = 16;

/// The party's last confirmed campaign step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DmCheckpoint {
    pub arc: u32,
    pub step: u32,
    pub carrier: u32,
    seq: u64,
}

/// One typed objective, keyed by `(quest_id, objective_id)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DmObjective {
    pub kind: String,
    pub current: u32,
    pub total: u32,
    pub required: u32,
    pub completed: bool,
    pub party_shared: bool,
    pub dm_triggered: bool,
}

/// The result of a DM's checkpoint preview or confirm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DmReconcile {
    pub arc: u32,
    pub step: u32,
    pub mode: String,
    pub eligible: u32,
    pub ahead: u32,
    pub offline: u32,
    pub unavailable: u32,
    pub changed: u32,
}

/// A flag snapshot being assembled from its numbered parts. Nothing in it
/// touches the flag table until every part of the same `seq` has arrived.
#[derive(Clone, Debug)]
pub(crate) struct SnapshotAssembly {
    seq: u64,
    of: u64,
    parts: BTreeMap<u64, Vec<(String, i64)>>,
}

#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct DmJournalState {
    #[hidden_element]
    checkpoint: Option<DmCheckpoint>,
    #[hidden_element]
    flags: HashMap<String, i64>,
    #[hidden_element]
    objectives: HashMap<(u32, u32), DmObjective>,
    #[hidden_element]
    last_reconcile: Option<DmReconcile>,
    #[hidden_element]
    snapshot: Option<SnapshotAssembly>,
    /// `[DMJ]` lines that were consumed but not understood (bad JSON, wrong
    /// version, unknown type, over the size cap). Diagnostic only.
    #[hidden_element]
    dropped: u32,
}

impl DmJournalState {
    /// Consume one server chat line. Returns true for **any** line that starts
    /// with `[DMJ]`, so the caller keeps it out of chat; the line is applied
    /// only if it is well formed and understood.
    pub fn receive_server_line(&mut self, text: &str) -> bool {
        let Some(body) = text.strip_prefix(PREFIX) else {
            return false;
        };
        if text.len() > MAX_LINE_BYTES || !self.apply(body) {
            self.dropped = self.dropped.saturating_add(1);
        }
        true
    }

    fn apply(&mut self, body: &str) -> bool {
        let Ok(message) = serde_json::from_str::<Value>(body.trim()) else {
            return false;
        };
        if message.get("v").and_then(Value::as_u64) != Some(VERSION) {
            return false;
        }
        let seq = message.get("seq").and_then(Value::as_u64).unwrap_or(0);
        match message.get("t").and_then(Value::as_str) {
            Some("checkpoint") => self.apply_checkpoint(&message, seq),
            Some("flag") => self.apply_flag(&message),
            Some("flags") => self.apply_flag_part(&message, seq),
            Some("objective") => self.apply_objective(&message),
            Some("reconcile") => self.apply_reconcile(&message),
            Some("reset") => {
                self.clear();
                true
            }
            _ => false,
        }
    }

    fn apply_checkpoint(&mut self, message: &Value, seq: u64) -> bool {
        let (Some(arc), Some(step), Some(carrier)) = (uint(message, "arc"), uint(message, "step"), uint(message, "carrier")) else {
            return false;
        };
        // A late or replayed line must not move the party backwards.
        if self.checkpoint.is_some_and(|current| seq < current.seq) {
            return true;
        }
        self.checkpoint = Some(DmCheckpoint { arc, step, carrier, seq });
        true
    }

    fn apply_flag(&mut self, message: &Value) -> bool {
        let Some(name) = message.get("name").and_then(Value::as_str) else {
            return false;
        };
        let Some(value) = message.get("value").and_then(Value::as_i64) else {
            return false;
        };
        if !valid_flag_name(name) {
            return false;
        }
        // Last write wins, in arrival order: a flag can legitimately go back to 0.
        if !self.flags.contains_key(name) && self.flags.len() >= MAX_FLAGS {
            return false;
        }
        self.flags.insert(name.to_owned(), value);
        true
    }

    /// One part of a flag snapshot. The whole snapshot replaces the flag table,
    /// and only once every part of one `seq` is in; a partial snapshot, or one
    /// overtaken by a newer `seq`, never changes what the client believes.
    fn apply_flag_part(&mut self, message: &Value, seq: u64) -> bool {
        let (Some(part), Some(of)) = (
            message.get("part").and_then(Value::as_u64),
            message.get("of").and_then(Value::as_u64),
        ) else {
            return false;
        };
        if of == 0 || of > MAX_SNAPSHOT_PARTS || part >= of {
            return false;
        }
        let Some(object) = message.get("flags").and_then(Value::as_object) else {
            return false;
        };
        if object.len() > MAX_FLAGS_PER_PART {
            return false;
        }
        let mut entries = Vec::with_capacity(object.len());
        for (name, value) in object {
            let Some(value) = value.as_i64() else {
                return false;
            };
            if !valid_flag_name(name) {
                return false;
            }
            entries.push((name.clone(), value));
        }

        // A different `seq` starts a new snapshot; the old one is abandoned.
        // An older one than the one in progress is a straggler: ignore it.
        match &self.snapshot {
            Some(current) if seq < current.seq => return true,
            Some(current) if seq == current.seq && current.of == of => {}
            _ => {
                self.snapshot = Some(SnapshotAssembly {
                    seq,
                    of,
                    parts: BTreeMap::new(),
                })
            }
        }
        let Some(assembly) = self.snapshot.as_mut() else {
            return false;
        };
        assembly.parts.insert(part, entries);
        if assembly.parts.len() as u64 != assembly.of {
            return true;
        }

        let complete = self.snapshot.take().map(|assembly| assembly.parts).unwrap_or_default();
        let mut table: HashMap<String, i64> = HashMap::new();
        for (name, value) in complete.into_values().flatten() {
            if table.len() >= MAX_FLAGS && !table.contains_key(&name) {
                return false;
            }
            table.insert(name, value);
        }
        self.flags = table;
        true
    }

    fn apply_objective(&mut self, message: &Value) -> bool {
        let (Some(quest_id), Some(objective_id)) = (uint(message, "quest_id"), uint(message, "objective_id")) else {
            return false;
        };
        let Some(kind) = message.get("kind").and_then(Value::as_str) else {
            return false;
        };
        let key = (quest_id, objective_id);
        if !self.objectives.contains_key(&key) && self.objectives.len() >= MAX_OBJECTIVES {
            return false;
        }
        self.objectives.insert(key, DmObjective {
            kind: kind.chars().take(24).collect(),
            current: uint(message, "current").unwrap_or(0),
            total: uint(message, "total").unwrap_or(0),
            required: uint(message, "required").unwrap_or(1),
            completed: uint(message, "completed").unwrap_or(0) != 0,
            party_shared: uint(message, "party_shared").unwrap_or(1) != 0,
            dm_triggered: uint(message, "dm_triggered").unwrap_or(1) != 0,
        });
        true
    }

    fn apply_reconcile(&mut self, message: &Value) -> bool {
        let (Some(arc), Some(step)) = (uint(message, "arc"), uint(message, "step")) else {
            return false;
        };
        let Some(mode) = message.get("mode").and_then(Value::as_str) else {
            return false;
        };
        self.last_reconcile = Some(DmReconcile {
            arc,
            step,
            mode: mode.chars().take(16).collect(),
            eligible: uint(message, "eligible").unwrap_or(0),
            ahead: uint(message, "ahead").unwrap_or(0),
            offline: uint(message, "offline").unwrap_or(0),
            unavailable: uint(message, "unavailable").unwrap_or(0),
            changed: uint(message, "changed").unwrap_or(0),
        });
        true
    }

    /// Forget everything the server told us (server `reset`, or logout).
    pub fn clear(&mut self) {
        let dropped = self.dropped;
        *self = Self::default();
        self.dropped = dropped;
    }

    /// A flag's value as the server last reported it; 0 if never reported.
    #[allow(dead_code)]
    pub fn flag(&self, name: &str) -> i64 {
        self.flags.get(name).copied().unwrap_or(0)
    }

    #[allow(dead_code)]
    pub fn checkpoint(&self) -> Option<DmCheckpoint> {
        self.checkpoint
    }

    #[allow(dead_code)]
    pub fn objective(&self, quest_id: u32, objective_id: u32) -> Option<&DmObjective> {
        self.objectives.get(&(quest_id, objective_id))
    }

    #[allow(dead_code)]
    pub fn last_reconcile(&self) -> Option<&DmReconcile> {
        self.last_reconcile.as_ref()
    }

    #[allow(dead_code)]
    pub fn dropped(&self) -> u32 {
        self.dropped
    }
}

fn uint(message: &Value, key: &str) -> Option<u32> {
    message.get(key)?.as_u64().and_then(|value| u32::try_from(value).ok())
}

/// Campaign flags are `dm_` identifiers; refuse anything else so a line can
/// never plant an odd key.
fn valid_flag_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_FLAG_NAME
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::{DmJournalState, MAX_FLAGS};

    fn line(body: &str) -> String {
        format!("[DMJ]{body}")
    }

    #[test]
    fn only_dmj_lines_are_consumed() {
        let mut state = DmJournalState::default();
        assert!(!state.receive_server_line("[DM] Campaign NPCs are active"));
        assert!(!state.receive_server_line("hello [DMJ]{}"));
        assert!(state.receive_server_line(&line("{}")));
    }

    #[test]
    fn a_flag_line_sets_the_flag_and_a_later_line_can_clear_it() {
        let mut state = DmJournalState::default();
        assert!(state.receive_server_line(&line(r#"{"t":"flag","v":1,"seq":5,"name":"dm_arc01_clue_mask","value":5}"#)));
        assert_eq!(state.flag("dm_arc01_clue_mask"), 5);
        state.receive_server_line(&line(r#"{"t":"flag","v":1,"seq":6,"name":"dm_arc01_clue_mask","value":0}"#));
        assert_eq!(state.flag("dm_arc01_clue_mask"), 0);
        assert_eq!(state.dropped(), 0);
    }

    #[test]
    fn a_checkpoint_never_moves_backwards() {
        let mut state = DmJournalState::default();
        state.receive_server_line(&line(r#"{"t":"checkpoint","v":1,"seq":200003,"arc":2,"step":3,"carrier":7}"#));
        state.receive_server_line(&line(r#"{"t":"checkpoint","v":1,"seq":100001,"arc":1,"step":1,"carrier":7}"#));
        let checkpoint = state.checkpoint().expect("checkpoint set");
        assert_eq!((checkpoint.arc, checkpoint.step), (2, 3));
    }

    #[test]
    fn an_objective_is_keyed_by_quest_and_objective() {
        let mut state = DmJournalState::default();
        state.receive_server_line(&line(
            r#"{"t":"objective","v":1,"seq":0,"quest_id":20101,"objective_id":2,"kind":"Explore","current":1,"total":2,"required":1,"completed":0,"party_shared":1,"dm_triggered":0}"#,
        ));
        let objective = state.objective(20101, 2).expect("objective stored");
        assert_eq!((objective.kind.as_str(), objective.current, objective.total), ("Explore", 1, 2));
        assert!(!objective.completed && objective.party_shared && !objective.dm_triggered);
        assert!(state.objective(20101, 1).is_none());
    }

    #[test]
    fn reset_clears_everything_but_the_drop_count() {
        let mut state = DmJournalState::default();
        state.receive_server_line(&line(r#"{"t":"flag","v":1,"seq":1,"name":"dm_x","value":1}"#));
        state.receive_server_line(&line("not json"));
        state.receive_server_line(&line(r#"{"t":"reset","v":1,"seq":100000}"#));
        assert_eq!(state.flag("dm_x"), 0);
        assert_eq!(state.dropped(), 1);
    }

    /// Every one of these is consumed (kept out of chat) and none changes
    /// state.
    #[test]
    fn unusable_lines_are_consumed_and_counted_not_applied() {
        let mut state = DmJournalState::default();
        let bad = [
            "not json".to_owned(),
            r#"{"t":"flag","v":2,"name":"dm_x","value":1}"#.to_owned(), // newer version
            r#"{"t":"mystery","v":1}"#.to_owned(),                      // unknown type
            r#"{"t":"flag","v":1,"name":"Not A Flag!","value":1}"#.to_owned(), // bad name
            r#"{"t":"flag","v":1,"name":"dm_x"}"#.to_owned(),           // no value
            r#"{"t":"checkpoint","v":1,"arc":1}"#.to_owned(),           // missing fields
            format!(r#"{{"t":"flag","v":1,"name":"dm_x","value":1,"pad":"{}"}}"#, "x".repeat(2000)), // oversize
        ];
        for body in &bad {
            assert!(state.receive_server_line(&line(body)), "{body:.40} must still be consumed");
        }
        assert_eq!(state.dropped() as usize, bad.len());
        assert_eq!(state.flag("dm_x"), 0);
        assert!(state.checkpoint().is_none());
    }

    #[test]
    fn the_flag_table_is_bounded() {
        let mut state = DmJournalState::default();
        for index in 0..MAX_FLAGS + 10 {
            state.receive_server_line(&line(&format!(r#"{{"t":"flag","v":1,"name":"dm_f{index}","value":1}}"#)));
        }
        assert_eq!(state.dropped(), 10);
        // An existing flag can still be updated when the table is full.
        state.receive_server_line(&line(r#"{"t":"flag","v":1,"name":"dm_f0","value":9}"#));
        assert_eq!(state.flag("dm_f0"), 9);
    }

    #[test]
    fn a_reconcile_result_is_kept() {
        let mut state = DmJournalState::default();
        state.receive_server_line(&line(
            r#"{"t":"reconcile","v":1,"seq":1,"party_id":4,"arc":3,"step":2,"mode":"preview","eligible":2,"ahead":1,"offline":0,"unavailable":0,"changed":0}"#,
        ));
        let reconcile = state.last_reconcile().expect("reconcile stored");
        assert_eq!(
            (reconcile.mode.as_str(), reconcile.eligible, reconcile.ahead),
            ("preview", 2, 1)
        );
    }

    fn part(seq: u64, part: u64, of: u64, flags: &str) -> String {
        line(&format!(
            r#"{{"t":"flags","v":1,"seq":{seq},"part":{part},"of":{of},"flags":{{{flags}}}}}"#
        ))
    }

    /// The snapshot replaces the table only when every part has arrived, and a
    /// flag absent from the snapshot goes back to 0.
    #[test]
    fn a_complete_snapshot_replaces_the_flag_table() {
        let mut state = DmJournalState::default();
        state.receive_server_line(&line(r#"{"t":"flag","v":1,"seq":0,"name":"dm_old","value":3}"#));
        assert!(state.receive_server_line(&part(9, 0, 2, r#""dm_a":1,"dm_b":0"#)));
        assert_eq!(state.flag("dm_old"), 3, "a partial snapshot must not change anything");
        assert_eq!(state.flag("dm_a"), 0);
        assert!(state.receive_server_line(&part(9, 1, 2, r#""dm_c":5"#)));
        assert_eq!((state.flag("dm_a"), state.flag("dm_c")), (1, 5));
        assert_eq!(state.flag("dm_old"), 0, "an unlisted flag is 0 after a snapshot");
        assert_eq!(state.dropped(), 0);
    }

    #[test]
    fn parts_may_arrive_out_of_order() {
        let mut state = DmJournalState::default();
        state.receive_server_line(&part(4, 1, 2, r#""dm_c":5"#));
        state.receive_server_line(&part(4, 0, 2, r#""dm_a":1"#));
        assert_eq!((state.flag("dm_a"), state.flag("dm_c")), (1, 5));
    }

    /// A new snapshot abandons an unfinished older one; a straggler from an
    /// older snapshot cannot revive it.
    #[test]
    fn a_newer_snapshot_abandons_an_unfinished_one() {
        let mut state = DmJournalState::default();
        state.receive_server_line(&part(5, 0, 2, r#""dm_a":1"#));
        state.receive_server_line(&part(6, 0, 1, r#""dm_a":7"#));
        assert_eq!(state.flag("dm_a"), 7);
        state.receive_server_line(&part(5, 1, 2, r#""dm_b":9"#));
        assert_eq!(state.flag("dm_b"), 0, "the older snapshot's last part must not apply");
        assert_eq!(state.flag("dm_a"), 7);
    }

    #[test]
    fn a_repeated_part_does_not_complete_a_snapshot() {
        let mut state = DmJournalState::default();
        state.receive_server_line(&part(3, 0, 2, r#""dm_a":1"#));
        state.receive_server_line(&part(3, 0, 2, r#""dm_a":1"#));
        assert_eq!(state.flag("dm_a"), 0);
    }

    #[test]
    fn malformed_snapshot_parts_are_consumed_and_counted() {
        let mut state = DmJournalState::default();
        let bad = [
            part(1, 2, 2, r#""dm_a":1"#),                                                // part out of range
            part(1, 0, 0, r#""dm_a":1"#),                                                // zero parts
            part(1, 0, 999, r#""dm_a":1"#),                                              // too many parts
            part(1, 0, 1, r#""Bad Name":1"#),                                            // bad flag name
            line(r#"{"t":"flags","v":1,"seq":1,"part":0,"of":1,"flags":{"dm_a":"x"}}"#), // non-integer value
            line(r#"{"t":"flags","v":1,"seq":1,"part":0,"of":1}"#),                      // no flags object
        ];
        for body in &bad {
            assert!(state.receive_server_line(body));
        }
        assert_eq!(state.dropped() as usize, bad.len());
        assert_eq!(state.flag("dm_a"), 0);
    }
}
