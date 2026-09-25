use korangar_interface::element::StateElement;
use ragnarok_packets::{AccountId, CharacterId, ClientTick, JobId, PartyId, PartyMember, PartyMemberInfoPacket, TilePosition};
use rust_state::RustState;

#[derive(Clone, Debug, RustState, StateElement)]
pub struct SharedDestination {
    sender: String,
    nonce: u32,
    map_name: String,
    #[hidden_element]
    position: Option<(u16, u16)>,
}

impl SharedDestination {
    pub fn sender(&self) -> &str {
        &self.sender
    }

    pub fn nonce(&self) -> u32 {
        self.nonce
    }

    pub fn map_name(&self) -> &str {
        &self.map_name
    }

    pub fn position(&self) -> Option<(u16, u16)> {
        self.position
    }

    pub fn display_text(&self) -> String {
        match self.position {
            Some((x, y)) => format!("Shared destination from {}: {} ({x}, {y})", self.sender, self.map_name),
            None => format!("Shared destination from {}: {}", self.sender, self.map_name),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartySessionMessage {
    DestinationSet {
        sender: String,
        nonce: u32,
        map_name: String,
        position: Option<(u16, u16)>,
    },
    DestinationAccepted {
        sender: String,
        nonce: u32,
    },
    ReadyStart {
        sender: String,
        nonce: u32,
    },
    ReadyResponse {
        sender: String,
        nonce: u32,
        ready: bool,
    },
}

pub fn parse_party_session_message(text: &str) -> Option<PartySessionMessage> {
    let (sender, body) = text.split_once(" : ")?;
    if sender.is_empty() || sender.len() > 24 {
        return None;
    }
    let mut fields = body.strip_prefix("[KORANGAR-SESSION:v1] ")?.split_whitespace();
    let action = fields.next()?;
    match action {
        "dest-set" => {
            let nonce = fields.next()?.parse::<u32>().ok()?;
            let map_name = fields.next()?;
            if map_name.is_empty() || map_name.len() > 24 || !map_name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
                return None;
            }
            let x = fields.next()?;
            let y = fields.next()?;
            let position = match (x, y) {
                ("*", "*") => None,
                (x, y) => Some((x.parse::<u16>().ok()?, y.parse::<u16>().ok()?)),
            };
            if fields.next().is_some() {
                return None;
            }
            Some(PartySessionMessage::DestinationSet {
                sender: sender.to_owned(),
                nonce,
                map_name: map_name.to_owned(),
                position,
            })
        }
        "dest-accept" => {
            let nonce = fields.next()?.parse::<u32>().ok()?;
            if fields.next().is_some() {
                return None;
            }
            Some(PartySessionMessage::DestinationAccepted {
                sender: sender.to_owned(),
                nonce,
            })
        }
        "ready-start" => {
            let nonce = fields.next()?.parse::<u32>().ok()?;
            if fields.next().is_some() {
                return None;
            }
            Some(PartySessionMessage::ReadyStart {
                sender: sender.to_owned(),
                nonce,
            })
        }
        "ready-response" => {
            let nonce = fields.next()?.parse::<u32>().ok()?;
            let ready = match fields.next()? {
                "ready" => true,
                "not-ready" => false,
                _ => return None,
            };
            if fields.next().is_some() {
                return None;
            }
            Some(PartySessionMessage::ReadyResponse {
                sender: sender.to_owned(),
                nonce,
                ready,
            })
        }
        _ => None,
    }
}

#[derive(Clone, Debug, RustState, StateElement)]
pub(crate) struct ReadyCheck {
    #[hidden_element]
    starter: String,
    #[hidden_element]
    nonce: u32,
    #[hidden_element]
    participants: Vec<String>,
    #[hidden_element]
    responses: Vec<(String, bool)>,
    #[hidden_element]
    expires_at: u32,
}

impl ReadyCheck {
    fn new(starter: String, nonce: u32, participants: Vec<String>, now: ClientTick) -> Self {
        let mut unique = Vec::new();
        for name in participants.into_iter().chain(std::iter::once(starter.clone())) {
            if !unique.iter().any(|existing: &String| existing.eq_ignore_ascii_case(&name)) {
                unique.push(name);
            }
        }
        let mut check = Self {
            starter: starter.clone(),
            nonce,
            participants: unique,
            responses: Vec::new(),
            expires_at: now.0.wrapping_add(30_000),
        };
        check.record_response(&starter, true);
        check
    }

    fn record_response(&mut self, sender: &str, ready: bool) -> bool {
        if !self.participants.iter().any(|name| name.eq_ignore_ascii_case(sender))
            || self.responses.iter().any(|(name, _)| name.eq_ignore_ascii_case(sender))
        {
            return false;
        }
        self.responses.push((sender.to_owned(), ready));
        true
    }

    fn remove_participant(&mut self, sender: &str) {
        self.participants.retain(|name| !name.eq_ignore_ascii_case(sender));
        self.responses.retain(|(name, _)| !name.eq_ignore_ascii_case(sender));
    }

    fn display_text(&self) -> String {
        let mut text = format!(
            "Ready check from {} ({}/{} replied)",
            self.starter,
            self.responses.len(),
            self.participants.len()
        );
        for name in &self.participants {
            let state = self
                .responses
                .iter()
                .find(|(responder, _)| responder.eq_ignore_ascii_case(name))
                .map(|(_, ready)| if *ready { "ready" } else { "not ready" })
                .unwrap_or("waiting");
            text.push_str(&format!("\n{name}: {state}"));
        }
        text
    }
}

fn client_tick_reached(now: u32, deadline: u32) -> bool {
    now.wrapping_sub(deadline) < (1 << 31)
}

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
    /// Class name for [`Self::job_id`], resolved by the caller. The state layer
    /// holds no `Library`, so it cannot turn a job id into a name itself -- the
    /// same reason `TradeState::add_partner_item` takes an item name.
    class_name: String,
    /// Set by `ZC_GROUP_ISALIVE`. Only ever describes other members, since the
    /// packet is sent `PARTY_WOS`.
    is_dead: bool,
    /// Cached [`Self::summary_line`]. The party window renders members as
    /// elements with their own buttons, and an element's text has to come from
    /// a *field* path rather than a method.
    display_label: String,
}

#[allow(dead_code)]
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

    pub fn job_id(&self) -> Option<JobId> {
        self.job_id
    }

    pub fn is_dead(&self) -> bool {
        self.is_dead
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
        // Match the friend-list blue/neutral/orange status palette while
        // retaining explicit words for color-independent status reading.
        let reset = crate::state::COLOR_RESET;
        let online = match (self.online, self.is_dead) {
            (false, _) => format!("{}offline{reset}", crate::state::COLOR_OFFLINE),
            (true, true) => format!("{}DEAD{reset}", crate::state::COLOR_DEAD),
            (true, false) => format!("{}online{reset}", crate::state::COLOR_ONLINE),
        };
        // `★` (U+2605) is absent from the bundled NotoSans subset and drew as a
        // tofu box, so the leader marker was unreadable -- and unreadable in
        // exactly the same way for leader and non-leader. `*` is ASCII, and the
        // gold makes it a marker rather than punctuation.
        let leader = match self.leader {
            true => format!(" {}*{reset}", crate::state::COLOR_LEADER),
            false => String::new(),
        };
        let level = self.base_level.map(|level| format!(" Lv{level}")).unwrap_or_default();
        let class = match self.class_name.is_empty() {
            true => String::new(),
            false => format!(" {}", self.class_name),
        };
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
        format!("{}{leader}{level}{class}  ({online}){hp}{sp}{map}", self.name)
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
            class_name: String::new(),
            is_dead: false,
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
            class_name: String::new(),
            is_dead: false,
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
    /// Our own account id, from `NetworkEvent::AccountId`. Needed to tell
    /// whether *we* are the leader, which gates kick and promote.
    local_account_id: Option<AccountId>,
    /// EXP share rule. Unlike the two item rules this is not in the member-info
    /// packet, so it is only known once the server sends a share-options
    /// broadcast.
    share_experience: bool,
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
    shared_destination: Option<SharedDestination>,
    shared_destination_text: String,
    #[hidden_element]
    ready_check: Option<ReadyCheck>,
    ready_check_text: String,
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
            local_account_id: None,
            share_experience: false,
            deny_invites: false,
            members: Vec::new(),
            share_pickup: false,
            share_loot: false,
            display_text: String::new(),
            status_text: "Not in a party.".to_owned(),
            shared_destination: None,
            shared_destination_text: "No shared destination.".to_owned(),
            ready_check: None,
            ready_check_text: "No active ready check.".to_owned(),
        }
    }
}

#[allow(dead_code)]
impl PartyState {
    pub fn party_name(&self) -> &str {
        &self.party_name
    }

    pub fn members(&self) -> &[PartyMemberState] {
        &self.members
    }

    /// Party chat carries a server-authenticated account id alongside its
    /// printable text. Only accept structured session/ping commands when the
    /// text's claimed sender matches that roster identity.
    pub fn message_sender_matches(&self, account_id: AccountId, sender: &str) -> bool {
        self.members
            .iter()
            .any(|member| member.account_id == account_id && member.name.eq_ignore_ascii_case(sender))
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

    pub fn shared_destination(&self) -> Option<&SharedDestination> {
        self.shared_destination.as_ref()
    }

    pub fn shared_destination_text(&self) -> &str {
        &self.shared_destination_text
    }

    pub fn set_shared_destination(&mut self, sender: String, nonce: u32, map_name: String, position: Option<(u16, u16)>) {
        let destination = SharedDestination {
            sender,
            nonce,
            map_name,
            position,
        };
        self.shared_destination_text = destination.display_text();
        self.shared_destination = Some(destination);
    }

    pub fn clear_shared_destination(&mut self, nonce: u32) -> bool {
        if !self
            .shared_destination
            .as_ref()
            .is_some_and(|destination| destination.nonce == nonce)
        {
            return false;
        }
        self.shared_destination = None;
        self.shared_destination_text = "No shared destination.".to_owned();
        true
    }

    pub fn clear_shared_destination_all(&mut self) {
        self.shared_destination = None;
        self.shared_destination_text = "No shared destination.".to_owned();
    }

    pub fn ready_check_text(&self) -> &str {
        &self.ready_check_text
    }

    pub fn ready_check_nonce(&self) -> Option<u32> {
        self.ready_check.as_ref().map(|check| check.nonce)
    }

    pub fn online_member_names(&self) -> Vec<String> {
        self.members
            .iter()
            .filter(|member| member.online)
            .map(|member| member.name.clone())
            .collect()
    }

    pub fn begin_ready_check(&mut self, starter: String, nonce: u32, participants: Vec<String>, now: ClientTick) {
        self.ready_check = Some(ReadyCheck::new(starter, nonce, participants, now));
        self.refresh_ready_check_text();
    }

    pub fn can_respond_ready_check(&self, nonce: u32, sender: &str) -> bool {
        self.ready_check.as_ref().is_some_and(|check| {
            check.nonce == nonce
                && check.participants.iter().any(|name| name.eq_ignore_ascii_case(sender))
                && check.responses.iter().all(|(name, _)| !name.eq_ignore_ascii_case(sender))
        })
    }

    pub fn record_ready_response(&mut self, nonce: u32, sender: &str, ready: bool) -> bool {
        let accepted = self
            .ready_check
            .as_mut()
            .is_some_and(|check| check.nonce == nonce && check.record_response(sender, ready));
        if accepted {
            self.refresh_ready_check_text();
        }
        accepted
    }

    pub fn tick_ready_check(&mut self, now: ClientTick) {
        if self
            .ready_check
            .as_ref()
            .is_some_and(|check| client_tick_reached(now.0, check.expires_at))
        {
            self.clear_ready_check();
            self.ready_check_text = "Ready check expired.".to_owned();
        }
    }

    fn refresh_ready_check_text(&mut self) {
        self.ready_check_text = self
            .ready_check
            .as_ref()
            .map_or_else(|| "No active ready check.".to_owned(), ReadyCheck::display_text);
    }

    fn clear_ready_check(&mut self) {
        self.ready_check = None;
        self.ready_check_text = "No active ready check.".to_owned();
    }

    /// True once we are actually in a party. Membership, not the party name —
    /// the name arrives with the roster and is empty for a party of one until
    /// the first member packet lands.
    /// Also gates inviting, because Hercules requires the *inviter* to already
    /// be in a party and refuses **silently** when they are not: `party.c:382`
    /// returns a bare `0` if `party->search(sd->status.party_id)` finds
    /// nothing, and it is the only failure path in `party_invite` that
    /// sends the client nothing at all — every branch below it answers with
    /// `party_inviteack` or a message. Sending anyway leaves no reply to
    /// key any feedback off.
    pub fn in_party(&self) -> bool {
        !self.members.is_empty()
    }

    pub fn has_pending_invite(&self) -> bool {
        self.pending_invite_id.is_some()
    }

    pub fn set_local_account_id(&mut self, account_id: AccountId) {
        self.local_account_id = Some(account_id);
        self.rebuild_display_text();
    }

    pub fn local_account_id(&self) -> Option<AccountId> {
        self.local_account_id
    }

    pub fn share_experience(&self) -> bool {
        self.share_experience
    }

    pub fn share_pickup(&self) -> bool {
        self.share_pickup
    }

    pub fn share_loot(&self) -> bool {
        self.share_loot
    }

    /// Whether *we* lead this party. Kick and promote are leader-only; the
    /// server silently ignores them from anyone else, so ungated buttons would
    /// appear to do nothing.
    pub fn local_is_leader(&self) -> bool {
        let Some(local_account_id) = self.local_account_id else {
            return false;
        };
        self.members
            .iter()
            .any(|member| member.account_id == local_account_id && member.leader)
    }

    pub fn is_local(&self, account_id: AccountId) -> bool {
        self.local_account_id == Some(account_id)
    }

    /// Apply a share-options broadcast. The item rules are `None` from the
    /// minimal 0x0101 form and are then left as they were.
    pub fn set_share_options(&mut self, experience: bool, pickup: Option<bool>, division: Option<bool>) {
        self.share_experience = experience;
        if let Some(pickup) = pickup {
            self.share_pickup = pickup;
        }
        if let Some(division) = division {
            self.share_loot = division;
        }
        self.rebuild_display_text();
    }

    /// Move the leader star after a `ZC_CHANGE_GROUP_MASTER` broadcast.
    pub fn set_leader(&mut self, account_id: AccountId) {
        for member in &mut self.members {
            member.leader = member.account_id == account_id;
        }
        self.rebuild_display_text();
    }

    pub fn deny_invites(&self) -> bool {
        self.deny_invites
    }

    /// A member died or was resurrected.
    pub fn set_member_dead(&mut self, account_id: AccountId, is_dead: bool) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.is_dead = is_dead;
            self.rebuild_display_text();
        }
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
        self.clear_shared_destination_all();
        self.clear_ready_check();
        self.share_pickup = false;
        self.share_loot = false;
        self.rebuild_display_text();
    }

    pub fn set_roster(&mut self, party_name: String, members: Vec<PartyMember>, class_name: impl Fn(JobId) -> String) {
        self.party_name = party_name;
        self.members = members
            .into_iter()
            .map(|member| {
                let class = class_name(member.job_id);
                let mut member = PartyMemberState::from_roster_member(member);
                member.class_name = class;
                member
            })
            .collect();
        self.rebuild_display_text();
    }

    pub fn add_or_update_member(&mut self, member: PartyMemberInfoPacket, class_name: String) {
        if self.members.is_empty() {
            self.clear_shared_destination_all();
            self.clear_ready_check();
        }
        self.party_name = member.party_name.clone();
        self.share_pickup = member.share_pickup != 0;
        self.share_loot = member.share_loot != 0;

        let mut member = PartyMemberState::from_member_info(member);
        member.class_name = class_name;

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

    pub fn update_job_and_level(&mut self, account_id: AccountId, job_id: JobId, base_level: u16, class_name: String) {
        if let Some(member) = self.members.iter_mut().find(|member| member.account_id == account_id) {
            member.job_id = Some(job_id);
            member.base_level = Some(base_level);
            member.class_name = class_name;
            self.rebuild_display_text();
        }
    }

    pub fn remove_member(&mut self, account_id: AccountId) {
        // **Our own departure arrives as a removal of us.** Hercules reports a
        // withdrawal to the whole party including the leaver -- `clif_party_withdraw`
        // sends to `PARTY` at `party.c:632`, *before* the member is zeroed on the
        // two lines after it -- so retaining the other rows here left the roster
        // populated for someone who is no longer in a party at all. `in_party()`
        // then stayed true, and every later invite was sent to a server that
        // refuses it silently (`party.c:382`), reporting "waiting for an answer"
        // for an answer that could never come.
        if let Some(member) = self.members.iter().find(|member| member.account_id == account_id)
            && let Some(check) = self.ready_check.as_mut()
        {
            check.remove_participant(member.name());
        }
        match self.is_local(account_id) {
            true => self.members.clear(),
            false => self.members.retain(|member| member.account_id != account_id),
        }

        if self.members.is_empty() {
            self.party_name.clear();
            self.clear_shared_destination_all();
            self.clear_ready_check();
            self.share_pickup = false;
            self.share_loot = false;
            // Same staleness, one row down: a share rule left over from the party
            // we just left would render as this party-less character's own state.
            self.share_experience = false;
            self.outgoing_invite = None;
        } else {
            self.refresh_ready_check_text();
        }
        self.rebuild_display_text();
    }

    fn rebuild_status_text(&mut self) {
        if self.deny_invites && self.members.is_empty() && self.pending_invite_name.is_empty() {
            self.status_text = "Not in a party. Invites are blocked.".to_owned();
            return;
        }

        self.status_text = match (&self.pending_invite_name, &self.outgoing_invite, self.members.is_empty()) {
            (name, ..) if !name.is_empty() => match &self.pending_inviter {
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
            "EXP: {}  |  Pickup: {}  |  Loot: {}",
            if self.share_experience { "shared" } else { "own" },
            if self.share_pickup { "shared" } else { "own" },
            if self.share_loot { "shared" } else { "finder" },
        );
        self.display_text = format!("Party: {}\n{share}", self.party_name);
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{AccountId, CharacterId, JobId, PartyMember, TilePosition};

    use super::*;

    #[test]
    fn structured_party_messages_require_authenticated_roster_sender() {
        let mut state = PartyState::default();
        state.members.push(PartyMemberState {
            account_id: AccountId(7),
            character_id: None,
            name: "Ada".to_owned(),
            map_name: String::new(),
            position: None,
            online: true,
            leader: false,
            job_id: None,
            base_level: None,
            health_points: None,
            maximum_health_points: None,
            spell_points: None,
            maximum_spell_points: None,
            class_name: String::new(),
            is_dead: false,
            display_label: String::new(),
        });

        assert!(state.message_sender_matches(AccountId(7), "ada"));
        assert!(!state.message_sender_matches(AccountId(8), "Ada"));
        assert!(!state.message_sender_matches(AccountId(7), "Mallory"));
    }

    #[test]
    fn party_session_destination_messages_are_versioned_bounded_and_nonce_matched() {
        assert_eq!(
            parse_party_session_message("Ada : [KORANGAR-SESSION:v1] dest-set 420 prt_fild08 120 154"),
            Some(PartySessionMessage::DestinationSet {
                sender: "Ada".to_owned(),
                nonce: 420,
                map_name: "prt_fild08".to_owned(),
                position: Some((120, 154)),
            })
        );
        assert_eq!(
            parse_party_session_message("Ada : [KORANGAR-SESSION:v1] dest-set 421 izlude * *"),
            Some(PartySessionMessage::DestinationSet {
                sender: "Ada".to_owned(),
                nonce: 421,
                map_name: "izlude".to_owned(),
                position: None,
            })
        );
        assert_eq!(
            parse_party_session_message("BigZ : [KORANGAR-SESSION:v1] dest-accept 420"),
            Some(PartySessionMessage::DestinationAccepted {
                sender: "BigZ".to_owned(),
                nonce: 420
            })
        );
        assert_eq!(
            parse_party_session_message("Ada : [KORANGAR-SESSION:v1] ready-start 700"),
            Some(PartySessionMessage::ReadyStart {
                sender: "Ada".to_owned(),
                nonce: 700
            })
        );
        assert_eq!(
            parse_party_session_message("BigZ : [KORANGAR-SESSION:v1] ready-response 700 not-ready"),
            Some(PartySessionMessage::ReadyResponse {
                sender: "BigZ".to_owned(),
                nonce: 700,
                ready: false
            })
        );
        assert!(parse_party_session_message("Ada : [KORANGAR-SESSION:v2] dest-set 420 izlude * *").is_none());
        assert!(parse_party_session_message("Ada : [KORANGAR-SESSION:v1] dest-set 420 izlude;@warp * *").is_none());
        assert!(parse_party_session_message("Ada : [KORANGAR-SESSION:v1] dest-set nope izlude * *").is_none());
        assert!(parse_party_session_message("Ada : [KORANGAR-SESSION:v1] dest-set 420 izlude 1 *").is_none());
        assert!(parse_party_session_message("Ada : [KORANGAR-SESSION:v1] dest-accept 420 extra").is_none());
        assert!(parse_party_session_message("Ada : [KORANGAR-SESSION:v1] ready-response 700 maybe").is_none());
    }

    #[test]
    fn ready_check_tracks_one_response_per_current_member_and_wrap_safe_timeout() {
        let mut state = PartyState::default();
        let now = ClientTick(u32::MAX - 10_000);
        state.begin_ready_check(
            "Ada".to_owned(),
            700,
            vec!["ada".to_owned(), "BigZ".to_owned(), "Cy".to_owned()],
            now,
        );
        assert_eq!(state.ready_check_nonce(), Some(700));
        assert!(state.can_respond_ready_check(700, "BigZ"));
        assert!(!state.can_respond_ready_check(700, "Unknown"));
        assert!(state.record_ready_response(700, "BigZ", false));
        assert!(!state.record_ready_response(700, "bigz", true));
        assert!(state.ready_check_text().contains("BigZ: not ready"));
        assert!(state.ready_check_text().contains("Cy: waiting"));
        state.tick_ready_check(ClientTick(now.0.wrapping_add(29_999)));
        assert_eq!(state.ready_check_nonce(), Some(700));
        state.tick_ready_check(ClientTick(now.0.wrapping_add(30_000)));
        assert_eq!(state.ready_check_nonce(), None);
        assert_eq!(state.ready_check_text(), "Ready check expired.");
    }

    #[test]
    fn shared_destination_persists_until_matching_acceptance_or_party_end() {
        let mut state = PartyState::default();
        state.set_shared_destination("Ada".to_owned(), 420, "izlude".to_owned(), Some((100, 80)));
        assert_eq!(state.shared_destination_text(), "Shared destination from Ada: izlude (100, 80)");
        assert!(!state.clear_shared_destination(419));
        assert!(state.shared_destination().is_some());
        assert!(state.clear_shared_destination(420));
        assert!(state.shared_destination().is_none());

        state.set_shared_destination("Ada".to_owned(), 421, "prt_fild08".to_owned(), None);
        state.clear();
        assert!(state.shared_destination().is_none());
        assert_eq!(state.shared_destination_text(), "No shared destination.");
    }

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

    /// Leaving a party arrives as a removal of *us*, so the whole roster has to
    /// go. Retaining the other rows left `in_party()` true for a character with
    /// `party_id = 0` server-side, and the client then sent invites that
    /// `party.c:382` refuses **silently** — reporting "waiting for an answer"
    /// for an answer that could never come. Nothing logs any of it.
    #[test]
    fn leaving_a_party_clears_the_whole_roster() {
        let mut state = PartyState::default();
        state.set_local_account_id(AccountId(11));

        let us = PartyMember {
            account_id: AccountId(11),
            player_name: "test".to_owned(),
            ..sample_member("test", true)
        };
        let them = PartyMember {
            account_id: AccountId(22),
            player_name: "HeadlessTwo".to_owned(),
            ..sample_member("HeadlessTwo", true)
        };
        state.set_roster("Testing".to_owned(), vec![us, them], |_| String::new());
        assert!(state.in_party());

        // Someone else leaving must only drop their row.
        state.remove_member(AccountId(22));
        assert!(state.in_party(), "we are still in the party after they leave");

        // Us leaving must drop everything, however many others were listed.
        state.set_roster(
            "Testing".to_owned(),
            vec![
                PartyMember {
                    account_id: AccountId(11),
                    ..sample_member("test", true)
                },
                PartyMember {
                    account_id: AccountId(22),
                    ..sample_member("HeadlessTwo", true)
                },
            ],
            |_| String::new(),
        );
        state.remove_member(AccountId(11));
        assert!(!state.in_party(), "leaving must not leave a roster behind to invite from");
        assert!(state.status_text().contains("Not in a party"));
    }

    #[test]
    fn party_membership_requires_a_roster_not_just_a_stale_party_label() {
        let mut state = PartyState::default();
        state.party_name = "Stale label".to_owned();
        assert!(!state.in_party());

        state.set_roster("Testing".to_owned(), vec![sample_member("test", true)], |_| String::new());
        assert!(state.in_party());
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

        state.set_roster("Seal Cascade".to_owned(), vec![sample_member("Alice", true)], |_| String::new());
        assert!(state.in_party());
        assert!(state.status_text().contains("1 member"));
    }

    #[test]
    fn job_change_updates_the_roster_label() {
        let mut state = PartyState::default();
        state.set_roster("Seal Cascade".to_owned(), vec![sample_member("Alice", true)], |_| {
            "Novice".to_owned()
        });
        assert!(state.members()[0].display_label().contains("Novice"));

        state.update_job_and_level(AccountId(1), JobId(4), 12, "Acolyte".to_owned());
        let label = state.members()[0].display_label();
        assert!(
            label.contains("Acolyte"),
            "job packet must rewrite the cached class name: {label}"
        );
        assert!(label.contains("Lv12"));
        assert!(!label.contains("Novice"));
    }

    #[test]
    fn roster_builds_display_text() {
        let mut state = PartyState::default();
        state.set_roster("Seal Cascade".to_owned(), vec![sample_member("Alice", true)], |_| {
            "Wizard".to_owned()
        });
        assert!(state.display_text().contains("Seal Cascade"));
        let label = state.members()[0].display_label();
        assert!(label.contains("Alice"));
        assert!(label.contains("Wizard"));
        assert!(label.contains("online"));
        assert!(label.contains(crate::state::COLOR_ONLINE));
        assert!(label.contains("Lv50"));

        state.set_member_dead(AccountId(1), true);
        let dead_label = state.members()[0].display_label();
        assert!(dead_label.contains("DEAD"));
        assert!(dead_label.contains(crate::state::COLOR_DEAD));
    }

    #[test]
    fn update_position_and_health() {
        let mut state = PartyState::default();
        state.set_roster("P".to_owned(), vec![sample_member("Bob", true)], |_| "Priest".to_owned());
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
