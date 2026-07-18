use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use rust_state::State;

use crate::input::InputEvent;
use crate::interface::resource::ItemSource;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::ResourceMetadata;

/// Compact right-click menu for an inventory item.
///
/// Stack "split" on Hercules is done by dropping a partial amount: the
/// remainder stays in inventory as its own stack, and the dropped pile is on
/// the ground (pick it up later, or leave it split off).
pub struct ItemActionsWindow {
    item: InventoryItem<ResourceMetadata>,
}

impl ItemActionsWindow {
    pub fn new(item: InventoryItem<ResourceMetadata>) -> Self {
        Self { item }
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
        let primary_label = primary_action_label(&self.item);
        let primary_event = ActionThenClose(primary_action_event(&self.item));

        let half = (amount / 2).max(1);
        let can_split = amount > 1;

        let split_half = ActionThenClose(InputEvent::DropItem {
            inventory_index,
            amount: half,
        });
        let split_one = ActionThenClose(InputEvent::DropItem {
            inventory_index,
            amount: 1,
        });
        let drop_all = ActionThenClose(InputEvent::DropItem { inventory_index, amount });

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
                    text: if can_split {
                        format!("Split half ({half})")
                    } else {
                        "Split half".to_owned()
                    },
                    disabled: !can_split,
                    disabled_tooltip: "Need a stack of 2+ to split",
                    event: split_half,
                },
                button! {
                    text: "Split off 1",
                    disabled: !can_split,
                    disabled_tooltip: "Need a stack of 2+ to split",
                    event: split_one,
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
                    text: "Cancel",
                    event: InputEvent::CloseItemActions,
                },
            ),
        }
    }
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
        InventoryItemDetails::Equippable { .. } => 1,
    }
}
