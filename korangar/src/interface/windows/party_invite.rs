use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

/// Popup shown when someone invites you to their party, mirroring
/// [`FriendRequestWindow`](super::FriendRequestWindow).
///
/// Without it an invite was only a chat line plus a status line in the party
/// window, so accepting meant knowing to press Alt+Z first — an invite could
/// scroll past unnoticed.
///
/// Names the inviter when the fork packet 0x0EFF supplied one, since
/// `ZC_PARTY_JOIN_REQ` itself carries only the party id and name. Falls back to
/// naming just the party when it did not — a stock server, or the Hercules
/// delta lost in a merge — so the invite is never blocked on the extra packet.
///
/// With a name in hand the popup also offers **Whisper**, so "who is this?" can
/// be answered before accepting.
///
/// The buttons queue the same events as the party window's, which resolve the
/// invite through `PartyState::pending_invite_id` rather than a captured id, so
/// the two paths cannot disagree about which invite is being answered.
pub struct PartyInviteWindow {
    party_name: String,
    inviter: Option<String>,
}

impl PartyInviteWindow {
    pub fn new(party_name: String, inviter: Option<String>) -> Self {
        Self { party_name, inviter }
    }
}

impl CustomWindow<ClientState> for PartyInviteWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::PartyInvite)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // "join testing" reads as a verb phrase, not as the name of a party, so
        // the word "party" carries the label. Only the fallback drops it, where
        // there is no name to qualify.
        let party_name = match self.party_name.trim().is_empty() {
            true => "a party".to_owned(),
            false => format!("the party ^000001{}^000000", self.party_name),
        };

        let message = match &self.inviter {
            Some(inviter) => format!("^000001{inviter}^000000 invites you to join {party_name}"),
            None => format!("You have been invited to join {party_name}"),
        };

        // The inviter is fixed for the life of this popup, so the button's
        // enabled state is decided up front rather than through a selector.
        let has_inviter = self.inviter.is_some();
        let whisper_target = self.inviter.clone().unwrap_or_default();

        window! {
            title: "Party invite",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: message,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Whisper",
                            tooltip: "Ask who they are before deciding",
                            disabled: !has_inviter,
                            disabled_tooltip: "The server did not say who invited you",
                            event: InputEvent::StartWhisper {
                                character_name: whisper_target,
                            },
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Decline",
                            tooltip: "Decline the invite [^000001/party reject^000000]",
                            event: InputEvent::RejectPartyInvite,
                        },
                        button! {
                            text: "Accept",
                            tooltip: "Join the party [^000001/party accept^000000]",
                            event: InputEvent::AcceptPartyInvite,
                        },
                    ),
                },
            ),
        }
    }
}
