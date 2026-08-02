//! Client-side friend list entries with online presence.

use korangar_interface::element::StateElement;
use ragnarok_packets::{AccountId, CharacterId, Friend};
use rust_state::RustState;

#[derive(Clone, Debug, RustState, StateElement)]
pub struct FriendEntry {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub name: String,
    pub online: bool,
    /// Name plus online glyph for the friend list UI.
    pub display_label: String,
}

impl FriendEntry {
    pub fn from_friend(friend: Friend, online: bool) -> Self {
        let display_label = Self::format_label(&friend.name, online);
        Self {
            account_id: friend.account_id,
            character_id: friend.character_id,
            name: friend.name,
            online,
            display_label,
        }
    }

    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn online(&self) -> bool {
        self.online
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
        self.display_label = Self::format_label(&self.name, online);
    }

    fn format_label(name: &str, online: bool) -> String {
        if online { format!("{name}  ●") } else { format!("{name}  ○") }
    }
}

/// Online friends first, then alphabetical — the order every modern client
/// uses, because the people you can actually talk to are the point of the list.
/// Case-insensitive so `alice` and `Bob` do not sort by ASCII case.
pub fn sort_friends(friends: &mut [FriendEntry]) {
    friends.sort_by(|left, right| {
        right
            .online
            .cmp(&left.online)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_online_first_then_by_name() {
        let entry = |name: &str, online: bool| {
            FriendEntry::from_friend(
                Friend {
                    account_id: AccountId(1),
                    character_id: CharacterId(2),
                    name: name.to_owned(),
                },
                online,
            )
        };

        let mut friends = vec![
            entry("zoe", false),
            entry("Bob", true),
            entry("alice", false),
            entry("Carol", true),
        ];
        sort_friends(&mut friends);

        let order: Vec<_> = friends.iter().map(|friend| friend.name.as_str()).collect();
        assert_eq!(order, ["Bob", "Carol", "alice", "zoe"]);
    }

    #[test]
    fn online_glyph_updates() {
        let friend = Friend {
            account_id: AccountId(1),
            character_id: CharacterId(2),
            name: "Bob".to_owned(),
        };
        let mut entry = FriendEntry::from_friend(friend, false);
        assert!(entry.display_label.contains('○'));
        entry.set_online(true);
        assert!(entry.display_label.contains('●'));
        assert!(entry.online());
    }
}
