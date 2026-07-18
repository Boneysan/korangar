# Implementation Plan — Animation Fidelity: Visible Gaps & Architectural Debt

| | |
|---|---|
| **Status** | Draft (2026-07-17) — follows the native combat animation runtime (`cbc03de5`) |
| **Milestone** | Post-M1 presentation fidelity |
| **Parent** | [ANIMATION_SYSTEM.md](../ANIMATION_SYSTEM.md) §7 Known gaps, [combat-animation-pipeline.md](../specs/combat-animation-pipeline.md) §14 gap matrix, FEATURE_ROADMAP.md "Classic skill-effect coverage pass" |
| **Depends on** | Native runtime boundaries (done); `provision-effect-roster` GUI roster (done) |

## 1. Scope and shape

Two independent work tracks that do not touch the same files and can run in
parallel (e.g. split across agents):

- **Engine track** (phases B–D): runtime layer composition and the frame-event
  cursor — the architectural debt that blocks sword trails, per-item weapons,
  shields, and reliable frame sounds.
- **Data track** (phase E): the remaining classic skill/status presentation
  recipes on top of the already-typed `world/skill_recipe.rs` registry.

Phase A gates both; phase F is cross-track polish. Per-skill work follows the
spec's §15 workflow and §16 acceptance rules — "works for Knight" is not done.

Out of scope here: monster/NPC per-family ACT audits, mercenary/homunculus
catalogs, and the golden rendered-frame corpus (spec §14 "Verification") beyond
the fixtures each phase adds.

## 2. Phase A — Close the verification debt first (small)

Everything below builds on code that has not all been watched live. Before
adding more:

1. **Live GUI pass of batch 2** with the provisioned roster on `prt_fild07`:
   EffectKnight (already green), EffectSinX (katar Attack3 + Sonic Blow ring +
   `sonicblow.str`), EffectStalker, EffectRune (folder-alias weapon layers),
   plus Magnum Break / Raid / Meteor Assault / Ignition Break caster bursts.
   **EffectSinX verified 2026-07-17** (katar layer + thrust action + Sonic
   Blow 滅 glyph + EDP status), after finding and fixing a real bug: async
   `AnimationData` completions were never delivered to `this_entity`, so the
   local player's weapon layer was loaded and discarded (see session notes).
   Still open: Meteor Assault streak visibility, EffectStalker, EffectRune,
   Knight regression glance.
2. **One mismatched-layer combo live** (female Novice melee or any first-class
   crit) to confirm the body-owned motion-index fallback visually.
3. **Packet fixtures + golden timeline tests** for the impact scheduler: a
   canned `ZC_NOTIFY_ACT`/`ZC_NOTIFY_SKILL2` byte sequence in, asserted
   (source action, due tick, hurt clock) timeline out. These become the
   regression net for every later phase.
   **Done 2026-07-17** (`golden_timeline_tests` in `world/animation/mod.rs`,
   damage fixtures in `korangar-networking` `packet_handlers`): sMotion/dMotion
   separation, miss/critical variants, div→hit_count, the 576 ms Knight spear
   due boundary, the monster `"atk"`/`motion_count - 2` boundary, and the
   Spear Boomerang state-12 resolution are pinned.

Exit: all batch-2 rows in the roadmap flip to live-verified, or bugs get filed
and fixed first.

## 3. Phase B — Crossed-motion event cursor (small, engine)

Replace the interim `SoundToken` with the native scanner semantics
(`0x008AC860`): on each actor update, walk every **body** motion crossed since
the previous update — including loop wrap — and fire each authored event
exactly once per crossing.

- Cursor state lives in `AnimationState` (last observed body motion + cycle
  count); `SoundState` consumes event occurrences instead of displayed frames.
- Held one-shot final frames produce no new crossings (current behavior kept);
  a slow application frame that jumps 3 motions fires all 3 events.
- Unit tests: multi-motion jump, loop wrap, held frame, motion-program
  duplicate steps (keep the `step_serial` behavior).

Exit: `SoundToken` deleted; ANIMATION_SYSTEM.md §5.3 caveat removed.

## 4. Phase C — Runtime layer composition (large, engine)

Stop permanently flattening body/head/weapon into one merged `AnimationData`.
Retain per-layer ACT/SPR resources and compose at render time, matching native
`CActRes` behavior the pre-merge already emulates in data form.

Steps, each landable separately behind the existing `get_frame` API:

1. **Multi-layer `AnimationData`**: keep the per-pair `Actions` handles;
   body owns clock + motion index; secondary layers resolve via the
   `get_motion` motion-0 fallback (`0x004F5360`) at render time instead of
   merge time. The existing merge tests become composition tests.
2. **Attach points**: apply ACT attach data (head↔body alignment, the special
   motion-0 attachment cases) instead of baked offsets.
3. **Layer ordering + per-direction draw order** as authored.
4. **Dynamic layer swaps**: weapon/head/appearance changes swap one layer
   without re-requesting the whole actor (today an appearance change reloads
   everything through `AsyncLoader`).
5. **Shield layer** (`방패` paths) — first new layer type the architecture
   must absorb cleanly.

Risks: draw-call count and the WSL GL fallback (no bindless arrays) — profile
on both backends; keep the flattened path compilable until step 3 lands.

Exit: pre-merge deleted; Knight/roster render identical before/after
(screenshot diff); shield renders; appearance swap causes no full reload.

## 5. Phase D — Weapon visual completeness (medium, engine, needs C)

1. **Item→view normalization**: model the raw item-ID→view lookup and the
   Assassin left/right-hand combination that precede the recovered
   `0x009A2DB0` selector (spec §7.6 "selector inputs").
2. **Per-item weapon sprites** (`기사_남_1530.spr` …): probe exact per-item
   paths first (extend `weapon-sprite-audit`), fall back to the class sprite,
   then to none — never the placeholder.
3. **`_검광` sword-trail layers**: additive layer driven by the same body
   motion index; audit which weapon classes ship trails and when native draws
   them.

Exit: audit tool enumerates per-item/trail coverage; roster + a per-item
weapon (e.g. a specific named spear) verified live.

## 6. Phase E — Skill/status presentation batches (data track, parallel)

Continue the proven per-batch method (wire probe → asset dump → roBrowserLegacy
semantic cross-check → typed recipe → GRF audit test → live check). Suggested
batch order by campaign visibility:

- **E1 — Code-drawn classic effects** (procedural `EffectBase` recipes like
  `FallingBolts`/`SkillBurst`): Napalm Beat, Soul Strike orbs, Frost Diver
  travel + freeze, Jupitel Thunder ball, Earth Spike / Heaven's Drive ground
  geometry, Fire Ball. These are common low-level mob/party spells — highest
  visible payoff.
- **E2 — Persistent skill units** beyond Firewall/Pneuma: Safety Wall,
  Sanctuary, Magnus, Ice Wall, Sage ground fields, Hunter traps (armed
  visual), song/dance areas. Requires keeping creator/level/range/visibility
  from `NotifySkillUnitPacket` and exact teardown on `RemoveSkillUnit`
  (spec §14 "Persistent units").
- **E3 — `DisplaySpecialEffectPacket` (0x01F3) + cast circles + sound
  retry**: register the packet → `effect id → recipe` table; classic
  cast-target circles; re-search the missing sounds via roBrowserLegacy's
  effect-table wav references instead of name guessing.
- **E4 — Status presentation depth**: opt2/option tints (poison/curse),
  stun/sleep/frozen poses and attached mini-effects, Freeze1/Freeze2 selection
  where native uses them, refresh variants.

Every batch: extend `all_mapped_skill_effect_assets_exist`, add its skills to
the roster provisioning if needed, and record the live check in the roadmap
row. Unknown IDs keep the explicit empty contract — no closest-recipe copying.

## 7. Phase F — Timing polish (after A fixtures exist)

- **Per-hit cadence**: spread a `div > 1` packet's numbers/hit effects at the
  native per-hit interval instead of one lump at the due tick.
- **Damage-type reaction guards**: the exact per-type flinch rules the trace
  left open (spec §7.7).
- **STR prewarm/async load** to retire the 1/15 s advancement cap: request STR
  + textures through `AsyncLoader` on recipe resolution (cast/begin phase),
  so first use never stalls the frame.

## 8. Sequencing summary

```
A (verify + fixtures)
├── engine: B (event cursor) → C (composition, 5 steps) → D (weapons/trails)
└── data:   E1 → E2 → E3 → E4        (independent of B–D)
F (cadence/guards/prewarm) after A; per-item timing bits after D
```

Rough effort: A ~1 session · B ~1 · C ~3–5 · D ~2 · E ~1–2 per batch · F ~1–2.

## 9. Acceptance

Track completion in this file per phase. A phase is done when its exit
criteria hold, its tests are in the fast suite (or ignored-GRF suite), the
relevant ANIMATION_SYSTEM.md / gap-matrix rows are updated, and — for anything
player-visible — a live GUI check is recorded here with date and observer.
