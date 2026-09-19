use ragnarok_packets::{
    EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, ItemOptions, Price, RegularItemFlags, SellItemInformation,
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

    /// Check if the item is equipped (i.e., has a non-empty equipped_position).
    /// This provides a unified accessor for both Regular and Equippable
    /// variants.
    pub fn is_equipped(&self) -> bool {
        match self {
            InventoryItemDetails::Regular { equipped_position, .. } => !equipped_position.is_empty(),
            InventoryItemDetails::Equippable { equipped_position, .. } => !equipped_position.is_empty(),
        }
    }

    /// Get the amount/quantity for this item. For Regular items, returns the
    /// stack amount. For Equippable items, also returns the real amount (which
    /// for ammo is the stack count, not hard-coded to 1).
    pub fn quantity(&self) -> u16 {
        match self {
            InventoryItemDetails::Regular { amount, .. } => *amount,
            InventoryItemDetails::Equippable { amount, .. } => *amount,
        }
    }

    /// Check if the item is equipped (i.e., has a non-empty equipped_position).
    /// This provides a unified accessor for both Regular and Equippable
    /// variants.
    pub fn equipped_position(&self) -> &EquipPosition {
        match self {
            InventoryItemDetails::Regular { equipped_position, .. } => equipped_position,
            InventoryItemDetails::Equippable { equipped_position, .. } => equipped_position,
        }
    }
}

impl<Meta> InventoryItem<Meta> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_equipped_works_for_regular_items() {
        // Un-equipped regular item
        let regular_unequipped = InventoryItemDetails::Regular {
            amount: 5,
            equipped_position: EquipPosition::empty(),
            flags: RegularItemFlags::empty(),
        };
        assert!(!regular_unequipped.is_equipped());

        // Equipped regular item (in RIGHT_HAND slot)
        let regular_equipped = InventoryItemDetails::Regular {
            amount: 1,
            equipped_position: EquipPosition::RIGHT_HAND,
            flags: RegularItemFlags::empty(),
        };
        assert!(regular_equipped.is_equipped());
    }

    #[test]
    fn is_equipped_works_for_equippable_items() {
        // Un-equipped equippable item (ammo on ground)
        let equippable_unequipped = InventoryItemDetails::Equippable {
            amount: 10,
            equip_position: EquipPosition::empty(),
            equipped_position: EquipPosition::empty(),
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 0,
            option_data: [ItemOptions::default(); 5],
            refinement_level: 0,
            enchantment_level: 0,
            flags: EquippableItemFlags::empty(),
        };
        assert!(!equippable_unequipped.is_equipped());

        // Equipped equippable item (ammo in slot)
        let equippable_equipped = InventoryItemDetails::Equippable {
            amount: 10,
            equip_position: EquipPosition::AMMO,
            equipped_position: EquipPosition::AMMO,
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 0,
            option_data: [ItemOptions::default(); 5],
            refinement_level: 0,
            enchantment_level: 0,
            flags: EquippableItemFlags::empty(),
        };
        assert!(equippable_equipped.is_equipped());
    }

    #[test]
    fn quantity_preserves_amount_for_both_variants() {
        // Regular item with stack amount
        let regular = InventoryItemDetails::Regular {
            amount: 42,
            equipped_position: EquipPosition::empty(),
            flags: RegularItemFlags::empty(),
        };
        assert_eq!(regular.quantity(), 42);

        // Equippable item (ammo) with stack count - should NOT be forced to 1
        let equippable = InventoryItemDetails::Equippable {
            amount: 99,
            equip_position: EquipPosition::AMMO,
            equipped_position: EquipPosition::empty(),
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 0,
            option_data: [ItemOptions::default(); 5],
            refinement_level: 0,
            enchantment_level: 0,
            flags: EquippableItemFlags::empty(),
        };
        assert_eq!(equippable.quantity(), 99);
    }

    #[test]
    fn equipped_position_provides_unified_accessor() {
        let regular = InventoryItemDetails::Regular {
            amount: 1,
            equipped_position: EquipPosition::RIGHT_HAND,
            flags: RegularItemFlags::empty(),
        };
        assert_eq!(*regular.equipped_position(), EquipPosition::RIGHT_HAND);

        let equippable = InventoryItemDetails::Equippable {
            amount: 50,
            equip_position: EquipPosition::AMMO,
            equipped_position: EquipPosition::AMMO,
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 0,
            option_data: [ItemOptions::default(); 5],
            refinement_level: 0,
            enchantment_level: 0,
            flags: EquippableItemFlags::empty(),
        };
        assert_eq!(*equippable.equipped_position(), EquipPosition::AMMO);
    }

    #[test]
    fn can_sell_item_filters_equipped_items() {
        // Regular equipped item - cannot sell
        let regular_equipped = InventoryItem {
            metadata: NoMetadata,
            index: InventoryIndex(0),
            item_id: ItemId(501),
            item_type: 2,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Regular {
                amount: 1,
                equipped_position: EquipPosition::RIGHT_HAND,
                flags: RegularItemFlags::empty(),
            },
        };
        assert!(!can_sell_item(&regular_equipped));

        // Equippable unequipped item - can sell (ammo on ground)
        let equippable_unequipped = InventoryItem {
            metadata: NoMetadata,
            index: InventoryIndex(1),
            item_id: ItemId(502),
            item_type: 10, // IT_AMMO
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Equippable {
                amount: 99,
                equip_position: EquipPosition::AMMO,
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: 0,
                option_data: [ItemOptions::default(); 5],
                refinement_level: 0,
                enchantment_level: 0,
                flags: EquippableItemFlags::empty(),
            },
        };
        assert!(can_sell_item(&equippable_unequipped));
    }

    #[test]
    fn filter_sell_items_filters_equipped_and_preserves_stacks() {
        // 1. Regular stack: Red Potion x15, unequipped
        let regular_stack = InventoryItem {
            metadata: "red_potion",
            index: InventoryIndex(1),
            item_id: ItemId(501),
            item_type: 0,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Regular {
                amount: 15,
                equipped_position: EquipPosition::empty(),
                flags: RegularItemFlags::empty(),
            },
        };

        // 2. Unequipped gear: Dagger x1, unequipped
        let unequipped_gear = InventoryItem {
            metadata: "dagger",
            index: InventoryIndex(2),
            item_id: ItemId(1201),
            item_type: 4, // IT_WEAPON
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Equippable {
                amount: 1,
                equip_position: EquipPosition::RIGHT_HAND,
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: 0,
                option_data: [ItemOptions::default(); 5],
                refinement_level: 0,
                enchantment_level: 0,
                flags: EquippableItemFlags::empty(),
            },
        };

        // 3. Equipped gear: Sword x1, equipped in RIGHT_HAND
        let equipped_gear = InventoryItem {
            metadata: "sword",
            index: InventoryIndex(3),
            item_id: ItemId(1101),
            item_type: 4, // IT_WEAPON
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Equippable {
                amount: 1,
                equip_position: EquipPosition::RIGHT_HAND,
                equipped_position: EquipPosition::RIGHT_HAND,
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: 0,
                option_data: [ItemOptions::default(); 5],
                refinement_level: 0,
                enchantment_level: 0,
                flags: EquippableItemFlags::empty(),
            },
        };

        // 4. Equipped ammo: Silver Arrow x500, equipped in AMMO slot
        let equipped_ammo = InventoryItem {
            metadata: "silver_arrow",
            index: InventoryIndex(4),
            item_id: ItemId(1751),
            item_type: IT_AMMO,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Equippable {
                amount: 500,
                equip_position: EquipPosition::AMMO,
                equipped_position: EquipPosition::AMMO,
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: 0,
                option_data: [ItemOptions::default(); 5],
                refinement_level: 0,
                enchantment_level: 0,
                flags: EquippableItemFlags::empty(),
            },
        };

        // 5. Unequipped ammo: Arrow x350, unequipped
        let unequipped_ammo = InventoryItem {
            metadata: "arrow",
            index: InventoryIndex(5),
            item_id: ItemId(1750),
            item_type: IT_AMMO,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Equippable {
                amount: 350,
                equip_position: EquipPosition::AMMO,
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: 0,
                option_data: [ItemOptions::default(); 5],
                refinement_level: 0,
                enchantment_level: 0,
                flags: EquippableItemFlags::empty(),
            },
        };

        let inventory = vec![regular_stack, unequipped_gear, equipped_gear, equipped_ammo, unequipped_ammo];

        let incoming_sell_list = vec![
            SellItemInformation {
                inventory_index: InventoryIndex(1),
                price: Price(50),
                overcharge_price: Price(62),
            },
            SellItemInformation {
                inventory_index: InventoryIndex(2),
                price: Price(100),
                overcharge_price: Price(124),
            },
            SellItemInformation {
                inventory_index: InventoryIndex(3),
                price: Price(500),
                overcharge_price: Price(620),
            },
            SellItemInformation {
                inventory_index: InventoryIndex(4),
                price: Price(3),
                overcharge_price: Price(3),
            },
            SellItemInformation {
                inventory_index: InventoryIndex(5),
                price: Price(1),
                overcharge_price: Price(1),
            },
            SellItemInformation {
                inventory_index: InventoryIndex(99),
                price: Price(999),
                overcharge_price: Price(999),
            },
        ];

        let filtered = filter_sell_items(incoming_sell_list, &inventory);

        // Exactly 3 items should pass: regular stack, unequipped gear, unequipped ammo
        assert_eq!(filtered.len(), 3);

        // Case 1: regular stack preserves amount 15
        assert_eq!(filtered[0].inventory_index, InventoryIndex(1));
        assert_eq!(filtered[0].metadata, ("red_potion", 15));
        assert_eq!(filtered[0].price, Price(50));
        assert_eq!(filtered[0].overcharge_price, Price(62));

        // Case 2: unequipped gear has amount 1
        assert_eq!(filtered[1].inventory_index, InventoryIndex(2));
        assert_eq!(filtered[1].metadata, ("dagger", 1));
        assert_eq!(filtered[1].price, Price(100));

        // Case 3: unequipped ammo preserves stack count 350
        assert_eq!(filtered[2].inventory_index, InventoryIndex(5));
        assert_eq!(filtered[2].metadata, ("arrow", 350));
        assert_eq!(filtered[2].price, Price(1));
    }
}

/// Helper function to check if an inventory item can be sold.
///
/// Returns `false` if the item is equipped (cannot be sold).
pub fn can_sell_item<Meta>(inventory_item: &InventoryItem<Meta>) -> bool {
    !inventory_item.details.is_equipped()
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

/// Filter offered items from a vendor sell-list against the player's inventory.
///
/// Any item that is currently equipped is excluded from sale.
/// The item's real quantity is preserved across both regular and equippable
/// (e.g. ammo) items. Items in `items` with no matching inventory index are
/// omitted.
pub fn filter_sell_items<Meta: Clone>(
    items: impl IntoIterator<Item = SellItemInformation>,
    inventory_items: &[InventoryItem<Meta>],
) -> Vec<SellItem<(Meta, u16)>> {
    items
        .into_iter()
        .filter_map(|item| {
            let inventory_item = inventory_items
                .iter()
                .find(|inventory_item| inventory_item.index == item.inventory_index)?;

            if !inventory_item.details.equipped_position().is_empty() {
                return None;
            }

            let quantity = inventory_item.details.quantity();

            Some(SellItem {
                metadata: (inventory_item.metadata.clone(), quantity),
                inventory_index: item.inventory_index,
                price: item.price,
                overcharge_price: item.overcharge_price,
            })
        })
        .collect()
}
