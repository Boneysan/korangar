//! Player↔player trade state.

use korangar_interface::element::StateElement;
use ragnarok_packets::{CharacterId, ItemId};
use rust_state::RustState;

#[derive(Clone, Debug, RustState, StateElement)]
pub struct TradeOfferItem {
    pub item_id: ItemId,
    pub amount: u32,
    pub identified: bool,
    pub refine: u8,
    pub label: String,
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
    we_locked: bool,
    they_locked: bool,
    display_text: String,
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

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    pub fn set_pending(&mut self, name: String, character_id: CharacterId, base_level: u16) {
        self.pending_name = name;
        self.pending_character_id = Some(character_id);
        self.pending_base_level = base_level;
        self.rebuild_display();
    }

    pub fn clear_pending(&mut self) {
        self.pending_name.clear();
        self.pending_character_id = None;
        self.pending_base_level = 0;
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
            item_id,
            amount,
            identified,
            refine,
            label,
        });
        self.rebuild_display();
    }

    pub fn note_our_item(&mut self, item_id: ItemId, amount: u32, label: String) {
        self.our_items.push(TradeOfferItem {
            item_id,
            amount,
            identified: true,
            refine: 0,
            label,
        });
        self.rebuild_display();
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
        state.open_with_partner("Alice".into(), CharacterId(1), 50);
        assert!(state.is_active());
        assert!(!state.has_pending());
        state.lock_side(0);
        state.lock_side(1);
        assert!(state.display_text().contains("you=yes"));
        assert!(state.display_text().contains("them=yes"));
    }
}
