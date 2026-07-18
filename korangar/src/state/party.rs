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

    /// One-line roster summary for the party window.
    pub fn summary_line(&self) -> String {
        let online = if self.online { "online" } else { "offline" };
        let leader = if self.leader { " ★" } else { "" };
        let level = self.base_level.map(|level| format!(" Lv{level}")).unwrap_or_default();
        let hp = match (self.health_points, self.maximum_health_points) {
            (Some(hp), Some(max)) if max > 0 => format!("  {hp}/{max} HP"),
            _ => String::new(),
        };
        let map = if self.map_name.is_empty() {
            String::new()
        } else {
            format!("  [{}]", self.map_name.trim_end_matches(".gat"))
        };
        format!("{}{leader}{level}  ({online}){hp}{map}", self.name)
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
        }
    }
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct PartyState {
    party_name: String,
    pending_invite_id: Option<PartyId>,
    members: Vec<PartyMemberState>,
    share_pickup: bool,
    share_loot: bool,
    /// Cached multi-line roster text for the party window.
    display_text: String,
}

impl Default for PartyState {
    fn default() -> Self {
        Self {
            party_name: String::new(),
            pending_invite_id: None,
            members: Vec::new(),
            share_pickup: false,
            share_loot: false,
            display_text: "Not in a party.\nUse /party create <name>".to_owned(),
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

    pub fn set_pending_invite(&mut self, party_id: PartyId) {
        self.pending_invite_id = Some(party_id);
    }

    pub fn clear_pending_invite(&mut self) {
        self.pending_invite_id = None;
    }

    pub fn clear(&mut self) {
        self.party_name.clear();
        self.pending_invite_id = None;
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

    pub fn update_health(&mut self, account_id: AccountId, health_points: usize, maximum_health_points: usize) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.health_points = Some(health_points);
            member.maximum_health_points = Some(maximum_health_points);
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

    fn rebuild_display_text(&mut self) {
        if self.members.is_empty() {
            self.display_text = "Not in a party.\nUse /party create <name>".to_owned();
            return;
        }

        let share = format!(
            "EXP share: {}  |  Item share: {}",
            if self.share_pickup { "on" } else { "off" },
            if self.share_loot { "on" } else { "off" },
        );
        let mut lines = vec![format!("Party: {}", self.party_name), share, String::new()];
        for member in &self.members {
            lines.push(member.summary_line());
        }
        self.display_text = lines.join("\n");
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
        assert!(state.display_text().contains("Not in a party"));
    }

    #[test]
    fn roster_builds_display_text() {
        let mut state = PartyState::default();
        state.set_roster("Seal Cascade".to_owned(), vec![sample_member("Alice", true)]);
        assert!(state.display_text().contains("Seal Cascade"));
        assert!(state.display_text().contains("Alice"));
        assert!(state.display_text().contains("online"));
        assert!(state.display_text().contains("Lv50"));
    }

    #[test]
    fn update_position_and_health() {
        let mut state = PartyState::default();
        state.set_roster("P".to_owned(), vec![sample_member("Bob", true)]);
        state.update_position(AccountId(1), TilePosition::new(10, 20));
        state.update_health(AccountId(1), 100, 200);
        let member = &state.members()[0];
        assert_eq!(member.position(), Some(TilePosition::new(10, 20)));
        assert_eq!(member.health_points(), Some(100));
        assert!(state.display_text().contains("100/200 HP"));
    }
}
