use korangar_interface::element::StateElement;
use ragnarok_packets::{AccountId, CharacterId, JobId, PartyId, PartyMember, PartyMemberInfoPacket, TilePosition};
use rust_state::RustState;

#[derive(Clone, Debug, RustState, StateElement)]
pub struct PartyMemberState {
    account_id: AccountId,
    character_id: Option<CharacterId>,
    name: String,
    map_name: String,
    position: Option<TilePosition>,
    online: bool,
    leader: bool,
    job_id: Option<JobId>,
    base_level: Option<u16>,
    health_points: Option<usize>,
    maximum_health_points: Option<usize>,
    spell_points: Option<usize>,
    maximum_spell_points: Option<usize>,
    /// Cached [`Self::summary_line`]. The party window renders members as
    /// elements with their own buttons, and an element's text has to come from
    /// a *field* path rather than a method.
    display_label: String,
}

impl PartyMemberState {
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn map_name(&self) -> &str {
        &self.map_name
    }

    pub fn position(&self) -> Option<TilePosition> {
        self.position
    }

    pub fn online(&self) -> bool {
        self.online
    }

    pub fn leader(&self) -> bool {
        self.leader
    }

    pub fn base_level(&self) -> Option<u16> {
        self.base_level
    }

    pub fn health_points(&self) -> Option<usize> {
        self.health_points
    }

    pub fn maximum_health_points(&self) -> Option<usize> {
        self.maximum_health_points
    }

    pub fn spell_points(&self) -> Option<usize> {
        self.spell_points
    }

    pub fn maximum_spell_points(&self) -> Option<usize> {
        self.maximum_spell_points
    }

    /// `(current, maximum)` HP, only once both are known and the maximum is
    /// non-zero — a zero maximum would divide by zero in the bar renderer.
    pub fn health(&self) -> Option<(usize, usize)> {
        match (self.health_points, self.maximum_health_points) {
            (Some(current), Some(maximum)) if maximum > 0 => Some((current, maximum)),
            _ => None,
        }
    }

    pub fn display_label(&self) -> &str {
        &self.display_label
    }

    /// `(current, maximum)` SP, same contract as [`Self::health`].
    pub fn spell(&self) -> Option<(usize, usize)> {
        match (self.spell_points, self.maximum_spell_points) {
            (Some(current), Some(maximum)) if maximum > 0 => Some((current, maximum)),
            _ => None,
        }
    }

    /// One-line roster summary for the party window.
    pub fn summary_line(&self) -> String {
        let online = if self.online { "online" } else { "offline" };
        let leader = if self.leader { " ★" } else { "" };
        let level = self.base_level.map(|level| format!(" Lv{level}")).unwrap_or_default();
        let hp = match self.health() {
            Some((hp, max)) => format!("  {hp}/{max} HP"),
            None => String::new(),
        };
        let sp = match self.spell() {
            Some((sp, max)) => format!("  {sp}/{max} SP"),
            None => String::new(),
        };
        let map = if self.map_name.is_empty() {
            String::new()
        } else {
            format!("  [{}]", self.map_name.trim_end_matches(".gat"))
        };
        format!("{}{leader}{level}  ({online}){hp}{sp}{map}", self.name)
    }

    fn from_roster_member(member: PartyMember) -> Self {
        Self {
            account_id: member.account_id,
            character_id: Some(member.character_id),
            name: member.player_name,
            map_name: member.map_name,
            position: None,
            online: member.offline == 0,
            leader: member.leader == 0,
            job_id: Some(member.job_id),
            base_level: Some(member.base_level),
            health_points: None,
            maximum_health_points: None,
            spell_points: None,
            maximum_spell_points: None,
            display_label: String::new(),
        }
    }

    fn from_member_info(member: PartyMemberInfoPacket) -> Self {
        Self {
            account_id: member.account_id,
            character_id: Some(member.character_id),
            name: member.player_name,
            map_name: member.map_name,
            position: Some(member.position),
            online: member.offline == 0,
            leader: member.leader == 0,
            job_id: Some(member.job_id),
            base_level: Some(member.base_level),
            health_points: None,
            maximum_health_points: None,
            spell_points: None,
            maximum_spell_points: None,
            display_label: String::new(),
        }
    }
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct PartyState {
    party_name: String,
    pending_invite_id: Option<PartyId>,
    /// Name of the party that invited us, shown next to Accept / Reject.
    pending_invite_name: String,
    /// Character we have invited and not yet heard back about. Cleared by
    /// `PartyInviteResult`, whatever the answer was.
    outgoing_invite: Option<String>,
    /// Name of whoever sent the pending invite, from the fork packet 0x0EFF
    /// that arrives just before it. `None` means the companion packet did not
    /// arrive -- a stock server, or the delta lost in a merge -- and the UI
    /// falls back to naming only the party.
    pending_inviter: Option<String>,
    /// Party the [`Self::pending_inviter`] name belongs to.
    pending_invite_id_for_inviter: Option<PartyId>,
    /// Server-side "refuse every party invite" flag. Only ever set from
    /// `PartyInvitationState`, so the toggle shows what the server actually
    /// believes rather than what we last asked for.
    deny_invites: bool,
    members: Vec<PartyMemberState>,
    share_pickup: bool,
    share_loot: bool,
    /// Cached multi-line roster text for the party window.
    display_text: String,
    /// One-line summary of what the window can do right now: whether an invite
    /// is waiting on you, whether one you sent is still outstanding, or whether
    /// you are simply party-less.
    status_text: String,
}

impl Default for PartyState {
    fn default() -> Self {
        Self {
            party_name: String::new(),
            pending_invite_id: None,
            pending_invite_name: String::new(),
            outgoing_invite: None,
            pending_inviter: None,
            pending_invite_id_for_inviter: None,
            deny_invites: false,
            members: Vec::new(),
            share_pickup: false,
            share_loot: false,
            display_text: String::new(),
            status_text: "Not in a party.".to_owned(),
        }
    }
}

impl PartyState {
    pub fn party_name(&self) -> &str {
        &self.party_name
    }

    pub fn members(&self) -> &[PartyMemberState] {
        &self.members
    }

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    pub fn pending_invite_id(&self) -> Option<PartyId> {
        self.pending_invite_id
    }

    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    /// True once we are actually in a party. Membership, not the party name —
    /// the name arrives with the roster and is empty for a party of one until
    /// the first member packet lands.
    pub fn in_party(&self) -> bool {
        !self.members.is_empty()
    }

    pub fn has_pending_invite(&self) -> bool {
        self.pending_invite_id.is_some()
    }

    pub fn deny_invites(&self) -> bool {
        self.deny_invites
    }

    pub fn set_deny_invites(&mut self, deny_invites: bool) {
        self.deny_invites = deny_invites;
        self.rebuild_status_text();
    }

    pub fn pending_inviter(&self) -> Option<&str> {
        self.pending_inviter.as_deref()
    }

    /// Record the sender of an invite that has not arrived yet (fork packet
    /// 0x0EFF, which precedes `ZC_PARTY_JOIN_REQ`). Kept keyed by party id so a
    /// stale name from an earlier, unanswered invite cannot be shown against a
    /// different party.
    pub fn set_pending_inviter(&mut self, party_id: PartyId, character_name: String) {
        self.pending_invite_id_for_inviter = Some(party_id);
        self.pending_inviter = Some(character_name);
    }

    pub fn set_pending_invite(&mut self, party_id: PartyId, party_name: String) {
        self.pending_invite_id = Some(party_id);
        self.pending_invite_name = party_name;

        // Only trust a sender name that was recorded for *this* party.
        if self.pending_invite_id_for_inviter != Some(party_id) {
            self.pending_inviter = None;
        }
        self.rebuild_status_text();
    }

    pub fn clear_pending_invite(&mut self) {
        self.pending_invite_id = None;
        self.pending_invite_name.clear();
        self.pending_inviter = None;
        self.pending_invite_id_for_inviter = None;
        self.rebuild_status_text();
    }

    /// Records that we invited `character_name` and are waiting on an answer.
    pub fn set_outgoing_invite(&mut self, character_name: String) {
        self.outgoing_invite = Some(character_name);
        self.rebuild_status_text();
    }

    pub fn clear_outgoing_invite(&mut self) {
        self.outgoing_invite = None;
        self.rebuild_status_text();
    }

    pub fn clear(&mut self) {
        self.party_name.clear();
        self.pending_invite_id = None;
        self.pending_invite_name.clear();
        self.pending_inviter = None;
        self.pending_invite_id_for_inviter = None;
        self.outgoing_invite = None;
        self.members.clear();
        self.share_pickup = false;
        self.share_loot = false;
        self.rebuild_display_text();
    }

    pub fn set_roster(&mut self, party_name: String, members: Vec<PartyMember>) {
        self.party_name = party_name;
        self.members = members.into_iter().map(PartyMemberState::from_roster_member).collect();
        self.rebuild_display_text();
    }

    pub fn add_or_update_member(&mut self, member: PartyMemberInfoPacket) {
        self.party_name = member.party_name.clone();
        self.share_pickup = member.share_pickup != 0;
        self.share_loot = member.share_loot != 0;

        let member = PartyMemberState::from_member_info(member);

        match self.members.iter_mut().find(|existing| existing.account_id == member.account_id) {
            Some(existing) => *existing = member,
            None => self.members.push(member),
        }
        self.rebuild_display_text();
    }

    pub fn update_position(&mut self, account_id: AccountId, position: TilePosition) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.position = Some(position);
            self.rebuild_display_text();
        }
    }

    /// Applies a `ZC_NOTIFY_HP_TO_GROUPM`. `spell_points` is `None` from the
    /// narrow 0x080E form; in that case any SP already known is **kept** rather
    /// than cleared, so a mixed stream never blanks the bar.
    pub fn update_health(
        &mut self,
        account_id: AccountId,
        health_points: usize,
        maximum_health_points: usize,
        spell_points: Option<(usize, usize)>,
    ) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.health_points = Some(health_points);
            member.maximum_health_points = Some(maximum_health_points);

            if let Some((current, maximum)) = spell_points {
                member.spell_points = Some(current);
                member.maximum_spell_points = Some(maximum);
            }
            self.rebuild_display_text();
        }
    }

    pub fn update_job_and_level(&mut self, account_id: AccountId, job_id: JobId, base_level: u16) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.job_id = Some(job_id);
            member.base_level = Some(base_level);
            self.rebuild_display_text();
        }
    }

    pub fn remove_member(&mut self, account_id: AccountId) {
        self.members.retain(|member| member.account_id != account_id);

        if self.members.is_empty() {
            self.party_name.clear();
            self.share_pickup = false;
            self.share_loot = false;
        }
        self.rebuild_display_text();
    }

    fn rebuild_status_text(&mut self) {
        if self.deny_invites && self.members.is_empty() && self.pending_invite_name.is_empty() {
            self.status_text = "Not in a party. Invites are blocked.".to_owned();
            return;
        }

        self.status_text = match (&self.pending_invite_name, &self.outgoing_invite, self.members.is_empty()) {
            (name, _, _) if !name.is_empty() => match &self.pending_inviter {
                Some(inviter) => format!("{inviter} invited you to {name} — Accept or Reject."),
                None => format!("{name} invited you — Accept or Reject."),
            },
            (_, Some(character_name), _) => format!("Invited {character_name}; waiting for an answer\u{2026}"),
            (_, None, true) => "Not in a party. Name it below and press Create.".to_owned(),
            (_, None, false) => {
                let count = self.members.len();
                let online = self.members.iter().filter(|member| member.online).count();
                match online == count {
                    true => format!("{count} member{}, all online.", if count == 1 { "" } else { "s" }),
                    false => format!("{count} members, {online} online."),
                }
            }
        };
    }

    fn rebuild_display_text(&mut self) {
        self.rebuild_status_text();

        // Members render as their own elements, so each caches its own line.
        for member in &mut self.members {
            member.display_label = member.summary_line();
        }

        if self.members.is_empty() {
            self.display_text = String::new();
            return;
        }

        let share = format!(
            "EXP share: {}  |  Item share: {}",
            if self.share_pickup { "on" } else { "off" },
            if self.share_loot { "on" } else { "off" },
        );
        self.display_text = format!("Party: {}\n{share}", self.party_name);
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{AccountId, CharacterId, JobId, PartyMember, TilePosition};

    use super::*;

    fn sample_member(name: &str, online: bool) -> PartyMember {
        PartyMember {
            account_id: AccountId(1),
            character_id: CharacterId(1),
            player_name: name.to_owned(),
            map_name: "izlude.gat".to_owned(),
            offline: if online { 0 } else { 1 },
            leader: 0,
            job_id: JobId(1),
            base_level: 50,
        }
    }

    #[test]
    fn empty_party_display_text() {
        let state = PartyState::default();
        // The roster is empty; the "no party" wording lives in the status line.
        assert!(state.display_text().is_empty());
        assert!(state.status_text().contains("Not in a party"));
    }

    #[test]
    fn status_text_tracks_invites() {
        let mut state = PartyState::default();

        state.set_outgoing_invite("Bob".to_owned());
        assert!(state.status_text().contains("Invited Bob"));
        state.clear_outgoing_invite();

        state.set_pending_invite(PartyId(7), "Seal Cascade".to_owned());
        assert!(state.has_pending_invite());
        assert!(state.status_text().contains("Seal Cascade"));

        state.clear_pending_invite();
        assert!(!state.has_pending_invite());
        assert!(state.status_text().contains("Not in a party"));

        state.set_roster("Seal Cascade".to_owned(), vec![sample_member("Alice", true)]);
        assert!(state.in_party());
        assert!(state.status_text().contains("1 member"));
    }

    #[test]
    fn roster_builds_display_text() {
        let mut state = PartyState::default();
        state.set_roster("Seal Cascade".to_owned(), vec![sample_member("Alice", true)]);
        assert!(state.display_text().contains("Seal Cascade"));
        let label = state.members()[0].display_label();
        assert!(label.contains("Alice"));
        assert!(label.contains("online"));
        assert!(label.contains("Lv50"));
    }

    #[test]
    fn update_position_and_health() {
        let mut state = PartyState::default();
        state.set_roster("P".to_owned(), vec![sample_member("Bob", true)]);
        state.update_position(AccountId(1), TilePosition::new(10, 20));
        state.update_health(AccountId(1), 100, 200, Some((30, 60)));
        let member = &state.members()[0];
        assert_eq!(member.position(), Some(TilePosition::new(10, 20)));
        assert_eq!(member.health_points(), Some(100));
        assert_eq!(member.spell_points(), Some(30));
        assert!(state.members()[0].display_label().contains("100/200 HP"));
        assert!(state.members()[0].display_label().contains("30/60 SP"));

        // A narrow 0x080E update must not blank the SP we already know.
        state.update_health(AccountId(1), 90, 200, None);
        let member = &state.members()[0];
        assert_eq!(member.health(), Some((90, 200)));
        assert_eq!(member.spell(), Some((30, 60)));
    }
}
