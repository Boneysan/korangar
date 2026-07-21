use std::sync::Arc;

use korangar_interface::element::StateElement;
use korangar_networking::{InventoryItem, InventoryItemDetails, NoMetadata};
use ragnarok_packets::{EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, RegularItemFlags};
use rust_state::RustState;

use crate::graphics::Texture;
use crate::loaders::AsyncLoader;
use crate::world::ResourceMetadata;

#[derive(Default, RustState, StateElement)]
pub struct Inventory {
    // TODO: Unhide this.
    #[hidden_element]
    items: Vec<InventoryItem<ResourceMetadata>>,
}

impl Inventory {
    pub fn fill(&mut self, async_loader: &AsyncLoader, items: Vec<InventoryItem<NoMetadata>>) {
        self.items = items
            .into_iter()
            .map(|item| async_loader.request_inventory_item_metadata_load(item))
            .collect();
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

        if let InventoryItemDetails::Regular { amount, .. } = &mut self.items[position].details
            && *amount > remove_amount
        {
            *amount -= remove_amount;
            return;
        }

        self.items.remove(position);
    }

    /// Move an item in the local inventory display order (grid drag-and-drop).
    ///
    /// Hercules does not expose a free inventory rearrange packet; this only
    /// changes how the client lays items out until the next full inventory
    /// sync.
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

    pub fn update_equipped_position(&mut self, index: InventoryIndex, new_equipped_position: EquipPosition) {
        let item = self.items.iter_mut().find(|item| item.index == index).unwrap();

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
                equipped_position
                    .contains(EquipPosition::RIGHT_HAND)
                    .then_some(item.item_id.0)
            })
            .unwrap_or(0)
    }

    /// Classic weapon class view for the equipped right-hand item (attack
    /// family only). Prefer [`Self::equipped_weapon_look`] for sprite paths.
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
    /// - `Some(view 1..=4)` — classic shield item → Guard/Buckler/Shield/Mirror.
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
            equipped_position
                .contains(EquipPosition::LEFT_HAND)
                .then_some(item.item_id.0)
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
    use super::shield_view_from_item_id;

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
