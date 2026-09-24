use std::sync::Arc;

use korangar_interface::element::StateElement;
use korangar_networking::{InventoryItem, InventoryItemDetails, NoMetadata};
use ragnarok_packets::{EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, RegularItemFlags};
use rust_state::RustState;

use crate::graphics::Texture;
use crate::loaders::AsyncLoader;
use crate::world::ResourceMetadata;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub enum InventoryTab {
    #[default]
    All,
    Equipped,
    Gear,
    Items,
    Consumables,
    Etc,
    Cards,
    Ammo,
}

#[derive(Default, RustState, StateElement)]
pub struct Inventory {
    // TODO: Unhide this.
    #[hidden_element]
    items: Vec<InventoryItem<ResourceMetadata>>,
    selected_tab: InventoryTab,
    split_amount: String,
    search_query: String,
}

impl Inventory {
    pub fn fill(&mut self, async_loader: &AsyncLoader, items: Vec<InventoryItem<NoMetadata>>) {
        let mut incoming: Vec<_> = items
            .into_iter()
            .map(|item| async_loader.request_inventory_item_metadata_load(item))
            .collect();
        // A full inventory sync used to replace the vec in server order, so a
        // trade or equip ack shuffled the grid. Keep the order the player
        // already had, and append only stacks that are new.
        let mut ordered = Vec::with_capacity(incoming.len());
        for previous in self.items.drain(..) {
            if let Some(position) = incoming.iter().position(|item| item.index == previous.index) {
                ordered.push(incoming.remove(position));
            }
        }
        ordered.append(&mut incoming);
        self.items = ordered;
    }

    pub fn add_item(&mut self, async_loader: &AsyncLoader, item: InventoryItem<NoMetadata>) {
        let Some(position) = self.items.iter().position(|inventory_item| inventory_item.index == item.index) else {
            self.items.push(async_loader.request_inventory_item_metadata_load(item));
            return;
        };

        // Stacking a pickup onto an existing slot merges the amount. Both
        // Regular items (potions) and Equippable-but-stackable ammo (arrows,
        // which carry an equip position) merge; any variant mismatch is a
        // re-report, so replace rather than panic (the old behaviour crashed
        // the client when ammo restacked).
        let merged = match (&mut self.items[position].details, &item.details) {
            (InventoryItemDetails::Regular { amount, .. }, InventoryItemDetails::Regular { amount: added, .. })
            | (InventoryItemDetails::Equippable { amount, .. }, InventoryItemDetails::Equippable { amount: added, .. }) => {
                *amount = amount.saturating_add(*added);
                true
            }
            _ => false,
        };

        if !merged {
            self.items[position] = async_loader.request_inventory_item_metadata_load(item);
        }
    }

    pub fn update_item_sprite(&mut self, item_id: ItemId, texture: Arc<Texture>) {
        self.items.iter_mut().filter(|item| item.item_id == item_id).for_each(|item| {
            item.metadata.texture = Some(texture.clone());
        });
    }

    pub fn remove_item(&mut self, index: InventoryIndex, remove_amount: u16) {
        let Some(position) = self.items.iter().position(|item| item.index == index) else {
            // Already removed (e.g. both 0x07FA and 0x00AF arrived for a drop).
            return;
        };

        // Ammo is stackable *and* `Equippable`, so a stack count has to be honoured
        // for both variants. Decrementing only `Regular` deleted the whole arrow
        // stack the first time a single arrow was spent: the item vanished from the
        // inventory mid-fight, the Ammo slot emptied, and every later shot fell back
        // to the generic arrow sprite because the equipped stack no longer existed.
        // Real gear is unaffected — it has `amount: 1`, so it still removes outright.
        let amount = match &mut self.items[position].details {
            InventoryItemDetails::Regular { amount, .. } | InventoryItemDetails::Equippable { amount, .. } => amount,
        };

        if *amount > remove_amount {
            *amount -= remove_amount;
            return;
        }

        self.items.remove(position);
    }

    /// Equipment before other items, whether or not it is worn.
    pub fn sort_gear(&mut self) {
        self.items.sort_by(|left, right| {
            fn gear(item: &InventoryItem<ResourceMetadata>) -> u8 {
                match &item.details {
                    InventoryItemDetails::Equippable { .. } => 0,
                    InventoryItemDetails::Regular { .. } => 1,
                }
            }
            gear(left)
                .cmp(&gear(right))
                .then_with(|| left.metadata.name.cmp(&right.metadata.name))
        });
    }

    /// Consumables and other non-equipment before gear.
    pub fn sort_items(&mut self) {
        self.items.sort_by(|left, right| {
            fn item_rank(item: &InventoryItem<ResourceMetadata>) -> u8 {
                match &item.details {
                    InventoryItemDetails::Regular { .. } => 0,
                    InventoryItemDetails::Equippable { .. } => 1,
                }
            }
            item_rank(left)
                .cmp(&item_rank(right))
                .then_with(|| left.metadata.name.cmp(&right.metadata.name))
        });
    }

    pub fn selected_tab(&self) -> InventoryTab {
        self.selected_tab
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn set_selected_tab(&mut self, selected_tab: InventoryTab) {
        self.selected_tab = selected_tab;
    }

    /// Group worn gear, then other equipment, then everything else, by name.
    pub fn sort_for_display(&mut self) {
        self.items.sort_by(|left, right| {
            fn rank(item: &InventoryItem<ResourceMetadata>) -> u8 {
                match &item.details {
                    InventoryItemDetails::Equippable { equipped_position, .. } if !equipped_position.is_empty() => 0,
                    InventoryItemDetails::Equippable { .. } => 1,
                    InventoryItemDetails::Regular { .. } => 2,
                }
            }
            rank(left)
                .cmp(&rank(right))
                .then_with(|| left.metadata.name.cmp(&right.metadata.name))
                .then_with(|| left.index.0.cmp(&right.index.0))
        });
    }

    /// Move an item in the visible order; the caller persists the resulting
    /// slot list.
    pub fn reorder_display(&mut self, from_index: InventoryIndex, to_slot: usize) {
        let Some(from) = self.items.iter().position(|item| item.index == from_index) else {
            return;
        };

        if from == to_slot {
            return;
        }

        let item = self.items.remove(from);
        let insert_at = to_slot.min(self.items.len());
        self.items.insert(insert_at, item);
    }

    pub fn reorder_display_in_tab(&mut self, from_index: InventoryIndex, to_slot: usize, tab: InventoryTab, query: &str) {
        if tab == InventoryTab::All {
            if query.is_empty() {
                self.reorder_display(from_index, to_slot);
                return;
            }
        }
        let Some(from) = self.items.iter().position(|item| item.index == from_index) else {
            return;
        };
        if !inventory_item_visible(&self.items[from], tab, query) {
            return;
        }
        let destinations: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| inventory_item_visible(item, tab, query))
            .map(|(index, _)| index)
            .collect();
        let target = destinations
            .get(to_slot)
            .copied()
            .or_else(|| destinations.last().map(|index| index + 1));
        let Some(mut target) = target else { return };
        let item = self.items.remove(from);
        if from < target {
            target -= 1;
        }
        self.items.insert(target.min(self.items.len()), item);
    }

    pub fn ordered_indices(&self) -> Vec<InventoryIndex> {
        self.items.iter().map(|item| item.index).collect()
    }

    pub fn apply_server_order(&mut self, indices: &[InventoryIndex]) {
        let mut ordered = Vec::with_capacity(self.items.len());
        for index in indices {
            if let Some(position) = self.items.iter().position(|item| item.index == *index) {
                ordered.push(self.items.remove(position));
            }
        }
        ordered.append(&mut self.items);
        self.items = ordered;
    }

    pub fn update_equipped_position(&mut self, index: InventoryIndex, new_equipped_position: EquipPosition) {
        // There is one ammo slot, so at most one stack may carry `AMMO`. The server
        // does unequip the previous stack first, but a dropped or reordered ack would
        // otherwise leave two stacks looking equipped — and then the *first* one wins
        // as the active ammunition, which is silently wrong rather than visibly wrong.
        if new_equipped_position.contains(EquipPosition::AMMO) {
            for item in self.items.iter_mut().filter(|item| item.index != index) {
                if let InventoryItemDetails::Equippable { equipped_position, .. } = &mut item.details {
                    equipped_position.remove(EquipPosition::AMMO);
                }
            }
        }

        // A stale/out-of-range index (e.g. an equip broadcast racing an
        // inventory reload) must not crash the client.
        let Some(item) = self.items.iter_mut().find(|item| item.index == index) else {
            return;
        };

        let InventoryItemDetails::Equippable { equipped_position, .. } = &mut item.details else {
            // This can happen for ammunition for example.
            return;
        };

        *equipped_position = new_equipped_position;
    }

    /// Mark an inventory item as identified after a successful identify.
    pub fn mark_identified(&mut self, index: InventoryIndex) {
        let Some(item) = self.items.iter_mut().find(|item| item.index == index) else {
            return;
        };
        match &mut item.details {
            InventoryItemDetails::Regular { flags, .. } => {
                *flags |= RegularItemFlags::IDENTIFIED;
            }
            InventoryItemDetails::Equippable { flags, .. } => {
                *flags |= EquippableItemFlags::IDENTIFIED;
            }
        }
    }

    pub fn items(&self) -> &[InventoryItem<ResourceMetadata>] {
        &self.items
    }

    /// How many of an item the character is carrying, across every stack.
    ///
    /// A stackable item normally occupies one slot, but the server is free to
    /// split it (a partial pickup on a full stack does), so this sums rather
    /// than finding the first match.
    pub fn count_of(&self, item_id: ItemId) -> u32 {
        self.items
            .iter()
            .filter(|item| item.item_id == item_id)
            .map(|item| u32::from(item.amount()))
            .sum()
    }

    /// Right-hand LOOK_WEAPON appearance for the local player.
    ///
    /// Character selection commonly reports weapon look 0. After map login the
    /// inventory is authoritative. Hercules `PACKETVER ≥ 4` sends the raw item
    /// ID (not the class view) on the appearance channel, so we do the same —
    /// per-item sprites and attack selection both need the nameid.
    pub fn equipped_weapon_look(&self) -> u32 {
        self.items
            .iter()
            .find_map(|item| {
                let InventoryItemDetails::Equippable { equipped_position, .. } = &item.details else {
                    return None;
                };
                equipped_position.contains(EquipPosition::RIGHT_HAND).then_some(item.item_id.0)
            })
            .unwrap_or(0)
    }

    /// Item currently loaded in the ammunition slot, if any.
    ///
    /// Ammo is stackable *and* occupies the AMMO equip slot, so it is modeled
    /// as `Equippable` throughout (see `InventoryItemDetails::ammo`) and its
    /// equipped state lives in `equipped_position` like any other gear. The
    /// classic client draws the flying projectile with this item's sprite,
    /// which is how Iron Arrow and Fire Arrow read differently in flight.
    pub fn equipped_ammunition(&self) -> Option<ItemId> {
        self.items.iter().find_map(|item| {
            let InventoryItemDetails::Equippable { equipped_position, .. } = &item.details else {
                return None;
            };
            equipped_position.contains(EquipPosition::AMMO).then_some(item.item_id)
        })
    }

    /// Classic weapon class view for the equipped right-hand item (attack
    /// family only). Prefer [`Self::equipped_weapon_look`] for sprite paths.
    #[allow(dead_code)]
    pub fn equipped_weapon_type(&self) -> u32 {
        let look = self.equipped_weapon_look();
        if look == 0 {
            0
        } else {
            crate::world::weapon_view_from_appearance(look)
        }
    }

    /// Left-hand LOOK_SHIELD appearance for the local player.
    ///
    /// - `Some(0)` — no left-hand equippable (clear shield / off-hand).
    /// - `Some(view 1..=4)` — classic shield item →
    ///   Guard/Buckler/Shield/Mirror.
    /// - `Some(item_id)` — off-hand weapon (Assassin dual-wield); matches
    ///   Hercules `get_weapon_view` which puts the left nameid on the shield
    ///   channel.
    /// - `None` — left hand holds something inventory cannot classify; leave
    ///   `common.shield` alone so `ChangeShield` stays authoritative.
    pub fn equipped_left_hand_look(&self) -> Option<u32> {
        let left_hand_id = self.items.iter().find_map(|item| {
            let InventoryItemDetails::Equippable { equipped_position, .. } = &item.details else {
                return None;
            };
            equipped_position.contains(EquipPosition::LEFT_HAND).then_some(item.item_id.0)
        });

        match left_hand_id {
            None => Some(0),
            Some(item_id) => {
                if let Some(shield_view) = shield_view_from_item_id(item_id) {
                    Some(shield_view)
                } else if crate::world::weapon_view_from_item_id(item_id) != 0 {
                    // Dual-wield / left-hand weapon: raw item ID like Hercules.
                    Some(item_id)
                } else {
                    None
                }
            }
        }
    }

    /// Shield view for the equipped left-hand item, when inventory can map it.
    ///
    /// Prefer [`Self::equipped_left_hand_look`] for full dual-wield support.
    /// This keeps the Phase C shield-only helper for callers that only care
    /// about Guard/Buckler/Shield/Mirror.
    #[allow(dead_code)]
    pub fn equipped_shield_view(&self) -> Option<u32> {
        match self.equipped_left_hand_look() {
            Some(0) => Some(0),
            Some(look) if look < crate::world::WEAPON_VIEW_CLASS_MAX => Some(look),
            Some(_) => {
                // Off-hand weapon item ID — not a shield view.
                None
            }
            None => None,
        }
    }
}

pub fn inventory_tab_matches(item: &InventoryItem<ResourceMetadata>, tab: InventoryTab) -> bool {
    match tab {
        InventoryTab::All => true,
        InventoryTab::Equipped => {
            matches!(&item.details, InventoryItemDetails::Equippable { equipped_position, .. } if !equipped_position.is_empty())
        }
        InventoryTab::Gear => {
            item.item_type != korangar_networking::IT_AMMO && matches!(&item.details, InventoryItemDetails::Equippable { .. })
        }
        InventoryTab::Items => matches!(&item.details, InventoryItemDetails::Regular { .. }),
        InventoryTab::Consumables | InventoryTab::Etc | InventoryTab::Cards | InventoryTab::Ammo => {
            item_type_matches_category(item.item_type, tab)
        }
    }
}

fn item_type_matches_category(item_type: u8, tab: InventoryTab) -> bool {
    // Hercules IT_* values shared by the 20220406 inventory and storage lists.
    match tab {
        InventoryTab::Consumables => matches!(item_type, 0 | 2 | 11),
        InventoryTab::Etc => item_type == 3,
        InventoryTab::Cards => item_type == 6,
        InventoryTab::Ammo => item_type == korangar_networking::IT_AMMO,
        _ => false,
    }
}

fn inventory_item_visible(item: &InventoryItem<ResourceMetadata>, tab: InventoryTab, query: &str) -> bool {
    inventory_tab_matches(item, tab) && item.metadata.name.to_lowercase().contains(&query.to_lowercase())
}

/// Classic shield item IDs → ViewSprite (Guard/Buckler/Shield/Mirror).
/// Unknown IDs return `None` so inventory never invents a shield view;
/// `ChangeShield` remains authoritative for custom / high-view shields.
fn shield_view_from_item_id(item_id: u32) -> Option<u32> {
    match item_id {
        2101 => Some(1), // Guard
        2102 => Some(2), // Buckler
        2103 => Some(3), // Shield
        2104 => Some(4), // Mirror Shield
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{InventoryTab, item_type_matches_category, shield_view_from_item_id};

    #[test]
    fn item_categories_follow_hercules_item_type_values() {
        for item_type in [0, 2, 11] {
            assert!(item_type_matches_category(item_type, InventoryTab::Consumables));
        }
        assert!(item_type_matches_category(3, InventoryTab::Etc));
        assert!(item_type_matches_category(6, InventoryTab::Cards));
        assert!(item_type_matches_category(10, InventoryTab::Ammo));
        assert!(!item_type_matches_category(3, InventoryTab::Cards));
        assert!(!item_type_matches_category(255, InventoryTab::Consumables));
    }

    #[test]
    fn classic_shield_item_ids_map_to_view_sprites() {
        assert_eq!(shield_view_from_item_id(2101), Some(1));
        assert_eq!(shield_view_from_item_id(2102), Some(2));
        assert_eq!(shield_view_from_item_id(2103), Some(3));
        assert_eq!(shield_view_from_item_id(2104), Some(4));
        // Not a shield (sword / dagger)
        assert_eq!(shield_view_from_item_id(1101), None);
        assert_eq!(shield_view_from_item_id(1201), None);
    }
}
