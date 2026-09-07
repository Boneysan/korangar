use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::settings::{GameSettings, GameSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

#[derive(Default)]
pub struct GameSettingsWindow<A> {
    game_settings_path: A,
}

impl<A> GameSettingsWindow<A> {
    pub fn new(game_settings_path: A) -> Self {
        Self { game_settings_path }
    }
}

impl<A> CustomWindow<ClientState> for GameSettingsWindow<A>
where
    A: Path<ClientState, GameSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::GameSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().game_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                state_button! {
                    text: client_state().localization().auto_attack_button_text(),
                    state: self.game_settings_path.auto_attack(),
                    event: Toggle(self.game_settings_path.auto_attack()),
                },
                state_button! {
                    text: client_state().localization().show_minimap_button_text(),
                    state: self.game_settings_path.show_minimap(),
                    event: InputEvent::ToggleMinimapWindow,
                },
                state_button! {
                    text: "WASD movement",
                    tooltip: "Walk with W A S D relative to the camera. Click-to-move still works.",
                    state: self.game_settings_path.wasd_movement(),
                    event: Toggle(self.game_settings_path.wasd_movement()),
                },
                // The one toggle here the client does not own. The server keeps
                // this setting -- per character, across sessions -- so the
                // button reads the server's answer and asks it to change, rather
                // than flipping a local value the server has never heard of. In
                // a party the server refuses to turn it off, and says so; the
                // button then simply stays on, which is the truth.
                state_button! {
                    text: "Automatic pickup",
                    tooltip: "Loot within two squares goes straight into your bag. Kept by the server; in a party it stays on for everyone [^000001@autopickup^000000]",
                    state: client_state().auto_pickup(),
                    event: |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        let text = match *state.get(&client_state().auto_pickup()) {
                            true => "@autopickup 0".to_owned(),
                            false => "@autopickup 2".to_owned(),
                        };

                        queue.queue(InputEvent::SendMessage { text });
                    },
                },
            ),
        }
    }
}
