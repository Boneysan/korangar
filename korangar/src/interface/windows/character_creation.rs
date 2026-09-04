use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::character_creation::{CharacterCreation, CharacterCreationPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

pub struct CharacterCreationWindow<A, B> {
    character_name_path: A,
    creation_path: B,
    slot: usize,
}

impl<A, B> CharacterCreationWindow<A, B> {
    pub fn new(character_name_path: A, creation_path: B, slot: usize) -> Self {
        Self {
            character_name_path,
            creation_path,
            slot,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for CharacterCreationWindow<A, B>
where
    A: Path<ClientState, String>,
    B: Path<ClientState, CharacterCreation>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterCreation)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct CharacterName;

        let disabled = ComputedSelector::new_default(move |state: &ClientState| {
            self.character_name_path.follow_safe(state).len() < MINIMUM_NAME_LENGTH
        });

        let create_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&self.character_name_path).clone();
            let sex = *state.get(&self.creation_path.sex());
            let hair_style = *state.get(&self.creation_path.hair_style());

            queue.queue(InputEvent::CreateCharacter {
                slot: self.slot,
                name,
                sex,
                hair_style,
            });
        };

        window! {
            title: client_state().localization().create_character_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            closable: true,
            elements: (
                text_box! {
                    ghost_text: client_state().localization().character_name_text(),
                    state: self.character_name_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.character_name_path, create_action),
                    focus_id: CharacterName,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    children: (
                        text! {
                            text: "Sex",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        drop_down! {
                            selected: self.creation_path.sex(),
                            options: self.creation_path.sexes(),
                        }
                    )
                },
                split! {
                    children: (
                        text! {
                            text: "Hair style",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        drop_down! {
                            selected: self.creation_path.hair_style(),
                            options: self.creation_path.hair_styles(),
                        }
                    )
                },
                button! {
                    text: client_state().localization().create_character_button_text(),
                    disabled,
                    disabled_tooltip: client_state().localization().create_character_button_tooltip(),
                    event: create_action,
                }
            ),
        }
    }
}
