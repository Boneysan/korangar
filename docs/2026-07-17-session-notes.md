# 2026-07-17 — M1-008 round 2: ground-cast effects, bolt volleys, portal tuning

Follow-up to the 2026-07-16 STR renderer fix, driven by live feedback:
Thunderstorm showed nothing, Fire/Cold Bolt lacked their falling projectiles,
the warp vortex read as too small. (Mob emote chat silence was confirmed
working.)

## Thunderstorm root cause: `ZC_NOTIFY_GROUNDSKILL` was noop

The original client plays ground-cast area effects (Thunderstorm, Storm
Gust) from `ZC_NOTIFY_GROUNDSKILL` (0x0117) at the targeted position —
independent of damage. Hercules sends it for every ground cast
(`skill_castend_pos2` default branch → `clif->skill_poseffect`), verified in
source and live: a new **temporary** headless scenario `probe-thunderstorm`
(skills.rs; runs on its own GM account `probe`/`probe`, created directly in
the `login` table, so it does not kick a GUI session) showed the packet
arriving with the cast position. Korangar had it registered as noop, so the
storm never played; per-target damage still arrived (skill 21, `div` 10).

Wiring now matches the original (cross-checked against roBrowserLegacy's
skill/effect tables, semantics only):

- New `NetworkEvent::GroundSkillEffect { skill_id, source_entity_id, level,
  position }`; lib.rs plays `thunderstorm.str` (21) / `stormgust.str` (89)
  at `map.get_world_position(position)`.
- `NetworkEvent::DamageEffect` gained `hit_count` (`div`, ≥1).
- Per-hit table `wizard_hit_effects`: Soul Strike → soulexpansion stand-in;
  Fire Bolt → random `firehit1-3.str` **delayed 0.5 s** to meet the volley;
  Lightning Bolt (20) → `lightning.str` + random `windhit1-3.str`;
  Thunderstorm → random `windhit1-3.str` per target. Cold Bolt's classic hit
  is sound-only (no STR). Load failures now `eprintln` (`[skill-effect]`)
  instead of failing silently.

## Classic falling-bolt volleys (`world/effect/bolts.rs`)

`FallingBolts` (EffectBase): `hit_count` projectiles staggered 0.15 s, each
falling 0.5 s from ~22 units above (slight randomized sideways offset) onto
the target — the classic `ef_firebolt`/`ef_coldbolt` code-drawn effect.
Fire Bolt animates `effect\불화살1-6.tga` at 30 ms/frame; Cold Bolt uses
`effect\icearrow.tga`. Sprites rotate to their on-screen direction of
travel; additive blend. `EffectWithLight` gained a `start_delay` parameter
(hit bursts waiting for the volley).

## Also

- Portal vortex enlarged (outer r6→5 h16, inner r3.6→3 h24, alpha up) after
  "little warp thing" feedback; awaiting another look.
- Sounds (`ef_firearrow%d.wav` etc.) are still not played — the classic
  tables name them; future work.
- The probe scenario + `probe` account are temporary diagnostics; remove
  before the next acceptance gate (scenario count parity: suite is 106).

All `cargo test -p korangar --lib` green (80); all mapped STRs parse with 0
unconsumed bytes; release rebuilt and relaunched for live verification.

## Live verification + future work

**Live-verified by the user 2026-07-17: Thunderstorm, Fire Bolt, and Cold
Bolt all render.** (Storm Gust and the emote silence were confirmed in the
previous round; the warp vortex was visible and has since been enlarged.)

Per the user's direction, a **classic skill-effect coverage pass** is now a
scoped High row in `FEATURE_ROADMAP.md`: audit the whole skill catalog for
the same three wiring-gap classes the Wizard kit hit — unmapped ground-cast
STRs (`ground_skill_effect` has only 21/89), missing per-hit STRs
(`wizard_hit_effects` has only 13/19/20/21), and code-drawn effects that
were never STR files (`ef_*` recipes in roBrowserLegacy). Plus skill-unit
visuals beyond Firewall/Pneuma, `DisplaySpecialEffectPacket`, cast circles,
and sounds. The roadmap row records the proven method (wire probe → STR
dump → reference tables → live check).

## Classic skill-effect coverage pass — asset-backed batch 1

The first catalog-wide follow-up keeps to effects that are already shipped as
classic STR files and are supported by the corrected renderer. The former
`wizard_hit_effects` table is now `skill_hit_effects` and covers additional
Mage/Wizard hits plus Knight, Priest, Hunter, Assassin, and Holy Light hits.
`ground_skill_effect` now includes Firewall, Sanctuary, Magnus, Fire Pillar,
Meteor Storm, Lord of Vermilion, Quagmire, Hammer Fall, Skid Trap, and Venom
Dust cast effects.

Live follow-up initially put Meteor's full STR and Frost Nova's freeze STR on
each damaged target. That exposed the packets and timing problem, but live
comparison showed only a small animation as the mob died instead of the
original spell presentation. Rechecking the classic recipes established the
correct split: Meteor's random `meteor1-4.str` starts at the ground-cast
position, while only `firehit1-3.str` belongs on each damaged target. Frost
Nova's `freeze.str` plays once on the caster and its per-target hit is
sound-only. The mappings now follow that split. A short-lived source/skill
gate protects Frost Nova's one caster effect from duplicate successful-use
notifications.

Every new mapping uses `EffectWithLight`, with elemental light colors at the
target or ground position. All 24 newly referenced STR assets were loaded and
parsed directly from the configured GRFs with zero unconsumed bytes. Effects
that are code-drawn in the classic client (Napalm Beat, Frost Diver travel,
Jupitel Thunder, Earth Spike/Heaven's Drive geometry, Ice Wall, persistent
Sanctuary/Magnus fields) remain intentionally separate work.

Live testing exposed a first-use timing failure in the new Meteor and Frost
Nova mappings: their elemental point lights appeared, but the STR geometry did
not. Loading a previously unseen STR and all of its textures is synchronous;
the resulting long application frame was passed into `FrameTimer`, which could
advance a short effect directly to its end. STR animation advancement is now
limited to 1/15 second per application frame, preserving the animation across
asset-loading stalls. A regression test covers a two-second loading frame;
client library tests are green (81 passed, 4 ignored), networking tests are
green (5 passed), and the release binary was rebuilt for live verification.

The next live pass verified the corrected Meteor Storm presentation. Frost
Nova still produced no caster animation; its damage packet's source identifier
did not resolve through the ordinary map-entity lookup. Frost Nova now falls
back to the authoritative local player entity before claiming its dedup key,
and missing-caster or STR-load failures are logged explicitly rather than
remaining silent. Release rebuilt for another live check.

Further live testing showed Frost Nova rendering when it damaged/killed a mob,
but not when cast with no enemies nearby. Hercules always calls
`clif->skill_nodamage` for Frost Nova before its area scan; Korangar previously
discarded that packet for every non-healing skill. The 0x09CB handler now emits
`SkillEffectNoDamage` for all skills (while lib.rs preserves the existing heal
number behavior), and Frost Nova's single caster-centered animation is driven
from that successful-use event rather than target damage. This restores an
animation even when the cast hits nothing. Tests: client 82 passed/4 ignored,
networking 5 passed; release rebuilt for live verification.

**Live-verified:** Meteor Storm now plays its falling-meteor animation at the
cast position, and Frost Nova plays its caster-centered animation both with
mobs in range and with no mobs nearby. The latter confirms 0x09CB successful
skill use—not per-target damage—is the required caster-effect trigger.

## Classic skill-effect coverage pass — caster batch 2

An audit of first/second/transcendent class recipes found the same trigger
families beyond Wizard. Hercules confirms Magnum Break (7), Raid (214), Meteor
Assault (406), and Ignition Break (2006) emit successful non-damage skill-use
notifications; they now use the same caster path that fixed empty-area Frost
Nova. Ignition Break loads its shipped `이그니션브레이크.str`. The other
three were procedural in the classic client, so `SkillBurst` adds reusable
expanding-cylinder, radial-streak, and eight-direction slash recipes using the
shipped `ring_yellow.tga`, `lens1.tga`, and `purpleslash.tga` textures. Each
recipe registers a matching point light.

Targeted Knight/Assassin attacks use damage as their available trigger and a
short-lived source/skill gate to avoid one caster animation per target. Pierce,
Brandish Spear, Spear Stab, Spear Boomerang, and Bowling Bash now load their
classic caster STRs; Brandish also plays its target sweep. Sonic Blow combines
the procedural expanding caster ring with `sonicblow.str` on the target.

All eight newly mapped STRs parse from the configured GRFs with zero
unconsumed bytes, and all four procedural textures were found in the GRFs.
Live GUI verification remains pending for this batch.

For repeatable GUI acceptance, the local `korangar` GM account now has a
four-character effect roster on `prt_fild07`: `EffectKnight` (Knight),
`EffectSinX` (Assassin Cross), `EffectStalker` (Stalker), and `EffectRune`
(Rune Knight). The `provision-effect-roster` headless scenario grants each
character its complete server skill tree, verifies all ten batch-2 skill IDs,
binds the relevant skills consecutively from F1, and supplies the necessary
sword, spear, or katar without duplicating existing items. The initial
provisioning run passed for all four characters.

## Knight weapon layers, classic recipes, and skill-range movement

The first Knight GUI pass exposed two independent omissions. Character packets
discarded the server's weapon and shield appearance, and the local player did
not refresh its appearance from equipped inventory. Both fields are now
promoted end-to-end. Equipped inventory derives the local weapon type, and an
appearance change reloads the entity ACT/SPR layers. Sparse ACT actions use an
empty frame instead of the format's `usize::MAX` sentinel, with an additional
render bounds check, so selecting the provisioned Knight cannot crash on a
missing weapon frame. The classic weapon-action table is zero-based: Knight
spear value 2 selects `Attack3`; Spear Boomerang deliberately keeps the
weaponless throwing action. Live inspection confirmed the spear weapon layer
and the Pierce damage packet's local source/action duration.

The follow-up visual pass compared all six provisioned Knight skills with the
official GRF assets and the independent roBrowserLegacy semantic tables. The
resulting layer matrix is recorded in
[KNIGHT_SKILL_EFFECT_RECIPES.md](KNIGHT_SKILL_EFFECT_RECIPES.md). Pierce,
Brandish Spear, Spear Stab, Spear Boomerang, and Bowling Bash combine the
weapon ACT with their caster/body/head STR, target STR or procedural hit, point
light, and classic spatial sound. Spear Boomerang also travels source-to-target
with the official spear sprite. Magnum Break uses its two original expanding
cylinders, sound, point light, and 50 ms camera quake. A focused ignored test
opens the configured official GRFs and confirms every referenced Knight
SPR/ACT, STR, texture, and sound exists.

Entity-targeted skills now share the normal attack/pickup movement recipe.
Clicking an out-of-range target computes a walkable path to the learned skill
range, buffers the exact skill id/level/range/target, and casts after movement
stops. If the target moves before arrival, the client recomputes the path and
keeps the same cast buffered. Removing the target clears the buffered action.
Range decisions use the server-compatible Chebyshev tile distance and have a
unit test covering melee, diagonal, and longer-range cases.

Validation after this pass: `cargo check -p korangar`; 83 client library tests
passed with 5 ignored; 5 networking tests passed; the focused official Knight
asset test passed; and the release client rebuilt successfully.
