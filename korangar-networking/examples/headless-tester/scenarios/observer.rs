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

/// Somewhere the partner cannot follow, to take the subject out of view.
const FAR_MAP: &str = "prt_fild08";

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

    primary.warp_random(FAR_MAP)?;
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
    primary.warp_random(FAR_MAP)?;
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
    primary.warp_random(FAR_MAP)?;
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
    let subject = primary.account_id;

    // A gun is needed before bullets will equip, and the ammo must go on after
    // it — the server force-unequips ammunition when the weapon comes off.
    // (There is no `@equipall`; only `@unequipall` exists.)
    primary.ensure_job(4011)?; // Gunslinger — can hold a revolver
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
