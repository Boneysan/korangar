use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::SkillId;

use super::selection_list::SelectionList;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct WarpSelectionWindow {
    skill_id: SkillId,
    destinations: Vec<String>,
}

impl WarpSelectionWindow {
    pub fn new(skill_id: SkillId, destinations: Vec<String>) -> Self {
        Self { skill_id, destinations }
    }
}

impl CustomWindow<ClientState> for WarpSelectionWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::WarpSelection)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let entries = self.destinations.into_iter().map(|map_name| {
            let display_name = map_name.trim_end_matches(".gat").to_owned();
            let tooltip = format!("Warp to {map_name}");
            let event = InputEvent::SelectWarpDestination {
                skill_id: self.skill_id,
                map_name,
            };
            (display_name, tooltip, event)
        });

        window! {
            title: "Select warp destination",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                SelectionList::new(entries),
                button! {
                    text: "Cancel",
                    event: InputEvent::CancelWarpSelection { skill_id: self.skill_id },
                },
            ),
        }
    }
}
