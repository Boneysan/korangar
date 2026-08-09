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
        Scenario::new("skill-fail-reason-packet", 5, skill_fail_reason_packet),
        Scenario::new("cast-cancel", 5, cast_cancel),
        Scenario::new("land-protector-status", 5, land_protector_status),
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
const PR_REDEMPTIO: SkillId = SkillId(1014);

/// What the client says for Redemptio when it has *only* the skill id to go on.
///
/// Cause 0 cannot say which of Redemptio's three conditions failed, so without
/// `ZC_SKILL_FAIL_REASON` the client can only name all three. Seeing this text is
/// how a lost fork delta looks from the outside — nothing errors, nothing goes
/// missing, the sentence just gets vaguer.
const REDEMPTIO_INFERRED: &str = "Redemptio needs a party, at least one dead party member in range, and 1% of your base and job experience to spend.";

/// The three things `ZC_SKILL_FAIL_REASON` can say about Redemptio, one per
/// cause-0 path in Hercules. Which one fires depends on the shared character's
/// party and experience at the time, and the scenario deliberately accepts any
/// of them: what is being guarded is that the *packet arrives*, not which branch
/// the server happened to take.
const REDEMPTIO_REASONS: &[&str] = &[
    "You have to be in a party to use that.",                                    // skill.c:7004
    "No dead party member was in range.",                                        // skill.c:7013
    "That spends 1% of your base and job experience, and you do not have it.",   // skill.c:16056
];

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

/// Every skill whose `skill_db` entry has a `Unit:` block — i.e. every skill
/// that is supposed to leave a skill unit on the ground.
///
/// This is the authority for "did the cast actually do anything". The generic
/// matcher accepts any response at all, and for these skills `SkillCast`
/// arrives first and satisfies it, so the sweep recorded a cast bar starting
/// and never checked what it produced. `ground-unit` was never once reported
/// across a full run before traps were held to this standard.
///
/// Regenerate from Hercules when skill_db changes:
/// ```sh
/// python3 - <<'EOF'
/// import re, pathlib
/// t = pathlib.Path("db/re/skill_db.conf").read_text(errors="replace")
/// ids = sorted(int(re.search(r'^\tId:\s*(\d+)', e, re.M).group(1))
///              for e in re.split(r'\n\{\n', t)
///              if re.search(r'^\tId:', e, re.M) and re.search(r'^\tUnit:\s*\{', e, re.M))
/// print(ids)
/// EOF
/// ```
const UNIT_CREATING_SKILLS: &[u16] = &[
    12, 18, 21, 25, 27, 47, 70, 79, 80, 83, 85, 87, 89, 91, 92, 115, 116, 117, 118, 119, 120, 121, 122, 123, 125,
    140, 220, 229, 254, 285, 286, 287, 288, 336, 339, 369, 395, 404, 405, 409, 410, 428, 429, 430, 488, 516, 521,
    525, 527, 535, 538, 541, 653, 670, 2032, 2044, 2213, 2216, 2238, 2239, 2249, 2250, 2251, 2252, 2253, 2254,
    2273, 2274, 2299, 2300, 2301, 2302, 2303, 2304, 2319, 2414, 2418, 2419, 2443, 2444, 2446, 2447, 2449, 2450,
    2452, 2453, 2465, 2466, 2467, 2468, 2479, 2482, 2484, 2485, 2487, 2488, 2490, 2555, 2567, 2587, 3006, 3008,
    3009, 3010, 3020, 5006, 5008, 5010, 5027, 5028, 5029, 8020, 8025, 8033, 8041, 8043, 8208, 8209, 8210, 8211,
    8212, 8403, 8406, 8409, 8412, 10006, 10007, 10008, 10009
];

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

/// `ZC_SKILL_FAIL_REASON` (fork packet 0x0EFE) has to actually arrive.
///
/// Everything else about this packet is covered by unit tests over bytes we
/// assembled ourselves, which proves the client's half and assumes the server's.
/// This is the only check that the eleven call sites are reached, that the wire
/// layout matches, and that the length entry survives in a real stream.
///
/// It exists because losing the delta in an upstream merge is **silent**: the
/// skill still fails, a red line still appears, and only the wording quietly
/// degrades. So the assertion is on the reason-derived text specifically —
/// asserting "a failure message arrived" would pass with the delta gone, which
/// is the one outcome this must catch.
///
/// Redemptio is the trigger because most of the obvious ones cannot be guarded:
/// for Party Flee, Benedictio and the ensemble songs the reason text is
/// deliberately identical to what the client infers, so a test there proves
/// nothing. Redemptio has three conditions, so its inferred text is a hedge that
/// no reason can produce.
/// A Sage, for `SA_VOLCANO`'s five second cast (4000 + 1000 fixed) — long
/// enough that a cancel lands while the cast is genuinely in progress.
const SAGE: u16 = 16;
const SA_VOLCANO: SkillId = SkillId(285);
const BLUE_GEMSTONE: u32 = 717;
/// Open ground, chosen rather than inherited. An unplaceable field looks exactly
/// like a cancelled one, which would let this scenario pass for the wrong
/// reason — see `observer.rs`'s `CAST_VENUE`, where that cost a debugging round.
const CAST_VENUE: (&str, u16, u16) = ("prt_fild08", 286, 338);

const SA_LANDPROTECTOR: SkillId = SkillId(288);
/// A map no other scenario visits, so a long-lived field cannot reach them.
const ISOLATION_MAP: &str = "prt_fild01";
const YELLOW_GEMSTONE: u32 = 715;
/// `SI_LANDPROTECTOR`, the icon index this fork *invented*. Must be matched on:
/// taking whatever status happens to arrive first would assert against an
/// unrelated one and look exactly like the delta working.
const SI_LANDPROTECTOR: u16 = 1150;

/// Guards the **`SC_LANDPROTECTOR`** fork delta, which spans **five** places and
/// **fails silently if any one of them is missing**.
///
/// Officially Land Protector grants nothing — it acts on the ground, not on
/// people — so this fork invented a status purely to tell the player their
/// ground magic is suppressed and for how long. The five sites are `status.h`
/// (the `sc_type` slot), `db/constants.conf` (**both** `SC_LANDPROTECTOR: 728`
/// and `SI_LANDPROTECTOR: 1150`), `db/re/sc_config.conf`, `db/re/skill_db.conf`
/// (`StatusChange:` on the skill), and `src/map/skill.c`
/// (`skill_unit_onplace` / `skill_unit_onout`).
///
/// **Why it needs a scenario rather than a code review:** `sc_config.conf` and
/// `skill_db.conf` resolve status names through `script->get_constant()`, so if
/// the `SC_` constant goes missing both bindings are skipped with nothing but a
/// `ShowWarning` — on a server whose stdout goes to `log/server-latest.log`, not
/// to any log anyone reads during a merge. The field still spawns and still
/// suppresses magic; only the *explanation* disappears.
fn land_protector_status(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    context.ensure_job(SAGE)?;
    context.say("@allskill")?;
    // Land Protector costs 66 SP at level 1 and scenarios share one character,
    // so an earlier SP drain would otherwise fail the cast before it began.
    context.say("@heal")?;
    context.pump(Duration::from_millis(500));

    // **This one gets a map to itself, and that is not fastidiousness.** At
    // level 1 the field stands for **165 seconds** over a **7×7** area
    // (`Layout: 3`) and its whole purpose is suppressing ground magic. Cast at
    // the shared venue it covers cells 285-291 — i.e. the meeting cell every
    // other paired scenario uses — and leaves a nearly three-minute dead zone
    // behind it. That silently took out `AL_PNEUMA` in the Acolyte sweep two
    // minutes downstream, which is exactly the "one scenario breaks two
    // unrelated ones much later" trap this file's header describes. Nothing
    // else in the suite goes to `prt_fild01`.
    context.warp_random(ISOLATION_MAP)?;
    context.give_item(BLUE_GEMSTONE, 1)?;
    context.give_item(YELLOW_GEMSTONE, 1)?;

    // Cast on our own cell. Land Protector carries only `UF_PATHCHECK` — no
    // `UF_NOFOOTSET` — so unlike the elemental fields it places under the
    // caster, and a cell we are already standing on is walkable by definition.
    // That removes the guessed-neighbour-cell fragility that cost `observer.rs`
    // a debugging round, and it is why no walk-in is needed below.
    let target = context.position;

    context.flush();
    context
        .net
        .cast_ground_skill(SA_LANDPROTECTOR, SkillLevel(1), target)
        .map_err(|error| format!("could not request the cast: {error:?}"))?;

    // Wait for the field before walking in: the unit only exists once the cast
    // completes, so stepping onto the cell early parks us there with nothing to
    // stand in.
    context.wait_for_within("the field to be placed", Duration::from_secs(15), &mut |event| match event {
        NetworkEvent::AddSkillUnit { .. } => Some(()),
        _ => None,
    })?;

    // No walk-in: at `Range: 2` the caster stands **inside** the footprint it
    // just placed, so the status arrives on placement. (`observer-status-values`
    // does walk, because `SA_VOLCANO`'s field is aimed clear of the caster.)
    let result = context.wait_for_within(
        "SC_LANDPROTECTOR once the field is standing",
        Duration::from_secs(10),
        &mut |event| match event {
            NetworkEvent::StatusChange {
                index, gained: true, ..
            } if *index == SI_LANDPROTECTOR => Some(()),
            _ => None,
        },
    );

    // Leave on every path, including the failure one. Walking out of a 7×7
    // field is not enough on its own — the *field* is what harms the next
    // scenario, not our position in it — so the character leaves the map
    // entirely and the field is stranded where nothing will stand in it.
    let (venue_map, venue_x, venue_y) = CAST_VENUE;
    let _ = context.warp(venue_map, venue_x, venue_y);

    result.map_err(|error| {
        format!(
            "{error}\n         the field was placed but no SI_LANDPROTECTOR (1150) status arrived. \
             This fork's SC_LANDPROTECTOR delta spans five sites and any one of them going missing \
             fails exactly like this, with only a ShowWarning on the server: check the sc_type slot in \
             src/map/status.h, BOTH constants in db/constants.conf, the sc_config.conf icon entry, \
             `StatusChange: \"SC_LANDPROTECTOR\"` in db/re/skill_db.conf, and skill_unit_onplace in \
             src/map/skill.c"
        )
    })
}

/// Guards the **`CZ_CANCEL_CAST` (0x0F00)** fork delta end to end — the only
/// check that the *server* half of it works.
///
/// Every other test over this packet asserts bytes we assembled ourselves
/// (`cancel_cast_packet_is_a_bare_two_byte_header`), which passes with the
/// Hercules delta deleted. That matters more here than for most deltas because
/// of how it fails: **a client packet with no length entry makes `clif_parse`
/// disconnect the session** rather than warn, and the length lives in
/// `src/common/packets_len.h`, which is hand-maintained and which
/// `generate_packet_lengths.sh` never reads. So an upstream merge that drops it
/// does not degrade the feature — it makes **right-click kick the player to the
/// login screen**, and nothing else in this suite would notice.
///
/// Two assertions, and the second is the one with teeth: the server must
/// acknowledge the cancel, **and the skill must not go off anyway**. Asserting
/// only the acknowledgement would pass while the field still landed.
fn cast_cancel(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    context.ensure_job(SAGE)?;
    context.say("@allskill")?;
    context.pump(Duration::from_millis(500));

    let (map, x, y) = CAST_VENUE;
    context.warp(map, x, y)?;
    context.give_item(BLUE_GEMSTONE, 1)?;

    // Two cells east: `SA_VOLCANO` has `Range: 2`, and an out-of-range ground
    // cast is dropped with a bare `return 0` and no `clif->skill_fail`.
    let target = TilePosition {
        x: context.position.x + 2,
        y: context.position.y,
    };

    context.flush();
    let caster = context.player_id;
    context
        .net
        .cast_ground_skill(SA_VOLCANO, SkillLevel(1), target)
        .map_err(|error| format!("could not request the cast: {error:?}"))?;

    context.wait_for_within("our own cast to start", Duration::from_secs(6), &mut |event| match event {
        NetworkEvent::SkillCast {
            source_entity_id,
            skill_id,
            ..
        } if source_entity_id.0 == caster.0 && *skill_id == SA_VOLCANO => Some(()),
        _ => None,
    })?;

    context.net.cancel_cast().map_err(|_| {
        "disconnected while sending CZ_CANCEL_CAST (0x0F00) — this is the signature of the missing \
         length entry: Hercules' `clif_parse` drops a session that sends a packet it has no length \
         for. Check `packetLen(0x0f00, 2)` in Hercules `src/common/packets_len.h`, which is \
         hand-maintained and is NOT regenerated by tools/generate_packet_lengths.sh"
            .to_owned()
    })?;

    context
        .wait_for_within("the server to acknowledge the cancel", Duration::from_secs(6), &mut |event| {
            match event {
                NetworkEvent::SkillCastCancelled { .. } => Some(()),
                _ => None,
            }
        })
        .map_err(|error| {
            format!(
                "{error}\n         no clif->skillcastcancel came back. The CZ_CANCEL_CAST (0x0F00) \
                 Hercules delta has probably been lost in an upstream merge — check \
                 `clif_parse_CancelCast` and the `clif->pCancelCast =` registration in \
                 src/map/clif.c, the interface member in clif.h, and the packets.h entry"
            )
        })?;

    // The cast was five seconds and we cancelled part way, so anything left of
    // it would land inside this window.
    let placed = context
        .collect_for(Duration::from_secs(6))
        .iter()
        .any(|event| matches!(event, NetworkEvent::AddSkillUnit { .. }));
    if placed {
        return Err("the cancel was acknowledged but the field was placed anyway — the skill still \
                    went off, so `unit->skillcastcancel` did not actually stop it"
            .to_owned());
    }

    Ok(())
}

fn skill_fail_reason_packet(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let result = skill_fail_reason_packet_body(&mut context);

    // Redemptio is a *quest* skill, so it has to be granted explicitly and taken
    // away again — 124 scenarios share one character, and leaving a quest skill
    // on it is exactly the "exotic state" the shared-state rule says to undo.
    let _ = context.say("@lostskill 1014");
    context.pump(Duration::from_millis(300));

    result
}

fn skill_fail_reason_packet_body(context: &mut TestContext) -> Result<(), String> {
    context.ensure_job(8)?;
    // `@allskill` deliberately skips quest skills (`pc->allskillup` honours
    // `battle_config.quest_skill_learn`, which is false here), so Redemptio has
    // to come from `@questskill`.
    context.flush();
    context.say("@questskill 1014")?;

    // Report what the server said if it refused; "the command did nothing" is
    // not a debuggable failure message.
    let replies: Vec<String> = context
        .collect_for(Duration::from_millis(600))
        .iter()
        .filter_map(|event| match event {
            NetworkEvent::ChatMessage { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();

    let level = context
        .skills
        .iter()
        .find(|skill| skill.skill_id == PR_REDEMPTIO)
        .map(|skill| skill.skill_level)
        .ok_or_else(|| format!("@questskill 1014 did not grant Redemptio; server said {replies:?}"))?;
    context.flush();

    let player_id = context.player_id;
    context
        .net
        .cast_skill(PR_REDEMPTIO, level, player_id)
        .map_err(|_| "disconnected mid Redemptio cast".to_owned())?;

    let text = context.wait_for("Redemptio failure ChatMessage", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if !text.is_empty() => Some(text.clone()),
        _ => None,
    })?;

    if text == REDEMPTIO_INFERRED {
        return Err(format!(
            "the failure was explained by skill id, not by the server: {text:?}\n         \
             ZC_SKILL_FAIL_REASON (0x0EFE) did not arrive — check the Hercules delta is still \
             applied and the server was rebuilt (see CLAUDE.md 3b)"
        ));
    }

    if !REDEMPTIO_REASONS.contains(&text.as_str()) {
        return Err(format!(
            "unexpected Redemptio failure text {text:?}; expected one of {REDEMPTIO_REASONS:?}"
        ));
    }

    Ok(())
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

    // Whitesmith's Weapon Refine success rate depends on job level and DEX/LUK.
    //
    // **`@stat` is not a Hercules command.** This used to say `@stat dex 99` /
    // `@stat luk 99`, which the server simply rejected, so two thirds of the
    // documented "boost the stats and retry" fix never did anything and the
    // retry loop below was carrying the scenario on its own. The real commands
    // are `@dex` / `@luk` (both aliases of `param`), and they take a **relative
    // adjustment**, not a target — `<+/-adjustment>` per the usage message —
    // clamped to the max, so repeated runs saturate rather than accumulate.
    //
    // `@jlevel` is relative and clamping too (`job_level += level`), so it
    // saturates at the job's maximum. All three only ever *raise* shared state,
    // which is the direction the shared-state rule permits.
    context.say("@jlevel 70")?;
    context.say("@dex 99")?;
    context.say("@luk 99")?;
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
        if UNIT_CREATING_SKILLS.contains(&skill.skill_id.0) {
            // 12s, not the 4s the generic path uses: these have cast times (Storm
            // Gust, Magnus, Warp), and the unit only appears once the bar
            // completes. `SkillCast` is deliberately NOT accepted here — a cast
            // bar starting is what the loose standard mistook for evidence.
            let placed = context.wait_for_within("skill unit", Duration::from_secs(12), &mut |event| match event {
                NetworkEvent::AddSkillUnit { .. } => Some(("ground-unit", 0)),
                NetworkEvent::ChatMessage { .. } | NetworkEvent::MessageTable { .. } => Some(("fail-feedback", 0)),
                NetworkEvent::SkillFailedMissingItem { .. } => Some(("fail-missing-item", 0)),
                // AL_WARP opens a destination picker *before* the portal exists,
                // so the warp list is its real response — not evidence of a unit,
                // but not silence either, and the sweep cannot choose a
                // destination. `teleport-select` covers the selection path.
                NetworkEvent::WarpList { .. } => Some(("warp-list", 0)),
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
