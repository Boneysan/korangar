//! Non-custodial crafting commission board (GDD §12.4 / F31, Decision D7).
//!
//! Provides a tracker for player crafting commissions (weapons, armor,
//! potions, arrows). **It is local to this client**: nothing is sent to the
//! server or to other players, so it is the requester's own ledger, not a
//! shared board, until a server half exists. Adheres strictly to Decision D7:
//! NO automated escrow or asset custody. Materials and compensation
//! Zeny must be transferred directly via peer-to-peer trade windows.
//! All requests explicitly disclose crafting failure risks.

use ragnarok_packets::ItemId;

pub const NON_CUSTODIAL_RISK_DISCLOSURE: &str = "Non-custodial: Materials & payment must be traded directly. No automated escrow. \
                                                 Crafting failure risk borne by requester unless negotiated.";

/// Status of a commission request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CommissionStatus {
    #[default]
    Open,
    Assigned,
    Completed,
    Cancelled,
}

impl CommissionStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Assigned => "In Progress",
            Self::Completed => "Completed",
            Self::Cancelled => "Cancelled",
        }
    }
}

/// A single commission board request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommissionRequest {
    pub id: u32,
    pub requester_name: String,
    pub requested_item_id: ItemId,
    pub requested_item_name: String,
    pub materials_summary: String,
    pub desired_refine: u8,
    pub compensation_zeny: u32,
    pub assigned_crafter: Option<String>,
    pub risk_disclosure: String,
    pub status: CommissionStatus,
}

impl CommissionRequest {
    pub fn format_row(&self) -> String {
        let crafter = self.assigned_crafter.as_deref().unwrap_or("None");
        let refine = if self.desired_refine > 0 {
            format!(" +{}", self.desired_refine)
        } else {
            String::new()
        };
        format!(
            "#{}: {}{} [{}] — Req: {}, Fee: {}z, Crafter: {} (Materials: {})",
            self.id,
            self.requested_item_name,
            refine,
            self.status.label(),
            self.requester_name,
            self.compensation_zeny,
            crafter,
            self.materials_summary
        )
    }
}

/// Board state maintaining open, active, and completed commissions.
#[derive(Clone, Debug, Default)]
pub struct CommissionBoardState {
    requests: Vec<CommissionRequest>,
    next_id: u32,
}

impl CommissionBoardState {
    /// Post a new commission request. Returns the allocated request ID.
    pub fn post_request(
        &mut self,
        requester_name: &str,
        requested_item_id: ItemId,
        requested_item_name: &str,
        materials_summary: &str,
        desired_refine: u8,
        compensation_zeny: u32,
    ) -> u32 {
        self.next_id = self.next_id.saturating_add(1).max(1);
        let id = self.next_id;

        let request = CommissionRequest {
            id,
            requester_name: requester_name.to_owned(),
            requested_item_id,
            requested_item_name: requested_item_name.to_owned(),
            materials_summary: materials_summary.to_owned(),
            desired_refine,
            compensation_zeny,
            assigned_crafter: None,
            risk_disclosure: NON_CUSTODIAL_RISK_DISCLOSURE.to_owned(),
            status: CommissionStatus::Open,
        };

        self.requests.push(request);
        id
    }

    /// Cancel an existing request. Only the original requester may cancel.
    pub fn cancel_request(&mut self, id: u32, requester_name: &str) -> Result<(), &'static str> {
        let req = self
            .requests
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Commission request not found")?;

        if req.status == CommissionStatus::Completed {
            return Err("Cannot cancel a completed commission");
        }
        if req.status == CommissionStatus::Cancelled {
            return Err("Commission is already cancelled");
        }
        if !req.requester_name.eq_ignore_ascii_case(requester_name) {
            return Err("Only the original requester may cancel this commission");
        }

        req.status = CommissionStatus::Cancelled;
        Ok(())
    }

    /// Assign a crafter to a commission request. Only the requester may.
    pub fn assign_crafter(&mut self, id: u32, requester_name: &str, crafter_name: &str) -> Result<(), &'static str> {
        let req = self
            .requests
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Commission request not found")?;

        if !req.requester_name.eq_ignore_ascii_case(requester_name) {
            return Err("Only the original requester may assign a crafter");
        }
        if crafter_name.trim().is_empty() {
            return Err("Name the crafter to assign");
        }
        if req.status != CommissionStatus::Open {
            return Err("Commission is not open for assignment");
        }

        req.assigned_crafter = Some(crafter_name.trim().to_owned());
        req.status = CommissionStatus::Assigned;
        Ok(())
    }

    /// Mark a commission request as completed once the peer trade is done.
    /// Only the requester may: they are the one who received the item.
    pub fn complete_request(&mut self, id: u32, requester_name: &str) -> Result<(), &'static str> {
        let req = self
            .requests
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Commission request not found")?;

        if !req.requester_name.eq_ignore_ascii_case(requester_name) {
            return Err("Only the original requester may mark this commission complete");
        }

        if req.status != CommissionStatus::Assigned && req.status != CommissionStatus::Open {
            return Err("Commission cannot be completed from its current status");
        }

        req.status = CommissionStatus::Completed;
        Ok(())
    }

    /// Get all currently active (open or assigned) commission requests.
    pub fn active_requests(&self) -> Vec<&CommissionRequest> {
        self.requests
            .iter()
            .filter(|r| r.status == CommissionStatus::Open || r.status == CommissionStatus::Assigned)
            .collect()
    }

    /// Get a request by its unique ID.
    #[cfg(test)]
    pub fn get_request(&self, id: u32) -> Option<&CommissionRequest> {
        self.requests.iter().find(|r| r.id == id)
    }

    /// Format summary list of active requests.
    pub fn format_list(&self) -> String {
        let active = self.active_requests();
        if active.is_empty() {
            return "No active commission requests on the board.".to_owned();
        }

        let mut lines = vec!["=== Active Commission Board ===".to_owned()];
        for req in active {
            lines.push(req.format_row());
        }
        lines.push(format!("Note: {NON_CUSTODIAL_RISK_DISCLOSURE}"));
        lines.join("\n")
    }
}

/// What a `/commission` command asks the client to do.
#[derive(Debug, PartialEq, Eq)]
pub enum CommissionReply {
    /// Open or close the board window.
    ToggleWindow,
    /// Print this line in chat.
    Say(String),
}

const USAGE: &str = "Usage: /commission <board | list | post <item> [fee] | assign <id> <crafter> | complete <id> | cancel <id>>";

/// Split `post`'s arguments into item name and fee. Item names have spaces
/// ("Fire Damascus"), so the fee is the last word only when it is all digits
/// and something comes before it.
fn split_item_and_fee(rest: &[&str]) -> (String, u32) {
    match rest.split_last() {
        Some((last, name)) if !name.is_empty() && last.chars().all(|c| c.is_ascii_digit()) => {
            (name.join(" "), last.parse().unwrap_or(u32::MAX))
        }
        _ => (rest.join(" "), 0),
    }
}

impl CommissionBoardState {
    /// Run one `/commission` command for `player_name`. `args` is everything
    /// after `/commission`.
    pub fn run_command(&mut self, player_name: &str, args: &str) -> CommissionReply {
        let words: Vec<&str> = args.split_whitespace().collect();
        let id = |index: usize| words.get(index).and_then(|word| word.trim_start_matches('#').parse::<u32>().ok());
        let say = CommissionReply::Say;

        match words.first().copied().unwrap_or("board") {
            "board" | "window" | "open" => CommissionReply::ToggleWindow,
            "list" => say(self.format_list()),
            "post" => {
                let (item_name, fee) = split_item_and_fee(&words[1..]);
                if item_name.is_empty() {
                    return say("Usage: /commission post <item name> [zeny fee]".to_owned());
                }
                let id = self.post_request(player_name, ItemId(0), &item_name, "Negotiated via peer trade", 0, fee);
                say(format!(
                    "Posted commission #{id} for {item_name} (fee {fee}z). It is on this client only: tell crafters yourself, and trade \
                     items and zeny directly."
                ))
            }
            "assign" => match id(1) {
                Some(id) => {
                    let crafter = words[2..].join(" ");
                    match self.assign_crafter(id, player_name, &crafter) {
                        Ok(()) => say(format!("Commission #{id} is now with {crafter}.")),
                        Err(error) => say(format!("Cannot assign commission #{id}: {error}.")),
                    }
                }
                None => say("Usage: /commission assign <id> <crafter name>".to_owned()),
            },
            "complete" | "done" => match id(1) {
                Some(id) => match self.complete_request(id, player_name) {
                    Ok(()) => say(format!("Commission #{id} marked complete.")),
                    Err(error) => say(format!("Cannot complete commission #{id}: {error}.")),
                },
                None => say("Usage: /commission complete <id>".to_owned()),
            },
            "cancel" => match id(1) {
                Some(id) => match self.cancel_request(id, player_name) {
                    Ok(()) => say(format!("Cancelled commission #{id}.")),
                    Err(error) => say(format!("Cannot cancel commission #{id}: {error}.")),
                },
                None => say("Usage: /commission cancel <id>".to_owned()),
            },
            _ => say(USAGE.to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commission_board_lifecycle_and_non_custodial_disclosure() {
        let mut board = CommissionBoardState::default();

        let id = board.post_request(
            "Alice",
            ItemId(1201), // Knife
            "Fire Stiletto +5",
            "Steel x5, Flame Heart x1 provided",
            5,
            50_000,
        );

        assert_eq!(id, 1);
        let req = board.get_request(id).expect("request exists");
        assert_eq!(req.status, CommissionStatus::Open);
        assert_eq!(req.requester_name, "Alice");
        assert_eq!(req.compensation_zeny, 50_000);
        assert_eq!(req.risk_disclosure, NON_CUSTODIAL_RISK_DISCLOSURE);

        // Assign crafter
        assert!(board.assign_crafter(id, "Alice", "BobTheSmith").is_ok());
        let assigned = board.get_request(id).unwrap();
        assert_eq!(assigned.status, CommissionStatus::Assigned);
        assert_eq!(assigned.assigned_crafter.as_deref(), Some("BobTheSmith"));

        // Complete commission
        assert!(board.complete_request(id, "Alice").is_ok());
        let completed = board.get_request(id).unwrap();
        assert_eq!(completed.status, CommissionStatus::Completed);
        assert!(board.active_requests().is_empty());
    }

    #[test]
    fn cancellation_requires_requester_authorization() {
        let mut board = CommissionBoardState::default();

        let id = board.post_request("Alice", ItemId(501), "Red Potion x100", "Herb x100 provided", 0, 10_000);

        // Unauthorized cancel fails
        assert_eq!(
            board.cancel_request(id, "Mallory"),
            Err("Only the original requester may cancel this commission")
        );

        // Requester cancel succeeds
        assert!(board.cancel_request(id, "Alice").is_ok());
        assert_eq!(board.get_request(id).unwrap().status, CommissionStatus::Cancelled);

        // Double cancel fails
        assert_eq!(board.cancel_request(id, "Alice"), Err("Commission is already cancelled"));
    }

    fn reply(board: &mut CommissionBoardState, player: &str, args: &str) -> String {
        match board.run_command(player, args) {
            CommissionReply::Say(line) => line,
            CommissionReply::ToggleWindow => "<window>".to_owned(),
        }
    }

    #[test]
    fn post_keeps_multi_word_item_names_and_the_fee() {
        let mut board = CommissionBoardState::default();
        reply(&mut board, "Alice", "post Fire Damascus 50000");
        let request = board.get_request(1).expect("posted");
        assert_eq!(request.requested_item_name, "Fire Damascus");
        assert_eq!(request.compensation_zeny, 50_000);

        // No fee: every word is the name. A lone number is a name, not a fee.
        reply(&mut board, "Alice", "post White Slim Potion");
        assert_eq!(board.get_request(2).unwrap().requested_item_name, "White Slim Potion");
        assert_eq!(board.get_request(2).unwrap().compensation_zeny, 0);
        reply(&mut board, "Alice", "post 100");
        assert_eq!(board.get_request(3).unwrap().requested_item_name, "100");
        assert_eq!(board.get_request(3).unwrap().compensation_zeny, 0);
        assert!(reply(&mut board, "Alice", "post").starts_with("Usage"));
    }

    #[test]
    fn assign_then_complete_through_commands() {
        let mut board = CommissionBoardState::default();
        reply(&mut board, "Alice", "post Fire Damascus 50000");

        assert_eq!(
            reply(&mut board, "Alice", "assign 1 Bob the Smith"),
            "Commission #1 is now with Bob the Smith."
        );
        let request = board.get_request(1).unwrap();
        assert_eq!(request.status, CommissionStatus::Assigned);
        assert_eq!(request.assigned_crafter.as_deref(), Some("Bob the Smith"));
        assert!(board.format_list().contains("In Progress"), "{}", board.format_list());

        // "#1" is accepted as well as "1", since that is how the list prints ids.
        assert_eq!(reply(&mut board, "Alice", "complete #1"), "Commission #1 marked complete.");
        assert_eq!(board.get_request(1).unwrap().status, CommissionStatus::Completed);
        assert!(board.active_requests().is_empty());
    }

    #[test]
    fn only_the_requester_assigns_or_completes() {
        let mut board = CommissionBoardState::default();
        reply(&mut board, "Alice", "post Red Potion 100");

        assert!(reply(&mut board, "Mallory", "assign 1 Mallory").contains("Only the original requester"));
        assert!(reply(&mut board, "Mallory", "complete 1").contains("Only the original requester"));
        assert_eq!(board.get_request(1).unwrap().status, CommissionStatus::Open);
    }

    #[test]
    fn assign_refuses_a_missing_crafter_and_a_closed_request() {
        let mut board = CommissionBoardState::default();
        reply(&mut board, "Alice", "post Red Potion");

        assert!(reply(&mut board, "Alice", "assign 1").contains("Name the crafter"));
        assert!(reply(&mut board, "Alice", "assign").starts_with("Usage"));
        assert!(reply(&mut board, "Alice", "assign 9 Bob").contains("not found"));
        reply(&mut board, "Alice", "cancel 1");
        assert!(reply(&mut board, "Alice", "assign 1 Bob").contains("not open"));
        assert!(reply(&mut board, "Alice", "complete 1").contains("cannot be completed"));
    }

    #[test]
    fn bare_command_opens_the_window_and_unknown_ones_explain() {
        let mut board = CommissionBoardState::default();
        assert_eq!(board.run_command("Alice", ""), CommissionReply::ToggleWindow);
        assert!(reply(&mut board, "Alice", "frobnicate").starts_with("Usage"));
    }
}
