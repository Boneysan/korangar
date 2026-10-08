use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use rust_state::{PathExt, State};

use crate::input::InputEvent;
use crate::interface::resource::ItemSource;
use crate::interface::windows::WindowClass;
use crate::state::inventory::InventoryPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::ResourceMetadata;

const MAXIMUM_SPLIT_DIGITS: usize = 5;

pub struct SplitAmountTextBox;

/// Compact right-click menu for an inventory item, including server-backed
/// stack splitting.
pub struct ItemActionsWindow {
    item: InventoryItem<ResourceMetadata>,
    protected: bool,
}

impl ItemActionsWindow {
    pub fn new(item: InventoryItem<ResourceMetadata>, protected: bool) -> Self {
        Self { item, protected }
    }
}

/// Queue an action, then close this popup.
struct ActionThenClose(InputEvent);

impl ClickHandler<ClientState> for ActionThenClose {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(self.0.clone());
        queue.queue(InputEvent::CloseItemActions);
    }
}

impl CustomWindow<ClientState> for ItemActionsWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ItemActions)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let inventory_index = self.item.index;
        let amount = inventory_item_amount(&self.item);
        let name = self.item.metadata.name.clone();
        let item_id = self.item.item_id;
        let protection_action = ActionThenClose(InputEvent::ToggleItemProtection(item_id));
        let protection_label = if self.protected {
            "Allow drop and sale"
        } else {
            "Protect from drop and sale"
        };
        let primary_label = primary_action_label(&self.item);
        let primary_event = ActionThenClose(primary_action_event(&self.item));
        let add_to_hotbar = ActionThenClose(InputEvent::AssignItemToHotbar {
            item_id: self.item.item_id,
        });
        let can_add_to_hotbar = consumable_can_be_assigned(&self.item);

        // One selector per button: `ComputedSelector` is not `Copy`.
        let trade_disabled =
            ComputedSelector::new_default(|state: &ClientState| !client_state().trade_state().follow_safe(state).is_active());
        let trade_one_disabled =
            ComputedSelector::new_default(|state: &ClientState| !client_state().trade_state().follow_safe(state).is_active());

        let half = (amount / 2).max(1);
        let can_split = amount > 1;
        let split_amount_path = client_state().inventory().split_amount();

        let split_half = ActionThenClose(InputEvent::SplitInventoryStack {
            inventory_index,
            amount: half,
        });
        let split_one = ActionThenClose(InputEvent::SplitInventoryStack {
            inventory_index,
            amount: 1,
        });
        let split_custom = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let Ok(split_amount) = state.get(&split_amount_path).trim().parse::<u16>() else {
                return;
            };
            if split_amount == 0 || split_amount >= amount {
                return;
            }
            queue.queue(InputEvent::SplitInventoryStack {
                inventory_index,
                amount: split_amount,
            });
            state.update_value_with(split_amount_path, |value| value.clear());
            queue.queue(Event::Unfocus);
            queue.queue(InputEvent::CloseItemActions);
        };
        let drop_all = ActionThenClose(InputEvent::DropItem { inventory_index, amount });

        // Adding to a trade previously required typing `/trade add
        // <inventory_index>`, and an inventory index is an internal number no
        // player can see. This menu already has it.
        let trade_all = ActionThenClose(InputEvent::TradeAddItem {
            inventory_index,
            amount: u32::from(amount),
        });
        let trade_one = ActionThenClose(InputEvent::TradeAddItem {
            inventory_index,
            amount: 1,
        });

        window! {
            title: name,
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                button! {
                    text: primary_label,
                    event: primary_event,
                },
                button! {
                    text: "Add to hotbar",
                    tooltip: "Puts this consumable on the hotbar. If it is already there, that slot is replaced. Drag it onto a slot to replace a different one.",
                    disabled: !can_add_to_hotbar,
                    disabled_tooltip: "Only a usable item can be added from this menu. Drag it onto a slot to replace one.",
                    event: add_to_hotbar,
                },
                button! {
                    text: protection_label,
                    tooltip: "Applies to this item type on this character.",
                    event: protection_action,
                },
                button! {
                    text: format!("Split half into inventory ({half})"),
                    disabled: !can_split,
                    disabled_tooltip: "Need a stack of 2+ to split",
                    event: split_half,
                },
                button! {
                    text: "Split 1 into inventory",
                    disabled: !can_split,
                    disabled_tooltip: "Need a stack of 2+ to split",
                    event: split_one,
                },
                text_box! {
                    ghost_text: "Amount to split into inventory",
                    state: split_amount_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_SPLIT_DIGITS>::new(split_amount_path, Event::Unfocus),
                    focus_id: SplitAmountTextBox,
                },
                text! {
                    text: if can_split {
                        format!("Enter 1–{}; both stacks stay in inventory.", amount - 1)
                    } else {
                        "This item stack cannot be split.".to_owned()
                    },
                },
                button! {
                    text: "Split amount into inventory",
                    disabled: !can_split,
                    disabled_tooltip: "Need a stack of 2+ to split",
                    event: split_custom,
                },
                button! {
                    text: if amount > 1 {
                        format!("Drop all ({amount})")
                    } else {
                        "Drop".to_owned()
                    },
                    event: drop_all,
                },
                button! {
                    text: if amount > 1 {
                        format!("Add all to trade ({amount})")
                    } else {
                        "Add to trade".to_owned()
                    },
                    disabled: trade_disabled,
                    disabled_tooltip: "No trade is open",
                    event: trade_all,
                },
                button! {
                    text: "Add 1 to trade",
                    disabled: trade_one_disabled,
                    disabled_tooltip: "No trade is open",
                    event: trade_one,
                },
                button! {
                    text: "Cancel",
                    event: InputEvent::CloseItemActions,
                },
            ),
        }
    }
}

/// Right-click "Add to hotbar" is for usable stacks (potions and the like).
/// Gear stays a drag onto the slot the player wants to replace.
pub fn consumable_can_be_assigned(item: &InventoryItem<ResourceMetadata>) -> bool {
    item.is_identified() && matches!(item.details, InventoryItemDetails::Regular { .. })
}

fn primary_action_label(item: &InventoryItem<ResourceMetadata>) -> &'static str {
    if !item.is_identified() {
        return "Identify";
    }

    match &item.details {
        InventoryItemDetails::Regular { .. } => "Use",
        InventoryItemDetails::Equippable { equipped_position, .. } => {
            if equipped_position.is_empty() {
                "Equip"
            } else {
                "Unequip"
            }
        }
    }
}

fn primary_action_event(item: &InventoryItem<ResourceMetadata>) -> InputEvent {
    let inventory_index = item.index;

    if !item.is_identified() {
        return InputEvent::IdentifyItem { inventory_index };
    }

    match &item.details {
        InventoryItemDetails::Regular { .. } => InputEvent::UseItem { inventory_index },
        InventoryItemDetails::Equippable {
            equip_position,
            equipped_position,
            ..
        } => {
            if equipped_position.is_empty() {
                InputEvent::MoveItem {
                    source: ItemSource::Inventory,
                    destination: ItemSource::Equipment { position: *equip_position },
                    item: item.clone(),
                }
            } else {
                InputEvent::MoveItem {
                    source: ItemSource::Equipment {
                        position: *equipped_position,
                    },
                    destination: ItemSource::Inventory,
                    item: item.clone(),
                }
            }
        }
    }
}

/// Amount to drop for a full-stack drop of this inventory item.
pub fn inventory_item_amount(item: &InventoryItem<ResourceMetadata>) -> u16 {
    match &item.details {
        InventoryItemDetails::Regular { amount, .. } => *amount,
        InventoryItemDetails::Equippable { amount, .. } => *amount,
    }
}

#[cfg(test)]
mod tests {
    use korangar_networking::{InventoryItem, InventoryItemDetails};
    use ragnarok_packets::{EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, ItemOptions, RegularItemFlags};

    use super::consumable_can_be_assigned;
    use crate::world::ResourceMetadata;

    fn item(details: InventoryItemDetails) -> InventoryItem<ResourceMetadata> {
        InventoryItem {
            metadata: ResourceMetadata {
                texture: None,
                name: "Red Potion".to_owned(),
            },
            index: InventoryIndex(0),
            item_id: ItemId(501),
            item_type: 0,
            slot: [0; 4],
            hire_expiration_date: 0,
            details,
        }
    }

    fn identified_potion() -> InventoryItem<ResourceMetadata> {
        item(InventoryItemDetails::Regular {
            amount: 5,
            equipped_position: EquipPosition::empty(),
            flags: RegularItemFlags::IDENTIFIED,
        })
    }

    #[test]
    fn a_usable_stack_can_be_assigned_and_gear_cannot() {
        assert!(consumable_can_be_assigned(&identified_potion()));

        let mut hidden = RegularItemFlags::empty();
        hidden.set(RegularItemFlags::IDENTIFIED, false);
        let unidentified = item(InventoryItemDetails::Regular {
            amount: 1,
            equipped_position: EquipPosition::empty(),
            flags: hidden,
        });
        assert!(!consumable_can_be_assigned(&unidentified));

        let mut flags = EquippableItemFlags::empty();
        flags.set(EquippableItemFlags::IDENTIFIED, true);
        let sword = item(InventoryItemDetails::Equippable {
            amount: 1,
            equip_position: EquipPosition::RIGHT_HAND,
            equipped_position: EquipPosition::empty(),
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 0,
            option_data: [ItemOptions::default(); 5],
            refinement_level: 0,
            enchantment_level: 0,
            flags,
        });
        assert!(!consumable_can_be_assigned(&sword));
    }
}
