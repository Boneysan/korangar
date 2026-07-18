# Combat and skill animation pipeline specification

Status: living implementation specification
Reference client (relative to the repository root):
`../../RO/client/2019-06-05fRagexe_patched.exe`
Reference SHA-256: `61663a6f3bca42e992e3d61418b9508db57e6ba18cc0069e297eb3730d4d825d`
Reference data order: `renewal2021.grf`, `resources2021.grf`, `data.grf`,
`rdata.grf`
Last native-client research pass: 2026-07-17

## 1. Purpose

This document defines the presentation pipeline Korangar needs in order to
reproduce Ragnarok Online combat for:

- every player job, sex, weapon, shield, mount, and cosmetic combination;
- normal, critical, ranged, and multi-hit attacks;
- active, support, ground, channeled, movement, summon, trap, and status
  skills;
- monsters, bosses, mercenaries, homunculi, pets, NPC combatants, and other
  actor types;
- actor sprites, skill effects, projectiles, damage/heal numbers, sounds,
  lights, camera motion, persistent units, and cleanup.

It is both a fidelity specification and a recipe format for implementing
skills. It deliberately separates facts recovered from the client from
Korangar's current behavior. A skill is not complete merely because an STR is
visible: its trigger, actor action, timing, spatial anchor, repetition,
reaction, audio, and lifetime must also agree.

The actor-format details shared by non-combat animation are summarized in
[`../ANIMATION_SYSTEM.md`](../ANIMATION_SYSTEM.md).

## 2. Evidence policy

Every rule and every per-skill recipe must carry one of these evidence grades:

| Grade | Meaning |
|---|---|
| `NATIVE` | Proven in the named Ragexe by disassembly, runtime observation, or both. Record function/handler addresses or a capture reference. |
| `DATA` | Read directly from the configured GRF asset, ACT/SPR/STR, Lua/Lub, or Hercules packet definition. Record the exact path or packet. |
| `OBSERVED` | Reproduced in a controlled reference-client capture, but its internal rule has not yet been recovered. Record the build, actors, skill level, equipment, and timestamps. |
| `INFERRED` | Best current interpretation supported by multiple pieces of evidence. It must not be described as verified. |
| `KORANGAR` | Describes the current implementation only. It is not evidence of original-client behavior. |
| `UNKNOWN` | Not researched or evidence conflicts. The recipe remains incomplete. |

Third-party clients can suggest search targets, but cannot promote a rule above
`INFERRED`. Asset existence proves that an asset ships; it does not prove
which packet triggers it, where it is placed, or how it is timed.

When different official builds disagree, rules are versioned by client build.
The reference above is the compatibility target until a different target is
chosen explicitly.

## 3. Definitions

- **Actor action**: one body animation such as walk, attack, hurt, skill, or
  death. Player actions may be rendered from several ACT/SPR resources.
- **Action index**: a flat ACT action number. Directional actor animations are
  normally stored in groups of eight.
- **Motion**: one frame record inside an ACT action.
- **Presentation**: every visual, audio, camera, and actor-state consequence of
  a gameplay event. Presentation never decides damage or gameplay outcome.
- **Recipe**: the declarative definition that maps normalized combat events to
  synchronized presentation tracks.
- **Track**: an independently timed actor, effect, projectile, audio, camera,
  UI, reaction, persistent-unit, or cleanup operation.
- **Occurrence**: one server-authoritative use of a skill or attack. One
  occurrence may produce many packets, targets, hits, cells, and effects.
- **Anchor**: the entity, tile, world point, bone/attach point, or interpolated
  path that owns a presentation element's position.
- **Authoritative clock**: the clock that advances a track. Actor ACT time,
  packet timestamps, effect-key time, and persistent-unit lifetime are not
  interchangeable.

## 4. Required end-to-end pipeline

```text
server packets / local prediction / replay
                 |
                 v
        packet-version decoder
                 |
                 v
      normalized combat events
                 |
       +---------+----------+
       |                    |
       v                    v
 occurrence correlator   actor state machine
       |                    |
       v                    v
 presentation recipe    layered ACT runtime
       |
       +--> caster tracks
       +--> travel/projectile tracks
       +--> impact and number tracks
       +--> ground/persistent-unit tracks
       +--> status/attachment tracks
       +--> audio/light/camera tracks
       +--> reaction and cleanup tracks
                 |
                 v
     effect/actor/particle render queues
```

The packet decoder must preserve all fields needed by presentation. The
normalizer may make packet versions uniform, but must not collapse distinct
semantic events such as cast start, successful use, ground placement, damage,
unit creation, status application, and unit removal.

The recipe evaluator must be data driven. Packet handlers should publish
facts; they should not contain growing skill-ID switch statements.

## 5. Server-to-presentation event model

### 5.1 Events that must remain distinct

| Normalized event | Important fields | Typical presentation responsibility |
|---|---|---|
| `CastStarted` | occurrence key, source, skill, level, target/entity or tile, start tick, cast duration | cast bar, cast aura, chant, pre-cast actor state |
| `CastCancelled` | occurrence/source/skill, reason, tick | stop cast tracks and clear actor cast state |
| `SkillUseSucceeded` | occurrence, source, destination, skill, level/value, result, tick | caster-centered and no-damage effects; must exist even when no target is damaged |
| `GroundCastPlaced` | occurrence, source, skill, level, tile, server start tick | initial ground-cast effect and sound |
| `DamageApplied` | occurrence, source, target, skill optional, damage/miss, hit count or hit list, source motion, target motion, damage type, server tick | source action, impact tracks, numbers, target reaction |
| `HealApplied` | occurrence, source if known, target, skill if known, amount, tick | heal effect/number and support actor action |
| `SkillUnitCreated` | unit entity, creator, skill/unit type, level, tile, range, visibility, tick | persistent cell/trap/aura instance |
| `SkillUnitRemoved` | unit entity, reason, tick | exact instance teardown/fade |
| `StatusChanged` | target, status, gain/loss, duration, values, tick | persistent actor overlay, tint, pose, icon, or sound |
| `ActorOptionChanged` | actor, option/body/health state bitfields, tick | hide/cloak, freeze, stun, poison, mount and other state presentation |
| `ActorMovedInstantly` | actor, old/new position, tick | movement-skill trail and endpoint effect when applicable |
| `ActorSpawned/Removed/Died/Revived` | actor identity and reason | spawn/death/revival action and cleanup ownership |

### 5.2 Current 2022 packet inputs

These are current Korangar inputs, not a claim that the native 2019 client
uses only these packet versions:

| Packet | Current normalized event | Fidelity notes |
|---|---|---|
| `ZC_NOTIFY_ACT` (`0x08C8` for the configured version) | `DamageEffect` | Basic attack, critical, miss, pickup, sit, and stand share a damage-type family. Native handler variants `0x008A`, `0x02E1`, and `0x08C8` normalize into the same internal record at `0x0091A4F0`; preserve the exact damage type rather than reducing it to two booleans. |
| `ZC_NOTIFY_SKILL2` (`0x01DE`) | `DamageEffect` | Native handler `0x00946260` reads: `+0x02 u16 skill`, `+0x04 u32 source`, `+0x08 u32 target`, `+0x0C u32 tick`, `+0x10 u32 sDelay`, `+0x14 u32 dDelay`, `+0x18 i32 damage`, `+0x1C i16 level`, `+0x1E i16 div`, and `+0x20 u8 type`. It dispatches source actor message `0x25`; no field may be discarded before recipe evaluation. Per-type miss/endure/multi-hit semantics still require a complete branch table. |
| `ZC_USESKILL_ACK` (`0x07FB`/`0x0B1A`) | `SkillCast` | Cast start/ack; the newer packet also carries attack-motion time. Zero-duration uses still matter to occurrence correlation even if no cast bar is shown. |
| `ZC_DISPEL` (`0x01B9`) | `SkillCastCancelled` | Carries the actor ID whose active cast must end. |
| `ZC_ACK_TOUSESKILL` (`0x0110`) | `SkillCastCancelled` plus failure message | Failure terminates the local cast even though this packet supplies no actor ID. |
| `ZC_USE_SKILL` (`0x09CB`) | `SkillEffectNoDamage` | Correct source for successful no-damage and empty-area caster effects. |
| `ZC_NOTIFY_GROUNDSKILL` (`0x0117`) | `GroundSkillEffect` | Carries source, skill, level, tile, and server start tick. All are retained; the current STR backend still starts on receipt and must adopt the retained tick. |
| `ZC_SKILL_ENTRY` (`0x09CA`) | `AddSkillUnit` | Carries creator, unit type, range, visibility, and level. Current event discards several of these fields and must be expanded. |
| `ZC_SKILL_DISAPPEAR` (`0x0120`) | `RemoveSkillUnit` | Teardown is keyed by unit entity ID. |
| `ZC_STATUS_CHANGE` (`0x0983`/`0x043F`) | `StatusChange` | Status values can select presentation variants; retain them. |
| `ZC_STATE_CHANGE` (`0x0229`) | `StateChange` | `bodyState`, `healthState`, `effectState`, and `isPKModeON` are one atomic actor input. Korangar stores all four, applies conceal options, pauses the ACT clock for stone/freeze opt1, and uses PK mode for neutral ReadyFight selection. |

Packet start ticks must be converted through the synchronized client tick.
Application-frame receipt time is not a substitute. Replay and live play must
produce the same sequence.

## 6. Occurrence correlation and deduplication

An area or multi-hit skill may emit one successful-use event, one ground event,
many unit events, and many damage packets. A fixed `(source, skill, 500 ms)`
gate is not a general occurrence identity.

The correlator must produce an `OccurrenceId` using the strongest available
identity, in this order:

1. an explicit server instance or unit entity ID;
2. source + skill + authoritative packet start tick;
3. source + skill + target/tile + a bounded, skill-specific correlation
   window;
4. a generated ID for events that genuinely cannot be correlated.

Recipes declare dedupe scope per track:

- `PerOccurrence`: one caster flash, chant, or camera shake;
- `PerTarget`: one target impact for each affected actor;
- `PerHit`: one bolt, slash, number, or sound for each hit;
- `PerCell`: one persistent ground cell;
- `PerPulse`: one effect on each server pulse;
- `PerStatusLifetime`: one attached aura until status loss;
- `NeverDedupe`: intentionally independent repeated events.

Correlation must not suppress a fast second cast of the same skill. It must
also tolerate packet reordering by buffering only for a small, documented
window and by making late tracks seek to their authoritative time.

## 7. Native actor animation contract

This section is `NATIVE` for the named executable unless marked otherwise.

### 7.1 ACT resource model

`CActRes` is parsed at `0x004F1C20`. It accepts an `AC` signature and ACT
versions through 2.5. The recovered object contains:

- actions at `+0x110..+0x118` (24-byte action records);
- event strings at `+0x120..+0x128` (24-byte strings);
- per-action float delays at `+0x12C..+0x134`;
- 68-byte motion records, with the event index at motion `+0x30`.

Relevant accessors:

| Address | Contract |
|---|---|
| `0x004F5140` | Return action or a static empty action. |
| `0x004F5280` | Return action delay; default to `4.0` when absent. |
| `0x004F52D0` | Return event string. |
| `0x004F5360` | Return requested motion. If the index is out of range and the action is non-empty, return motion 0. Empty actions return a static empty motion. |
| `0x004F5560` | Return motion count, with a minimum result of one for counts at or below one. |

Empty authored motions must therefore remain in the timeline even when they
contain no sprite clips. Removing them shifts visuals and events.

### 7.2 Player action groups

Player body ACTs conventionally use these base indices:

| Flat base | Group | Meaning |
|---:|---:|---|
| `0x00` | 0 | Idle |
| `0x08` | 1 | Walk |
| `0x10` | 2 | Sit |
| `0x18` | 3 | Pickup |
| `0x20` | 4 | Ready-fight stance |
| `0x28` | 5 | Attack1 / generic attack route |
| `0x30` | 6 | Hurt |
| `0x38` | 7 | Freeze1 |
| `0x40` | 8 | Die |
| `0x48` | 9 | Freeze2 |
| `0x50` | 10 | Attack2 |
| `0x58` | 11 | Attack3 |
| `0x60` | 12 | Skill |

Directions are resolved by adding the actor's direction-specific offset to a
base action. Do not assume every non-player resource has the player group
layout; its state-to-action function and data must be audited by actor family.

### 7.3 Action state and clock

The core actor has:

- base action at `+0x34`;
- direction-resolved action at `+0x38`;
- body motion index at `+0x3C`;
- ACT delay/timing value at `+0x58` and additional factors at `+0x60/+0x64`;
- playback mode at `+0x6C`;
- higher actor state at `+0x70`;
- action start tick at `+0x8C`;
- body SPR and ACT resources at `+0x104/+0x108`;
- finish/clamp flags at `+0x9D/+0x9E`.

The SetAction-like virtual at `0x008B7DC0` reads the selected body's ACT
delay, resets the motion and flags, and installs the playback mode. The core
update at `0x008AC2E0` calculates:

```text
act_time = elapsed_milliseconds / 24.0
raw_motion = floor(act_time / actor_delay)
```

Timing modifiers stored on the actor can change `actor_delay`. Mode 0 loops
with body motion-count modulo. Mode 1 is one-shot and clamps to the last body
motion after a cycle. The higher-state helpers now recovered for this boundary
are:

- `0x008BA040` (eight arguments) installs one primary action, a start/maximum
  motion range, a terminal `(action, motion)` pair, timed/deadline and repeat
  flags. It writes the primary action at `+0xB5`, start motion at `+0xB6/+0xC5`,
  maximum at `+0xC1`, terminal pair at `+0xC3/+0xC4`, repeat at `+0xCC`, timed
  mode at `+0xD0`, enables range mode at `+0xC8`, and initializes the update
  counter at `+0xE0` to one.
- `0x008B9E90` (thirteen arguments) installs five exact `(action, motion)`
  pairs at `+0xB7..+0xC0`, timed/deadline and repeat flags, marks sequence mode
  at `+0xD8`, and initializes `+0xE0` to one.

`0x008AC2E0` advances either program by an actor-update-call cadence stored at
`+0xC2`; this cadence is not ACT delay time and must not be converted to
milliseconds. Its independent deadline uses wall ticks. A non-repeating
program holds its last selected step after exhaustion/deadline. Repeating
range programs cycle; state 16's zero terminal-action marker produces
ping-pong order. Each selected step is an event occurrence even when two
adjacent steps contain the same motion number.

Packet `sMotion` is not a source-action playback duration. Both ordinary
damage dispatch (`0x008BFB00`) and skill damage dispatch (`0x008C1390`) select
the source action and leave its ACT clock natural. The source's attack-event
position, action delay, and a 24 ms unit determine when the native client
queues the impact; the packet field must not linearly stretch every source
motion.

Packet `dMotion` controls the target reaction after the delayed impact event.
`0x008BFB00` converts it to `reaction_cycles = dMotion / 288.0` and stores that
float on the queued event. When message `0x6F` applies the reaction:

- for `0 < reaction_cycles <= 1.0`, a player-job actor submits
  `effective_delay = ACT_delay * reaction_cycles`;
- for `0 < reaction_cycles <= 1.0`, a non-player actor submits
  `effective_delay = reaction_cycles` directly;
- for `reaction_cycles > 1.0`, the ACT delay stays natural, the one-shot runs
  through its first cycle, and its final motion is held until the completion
  threshold reaches `reaction_cycles`.

The delay setter at `0x008BA540` rejects values at or below its tiny lower
bound, so a zero `dMotion` must not install a zero-length frame clock. The
damage-type and actor-state guards still decide whether a target reaction is
requested at all. The player-job predicate is exactly `0x009A3400`: raw job
IDs `0..=30` or `4001..=5999`; it is not inferred from the actor object's
Korangar entity type.

Thus a blanket `duration = dMotion` interpolation can have a similar total
length in some cases but is not the native clock. It also cannot substitute
for scheduling the target reaction at the impact time.

### 7.4 Layer composition

Players are rendered from separate resources, including body, head, weapon,
shield/cosmetics, and optional weapon trails. The native renderer does not
first flatten all parts to one longest timeline.

The body ACT owns the actor clock and motion count. Each visible layer is
queried with the same direction-resolved action and body motion index. The
`CActRes` accessor then provides the exact fallback:

- a shorter non-empty layer displays its motion 0 when the body index is out
  of range;
- a longer layer's extra motions are unreachable;
- an empty action contributes no visible clips;
- a secondary layer does not wrap at its own length and does not hold its last
  motion.

Attach points align child parts to their parent. Native rendering has special
attachment/action cases, including use of motion 0 for some low action
indices. These need a dedicated render-function trace before the composition
system can be called complete.

Permanent pre-merging is an architectural compromise. Exact equipment swaps,
layer ordering, attach fallback, shields, per-item weapon models, and
`_검광` weapon-trail layers are best implemented by preserving layer ACTs and
composing the selected motions at render time.

### 7.5 ACT frame events

The event scanner at `0x008AC860` is body-owned. It remembers the prior body
action and motion, scans every crossed body motion in order (including loop
wrap), and dispatches `.wav` event strings. It does not merely inspect the
frame visible on the current application update. `0x008A9500` separately
searches the body action for the exact `"atk"` event.

Required behavior:

- only body ACT events drive actor event dispatch unless another native path
  is proven for a specific auxiliary layer;
- process all crossed motions exactly once, even after a slow application
  frame;
- process wraparound in chronological order;
- reset the cursor when the base action changes;
- never replay an event just because a one-shot final frame is held;
- resolve `"atk"` through the actor/weapon sound rule; resolve `.wav` through
  the asset namespace and spatial audio system.

Korangar's current displayed-frame token prevents repeated sounds, but it can
miss events when more than one motion is crossed between updates. Replace it
with a crossed-motion event cursor.

### 7.6 Player attack selection

The classic sprite-player state function (`CPc`) at `0x009779A0` routes both ordinary and critical
basic-attack notifications through the same selector. Damage type does not
select Attack3. `0x009A2DB0` returns false for flat Attack2 (`0x50`) or true
for flat Attack3 (`0x58`) after `GetRealWeaponId` normalizes expansion weapon
appearance IDs. The exact predicates are:

| Native row | Attack3 when normalized weapon is... | Otherwise |
|---|---|---|
| Swordman family | `4` or `5` | Attack2 |
| Magician family | `1` | Attack2 |
| Archer base row | anything except `11` | Attack2 |
| Bow-user family | `11` | Attack2 |
| Priest family | `15` | Attack2 |
| Wizard family | male with `1`, or female with `10`/`23` | Attack2 |
| Smith family | `2` or `6..=8` | Attack2 |
| Assassin family | `16` or `25..=30` | Attack2 |
| Monk family | `0` or `12` | Attack2 |
| Sage family | `5`, `10`, `15`, or `23` | Attack2 |
| Novice family | female with `1`; male with `2`, `3`, `6..=10`, or `23` | Attack2 |
| Gunslinger family | `18..=21` | Attack2 |
| Ninja family | `22` | Attack2 |
| Default row | never | Attack2 |

The executable's exact job-ID-to-row table is mirrored and unit-tested in
`native_player_attack_row` in
`korangar/src/world/animation/mod.rs`. `GetRealWeaponId` maps appearances
`31..=97`, `98..=102`, and their subranges back to classic families; that
normalization is mirrored by `native_real_weapon_id` in the same module.
Raw item-ID lookup and Assassin left/right-hand appearance combination occur
before this selector and remain input-normalization work for Korangar.

`0x00991580` does not return a playback-speed multiplier. It returns the
attack-event position stored at actor `+0x24C`:

- Attack2: `5.85` for male job `5`, `5.75` for job `6`, otherwise `6.0`;
- Attack3: `3.0` for Assassin-family jobs with weapon `16` or `25..=30`,
  `5.85` for male Novice-family jobs, otherwise `6.0`.

The dispatcher computes the basic impact offset as:

```text
round(attack_event_position * current_ACT_delay * 24 milliseconds)
```

This value schedules travel/impact presentation. It does not alter the source
ACT clock.

### 7.7 Basic-damage dispatch and delayed impact

The old and current `ZC_NOTIFY_ACT` handlers normalize at `0x0091A4F0`, then
send actor message `0x0B` to the source. `0x008BFB00` owns the main basic
damage path. Its normalized record is:

| Offset | Value |
|---:|---|
| `+0x00/+0x04` | damage fields |
| `+0x08` | exact damage type |
| `+0x0C` | destination actor ID |
| `+0x10` | server tick |
| `+0x14` | division/hit count |
| `+0x20` | packet `sMotion` |
| `+0x24` | packet `dMotion` |
| `+0x28` | special/SP flag |

The damage-type source-action branches recovered so far are exact:

| Type | Native source behavior |
|---:|---|
| `0` | main Damage route |
| `1` | Pickup/state `5` |
| `2` | Sit/state `6` |
| `3` | Stand/state `0` |
| `4` | DamageEndure main route |
| `5` | Splash, no source actor action |
| `6` | Skill, no basic source action |
| `7` | Repeat, no basic source action |
| `8` | MultiHit main route |
| `9` | MultiHitEndure main route |
| `10` | Critical main route, same player selector as type `0` |
| `11` | Lucky, no source actor action |
| `12` | Touch, no source actor action |
| `13` | CriticalMulti main route, same player selector |

The target is not hurt when the packet is received. The source queues actor
message `0x1B` at the calculated impact tick. The actor's timed-message list
at `+0x2A0` is advanced by `0x008AD520`; `0x008B18A7` converts the due event to
message `0x6F`, whose target reaction is dispatched through `0x008B0EDF`.
Damage numbers, target effects, sounds, and hurt must therefore share a
scheduled impact boundary rather than running directly in the packet handler.

The main basic-damage route adds these executable constants after the ACT
event offset:

| Added delay | Actor/job IDs |
|---:|---|
| `192 ms` | `1016`, `1420` |
| `912 ms` | `1285`, `1830` |
| `408 ms` | `1286`, `1287`, `1829` |

These are native actor-family exceptions, not universal ranged-projectile
durations.

### 7.8 Skill-damage source dispatch

`ZC_NOTIFY_SKILL2` handler `0x00946260` normalizes its packet and sends actor
message `0x25`. The base dispatcher routes that message through `0x008B1928`
to `0x008C1390`. The controller:

1. clamps `div` to `1..=36` and derives per-hit damage;
2. resolves and faces the target;
3. calls `0x009B4B30(skill_id, &begin_effect_id, &actor_state)`;
4. requests `actor_state` through the source actor's state virtual;
5. derives the source ACT attack-event offset exactly as in the basic route;
6. queues the target event with `dMotion / 288.0` timing;
7. executes additional type-, skill-, effect-, number-, and hit-specific
   branches.

`0x009B4B30` calls `0x009AF9A0`, whose Lua contract tests
`HaveSkillEffectInfo` and reads `GetBeginEffectID`. That result is the begin
effect ID; it is not the actor state. The actor state is a separate hard-coded
skill-ID lookup whose default is state `7`. Every non-default result for this
build is listed in section 18.

Skill damage therefore does **not** generally reuse the normal job/weapon
attack selector. Examples: skill `59` (Spear Boomerang) requests state `12`,
which the shared resolver maps to Attack1; many weapon skills request state
`2`, which the player resolver sends through the dynamic Attack2/Attack3
selector; most unlisted skills request state `7`, which has a dedicated player
Skill-action route. Effect selection and actor-action selection must remain
separate recipe fields.

The shared state resolver at `0x008A92D0` has the following flat-action
categories. Player states `2`, `7`, `8`, and `9` are intercepted by
`0x009779A0` as described below; other player states above `9` fall through to
the shared resolver.

| Shared action | Actor states |
|---|---|
| Idle `0x00` | `0` |
| Walk `0x08` | `1` |
| Attack2 `0x50` | `2`, `43`, `44` |
| Die `0x40` | `3` |
| Hurt `0x30` | `4` |
| Pickup `0x18` | `5`, `20`, `25` |
| Sit `0x10` | `6`, `7`, `8`, `10`, `14`, `15`, `29`, `31`, `46` |
| Attack3 `0x58` | `9`, `18`, `32`, `49` |
| ReadyFight `0x20` | `11` |
| Attack1 `0x28` | `12`, `19`, `28`, `40`, `42`, `50`, `51` |
| Dynamic attack category | `13` |
| Skill `0x60` | `16`, `17`, `21..=24`, `27`, `30`, `33..=39`, `41`, `45`, `47` |
| Actor-family-specific Attack1/Attack3 | `26` |

For classic SPR/ACT players, state `2` and state `9` both begin with
`0x009A2DB0`; neither means "critical." State 2 then has one CPc-only random
override: raw jobs `4046`, `4047`, `4048`, `4225`, `4226`, `4238`, `4239`,
`4241`, `4243`, and `4244` select Attack3 when `native_random % 10 < 7`, else
Attack2. State 9 keeps the selector result.

State 8 selects Attack1 `0x28` for jobs `4218` and `4220`, otherwise Skill
`0x60`. State 7 selects:

- Attack2 `0x50` for jobs `4218..=4221`;
- ReadyFight `0x20` for jobs `14`, `19`, `20`, `24`, `4037`, `4042`, `4043`,
  `4046`, `4047`, `4048`, `4225`, `4226`, `4228`, `4238`, `4239`, `4241`,
  `4243`, and `4244`;
- Skill `0x60` for every other job.

After that base selection, jobs `15`, `4015`, `4016`, `4020`, `4021`, `4038`,
`4066`, `4070`, `4073`, and `4077` install a cadence-4 Skill-motion-0 hold
with a `400 ms` deadline. Jobs `25` and `4222` install cadence-4 Skill motions
`2,3,4,5,5` with a `1000 ms` deadline. Jobs `24` and `4228` install cadence-4
Skill motion 3 with a `1000 ms` deadline, overriding the displayed
ReadyFight group while the program runs. Every classic-player state-7 route
writes attack-event position `6.0`. A state-7 request is ignored while the
player's current higher state is 6 or actor `+0x114` is nonzero. The owner of
`+0x114` is `SI_SUHIDE` (status 933); section 7.10 gives the full status and
request-precedence contract.

The neighboring function at `0x00977430` belongs to `CGrannyPc`. Its job
lists are not evidence for Korangar's classic SPR/ACT actor path. Earlier
versions of this specification used that table; the runtime and tests now use
the RTTI-confirmed CPc function at `0x009779A0`.

The skill route currently has a proven additional `192 ms` impact offset for
actor/job IDs `1016` and `1420`. It also contains many skill-specific branches;
the appendix is an action-state catalog, not a complete visual recipe table.

Korangar now implements this first runtime boundary with a wrap-safe
`PendingImpactQueue`. Damage dispatch starts source actor and caster/travel
tracks immediately, derives the due tick from the selected body ACT, and
defers the damage number or miss, target hit STR/sound, and Hurt transition as
one target phase. The retained packet tick is presentation/correlation data;
the local due tick is based on dispatch time, matching the native actor-message
queue. Non-death entity removal cancels queued target impacts. Death preserves
them long enough to display the authoritative number/effect at the dead
actor's last position but suppresses Hurt, while map change and disconnect
clear the queue. Events due during a received packet batch are drained after
the whole batch, so same-batch movement, death, and removal are resolved first.

The source-action boundary is implemented. Every skill packet now
runs through the exhaustive section-18 lookup before its impact delay is
derived. Runtime selection preserves native flat ACT groups, applies the
classic-player state `2`/`9` intercept, all CPc state-7/state-8 routes and
programs, state-26 jobs, generic higher programs, the state-49 normalization,
current-dead/sit guards, and the `-1` no-action sentinel. The renderer,
impact scheduler, completion test, and sound occurrence token all consume the
currently selected program action/motion instead of reconstructing a group
from a semantic action name.

This does not make every skill presentation complete. The request guards are
now implemented and every previously supported effect/projectile/audio branch
has a typed recipe, but unmapped skills, cadence variants, persistent units,
and cleanup branches remain open. Missing/asynchronously unavailable animation
data still falls back to zero local impact delay.

### 7.9 Higher actor-state motion programs

The generic state dispatcher at `0x008BAB30` calls the range and sequence
helpers described in section 7.3. Let `G` mean the flat group selected by the
shared state table in section 7.8. Motion numbers below are zero-based. “Hold”
means a non-repeating program retains its last selected step until its
deadline; “cycle” means it wraps while the deadline remains in the future.

| State | Exact selected steps / range | Cadence (actor updates) | Deadline | Repeat |
|---:|---|---:|---:|---|
| `16` | `G:1, G:2, G:3, G:2` | 10 | `99,999,999 ms` | ping-pong cycle |
| `17` | `G:1, G:2, G:3` | 10 | `99,999,999 ms` | cycle |
| `18` | `G:0` | 10 | `3,000 ms` | hold |
| `19` | `G:0, G:0, G:0, G:1, G:1` | 10 | `3,000 ms` | hold |
| `20` | `G:1` four times, then Walk `1:5` | 10 | `3,000 ms` | hold |
| `21` | `G:0` | 10 | `2,000 ms` | hold |
| `22` | `G:2` | 10 | `2,000 ms` | hold |
| `23` | `G:3` | 10 | `2,000 ms` | hold |
| `24` | `G:4` | 10 | `2,000 ms` | hold |
| `25` | `G:1` | 10 | `2,000 ms` | hold |
| `26` | jobs `4049/4227/4240/4242`: `G:4,5,6,7,2`; others: `G:2,3,4,4,4` | 4 | `500 ms` | hold |
| `27` | `G:5` | 10 | `3,000 ms` | hold |
| `28` | `G:0,1,2,3,4` | 10 | `1,000 ms` | hold |
| `30` | `G:1` | 10 | `2,000 ms` | hold |
| `31` | Attack2 `10:3,5,7,8,2` | 5 | `500 ms` | hold |
| `32` | `G:4,5,6,7,2` | 5 | `500 ms` | hold |
| `33` | `G:2` | 10 | `1,000 ms` | hold |
| `34` | `G:3` | 10 | `1,000 ms` | hold |
| `35` | range `G:0..1`, terminal `G:1` | 6 | `500 ms` | hold |
| `36` | range `G:2..5`, terminal `G:5` | 6 | `1,000 ms` | hold |
| `37` | random `G:2` or `G:3` | 10 | `500 ms` | hold |
| `38` | random branch A: range `G:0..3`, terminal `G:3`; branch B: `G:3,2,1,0,0` | 6 | `1,000 ms` | hold |
| `39`, `48` | range `G:4..5`, terminal `G:5` | 6 | `500 ms` | hold |
| `40` | range `G:0..2`, terminal `G:2` | 6 | `500 ms` | hold |
| `41` | random `G:1` or `G:2` | 10 | `500 ms` | hold |
| `42` | `G:0` | 10 | `500 ms` | hold |
| `43` | `G:5` | 10 | `500 ms` | hold |
| `44` | `G:4` | 10 | `500 ms` | hold |
| `46` | `G:0, G:0, G:0, G:1, G:1` | 20 | `3,000 ms` | hold |
| `47` | `G:0,1,2,3` | 6 | `9,999,999 ms` | cycle |
| `49` | range `G:0..7`, terminal `G:7` | 6 | `1,000 ms` | hold |
| `50` | `G:0` | 10 | `9,999,999 ms` | hold |
| `51` | `G:2,3,4,4,4` | 4 | `500 ms` | hold |

State 45 does not use either helper: it directly sets `G` in native playback
mode 3. State 29 has no program. Program states not present in section 18's
current skill catalog are still part of the reusable actor dispatcher and are
implemented because other actor messages/builds can request them.

State 49 is special: the dispatcher installs the state-49 program but stores
logical higher state 7 (`requested 7` and `requested 49` both normalize to
7). Its deadline therefore exits through state 7's completion rule. State 51
clears its initial finish marker and has a classic-player two-stage exit
described below.

Korangar implements this table as `NativeMotionProgram`. ACT action selection,
rendering, impact-event lookup, completion, and sound-event identity all read
the program's current step. The runtime advances a cadence at most once per
`Common::update`, even after a long wall-time gap; it does not synthesize
missed actor-update calls. The wall deadline remains wrap-safe elapsed client
time. Duplicate steps increase a motion-occurrence serial so their sounds are
not collapsed into one token.

### 7.10 Completion transitions and request precedence

The base high-level update at `0x008ACCC0` is a higher-state jump table. Its
post-completion behavior is exact for this client:

| Logical higher state after playback/program completion | Native result |
|---|---|
| `2`, `4`, `5`, `7`, `9`, `12`, `45` | request state `0` |
| classic-player `51` | set logical state `2`; on the next actor update, state `2` requests state `0` |
| `3`, `8`, `10`, `13`, `16..44` except the exit states above, `47..50` | no automatic neutral request; retain the current/held action until another event |
| `0` | idle/loop handling; a one-shot-finish edge can request state `0` again |
| `1` | movement controller owns the transition |
| `6`, `11`, `14`, `15`, `29`, `46` | special/no ordinary core branch; no generic completion exit |

The same ACT group can therefore have different exits. State 12 displays
Attack1 and returns to Idle; state 18 displays Attack3 and holds. Action-name
heuristics such as “every attack-looking group enters ReadyFight” are wrong.

Classic `CPc` state 0 first installs Idle. It then displays ReadyFight while
keeping logical state 0 when the previous higher state was `2`, `4`, or `8`,
or when actor `+0x2C0` is nonzero. `+0x2C0` is the packet field
`isPKModeON`, not a transient target/combat flag: the state-change controller
writes it at `0x008B19A5`, spawn variants write it at `0x008B1BF5`,
`0x008B1D1C`, and `0x008B1F00`, and `CPc::SetState(0)` tests it at
`0x00977BF6`. Korangar preserves it on spawn and applies the complete
`ZC_STATE_CHANGE` record atomically. This gives the following reachable
results:

- classic-player state 2 basic/weapon attack → logical state 0 displaying
  looping ReadyFight;
- classic-player state 4 Hurt → logical state 0 displaying looping ReadyFight;
- state 5 Pickup, state 7 skill, state 9 weapon-route skill, state 12 Attack1
  skill, and state 45 → logical state 0 displaying Idle;
- monsters/NPCs taking any automatic state-0 exit → Idle, because their ACT
  layout has no ReadyFight group;
- state 49 program → normalized state 7 → Idle after its deadline;
- classic-player state 51 → state 2 for one update → state 0/ReadyFight.

Request-time guards are distinct from completion transitions. Their recovered
owners are:

| Actor field / status | Native evidence and behavior |
|---|---|
| `+0x16C`, `SI_TRICKDEAD` (`29`) | The status switch branch at `0x008B3C62` sets the field on gain and directly displays the death action; loss clears it and requests neutral. Generic `SetState` (`0x008BAB59`) and `CPc::SetState` (`0x009779D4`) reject every ordinary request while it is set, just as they do for actual logical death state `3`. It is a pose lock, not real death. |
| `+0x114`, `SI_SUHIDE` (`933`) | The status branch at `0x008B6855` requests native state `48` and sets the field; loss clears it. `CPc::SetState(7)` tests it at `0x00977C2B`, so it blocks only the state-7 skill request, not all actor actions. |
| `SI_SU_STOOP` (`893`) | The adjacent branch at `0x008B68B0` requests state `47`. On loss it returns neutral only when `+0x114` is clear, preserving SU Hide's higher-priority state-48 pose. |
| `bodyState` / opt1 at `+0x2B4` | `0x008B8B57` preserves a playback hold for stone (`1`) and freeze (`2`). Requests and movement ownership remain live; the ACT clock is held rather than replacing all inputs with a broad status lock. |

Korangar mirrors those fields for every visible actor and uses this input
precedence:

1. Authoritative status, state-change, death, and removal packets update their
   owners first. Trick Dead gain stops movement and cast, installs its held
   death-looking pose, and rejects later movement/action requests. Actual death
   has the same request guard but remains semantically dead.
2. A successful damage, no-damage, or ground skill result terminates the
   source cast before requesting the source pose. A rejected pose can therefore
   never leave a stale cast. `ZC_DISPEL` (`0x01B9`) and local skill-request
   failure also terminate the cast; status cancellation is never inferred from
   local animation completion.
3. Movement does not cancel casting: the trajectory and cast overlay coexist,
   which is required for Free Cast. Hurt also no longer rejects a moving actor
   or destroys its trajectory. A server cast-cancel packet remains authoritative
   when damage or another rule interrupts the cast.
4. Stone/freeze opt1 pauses and later resumes the ACT clock with its elapsed
   phase preserved. Movement trajectory, cast lifetime, and incoming action
   selection remain independent of that visual hold. Other body/health/option
   values remain stored for their future visual/status recipes.
5. `CPc` state 7 alone is rejected while sitting in native state 6 or while
   `SI_SUHIDE` owns `+0x114`; other skill states follow the generic death,
   Trick Dead, and `-1` guards. SU Stoop and SU Hide loss restore the surviving
   higher-priority pose or the PK-aware neutral action.

Movement completion explicitly requests state 0; sit/stand and death are
packet/controller requests rather than automatic ACT exits. This completes
the recovered request-time boundary, while damage-type-specific reaction
guards and unmodeled status visuals remain separate work.

## 8. Skill presentation recipe schema

Every skill, monster skill, and special basic attack is represented by one or
more versioned recipes. The implementation may use Rust or a validated data
file, but it must expose the following semantics.

```text
SkillPresentationRecipe
  identity
    skill_id / skill family / actor-family override
    client-build range
    variant predicate (level, job, sex, weapon, status, result, map)
    evidence and source references

  occurrence
    correlation strategy
    completion condition

  actor_action
    trigger event
    source actor selector
    native state/action request or explicit override
    playback mode and timing source
    direction/target rule
    layer visibility overrides
    completion transition

  tracks[]
    trigger event and condition
    dedupe scope
    start offset and authoritative clock
    anchor and follow policy
    renderer/backend and asset/procedure
    transform, orientation, scale, color, blend, height
    repetition/randomization/hit scheduling
    sound/light/camera companions
    lifetime and teardown

  target_reaction
    hit/miss/immune/endure/dead/moving conditions
    hurt/state action and timing source
    number type and scheduling

  tests
    packet fixture
    asset fixture
    timeline assertions
    reference capture
```

No recipe may hide an unexplained timing constant. Every duration or offset is
one of: packet-derived, asset-derived, native constant with address, observed
with capture, or explicitly provisional.

### 8.1 Implemented typed recipe boundary

`korangar/src/world/skill_recipe.rs` is now the single skill-ID presentation
registry. `SkillPresentationRecipe` separates these independently executable
phases:

| Recipe field | Owning normalized event / time boundary |
|---|---|
| `successful_caster_effect` / `successful_caster_sounds` | successful `SkillEffectNoDamage`, immediately at the source; still occurs for an empty-area success |
| `damage_caster_effect` / `damage_caster_sounds` | `DamageEffect` source dispatch, alongside the source actor action |
| `projectile` | damage source/launch phase; the projectile implementation owns travel policy |
| `damage_target_effect` / `damage_target_sounds` | scheduled native impact boundary at the destination |
| `hit_effects` / `hit_sounds` | scheduled native impact boundary, independently repeatable by future per-hit cadence |
| `ground_effect` / `ground_sounds` | `GroundSkillEffect` target tile; the server start tick is retained for the STR-clock follow-up |

An absent effect never suppresses audio or a projectile in the same phase.
Random assets expose both `resolve()` for runtime selection and `variants()`
for deterministic asset audits. Unknown skill IDs return a complete empty
recipe rather than falling through unrelated switches. The exhaustive native
skill-ID → actor-state resolver remains separate: an empty presentation recipe
still gets the correct source actor action.

The registry currently contains every presentation track that existed before
this boundary, grouped here so coverage is reviewable:

| Family | Mapped skill IDs |
|---|---|
| Swordsman/Mage first-job and elemental | `5`, `7`, `11`, `13`–`15`, `17`–`21` |
| Knight | `56`–`59`, `62` |
| Acolyte/Priest | `70`, `77`, `79`, `156` |
| Wizard | `80`, `81`, `83`, `85`, `88`–`92` |
| Hunter | `110`, `115`, `118`, `119`, `121`–`123` |
| Assassin/Rogue | `136`, `139`, `140`, `214`, `406` |
| Rune Knight | `2006` |

This is an implemented registry boundary, not full RO skill coverage. Each
remaining player, monster, homunculus, mercenary, and NPC skill must be added
from native/capture evidence using section 15. `MAPPED_SKILL_IDS` is the
machine-readable current coverage set; tests require each listed recipe to be
non-empty, require every data asset to expose an audit variant set, and the
ignored GRF audit checks every declared STR/texture/wav path.

### 8.2 Track backends

The pipeline must support these independent backends:

| Backend | Use |
|---|---|
| `ActorAction` | Body/layer ACT state transition. |
| `StrEffect` | STR keyframe effect with exact FPS, key lifetime, texture animation, blend, and interpolation. |
| `SprActEffect` | Standalone ACT/SPR effect actor, including animated projectiles and weapon trails. |
| `BillboardSequence` | Client-code-drawn textured quad sequences such as bolts. |
| `Projectile` | Source-to-target, ballistic, homing, beam, boomerang, chain, or orbit motion. |
| `Geometry` | Client-code-drawn rings, cylinders, cones, lines, meshes, and screen-space shapes. |
| `PersistentUnit` | Server-created ground cell, trap, aura, wall, field, or summon keyed by unit entity. |
| `AttachedStatus` | Effect following an entity for a status lifetime. |
| `ParticleNumber` | Damage, critical, miss, heal, resource, and combo numbers. |
| `Audio` | Positional or non-positional one-shot/loop with correct owner and range. |
| `Light` | Point/directional light track; enhancement must be separately configurable when not native. |
| `Camera` | Shake, quake, zoom, flash, or post-process. |

Korangar-specific lights added for atmosphere are not automatically classic
fidelity. Recipes must label native presentation separately from optional
enhancement tracks.

### 8.3 Anchors and follow policy

Supported anchors must include:

- `SourceOrigin`, `SourceBody`, `SourceHead`, and an authored source attach
  point;
- `TargetOrigin`, `TargetBody`, `TargetHead`, and an authored target attach
  point;
- `GroundTile`, packet world point, or an offset from either;
- a path from source snapshot to target snapshot;
- a path that follows a moving target;
- a persistent unit entity;
- screen/camera space.

Each track states whether it snapshots its anchor at trigger time, follows it
for its entire life, follows until launch and then detaches, or retargets.
Source/target disappearance behavior must be explicit.

### 8.4 Timing and scheduling

Tracks use one of these clocks:

- actor ACT clock (`delay × 24 ms`, modified by native actor factors);
- packet/client tick;
- STR key clock (`key / fps`);
- standalone ACT/SPR clock;
- persistent unit lifetime/removal event;
- status duration/removal event;
- observed provisional seconds, pending native recovery.

Multi-hit scheduling must distinguish:

- one packet with `div > 1` and visually staggered hits;
- several damage packets for one occurrence;
- simultaneous area targets;
- damage-over-time pulses;
- chained or bouncing targets.

Damage numbers, target reactions, impact effects, and sounds each declare
whether they occur once, per target, per packet, or per hit. They must not all
inherit one blanket `hit_count` loop.

### 8.5 Random variants

When the native client selects among effect or sound variants, record:

- variant set and exact assets;
- distribution or selector rule;
- whether selection is per occurrence, target, hit, or loop;
- deterministic seed policy for tests and replays.

Randomness must not change gameplay and must be injectable in tests.

## 9. Skill-family requirements

Every recipe is assigned to a family so reviewers know which tracks and edge
cases must be considered.

| Family | Required questions |
|---|---|
| Basic melee/critical | Which native actor state and job/weapon action? Which ACT event owns swing audio? When does target hurt begin? Critical number/effect/sound? |
| Bow/gun/thrown basic | Launch action and frame, ammo/weapon layer, projectile path, hit/miss endpoint, ranged sound, target movement behavior. |
| Instant weapon skill | Does it reuse attack selection, Skill action, or a skill-specific override? Caster versus target effects; area dedupe. |
| Casted direct spell | Cast start/cancel, cast aura, completion action, projectile/beam, impact and number schedule, elemental variants. |
| Bolt/multi-hit spell | Count source, stagger, individual impact timing, one versus many hurt reactions and numbers. |
| Ground burst | Successful use versus ground-placement trigger, tile anchor, empty cast behavior, area targets, camera/audio ownership. |
| Persistent field/wall | Initial cast, one instance per server unit/cell, loop phase, pulse effects, removal/fade, map-change cleanup. |
| Trap | Placement actor action, hidden/visible states, unit sprite, trigger, detonation, owner, removal race. |
| Buff/debuff/status | Cast effect, target attach, status gain/loss authority, duration refresh, stack/variant values, dispel and death cleanup. |
| Heal/resurrection | No-damage event versus heal-number packet, source/target effects, amount display, revive actor transition. |
| Channel/toggle | Start, maintained loop, periodic costs/pulses, explicit stop, interruption, disconnect/map cleanup. |
| Movement/teleport | Pre-action, origin trail, authoritative slide/warp, destination effect, facing, invulnerability/status visuals. |
| Summon/companion | Cast, spawned actor family, ownership, spawn/death effect, its own attacks and skills. |
| Monster/NPC skill | Actor resource action availability, monster skill packet route, effect recipe, size/height adjustment, target reactions. Do not force player action groups onto mobs. |
| Code-drawn legacy effect | Recover native geometry, texture sequence, blend, and constants; an unrelated STR is not a fidelity substitute. |

Monster basic attacks and skills use the same recipe machinery with an actor-
family-specific action resolver. Mob size, sprite scale, flying height,
invisibility, and boss effect variants are recipe inputs rather than special
cases in packet handlers.

## 10. Asset and renderer contracts

### 10.1 ACT/SPR actors and effects

- Preserve every action, motion, clip, attach point, event, and delay.
- Preserve empty motions.
- Preserve separate layer resources when runtime composition is required.
- Apply palette, mirror, rotation, scale, color, and clip ordering exactly.
- Resolve path case and Korean filenames through the archive loader, not host
  filesystem assumptions.

### 10.2 STR effects

The loader and renderer must preserve:

- declared FPS and maximum key;
- basic versus morphing frame semantics;
- texture animation types and delays;
- UV/XY interpolation, offset, angle, color, alpha, and blend factors;
- per-layer start/end visibility and ordering;
- effect origin and world/screen-space transform.

The current Korangar timer caps a long application step to 1/15 s to avoid
first-load skipping. This is a usability workaround, not a recovered native
clock rule. Asset loading should ultimately be asynchronous/prewarmed so the
timeline can seek to authoritative time without hiding the whole effect.

### 10.3 Audio

Audio tracks specify asset, owner/anchor, spatial range, start motion/key,
looping, stop/fade condition, and variant selection. ACT `.wav` events,
`"atk"` weapon sounds, skill table sounds, impact sounds, and ambient
persistent-unit loops are separate sources and can coexist.

### 10.4 Lifetime and cleanup

Every spawned presentation object has an owner and cleanup rule. Mandatory
cleanup boundaries include occurrence completion, unit removal, status loss,
entity removal, death where applicable, cast cancel, map change, disconnect,
and replay seek. Persistent effects should fade only when the reference does;
otherwise remove immediately.

## 11. Target reactions and combat numbers

Reaction selection must consume the full damage result, not only
`damage > 0`:

- hit, miss, critical, blocked, absorbed, immune, endure/no-flinch;
- multi-hit and repeated-hit family;
- target dead/dying, sitting, moving, casting, frozen, or already reacting;
- player versus monster/NPC actor action layout;
- packet target-motion field and native timing factors.

The pipeline must define whether a skill shows one aggregate number or one
number per hit, whether numbers are simultaneous or staggered, and whether a
miss still launches/travels. Hurt animation cadence is not automatically the
same as number or impact cadence. For native basic and skill damage, the
minimum scheduling unit is a pending impact containing the due client tick,
target, result/type, `div`, number/effect recipe data, and the normalized
`dMotion / 288.0` reaction value. Applying hurt at packet receipt and delaying
only the projectile is invalid.

## 12. Required catalogs

Coverage for “all skills and combat” is tracked in generated catalogs, not by
memory. The implementation must produce:

1. **Actor action catalog**: every actor resource, action count, per-direction
   motion count, delay, event placement, empty motions, attach points, and
   layer mismatch.
2. **Player attack matrix**: every job/sex/weapon/offhand/option combination,
   native selected base action, timing factor, ACT assets, and event frames.
3. **Skill recipe catalog**: every server skill ID and name, owning jobs/mobs,
   family, packet triggers, recipe status, evidence grade, asset dependencies,
   and acceptance capture.
4. **Monster presentation catalog**: mob/job ID, ACT layout, basic attack/hurt/
   death mapping, used skills, size/height modifiers, and recipe coverage.
5. **Effect asset catalog**: all STR, ACT/SPR effect pairs, textures, sounds,
   internal timing, and every recipe consumer.
6. **Unmapped packet/effect catalog**: server presentation packets and native
   effect IDs that are currently ignored.

Each catalog reports `proven`, `implemented`, `tested`, and `visually
accepted` separately.

## 13. Implementation architecture

The target architecture has six boundaries:

1. `korangar-networking` decodes packets without losing presentation fields.
2. A combat normalizer publishes typed events and correlates occurrences.
3. An impact scheduler owns native due ticks and emits launch/impact phases.
4. An actor state machine resolves actions and native timing by actor family.
5. A recipe registry expands events into independent presentation tracks.
6. Track runtimes own rendering, event crossing, attachment, and cleanup.

Suggested core types:

```text
CombatPresentationEvent
OccurrenceId / OccurrenceContext
PendingImpact / ImpactSchedule
ActorActionRequest / ActorPlaybackState
ActorLayerSet / ActorEventCursor
SkillPresentationRecipe / RecipeVariant
PresentationTrack / TrackInstance
Anchor / FollowPolicy / TimeSource / DedupeScope
PresentationRegistry / PresentationWorld
```

The main `Client` event loop should eventually contain only event publication
and resource/world services. Skill-ID selection for `skill_hit_effects`,
`ground_skill_effect`, bolt/spear projectiles, audio, and procedural
`spawn_*_skill_effect` tracks now lives in the registry; the remaining helper
functions are backend adapters selected by typed recipe values. Occurrence
correlation and generalized track instances are still required to remove
those adapters from the event loop completely.

## 14. Current Korangar gap matrix

Snapshot as of 2026-07-17:

| Area | Current state | Required work |
|---|---|---|
| ACT delay | Changed to native `delay × 24 ms`; missing delay defaults to 4.0. | Trace all actor timing factors and special modes. |
| Empty ACT motions | Preserved by the loader. | Add real-GRF fixtures across player and mob resources. |
| Layer mismatch | Pre-merge now follows body count and motion-0 fallback. | Preserve layers at runtime; implement full attach, shield/cosmetic, per-item weapon, and `_검광` paths. |
| ACT events | Current frame token prevents repeated held-frame audio. | Replace with body crossed-motion cursor so skipped frames still fire. |
| Player attack action | Native `0x009A2DB0` job/sex/weapon predicates and `GetRealWeaponId` normalization are implemented and unit-tested. | Recover raw item-to-view and Assassin left/right-hand combination inputs; generate the exhaustive matrix from real assets. |
| Source attack timing | Source attacks run on natural `ACT delay × 24 ms`; packet `sMotion` no longer stretches the action. Every skill resolves its native actor state, classic-player/shared flat group, and higher motion program; impact offsets use the program's current action plus player `0x00991580`/state-7 marker or the shared body's authored `"atk"` motion / `motion_count - 2` fallback. | Recover remaining actor timing modifiers and skill-specific recipe branches. |
| Target impact/hurt timing | `PendingImpactQueue` now begins Hurt, numbers, hit effects, and hit sounds together at the ACT-derived due tick. Hurt then uses the native `dMotion / 288.0` player-job/non-player accelerate-versus-hold clock. Queue cancellation covers removal, map change, and disconnect; death keeps number/effect delivery but suppresses Hurt. | Add packet fixtures/golden timelines, exact damage-type reaction guards, per-hit cadence, and remaining skill-specific timing branches. |
| Post-action state | Completion is keyed by logical native state: `2/4/5/7/9/12/45` request state 0, CPc state 51 takes the two-stage state-2 exit, other states hold, and CPc neutral displays ReadyFight for previous `2/4/8` or packet `isPKModeON` (`+0x2C0`). | Add packet fixtures and golden state timelines. |
| Skill actor action | The exhaustive `u16` catalog drives runtime selection. Shared flat groups, classic-player states `2/7/8/9`, higher programs, state-26 jobs, state-49 normalization, completion exits, real-death/Trick Dead (`+0x16C`), SU Hide (`+0x114`), sit, and `-1` request guards are implemented and tested. Damage/no-damage/ground execution clears casts before requesting the guarded actor pose. | Recover remaining packet/type-specific action branches and add live capture acceptance across actor families. |
| Skill effects | A typed per-skill registry owns successful-caster, damage-caster, projectile, damage-target, hit, and ground effect/audio phases for every previously supported mapping. Unknown skills have an explicit empty contract and mapped variants are auditable. | Audit and implement every remaining server skill ID, native effect path, cadence, variant predicate, and cleanup rule. |
| Persistent units | Firewall and Pneuma only; some packet fields discarded. | Preserve creator/level/range/visibility; implement all UnitIds and exact teardown. |
| Status presentation | Status guards apply to all visible actors; Trick Dead, SU Hide, SU Stoop, and stone/freeze opt1 playback hold are modeled. The atomic option/body/health/PK state is retained; local status icons and conceal alpha remain. | Add status-specific attached effects, tint/material behavior, stun/sleep/etc. presentation, refresh variants, and visual acceptance. |
| Projectiles | Spear and bolt ownership is declared per skill in the typed registry; bespoke render backends remain. | Generalize projectile tracks with native launch/impact timing, orientation, interpolation, and target-loss policy. |
| STR timeline | Core interpolation implemented; long-frame cap is a workaround. | Prewarm/async loading, reference renders, blend/order verification. |
| Monsters/NPCs | Basic compact action mapping and generic skill routing. | Audit each actor family; mob skill catalog and size/height/action variants. |
| Dedupe | Fixed short `(source, skill)` gate. | Occurrence correlation and per-track dedupe scope. |
| Verification | Asset probes and unit tests; no complete visual corpus. | Packet fixtures, golden timelines, rendered-frame captures, and side-by-side reference acceptance. |

## 15. Per-skill implementation workflow

No skill is accepted by copying the closest existing recipe. Use this order:

1. Identify every server packet emitted for success, failure, empty cast,
   multi-target, multi-hit, unit creation/removal, status gain/loss, and death.
2. Capture the reference client with controlled source/target positions, job,
   sex, weapon, level, ASPD, movement, and status state.
3. Inventory candidate assets by exact path and inspect internal timing. Record
   when the native effect is code-drawn rather than asset-driven.
4. Trace the relevant native packet handler/state/effect dispatch or label the
   sequence `OBSERVED`/`INFERRED` honestly.
5. Fill one recipe sheet with every track, anchor, trigger, clock, dedupe scope,
   variant, and cleanup rule.
6. Add packet fixtures and a deterministic timeline test before renderer work.
7. Add asset-existence and parser tests.
8. Add render tests at meaningful timestamps, including direction and camera
   rotation where relevant.
9. Test edge cases: miss, zero damage, Endure/no-flinch, empty area, moving or
   disappearing target, repeated cast, multiple casters, lagged frame, map
   change, and cancellation.
10. Record a side-by-side acceptance capture and update the generated catalog.

## 16. Acceptance requirements

A recipe can be marked complete only when:

- all packet fields used by the native presentation are retained;
- actor action, direction, playback mode, timing, and completion transition
  are supported by evidence;
- every caster/travel/impact/unit/status/audio track has the right trigger,
  anchor, clock, repetition, and teardown;
- multi-hit, multi-target, empty-cast, miss, and interruption behavior is
  tested as applicable;
- all assets resolve from the configured archives and parse without fallback
  substitution;
- a slow application frame does not lose ACT events or duplicate occurrence
  tracks;
- player, remote player, monster/NPC, and local-player storage paths behave
  consistently;
- deterministic timeline tests pass;
- visual acceptance against the named reference build is recorded.

“Looks plausible,” “uses an official asset,” and “works for Knight” are not
completion criteria for a system intended to cover every class and mob.

## 17. Native research ledger

The following addresses currently anchor the recovered actor contract:

| Address | Recovered role |
|---|---|
| `0x004F1C20` | ACT parser |
| `0x004F5140`–`0x004F5670` | ACT action, delay, event, motion, count, and validity accessors |
| `0x008A92D0` | Actor state/event to base action selection |
| `0x008A9500` | Search body action for `"atk"` |
| `0x008AC2E0` | Core actor action/time/motion update |
| `0x008AC860` | Crossed body-motion `.wav` event scanner |
| `0x008AD520` | Advance and dispatch the actor timed-message list at `+0x2A0` |
| `0x008ADAB0` and related render paths | Body/layer motion retrieval and composition |
| `0x008B0EDF` | Due target damage/reaction message (`0x6F`) branch |
| `0x008B18A7` | Convert queued impact message `0x1B` to due message `0x6F` |
| `0x008B19A5` | Store `ZC_STATE_CHANGE.isPKModeON` in actor `+0x2C0` |
| `0x008B1BF5`, `0x008B1D1C`, `0x008B1F00` | Store spawn-record `isPKModeON` in actor `+0x2C0` for packet variants |
| `0x008B3C62` | `SI_TRICKDEAD` (`29`) gain/loss owner of actor `+0x16C` |
| `0x008B6855` | `SI_SUHIDE` (`933`) gain/loss owner of actor `+0x114` and state 48 |
| `0x008B68B0` | `SI_SU_STOOP` (`893`) state-47 branch with SU Hide precedence |
| `0x008B7DC0` | SetAction-like state reset and delay setup |
| `0x008B8B57` | Body-state/opt1 transition; stone/freeze playback hold behavior |
| `0x008BAB30` | Generic actor state/action dispatcher |
| `0x008BFB00` | Normalized basic-damage source controller and target-event scheduling |
| `0x008C1390` | Skill-damage source controller, actor state, effect, hit, and target scheduling |
| `0x0091A4F0` | Normalize `ZC_NOTIFY_ACT` variants into the actor damage record |
| `0x00946260` | `ZC_NOTIFY_SKILL2` (`0x01DE`) packet handler and normalizer |
| `0x009779A0` | Classic SPR/ACT player (`CPc`) state transition and attack/skill routes; includes `+0x16C`, `+0x114`, and `+0x2C0` request/neutral tests |
| `0x00977430` | Granny-model player (`CGrannyPc`) state function; not Korangar's actor path |
| `0x009A2DB0` | Player job/equipment attack-action selector |
| `0x009AF9A0` | Lua `HaveSkillEffectInfo`/`GetBeginEffectID` bridge |
| `0x009B4B30` | Skill ID to begin-effect ID and actor-state lookup |
| `0x00991580` | Player attack-event-position selector |

For reproducibility, future findings must record executable hash, address,
calling context, input fields, output, and how the conclusion was tested. The
ledger grows with packet handlers and skill/effect dispatch functions as they
are recovered.

## 18. Native skill-ID to actor-state catalog

This catalog is `NATIVE` for the executable hash at the top of this document.
It is the exhaustive set of non-default results of `0x009B4B30` for the full
unsigned 16-bit skill-ID domain. Ranges are inclusive. Every skill ID not
listed returns actor state `7`.

The table was extracted by `korangar/tools/native_skill_state_audit.py`, which
emulates the function's comparisons and native jump tables while forcing the
independent Lua begin-effect result to `-1`. Representative IDs and every
jump-table boundary were checked against the disassembly. This table selects
the source actor state only; it does not define the complete effect, hit
cadence, number, audio, projectile, or cleanup recipe.

| Actor state | Skill IDs |
|---:|---|
| `0` | `29`, `151`, `196`, `209`, `249`, `252`, `256-257`, `261`, `268-269`, `304`, `349-350`, `356`, `369`, `380`, `384`, `387`, `401`, `411`, `444`, `690`, `730`, `1015`, `2334`, `8204`, `8219-8220` |
| `1` | `364` |
| `2` | `5`, `7`, `42`, `46-47`, `56-58`, `61-62`, `109`, `212`, `214`, `219`, `253`, `263`, `266-267`, `338`, `366-368`, `370`, `372`, `379`, `382`, `397-399`, `406`, `484`, `499`, `502-503`, `512-515`, `518-520`, `661`, `674`, `712`, `728`, `732`, `740`, `1001`, `1005`, `1009`, `1016`, `2002`, `2004-2006`, `2017`, `2022-2023`, `2029-2031`, `2036-2037`, `2233`, `2236`, `2259`, `2261`, `2279-2280`, `2284`, `2288`, `2298`, `2314`, `2317`, `2320`, `2324`, `2342`, `2476` |
| `5` | `115-125`, `688`, `1013`, `2238-2239`, `2249-2254`, `2267` |
| `9` | `316`, `324`, `394`, `530`, `2258`, `2307-2308`, `2312`, `3004` |
| `11` | `707`, `2003`, `2011` |
| `12` | `59`, `149`, `152`, `229-232`, `250-251`, `480`, `1004`, `2278`, `2480`, `2493` |
| `16` | `306-313`, `325`, `327-330`, `395-396`, `488`, `1011` |
| `17` | `317`, `319-322` |
| `18` | `2327`, `2330` |
| `20` | `426`, `2581`, `5023` |
| `21` | `412`, `2015`, `2285` |
| `22` | `414`, `2478` |
| `23` | `416`, `445`, `447-458`, `460-461`, `494`, `572-575`, `2333`, `2340`, `2596-2599` |
| `24` | `418`, `2018`, `2343` |
| `25` | `420`, `2269`, `2323` |
| `26` | `419` |
| `27` | `421`, `2336` |
| `30` | `413`, `2337`, `2576` |
| `31` | `415`, `2313` |
| `32` | `417`, `2311`, `2344-2348`, `2593` |
| `33` | `431-433` |
| `34` | `428-430` |
| `35` | `527-529`, `532`, `535-538` |
| `36` | `523-526`, `531`, `534`, `539-543`, `3005-3009`, `3020`, `3022-3023` |
| `37` | `3011`, `3013-3018`, `3021`, `3026-3027`, `3029` |
| `38` | `501`, `504-506`, `516-517` |
| `39` | `500`, `2256-2257`, `2260`, `2271-2272`, `2445` |
| `40` | `508`, `521` |
| `43` | `2268`, `2273-2275` |
| `44` | `2270` |
| `49` | `5021` |
| `51` | `2580` |
| `-1` (`0xFFFFFFFF`) | `446`, `478`, `490`, `2028`, `2215`, `2218-2221`, `2240-2242`, `2263-2265`, `2415-2416`, `2481-2482`, `2485-2486`, `2490`, `2516`, `2520` |
| default `7` | every other `u16` skill ID |

State `-1` is deliberately preserved as a native no-action sentinel:
`0x008BAB30` compares the requested state to `-1` and returns without changing
the actor. It must not be silently coerced to state `7`.

Korangar's runtime lookup mirrors this complete table. Tests cover a
representative of every row, the exact full-domain output distribution, range
boundaries/defaults, player state-7 job routes and guards, the no-action
sentinel, state-26 job behavior, generic/CPc higher motion programs,
post-completion transitions, and player versus monster resolution. This
catalog plus section 7.9 resolves source actor state/action/motion; it does not
close the per-skill effect, projectile, cadence, audio, status, or cleanup
recipe gaps described above.
