use korangar_interface::window::{CustomWindow, Window};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::FontSize;
use crate::state::theme::InterfaceThemeType;
use crate::state::ClientState;

/// Explains a disconnect the *server* initiated — a kick, a ban, a shutdown, or
/// someone else logging into the account.
///
/// Distinct from [`super::ErrorWindow`] because this one has to be acknowledged:
/// the connection is already gone by the time it appears (`clif_authfail_fd`
/// closes the socket in the same breath as sending the reason), so the player is
/// being told what already happened, and the button drops the remaining
/// connection rather than pretending the session can continue.
pub struct DisconnectNoticeWindow {
    message: String,
}

impl DisconnectNoticeWindow {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl CustomWindow<ClientState> for DisconnectNoticeWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::DisconnectNotice)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // Not closable: the only way out is the button, so the disconnect is
        // always acknowledged rather than dismissed into an ambiguous state.
        window! {
            title: "Disconnected",
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            closable: false,
            resizable: false,
            minimum_width: 360.0,
            maximum_width: 520.0,
            minimum_height: 140.0,
            elements: (
                text! {
                    text: self.message,
                    color: Color::rgb_u8(255, 90, 90),
                    height: 40.0,
                    font_size: FontSize(18.0),
                },
                button! {
                    text: "OK",
                    event: InputEvent::AcknowledgeDisconnect,
                },
            ),
        }
    }
}
