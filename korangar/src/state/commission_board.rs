//! Shared crafting commission board (GDD §12.4 / F31, Decision D7).
//!
//! The board lives on the server (Hercules `npc/custom/commission_board.txt`,
//! table `korangar_commission`): every player sees the same requests, and
//! only a request's poster may assign, complete or cancel it. This client
//! state is a read-only mirror of the list the server last sent, plus the
//! translation from `/commission` to the server's `@commission` command.
//!
//! The list arrives as `[CMB]{json}` lines on the server-message channel
//! (`dispbottom`), the same shape as the campaign's `[DMJ]` lines: `begin`,
//! one `row` per request, then `end`. The mirror is replaced only on `end`,
//! so a list cut off part-way never shows half a board.
//!
//! Non-custodial: materials, the item and the fee move by ordinary player
//! trade. Nothing here, or on the server, holds them.

use serde_json::Value;

pub const NON_CUSTODIAL_RISK_DISCLOSURE: &str = "Non-custodial: materials and payment are traded directly. No escrow. Crafting failure \
                                                 risk is borne by the requester unless negotiated.";

const PREFIX: &str = "[CMB]";
const VERSION: u64 = 1;
/// The server sends at most 50 rows; anything far past that is not ours.
const MAX_ROWS: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommissionStatus {
    Open,
    Assigned,
}

/// One request as the server described it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommissionRow {
    pub id: u32,
    pub item_name: String,
    pub fee: u32,
    pub requester_name: String,
    pub crafter_name: String,
    pub status: CommissionStatus,
    /// The viewing character posted it.
    pub mine: bool,
    /// The viewing character is its assigned crafter.
    pub yours: bool,
}

impl CommissionRow {
    fn from_json(message: &Value) -> Option<Self> {
        let text = |key: &str| message.get(key).and_then(Value::as_str).map(str::to_owned);
        let number = |key: &str| message.get(key).and_then(Value::as_u64).and_then(|value| u32::try_from(value).ok());
        let flag = |key: &str| message.get(key).and_then(Value::as_u64) == Some(1);
        Some(Self {
            id: number("id")?,
            item_name: text("item")?,
            fee: number("fee")?,
            requester_name: text("by")?,
            crafter_name: text("crafter")?,
            status: match message.get("status")?.as_str()? {
                "open" => CommissionStatus::Open,
                "assigned" => CommissionStatus::Assigned,
                _ => return None,
            },
            mine: flag("mine"),
            yours: flag("yours"),
        })
    }

    pub fn format_row(&self) -> String {
        let state = match self.status {
            CommissionStatus::Open => "open".to_owned(),
            CommissionStatus::Assigned => format!("with {}", self.crafter_name),
        };
        let note = match (self.mine, self.yours) {
            (true, _) => " (your request)",
            (_, true) => " (yours to craft)",
            _ => "",
        };
        format!(
            "#{} {} for {}z, by {}, {state}{note}",
            self.id, self.item_name, self.fee, self.requester_name
        )
    }
}

/// The client's mirror of the server's board.
#[derive(Clone, Debug, Default)]
pub struct CommissionBoardState {
    rows: Vec<CommissionRow>,
    /// Rows of a list still arriving; `None` outside `begin` .. `end`.
    incoming: Option<Vec<CommissionRow>>,
    /// Whether any complete list has arrived yet.
    received: bool,
    /// Lines that had the prefix but could not be understood.
    dropped: u32,
}

impl CommissionBoardState {
    /// Take one server-message line. Returns true when the line belongs to
    /// the board (understood or not), so it stays out of chat.
    pub fn receive_server_line(&mut self, text: &str) -> bool {
        let Some(body) = text.strip_prefix(PREFIX) else {
            return false;
        };
        if !self.apply(body) {
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
        match message.get("t").and_then(Value::as_str) {
            Some("begin") => {
                self.incoming = Some(Vec::new());
                true
            }
            Some("row") => match (self.incoming.as_mut(), CommissionRow::from_json(&message)) {
                (Some(rows), Some(row)) if rows.len() < MAX_ROWS => {
                    rows.push(row);
                    true
                }
                _ => false,
            },
            Some("end") => match self.incoming.take() {
                Some(rows) => {
                    self.rows = rows;
                    self.received = true;
                    true
                }
                None => false,
            },
            _ => false,
        }
    }

    #[cfg(test)]
    pub fn rows(&self) -> &[CommissionRow] {
        &self.rows
    }

    /// Forget the board, e.g. on logout: the next character sees its own
    /// `mine` / `yours` flags only after the server sends them.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn format_list(&self) -> String {
        if !self.received {
            return "Loading the board from the server...".to_owned();
        }
        if self.rows.is_empty() {
            return "No open commissions. Post one below.".to_owned();
        }
        // Decision D7: every listed request carries the failure-risk disclosure.
        let mut lines: Vec<String> = self.rows.iter().map(CommissionRow::format_row).collect();
        lines.push(NON_CUSTODIAL_RISK_DISCLOSURE.to_owned());
        lines.join("\n")
    }
}

/// What a `/commission` command asks the client to do.
#[derive(Debug, PartialEq, Eq)]
pub enum CommissionReply {
    /// Open or close the board window, and refresh it from the server.
    ToggleWindow,
    /// Send this `@commission` command to the server.
    Send(String),
    /// Print this line in chat, sending nothing.
    Say(String),
}

const USAGE: &str = "Usage: /commission [post <item> [fee] | assign <id> <crafter> | complete <id> | cancel <id>]";

/// The server command that refreshes the list.
pub const LIST_COMMAND: &str = "@commission list";

/// Split `post`'s arguments into item name and fee. Item names have spaces
/// ("Fire Damascus"), so the fee is the last word only when it is all digits
/// and something comes before it.
fn split_item_and_fee<'a>(rest: &[&'a str]) -> (String, Option<&'a str>) {
    match rest.split_last() {
        Some((last, name)) if !name.is_empty() && last.chars().all(|c| c.is_ascii_digit()) => (name.join(" "), Some(last)),
        _ => (rest.join(" "), None),
    }
}

/// Translate one `/commission` command (`args` is everything after it) into
/// what the server understands. The server decides whether it is allowed.
pub fn commission_command(args: &str) -> CommissionReply {
    let words: Vec<&str> = args.split_whitespace().collect();
    let id = words
        .get(1)
        .map(|word| word.trim_start_matches('#'))
        .filter(|word| !word.is_empty() && word.chars().all(|c| c.is_ascii_digit()));

    match words.first().copied().unwrap_or("board") {
        "board" | "window" | "open" | "list" => CommissionReply::ToggleWindow,
        "refresh" => CommissionReply::Send(LIST_COMMAND.to_owned()),
        "post" => match split_item_and_fee(&words[1..]) {
            (item, _) if item.is_empty() => CommissionReply::Say("Usage: /commission post <item name> [zeny fee]".to_owned()),
            (item, fee) => CommissionReply::Send(format!("@commission post {} {item}", fee.unwrap_or("0"))),
        },
        "assign" => match (id, words.len() > 2) {
            (Some(id), true) => CommissionReply::Send(format!("@commission assign {id} {}", words[2..].join(" "))),
            _ => CommissionReply::Say("Usage: /commission assign <id> <crafter name>".to_owned()),
        },
        action @ ("complete" | "cancel") => match id {
            Some(id) => CommissionReply::Send(format!("@commission {action} {id}")),
            None => CommissionReply::Say(format!("Usage: /commission {action} <id>")),
        },
        _ => CommissionReply::Say(USAGE.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line exactly as `commission_board.txt`'s S_List builds it.
    fn row(id: u32, item: &str, status: &str, mine: u8, yours: u8) -> String {
        format!(
            "[CMB]{{\"t\":\"row\",\"v\":1,\"id\":{id},\"item\":\"{item}\",\"fee\":50000,\"by\":\"Alice\",\"crafter\":\"Bob\",\"status\":\"\
             {status}\",\"mine\":{mine},\"yours\":{yours}}}"
        )
    }

    fn feed(board: &mut CommissionBoardState, lines: &[String]) {
        for line in lines {
            assert!(board.receive_server_line(line), "{line}");
        }
    }

    #[test]
    fn a_complete_list_replaces_the_board() {
        let mut board = CommissionBoardState::default();
        assert!(board.format_list().starts_with("Loading"));
        feed(&mut board, &[
            "[CMB]{\"t\":\"begin\",\"v\":1,\"n\":2}".to_owned(),
            row(3, "Fire Damascus", "assigned", 1, 0),
            row(2, "Red Potion", "open", 0, 0),
            "[CMB]{\"t\":\"end\",\"v\":1}".to_owned(),
        ]);
        assert_eq!(board.rows().len(), 2);
        assert_eq!(board.rows()[0].status, CommissionStatus::Assigned);
        assert_eq!(
            board.format_list(),
            format!(
                "#3 Fire Damascus for 50000z, by Alice, with Bob (your request)\n#2 Red Potion for 50000z, by Alice, \
                 open\n{NON_CUSTODIAL_RISK_DISCLOSURE}"
            )
        );

        // An empty list is a real answer, not "loading".
        feed(&mut board, &[
            "[CMB]{\"t\":\"begin\",\"v\":1,\"n\":0}".to_owned(),
            "[CMB]{\"t\":\"end\",\"v\":1}".to_owned(),
        ]);
        assert!(board.rows().is_empty());
        assert!(board.format_list().starts_with("No open commissions"));
    }

    #[test]
    fn a_list_cut_off_before_end_leaves_the_old_board() {
        let mut board = CommissionBoardState::default();
        feed(&mut board, &[
            "[CMB]{\"t\":\"begin\",\"v\":1,\"n\":1}".to_owned(),
            row(1, "Red Potion", "open", 0, 0),
            "[CMB]{\"t\":\"end\",\"v\":1}".to_owned(),
        ]);
        feed(&mut board, &[
            "[CMB]{\"t\":\"begin\",\"v\":1,\"n\":2}".to_owned(),
            row(5, "Sword", "open", 0, 0),
        ]);
        assert_eq!(board.rows().len(), 1);
        assert_eq!(board.rows()[0].id, 1);
    }

    #[test]
    fn foreign_and_malformed_lines_are_handled_apart() {
        let mut board = CommissionBoardState::default();
        // Ordinary server text is not ours and goes to chat.
        assert!(!board.receive_server_line("Commission #3 posted: Fire Damascus for 50000z."));
        // Ours but unreadable: kept out of chat, counted, board unchanged.
        assert!(board.receive_server_line("[CMB]not json"));
        assert!(board.receive_server_line("[CMB]{\"t\":\"begin\",\"v\":2,\"n\":0}"));
        assert!(board.receive_server_line("[CMB]{\"t\":\"end\",\"v\":1}"));
        assert_eq!(board.dropped, 3);
        assert!(board.format_list().starts_with("Loading"));
    }

    #[test]
    fn escaped_names_survive_the_round_trip() {
        // S_Json escapes `\` and `"` in names; serde must read them back.
        let mut board = CommissionBoardState::default();
        feed(&mut board, &[
            "[CMB]{\"t\":\"begin\",\"v\":1,\"n\":1}".to_owned(),
            "[CMB]{\"t\":\"row\",\"v\":1,\"id\":1,\"item\":\"Sword\",\"fee\":0,\"by\":\"Al\\\"x\\\\\",\"crafter\":\"\",\"status\":\"open\"\
             ,\"mine\":0,\"yours\":0}"
                .to_owned(),
            "[CMB]{\"t\":\"end\",\"v\":1}".to_owned(),
        ]);
        assert_eq!(board.rows()[0].requester_name, "Al\"x\\");
    }

    #[test]
    fn commands_translate_to_the_servers_syntax() {
        let send = |text: &str| CommissionReply::Send(text.to_owned());
        // The server takes the fee first so names keep their spaces.
        assert_eq!(
            commission_command("post Fire Damascus 50000"),
            send("@commission post 50000 Fire Damascus")
        );
        assert_eq!(
            commission_command("post White Slim Potion"),
            send("@commission post 0 White Slim Potion")
        );
        assert_eq!(commission_command("post 100"), send("@commission post 0 100"));
        assert_eq!(
            commission_command("assign #3 Bob the Smith"),
            send("@commission assign 3 Bob the Smith")
        );
        assert_eq!(commission_command("complete 3"), send("@commission complete 3"));
        assert_eq!(commission_command("cancel #3"), send("@commission cancel 3"));
        assert_eq!(commission_command(""), CommissionReply::ToggleWindow);
        assert_eq!(commission_command("list"), CommissionReply::ToggleWindow);
        assert_eq!(commission_command("refresh"), send("@commission list"));
    }

    #[test]
    fn malformed_commands_explain_and_send_nothing() {
        for args in ["post", "assign 3", "assign x Bob", "complete", "cancel abc", "frobnicate"] {
            assert!(
                matches!(commission_command(args), CommissionReply::Say(ref line) if line.starts_with("Usage")),
                "{args}"
            );
        }
    }
}
