use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::state::party::{PartyState, PartyStatePathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::ClientState;

/// Simple party roster bound to [`PartyState::display_text`].
pub struct PartyWindow<P> {
    party_path: P,
}

impl<P> PartyWindow<P> {
    pub fn new(party_path: P) -> Self {
        Self { party_path }
    }
}

impl<P> CustomWindow<ClientState> for PartyWindow<P>
where
    P: Path<ClientState, PartyState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Party)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let text_path = self.party_path.display_text();

        window! {
            title: "Party",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: text_path,
                },
            )
        }
    }
}
