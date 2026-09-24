//! Server-authoritative, account-scoped monster discovery synchronization.
//!
//! Hercules transports these private server messages through its existing
//! `dispbottom` packet. They are intercepted before chat rendering. Party-chat
//! text is deliberately never parsed as an authority for discovery.

use std::collections::{HashMap, HashSet};

use rust_state::RustState;

const PREFIX: &str = "[KORANGAR-DISCOVERY:v1:";
const MAP_PREFIX: &str = "[KORANGAR-MAP-DISCOVERY:v1:";
const MAX_DISCOVERIES: usize = 20_000;
const MAX_CHUNKS: usize = 1_000;
const PAIRS_PER_CHUNK: usize = 20;
const MAX_VISITED_MAPS: usize = 2_000;
const MAPS_PER_CHUNK: usize = 8;

#[derive(Clone, Debug, PartialEq, RustState)]
pub(crate) struct MapSnapshotAssembly {
    sequence: u32,
    expected_entries: usize,
    chunks: Vec<Option<Vec<String>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, RustState)]
pub struct DiscoveryMilestone {
    pub monster_id: u16,
    pub milestone: u8,
}

#[derive(Clone, Debug, PartialEq, RustState)]
pub struct SnapshotAssembly {
    sequence: u32,
    expected_entries: usize,
    chunks: Vec<Option<Vec<DiscoveryMilestone>>>,
    pending_deltas: Vec<DiscoveryMilestone>,
}

/// Monotonic account discoveries received from the authoritative game server.
#[derive(Clone, Default, Debug, PartialEq, RustState)]
pub struct DiscoveryState {
    /// Highest milestone known for each mob. This is refreshed by complete
    /// server snapshots and merged monotonically with subsequent deltas.
    discoveries: HashMap<u16, u8>,
    account_id: Option<u32>,
    snapshot_complete: bool,
    snapshot: Option<SnapshotAssembly>,
    visited_maps: HashSet<String>,
    map_snapshot_complete: bool,
    map_snapshot: Option<MapSnapshotAssembly>,
}

impl DiscoveryState {
    /// Consume one private server line. Returns true only for a recognized
    /// protocol line, allowing the caller to keep it out of the chat history.
    pub fn receive_server_line(&mut self, text: &str, expected_account_id: Option<u32>) -> bool {
        if let Some(body) = text.strip_prefix(MAP_PREFIX).and_then(|rest| rest.strip_suffix(']')) {
            return self.receive_map_body(body, expected_account_id);
        }
        let Some(body) = text.strip_prefix(PREFIX).and_then(|rest| rest.strip_suffix(']')) else {
            return false;
        };
        let Some(expected_account_id) = expected_account_id else {
            return false;
        };
        if self.account_id != Some(expected_account_id) {
            return false;
        }
        let mut fields = body.splitn(5, ':');
        let Some(kind) = fields.next() else { return false };
        match kind {
            "begin" => self.receive_begin(fields, expected_account_id),
            "chunk" => self.receive_chunk(fields, expected_account_id),
            "end" => self.receive_end(fields, expected_account_id),
            "delta" => self.receive_delta(fields, expected_account_id),
            _ => false,
        }
    }

    pub fn milestone(&self, monster_id: u16) -> Option<u8> {
        self.discoveries.get(&monster_id).copied()
    }

    pub fn discovered_count(&self) -> usize {
        self.discoveries.len()
    }

    pub fn visited_map(&self, map_name: &str) -> bool {
        self.visited_maps.contains(&map_name.to_ascii_lowercase())
    }

    pub fn visited_map_count(&self) -> usize {
        self.visited_maps.len()
    }

    pub fn map_snapshot_complete(&self) -> bool {
        self.map_snapshot_complete
    }

    pub fn snapshot_complete(&self) -> bool {
        self.snapshot_complete
    }

    /// Account changes invalidate the prior ledger and any in-flight snapshot.
    pub fn set_account_id(&mut self, account_id: u32) {
        if self.account_id != Some(account_id) {
            self.account_id = Some(account_id);
            self.discoveries.clear();
            self.snapshot = None;
            self.snapshot_complete = false;
            self.visited_maps.clear();
            self.map_snapshot = None;
            self.map_snapshot_complete = false;
        }
    }

    fn receive_map_body(&mut self, body: &str, expected_account_id: Option<u32>) -> bool {
        let Some(expected_account_id) = expected_account_id else {
            return false;
        };
        if self.account_id != Some(expected_account_id) {
            return false;
        }
        let fields = body.split(':').collect::<Vec<_>>();
        let Some(kind) = fields.first().copied() else { return false };
        let Some(account) = fields.get(1).and_then(|field| field.parse::<u32>().ok()) else {
            return false;
        };
        let Some(sequence_or_map) = fields.get(2).copied() else {
            return false;
        };
        if account != expected_account_id {
            return false;
        }
        match kind {
            "visited" => {
                if fields.len() != 3 {
                    return false;
                }
                let Some(map_name) = valid_map_name(sequence_or_map) else {
                    return false;
                };
                if self.visited_maps.len() < MAX_VISITED_MAPS || self.visited_maps.contains(&map_name) {
                    self.visited_maps.insert(map_name);
                }
                true
            }
            "begin" => {
                if fields.len() != 5 {
                    return false;
                }
                let Some((entries, chunks)) = parse_two_usizes(fields[3], fields[4]) else {
                    return false;
                };
                if entries > MAX_VISITED_MAPS || chunks > MAX_CHUNKS || chunks != entries.div_ceil(MAPS_PER_CHUNK) {
                    return false;
                }
                let Ok(sequence) = sequence_or_map.parse::<u32>() else {
                    return false;
                };
                self.map_snapshot = Some(MapSnapshotAssembly {
                    sequence,
                    expected_entries: entries,
                    chunks: vec![None; chunks],
                });
                true
            }
            "chunk" => {
                if fields.len() != 5 {
                    return false;
                }
                let (Some(index), payload) = (fields[3].parse::<usize>().ok(), fields[4]) else {
                    return false;
                };
                let Ok(sequence) = sequence_or_map.parse::<u32>() else {
                    return false;
                };
                let Some(snapshot) = self.map_snapshot.as_mut().filter(|snapshot| snapshot.sequence == sequence) else {
                    return false;
                };
                let Some(slot) = snapshot.chunks.get_mut(index) else {
                    self.map_snapshot = None;
                    return false;
                };
                let Some(names) = parse_map_names(payload) else {
                    self.map_snapshot = None;
                    return false;
                };
                if names.len() > MAPS_PER_CHUNK {
                    self.map_snapshot = None;
                    return false;
                }
                match slot {
                    Some(previous) if *previous == names => true,
                    Some(_) => {
                        self.map_snapshot = None;
                        false
                    }
                    None => {
                        *slot = Some(names);
                        true
                    }
                }
            }
            "end" => {
                if fields.len() != 3 {
                    return false;
                }
                let Ok(sequence) = sequence_or_map.parse::<u32>() else {
                    return false;
                };
                let Some(snapshot) = self.map_snapshot.take().filter(|snapshot| snapshot.sequence == sequence) else {
                    return false;
                };
                let Some(chunks) = snapshot.chunks.into_iter().collect::<Option<Vec<_>>>() else {
                    return false;
                };
                let names = chunks.into_iter().flatten().collect::<Vec<_>>();
                if names.len() != snapshot.expected_entries || names.iter().collect::<HashSet<_>>().len() != snapshot.expected_entries {
                    return false;
                }
                self.visited_maps
                    .extend(names.into_iter().take(MAX_VISITED_MAPS - self.visited_maps.len()));
                self.map_snapshot_complete = true;
                true
            }
            _ => false,
        }
    }

    fn receive_begin<'a>(&mut self, mut fields: impl Iterator<Item = &'a str>, expected_account: u32) -> bool {
        let (Some(account), Some(sequence), Some(entries), Some(chunks), None) =
            (fields.next(), fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return false;
        };
        let (Ok(account), Ok(sequence), Ok(expected_entries), Ok(chunk_count)) = (
            account.parse::<u32>(),
            sequence.parse::<u32>(),
            entries.parse::<usize>(),
            chunks.parse::<usize>(),
        ) else {
            return false;
        };
        if account != expected_account
            || expected_entries > MAX_DISCOVERIES
            || chunk_count > MAX_CHUNKS
            || chunk_count != expected_entries.div_ceil(PAIRS_PER_CHUNK)
        {
            return false;
        }
        self.snapshot = Some(SnapshotAssembly {
            sequence,
            expected_entries,
            chunks: vec![None; chunk_count],
            pending_deltas: Vec::new(),
        });
        true
    }

    fn receive_chunk<'a>(&mut self, mut fields: impl Iterator<Item = &'a str>, expected_account: u32) -> bool {
        let (Some(account), Some(sequence), Some(index), Some(payload), None) =
            (fields.next(), fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return false;
        };
        let (Ok(account), Ok(sequence), Ok(index)) = (account.parse::<u32>(), sequence.parse::<u32>(), index.parse::<usize>()) else {
            return false;
        };
        if account != expected_account {
            return false;
        }
        let Some(snapshot) = self.snapshot.as_mut().filter(|snapshot| snapshot.sequence == sequence) else {
            return false;
        };
        let Some(slot) = snapshot.chunks.get_mut(index) else {
            self.snapshot = None;
            return false;
        };
        let Some(entries) = parse_pairs(payload) else {
            self.snapshot = None;
            return false;
        };
        if entries.len() > PAIRS_PER_CHUNK {
            self.snapshot = None;
            return false;
        }
        match slot {
            Some(previous) if *previous == entries => true,
            Some(_) => {
                self.snapshot = None;
                false
            }
            None => {
                *slot = Some(entries);
                true
            }
        }
    }

    fn receive_end<'a>(&mut self, mut fields: impl Iterator<Item = &'a str>, expected_account: u32) -> bool {
        let (Some(account), Some(sequence), None) = (fields.next(), fields.next(), fields.next()) else {
            return false;
        };
        let (Ok(account), Ok(sequence)) = (account.parse::<u32>(), sequence.parse::<u32>()) else {
            return false;
        };
        if account != expected_account {
            return false;
        }
        let Some(snapshot) = self.snapshot.take().filter(|snapshot| snapshot.sequence == sequence) else {
            return false;
        };
        let Some(chunks) = snapshot.chunks.into_iter().collect::<Option<Vec<_>>>() else {
            // Interrupted or reordered snapshots never clear the prior ledger.
            return false;
        };
        let mut complete = HashMap::with_capacity(snapshot.expected_entries);
        for discovery in chunks.into_iter().flatten() {
            if complete.insert(discovery.monster_id, discovery.milestone).is_some() {
                return false;
            }
        }
        if complete.len() != snapshot.expected_entries {
            return false;
        }
        // Encounter milestones are permanent account history. Keep the client
        // ledger monotonic even if an older/incomplete server snapshot arrives
        // after a newer delta (for example during reconnect overlap).
        for (&monster_id, &milestone) in &self.discoveries {
            complete
                .entry(monster_id)
                .and_modify(|known| *known = (*known).max(milestone))
                .or_insert(milestone);
        }
        for discovery in snapshot.pending_deltas {
            merge(&mut complete, discovery);
        }
        self.discoveries = complete;
        self.snapshot_complete = true;
        true
    }

    fn receive_delta<'a>(&mut self, mut fields: impl Iterator<Item = &'a str>, expected_account: u32) -> bool {
        let (Some(account), Some(id), Some(milestone), None) = (fields.next(), fields.next(), fields.next(), fields.next()) else {
            return false;
        };
        let Ok(account) = account.parse::<u32>() else { return false };
        if account != expected_account {
            return false;
        }
        let Some(discovery) = parse_pair(id, milestone) else { return false };
        if let Some(snapshot) = self.snapshot.as_mut() {
            if snapshot.pending_deltas.len() >= MAX_DISCOVERIES {
                self.snapshot = None;
                return false;
            }
            snapshot.pending_deltas.push(discovery);
        } else {
            merge(&mut self.discoveries, discovery);
        }
        true
    }
}

fn parse_pairs(payload: &str) -> Option<Vec<DiscoveryMilestone>> {
    if payload.is_empty() {
        return None;
    }
    let mut entries = Vec::new();
    for pair in payload.split(',') {
        let Some((id, milestone)) = pair.split_once('=') else { return None };
        entries.push(parse_pair(id, milestone)?);
    }
    Some(entries)
}

fn parse_pair(id: &str, milestone: &str) -> Option<DiscoveryMilestone> {
    let monster_id = id.parse::<u16>().ok()?;
    let milestone = milestone.parse::<u8>().ok()?;
    (monster_id > 0 && (1..=3).contains(&milestone)).then_some(DiscoveryMilestone { monster_id, milestone })
}

fn merge(discoveries: &mut HashMap<u16, u8>, discovery: DiscoveryMilestone) {
    discoveries
        .entry(discovery.monster_id)
        .and_modify(|milestone| *milestone = (*milestone).max(discovery.milestone))
        .or_insert(discovery.milestone);
}

fn parse_two_usizes(first: &str, second: &str) -> Option<(usize, usize)> {
    Some((first.parse().ok()?, second.parse().ok()?))
}

fn parse_map_names(payload: &str) -> Option<Vec<String>> {
    if payload.is_empty() {
        return None;
    }
    payload.split(',').map(valid_map_name).collect()
}

fn valid_map_name(map_name: &str) -> Option<String> {
    (1..=24)
        .contains(&map_name.len())
        .then_some(map_name)
        .filter(|map_name| {
            map_name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'@'))
        })
        .map(str::to_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::DiscoveryState;

    const ACCOUNT_ID: u32 = 42;

    fn receive(state: &mut DiscoveryState, line: &str) -> bool {
        state.set_account_id(ACCOUNT_ID);
        state.receive_server_line(line, Some(ACCOUNT_ID))
    }

    #[test]
    fn snapshot_replaces_only_after_a_complete_valid_sequence() {
        let mut state = DiscoveryState::default();
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1002:1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:begin:42:9:22:2]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:chunk:42:9:0:1002=2,1010=1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1011:1]"));
        assert_eq!(state.milestone(1002), Some(1));
        assert!(!receive(&mut state, "[KORANGAR-DISCOVERY:v1:end:42:9]"));
        assert_eq!(state.milestone(1002), Some(1));

        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:begin:42:10:1:1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:chunk:42:10:0:1002=2]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1011:2]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:end:42:10]"));
        assert_eq!(state.milestone(1002), Some(2));
        assert_eq!(state.milestone(1011), Some(2));
        assert_eq!(state.discovered_count(), 2);
    }

    #[test]
    fn stale_complete_snapshot_cannot_downgrade_account_discovery() {
        let mut state = DiscoveryState::default();
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1002:3]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:begin:42:10:1:1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:chunk:42:10:0:1002=1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:end:42:10]"));

        assert_eq!(state.milestone(1002), Some(3));
        assert_eq!(state.discovered_count(), 1);
        assert!(state.snapshot_complete());
    }

    #[test]
    fn rejects_forged_party_style_or_malformed_lines_and_never_downgrades_deltas() {
        let mut state = DiscoveryState::default();
        assert!(!receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1002:99]"));
        assert!(!receive(&mut state, "[KORANGAR-PING:v1:prontera 10 10]"));
        assert!(!state.receive_server_line("[KORANGAR-DISCOVERY:v1:delta:43:1002:3]", Some(ACCOUNT_ID)));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1002:3]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1002:1]"));
        assert_eq!(state.milestone(1002), Some(3));
    }

    #[test]
    fn empty_snapshot_preserves_monotonic_history_and_duplicate_chunks_are_idempotent() {
        let mut state = DiscoveryState::default();
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1002:1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:begin:42:11:0:0]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:end:42:11]"));
        assert_eq!(state.discovered_count(), 1);
        assert_eq!(state.milestone(1002), Some(1));

        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:begin:42:12:1:1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:chunk:42:12:0:1002=1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:chunk:42:12:0:1002=1]"));
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:end:42:12]"));
        assert_eq!(state.milestone(1002), Some(1));
    }

    #[test]
    fn account_switch_clears_prior_discoveries_and_rejects_old_account_messages() {
        let mut state = DiscoveryState::default();
        assert!(receive(&mut state, "[KORANGAR-DISCOVERY:v1:delta:42:1002:1]"));
        state.set_account_id(43);
        assert_eq!(state.milestone(1002), None);
        assert!(!state.snapshot_complete());
        assert!(!state.receive_server_line("[KORANGAR-DISCOVERY:v1:delta:42:1002:2]", Some(43)));
    }

    #[test]
    fn map_visit_snapshot_is_account_scoped_and_merges_deltas() {
        let mut state = DiscoveryState::default();
        state.set_account_id(ACCOUNT_ID);
        assert!(state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:42:izlude]", Some(ACCOUNT_ID)));
        assert!(state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:begin:42:8:2:1]", Some(ACCOUNT_ID)));
        assert!(state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:chunk:42:8:0:prontera,prt_fild08]", Some(ACCOUNT_ID)));
        assert!(state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:42:geffen]", Some(ACCOUNT_ID)));
        assert!(state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:end:42:8]", Some(ACCOUNT_ID)));
        assert!(state.map_snapshot_complete());
        assert_eq!(state.visited_map_count(), 4);
        assert!(state.visited_map("PRT_FILD08"));
        assert!(!state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:43:payon]", Some(ACCOUNT_ID)));
        state.set_account_id(43);
        assert_eq!(state.visited_map_count(), 0);
        assert!(!state.map_snapshot_complete());
    }

    #[test]
    fn malformed_or_incomplete_map_snapshots_do_not_forge_visits() {
        let mut state = DiscoveryState::default();
        state.set_account_id(ACCOUNT_ID);
        assert!(!state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:42:payon;@warp]", Some(ACCOUNT_ID)));
        assert!(state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:begin:42:9:2:1]", Some(ACCOUNT_ID)));
        assert!(state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:chunk:42:9:0:prontera]", Some(ACCOUNT_ID)));
        assert!(!state.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:end:42:9]", Some(ACCOUNT_ID)));
        assert!(!state.visited_map("prontera"));
    }
}
