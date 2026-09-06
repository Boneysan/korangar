use korangar_interface::element::StateElement;
use korangar_networking::NetworkingSystem;
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData, HotkeyType, ItemId};
use rust_state::RustState;

use crate::state::skills::LearnableSkill;

/// Official 2022 hotkey rows are nine slots. Three rows fit in the 38-slot
/// server table (tab 0, slots 0–26) and map to 1–9 / Ctrl+1–9 / Alt+1–9.
pub const HOTBAR_COLUMNS: usize = 9;
pub const HOTBAR_ROWS: usize = 3;
pub const HOTBAR_SLOTS: usize = HOTBAR_COLUMNS * HOTBAR_ROWS;

#[derive(Clone, Debug, RustState, StateElement)]
pub enum HotbarBinding {
    Skill(LearnableSkill),
    Item { item_id: ItemId },
}

#[derive(Default, RustState, StateElement)]
pub struct Hotbar {
    slots: [Option<HotbarBinding>; HOTBAR_SLOTS],
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
    fn three_rows_fit_the_server_table() {
        assert_eq!(HOTBAR_SLOTS, 27);
        assert!(HOTBAR_SLOTS < 38);
    }
}
