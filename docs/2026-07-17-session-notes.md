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
  **Done post-review 2026-07-17**: `probe-thunderstorm` and
  `probe-knight-action` removed from the suite; `provision-effect-roster`
  stays as a permanent idempotent scenario, so the documented gate count is
  now 107. The `probe` login-table account was removed separately.

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

## Per-job classic weapon-action table

The Knight pass left `set_skill_attack` with a job-agnostic hardcode (spear →
Attack3, bow → Attack2, else Attack1). The classic client instead selects the
attack ACT action from a per-job — and for Novice, Wizard, Super Novice, and
Soul Linker per-sex — weapon table. Auditing that table (semantics
cross-checked against roBrowserLegacy's independent weapon-action table)
showed the hardcode was wrong for several provisioned or likely classes:
Hunter, Thief, Rogue, Bard, and Dancer bows use Attack3 (only Archer uses
Attack2), and Assassin katars use Attack3 — so `EffectSinX`'s Sonic Blow
would have played the Attack1 bash.

`classic_weapon_attack_action(job_id, sex, weapon)` in `world/animation/`
now encodes the full table for all first/second classes plus Gunslinger and
Ninja, with transcendent, baby, peco, and third classes folded onto their
parent row (Rune Knight → Knight, Guillotine Cross → Assassin, ...).
`AnimationState::weapon_attack` applies it; `set_skill_attack` routes every
player attack and skill through it, keeping the Spear Boomerang weaponless
throwing-action override and the critical → Attack3 selection for normal
attacks, and leaving monster/NPC behavior unchanged. Ten unit tests cover
the Knight spear/sword split, Archer-vs-Hunter bows, Assassin Cross katar,
the Wizard sex mirror, third-class inheritance, and the bare-hand and
Gunslinger fallbacks. Client library tests: 93 passed, 5 ignored. Live GUI
verification (EffectSinX katar action, batch-2 pass) is pending.

## Weapon sprite layer audit against the official GRFs

A new `weapon-sprite-audit` tool (`cargo run --release --bin
weapon-sprite-audit`, run from `korangar/korangar/`) probes every player job
folder × sex × candidate weapon name against the configured archives.
`get_files_with_extension` demonstrably under-reports the classic GRF (the
기사 folder listed 3 files while direct `file_exists` probes found 16), so
the audit's per-name probes are the source of truth.

Findings, all now fixed in `weapon_resource_suffix` /
`get_weapon_sprite_folder`:

- Two-handed swords, spears, and axes ship their own `양손검`/`양손창`/
  `양손도끼` sprites; the client previously reused the one-handed names.
- Classic rods/staves ship **no** weapon sprite (the original client draws
  nothing for a casting Mage/Priest staff) — views 10/23 now map to `None`.
- Long guns are per-class (`라이플`, `기관총`, `샷건`); no grenade-launcher
  sprite exists. Assassin dual-wield views 25..=30 map to the pair sprites
  (`단검_단검`, `검_검`, ..., matching the katar's `카타르_카타르` pattern).
- Transcendent second classes (로드나이트, 어쌔신크로스, 스토커, 스나이퍼,
  하이위저드, 팔라딘, ...) ship **no weapon sprites at all**; the classic
  client reuses the base second-class folders. Without the new folder alias
  the provisioned EffectSinX/EffectStalker weapon layers pointed at
  nonexistent files. Priest-family weapons live under `프리스트` (the body
  table's name for job 8 is `성투사`), and Royal Guard's under `로얄가드`.
- `push_weapon_part_file` now verifies the exact SPR exists before adding
  the layer — a missing combination renders no weapon, like the original,
  instead of the placeholder fallback sprite.

The ignored `loads_classic_weapon_layers_for_roster` test pins the exact
paths for the effect roster (Knight spears/swords, Assassin katar + duals,
Stalker-via-Rogue, Rune Knight, Priest, Hunter) and passes against the
configured GRFs.

## Classic skill sounds confirmed and wired

`probes_classic_skill_sound_candidates` (ignored diagnostic) probed the
classic sound names. Confirmed and wired: Fire Bolt volley
(`ef_firearrow1-3.wav`, random per cast), Cold Bolt (`ef_icearrow.wav`, its
classic hit is sound-only), Napalm Beat + Bash (sound-only; their visuals
are code-drawn), Soul Strike, Frost Diver, Fireball, Sonic Blow
(`assasin_sonicblow.wav` — kRO's own spelling), and ground casts Firewall
(`ef_firewall.wav`), Thunderstorm (`ef_thunderstorm.wav`), Storm Gust
(`storm.wav`). Hit sounds play at the struck entity via `skill_hit_sound`;
ground sounds at the cast position via `ground_skill_sound`. Not found under
any probed name (still silent): Lightning Bolt, Meteor Storm, Lord of
Vermilion, Frost Nova, Fire Pillar, Quagmire, Sanctuary, Magnus, Holy
Light, Turn Undead, Hammer Fall, the Hunter traps, Venom Dust, Raid,
Meteor Assault, Ignition Break.

The new ignored `all_mapped_skill_effect_assets_exist` test walks every
skill-effect table (hit STRs, ground STRs, bolt volleys, both sound tables,
plus the hardcoded caster-recipe assets) and asserts each referenced file
exists in the GRFs — it passes, so nothing the tables can request is
missing. Validation: 98 client library tests passed (8 ignored GRF-backed
tests all green when run explicitly), release rebuilt.

## Live-test round 1: repeating attack sound → the full classic combat loop

The first live pass confirmed the spear weapon layer but exposed a repeating
sound after each swing. ACT ground truth (`reports_knight_attack_frame_structure`,
new ignored dump): Knight body Attack3 has 8 frames with `attack_spear.wav`
on frame 5, and the weapon/head ACTs mirror the body frame-for-frame — no
frames were missing. The repetition was the 200 ms `SoundState` cooldown:
an attack stretched over a slow attack delay displays its event frame longer
than the cooldown, so the event re-fired. Frame events now fire exactly once
per displayed frame occurrence (`SoundToken` = animation start + frame
position; stable on a held final frame, fresh on every loop pass).

Cross-checking roBrowserLegacy's ZC_NOTIFY_ACT handler exposed two further
gaps, both now implemented: the attacker drops into the looping READYFIGHT
battle stance after the swing (monsters fall back to idle — they have no such
action), and the damaged entity plays its HURT flinch stretched over the
packet's dMotion, then READYFIGHT. `NetworkEvent::DamageEffect` gained
`damage_delay` (dMotion) from all five packet registrations, and the skill
packet's attacker duration was corrected from `max(sdelay, ddelay)` to
`sdelay`. No flinch on miss, dMotion 0 (Endure), death, or while walking.
Zero-duration division guards added.

New: [ANIMATION_SYSTEM.md](ANIMATION_SYSTEM.md) documents the whole actor
animation system (layers, ACT structure, action tables, timing, combat loop,
diagnostics, known gaps — including the unrendered `_검광` weapon trails and
per-item weapon sprites the GRFs ship).

Validation: client 98 passed / 9 ignored (all ignored GRF tests green when
run explicitly), networking 5 passed, headless examples compile, release
rebuilt. Live GUI verification of the stance/flinch/sound loop pending.

## Class-wide animation audit → per-layer frame synchronization

Live round 2 confirmed swing, fight stance, and mob recoil. To answer "do we
understand every class?", the new `animation-audit` tool parses every player
job's body, head, and weapon ACTs (both sexes) and compares per-direction
frame counts across layers. Result: ~200 action groups where layers disagree.
Most are in actions the job×weapon table never selects (e.g. sword frames
under the bare-hand Attack1), but the gameplay-relevant set included: first
class crit swings (weapon 1–2 frames short, some male directions only 1
frame), Archer crit bow (5 vs 8), Novice/Super Novice melee (6 vs 9), and
Merchant two-handed axe (weapon 10 vs body 9 — frames were being dropped).
Knight, Hunter, Assassin, and the provisioned effect roster all match
exactly, which is why the Knight pass looked right.

The original client drives all layers with one clock but indexes each
layer's own action, so the merge now takes the longest layer's count per
action, holds a finished layer's last frame in one-shot actions (image only,
sound event suppressed on held frames), and wraps each layer at its own
cycle in looping actions (`layer_motion_index` + `is_looping_action_group`,
unit-tested). Previously the merge truncated everything to the body's count,
deleting longer layers' frames and blanking shorter layers mid-swing.

[ANIMATION_SYSTEM.md](ANIMATION_SYSTEM.md) updated (new tool, new merge
semantics, attach-point caveat). Tests 101 passed / 9 ignored; release
rebuilt; live check of a mismatched combo (e.g. female Novice or any crit)
pending.

## Native Ragexe trace supersedes two earlier timing/layer assumptions

The preceding live-pass sections are a historical record of the intermediate
implementation, not the final native contract. Direct tracing of the named
2019 Ragexe established these corrections:

- Source attacks are not stretched to packet `sMotion`. Basic and skill
  source actions run on their natural `ACT delay × 24 ms` clock. Korangar now
  follows this source-clock rule.
- The original client does not merge to the longest layer, hold a short
  layer's last motion, or let each layer wrap independently. The body owns the
  action clock and motion index. Each secondary ACT receives that index;
  `CActRes::get_motion` (`0x004F5360`) returns motion 0 when a non-empty layer
  is shorter, and longer-layer motions beyond the body count are unreachable.
  The loader and tests now reflect this exact fallback.
- Target Hurt is not applied when the packet arrives. The source queues a due
  impact using `round(actor[0x24C] × ACT_delay × 24 ms)` plus narrow
  actor-specific constants. At impact, native code derives
  `reaction_cycles = dMotion / 288.0`: short reactions accelerate the target
  clock, while values above one play naturally and hold the last motion.
  Korangar implemented that target clock first; the pending-impact start
  boundary was still open at this point and is completed in the section below.
- Player normal and critical attacks use the same exact `0x009A2DB0`
  job/sex/weapon selector. `0x00991580` returns attack-event position, not an
  animation-speed factor. The selector and weapon normalization are now
  implemented and unit-tested.
- `ZC_NOTIFY_SKILL2` dispatches through a distinct native controller
  (`0x008C1390`). `0x009B4B30` selects a source actor state by skill ID; it
  does not blanket-reuse the normal weapon action. The complete non-default
  `u16` table, state resolver, player state-7 job guards, packet layout,
  damage-type routes, and native addresses are recorded in
  [combat-animation-pipeline.md](specs/combat-animation-pipeline.md).

`korangar/tools/native_skill_state_audit.py` hash-checks the reference
executable and regenerates the exhaustive actor-state catalog. Spear Boomerang
skill 59 is proven state 12 → Attack1; its former throwing/Attack2 override was
corrected. Current validation: 109 passed, 10 ignored, 0 failed.

## Delayed target-impact runtime boundary

The first native actor-message timing boundary is now implemented. Every
`DamageEffect` retains the packet's authoritative tick and the complete source
and target motion inputs. Packet dispatch immediately starts the source ACT
and caster/travel presentation, then calculates a local due offset from the
selected body ACT: player weapon Attack2/Attack3 uses the exact `0x00991580`
marker; shared actor actions scan the body for `"atk"` and use native
`motion_count - 2` when no marker is authored. The actor-family constants
recovered from `0x008BFB00`/`0x008C1390` are included.

`PendingImpactQueue` owns the wrap-safe due tick. At that boundary the client
emits the damage number or miss, target hit STR, hit sound, and Hurt together;
Hurt then follows the already implemented `dMotion / 288.0` clock. Non-death
entity removal cancels queued target work. Death retains the actor's final
position for the number/effect but suppresses Hurt. Map change and disconnect
clear the queue, and due work drains after a whole received packet batch so
same-batch world changes win.

Fast tests pin due-boundary ordering, `u32` tick wraparound, target removal,
queue clearing, player event-position constants, the shared `"atk"` fallback,
and the 24 ms conversion. The remaining fidelity limitation is upstream:
until the exhaustive native skill actor-state catalog is used by the runtime,
most skills still choose a provisional weapon action and therefore can have a
provisional ACT-derived due offset even though launch/impact phase ownership is
now correct.

Validation after the boundary: 118 client library tests passed with 10
archive-dependent tests ignored, all 5 networking tests passed, and
`cargo check -p korangar` plus the optimized release build completed
successfully. Only the pre-existing unused-method warnings remain.

## Native skill actor-state runtime boundary

The exhaustive `u16` catalog recovered from Ragexe `0x009B4B30` now drives
every skill-damage source action. The implementation preserves the resolved
flat ACT group in `AnimationState`, so rendering, completion, sound selection,
and impact scheduling no longer reinterpret a shared native group through the
player/monster semantic action table.

The runtime mirrors both native dispatch layers: shared state-to-group
selection and what this intermediate pass identified as the player
`0x00977430` intercept. Player states 2 and 9 use the
exact job/sex/weapon selector; state 7 selects Skill or its job-specific
ReadyFight route, writes event position 6.0, honors the sitting-state guard,
and holds Skill motion 0 for 400 ms for the Monk-family special route. State
26 applies its four job overrides. Current-dead and all 24 no-action-sentinel
skills preserve the active animation unchanged. Non-player default state 7
keeps native flat group 2 rather than being remapped through a player action
name.

The catalog has a full-domain regression: all 65,536 IDs must reproduce the
reference audit's exact state distribution. Resolver tests cover every table
row, range boundaries/defaults, player job routes, weapon interception,
sentinel and precedence behavior, monster resolution, state 26, and the held
motion clock. Focused animation tests and the full client library suite pass.

This closes actor-state and flat-action selection, not the complete skill
presentation pipeline. Bespoke native motion programs used by several shared
states beyond the flat Skill group, the actor `0x16C` guard, complete
post-action precedence, and per-skill effect/projectile/audio/cleanup recipes
remain separate boundaries.

Validation after this boundary: 127 client library tests passed with 10
archive-dependent tests ignored, all 5 networking tests passed,
`cargo check -p korangar` completed, and the optimized release client rebuilt.
Only the pre-existing unused-method warnings remain.

The later higher-state pass below corrected this address/class attribution:
RTTI identifies `0x00977430` as `CGrannyPc`, while Korangar's classic SPR/ACT
actors use `CPc::SetState` at `0x009779A0`. The final runtime and specification
supersede the intermediate state-7 job table recorded in this section.

## Higher motion-program and post-completion runtime boundaries

Both remaining actor-state runtime boundaries are now implemented. The generic
dispatcher at `0x008BAB30`, its range helper `0x008BA040`, sequence helper
`0x008B9E90`, and core update `0x008AC2E0` establish a clock above normal ACT
playback. `NativeMotionProgram` now represents every installed program for
states 16..51: exact `(action, motion)` steps, actor-update cadence, independent
wall deadline, hold/cycle/ping-pong behavior, job variants, and random
branches. Rendering, ACT impact lookup, completion, and sound identity consume
the current program step. Duplicate authored motions are distinct event
occurrences, while a terminal held motion does not replay its sound.

RTTI corrected the classic-player intercept before this implementation:
Korangar follows `CPc::SetState` at `0x009779A0`, not `CGrannyPc` at
`0x00977430`. The runtime now includes the exact CPc state-2 70/30 job
override, state-7 action routes and three custom program families, and state-8
exceptions. Generic state 49 installs its program but normalizes logical state
to 7.

The post-playback transition is now keyed by logical native state rather than
the displayed ACT group. States `2/4/5/7/9/12/45` request state 0; classic
state 51 first becomes state 2; other higher states hold. CPc state 0 displays
ReadyFight while remaining logical state 0 after previous states `2/4/8`, so
basic attack/Hurt return to stance but Attack1-looking state 12 returns to
Idle and Attack3-looking state 18 holds. The full program and completion
tables, native fields, addresses, correction ledger, and still-open
request-time guards are in
[combat-animation-pipeline.md](specs/combat-animation-pipeline.md).

Validation after both boundaries: 139 client library tests passed with 10
archive-dependent tests ignored, all 5 networking tests passed,
`cargo check -p korangar` completed, `git diff --check` was clean, and the
optimized release client rebuilt. Only pre-existing unused-method warnings
remain.

## Request-time guards, input precedence, and typed skill recipes

The final runtime boundary resolves the three actor fields left open above
against the same reference executable:

- `+0x16C` is owned by `SI_TRICKDEAD` (status 29). The status branch at
  `0x008B3C62` sets a held death-looking pose without entering real death;
  generic `SetState` (`0x008BAB59`) and `CPc::SetState` (`0x009779D4`) reject
  later ordinary action requests while the field is set.
- `+0x114` is owned by `SI_SUHIDE` (933). Its branch at `0x008B6855` installs
  state 48, and the classic-player state-7 branch tests the field at
  `0x00977C2B`. It therefore blocks only native state-7 skill actions, not
  every actor request. Adjacent `SI_SU_STOOP` (893) installs state 47 and
  yields to SU Hide on removal.
- `+0x2C0` is packet `isPKModeON`, written by state-change and spawn paths and
  tested by `CPc::SetState(0)` at `0x00977BF6`. It is preserved across entity
  creation and the complete `ZC_STATE_CHANGE` option/body/health/PK record is
  applied atomically. A neutral PK-mode player displays ReadyFight.

Stone/freeze opt1 now pauses and resumes the selected ACT clock without
stealing movement, casting, or incoming action ownership. Hurt is accepted
while walking and leaves the trajectory intact. Movement does not cancel the
cast overlay, preserving Free Cast. Cast termination is authoritative:
`ZC_DISPEL` (`0x01B9`), failed local use acknowledgement, execution, expiry,
Trick Dead, death, and map/session cleanup own it. Damage, successful
no-damage, and ground-skill execution clear the cast before requesting the
source actor action, so a Trick Dead/death pose guard cannot leave a stale cast
bar.

All pre-existing skill-ID switches for effects, projectiles, and sounds have
been consolidated in `world/skill_recipe.rs`. Each typed recipe independently
declares successful-caster, damage-caster, projectile, scheduled target/hit,
and ground phases. An absent effect does not suppress same-phase audio or a
projectile. Random assets enumerate every variant for deterministic GRF
audits. The registry contains the 42 previously evidenced skill mappings;
unknown IDs return an explicit empty presentation contract while still using
the exhaustive native actor-state resolver. This is the extensible boundary
for the remaining player, monster, mercenary, homunculus, and NPC skill
catalog—not a claim that all RO presentation recipes are complete.

The canonical behavior, address ledger, packet ownership, precedence table,
typed recipe schema, current coverage, acceptance rules, and remaining gaps
are now maintained in [ANIMATION_SYSTEM.md](ANIMATION_SYSTEM.md) and
[combat-animation-pipeline.md](specs/combat-animation-pipeline.md).

## Phase-A live pass round 1: local-player animation loads were dropped

The first animation-fidelity phase-A GUI pass (EffectSinX vs a Barricade)
found the katar rendering as a bare-hand "punch" even though the packet log
proved the wire and derivation were right (`local equipped weapon=16`, katar
part file requested, impacts due on schedule). Root cause: the async-load
completion handler (`update_loaded_resources`) delivered finished
`AnimationData` to the `entities()`, `dead_entities()`, and `ground_items()`
lists — never to `this_entity`. The char-select body/head set is cached, so
the local player spawns fine; the first-ever load of a weapon-bearing part
set (login `SetInventory` or re-equip) completed and was silently discarded,
leaving the local actor without a weapon layer forever. Remote actors were
unaffected, and headless testing structurally cannot see this class.

Fixed with the missing `this_entity` branch. Live-verified: the Jur katar
renders in both hands, the Attack3 thrust reads as a blade stab, and EDP
(with Poison Bottles; fail reason 71 is `USESKILL_FAIL_NEED_ITEM`) applies
its 80 s status with visibly boosted damage. Sonic Blow's white→red glyph is
authentic — `sonicblow.str` uses the `myul_a/b` (멸/滅) textures. EDP has no
cast visual yet (unmapped recipe, phase E).

The rest of round 1 completed the same day, finding one more visual bug:
the Raid/Meteor Assault `SkillBurst` streaks were tuned so faint (thin dark
additive quads at ≤0.45 alpha) that both skills read as a bare point light.
After brightening (central flash + wider ice-blue streaks; brighter purple),
both bursts are live-verified visible. Also verified: EffectStalker's
Rogue-alias weapon layer, Hiding, and the authentic weaponless ReadyFight
skill exit; EffectRune's weapon layer and Ignition Break's classic STR;
EffectKnight's Magnum Break cylinders/knockback and spear-equipped
Pierce/Spear Stab (an initial "no spear" report was a sword-equipped cast);
and the female-Novice mismatched-layer combo (no blanking, hurt flinch
plays). Raid's "Skill level not high enough" failure text is Hercules'
generic `USESKILL_FAIL_LEVEL` for its `State: "Hiding"` requirement — a
better client message is future polish. One watch item remains: a single
unreproduced logout-to-char-select panic (rust-state safe-selector unwrap on
`None`); the client now runs with `RUST_BACKTRACE=1` during GUI passes to
capture it if it recurs. The roster gained a female `EffectNovice` (slot 1)
and per-character consumables (EDP Poison Bottles) in provisioning.

## Phase B started: crossed-motion event cursor (implementation only)

After the round-1 pass closed, phase B of
[plans/animation-fidelity.md](plans/animation-fidelity.md) was implemented:
`FrameEventCursor` in `AnimationState` walks every ACT motion crossed since
the previous actor update (including loop wrap), replacing the displayed-frame
`SoundToken`/`SoundState` dedup that could drop an event when a slow frame
skipped motions. Events are delivered before the completion transition (a
new playback identity would discard final-frame crossings), motion-program
steps fire once per `step_serial` occurrence, and stall recovery is bounded
to one full cycle as an audio-flood guard (deliberate deviation from the
unbounded native walk). Compiles clean; the pre-existing 151-test suite is
green. **Phase B is not closed**: the dedicated cursor unit tests, the
ANIMATION_SYSTEM.md §5.3 update, and a live sound check are still pending.

Final validation for the complete worktree:

- Korangar library: 148 passed, 10 archive-dependent tests ignored;
- networking: 7 passed;
- packet layouts: 47 passed;
- `all_mapped_skill_effect_assets_exist`: passed against the configured GRFs;
- `cargo check --workspace --all-targets`: passed;
- optimized `korangar` release build: passed;
- `git diff --check`: clean.

Only the pre-existing unused-method warnings remain.
