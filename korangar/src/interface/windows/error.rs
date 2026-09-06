use korangar_interface::window::{CustomWindow, Window};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct ErrorWindow {
    message: String,
}

impl ErrorWindow {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl CustomWindow<ClientState> for ErrorWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Error)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // Fixed class + natural height (not resizable) so a failed login is
        // never an invisible zero-width / clipped popup. Login failures also
        // write into LoginWindowState.status_message.
        window! {
            title: client_state().localization().error_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            closable: true,
            resizable: false,
            minimum_width: 360.0,
            maximum_width: 520.0,
            minimum_height: 120.0,
            elements: (
                text! {
                    text: self.message,
                    color: Color::rgb_u8(255, 90, 90),
                    font_size: FontSize(18.0),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
            ),
        }
    }
}

/// Centered popup when the login server refuses this pack as too old.
/// Separate from [`ErrorWindow`] so the title is "Out of date" and an OK
/// button is obvious — a red line on the login form is easy to miss.
pub struct OutdatedClientWindow {
    message: String,
}

impl OutdatedClientWindow {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl CustomWindow<ClientState> for OutdatedClientWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Error)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Out of date",
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            closable: true,
            resizable: false,
            minimum_width: 420.0,
            maximum_width: 560.0,
            minimum_height: 180.0,
            elements: (
                text! {
                    text: self.message,
                    color: Color::rgb_u8(255, 90, 90),
                    font_size: FontSize(18.0),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                button! {
                    text: "OK",
                    event: InputEvent::CloseTopWindow,
                },
            ),
        }
    }
}
