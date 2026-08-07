//! Official `data\msgstringtable.txt` (0-based ids used by `ZC_MSG` /
//! `ZC_MSG_COLOR`).
//!
//! Each line ends with `#`. Index 0 is the first line. Hercules `enum
//! clif_messages` values match these indices for main-client msg ids — **but
//! only for the client build the table came from**, which is not the one we
//! ship. Measured 2026-08-07 against Hercules' own documented glosses: of 4006
//! ids, **1614 disagree and 433 are missing** (the table is 3577 lines, the ids
//! run past 4000). Most of that is harmless rewording, but the drift is not
//! uniform, and where a region is off by a line the text is wrong rather than
//! merely different — `MSG_SKILL_SUCCESS` resolved to "Item does not exist."
//! and `MSG_SKILL_FAIL` to "Successful.", i.e. **inverted**.
//!
//! So the server's own header wins: `hercules_messages.tsv` is generated from
//! `messages_main.h` by `tools/generate_message_table.py`, pairing each id with
//! the gloss Hercules documents for it. The shipped table stays the fallback for
//! ids Hercules glosses only in Korean, where it is at least in the player's
//! language.

use korangar_loaders::FileLoader;

use crate::loaders::GameFileLoader;

/// Generated from the server's `messages_main.h` — see the module comment.
const HERCULES_MESSAGES: &str = include_str!("hercules_messages.tsv");

/// Lookup table for server message ids from msgstringtable.txt.
#[derive(Debug, Default, Clone)]
pub struct MsgStringTable {
    lines: Vec<String>,
}

impl MsgStringTable {
    pub fn load(game_file_loader: &GameFileLoader) -> Self {
        let Some(data) = load_msgstringtable_bytes(game_file_loader) else {
            #[cfg(feature = "debug")]
            {
                use korangar_debug::logging::{Colorize, print_debug};
                print_debug!(
                    "[{}] msgstringtable.txt not found; ZC_MSG ids fall back to hardcoded/generic text",
                    "warning".yellow()
                );
            }
            return Self::default();
        };

        let table = parse_msgstringtable(&data);
        #[cfg(feature = "debug")]
        {
            use korangar_debug::logging::print_debug;
            print_debug!("loaded msgstringtable: {} entries", table.lines.len());
        }
        table
    }

    /// Resolve a 0-based msgstringtable id to display text.
    pub fn get(&self, message_id: u16) -> Option<&str> {
        self.lines.get(message_id as usize).map(String::as_str)
    }

    /// Resolve with hardcoded fallbacks for common Hercules ids when the table
    /// is missing or incomplete.
    pub fn resolve(&self, message_id: u16) -> String {
        // Hand-written wording first — these few were checked against what the
        // player actually sees. Then the server's own gloss, because the server
        // chose this id from that header. The shipped table is last: it belongs
        // to a different client build and is wrong often enough to matter.
        if let Some(text) = curated_message(message_id) {
            return text.to_owned();
        }
        if let Some(text) = hercules_message(message_id) {
            return text.to_owned();
        }
        if let Some(text) = self.get(message_id) {
            if !text.is_empty() {
                return text.to_owned();
            }
        }
        format!("Server message #{message_id} (see msgstringtable).")
    }
}

fn load_msgstringtable_bytes(game_file_loader: &GameFileLoader) -> Option<Vec<u8>> {
    const GAME_PATHS: &[&str] = &[
        "data\\msgstringtable.txt",
        "data\\MsgStringTable.txt",
        "System\\msgstringtable.txt",
        "system\\msgstringtable.txt",
    ];
    for path in GAME_PATHS {
        if let Ok(data) = game_file_loader.get(path) {
            return Some(data);
        }
    }

    const FS_PATHS: &[&str] = &[
        "archive/data/msgstringtable.txt",
        "data/msgstringtable.txt",
        "System/msgstringtable.txt",
    ];
    for path in FS_PATHS {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }

    None
}

fn parse_msgstringtable(data: &[u8]) -> MsgStringTable {
    // Official files are often EUC-KR; English tables are UTF-8/ASCII.
    let text = match std::str::from_utf8(data) {
        Ok(s) => s.to_owned(),
        Err(_) => encoding_rs::EUC_KR.decode(data).0.into_owned(),
    };

    let lines = text
        .lines()
        .map(|line| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            // Entries are written as `text#` (sometimes empty `#` only).
            let line = line.strip_suffix('#').unwrap_or(line);
            line.to_owned()
        })
        .collect();

    MsgStringTable { lines }
}

/// The gloss Hercules documents for this id, or `None` when it documents only
/// Korean text (those are left to the shipped table).
fn hercules_message(message_id: u16) -> Option<&'static str> {
    HERCULES_MESSAGES.lines().find_map(|line| {
        let (id, text) = line.split_once('\t')?;
        (id.parse::<u16>().ok()? == message_id).then_some(text)
    })
}

/// Wording checked by hand for ids the player meets often. Takes precedence
/// over both generated sources.
fn curated_message(message_id: u16) -> Option<&'static str> {
    match message_id {
        // Need Basic Skill / similar (legacy mapping).
        164 => Some("You need to learn the basic skills first."),
        // MSG_BUSY — NPC click while already in a dialog.
        0x783 => Some("You are currently busy. Close the open dialog first."),
        // Attendance system (sent on login when no event is active).
        0xD90 => Some("Attendance check failed. Please try again."),
        0xD91 => Some("Attendance check"),
        0xD92 => Some("Currently there is no attendance check event."),
        0xD8E => Some("D-day"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_zero_based_attendance_not_event() {
        // Build a tiny table with enough lines to reach 0xd92 (3474).
        let mut lines = vec!["first#".to_owned(); 3475];
        lines[3474] = "Currently there is no attendance check event.#".to_owned();
        let raw = lines.join("\n");
        let table = parse_msgstringtable(raw.as_bytes());
        assert_eq!(table.resolve(0xD92), "Currently there is no attendance check event.");
        // Id 119 is one Hercules does not gloss, so the shipped table answers it
        // and this still exercises the 0-based lookup.
        assert_eq!(table.resolve(119), "first");
    }

    #[test]
    fn hardcoded_attendance_when_empty() {
        let table = MsgStringTable::default();
        assert_eq!(table.resolve(0xD92), "Currently there is no attendance check event.");
    }

    /// The shipped msgstringtable is a *different client build's*, and where it
    /// drifts by a line the text is not merely different but wrong. These two
    /// were **inverted**: a successful production skill reported "Item does not
    /// exist." and a failed one reported "Successful.".
    #[test]
    fn production_skill_results_are_not_inverted() {
        // A table deliberately shifted the way the shipped one is.
        let mut lines = vec![String::new(); 1600];
        lines[1574] = "Item does not exist.#".to_owned();
        lines[1575] = "Successful.#".to_owned();
        let table = parse_msgstringtable(lines.join("\n").as_bytes());

        assert_eq!(table.resolve(0x626), "Successful."); // MSG_SKILL_SUCCESS
        assert_eq!(table.resolve(0x627), "Failed."); // MSG_SKILL_FAIL
    }

    /// Ids Hercules glosses only in Korean must still fall through to the
    /// shipped table rather than going blank.
    #[test]
    fn shipped_table_still_answers_ids_hercules_does_not_gloss() {
        assert!(hercules_message(0xFFFF).is_none());
        let mut lines = vec![String::new(); 3475];
        lines[3474] = "Currently there is no attendance check event.#".to_owned();
        let table = parse_msgstringtable(lines.join("\n").as_bytes());
        assert_eq!(table.resolve(0xD92), "Currently there is no attendance check event.");
    }
}
