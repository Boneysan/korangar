use std::cell::UnsafeCell;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::event::{Event, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, RustState, Selector, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::commission_board::CommissionBoardState;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

const MAXIMUM_INPUT_LENGTH: usize = 60;

/// ZST focus ID for requested item name text box.
pub struct CommissionItemTextBox;

/// ZST focus ID for offered zeny fee text box.
pub struct CommissionFeeTextBox;

/// ZST focus ID for the request id text box.
pub struct CommissionIdTextBox;

/// ZST focus ID for the crafter name text box.
pub struct CommissionCrafterTextBox;

/// Internal state of the crafting commission board window.
#[derive(Default, RustState, StateElement)]
pub struct CommissionBoardWindowState {
    /// Item name requested by player (e.g. "Fire Damascus").
    item_name: String,
    /// Offered crafting fee in Zeny.
    fee_zeny: String,
    /// Request id the Assign / Complete / Cancel buttons act on.
    request_id: String,
    /// Crafter to assign.
    crafter_name: String,
}

/// The board's current requests, drawn in the window rather than only
/// printed to chat.
/// Generic over the board path, as a non-generic selector is ambiguous with
/// `rust_state`'s constant-value `Selector` impl.
struct RequestListSelector<B> {
    board_path: B,
    text: UnsafeCell<String>,
}

impl<B> Selector<ClientState, String> for RequestListSelector<B>
where
    B: Path<ClientState, CommissionBoardState>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let text = self.board_path.follow_safe(state).format_list();
        unsafe {
            *self.text.get() = text;
            Some(self.text.as_ref_unchecked())
        }
    }
}

/// Run `/commission <action> <id> [crafter]` from the id (and crafter) box.
/// Like posting, it goes through the slash command, so both share one parser
/// and the reply lands in chat.
fn act_on_request<I, C>(
    action: &'static str,
    id_path: I,
    crafter_path: C,
) -> impl Fn(&State<ClientState>, &mut EventQueue<ClientState>) + 'static
where
    I: Path<ClientState, String> + Copy + 'static,
    C: Path<ClientState, String> + Copy + 'static,
{
    move |state, queue| {
        let id = state.get(&id_path).trim().to_owned();
        let text = match action {
            "assign" => format!("/commission assign {id} {}", state.get(&crafter_path).trim()),
            _ => format!("/commission {action} {id}"),
        };
        queue.queue(InputEvent::SendMessage { text });
        queue.queue(Event::Unfocus);
    }
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

/// Graphical window for the Non-Custodial Crafting Commission Board (F31 /
/// Decision D7).
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
        let id_path = self.commission_board_window_state.request_id();
        let crafter_path = self.commission_board_window_state.crafter_name();

        window! {
            title: "Crafting Commission Board",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: "Your own commission list, kept on this client only: other players do not see it. Tell crafters yourself, and trade materials and zeny directly (non-custodial).",
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
                            tooltip: "Add a request to your list (other players do not see it)",
                            event: post_commission(item_path, fee_path),
                        },
                        button! {
                            text: "Copy list to chat",
                            tooltip: "Print your active requests in chat",
                            event: list_commissions(),
                        },
                    ),
                },
                text! {
                    text: RequestListSelector {
                        board_path: client_state().commission_board(),
                        text: UnsafeCell::default(),
                    },
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: "Request # and crafter:",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text_box! {
                            ghost_text: "e.g. 1",
                            state: id_path,
                            input_handler: DefaultHandler::<_, _, MAXIMUM_INPUT_LENGTH>::new(id_path, act_on_request("complete", id_path, crafter_path)),
                            focus_id: CommissionIdTextBox,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "crafter name",
                            state: crafter_path,
                            input_handler: DefaultHandler::<_, _, MAXIMUM_INPUT_LENGTH>::new(crafter_path, act_on_request("assign", id_path, crafter_path)),
                            focus_id: CommissionCrafterTextBox,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Assign",
                            tooltip: "Record the crafter who took request # (open requests only)",
                            event: act_on_request("assign", id_path, crafter_path),
                        },
                        button! {
                            text: "Complete",
                            tooltip: "Mark request # done once the trade is finished",
                            event: act_on_request("complete", id_path, crafter_path),
                        },
                        button! {
                            text: "Cancel",
                            tooltip: "Withdraw request #",
                            event: act_on_request("cancel", id_path, crafter_path),
                        },
                    ),
                },
            ),
        }
    }
}
