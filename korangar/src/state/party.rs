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

#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct PartyState {
    party_name: String,
    pending_invite_id: Option<PartyId>,
    members: Vec<PartyMemberState>,
    share_pickup: bool,
    share_loot: bool,
}

impl PartyState {
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
    }

    pub fn set_roster(&mut self, party_name: String, members: Vec<PartyMember>) {
        self.party_name = party_name;
        self.members = members.into_iter().map(PartyMemberState::from_roster_member).collect();
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
    }

    pub fn update_position(&mut self, account_id: AccountId, position: TilePosition) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.position = Some(position);
        }
    }

    pub fn update_health(&mut self, account_id: AccountId, health_points: usize, maximum_health_points: usize) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.health_points = Some(health_points);
            member.maximum_health_points = Some(maximum_health_points);
        }
    }

    pub fn update_job_and_level(&mut self, account_id: AccountId, job_id: JobId, base_level: u16) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.job_id = Some(job_id);
            member.base_level = Some(base_level);
        }
    }

    pub fn remove_member(&mut self, account_id: AccountId) {
        self.members.retain(|member| member.account_id != account_id);

        if self.members.is_empty() {
            self.party_name.clear();
            self.share_pickup = false;
            self.share_loot = false;
        }
    }
}
