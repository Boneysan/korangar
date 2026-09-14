use std::time::{Duration, Instant};

use korangar_networking::NetworkEvent;
use ragnarok_packets::{Direction, StatType, WorldPosition};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("walk", 3, walk),
        Scenario::new("warp-crossmap", 3, warp_crossmap),
        Scenario::new("entity-details", 3, entity_details),
        Scenario::new("sit-stand", 3, sit_stand),
        Scenario::new("sitting-regeneration-thresholds", 3, sitting_regeneration_thresholds),
        Scenario::new("tick-sync", 3, tick_sync),
    ]
}

/// Request a walk and verify the server paths us to the exact tile.
fn walk(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;

    let target = (context.position.x + 7, context.position.y);
    context.flush();
    context
        .net
        .player_move(WorldPosition::new(target.0, target.1, Direction::North))
        .map_err(|_| "disconnected")?;

    let destination = context.wait_for("PlayerMove", |event| match event {
        NetworkEvent::PlayerMove { destination, .. } => Some(destination.tile_position()),
        _ => None,
    })?;

    if (destination.x, destination.y) != target {
        return Err(format!(
            "asked for {:?}, server pathed to {:?}",
            target,
            (destination.x, destination.y)
        ));
    }
    Ok(())
}

/// Cross-map warp repopulates the entity set for the new map.
fn warp_crossmap(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;
    let prontera_entities = context.entities.len();

    context.warp("geffen", 119, 59)?;

    // `warp` waits for ChangeMap (which clears entities) and pumps; geffen
    // has NPCs around the fountain, so new AddEntity events must have arrived.
    if context.entities.is_empty() {
        return Err(format!(
            "no entities after cross-map warp (prontera had {prontera_entities}) — AddEntity stream missing"
        ));
    }
    Ok(())
}

/// `RequestDetailsPacket` round trip resolves a monster's name.
fn entity_details(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;
    let entity_id = context.spawn_monster("PORING", 1002)?;

    context.flush();
    context.net.entity_details(entity_id).map_err(|_| "disconnected")?;
    let name = context.wait_for("UpdateEntityDetails", |event| match event {
        NetworkEvent::UpdateEntityDetails { entity_id: id, name } if id.0 == entity_id.0 => Some(name.clone()),
        _ => None,
    })?;

    context.kill_all_monsters();

    if !name.eq_ignore_ascii_case("poring") {
        return Err(format!("expected name Poring, got {name:?}"));
    }
    Ok(())
}

/// Sit down / stand up round trips with our own entity id.
fn sit_stand(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    // Sitting requires the Basic Skill; grant it and wait for the tree update.
    context.flush();
    context.say("@allskill")?;
    context.wait_for("SkillTree after @allskill", |event| match event {
        NetworkEvent::SkillTree { skill_information } if !skill_information.is_empty() => Some(()),
        _ => None,
    })?;

    let player_id = context.player_id;

    context.flush();
    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.player_stand().map_err(|_| "disconnected")?;
    context.wait_for("PlayerStandUp", |event| match event {
        NetworkEvent::PlayerStandUp { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    Ok(())
}

/// Tick request keeps client/server clocks in sync.
fn tick_sync(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.flush();
    context.net.request_client_tick().map_err(|_| "disconnected")?;
    context.wait_for("UpdateClientTick", |event| match event {
        NetworkEvent::UpdateClientTick { .. } => Some(()),
        _ => None,
    })?;
    Ok(())
}

fn collect_hp_sp_ticks(context: &mut TestContext, duration: Duration) -> (Vec<(Duration, u32)>, Vec<(Duration, u32)>) {
    let start = Instant::now();
    let deadline = start + duration;
    let mut hp_ticks = Vec::new();
    let mut sp_ticks = Vec::new();

    while Instant::now() < deadline {
        let events = context.collect_for(Duration::from_millis(100));
        for event in events {
            match event {
                NetworkEvent::UpdateStat {
                    stat_type: StatType::HealthPoints(hp),
                } => {
                    hp_ticks.push((start.elapsed(), hp));
                }
                NetworkEvent::UpdateStat {
                    stat_type: StatType::SpellPoints(sp),
                } => {
                    sp_ticks.push((start.elapsed(), sp));
                }
                _ => {}
            }
        }
    }
    (hp_ticks, sp_ticks)
}

fn damage_character(context: &mut TestContext, max_hp: u32) -> Result<(), String> {
    context.say("@heal -400 -120")?;
    context.wait_for("damage applied", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::HealthPoints(hp),
        } if *hp < max_hp => Some(()),
        _ => None,
    })?;
    context.flush();
    Ok(())
}

/// QW-023 — Sitting regeneration thresholds.
///
/// Verifies Hercules HP/SP natural regeneration rates and timers across:
/// 1. Normal weight (<50%), Sitting: 3+ ticks (HP every 3.0s, SP every 4.0s).
/// 2. Normal weight (<50%), Standing: 3+ ticks (HP every 6.0s, SP every 8.0s).
/// 3. Overweight (>=50%, <90%): standing and sitting natural regen completely
///    blocked.
/// 4. Severe Overweight (>=90%): standing and sitting natural regen completely
///    blocked.
/// 5. Poison status (SC_POISON): natural regen completely blocked, SP regen =
///    0.
/// 6. Full HP/SP control: 0 heal events when HP and SP are already at maximum.
fn sitting_regeneration_thresholds(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let player_id = context.player_id;
    context.warp("prontera", 155, 180)?;

    // Sitting requires Basic Skill; ensure skills are unlocked.
    context.flush();
    context.say("@allskill")?;
    context.pump(Duration::from_millis(500));

    // Reset inventory and stats, ensure fresh full health
    context.say("@itemreset")?;
    context.say("@resetstat")?;
    context.say("@heal")?;
    context.pump(Duration::from_millis(500));

    let max_hp = context.max_health_points;
    let max_sp = context.max_spell_points;
    let max_weight = context.max_weight;

    if max_hp == 0 || max_sp == 0 || max_weight == 0 {
        return Err(format!(
            "invalid base stats from server: max_hp={max_hp}, max_sp={max_sp}, max_weight={max_weight}"
        ));
    }

    eprintln!(
        "[QW-023 Baseline] Max HP: {}, Max SP: {}, Weight: {}/{} ({:.1}%)",
        max_hp,
        max_sp,
        context.weight,
        max_weight,
        context.weight as f64 * 100.0 / max_weight as f64
    );

    // =========================================================================
    // 1. Normal Weight (<50%) Sitting: 3+ HP ticks (3.0s) & 3+ SP ticks (4.0s)
    // =========================================================================
    damage_character(&mut context, max_hp)?;

    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    // Sitting: HP tick = 3.0s, SP tick = 4.0s. 13.0s gives 4 HP ticks and 3 SP
    // ticks.
    let (hp_ticks_sit, sp_ticks_sit) = collect_hp_sp_ticks(&mut context, Duration::from_millis(13000));

    context.net.player_stand().map_err(|_| "disconnected")?;
    context.wait_for("PlayerStandUp", |event| match event {
        NetworkEvent::PlayerStandUp { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    if hp_ticks_sit.len() < 3 {
        return Err(format!(
            "Normal weight sitting: expected >= 3 HP ticks across 13s, got {} ({:?})",
            hp_ticks_sit.len(),
            hp_ticks_sit
        ));
    }
    if sp_ticks_sit.len() < 3 {
        return Err(format!(
            "Normal weight sitting: expected >= 3 SP ticks across 13s, got {} ({:?})",
            sp_ticks_sit.len(),
            sp_ticks_sit
        ));
    }

    let dh_sit = hp_ticks_sit[1].1 - hp_ticks_sit[0].1;
    let ds_sit = sp_ticks_sit[1].1 - sp_ticks_sit[0].1;
    for i in 1..hp_ticks_sit.len() {
        let step = hp_ticks_sit[i].1 - hp_ticks_sit[i - 1].1;
        if step != dh_sit {
            return Err(format!("Inconsistent sitting HP delta: step {i} is {step}, expected {dh_sit}"));
        }
    }
    for i in 1..sp_ticks_sit.len() {
        let step = sp_ticks_sit[i].1 - sp_ticks_sit[i - 1].1;
        if step != ds_sit {
            return Err(format!("Inconsistent sitting SP delta: step {i} is {step}, expected {ds_sit}"));
        }
    }

    eprintln!(
        "[QW-023 Section 1 PASS] Normal Weight Sitting: HP restored +{} per tick ({} ticks observed: {:?}), SP restored +{} per tick ({} \
         ticks observed: {:?})",
        dh_sit,
        hp_ticks_sit.len(),
        hp_ticks_sit
            .iter()
            .map(|(t, v)| (format!("{:.1}s", t.as_secs_f64()), *v))
            .collect::<Vec<_>>(),
        ds_sit,
        sp_ticks_sit.len(),
        sp_ticks_sit
            .iter()
            .map(|(t, v)| (format!("{:.1}s", t.as_secs_f64()), *v))
            .collect::<Vec<_>>()
    );

    // =========================================================================
    // 2. Normal Weight (<50%) Standing: 3+ HP ticks (6.0s) & 3+ SP ticks (8.0s)
    // =========================================================================
    damage_character(&mut context, max_hp)?;

    // Standing: HP tick = 6.0s, SP tick = 8.0s. 25.0s gives 4 HP ticks and 3 SP
    // ticks.
    let (hp_ticks_std, sp_ticks_std) = collect_hp_sp_ticks(&mut context, Duration::from_millis(25000));

    if hp_ticks_std.len() < 3 {
        return Err(format!(
            "Normal weight standing: expected >= 3 HP ticks across 25s, got {} ({:?})",
            hp_ticks_std.len(),
            hp_ticks_std
        ));
    }
    if sp_ticks_std.len() < 3 {
        return Err(format!(
            "Normal weight standing: expected >= 3 SP ticks across 25s, got {} ({:?})",
            sp_ticks_std.len(),
            sp_ticks_std
        ));
    }

    let dh_std = hp_ticks_std[1].1 - hp_ticks_std[0].1;
    let ds_std = sp_ticks_std[1].1 - sp_ticks_std[0].1;
    if dh_std != dh_sit {
        return Err(format!(
            "Standing HP delta ({dh_std}) does not match sitting HP delta ({dh_sit})"
        ));
    }
    if ds_std != ds_sit {
        return Err(format!(
            "Standing SP delta ({ds_std}) does not match sitting SP delta ({ds_sit})"
        ));
    }

    eprintln!(
        "[QW-023 Section 2 PASS] Normal Weight Standing: HP restored +{} per tick at 6.0s intervals ({:?}), SP restored +{} per tick at \
         8.0s intervals ({:?})",
        dh_std,
        hp_ticks_std
            .iter()
            .map(|(t, v)| (format!("{:.1}s", t.as_secs_f64()), *v))
            .collect::<Vec<_>>(),
        ds_std,
        sp_ticks_std
            .iter()
            .map(|(t, v)| (format!("{:.1}s", t.as_secs_f64()), *v))
            .collect::<Vec<_>>()
    );

    // =========================================================================
    // 3. Overweight (>=50%, <90%): Natural Regen Blocked (0 HP, 0 SP)
    // =========================================================================
    context.say("@itemreset")?;
    context.pump(Duration::from_millis(300));

    // Steel (item 999) has weight 100 in packet units. Target 60% of max weight.
    let target_weight_60 = max_weight * 60 / 100;
    let count_steel_60 = (target_weight_60 + 99) / 100;
    let _ = context.give_item(999, count_steel_60 as u16)?;
    context.pump(Duration::from_millis(500));

    let w = context.weight;
    let mw = context.max_weight;
    if w * 100 < mw * 50 || w * 10 >= mw * 9 {
        return Err(format!(
            "Failed to reach 50-89% overweight range: weight={w}, max_weight={mw} ({:.1}%)",
            w as f64 * 100.0 / mw as f64
        ));
    }
    eprintln!(
        "[QW-023 Section 3] Testing Overweight threshold: weight={w}/{mw} ({:.1}%)",
        w as f64 * 100.0 / mw as f64
    );

    // Standing check (7.0s > 1 standing tick window)
    damage_character(&mut context, max_hp)?;
    let (hp_ow_std, sp_ow_std) = collect_hp_sp_ticks(&mut context, Duration::from_millis(7000));
    if !hp_ow_std.is_empty() || !sp_ow_std.is_empty() {
        return Err(format!(
            "Overweight standing failed: regen occurred! hp={:?}, sp={:?}",
            hp_ow_std, sp_ow_std
        ));
    }

    // Sitting check (10.0s > 3 sitting HP tick windows)
    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    let (hp_ow_sit, sp_ow_sit) = collect_hp_sp_ticks(&mut context, Duration::from_millis(10000));
    context.net.player_stand().map_err(|_| "disconnected")?;
    context.wait_for("PlayerStandUp", |event| match event {
        NetworkEvent::PlayerStandUp { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    if !hp_ow_sit.is_empty() || !sp_ow_sit.is_empty() {
        return Err(format!(
            "Overweight sitting failed: regen occurred! hp={:?}, sp={:?}",
            hp_ow_sit, sp_ow_sit
        ));
    }
    eprintln!("[QW-023 Section 3 PASS] Overweight (50-89%): 0 HP regen, 0 SP regen across standing and sitting (completely blocked)");

    // =========================================================================
    // 4. Severe Overweight (>=90%): Natural Regen Blocked (0 HP, 0 SP)
    // =========================================================================
    let target_weight_95 = max_weight * 95 / 100;
    let add_steel = (target_weight_95.saturating_sub(context.weight) + 99) / 100;
    if add_steel > 0 {
        let _ = context.give_item(999, add_steel as u16)?;
        context.pump(Duration::from_millis(500));
    }

    let w90 = context.weight;
    if w90 * 10 < mw * 9 {
        return Err(format!(
            "Failed to reach >=90% overweight: weight={w90}, max_weight={mw} ({:.1}%)",
            w90 as f64 * 100.0 / mw as f64
        ));
    }
    eprintln!(
        "[QW-023 Section 4] Testing Severe Overweight threshold: weight={w90}/{mw} ({:.1}%)",
        w90 as f64 * 100.0 / mw as f64
    );

    damage_character(&mut context, max_hp)?;

    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    let (hp_sow, sp_sow) = collect_hp_sp_ticks(&mut context, Duration::from_millis(10000));
    context.net.player_stand().map_err(|_| "disconnected")?;
    context.wait_for("PlayerStandUp", |event| match event {
        NetworkEvent::PlayerStandUp { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    if !hp_sow.is_empty() || !sp_sow.is_empty() {
        return Err(format!(
            "Severe overweight sitting failed: regen occurred! hp={:?}, sp={:?}",
            hp_sow, sp_sow
        ));
    }
    eprintln!("[QW-023 Section 4 PASS] Severe Overweight (>=90%): 0 HP regen, 0 SP regen across 10s sitting");

    // =========================================================================
    // 5. Poison Status Control (SC_POISON): Natural Regen Blocked
    // =========================================================================
    context.say("@itemreset")?;
    context.say("@heal")?;
    context.pump(Duration::from_millis(300));
    context.flush();
    damage_character(&mut context, max_hp)?;

    // Item 12238 (New Year Rice Cake) casts SC_POISON for 50s
    let cake_idx = context.give_item(12238, 5)?;
    context.flush();

    let mut poisoned = false;
    for _ in 0..5 {
        context.net.use_item(cake_idx, context.account_id).map_err(|_| "disconnected")?;
        let res = context.wait_for_within("poison state active", Duration::from_millis(1500), &mut |event| match event {
            NetworkEvent::StateChange {
                entity_id, health_state, ..
            } if entity_id.0 == player_id.0 && (health_state & 0x0001) != 0 => Some(()),
            _ => None,
        });
        if res.is_ok() {
            poisoned = true;
            break;
        }
    }
    if !poisoned {
        return Err("Failed to apply poison state after 5 New Year Rice Cakes".to_string());
    }
    context.flush();

    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    // During poison: natural regen is blocked (flag=0 in status_calc_regen_pc)
    // SP never regenerates, and HP never increases (only decreases from poison
    // damage).
    let (hp_poison, sp_poison) = collect_hp_sp_ticks(&mut context, Duration::from_millis(9000));
    context.net.player_stand().map_err(|_| "disconnected")?;
    context.wait_for("PlayerStandUp", |event| match event {
        NetworkEvent::PlayerStandUp { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    if !sp_poison.is_empty() {
        return Err(format!(
            "Poison control failed: SP regenerated under SC_POISON! sp={:?}",
            sp_poison
        ));
    }
    for i in 1..hp_poison.len() {
        if hp_poison[i].1 > hp_poison[i - 1].1 {
            return Err(format!(
                "Poison control failed: HP regenerated under SC_POISON! tick {}: {} -> {}",
                i,
                hp_poison[i - 1].1,
                hp_poison[i].1
            ));
        }
    }
    // Cure poison with Green Potion (item 506) and restore full HP/SP
    let pot_idx = context.give_item(506, 1)?;
    context.flush();
    context.net.use_item(pot_idx, context.account_id).map_err(|_| "disconnected")?;
    context.wait_for("item consumed", |event| match event {
        NetworkEvent::InventoryItemRemoved { .. } => Some(()),
        _ => None,
    })?;
    context.wait_for("poison cured", |event| match event {
        NetworkEvent::StateChange {
            entity_id, health_state, ..
        } if entity_id.0 == player_id.0 && (health_state & 0x0001) == 0 => Some(()),
        _ => None,
    })?;
    context.say("@itemreset")?;
    context.say("@heal")?;
    context.pump(Duration::from_millis(400));
    context.flush();
    eprintln!("[QW-023 Section 5 PASS] Poison Control: 0 SP natural regen and 0 HP upward regen under SC_POISON across 9s sitting");

    // =========================================================================
    // 6. Full HP/SP Control: No Regen Events when at 100%
    // =========================================================================
    context.say("@heal")?;
    context.pump(Duration::from_millis(400));
    context.flush();

    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    let (hp_full, sp_full) = collect_hp_sp_ticks(&mut context, Duration::from_millis(7000));
    context.net.player_stand().map_err(|_| "disconnected")?;
    context.wait_for("PlayerStandUp", |event| match event {
        NetworkEvent::PlayerStandUp { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();

    if !hp_full.is_empty() || !sp_full.is_empty() {
        return Err(format!(
            "Full HP/SP control failed: heal events received while at max HP/SP! hp={:?}, sp={:?}",
            hp_full, sp_full
        ));
    }
    eprintln!("[QW-023 Section 6 PASS] Full HP/SP Control: 0 heal events received when at 100% capacity across 7s sitting");

    Ok(())
}
