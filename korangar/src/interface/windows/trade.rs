use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::theme::InterfaceThemeType;
use crate::state::trade::{TradeState, TradeStatePathExt};
use crate::state::ClientState;

/// Active trade window (after accept / when we initiated and partner accepted).
pub struct TradeWindow<P> {
    trade_path: P,
}

impl<P> TradeWindow<P> {
    pub fn new(trade_path: P) -> Self {
        Self { trade_path }
    }
}

impl<P> CustomWindow<ClientState> for TradeWindow<P>
where
    P: Path<ClientState, TradeState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Trade)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let text_path = self.trade_path.display_text();

        window! {
            title: "Trade",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! { text: text_path },
                text! {
                    text: "Add item: /trade add <inv_index> [amount]\nAdd zeny: /trade zeny <amount>",
                },
                button! {
                    text: "Lock offer",
                    event: InputEvent::TradeOk,
                },
                button! {
                    text: "Confirm trade",
                    event: InputEvent::TradeCommit,
                },
                button! {
                    text: "Cancel",
                    event: InputEvent::TradeCancel,
                },
            )
        }
    }
}

/// Incoming trade request accept/reject.
pub struct TradeRequestWindow;

impl CustomWindow<ClientState> for TradeRequestWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::TradeRequest)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Trade request",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! { text: "A player wants to trade with you." },
                button! {
                    text: "Accept",
                    event: InputEvent::TradeAccept,
                },
                button! {
                    text: "Reject",
                    event: InputEvent::TradeReject,
                },
            )
        }
    }
}
