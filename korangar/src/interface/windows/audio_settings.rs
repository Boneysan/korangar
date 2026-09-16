use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use crate::interface::windows::WindowClass;
use crate::settings::{AudioSettings, AudioSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

#[derive(Default)]
pub struct AudioSettingsWindow<A> {
    audio_settings_path: A,
}

impl<A> AudioSettingsWindow<A> {
    pub fn new(audio_settings_path: A) -> Self {
        Self { audio_settings_path }
    }
}

impl<A> CustomWindow<ClientState> for AudioSettingsWindow<A>
where
    A: Path<ClientState, AudioSettings> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::AudioSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let audio_path = self.audio_settings_path;

        window! {
            title: client_state().localization().audio_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                state_button! {
                    text: client_state().localization().mute_audio_on_focus_loss_button_text(),
                    state: self.audio_settings_path.mute_on_focus_loss(),
                    event: Toggle(self.audio_settings_path.mute_on_focus_loss()),
                },
                state_button! {
                    text: "Enable UI sound effects",
                    state: self.audio_settings_path.ui_sound_enabled(),
                    event: Toggle(self.audio_settings_path.ui_sound_enabled()),
                },
                button! {
                    text: "Cycle UI sound volume",
                    event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                        state.update_value_with(audio_path, |settings| {
                            let current = settings.ui_sound_volume;
                            let next = if current > 0.75 {
                                0.75
                            } else if current > 0.50 {
                                0.50
                            } else if current > 0.25 {
                                0.25
                            } else if current > 0.01 {
                                0.0
                            } else {
                                1.0
                            };
                            settings.set_ui_sound_volume(next);
                        });
                    },
                },
            ),
        }
    }
}
