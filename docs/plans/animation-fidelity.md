# Implementation Plan — Animation Fidelity: Visible Gaps & Architectural Debt

| | |
|---|---|
| **Status** | Active (2026-07-22) — phases A–D closed live; **E1 closed live (all 7 rows PASS on mechanism)** |
| **Milestone** | Post-M1 presentation fidelity |
| **Parent** | [ANIMATION_SYSTEM.md](../ANIMATION_SYSTEM.md) §7 Known gaps, [combat-animation-pipeline.md](../specs/combat-animation-pipeline.md) §14 gap matrix, FEATURE_ROADMAP.md "Classic skill-effect coverage pass" |
| **Depends on** | Native runtime boundaries (done); `provision-effect-roster` GUI roster (done) |
| **NEXT AGENT** | E1 **DONE + live-verified 7/7** (2026-07-23, rebuilt from the reverse-engineered original-client effect table). E2 **batch 1 DONE + live-verified** (10 units incl. Safety Wall/Sanctuary/Magnus/Ice Wall/portals). Next: E2 batch 2 (traps, Sage fields) or E3 remainder. See [classic-effect-fidelity.md](classic-effect-fidelity.md). |

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
   **Round 1 complete 2026-07-17 — all four roster characters verified live**,
   after finding and fixing two real bugs: (a) async `AnimationData`
   completions were never delivered to `this_entity`, silently discarding the
   local player's weapon layer; (b) the Raid/Meteor Assault procedural bursts
   were tuned so faint they read as a bare point light. Verified green:
   EffectSinX (katar layer, Attack3 thrust, Sonic Blow 滅 glyph, Meteor
   Assault burst, EDP status), EffectStalker (Rogue-alias weapon layer,
   Hiding, Raid starburst, authentic weaponless ReadyFight exit),
   EffectRune (weapon layer, Ignition Break STR), EffectKnight glance
   (Magnum Break cylinders + knockback, Pierce/Spear Stab spear layer —
   spear must be equipped; the "missing spear" report was a sword-equipped
   cast). Watch item: one unreproduced logout-to-char-select panic
   (rust-state safe-selector unwrap; backtrace capture armed).
2. **One mismatched-layer combo live** (female Novice melee or any first-class
   crit) to confirm the body-owned motion-index fallback visually.
   **Done 2026-07-17**: female Novice (EffectNovice) melee shows no layer
   blanking or freezing across sustained swings, dagger draws only during
   swing frames, and the dMotion hurt flinch plays on incoming hits.
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

**Closed 2026-07-17 (live-verified).** Implementation (`FrameEventCursor` +
`AnimationData::take_crossed_events` + `collect_crossed_events`; `SoundToken`
and `SoundState` deleted; events delivered before the completion transition
so final-frame crossings are not lost; stall recovery bounded to one full
cycle as a deliberate audio-flood guard). Unit tests in
`frame_event_cursor_tests` cover multi-motion jump, loop wrap, held final
frame, one-cycle stall bound, motion-program `step_serial`, playback-
identity reset, empty-body synthetic Attack (SinX katar), and authored-event
suppression. ANIMATION_SYSTEM.md §5 documents the cursor (the SoundToken
caveat is removed). **Live listen:** EffectKnight spear once-per-swing;
EffectSinX Jur once-per-swing after empty-ACT synthetic `ActionEvent::Attack`
fallback (body ACTs for male Assassin / Assassin Cross ship no events).

Exit: `SoundToken` deleted; ANIMATION_SYSTEM.md §5.3 caveat removed. **Met.**

## 4. Phase C — Runtime layer composition (large, engine)

Stop permanently flattening body/head/weapon into one merged `AnimationData`.
Retain per-layer ACT/SPR resources and compose at render time, matching native
`CActRes` behavior the pre-merge already emulates in data form.

Steps, each landable separately behind the existing `get_frame` API:

1. **Multi-layer `AnimationData`**: keep the per-pair `Actions` handles;
   body owns clock + motion index; secondary layers resolve via the
   `get_motion` motion-0 fallback (`0x004F5360`) at render time instead of
   merge time. The existing merge tests become composition tests.
   **Done + live-verified 2026-07-17 (C1):** `AnimationLayer` + `compose_frame` /
   `compose_action_motion`; loader no longer cross-merges; action-global AABB
   measured at load (`action_layouts`) and applied at compose; body-only
   events preserved (Phase B cursor still green). Head attach still load-baked
   (C2). **Live:** EffectSinX Jur dual-wield, head attach idle/walk (no jitter),
   basic attack + one sound, Sonic Blow glyph; EffectKnight spear/sword attacks
   hold weapon through swing, one sound each, Magnum shows sword.
2. **Attach points**: apply ACT attach data (head↔body alignment, the special
   motion-0 attachment cases) instead of baked offsets.
   **Done + live-verified 2026-07-17 (C2):** load stores motion `attach_point`
   only; compose applies `offset += -child + body` for every non-body layer
   that authors an attach (not hard-coded to layer 1). Body attach comes from
   the **body** motion index; when a secondary layer falls back to motion 0
   its attach is still that motion's point. Action AABB measurement uses the
   same rule. Unit tests pin the delta and the motion-0 fallback case.
   **Live:** idle/walk head on neck; after attack head still on body.
3. **Layer ordering + per-direction draw order** as authored.
   **Done + live-verified 2026-07-18 (C3):** shield paint order follows
   camera-relative facing — dirs `2..=5` (W/NW/N/NE, back half) draw shield
   **before** body so the torso covers it; dirs `0,1,6,7` draw shield after
   weapon (in front). Matches roBrowser `EntityRender` / classic hardcode
   (`behind = direction > 1 && direction < 6`). Unit test
   `shield_draw_order_follows_view_direction`. **Live:** Guard under body
   when facing away; in front when facing camera.
4. **Dynamic layer swaps**: weapon/head/appearance changes swap one layer
   without re-requesting the whole actor (today an appearance change reloads
   everything through `AsyncLoader`).
   **Done + live-verified 2026-07-17 (C4):** `AnimationLayer.path_key` +
   `load_layer`; `AnimationData::with_weapon_layer` / `with_head_layer`
   recompute layouts without touching body delays. `SetInventory` / equip /
   `ChangeWeapon` / `ChangeHair` call partial swap when body+head are present;
   job change still full-reloads. Unit test:
   `weapon_layer_swap_preserves_body_and_head_paths`. **Live:** head stays put
   on equip; attack animation follows weapon type (spear vs sword). Per-item
   weapon models remain Phase D.
5. **Shield layer** (`방패` paths) — first new layer type the architecture
   must absorb cleanly.
   **Done + live-verified 2026-07-18 (C5):** paths are
   `방패\{job}\{job}_{sex}_{suffix}` (not under `인간족`). Views 1–4 →
   `가드`/`버클러`/`쉴드`/`미러쉴드` (view ≥ 5 probes `{id}_방패`).
   `push_shield_part_file` only when SPR exists. `with_shield_layer` preserves
   weapon; `ChangeShield` partial-swaps and re-applies weapon. Local inventory
   maps classic shield item IDs 2101–2104 → views and refreshes both gear
   layers on equip (`refresh_entity_player_gear`). Weapon path selection uses
   content (`is_weapon_part_path`), never fixed `parts[2]`.
   **Original client confirmed:** same path form; Knight ships class shields
   (`file_exists` YES; listing tools under-report).
   **Live:** EffectKnight Sword + Guard idle/walk/attack; body covers Guard
   from behind.

**Phase C closed 2026-07-18.** Explicitly out of C (later phases):
hat/accessory stack, per-item weapons, `_검광` trails, Assassin L/R combo
(Phase D); skill/status presentation (Phase E).

Risks (historical): draw-call count / WSL GL — not blocking; layers are
already runtime-composed.

Exit: pre-merge deleted; Knight/roster render with body+head+weapon+shield;
appearance swap causes no full reload. **Met.**

### Phase C test map

Documented in [ANIMATION_SYSTEM.md §6](../ANIMATION_SYSTEM.md#6-diagnostics-and-tests).

| Suite | Command |
|---|---|
| C compose + gear | `cargo test -p korangar --lib runtime_compose_tests` |
| Shield item→view | `cargo test -p korangar --lib classic_shield` |
| Shield SPR path forms | `cargo test -p korangar --lib native_shield_paths` |
| B event cursor | `cargo test -p korangar --lib frame_event_cursor_tests` |
| Full lib | `cargo test -p korangar --lib` |

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

**Done 2026-07-18 (unit + GRF path probes + Ragexe RE; live GUI pending):**

- Hercules `PACKETVER ≥ 4` sends raw item nameids on LOOK_WEAPON / LOOK_SHIELD
  (`clif_get_weapon_view`). Local inventory now stores those nameids via
  `equipped_weapon_look` / `equipped_left_hand_look` instead of collapsing to
  class views only.
- `weapon_view_from_item_id` + `weapon_view_from_appearance` +
  `combine_dual_wield_view` / `effective_weapon_view` feed attack selection
  and class sprite fallback (dagger+dagger → 25, … sword+axe → 30).
- Sprite probe order: per-item → dual class pair (when off-hand is a weapon)
  → single class → none. Dual off-hand becomes a second weapon layer when the
  combined pair sprite was not used.
- `_검광` trails: native allowlist views 1–7 / 16–18 / 25–30 (Ragexe
  `0x976EC0`); per-item bases still probe `{base}_검광`. `_발광` not wired.
- Multi weapon-family partial swaps (`with_weapon_layers` /
  `apply_weapon_layers_swap`) keep body/head/shield while refreshing
  base+trail+off-hand together.
- `weapon-sprite-audit` reports per-item and trail inventory counts.
- Unit tests: item→view, dual combine, candidate order, trail suffix,
  multi-layer swap, native trail allowlist. GRF: Mjolnir `기사_*_1530` +
  trail on the classic weapon roster probe.
- Native path builders confirmed on
  `../../RO/client/2019-06-05fRagexe_patched.exe` (`0x7C4F90` weapon,
  `0x7C4B30` trail).

**LIVE GUI — DONE 2026-07-21.** All 8 rows PASS
(**[phase-d-live-verification.md](phase-d-live-verification.md)**). Bugs found &
fixed while verifying (branch `agent/platform-connectivity-controls`): idle-gear —
armed players stand in ReadyFight so weapon/shield render at idle, and relax to Idle
on town/safe maps (`78f57915`, `abb519f1`); dual-wield attack re-trigger (`4df41419`);
respawn/resurrect sprite revive (`78f57915`); ammo equip/stack/count +
normal-bow-attack arrow projectile (`d3f7c5dd`, `2b637bac`).

Exit: audit tool enumerates per-item/trail coverage; roster + a per-item
weapon (e.g. Mjolnir 1530 on Knight) verified live. **Met — code exit AND live GUI
signed off. Phase D complete; next engine work is Phase E (§6).**

## 6. Phase E — Skill/status presentation batches (data track, parallel)

Continue the proven per-batch method (wire probe → asset dump → roBrowserLegacy
semantic cross-check → typed recipe → GRF audit test → live check). Suggested
batch order by campaign visibility:

### E1 — Code-drawn classic effects — **CLOSED LIVE 2026-07-22**

Procedural `EffectBase` recipes on top of `FallingBolts` / `SkillBurst` /
`SkillProjectile`. **Live GUI pass complete 2026-07-22 — all 7 rows PASS on
mechanism** (spawn position, timing vs the damage number, texture loading); full
per-row evidence and the driving traps are in
[phase-e1-live-verification.md](phase-e1-live-verification.md).

**Exit met — with one explicit caveat:** the pass verified *mechanism only*. All
seven effects remain procedural stand-ins that do not read as RO spells; that is
tracked separately in
[classic-effect-fidelity.md](classic-effect-fidelity.md) and is not an E1
regression.

| Skill (ID) | Track | Implementation |
|---|---|---|
| Napalm Beat (11) | Target burst | `SkillBurstStyle::NapalmBeat` (violet rings) |
| Soul Strike (13) | Travel + hit | `SoulStrikeOrbs` (orb count = `hit_count`) + existing hit STR |
| Frost Diver (15) | Travel + hit | `TravelBall(FrostDiver)` + `freeze.str` hit |
| Fire Ball (17) | Travel + hit | `TravelBall(FireBall)` + firehit |
| Jupitel Thunder (84) | Travel + hit | `TravelBall(Jupitel)` + lightning/wind hits |
| Earth Spike (90) | Target geometry | `SkillBurstStyle::EarthSpike` + earthhit |
| Heaven's Drive (91) | Target geometry | `SkillBurstStyle::HeavensDrive` (6-spike ring) + earthhit |

Recipe table: `world/skill_recipe.rs`. Spawn unified through
`spawn_damage_caster_skill_effect` (projectiles) and
`spawn_damage_target_skill_effect` (target bursts). Unit tests:
`phase_e1_code_drawn_recipes_are_wired`. GRF audit
`all_mapped_skill_effect_assets_exist` green for E1 textures.

**Live checklist (E1):** Mage/Wizard on field — cast each skill above, confirm
travel or burst is visible (not sound/point-light only). Record date + observer
here when done.

- **E2 — Persistent skill units**: **batch 1 DONE + live-verified 2026-07-23**
  — typed `unit_recipe.rs` table (Safety Wall, Fire Wall, Pneuma, warp
  portals, Sanctuary, Magnus, Fire Pillar armed, Ice Wall, Quagmire), exact
  teardown on `RemoveSkillUnit`, two live bugs fixed (sprite-change crash,
  warp-cancel modal). Remaining: Hunter traps, Sage ground fields, Venom
  Dust, song/dance areas — see classic-effect-fidelity.md "Phase E2".
- **E3 — `DisplaySpecialEffectPacket` (0x01F3) + cast circles + sound
  retry**: **Partial 2026-07-22** — 0x01F3 promoted → `NetworkEvent::SpecialEffect`
  → `special_effect_recipe` (E1 IDs + common STR/procedural). Travel balls and
  Soul Strike orbs carry mid-flight point lights. Semantic shape hints
  (`EffectShape`: orb/ring/spike/ball/flash/str) for recipe design without
  third-party code. Still open: cast-target circles (EF_LOCKON), catalog-wide
  ID coverage, sound retry.
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
