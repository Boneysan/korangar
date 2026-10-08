use ragnarok_packets::{
    EntityId, EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, ItemOptions, Price, RegularItemFlags, SellItemInformation,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoMetadata;

#[derive(Clone, Debug)]
pub enum InventoryItemDetails {
    Regular {
        amount: u16,
        equipped_position: EquipPosition,
        flags: RegularItemFlags,
    },
    Equippable {
        /// Stack size. Real gear is always 1; ammo (arrows) is equippable *and*
        /// stackable, so it carries its real count here for display/merging.
        amount: u16,
        equip_position: EquipPosition,
        equipped_position: EquipPosition,
        bind_on_equip_type: u16,
        w_item_sprite_number: u16,
        option_count: u8,
        option_data: [ItemOptions; 5], // fix count
        refinement_level: u8,
        enchantment_level: u8,
        flags: EquippableItemFlags,
    },
}

#[derive(Clone, Debug)]
pub struct InventoryItem<Meta> {
    pub metadata: Meta,
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub details: InventoryItemDetails,
}

/// Hercules `IT_AMMO` item type (arrows, bullets, …). Ammo is stackable yet
/// occupies the AMMO equip slot, so it must be modeled as `Equippable` (which
/// carries the equip slot + amount) rather than `Regular`, and consistently so
/// across every inventory source (normal list, pickup, storage).
pub const IT_AMMO: u8 = 10;

impl InventoryItemDetails {
    /// Build the `Equippable` details for stackable ammo, which the server may
    /// report without the equippable fields (e.g. in the normal/stackable
    /// list). Ammo is always the AMMO slot, unrefined, no options.
    pub fn ammo(amount: u16, equipped_position: EquipPosition, identified: bool) -> Self {
        let mut flags = EquippableItemFlags::empty();
        flags.set(EquippableItemFlags::IDENTIFIED, identified);
        InventoryItemDetails::Equippable {
            amount,
            equip_position: EquipPosition::AMMO,
            equipped_position,
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 0,
            option_data: [ItemOptions::default(); 5],
            refinement_level: 0,
            enchantment_level: 0,
            flags,
        }
    }
}

impl<Meta> InventoryItem<Meta> {
    /// Whether the item sits in an equipment slot right now.
    pub fn is_equipped(&self) -> bool {
        match &self.details {
            InventoryItemDetails::Regular { equipped_position, .. } => !equipped_position.is_empty(),
            InventoryItemDetails::Equippable { equipped_position, .. } => !equipped_position.is_empty(),
        }
    }

    /// Stack size. Real gear is always 1; stackables and ammo carry a count.
    pub fn amount(&self) -> u16 {
        match &self.details {
            InventoryItemDetails::Regular { amount, .. } => *amount,
            InventoryItemDetails::Equippable { amount, .. } => *amount,
        }
    }

    pub fn is_identified(&self) -> bool {
        match &self.details {
            InventoryItemDetails::Regular { flags, .. } => flags.contains(RegularItemFlags::IDENTIFIED),
            InventoryItemDetails::Equippable { flags, .. } => flags.contains(EquippableItemFlags::IDENTIFIED),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemQuantity {
    Fixed(u32),
    Infinite,
}

impl From<u32> for ItemQuantity {
    fn from(value: u32) -> Self {
        match value == !0 {
            true => ItemQuantity::Infinite,
            false => ItemQuantity::Fixed(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopItem<Meta> {
    pub metadata: Meta,
    pub item_id: ItemId,
    pub item_type: u8,
    pub price: Price,
    pub quantity: ItemQuantity,
    pub weight: u16,
    pub location: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SellItem<Meta> {
    pub metadata: Meta,
    pub inventory_index: InventoryIndex,
    pub price: Price,
    pub overcharge_price: Price,
}

/// Pair the server's sell offers with the inventory, each carrying the whole
/// stack so a quiver of 500 arrows can be sold as 500, not one. Equipped items
/// are left out: this server never offers them (Hercules 8b850e4e5) and
/// refuses the sale, but a stock server lists them.
pub fn sell_entries<Meta: Clone>(
    offers: impl IntoIterator<Item = SellItemInformation>,
    inventory: &[InventoryItem<Meta>],
) -> Vec<SellItem<(Meta, u16)>> {
    offers
        .into_iter()
        .filter_map(|offer| {
            let item = inventory.iter().find(|item| item.index == offer.inventory_index)?;
            (!item.is_equipped()).then(|| SellItem {
                metadata: (item.metadata.clone(), item.amount()),
                inventory_index: offer.inventory_index,
                price: offer.price,
                overcharge_price: offer.overcharge_price,
            })
        })
        .collect()
}

/// Apply `ZC_USE_ITEM_ACK` (0x01C8) to one inventory.
///
/// Hercules reports the stack still in the slot (`amount - 1` for an immediate
/// consume, the unchanged count when the item is kept) and deletes it with no
/// separate delete packet. A success is broadcast to the whole area, so an
/// `entity_id` other than `local_entity_id` is ignored. A failed use is
/// ignored. Zero removes the slot. A negative count is ignored.
pub fn apply_use_item_ack<Meta>(
    inventory: &mut Vec<InventoryItem<Meta>>,
    local_entity_id: EntityId,
    entity_id: EntityId,
    index: InventoryIndex,
    amount: i16,
    success: bool,
) {
    if !success || entity_id != local_entity_id {
        return;
    }
    let Ok(amount) = u16::try_from(amount) else {
        return;
    };

    let Some(position) = inventory.iter().position(|item| item.index == index) else {
        return;
    };
    if amount == 0 {
        inventory.remove(position);
        return;
    }
    match &mut inventory[position].details {
        InventoryItemDetails::Regular { amount: slot_amount, .. } | InventoryItemDetails::Equippable { amount: slot_amount, .. } => {
            *slot_amount = amount;
        }
    }
}

#[cfg(test)]
mod sell_entry_tests {
    use super::*;

    fn item(index: u16, details: InventoryItemDetails) -> InventoryItem<NoMetadata> {
        InventoryItem {
            metadata: NoMetadata,
            index: InventoryIndex(index),
            item_id: ItemId(1750),
            item_type: IT_AMMO,
            slot: [0; 4],
            hire_expiration_date: 0,
            details,
        }
    }

    fn offer(index: u16) -> SellItemInformation {
        SellItemInformation {
            inventory_index: InventoryIndex(index),
            price: Price(1),
            overcharge_price: Price(1),
        }
    }

    #[test]
    fn an_ammo_stack_is_offered_whole() {
        let inventory = [item(2, InventoryItemDetails::ammo(500, EquipPosition::empty(), true))];
        let entries = sell_entries([offer(2)], &inventory);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].metadata.1, 500);
    }

    #[test]
    fn equipped_items_and_unknown_indices_are_left_out() {
        let inventory = [
            item(2, InventoryItemDetails::ammo(500, EquipPosition::AMMO, true)),
            item(3, InventoryItemDetails::Regular {
                amount: 7,
                equipped_position: EquipPosition::empty(),
                flags: RegularItemFlags::IDENTIFIED,
            }),
        ];
        let entries = sell_entries([offer(2), offer(3), offer(9)], &inventory);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].inventory_index, InventoryIndex(3));
        assert_eq!(entries[0].metadata.1, 7);
    }
}

#[cfg(test)]
mod use_item_ack_tests {
    use super::*;

    fn potion(amount: u16) -> InventoryItem<NoMetadata> {
        InventoryItem {
            metadata: NoMetadata,
            index: InventoryIndex(3),
            item_id: ItemId(569),
            item_type: 0,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Regular {
                amount,
                equipped_position: EquipPosition::empty(),
                flags: RegularItemFlags::empty(),
            },
        }
    }

    fn apply(inventory: &mut Vec<InventoryItem<NoMetadata>>, entity_id: EntityId, amount: i16, success: bool) {
        apply_use_item_ack(inventory, EntityId(1), entity_id, InventoryIndex(3), amount, success);
    }

    #[test]
    fn a_successful_ack_sets_the_remaining_stack() {
        let mut inventory = vec![potion(5)];
        apply(&mut inventory, EntityId(1), 2, true);
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].amount(), 2);
    }

    #[test]
    fn another_players_ack_leaves_the_stack_alone() {
        let mut inventory = vec![potion(5)];
        apply(&mut inventory, EntityId(99), 0, true);
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].amount(), 5);
    }

    #[test]
    fn a_failed_use_leaves_the_stack_alone() {
        let mut inventory = vec![potion(5)];
        apply(&mut inventory, EntityId(1), 0, false);
        assert_eq!(inventory[0].amount(), 5);
    }

    #[test]
    fn a_remaining_count_of_zero_removes_the_slot() {
        let mut inventory = vec![potion(1)];
        apply(&mut inventory, EntityId(1), 0, true);
        assert!(inventory.is_empty());
    }

    #[test]
    fn a_negative_remaining_count_is_ignored() {
        let mut inventory = vec![potion(5)];
        apply(&mut inventory, EntityId(1), -1, true);
        assert_eq!(inventory[0].amount(), 5);
    }
}
