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

    #[allow(dead_code)]
    pub fn online(&self) -> bool {
        self.online
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
        self.display_label = Self::format_label(&self.name, online);
    }

    /// **Every glyph here must exist in `archive/data/font/NotoSans.ttf`.** The
    /// bundled font is a 3095-codepoint subset with no `●` (U+25CF) or `○`
    /// (U+25CB), so the original pair rendered as tofu boxes -- the friend list
    /// read `HeadlessTwo []` and never appeared to change. `•` (U+2022) is
    /// present and is the dot used here.
    ///
    /// The explicit label and color both distinguish presence. The shared
    /// status palette avoids a red/green-only distinction for color-vision
    /// accessibility.
    fn format_label(name: &str, online: bool) -> String {
        let (color, state) = match online {
            true => (crate::state::COLOR_ONLINE, "online"),
            false => (crate::state::COLOR_OFFLINE, "offline"),
        };
        let reset = crate::state::COLOR_RESET;
        format!("{name}  {color}\u{2022} {state}{reset}")
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

        let mut friends = vec![entry("zoe", false), entry("Bob", true), entry("alice", false), entry("Carol", true)];
        sort_friends(&mut friends);

        let order: Vec<_> = friends.iter().map(|friend| friend.name.as_str()).collect();
        assert_eq!(order, ["Bob", "Carol", "alice", "zoe"]);
    }

    /// Covers the stored status tag and words, not the rendered contrast;
    /// visual acceptance is still a separate GUI pass.
    #[test]
    fn online_status_color_tags_update() {
        let friend = Friend {
            account_id: AccountId(1),
            character_id: CharacterId(2),
            name: "Bob".to_owned(),
        };

        let mut entry = FriendEntry::from_friend(friend, false);
        assert!(entry.display_label.contains(crate::state::COLOR_OFFLINE));
        assert!(entry.display_label.contains("offline"));
        assert!(!entry.online());

        entry.set_online(true);
        assert!(entry.display_label.contains(crate::state::COLOR_ONLINE));
        assert!(entry.display_label.contains("online"));
        assert!(entry.online());

        // The colour is the signal, so the two states must not share one.
        assert_ne!(crate::state::COLOR_ONLINE, crate::state::COLOR_OFFLINE);
        // Neither may collide with the reserved reset/highlight codes, which
        // would silently render as default text instead of a presence colour.
        for code in [crate::state::COLOR_ONLINE, crate::state::COLOR_OFFLINE] {
            assert_ne!(code, "^000000", "collides with the reset code");
            assert_ne!(code, "^000001", "collides with the highlight code");
        }
        assert_eq!(crate::state::COLOR_ONLINE, "^0072B2", "Wong-palette blue");
        assert_eq!(crate::state::COLOR_OFFLINE, "^6B6B6B", "neutral offline state");
        assert_eq!(crate::state::COLOR_DEAD, "^D55E00", "Wong-palette vermillion");
    }
}
