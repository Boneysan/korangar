//! Phase 11 — observer parity.
//!
//! Every scenario here asserts on the **observer**, never on the actor. That is
//! the whole point: all six bugs found on 2026-07-29 looked perfectly correct
//! from the session that performed the action, and were only visible from a
//! second seat.
//!
//! The property under test is convergence:
//!
//! > For every observable attribute of character A, every client that can see A
//! > reaches the same value within a bounded time — regardless of when that
//! > client arrived, what order the packets landed in, or what happened while it
//! > was away.
//!
//! Three quantifiers, three timing axes. The five timings below are the
//! generalisation of what caught the four separate `LOOK_AMMO` bugs:
//!
//! | | Timing | Catches |
//! |---|---|---|
//! | T1 | change while observed | broadcast missing or wrong target |
//! | T2 | change while out of view, then return | no enter-view recovery |
//! | T3 | change, then the observer connects fresh | login seed ordering (`addblock`) |
//! | T4 | change, then the subject leaves and returns | rebuild wipe |
//! | T5 | change **to none/zero** | guards that cannot transmit "none" |
//! | T6 | disguise / undisguise | wholesale `sd->vd` writes (audit A8) |
//!
//! What these cannot see is the client's *use* of the value — the headless
//! tester consumes `NetworkEvent`s and never runs `korangar/src/lib.rs`. Of the
//! six bugs, this layer could have caught two. See
//! `docs/plans/observer-parity-harness.md` §5 for the layer that covers the
//! rest.
//!
//! **What each row actually proves**, measured 2026-07-29 by deleting the
//! `ChangeLook` tracking and re-running — worth knowing before trusting a pass:
//!
//! - Four of six then fail, so these do exercise the live broadcast path.
//! - `observer-look-fresh-login` still **passes**, because it takes everything
//!   from the spawn packet and never needs a broadcast at all. That is the
//!   point of it, and it is the row that would have caught the LOOK_AMMO login
//!   seed reaching nobody.
//! - T2/T4/T5 converge via the spawn packet too, on re-entering view. They
//!   assert that recovery *happens*, not which of the three mechanisms
//!   delivered it — which is the right property, but do not read a pass as
//!   "the enter-view re-send works".
//!
//! **Shared-state rule, learned the hard way here.** All 114 scenarios share
//! one test character. The first version of this file called
//! `ensure_base_level(10)` unconditionally, which *lowered* it from 99, and
//! left it a gun-wielding Gunslinger. That broke two unrelated scenarios much
//! later in the suite: `weapon-refine-missing-material` (a level-10 character
//! has no refinable weapon) and `incoming-damage` (which deliberately does not
//! call `ensure_job`, so a ranged attacker meant the wolf never retaliated).
//! Both looked like flaky pre-existing failures and were neither.
//!
//! So: raise shared state, never lower it, and restore anything exotic on the
//! way out — including on the failure path.

use std::time::Duration;

use ragnarok_packets::EquipPosition;

use crate::context::{Appearance, Config, TestContext};
use crate::scenarios::Scenario;

/// Values chosen so the "changed" state can never be confused with the default
/// or with a fallback — audit A6. A hair style of 1 would be worthless here,
/// because 1 is exactly what a failed lookup returns.
const HAIR_STYLE: u32 = 12;
const HAIR_COLOR: u32 = 5;
const CLOTHES_COLOR: u32 = 4;
/// Silver Bullet, deliberately not the `ranged_attack_default_ammunition`
/// fallback (13200) — that collision cost a full extra test round.
const SILVER_BULLET: u32 = 13201;
const REVOLVER: u32 = 13100;

/// Somewhere the partner cannot see, to take the subject out of view.
///
/// Must be a DIFFERENT map from wherever the partner spawns (`prt_fild08`).
/// An earlier version used `prt_fild08` itself and relied on `warp_random`
/// landing far enough away to drop out of view — which passed, but only by
/// luck: a random cell near the partner would have left the subject visible
/// and timed the scenario out for a reason having nothing to do with parity.
const FAR_MAP: &str = "prontera";

/// Used when the pair happens to be standing on [`FAR_MAP`] itself.
const FAR_MAP_ALTERNATE: &str = "geffen";

/// Pick a map the pair is demonstrably not standing on.
///
/// The comment on [`FAR_MAP`] was right about the hazard and wrong about who
/// could trigger it: it assumed the partner always spawns on `prt_fild08`, so a
/// constant was enough. The partner does not stay put. `connect_pair` meets on
/// wherever the partner character was *last left*, and that position persists in
/// the `char` table across scenarios — so the meeting map is shared mutable
/// state while `FAR_MAP` is a constant. When scenario order parked the partner
/// on `prontera`, "warp far away" became "warp to a random cell on the
/// observer's own map", `assert_in_view(false)` stopped holding, and the
/// scenario failed for a reason having nothing to do with parity — the precise
/// failure the constant's own doc comment warned about.
///
/// Found by `--shuffle 42` (`observer-look-clear`). The double-run gate cannot
/// see this class of bug: it runs the same order twice.
fn far_map_from(home_map: &str) -> &'static str {
    if home_map == FAR_MAP { FAR_MAP_ALTERNATE } else { FAR_MAP }
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("observer-look-live", 11, look_change_while_observed),
        Scenario::new("observer-look-return", 11, look_change_while_out_of_view),
        Scenario::new("observer-look-fresh-login", 11, look_change_then_observer_logs_in),
        Scenario::new("observer-look-rebuild", 11, look_survives_entity_rebuild),
        Scenario::new("observer-look-clear", 11, clearing_a_look_reaches_the_observer),
        Scenario::new("observer-ammo-disguise", 11, ammunition_survives_a_disguise),
    ]
}

/// Put the primary in a known appearance state and wait for the partner to
/// agree, so a later assertion cannot pass on a value that was already there.
fn set_baseline(primary: &mut TestContext, partner: &mut TestContext) -> Result<(), String> {
    primary.say("@hairstyle 2")?;
    primary.say("@haircolor 0")?;
    primary.say("@dye 0")?;
    primary.pump(Duration::from_millis(600));
    let subject = primary.account_id;
    partner.assert_converges(subject, Appearance::HairStyle, 2)?;
    partner.assert_converges(subject, Appearance::HairColor, 0)?;
    partner.assert_converges(subject, Appearance::ClothesColor, 0)?;
    Ok(())
}

/// T1 — the basic case. A change made while the observer is watching must
/// reach them. Covers hair style (its own event) and hair/clothes colour (which
/// arrive as `ChangeLook`, and used to be discarded by `_ => None`).
fn look_change_while_observed(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    set_baseline(&mut primary, &mut partner)?;
    let subject = primary.account_id;

    primary.say(&format!("@hairstyle {HAIR_STYLE}"))?;
    primary.say(&format!("@haircolor {HAIR_COLOR}"))?;
    primary.say(&format!("@dye {CLOTHES_COLOR}"))?;

    partner.assert_converges(subject, Appearance::HairStyle, HAIR_STYLE)?;
    partner.assert_converges(subject, Appearance::HairColor, HAIR_COLOR)?;
    partner.assert_converges(subject, Appearance::ClothesColor, CLOTHES_COLOR)?;
    Ok(())
}

/// T2 — enter-view recovery. The change happens while the observer cannot see
/// the subject at all, so the broadcast reaches nobody; the observer must learn
/// it on arrival instead. This is the axis `LOOK_AMMO` failed for months.
fn look_change_while_out_of_view(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    set_baseline(&mut primary, &mut partner)?;
    let subject = primary.account_id;
    let (home_map, home_x, home_y) = (partner.map_name.clone(), partner.position.x + 1, partner.position.y);

    primary.warp_random(far_map_from(&home_map))?;
    partner.assert_in_view(subject, false)?;

    // Out of sight: the AREA broadcast for these reaches nobody.
    primary.say(&format!("@hairstyle {HAIR_STYLE}"))?;
    primary.say(&format!("@haircolor {HAIR_COLOR}"))?;
    primary.pump(Duration::from_millis(600));

    primary.warp(&home_map, home_x, home_y)?;
    partner.assert_in_view(subject, true)?;

    partner.assert_converges(subject, Appearance::HairStyle, HAIR_STYLE)?;
    partner.assert_converges(subject, Appearance::HairColor, HAIR_COLOR)?;
    Ok(())
}

/// T3 — the login seed. The observer was not even connected when the change
/// happened, so everything must come from the spawn packet. This is the axis
/// that caught the `LOOK_AMMO` broadcast issued before `map->addblock`, which
/// reached nobody because the character had not joined the block list yet.
fn look_change_then_observer_logs_in(config: &Config) -> Result<(), String> {
    let mut primary = TestContext::connect(config)?;
    let subject = primary.account_id;

    primary.say(&format!("@hairstyle {HAIR_STYLE}"))?;
    primary.say(&format!("@haircolor {HAIR_COLOR}"))?;
    primary.say(&format!("@dye {CLOTHES_COLOR}"))?;
    primary.pump(Duration::from_millis(800));

    // Connect the observer only now, and bring the subject to them.
    let mut partner = TestContext::connect_partner(config)?;
    let map_name = partner.map_name.clone();
    primary.warp(&map_name, partner.position.x + 1, partner.position.y)?;
    partner.assert_in_view(subject, true)?;

    partner.assert_converges(subject, Appearance::HairStyle, HAIR_STYLE)?;
    partner.assert_converges(subject, Appearance::HairColor, HAIR_COLOR)?;
    partner.assert_converges(subject, Appearance::ClothesColor, CLOTHES_COLOR)?;
    Ok(())
}

/// T4 — entity rebuild. The subject leaves and comes back, which makes the
/// observer tear the entity down and build a new one. Anything not carried by
/// the spawn packet is wiped at that moment; this asserts the appearance is.
fn look_survives_entity_rebuild(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    set_baseline(&mut primary, &mut partner)?;
    let subject = primary.account_id;
    let (home_map, home_x, home_y) = (partner.map_name.clone(), partner.position.x + 1, partner.position.y);

    primary.say(&format!("@hairstyle {HAIR_STYLE}"))?;
    primary.say(&format!("@dye {CLOTHES_COLOR}"))?;
    partner.assert_converges(subject, Appearance::HairStyle, HAIR_STYLE)?;
    partner.assert_converges(subject, Appearance::ClothesColor, CLOTHES_COLOR)?;

    // Leave and return: the observer rebuilds the entity from EntityData alone.
    primary.warp_random(far_map_from(&home_map))?;
    partner.assert_in_view(subject, false)?;
    primary.warp(&home_map, home_x, home_y)?;
    partner.assert_in_view(subject, true)?;

    partner.assert_converges(subject, Appearance::HairStyle, HAIR_STYLE)?;
    partner.assert_converges(subject, Appearance::ClothesColor, CLOTHES_COLOR)?;
    Ok(())
}

/// T5 — transmitting "none". A guard of the form `if (x) refreshlook(...)` can
/// tell an observer that a value *is* something but never that it is nothing,
/// so a reset that happened out of view is never corrected. Harmless for
/// anything the spawn packet carries, which is exactly what this proves.
fn clearing_a_look_reaches_the_observer(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    let subject = primary.account_id;
    let (home_map, home_x, home_y) = (partner.map_name.clone(), partner.position.x + 1, partner.position.y);

    primary.say(&format!("@dye {CLOTHES_COLOR}"))?;
    primary.say(&format!("@haircolor {HAIR_COLOR}"))?;
    partner.assert_converges(subject, Appearance::ClothesColor, CLOTHES_COLOR)?;
    partner.assert_converges(subject, Appearance::HairColor, HAIR_COLOR)?;

    // Reset to zero while out of view, so only the enter-view path can correct
    // the observer — the case a non-zero guard silently drops.
    primary.warp_random(far_map_from(&home_map))?;
    partner.assert_in_view(subject, false)?;
    primary.say("@dye 0")?;
    primary.say("@haircolor 0")?;
    primary.pump(Duration::from_millis(600));
    primary.warp(&home_map, home_x, home_y)?;
    partner.assert_in_view(subject, true)?;

    partner.assert_converges(subject, Appearance::ClothesColor, 0)?;
    partner.assert_converges(subject, Appearance::HairColor, 0)?;
    Ok(())
}

/// T6 — `sd->vd` lifetime (audit A8). `status_set_viewdata` memcpy's a whole
/// `view_data` over `sd->vd` when a player is disguised, zeroing every
/// fork-added field; un-disguising returns through the branch that assigns
/// fields individually and used to never re-assign ammunition, so it stayed
/// zero forever. DM mode uses disguises, which is why this matters here.
///
/// Ammunition is also the one attribute with no spawn-packet slot, so it is the
/// only thing in this file read from `remote_ammunition` rather than the
/// tracked `EntityData`.
fn ammunition_survives_a_disguise(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    let result = ammunition_disguise_body(&mut primary, &mut partner);

    // ALWAYS restore, including on failure. Every scenario shares one test
    // character, and this is the only one that leaves it holding a gun. That
    // broke `incoming-damage`, which deliberately does NOT call `ensure_job` —
    // it uses whatever job it finds, to A/B mob behaviour — so a ranged
    // Gunslinger meant the wolf never retaliated. 4008 (Lord Knight) is the
    // melee default `combat_bootstrap` uses.
    let _ = primary.say("@unequipall");
    primary.pump(Duration::from_millis(300));
    let _ = primary.ensure_job(4008);

    // Hand the ammunition back too, not just the gun.
    //
    // This grants 100 rounds every run and used to keep them. Ammunition
    // stacks, so nothing ever looked wrong — it just accumulated, run after
    // run, until the shared character was carrying ~1600 Silver Bullets and
    // crossed its weight limit. Past that point Hercules answers `@item` with
    // "Failed to pick up item.", and **every scenario that needs an item
    // breaks at once**: `use-consumable` timed out waiting for an inventory
    // add, and `skills-hunter` reported seven skills as silent because its
    // traps consume items the harness could no longer hand over.
    //
    // That failure mode is nastier than an ordinary shared-state bug: it is
    // invisible for dozens of runs, then appears far away from its cause and
    // looks like several unrelated regressions at once.
    let _ = primary.say(&format!("@delitem {SILVER_BULLET} 30000"));
    let _ = primary.say(&format!("@delitem {REVOLVER} 100"));
    primary.pump(Duration::from_millis(300));

    result
}

fn ammunition_disguise_body(primary: &mut TestContext, partner: &mut TestContext) -> Result<(), String> {
    let subject = primary.account_id;

    // A gun is needed before bullets will equip, and the ammo must go on after
    // it — the server force-unequips ammunition when the weapon comes off.
    // (There is no `@equipall`; only `@unequipall` exists.)
    // Six Shooter is `Job: { Gunslinger: true }`, `EquipLv: 10`. Gunslinger is
    // job 24 — NOT 4011, which is Whitesmith and cannot hold a gun at all.
    // Resolve job ids from db/constants.conf, never from memory: nine of
    // eighteen skill-id guesses were wrong the same way on an earlier pass.
    primary.ensure_job(24)?;
    // RAISE ONLY. `ensure_base_level` sets an exact level, so calling it
    // unconditionally *lowered* the shared test character from 99 to 10 and
    // broke two later scenarios in the full suite — a level-10 character cannot
    // make a wolf retaliate and has no refinable weapon. Scenarios share one
    // character; never move shared state downward to satisfy a local need.
    if primary.base_level < 10 {
        primary.ensure_base_level(10)?;
    }
    let gun = primary.give_item(REVOLVER, 1)?;
    primary
        .net
        .request_item_equip(gun, EquipPosition::RIGHT_HAND)
        .map_err(|_| "disconnected")?;
    primary.pump(Duration::from_millis(500));

    let bullets = primary.give_item(SILVER_BULLET, 100)?;
    primary
        .net
        .request_item_equip(bullets, EquipPosition::AMMO)
        .map_err(|_| "disconnected")?;
    primary.pump(Duration::from_millis(500));

    partner.assert_converges(subject, Appearance::Ammunition, SILVER_BULLET)?;

    primary.say("@disguise 1002")?;
    primary.pump(Duration::from_millis(600));
    primary.say("@undisguise")?;
    primary.pump(Duration::from_millis(600));

    partner.assert_converges(subject, Appearance::Ammunition, SILVER_BULLET)?;
    Ok(())
}

// NOT COVERED HERE: turning in place.
//
// `ZC_CHANGE_DIRECTION` stopped being a `register_noop` on 2026-07-29 and its
// inbound path has a unit test (`turning_in_place_reaches_the_client`), but a
// scenario cannot *provoke* one: the outgoing `CZ_CHANGE_DIRECTION` (0x009B /
// 0x0361 — present in the length table, modeled nowhere) does not exist in this
// fork, because korangar never sends the player's facing. Adding it is a
// feature, not a test fixture, so it stays out of the harness.
