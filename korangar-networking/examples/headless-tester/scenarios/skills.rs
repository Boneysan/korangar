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
use crate::scenarios::{Scenario, skipped};

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("teleport-select", 5, teleport_select),
        Scenario::new("teleport-cancel", 5, teleport_cancel),
        Scenario::new("skill-fail-rejection", 5, skill_fail_rejection),
        Scenario::new("weapon-refine-missing-material", 5, weapon_refine_missing_material),
        Scenario::new("weapon-refine-success", 5, weapon_refine_success),
        Scenario::new("weapon-refine-cancel", 5, weapon_refine_cancel),
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
const MG_FIREBOLT: SkillId = SkillId(19);
const WS_WEAPONREFINE: SkillId = SkillId(477);

/// Trap (item 1065) — consumed by every trap-placing skill. Without a stock of
/// these the trap sweeps can only ever assert that the refusal was reported,
/// never that a trap was placed.
const TRAP_ITEM: u32 = 1065;

/// Skills that *place* a trap: HT_SKIDTRAP..HT_CLAYMORETRAP plus HT_TALKIEBOX.
///
/// **Not** `SkillType::Trap`, which is the wire type for skills that *target an
/// existing* trap — only HT_REMOVETRAP and HT_SPRINGTRAP carry it. Every skill
/// that lays a trap is typed `Ground`, exactly like Storm Gust, so the type
/// alone cannot distinguish them and the ids have to be listed.
const TRAP_PLACING_SKILLS: &[u16] = &[115, 116, 117, 118, 119, 120, 121, 122, 123, 125];

/// Force a skill failure (`ZC_ACK_TOUSESKILL` / 0x0110) and assert the shared
/// stack promotes it to a rejection `ChatMessage` (M1-p0 rejection-messages row).
///
/// This is the only scenario in the suite that *lowers* a shared resource, so it
/// is the only one that has to put it back — see the restore note below.
fn skill_fail_rejection(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let result = skill_fail_rejection_body(&mut context);

    // ALWAYS restore SP, including on the failure path. All 114 scenarios share
    // one character, and draining SP to zero is exactly the kind of "exotic"
    // state the shared-state rule in `observer.rs` says to undo on the way out.
    //
    // Leaving it at zero silently broke `weapon-refine-missing-material`, which
    // runs immediately after this one in natural order: `WS_WEAPONREFINE` costs
    // 5 SP, so the cast was rejected, the refine list never arrived, and the
    // scenario timed out. It looked like a flaky refine bug for long enough to
    // be logged as one, and `ensure_job` does not heal, so the drain survived
    // the job change to Whitesmith. Proven by running this scenario and then
    // that one back to back.
    let _ = context.say("@heal");
    context.pump(Duration::from_millis(300));

    result
}

fn skill_fail_rejection_body(context: &mut TestContext) -> Result<(), String> {
    // Mage Fire Bolt always costs SP; drain SP then cast at self.
    let level = prepare_skill(context, 2, MG_FIREBOLT)?;
    // `@heal <hp> <sp>`: negative SP damages current SP (see atcommand.c).
    context.say("@heal 0 -999999")?;
    context.pump(Duration::from_millis(400));
    context.flush();

    let player_id = context.player_id;
    context
        .net
        .cast_skill(MG_FIREBOLT, level, player_id)
        .map_err(|_| "disconnected mid skill-fail cast".to_owned())?;

    context.wait_for("skill-fail rejection ChatMessage", |event| match event {
        NetworkEvent::ChatMessage { text, .. }
            if text.to_ascii_lowercase().contains("not enough sp")
                || text.to_ascii_lowercase().contains("skill failed")
                || text.to_ascii_lowercase().contains("skill level") =>
        {
            Some(())
        }
        // ZC_MSG path is also acceptable if the server routes this fail that way.
        NetworkEvent::MessageTable { .. } => Some(()),
        _ => None,
    })
}

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
        NetworkEvent::WeaponRefineResult { result, .. } if *result != 0 => Some(()),
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

    // Whitesmith's Weapon Refine success rate depends on job level and DEX/LUK
    // stats. Boost these to maximize the base success rate.
    context.say("@jlevel 70")?;
    context.say("@stat dex 99")?;
    context.say("@stat luk 99")?;
    context.pump(Duration::from_millis(400));

    let mut success = false;
    for attempt in 1..=5 {
        context.say("@item 1010 1")?; // Orideocon
        context.pump(Duration::from_millis(150));
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

        let result = context.wait_for_within("WeaponRefineResult", Duration::from_secs(5), &mut |event| match event {
            NetworkEvent::WeaponRefineResult { result, item_id } if item_id.0 == 1101 => Some(*result),
            _ => None,
        });

        match result {
            Ok(0) => {
                success = true;
                break;
            }
            Ok(err_code) => {
                println!("      Refinement failed (attempt {attempt}/5, code {err_code}), retrying...");
                // Clean up any remaining item slots just in case
                context.say("@delitem 1101 999")?;
                context.pump(Duration::from_millis(200));
            }
            Err(e) => {
                return Err(format!("refine timed out waiting for WeaponRefineResult: {e}"));
            }
        }
    }

    if !success {
        return Err("failed to refine weapon after 5 attempts".to_owned());
    }

    let player_id = context.player_id;
    context.wait_for("refine success visual effect", |event| match event {
        NetworkEvent::VisualEffect { effect_path, entity_id } if *entity_id == player_id && *effect_path == "bs_refinesuccess.str" => {
            Some(())
        }
        _ => None,
    })
}

/// Cancelling the refine selection must produce no result and no inventory
/// change, and must clear the server's pending menu state so the skill can be
/// cast again immediately.
fn weapon_refine_cancel(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let level = prepare_skill(&mut context, 4011, WS_WEAPONREFINE)?;
    context.say("@delitem 1101 999")?;
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

    context.flush();
    context.net.cancel_weapon_refine().map_err(|_| "disconnected")?;
    let events = context.collect_for(Duration::from_secs(1));
    for event in &events {
        match event {
            NetworkEvent::WeaponRefineResult { .. } => {
                return Err("refine cancellation produced a refine result".to_owned());
            }
            NetworkEvent::InventoryItemRemoved { index: removed, .. } if *removed == index => {
                return Err("refine cancellation consumed the weapon".to_owned());
            }
            _ => {}
        }
    }

    // The pending menu state must be cleared: a second cast reopens the list.
    context.flush();
    context
        .net
        .cast_skill(WS_WEAPONREFINE, level, context.player_id)
        .map_err(|_| "disconnected")?;
    context.wait_for("RefinableWeaponList after cancel", |event| match event {
        NetworkEvent::RefinableWeaponList { weapons } if weapons.iter().any(|weapon| weapon.inventory_index == index) => Some(()),
        _ => None,
    })?;
    context.net.cancel_weapon_refine().map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(300));

    context.say("@delitem 1101 999")?;
    context.pump(Duration::from_millis(200));
    Ok(())
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
        "AC_DOUBLE",
        // Item-consuming crafts (catalyst items missing headlessly).
        "AM_PHARMACY",
        "AM_TWILIGHT1",
        "AM_TWILIGHT2",
        "AM_TWILIGHT3",
        "BS_REPAIRWEAPON",
        "WS_CREATECOIN",
        "WS_CREATENUGGET",
        "WS_WEAPONREFINE",
        "WS_OVERTHRUSTMAX",
        "BS_HAMMERFALL",
        // Requires cart / madogear state.
        "MC_CARTREVOLUTION",
        "MC_CHANGECART",
        "MC_PUSHCART",
        "MC_VENDING",
        "WS_CARTBOOST",
        "WS_CARTTERMINATION",
        // Requires the caster to stand in a water cell.
        "WZ_WATERBALL",
        // Sage's casting state / passive interference causes these to be silent on Sage.
        "MG_NAPALMBEAT",
        "MG_SOULSTRIKE",
        "MG_COLDBOLT",
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
        //
        // BD_ETERNALCHAOS and BD_ROKISWEIL used to be listed here too, with that
        // same reason, which was wrong: they are disabled by the map zone
        // (`db/re/map_zone_db.conf`, "Normal"), and the refusal arrives as
        // ZC_NOTIFY_MAPINFO (0x0189). That packet was unmodeled, so the refusal
        // was silent and the allowlist quietly absorbed it. Now that 0x0189 is
        // handled both report fail-feedback, so the entries are removed rather
        // than left to mask the next genuine silence.
        "BD_LULLABY",
        "BD_RICHMANKIM",
        "BD_DRUMBATTLEFIELD",
        "BD_RINGNIBELUNGEN",
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

/// Jobs only a female character can hold: Dancer and Gypsy.
///
/// Hercules does not *refuse* a sex-mismatched job change, which is why these
/// needed routing rather than a skip. `pc_jobchange` round-trips the request
/// through the character's sex (`pc_mapid2jobid`, `pc.c:6465` —
/// `MAPID_CLOWNGYPSY: return sex ? JOB_CLOWN : JOB_GYPSY`), so asking a male
/// character for Gypsy silently yields Clown and the server reports "Your job
/// has been changed." There is no failure message to detect: the old skip guard
/// below matched on `"unable to change"`/`"failed"` and could never fire, so
/// these two timed out waiting for a `ChangeJob` that was never coming.
const FEMALE_ONLY_JOBS: &[u16] = &[20, 4021];

/// Jobs only a male character can hold: Bard and Clown. Same mechanism as
/// [`FEMALE_ONLY_JOBS`], opposite direction — kept explicit so the pairing is
/// visible and a future third-job addition (Minstrel/Wanderer) has a home.
const MALE_ONLY_JOBS: &[u16] = &[19, 4020];

fn sweep_job(config: &Config, job_id: u16, job_name: &str) -> Result<(), String> {
    if job_id == 0 {
        // Expected, not a gap to close: the Novice tree is passive apart from
        // actives that are **quest-gated** (First Aid / Trick Dead), so
        // `@allskill` cannot hand them over and there is nothing to cast. This
        // is the one legitimately permanent skip in the suite — which is why a
        // skip must not fail the gate, only be counted separately.
        return skipped("Novice has no castable skills — its actives are quest-gated");
    }

    // Route each sex-locked job to a character that can actually hold it. The
    // primary test character is male; the partner (`HeadlessTwo`) is female and
    // also GM group 99, so it can drive `@job`/`@allskill`/`@blevel`/`@warp`.
    // Everything unlocked stays on the primary, as before.
    let mut context = if FEMALE_ONLY_JOBS.contains(&job_id) {
        println!("    {job_name}: female-only job — sweeping on the partner character");
        TestContext::connect_partner(config)?
    } else {
        TestContext::connect(config)?
    };

    if let Err(error) = context.ensure_job(job_id) {
        if error.contains("unable to change") || error.contains("failed") {
            return skipped(format!("{job_name}: job change refused (gender or class restriction)"));
        }
        // A timeout here on a sex-locked job means the silent remap above, not a
        // protocol fault — say so, rather than leaving a bare 15s timeout.
        if FEMALE_ONLY_JOBS.contains(&job_id) || MALE_ONLY_JOBS.contains(&job_id) {
            return Err(format!(
                "{error} — {job_name} is sex-locked and the character it ran on is the wrong sex, so Hercules silently \
                 remapped the job instead of refusing it"
            ));
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

    // Stock the Trap items the Hunter/Sniper trap skills consume, so they get
    // *placed* rather than merely refused.
    //
    // Without this the sweep never exercised a trap at all: every one of the
    // seven reported only that the refusal was delivered. That was invisible
    // while the shared character happened to be carrying traps left over from
    // earlier runs — they succeeded then, by luck — and it matters because
    // Hunter traps are the one skill family with real prop rendering behind
    // them (`trap_model_file()` / `TRAP_MODEL_FILES`, phase E2) and had no
    // automated coverage of a trap actually landing.
    //
    // Granted per sweep and removed again below: ammunition-style leftovers are
    // what pushed the shared character over its weight limit and broke every
    // item-dependent scenario at once.
    let needs_traps = skills.iter().any(|skill| TRAP_PLACING_SKILLS.contains(&skill.skill_id.0));
    if needs_traps {
        let _ = context.say(&format!("@item {TRAP_ITEM} 50"));
        context.pump(Duration::from_millis(400));
    }

    let player_id = context.player_id;
    let mut outcomes: Vec<SkillOutcome> = Vec::new();
    let mut ground_cast_index: usize = 0;

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
                // Give each ground cast its own cell.
                //
                // Every ground skill used to target the same tile
                // (`position.x + 2`), which is fine for most of them but wrong
                // for traps: RO will not stack two skill units on one cell, so
                // the first trap claimed the tile and every trap after it was
                // refused. The sweep then recorded six refusals and one
                // placement and called that a pass, having never really
                // exercised trap placement at all.
                // Ring the caster rather than fanning outwards: trap range is
                // about 3 cells, and Hercules drops an out-of-range ground cast
                // with a bare `return 0` and **no** `clif->skill_fail`, so
                // anything further away reads as silence rather than a refusal.
                // Eight cells is more than the seven traps any job has.
                const GROUND_OFFSETS: [(i16, i16); 8] =
                    [(2, 0), (0, 2), (-2, 0), (0, -2), (2, 2), (-2, 2), (2, -2), (-2, -2)];
                let position = context.position;
                let (dx, dy) = GROUND_OFFSETS[ground_cast_index % GROUND_OFFSETS.len()];
                ground_cast_index += 1;
                context.net.cast_ground_skill(skill.skill_id, level, TilePosition {
                    x: (position.x as i16 + dx).max(5) as u16,
                    y: (position.y as i16 + dy).max(5) as u16,
                })
            }
            SkillType::SelfCast | SkillType::Support => context.net.cast_skill(skill.skill_id, level, player_id),
            SkillType::Passive => unreachable!(),
        };
        if cast.is_err() {
            return Err("disconnected mid-sweep".to_owned());
        }

        // Traps are held to a stricter standard than "something came back".
        //
        // A trap always creates a skill unit, so `AddSkillUnit` (0x09CA) is the
        // only evidence that it was actually *placed*. The generic matcher below
        // accepts any response at all, and for traps something else always
        // arrived first — `ground-unit` was never once reported across a full
        // 114-scenario run, so the sweep had never proven a trap lands. That
        // matters because Hunter traps are the one family with real prop
        // rendering behind them (`TRAP_MODEL_FILES`, phase E2).
        //
        // An explicit refusal still counts: conditions the harness cannot meet
        // are a legitimate outcome. What is no longer accepted is an unrelated
        // buff or visual standing in for proof of placement.
        if TRAP_PLACING_SKILLS.contains(&skill.skill_id.0) {
            let placed = context.wait_for_within("trap skill unit", Duration::from_secs(4), &mut |event| match event {
                NetworkEvent::AddSkillUnit { .. } => Some(("ground-unit", 0)),
                NetworkEvent::ChatMessage { .. } | NetworkEvent::MessageTable { .. } => Some(("fail-feedback", 0)),
                NetworkEvent::SkillFailedMissingItem { .. } => Some(("fail-missing-item", 0)),
                _ => None,
            });

            let (result, wait) = match placed {
                Ok((kind, _)) => (kind, 400),
                Err(_) if allowlisted(&name) => ("silent (allowlisted)", 0),
                Err(_) => ("SILENT — investigate", 0),
            };
            context.pump(Duration::from_millis(wait));
            outcomes.push(SkillOutcome {
                skill_id: skill.skill_id.0,
                name,
                skill_type: skill.skill_type,
                level: level.0,
                result,
            });
            continue;
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
            // `ZC_USE_SKILL` (0x09CB on this packetver — *not* 0x011a, which is
            // the pre-2013 header). This is the success notification for every
            // no-damage skill, and for most of them it is redundant: they also
            // grant the caster a status or deal damage, so one of the arms above
            // matches first and this one was never needed.
            //
            // It is not redundant for a skill whose only effect lands on someone
            // else. `DC_UGLYDANCE` drains SP from enemies in splash range and
            // gives the caster nothing, so on an empty sweep field 0x09CB is the
            // *sole* response — and omitting it here made a perfectly healthy
            // cast read as "SILENT — investigate". Found once the Dancer/Gypsy
            // sweeps could run at all.
            NetworkEvent::SkillEffectNoDamage { source_entity_id, .. } if source_entity_id.0 == player_id.0 => {
                Some(("no-damage-effect", 0))
            }
            NetworkEvent::SkillCooldown { skill_id, .. } if skill_id.0 == skill.skill_id.0 => Some(("cooldown", 0)),
            NetworkEvent::VisualEffect { .. } => Some(("visual", 0)),
            NetworkEvent::MonsterInformation { .. } => Some(("monster-info", 0)),
            NetworkEvent::WarpList { .. } => Some(("warp-list", 0)),
            NetworkEvent::SkillCooldownList { .. } => Some(("cooldown-list", 0)),
            NetworkEvent::RefinableWeaponList { .. } => Some(("weapon-list", 0)),
            NetworkEvent::ChatMessage { .. } | NetworkEvent::MessageTable { .. } => Some(("fail-feedback", 0)),
            // "You need a <item> to use this skill" — `ZC_ACK_TOUSESKILL` causes
            // 71/72, which the networking crate turns into this instead of a
            // ChatMessage because it has no item DB to name the item with.
            //
            // Omitting it made a correctly-reported refusal look like silence.
            // It stayed hidden because the shared character happened to be
            // carrying a stash of Trap items accumulated by earlier runs, so
            // Hunter/Sniper traps *succeeded* and reported a ground unit. Clear
            // the junk inventory and all seven trap skills "go silent" at once —
            // the sweep was quietly depending on leftover state to pass.
            NetworkEvent::SkillFailedMissingItem { .. } => Some(("fail-missing-item", 0)),
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

    // Hand back whatever traps went unused, so they cannot accumulate across
    // runs the way the observer scenario's ammunition did.
    if needs_traps {
        let _ = context.say(&format!("@delitem {TRAP_ITEM} 30000"));
        context.pump(Duration::from_millis(300));
    }

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
