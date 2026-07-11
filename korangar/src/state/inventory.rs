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
        if let Some(found_item) = self.items.iter_mut().find(|inventory_item| inventory_item.index == item.index) {
            let InventoryItemDetails::Regular { amount, .. } = &mut found_item.details else {
                panic!();
            };

            let InventoryItemDetails::Regular { amount: added_amount, .. } = item.details else {
                panic!();
            };

            *amount += added_amount;
        } else {
            let item = async_loader.request_inventory_item_metadata_load(item);

            self.items.push(item);
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
    /// changes how the client lays items out until the next full inventory sync.
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
}
