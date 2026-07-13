use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::RepairableItemInformation;

use super::selection_list::SelectionList;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct RepairWeaponWindow {
    items: Vec<(RepairableItemInformation, String)>,
}

impl RepairWeaponWindow {
    pub fn new(items: Vec<(RepairableItemInformation, String)>) -> Self {
        Self { items }
    }
}

impl CustomWindow<ClientState> for RepairWeaponWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::RepairWeapon)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let entries = self.items.into_iter().map(|(item, name)| {
            let text = format!("{name}  +{}", item.refinement_level);
            let tooltip = format!(
                "Inventory slot {}\nCards: {}, {}, {}, {}",
                item.inventory_index.0, item.cards[0].0, item.cards[1].0, item.cards[2].0, item.cards[3].0
            );
            (text, tooltip, InputEvent::RepairItem { item })
        });

        window! {
            title: "Select equipment to repair",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                SelectionList::new(entries),
                button! {
                    text: "Cancel",
                    event: InputEvent::CancelItemRepair,
                },
            ),
        }
    }
}
