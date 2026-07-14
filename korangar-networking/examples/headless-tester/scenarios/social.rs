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
        Scenario::new("trade-cancel", 8, trade_cancel),
        Scenario::new("trade-commit", 8, trade_commit),
    ]
}

pub(super) fn connect_pair(config: &Config) -> Result<(TestContext, TestContext), String> {
    let mut primary = TestContext::connect(config)?;
    let mut partner = TestContext::connect_partner(config)?;
    // The partner is deliberately non-GM and cannot use @warp. Bring the GM
    // test character to the partner's current map instead.
    let map_name = partner.map_name.clone();
    let x = partner.position.x.saturating_add(1);
    let y = partner.position.y;
    primary.warp(&map_name, x, y)?;
    primary.pump(Duration::from_millis(500));
    partner.pump(Duration::from_millis(500));
    eprintln!(
        "    [connect_pair] partner at {}({}, {}), primary at {}({}, {}), partner sees primary: {}",
        partner.map_name,
        partner.position.x,
        partner.position.y,
        primary.map_name,
        primary.position.x,
        primary.position.y,
        partner.entities.contains_key(&primary.player_id)
    );
    Ok((primary, partner))
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
