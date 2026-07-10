//! Official `data\msgstringtable.txt` (0-based ids used by `ZC_MSG` / `ZC_MSG_COLOR`).
//!
//! Each line ends with `#`. Index 0 is the first line. Hercules `enum clif_messages`
//! values match these indices for main-client msg ids.

use korangar_loaders::FileLoader;

use crate::loaders::GameFileLoader;

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
        if let Some(text) = self.get(message_id) {
            if !text.is_empty() {
                return text.to_owned();
            }
        }
        hardcoded_message(message_id)
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

/// Fallback when msgstringtable is not installed.
fn hardcoded_message(message_id: u16) -> String {
    match message_id {
        // Need Basic Skill / similar (legacy mapping).
        164 => "You need to learn the basic skills first.".to_owned(),
        // MSG_BUSY — NPC click while already in a dialog.
        0x783 => "You are currently busy. Close the open dialog first.".to_owned(),
        // Attendance system (sent on login when no event is active).
        // MSG_CHECK_ATTENDANCE_FAIL_RESTART
        0xd90 => "Attendance check failed. Please try again.".to_owned(),
        // MSG_CHECK_ATTENDANCE
        0xd91 => "Attendance check".to_owned(),
        // MSG_CHECK_ATTENDANCE_NOT_EVENT — common boot message.
        0xd92 => "Currently there is no attendance check event.".to_owned(),
        // MSG_CHECK_ATTENDANCE_DDAY
        0xd8e => "D-day".to_owned(),
        _ => format!("Server message #{message_id} (see msgstringtable)."),
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
        assert_eq!(
            table.resolve(0xd92),
            "Currently there is no attendance check event."
        );
        assert_eq!(table.resolve(0), "first");
    }

    #[test]
    fn hardcoded_attendance_when_empty() {
        let table = MsgStringTable::default();
        assert_eq!(
            table.resolve(0xd92),
            "Currently there is no attendance check event."
        );
    }
}
