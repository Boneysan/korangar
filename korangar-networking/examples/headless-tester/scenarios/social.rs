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
    primary.wait_for("successful CreatePartyResult", |event| match event {
        NetworkEvent::CreatePartyResult { result: 0 } => Some(()),
        _ => None,
    })
}

/// Best-effort: leave any party a previous (possibly interrupted) run left
/// behind, so `create_party` starts from a clean slate.
pub(super) fn ensure_no_party(context: &mut TestContext) {
    let _ = context.net.leave_party();
    context.pump(Duration::from_millis(300));
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

fn begin_trade(primary: &mut TestContext, partner: &mut TestContext) -> Result<(), String> {
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
