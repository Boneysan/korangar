//! Player↔player trade state.

use korangar_interface::element::StateElement;
use ragnarok_packets::{CharacterId, InventoryIndex, ItemId};
use rust_state::RustState;

#[derive(Clone, Debug, RustState, StateElement)]
pub struct TradeOfferItem {
    /// The slot the item came out of, for our own offers; `None` for the
    /// partner's, whose slots are on their client and mean nothing here.
    /// Needed because a completed trade has to remove the item from our
    /// inventory locally -- see `TradeState::our_items`. Item id alone is
    /// ambiguous: two identical stacks are indistinguishable.
    pub inventory_index: Option<InventoryIndex>,
    pub item_id: ItemId,
    pub amount: u32,
    pub identified: bool,
    pub refine: u8,
    pub label: String,
}

/// An add we have sent but not yet had acked.
///
/// `ZC_ACK_ADD_EXCHANGE_ITEM` carries an index and a result but **no amount**,
/// so the figure we asked for is only knowable from the request. Reading the
/// stack out of the inventory instead is wrong for a partial offer: "trade one"
/// of a stack of twenty would record twenty.
#[derive(Clone, Debug, RustState, StateElement)]
pub struct PendingTradeAdd {
    pub inventory_index: InventoryIndex,
    pub amount: u32,
}

#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct TradeState {
    active: bool,
    partner_name: String,
    partner_character_id: Option<CharacterId>,
    partner_base_level: u16,
    /// Pending invite before we accept.
    pending_name: String,
    pending_character_id: Option<CharacterId>,
    pending_base_level: u16,
    our_zeny: u32,
    partner_zeny: u32,
    our_items: Vec<TradeOfferItem>,
    partner_items: Vec<TradeOfferItem>,
    /// Sent-but-unacked adds, oldest first. Hercules acks each add in order, so
    /// the first entry matching an index is the one that ack belongs to.
    pending_adds: Vec<PendingTradeAdd>,
    we_locked: bool,
    they_locked: bool,
    display_text: String,
    /// Cached "<name> (Lv<n>) wants to trade with you" line for the request
    /// popup. The name and level arrive on `ZC_REQ_EXCHANGE_ITEM` and were
    /// already stored; only the window was ignoring them.
    request_text: String,
}

impl TradeState {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn has_pending(&self) -> bool {
        self.pending_character_id.is_some()
    }

    pub fn pending_name(&self) -> &str {
        &self.pending_name
    }

    pub fn request_text(&self) -> &str {
        &self.request_text
    }

    fn rebuild_request_text(&mut self) {
        self.request_text = match self.pending_name.is_empty() {
            true => "A player wants to trade with you.".to_owned(),
            false => format!("^000001{}^000000 (Lv{}) wants to trade with you.", self.pending_name, self.pending_base_level),
        };
    }

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    pub fn set_pending(&mut self, name: String, character_id: CharacterId, base_level: u16) {
        self.pending_name = name;
        self.pending_character_id = Some(character_id);
        self.pending_base_level = base_level;
        self.rebuild_request_text();
        self.rebuild_display();
    }

    pub fn clear_pending(&mut self) {
        self.pending_name.clear();
        self.pending_character_id = None;
        self.pending_base_level = 0;
        self.rebuild_request_text();
        self.rebuild_display();
    }

    pub fn open_with_partner(&mut self, name: String, character_id: CharacterId, base_level: u16) {
        self.clear_pending();
        self.active = true;
        self.partner_name = name;
        self.partner_character_id = Some(character_id);
        self.partner_base_level = base_level;
        self.our_zeny = 0;
        self.partner_zeny = 0;
        self.our_items.clear();
        self.partner_items.clear();
        self.we_locked = false;
        self.they_locked = false;
        self.rebuild_display();
    }

    /// `name` comes from the caller because the item tables live in `Library`,
    /// which the state layer does not hold. `None` means the tables could not
    /// name the item and the label falls back to its id.
    pub fn add_partner_item(&mut self, item_id: ItemId, amount: u32, identified: bool, refine: u8, name: Option<&str>) {
        let label = crate::trade_item_label(name, item_id, amount, refine);
        self.partner_items.push(TradeOfferItem {
            inventory_index: None,
            item_id,
            amount,
            identified,
            refine,
            label,
        });
        self.rebuild_display();
    }

    pub fn note_our_item(&mut self, inventory_index: InventoryIndex, item_id: ItemId, amount: u32, label: String) {
        self.our_items.push(TradeOfferItem {
            inventory_index: Some(inventory_index),
            item_id,
            amount,
            identified: true,
            refine: 0,
            label,
        });
        self.rebuild_display();
    }

    /// Record an add at send time so its amount survives to the ack.
    pub fn note_pending_add(&mut self, inventory_index: InventoryIndex, amount: u32) {
        self.pending_adds.push(PendingTradeAdd { inventory_index, amount });
    }

    /// Claim the amount for an acked add. Returns `None` if we have no record of
    /// it, which the caller falls back from rather than dropping the item.
    pub fn take_pending_add(&mut self, inventory_index: InventoryIndex) -> Option<u32> {
        let position = self
            .pending_adds
            .iter()
            .position(|pending| pending.inventory_index == inventory_index)?;
        Some(self.pending_adds.remove(position).amount)
    }

    /// What we put into the offer, for removing it from our own inventory once
    /// the trade commits.
    pub fn our_items(&self) -> &[TradeOfferItem] {
        &self.our_items
    }

    pub fn set_our_zeny(&mut self, zeny: u32) {
        self.our_zeny = zeny;
        self.rebuild_display();
    }

    pub fn lock_side(&mut self, who: u8) {
        if who == 0 {
            self.we_locked = true;
        } else {
            self.they_locked = true;
        }
        self.rebuild_display();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
        self.rebuild_display();
    }

    fn rebuild_display(&mut self) {
        if let Some(_) = self.pending_character_id {
            self.display_text = format!(
                "Trade request from {} (Lv{}).\nAccept or reject.",
                self.pending_name, self.pending_base_level
            );
            return;
        }
        if !self.active {
            self.display_text = "No active trade.".to_owned();
            return;
        }
        let mut lines = vec![
            format!("Trading with {} (Lv{})", self.partner_name, self.partner_base_level),
            format!(
                "Lock: you={}  them={}",
                if self.we_locked { "yes" } else { "no" },
                if self.they_locked { "yes" } else { "no" }
            ),
            format!("Your zeny: {}", self.our_zeny),
            "Your items:".to_owned(),
        ];
        if self.our_items.is_empty() {
            lines.push("  (none)".to_owned());
        } else {
            for item in &self.our_items {
                lines.push(format!("  {}", item.label));
            }
        }
        lines.push(format!("Their zeny: {}", self.partner_zeny));
        lines.push("Their items:".to_owned());
        if self.partner_items.is_empty() {
            lines.push("  (none)".to_owned());
        } else {
            for item in &self.partner_items {
                lines.push(format!("  {}", item.label));
            }
        }
        self.display_text = lines.join("\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_then_open() {
        let mut state = TradeState::default();
        state.set_pending("Alice".into(), CharacterId(1), 50);
        assert!(state.has_pending());
        assert!(state.display_text().contains("Alice"));
        // The popup must name the requester, not say "a player".
        assert!(state.request_text().contains("Alice"));
        assert!(state.request_text().contains("Lv50"));
        state.open_with_partner("Alice".into(), CharacterId(1), 50);
        assert!(state.is_active());
        assert!(!state.has_pending());
        // Clearing the pending request must not leave a stale name behind.
        assert!(!state.request_text().contains("Alice"));
        state.lock_side(0);
        state.lock_side(1);
        assert!(state.display_text().contains("you=yes"));
        assert!(state.display_text().contains("them=yes"));
    }

    /// A completed trade has to remove our offered items from the inventory
    /// locally, because Hercules deliberately sends no delete for them
    /// (`trade.c:600` passes `type = 1`, which `pc.c:4960` reads as "do not
    /// notify"). That removal needs the slot and the amount, so losing either
    /// here leaves a phantom item in the inventory with **nothing in any log** —
    /// and a stale count then reads as though unrelated trades transferred.
    #[test]
    fn our_offer_records_the_slot_and_the_amount_actually_offered() {
        let mut state = TradeState::default();
        state.open_with_partner("Alice".into(), CharacterId(1), 99);

        // Offer one out of a stack of twenty.
        state.note_pending_add(InventoryIndex(7), 1);
        let requested = state.take_pending_add(InventoryIndex(7));
        assert_eq!(requested, Some(1), "the amount asked for must survive to the ack");
        state.note_our_item(InventoryIndex(7), ItemId(501), requested.unwrap(), "Red Potion x1".into());

        let ours = state.our_items();
        assert_eq!(ours.len(), 1);
        assert_eq!(
            ours[0].inventory_index,
            Some(InventoryIndex(7)),
            "without the slot, two identical stacks are indistinguishable"
        );
        assert_eq!(ours[0].amount, 1, "the stack count would over-remove on a partial offer");

        // The ack carries no amount, so a claimed add must not be claimable twice.
        assert_eq!(state.take_pending_add(InventoryIndex(7)), None);

        // The partner's slots live on their client and must not be mistaken for ours.
        state.add_partner_item(ItemId(1301), 1, true, 0, Some("Axe"));
        assert_eq!(state.partner_items[0].inventory_index, None);
    }
}
