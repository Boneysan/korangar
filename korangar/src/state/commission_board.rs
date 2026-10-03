//! Non-custodial crafting commission board (GDD §12.4 / F31, Decision D7).
//!
//! Provides a bulletin/matching tracker for player crafting commissions
//! (weapons, armor, potions, arrows). Adheres strictly to Decision D7:
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

    /// Assign an active crafter to a commission request.
    #[cfg_attr(not(test), allow(dead_code))] // the board window has no assign/complete action yet (F31)
    pub fn assign_crafter(&mut self, id: u32, crafter_name: &str) -> Result<(), &'static str> {
        let req = self
            .requests
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Commission request not found")?;

        if req.status != CommissionStatus::Open {
            return Err("Commission is not open for assignment");
        }

        req.assigned_crafter = Some(crafter_name.to_owned());
        req.status = CommissionStatus::Assigned;
        Ok(())
    }

    /// Mark a commission request as completed following verified peer trade.
    #[cfg_attr(not(test), allow(dead_code))] // the board window has no assign/complete action yet (F31)
    pub fn complete_request(&mut self, id: u32) -> Result<(), &'static str> {
        let req = self
            .requests
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Commission request not found")?;

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
        assert!(board.assign_crafter(id, "BobTheSmith").is_ok());
        let assigned = board.get_request(id).unwrap();
        assert_eq!(assigned.status, CommissionStatus::Assigned);
        assert_eq!(assigned.assigned_crafter.as_deref(), Some("BobTheSmith"));

        // Complete commission
        assert!(board.complete_request(id).is_ok());
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
}
