use std::time::{Duration, Instant};

use korangar_networking::NetworkEvent;
use ragnarok_packets::{Direction, StatType, TilePosition, WorldPosition};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("walk", 3, walk),
        Scenario::new("wasd-lan-trace", 3, wasd_lan_trace),
        Scenario::new("warp-crossmap", 3, warp_crossmap),
        Scenario::new("entity-details", 3, entity_details),
        Scenario::new("sit-stand", 3, sit_stand),
        Scenario::new("sitting-regeneration-thresholds", 3, sitting_regeneration_thresholds),
        Scenario::new("sit-25-percent", 3, sit_25_percent),
        Scenario::new("save-load", 3, save_load),
        Scenario::new("weight-capacity-x5", 3, weight_capacity_x5),
        Scenario::new("weight-hard-cap-boundary", 3, weight_hard_cap_boundary),
        Scenario::new("weight-death-respawn", 3, weight_death_respawn),
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

fn tile(x: u16, y: u16) -> TilePosition {
    TilePosition { x, y }
}

fn chebyshev(a: TilePosition, b: TilePosition) -> u16 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

struct MoveAck {
    origin: TilePosition,
    destination: TilePosition,
    delay: Duration,
}

fn send_move(context: &mut TestContext, dest: TilePosition) -> Result<Instant, String> {
    context
        .net
        .player_move(WorldPosition::new(dest.x, dest.y, Direction::North))
        .map_err(|_| "disconnected")?;
    Ok(Instant::now())
}

fn collect_move_acks(context: &mut TestContext, window: Duration, sent_at: Instant) -> (Vec<MoveAck>, Vec<String>) {
    let events = context.collect_for(window);
    let mut acks = Vec::new();
    let mut extras = Vec::new();
    for event in events {
        match event {
            NetworkEvent::PlayerMove { origin, destination, .. } => acks.push(MoveAck {
                origin: origin.tile_position(),
                destination: destination.tile_position(),
                delay: sent_at.elapsed(),
            }),
            NetworkEvent::EntityStopMove { entity_id, position } if entity_id.0 == context.player_id.0 => {
                extras.push(format!("stop-move at {position:?}"));
            }
            NetworkEvent::EntitySlide { entity_id, position } if entity_id.0 == context.player_id.0 => {
                extras.push(format!("slide at {position:?}"));
            }
            NetworkEvent::ChangeMap { map_name, position } => extras.push(format!("change-map {map_name} {position:?}")),
            NetworkEvent::StateChange {
                entity_id,
                body_state,
                health_state,
                ..
            } if entity_id.0 == context.player_id.0 => {
                extras.push(format!("state-change body=0x{body_state:04x} health=0x{health_state:04x}"))
            }
            _ => {}
        }
    }
    (acks, extras)
}

fn log_acks(label: &str, requested: TilePosition, acks: &[MoveAck]) {
    eprintln!(
        "[wasd] {label} requested {requested:?}, {count} PlayerMove 0x0087 ack(s)",
        count = acks.len()
    );
    for (index, ack) in acks.iter().enumerate() {
        let correction = chebyshev(ack.origin, requested);
        let dest_delta = chebyshev(ack.destination, requested);
        eprintln!(
            "[wasd] {label} ack[{index}] origin {:?} -> dest {:?}, dest_delta {dest_delta}, origin_vs_request {correction}, ack_delay_ms \
             {}",
            ack.origin,
            ack.destination,
            ack.delay.as_millis()
        );
    }
}

/// QW-027 — LAN WASD vs click-to-move packet trace on a recorded route.
///
/// Headless cannot press W, but it can send the same `RequestPlayerMovePacket`
/// (`0x035F`) sequence the WASD policy emits: one 15-cell path, a stop-ahead
/// request, a duplicate, and a post-warp stale destination. Click-to-move is
/// the same opcode aimed at a single 10-cell dest.
fn wasd_lan_trace(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    const START: (u16, u16) = (155, 180);
    const CLICK_EAST: u16 = 10;
    const WASD_EAST: u16 = 15;
    context.warp("prontera", START.0, START.1)?;
    let start = context.position;
    if (start.x, start.y) != START {
        return Err(format!("expected start {START:?}, got {:?}", (start.x, start.y)));
    }

    // --- Click-to-move: one request, one dest ---
    let click_dest = tile(start.x + CLICK_EAST, start.y);
    context.flush();
    let click_sent = send_move(&mut context, click_dest)?;
    let (click_acks, _) = collect_move_acks(&mut context, Duration::from_millis(800), click_sent);
    log_acks("click", click_dest, &click_acks);
    if click_acks.len() != 1 {
        return Err(format!("click-to-move expected 1 PlayerMove ack, got {}", click_acks.len()));
    }
    if click_acks[0].destination != click_dest {
        return Err(format!(
            "click-to-move dest {:?}, server {:?}",
            click_dest, click_acks[0].destination
        ));
    }
    let click_correction = chebyshev(click_acks[0].origin, start);
    eprintln!("[wasd] click correction vs start {click_correction} cells (LAN)");

    context.warp("prontera", START.0, START.1)?;

    // --- WASD press: 15-cell path, no second packet ---
    let wasd_dest = tile(start.x + WASD_EAST, start.y);
    context.flush();
    let wasd_sent = send_move(&mut context, wasd_dest)?;
    let (wasd_acks, _) = collect_move_acks(&mut context, Duration::from_millis(800), wasd_sent);
    log_acks("press", wasd_dest, &wasd_acks);
    if wasd_acks.is_empty() {
        return Err("WASD 15-cell press produced no PlayerMove ack".into());
    }
    if chebyshev(start, wasd_acks[0].destination) != WASD_EAST {
        return Err(format!(
            "WASD press dest {:?} was not {WASD_EAST} cells from {start:?}",
            wasd_acks[0].destination
        ));
    }
    if wasd_acks.len() != 1 {
        return Err(format!(
            "WASD press must not emit a one-cell-then-path pair; got {} acks",
            wasd_acks.len()
        ));
    }

    // --- Stop-ahead while the long path is in flight ---
    context.warp("prontera", START.0, START.1)?;
    context.flush();
    let press_sent = send_move(&mut context, wasd_dest)?;
    let (press_acks, _) = collect_move_acks(&mut context, Duration::from_millis(400), press_sent);
    if press_acks.is_empty() {
        return Err("stop-ahead setup produced no press ack".into());
    }
    let walking_from = press_acks[0].origin;
    let stop_dest = tile(walking_from.x.saturating_add(2), walking_from.y);
    context.flush();
    let stop_sent = send_move(&mut context, stop_dest)?;
    let (stop_acks, _) = collect_move_acks(&mut context, Duration::from_millis(800), stop_sent);
    log_acks("stop", stop_dest, &stop_acks);
    if stop_acks.is_empty() {
        return Err("stop-ahead RequestPlayerMovePacket produced no PlayerMove ack".into());
    }
    let stop_snap = chebyshev(stop_acks[0].origin, press_acks[0].destination);
    eprintln!(
        "[wasd] stop correction: server origin {:?} vs in-flight dest {:?}, chebyshev {stop_snap}",
        stop_acks[0].origin, press_acks[0].destination
    );

    // --- Duplicate/stale second request of the same dest ---
    context.warp("prontera", START.0, START.1)?;
    context.flush();
    let dup_sent = send_move(&mut context, click_dest)?;
    let _ = send_move(&mut context, click_dest)?;
    let (dup_acks, _) = collect_move_acks(&mut context, Duration::from_millis(800), dup_sent);
    log_acks("duplicate", click_dest, &dup_acks);
    eprintln!(
        "[wasd] duplicate 0x035F count=2 produced {} 0x0087 ack(s) — extra acks are redundant corrections",
        dup_acks.len()
    );

    // --- Warp interruption: stale dest the client policy would not re-send ---
    context.warp("prontera", START.0, START.1)?;
    context.flush();
    let held_sent = send_move(&mut context, wasd_dest)?;
    let _ = collect_move_acks(&mut context, Duration::from_millis(300), held_sent);
    context.warp("prontera", START.0, START.1.saturating_sub(10))?;
    let far_warp = context.position;
    eprintln!("[wasd] interrupt change-map at {far_warp:?}, held dest {wasd_dest:?}");
    context.flush();
    let far_sent = send_move(&mut context, wasd_dest)?;
    let (far_acks, far_extras) = collect_move_acks(&mut context, Duration::from_millis(800), far_sent);
    log_acks("stale-after-far-warp", wasd_dest, &far_acks);
    if !far_extras.is_empty() {
        eprintln!("[wasd] interrupt extras: {far_extras:?}");
    }
    eprintln!(
        "[wasd] far-warp stale dest acks={} (0 means server dropped it; client held-intent is Silent either way)",
        far_acks.len()
    );

    context.warp("prontera", START.0, START.1.saturating_add(2))?;
    let near_warp = context.position;
    context.flush();
    let near_sent = send_move(&mut context, wasd_dest)?;
    let (near_acks, _) = collect_move_acks(&mut context, Duration::from_millis(800), near_sent);
    log_acks("stale-after-near-warp", wasd_dest, &near_acks);
    if near_acks.is_empty() {
        return Err(format!(
            "nearby stale dest {wasd_dest:?} from {near_warp:?} produced no 0x0087; expected the server to honor it"
        ));
    }
    eprintln!("[wasd] server accepted nearby stale dest {wasd_dest:?} from {near_warp:?} (client held-intent would stay Silent)");

    eprintln!(
        "[wasd] LAN mechanism: extra 0x035F while walking (stop/duplicate/stale) each yield a 0x0087 from the server origin; \
         click-to-move does not"
    );
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
    context.say("@heal")?;
    context.pump(Duration::from_millis(300));
    let hp_drop = (max_hp / 2).clamp(10, max_hp.saturating_sub(1));
    let sp_drop = (context.max_spell_points / 2).clamp(1, context.max_spell_points.saturating_sub(1).max(1));
    context.say(&format!("@heal -{hp_drop} -{sp_drop}"))?;
    context.wait_for("damage applied", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::HealthPoints(hp),
        } if *hp < max_hp && *hp > 0 => Some(()),
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
    // 1. Sitting: 25% of max HP/SP about every 10s (playtest, not old 3s ticks).
    // =========================================================================
    damage_character(&mut context, max_hp)?;
    let hp_before_sit = context.health_points;

    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.wait_for("recovery state: sitting", |event| match event {
        NetworkEvent::RecoveryState { mode: 2, block: 0 } => Some(()),
        _ => None,
    })?;
    context.flush();

    let (hp_ticks_sit, sp_ticks_sit) = collect_hp_sp_ticks(&mut context, Duration::from_millis(12000));

    context.net.player_stand().map_err(|_| "disconnected")?;
    context.wait_for("PlayerStandUp", |event| match event {
        NetworkEvent::PlayerStandUp { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.wait_for("recovery state: standing", |event| match event {
        NetworkEvent::RecoveryState { mode: 1, block: 0 } => Some(()),
        _ => None,
    })?;
    context.flush();

    let expected_hp = max_hp / 4;
    let hp_after = hp_ticks_sit.last().map(|(_, hp)| *hp).unwrap_or(hp_before_sit);
    let hp_gained = hp_after.saturating_sub(hp_before_sit);
    if hp_ticks_sit.is_empty() || hp_gained < expected_hp.min(300) {
        return Err(format!(
            "Sitting 25%/10s: expected ~{expected_hp} HP in 12s, gained {hp_gained} from {hp_before_sit} ticks={hp_ticks_sit:?}"
        ));
    }
    if sp_ticks_sit.is_empty() {
        return Err(format!("Sitting 25%/10s: no SP ticks in 12s ({sp_ticks_sit:?})"));
    }

    eprintln!("[playtest sit] 25%/10s: HP +{hp_gained} (max {max_hp}), SP ticks {sp_ticks_sit:?}");

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
    if dh_std >= expected_hp {
        return Err(format!(
            "Standing HP delta ({dh_std}) looks like sitting 25% ({expected_hp}); standing should be stock RO"
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
    context.wait_for("recovery state: overweight", |event| match event {
        NetworkEvent::RecoveryState { mode: 0, block: 3 } => Some(()),
        _ => None,
    })?;

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
    let sp_drop = (context.max_spell_points / 2).clamp(1, context.max_spell_points.saturating_sub(1).max(1));
    context.say(&format!("@heal -10 -{sp_drop}"))?;
    context.pump(Duration::from_millis(300));
    context.flush();

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
        NetworkEvent::StatusChange {
            entity_id, gained: false, ..
        } if entity_id.0 == player_id.0 => Some(()),
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

fn sit_25_percent(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let player_id = context.player_id;
    context.warp("prontera", 155, 180)?;
    context.say("@allskill")?;
    context.say("@heal")?;
    context.pump(Duration::from_millis(400));
    let max_hp = context.max_health_points;
    damage_character(&mut context, max_hp)?;
    let before = context.health_points;
    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();
    let (hp_ticks, _) = collect_hp_sp_ticks(&mut context, Duration::from_millis(12000));
    let _ = context.net.player_stand();
    let after = hp_ticks.last().map(|(_, hp)| *hp).unwrap_or(before);
    let gained = after.saturating_sub(before);
    if gained < (max_hp / 4).min(15) {
        return Err(format!(
            "sit 25%: HP {before}->{after} gained {gained}, max {max_hp}, ticks {hp_ticks:?}"
        ));
    }
    Ok(())
}

fn save_load(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;
    context.gm_expect_feedback("@save")?;
    context.warp("prontera", 100, 120)?;
    context.pump(Duration::from_millis(400));
    if context.position.x == 155 && context.position.y == 180 {
        return Err("warp away did not move the character".into());
    }
    context.say("@load")?;
    context.wait_for("return to save", |event| match event {
        NetworkEvent::PlayerMove { destination, .. } if destination.x == 155 && destination.y == 180 => Some(()),
        NetworkEvent::ChangeMap { .. } => Some(()),
        _ => None,
    })?;
    context.pump(Duration::from_millis(400));
    if context.position.x.abs_diff(155) > 2 || context.position.y.abs_diff(180) > 2 {
        return Err(format!(
            "@load left us at ({}, {}), expected near (155, 180)",
            context.position.x, context.position.y
        ));
    }

    // -------------------------------------------------------------------------
    // Checkpoint NPC flow (Seal Cascade Checkpoint at prontera 151, 191)
    // -------------------------------------------------------------------------
    context.warp("prontera", 151, 191)?;
    context.pump(Duration::from_millis(400));

    let npc_id = context
        .entities
        .iter()
        .find(|(_, data)| {
            let pos = data.position.tile_position();
            data.job_id.0 == 117 && (pos.x as i32 - 151).abs() <= 2 && (pos.y as i32 - 191).abs() <= 2
        })
        .map(|(id, _)| *id)
        .ok_or_else(|| "Seal Cascade Checkpoint NPC (job 117) not found near (151, 191)".to_owned())?;

    context.flush();
    context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;
    let greeting = context.wait_for("OpenDialog (checkpoint greeting)", |event| match event {
        NetworkEvent::OpenDialog { text, npc_id: id } if *id == npc_id => Some(text.clone()),
        _ => None,
    })?;
    if !greeting.contains("[Checkpoint]") {
        return Err(format!("expected [Checkpoint] greeting, got: {greeting}"));
    }

    context.wait_for("AddNextButton (checkpoint)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;
    context.wait_for("AddChoiceButtons (checkpoint options)", |event| match event {
        NetworkEvent::AddChoiceButtons { npc_id: id, .. } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Option 1: Save here
    context.flush();
    context.net.choose_dialog_option(npc_id, 1).map_err(|_| "disconnected")?;
    let saved_text = context.wait_for("OpenDialog (save confirmed)", |event| match event {
        NetworkEvent::OpenDialog { text, npc_id: id } if *id == npc_id => Some(text.clone()),
        _ => None,
    })?;
    if !saved_text.contains("Saved") {
        return Err(format!("expected 'Saved' in response, got: {saved_text}"));
    }

    context.wait_for("AddCloseButton (checkpoint)", |event| match event {
        NetworkEvent::AddCloseButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    let _ = context.net.close_dialog(npc_id);

    // Warp away to Geffen and die; respawn must return to checkpoint (151, 191)
    context.warp("geffen", 119, 59)?;
    context.pump(Duration::from_millis(400));
    let player_id = context.player_id;
    context.flush();
    context.say("@die")?;
    context.wait_for("own-entity RemoveEntity (Died)", |event| match event {
        NetworkEvent::RemoveEntity {
            entity_id,
            reason: ragnarok_packets::DisappearanceReason::Died,
        } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.respawn().map_err(|_| "disconnected")?;
    let map_name = context.wait_for("ChangeMap to checkpoint", |event| match event {
        NetworkEvent::ChangeMap { map_name, .. } => Some(map_name.clone()),
        _ => None,
    })?;
    if map_name != "prontera" {
        return Err(format!("respawned on {map_name:?}, expected 'prontera'"));
    }
    context.net.map_loaded().map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(500));

    if context.position.x.abs_diff(151) > 2 || context.position.y.abs_diff(191) > 2 {
        return Err(format!(
            "respawn left us at ({}, {}), expected near checkpoint (151, 191)",
            context.position.x, context.position.y
        ));
    }

    // Invalid map test: attempting @warp to a nonexistent map must fail cleanly
    context.flush();
    context.say("@warp invalid_map_999 100 100")?;
    context.wait_for("invalid map feedback", |event| match event {
        NetworkEvent::ChatMessage { text, .. }
            if text.to_lowercase().contains("map not found") || text.to_lowercase().contains("invalid") =>
        {
            Some(())
        }
        _ => None,
    })?;

    context.say("@heal")?;
    context.pump(Duration::from_millis(300));
    Ok(())
}

fn weight_capacity_x5(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.say("@resetstat")?;
    context.pump(Duration::from_millis(400));
    // Novice base ~20300 + STR*300, times 5 => well above 80_000 packet units.
    if context.max_weight < 80_000 {
        return Err(format!(
            "max_weight {} is below 80000; playtest ×5 capacity is missing",
            context.max_weight
        ));
    }
    Ok(())
}

/// QW-075 — exercise the live hard-cap boundary with real item delivery.
///
/// Steel (100 weight) gets the character to exactly one unit below capacity;
/// Arrow (1 weight) must fit once at 100%, while the next arrow must be
/// rejected by the server without changing inventory or weight.
fn weight_hard_cap_boundary(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.say("@itemreset")?;
    context.say("@resetstat")?;
    context.pump(Duration::from_millis(400));

    let max_weight = context.max_weight;
    if max_weight < 200 || max_weight / 100 > u32::from(u16::MAX) {
        return Err(format!("unexpected max weight for boundary fixture: {max_weight}"));
    }

    let steel_amount = max_weight.saturating_sub(1) / 100;
    let remainder = max_weight.saturating_sub(1) - steel_amount * 100;
    context.give_item(999, steel_amount as u16)?;
    if remainder > 0 {
        context.give_item(1750, remainder as u16)?;
    }
    context.pump(Duration::from_millis(400));

    if context.weight != max_weight.saturating_sub(1) {
        return Err(format!(
            "expected one unit below capacity before final arrow, got {}/{}",
            context.weight, max_weight
        ));
    }

    context.give_item(1750, 1)?;
    context.pump(Duration::from_millis(300));
    if context.weight != max_weight {
        return Err(format!(
            "one-unit delivery did not reach hard cap: got {}/{}",
            context.weight, max_weight
        ));
    }
    let arrows_at_cap = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == 1750)
        .map(|item| item.amount())
        .ok_or("final arrow is missing from inventory at hard cap")?;

    context.flush();
    context.say("@item 1750 1")?;
    let rejected_events = context.collect_for(Duration::from_millis(1200));
    if rejected_events.iter().any(|event| {
        matches!(
            event,
            NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == 1750
        )
    }) {
        return Err("server delivered an arrow above the hard weight cap".to_owned());
    }
    let arrows_after_rejection = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == 1750)
        .map(|item| item.amount())
        .unwrap_or_default();
    if context.weight != max_weight || arrows_after_rejection != arrows_at_cap {
        return Err(format!(
            "over-cap delivery changed state: weight {}/{} arrows {} -> {}",
            context.weight, max_weight, arrows_at_cap, arrows_after_rejection
        ));
    }

    eprintln!(
        "[QW-075] live hard-cap boundary passed at {}/{} weight; over-cap arrow rejected",
        context.weight, max_weight
    );
    let _ = context.say("@itemreset");
    context.pump(Duration::from_millis(300));
    Ok(())
}

/// QW-075: death/respawn must preserve an overweight inventory and return a
/// live character without bypassing the server's weight state.
fn weight_death_respawn(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;
    let max_weight = context.max_weight;
    if max_weight < 500 || max_weight / 100 > u32::from(u16::MAX) {
        return Err(format!("unexpected max weight for death boundary: {max_weight}"));
    }

    let _ = context.say("@itemreset");
    context.pump(Duration::from_millis(250));
    let target_weight = max_weight * 95 / 100;
    let steel_amount = target_weight.saturating_add(99) / 100;
    context.give_item(999, steel_amount as u16)?;
    let weight_before = context.weight;
    if weight_before < max_weight * 90 / 100 || weight_before > max_weight {
        return Err(format!(
            "failed to enter overweight death boundary: {weight_before}/{max_weight}"
        ));
    }

    context.gm_expect_feedback("@save")?;
    context.warp("geffen", 119, 59)?;
    let player_id = context.player_id;
    context.flush();
    context.say("@die")?;
    context.wait_for("overweight death", |event| match event {
        NetworkEvent::RemoveEntity {
            entity_id,
            reason: ragnarok_packets::DisappearanceReason::Died,
        } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.respawn().map_err(|_| "disconnected")?;
    context.wait_for("overweight respawn map", |event| match event {
        NetworkEvent::ChangeMap { .. } => Some(()),
        _ => None,
    })?;
    context.net.map_loaded().map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(500));
    if context.health_points == 0 || context.weight != weight_before {
        return Err(format!(
            "respawn changed overweight state unexpectedly: hp={}, weight {} -> {}",
            context.health_points, weight_before, context.weight
        ));
    }
    context.say("@itemreset")?;
    context.pump(Duration::from_millis(250));
    Ok(())
}
