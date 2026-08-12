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
use crate::scenarios::social::{connect_pair, form_party, leave_party_both};

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

fn wait_for_text(context: &mut TestContext, label: &str, needle: &str) -> Result<String, String> {
    context.wait_for(label, |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains(needle) => Some(text.clone()),
        _ => None,
    })
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
        ("@dmflag bogusaction some_flag", "Unknown flag action"),
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
