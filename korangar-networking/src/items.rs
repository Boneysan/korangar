use ragnarok_packets::{EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, ItemOptions, Price, RegularItemFlags};

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
