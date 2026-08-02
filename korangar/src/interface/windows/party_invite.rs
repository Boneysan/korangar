use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::PartyId;

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
/// **It names the party, not the inviter, and that is a protocol limit rather
/// than an omission.** `ZC_PARTY_JOIN_REQ` carries only `party_id` and
/// `party_name` (`PartyInvitePacket`), so unlike WoW or FFXIV there is no
/// character name to show.
///
/// The buttons queue the same events as the party window's, which resolve the
/// invite through `PartyState::pending_invite_id` rather than a captured id, so
/// the two paths cannot disagree about which invite is being answered.
pub struct PartyInviteWindow {
    party_name: String,
}

impl PartyInviteWindow {
    pub fn new(_party_id: PartyId, party_name: String) -> Self {
        Self { party_name }
    }
}

impl CustomWindow<ClientState> for PartyInviteWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::PartyInvite)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let party_name = match self.party_name.trim().is_empty() {
            true => "a party".to_owned(),
            false => format!("^000001{}^000000", self.party_name),
        };

        window! {
            title: "Party invite",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: format!("You have been invited to join {party_name}"),
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
