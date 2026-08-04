use std::cmp::Ordering;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox, StateElement};
use korangar_interface::event::{Event, EventQueue};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, PathExt, RustState, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::party::{PartyMemberState, PartyMemberStatePathExt, PartyState, PartyStatePathExt};
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

/// Per-member roster rows, each with its own actions.
///
/// Modelled on `FriendList`. A single text blob could not carry buttons, and
/// per-member Whisper/Trade is the cheapest way to act on a named player --
/// `/trade request` otherwise needs an account id typed by hand.
struct PartyMemberList<A, B> {
    members_path: A,
    party_path: B,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A, B> PartyMemberList<A, B> {
    fn new(members_path: A, party_path: B) -> Self {
        Self {
            members_path,
            party_path,
            elements: Vec::new(),
        }
    }
}

impl<A, B> Element<ClientState> for PartyMemberList<A, B>
where
    A: Path<ClientState, Vec<PartyMemberState>>,
    B: Path<ClientState, PartyState> + Copy + 'static,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;

            let members = state.get(&self.members_path);

            match members.len().cmp(&self.elements.len()) {
                Ordering::Less => self.elements.truncate(members.len()),
                Ordering::Equal => {}
                Ordering::Greater => {
                    for index in self.elements.len()..members.len() {
                        let member_path = self.members_path.index(index).manually_asserted();
                        let party_path = self.party_path;

                        // Kick and promote are leader-only and never apply to
                        // yourself; the server ignores both silently, so an
                        // ungated button would appear to do nothing. One
                        // selector per button, since they are not `Copy`.
                        let promote_blocked = ComputedSelector::new_default(move |state: &ClientState| {
                            let party = party_path.follow_safe(state);
                            !party.local_is_leader() || party.is_local(member_path.follow_safe(state).account_id())
                        });
                        let kick_blocked = ComputedSelector::new_default(move |state: &ClientState| {
                            let party = party_path.follow_safe(state);
                            !party.local_is_leader() || party.is_local(member_path.follow_safe(state).account_id())
                        });
                        let label_path = member_path.display_label();

                        self.elements.push(ErasedElement::new(collapsible! {
                            text: label_path,
                            children: (
                                split! {
                                gaps: theme().window().gaps(),
                                children: (
                                    button! {
                                        text: "Whisper",
                                        tooltip: "Aim the chat box at this member [^000001/w <name>^000000]",
                                        event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                            let character_name = state.get(&member_path).name().to_owned();
                                            queue.queue(InputEvent::StartWhisper { character_name });
                                        },
                                    },
                                    button! {
                                        text: "Trade",
                                        tooltip: "Ask this member to trade (they must be nearby)",
                                        event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                            let member = state.get(&member_path);
                                            queue.queue(InputEvent::RequestTrade {
                                                account_id: member.account_id(),
                                                character_name: member.name().to_owned(),
                                            });
                                        },
                                    },
                                ),
                                },
                                split! {
                                    gaps: theme().window().gaps(),
                                    children: (
                                        button! {
                                            text: "Promote",
                                            tooltip: "Make this member the party leader",
                                            // Leader-only, and never yourself: the
                                            // server ignores both silently, so an
                                            // ungated button would just do nothing.
                                            disabled: promote_blocked,
                                            disabled_tooltip: "Only the party leader can do this",
                                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                                let account_id = state.get(&member_path).account_id();
                                                queue.queue(InputEvent::PromotePartyLeader { account_id });
                                            },
                                        },
                                        button! {
                                            text: "Kick",
                                            tooltip: "Remove this member from the party",
                                            disabled: kick_blocked,
                                            disabled_tooltip: "Only the party leader can do this",
                                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                                let member = state.get(&member_path);
                                                queue.queue(InputEvent::KickPartyMember {
                                                    account_id: member.account_id(),
                                                    character_name: member.name().to_owned(),
                                                });
                                            },
                                        },
                                    ),
                                },
                            ),
                        }));
                    }
                }
            }

            self.elements
                .iter_mut()
                .zip(members.iter())
                .enumerate()
                .for_each(|(index, (element, _))| {
                    element.create_layout_info(state, store.child_store(index as u64), resolver);
                });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let members = state.get(&self.members_path);

        self.elements
            .iter()
            .zip(members.iter())
            .enumerate()
            .for_each(|(index, (element, _))| {
                element.lay_out(state, store.child_store(index as u64), &(), layout);
            });
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
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        state_button! {
                            text: "Share EXP",
                            tooltip: "Split experience across the party (leader only)",
                            state: self.party_path.share_experience(),
                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                // All three rules ride one packet, so the two we
                                // are not changing must be sent as they stand.
                                let party = state.get(&party_path);
                                queue.queue(InputEvent::SetPartyShare {
                                    experience: !party.share_experience(),
                                    pickup: party.share_pickup(),
                                    division: party.share_loot(),
                                });
                            },
                        },
                        state_button! {
                            text: "Share pickup",
                            tooltip: "Everyone picks up for the party (leader only)",
                            state: self.party_path.share_pickup(),
                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                let party = state.get(&party_path);
                                queue.queue(InputEvent::SetPartyShare {
                                    experience: party.share_experience(),
                                    pickup: !party.share_pickup(),
                                    division: party.share_loot(),
                                });
                            },
                        },
                        state_button! {
                            text: "Share loot",
                            tooltip: "Distribute looted items (leader only)",
                            state: self.party_path.share_loot(),
                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                let party = state.get(&party_path);
                                queue.queue(InputEvent::SetPartyShare {
                                    experience: party.share_experience(),
                                    pickup: party.share_pickup(),
                                    division: !party.share_loot(),
                                });
                            },
                        },
                    ),
                },
                state_button! {
                    text: "Block invites",
                    tooltip: "Refuse all party invites server-side [^000001/party block on^000000]",
                    state: self.party_path.deny_invites(),
                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        // Send the opposite of what the server last told us,
                        // never of a locally-toggled value, so a refused
                        // request cannot leave the button lying.
                        let blocked = !*state.get(&party_path.deny_invites());
                        queue.queue(InputEvent::SetPartyInvitationBlock { blocked });
                    },
                },
                text! {
                    text: self.party_path.display_text(),
                },
                PartyMemberList::new(self.party_path.members(), self.party_path),
            )
        }
    }
}
