use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::RefinableWeaponInformation;

use super::selection_list::SelectionList;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct WeaponRefineWindow {
    weapons: Vec<(RefinableWeaponInformation, String)>,
}

impl WeaponRefineWindow {
    pub fn new(weapons: Vec<(RefinableWeaponInformation, String)>) -> Self {
        Self { weapons }
    }
}

impl CustomWindow<ClientState> for WeaponRefineWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::WeaponRefine)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let entries = self.weapons.into_iter().map(|(weapon, name)| {
            let text = format!("{name}  +{}", weapon.refinement_level);
            let tooltip = format!(
                "Inventory slot {}\nCards: {}, {}, {}, {}",
                weapon.inventory_index.0, weapon.cards[0].0, weapon.cards[1].0, weapon.cards[2].0, weapon.cards[3].0
            );
            let event = InputEvent::RefineWeapon {
                inventory_index: weapon.inventory_index,
            };
            (text, tooltip, event)
        });

        window! {
            title: "Select weapon to refine",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                SelectionList::new(entries),
                button! {
                    text: "Cancel",
                    event: InputEvent::CancelWeaponRefine,
                },
            ),
        }
    }
}
