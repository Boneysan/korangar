use korangar_interface::window::{CustomWindow, Window};

use crate::WorldThemePathExt;
use crate::interface::windows::WindowClass;
use crate::state::theme::{InterfaceThemeType, StatusBarThemePathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Reactive target frame for the currently selected monster.
pub struct MonsterTargetWindow;

impl CustomWindow<ClientState> for MonsterTargetWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::MonsterTarget)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Monster Target",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: client_state().targeted_monster_summary(),
                    color: client_state().world_theme().status_bar().enemy_health_color(),
                },
            ),
        }
    }
}
