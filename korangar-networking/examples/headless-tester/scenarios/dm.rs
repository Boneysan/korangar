//! Phase 9 — Seal Cascade DM command suite.
//!
//! Covers the whole `@dm*` console surface: command contracts, dice rolls,
//! flags, quests, rewards, experience, party warp/recall, periodic hazards,
//! instanced dungeons, and a dynamic sweep of every configured beat menu.
//! Party scenarios reuse the Phase 8 dual-client machinery from `social.rs`.

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::ExperienceType;

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;
use crate::scenarios::social::{connect_pair, create_party, ensure_no_party, form_party, leave_party_both};

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
        Scenario::new("quest-log-multi", 9, quest_log_multi),
        Scenario::new("gm-metrics", 9, gm_metrics),
        // These party scenarios run BEFORE the two whose end-of-run SQL audits
        // read the journal of one specific party (offline-replay, alternate-
        // character): party ids are reused, so a later party scenario would
        // append to that journal and move its tail past the audited cursors.
        Scenario::new("dm-dmj-echo", 9, dm_dmj_echo),
        Scenario::new("dm-flag-channel", 9, dm_flag_channel),
        Scenario::new("dm-party-offline-transitions", 9, dm_party_offline_transitions),
        Scenario::new("dm-party-reward-isolation", 9, dm_party_reward_isolation),
        Scenario::new("dm-party-recreation-isolation", 9, dm_party_recreation_isolation),
        Scenario::new("dm-party-offline-replay", 9, dm_party_offline_replay),
        Scenario::new("dm-reset-keeps-owed-rewards", 9, dm_reset_keeps_owed_rewards),
        Scenario::new("dm-party-alternate-character", 9, dm_party_alternate_character),
        Scenario::new("dm-reward-delta", 9, dm_reward_delta),
        Scenario::new("dm-experience", 9, dm_experience),
        Scenario::new("dm-warp-recall", 9, dm_warp_recall),
        Scenario::new("dm-hazard-periodic", 9, dm_hazard_periodic),
        Scenario::new("dm-instance-lifecycle", 9, dm_instance_lifecycle),
        Scenario::new("dm-beat-table", 9, dm_beat_table),
        Scenario::new("dm-story-beats", 9, dm_story_beats),
        Scenario::new("dm-golden-beats", 9, dm_golden_beats),
    ]
}

pub(super) fn wait_for_text(context: &mut TestContext, label: &str, needle: &str) -> Result<String, String> {
    context.wait_for(label, |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains(needle) => Some(text.clone()),
        _ => None,
    })
}

/// Send a command and require feedback text containing `needle`.
pub(super) fn say_expect(context: &mut TestContext, command: &str, needle: &str) -> Result<String, String> {
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
        .and_then(|(_, total)| total.split_whitespace().next())
        .and_then(|total| total.parse::<i32>().ok())
        .ok_or_else(|| format!("could not parse roll total from {text:?}"))
}

fn dm_roll_hidden(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
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

/// Record party campaign state while one member is offline, then verify the
/// returning character replays both quest and flag transitions and can catch
/// up again without duplicating the quest notification.
/// A reward earned while a member is offline is queued for them
/// (`DM_QueueGrant`). `@dm reset` restarts the story; it must not confiscate
/// what was already earned (owner decision 2026-10-04). The returning member
/// is paid at their next sync even though the run was reset in between.
fn dm_reset_keeps_owed_rewards(config: &Config) -> Result<(), String> {
    const BASE: usize = 5000;

    let (mut primary, partner) = TestContext::connect_pair(config)?;
    let partner_account = partner.account_id.0;
    let mut partner = Some(partner);
    let result: Result<(), String> = (|| {
        // A capped character gains no EXP and so shows none: keep the partner
        // below the cap so the owed reward is visible when it is paid.
        partner.as_mut().ok_or("partner disconnected")?.ensure_base_level(50)?;
        form_party(&mut primary, partner.as_mut().ok_or("partner disconnected")?)?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;

        drop(partner.take().ok_or("partner disconnected")?);
        std::thread::sleep(Duration::from_millis(700));

        // Online: paid now. Offline partner: queued.
        say_expect(
            &mut primary,
            &format!("@dm exp {BASE} 2000"),
            "Granted 5000 base / 2000 job EXP to 1 party member",
        )?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;

        partner = Some(TestContext::connect_partner(config)?);
        let returning = partner.as_mut().ok_or("partner reconnect failed")?;
        returning
            .wait_for("the EXP earned before the reset", |event| match event {
                NetworkEvent::GainedExperience {
                    account_id,
                    amount,
                    experience_type: ExperienceType::BaseExperience,
                    ..
                } if account_id.0 == partner_account && *amount as usize == BASE => Some(()),
                _ => None,
            })
            .map_err(|error| format!("@dm reset confiscated a reward owed to an offline member: {error}"))
    })();

    if let Some(partner) = partner.as_mut() {
        let _ = primary.say("@dm mode off");
        primary.pump(Duration::from_millis(150));
        leave_party_both(&mut primary, partner);
    } else {
        let _ = primary.say("@dm reset confirm");
        primary.pump(Duration::from_millis(250));
    }
    result
}

fn dm_party_offline_replay(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 20001;
    const FLAG_NAME: &str = "dm_replay_probe";

    let (mut primary, partner) = TestContext::connect_pair(config)?;
    let mut partner = Some(partner);
    let result: Result<(), String> = (|| {
        form_party(&mut primary, partner.as_mut().ok_or("partner disconnected")?)?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;

        drop(partner.take().ok_or("partner disconnected")?);
        std::thread::sleep(Duration::from_millis(700));

        primary.flush();
        primary.say(&format!("@dmquest start {QUEST_ID}"))?;
        primary.wait_for("primary starts campaign quest before offline replay", |event| match event {
            NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        say_expect(&mut primary, &format!("@dmflag set {FLAG_NAME} 17"), "set to 17")?;

        partner = Some(TestContext::connect_partner(config)?);
        let returning = partner.as_mut().ok_or("partner reconnect failed")?;
        returning.wait_for("offline party quest replay", |event| match event {
            NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_ID => Some(()),
            _ => None,
        })?;
        say_expect(returning, &format!("@dmflag get {FLAG_NAME}"), &format!("{FLAG_NAME} = 17"))?;

        say_expect(returning, "@dm catchup", "Catch-up ran")?;
        if returning
            .collect_for(Duration::from_millis(350))
            .iter()
            .any(|event| matches!(event, NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_ID))
        {
            return Err("repeated party catch-up replayed the same active quest notification".to_owned());
        }
        say_expect(returning, &format!("@dmflag get {FLAG_NAME}"), &format!("{FLAG_NAME} = 17"))?;
        Ok(())
    })();

    if let Some(partner) = partner.as_mut() {
        // Keep the journal/cursors intact until the integration runner audits
        // them. Clear the character state through real party transitions.
        let _ = primary.say(&format!("@dmquest erase {QUEST_ID}"));
        primary.pump(Duration::from_millis(150));
        partner.pump(Duration::from_millis(150));
        let _ = primary.say(&format!("@dmflag clear {FLAG_NAME}"));
        primary.pump(Duration::from_millis(150));
        partner.pump(Duration::from_millis(150));
        let _ = primary.say("@dm mode off");
        primary.pump(Duration::from_millis(150));
        leave_party_both(&mut primary, partner);
    } else {
        let _ = primary.say("@dm reset confirm");
        primary.pump(Duration::from_millis(250));
    }
    result
}

/// The server reports campaign state to the client as `[DMJ]{json}` lines sent
/// with `dispbottom` (`dm_dmj.txt`, `dm_checkpoint.txt`). They arrive as
/// ordinary chat text, so the *client* is what must intercept them; this pins
/// the server half: a reconcile preview by a party member really produces
/// typed, versioned JSON, and nothing else about the line is free-form.
fn dm_dmj_echo(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    let result: Result<(), String> = (|| {
        ensure_no_party(&mut primary);
        ensure_no_party(&mut partner);
        form_party(&mut primary, &mut partner)?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;

        primary.flush();
        partner.flush();
        primary.say("@dm reconcile preview")?;
        let mut lines: Vec<String> = Vec::new();
        for context in [&mut primary, &mut partner] {
            for event in context.collect_for(Duration::from_millis(900)) {
                if let NetworkEvent::ChatMessage { text, color } = event
                    && text.contains("[DMJ]")
                {
                    // The client intercepts only server-coloured lines that START
                    // with the prefix (as the discovery channel does). A line that
                    // arrives differently would show up in chat as raw JSON.
                    if !matches!(color, korangar_networking::MessageColor::Server) {
                        return Err(format!("[DMJ] line arrived with a non-server colour: {text:?}"));
                    }
                    let Some(json) = text.strip_prefix("[DMJ]") else {
                        return Err(format!("[DMJ] line does not start with the prefix: {text:?}"));
                    };
                    lines.push(json.to_owned());
                }
            }
        }
        if lines.is_empty() {
            return Err("no [DMJ] echo reached either party member after `@dm reconcile preview`".to_owned());
        }
        for json in &lines {
            let object: serde_json::Value =
                serde_json::from_str(json.trim()).map_err(|error| format!("[DMJ] line is not JSON ({error}): {json:?}"))?;
            if object.get("t").and_then(|t| t.as_str()).is_none() || object.get("v").and_then(|v| v.as_u64()) != Some(1) {
                return Err(format!("[DMJ] line lacks a string \"t\" and \"v\": 1: {json:?}"));
            }
        }
        Ok(())
    })();

    let _ = primary.say("@dm mode off");
    primary.pump(Duration::from_millis(150));
    leave_party_both(&mut primary, &mut partner);
    result
}

/// The change `@metrics baseline diff` reports for `key` (`area.name`), as the
/// signed number on that line.
fn metrics_delta(context: &mut TestContext, label: &str, key: &str) -> Result<i64, String> {
    context.flush();
    context.say(&format!("@metrics baseline diff {label}"))?;
    // Collect the whole reply: a missing line is only explicable by what did
    // arrive.
    let replies: Vec<String> = context
        .collect_for(Duration::from_millis(1500))
        .into_iter()
        .filter_map(|event| match event {
            NetworkEvent::ChatMessage { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    let needle = format!("{key} ");
    let line = replies
        .iter()
        .find(|text| text.contains(&needle))
        .ok_or_else(|| format!("no `{needle}` line in the diff reply: {replies:?}"))?;
    let after = line
        .split(&needle)
        .nth(1)
        .ok_or_else(|| format!("no value after {key} in {line:?}"))?;
    after
        .trim()
        .trim_start_matches('+')
        .parse()
        .map_err(|error| format!("could not read the change for {key} from {line:?}: {error}"))
}

/// F36 GM metrics: collection is opt-in and default off; once on, events are
/// counted as totals; a baseline diff shows the change; `off` stops counting;
/// the live reports answer; a bad baseline label is refused.
fn gm_metrics(config: &Config) -> Result<(), String> {
    const LABEL: &str = "headless_probe";

    let mut context = TestContext::connect(config)?;
    let result: Result<(), String> = (|| {
        say_expect(&mut context, "@metrics off", "Collection OFF")?;
        say_expect(&mut context, "@metrics status", "Collection is OFF")?;

        say_expect(&mut context, "@metrics on", "Collection ON")?;
        say_expect(&mut context, "@metrics status", "Collection is ON")?;
        say_expect(&mut context, &format!("@metrics baseline save {LABEL}"), "saved")?;

        // One death while collection is on. (`@alive` so later steps are not dead.)
        context.say("@die")?;
        std::thread::sleep(Duration::from_millis(900));
        context.say("@alive")?;
        std::thread::sleep(Duration::from_millis(500));
        let deaths = metrics_delta(&mut context, LABEL, "combat.player_deaths")?;
        if deaths < 1 {
            return Err(format!("a death with collection on counted as {deaths}"));
        }
        say_expect(&mut context, "@metrics report", "combat.player_deaths = ")?;

        // Two fresh logins while collection is on.
        for _ in 0..2 {
            std::thread::sleep(Duration::from_millis(700));
            context = TestContext::connect(config)?;
        }
        let logins = metrics_delta(&mut context, LABEL, "session.logins")?;
        if logins < 2 {
            return Err(format!("two logins with collection on counted as {logins}"));
        }

        // Off: another death and another login must not change either count.
        say_expect(&mut context, "@metrics off", "Collection OFF")?;
        context.say("@die")?;
        std::thread::sleep(Duration::from_millis(900));
        context.say("@alive")?;
        std::thread::sleep(Duration::from_millis(500));
        std::thread::sleep(Duration::from_millis(700));
        context = TestContext::connect(config)?;
        let deaths_after = metrics_delta(&mut context, LABEL, "combat.player_deaths")?;
        let logins_after = metrics_delta(&mut context, LABEL, "session.logins")?;
        if (deaths_after, logins_after) != (deaths, logins) {
            return Err(format!(
                "events were counted while collection was off: deaths {deaths} -> {deaths_after}, logins {logins} -> {logins_after}"
            ));
        }

        // Economy gauges ride along with the baseline.
        let economy = wait_for_text_after(&mut context, &format!("@metrics baseline diff {LABEL}"), "economy.zeny_total ")?;
        if !economy.contains("->") {
            return Err(format!("economy gauge has no before/after: {economy:?}"));
        }

        // The live reports.
        say_expect(&mut context, "@metrics spawns prt_fild08", "monster(s)")?;
        say_expect(&mut context, "@metrics party", "player(s) online")?;
        say_expect(&mut context, "@metrics economy", "character(s) hold")?;

        // Quest divergence is a read-only wrapper: it either says no session is active
        // or reports for the active party. Both are correct; silence is not.
        context.flush();
        context.say("@metrics quests")?;
        let quest_reply: Vec<String> = context
            .collect_for(Duration::from_millis(1200))
            .into_iter()
            .filter_map(|event| match event {
                NetworkEvent::ChatMessage { text, .. } if text.contains("[Metrics]") => Some(text),
                _ => None,
            })
            .collect();
        if !quest_reply
            .iter()
            .any(|text| text.contains("No campaign session is active") || text.contains("Campaign state divergence"))
        {
            return Err(format!("@metrics quests gave no usable reply: {quest_reply:?}"));
        }

        // A label that is not a plain name is refused, and an unknown baseline is
        // reported.
        say_expect(&mut context, "@metrics baseline save Bad Label!", "A label is")?;
        say_expect(&mut context, "@metrics baseline diff no_such_baseline", "No baseline named")?;
        Ok(())
    })();

    let _ = context.say("@metrics off");
    context.pump(Duration::from_millis(150));
    result
}

fn wait_for_text_after(context: &mut TestContext, command: &str, needle: &str) -> Result<String, String> {
    context.flush();
    context.say(command)?;
    wait_for_text(context, &format!("`{needle}` after {command}"), needle)
}

/// The `[DMJ]` lines among some events, parsed. Anything that is not a
/// server-coloured line starting with the prefix is not ours.
fn dmj_lines(events: Vec<NetworkEvent>) -> Result<Vec<serde_json::Value>, String> {
    let mut lines = Vec::new();
    for event in events {
        let NetworkEvent::ChatMessage { text, color } = event else {
            continue;
        };
        let Some(json) = text.strip_prefix("[DMJ]") else {
            continue;
        };
        if !matches!(color, korangar_networking::MessageColor::Server) {
            return Err(format!("[DMJ] line with a non-server colour: {text:?}"));
        }
        lines.push(serde_json::from_str(json.trim()).map_err(|error| format!("[DMJ] line is not JSON ({error}): {text:?}"))?);
    }
    Ok(lines)
}

/// Every complete flag snapshot among `lines`, as `(seq, flag name -> value)`.
/// Per sequence number: the expected part count, and the parts seen so far.
type SnapshotParts = std::collections::BTreeMap<u64, (u64, std::collections::BTreeMap<u64, Vec<(String, i64)>>)>;

/// A snapshot counts only when all of its `of` parts are present.
fn flag_snapshots(lines: &[serde_json::Value]) -> Vec<(u64, std::collections::BTreeMap<String, i64>)> {
    let mut by_seq: SnapshotParts = Default::default();
    for line in lines.iter().filter(|line| line.get("t").and_then(|t| t.as_str()) == Some("flags")) {
        let (Some(seq), Some(part), Some(of)) = (
            line.get("seq").and_then(|v| v.as_u64()),
            line.get("part").and_then(|v| v.as_u64()),
            line.get("of").and_then(|v| v.as_u64()),
        ) else {
            continue;
        };
        let flags = line
            .get("flags")
            .and_then(|f| f.as_object())
            .map(|object| object.iter().filter_map(|(k, v)| Some((k.clone(), v.as_i64()?))).collect())
            .unwrap_or_default();
        by_seq.entry(seq).or_insert((of, Default::default())).1.insert(part, flags);
    }
    by_seq
        .into_iter()
        .filter(|(_, (of, parts))| parts.len() as u64 == *of)
        .map(|(seq, (_, parts))| (seq, parts.into_values().flatten().collect()))
        .collect()
}

/// The server half of the flag channel (docs/specs/dm-flag-channel.md, C2): an
/// allowlisted flag change is reported to the member who changed it; a flag off
/// the allowlist is never reported; the other member gets a complete snapshot
/// of exactly the allowlisted flags when the party publishes, and again on
/// login, carrying the value that was replayed to them.
fn dm_flag_channel(config: &Config) -> Result<(), String> {
    // Must match `DM_ClientFlagList` in dm_client_flags.txt.
    const ALLOWED: [&str; 6] = [
        "dm_arc01_started",
        "dm_arc01_child_found",
        "dm_arc01_clue_mask",
        "dm_arc01_chamber_drained",
        "dm_arc01_binding_applied",
        "dm_arc01_holt_approach",
    ];
    const CLUE_FLAG: &str = "dm_arc01_clue_mask";
    const SECRET_FLAG: &str = "dm_flagchan_private";

    let (mut primary, partner) = TestContext::connect_pair(config)?;
    let mut partner = Some(partner);
    let result: Result<(), String> = (|| {
        form_party(&mut primary, partner.as_mut().ok_or("partner disconnected")?)?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;

        // The delta: an allowlisted flag is reported to the member who set it.
        primary.flush();
        primary.say(&format!("@dmflag set {CLUE_FLAG} 5"))?;
        let delta = dmj_lines(primary.collect_for(Duration::from_millis(900)))?;
        let reported = delta.iter().any(|line| {
            line.get("t").and_then(|t| t.as_str()) == Some("flag")
                && line.get("name").and_then(|n| n.as_str()) == Some(CLUE_FLAG)
                && line.get("value").and_then(|v| v.as_i64()) == Some(5)
        });
        if !reported {
            return Err(format!(
                "no flag delta for {CLUE_FLAG} = 5 reached the member who set it: {delta:?}"
            ));
        }

        // A flag off the allowlist is never reported.
        primary.flush();
        say_expect(&mut primary, &format!("@dmflag set {SECRET_FLAG} 7"), "set to 7")?;
        let leaked = dmj_lines(primary.collect_for(Duration::from_millis(700)))?
            .into_iter()
            .any(|line| line.to_string().contains(SECRET_FLAG));
        if leaked {
            return Err(format!("{SECRET_FLAG} is not on the allowlist but was reported to the client"));
        }

        // The party publishes: the other member gets a complete, exact snapshot.
        let other = partner.as_mut().ok_or("partner disconnected")?;
        other.flush();
        say_expect(&mut primary, "@dm catchup", "Catch-up ran")?;
        let lines = dmj_lines(other.collect_for(Duration::from_millis(1500)))?;
        check_snapshot("after the party published", &lines, &ALLOWED, CLUE_FLAG, 5)?;

        // Login: the same snapshot, from the quest-log restore hook.
        drop(partner.take().ok_or("partner disconnected")?);
        std::thread::sleep(Duration::from_millis(700));
        let mut returning = TestContext::connect_partner(config)?;
        let lines = dmj_lines(returning.collect_for(Duration::from_millis(2500)))?;
        check_snapshot("after logging in", &lines, &ALLOWED, CLUE_FLAG, 5)?;
        partner = Some(returning);
        Ok(())
    })();

    let _ = primary.say(&format!("@dmflag clear {CLUE_FLAG}"));
    primary.pump(Duration::from_millis(150));
    let _ = primary.say(&format!("@dmflag clear {SECRET_FLAG}"));
    primary.pump(Duration::from_millis(150));
    let _ = primary.say("@dm mode off");
    primary.pump(Duration::from_millis(150));
    if let Some(partner) = partner.as_mut() {
        leave_party_both(&mut primary, partner);
    } else {
        let _ = primary.say("@dm reset confirm");
        primary.pump(Duration::from_millis(250));
    }
    result
}

fn check_snapshot(when: &str, lines: &[serde_json::Value], allowed: &[&str], flag: &str, value: i64) -> Result<(), String> {
    let snapshots = flag_snapshots(lines);
    let Some((_, table)) = snapshots.last() else {
        return Err(format!("no complete flag snapshot arrived {when}: {lines:?}"));
    };
    let names: Vec<&str> = table.keys().map(String::as_str).collect();
    let mut expected: Vec<&str> = allowed.to_vec();
    expected.sort_unstable();
    if names != expected {
        return Err(format!(
            "snapshot {when} lists {names:?}, expected exactly the allowlist {expected:?}"
        ));
    }
    if table.get(flag).copied() != Some(value) {
        return Err(format!("snapshot {when} has {flag} = {:?}, expected {value}", table.get(flag)));
    }
    Ok(())
}

/// S10 acceptance 2, the multi-transition half: while one enrolled member is
/// offline the party starts two quests, completes one, sets two flags and
/// clears one. On return the member must land in exactly the final state, and a
/// forced second catch-up must change nothing and re-announce nothing.
fn dm_party_offline_transitions(config: &Config) -> Result<(), String> {
    const QUEST_DONE: u32 = 20001; // started, then completed
    const QUEST_OPEN: u32 = 20002; // started, left active
    const FLAG_KEPT: &str = "dm_trans_kept";
    const FLAG_CLEARED: &str = "dm_trans_cleared";

    let (mut primary, partner) = TestContext::connect_pair(config)?;
    let mut partner = Some(partner);
    let result: Result<(), String> = (|| {
        form_party(&mut primary, partner.as_mut().ok_or("partner disconnected")?)?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;

        drop(partner.take().ok_or("partner disconnected")?);
        std::thread::sleep(Duration::from_millis(700));

        say_expect(&mut primary, &format!("@dmquest start {QUEST_DONE}"), "started")?;
        say_expect(&mut primary, &format!("@dmquest start {QUEST_OPEN}"), "started")?;
        say_expect(&mut primary, &format!("@dmquest complete {QUEST_DONE}"), "completed")?;
        say_expect(&mut primary, &format!("@dmflag set {FLAG_KEPT} 3"), "set to 3")?;
        say_expect(&mut primary, &format!("@dmflag set {FLAG_CLEARED} 5"), "set to 5")?;
        say_expect(&mut primary, &format!("@dmflag clear {FLAG_CLEARED}"), "cleared")?;

        partner = Some(TestContext::connect_partner(config)?);
        let returning = partner.as_mut().ok_or("partner reconnect failed")?;
        returning.wait_for("the still-active quest replays", |event| match event {
            NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_OPEN => Some(()),
            _ => None,
        })?;
        say_expect(returning, &format!("@dmflag get {FLAG_KEPT}"), &format!("{FLAG_KEPT} = 3"))?;
        say_expect(
            returning,
            &format!("@dmflag get {FLAG_CLEARED}"),
            &format!("{FLAG_CLEARED} = 0"),
        )?;
        // Arc 1's tracker digit 2 means "completed"; arc 1 must read as completed, not
        // merely started.
        say_expect(returning, "@dmstatus", "A01:2")?;

        say_expect(returning, "@dm catchup", "Catch-up ran")?;
        let again = returning.collect_for(Duration::from_millis(400));
        if again.iter().any(|event| matches!(event, NetworkEvent::QuestAdded { .. })) {
            return Err("a repeated catch-up re-announced a quest after several offline transitions".to_owned());
        }
        say_expect(returning, &format!("@dmflag get {FLAG_KEPT}"), &format!("{FLAG_KEPT} = 3"))?;
        say_expect(returning, "@dmstatus", "A01:2")?;
        Ok(())
    })();

    if let Some(partner) = partner.as_mut() {
        for id in [QUEST_DONE, QUEST_OPEN] {
            let _ = primary.say(&format!("@dmquest erase {id}"));
            primary.pump(Duration::from_millis(150));
            partner.pump(Duration::from_millis(150));
        }
        for flag in [FLAG_KEPT, FLAG_CLEARED] {
            let _ = primary.say(&format!("@dmflag clear {flag}"));
            primary.pump(Duration::from_millis(150));
            partner.pump(Duration::from_millis(150));
        }
        let _ = primary.say("@dm mode off");
        primary.pump(Duration::from_millis(150));
        leave_party_both(&mut primary, partner);
    } else {
        let _ = primary.say("@dm reset confirm");
        primary.pump(Duration::from_millis(250));
    }
    result
}

/// S10 reward isolation: a reward granted while a member is offline is queued
/// once for that character (`DM_QueueGrant`) and paid exactly once on return.
/// Quest/flag catch-up replays progress, never rewards, so repeating it - or
/// logging in again - must not pay the member a second time.
fn dm_party_reward_isolation(config: &Config) -> Result<(), String> {
    const FLAG_NAME: &str = "dm_reward_probe";

    let (mut primary, partner) = TestContext::connect_pair(config)?;
    let mut partner = Some(partner);
    let result: Result<(), String> = (|| {
        form_party(&mut primary, partner.as_mut().ok_or("partner disconnected")?)?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;

        let zeny_before = probe_zeny(partner.as_mut().ok_or("partner disconnected")?)?;
        drop(partner.take().ok_or("partner disconnected")?);
        std::thread::sleep(Duration::from_millis(700));

        say_expect(&mut primary, &format!("@dmflag set {FLAG_NAME} 11"), "set to 11")?;
        let announce = say_expect(&mut primary, "@dmreward 2 uncommon", "Awarded uncommon Arc 2 loot")?;
        let zeny_granted: u32 = announce
            .split_once(" and ")
            .and_then(|(_, tail)| tail.split_once(" zeny"))
            .map(|(zeny, _)| zeny.replace(',', ""))
            .and_then(|zeny| zeny.trim().parse().ok())
            .ok_or_else(|| format!("could not parse zeny amount from {announce:?}"))?;
        if zeny_granted == 0 {
            return Err(format!(
                "the reward paid no zeny, so isolation cannot be observed: {announce:?}"
            ));
        }

        // First return: the queued grant is claimed, once.
        partner = Some(TestContext::connect_partner(config)?);
        let returning = partner.as_mut().ok_or("partner reconnect failed")?;
        say_expect(returning, &format!("@dmflag get {FLAG_NAME}"), &format!("{FLAG_NAME} = 11"))?;
        let expected = zeny_before + zeny_granted;
        let after_return = probe_zeny(returning)?;
        if after_return != expected {
            return Err(format!(
                "offline member holds {after_return} zeny after returning; expected {expected} ({zeny_before} + the announced \
                 {zeny_granted})"
            ));
        }

        // Repeating catch-up must not pay again.
        say_expect(returning, "@dm catchup", "Catch-up ran")?;
        returning.collect_for(Duration::from_millis(500));
        let after_catchup = probe_zeny(returning)?;
        if after_catchup != expected {
            return Err(format!(
                "a repeated catch-up changed the member's zeny from {expected} to {after_catchup}"
            ));
        }

        // A fresh login runs the same hooks again; it must not pay again either.
        drop(partner.take().ok_or("partner disconnected")?);
        std::thread::sleep(Duration::from_millis(700));
        partner = Some(TestContext::connect_partner(config)?);
        let relogged = partner.as_mut().ok_or("partner reconnect failed")?;
        let after_relog = probe_zeny(relogged)?;
        if after_relog != expected {
            return Err(format!(
                "logging in again changed the member's zeny from {expected} to {after_relog}"
            ));
        }
        Ok(())
    })();

    if let Some(partner) = partner.as_mut() {
        let _ = primary.say(&format!("@dmflag clear {FLAG_NAME}"));
        primary.pump(Duration::from_millis(150));
        partner.pump(Duration::from_millis(150));
        let _ = primary.say("@dm mode off");
        primary.pump(Duration::from_millis(150));
        leave_party_both(&mut primary, partner);
    } else {
        let _ = primary.say("@dm reset confirm");
        primary.pump(Duration::from_millis(250));
    }
    result
}

/// S10 acceptance 4: re-creating a party must not copy one run's progress onto
/// a character who was never part of that run. Progress journaled under party A
/// (primary alone) must not arrive when the partner later joins a *new* party.
fn dm_party_recreation_isolation(config: &Config) -> Result<(), String> {
    const FLAG_NAME: &str = "dm_recreate_probe";
    const QUEST_ID: u32 = 20003;

    let (mut primary, mut partner) = connect_pair(config)?;
    let result: Result<(), String> = (|| {
        ensure_no_party(&mut primary);
        ensure_no_party(&mut partner);
        // Run 1: the primary alone, in a party of one.
        create_party(&mut primary)?;
        say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;
        say_expect(&mut primary, &format!("@dmflag set {FLAG_NAME} 9"), "set to 9")?;
        say_expect(&mut primary, &format!("@dmquest start {QUEST_ID}"), "started")?;
        say_expect(&mut partner, &format!("@dmflag get {FLAG_NAME}"), &format!("{FLAG_NAME} = 0"))?;

        // End run 1, then make a different party and bring the partner in.
        say_expect(&mut primary, "@dm mode off", "DnD mode disabled")?;
        let _ = primary.net.leave_party();
        primary.pump(Duration::from_millis(400));
        form_party(&mut primary, &mut partner)?;
        say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;

        partner.flush();
        let joined = partner.collect_for(Duration::from_millis(700));
        if joined
            .iter()
            .any(|event| matches!(event, NetworkEvent::QuestAdded { quest_id, .. } if *quest_id == QUEST_ID))
        {
            return Err("a quest from a previous party's run reached a character who joined a new party".to_owned());
        }
        say_expect(&mut partner, &format!("@dmflag get {FLAG_NAME}"), &format!("{FLAG_NAME} = 0"))?;
        // The original owner keeps their own progress through the re-creation.
        say_expect(&mut primary, &format!("@dmflag get {FLAG_NAME}"), &format!("{FLAG_NAME} = 9"))?;
        Ok(())
    })();

    let _ = primary.say(&format!("@dmquest erase {QUEST_ID}"));
    primary.pump(Duration::from_millis(150));
    let _ = primary.say(&format!("@dmflag clear {FLAG_NAME}"));
    primary.pump(Duration::from_millis(150));
    let _ = primary.say("@dm mode off");
    primary.pump(Duration::from_millis(150));
    leave_party_both(&mut primary, &mut partner);
    result
}

/// A second character on the campaign owner's account stays isolated until it
/// explicitly joins the active party, then catches up from the same journal.
fn dm_party_alternate_character(config: &Config) -> Result<(), String> {
    const QUEST_ID: u32 = 20002;
    const FLAG_NAME: &str = "dm_alt_probe";

    let mut primary = TestContext::connect(config)?;
    let mut partner = TestContext::connect_partner(config)?;
    if primary.account_id == partner.account_id {
        return Err("alternate-character fixture requires two distinct accounts".to_owned());
    }
    ensure_no_party(&mut primary);
    ensure_no_party(&mut partner);
    create_party(&mut partner)?;
    primary.flush();
    partner
        .net
        .invite_to_party(&primary.character_name)
        .map_err(|_| "partner disconnected while inviting primary")?;
    let party_id = primary.wait_for("primary accepts partner-led party", |event| match event {
        NetworkEvent::PartyInvite { party_id, .. } => Some(*party_id),
        _ => None,
    })?;
    primary.net.accept_party_invite(party_id).map_err(|_| "primary disconnected")?;
    partner.wait_for("primary joins partner-led party", |event| match event {
        NetworkEvent::PartyMemberAdded { member } if member.player_name == primary.character_name => Some(()),
        _ => None,
    })?;
    say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;
    primary.say(&format!("@dmquest start {QUEST_ID}"))?;
    primary.wait_for("primary starts alternate-character campaign quest", |event| match event {
        NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_ID => Some(()),
        _ => None,
    })?;
    say_expect(&mut primary, &format!("@dmflag set {FLAG_NAME} 23"), "set to 23")?;

    let primary_account = primary.account_id;
    drop(primary);
    let mut alternate = TestContext::connect_as(config, &config.username, &config.password, Some("HeadlessAlt"), None)?;
    if alternate.account_id != primary_account {
        return Err("alternate character did not authenticate to the primary account".to_owned());
    }
    let pre_join_events = alternate.collect_for(Duration::from_millis(300));
    if pre_join_events.iter().any(|event| {
        matches!(event,
            NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_ID
        )
    }) {
        return Err("same-account alternate received a campaign quest before joining the party".to_owned());
    }
    say_expect(&mut alternate, &format!("@dmflag get {FLAG_NAME}"), &format!("{FLAG_NAME} = 0"))?;

    partner
        .net
        .invite_to_party(&alternate.character_name)
        .map_err(|_| "partner disconnected before inviting alternate")?;
    let party_id = alternate.wait_for("alternate receives party invitation", |event| match event {
        NetworkEvent::PartyInvite { party_id, .. } => Some(*party_id),
        _ => None,
    })?;
    alternate.net.accept_party_invite(party_id).map_err(|_| "alternate disconnected")?;
    partner.wait_for("alternate joins active campaign party", |event| match event {
        NetworkEvent::PartyMemberAdded { member } if member.player_name == alternate.character_name => Some(()),
        _ => None,
    })?;
    alternate.wait_for("campaign quest replay after alternate joins", |event| match event {
        NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST_ID => Some(()),
        _ => None,
    })?;
    say_expect(
        &mut alternate,
        &format!("@dmflag get {FLAG_NAME}"),
        &format!("{FLAG_NAME} = 23"),
    )?;

    // Journal cleanup as ordinary character-scoped transitions so the
    // disposable SQL audit can verify each enrolled cursor reaches the tail.
    let _ = alternate.say(&format!("@dmquest erase {QUEST_ID}"));
    alternate.pump(Duration::from_millis(150));
    partner.pump(Duration::from_millis(150));
    let _ = alternate.say(&format!("@dmflag clear {FLAG_NAME}"));
    alternate.pump(Duration::from_millis(150));
    partner.pump(Duration::from_millis(150));
    let _ = alternate.net.leave_party();
    alternate.pump(Duration::from_millis(300));
    drop(alternate);
    let mut primary = TestContext::connect(config)?;
    say_expect(&mut primary, &format!("@dmflag get {FLAG_NAME}"), &format!("{FLAG_NAME} = 0"))?;
    let _ = primary.say("@dm mode off");
    primary.pump(Duration::from_millis(150));
    let _ = primary.net.leave_party();
    primary.pump(Duration::from_millis(300));
    let _ = partner.net.leave_party();
    partner.pump(Duration::from_millis(300));
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

/// `@dmexp` must produce exact GainedExperience events for base and job.
fn dm_experience(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    // A mid-level first-job character can always gain both experience types.
    context.ensure_job(1)?;
    context.ensure_base_level(50)?;

    let account_id = context.account_id;
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
