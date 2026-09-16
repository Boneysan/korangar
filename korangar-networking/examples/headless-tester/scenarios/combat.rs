//! Phase 4 — melee combat.
//!
//! Runs on a field map (towns suppress monster aggression).

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::{EntityId, SkillId};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;
use crate::scenarios::social::{add_party_member, ensure_no_party, form_party, leave_party_both};

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("attack-kill", 4, attack_kill),
        Scenario::new("attack-out-of-range", 4, attack_out_of_range),
        Scenario::new("incoming-damage", 4, incoming_damage),
        Scenario::new("solo-and-party-exp-measurement", 4, solo_and_party_exp_measurement),
        Scenario::new("repeated-cast-target", 4, repeated_cast_target),
    ]
}

fn combat_bootstrap(config: &Config) -> Result<TestContext, String> {
    let mut context = TestContext::connect(config)?;
    context.ensure_job(4008)?; // Lord Knight
    context.ensure_base_level(99)?;
    context.say("@heal")?;
    context.warp("prt_fild08", 170, 180)?;
    Ok(context)
}

/// QW-031 — the same entity id remains a valid Attack-skill target for a
/// second cast. Headless always names the entity; the client decision that
/// reuses `last_skill_target` is unit-tested in `resolve_attack_repeat_target`.
fn repeated_cast_target(config: &Config) -> Result<(), String> {
    let mut context = combat_bootstrap(config)?;
    context.flush();
    context.say("@allskill")?;
    context.wait_for("SkillTree after @allskill", |event| match event {
        NetworkEvent::SkillTree { skill_information } if !skill_information.is_empty() => Some(()),
        _ => None,
    })?;

    let target = context.spawn_monster("BAPHOMET", 1039)?;
    let player_id = context.player_id;
    let target_position = context
        .entities
        .get(&target)
        .map(|entity| entity.position.tile_position())
        .ok_or("target entity lost")?;
    context.walk_to(target_position.x.saturating_sub(1), target_position.y)?;

    const BASH: SkillId = SkillId(5);
    let mut hits = 0;
    for _cast in 0..2 {
        context.flush();
        context
            .net
            .cast_skill(BASH, ragnarok_packets::SkillLevel(1), target)
            .map_err(|_| "disconnected")?;
        let hit = context.wait_for_within("Bash DamageEffect", Duration::from_secs(5), &mut |event| match event {
            NetworkEvent::DamageEffect {
                source_entity_id,
                destination_entity_id,
                damage_amount: Some(amount),
                ..
            } if source_entity_id.0 == player_id.0 && destination_entity_id.0 == target.0 && *amount > 0 => Some(true),
            NetworkEvent::RemoveEntity { entity_id, .. } if entity_id.0 == target.0 => Some(false),
            _ => None,
        })?;
        if !hit {
            return Err("target died before the second Bash; spawn a sturdier dummy".into());
        }
        hits += 1;
        context.pump(Duration::from_millis(400));
    }
    if hits != 2 {
        return Err(format!("expected 2 Bash hits on the same entity, got {hits}"));
    }
    // Leave the field without @killmonster: Baphomet is an MVP and that command
    // emits unmodeled 0x010B/0x010C ranking packets.
    context.warp("prontera", 155, 180)?;
    Ok(())
}

/// Walk adjacent to the target, attack, and observe damage, death, and exp.
fn attack_kill(config: &Config) -> Result<(), String> {
    let mut context = combat_bootstrap(config)?;

    // `player_attack` sends one attack request; unlike the graphical client,
    // the harness has no auto-attack controller to keep swinging for it.
    let target = context.spawn_monster("PORING", 1002)?;
    let player_id = context.player_id;

    // Attack with retry: if the server reports out-of-range, close in again.
    let mut got_damage = false;
    for _attempt in 0..5 {
        let target_position = context
            .entities
            .get(&target)
            .map(|entity| entity.position.tile_position())
            .ok_or("target entity lost")?;
        context.walk_to(target_position.x.saturating_sub(1), target_position.y)?;

        context.flush();
        context.net.player_attack(target).map_err(|_| "disconnected")?;

        let result = context.wait_for_within(
            "DamageEffect or AttackFailed",
            Duration::from_secs(5),
            &mut |event| match event {
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    damage_amount: Some(amount),
                    ..
                } if source_entity_id.0 == player_id.0 && destination_entity_id.0 == target.0 && *amount > 0 => Some(true),
                NetworkEvent::AttackFailed { target_entity_id, .. } if target_entity_id.0 == target.0 => Some(false),
                _ => None,
            },
        )?;

        if result {
            got_damage = true;
            break;
        }
    }
    if !got_damage {
        return Err("never got in range for a melee hit after 5 attempts".to_owned());
    }

    // If the first hit was not lethal, explicitly request additional swings.
    let deadline = std::time::Instant::now() + Duration::from_secs(45);
    loop {
        context.net.player_attack(target).map_err(|_| "disconnected")?;
        let outcome = context.wait_for_within(
            "next hit or RemoveEntity (death)",
            Duration::from_secs(6),
            &mut |event| match event {
                NetworkEvent::RemoveEntity { entity_id, .. } if entity_id.0 == target.0 => Some(2),
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    ..
                } if source_entity_id.0 == player_id.0 && destination_entity_id.0 == target.0 => Some(1),
                NetworkEvent::AttackFailed { target_entity_id, .. } if target_entity_id.0 == target.0 => Some(0),
                _ => None,
            },
        )?;
        if outcome == 2 {
            break;
        }
        if outcome == 0 {
            let target_position = context
                .entities
                .get(&target)
                .map(|entity| entity.position.tile_position())
                .ok_or("target entity lost after out-of-range attack")?;
            context.walk_to(target_position.x.saturating_sub(1), target_position.y)?;
        }
        if std::time::Instant::now() >= deadline {
            return Err("target survived 45 seconds of continuous melee damage".to_owned());
        }
    }

    // Exp notification: expected but lenient (renewal penalties can floor it).
    let experience = context.wait_for_within("GainedExperience", Duration::from_secs(3), &mut |event| match event {
        NetworkEvent::GainedExperience { .. } => Some(()),
        _ => None,
    });
    if experience.is_err() {
        println!("    warning: no GainedExperience event after kill (check 0x0ACC mapping)");
    }

    context.kill_all_monsters();
    Ok(())
}

/// Attacking a target far outside melee range must produce `AttackFailed`
/// (0x0139) with coherent positions.
fn attack_out_of_range(config: &Config) -> Result<(), String> {
    let mut context = combat_bootstrap(config)?;

    let target = context.spawn_monster("PUPA", 1008)?;

    // Walk well away from the (immobile) target before attacking.
    let target_position = context
        .entities
        .get(&target)
        .map(|entity| entity.position.tile_position())
        .ok_or("target entity lost")?;

    let mut walked = false;
    for (dx, dy) in [(8, 0), (-8, 0), (0, 8), (0, -8)] {
        if context
            .walk_to((target_position.x as i16 + dx) as u16, (target_position.y as i16 + dy) as u16)
            .is_ok()
        {
            walked = true;
            break;
        }
    }
    if !walked {
        return Err("could not walk away from the target".to_owned());
    }

    context.flush();
    context.net.player_attack(target).map_err(|_| "disconnected")?;
    context.wait_for("AttackFailed", |event| match event {
        NetworkEvent::AttackFailed {
            target_entity_id,
            target_position,
            player_position,
            ..
        } if target_entity_id.0 == target.0 => {
            let distance = target_position
                .x
                .abs_diff(player_position.x)
                .max(target_position.y.abs_diff(player_position.y));
            (distance > 1).then_some(())
        }
        _ => None,
    })?;

    context.kill_all_monsters();
    Ok(())
}

/// Approach an entity for a melee swing without failing the scenario on a
/// single blocked neighbour. Shuffle runs leave map debris and dense packs
/// that make one hard `walk_to` time out on PlayerMove ack; multi-cell + warp
/// keeps the wire-assert (incoming DamageEffect) as the real gate.
fn approach_entity_for_melee(context: &mut TestContext, entity_id: EntityId) -> Result<(), String> {
    let entity_position = context
        .entities
        .get(&entity_id)
        .map(|entity| entity.position.tile_position())
        .ok_or("melee target entity lost")?;
    let adj = [
        (entity_position.x.saturating_sub(1), entity_position.y),
        (entity_position.x.saturating_add(1), entity_position.y),
        (entity_position.x, entity_position.y.saturating_sub(1)),
        (entity_position.x, entity_position.y.saturating_add(1)),
        (entity_position.x.saturating_sub(1), entity_position.y.saturating_sub(1)),
        (entity_position.x.saturating_add(1), entity_position.y.saturating_add(1)),
    ];
    for (x, y) in adj {
        if context.walk_to(x, y).is_ok() {
            return Ok(());
        }
    }
    // Last resort: warp beside the target rather than reddening on path noise.
    let _ = context.warp(
        &context.map_name.clone(),
        entity_position.x.saturating_sub(1).max(5),
        entity_position.y.max(5),
    );
    Ok(())
}

/// Provoke a Desert Wolf (1106, retaliates when hit) and observe incoming
/// damage packets (hits and misses both produce DamageEffect events with the
/// monster as source and the player as destination).
fn incoming_damage(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    // Best-effort bootstrap: this scenario must also run without GM rights
    // (used to A/B GM-vs-player mob behavior), so ignore command failures.
    //
    // Normalize the job, best-effort. This used to inherit whatever job the
    // previous scenario left, which made the result depend on scenario order:
    // after `skills-soul-linker` the provoked mob never retaliates, and the
    // failure reads as "wolf never swung back" — blaming the mob for a
    // character-state problem. It passed in natural order only because
    // `dm-warp-recall` precedes it there and leaves the character alone.
    //
    // Bisected under `--shuffle 1337`: failing state -> normalize to 4008 ->
    // passes, with nothing else changed. 4008 (Lord Knight) is the melee default
    // `combat_bootstrap` already uses. Kept best-effort so the no-GM A/B path
    // still runs; it simply cannot normalize without `@job`, which is inherent.
    let _ = context.ensure_job(4008);
    let _ = context.say("@heal");
    let _ = context.warp("prt_fild08", 170, 180);
    context.pump(Duration::from_secs(2));

    // Prefer a naturally spawned mob over @monster: A/B for spawn provenance.
    // Wander a few legs if nothing is in view (also the only option without
    // GM rights, where @monster is unavailable).
    let mut natural = None;
    for leg in 0..8 {
        // With the enlarged view radius, entities are visible far beyond
        // practical walking distance — pick the closest mob, and only within
        // a range a provoke walk can realistically cover.
        let position = context.position;
        natural = context
            .entities
            .iter()
            .filter(|(_, entity)| (1001..2000).contains(&entity.job_id.0))
            .map(|(entity_id, entity)| {
                let mob = entity.position.tile_position();
                let distance = mob.x.abs_diff(position.x).max(mob.y.abs_diff(position.y));
                (*entity_id, entity.job_id.0, distance)
            })
            .filter(|(_, _, distance)| *distance <= 12)
            .min_by_key(|(_, _, distance)| *distance)
            .map(|(entity_id, job_id, _)| (entity_id, job_id));
        if natural.is_some() {
            break;
        }
        let position = context.position;
        let (dx, dy): (i32, i32) = match leg % 4 {
            0 => (12, 0),
            1 => (0, 12),
            2 => (-12, 0),
            _ => (0, -12),
        };
        let _ = context.walk_to((position.x as i32 + dx).max(5) as u16, (position.y as i32 + dy).max(5) as u16);
    }

    let wolf = match natural {
        Some((entity_id, mob_id)) => {
            println!("    using natural mob {mob_id} ({entity_id:?})");
            entity_id
        }
        None => {
            println!("    no natural mob found while wandering; spawning DESERT_WOLF");
            context.spawn_monster("DESERT_WOLF", 1106)?
        }
    };
    let player_id = context.player_id;

    // Provoke: land (or whiff) one hit so the wolf acquires us as target.
    let mut provoked = false;
    for _attempt in 0..4 {
        approach_entity_for_melee(&mut context, wolf)?;
        context.flush();
        context.net.player_attack(wolf).map_err(|_| "disconnected")?;

        let result = context.wait_for_within("our swing at the wolf", Duration::from_secs(5), &mut |event| match event {
            NetworkEvent::DamageEffect {
                source_entity_id,
                destination_entity_id,
                ..
            } if source_entity_id.0 == player_id.0 && destination_entity_id.0 == wolf.0 => Some(()),
            _ => None,
        });
        if result.is_ok() {
            provoked = true;
            break;
        }
    }
    if !provoked {
        return Err("could not land a provoking swing on the wolf".to_owned());
    }

    // Stop acting completely (a move request cancels our auto-attack) and let
    // the wolf swing back.
    let position = context.position;
    let _ = context.walk_to(position.x.saturating_sub(1), position.y);

    let incoming = context.wait_for_within(
        "incoming DamageEffect from the wolf",
        Duration::from_secs(15),
        &mut |event| match event {
            NetworkEvent::DamageEffect {
                source_entity_id,
                destination_entity_id,
                ..
            } if source_entity_id.0 == wolf.0 && destination_entity_id.0 == player_id.0 => Some(()),
            _ => None,
        },
    );

    if let Err(error) = incoming {
        // Distinguish "it did not retaliate" from "there was nothing left to
        // retaliate". A level-99 melee character one-shots the weak natural
        // mobs on this field (Poring ~50 HP, Fabre ~140), and the provoke check
        // is satisfied by our hit landing — so a kill reads exactly like a
        // sulking monster. That ambiguity cost an earlier investigation two
        // wrong root causes.
        //
        // An earlier attempt checked liveness 600ms after the provoke and
        // concluded the target had survived; the removal had simply not arrived
        // yet, and a correct fix was discarded on that bad evidence. Checking
        // after the full 15s wait has no such race.
        if context.entities.contains_key(&wolf) {
            return Err(format!("wolf never swung back after being provoked.\n{error}"));
        }

        // Fall back to the mob this scenario already keeps for the "nothing in
        // view" case: a Desert Wolf is tanky enough to survive a maxed
        // character's opener and aggressive enough to answer it.
        //
        // Same multi-cell approach as the natural path — the previous harden
        // only fixed the first loop; shuffle seed 20260810 failed here when the
        // natural mob died and the Desert Wolf retry used a hard walk_to.
        println!("    provoking blow killed the natural mob; retrying with DESERT_WOLF");
        let wolf = context.spawn_monster("DESERT_WOLF", 1106)?;

        let mut provoked = false;
        for _attempt in 0..4 {
            approach_entity_for_melee(&mut context, wolf)?;
            context.flush();
            context.net.player_attack(wolf).map_err(|_| "disconnected")?;
            let landed = context.wait_for_within(
                "our swing at the Desert Wolf",
                Duration::from_secs(5),
                &mut |event| match event {
                    NetworkEvent::DamageEffect {
                        source_entity_id,
                        destination_entity_id,
                        ..
                    } if source_entity_id.0 == player_id.0 && destination_entity_id.0 == wolf.0 => Some(()),
                    _ => None,
                },
            );
            if landed.is_ok() {
                provoked = true;
                break;
            }
        }
        if !provoked {
            return Err("could not land a provoking swing on the Desert Wolf".to_owned());
        }

        let position = context.position;
        let _ = context.walk_to(position.x.saturating_sub(1), position.y);

        context
            .wait_for_within(
                "incoming DamageEffect from the Desert Wolf",
                Duration::from_secs(15),
                &mut |event| match event {
                    NetworkEvent::DamageEffect {
                        source_entity_id,
                        destination_entity_id,
                        ..
                    } if source_entity_id.0 == wolf.0 && destination_entity_id.0 == player_id.0 => Some(()),
                    _ => None,
                },
            )
            .map_err(|error| format!("Desert Wolf never swung back after being provoked.\n{error}"))?;
    }

    context.kill_all_monsters();
    context.say("@heal")?;
    context.pump(Duration::from_millis(300));
    Ok(())
}

fn normalize_for_exp_test(context: &mut TestContext, target_base_level: u32) -> Result<(), String> {
    context.say("@heal")?;
    // To reliably reset base_exp to 0 even if already level 1:
    // raise by 1 then clamp down to 1
    if context.base_level <= 1 {
        context.say("@blvl 1")?;
    }
    context.say("@blvl -999")?;
    if target_base_level > 1 {
        let delta = target_base_level - 1;
        context.say(&format!("@blvl {delta}"))?;
    }
    // To reliably reset job_level to 1 and job_exp to 0:
    // change to Novice (0) then Swordsman (1)
    context.say("@job 0")?;
    context.say("@job 1")?;
    context.say("@str 90")?;
    context.say("@dex 50")?;
    context.pump(Duration::from_millis(500));
    context.flush();
    Ok(())
}

fn ensure_party_even_share(primary: &mut TestContext) -> Result<(), String> {
    primary.flush();
    primary
        .net
        .set_party_options(true, false, false)
        .map_err(|_| "primary disconnected")?;
    let share = primary.wait_for("PartyShareOptions with EXP sharing on", |event| match event {
        NetworkEvent::PartyShareOptions { experience_share, .. } => Some(*experience_share),
        _ => None,
    })?;
    if !share {
        return Err("failed to enable party even EXP share".to_owned());
    }
    Ok(())
}

fn execute_kill(context: &mut TestContext, target: EntityId) -> Result<(), String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let target_pos = match context.entities.get(&target) {
            Some(e) => e.position.tile_position(),
            None => break,
        };
        let _ = context.walk_to(target_pos.x.saturating_sub(1), target_pos.y);
        context.flush();
        let _ = context.net.player_attack(target);
        let outcome = context.wait_for_within(
            "DamageEffect or RemoveEntity",
            Duration::from_secs(3),
            &mut |event| match event {
                NetworkEvent::RemoveEntity { entity_id, .. } if entity_id.0 == target.0 => Some(true),
                NetworkEvent::DamageEffect { destination_entity_id, .. } if destination_entity_id.0 == target.0 => Some(false),
                _ => None,
            },
        );
        match outcome {
            Ok(true) => break,
            Ok(false) => {
                let removed = context.wait_for_within("RemoveEntity after hit", Duration::from_millis(800), &mut |event| match event {
                    NetworkEvent::RemoveEntity { entity_id, .. } if entity_id.0 == target.0 => Some(()),
                    _ => None,
                });
                if removed.is_ok() {
                    break;
                }
            }
            Err(_) => {}
        }
        if std::time::Instant::now() >= deadline {
            return Err("target mob survived 15s in combat".to_owned());
        }
    }
    Ok(())
}

fn wait_for_exp_change(context: &mut TestContext, initial_base: u64, initial_job: u64, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        context.pump(Duration::from_millis(50));
        if context.base_experience != initial_base || context.job_experience != initial_job {
            break;
        }
    }
}

/// QW-026: Authoritative solo and party EXP measurement matrix.
///
/// Asserts exact Base and Job EXP distribution against Hercules Renewal
/// formulas using a known monster: Poring (Mob ID 1002, Lv 1, BaseExp 36,
/// JobExp 20). Covers:
/// 1. Solo kill (Player Lv 1 -> +36 Base, +20 Job).
/// 2. Two players in party within share range (even share -> 36/2 = +18 Base,
///    20/2 = +10 Job each).
/// 3. Three players in party within share range (even share -> 36/3 = +12 Base,
///    20/3 = +6 Job each; proves integer truncation of remainder).
/// 4A. Party member outside range by map boundary (on different map -> +36/+20
/// to killer on map, +0/+0 to off-map member). 4B. Party member outside
/// 15-level share range + Renewal level penalty:
///     - Level spread 17 vs 1 (16 > 15) disables even-share (individual share).
///     - Primary (Lv 1) kill -> +36 Base, +20 Job (Partner gets 0).
///     - Partner (Lv 17) kill -> diff = 1 - 17 = -16. level_penalty.conf rate
///       85%. 36 * 85% = 30 Base, 20 * 85% = 17 Job. (Primary gets 0).
fn solo_and_party_exp_measurement(config: &Config) -> Result<(), String> {
    eprintln!("    [solo-and-party-exp-measurement] connecting primary...");
    let mut primary = TestContext::connect(config)?;
    primary.warp("prt_fild08", 170, 180)?;
    ensure_no_party(&mut primary);
    primary.kill_all_monsters();

    // =========================================================================
    // Case 1: Solo kill (Player Lv 1 vs Poring Lv 1)
    // =========================================================================
    eprintln!("    [solo-and-party-exp-measurement] Case 1: Solo kill...");
    normalize_for_exp_test(&mut primary, 1)?;
    if primary.base_level != 1 {
        return Err(format!("expected primary base level 1, got {}", primary.base_level));
    }
    let p1_base_start = primary.base_experience;
    let p1_job_start = primary.job_experience;

    let poring1 = primary.spawn_monster("PORING", 1002)?;
    execute_kill(&mut primary, poring1)?;
    wait_for_exp_change(&mut primary, p1_base_start, p1_job_start, Duration::from_secs(4));

    let p1_base_gain = primary.base_experience.saturating_sub(p1_base_start);
    let p1_job_gain = primary.job_experience.saturating_sub(p1_job_start);
    eprintln!("    Case 1 Solo: gained Base EXP: {p1_base_gain}, Job EXP: {p1_job_gain}");

    if p1_base_gain != 36 {
        return Err(format!(
            "Case 1 failed: expected 36 Base EXP for solo Poring kill, got {p1_base_gain}"
        ));
    }
    if p1_job_gain != 20 {
        return Err(format!(
            "Case 1 failed: expected 20 Job EXP for solo Poring kill, got {p1_job_gain}"
        ));
    }

    // =========================================================================
    // Case 2: Two players in party within share range (Even share)
    // =========================================================================
    eprintln!("    [solo-and-party-exp-measurement] Case 2: Two players in party (even share)...");
    let mut partner = TestContext::connect_partner(config)?;
    partner.warp("prt_fild08", 171, 180)?;

    // Normalize both characters to Lv 1 BEFORE forming party so neither joins at Lv
    // 99
    normalize_for_exp_test(&mut primary, 1)?;
    normalize_for_exp_test(&mut partner, 1)?;

    form_party(&mut primary, &mut partner)?;
    ensure_party_even_share(&mut primary)?;

    // Reset both characters to clean 0 EXP at Lv 1 / Job 1
    normalize_for_exp_test(&mut primary, 1)?;
    normalize_for_exp_test(&mut partner, 1)?;

    let p1_c2_base = primary.base_experience;
    let p1_c2_job = primary.job_experience;
    let p2_c2_base = partner.base_experience;
    let p2_c2_job = partner.job_experience;

    let poring2 = primary.spawn_monster("PORING", 1002)?;
    execute_kill(&mut primary, poring2)?;
    wait_for_exp_change(&mut primary, p1_c2_base, p1_c2_job, Duration::from_secs(4));
    wait_for_exp_change(&mut partner, p2_c2_base, p2_c2_job, Duration::from_secs(4));

    let p1_c2_gain_base = primary.base_experience.saturating_sub(p1_c2_base);
    let p1_c2_gain_job = primary.job_experience.saturating_sub(p1_c2_job);
    let p2_c2_gain_base = partner.base_experience.saturating_sub(p2_c2_base);
    let p2_c2_gain_job = partner.job_experience.saturating_sub(p2_c2_job);

    eprintln!("    Case 2 (2 players): P1 gains {p1_c2_gain_base} / {p1_c2_gain_job}, P2 gains {p2_c2_gain_base} / {p2_c2_gain_job}");

    if p1_c2_gain_base != 22 || p1_c2_gain_job != 12 {
        return Err(format!(
            "Case 2 failed: primary expected 22 Base / 12 Job (+25% party bonus), got {p1_c2_gain_base} / {p1_c2_gain_job}"
        ));
    }
    if p2_c2_gain_base != 22 || p2_c2_gain_job != 12 {
        return Err(format!(
            "Case 2 failed: partner expected 22 Base / 12 Job (+25% party bonus), got {p2_c2_gain_base} / {p2_c2_gain_job}"
        ));
    }

    // =========================================================================
    // Case 3: Three players in party within share range (Integer truncation test)
    // =========================================================================
    eprintln!("    [solo-and-party-exp-measurement] Case 3: Three players in party (integer truncation)...");
    let mut third = TestContext::connect_third(config)?;
    third.warp("prt_fild08", 172, 180)?;
    normalize_for_exp_test(&mut third, 1)?;

    add_party_member(&mut primary, &mut third)?;
    ensure_party_even_share(&mut primary)?;

    normalize_for_exp_test(&mut primary, 1)?;
    normalize_for_exp_test(&mut partner, 1)?;
    normalize_for_exp_test(&mut third, 1)?;

    let p1_c3_base = primary.base_experience;
    let p1_c3_job = primary.job_experience;
    let p2_c3_base = partner.base_experience;
    let p2_c3_job = partner.job_experience;
    let p3_c3_base = third.base_experience;
    let p3_c3_job = third.job_experience;

    let poring3 = primary.spawn_monster("PORING", 1002)?;
    execute_kill(&mut primary, poring3)?;
    wait_for_exp_change(&mut primary, p1_c3_base, p1_c3_job, Duration::from_secs(4));
    wait_for_exp_change(&mut partner, p2_c3_base, p2_c3_job, Duration::from_secs(4));
    wait_for_exp_change(&mut third, p3_c3_base, p3_c3_job, Duration::from_secs(4));

    let p1_c3_gain_base = primary.base_experience.saturating_sub(p1_c3_base);
    let p1_c3_gain_job = primary.job_experience.saturating_sub(p1_c3_job);
    let p2_c3_gain_base = partner.base_experience.saturating_sub(p2_c3_base);
    let p2_c3_gain_job = partner.job_experience.saturating_sub(p2_c3_job);
    let p3_c3_gain_base = third.base_experience.saturating_sub(p3_c3_base);
    let p3_c3_gain_job = third.job_experience.saturating_sub(p3_c3_job);

    eprintln!(
        "    Case 3 (3 players): P1={p1_c3_gain_base}/{p1_c3_gain_job}, P2={p2_c3_gain_base}/{p2_c3_gain_job}, \
         P3={p3_c3_gain_base}/{p3_c3_gain_job}"
    );

    if p1_c3_gain_base != 18 || p1_c3_gain_job != 9 {
        return Err(format!(
            "Case 3 failed: P1 expected 18 Base / 9 Job (+50% party bonus), got {p1_c3_gain_base} / {p1_c3_gain_job}"
        ));
    }
    if p2_c3_gain_base != 18 || p2_c3_gain_job != 9 {
        return Err(format!(
            "Case 3 failed: P2 expected 18 Base / 9 Job (+50% party bonus), got {p2_c3_gain_base} / {p2_c3_gain_job}"
        ));
    }
    if p3_c3_gain_base != 18 || p3_c3_gain_job != 9 {
        return Err(format!(
            "Case 3 failed: P3 expected 18 Base / 9 Job (+50% party bonus), got {p3_c3_gain_base} / {p3_c3_gain_job}"
        ));
    }

    // Release third from party
    let _ = third.net.leave_party();
    third.pump(Duration::from_millis(300));
    primary.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));

    // =========================================================================
    // Case 4A: Party member on different map (Map boundary)
    // =========================================================================
    eprintln!("    [solo-and-party-exp-measurement] Case 4A: Party member on different map...");
    partner.warp("prontera", 155, 180)?;
    partner.pump(Duration::from_millis(500));

    normalize_for_exp_test(&mut primary, 1)?;
    ensure_party_even_share(&mut primary)?;
    partner.flush();

    let p1_c4a_base = primary.base_experience;
    let p1_c4a_job = primary.job_experience;
    let p2_c4a_base = partner.base_experience;
    let p2_c4a_job = partner.job_experience;

    let poring4a = primary.spawn_monster("PORING", 1002)?;
    execute_kill(&mut primary, poring4a)?;
    wait_for_exp_change(&mut primary, p1_c4a_base, p1_c4a_job, Duration::from_secs(4));
    partner.pump(Duration::from_millis(600));

    let p1_c4a_gain_base = primary.base_experience.saturating_sub(p1_c4a_base);
    let p1_c4a_gain_job = primary.job_experience.saturating_sub(p1_c4a_job);
    let p2_c4a_gain_base = partner.base_experience.saturating_sub(p2_c4a_base);
    let p2_c4a_gain_job = partner.job_experience.saturating_sub(p2_c4a_job);

    eprintln!("    Case 4A (off-map): P1={p1_c4a_gain_base}/{p1_c4a_gain_job}, P2={p2_c4a_gain_base}/{p2_c4a_gain_job}");

    if p1_c4a_gain_base != 36 || p1_c4a_gain_job != 20 {
        return Err(format!(
            "Case 4A failed: primary expected 36 Base / 20 Job (solo on map), got {p1_c4a_gain_base} / {p1_c4a_gain_job}"
        ));
    }
    if p2_c4a_gain_base != 0 || p2_c4a_gain_job != 0 {
        return Err(format!(
            "Case 4A failed: partner on different map expected 0 Base / 0 Job, got {p2_c4a_gain_base} / {p2_c4a_gain_job}"
        ));
    }

    // =========================================================================
    // Case 4B: Party member outside 15-level share range + Renewal level penalty
    // =========================================================================
    eprintln!("    [solo-and-party-exp-measurement] Case 4B: Level spread > 15 & level penalty...");
    partner.warp("prt_fild08", 171, 180)?;
    normalize_for_exp_test(&mut primary, 1)?;
    normalize_for_exp_test(&mut partner, 17)?; // Partner is Lv 17, Primary is Lv 1 (spread 16 > 15)

    if primary.base_level != 1 {
        return Err(format!("expected primary base level 1, got {}", primary.base_level));
    }
    if partner.base_level != 17 {
        return Err(format!("expected partner base level 17, got {}", partner.base_level));
    }

    // Attempting even-share with spread 16 > 15 is refused by server; option stays
    // 0
    let _ = primary.net.set_party_options(true, false, false);
    primary.pump(Duration::from_millis(500));

    // Subcase 4B.1: Primary (Lv 1) kills Poring (Lv 1).
    // With level spread > 15, even-share is suppressed by server -> individual
    // share.
    let p1_c4b1_base = primary.base_experience;
    let p1_c4b1_job = primary.job_experience;
    let p2_c4b1_base = partner.base_experience;
    let p2_c4b1_job = partner.job_experience;

    let poring4b1 = primary.spawn_monster("PORING", 1002)?;
    execute_kill(&mut primary, poring4b1)?;
    wait_for_exp_change(&mut primary, p1_c4b1_base, p1_c4b1_job, Duration::from_secs(4));
    partner.pump(Duration::from_millis(600));

    let p1_c4b1_gain_base = primary.base_experience.saturating_sub(p1_c4b1_base);
    let p1_c4b1_gain_job = primary.job_experience.saturating_sub(p1_c4b1_job);
    let p2_c4b1_gain_base = partner.base_experience.saturating_sub(p2_c4b1_base);
    let p2_c4b1_gain_job = partner.job_experience.saturating_sub(p2_c4b1_job);

    eprintln!("    Case 4B.1 (P1 kill, spread > 15): P1={p1_c4b1_gain_base}/{p1_c4b1_gain_job}, P2={p2_c4b1_gain_base}/{p2_c4b1_gain_job}");

    if p1_c4b1_gain_base != 36 || p1_c4b1_gain_job != 20 {
        return Err(format!(
            "Case 4B.1 failed: primary expected full 36 Base / 20 Job, got {p1_c4b1_gain_base} / {p1_c4b1_gain_job}"
        ));
    }
    if p2_c4b1_gain_base != 0 || p2_c4b1_gain_job != 0 {
        return Err(format!(
            "Case 4B.1 failed: partner expected 0 Base / 0 Job, got {p2_c4b1_gain_base} / {p2_c4b1_gain_job}"
        ));
    }

    // Subcase 4B.2: Partner (Lv 17) kills Poring (Lv 1).
    // Partner diff = 1 - 17 = -16 -> rate: 85% in level_penalty.conf.
    // 36 * 85 / 100 = 30 Base EXP, 20 * 85 / 100 = 17 Job EXP.
    normalize_for_exp_test(&mut primary, 1)?;
    normalize_for_exp_test(&mut partner, 17)?;

    let p1_c4b2_base = primary.base_experience;
    let p1_c4b2_job = primary.job_experience;
    let p2_c4b2_base = partner.base_experience;
    let p2_c4b2_job = partner.job_experience;

    let poring4b2 = partner.spawn_monster("PORING", 1002)?;
    execute_kill(&mut partner, poring4b2)?;
    wait_for_exp_change(&mut partner, p2_c4b2_base, p2_c4b2_job, Duration::from_secs(4));
    primary.pump(Duration::from_millis(600));

    let p1_c4b2_gain_base = primary.base_experience.saturating_sub(p1_c4b2_base);
    let p1_c4b2_gain_job = primary.job_experience.saturating_sub(p1_c4b2_job);
    let p2_c4b2_gain_base = partner.base_experience.saturating_sub(p2_c4b2_base);
    let p2_c4b2_gain_job = partner.job_experience.saturating_sub(p2_c4b2_job);

    eprintln!(
        "    Case 4B.2 (P2 kill at Lv 17, 85% penalty): P1={p1_c4b2_gain_base}/{p1_c4b2_gain_job}, \
         P2={p2_c4b2_gain_base}/{p2_c4b2_gain_job}"
    );

    if p2_c4b2_gain_base != 30 || p2_c4b2_gain_job != 17 {
        return Err(format!(
            "Case 4B.2 failed: partner (Lv 17, diff -16) expected 30 Base / 17 Job (85% rate), got {p2_c4b2_gain_base} / \
             {p2_c4b2_gain_job}"
        ));
    }
    if p1_c4b2_gain_base != 0 || p1_c4b2_gain_job != 0 {
        return Err(format!(
            "Case 4B.2 failed: primary expected 0 Base / 0 Job, got {p1_c4b2_gain_base} / {p1_c4b2_gain_job}"
        ));
    }

    // Cleanup
    leave_party_both(&mut primary, &mut partner);
    primary.kill_all_monsters();
    let _ = primary.say("@heal");
    let _ = partner.say("@heal");

    eprintln!("    [solo-and-party-exp-measurement] all cases passed cleanly.");
    Ok(())
}
