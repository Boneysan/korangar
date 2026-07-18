use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::identify::{IdentifyState, IdentifyStatePathExt};
use crate::state::theme::InterfaceThemeType;

pub struct IdentifyWindow<P> {
    identify_path: P,
}

impl<P> IdentifyWindow<P> {
    pub fn new(identify_path: P) -> Self {
        Self { identify_path }
    }
}

impl<P> CustomWindow<ClientState> for IdentifyWindow<P>
where
    P: Path<ClientState, IdentifyState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Identify)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let text_path = self.identify_path.display_text();

        window! {
            title: "Identify",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! { text: text_path },
                button! {
                    text: "Cancel",
                    event: InputEvent::IdentifyCancel,
                },
            )
        }
    }
}
