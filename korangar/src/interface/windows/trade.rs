use korangar_interface::window::{CustomWindow, Window};
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::event::{Event, EventQueue};
use rust_state::{Path, RustState, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::state::theme::InterfaceThemeType;
use crate::state::trade::{TradeState, TradeStatePathExt};

/// Active trade window (after accept / when we initiated and partner accepted).
/// Zeny amounts top out well below `u32::MAX`; ten digits is plenty and stops
/// a paste from overflowing the parse.
const MAXIMUM_ZENY_DIGITS: usize = 10;

/// ZST for the zeny field's focus id.
pub struct TradeZenyTextBox;

/// Internal state of the trade window.
#[derive(Default, RustState, StateElement)]
pub struct TradeWindowState {
    zeny_input: String,
}

pub struct TradeWindow<A, B> {
    window_state_path: A,
    trade_path: B,
}

impl<A, B> TradeWindow<A, B> {
    pub fn new(window_state_path: A, trade_path: B) -> Self {
        Self {
            window_state_path,
            trade_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for TradeWindow<A, B>
where
    A: Path<ClientState, TradeWindowState> + Copy + 'static,
    B: Path<ClientState, TradeState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Trade)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let text_path = self.trade_path.display_text();
        let zeny_path = self.window_state_path.zeny_input();

        // Non-numeric input is ignored rather than reported: the field is free
        // text and a stray character should not cost the player a trade.
        let add_zeny = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let Ok(amount) = state.get(&zeny_path).trim().parse::<u32>() else {
                return;
            };
            if amount > 0 {
                state.update_value_with(zeny_path, |input| input.clear());
                queue.queue(InputEvent::TradeAddZeny { amount });
                queue.queue(Event::Unfocus);
            }
        };

        window! {
            title: "Trade",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // **Deliberately not closable, and this one bricks trading for the
            // rest of the session if it is.** `trade.c:185` sets
            // `sd->state.trading = 1` on *both* players when a trade starts, and
            // only `CZ_CANCEL_EXCHANGE_ITEM` clears it. The framework's close
            // button sends nothing, so closing this window left the flag set --
            // whereupon `clif_parse_TradeRequest` (`clif.c`) returns on
            // `sd->state.trading` **before generating any response at all**. Every
            // later trade attempt, in either direction, silently does nothing:
            // no packet back, so nothing for the client to report. Cancel is the
            // way out, and it tells the server. `TradeCancelled` and
            // `TradeCompleted` still close this window, so it cannot strand.
            closable: false,
            elements: (
                text! { text: text_path },
                text! {
                    text: "Right-click an inventory item to add it.",
                },
                text_box! {
                    ghost_text: "Zeny to offer",
                    state: zeny_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_ZENY_DIGITS>::new(zeny_path, add_zeny),
                    focus_id: TradeZenyTextBox,
                },
                button! {
                    text: "Add zeny",
                    tooltip: "Put the amount above into the trade",
                    event: add_zeny,
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
///
/// Names the requester and their level, both of which ride on
/// `ZC_REQ_EXCHANGE_ITEM` and were already being stored in `TradeState` -- this
/// window simply used to ignore them and say "A player wants to trade with
/// you." No protocol work was needed, unlike the party invite, whose sender
/// name genuinely is not on the wire.
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
            // **Deliberately not closable: a trade request must be answered.**
            // The framework's close button only closes the window -- there is no
            // close hook to hang a packet on -- so dismissing it sent no reply
            // while Hercules had already set `trade_partner` on *both* sides
            // (`trade.c` `trade_request`). The pair stayed locked with nothing on
            // screen to say so, and the client's own `pending` state was never
            // cleared either. Reject is the close button, and it tells the server.
            //
            // This cannot strand the popup: `TradeCancelled` (`ZC_CANCEL_EXCHANGE_ITEM`)
            // closes it, which is what arrives if the requester cancels or logs out.
            closable: false,
            elements: (
                text! { text: client_state().trade_state().request_text() },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Whisper",
                            tooltip: "Ask what they want before deciding",
                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                let character_name = state.get(&client_state().trade_state()).pending_name().to_owned();
                                if !character_name.is_empty() {
                                    queue.queue(InputEvent::StartWhisper { character_name });
                                }
                            },
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Reject",
                            event: InputEvent::TradeReject,
                        },
                        button! {
                            text: "Accept",
                            event: InputEvent::TradeAccept,
                        },
                    ),
                },
            )
        }
    }
}
