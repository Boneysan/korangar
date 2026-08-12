//! Phase 4 — melee combat.
//!
//! Runs on a field map (towns suppress monster aggression).

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::EntityId;

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("attack-kill", 4, attack_kill),
        Scenario::new("attack-out-of-range", 4, attack_out_of_range),
        Scenario::new("incoming-damage", 4, incoming_damage),
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
