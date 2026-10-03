//! Player↔player trade state.

use korangar_interface::element::StateElement;
use ragnarok_packets::{CharacterId, ClientTick, InventoryIndex, ItemId};
use rust_state::RustState;

/// The shortest time between the partner's last change and our Confirm. A
/// change that lands a moment before the click was almost certainly not read.
pub const CONFIRM_GUARD_MS: u32 = 1_500;

/// What pressing Confirm should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfirmDecision {
    /// Send the commit.
    Send,
    /// Do not send; tell the player why.
    Refuse(String),
}

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
    /// The partner changed their offer after we locked ours. Hercules only
    /// stops a side from adding once *that* side has locked, so the partner
    /// can still add items or lower their zeny after we lock, and our lock
    /// stays.
    #[hidden_element]
    partner_changed_after_our_lock: bool,
    /// We were told about that change and pressed Confirm again.
    #[hidden_element]
    change_acknowledged: bool,
    #[hidden_element]
    last_partner_change: Option<ClientTick>,
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

    #[allow(dead_code)]
    pub fn has_pending(&self) -> bool {
        self.pending_character_id.is_some()
    }

    pub fn pending_name(&self) -> &str {
        &self.pending_name
    }

    #[allow(dead_code)]
    pub fn request_text(&self) -> &str {
        &self.request_text
    }

    fn rebuild_request_text(&mut self) {
        self.request_text = match self.pending_name.is_empty() {
            true => "A player wants to trade with you.".to_owned(),
            false => format!(
                "^000001{}^000000 (Lv{}) wants to trade with you.",
                self.pending_name, self.pending_base_level
            ),
        };
    }

    #[allow(dead_code)]
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
        self.partner_changed_after_our_lock = false;
        self.change_acknowledged = false;
        self.last_partner_change = None;
        self.rebuild_display();
    }

    /// `name` comes from the caller because the item tables live in `Library`,
    /// which the state layer does not hold. `None` means the tables could not
    /// name the item and the label falls back to its id.
    pub fn add_partner_item(&mut self, item_id: ItemId, amount: u32, identified: bool, refine: u8, name: Option<&str>, now: ClientTick) {
        // Zeny rides the item packet as item id 0 (`clif_tradeadditem` leaves the id
        // zeroed for index 0), and `amount` is the NEW TOTAL: the server sets
        // `deal.zeny = amount`. It is not an item and does not accumulate.
        if item_id.0 == 0 {
            self.set_partner_zeny(amount, now);
            return;
        }
        self.note_partner_change(now);
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

    pub fn set_partner_zeny(&mut self, zeny: u32, now: ClientTick) {
        if self.partner_zeny != zeny {
            self.note_partner_change(now);
        }
        self.partner_zeny = zeny;
        self.rebuild_display();
    }

    #[cfg(test)]
    pub fn partner_zeny(&self) -> u32 {
        self.partner_zeny
    }

    fn note_partner_change(&mut self, now: ClientTick) {
        self.last_partner_change = Some(now);
        if self.we_locked {
            self.partner_changed_after_our_lock = true;
            self.change_acknowledged = false;
        }
    }

    /// Whether pressing Confirm should send the commit now.
    ///
    /// Refused when we have not locked, when the partner has not locked, when
    /// the partner's offer changed within [`CONFIRM_GUARD_MS`], and, once,
    /// when it changed after we locked: that first press only acknowledges
    /// the change (the window already says so) and the next press goes
    /// through.
    pub fn confirm_decision(&mut self, now: ClientTick) -> ConfirmDecision {
        if !self.we_locked {
            return ConfirmDecision::Refuse("Lock your offer first.".to_owned());
        }
        if !self.they_locked {
            return ConfirmDecision::Refuse("Wait until they have locked their offer.".to_owned());
        }
        if let Some(changed) = self.last_partner_change
            && now.0.wrapping_sub(changed.0) < CONFIRM_GUARD_MS
        {
            return ConfirmDecision::Refuse("Their offer just changed. Read it, then confirm.".to_owned());
        }
        if self.partner_changed_after_our_lock && !self.change_acknowledged {
            self.change_acknowledged = true;
            self.rebuild_display();
            return ConfirmDecision::Refuse(
                "Their offer changed after you locked. Check the final summary, then press Confirm again.".to_owned(),
            );
        }
        ConfirmDecision::Send
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

    /// Claim the amount for an acked add. Returns `None` if we have no record
    /// of it, which the caller falls back from rather than dropping the
    /// item.
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

    pub fn our_zeny(&self) -> u32 {
        self.our_zeny
    }

    pub fn lock_side(&mut self, who: u8) {
        if who == 0 {
            self.we_locked = true;
            // What counts as "changed after we locked" starts now.
            self.partner_changed_after_our_lock = false;
            self.change_acknowledged = false;
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
        if self.pending_character_id.is_some() {
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
        let mut lines = Vec::new();
        if self.partner_changed_after_our_lock {
            lines.push(format!(
                "!! {} CHANGED their offer after you locked yours. Read it again before you confirm.",
                self.partner_name
            ));
        }
        lines.extend([
            format!("Trading with {} (Lv{})", self.partner_name, self.partner_base_level),
            format!(
                "Lock: you={}  them={}",
                if self.we_locked { "yes" } else { "no" },
                if self.they_locked { "yes" } else { "no" }
            ),
            format!("Your zeny: {}", self.our_zeny),
            "Your items:".to_owned(),
        ]);
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
        if self.we_locked && self.they_locked {
            lines.push(String::new());
            lines.push("FINAL - if you confirm:".to_owned());
            lines.push(format!("  You give:    {}", offer_summary(self.our_zeny, &self.our_items)));
            lines.push(format!(
                "  You receive: {}",
                offer_summary(self.partner_zeny, &self.partner_items)
            ));
        } else if self.we_locked {
            lines.push(String::new());
            lines.push("Your offer is locked. Waiting for them to lock.".to_owned());
        }
        self.display_text = lines.join("\n");
    }
}

/// "5,000 zeny, Red Potion x1" or "nothing": one side of the final summary.
fn offer_summary(zeny: u32, items: &[TradeOfferItem]) -> String {
    let mut parts = Vec::new();
    if zeny > 0 {
        parts.push(format!("{zeny} zeny"));
    }
    parts.extend(items.iter().map(|item| item.label.clone()));
    if parts.is_empty() { "nothing".to_owned() } else { parts.join(", ") }
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
    /// here leaves a phantom item in the inventory with **nothing in any log**
    /// — and a stale count then reads as though unrelated trades
    /// transferred.
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
        state.add_partner_item(ItemId(1301), 1, true, 0, Some("Axe"), ClientTick(0));
        assert_eq!(state.partner_items[0].inventory_index, None);
    }

    fn open() -> TradeState {
        let mut state = TradeState::default();
        state.open_with_partner("Alice".into(), CharacterId(1), 50);
        state
    }

    fn tick(ms: u32) -> ClientTick {
        ClientTick(ms)
    }

    /// Zeny arrives as an item with id 0 whose amount is the NEW TOTAL (the
    /// server sets `deal.zeny = amount`). It used to be listed as "item #0 xN"
    /// while "Their zeny" stayed at 0.
    #[test]
    fn the_partners_zeny_is_zeny_not_an_item_and_each_offer_replaces_the_last() {
        let mut state = open();
        state.add_partner_item(ItemId(0), 5_000, true, 0, None, tick(100));
        assert_eq!(state.partner_zeny(), 5_000);
        assert!(state.partner_items.is_empty(), "zeny must not become an item row");
        assert!(state.display_text().contains("Their zeny: 5000"));

        // Raising it replaces; it does not add up to 8000.
        state.add_partner_item(ItemId(0), 3_000, true, 0, None, tick(200));
        assert_eq!(state.partner_zeny(), 3_000);
        assert!(state.display_text().contains("Their zeny: 3000"));
    }

    #[test]
    fn a_real_item_is_still_listed() {
        let mut state = open();
        state.add_partner_item(ItemId(1301), 1, true, 0, Some("Axe"), tick(0));
        assert_eq!(state.partner_items.len(), 1);
        assert_eq!(state.partner_zeny(), 0);
    }

    #[test]
    fn confirm_needs_both_sides_locked() {
        let mut state = open();
        assert!(matches!(state.confirm_decision(tick(10_000)), ConfirmDecision::Refuse(reason) if reason.contains("Lock your offer")));
        state.lock_side(0);
        assert!(matches!(state.confirm_decision(tick(10_000)), ConfirmDecision::Refuse(reason) if reason.contains("they have locked")));
        state.lock_side(1);
        assert_eq!(state.confirm_decision(tick(10_000)), ConfirmDecision::Send);
    }

    /// The scam this exists for: we lock, then the partner lowers their zeny.
    #[test]
    fn a_change_after_our_lock_is_flagged_and_needs_a_second_press() {
        let mut state = open();
        state.add_partner_item(ItemId(0), 50_000, true, 0, None, tick(0));
        state.lock_side(0);
        assert!(!state.display_text().contains("CHANGED"), "nothing has changed since we locked");

        state.add_partner_item(ItemId(0), 1, true, 0, None, tick(5_000));
        state.lock_side(1);
        assert!(state.display_text().contains("CHANGED their offer after you locked"));

        // First press: refused, explains, and counts as having been told.
        match state.confirm_decision(tick(20_000)) {
            ConfirmDecision::Refuse(reason) => assert!(reason.contains("changed after you locked"), "{reason}"),
            other => panic!("expected a refusal, got {other:?}"),
        }
        // Second press: goes through.
        assert_eq!(state.confirm_decision(tick(21_000)), ConfirmDecision::Send);
    }

    /// A change before we locked is just negotiation, not a trap.
    #[test]
    fn a_change_before_our_lock_is_not_flagged() {
        let mut state = open();
        state.add_partner_item(ItemId(0), 10, true, 0, None, tick(0));
        state.add_partner_item(ItemId(0), 20, true, 0, None, tick(1_000));
        state.lock_side(0);
        state.lock_side(1);
        assert!(!state.display_text().contains("CHANGED"));
        assert_eq!(state.confirm_decision(tick(30_000)), ConfirmDecision::Send);
    }

    /// Being told once is not a blank cheque: a further change asks again.
    #[test]
    fn a_new_change_after_acknowledging_asks_again() {
        let mut state = open();
        state.lock_side(0);
        state.add_partner_item(ItemId(0), 100, true, 0, None, tick(0));
        state.lock_side(1);
        assert!(matches!(state.confirm_decision(tick(10_000)), ConfirmDecision::Refuse(_)));
        state.add_partner_item(ItemId(0), 1, true, 0, None, tick(11_000));
        assert!(
            matches!(state.confirm_decision(tick(20_000)), ConfirmDecision::Refuse(_)),
            "the second change must be acknowledged too"
        );
        assert_eq!(state.confirm_decision(tick(21_000)), ConfirmDecision::Send);
    }

    /// A change that lands a moment before the click was not read, acknowledged
    /// or not.
    #[test]
    fn a_change_within_the_guard_window_is_refused_even_after_acknowledging() {
        let mut state = open();
        state.lock_side(0);
        state.add_partner_item(ItemId(0), 100, true, 0, None, tick(1_000));
        state.lock_side(1);
        assert!(matches!(state.confirm_decision(tick(10_000)), ConfirmDecision::Refuse(_))); // acknowledges
        state.add_partner_item(ItemId(0), 50, true, 0, None, tick(10_500));
        // 600 ms after the change: still inside the window.
        assert!(matches!(state.confirm_decision(tick(11_100)), ConfirmDecision::Refuse(reason) if reason.contains("just changed")));
        assert!(
            matches!(
                state.confirm_decision(tick(10_500 + CONFIRM_GUARD_MS)),
                ConfirmDecision::Refuse(_)
            ),
            "acknowledges the second change"
        );
        assert_eq!(
            state.confirm_decision(tick(10_500 + CONFIRM_GUARD_MS + 1)),
            ConfirmDecision::Send
        );
    }

    /// The tick counter wraps; a change just before the wrap must still count
    /// as recent.
    #[test]
    fn the_guard_window_survives_tick_wraparound() {
        let mut state = open();
        state.add_partner_item(ItemId(0), 100, true, 0, None, tick(u32::MAX - 100));
        state.lock_side(0);
        state.lock_side(1);
        // 200 ms later, across the wrap.
        assert!(matches!(state.confirm_decision(tick(99)), ConfirmDecision::Refuse(reason) if reason.contains("just changed")));
    }

    /// The mirror case: a change made long BEFORE the wrap must not keep
    /// blocking Confirm afterwards. A saturating subtraction reads it as
    /// "just now" forever.
    #[test]
    fn an_old_change_does_not_block_confirm_after_the_tick_wraps() {
        let mut state = open();
        state.add_partner_item(ItemId(0), 100, true, 0, None, tick(u32::MAX - 10_000));
        state.lock_side(0);
        state.lock_side(1);
        // 15 seconds later, across the wrap.
        assert_eq!(state.confirm_decision(tick(5_000)), ConfirmDecision::Send);
    }

    /// When both sides are locked the window states exactly what changes hands.
    #[test]
    fn the_final_summary_lists_what_you_give_and_receive() {
        let mut state = open();
        state.set_our_zeny(2_000);
        state.note_our_item(InventoryIndex(5), ItemId(501), 3, "Red Potion x3".into());
        state.add_partner_item(ItemId(0), 7_500, true, 0, None, tick(0));
        state.add_partner_item(ItemId(1301), 1, true, 0, Some("Axe"), tick(0));
        assert!(!state.display_text().contains("FINAL"), "no summary until both lock");
        state.lock_side(0);
        assert!(state.display_text().contains("Waiting for them to lock"));
        state.lock_side(1);
        let text = state.display_text();
        assert!(text.contains("FINAL"), "{text}");
        assert!(text.contains("You give:    2000 zeny, Red Potion x3"), "{text}");
        assert!(text.contains("You receive: 7500 zeny, Axe x1"), "{text}");
    }

    #[test]
    fn an_empty_side_reads_as_nothing() {
        let mut state = open();
        state.lock_side(0);
        state.lock_side(1);
        let text = state.display_text();
        assert!(
            text.contains("You give:    nothing") && text.contains("You receive: nothing"),
            "{text}"
        );
    }

    /// A new trade must not inherit the last one's warning.
    #[test]
    fn opening_a_new_trade_clears_the_change_flags() {
        let mut state = open();
        state.lock_side(0);
        state.add_partner_item(ItemId(0), 1, true, 0, None, tick(0));
        state.open_with_partner("Bob".into(), CharacterId(2), 60);
        state.lock_side(0);
        state.lock_side(1);
        assert!(!state.display_text().contains("CHANGED"));
        assert_eq!(state.confirm_decision(tick(10_000)), ConfirmDecision::Send);
    }
}
