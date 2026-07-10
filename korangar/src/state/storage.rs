//! Kafra / personal storage state.

use std::sync::Arc;

use korangar_interface::element::StateElement;
use korangar_networking::{InventoryItem, InventoryItemDetails, NoMetadata};
use ragnarok_packets::InventoryIndex;
use rust_state::RustState;

use crate::graphics::Texture;
use crate::loaders::AsyncLoader;
use crate::world::ResourceMetadata;

#[derive(Default, RustState, StateElement)]
pub struct StorageState {
    #[hidden_element]
    items: Vec<InventoryItem<ResourceMetadata>>,
    amount: u16,
    max_amount: u16,
    open: bool,
    capacity_text: String,
}

impl StorageState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn items(&self) -> &[InventoryItem<ResourceMetadata>] {
        &self.items
    }

    pub fn capacity_text(&self) -> &str {
        &self.capacity_text
    }

    pub fn set_list(&mut self, async_loader: &AsyncLoader, items: Vec<InventoryItem<NoMetadata>>) {
        self.items = items
            .into_iter()
            .map(|item| async_loader.request_inventory_item_metadata_load(item))
            .collect();
        self.open = true;
        self.rebuild_capacity_text();
    }

    pub fn set_amount(&mut self, amount: u16, max_amount: u16) {
        self.amount = amount;
        self.max_amount = max_amount;
        self.rebuild_capacity_text();
    }

    pub fn add_item(&mut self, async_loader: &AsyncLoader, item: InventoryItem<NoMetadata>) {
        if let Some(found) = self.items.iter_mut().find(|i| i.index == item.index) {
            if let (
                InventoryItemDetails::Regular { amount, .. },
                InventoryItemDetails::Regular {
                    amount: added, ..
                },
            ) = (&mut found.details, &item.details)
            {
                *amount = amount.saturating_add(*added);
                return;
            }
        }
        self.items
            .push(async_loader.request_inventory_item_metadata_load(item));
        self.rebuild_capacity_text();
    }

    pub fn remove_item(&mut self, index: InventoryIndex, remove_amount: u32) {
        let Some(position) = self.items.iter().position(|item| item.index == index) else {
            return;
        };
        if let InventoryItemDetails::Regular { amount, .. } = &mut self.items[position].details {
            let remove = remove_amount.min(u32::from(*amount)) as u16;
            if *amount > remove {
                *amount -= remove;
                return;
            }
        }
        self.items.remove(position);
        self.rebuild_capacity_text();
    }

    pub fn update_item_sprite(&mut self, item_id: ragnarok_packets::ItemId, texture: Arc<Texture>) {
        self.items.iter_mut().filter(|item| item.item_id == item_id).for_each(|item| {
            item.metadata.texture = Some(texture.clone());
        });
    }

    pub fn close(&mut self) {
        self.open = false;
        self.items.clear();
        self.amount = 0;
        self.max_amount = 0;
        self.capacity_text.clear();
    }

    fn rebuild_capacity_text(&mut self) {
        if self.max_amount > 0 {
            self.capacity_text = format!("{}/{}", self.amount, self.max_amount);
        } else {
            self.capacity_text = format!("{} items", self.items.len());
        }
    }
}
