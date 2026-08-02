use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::event::{Event, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, RustState, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::party::{PartyState, PartyStatePathExt};
use crate::state::theme::InterfaceThemeType;

/// Character names cap at 24 in Ragnarok, and party names share the limit.
const MAXIMUM_NAME_LENGTH: usize = 24;

/// ZST for the name field's focus id.
pub struct PartyNameTextBox;

/// Internal state of the party window.
#[derive(Default, RustState, StateElement)]
pub struct PartyWindowState {
    /// Doubles as the party name when creating and the character name when
    /// inviting — the two are never needed at once, since you must already be
    /// in a party to invite anyone.
    name_input: String,
}

/// Builds a click handler that spends the name field on a party action, then
/// clears it. `build_event` is only called for a non-empty name, so the buttons
/// are inert rather than sending an empty request.
fn with_name<N, F>(name_path: N, build_event: F) -> impl Fn(&State<ClientState>, &mut EventQueue<ClientState>) + 'static
where
    N: Path<ClientState, String> + Copy + 'static,
    F: Fn(String) -> InputEvent + 'static,
{
    move |state, queue| {
        let name = state.get(&name_path).trim().to_owned();
        if name.is_empty() {
            return;
        }
        state.update_value_with(name_path, |current| current.clear());
        queue.queue(build_event(name));
        queue.queue(Event::Unfocus);
    }
}

/// Party roster and controls. Every `/party` chat command has a button here;
/// the commands still work and remain the only way to pass an explicit party
/// id.
///
/// Buttons disable themselves rather than disappearing, each with a
/// `disabled_tooltip` saying what is missing — so the window always shows the
/// full set of things a party can do, and never silently drops a request the
/// server would only reject.
pub struct PartyWindow<A, B> {
    window_state_path: A,
    party_path: B,
}

impl<A, B> PartyWindow<A, B> {
    pub fn new(window_state_path: A, party_path: B) -> Self {
        Self {
            window_state_path,
            party_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for PartyWindow<A, B>
where
    A: Path<ClientState, PartyWindowState> + Copy + 'static,
    B: Path<ClientState, PartyState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Party)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let name_path = self.window_state_path.name_input();
        let party_path = self.party_path;

        let create = with_name(name_path, |party_name| InputEvent::CreateParty { party_name });
        let invite = with_name(name_path, |character_name| InputEvent::InviteToParty { character_name });

        // Enter does whichever of the two is possible right now: you cannot
        // invite before you have a party, and the server refuses a second one.
        let submit = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            match !state.get(&party_path).members().is_empty() {
                true => with_name(name_path, |character_name| InputEvent::InviteToParty { character_name })(state, queue),
                false => with_name(name_path, |party_name| InputEvent::CreateParty { party_name })(state, queue),
            }
        };

        let cannot_create = ComputedSelector::new_default(move |state: &ClientState| {
            !party_path.members().follow_safe(state).is_empty() || name_path.follow_safe(state).trim().is_empty()
        });
        let cannot_invite = ComputedSelector::new_default(move |state: &ClientState| {
            party_path.members().follow_safe(state).is_empty() || name_path.follow_safe(state).trim().is_empty()
        });
        // One selector per button: `ComputedSelector` is not `Copy`, so Accept
        // and Reject cannot share one.
        let no_invite_for_accept =
            ComputedSelector::new_default(move |state: &ClientState| party_path.pending_invite_id().follow_safe(state).is_none());
        let no_invite_for_reject =
            ComputedSelector::new_default(move |state: &ClientState| party_path.pending_invite_id().follow_safe(state).is_none());
        let not_in_party = ComputedSelector::new_default(move |state: &ClientState| party_path.members().follow_safe(state).is_empty());

        window! {
            title: "Party",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: self.party_path.status_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text_box! {
                    ghost_text: "Party name, or character to invite",
                    state: name_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(name_path, submit),
                    focus_id: PartyNameTextBox,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Create",
                            tooltip: "Create a party with the name above [^000001/party create <name>^000000]",
                            disabled: cannot_create,
                            disabled_tooltip: "Type a party name first — and leave your current party, you can only be in one",
                            event: create,
                        },
                        button! {
                            text: "Invite",
                            tooltip: "Invite the character named above [^000001/party invite <name>^000000]",
                            disabled: cannot_invite,
                            disabled_tooltip: "Type a character name, and create or join a party first",
                            event: invite,
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Accept",
                            tooltip: "Join the party that invited you [^000001/party accept^000000]",
                            disabled: no_invite_for_accept,
                            disabled_tooltip: "Nobody has invited you",
                            event: |_state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                queue.queue(InputEvent::AcceptPartyInvite);
                            },
                        },
                        button! {
                            text: "Reject",
                            tooltip: "Decline the invite [^000001/party reject^000000]",
                            disabled: no_invite_for_reject,
                            disabled_tooltip: "Nobody has invited you",
                            event: |_state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                queue.queue(InputEvent::RejectPartyInvite);
                            },
                        },
                        button! {
                            text: "Leave",
                            tooltip: "Leave the current party [^000001/party leave^000000]",
                            disabled: not_in_party,
                            disabled_tooltip: "You are not in a party",
                            event: |_state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                queue.queue(InputEvent::LeaveParty);
                            },
                        },
                    ),
                },
                text! {
                    text: self.party_path.display_text(),
                },
            )
        }
    }
}
