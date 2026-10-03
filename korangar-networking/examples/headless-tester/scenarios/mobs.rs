//! Phase 10 — monster behaviour (GDD F14 / F15).
//!
//! The AI profile checkers prove a profile FILE is valid and that the server
//! read it. They cannot say whether a monster then behaves as the profile
//! promises. These watch real monsters on the real server and judge the
//! movement and casts they produce, each against a control: the same monster,
//! same attacks, on a map with no profile must NOT do it.
//!
//! What each profile promises is in `mob_ai_profile_db.conf` and `mob.c`:
//!   * Skirmisher: after `HitThreshold` hits from a player, if the monster is
//!     standing still and its cooldown has passed, it takes ONE step to an
//!     adjacent cell farther from the player.
//!   * RangedKeeper: if the player is closer than `PreferredRange`, it takes
//!     one such step per cooldown.

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::{EntityId, TilePosition};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;
use crate::scenarios::dm::{say_expect, wait_for_text};

const ORC_SKELETON: (&str, u16) = ("ORC_SKELETON", 1152);
const ORC_ARCHER: (&str, u16) = ("ORC_ARCHER", 1189);
const RAYDRIC_ARCHER: (&str, u16) = ("RAYDRIC_ARCHER", 1276);
const ELITE_ORC_SKELETON: (&str, u16) = ("ELITE_ORC_SKELETON", 20901);
const EDDGA: (&str, u16) = ("EDDGA", 1115);

const SKILL_MAGNUM: u16 = 7;
const SKILL_TELEPORT: u16 = 26;
const SKILL_METEOR: u16 = 83;
const SKILL_POWERUP: u16 = 349;
const SKILL_SPEEDUP: u16 = 332;

/// A map with a profile for the monster, and one with none.
const SKELETON_PROFILE_MAP: &str = "orcsdun01";
const ARCHER_PROFILE_MAP: &str = "orcsdun02";
const RAYDRIC_PROFILE_MAP: &str = "gl_knt01";
const NO_PROFILE_MAP: &str = "prt_fild08";

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("mob-skirmisher-orc-skeleton", 10, skirmisher_orc_skeleton),
        Scenario::new("mob-skirmisher-elite", 10, skirmisher_elite),
        Scenario::new("mob-ranged-keeper-orc-archer", 10, ranged_keeper_orc_archer),
        Scenario::new("mob-raydric-archer-keeper", 10, ranged_keeper_raydric_archer),
        Scenario::new("mob-coward-poring", 10, coward_poring),
        Scenario::new("mob-eddga-pilot-skills", 10, eddga_pilot_skills),
        Scenario::new("mob-eddga-meteor-and-enrage", 10, eddga_meteor_and_enrage),
        Scenario::new("mob-elite-population", 10, elite_population),
        Scenario::new("mob-elite-rollback-switch", 10, elite_rollback_switch),
    ]
}

// ---- observing a monster
// -------------------------------------------------------

/// One thing that happened to or around the monster, in the order it arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seen {
    /// The player damaged it, for this much.
    Hit(usize),
    /// The player swung and did no damage (a miss, or zero).
    Miss,
    /// The server refused the attack (out of range, not attackable...).
    Refused,
    /// It moved one cell. `away` means that cell is farther from the player.
    Step {
        tick: u32,
        away: bool,
        distance_after: i32,
        length: i32,
    },
}

fn chebyshev(a: TilePosition, b: TilePosition) -> i32 {
    (i32::from(a.x) - i32::from(b.x)).abs().max((i32::from(a.y) - i32::from(b.y)).abs())
}

fn record(context: &TestContext, mob: EntityId, events: Vec<NetworkEvent>, seen: &mut Vec<Seen>) {
    for event in events {
        match event {
            NetworkEvent::DamageEffect {
                source_entity_id,
                destination_entity_id,
                damage_amount: Some(amount),
                ..
            } if source_entity_id == context.player_id && destination_entity_id == mob && amount > 0 => seen.push(Seen::Hit(amount)),
            NetworkEvent::DamageEffect {
                source_entity_id,
                destination_entity_id,
                ..
            } if source_entity_id == context.player_id && destination_entity_id == mob => seen.push(Seen::Miss),
            NetworkEvent::AttackFailed { .. } => seen.push(Seen::Refused),
            NetworkEvent::EntityMove {
                entity_id,
                origin,
                destination,
                starting_timestamp,
            } if entity_id == mob => {
                let me = context.position;
                let (from, to) = (origin.tile_position(), destination.tile_position());
                let distance_after = chebyshev(to, me);
                seen.push(Seen::Step {
                    tick: starting_timestamp.0,
                    away: chebyshev(from, to) == 1 && distance_after > chebyshev(from, me),
                    distance_after,
                    length: chebyshev(from, to),
                });
            }
            _ => {}
        }
    }
}

fn retreat_ticks(seen: &[Seen]) -> Vec<u32> {
    seen.iter()
        .filter_map(|item| match item {
            Seen::Step { tick, away: true, .. } => Some(*tick),
            _ => None,
        })
        .collect()
}

/// "12 hit, 3 missed, 0 refused, 2 steps (1 away)": what a failed run actually
/// saw.
fn summary(seen: &[Seen]) -> String {
    let count = |wanted: fn(&Seen) -> bool| seen.iter().filter(|item| wanted(item)).count();
    format!(
        "{} hit, {} missed, {} refused, {} step(s) ({} away)",
        count(|item| matches!(item, Seen::Hit(_))),
        count(|item| matches!(item, Seen::Miss)),
        count(|item| matches!(item, Seen::Refused)),
        count(|item| matches!(item, Seen::Step { .. })),
        count(|item| matches!(item, Seen::Step { away: true, .. })),
    )
}

fn hits(seen: &[Seen]) -> usize {
    seen.iter().filter(|item| matches!(item, Seen::Hit(_))).count()
}

/// Hits the player had landed when the first retreat happened.
fn hits_before_first_retreat(seen: &[Seen]) -> Option<usize> {
    let mut landed = 0;
    for item in seen {
        match item {
            Seen::Hit(_) => landed += 1,
            Seen::Step { away: true, .. } => return Some(landed),
            _ => {}
        }
    }
    None
}

/// A level-99 Novice: tough enough to be healed between swings, weak enough
/// that a monster with thousands of HP survives many hits (the skirmisher
/// needs several, and a strong class would simply kill it first).
fn novice_on(config: &Config, map: &str) -> Result<TestContext, String> {
    attacker_on(config, map, 0)
}

/// A level-99 character of `job` on `map`.
fn attacker_on(config: &Config, map: &str, job: u16) -> Result<TestContext, String> {
    attacker_at_level(config, map, job, 99)
}

fn attacker_at_level(config: &Config, map: &str, job: u16, level: u32) -> Result<TestContext, String> {
    let mut context = TestContext::connect(config)?;
    context.ensure_job(job)?;
    context.ensure_base_level(level)?;
    if job != 0 {
        // The elite has 90 DEF; an unarmed character's damage is below that and
        // every swing is a zero. Strength and dexterity make the swings count.
        context.say("@str 90")?;
        context.say("@dex 60")?;
        context.pump(Duration::from_millis(300));
    }
    context.say("@heal")?;
    context.warp_random(map)?;
    context.pump(Duration::from_millis(500));
    Ok(context)
}

fn clear_map(context: &mut TestContext) {
    let _ = context.say("@killmonster");
    context.pump(Duration::from_millis(300));
    let _ = context.say("@alive");
    context.pump(Duration::from_millis(200));
}

/// Hit a freshly spawned monster `swings` times, one swing every `gap`.
fn beat_on(context: &mut TestContext, mob: (&str, u16), swings: usize, gap: Duration) -> Result<Vec<Seen>, String> {
    let target = context.spawn_monster(mob.0, mob.1)?;
    let mut seen = Vec::new();
    for swing in 0..swings {
        if swing % 3 == 0 {
            context.say("@heal")?;
        }
        context.flush();
        context.net.player_attack(target).map_err(|_| "disconnected")?;
        let events = context.collect_for(gap);
        record(context, target, events, &mut seen);
    }
    Ok(seen)
}

/// How many times to try a profile case at a fresh spot before giving up. The
/// step needs an open cell next to the monster and farther from the player; in
/// a corridor there is none and the code, by design, falls through to the stock
/// AI. That is terrain, not a fault, so a case may need another spot.
const ATTEMPTS: usize = 4;

/// A skirmisher on its profile map, and the same monster and swings on a map
/// without one.
fn skirmisher_case(config: &Config, mob: (&str, u16), job: u16, swings: usize, threshold: usize, cooldown_ms: u32) -> Result<(), String> {
    let gap = Duration::from_millis(700);

    let mut last_problem = String::new();
    let mut proven = false;
    for attempt in 1..=ATTEMPTS {
        let mut context = attacker_on(config, SKELETON_PROFILE_MAP, job)?;
        let seen = beat_on(&mut context, mob, swings, gap);
        clear_map(&mut context);
        let seen = seen?;

        let landed = hits(&seen);
        if landed < threshold * 2 {
            last_problem = format!(
                "attempt {attempt}: only {landed} hits landed on {} in {swings} swings ({seen:?})",
                mob.0
            );
            continue;
        }
        let ticks = retreat_ticks(&seen);
        if ticks.is_empty() {
            last_problem = format!(
                "attempt {attempt}: {} took {landed} hits and never stepped away ({seen:?})",
                mob.0
            );
            continue;
        }
        // Once it has stepped away, everything it promises must hold in that same
        // attempt.
        let first = hits_before_first_retreat(&seen).unwrap_or(0);
        if first < threshold {
            return Err(format!(
                "{} retreated after only {first} hit(s); the threshold is {threshold} ({seen:?})",
                mob.0
            ));
        }
        for pair in ticks.windows(2) {
            let gap_ms = pair[1].wrapping_sub(pair[0]);
            // Allow up to one server tick (100ms) of timer jitter.
            if gap_ms < cooldown_ms.saturating_sub(100) {
                return Err(format!(
                    "two retreats {gap_ms} ms apart; the cooldown is {cooldown_ms} ms ({ticks:?})"
                ));
            }
        }
        let most_allowed = landed / threshold + 1;
        if ticks.len() > most_allowed {
            return Err(format!(
                "{} retreats from {landed} hits; at most {most_allowed} are possible at threshold {threshold}",
                ticks.len()
            ));
        }
        proven = true;
        break;
    }
    if !proven {
        return Err(format!("no attempt showed the skirmisher stepping away: {last_problem}"));
    }

    // The control: identical monster and swings where no profile exists. One
    // attempt is enough, because it must show NOTHING.
    let mut control = attacker_on(config, NO_PROFILE_MAP, job)?;
    let seen = beat_on(&mut control, mob, swings, gap);
    clear_map(&mut control);
    let seen = seen?;
    if hits(&seen) < threshold * 2 {
        return Err(format!(
            "control: only {} hits landed, so it proves nothing ({seen:?})",
            hits(&seen)
        ));
    }
    let stray = retreat_ticks(&seen);
    if !stray.is_empty() {
        return Err(format!(
            "control: {} stepped away {} time(s) on {NO_PROFILE_MAP}, where it has no profile ({seen:?})",
            mob.0,
            stray.len()
        ));
    }
    Ok(())
}

// ---- scenarios
// ------------------------------------------------------------------

/// `ORC_SKELETON` on `orcsdun01`: Skirmisher, HitThreshold 3, Cooldown 3000.
fn skirmisher_orc_skeleton(config: &Config) -> Result<(), String> {
    skirmisher_case(config, ORC_SKELETON, 0, 16, 3, 3000)
}

/// The elite: Skirmisher, HitThreshold 2, StepDistance 4, Cooldown 4000.
/// Different numbers from the plain skeleton, so honouring them per profile is
/// what is tested.
fn skirmisher_elite(config: &Config) -> Result<(), String> {
    // A Novice cannot land a hit on the elite (0 of 16), so use a Lord Knight; 10
    // swings leave an elite with 6,000 HP alive long enough to retreat twice.
    skirmisher_case(config, ELITE_ORC_SKELETON, 4008, 10, 2, 4000)
}

/// `ORC_ARCHER` on `orcsdun02`: RangedKeeper, PreferredRange 5, Cooldown 1500.
/// Stand still next to one: it should back off a cell at a time until it is out
/// of the band.
fn ranged_keeper_orc_archer(config: &Config) -> Result<(), String> {
    const PREFERRED_RANGE: i32 = 5;
    const COOLDOWN_MS: u32 = 1500;

    let observe = |map: &str| -> Result<Vec<Seen>, String> {
        let mut context = novice_on(config, map)?;
        let target = context.spawn_monster(ORC_ARCHER.0, ORC_ARCHER.1)?;
        context.flush();
        let mut seen = Vec::new();
        for second in 0..12 {
            if second % 3 == 0 {
                context.say("@heal")?;
            }
            let events = context.collect_for(Duration::from_millis(1000));
            record(&context, target, events, &mut seen);
        }
        clear_map(&mut context);
        Ok(seen)
    };

    let mut last_problem = String::new();
    let mut proven = false;
    for attempt in 1..=ATTEMPTS {
        let seen = observe(ARCHER_PROFILE_MAP)?;
        let ticks = retreat_ticks(&seen);
        if ticks.len() < 2 {
            last_problem = format!("attempt {attempt}: {} retreat step(s) in 12 s ({seen:?})", ticks.len());
            continue;
        }
        for pair in ticks.windows(2) {
            let gap_ms = pair[1].wrapping_sub(pair[0]);
            if gap_ms < COOLDOWN_MS.saturating_sub(100) {
                return Err(format!(
                    "two retreat steps {gap_ms} ms apart; the cooldown is {COOLDOWN_MS} ms ({ticks:?})"
                ));
            }
        }
        let farthest = seen
            .iter()
            .filter_map(|item| match item {
                Seen::Step { distance_after, .. } => Some(*distance_after),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        if farthest < PREFERRED_RANGE - 1 {
            last_problem = format!("attempt {attempt}: only {farthest} cell(s) away; the band is {PREFERRED_RANGE} ({seen:?})");
            continue;
        }
        proven = true;
        break;
    }
    if !proven {
        return Err(format!("no attempt showed the archer keeping its distance: {last_problem}"));
    }

    // The control: a stock archer next to the player does not back off.
    let control = observe(NO_PROFILE_MAP)?;
    let stray = retreat_ticks(&control);
    if !stray.is_empty() {
        return Err(format!(
            "control: the archer backed off {} time(s) on {NO_PROFILE_MAP}, where it has no profile ({control:?})",
            stray.len()
        ));
    }
    Ok(())
}

/// `RAYDRIC_ARCHER` on `gl_knt01`: RangedKeeper, PreferredRange 6, Cooldown 2000.
/// Stand still next to one: it should back off a cell at a time until it is out
/// of the band.
fn ranged_keeper_raydric_archer(config: &Config) -> Result<(), String> {
    const PREFERRED_RANGE: i32 = 6;
    const COOLDOWN_MS: u32 = 2000;

    let observe = |map: &str| -> Result<Vec<Seen>, String> {
        let mut context = attacker_on(config, map, 4008)?;
        context.say("@vit 99")?;
        context.say("@agi 99")?;
        context.say("@heal")?;
        context.pump(Duration::from_millis(200));
        let target = context.spawn_monster(RAYDRIC_ARCHER.0, RAYDRIC_ARCHER.1)?;
        context.flush();
        let mut seen = Vec::new();
        for second in 0..14 {
            if second % 2 == 0 {
                context.say("@heal")?;
            }
            let events = context.collect_for(Duration::from_millis(1000));
            record(&context, target, events, &mut seen);
        }
        clear_map(&mut context);
        Ok(seen)
    };

    let mut last_problem = String::new();
    let mut proven = false;
    for attempt in 1..=ATTEMPTS {
        let seen = observe(RAYDRIC_PROFILE_MAP)?;
        let ticks = retreat_ticks(&seen);
        if ticks.len() < 2 {
            last_problem = format!("attempt {attempt}: {} retreat step(s) in 14 s ({seen:?})", ticks.len());
            continue;
        }
        for pair in ticks.windows(2) {
            let gap_ms = pair[1].wrapping_sub(pair[0]);
            if gap_ms < COOLDOWN_MS.saturating_sub(100) {
                return Err(format!(
                    "two retreat steps {gap_ms} ms apart; the cooldown is {COOLDOWN_MS} ms ({ticks:?})"
                ));
            }
        }
        let farthest = seen
            .iter()
            .filter_map(|item| match item {
                Seen::Step { distance_after, .. } => Some(*distance_after),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        if farthest < PREFERRED_RANGE - 1 {
            last_problem = format!("attempt {attempt}: only {farthest} cell(s) away; the band is {PREFERRED_RANGE} ({seen:?})");
            continue;
        }
        proven = true;
        break;
    }
    if !proven {
        return Err(format!("no attempt showed the raydric archer keeping its distance: {last_problem}"));
    }

    // The control: a stock raydric archer next to the player does not back off.
    let control = observe(NO_PROFILE_MAP)?;
    let stray = retreat_ticks(&control);
    if !stray.is_empty() {
        return Err(format!(
            "control: the raydric archer backed off {} time(s) on {NO_PROFILE_MAP}, where it has no profile ({control:?})",
            stray.len()
        ));
    }
    Ok(())
}

/// Eddga with the F15 pilot skills: Magnum as its signature (a 2 s cast, 15 s
/// apart), and the stock teleport-on-hit removed. The skills only fire once
/// Eddga is provoked (they sit in the retaliate states), so the player hits it
/// first.
fn eddga_pilot_skills(config: &Config) -> Result<(), String> {
    // Eddga's skills fire on a rolled chance once it is provoked, and a character
    // can die before it rolls well; each attempt is a fresh Eddga, and one
    // clean showing proves it.
    let mut problems = Vec::new();
    for attempt in 1..=ATTEMPTS {
        match eddga_attempt(config) {
            Ok(()) => return Ok(()),
            Err(problem) if problem.starts_with("Eddga never cast Magnum") => problems.push(format!("attempt {attempt}: {problem}")),
            Err(other) => return Err(other),
        }
    }
    Err(problems.join(" | "))
}

fn eddga_attempt(config: &Config) -> Result<(), String> {
    let mut context = attacker_on(config, "pay_fild10", 4008)?;
    context.say("@str 90")?;
    context.say("@dex 90")?;
    context.say("@agi 80")?;
    context.say("@heal")?;
    context.pump(Duration::from_millis(200));
    let result: Result<(), String> = (|| {
        let eddga = context.spawn_monster(EDDGA.0, EDDGA.1)?;
        let mut casts: Vec<(u32, u16, u32)> = Vec::new(); // (when ms since start, skill id, cast ms), Eddga's own
        let mut others: Vec<(u32, u16, u32)> = Vec::new(); // every other caster in view, for diagnosis
        let mut seen: Vec<Seen> = Vec::new();
        let started = std::time::Instant::now();
        for swing in 0..40 {
            if swing % 2 == 0 {
                context.say("@heal")?;
                context.say("@alive")?;
            }
            context.flush();
            context.net.player_attack(eddga).map_err(|_| "disconnected")?;
            let events = context.collect_for(Duration::from_millis(600));
            for event in &events {
                if let NetworkEvent::SkillCast {
                    source_entity_id,
                    skill_id,
                    cast_ms,
                    ..
                } = event
                {
                    let row = (started.elapsed().as_millis() as u32, skill_id.0, *cast_ms);
                    if *source_entity_id == eddga {
                        casts.push(row);
                    } else {
                        others.push(row);
                    }
                }
            }
            record(&context, eddga, events, &mut seen);
            if casts.iter().any(|(_, skill, _)| *skill == SKILL_MAGNUM) && swing >= 12 {
                break;
            }
        }

        let magnums: Vec<&(u32, u16, u32)> = casts.iter().filter(|(_, skill, _)| *skill == SKILL_MAGNUM).collect();
        if magnums.is_empty() {
            let distance = context
                .entities
                .get(&eddga)
                .map(|entity| chebyshev(entity.position.tile_position(), context.position).to_string())
                .unwrap_or_else(|| "not in view".to_owned());
            return Err(format!(
                "Eddga never cast Magnum in {} ms of being fought. Our swings: {}. Eddga's casts: {casts:?}. Other casters: {others:?}. \
                 Eddga is {distance} cell(s) away; our HP {}/{}",
                started.elapsed().as_millis(),
                summary(&seen),
                context.health_points,
                context.max_health_points
            ));
        }
        for (_, _, cast_ms) in &magnums {
            if !(1500..=2500).contains(cast_ms) {
                return Err(format!("a Magnum cast bar of {cast_ms} ms; the pilot file says 2000"));
            }
        }
        for pair in magnums.windows(2) {
            if pair[1].0.saturating_sub(pair[0].0) < 14_000 {
                return Err(format!("two Magnums {} ms apart; the delay is 15000", pair[1].0 - pair[0].0));
            }
        }
        if casts.iter().any(|(_, skill, _)| *skill == SKILL_TELEPORT) {
            return Err(format!(
                "Eddga cast Teleport; the pilot file removes the stock teleport-on-hit ({casts:?})"
            ));
        }
        Ok(())
    })();
    clear_map(&mut context);
    result
}

/// Eddga below 80% HP casts Meteor Storm (3.0 s cast), and below 30% HP
/// activates its enrage skills (Powerup 349 / Speedup 332).
///
/// Eddga has 947,500 HP; @setmobhp is used to deterministically bring Eddga
/// to 700,000 HP (< 80% HP) to test Meteor Storm, and then to 200,000 HP (< 30% HP)
/// to test the escalation enrage window without killing it.
fn eddga_meteor_and_enrage(config: &Config) -> Result<(), String> {
    let mut problems = Vec::new();
    for attempt in 1..=ATTEMPTS {
        match eddga_meteor_attempt(config) {
            Ok(()) => return Ok(()),
            Err(problem) => problems.push(format!("attempt {attempt}: {problem}")),
        }
    }
    Err(problems.join(" | "))
}

fn eddga_meteor_attempt(config: &Config) -> Result<(), String> {
    let mut context = attacker_on(config, "pay_fild10", 4008)?;
    let result: Result<(), String> = (|| {
        // High DEX and AGI ensure our swings hit reliably and quickly.
        context.say("@str 90")?;
        context.say("@dex 90")?;
        context.say("@agi 80")?;
        context.pump(Duration::from_millis(300));

        let eddga = context.spawn_monster(EDDGA.0, EDDGA.1)?;
        // Bring Eddga under 80% HP (947,500 * 0.80 = 758,000) so Meteor Storm is eligible.
        context.say(&format!("@setmobhp {} 700000", eddga.0))?;
        context.pump(Duration::from_millis(200));

        let mut meteor_casts: Vec<u32> = Vec::new();
        let mut enrage_effects: Vec<u16> = Vec::new();
        let mut all_casts: Vec<(u32, u16, u32)> = Vec::new();
        let mut all_nodamage: Vec<(u32, u16)> = Vec::new();
        let mut seen: Vec<Seen> = Vec::new();
        let mut set_enrage_hp = false;
        let started = std::time::Instant::now();

        for swing in 0..40 {
            if swing % 2 == 0 {
                context.say("@heal")?;
                context.say("@alive")?;
            }

            // Once Meteor Storm has been observed, bring Eddga under 30% HP (947,500 * 0.30 = 284,250)
            // to test the escalation enrage window.
            if !meteor_casts.is_empty() && !set_enrage_hp {
                context.say(&format!("@setmobhp {} 200000", eddga.0))?;
                context.pump(Duration::from_millis(200));
                set_enrage_hp = true;
            }

            if let Some(target_pos) = context.entities.get(&eddga).map(|e| e.position.tile_position()) {
                if chebyshev(target_pos, context.position) > 1 {
                    let _ = context.walk_to(target_pos.x.saturating_sub(1), target_pos.y);
                    context.pump(Duration::from_millis(150));
                }
            }
            context.flush();
            context.net.player_attack(eddga).map_err(|_| "disconnected")?;
            let events = context.collect_for(Duration::from_millis(600));
            for event in &events {
                match event {
                    NetworkEvent::SkillCast {
                        source_entity_id,
                        skill_id,
                        cast_ms,
                        ..
                    } if *source_entity_id == eddga => {
                        all_casts.push((started.elapsed().as_millis() as u32, skill_id.0, *cast_ms));
                        if skill_id.0 == SKILL_METEOR {
                            meteor_casts.push(*cast_ms);
                        }
                    }
                    NetworkEvent::SkillEffectNoDamage {
                        source_entity_id,
                        skill_id,
                        ..
                    } if *source_entity_id == eddga => {
                        all_nodamage.push((started.elapsed().as_millis() as u32, skill_id.0));
                        if skill_id.0 == SKILL_POWERUP || skill_id.0 == SKILL_SPEEDUP {
                            enrage_effects.push(skill_id.0);
                        }
                    }
                    _ => {}
                }
            }
            record(&context, eddga, events, &mut seen);

            // Once both Meteor Storm (<80% HP) and Enrage (<30% HP) have been proven, stop early.
            if !meteor_casts.is_empty() && !enrage_effects.is_empty() {
                break;
            }
        }

        if meteor_casts.is_empty() {
            return Err(format!(
                "Eddga never cast Meteor Storm (skill {SKILL_METEOR}) in {} ms of being fought. All casts: {all_casts:?}. All nodamage: {all_nodamage:?}. Swings: {}",
                started.elapsed().as_millis(),
                summary(&seen)
            ));
        }
        for cast_ms in &meteor_casts {
            if !(2500..=3500).contains(cast_ms) {
                return Err(format!("Meteor Storm cast bar of {cast_ms} ms; pilot file says 3000 ms"));
            }
        }
        if enrage_effects.is_empty() {
            return Err(format!(
                "Eddga never activated enrage skills (Powerup {SKILL_POWERUP} / Speedup {SKILL_SPEEDUP}) in {} ms of being fought. All casts: {all_casts:?}. All nodamage: {all_nodamage:?}. Swings: {}",
                started.elapsed().as_millis(),
                summary(&seen)
            ));
        }

        Ok(())
    })();
    clear_map(&mut context);
    result
}

/// A flee: one long move, not a one-cell step, that ends farther from the
/// player.
fn flees(seen: &[Seen]) -> Vec<usize> {
    // Returns the damage the player had dealt when each flee happened.
    let mut dealt = 0;
    let mut has_hit = false;
    let mut out = Vec::new();
    for item in seen {
        match item {
            Seen::Hit(amount) => {
                dealt += amount;
                has_hit = true;
            }
            Seen::Step {
                length,
                away: _,
                distance_after,
                ..
            } if has_hit && *length >= 4 && *distance_after >= 4 => out.push(dealt),
            _ => {}
        }
    }
    out
}

/// Wear a Poring down with a level-1 character (1 to 3 damage a swing, so its
/// 50 HP last) until it flees or dies; return what was seen and the damage
/// dealt.
fn wear_down_poring(config: &Config, map: &str) -> Result<(Vec<Seen>, usize), String> {
    let mut context = attacker_at_level(config, map, 0, 1)?;
    context.say("@dex 50")?;
    context.pump(Duration::from_millis(200));
    let target = context.spawn_monster("PORING", 1002)?;
    let mut seen = Vec::new();
    let mut dealt = 0;
    for swing in 0..80 {
        if swing % 2 == 0 {
            context.say("@heal")?;
        }
        if let Some(target_pos) = context.entities.get(&target).map(|e| e.position.tile_position()) {
            if chebyshev(target_pos, context.position) > 1 {
                let _ = context.walk_to(target_pos.x.saturating_sub(1), target_pos.y);
            }
        }
        context.flush();
        context.net.player_attack(target).map_err(|_| "disconnected")?;
        let events = context.collect_for(Duration::from_millis(600));
        let before = seen.len();
        record(&context, target, events, &mut seen);
        dealt += seen[before..]
            .iter()
            .map(|item| if let Seen::Hit(amount) = item { *amount } else { 0 })
            .sum::<usize>();
        // Stop once it has fled, or once it is clearly dead (a long run with no more
        // hits).
        if !flees(&seen).is_empty() {
            break;
        }
        if dealt >= 50 {
            break;
        }
    }
    clear_map(&mut context);
    Ok((seen, dealt))
}

/// `PORING` on `prt_fild08`: Coward, HpThreshold 35, FleeDistance 4. It runs
/// only once it is down to 35% of its 50 HP, never before; and on a map with no
/// profile it never runs.
fn coward_poring(config: &Config) -> Result<(), String> {
    // Poring max HP is 50: 35% is 17.5, so the flee may start once 33 or more
    // damage is dealt.
    const FIRST_DAMAGE_THAT_CAN_TRIGGER: usize = 33;

    let (seen, dealt) = wear_down_poring(config, "prt_fild08")?;
    let fled = flees(&seen);
    if fled.is_empty() {
        return Err(format!(
            "a Poring at or below 35% HP ({dealt} damage dealt) never fled on prt_fild08: {}",
            summary(&seen)
        ));
    }
    if fled[0] < FIRST_DAMAGE_THAT_CAN_TRIGGER {
        return Err(format!(
            "the Poring fled after only {} damage; it should hold until 33 (35% of 50 HP)",
            fled[0]
        ));
    }

    // The control: the same Poring, same swings, a neighbouring field with no
    // profile.
    let (control, dealt) = wear_down_poring(config, "prt_fild07")?;
    if dealt < FIRST_DAMAGE_THAT_CAN_TRIGGER {
        return Err(format!(
            "control: only {dealt} damage dealt, so it proves nothing ({})",
            summary(&control)
        ));
    }
    if !flees(&control).is_empty() {
        return Err(format!(
            "control: a Poring fled on prt_fild07, where it has no profile ({})",
            summary(&control)
        ));
    }
    Ok(())
}

// ---- the elite population
// -----------------------------------------------------------

const ELITE_MAP: &str = "orcsdun01";
/// The script spawns on a 60 s timer, so allow a full tick and a margin.
const TIMER_MS: u64 = 60_000;

fn elites_on_map(context: &mut TestContext) -> Result<usize, String> {
    context.flush();
    context.say(&format!("@metrics spawns {ELITE_MAP} {}", ELITE_ORC_SKELETON.1))?;
    let line = wait_for_text(context, "the elite count", "[Elite] Orc Skeleton")?;
    line.split(" x ")
        .next()
        .and_then(|front| front.trim().rsplit(' ').next())
        .and_then(|count| count.parse().ok())
        .ok_or_else(|| format!("could not read the elite count from {line:?}"))
}

/// Poll until the elite count satisfies `done`, for at most `limit`.
fn wait_for_elites(context: &mut TestContext, what: &str, limit: Duration, done: impl Fn(usize) -> bool) -> Result<usize, String> {
    let deadline = std::time::Instant::now() + limit;
    let mut last;
    loop {
        last = elites_on_map(context)?;
        if done(last) {
            return Ok(last);
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!("{what}: gave up after {} s; the count was {last}", limit.as_secs()));
        }
        context.pump(Duration::from_secs(5));
    }
}

/// F15 part 2: `f15_elites.txt` keeps exactly two `[Elite] Orc Skeleton` on
/// `orcsdun01` (its timer tops the population up to two, never past it).
fn elite_population(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    say_expect(&mut context, "@metrics status", "Collection is")?;
    // Make sure the rollback switch is on (an earlier scenario may have toggled
    // it).
    say_expect(&mut context, "@setbattleflag mob_pilot_version 1", "as requested")?;

    let limit = Duration::from_millis(TIMER_MS + 40_000);
    wait_for_elites(&mut context, "the elites to appear", limit, |count| count == 2)?;

    // Another full timer tick must not add more.
    context.pump(Duration::from_millis(TIMER_MS + 8_000));
    let after = elites_on_map(&mut context)?;
    if after != 2 {
        return Err(format!(
            "the elite population drifted to {after} after another timer tick; it should hold at 2"
        ));
    }
    Ok(())
}

/// F15's rollback switch: with `mob_pilot_version` 0 the elites are removed on
/// the next tick and none respawn; back at 1 they return.
fn elite_rollback_switch(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let result: Result<(), String> = (|| {
        say_expect(&mut context, "@setbattleflag mob_pilot_version 1", "as requested")?;
        let limit = Duration::from_millis(TIMER_MS + 40_000);
        wait_for_elites(
            &mut context,
            "the elites to exist before the switch is thrown",
            limit,
            |count| count == 2,
        )?;

        say_expect(&mut context, "@setbattleflag mob_pilot_version 0", "as requested")?;
        wait_for_elites(&mut context, "the elites to be removed with the switch off", limit, |count| {
            count == 0
        })?;
        // And they stay gone through another tick.
        context.pump(Duration::from_millis(TIMER_MS + 8_000));
        let still = elites_on_map(&mut context)?;
        if still != 0 {
            return Err(format!("{still} elite(s) respawned while mob_pilot_version was 0"));
        }

        say_expect(&mut context, "@setbattleflag mob_pilot_version 1", "as requested")?;
        wait_for_elites(
            &mut context,
            "the elites to return when the switch is back on",
            limit,
            |count| count == 2,
        )?;
        Ok(())
    })();
    // Leave the server as it was found.
    let _ = context.say("@setbattleflag mob_pilot_version 1");
    context.pump(Duration::from_millis(200));
    result
}
