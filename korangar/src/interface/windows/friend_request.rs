use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::Friend;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct FriendRequestWindow {
    friend: Friend,
}

impl FriendRequestWindow {
    pub fn new(friend: Friend) -> Self {
        Self { friend }
    }
}

impl CustomWindow<ClientState> for FriendRequestWindow {
    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Friend request",
            class: Some(WindowClass::FriendRequest),
            theme: InterfaceThemeType::InGame,
            // **Deliberately not closable: a friend request must be answered.**
            // Same class as the party invite popup beside it and the trade
            // windows in `57308acd` -- this framework has no close hook, so the
            // close button sends nothing. `clif.c:17109` sets `friend_req` on
            // **both** sides when the request goes out, and `clif.c:17144` only
            // honours a reply while both still match, so dismissing this popup
            // leaves the requester with no answer at all: no accept, no reject,
            // nothing to report.
            //
            // Reject is the close button and it tells the server, so this cannot
            // strand the player.
            closable: false,
            elements: (
                text! {
                    text: format!("^000001{}^000000 wants to be friends with you", self.friend.name),
                },
                // "Who is this?" should be answerable before deciding, the same
                // as on the party invite popup.
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Whisper",
                            tooltip: "Ask who they are before deciding",
                            event: InputEvent::StartWhisper {
                                character_name: self.friend.name.clone(),
                            },
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Reject",
                            event: InputEvent::RejectFriendRequest {
                                account_id: self.friend.account_id,
                                character_id: self.friend.character_id,
                            },
                        },
                        button! {
                            text: "Accept",
                            event: InputEvent::AcceptFriendRequest {
                                account_id: self.friend.account_id,
                                character_id: self.friend.character_id,
                            },
                        },
                    ),
                },
            ),
        }
    }
}
