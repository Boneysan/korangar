use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::event::{Event, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, RustState, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

const MAXIMUM_INPUT_LENGTH: usize = 60;

/// ZST focus ID for requested item name text box.
pub struct CommissionItemTextBox;

/// ZST focus ID for offered zeny fee text box.
pub struct CommissionFeeTextBox;

/// Internal state of the crafting commission board window.
#[derive(Default, RustState, StateElement)]
pub struct CommissionBoardWindowState {
    /// Item name requested by player (e.g. "Fire Damascus").
    item_name: String,
    /// Offered crafting fee in Zeny.
    fee_zeny: String,
}

fn post_commission<I, F>(item_path: I, fee_path: F) -> impl Fn(&State<ClientState>, &mut EventQueue<ClientState>) + 'static
where
    I: Path<ClientState, String> + Copy + 'static,
    F: Path<ClientState, String> + Copy + 'static,
{
    move |state, queue| {
        let item = state.get(&item_path).trim().to_owned();
        if item.is_empty() {
            return;
        }
        let fee_str = state.get(&fee_path).trim();
        let text = match fee_str.is_empty() {
            true => format!("/commission post {item}"),
            false => format!("/commission post {item} {fee_str}"),
        };
        state.update_value_with(item_path, |current| current.clear());
        state.update_value_with(fee_path, |current| current.clear());
        queue.queue(InputEvent::SendMessage { text });
        queue.queue(Event::Unfocus);
    }
}

fn list_commissions() -> impl Fn(&State<ClientState>, &mut EventQueue<ClientState>) + 'static {
    move |_, queue| {
        queue.queue(InputEvent::SendMessage {
            text: "/commission list".to_string(),
        });
    }
}

/// Graphical window for the Non-Custodial Crafting Commission Board (F31 / Decision D7).
pub struct CommissionBoardWindow<A> {
    commission_board_window_state: A,
}

impl<A> CommissionBoardWindow<A> {
    pub fn new(commission_board_window_state: A) -> Self {
        Self {
            commission_board_window_state,
        }
    }
}

impl<A> CustomWindow<ClientState> for CommissionBoardWindow<A>
where
    A: Path<ClientState, CommissionBoardWindowState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CommissionBoard)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let item_path = self.commission_board_window_state.item_name();
        let fee_path = self.commission_board_window_state.fee_zeny();

        window! {
            title: "Crafting Commission Board",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: "Non-custodial peer requests. Materials & payment must be traded directly.",
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: "Item Name:",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text_box! {
                    ghost_text: "e.g. Fire Damascus, White Slim Potion",
                    state: item_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_INPUT_LENGTH>::new(item_path, post_commission(item_path, fee_path)),
                    focus_id: CommissionItemTextBox,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: "Offered Fee (Zeny, optional):",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text_box! {
                    ghost_text: "e.g. 50000",
                    state: fee_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_INPUT_LENGTH>::new(fee_path, post_commission(item_path, fee_path)),
                    focus_id: CommissionFeeTextBox,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Post Request",
                            tooltip: "Post crafting request to bulletin board",
                            event: post_commission(item_path, fee_path),
                        },
                        button! {
                            text: "View Active Requests",
                            tooltip: "List active peer requests in chat",
                            event: list_commissions(),
                        },
                    ),
                },
                text! {
                    text: "Tip: Use /commission cancel <id> to withdraw a request.",
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
            ),
        }
    }
}
