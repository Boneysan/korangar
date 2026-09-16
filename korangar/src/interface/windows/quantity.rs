use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct QuantityWindow;

impl QuantityWindow {
    pub fn new(_quantity_path: impl Copy) -> Self {
        Self
    }
}

impl CustomWindow<ClientState> for QuantityWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Quantity)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Choose amount",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! { text: "Enter 1 up to the stack. Cancel sends nothing." },
                split! {
                    children: (
                        button! {
                            text: "−",
                            event: InputEvent::QuantityDecrement,
                        },
                        button! {
                            text: "+",
                            event: InputEvent::QuantityIncrement,
                        },
                        button! {
                            text: "All",
                            event: InputEvent::QuantitySetAll,
                        },
                    )
                },
                button! {
                    text: "Confirm",
                    event: InputEvent::QuantityConfirm,
                },
                button! {
                    text: "Cancel",
                    event: InputEvent::QuantityCancel,
                },
            )
        }
    }
}
