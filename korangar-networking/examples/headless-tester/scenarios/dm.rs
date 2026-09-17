//! Phase 9 — Seal Cascade DM command suite.
//!
//! Covers the whole `@dm*` console surface: command contracts, dice rolls,
//! flags, quests, rewards, experience, party warp/recall, periodic hazards,
//! instanced dungeons, and a dynamic sweep of every configured beat menu.
//! Party scenarios reuse the Phase 8 dual-client machinery from `social.rs`.

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::{EquipPosition, ExperienceType};

use crate::context::{Config, TestContext};
use crate::scenarios::social::{add_party_member, connect_pair, form_party, leave_party_both};
use crate::scenarios::{Scenario, skipped};

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("dm-roll", 9, dm_roll),
        Scenario::new("dm-roll-hidden", 9, dm_roll_hidden),
        Scenario::new("dm-roll-override", 9, dm_roll_override),
        Scenario::new("dm-roll-bounds", 9, dm_roll_bounds),
        Scenario::new("dm-command-help", 9, dm_command_help),
        Scenario::new("dm-command-contract", 9, dm_command_contract),
        Scenario::new("dm-flags-status", 9, dm_flags_status),
        Scenario::new("dm-quest-lifecycle", 9, dm_quest_lifecycle),
        Scenario::new("dm-party-quest-refresh", 9, dm_party_quest_refresh),
        Scenario::new("quest-log-multi", 9, quest_log_multi),
        Scenario::new("dm-reward-delta", 9, dm_reward_delta),
        Scenario::new("dm-experience", 9, dm_experience),
        Scenario::new("dm-authored-exp-award", 9, dm_authored_exp_award),
        Scenario::new("dm-warp-recall", 9, dm_warp_recall),
        Scenario::new("dm-hazard-periodic", 9, dm_hazard_periodic),
        Scenario::new("dm-instance-lifecycle", 9, dm_instance_lifecycle),
        Scenario::new("dm-checkpoint-reconcile", 9, dm_checkpoint_reconcile),
        Scenario::new("dm-typed-objective-sync", 9, dm_typed_objective_sync),
        Scenario::new("dm-encounter-objective-sync", 9, dm_encounter_objective_sync),
        Scenario::new("dm-story-objective-sync", 9, dm_story_objective_sync),
        Scenario::new("dm-collect-objective-refresh", 9, dm_collect_objective_refresh),
        Scenario::new("dm-checkpoint-item-consume", 9, dm_checkpoint_item_consume),
        Scenario::new("dm-kill-objective-refresh", 9, dm_kill_objective_refresh),
        Scenario::new("dm-objective-type-matrix", 9, dm_objective_type_matrix),
        Scenario::new("dm-beat-table", 9, dm_beat_table),
        Scenario::new("dm-story-beats", 9, dm_story_beats),
        Scenario::new("dm-golden-beats", 9, dm_golden_beats),
    ]
}

fn wait_for_text(context: &mut TestContext, label: &str, needle: &str) -> Result<String, String> {
    context.wait_for(label, |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains(needle) => Some(text.clone()),
        _ => None,
    })
}

/// QW-052: run one real fixture for every typed objective kind against the
/// same disposable server build. Each constituent fixture owns its cleanup;
/// this wrapper only provides the matrix-level contract and failure context.
fn dm_objective_type_matrix(config: &Config) -> Result<(), String> {
    let fixtures: [(&str, fn(&Config) -> Result<(), String>); 5] = [
        ("Talk", dm_typed_objective_sync),
        ("DM encounter", dm_encounter_objective_sync),
        ("Explore/Interact", dm_story_objective_sync),
        ("Collect", dm_collect_objective_refresh),
        ("Kill", dm_kill_objective_refresh),
    ];
    for (kind, fixture) in fixtures {
        fixture(config).map_err(|error| format!("{kind} objective fixture failed: {error}"))?;
    }
    Ok(())
}

/// Send a command and require feedback text containing `needle`.
fn say_expect(context: &mut TestContext, command: &str, needle: &str) -> Result<String, String> {
    context.flush();
    context.say(command)?;
    wait_for_text(context, &format!("feedback for {command}"), needle)
        .map_err(|error| format!("{command}: expected feedback containing {needle:?}: {error}"))
}

fn dm_roll(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.flush();
    context.say("@roll 2d6+3")?;
    let text = wait_for_text(&mut context, "public dice result", "rolled 2d6+3:")?;
    let total = parse_roll_total(&text, "rolled 2d6+3:")?;
    if !(5..=15).contains(&total) {
        return Err(format!("2d6+3 total {total} is outside 5..=15"));
    }
    Ok(())
}

fn parse_roll_total(text: &str, marker: &str) -> Result<i32, String> {
    text.split_once(marker)
        .and_then(|(_, total)| total.trim().split_whitespace().next())
        .and_then(|total| total.parse::<i32>().ok())
        .ok_or_else(|| format!("could not parse roll total from {text:?}"))
}

fn dm_roll_hidden(config: &Config) -> Result<(), String> {
    let (mut context, mut partner) = connect_pair(config)?;
    form_party(&mut context, &mut partner)?;
    context.flush();
    context.say("@roll hidden 1d20+1")?;
    wait_for_text(&mut context, "hidden dice result", "1d20+1")?;
    Ok(())
}

fn dm_roll_override(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.flush();
    context.say("@roll override 17 headless-check")?;
    let text = wait_for_text(&mut context, "transparent overridden roll", "17")?;
    if !text.contains("headless-check") {
        return Err(format!("override result omitted its audit note: {text:?}"));
    }
    Ok(())
}

/// Deterministic bounds, malformed input, and public-vs-hidden delivery.
/// Note: the server clamps dice sides to a minimum of 2, so `1d1` cannot
/// equal 1 — it behaves as 1d2 (design delta recorded in the test docs).
fn dm_roll_bounds(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;

    primary.flush();
    primary.say("@roll 1d1")?;
    let text = wait_for_text(&mut primary, "clamped 1d1 result", "rolled 1d1:")?;
    let total = parse_roll_total(&text, "rolled 1d1:")?;
    if !(1..=2).contains(&total) {
        return Err(format!("1d1 (clamped to 1d2) total {total} is outside 1..=2 — raw: {text:?}"));
    }

    primary.flush();
    primary.say("@roll 4d8-2")?;
    let text = wait_for_text(&mut primary, "4d8-2 result", "rolled 4d8-2:")?;
    let total = parse_roll_total(&text, "rolled 4d8-2:")?;
    if !(2..=30).contains(&total) {
        return Err(format!("4d8-2 total {total} is outside 2..=30"));
    }

    say_expect(&mut primary, "@roll garbage", "Usage: @roll")?;

    // Public rolls are map announcements and must reach the partner.
    partner.flush();
    primary.flush();
    primary.say("@roll 2d6+3")?;
    partner.wait_for("public roll reaching the partner", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("rolled 2d6+3:") => Some(()),
        _ => None,
    })?;

    // Hidden rolls must NOT reach the partner.
    partner.flush();
    primary.flush();
    primary.say("@roll hidden 3d4+1")?;
    wait_for_text(&mut primary, "hidden roll self-feedback", "3d4+1")?;
    let leaked = partner
        .collect_for(Duration::from_secs(1))
        .iter()
        .any(|event| matches!(event, NetworkEvent::ChatMessage { text, .. } if text.contains("3d4+1")));
    if leaked {
        return Err("hidden roll was announced to the partner".to_owned());
    }
    Ok(())
}

fn dm_command_help(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    // Only read-only commands belong in this contract test. Several shortcut
    // commands intentionally perform a default action when invoked without
    // arguments (for example, @dmreward grants loot).
    for (command, expected) in [("@dm", "[DM]"), ("@dm help", "[DM]"), ("@dmstatus", "[DM]")] {
        context.flush();
        context.say(command)?;
        wait_for_text(&mut context, &format!("feedback for {command}"), expected)?;
    }
    Ok(())
}

/// Table-driven no-arg / invalid-arg contract for every bound `@dm*` command
/// that answers with deterministic feedback and no side effects. Commands
/// whose no-arg form mutates state (@dmreward grants, @dmhazard arms) are
/// exercised through their read-only/cleanup forms here and get dedicated
/// scenarios for the mutating paths.
fn dm_command_contract(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let table: &[(&str, &str)] = &[
        ("@dm", "@dm mode <on|off>"),
        ("@dm bogussub", "Unknown subcommand"),
        ("@dmflag", "Usage: @dmflag"),
        // The flag NAME is validated before the action is dispatched, so this
        // row has to pass a well-formed `dm_*` name or it never reaches the
        // unknown-action branch. It used to say `some_flag` and silently
        // asserted the wrong message from 2026-08-18, when the security pass
        // added DM_ValidFlagName, until the first full-suite run found it.
        ("@dmflag bogusaction dm_bogus", "Unknown flag action"),
        // And the validation itself, which nothing covered.
        ("@dmflag get some_flag", "Flag names must match"),
        ("@dmquest", "Usage: @dmquest"),
        ("@dmquest start notanumber", "Usage: @dmquest"),
        ("@dmwarp", "Usage: @dm warp"),
        ("@dminstance", "Usage: @dm instance"),
        ("@dminstance bogus", "Usage: @dm instance"),
        ("@dmmode", "Mode is currently"),
        ("@dmexp", "Usage: @dm exp"),
        ("@dmreset", "reset confirm"),
        ("@dmhazard clear", "Hazard cleared"),
        ("@dmstatus", "Mode="),
        ("@dmrecall", "Recalled"),
        ("@dmcleanup", "cleanup complete"),
        ("@dmstory headless contract probe", "headless contract probe"),
    ];
    for (command, expected) in table {
        say_expect(&mut context, command, expected)?;
    }
    Ok(())
}

/// QW-055: exercise the real two-client checkpoint transport. The scenario is
/// an explicit expected skip until the DBA applies the checkpoint migration;
/// once the table exists, a missing DMJ preview/confirm result is a failure.
fn dm_checkpoint_reconcile(config: &Config) -> Result<(), String> {
    if std::env::var("QW_CHECKPOINT_DB_READY").ok().as_deref() != Some("1") {
        return skipped("campaign checkpoint migration unavailable");
    }
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let result = (|| -> Result<(), String> {
        primary.flush();
        partner.flush();
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        primary.say("@dm mode on")?;
        wait_for_text(&mut primary, "campaign mode enable", "DnD mode enabled")?;
        partner.wait_for("partner checkpoint snapshot", |event| match event {
            NetworkEvent::ChatMessage { text, .. } if text.starts_with("[DMJ]") && text.contains("\"t\":\"checkpoint") => Some(()),
            _ => None,
        })?;
        // Advance through the same party-authoritative path used by campaign
        // beats. The checkpoint echo carries the actor who owns the current
        // item-bearing step; it is not inferred from the local client.
        let primary_char_id = primary.character_id.0;
        primary.flush();
        partner.flush();
        primary.say("@dmflag set dm_qw055_probe_a 1")?;
        let first = primary.wait_for("primary checkpoint after first party transition", |event| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]")
                    && text.contains("\"t\":\"checkpoint")
                    && text.contains("\"step\":1")
                    && text.contains(&format!("\"carrier\":{primary_char_id}")) =>
            {
                Some(text.clone())
            }
            _ => None,
        })?;
        partner.wait_for("partner checkpoint after first party transition", |event| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]") && text.contains("\"t\":\"checkpoint") && text.contains("\"step\":1") =>
            {
                Some(())
            }
            _ => None,
        })?;
        if !first.contains("\"carrier\":") {
            return Err(format!("first checkpoint omitted carried-item owner: {first}"));
        }

        // Give the independent partner a deliberately-ahead account mirror
        // through Hercules' normal registry command. This is the same
        // account variable consumed by DM_CheckpointSyncParty; no SQL or
        // client-side state is injected into the fixture.
        partner.say("@set #dm_campaign_checkpoint_step 99")?;
        partner.pump(Duration::from_millis(500));

        // Logout the partner before the next authoritative transition. The
        // server must advance the party row while leaving the independent
        // member's ahead mirror untouched; reconnect reconciliation then
        // catches the member up only if the guard permits it.
        drop(partner);
        primary.flush();
        primary.say("@dmflag set dm_qw055_probe_b 1")?;
        primary.wait_for("primary checkpoint while partner is offline", |event| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]")
                    && text.contains("\"t\":\"checkpoint")
                    && text.contains("\"step\":2")
                    && text.contains(&format!("\"carrier\":{primary_char_id}")) =>
            {
                Some(())
            }
            _ => None,
        })?;

        partner = TestContext::connect_partner(config)?;
        partner.pump(Duration::from_millis(500));
        primary.flush();
        partner.flush();
        primary.say("@dm reconcile confirm")?;
        primary.wait_for("reconnect reconciliation result", |event| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]") && text.contains("\"t\":\"reconcile") && text.contains("\"mode\":\"confirm") =>
            {
                Some(())
            }
            _ => None,
        })?;
        // Leave and rejoin without erasing the durable member record. The
        // following transition must still reach the returning member and keep
        // the same carried-item owner.
        partner.net.leave_party().map_err(|_| "partner disconnected while leaving party")?;
        partner.pump(Duration::from_millis(500));
        primary.pump(Duration::from_millis(500));
        add_party_member(&mut primary, &mut partner)?;
        primary.flush();
        partner.flush();
        primary.say("@dmflag set dm_qw055_probe_c 1")?;
        primary.wait_for("primary checkpoint after leave and rejoin", |event| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]")
                    && text.contains("\"t\":\"checkpoint")
                    && text.contains("\"step\":3")
                    && text.contains(&format!("\"carrier\":{primary_char_id}")) =>
            {
                Some(())
            }
            _ => None,
        })?;
        partner.wait_for("ahead refusal after leave and rejoin", |event| match event {
            NetworkEvent::ChatMessage { text, .. } if text.contains("checkpoint member") && text.contains("REFUSED (ahead)") => Some(()),
            _ => None,
        })?;

        primary.flush();
        primary.say("@dm reconcile")?;
        let preview = primary.collect_for(Duration::from_secs(2));
        let preview_text = preview.iter().find_map(|event| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]") && text.contains("\"t\":\"reconcile\"") && text.contains("\"mode\":\"preview\"") =>
            {
                Some(text.clone())
            }
            _ => None,
        });
        if preview
            .iter()
            .any(|event| matches!(event, NetworkEvent::ChatMessage { text, .. } if text.contains("No durable checkpoint exists")))
        {
            return skipped("campaign checkpoint migration unavailable");
        }
        let Some(preview_text) = preview_text else {
            return Err("@dm reconcile preview produced no DMJ result".to_owned());
        };
        if !preview_text.contains("\"t\":\"reconcile") || !preview_text.contains("\"mode\":\"preview") {
            return Err(format!("unexpected preview DMJ payload: {preview_text}"));
        }
        if !preview_text.contains("\"ahead\":1") {
            return Err(format!("preview did not count the independent ahead member: {preview_text}"));
        }

        primary.flush();
        partner.flush();
        primary.say("@dm reconcile confirm")?;
        let confirm = primary.wait_for("DMJ reconciliation confirmation", |event| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]") && text.contains("\"t\":\"reconcile") && text.contains("\"mode\":\"confirm") =>
            {
                Some(text.clone())
            }
            _ => None,
        })?;
        if !confirm.contains("\"changed\":") {
            return Err(format!("confirm DMJ omitted changed count: {confirm}"));
        }
        partner.wait_for("ahead partner refusal after confirm", |event| match event {
            NetworkEvent::ChatMessage { text, .. } if text.contains("checkpoint member") && text.contains("REFUSED (ahead)") => Some(()),
            _ => None,
        })?;
        // Reset while the party still exists so the durable row and member
        // mirrors are removed before the normal party teardown.
        let _ = primary.say("@dm reset confirm");
        let _ = primary.say("@dm mode off");
        leave_party_both(&mut primary, &mut partner);
        Ok(())
    })();

    let _ = primary.say("@dm reset confirm");
    let _ = primary.say("@dm mode off");
    result
}

/// QW-052: run the real Arc 1 Mira beat and require both party clients to
/// receive the server-authored typed Talk objective for quest 20006.
fn dm_typed_objective_sync(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let result = (|| -> Result<(), String> {
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "DnD mode enabled")?;

        let menu = open_beat_menu(&mut primary, 1)?;
        let choice = menu
            .choices
            .iter()
            .position(|choice| choice == "Beat - Mira found (20006)")
            .ok_or_else(|| format!("Arc 1 beat menu omitted Mira objective: {:?}", menu.choices))?;
        primary.flush();
        partner.flush();
        primary
            .net
            .choose_dialog_option(menu.npc_id, (choice + 1) as i8)
            .map_err(|_| "primary disconnected while selecting Mira beat")?;

        let objective_match = |event: &NetworkEvent| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]")
                    && text.contains("\"t\":\"objective")
                    && text.contains("\"quest_id\":20006")
                    && text.contains("\"kind\":\"Talk\"")
                    && text.contains("\"completed\":1") =>
            {
                Some(())
            }
            _ => None,
        };
        primary.wait_for("primary typed Talk objective", objective_match)?;
        partner.wait_for("partner typed Talk objective", objective_match)?;

        // The menu beat pauses on its final page after producing the objective.
        // Close it explicitly before resetting so no dialog state leaks to the
        // next scenario.
        let _ = primary.net.close_dialog(menu.npc_id);
        primary.pump(Duration::from_millis(300));
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode off", "DnD mode disabled")?;
        leave_party_both(&mut primary, &mut partner);
        Ok(())
    })();

    let _ = primary.say("@dm reset confirm");
    let _ = primary.say("@dm mode off");
    result
}

/// QW-052: run the real Arc 2 completion helper and require both party clients
/// to receive its server-authored DM encounter objective for quest 20012.
fn dm_encounter_objective_sync(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let result = (|| -> Result<(), String> {
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "DnD mode enabled")?;

        let menu = open_beat_menu(&mut primary, 2)?;
        let choice = menu
            .choices
            .iter()
            .position(|choice| choice == "Beat - Complete Arc 2")
            .ok_or_else(|| format!("Arc 2 beat menu omitted completion helper: {:?}", menu.choices))?;
        primary.flush();
        partner.flush();
        primary
            .net
            .choose_dialog_option(menu.npc_id, (choice + 1) as i8)
            .map_err(|_| "primary disconnected while selecting Arc 2 completion")?;

        let objective_match = |event: &NetworkEvent| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]")
                    && text.contains("\"t\":\"objective")
                    && text.contains("\"quest_id\":20012")
                    && text.contains("\"kind\":\"DM\"")
                    && text.contains("\"completed\":1") =>
            {
                Some(())
            }
            _ => None,
        };
        primary.wait_for("primary DM encounter objective", objective_match)?;
        partner.wait_for("partner DM encounter objective", objective_match)?;

        let _ = primary.net.close_dialog(menu.npc_id);
        primary.pump(Duration::from_millis(300));
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode off", "DnD mode disabled")?;
        leave_party_both(&mut primary, &mut partner);
        Ok(())
    })();

    let _ = primary.say("@dm reset confirm");
    let _ = primary.say("@dm mode off");
    leave_party_both(&mut primary, &mut partner);
    result
}

/// QW-052: drive the real Arc 1 painted-sluice and binding-stone interactions
/// and require both party clients to receive their server-authored typed
/// updates.
fn dm_story_objective_sync(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let result = (|| -> Result<(), String> {
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "DnD mode enabled")?;
        say_expect(&mut primary, "@dmquest start 20001", "started")?;

        // The shortcut mirrors the real Painted Sluice#dm producer. The
        // map-server fixture does not expose NPC entities on prt_sewb1, so a
        // direct client click is a separate GUI/content gate.
        run_beat_objective(&mut primary, &mut partner, 1, "Beat - Drain chamber", 20001, 2, "Explore", 1)?;

        // Reset the authoritative party state before the independent Interact
        // fixture, so this cannot pass from a prior story transition.
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "DnD mode enabled")?;
        say_expect(&mut primary, "@dmquest start 20005", "started")?;

        // The shortcut mirrors the real Binding Stone#dm producer. The direct
        // Tide-Wheel#dm path remains a separate GUI/content gate.
        run_beat_objective(&mut primary, &mut partner, 1, "Beat - Binding word", 20005, 1, "Interact", 0)?;

        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode off", "DnD mode disabled")?;
        leave_party_both(&mut primary, &mut partner);
        Ok(())
    })();

    let _ = primary.say("@dm reset confirm");
    let _ = primary.say("@dm mode off");
    leave_party_both(&mut primary, &mut partner);
    result
}

fn wait_for_typed_objective(context: &mut TestContext, quest_id: u32, objective_id: u32, kind: &str, completed: u8) -> Result<(), String> {
    let label = format!("typed {kind} objective for quest {quest_id}");
    context.wait_for(&label, |event| match event {
        NetworkEvent::ChatMessage { text, .. }
            if text.starts_with("[DMJ]")
                && text.contains("\"t\":\"objective")
                && text.contains(&format!("\"quest_id\":{quest_id}"))
                && text.contains(&format!("\"objective_id\":{objective_id}"))
                && text.contains(&format!("\"kind\":\"{kind}\""))
                && text.contains(&format!("\"completed\":{completed}")) =>
        {
            Some(())
        }
        _ => None,
    })
}

fn run_beat_objective(
    primary: &mut TestContext,
    partner: &mut TestContext,
    arc: u8,
    label: &str,
    quest_id: u32,
    objective_id: u32,
    kind: &str,
    completed: u8,
) -> Result<(), String> {
    let menu = open_beat_menu(primary, arc)?;
    let choice = menu
        .choices
        .iter()
        .position(|choice| choice == label)
        .ok_or_else(|| format!("Arc {arc} beat menu omitted {label:?}: {:?}", menu.choices))?;
    primary.flush();
    partner.flush();
    primary
        .net
        .choose_dialog_option(menu.npc_id, (choice + 1) as i8)
        .map_err(|_| format!("disconnected selecting {label}"))?;
    wait_for_typed_objective(primary, quest_id, objective_id, kind, completed)?;
    wait_for_typed_objective(partner, quest_id, objective_id, kind, completed)?;
    let _ = primary.net.close_dialog(menu.npc_id);
    primary.pump(Duration::from_millis(300));
    Ok(())
}

/// Set/get/clear a probe flag (with relogin persistence — campaign flags are
/// permanent character variables) and check the @dmstatus flag surface.
fn dm_flags_status(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    say_expect(&mut context, "@dmflag set dm_hl_probe 7", "dm_hl_probe set to 7")?;
    say_expect(&mut context, "@dmflag get dm_hl_probe", "dm_hl_probe = 7")?;

    // Permanent character variable: must survive a relogin.
    drop(context);
    std::thread::sleep(Duration::from_millis(700));
    let mut context = TestContext::connect(config)?;
    say_expect(&mut context, "@dmflag get dm_hl_probe", "dm_hl_probe = 7")
        .map_err(|error| format!("flag did not persist across relogin: {error}"))?;

    say_expect(&mut context, "@dmflag clear dm_hl_probe", "cleared")?;
    say_expect(&mut context, "@dmflag get dm_hl_probe", "dm_hl_probe = 0")?;

    // A campaign flag surfaced by @dmstatus.
    say_expect(&mut context, "@dmflag set dm_arc04_cassell_unmasked 1", "set to 1")?;
    say_expect(&mut context, "@dmstatus", "cassell=1")?;
    say_expect(&mut context, "@dmflag clear dm_arc04_cassell_unmasked", "cleared")?;
    say_expect(&mut context, "@dmstatus", "cassell=0")?;
    Ok(())
}

/// Quest start/complete/erase over the wire: QuestAdded / QuestList (relogin
/// persistence) / QuestRemoved events plus the @dmstatus arc progress digits.
fn dm_quest_lifecycle(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 20001; // "Omens at the Fountain" (Arc 1 anchor quest)

    let mut context = TestContext::connect(config)?;
    // Clean slate: erase is idempotent feedback-wise.
    say_expect(&mut context, &format!("@dmquest erase {QUEST_ID}"), "erased")?;

    context.flush();
    context.say(&format!("@dmquest start {QUEST_ID}"))?;
    context.wait_for("QuestAdded", |event| match event {
        NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == QUEST_ID => Some(()),
        _ => None,
    })?;
    wait_for_text(&mut context, "quest start feedback", "started")?;
    say_expect(&mut context, "@dmstatus", "A01:1")?;

    // The active quest must be in the quest log after a fresh map login.
    drop(context);
    std::thread::sleep(Duration::from_millis(700));
    let mut context = TestContext::connect(config)?;
    context.wait_for("QuestList containing the started quest", |event| match event {
        NetworkEvent::QuestList { quest_ids } if quest_ids.contains(&QUEST_ID) => Some(()),
        _ => None,
    })?;

    say_expect(&mut context, &format!("@dmquest complete {QUEST_ID}"), "completed")?;
    say_expect(&mut context, "@dmstatus", "A01:2")?;

    context.flush();
    context.say(&format!("@dmquest erase {QUEST_ID}"))?;
    context.wait_for("QuestRemoved", |event| match event {
        NetworkEvent::QuestRemoved { quest_id } if *quest_id == QUEST_ID => Some(()),
        _ => None,
    })?;
    wait_for_text(&mut context, "quest erase feedback", "erased")?;
    say_expect(&mut context, "@dmstatus", "A01:0")?;
    Ok(())
}

/// QW-056: verify party quest refreshes, offline retention, reconnect
/// hydration, and another member's authoritative completion/removal.
fn dm_party_quest_refresh(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 20001;

    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let result = (|| -> Result<(), String> {
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "DnD mode enabled")?;
        say_expect(&mut primary, &format!("@dmquest erase {QUEST_ID}"), "erased")?;

        primary.flush();
        partner.flush();
        primary.say(&format!("@dmquest start {QUEST_ID}"))?;
        primary.wait_for("primary party QuestAdded", |event| match event {
            NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        partner.wait_for("partner party QuestAdded", |event| match event {
            NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;

        // Completion while the partner is offline must not mutate that
        // member's character quest row behind its back.
        drop(partner);
        primary.flush();
        primary.say(&format!("@dmquest complete {QUEST_ID}"))?;
        primary.wait_for("primary QuestRemoved after completion", |event| match event {
            NetworkEvent::QuestRemoved { quest_id } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        wait_for_text(&mut primary, "primary quest completion feedback", "completed")?;

        // Reconnect must hydrate the offline member's own authoritative quest
        // list; it should still have the active quest that was not completed
        // while offline.
        partner = TestContext::connect_partner(config)?;
        partner.wait_for("reconnected QuestList retaining active quest", |event| match event {
            NetworkEvent::QuestList { quest_ids } if quest_ids.contains(&QUEST_ID) => Some(()),
            _ => None,
        })?;

        add_party_member(&mut primary, &mut partner)?;
        primary.flush();
        partner.flush();
        primary.say(&format!("@dmquest erase {QUEST_ID}"))?;
        primary.wait_for("primary QuestRemoved after party erase", |event| match event {
            NetworkEvent::QuestRemoved { quest_id } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        partner.wait_for("partner QuestRemoved after party erase", |event| match event {
            NetworkEvent::QuestRemoved { quest_id } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        wait_for_text(&mut primary, "party quest erase feedback", "erased")?;

        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode off", "DnD mode disabled")?;
        leave_party_both(&mut primary, &mut partner);
        Ok(())
    })();

    let _ = primary.say("@dm reset confirm");
    let _ = primary.say("@dm mode off");
    result
}

/// QW-052: start the real Arc 1 inventory-backed hunt and verify its Collect
/// requirements update from authoritative inventory additions.
fn dm_collect_objective_refresh(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 20002;
    const REQUIRED: [(u32, u16); 3] = [(1016, 7), (1052, 7), (955, 3)];

    let (mut context, mut partner) = connect_pair(config)?;
    form_party(&mut context, &mut partner)?;
    let result = (|| -> Result<(), String> {
        say_expect(&mut context, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut context, "@dm mode on", "DnD mode enabled")?;
        say_expect(&mut context, &format!("@dmquest erase {QUEST_ID}"), "erased")?;
        for (item_id, _) in REQUIRED {
            let _ = context.say(&format!("@delitem {item_id} 30000"));
        }
        context.pump(Duration::from_millis(300));

        context.flush();
        context.say(&format!("@dmquest start {QUEST_ID}"))?;
        context.wait_for("collect quest added", |event| match event {
            NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        wait_for_text(&mut context, "collect quest start feedback", "started")?;

        for (item_id, required) in REQUIRED {
            context.give_item(item_id, required)?;
            let carried = context
                .inventory
                .iter()
                .filter(|item| item.item_id.0 == item_id)
                .map(|item| item.amount() as u32)
                .sum::<u32>();
            if carried < required as u32 {
                return Err(format!("collect objective item {item_id} has {carried}, need {required}"));
            }
        }

        // The quest remains active until its authoritative turn-in NPC path;
        // this scenario deliberately does not complete it by admin command.
        if !context.inventory.iter().any(|item| item.item_id.0 == 1016) {
            return Err("collect objective inventory state disappeared after additions".to_owned());
        }
        for (item_id, _) in REQUIRED {
            let _ = context.say(&format!("@delitem {item_id} 30000"));
        }
        context.pump(Duration::from_millis(300));
        say_expect(&mut context, &format!("@dmquest erase {QUEST_ID}"), "erased")?;
        say_expect(&mut context, "@dm mode off", "DnD mode disabled")?;
        context.pump(Duration::from_millis(300));
        leave_party_both(&mut context, &mut partner);
        Ok(())
    })();

    let _ = context.say(&format!("@dmquest erase {QUEST_ID}"));
    let _ = context.say("@dm mode off");
    leave_party_both(&mut context, &mut partner);
    result
}

/// QW-055: consume a carried contract through the real Wynne turn-in and
/// verify the durable checkpoint clears its authoritative carrier for both
/// party members.
fn dm_checkpoint_item_consume(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 20002;
    const REQUIRED: [(u32, u16); 3] = [(1016, 7), (1052, 7), (955, 3)];

    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;
    let result = (|| -> Result<(), String> {
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "DnD mode enabled")?;
        say_expect(&mut primary, "@dmflag set dm_arc01_started 1", "set to 1")?;
        say_expect(&mut primary, "@dmflag set dm_qw055_consume_probe 1", "set to 1")?;
        say_expect(&mut primary, &format!("@dmquest erase {QUEST_ID}"), "erased")?;
        for (item_id, _) in REQUIRED {
            let _ = primary.say(&format!("@delitem {item_id} 30000"));
        }
        primary.pump(Duration::from_millis(300));
        primary.say(&format!("@dmquest start {QUEST_ID}"))?;
        primary.wait_for("checkpoint consume hunt quest added", |event| match event {
            NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        wait_for_text(&mut primary, "checkpoint consume hunt start feedback", "started")?;
        for (item_id, required) in REQUIRED {
            primary.give_item(item_id, required)?;
        }

        primary.warp("prontera", 156, 190)?;
        primary.pump(Duration::from_millis(400));
        let wynne = primary
            .entities
            .iter()
            .filter_map(|(id, entity)| {
                let position = entity.position.tile_position();
                let distance = position.x.abs_diff(156).max(position.y.abs_diff(191));
                (distance <= 2).then_some((*id, distance))
            })
            .min_by_key(|(_, distance)| *distance)
            .map(|(id, _)| id)
            .ok_or("Quartermaster Wynne was not visible near (156,191)")?;

        primary.flush();
        partner.flush();
        primary.net.start_dialog(wynne).map_err(|_| "primary disconnected at Wynne")?;
        primary.wait_for("Wynne contract dialog", |event| match event {
            NetworkEvent::OpenDialog { npc_id, .. } if *npc_id == wynne => Some(()),
            _ => None,
        })?;

        let mut carrier_cleared_on_primary = false;
        let mut choices = None;
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while choices.is_none() && std::time::Instant::now() < deadline {
            for event in primary.collect_for(Duration::from_millis(200)) {
                match event {
                    NetworkEvent::ChatMessage { text, .. }
                        if text.starts_with("[DMJ]") && text.contains("\"t\":\"checkpoint") && text.contains("\"carrier\":0") =>
                    {
                        carrier_cleared_on_primary = true;
                    }
                    NetworkEvent::AddNextButton { npc_id } if npc_id == wynne => {
                        let _ = primary.net.next_dialog(wynne);
                    }
                    NetworkEvent::AddChoiceButtons { npc_id, choices: found } if npc_id == wynne => {
                        choices = Some(found);
                    }
                    _ => {}
                }
            }
        }
        let choices = choices.ok_or("Wynne turn-in menu did not arrive")?;
        let finish = choices
            .iter()
            .position(|choice| choice == "That's all.")
            .unwrap_or_else(|| choices.len().saturating_sub(1));
        primary
            .net
            .choose_dialog_option(wynne, (finish + 1) as i8)
            .map_err(|_| "primary disconnected choosing Wynne close")?;
        primary.wait_for("Wynne close button", |event| match event {
            NetworkEvent::AddCloseButton { npc_id } if *npc_id == wynne => Some(()),
            _ => None,
        })?;
        primary.net.close_dialog(wynne).map_err(|_| "primary disconnected closing Wynne")?;
        primary.pump(Duration::from_millis(300));

        if !carrier_cleared_on_primary {
            primary.wait_for("primary carrier-cleared checkpoint", |event| match event {
                NetworkEvent::ChatMessage { text, .. }
                    if text.starts_with("[DMJ]") && text.contains("\"t\":\"checkpoint") && text.contains("\"carrier\":0") =>
                {
                    Some(())
                }
                _ => None,
            })?;
        }
        partner.wait_for("partner carrier-cleared checkpoint", |event| match event {
            NetworkEvent::ChatMessage { text, .. }
                if text.starts_with("[DMJ]") && text.contains("\"t\":\"checkpoint") && text.contains("\"carrier\":0") =>
            {
                Some(())
            }
            _ => None,
        })?;

        if REQUIRED
            .iter()
            .any(|(item_id, _)| primary.inventory.iter().any(|item| item.item_id.0 == *item_id))
        {
            return Err("Wynne turn-in did not consume all carried contract items".to_owned());
        }
        say_expect(&mut primary, &format!("@dmquest erase {QUEST_ID}"), "erased")?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode off", "DnD mode disabled")?;
        leave_party_both(&mut primary, &mut partner);
        Ok(())
    })();

    let _ = primary.say(&format!("@dmquest erase {QUEST_ID}"));
    let _ = primary.say("@dm reset confirm");
    let _ = primary.say("@dm mode off");
    leave_party_both(&mut primary, &mut partner);
    result
}

/// QW-052: use Hercules' real hunting quest/objective packet path and verify
/// one Zerom kill advances the authoritative Kill count on the client.
fn dm_kill_objective_refresh(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 1100;
    const ZEROM_ID: u32 = 1178;

    let mut context = TestContext::connect(config)?;
    let result =
        (|| -> Result<(), String> {
            context.ensure_job(4008)?;
            context.ensure_base_level(99)?;
            context.say("@allstats")?;
            context.say("@heal")?;
            context.pump(Duration::from_millis(400));
            context.warp("prt_fild08", 170, 180)?;
            context.say("@allskill")?;
            context.wait_for("SkillTree after @allskill", |event| match event {
                NetworkEvent::SkillTree { skill_information } if !skill_information.is_empty() => Some(()),
                _ => None,
            })?;
            context.say("@delitem 1126 10")?;
            context.pump(Duration::from_millis(200));
            let sword = context.give_item(1126, 1)?;
            context
                .net
                .request_item_equip(sword, EquipPosition::RIGHT_HAND)
                .map_err(|_| "disconnected equipping Saber")?;
            context.wait_for("Saber equipped", |event| match event {
                NetworkEvent::UpdateEquippedPosition { index, equipped_position }
                    if *index == sword && equipped_position.contains(EquipPosition::RIGHT_HAND) =>
                {
                    Some(())
                }
                _ => None,
            })?;
            let _ = context.say(&format!("@quest delete {QUEST_ID}"));
            context.pump(Duration::from_millis(300));
            context.flush();
            context.say(&format!("@quest add {QUEST_ID}"))?;
            context.wait_for("kill quest added", |event| match event {
                NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == QUEST_ID => Some(()),
                _ => None,
            })?;

            let initial = context.wait_for("initial Zerom objective packet", |event| match event {
                NetworkEvent::QuestObjectiveProgress { objectives }
                    if objectives.iter().any(|objective| {
                        objective.quest_id == QUEST_ID
                            && objective.mob_id == ZEROM_ID
                            && objective.current_count == 0
                            && objective.total_count >= 1
                    }) =>
                {
                    Some(())
                }
                _ => None,
            });
            if initial.is_err() {
                return Err(initial.unwrap_err());
            }

            context.kill_all_monsters();
            let target = context.spawn_monster("ZEROM", ZEROM_ID as u16)?;
            let player_id = context.player_id;
            let mut landed = false;
            let mut killed = false;
            let mut objective_seen = false;
            for _ in 0..5 {
                let position = context
                    .entities
                    .get(&target)
                    .map(|entity| entity.position.tile_position())
                    .ok_or("Zerom disappeared before the first attack")?;
                context.walk_to(position.x.saturating_sub(1).max(5), position.y.max(5))?;
                context.flush();
                context.net.player_attack(target).map_err(|_| "disconnected attacking Zerom")?;
                let outcome = match context.wait_for_within("Zerom DamageEffect or death", Duration::from_secs(5), &mut |event| match event
                {
                    NetworkEvent::QuestObjectiveProgress { objectives }
                        if objectives
                            .iter()
                            .any(|objective| objective.quest_id == QUEST_ID && objective.current_count >= 1) =>
                    {
                        Some(3)
                    }
                    NetworkEvent::RemoveEntity {
                        entity_id,
                        reason: ragnarok_packets::DisappearanceReason::Died,
                    } if *entity_id == target => Some(2),
                    NetworkEvent::DamageEffect {
                        source_entity_id,
                        destination_entity_id,
                        damage_amount: Some(amount),
                        ..
                    } if *source_entity_id == player_id && *destination_entity_id == target && *amount > 0 => Some(1),
                    NetworkEvent::AttackFailed { target_entity_id, .. } if *target_entity_id == target => Some(0),
                    _ => None,
                }) {
                    Ok(outcome) => outcome,
                    Err(_) => continue,
                };
                match outcome {
                    3 => {
                        killed = true;
                        objective_seen = true;
                        break;
                    }
                    2 => {
                        killed = true;
                        break;
                    }
                    1 => {
                        landed = true;
                        context.pump(Duration::from_millis(500));
                        break;
                    }
                    _ => {}
                }
            }
            if landed && !killed {
                let deadline = std::time::Instant::now() + Duration::from_secs(30);
                while std::time::Instant::now() < deadline {
                    context.pump(Duration::from_millis(500));
                    context.net.player_attack(target).map_err(|_| "disconnected attacking Zerom")?;
                    let outcome = match context.wait_for_within("next Zerom hit or death", Duration::from_secs(6), &mut |event| match event
                    {
                        NetworkEvent::QuestObjectiveProgress { objectives }
                            if objectives
                                .iter()
                                .any(|objective| objective.quest_id == QUEST_ID && objective.current_count >= 1) =>
                        {
                            Some(3)
                        }
                        NetworkEvent::RemoveEntity {
                            entity_id,
                            reason: ragnarok_packets::DisappearanceReason::Died,
                        } if *entity_id == target => Some(2),
                        NetworkEvent::DamageEffect {
                            source_entity_id,
                            destination_entity_id,
                            ..
                        } if *source_entity_id == player_id && *destination_entity_id == target => Some(1),
                        NetworkEvent::AttackFailed { target_entity_id, .. } if *target_entity_id == target => Some(0),
                        _ => None,
                    }) {
                        Ok(outcome) => outcome,
                        Err(_) => {
                            if let Some(position) = context.entities.get(&target).map(|entity| entity.position.tile_position()) {
                                let _ = context.walk_to(position.x.saturating_sub(1).max(5), position.y.max(5));
                            }
                            continue;
                        }
                    };
                    if outcome == 3 {
                        killed = true;
                        objective_seen = true;
                        break;
                    }
                    if outcome == 2 {
                        killed = true;
                        break;
                    }
                    if outcome == 0 {
                        let position = context
                            .entities
                            .get(&target)
                            .map(|entity| entity.position.tile_position())
                            .ok_or("Zerom disappeared after an attack failure")?;
                        context.walk_to(position.x.saturating_sub(1).max(5), position.y.max(5))?;
                    }
                }
            }
            if !killed {
                return Err("spawned Zerom did not die during the kill objective fixture".to_owned());
            }

            if !objective_seen {
                context.wait_for("authoritative Zerom kill objective update", |event| match event {
                    NetworkEvent::QuestObjectiveProgress { objectives }
                        if objectives.iter().any(|objective| {
                            objective.quest_id == QUEST_ID
                                && objective.mob_id == ZEROM_ID
                                && objective.current_count >= 1
                                && objective.total_count >= objective.current_count
                        }) =>
                    {
                        Some(())
                    }
                    _ => None,
                })?;
            }

            let _ = context.say(&format!("@quest delete {QUEST_ID}"));
            let _ = context.say("@delitem 1101 1");
            context.pump(Duration::from_millis(300));
            Ok(())
        })();

    let _ = context.say(&format!("@quest delete {QUEST_ID}"));
    let _ = context.say("@delitem 1126 1");
    result
}

/// Two campaign quests must both survive a fresh map login and appear together
/// on `QuestList` — the multipacket shape a quest journal UI will consume.
///
/// Complements `dm-quest-lifecycle` (single-id start/complete/erase). Uses one
/// Act I and one Act II quest so the list is not a single-slot coincidence.
fn quest_log_multi(config: &Config) -> Result<(), String> {
    const QUEST_A: u32 = 20001; // Arc 1 anchor
    const QUEST_B: u32 = 20101; // Arc 6-area Act II id block start

    let mut context = TestContext::connect(config)?;
    for id in [QUEST_A, QUEST_B] {
        say_expect(&mut context, &format!("@dmquest erase {id}"), "erased")?;
    }

    for id in [QUEST_A, QUEST_B] {
        context.flush();
        context.say(&format!("@dmquest start {id}"))?;
        context.wait_for(&format!("QuestAdded {id}"), |event| match event {
            NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == id => Some(()),
            _ => None,
        })?;
        wait_for_text(&mut context, &format!("quest {id} start feedback"), "started")?;
    }

    drop(context);
    std::thread::sleep(Duration::from_millis(700));
    let mut context = TestContext::connect(config)?;
    context.wait_for("QuestList with both campaign quests", |event| match event {
        NetworkEvent::QuestList { quest_ids } if quest_ids.contains(&QUEST_A) && quest_ids.contains(&QUEST_B) => Some(()),
        _ => None,
    })?;

    for id in [QUEST_A, QUEST_B] {
        context.flush();
        context.say(&format!("@dmquest erase {id}"))?;
        context.wait_for(&format!("QuestRemoved {id}"), |event| match event {
            NetworkEvent::QuestRemoved { quest_id } if *quest_id == id => Some(()),
            _ => None,
        })?;
    }
    Ok(())
}

/// Read the wallet through an explicit `@zeny` round trip. The map-login
/// burst does not reliably produce a tracked Zeny stat update, so the tracked
/// value cannot serve as a baseline on its own.
fn probe_zeny(context: &mut TestContext) -> Result<u32, String> {
    context.flush();
    context.say("@zeny 1")?;
    context.wait_for("zeny probe (+1)", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::Zeny(value),
        } => Some(*value),
        _ => None,
    })?;
    context.flush();
    context.say("@zeny -1")?;
    context.wait_for("zeny probe (-1)", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::Zeny(value),
        } => Some(*value),
        _ => None,
    })
}

/// Grant one reward roll and assert the exact announced item and zeny land in
/// the inventory/wallet, persist across relogin, and can be cleaned up.
fn dm_reward_delta(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let zeny_before = probe_zeny(&mut context)?;

    context.flush();
    context.say("@dmreward 2 uncommon")?;
    let announce = wait_for_text(&mut context, "reward announce", "Awarded uncommon Arc 2 loot")?;

    // "[DM] Awarded uncommon Arc 2 loot at reward level 28: 2x Blue Potion
    //  and 3,164 zeny per online member (1 target)."
    let amount: u16 = announce
        .split_once(": ")
        .and_then(|(_, tail)| tail.split_once('x'))
        .and_then(|(amount, _)| amount.trim().parse().ok())
        .ok_or_else(|| format!("could not parse item amount from {announce:?}"))?;
    let zeny_granted: u32 = announce
        .split_once(" and ")
        .and_then(|(_, tail)| tail.split_once(" zeny"))
        .map(|(zeny, _)| zeny.replace(',', ""))
        .and_then(|zeny| zeny.trim().parse().ok())
        .ok_or_else(|| format!("could not parse zeny amount from {announce:?}"))?;

    let item_id = context.wait_for("granted item in inventory", |event| match event {
        NetworkEvent::IventoryItemAdded { item } => Some(item.item_id),
        _ => None,
    })?;
    let expected_zeny = zeny_before + zeny_granted;
    context.wait_for("zeny delta", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::Zeny(value),
        } if *value == expected_zeny => Some(()),
        _ => None,
    })?;

    // Persistence across relogin.
    drop(context);
    std::thread::sleep(Duration::from_millis(700));
    let mut context = TestContext::connect(config)?;
    if !context.inventory.iter().any(|item| item.item_id == item_id) {
        return Err(format!("granted item {} missing from inventory after relogin", item_id.0));
    }
    let zeny_after_relogin = probe_zeny(&mut context)?;
    if zeny_after_relogin != expected_zeny {
        return Err(format!(
            "zeny after relogin is {zeny_after_relogin} but {expected_zeny} was expected"
        ));
    }

    // Cleanup: remove exactly what was granted.
    context.say(&format!("@delitem {} {}", item_id.0, amount))?;
    context.say(&format!("@zeny -{zeny_granted}"))?;
    context.pump(Duration::from_millis(300));
    Ok(())
}

/// `@dmexp` must produce exact GainedExperience events for base and job,
/// and EXP totals must persist accurately across relog.
fn dm_experience(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    // A mid-level first-job character can always gain both experience types
    // without triggering a level-up.
    context.ensure_job(0)?;
    context.ensure_job(1)?;
    context.ensure_base_level(50)?;
    context.say("@jlvl 20")?;
    context.pump(Duration::from_millis(400));

    let account_id = context.account_id;
    let base_before = context.base_experience;
    let job_before = context.job_experience;

    context.flush();
    context.say("@dmexp 1000 500")?;
    context.wait_for("base GainedExperience of 1000", |event| match event {
        NetworkEvent::GainedExperience {
            account_id: event_account,
            amount: 1000,
            experience_type: ExperienceType::BaseExperience,
            ..
        } if event_account.0 == account_id.0 => Some(()),
        _ => None,
    })?;
    context.wait_for("job GainedExperience of 500", |event| match event {
        NetworkEvent::GainedExperience {
            account_id: event_account,
            amount: 500,
            experience_type: ExperienceType::JobExperience,
            ..
        } if event_account.0 == account_id.0 => Some(()),
        _ => None,
    })?;
    wait_for_text(&mut context, "exp grant feedback", "Granted 1000 base / 500 job EXP")?;

    context.say("@save")?;
    context.pump(Duration::from_millis(300));
    drop(context);

    // Reconnect and assert persisted EXP totals
    std::thread::sleep(Duration::from_millis(500));
    let context = TestContext::connect(config)?;
    let base_after = context.base_experience;
    let job_after = context.job_experience;
    if base_after != base_before + 1000 {
        return Err(format!(
            "persisted base EXP mismatch: expected {}, got {}",
            base_before + 1000,
            base_after
        ));
    }
    if job_after != job_before + 500 {
        return Err(format!(
            "persisted job EXP mismatch: expected {}, got {}",
            job_before + 500,
            job_after
        ));
    }
    Ok(())
}

/// QW-079: exercise an authored quest dialogue that calls DM_PartyExp rather
/// than the administrative @dmexp command, and require exact base/job packets
/// plus quest completion.
fn dm_authored_exp_award(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 2000;
    let mut context = TestContext::connect(config)?;
    context.ensure_job(0)?;
    context.ensure_job(1)?;
    context.ensure_base_level(50)?;
    context.say("@jlvl 20")?;
    context.pump(Duration::from_millis(400));
    context.say(&format!("@quest delete {QUEST_ID}"))?;
    context.pump(Duration::from_millis(300));
    context.warp("prontera", 163, 200)?;

    let npc_id = context
        .entities
        .iter()
        .filter(|(id, data)| {
            **id != context.player_id
                && data.position.tile_position().x.abs_diff(163) <= 1
                && data.position.tile_position().y.abs_diff(200) <= 1
        })
        .min_by_key(|(_, data)| {
            let position = data.position.tile_position();
            position.x.abs_diff(163) + position.y.abs_diff(200)
        })
        .map(|(id, _)| *id)
        .ok_or_else(|| "authored EXP award NPC not found near (163, 200)".to_owned())?;

    let base_before = context.base_experience;
    let job_before = context.job_experience;

    context.flush();
    context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;
    context.wait_for("EXP award NPC greeting", |event| match event {
        NetworkEvent::OpenDialog { npc_id: id, text } if *id == npc_id && text.contains("EXP Quest Award Test") => Some(()),
        _ => None,
    })?;
    context.wait_for("EXP award NPC greeting next", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;
    context.wait_for("authored quest added", |event| match event {
        NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == QUEST_ID => Some(()),
        _ => None,
    })?;
    context.wait_for("authored base EXP award", |event| match event {
        NetworkEvent::GainedExperience {
            amount: 1000,
            experience_type: ExperienceType::BaseExperience,
            ..
        } => Some(()),
        _ => None,
    })?;
    context.wait_for("authored job EXP award", |event| match event {
        NetworkEvent::GainedExperience {
            amount: 500,
            experience_type: ExperienceType::JobExperience,
            ..
        } => Some(()),
        _ => None,
    })?;
    context.wait_for("authored quest removal", |event| match event {
        NetworkEvent::QuestRemoved { quest_id } if *quest_id == QUEST_ID => Some(()),
        _ => None,
    })?;
    if context.base_experience != base_before + 1000 || context.job_experience != job_before + 500 {
        return Err(format!(
            "authored EXP totals mismatch: base {} -> {}, job {} -> {}",
            base_before, context.base_experience, job_before, context.job_experience
        ));
    }
    Ok(())
}

fn wait_change_map(context: &mut TestContext, label: &str, map: &str) -> Result<ragnarok_packets::TilePosition, String> {
    let expected = map.to_owned();
    context.wait_for(label, move |event| match event {
        NetworkEvent::ChangeMap { map_name, position } if *map_name == expected => Some(*position),
        _ => None,
    })
}

/// `@dmwarp` moves the whole party; `@dmrecall` pulls it to the DM.
fn dm_warp_recall(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    primary.flush();
    partner.flush();
    primary.say("@dmwarp prontera 156 191")?;
    wait_change_map(&mut primary, "primary ChangeMap (dmwarp)", "prontera")?;
    let position = wait_change_map(&mut partner, "partner ChangeMap (dmwarp)", "prontera")?;
    if position.x.abs_diff(156).max(position.y.abs_diff(191)) > 5 {
        leave_party_both(&mut primary, &mut partner);
        return Err(format!(
            "partner landed at ({}, {}), not near (156, 191)",
            position.x, position.y
        ));
    }

    // Move only the DM elsewhere, then pull the party.
    primary.warp("geffen", 119, 59)?;
    partner.flush();
    primary.flush();
    primary.say("@dmrecall")?;
    wait_for_text(&mut primary, "recall feedback", "Recalled")?;
    let position = wait_change_map(&mut partner, "partner ChangeMap (dmrecall)", "geffen")?;
    if position.x.abs_diff(119).max(position.y.abs_diff(59)) > 5 {
        leave_party_both(&mut primary, &mut partner);
        return Err(format!(
            "recall landed partner at ({}, {}), not near (119, 59)",
            position.x, position.y
        ));
    }

    // Both sessions must remain actionable.
    let (x, y) = (primary.position.x, primary.position.y);
    primary.walk_to(x + 2, y)?;
    let (x, y) = (partner.position.x, partner.position.y);
    partner.walk_to(x + 2, y)?;

    leave_party_both(&mut primary, &mut partner);
    Ok(())
}

/// Count HP-decreasing stat updates arriving within `window`.
fn count_hp_drops(context: &mut TestContext, window: Duration) -> usize {
    let mut last = context.health_points;
    let mut drops = 0;
    for event in context.collect_for(window) {
        if let NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::HealthPoints(value),
        } = event
        {
            if value < last {
                drops += 1;
            }
            last = value;
        }
    }
    drops
}

/// Periodic hazard: party members inside the area take ticks, members outside
/// do not, leaving the area stops the ticks, and `clear` disarms the timer.
fn dm_hazard_periodic(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let result = (|| {
        // Both party members together first, then move only the DM away so
        // the partner is deterministically outside the hazard area (the area
        // check is map + range; map-level separation avoids brittle walks).
        primary.flush();
        partner.flush();
        primary.say("@dmwarp prontera 156 191")?;
        wait_change_map(&mut primary, "primary ChangeMap (hazard setup)", "prontera")?;
        wait_change_map(&mut partner, "partner ChangeMap (hazard setup)", "prontera")?;
        primary.warp("geffen", 119, 59)?;

        primary.say("@heal")?;
        primary.pump(Duration::from_millis(300));
        primary.flush();
        partner.flush();

        // 10% damage, 8 ticks, one every 3 seconds, no status effect. The
        // hazard is centered on the DM's position in geffen.
        primary.say("@dmhazard 3 10 8 3000")?;
        wait_for_text(&mut primary, "hazard placement feedback", "Hazard placed")?;

        // At least two ticks on the DM standing inside...
        let drops = count_hp_drops(&mut primary, Duration::from_secs(8));
        if drops < 2 {
            return Err(format!("expected at least 2 hazard ticks on the inside player, saw {drops}"));
        }
        // ...and none on the party member on another map.
        let partner_drops = count_hp_drops(&mut partner, Duration::from_millis(100));
        if partner_drops > 0 {
            return Err(format!("partner outside the hazard took {partner_drops} tick(s)"));
        }

        // Leaving the area stops further ticks (the timer keeps running).
        primary.warp("geffen", 140, 85)?;
        primary.flush();
        let drops = count_hp_drops(&mut primary, Duration::from_secs(8));
        if drops > 0 {
            return Err(format!("player outside the hazard area still took {drops} tick(s)"));
        }

        say_expect(&mut primary, "@dmhazard clear", "Hazard cleared")?;
        primary.say("@heal")?;
        primary.pump(Duration::from_millis(300));
        Ok(())
    })();

    leave_party_both(&mut primary, &mut partner);
    result
}

/// Instance lifecycle: party requirement, creation, duplicate rejection,
/// teardown, and no-instance-remains.
fn dm_instance_lifecycle(config: &Config) -> Result<(), String> {
    // Solo: instance creation requires a party. Clean up any instance/party
    // a previous interrupted run left behind before asserting.
    {
        let mut solo = TestContext::connect(config)?;
        let _ = solo.say("@dminstance end");
        solo.pump(Duration::from_millis(500));
        let _ = solo.warp("prontera", 155, 180);
        crate::scenarios::social::ensure_no_party(&mut solo);
        say_expect(&mut solo, "@dminstance start prontera 156 191", "must be in a party")?;
    }
    std::thread::sleep(Duration::from_millis(700));

    let (mut primary, mut partner) = connect_pair(config)?;
    form_party(&mut primary, &mut partner)?;

    let result = (|| {
        // Stand somewhere that is not the instance source map so entering it
        // is an observable map change.
        primary.flush();
        partner.flush();
        primary.say("@dmwarp geffen 119 59")?;
        wait_change_map(&mut primary, "primary ChangeMap (instance setup)", "geffen")?;
        wait_change_map(&mut partner, "partner ChangeMap (instance setup)", "geffen")?;

        primary.flush();
        partner.flush();
        primary.say("@dminstance start prontera 156 191 HeadlessInstance")?;
        // "[DM] Instance up: 000#pronter (id 0). Warped 2 party member(s) in."
        let feedback = wait_for_text(&mut primary, "instance creation feedback", "Instance")?;
        let instance_map = feedback
            .split_once("Instance up: ")
            .and_then(|(_, tail)| tail.split_once(" (id"))
            .map(|(map, _)| map.to_owned())
            .ok_or_else(|| format!("instance creation failed: {feedback:?}"))?;
        // The shared networking crate reports the client-side resource name,
        // which is the part after '#' of the (11-char-truncated) instanced
        // map name — "000#pronter" arrives as "pronter". Resource-name
        // resolution for instanced town maps is a known graphical-client gap
        // recorded in headless_findings.md.
        let client_map = instance_map.rsplit('#').next().unwrap_or(&instance_map).to_owned();
        wait_change_map(&mut primary, "primary warped into instance", &client_map)?;
        wait_change_map(&mut partner, "partner warped into instance", &client_map)?;

        // One live instance per party.
        say_expect(
            &mut primary,
            "@dminstance start prontera 156 191",
            "already has a live instance",
        )?;

        primary.flush();
        partner.flush();
        primary.say("@dminstance end")?;
        wait_for_text(&mut primary, "instance teardown feedback", "destroyed")?;
        // Teardown kicks both members out (to their save points).
        primary.wait_for("primary kicked out of instance", |event| match event {
            NetworkEvent::ChangeMap { .. } => Some(()),
            _ => None,
        })?;
        partner.wait_for("partner kicked out of instance", |event| match event {
            NetworkEvent::ChangeMap { .. } => Some(()),
            _ => None,
        })?;

        // No instance remains on record.
        say_expect(&mut primary, "@dminstance end", "no live instance")?;
        Ok(())
    })();

    leave_party_both(&mut primary, &mut partner);
    result
}

// --- beat table -------------------------------------------------------------

struct BeatMenu {
    npc_id: ragnarok_packets::EntityId,
    choices: Vec<String>,
    /// Every line the menu itself shows. Kept so a beat cannot "speak" by
    /// re-showing one — exactly how a bogus-choice run passed its first beat,
    /// quoting the menu's prompt back as if it were the beat's message.
    prompts: Vec<String>,
}

/// Open `@dmbeat <arc>` and walk the mes/next preamble to the choice list.
fn open_beat_menu(context: &mut TestContext, arc: u8) -> Result<BeatMenu, String> {
    context.flush();
    context.say(&format!("@dmbeat {arc}"))?;
    let (npc_id, prompt) = context.wait_for(&format!("arc {arc} beat menu OpenDialog"), |event| match event {
        NetworkEvent::OpenDialog { npc_id, text } => Some((*npc_id, text.trim().to_owned())),
        _ => None,
    })?;
    context.wait_for(&format!("arc {arc} beat menu AddNextButton"), |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;
    // **Collect every line the menu itself shows, not just the first.** The menu
    // speaks twice — an intro, then the prompt above the choice list — and
    // capturing only the first let a beat "speak" by re-showing the second. A
    // bogus-choice run then reported its first beat as `ok` quoting the menu's
    // own prompt back, which is the vacuous pass this guard exists to stop.
    let mut prompts = vec![prompt];
    let mut choices = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while choices.is_none() && std::time::Instant::now() < deadline {
        for event in context.collect_for(Duration::from_millis(200)) {
            match event {
                NetworkEvent::OpenDialog { npc_id: id, text } if id == npc_id => {
                    let line = text.trim().to_owned();
                    if !line.is_empty() {
                        prompts.push(line);
                    }
                }
                NetworkEvent::AddNextButton { npc_id: id } if id == npc_id => {
                    let _ = context.net.next_dialog(npc_id);
                }
                NetworkEvent::AddChoiceButtons {
                    npc_id: id,
                    choices: found,
                } if id == npc_id => {
                    choices = Some(found);
                }
                _ => {}
            }
        }
    }
    let choices = choices.ok_or_else(|| format!("arc {arc} beat menu choices never arrived"))?;
    Ok(BeatMenu { npc_id, choices, prompts })
}

/// Run one selected "Warp:" beat: assert it changes the map, tolerating the
/// mes/next preamble some warp beats show first, then close any dialog.
fn resolve_warp_beat(context: &mut TestContext, npc_id: ragnarok_packets::EntityId) -> Result<(), String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(6);
    let mut changed = false;
    loop {
        if std::time::Instant::now() > deadline {
            break;
        }
        for event in context.collect_for(Duration::from_millis(250)) {
            match event {
                NetworkEvent::ChangeMap { .. } => changed = true,
                NetworkEvent::AddNextButton { npc_id: id } if id == npc_id => {
                    let _ = context.net.next_dialog(npc_id);
                }
                NetworkEvent::AddCloseButton { npc_id: id } if id == npc_id => {
                    let _ = context.net.close_dialog(npc_id);
                }
                _ => {}
            }
        }
        if changed {
            // Acknowledge the new map so the server processes later commands.
            let _ = context.net.map_loaded();
            context.pump(Duration::from_millis(300));
            return Ok(());
        }
    }
    Err("warp beat did not change maps".to_owned())
}

/// Dynamic beat sweep. For every arc (1-19) the arc's beat menu must open with
/// a stable choice list, and every "Warp:" beat in it must actually change the
/// map. Story/encounter beats are content (they spawn bosses and mutate
/// campaign flags), not protocol surface, so they are catalogued but not
/// executed — see the beat-table note in headless_findings.md. Campaign state
/// is wiped with `@dm reset confirm` at both ends.
fn dm_beat_table(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    say_expect(&mut context, "@dm reset confirm", "Campaign reset complete")?;

    let mut failures = Vec::new();
    let mut arcs_checked = 0;
    let mut warps_run = 0;
    let mut story_beats = 0;

    for arc in 1..=19u8 {
        // Return to a quiet town before each arc so a previous warp beat's
        // mob-heavy map cannot flood the menu round trip.
        context.warp("prontera", 156, 191)?;
        context.say("@heal")?;
        context.pump(Duration::from_millis(200));

        let menu = match open_beat_menu(&mut context, arc) {
            Ok(menu) => menu,
            Err(error) => {
                failures.push(format!("arc {arc}: menu failed to open: {error}"));
                continue;
            }
        };
        // Leave the probing menu without running anything.
        context.net.choose_dialog_option(menu.npc_id, -1).map_err(|_| "disconnected")?;
        context.pump(Duration::from_millis(200));
        arcs_checked += 1;

        for (index, label) in menu.choices.iter().enumerate() {
            let lowered = label.to_ascii_lowercase();
            if lowered == "back" || lowered == "cancel" {
                continue;
            }
            if !label.starts_with("Warp") {
                story_beats += 1;
                continue;
            }

            let result = (|| -> Result<(), String> {
                context.warp("prontera", 156, 191)?;
                let reopened = open_beat_menu(&mut context, arc)?;
                if reopened.choices != menu.choices {
                    return Err("beat menu changed between openings".to_owned());
                }
                context
                    .net
                    .choose_dialog_option(reopened.npc_id, (index + 1) as i8)
                    .map_err(|_| "disconnected")?;
                resolve_warp_beat(&mut context, reopened.npc_id)
            })();

            match result {
                Ok(()) => {
                    warps_run += 1;
                    println!("      arc {arc:>2}  {label}  ok");
                }
                Err(error) => {
                    println!("      arc {arc:>2}  {label}  FAILED: {error}");
                    failures.push(format!("arc {arc} \"{label}\": {error}"));
                }
            }
        }
    }

    context.warp("prontera", 156, 191)?;
    say_expect(&mut context, "@dm reset confirm", "Campaign reset complete")?;
    println!("      beat sweep: {arcs_checked}/19 arc menus, {warps_run} warp beats verified, {story_beats} story beats catalogued");

    if arcs_checked < 19 {
        return Err(format!("only {arcs_checked}/19 arc beat menus opened"));
    }
    if warps_run == 0 {
        return Err("no warp beats were exercised".to_owned());
    }
    if !failures.is_empty() {
        return Err(format!("{} beat(s) failed:\n    {}", failures.len(), failures.join("\n    ")));
    }
    Ok(())
}

/// Execute every campaign **story** beat, which nothing has ever done.
///
/// `dm-beat-table` walks all 19 arc menus and runs the `Warp:` beats, but
/// deliberately only *catalogues* the 103 story/encounter beats — they spawn
/// bosses and mutate campaign flags, so they were treated as content rather
/// than protocol surface. The result is that the suite proves the campaign's
/// **menus** open and has never run a beat. For a fork whose stated purpose is
/// this campaign (CLAUDE.md rule 1), that was the largest untested surface in
/// the tree: 30 files, 10,389 lines, 66 scripted NPCs.
///
/// **What this asserts, and what it deliberately does not.** A story beat is
/// content: what it *should* say and spawn is a design question no test can
/// hold. What a test can hold is that the beat is **reachable and terminates**
/// — the dialog opens, runs to an end, and hands control back. That catches the
/// ways campaign script actually breaks: a renamed label, a missing NPC, a
/// typo'd variable that aborts the script mid-dialog, a beat that hangs waiting
/// on input nobody sends. Those are invisible until someone plays that arc.
///
/// **Every beat is cleaned up on every path, including failure.** These spawn
/// mobs and set flags, and this suite's most expensive bugs have all been one
/// scenario leaving state behind for an unrelated one much later — a 165-second
/// Land Protector field silently killed `AL_PNEUMA` two minutes downstream on
/// 2026-08-09, and it took a 4x timing anomaly to notice.
fn dm_story_beats(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    say_expect(&mut context, "@dm reset confirm", "Campaign reset complete")?;

    let mut failures = Vec::new();
    let mut ran = 0;
    let mut arcs = 0;

    for arc in 1..=19u8 {
        // A quiet town between beats: a mob-heavy map floods the menu round trip,
        // which reads as a menu failure and is not one.
        context.warp("prontera", 156, 191)?;
        context.say("@heal")?;
        context.pump(Duration::from_millis(200));

        let menu = match open_beat_menu(&mut context, arc) {
            Ok(menu) => menu,
            Err(error) => {
                failures.push(format!("arc {arc}: menu failed to open: {error}"));
                continue;
            }
        };
        context.net.choose_dialog_option(menu.npc_id, -1).map_err(|_| "disconnected")?;
        context.pump(Duration::from_millis(200));
        arcs += 1;

        for (index, label) in menu.choices.iter().enumerate() {
            let lowered = label.to_ascii_lowercase();
            if lowered == "back" || lowered == "cancel" || label.starts_with("Warp") {
                continue;
            }

            let result = (|| -> Result<String, String> {
                context.warp("prontera", 156, 191)?;
                context.say("@heal")?;
                let reopened = open_beat_menu(&mut context, arc)?;
                if reopened.choices != menu.choices {
                    return Err("beat menu changed between openings".to_owned());
                }
                context
                    .net
                    .choose_dialog_option(reopened.npc_id, (index + 1) as i8)
                    .map_err(|_| "disconnected")?;
                run_story_beat(&mut context, reopened.npc_id, &reopened.prompts)
            })();

            // Cleanup runs whether the beat passed or failed. A boss left alive
            // follows the character into the next beat and kills it.
            context.kill_all_monsters();
            let _ = context.say("@heal");
            context.pump(Duration::from_millis(200));

            match result {
                Ok(said) => {
                    ran += 1;
                    let short: String = said.chars().take(64).collect();
                    println!("      arc {arc:>2}  {label}  ok — {short:?}");
                }
                Err(error) => {
                    println!("      arc {arc:>2}  {label}  FAILED: {error}");
                    failures.push(format!("arc {arc} \"{label}\": {error}"));
                }
            }
        }
    }

    // Reset before returning on every path, so a half-run arc is never inherited.
    context.warp("prontera", 156, 191)?;
    let _ = say_expect(&mut context, "@dm reset confirm", "Campaign reset complete");
    println!("      story sweep: {arcs}/19 arcs, {ran} story beats executed");

    if arcs < 19 {
        return Err(format!("only {arcs}/19 arc beat menus opened"));
    }
    if ran == 0 {
        return Err("no story beats were executed — the menus opened but every beat was skipped".to_owned());
    }
    if !failures.is_empty() {
        return Err(format!(
            "{} story beat(s) failed:\n    {}",
            failures.len(),
            failures.join("\n    ")
        ));
    }
    Ok(())
}

/// A small golden subset of story beats that must not only terminate but also
/// leave a queryable campaign flag / status side effect.
///
/// Full story coverage stays in `dm-story-beats` (reachability). This scenario
/// is the correctness sample: pick known-stable arc/menu rows, run them, and
/// assert `@dmstatus` reflects the change. Keep the list small — each row is
/// content that must stay true after script edits.
///
/// **Coverage:** Act I (1–5) + early Act II (6–10). Expand only with
/// content-reviewed rows; do not bulk-add all 19 arcs here.
fn dm_golden_beats(config: &Config) -> Result<(), String> {
    const GOLDEN_ARCS: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let mut context = TestContext::connect(config)?;
    say_expect(&mut context, "@dm reset confirm", "Campaign reset complete")?;

    // First non-warp choice per arc is the cheapest smoke of flag-setting story
    // content. If a menu shape changes, refresh this list deliberately.
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for &arc in GOLDEN_ARCS {
        context.warp("prontera", 156, 191)?;
        context.say("@heal")?;
        context.pump(Duration::from_millis(200));
        let menu = match open_beat_menu(&mut context, arc) {
            Ok(menu) => menu,
            Err(error) => {
                failures.push(format!("arc {arc}: menu failed: {error}"));
                continue;
            }
        };
        context.net.choose_dialog_option(menu.npc_id, -1).map_err(|_| "disconnected")?;
        context.pump(Duration::from_millis(200));

        let Some((index, label)) = menu.choices.iter().enumerate().find(|(_, label)| {
            let lowered = label.to_ascii_lowercase();
            lowered != "back" && lowered != "cancel" && !label.starts_with("Warp")
        }) else {
            failures.push(format!("arc {arc}: no story beat in menu"));
            continue;
        };

        let result = (|| -> Result<(), String> {
            context.warp("prontera", 156, 191)?;
            let reopened = open_beat_menu(&mut context, arc)?;
            context
                .net
                .choose_dialog_option(reopened.npc_id, (index + 1) as i8)
                .map_err(|_| "disconnected")?;
            let said = run_story_beat(&mut context, reopened.npc_id, &reopened.prompts)?;
            // Correctness sample: @dmstatus prints Mode on a [DM] line, then
            // arc progress on separate lines like `[Arcs 01-05] A01:1 …` (no
            // [DM] prefix). Wait for this arc's token, not just the Mode line.
            let token = if arc < 10 { format!("A0{arc}:") } else { format!("A{arc}:") };
            context.flush();
            context.say("@dmstatus")?;
            let _mode = wait_for_text(&mut context, "dmstatus Mode line", "[DM]")?;
            let status = wait_for_text(&mut context, &format!("dmstatus arc token {token}"), &token)?;
            let pos = status
                .find(&token)
                .ok_or_else(|| format!("internal: {token} missing from {status:?}"))?;
            let progress = status[pos + token.len()..]
                .chars()
                .next()
                .filter(|c| c.is_ascii_digit())
                .ok_or_else(|| format!("{token} has no progress digit in {status:?}"))?;
            let label_l = label.to_ascii_lowercase();
            // Contract-start beats call DM_InstanceQuestStart on the arc anchor
            // quest — progress must leave 0.
            if (label_l.contains("starts contracts") || label_l.contains("start contracts")) && progress == '0' {
                return Err(format!("arc {arc} start-contracts left {token}0 (expected in-progress)"));
            }
            context.flush();
            let _ = context.say("@dmflag");
            context.pump(Duration::from_millis(300));
            println!(
                "      golden arc {arc} {label:?} ok — {token}{progress} snip {:?}",
                said.chars().take(64).collect::<String>()
            );
            Ok(())
        })();

        context.kill_all_monsters();
        let _ = context.say("@heal");
        let _ = say_expect(&mut context, "@dm reset confirm", "Campaign reset complete");
        context.pump(Duration::from_millis(200));

        match result {
            Ok(()) => checked += 1,
            Err(error) => failures.push(format!("arc {arc} \"{label}\": {error}")),
        }
    }

    if !failures.is_empty() {
        return Err(format!(
            "{checked}/{} golden arcs ok; {} failed:\n    {}",
            GOLDEN_ARCS.len(),
            failures.len(),
            failures.join("\n    ")
        ));
    }
    if checked < GOLDEN_ARCS.len() {
        return Err(format!("only {checked}/{} golden arcs checked", GOLDEN_ARCS.len()));
    }
    Ok(())
}

/// Drive one story beat to its end.
///
/// A beat is a script conversation: `mes` pages behind Next buttons, sometimes
/// a menu, ending in a Close. It may also warp, spawn, and set flags along the
/// way. Success is **reaching an end** — the terminating close, or the dialog
/// falling silent after the script has run.
///
/// The failure this is really looking for is a beat that **stalls**: a script
/// that aborts mid-dialog leaves the client holding a dialog with no button,
/// and the next beat then opens its menu into a session that is still busy.
/// That presents as an unrelated later failure, which is the hardest kind to
/// trace.
fn run_story_beat(context: &mut TestContext, menu_npc: ragnarok_packets::EntityId, menu_prompts: &[String]) -> Result<String, String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut sawanything = false;
    // **The beat must say something of its own.** "Events happened and then
    // stopped" is too weak a signal: a NEGATIVE_TEST run that picked a
    // non-existent option still reported the first beat as `ok`, because
    // residual menu traffic satisfied it. Every beat in `dm_beats.txt` ends its
    // case with `mes(...)`, so that text arriving is the evidence the script
    // reached its end — and printing it proves the beat ran rather than
    // asserting it did.
    let mut spoke: Option<String> = None;
    let mut closed = false;
    // **A beat ends by going quiet, not by closing.** The scripts end a case
    // with `mes(...)` and `break` and no `close` — so Hercules finishes the
    // script and the client is left holding text with no button. Waiting for a
    // terminator reports all 103 beats as hangs, which is what the first two
    // versions of this driver did: once for answering the wrong NPC, and once
    // for expecting a close that the campaign never sends.
    let mut quiet_polls = 0;
    const QUIET_POLLS_TO_FINISH: u32 = 8; // ~2s of silence at 250ms per poll
    // **Answer whichever NPC is speaking, not the one that opened the menu.**
    // A beat routinely hands the conversation to another actor — a spawned NPC,
    // a set-piece script — and the first version of this driver only pressed
    // buttons whose `npc_id` matched the *menu*. Every beat then ran, produced
    // dialog nobody answered, and timed out: 103 beats reported as hangs when
    // the harness was the thing not responding.
    let mut speaking = menu_npc;

    while std::time::Instant::now() < deadline && !closed {
        let events = context.collect_for(Duration::from_millis(250));
        if events.is_empty() {
            if sawanything {
                quiet_polls += 1;
                if quiet_polls >= QUIET_POLLS_TO_FINISH {
                    break;
                }
            }
            continue;
        }
        quiet_polls = 0;
        for event in events {
            match event {
                NetworkEvent::AddNextButton { npc_id } => {
                    sawanything = true;
                    speaking = npc_id;
                    let _ = context.net.next_dialog(npc_id);
                }
                NetworkEvent::AddCloseButton { npc_id } => {
                    sawanything = true;
                    speaking = npc_id;
                    let _ = context.net.close_dialog(npc_id);
                    closed = true;
                }
                // A nested menu: take the first option so the beat can finish.
                // Its content is not what this asserts — reachability is.
                NetworkEvent::AddChoiceButtons { npc_id, .. } => {
                    sawanything = true;
                    speaking = npc_id;
                    let _ = context.net.choose_dialog_option(npc_id, 1);
                }
                NetworkEvent::OpenDialog { npc_id, text } => {
                    sawanything = true;
                    speaking = npc_id;
                    // Anything but the menu's own prompt: a beat re-showing the
                    // menu is not the beat talking.
                    let line = text.trim();
                    if !line.is_empty() && !menu_prompts.iter().any(|prompt| prompt == line) {
                        spoke = Some(line.to_owned());
                    }
                }
                NetworkEvent::DisplayEmotion { .. } => sawanything = true,
                NetworkEvent::ChangeMap { .. } => {
                    sawanything = true;
                    let _ = context.net.map_loaded();
                }
                _ => {}
            }
        }
    }

    if !sawanything {
        return Err(
            "the beat produced nothing at all — the menu entry is unreachable or the script aborted before its first line".to_owned(),
        );
    }
    let Some(said) = spoke else {
        let _ = context.net.close_dialog(speaking);
        return Err(
            "the beat produced traffic but never spoke a line — every beat ends its case with `mes(...)`, so without a line of its own \
             (the menu prompt does not count) the script did not run"
                .to_owned(),
        );
    };
    if closed || quiet_polls >= QUIET_POLLS_TO_FINISH {
        // Close whatever is still on screen so the next beat opens its menu
        // into a clean session rather than inheriting a live dialog.
        let _ = context.net.close_dialog(speaking);
        context.pump(Duration::from_millis(200));
        return Ok(said);
    }
    // Ran, but never terminated. Leave the dialog closed so the next beat starts
    // from a clean session rather than inheriting a stuck one.
    let _ = context.net.close_dialog(speaking);
    context.pump(Duration::from_millis(300));
    Err(
        "the beat started but never reached an end within 20s — a script that aborts mid-dialog leaves the session stuck, and the damage \
         surfaces on a later, unrelated beat"
            .to_owned(),
    )
}
