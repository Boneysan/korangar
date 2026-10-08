use korangar_interface::element::StateElement;
use korangar_networking::NetworkingSystem;
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData, HotkeyType, ItemId};
use rust_state::RustState;

use crate::state::skills::LearnableSkill;

/// Official 2022 hotkey rows are nine slots. Four rows fit in the 38-slot
/// server table (tab 0, slots 0–35) and map to 1–9 / Ctrl+1–9 / Alt+1–9 /
/// Shift+1–9.
pub const HOTBAR_COLUMNS: usize = 9;
pub const HOTBAR_ROWS: usize = 4;
pub const HOTBAR_SLOTS: usize = HOTBAR_COLUMNS * HOTBAR_ROWS;

#[derive(Clone, Debug, RustState, StateElement)]
pub enum HotbarBinding {
    Skill(LearnableSkill),
    Item { item_id: ItemId },
}

#[derive(RustState, StateElement)]
pub struct Hotbar {
    slots: [Option<HotbarBinding>; HOTBAR_SLOTS],
}

impl Default for Hotbar {
    fn default() -> Self {
        Self {
            slots: [const { None }; HOTBAR_SLOTS],
        }
    }
}

impl Hotbar {
    /// Clear local bindings when leaving an account/character. The next map
    /// login repopulates them from the server's hotkey packet.
    pub fn clear(&mut self) {
        self.slots.fill(None);
    }

    pub fn first_empty_slot(&self) -> Option<HotbarSlot> {
        self.slots.iter().position(Option::is_none).map(|index| HotbarSlot(index as u16))
    }

    /// Where a right-click "add" lands. A slot that already holds this skill
    /// or item is replaced, so a second add updates that slot instead of
    /// filling another one. Otherwise the first empty slot. `None` when the
    /// bar is full and this thing is not already on it; a drag onto a slot
    /// still replaces that slot directly.
    pub fn slot_to_assign(&self, is_same: impl Fn(&HotbarBinding) -> bool) -> Option<HotbarSlot> {
        if let Some(index) = self.slots.iter().position(|slot| slot.as_ref().is_some_and(&is_same)) {
            return Some(HotbarSlot(index as u16));
        }
        self.first_empty_slot()
    }

    pub fn get_slot(&self, slot: HotbarSlot) -> &Option<HotbarBinding> {
        self.slots.get(slot.0 as usize).unwrap_or(&None)
    }

    /// Kept for call sites that only care about skills (channeling stop, etc.).
    pub fn get_skill_in_slot(&self, slot: HotbarSlot) -> Option<&LearnableSkill> {
        match self.get_slot(slot) {
            Some(HotbarBinding::Skill(skill)) => Some(skill),
            _ => None,
        }
    }

    pub fn set_slot(&mut self, slot: HotbarSlot, binding: HotbarBinding) {
        if let Some(entry) = self.slots.get_mut(slot.0 as usize) {
            *entry = Some(binding);
        }
    }

    pub fn unset_slot(&mut self, slot: HotbarSlot) {
        if let Some(entry) = self.slots.get_mut(slot.0 as usize) {
            *entry = None;
        }
    }

    pub fn update_slot<Callback>(&mut self, networking_system: &mut NetworkingSystem<Callback>, slot: HotbarSlot, binding: HotbarBinding)
    where
        Callback: PacketCallback + Send,
    {
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, binding_to_hotkey(&binding));
        self.set_slot(slot, binding);
    }

    pub fn clear_slot<Callback>(&mut self, networking_system: &mut NetworkingSystem<Callback>, slot: HotbarSlot)
    where
        Callback: PacketCallback + Send,
    {
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, HotkeyData::UNBOUND);
        self.unset_slot(slot);
    }

    pub fn swap_slot<Callback>(
        &mut self,
        networking_system: &mut NetworkingSystem<Callback>,
        source_slot: HotbarSlot,
        destination_slot: HotbarSlot,
    ) where
        Callback: PacketCallback + Send,
    {
        if source_slot == destination_slot {
            return;
        }
        let Some(source_index) = self.slots.get(source_slot.0 as usize).map(|_| source_slot.0 as usize) else {
            return;
        };
        let Some(destination_index) = self.slots.get(destination_slot.0 as usize).map(|_| destination_slot.0 as usize) else {
            return;
        };

        let first = self.slots[source_index].take();
        let second = self.slots[destination_index].take();

        let _ = networking_system.set_hotkey_data(HotbarTab(0), destination_slot, optional_binding_to_hotkey(first.as_ref()));
        let _ = networking_system.set_hotkey_data(HotbarTab(0), source_slot, optional_binding_to_hotkey(second.as_ref()));

        self.slots[source_index] = second;
        self.slots[destination_index] = first;
    }

    pub fn for_each_skill_mut(&mut self, mut visit: impl FnMut(&mut LearnableSkill)) {
        for slot in &mut self.slots {
            if let Some(HotbarBinding::Skill(skill)) = slot {
                visit(skill);
            }
        }
    }
}

fn binding_to_hotkey(binding: &HotbarBinding) -> HotkeyData {
    match binding {
        HotbarBinding::Skill(skill) => HotkeyData {
            hotkey_type: HotkeyType::Skill,
            item_or_skill_id: u32::from(skill.skill_id.0),
            quantity_or_skill_level: skill.maximum_level.0,
        },
        HotbarBinding::Item { item_id } => HotkeyData {
            hotkey_type: HotkeyType::Item,
            item_or_skill_id: item_id.0,
            quantity_or_skill_level: 0,
        },
    }
}

fn optional_binding_to_hotkey(binding: Option<&HotbarBinding>) -> HotkeyData {
    binding.map(binding_to_hotkey).unwrap_or(HotkeyData::UNBOUND)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_rows_fit_the_server_table() {
        assert_eq!(HOTBAR_SLOTS, 36);
        assert!(HOTBAR_SLOTS < 38);
    }

    fn holds_item(item_id: u32) -> impl Fn(&HotbarBinding) -> bool {
        move |binding| matches!(binding, HotbarBinding::Item { item_id: bound } if bound.0 == item_id)
    }

    #[test]
    fn right_click_assign_replaces_the_slot_that_already_has_the_item() {
        let mut hotbar = Hotbar::default();
        hotbar.set_slot(HotbarSlot(0), HotbarBinding::Item { item_id: ItemId(504) });
        hotbar.set_slot(HotbarSlot(2), HotbarBinding::Item { item_id: ItemId(501) });

        assert_eq!(hotbar.slot_to_assign(holds_item(501)), Some(HotbarSlot(2)));
        assert_eq!(hotbar.slot_to_assign(holds_item(502)), Some(HotbarSlot(1)));
    }

    #[test]
    fn right_click_assign_refuses_a_full_bar_that_does_not_already_hold_the_item() {
        let mut hotbar = Hotbar::default();
        for index in 0..HOTBAR_SLOTS {
            hotbar.set_slot(HotbarSlot(index as u16), HotbarBinding::Item { item_id: ItemId(501) });
        }

        assert_eq!(hotbar.slot_to_assign(holds_item(501)), Some(HotbarSlot(0)));
        assert_eq!(hotbar.slot_to_assign(holds_item(502)), None);
    }

    #[test]
    fn putting_an_item_in_a_filled_slot_replaces_that_slot() {
        let mut hotbar = Hotbar::default();
        hotbar.set_slot(HotbarSlot(4), HotbarBinding::Item { item_id: ItemId(501) });
        hotbar.set_slot(HotbarSlot(4), HotbarBinding::Item { item_id: ItemId(502) });

        match hotbar.get_slot(HotbarSlot(4)) {
            Some(HotbarBinding::Item { item_id }) => assert_eq!(item_id.0, 502),
            other => panic!("slot 4 should hold the dropped item, got {other:?}"),
        }
    }
}
