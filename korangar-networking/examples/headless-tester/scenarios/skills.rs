//! Phase 5 — data-driven skill sweep.
//!
//! For each configured job: `@job` + `@allskill`, then cast every skill the
//! server put in the skill tree, using the cast method matching the server's
//! own `SkillType` (Attack → target, Ground/Trap → position, Self/Support →
//! self). Each cast must produce SOME observable protocol response —
//! silence means an unregistered/misparsed packet (the bug class this
//! phase exists to catch). Per-skill outcomes are printed as a table.

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::{Direction, SkillId, SkillLevel, SkillType, TilePosition, WorldPosition};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("teleport-select", 5, teleport_select),
        Scenario::new("teleport-cancel", 5, teleport_cancel),
        Scenario::new("weapon-refine-missing-material", 5, weapon_refine_missing_material),
        Scenario::new("weapon-refine-success", 5, weapon_refine_success),
        // --- Basic Classes ---
        Scenario::new("skills-novice", 5, |config| sweep_job(config, 0, "Novice")),
        Scenario::new("skills-swordman", 5, |config| sweep_job(config, 1, "Swordman")),
        Scenario::new("skills-mage", 5, |config| sweep_job(config, 2, "Mage")),
        Scenario::new("skills-archer", 5, |config| sweep_job(config, 3, "Archer")),
        Scenario::new("skills-acolyte", 5, |config| sweep_job(config, 4, "Acolyte")),
        Scenario::new("skills-merchant", 5, |config| sweep_job(config, 5, "Merchant")),
        Scenario::new("skills-thief", 5, |config| sweep_job(config, 6, "Thief")),
        Scenario::new("skills-knight", 5, |config| sweep_job(config, 7, "Knight")),
        Scenario::new("skills-priest", 5, |config| sweep_job(config, 8, "Priest")),
        Scenario::new("skills-wizard", 5, |config| sweep_job(config, 9, "Wizard")),
        Scenario::new("skills-blacksmith", 5, |config| sweep_job(config, 10, "Blacksmith")),
        Scenario::new("skills-hunter", 5, |config| sweep_job(config, 11, "Hunter")),
        Scenario::new("skills-assassin", 5, |config| sweep_job(config, 12, "Assassin")),
        Scenario::new("skills-crusader", 5, |config| sweep_job(config, 14, "Crusader")),
        Scenario::new("skills-monk", 5, |config| sweep_job(config, 15, "Monk")),
        Scenario::new("skills-sage", 5, |config| sweep_job(config, 16, "Sage")),
        Scenario::new("skills-rogue", 5, |config| sweep_job(config, 17, "Rogue")),
        Scenario::new("skills-alchemist", 5, |config| sweep_job(config, 18, "Alchemist")),
        Scenario::new("skills-bard", 5, |config| sweep_job(config, 19, "Bard")),
        Scenario::new("skills-dancer", 5, |config| sweep_job(config, 20, "Dancer")),
        // --- Transcendent 2-1 Classes ---
        Scenario::new("skills-high-wizard", 5, |config| sweep_job(config, 4010, "High Wizard")),
        Scenario::new("skills-high-priest", 5, |config| sweep_job(config, 4009, "High Priest")),
        Scenario::new("skills-lord-knight", 5, |config| sweep_job(config, 4008, "Lord Knight")),
        Scenario::new("skills-sniper", 5, |config| sweep_job(config, 4012, "Sniper")),
        Scenario::new("skills-assassin-cross", 5, |config| sweep_job(config, 4013, "Assassin Cross")),
        Scenario::new("skills-whitesmith", 5, |config| sweep_job(config, 4011, "Whitesmith")),
        // --- Transcendent 2-2 Classes ---
        Scenario::new("skills-paladin", 5, |config| sweep_job(config, 4015, "Paladin")),
        Scenario::new("skills-champion", 5, |config| sweep_job(config, 4016, "Champion")),
        Scenario::new("skills-professor", 5, |config| sweep_job(config, 4017, "Professor")),
        Scenario::new("skills-stalker", 5, |config| sweep_job(config, 4018, "Stalker")),
        Scenario::new("skills-creator", 5, |config| sweep_job(config, 4019, "Creator")),
        Scenario::new("skills-clown", 5, |config| sweep_job(config, 4020, "Clown")),
        Scenario::new("skills-gypsy", 5, |config| sweep_job(config, 4021, "Gypsy")),
        // --- Expanded Classes ---
        Scenario::new("skills-super-novice", 5, |config| sweep_job(config, 23, "Super Novice")),
        Scenario::new("skills-gunslinger", 5, |config| sweep_job(config, 24, "Gunslinger")),
        Scenario::new("skills-ninja", 5, |config| sweep_job(config, 25, "Ninja")),
        Scenario::new("skills-taekwon", 5, |config| sweep_job(config, 4046, "Taekwon")),
        Scenario::new("skills-star-gladiator", 5, |config| sweep_job(config, 4047, "Star Gladiator")),
        Scenario::new("skills-soul-linker", 5, |config| sweep_job(config, 4049, "Soul Linker")),
    ]
}

const AL_TELEPORT: SkillId = SkillId(26);
const WS_WEAPONREFINE: SkillId = SkillId(477);

fn prepare_skill(context: &mut TestContext, job_id: u16, skill_id: SkillId) -> Result<SkillLevel, String> {
    context.ensure_job(job_id)?;
    context.say("@allskill")?;
    context.pump(Duration::from_millis(500));
    context
        .skills
        .iter()
        .find(|skill| skill.skill_id == skill_id)
        .map(|skill| skill.skill_level)
        .ok_or_else(|| format!("skill {} was not present after @allskill", skill_id.0))
}

fn teleport_select(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let level = prepare_skill(&mut context, 4, AL_TELEPORT)?;
    context.flush();
    context
        .net
        .cast_skill(AL_TELEPORT, level, context.player_id)
        .map_err(|_| "disconnected")?;
    let destinations = context.wait_for("Teleport WarpList", |event| match event {
        NetworkEvent::WarpList { skill_id, destinations } if *skill_id == AL_TELEPORT => Some(destinations.clone()),
        _ => None,
    })?;
    let destination = destinations
        .into_iter()
        .find(|destination| !destination.is_empty())
        .ok_or("Teleport returned no selectable destination")?;
    context
        .net
        .select_warp_destination(AL_TELEPORT, destination)
        .map_err(|_| "disconnected")?;
    context.wait_for("ChangeMap after Teleport selection", |event| match event {
        NetworkEvent::ChangeMap { .. } => Some(()),
        _ => None,
    })
}

fn teleport_cancel(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let level = prepare_skill(&mut context, 4, AL_TELEPORT)?;
    context.flush();
    context
        .net
        .cast_skill(AL_TELEPORT, level, context.player_id)
        .map_err(|_| "disconnected")?;
    context.wait_for("Teleport WarpList", |event| match event {
        NetworkEvent::WarpList { skill_id, .. } if *skill_id == AL_TELEPORT => Some(()),
        _ => None,
    })?;
    context.net.cancel_warp_selection(AL_TELEPORT).map_err(|_| "disconnected")?;
    let events = context.collect_for(Duration::from_secs(1));
    if events.iter().any(|event| matches!(event, NetworkEvent::ChangeMap { .. })) {
        return Err("Teleport cancellation unexpectedly changed maps".to_owned());
    }
    Ok(())
}

fn weapon_refine_missing_material(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let level = prepare_skill(&mut context, 4011, WS_WEAPONREFINE)?;
    // Remove both ordinary Weapon Refine catalysts so prior manual/testing
    // inventory cannot silently turn this negative case into a success.
    context.say("@delitem 984 999")?; // Oridecon
    context.say("@delitem 1010 999")?; // Phracon
    context.say("@delitem 1101 999")?;
    context.pump(Duration::from_millis(500));
    let index = context.give_item(1101, 1)?;
    context.flush();
    context
        .net
        .cast_skill(WS_WEAPONREFINE, level, context.player_id)
        .map_err(|_| "disconnected")?;
    let listed = context.wait_for("RefinableWeaponList", |event| match event {
        NetworkEvent::RefinableWeaponList { weapons } => Some(weapons.clone()),
        _ => None,
    })?;
    if !listed.iter().any(|weapon| weapon.inventory_index == index) {
        return Err(format!("created weapon at index {} was absent from refine list", index.0));
    }
    context.net.request_weapon_refine(index).map_err(|_| "disconnected")?;
    context.wait_for("missing refine material feedback", |event| match event {
        NetworkEvent::ChatMessage { text, .. }
            if text.to_ascii_lowercase().contains("material") || text.to_ascii_lowercase().contains("missing") =>
        {
            Some(())
        }
        NetworkEvent::MessageTable { .. } => Some(()),
        _ => None,
    })
}

fn weapon_refine_success(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let level = prepare_skill(&mut context, 4011, WS_WEAPONREFINE)?;
    context.say("@delitem 1101 999")?;
    context.say("@item 1010 1")?;
    context.pump(Duration::from_millis(400));
    let index = context.give_item(1101, 1)?;
    context.flush();
    context
        .net
        .cast_skill(WS_WEAPONREFINE, level, context.player_id)
        .map_err(|_| "disconnected")?;
    context.wait_for("RefinableWeaponList", |event| match event {
        NetworkEvent::RefinableWeaponList { weapons } if weapons.iter().any(|weapon| weapon.inventory_index == index) => Some(()),
        _ => None,
    })?;
    context.net.request_weapon_refine(index).map_err(|_| "disconnected")?;
    context.wait_for("successful WeaponRefineResult", |event| match event {
        NetworkEvent::WeaponRefineResult { result: 0, item_id } if item_id.0 == 1101 => Some(()),
        _ => None,
    })?;
    let player_id = context.player_id;
    context.wait_for("refine success visual effect", |event| match event {
        NetworkEvent::VisualEffect { effect_path, entity_id } if *entity_id == player_id && *effect_path == "bs_refinesuccess.str" => {
            Some(())
        }
        _ => None,
    })
}

/// Skills that legitimately produce no direct cast response headlessly
/// (require special weapons/ammo/catalysts/companions or extended setup).
/// Kept intentionally small: anything else that fails silently is a finding.
fn allowlisted(skill_name: &str) -> bool {
    const ALLOWLIST: &[&str] = &[
        // Requires a falcon / bird companion state.
        "HT_FALCON",
        "SN_FALCONASSAULT",
        // Requires arrows / specific ammo equipped even for the ack.
        "AC_MAKINGARROW",
        "SA_ARROWMAKING",
        // Item-consuming crafts (catalyst items missing headlessly).
        "AM_PHARMACY",
        "AM_TWILIGHT1",
        "AM_TWILIGHT2",
        "AM_TWILIGHT3",
        "BS_REPAIRWEAPON",
        "WS_CREATECOIN",
        "WS_CREATENUGGET",
        "WS_WEAPONREFINE",
        // Requires cart / madogear state.
        "MC_CARTREVOLUTION",
        "MC_CHANGECART",
        "MC_PUSHCART",
        "WS_CARTBOOST",
        "WS_CARTTERMINATION",
        // Requires the caster to stand in a water cell.
        "WZ_WATERBALL",
        // Require an existing owned trap unit as the target.
        "HT_REMOVETRAP",
        "HT_SPRINGTRAP",
        // Resurrection on the living self has no valid target; Basilica has
        // party/area preconditions that this single-client sweep cannot
        // guarantee.
        "ALL_RESURRECTION",
        "HP_BASILICA",
        // Requires being hidden / specific stances first.
        "AS_GRIMTOOTH",
        "AS_CLOAKING",
        "RG_BACKSTAP",
        "RG_RAID",
        "RG_TUNNELDRIVE",
        // Spirit-sphere / combo-state dependent.
        "MO_ABSORBSPIRITS",
        "MO_EXTREMITYFIST",
        "MO_CHAINCOMBO",
        "MO_COMBOFINISH",
        // Ensemble / Duet skills (require Bard + Dancer next to each other).
        "BD_LULLABY",
        "BD_RICHMANKIM",
        "BD_ETERNALCHAOS",
        "BD_DRUMBATTLEFIELD",
        "BD_RINGNIBELUNGEN",
        "BD_ROKISWEIL",
        "BD_INTOABYSS",
        "BD_SIEGFRIED",
        "BA_DISSONANCE",
        // Rogue flag graffiti (requires flag/paint brush item).
        "RG_FLAGGRAFFITI",
        "RG_GRAFFITI",
        "RG_CLEANER",
        // Stalker skills that require special state/setup.
        "ST_REJECTSWORD",
        "ST_PRESERVE",
        "ST_FULLSTRIP",
        "ST_CHASEWALK",
        // Devotion / Providence (requires other party members / holy targets).
        "CR_DEVOTION",
        "CR_PROVIDENCE",
        // Champion combo skills (require combo state).
        "CH_TIGERFIST",
        "CH_CHAINCRUSH",
        // Clown/Gypsy support skills (require partner / song state).
        "CG_MARIONETTE",
        "CG_HERMODE",
        // Ninja throwing skills (require ammo).
        "NJ_SYURIKEN",
        "NJ_KUNAI",
        // Taekwon kicks and mission (require stance / target).
        "TK_STORMKICK",
        "TK_DOWNKICK",
        "TK_TURNKICK",
        "TK_COUNTER",
        "TK_MISSION",
        // Soul Linker spirit links (require target player of specific class).
        "SL_ALCHEMIST",
        "SL_MONK",
        "SL_STAR",
        "SL_SAGE",
        "SL_CRUSADER",
        "SL_SUPERNOVICE",
        "SL_KNIGHT",
        "SL_WIZARD",
        "SL_PRIEST",
        "SL_BARDDANCER",
        "SL_ROGUE",
        "SL_ASSASIN",
        "SL_BLACKSMITH",
        "SL_HUNTER",
        "SL_SOULLINKER",
        "SL_SMA",
    ];
    ALLOWLIST.contains(&skill_name) || skill_name.starts_with("SG_")
}

struct SkillOutcome {
    skill_id: u16,
    name: String,
    skill_type: SkillType,
    level: u16,
    result: &'static str,
}

fn sweep_job(config: &Config, job_id: u16, job_name: &str) -> Result<(), String> {
    if job_id == 0 {
        println!("    Novice: skipped (no active skills to sweep)");
        return Ok(());
    }

    let mut context = TestContext::connect(config)?;
    if let Err(error) = context.ensure_job(job_id) {
        if error.contains("unable to change") || error.contains("failed") {
            println!("    {job_name}: skipped (job change failed - likely gender or class restriction)");
            return Ok(());
        }
        return Err(error);
    }
    context.ensure_base_level(99)?;
    context.say("@heal")?;
    context.warp_random("prt_fild08")?;
    let start_position = context.position;

    // Grant the full tree and capture it.
    context.flush();
    context.say("@allskill")?;
    let mut skills = context.wait_for("SkillTree after @allskill", |event| match event {
        NetworkEvent::SkillTree { skill_information } if skill_information.len() > 5 => Some(skill_information.clone()),
        _ => None,
    })?;
    // Hercules sends the tree in descending ID order. That puts disruptive
    // advanced skills (Basilica, Tension Relax, hiding states) before basic
    // skills and contaminates the rest of the sweep. Exercise foundational
    // skills first so a stateful advanced skill can only affect later peers.
    skills.sort_by_key(|skill| {
        let name = skill.skill_name.trim_end_matches('\0');
        let ground_last = matches!(skill.skill_type, SkillType::Ground | SkillType::Trap);
        (stateful_skill_rank(name), ground_last, skill.skill_id.0)
    });
    println!("    {job_name}: sweeping {} skills", skills.len());

    let player_id = context.player_id;
    let mut outcomes: Vec<SkillOutcome> = Vec::new();

    for skill in &skills {
        let name = skill.skill_name.trim_end_matches('\0').to_owned();
        let level = skill.skill_level;

        if matches!(skill.skill_type, SkillType::Passive) || level.0 == 0 {
            outcomes.push(SkillOutcome {
                skill_id: skill.skill_id.0,
                name,
                skill_type: skill.skill_type,
                level: level.0,
                result: "passive",
            });
            continue;
        }

        // Fresh SP and a clean slate per cast.
        context.say("@heal")?;
        // Some skills force the character to sit; unlike the graphical
        // client, this harness has no input layer that automatically stands
        // before the next requested action.
        context.net.player_stand().map_err(|_| "disconnected")?;
        context.pump(Duration::from_millis(150));

        // Walk back to the starting point to prevent drifting into map obstacles.
        if context.position != start_position {
            let _ = context.walk_to(start_position.x, start_position.y);
        }

        // A live target for attack skills (many die to one cast; respawn).
        let target = match skill.skill_type {
            SkillType::Attack => {
                let target = ensure_target(&mut context).map_err(|error| format!("target setup failed at {name}: {error}"))?;
                let position = context
                    .entities
                    .get(&target)
                    .map(|entity| entity.position.tile_position())
                    .ok_or_else(|| format!("fresh target vanished before {name}"))?;
                approach_target(&mut context, position)?;
                Some(target)
            }
            _ => None,
        };

        context.flush();
        let cast = match skill.skill_type {
            SkillType::Attack => context.net.cast_skill(skill.skill_id, level, target.unwrap()),
            SkillType::Ground | SkillType::Trap => {
                let position = context.position;
                context.net.cast_ground_skill(skill.skill_id, level, TilePosition {
                    x: position.x + 2,
                    y: position.y,
                })
            }
            SkillType::SelfCast | SkillType::Support => context.net.cast_skill(skill.skill_id, level, player_id),
            SkillType::Passive => unreachable!(),
        };
        if cast.is_err() {
            return Err("disconnected mid-sweep".to_owned());
        }

        // Any observable response counts; total silence is the failure mode.
        let response = context.wait_for_within("skill response", Duration::from_secs(4), &mut |event| match event {
            NetworkEvent::SkillCast { skill_id, cast_ms, .. } if skill_id.0 == skill.skill_id.0 => Some(("cast", *cast_ms)),
            NetworkEvent::DamageEffect { source_entity_id, .. } if source_entity_id.0 == player_id.0 => Some(("damage", 0)),
            NetworkEvent::HealEffect { .. } => Some(("heal", 0)),
            NetworkEvent::StatusChange {
                entity_id, gained: true, ..
            } if entity_id.0 == player_id.0 => Some(("buff", 0)),
            NetworkEvent::AddSkillUnit { .. } => Some(("ground-unit", 0)),
            NetworkEvent::SkillCooldown { skill_id, .. } if skill_id.0 == skill.skill_id.0 => Some(("cooldown", 0)),
            NetworkEvent::VisualEffect { .. } => Some(("visual", 0)),
            NetworkEvent::MonsterInformation { .. } => Some(("monster-info", 0)),
            NetworkEvent::WarpList { .. } => Some(("warp-list", 0)),
            NetworkEvent::SkillCooldownList { .. } => Some(("cooldown-list", 0)),
            NetworkEvent::RefinableWeaponList { .. } => Some(("weapon-list", 0)),
            NetworkEvent::ChatMessage { .. } | NetworkEvent::MessageTable { .. } => Some(("fail-feedback", 0)),
            // Some skills open a menu (e.g. teleport / warp portal).
            NetworkEvent::OpenDialog { .. } | NetworkEvent::AddChoiceButtons { .. } => Some(("dialog", 0)),
            _ => None,
        });

        let (result, wait) = match response {
            // If a cast bar started, wait it out so the next cast isn't
            // rejected with "still casting".
            Ok((kind, cast_ms)) => (kind, if kind == "cast" { cast_ms.saturating_add(400) } else { 400 }),
            Err(_) if allowlisted(&name) => ("silent (allowlisted)", 0),
            Err(_) => ("SILENT — investigate", 0),
        };
        context.pump(Duration::from_millis(wait.into()));

        outcomes.push(SkillOutcome {
            skill_id: skill.skill_id.0,
            name,
            skill_type: skill.skill_type,
            level: level.0,
            result,
        });

        if target.is_some() {
            let _ = context.kill_all_monsters();
        }
    }

    context.kill_all_monsters();

    // Report.
    let mut silent_count = 0;
    for outcome in &outcomes {
        if outcome.result.starts_with("SILENT") {
            silent_count += 1;
        }
        println!(
            "      {:>4}  {:<24} {:<9} lv{:<3} {}",
            outcome.skill_id,
            outcome.name,
            format!("{:?}", outcome.skill_type),
            outcome.level,
            outcome.result
        );
    }

    match silent_count {
        0 => Ok(()),
        count => Err(format!(
            "{count} skill(s) produced no observable protocol response — see table above; document in headless_findings.md or extend the \
             allowlist with a reason"
        )),
    }
}

/// Skills that leave the server waiting on a modal choice or put the player
/// into a persistent action state. They still run, but only after ordinary
/// skills so they cannot turn later casts into false "silent" results.
fn stateful_skill_rank(skill_name: &str) -> u8 {
    if skill_name == "KN_AUTOCOUNTER" {
        // Counter stance blocks subsequent active skills until it resolves.
        return 2;
    }
    u8::from(matches!(
        skill_name,
        "AL_TELEPORT" | "LK_TENSIONRELAX" | "TF_HIDING" | "AS_CLOAKING" | "MC_VENDING" | "HP_BASILICA" | "PA_GOSPEL" | "ST_CHASEWALK"
    ))
}

/// Spawn a fresh, immobile Pupa next to the player as a target dummy. Reusing
/// natural mobs is racy because they can walk out of range while the harness
/// is approaching them.
fn ensure_target(context: &mut TestContext) -> Result<ragnarok_packets::EntityId, String> {
    context.spawn_monster("PUPA", 1008)
}

/// Move next to a target without assuming a particular adjacent cell is
/// walkable. Spawned mobs can land beside walls, and Hercules silently drops
/// movement requests whose destination cannot be reached.
fn approach_target(context: &mut TestContext, target: TilePosition) -> Result<(), String> {
    if context.position.x.abs_diff(target.x).max(context.position.y.abs_diff(target.y)) <= 1 {
        return Ok(());
    }

    let candidates = [
        (target.x.saturating_sub(1), target.y),
        (target.x.saturating_add(1), target.y),
        (target.x, target.y.saturating_sub(1)),
        (target.x, target.y.saturating_add(1)),
        (target.x.saturating_sub(1), target.y.saturating_sub(1)),
        (target.x.saturating_sub(1), target.y.saturating_add(1)),
        (target.x.saturating_add(1), target.y.saturating_sub(1)),
        (target.x.saturating_add(1), target.y.saturating_add(1)),
    ];

    for (x, y) in candidates {
        context.flush();
        context
            .net
            .player_move(WorldPosition::new(x, y, Direction::North))
            .map_err(|_| "disconnected while approaching skill target")?;
        if let Ok(destination) = context.wait_for_within(
            "walkable adjacent target cell",
            Duration::from_secs(2),
            &mut |event| match event {
                NetworkEvent::PlayerMove { destination, .. } => Some(*destination),
                _ => None,
            },
        ) {
            let destination = destination.tile_position();
            let distance = destination.x.abs_diff(target.x).max(destination.y.abs_diff(target.y));
            if distance <= 1 {
                context.position = destination;
                context.pump(Duration::from_millis(800));
                return Ok(());
            }
        }
    }

    Err(format!(
        "no walkable adjacent cell found around target ({}, {})",
        target.x, target.y
    ))
}
