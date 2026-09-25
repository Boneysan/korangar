//! Phase 8 — multi-client social protocol flows.

use std::collections::HashSet;
use std::time::Duration;

use korangar_networking::{MessageColor, NetworkEvent, QuestHuntProgress};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("whisper-emotion", 8, whisper_emotion),
        Scenario::new("friend-lifecycle", 8, friend_lifecycle),
        Scenario::new("friend-reject", 8, friend_reject),
        Scenario::new("party-lifecycle", 8, party_lifecycle),
        Scenario::new("party-message-carrier", 8, party_message_carrier),
        Scenario::new("party-quest-credit", 8, party_quest_credit),
        Scenario::new("account-discovery-isolation", 8, account_discovery_isolation),
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
        Scenario::new("trade-reject", 8, trade_reject),
        Scenario::new("trade-invalid-offers", 8, trade_invalid_offers),
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

/// Guarantee the Basic Skill level that trading and party creation are gated
/// on.
///
/// **This is a server rule, not a quirk of the harness.** With
/// `basic_skill_check: true` (conf/map/battle/player.conf), Hercules requires
/// Basic Skill 1 to send a trade request (`clif.c:13262`) and Basic Skill 7 to
/// create a party (`clif.c:14633`). In both cases it answers with
/// `clif->skill_fail(sd, 1, USESKILL_FAIL_LEVEL, ...)` and **returns without
/// performing the action** — so the partner never receives a `TradeRequest` and
/// no `CreatePartyResult` is ever sent. The scenario then times out as though a
/// packet went missing.
///
/// Any job change wipes skills, and only the sweeps restore them with
/// `@allskill`. In natural order a sweep always happens to run first; shuffled,
/// it does not — which is why `trade-cancel` and `party-kick` failed only under
/// `--shuffle 20260810`.
///
/// Note what the refusal looks like on the wire: cause 0 against skill id 1.
/// That is the overloaded fallback this fork documents in
/// docs/protocol/server-error-channels.md — the server answered, in the one
/// dialect that carries no information.
fn ensure_basic_skill(context: &mut TestContext) {
    let _ = context.say("@allskill");
    context.pump(Duration::from_millis(400));
    context.flush();
}

pub(super) fn create_party(primary: &mut TestContext) -> Result<(), String> {
    ensure_basic_skill(primary);
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
            "party creation refused with result {code} — the character is most likely still in a party left behind by an earlier \
             scenario; `ensure_no_party` did not clear it"
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

    let died = primary.wait_for_within(
        "the party member to be reported dead",
        Duration::from_secs(10),
        &mut |event| match event {
            NetworkEvent::PartyMemberAlive { account_id, is_dead: true } if account_id.0 == subject.0 => Some(()),
            _ => None,
        },
    );
    if let Err(error) = died {
        leave_party_both(&mut primary, &mut partner);
        let _ = partner.say("@alive");
        return Err(format!(
            "{error}\n         ZC_GROUP_ISALIVE did not arrive. Remember it is sent PARTY_WOS, so it is only ever seen by the *other* \
             members — asserting on the dying seat would prove nothing"
        ));
    }

    // And back again.
    primary.flush();
    partner.say("@alive")?;
    let revived = primary.wait_for_within(
        "the party member to be reported alive again",
        Duration::from_secs(10),
        &mut |event| match event {
            NetworkEvent::PartyMemberAlive {
                account_id,
                is_dead: false,
            } if account_id.0 == subject.0 => Some(()),
            _ => None,
        },
    );

    leave_party_both(&mut primary, &mut partner);
    let _ = partner.say("@heal");
    partner.pump(Duration::from_millis(300));

    revived.map_err(|error| {
        format!(
            "{error}\n         the death was reported but the revival was not, which leaves the member greyed out in the roster for the \
             rest of the session"
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

/// Verify the server relays one versioned ping, rate-limits a second ping from
/// the same character, and rejects payloads over its 128-byte carrier bound.
fn party_message_carrier(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    let primary_account = primary.account_id;
    let primary_chat_prefix = format!("{} : ", primary.character_name);
    let result: Result<(), String> = (|| {
        form_party(&mut primary, &mut partner)?;

        const FIRST: &str = "[KORANGAR-PING:v2] danger prontera 155 180";
        const RATE_LIMITED: &str = "[KORANGAR-PING:v2] assist prontera 156 180";
        const ACCEPTED_AFTER_LIMIT: &str = "[KORANGAR-PING:v2] retreat prontera 157 180";
        const BOUNDARY_MARKER: &str = "[KORANGAR-PING:v2] boundary ";
        const OVERSIZED_MARKER: &str = "[KORANGAR-PING:v2] boundary";

        primary.flush();
        partner.flush();
        primary
            .net
            .send_party_chat_message(&primary.character_name, FIRST)
            .map_err(|_| "primary disconnected")?;
        primary
            .net
            .send_party_chat_message(&primary.character_name, RATE_LIMITED)
            .map_err(|_| "primary disconnected")?;

        partner.wait_for("first v2 party ping relay", |event| match event {
            NetworkEvent::PartyChatMessage { account_id, text }
                if *account_id == primary_account && text.starts_with(&primary_chat_prefix) && text.contains(FIRST) =>
            {
                Some(())
            }
            _ => None,
        })?;
        let burst_messages = partner.collect_for(Duration::from_millis(1250));
        if burst_messages
            .iter()
            .any(|event| matches!(event, NetworkEvent::PartyChatMessage { text, .. } if text.contains(RATE_LIMITED)))
        {
            return Err("Hercules relayed a second v2 party ping inside the one-second sender cooldown".to_owned());
        }

        let boundary_payload = format!("{BOUNDARY_MARKER}{}", "x".repeat(128 - BOUNDARY_MARKER.len()));
        if boundary_payload.len() != 128 {
            return Err("party-ping boundary fixture must be exactly 128 bytes".to_owned());
        }
        primary.flush();
        partner.flush();
        primary
            .net
            .send_party_chat_message(&primary.character_name, &boundary_payload)
            .map_err(|_| "primary disconnected")?;
        partner.wait_for("128-byte v2 party-ping relay", |event| match event {
            NetworkEvent::PartyChatMessage { account_id, text }
                if *account_id == primary_account && text.starts_with(&primary_chat_prefix) && text.contains(OVERSIZED_MARKER) =>
            {
                Some(())
            }
            _ => None,
        })?;

        let oversized = format!("{boundary_payload}x");
        if oversized.len() != 129 {
            return Err("oversized party-ping fixture must be exactly 129 bytes".to_owned());
        }
        primary.flush();
        partner.flush();
        primary
            .net
            .send_party_chat_message(&primary.character_name, &oversized)
            .map_err(|_| "primary disconnected")?;
        let oversized_messages = partner.collect_for(Duration::from_millis(300));
        if oversized_messages
            .iter()
            .any(|event| matches!(event, NetworkEvent::PartyChatMessage { text, .. } if text.contains(OVERSIZED_MARKER)))
        {
            return Err("Hercules relayed an oversized party-ping payload".to_owned());
        }

        let _ = partner.collect_for(Duration::from_millis(1000));
        primary.flush();
        partner.flush();
        primary
            .net
            .send_party_chat_message(&primary.character_name, ACCEPTED_AFTER_LIMIT)
            .map_err(|_| "primary disconnected")?;
        partner.wait_for("v2 party ping after the cooldown", |event| match event {
            NetworkEvent::PartyChatMessage { account_id, text }
                if *account_id == primary_account && text.starts_with(&primary_chat_prefix) && text.contains(ACCEPTED_AFTER_LIMIT) =>
            {
                Some(())
            }
            _ => None,
        })?;

        const SESSION_START: &str = "[KORANGAR-SESSION:v1] ready-start 420";
        const SESSION_RATE_LIMITED: &str = "[KORANGAR-SESSION:v1] ready-response 420 ready";
        let _ = partner.collect_for(Duration::from_millis(1050));
        primary.flush();
        partner.flush();
        primary
            .net
            .send_party_chat_message(&primary.character_name, SESSION_START)
            .map_err(|_| "primary disconnected")?;
        primary
            .net
            .send_party_chat_message(&primary.character_name, SESSION_RATE_LIMITED)
            .map_err(|_| "primary disconnected")?;
        partner.wait_for("v1 party session relay", |event| match event {
            NetworkEvent::PartyChatMessage { account_id, text }
                if *account_id == primary_account && text.starts_with(&primary_chat_prefix) && text.contains(SESSION_START) =>
            {
                Some(())
            }
            _ => None,
        })?;
        let session_burst = partner.collect_for(Duration::from_millis(350));
        if session_burst
            .iter()
            .any(|event| matches!(event, NetworkEvent::PartyChatMessage { text, .. } if text.contains(SESSION_RATE_LIMITED)))
        {
            return Err("Hercules relayed a second v1 party session message inside the one-second sender cooldown".to_owned());
        }

        let session_boundary_marker = "[KORANGAR-SESSION:v1] boundary ";
        let session_oversized_marker = "[KORANGAR-SESSION:v1] boundary";
        let session_boundary = format!("{session_boundary_marker}{}", "x".repeat(128 - session_boundary_marker.len()));
        if session_boundary.len() != 128 {
            return Err("party-session boundary fixture must be exactly 128 bytes".to_owned());
        }
        let _ = partner.collect_for(Duration::from_millis(750));
        primary.flush();
        partner.flush();
        primary
            .net
            .send_party_chat_message(&primary.character_name, &session_boundary)
            .map_err(|_| "primary disconnected")?;
        partner.wait_for("128-byte v1 party session relay", |event| match event {
            NetworkEvent::PartyChatMessage { account_id, text }
                if *account_id == primary_account && text.starts_with(&primary_chat_prefix) && text.contains(session_oversized_marker) =>
            {
                Some(())
            }
            _ => None,
        })?;

        let oversized_session = format!("{session_boundary}x");
        if oversized_session.len() != 129 {
            return Err("oversized party-session fixture must be exactly 129 bytes".to_owned());
        }
        let _ = partner.collect_for(Duration::from_millis(1050));
        primary.flush();
        partner.flush();
        primary
            .net
            .send_party_chat_message(&primary.character_name, &oversized_session)
            .map_err(|_| "primary disconnected")?;
        let rejected_session = partner.collect_for(Duration::from_millis(350));
        if rejected_session
            .iter()
            .any(|event| matches!(event, NetworkEvent::PartyChatMessage { text, .. } if text.contains(session_oversized_marker)))
        {
            return Err("Hercules relayed an oversized v1 party session payload".to_owned());
        }
        Ok(())
    })();
    leave_party_both(&mut primary, &mut partner);
    result
}

/// Party members receive kill credit only for their own active quest entries.
fn party_quest_credit(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 11118; // Request: Hunt Spore, one Spore per objective event.
    let (mut primary, mut partner) = connect_pair(config)?;
    let result = (|| {
        form_party(&mut primary, &mut partner)?;
        primary.ensure_job(4008)?; // Lord Knight
        primary.ensure_base_level(99)?;
        primary.say("@allskill")?;
        primary.say("@heal")?;
        primary.say(&format!("@quest add {QUEST_ID}"))?;
        primary.wait_for("primary quest added", |event| match event {
            NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;

        primary.warp("prt_fild08", 170, 180)?;
        partner.warp("prt_fild08", 174, 180)?;
        primary.pump(Duration::from_millis(300));
        partner.pump(Duration::from_millis(300));
        primary.flush();
        partner.flush();

        let first = kill_quest_spore(&mut primary)?;
        primary.wait_for("active primary receives its first Spore quest credit", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(1) => Some(()),
            _ => None,
        })?;
        let inactive_partner_events = partner.collect_for(Duration::from_millis(300));
        if inactive_partner_events.iter().any(|event| {
            matches!(event, NetworkEvent::QuestHuntProgress { objectives }
                if objectives.iter().any(|objective| objective.quest_id == QUEST_ID))
        }) {
            return Err("party kill credited a member without that active quest".to_owned());
        }

        partner.say(&format!("@quest add {QUEST_ID}"))?;
        partner.wait_for("partner quest added", |event| match event {
            NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        primary.flush();
        partner.flush();
        let second = kill_quest_spore(&mut primary)?;
        if first == second {
            return Err("quest fixture reused the same monster entity id".to_owned());
        }
        primary.wait_for("primary receives second Spore quest credit", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(2) => Some(()),
            _ => None,
        })?;
        partner.wait_for("active partner receives shared Spore quest credit", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(1) => Some(()),
            _ => None,
        })?;

        let third = kill_quest_spore_with_partner_distance(&mut primary, &mut partner, 30)?;
        if third == first || third == second {
            return Err("quest fixture reused a prior monster entity id".to_owned());
        }
        primary.wait_for("primary receives third Spore quest credit", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(3) => Some(()),
            _ => None,
        })?;
        partner.wait_for("party member at the 30-cell boundary receives credit", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(2) => Some(()),
            _ => None,
        })?;

        let fourth = kill_quest_spore_with_partner_distance(&mut primary, &mut partner, 31)?;
        if fourth == first || fourth == second || fourth == third {
            return Err("quest fixture reused a prior monster entity id".to_owned());
        }
        primary.wait_for("primary receives fourth Spore quest credit", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(4) => Some(()),
            _ => None,
        })?;
        let outside_range_events = partner.collect_for(Duration::from_millis(400));
        if outside_range_events.iter().any(|event| {
            matches!(event, NetworkEvent::QuestHuntProgress { objectives }
            if objectives.iter().any(|objective| {
                objective.quest_id == QUEST_ID && objective.current_count > 2
            }))
        }) {
            return Err("party member one cell beyond the configured AREA_SIZE received quest credit".to_owned());
        }

        let partner_name = partner.character_name.clone();
        partner.net.leave_party().map_err(|_| "partner disconnected")?;
        primary.wait_for("partner left before solo quest test", |event| match event {
            NetworkEvent::PartyMemberRemoved { character_name, .. } if character_name == &partner_name => Some(()),
            _ => None,
        })?;
        primary.net.leave_party().map_err(|_| "primary disconnected")?;
        primary.pump(Duration::from_millis(300));
        partner.pump(Duration::from_millis(300));
        primary.flush();
        partner.flush();

        let fifth = kill_quest_spore(&mut primary)?;
        if [first, second, third, fourth].contains(&fifth) {
            return Err("solo quest fixture reused a prior monster entity id".to_owned());
        }
        primary.wait_for("solo player receives Spore quest credit", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(5) => Some(()),
            _ => None,
        })?;
        let solo_partner_events = partner.collect_for(Duration::from_millis(300));
        if solo_partner_events.iter().any(|event| {
            matches!(event, NetworkEvent::QuestHuntProgress { objectives }
                if objectives.iter().any(|objective| objective.quest_id == QUEST_ID))
        }) {
            return Err("solo kill credited a former party member".to_owned());
        }

        form_party(&mut primary, &mut partner)?;
        partner.ensure_job(4008)?;
        partner.ensure_base_level(99)?;
        partner.say("@allskill")?;
        partner.say("@heal")?;
        primary.pump(Duration::from_millis(300));
        partner.pump(Duration::from_millis(300));
        primary.flush();
        partner.flush();
        let contested = kill_contested_quest_spore(&mut primary, &mut partner)?;
        if [first, second, third, fourth, fifth].contains(&contested) {
            return Err("contested quest fixture reused a prior monster entity id".to_owned());
        }
        primary.wait_for("contested Spore kill increments primary quest once", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(6) => Some(()),
            _ => None,
        })?;
        partner.wait_for("contested Spore kill increments partner quest once", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(3) => Some(()),
            _ => None,
        })?;
        let primary_late_progress = primary.collect_for(Duration::from_millis(300));
        let partner_late_progress = partner.collect_for(Duration::from_millis(300));
        if primary_late_progress.iter().any(|event| {
            matches!(event, NetworkEvent::QuestHuntProgress { objectives }
            if objectives.iter().any(|objective| {
                objective.quest_id == QUEST_ID && objective.current_count > 6
            }))
        }) || partner_late_progress.iter().any(|event| {
            matches!(event, NetworkEvent::QuestHuntProgress { objectives }
            if objectives.iter().any(|objective| {
                objective.quest_id == QUEST_ID && objective.current_count > 3
            }))
        }) {
            return Err("one contested monster death advanced a party quest more than once".to_owned());
        }

        partner.warp("prt_fild08", primary.position.x.saturating_add(10), primary.position.y)?;
        primary.pump(Duration::from_millis(300));
        partner.pump(Duration::from_millis(300));
        primary.flush();
        partner.flush();
        let primary_target = primary.spawn_monster("SPORE", 1014)?;
        let partner_target = partner.spawn_monster("SPORE", 1014)?;
        if primary_target == partner_target {
            return Err("simultaneous quest fixtures reused a monster entity id".to_owned());
        }
        let primary_target_position = primary
            .entities
            .get(&primary_target)
            .map(|entity| entity.position.tile_position())
            .ok_or("primary simultaneous Spore has no visible tile")?;
        let partner_target_position = partner
            .entities
            .get(&partner_target)
            .map(|entity| entity.position.tile_position())
            .ok_or("partner simultaneous Spore has no visible tile")?;
        primary.walk_to(primary_target_position.x.saturating_sub(1), primary_target_position.y)?;
        partner.walk_to(partner_target_position.x.saturating_sub(1), partner_target_position.y)?;
        primary.flush();
        partner.flush();
        primary.net.player_attack(primary_target).map_err(|_| "primary disconnected")?;
        partner.net.player_attack(partner_target).map_err(|_| "partner disconnected")?;

        let mut primary_target_dead = false;
        let mut partner_target_dead = false;
        for _ in 0..30 {
            if !primary_target_dead {
                primary_target_dead = await_quest_target_attack(&mut primary, primary_target)?;
                if !primary_target_dead {
                    primary.net.player_attack(primary_target).map_err(|_| "primary disconnected")?;
                }
            }
            if !partner_target_dead {
                partner_target_dead = await_quest_target_attack(&mut partner, partner_target)?;
                if !partner_target_dead {
                    partner.net.player_attack(partner_target).map_err(|_| "partner disconnected")?;
                }
            }
            if primary_target_dead && partner_target_dead {
                break;
            }
        }
        if !primary_target_dead || !partner_target_dead {
            return Err(format!(
                "simultaneous quest fixtures did not both die (primary: {primary_target_dead}, partner: {partner_target_dead})"
            ));
        }
        primary.wait_for("two concurrent kills advance primary quest twice", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(8) => Some(()),
            _ => None,
        })?;
        partner.wait_for("two concurrent kills advance partner quest twice", |event| match event {
            NetworkEvent::QuestHuntProgress { objectives } if quest_progress_count(objectives, QUEST_ID) == Some(5) => Some(()),
            _ => None,
        })?;

        primary.say(&format!("@quest del {QUEST_ID}"))?;
        partner.say(&format!("@quest del {QUEST_ID}"))?;
        primary.wait_for("primary quest removed", |event| match event {
            NetworkEvent::QuestRemoved { quest_id } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        partner.wait_for("partner quest removed", |event| match event {
            NetworkEvent::QuestRemoved { quest_id } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        Ok(())
    })();
    leave_party_both(&mut primary, &mut partner);
    result
}

fn quest_progress_count(objectives: &[QuestHuntProgress], quest_id: u32) -> Option<u16> {
    objectives
        .iter()
        .find(|objective| objective.quest_id == quest_id && objective.objective_index == 0)
        .map(|objective| objective.current_count)
}

fn kill_quest_spore(context: &mut TestContext) -> Result<ragnarok_packets::EntityId, String> {
    let target = context.spawn_monster("SPORE", 1014)?;
    kill_spawned_quest_spore(context, target)
}

fn kill_quest_spore_with_partner_distance(
    context: &mut TestContext,
    partner: &mut TestContext,
    distance: u16,
) -> Result<ragnarok_packets::EntityId, String> {
    let target = context.spawn_monster("SPORE", 1014)?;
    let target_position = context
        .entities
        .get(&target)
        .map(|entity| entity.position.tile_position())
        .ok_or("spawned quest Spore has no visible tile")?;
    partner.warp("prt_fild08", target_position.x.saturating_add(distance), target_position.y)?;
    context.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));
    context.flush();
    partner.flush();
    kill_spawned_quest_spore(context, target)
}

fn kill_spawned_quest_spore(context: &mut TestContext, target: ragnarok_packets::EntityId) -> Result<ragnarok_packets::EntityId, String> {
    let player_id = context.player_id;
    for _ in 0..30 {
        let target_position = context
            .entities
            .get(&target)
            .map(|entity| entity.position.tile_position())
            .ok_or("quest Spore disappeared before death")?;
        context.walk_to(target_position.x.saturating_sub(1), target_position.y)?;
        context.flush();
        context.net.player_attack(target).map_err(|_| "primary disconnected")?;
        let outcome = context.wait_for_within(
            "quest Spore damage or death",
            Duration::from_secs(6),
            &mut |event| match event {
                NetworkEvent::RemoveEntity { entity_id, .. } if *entity_id == target => Some(2),
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    ..
                } if *source_entity_id == player_id && *destination_entity_id == target => Some(1),
                NetworkEvent::AttackFailed { target_entity_id, .. } if *target_entity_id == target => Some(0),
                _ => None,
            },
        )?;
        match outcome {
            2 => return Ok(target),
            1 => {}
            _ => {}
        }
    }
    Err("could not kill quest Spore within 30 attacks".to_owned())
}

fn kill_contested_quest_spore(primary: &mut TestContext, partner: &mut TestContext) -> Result<ragnarok_packets::EntityId, String> {
    let target = primary.spawn_monster("SPORE", 1014)?;
    let mut target_position = primary
        .entities
        .get(&target)
        .map(|entity| entity.position.tile_position())
        .ok_or("spawned contested quest Spore has no visible tile")?;
    primary.walk_to(target_position.x.saturating_sub(1), target_position.y)?;
    partner.warp("prt_fild08", target_position.x.saturating_add(1), target_position.y)?;
    primary.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));
    primary.flush();
    partner.flush();
    let primary_id = primary.player_id;
    let partner_id = partner.player_id;
    let mut primary_landed_hit = false;
    let mut partner_landed_hit = false;

    for _ in 0..30 {
        if let Some(entity) = primary.entities.get(&target) {
            target_position = entity.position.tile_position();
        } else if primary_landed_hit && partner_landed_hit {
            return Ok(target);
        } else {
            return Err("contested kill ended before both party members landed a hit".to_owned());
        }
        primary.walk_to(target_position.x.saturating_sub(1), target_position.y)?;
        partner.walk_to(target_position.x.saturating_add(1), target_position.y)?;
        primary.net.player_attack(target).map_err(|_| "primary disconnected")?;
        partner.net.player_attack(target).map_err(|_| "partner disconnected")?;
        let outcome = primary.wait_for_within(
            "contested quest Spore hit or death",
            Duration::from_secs(6),
            &mut |event| match event {
                NetworkEvent::RemoveEntity { entity_id, .. } if *entity_id == target => Some(3),
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    ..
                } if *destination_entity_id == target && *source_entity_id == primary_id => Some(1),
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    ..
                } if *destination_entity_id == target && *source_entity_id == partner_id => Some(2),
                NetworkEvent::AttackFailed { target_entity_id, .. } if *target_entity_id == target => Some(0),
                _ => None,
            },
        )?;
        match outcome {
            3 if primary_landed_hit && partner_landed_hit => return Ok(target),
            3 => return Err("contested kill ended before both party members landed a hit".to_owned()),
            1 => primary_landed_hit = true,
            2 => partner_landed_hit = true,
            0 => {}
            _ => {}
        }
        if !primary.entities.contains_key(&target) {
            if primary_landed_hit && partner_landed_hit {
                return Ok(target);
            }
            return Err("contested kill ended before both party members landed a hit".to_owned());
        }
    }
    Err("could not kill contested quest Spore within 30 attack rounds".to_owned())
}

fn await_quest_target_attack(context: &mut TestContext, target: ragnarok_packets::EntityId) -> Result<bool, String> {
    let outcome = context.wait_for_within(
        "simultaneous quest target damage or death",
        Duration::from_secs(6),
        &mut |event| match event {
            NetworkEvent::RemoveEntity { entity_id, .. } if *entity_id == target => Some(2),
            NetworkEvent::DamageEffect { destination_entity_id, .. } if *destination_entity_id == target => Some(1),
            NetworkEvent::AttackFailed { target_entity_id, .. } if *target_entity_id == target => Some(0),
            _ => None,
        },
    )?;
    if outcome == 0 {
        if let Some(entity) = context.entities.get(&target) {
            let position = entity.position.tile_position();
            context.walk_to(position.x.saturating_sub(1), position.y)?;
        }
    }
    Ok(outcome == 2)
}

/// A first kill is saved to the account ledger, delivered to an active client,
/// and replayed to another character on that account—but never to another
/// account.
fn account_discovery_isolation(config: &Config) -> Result<(), String> {
    let alternate_name = format!("GuideAlt{:05}", std::process::id() % 100_000);
    let alternate_creator = TestContext::connect_as(
        config,
        &config.username,
        &config.password,
        Some(&alternate_name),
        Some(&alternate_name),
    )?;
    let account_id = alternate_creator.account_id.0;
    drop(alternate_creator);

    let (mut primary, mut other_account) = connect_pair(config)?;
    if primary.account_id.0 != account_id {
        return Err(format!(
            "alternate character belongs to account {account_id}, primary logged in as account {}",
            primary.account_id.0
        ));
    }
    if other_account.account_id.0 == account_id {
        return Err("discovery isolation fixture did not use a separate account".to_owned());
    }

    let existing = primary.collect_for(Duration::from_millis(150));
    let known_mobs = discovery_mob_ids(&existing, account_id);
    let known_maps = discovery_map_ids(&existing, account_id);
    let other_existing = other_account.collect_for(Duration::from_millis(150));
    let other_known_mobs = discovery_mob_ids(&other_existing, other_account.account_id.0);
    let other_known_maps = discovery_map_ids(&other_existing, other_account.account_id.0);
    const CANDIDATES: &[(u16, &str)] = &[
        (1002, "PORING"),
        (1007, "FABRE"),
        (1008, "PUPA"),
        (1009, "CONDOR"),
        (1012, "RODA FROG"),
        (1014, "SPORE"),
        (1015, "ZOMBIE"),
        (1049, "PICKY"),
    ];
    let (mob_id, mob_name) = CANDIDATES
        .iter()
        .copied()
        .find(|(mob_id, _)| !known_mobs.contains(mob_id) && !other_known_mobs.contains(mob_id))
        .ok_or("all discovery test mobs were already recorded on one of the fixture accounts")?;
    let visit_candidates = [("geffen", 119, 59), ("payon", 150, 100), ("morocc", 156, 97), ("alberta", 135, 100)];
    let (visit_map, visit_x, visit_y) = visit_candidates
        .iter()
        .copied()
        .find(|(map_name, ..)| *map_name != primary.map_name && !known_maps.contains(*map_name) && !other_known_maps.contains(*map_name))
        .ok_or("all account-discovery map fixtures were already visited on one of the fixture accounts")?;

    primary.ensure_job(4008)?; // Lord Knight
    primary.ensure_base_level(99)?;
    primary.say("@allskill")?;
    primary.say("@heal")?;
    other_account.flush();
    primary.warp(visit_map, visit_x, visit_y)?;
    if primary.map_name != visit_map {
        return Err(format!(
            "map-discovery fixture expected {visit_map}, landed on {}",
            primary.map_name
        ));
    }
    let visit_delta = format!("[KORANGAR-MAP-DISCOVERY:v1:visited:{account_id}:{visit_map}]");
    primary.wait_for("first-visit account discovery delta", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&visit_delta) => Some(()),
        _ => None,
    })?;
    let other_map_events = other_account.collect_for(Duration::from_millis(300));
    if other_map_events.iter().any(|event| {
        matches!(event, NetworkEvent::ChatMessage { color: MessageColor::Server, text }
            if text.contains(&visit_delta))
    }) {
        return Err("map-visit discovery delta leaked to a different account".to_owned());
    }

    primary.warp("prt_fild08", 170, 180)?;
    primary.pump(Duration::from_millis(300));
    primary.flush();
    other_account.flush();

    let target = primary.spawn_monster(mob_name, mob_id)?;
    let target_position = primary
        .entities
        .get(&target)
        .map(|entity| entity.position.tile_position())
        .ok_or("spawned discovery target has no visible tile")?;
    primary.walk_to(target_position.x.saturating_sub(1), target_position.y)?;

    let player_id = primary.player_id;
    let mut target_died = false;
    for _ in 0..30 {
        primary.flush();
        primary.net.player_attack(target).map_err(|_| "primary disconnected")?;
        let outcome = primary.wait_for_within(
            "discovery target damage or death",
            Duration::from_secs(6),
            &mut |event| match event {
                NetworkEvent::RemoveEntity { entity_id, .. } if *entity_id == target => Some(2),
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    ..
                } if *source_entity_id == player_id && *destination_entity_id == target => Some(1),
                NetworkEvent::AttackFailed { target_entity_id, .. } if *target_entity_id == target => Some(0),
                _ => None,
            },
        )?;
        if outcome == 2 {
            target_died = true;
            break;
        }
        if outcome == 0 {
            let target_position = primary
                .entities
                .get(&target)
                .map(|entity| entity.position.tile_position())
                .ok_or("discovery target disappeared without a death event")?;
            primary.walk_to(target_position.x.saturating_sub(1), target_position.y)?;
        }
    }
    if !target_died {
        return Err(format!("could not kill discovery fixture mob {mob_name} ({mob_id})"));
    }

    let delta = format!("[KORANGAR-DISCOVERY:v1:delta:{account_id}:{mob_id}:1]");
    primary.wait_for("account-bound first-kill discovery delta", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&delta) => Some(()),
        _ => None,
    })?;

    let other_events = other_account.collect_for(Duration::from_millis(300));
    if other_events.iter().any(|event| {
        matches!(event, NetworkEvent::ChatMessage { color: MessageColor::Server, text }
            if text.contains("[KORANGAR-DISCOVERY:v1:delta:"))
    }) {
        return Err("first-kill discovery delta leaked to a different account".to_owned());
    }
    drop(other_account);
    drop(primary);

    let mut same_account_alt = TestContext::connect_as(config, &config.username, &config.password, Some(&alternate_name), None)?;
    if same_account_alt.account_id.0 != account_id {
        return Err("alternate-character reconnect changed account identity".to_owned());
    }
    let begin_prefix = format!("[KORANGAR-DISCOVERY:v1:begin:{account_id}:");
    same_account_alt.wait_for("account discovery snapshot begin", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&begin_prefix) => Some(()),
        _ => None,
    })?;
    same_account_alt.wait_for("discovered mob in same-account snapshot", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if discovery_line_contains_mob(text, account_id, mob_id) => Some(()),
        _ => None,
    })?;
    let end_marker = format!("[KORANGAR-DISCOVERY:v1:end:{account_id}:");
    same_account_alt.wait_for("account discovery snapshot end", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&end_marker) => Some(()),
        _ => None,
    })?;
    let map_begin_prefix = format!("[KORANGAR-MAP-DISCOVERY:v1:begin:{account_id}:");
    same_account_alt.wait_for("account map-discovery snapshot begin", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&map_begin_prefix) => Some(()),
        _ => None,
    })?;
    let map_end_prefix = format!("[KORANGAR-MAP-DISCOVERY:v1:end:{account_id}:");
    same_account_alt.wait_for("account map-discovery snapshot end", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&map_end_prefix) => Some(()),
        _ => None,
    })?;
    let map_snapshot = same_account_alt.collect_for(Duration::from_millis(100));
    if !discovery_map_ids(&map_snapshot, account_id).contains(visit_map) {
        return Err(format!("same-account snapshot omitted visited map {visit_map}"));
    }
    drop(same_account_alt);

    let mut separate_account = TestContext::connect_as(
        config,
        &config.partner_username,
        &config.partner_password,
        Some("HeadlessTwo"),
        None,
    )?;
    let separate_account_id = separate_account.account_id.0;
    if separate_account_id == account_id {
        return Err("second account unexpectedly shares the primary account id".to_owned());
    }
    let separate_snapshot = format!("[KORANGAR-DISCOVERY:v1:begin:{separate_account_id}:");
    separate_account.wait_for("separate account's discovery snapshot", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&separate_snapshot) => Some(()),
        _ => None,
    })?;
    let separate_end = format!("[KORANGAR-DISCOVERY:v1:end:{separate_account_id}:");
    separate_account.wait_for("separate account's empty snapshot end", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&separate_end) => Some(()),
        _ => None,
    })?;
    let separate_map_begin = format!("[KORANGAR-MAP-DISCOVERY:v1:begin:{separate_account_id}:");
    separate_account.wait_for("separate account's map-discovery snapshot begin", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&separate_map_begin) => Some(()),
        _ => None,
    })?;
    let separate_map_end = format!("[KORANGAR-MAP-DISCOVERY:v1:end:{separate_account_id}:");
    separate_account.wait_for("separate account's map-discovery snapshot end", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } if text.contains(&separate_map_end) => Some(()),
        _ => None,
    })?;
    let separate_map_snapshot = separate_account.collect_for(Duration::from_millis(100));
    if discovery_mob_ids(&separate_map_snapshot, separate_account_id).contains(&mob_id) {
        return Err(format!("discovered mob {mob_id} leaked into a different account's snapshot"));
    }
    if discovery_map_ids(&separate_map_snapshot, separate_account_id).contains(visit_map) {
        return Err(format!("visited map {visit_map} leaked into a different account's snapshot"));
    }
    Ok(())
}

fn discovery_line_contains_mob(text: &str, account_id: u32, mob_id: u16) -> bool {
    let Some((_, chunk)) = text.split_once("[KORANGAR-DISCOVERY:v1:chunk:") else {
        return false;
    };
    let mut fields = chunk.splitn(4, ':');
    let Some(parsed_account) = fields.next().and_then(|field| field.parse::<u32>().ok()) else {
        return false;
    };
    let _sequence = fields.next();
    let _index = fields.next();
    let Some(payload) = fields.next() else {
        return false;
    };
    parsed_account == account_id
        && payload
            .trim_end_matches(']')
            .split(',')
            .filter_map(|pair| pair.split_once('='))
            .any(|(id, tier)| id.parse::<u16>().ok() == Some(mob_id) && tier == "1")
}

fn discovery_mob_ids(events: &[NetworkEvent], account_id: u32) -> HashSet<u16> {
    let mut known = HashSet::new();
    for event in events {
        if let NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } = event
        {
            let Some((_, chunk)) = text.split_once("[KORANGAR-DISCOVERY:v1:chunk:") else {
                continue;
            };
            let mut fields = chunk.splitn(4, ':');
            if fields.next().and_then(|field| field.parse::<u32>().ok()) != Some(account_id) {
                continue;
            }
            let _sequence = fields.next();
            let _index = fields.next();
            if let Some(payload) = fields.next() {
                for (id, _) in payload.trim_end_matches(']').split(',').filter_map(|pair| pair.split_once('=')) {
                    if let Ok(id) = id.parse() {
                        known.insert(id);
                    }
                }
            }
        }
    }
    known
}

fn discovery_map_ids(events: &[NetworkEvent], account_id: u32) -> HashSet<String> {
    let mut known = HashSet::new();
    for event in events {
        let NetworkEvent::ChatMessage {
            color: MessageColor::Server,
            text,
        } = event
        else {
            continue;
        };
        if let Some((_, visited)) = text.split_once("[KORANGAR-MAP-DISCOVERY:v1:visited:") {
            if let Some((parsed_account, map_name)) = visited.split_once(':')
                && parsed_account.parse::<u32>().ok() == Some(account_id)
            {
                known.insert(map_name.trim_end_matches(']').to_owned());
            }
            continue;
        }
        let Some((_, chunk)) = text.split_once("[KORANGAR-MAP-DISCOVERY:v1:chunk:") else {
            continue;
        };
        let mut fields = chunk.splitn(4, ':');
        if fields.next().and_then(|field| field.parse::<u32>().ok()) != Some(account_id) {
            continue;
        }
        let _sequence = fields.next();
        let _index = fields.next();
        if let Some(payload) = fields.next() {
            known.extend(payload.trim_end_matches(']').split(',').map(str::to_owned));
        }
    }
    known
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
/// working, the party SP bar just never appears again, which is exactly the
/// kind of quiet regression no other test would catch.
///
/// The vitals packet is sent `PARTY_AREA_WOS` — party, in area, *excluding
/// self* — so the assertion has to be made from the partner seat, and both
/// seats have to be near each other. `connect_pair` guarantees that.
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
            "party vitals carried no SP: the server sent the narrow 0x080E form, so the Hercules KORANGAR_PARTY_SP_TO_GROUPM delta is \
             missing or was lost in an upstream merge"
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
/// Narrower than [`party_member_vitals`] and aimed at one line: the `case
/// SP_SP:` arm of `clif_updatestatus` (`clif.c:3861`), which is what *triggers*
/// `clif->party_hp`. Widening only the packet layout and forgetting the trigger
/// leaves SP riding along on HP updates, so the bar freezes between hits and
/// looks like a client bug.
///
/// Full-heals first and asserts HP stays at maximum: at full HP there is no
/// natural HP regeneration, so any vitals packet that arrives can only have
/// been triggered by the SP change.
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
            "HP moved during an SP-only probe ({health_points}/{maximum_health_points}), so this run cannot prove the SP_SP trigger fired"
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
            "sender packet was for party {sender_party_id:?} but the invite was for {invite_party_id:?};              the client pairs \
             them by id, so a mismatch would show the wrong name"
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

/// Leadership can be handed to another member (`CZ_CHANGE_GROUP_MASTER`,
/// 0x07DA).
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

/// Share rules can be changed and are broadcast back
/// (`CZ_GROUPINFO_CHANGE_V2`).
///
/// The reply is **either** the rich 0x07D8 form or the 6-byte 0x0101 depending
/// on `send_party_options` in `conf/map/battle/party.conf`, so the item fields
/// are `Option` and only the EXP rule is asserted -- the one field both forms
/// carry. A run against a server configured to send only 0x0101 must still
/// pass.
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
/// exactly the slow-motion shared-state failure `observer-ammo-disguise`
/// caused.
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
    ensure_basic_skill(primary);
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
    primary.net.trade_add_item(index, 1).map_err(|_| "primary disconnected")?;

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

/// Partner explicitly rejects a trade request.
///
/// Complements `trade-cancel` (which cancels an already-accepted trade) and
/// covers `reject_trade` — previously only the happy path and mid-trade cancel
/// were exercised. Asserts the requester sees a non-start outcome, no trade
/// window opens, and both seats remain usable for a follow-up whisper.
fn trade_reject(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    ensure_basic_skill(&mut primary);
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
    partner.net.reject_trade().map_err(|_| "partner disconnected")?;

    // Hercules reports the refusal to the requester as TradeStart with a
    // non-success result code (not TradeCancelled — that is mid-trade only).
    primary.wait_for("trade rejection on requester", |event| match event {
        NetworkEvent::TradeStart { result, .. } if *result != 3 => Some(*result),
        NetworkEvent::TradeCancelled => Some(u8::MAX),
        _ => None,
    })?;

    // Quiet window: no successful trade start, no partner-item offers.
    let late = primary.collect_for(Duration::from_secs(1));
    if late.iter().any(|event| {
        matches!(
            event,
            NetworkEvent::TradeStart { result: 3, .. } | NetworkEvent::TradePartnerItem { .. } | NetworkEvent::TradeCompleted { .. }
        )
    }) {
        return Err("a rejected trade still opened or completed".to_owned());
    }

    // Both seats must still be actionable — the rejection path used to leave
    // sessions half-stuck in older clients.
    partner.flush();
    primary
        .net
        .send_whisper_message(&partner.character_name, "post-reject whisper")
        .map_err(|_| "primary disconnected after reject")?;
    partner.wait_for("whisper after trade reject", |event| match event {
        NetworkEvent::WhisperReceived { message, .. } if message.contains("post-reject whisper") => Some(()),
        _ => None,
    })?;
    Ok(())
}

/// Invalid trade offers are refused without mutating either inventory.
///
/// Hercules accepts a zero-amount add with result 0 on some builds (it is a
/// no-op), so zero is only checked for "does not deliver a partner item".
/// Excess stack, bogus index, and excess zeny must not report success.
fn trade_invalid_offers(config: &Config) -> Result<(), String> {
    const RED_POTION: u32 = 501;
    let (mut primary, mut partner) = connect_pair(config)?;
    let index = primary.give_item(RED_POTION, 2)?;
    begin_trade(&mut primary, &mut partner)?;

    // Zero amount — must not show a partner offer.
    partner.flush();
    primary.flush();
    primary.net.trade_add_item(index, 0).map_err(|_| "primary disconnected")?;
    let zero_partner = partner.collect_for(Duration::from_millis(800));
    if zero_partner
        .iter()
        .any(|event| matches!(event, NetworkEvent::TradePartnerItem { amount, .. } if *amount > 0))
    {
        return Err("zero-amount trade add was shown to the partner".to_owned());
    }

    // Excess amount.
    primary.flush();
    primary.net.trade_add_item(index, 99).map_err(|_| "primary disconnected")?;
    let excess = primary.wait_for_within("TradeAddItemResult excess", Duration::from_secs(2), &mut |event| match event {
        NetworkEvent::TradeAddItemResult { result, .. } => Some(*result),
        _ => None,
    });

    // Nonexistent inventory index.
    primary.flush();
    primary
        .net
        .trade_add_item(ragnarok_packets::InventoryIndex(u16::MAX), 1)
        .map_err(|_| "primary disconnected")?;
    let bogus = primary.wait_for_within(
        "TradeAddItemResult bogus index",
        Duration::from_secs(2),
        &mut |event| match event {
            NetworkEvent::TradeAddItemResult { result, .. } => Some(*result),
            _ => None,
        },
    );

    // Excess zeny.
    primary.flush();
    primary.net.trade_add_zeny(u32::MAX).map_err(|_| "primary disconnected")?;
    let zeny = primary.wait_for_within("zeny offer response", Duration::from_secs(2), &mut |event| match event {
        NetworkEvent::TradeAddItemResult { result, .. } => Some(*result),
        NetworkEvent::ChatMessage { .. } | NetworkEvent::MessageTable { .. } => Some(u8::MAX),
        _ => None,
    });

    let _ = primary.net.trade_cancel();
    primary.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));

    for (label, outcome) in [("excess", excess), ("bogus", bogus), ("zeny", zeny)] {
        match outcome {
            Ok(0) => return Err(format!("invalid trade offer ({label}) was accepted")),
            Ok(_) | Err(_) => {}
        }
    }

    // Valid trade still works after the refusals.
    begin_trade(&mut primary, &mut partner)?;
    primary.flush();
    primary.net.trade_add_item(index, 1).map_err(|_| "primary disconnected")?;
    primary.wait_for("TradeAddItemResult valid after invalids", |event| match event {
        NetworkEvent::TradeAddItemResult {
            inventory_index,
            result: 0,
        } if *inventory_index == index => Some(()),
        _ => None,
    })?;
    let _ = primary.net.trade_cancel();
    primary.pump(Duration::from_millis(300));
    Ok(())
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
