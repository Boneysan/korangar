use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::AccountId;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

/// Target frame for another player: who they are, and everything you can do to
/// them.
///
/// Opened by left-clicking a player, which previously did **nothing** — the
/// `PlayerInteract` match fell through to `_ => Ok(())` for `EntityType::Player`.
/// That empty slot is why this is on left-click rather than right, which is
/// already bound to both camera rotation and cast cancel.
///
/// It exists because whisper, invite and trade all needed the same missing
/// primitive: a way to act on a named player. Trade is the starkest — the only
/// other route is `/trade request <account_id>`, and account ids are not
/// something a player can know.
///
/// **A stranger's frame is thin, and that is the protocol's doing.** The server
/// sends name and job for anyone visible, but level and HP only for party
/// members, so there is no equivalent of a WoW inspect. Those details appear in
/// the party window once you are grouped.
pub struct PlayerTargetWindow {
    account_id: AccountId,
    character_name: String,
    class_name: String,
}

impl PlayerTargetWindow {
    pub fn new(account_id: AccountId, character_name: String, class_name: String) -> Self {
        Self {
            account_id,
            character_name,
            class_name,
        }
    }
}

impl CustomWindow<ClientState> for PlayerTargetWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::PlayerTarget)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let Self {
            account_id,
            character_name,
            class_name,
        } = self;

        let whisper_name = character_name.clone();
        let invite_name = character_name.clone();
        let friend_name = character_name.clone();
        let ignore_name = character_name.clone();
        let trade_name = character_name.clone();

        window! {
            title: "Target",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: format!("^000001{character_name}^000000"),
                },
                text! {
                    text: class_name,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Whisper",
                            tooltip: "Aim the chat box at this player [^000001/w <name>^000000]",
                            event: InputEvent::StartWhisper { character_name: whisper_name },
                        },
                        button! {
                            text: "Invite",
                            tooltip: "Invite to your party [^000001/party invite <name>^000000]",
                            event: InputEvent::InviteToParty { character_name: invite_name },
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Trade",
                            tooltip: "Ask this player to trade",
                            event: InputEvent::RequestTrade {
                                account_id,
                                character_name: trade_name,
                            },
                        },
                        button! {
                            text: "Add friend",
                            tooltip: "Send a friend request [^000001friend list^000000]",
                            event: InputEvent::AddFriend { character_name: friend_name },
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Ignore",
                            tooltip: "Block whispers from this player [^000001/ignore <name>^000000]",
                            event: InputEvent::SetPlayerIgnored {
                                character_name: ignore_name,
                                ignored: true,
                            },
                        },
                    ),
                },
            ),
        }
    }
}
