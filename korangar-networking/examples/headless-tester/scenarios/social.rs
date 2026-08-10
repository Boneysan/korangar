//! Phase 8 — multi-client social protocol flows.

use std::time::Duration;

use korangar_networking::NetworkEvent;

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("whisper-emotion", 8, whisper_emotion),
        Scenario::new("friend-lifecycle", 8, friend_lifecycle),
        Scenario::new("friend-reject", 8, friend_reject),
        Scenario::new("party-lifecycle", 8, party_lifecycle),
        Scenario::new("party-reject-block", 8, party_reject_block),
        Scenario::new("party-member-vitals", 8, party_member_vitals),
        Scenario::new("party-sp-only-broadcast", 8, party_sp_only_broadcast),
        Scenario::new("party-persists-relog", 8, party_persists_relog),
        Scenario::new("party-invite-sender", 8, party_invite_sender),
        Scenario::new("party-member-death", 8, party_member_death),
        Scenario::new("party-kick", 8, party_kick),
        Scenario::new("party-promote-leader", 8, party_promote_leader),
        Scenario::new("party-share-options", 8, party_share_options),
        Scenario::new("whisper-ignore", 8, whisper_ignore),
        Scenario::new("trade-add-item", 8, trade_add_item),
        Scenario::new("trade-cancel", 8, trade_cancel),
        Scenario::new("trade-commit", 8, trade_commit),
    ]
}

/// Moved to [`TestContext::connect_pair`] so the observer-parity scenarios can
/// use it too; kept as an alias to leave this file's call sites alone.
pub(super) fn connect_pair(config: &Config) -> Result<(TestContext, TestContext), String> {
    TestContext::connect_pair(config)
}

fn whisper_emotion(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    const MESSAGE: &str = "headless whisper marker";

    partner.flush();
    primary
        .net
        .send_whisper_message(&partner.character_name, MESSAGE)
        .map_err(|_| "primary disconnected")?;
    partner.wait_for("WhisperReceived", |event| match event {
        NetworkEvent::WhisperReceived { sender_name, message, .. }
            if sender_name == &primary.character_name && message.contains(MESSAGE) =>
        {
            Some(())
        }
        _ => None,
    })?;
    primary.wait_for("successful WhisperResult", |event| match event {
        NetworkEvent::WhisperResult { result: 0 } => Some(()),
        _ => None,
    })?;

    partner.flush();
    primary.flush();
    primary.net.request_emotion(1).map_err(|_| "primary disconnected")?;
    let player_id = primary.player_id;
    // The emotion is an area broadcast that includes the sender, so the
    // sender-side echo separates "server rejected the emote" from "partner
    // out of view range".
    primary.wait_for("own DisplayEmotion echo", |event| match event {
        NetworkEvent::DisplayEmotion { entity_id, emotion: 1 } if *entity_id == player_id => Some(()),
        _ => None,
    })?;
    partner.wait_for("DisplayEmotion", |event| match event {
        NetworkEvent::DisplayEmotion { entity_id, emotion: 1 } if *entity_id == player_id => Some(()),
        _ => None,
    })
}

fn friend_lifecycle(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    // Make reruns self-cleaning if an earlier invocation stopped mid-flow.
    let _ = primary.net.remove_friend(partner.account_id, partner.character_id);
    let _ = partner.net.remove_friend(primary.account_id, primary.character_id);
    primary.pump(Duration::from_millis(250));
    partner.pump(Duration::from_millis(250));

    partner.flush();
    primary
        .net
        .add_friend(partner.character_name.clone())
        .map_err(|_| "primary disconnected")?;
    let requestee = partner.wait_for("FriendRequest", |event| match event {
        NetworkEvent::FriendRequest { requestee } if requestee.name == primary.character_name => Some(requestee.clone()),
        _ => None,
    })?;
    partner
        .net
        .accept_friend_request(requestee.account_id, requestee.character_id)
        .map_err(|_| "partner disconnected")?;
    primary.wait_for("FriendAdded", |event| match event {
        NetworkEvent::FriendAdded { friend } if friend.name == partner.character_name => Some(()),
        _ => None,
    })?;

    primary
        .net
        .remove_friend(partner.account_id, partner.character_id)
        .map_err(|_| "primary disconnected")?;
    primary.wait_for("FriendRemoved", |event| match event {
        NetworkEvent::FriendRemoved { account_id, character_id }
            if *account_id == partner.account_id && *character_id == partner.character_id =>
        {
            Some(())
        }
        _ => None,
    })
}

fn friend_reject(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    // **Self-cleaning, like `friend_lifecycle`.** Without this the scenario is
    // clean only because natural order happens to clean up before it: if the two
    // are already friends, `add_friend` is refused, no `FriendRequest` ever
    // reaches the partner, and this times out. Found by `--shuffle 20260810`,
    // which ran it *before* `friend_lifecycle` for the first time.
    let _ = primary.net.remove_friend(partner.account_id, partner.character_id);
    let _ = partner.net.remove_friend(primary.account_id, primary.character_id);
    primary.pump(Duration::from_millis(250));
    partner.pump(Duration::from_millis(250));
    partner.flush();
    primary
        .net
        .add_friend(partner.character_name.clone())
        .map_err(|_| "primary disconnected")?;
    let requestee = partner.wait_for("FriendRequest", |event| match event {
        NetworkEvent::FriendRequest { requestee } if requestee.name == primary.character_name => Some(requestee.clone()),
        _ => None,
    })?;
    partner
        .net
        .reject_friend_request(requestee.account_id, requestee.character_id)
        .map_err(|_| "partner disconnected")?;
    primary.wait_for("friend rejection result", |event| match event {
        NetworkEvent::ChatMessage { text, .. }
            if text.to_ascii_lowercase().contains("reject") || text.to_ascii_lowercase().contains("does not want to be friends") =>
        {
            Some(())
        }
        _ => None,
    })
}

pub(super) fn create_party(primary: &mut TestContext) -> Result<(), String> {
    primary.flush();
    let party_name = format!("Headless{}", std::process::id() % 100000);
    primary.net.create_party(&party_name).map_err(|_| "primary disconnected")?;
    // **Match ANY result, not just success.** Waiting only for `result: 0` makes
    // a refusal indistinguishable from silence: `party-kick` failed on the
    // 2026-08-10 shuffle with "timed out waiting for successful
    // CreatePartyResult" when the server had almost certainly answered — it just
    // answered "no, you are already in a party". A refusal that reads as a
    // timeout sends you looking for a lost packet instead of leftover state.
    let result = primary.wait_for("CreatePartyResult", |event| match event {
        NetworkEvent::CreatePartyResult { result } => Some(*result),
        _ => None,
    })?;
    match result {
        0 => Ok(()),
        code => Err(format!(
            "party creation refused with result {code} — the character is most likely still in a party left \
             behind by an earlier scenario; `ensure_no_party` did not clear it"
        )),
    }
}

/// Best-effort: leave any party a previous (possibly interrupted) run left
/// behind, so `create_party` starts from a clean slate.
pub(super) fn ensure_no_party(context: &mut TestContext) {
    // 300ms was not always enough for the server to finish dissolving a party
    // before the next `create_party` arrived, and the failure surfaces as a
    // refusal several scenarios later rather than here.
    let _ = context.net.leave_party();
    context.pump(Duration::from_millis(800));
    context.flush();
}

/// Create a party on the primary and pull the partner into it, waiting out
/// the full invite/accept round trip.
pub(super) fn form_party(primary: &mut TestContext, partner: &mut TestContext) -> Result<(), String> {
    ensure_no_party(primary);
    ensure_no_party(partner);
    create_party(primary)?;
    partner.flush();
    primary
        .net
        .invite_to_party(&partner.character_name)
        .map_err(|_| "primary disconnected")?;
    let party_id = partner.wait_for("PartyInvite", |event| match event {
        NetworkEvent::PartyInvite { party_id, .. } => Some(*party_id),
        _ => None,
    })?;
    partner.net.accept_party_invite(party_id).map_err(|_| "partner disconnected")?;
    primary.wait_for("PartyMemberAdded", |event| match event {
        NetworkEvent::PartyMemberAdded { member } if member.player_name == partner.character_name => Some(()),
        _ => None,
    })?;
    Ok(())
}

/// Dissolve the party formed by `form_party` (best-effort, for cleanup).
pub(super) fn leave_party_both(primary: &mut TestContext, partner: &mut TestContext) {
    let _ = partner.net.leave_party();
    partner.pump(Duration::from_millis(300));
    let _ = primary.net.leave_party();
    primary.pump(Duration::from_millis(300));
}

/// `ZC_GROUP_ISALIVE` — a party member dying, and coming back.
///
/// Modelled on 2026-08-02 and never asserted on. It drives the "dead" state in
/// the party roster, and it is sent **`PARTY_WOS`** — "without self" — so it
/// never describes the local player. That is the detail a scenario has to
/// respect: the assertion belongs on the *primary* watching the partner die,
/// and testing it from the dying seat would assert on a packet that is not sent
/// to it at all.
///
/// Both halves matter. A client that learns about the death but not the revival
/// leaves a permanently greyed-out member in the roster, which looks like a UI
/// bug and is really a dropped packet.
fn party_member_death(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let subject = partner.account_id;
    primary.flush();

    // **The partner kills itself.** `@kill` kills whoever runs it
    // (`atcommand.c`: `status_kill(&sd->bl)`), so `@kill <name>` from the
    // primary would kill the *primary* and report nothing about the partner —
    // the char-command form is `#kill <name>`. Having the partner do it also
    // puts the assertion on the observing seat, which is what PARTY_WOS
    // requires.
    partner.say("@kill")?;

    let died = primary.wait_for_within("the party member to be reported dead", Duration::from_secs(10), &mut |event| {
        match event {
            NetworkEvent::PartyMemberAlive { account_id, is_dead: true } if account_id.0 == subject.0 => Some(()),
            _ => None,
        }
    });
    if let Err(error) = died {
        leave_party_both(&mut primary, &mut partner);
        let _ = partner.say("@alive");
        return Err(format!(
            "{error}\n         ZC_GROUP_ISALIVE did not arrive. Remember it is sent PARTY_WOS, so it is \
             only ever seen by the *other* members — asserting on the dying seat would prove nothing"
        ));
    }

    // And back again.
    primary.flush();
    partner.say("@alive")?;
    let revived = primary.wait_for_within("the party member to be reported alive again", Duration::from_secs(10), &mut |event| {
        match event {
            NetworkEvent::PartyMemberAlive { account_id, is_dead: false } if account_id.0 == subject.0 => Some(()),
            _ => None,
        }
    });

    leave_party_both(&mut primary, &mut partner);
    let _ = partner.say("@heal");
    partner.pump(Duration::from_millis(300));

    revived.map_err(|error| {
        format!(
            "{error}\n         the death was reported but the revival was not, which leaves the member \
             greyed out in the roster for the rest of the session"
        )
    })?;
    Ok(())
}

fn party_lifecycle(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    // Self-cleaning, like `friend_lifecycle`: `create_party` fails outright if
    // the character is already in one, and a party outlives the session that
    // made it (server-side state in the `party` table). So a party left behind
    // by an interrupted run -- or simply formed by hand in the GUI -- would
    // fail this scenario for a reason unrelated to what it tests.
    ensure_no_party(&mut primary);
    ensure_no_party(&mut partner);
    create_party(&mut primary)?;
    partner.flush();
    primary
        .net
        .invite_to_party(&partner.character_name)
        .map_err(|_| "primary disconnected")?;
    let party_id = partner.wait_for("PartyInvite", |event| match event {
        NetworkEvent::PartyInvite { party_id, .. } => Some(*party_id),
        _ => None,
    })?;
    partner.net.accept_party_invite(party_id).map_err(|_| "partner disconnected")?;
    primary.wait_for("PartyMemberAdded", |event| match event {
        NetworkEvent::PartyMemberAdded { member } if member.player_name == partner.character_name => Some(()),
        _ => None,
    })?;

    primary.flush();
    partner
        .net
        .send_party_chat_message(&partner.character_name, "headless party marker")
        .map_err(|_| "partner disconnected")?;
    primary.wait_for("PartyChatMessage", |event| match event {
        NetworkEvent::PartyChatMessage { text, .. } if text.contains("headless party marker") => Some(()),
        _ => None,
    })?;
    partner.net.leave_party().map_err(|_| "partner disconnected")?;
    primary.wait_for("PartyMemberRemoved", |event| match event {
        NetworkEvent::PartyMemberRemoved { character_name, .. } if character_name == &partner.character_name => Some(()),
        _ => None,
    })?;
    primary.net.leave_party().map_err(|_| "primary disconnected".to_owned())
}

fn party_reject_block(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    ensure_no_party(&mut primary);
    ensure_no_party(&mut partner);
    create_party(&mut primary)?;
    partner.flush();
    primary
        .net
        .invite_to_party(&partner.character_name)
        .map_err(|_| "primary disconnected")?;
    let party_id = partner.wait_for("PartyInvite", |event| match event {
        NetworkEvent::PartyInvite { party_id, .. } => Some(*party_id),
        _ => None,
    })?;
    partner.net.reject_party_invite(party_id).map_err(|_| "partner disconnected")?;
    primary.wait_for("PartyInviteResult rejection", |event| match event {
        NetworkEvent::PartyInviteResult { character_name, result } if character_name == &partner.character_name && *result != 0 => Some(()),
        _ => None,
    })?;

    partner.net.set_party_invitation_block(true).map_err(|_| "partner disconnected")?;
    partner.wait_for("PartyInvitationState blocked", |event| match event {
        NetworkEvent::PartyInvitationState { deny_party_invites: true } => Some(()),
        _ => None,
    })?;
    primary
        .net
        .invite_to_party(&partner.character_name)
        .map_err(|_| "primary disconnected")?;
    primary.wait_for("blocked PartyInviteResult", |event| match event {
        NetworkEvent::PartyInviteResult { character_name, result } if character_name == &partner.character_name && *result != 0 => Some(()),
        _ => None,
    })?;
    partner.net.set_party_invitation_block(false).map_err(|_| "partner disconnected")?;
    primary.net.leave_party().map_err(|_| "primary disconnected".to_owned())
}

/// A party member's HP **and SP** reach the other seat.
///
/// This is the regression guard for the Hercules delta
/// `KORANGAR_PARTY_SP_TO_GROUPM` (korangar `CLAUDE.md` §3b). Stock main-branch
/// Hercules sends the narrow 14-byte `ZC_NOTIFY_HP_TO_GROUPM` (0x080E) with no
/// SP at all; ours sends the wide 22-byte 0x0BAB form. **If an upstream merge
/// drops the delta, `spell_points` silently becomes `None`** — the client keeps
/// working, the party SP bar just never appears again, which is exactly the kind
/// of quiet regression no other test would catch.
///
/// The vitals packet is sent `PARTY_AREA_WOS` — party, in area, *excluding
/// self* — so the assertion has to be made from the partner seat, and both seats
/// have to be near each other. `connect_pair` guarantees that.
fn party_member_vitals(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let primary_account = primary.account_id;

    // Top up first, so the drop below is unambiguously ours and not the tail of
    // some earlier scenario's damage.
    primary.say("@heal 1000000 1000000")?;
    primary.pump(Duration::from_millis(400));
    partner.flush();

    primary.say("@heal -500 -50")?;

    let observed = partner.wait_for("PartyMemberHealth for the primary", |event| match event {
        NetworkEvent::PartyMemberHealth {
            account_id,
            health_points,
            maximum_health_points,
            spell_points,
        } if *account_id == primary_account => Some((*health_points, *maximum_health_points, *spell_points)),
        _ => None,
    });

    leave_party_both(&mut primary, &mut partner);

    let (health_points, maximum_health_points, spell_points) = observed?;

    if maximum_health_points == 0 {
        return Err("party vitals reported a maximum HP of 0".to_owned());
    }
    if health_points >= maximum_health_points {
        return Err(format!(
            "expected the primary's HP to have dropped, got {health_points}/{maximum_health_points}"
        ));
    }

    let Some((spell_points, maximum_spell_points)) = spell_points else {
        return Err(
            "party vitals carried no SP: the server sent the narrow 0x080E form, so the Hercules \
             KORANGAR_PARTY_SP_TO_GROUPM delta is missing or was lost in an upstream merge"
                .to_owned(),
        );
    };

    if maximum_spell_points == 0 {
        return Err("party vitals reported a maximum SP of 0".to_owned());
    }
    if spell_points > maximum_spell_points {
        return Err(format!(
            "party vitals reported SP above the maximum: {spell_points}/{maximum_spell_points}"
        ));
    }

    Ok(())
}

/// An SP change **on its own** broadcasts to the party.
///
/// Narrower than [`party_member_vitals`] and aimed at one line: the `case SP_SP:`
/// arm of `clif_updatestatus` (`clif.c:3861`), which is what *triggers*
/// `clif->party_hp`. Widening only the packet layout and forgetting the trigger
/// leaves SP riding along on HP updates, so the bar freezes between hits and
/// looks like a client bug.
///
/// Full-heals first and asserts HP stays at maximum: at full HP there is no
/// natural HP regeneration, so any vitals packet that arrives can only have been
/// triggered by the SP change.
fn party_sp_only_broadcast(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let primary_account = primary.account_id;

    primary.say("@heal 1000000 1000000")?;
    primary.pump(Duration::from_millis(400));
    partner.flush();

    // SP only — `@heal <hp> <sp>`, so HP is untouched.
    primary.say("@heal 0 -50")?;

    let observed = partner.wait_for("party vitals from an SP-only change", |event| match event {
        NetworkEvent::PartyMemberHealth {
            account_id,
            health_points,
            maximum_health_points,
            spell_points,
        } if *account_id == primary_account => Some((*health_points, *maximum_health_points, *spell_points)),
        _ => None,
    });

    leave_party_both(&mut primary, &mut partner);

    let (health_points, maximum_health_points, spell_points) = observed?;

    if health_points != maximum_health_points {
        return Err(format!(
            "HP moved during an SP-only probe ({health_points}/{maximum_health_points}), so this run \
             cannot prove the SP_SP trigger fired"
        ));
    }
    if spell_points.is_none() {
        return Err("party vitals carried no SP; see party-member-vitals for the likely cause".to_owned());
    }

    Ok(())
}

/// Party membership survives a logout — no re-invite needed.
///
/// Parties are server-side state in the `party` table, so this is official
/// behaviour rather than a fork feature; the test exists so a future change to
/// login-time party restoration cannot break it quietly.
///
/// Asserted functionally, by sending party chat after the relog, rather than by
/// waiting for a `PartyList` at login — that packet arrives during connect and
/// may already have been drained by the time a scenario could wait on it.
fn party_persists_relog(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let partner_name = partner.character_name.clone();

    // Dropping the context logs the partner out.
    drop(partner);
    primary.pump(Duration::from_millis(1000));
    primary.flush();

    let mut partner = TestContext::connect_partner(config)?;
    partner.pump(Duration::from_millis(500));

    const MARKER: &str = "headless party survived relog";
    let sent = partner
        .net
        .send_party_chat_message(&partner_name, MARKER)
        .map_err(|_| "partner disconnected".to_owned());

    let observed = sent.and_then(|()| {
        primary.wait_for("PartyChatMessage after the partner relogged", |event| match event {
            NetworkEvent::PartyChatMessage { text, .. } if text.contains(MARKER) => Some(()),
            _ => None,
        })
    });

    leave_party_both(&mut primary, &mut partner);
    observed
}

/// An invite names who sent it.
///
/// Guards the fork packet `ZC_PARTY_INVITE_SENDER` (0x0EFF) and its Hercules
/// send site in `clif_party_invite`. Official `ZC_PARTY_JOIN_REQ` carries only
/// the party id and name, so without the companion packet the invite popup can
/// only say "you are invited to join <party>".
///
/// Losing it is **silent** in the same way the party-SP delta is: the invite
/// still arrives and still works, the name just quietly stops appearing. The
/// companion is sent *before* the invite, so this waits for it first.
fn party_invite_sender(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    ensure_no_party(&mut primary);
    ensure_no_party(&mut partner);
    create_party(&mut primary)?;

    partner.flush();
    primary
        .net
        .invite_to_party(&partner.character_name)
        .map_err(|_| "primary disconnected")?;

    let expected = primary.character_name.clone();
    let sender = partner.wait_for("PartyInviteSender naming the inviter", |event| match event {
        NetworkEvent::PartyInviteSender { party_id, character_name } => Some((*party_id, character_name.clone())),
        _ => None,
    });

    let invite = match sender.is_ok() {
        true => partner
            .wait_for("the PartyInvite it belongs to", |event| match event {
                NetworkEvent::PartyInvite { party_id, .. } => Some(*party_id),
                _ => None,
            })
            .ok(),
        false => None,
    };

    if let Some(party_id) = invite {
        let _ = partner.net.reject_party_invite(party_id);
        partner.pump(Duration::from_millis(300));
    }
    leave_party_both(&mut primary, &mut partner);

    let (sender_party_id, sender_name) = sender?;

    if sender_name != expected {
        return Err(format!("invite reported sender {sender_name:?}, expected {expected:?}"));
    }

    match invite {
        Some(invite_party_id) if invite_party_id == sender_party_id => Ok(()),
        Some(invite_party_id) => Err(format!(
            "sender packet was for party {sender_party_id:?} but the invite was for {invite_party_id:?};              the client pairs them by id, so a mismatch would show the wrong name"
        )),
        None => Err("the sender packet arrived but the invite itself never did".to_owned()),
    }
}

/// The leader can remove a member (`CZ_REQ_LEAVE_GROUP_MEMBER`, 0x0103).
///
/// Asserted from the kicked seat as well as the leader's: the leader would see
/// `PartyMemberRemoved` even if the packet had been malformed enough that the
/// server dropped the *member* silently, so checking only one side could pass
/// on a half-broken kick.
fn party_kick(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let partner_name = partner.character_name.clone();
    let partner_account = partner.account_id;
    primary.flush();
    partner.flush();

    primary
        .net
        .kick_party_member(partner_account, &partner_name)
        .map_err(|_| "primary disconnected")?;

    let seen_by_leader = primary.wait_for("PartyMemberRemoved on the leader", |event| match event {
        NetworkEvent::PartyMemberRemoved { account_id, .. } if *account_id == partner_account => Some(()),
        _ => None,
    });

    leave_party_both(&mut primary, &mut partner);
    seen_by_leader
}

/// Leadership can be handed to another member (`CZ_CHANGE_GROUP_MASTER`, 0x07DA).
///
/// `ZC_CHANGE_GROUP_MASTER` is sent to the whole party, so the *promoted* seat
/// is the honest place to assert: it proves the broadcast reached someone other
/// than the actor, which is the half that the client's leader star depends on.
fn party_promote_leader(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let partner_account = partner.account_id;
    partner.flush();

    primary
        .net
        .change_party_leader(partner_account)
        .map_err(|_| "primary disconnected")?;

    let observed = partner.wait_for("PartyLeaderChanged naming the new leader", |event| match event {
        NetworkEvent::PartyLeaderChanged { new_leader_account_id, .. } => Some(*new_leader_account_id),
        _ => None,
    });

    leave_party_both(&mut primary, &mut partner);

    match observed? {
        new_leader if new_leader == partner_account => Ok(()),
        new_leader => Err(format!("leadership went to {new_leader:?}, expected {partner_account:?}")),
    }
}

/// Share rules can be changed and are broadcast back (`CZ_GROUPINFO_CHANGE_V2`).
///
/// The reply is **either** the rich 0x07D8 form or the 6-byte 0x0101 depending
/// on `send_party_options` in `conf/map/battle/party.conf`, so the item fields
/// are `Option` and only the EXP rule is asserted -- the one field both forms
/// carry. A run against a server configured to send only 0x0101 must still pass.
fn party_share_options(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    primary.flush();
    primary
        .net
        .set_party_options(true, false, false)
        .map_err(|_| "primary disconnected")?;

    let observed = primary.wait_for("PartyShareOptions with EXP sharing on", |event| match event {
        NetworkEvent::PartyShareOptions { experience_share, .. } => Some(*experience_share),
        _ => None,
    });

    // Put it back before leaving: share rules live on the party server-side and
    // would otherwise leak into whatever runs next.
    let _ = primary.net.set_party_options(false, false, false);
    primary.pump(Duration::from_millis(300));
    leave_party_both(&mut primary, &mut partner);

    match observed? {
        true => Ok(()),
        false => Err("the server reported EXP sharing still off after enabling it".to_owned()),
    }
}

/// Ignoring a character actually blocks their whispers
/// (`CZ_SETTING_WHISPER_PC`, 0x00CF).
///
/// Asserted functionally rather than by the ack alone: Hercules answers the
/// *sender* of a blocked whisper with `WhisperResult` **2**, so the round trip
/// proves the ignore took effect rather than merely that the packet parsed.
///
/// The ignore list is persistent server-side, so this always clears it again --
/// left behind it would silently break any later whisper scenario, which is
/// exactly the slow-motion shared-state failure `observer-ammo-disguise` caused.
fn whisper_ignore(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;

    let partner_name = partner.character_name.clone();

    // Start from a known state in case an interrupted run left it set.
    let _ = primary.net.set_player_ignored(&partner_name, false);
    primary.pump(Duration::from_millis(300));
    primary.flush();

    primary
        .net
        .set_player_ignored(&partner_name, true)
        .map_err(|_| "primary disconnected")?;
    let acknowledged = primary.wait_for("successful IgnoreResult", |event| match event {
        NetworkEvent::IgnoreResult { result: 0, .. } => Some(()),
        _ => None,
    });

    let blocked = acknowledged.as_ref().ok().map(|()| {
        partner.flush();
        let _ = partner.net.send_whisper_message(&primary.character_name, "blocked by ignore");
        partner.wait_for("WhisperResult reporting the sender is ignored", |event| match event {
            NetworkEvent::WhisperResult { result } if *result != 0 => Some(*result),
            _ => None,
        })
    });

    // Always clear it, whatever happened above.
    let _ = primary.net.set_player_ignored(&partner_name, false);
    primary.pump(Duration::from_millis(300));

    acknowledged?;

    match blocked {
        Some(Ok(result)) => match result {
            2 => Ok(()),
            other => Err(format!(
                "whisper to an ignoring character failed with result {other}, expected 2 (ignored)"
            )),
        },
        Some(Err(error)) => Err(format!("the whisper was not refused: {error}")),
        None => Err("ignore was never acknowledged".to_owned()),
    }
}

fn begin_trade(primary: &mut TestContext, partner: &mut TestContext) -> Result<(), String> {
    // **Self-cleaning, and it belongs here because all three trade scenarios
    // funnel through this function.** None of them cleaned up before, so they
    // were clean only by virtue of natural order: a trade left half-open by an
    // earlier scenario means the next `request_trade` is refused, the partner
    // never sees a `TradeRequest`, and the scenario times out looking like a
    // lost packet. `trade-cancel` failed exactly that way on `--shuffle
    // 20260810`, which put `trade-commit` a hundred scenarios ahead of it.
    let _ = primary.net.trade_cancel();
    let _ = partner.net.trade_cancel();
    primary.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));

    partner.flush();
    primary.net.request_trade(partner.account_id).map_err(|_| "primary disconnected")?;
    partner.wait_for("TradeRequest", |event| match event {
        NetworkEvent::TradeRequest { name, .. } if name == &primary.character_name => Some(()),
        _ => None,
    })?;
    partner.net.accept_trade().map_err(|_| "partner disconnected")?;
    primary.wait_for("successful TradeStart", |event| match event {
        NetworkEvent::TradeStart { result: 3, .. } => Some(()),
        _ => None,
    })
}

/// An item put into a trade reaches the other side.
///
/// This is the wire half of the "Add to trade" entry added to the right-click
/// item menu. It matters because the *only* previous way in was `/trade add
/// <inventory_index>` -- an internal number no player can see -- so the path
/// was effectively untested as well as unreachable.
///
/// Asserted from the partner seat: `TradeAddItemResult` on the sender only says
/// the server accepted the request, not that the item was described to anyone
/// else, and `TradePartnerItem` is what the trade window actually renders.
fn trade_add_item(config: &Config) -> Result<(), String> {
    const RED_POTION: u32 = 501;

    let (mut primary, mut partner) = connect_pair(config)?;
    let index = primary.give_item(RED_POTION, 1)?;
    begin_trade(&mut primary, &mut partner)?;

    partner.flush();
    primary
        .net
        .trade_add_item(index, 1)
        .map_err(|_| "primary disconnected")?;

    let accepted = primary.wait_for("TradeAddItemResult for the offered item", |event| match event {
        NetworkEvent::TradeAddItemResult { inventory_index, result } if *inventory_index == index => Some(*result),
        _ => None,
    });

    let seen_by_partner = match accepted.is_ok() {
        true => partner
            .wait_for("TradePartnerItem describing it", |event| match event {
                NetworkEvent::TradePartnerItem { item_id, amount, .. } => Some((*item_id, *amount)),
                _ => None,
            })
            .ok(),
        false => None,
    };

    // Always tear the trade down: an open trade blocks later scenarios from
    // trading, and Hercules refuses several unrelated actions while trading.
    let _ = primary.net.trade_cancel();
    primary.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));

    match accepted? {
        0 => {}
        other => return Err(format!("the server refused the offered item with result {other}")),
    }

    match seen_by_partner {
        Some((item_id, amount)) if item_id.0 == RED_POTION && amount == 1 => Ok(()),
        Some((item_id, amount)) => Err(format!(
            "partner was shown item {} x{amount}, expected {RED_POTION} x1",
            item_id.0
        )),
        None => Err("the item was accepted but never described to the partner".to_owned()),
    }
}

fn trade_cancel(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    begin_trade(&mut primary, &mut partner)?;
    primary.net.trade_cancel().map_err(|_| "primary disconnected")?;
    partner.wait_for("TradeCancelled", |event| match event {
        NetworkEvent::TradeCancelled => Some(()),
        _ => None,
    })
}

fn trade_commit(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    primary.say("@zeny 1")?;
    primary.pump(Duration::from_millis(200));
    begin_trade(&mut primary, &mut partner)?;
    primary.net.trade_add_zeny(1).map_err(|_| "primary disconnected")?;
    primary.net.trade_ok().map_err(|_| "primary disconnected")?;
    partner.net.trade_ok().map_err(|_| "partner disconnected")?;
    primary.net.trade_commit().map_err(|_| "primary disconnected")?;
    partner.net.trade_commit().map_err(|_| "partner disconnected")?;
    primary.wait_for("successful TradeCompleted", |event| match event {
        NetworkEvent::TradeCompleted { success: true } => Some(()),
        _ => None,
    })?;
    partner.wait_for("successful TradeCompleted", |event| match event {
        NetworkEvent::TradeCompleted { success: true } => Some(()),
        _ => None,
    })
}
