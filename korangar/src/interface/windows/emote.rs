use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::EMOTION_NAMES;

const EMOTE_COLUMNS: usize = 3;
const EMOTE_ROWS: usize = EMOTION_NAMES.len() / EMOTE_COLUMNS;

/// Scrollable palette for every emote supported by the current wire table.
/// Selecting an entry sends the same request as `/e <id>`.
#[derive(Default)]
pub struct EmoteWindow;

impl CustomWindow<ClientState> for EmoteWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Emotes)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Emotes",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: scroll_view! {
                children: std::array::from_fn::<_, EMOTE_ROWS, _>(|row| {
                    split! {
                        gaps: theme().window().gaps(),
                        children: std::array::from_fn::<_, EMOTE_COLUMNS, _>(|column| {
                            let index = row * EMOTE_COLUMNS + column;
                            let name = EMOTION_NAMES[index];

                            button! {
                                text: name,
                                tooltip: format!("Play {name} [^000001/e {index}^000000]"),
                                event: InputEvent::UseEmotion { emotion: index as u8 },
                            }
                        }),
                    }
                }),
            },
        }
    }
}
