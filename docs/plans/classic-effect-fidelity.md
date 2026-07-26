# Classic effect fidelity — bringing authentic RO skill effects to Korangar

| | |
|---|---|
| **Status** | 2026-07-26: **E1 (7 skills) + E2 batches 1 & 2 all live-verified.** E2 batch 2 closed 6/6 (Volcano, Deluge, Violent Gale, Land Protector, Venom Dust, Demonstration). **Depth-tested ground-decal pass landed + live-verified** — Land Protector now draws under the player. **Status-effect entity visuals landed + live-verified 2026-07-25** (freeze cyan, petrification greyscale — see §Status-effect entity tints). Code green (223 lib tests + GRF asset audit). **Hunter traps LANDED + live-verified 2026-07-26** via runtime RSM prop spawning; the shadow question resolved as a non-bug (it falls under the prop). No engine follow-ups remain. **Song/dance investigated + closed 2026-07-26:** 18 of 20 create no ground unit at all (not a missing asset), and the two that do — `CG_MOONLIT` and `CG_HERMODE` — are now mapped from the reverse-engineered table, **code+tests only, not yet seen or heard live**. |
| **Branch** | `agent/platform-connectivity-controls` |
| **Parent** | [animation-fidelity.md](animation-fidelity.md) §6 Phase E |
| **Trigger** | Phase E1 skills work but "don't look right" — particles travel, but they don't read as RO spells |

## The core finding

RO effects come in **two distinct families**, and Korangar only supports one of them.

| Family | Location | Used by | Korangar support |
|---|---|---|---|
| **STR scripts** | `data\texture\effect\*.str` | Ground / AoE spells | ✅ Full — `EffectLoader`, `EffectAsset::Fixed` |
| **Sprite effects** | `data\sprite\이팩트\*.spr` + `.act` | Classic single-target spells | ✅ Pipeline (`SpriteEffects` + `SpriteTravel`) — Soul Strike live-OK, Fire Ball mapped via the reverse-engineered table |

This cleanly explains the symptom. The skills that look right are all STR-backed:
Thunderstorm (21), Sanctuary (70), Magnus (79), Fire Pillar (80), Meteor (83),
Lord of Vermilion (85), Storm Gust (89), Quagmire (92), Hammer Fall (110).

The seven Phase E1 skills have **no `.str` file at all**, so they were given
procedural code-drawn stand-ins — flat textures on generated geometry. That is
why they read as "particles moving" rather than as RO spells.

### Proof (ground truth, not API-derived)

`GameFileLoader::get_files_with_extension` **under-reports the GRFs badly** — it
listed 26 root-level `.str` files while `data.grf` actually contains **311**, and
it omitted `stormgust.str` / `sanctuary.str` / `firepillar.str` which the client
demonstrably loads. Do not trust it for inventory work.

Ground truth came from parsing the GRF file table directly (v2xx: zlib-compressed
table at `0x2E + offset`, CP949 filenames). `data.grf` holds **151,062 entries**,
**2,070** of them `.str`.

Probed with `file_exists` (reliable, unlike listing) — all absent:

```
napalmbeat.str  napalm.str  soulstrike.str  soul.str  fireball.str  fball.str
frostdiver.str  jupitel.str  jupitelthunder.str  earthspike.str  spike.str
heavensdrive.str  heavendrive.str
```

Only `freeze.str` / `freezed.str` exist — those are Frost Diver's **freeze status**,
not its projectile. Control group (`stormgust`, `thunderstorm`, `sanctuary`,
`firepillar`, `quagmire`, `lord`) all FOUND, confirming the probe works.

## The classic sprite library

`data\sprite\이팩트\` — **164 root-level sprites**, each with a matching `.act`.
Verified spr+act pairs include:

| Asset | Meaning | Likely use |
|---|---|---|
| `soule.spr` | soul | **Soul Strike** flying ghosts |
| `유령.spr` | ghost | Soul Strike / ghost-element |
| `waterball.spr` | water ball | Water Ball |
| `썬더스톰.spr` | thunderstorm | Thunderstorm |
| `거스트.spr` | gust | Storm Gust |
| `화염진.spr` | fire formation | fire ground |
| `얼음땡.spr` | freeze-tag | freeze status |
| `일섬.spr` | flash | slash effects |
| `창.spr` | spear | already used — Spear Boomerang |
| `sight.spr` | Sight | Sight |
| `particle1-7.spr` | particles | generic particle beds |
| `poisonhit.spr` | poison hit | poison |

### The authoritative mapping (landed 2026-07-23)

The dead end was guessing GRF filenames. The fix was the **reverse-engineered
original-client effect table**: roBrowserLegacy's `SkillEffect.js` (skill →
effect ID, reproducing the exe's hardcoded table) + `EffectTable.js` (effect ID
→ files/primitives, with comments citing the C++ client's own constants and
values). We used it as **data only** — which files and parameters each skill
uses — and wrote our own renderers, so CLAUDE.md §5 (no GRAVITY code) holds.

Skill → original effect IDs: Napalm=32/hit=1 · Soul Strike travel=15/hit=1 ·
Frost Diver travel=27/hit=28 · Fire Ball travel=24/hit=49 · Jupitel ball=93/
hit=94 · Earth Spike=79/hit=147 · Heaven's Drive=142/hit=147.

Verified three ways (2026-07-23 double-check): skill IDs against
`Hercules/db/pre-re/skill_db.conf`; effect ID numbering against Hercules
`doc/effect_list.md`; and the same numbering against `ragnarok-packets`'
`EffectId` enum discriminants (Hit2=1, Soulstrike=15, Fireball=24,
Frostdiver=27/28, Firehit=49, Earthspike=79, Yufitel=93/94, Heavensdrive=142,
Earthhit=147 — all three sources agree). roBrowser's `spriteName` resolution
was also confirmed against its `EffectManager.js`: `spriteName: 'fireball'`
loads `data/sprite/이팩트/fireball.spr`, matching our mapping.

**EF_NAPALMBEAT (32) recovered:** roBrowser left the skill's own effect
unimplemented (mojibake'd filename comment, "eight files for an animated
explosion"). The set is `effect\폭발1.tga`–`폭발8.tga` (폭발 = explosion, the
only 8-frame explosion cycle in the GRF) — now played as a three-puff cluster
at the target over the effect-1 lens streaks, which is why the recipe keeps
`ef_napalmbeat.wav` (it belongs to effect 32, while effect 1 carries the
generic `ef_hit2`).

Key insight: **Napalm Beat, Earth Spike, and Heaven's Drive are procedural in
the original client too** (lens-streak circle pattern; textured stone horns) —
our procedural approach was right, it just used the wrong textures/shapes.

E1 presentation as implemented (every asset verified in `data.grf`):

| Skill | Presentation (per original effect table) |
|---|---|
| 11 Napalm Beat | 3-puff `폭발1–8.tga` explosion cluster (effect 32) over 8 × `lens1/lens2.tga` streaks converging in a circle (effect 1) |
| 13 Soul Strike | `이팩트\soule` multi travel — **live OK 2026-07-22** (kept) |
| 15 Frost Diver | `effect\ice.tga` travel (effect 27 uses exactly this file) + `freeze.str`; launch `ef_frostdiver.wav`, hit `ef_frostdiver2.wav` |
| 17 Fire Ball | `이팩트\fireball` sprite travels with 4 dimmed trail ghosts (effect 24 = 5 low-alpha duplicates); fire-hit STR; launch `ef_fireball.wav`, hit `ef_firehit.wav` |
| 84 Jupitel Thunder | Animated `thunder_ball_a–f.bmp` ball over `thunder_center.bmp` glow (93); hit = growing `thunder_pang.bmp` + `thunder_plazma_blast` frames (94); launch `hunter_shockwavetrap.wav` |
| 90 Earth Spike | 1 main + 4 small `effect\stone.bmp` horns rise/hold/sink (79) + `earthhit.str` + `wizard_earthspike.wav` |
| 91 Heaven's Drive | 5×5 cell grid of stone horns (142) + `earthhit.str` + `wizard_earthspike.wav`, once per cast |

Deliberate deviations from the original: Earth Spike horns hold 2.0 s (original
5 s — combat readability); no camera quake (no engine support yet); Heaven's
Drive horns share the cast cell's ground height (no per-cell terrain sampling);
Fire Ball's lead sprite is full-alpha with dimmed ghosts (all-0.2-alpha stacks
need additive blending the sprite path doesn't do).

## Backend — DONE 2026-07-22

The pipeline now supports both families. Full `korangar` lib suite green
(193 passed), no new warnings.

- **`world/sprite_effect.rs`** (new) — `SpriteEffects` holder: per-path lazy
  loading, lifetime/expiry, and rendering through
  `AnimationData::render_action_frame`, the same path emotes use, so ACT frame
  timing and per-frame offsets behave correctly. 4 unit tests.
- **Sentinel routing** — sprite loads are keyed by `EntityId` sentinels counting
  down from `u32::MAX - 1`, just below the emote sentinel (`u32::MAX`), so the
  two never collide and a real entity ID can never resolve to a sprite. Covered
  by test.
- **`EffectAsset::Sprite { path, action_index }`** + `EffectAsset::sprite(path)`
  helper. `resolve()` now returns **`ResolvedEffect::{Str, Sprite}`** rather than
  a bare `&str`, so the two families are impossible to confuse at a call site.
- **`Client::spawn_resolved_effect`** — single dispatch point for both families;
  the hit-effect and ground-effect call sites now share it instead of each
  open-coding the STR loader. Handles the cache-hit case where
  `request_animation_data_load` returns data immediately.
- **Lifecycle** — cleared on map change alongside emotes, updated each tick.

Unloaded spawns expire on a 3 s fallback so a sprite that never arrives cannot
leak. First cast of a skill draws nothing while its sheet loads; later casts are
immediate — identical to emote behaviour.

## Remaining work

1. ~~**Add a sprite-effect asset kind.**~~ **DONE — see "Backend" above.**
2. **Anchor correctly.** The emote work found the generic animation loader
   normalizes each ACT frame to its bottom edge; emotes fixed this by deriving the
   anchor from the composed sprite-frame height. Effects need the same care —
   ground-anchored (Earth Spike) vs entity-centered (Napalm Beat) differ.
3. **Map skill → asset.** No authoritative table ships with the client — the
   skill→effect-ID mapping is hardcoded in the exe, and `effecttool\*.lub` is
   per-map ambient effects, not skills. Mapping must be derived by inspecting
   candidate sprites and comparing against reference video/screenshots.
4. **Verify live**, per [phase-e1-live-verification.md](phase-e1-live-verification.md).
   The `test` character (char_id 150000) is already a Wizard with all seven E1
   skills bound to F1-F7 — see that doc's "Test character" section.

### E1 sprite pass — status 2026-07-23 (evening)

The reverse-engineered mapping (see "The authoritative mapping" above) replaced
all filename guessing. New machinery this landed:

- `ProjectileRecipe::JupitelBall` + `SkillProjectile::jupitel_ball` — animated
  frame-cycle billboard with a static core glow, screen-upright, additive.
- `ProjectileRecipe::SpriteTravel.trail_ghosts` + per-spawn `alpha` on
  `SpriteEffects::spawn_travel` (ghost instructions are dimmed post-render).
- `SkillBurstStyle::JupitelHit` (frame-cycle burst via `SkillBurst::with_frames`),
  rebuilt `NapalmBeat` (lens circle pattern) and `EarthSpike`/`HeavensDrive`
  (crossed-triangle stone horns, rise/hold/sink, deterministic per-horn jitter).
- `SkillPresentationRecipe::projectile_sounds` — launch sounds riding the
  once-per-cast travel gate.

**Live pass 2026-07-23 (user, F1–F7):**

| Key | Skill | Result |
|---|---|---|
| F1 | Napalm Beat | PASS — explosion cluster on target |
| F2 | Soul Strike | PASS (unchanged) |
| F3 | Frost Diver | PASS — ice travels, freeze burst on hit |
| F4 | Fire Ball | PASS — travel + explosion (reads yellow-ish) |
| F5 | Jupitel | PASS — crackling ball + burst on impact |
| F6 | Earth Spike | PASS after two live tweaks: duration 2.0 s → 3.5 s, horns sized up (main 1.35 → 2.1 tiles, minors 0.55 → 0.95) |
| F7 | Heaven's Drive | PASS — 5×5 stone-horn grid |

**All seven live-verified 2026-07-23** (committed `7e6daf85`). Session
write-up: [2026-07-22-session-notes.md](../2026-07-22-session-notes.md).

## Phase E2 — persistent skill units (batch 1, live-verified 2026-07-23)

Skill units now resolve through a typed table (`world/unit_recipe.rs`) instead
of the old hardcoded Firewall/Pneuma match. A unit lives from `AddSkillUnit`
until the server's `RemoveSkillUnit` — exact teardown, no client timers.
Mapping source: the same reverse-engineered original-client tables
(`SkillUnit` unit-ID→effect-ID, then the effect table for assets); every
referenced asset is GRF-verified by the extended
`all_mapped_skill_effect_assets_exist` audit.

New machinery (`world/effect/unit.rs`): `UnitCylinders` — layered rotating
textured cylinders, N-sided (4 = the original's square map-unit footprint),
with a steady point light; `UnitIceHorns` — per-cell ice horn cluster with
grow-in. STR-backed units loop `EffectWithLight` (repeating) under the unit's
entity id.

| Unit | Presentation | Live |
|---|---|---|
| Safety Wall | **looping `safetywall.str`** + ef_glasswall.wav | PASS — procedural pyramid/cylinder attempts read as flat light blobs; Gravity's authored STR was the answer |
| Fire Wall | looping `firewall.str` (+ new ef_firewall.wav) | PASS (3 server cells) |
| Pneuma | looping `pneuma1.str` **(was one-shot before — vanished)** + wide soft dome marking the 3×3 footprint | PASS after dome+brightness |
| Warp portal (pre/active) | flat blue ring → ring + swirling ring_blue vortex + portal wavs | PASS after brightening |
| Sanctuary | square magic_green glow ×2 layers, 45° yaw | PASS after brightening |
| Magnus | square ring_red pillars ×2 layers, 45° yaw | PASS after brightening |
| Fire Pillar (armed) | 3 nested magic_red swirls | PASS after brightening |
| Ice Wall | 3 ice.tga horns per cell + wizard_icewall.wav | PASS (5 server cells) |
| Quagmire | looping `quagmire.str` + wizard_quagmire.wav | PASS (light raised) |

**Live lesson:** additive cylinder alphas need ~0.5–0.8 to read against lit
terrain (0.3–0.4 washes out) — same class of finding as the 2026-07-17 Raid/
Meteor Assault brightening.

### Bugs found & fixed during the E2 live pass

1. **Client crash on warp** — `SpriteChangePacket` Base for *any* entity was
   mapped to `NetworkEvent::ChangeJob`, and the handler unconditionally
   rebuilt the local skill tree from that entity's class id →
   `SkillTree::get` unwrap panic for non-player ids. Fixed: skill-tree rebuild
   gated on the local player, `Library::try_get` + empty-layout fallback,
   entity lookups de-unwrapped (ChangeJob + ChangeHair), regression test
   `classic_first_and_second_jobs_have_skill_trees`.
2. **Warp cancel left the server modal** — `cancel_warp_selection` was a
   no-op (wrong "no server-side state" assumption); every later skill failed
   with "Any work in progress…". Fixed: send the warp-point selection with
   the literal map name `"cancel"`, which Hercules' `skill_castend_map`
   (skill.c:12280) treats as the dismiss command. Verified live (cancel →
   recast works). Caveat: only the Cancel button sends it — closing the
   window another way still leaves the server state set (follow-up).

### Open items

- ~~Cast bar for menu/priest skills~~ **RESOLVED 2026-07-23**: packets healthy
  (`[skill-cast]` diagnostic — warp 1000 ms, Magnus 2005 ms) **and the bar
  renders** (user-confirmed on Magnus). The earlier "no cast bar" report was
  short DEX-reduced casts going by unnoticed.
- Warp-selection window close-without-Cancel leaves server menuskill state.

### Batch 2 (mappings pulled + GRF-verified 2026-07-23) — CLOSED LIVE 6/6 2026-07-24

All from the reverse-engineered original-client tables, every asset confirmed
present in our `data.grf` and pinned in `all_mapped_skill_effect_assets_exist`.

| Unit | Original presentation | Assets (verified) | Status |
|---|---|---|---|
| Volcano | Rotating, size-pulsing truncated cone on the ground (`PropertyGround`: top 3.0 cells, bottom 1.0, height 2, pulse 0.5–1.0×) | `effect\ring_red.tga` | **Wired** — `UnitCylinders`, top 15 / bottom 5 / height 10 world, alpha 0.7 |
| Deluge | Same geometry | `effect\ring_blue.tga` | **Wired** — shares `ELEMENTAL_FIELD_CYLINDERS` |
| Violent Gale | Same geometry | `effect\ring_yellow.tga` | **Wired** — shares `ELEMENTAL_FIELD_CYLINDERS` |
| Land Protector | Flat pulsing texture tile per cell (`LPEffect`, size ~0.8 cell) | `effect\aaa copy.bmp` | **Wired** — new `UnitGroundQuad` body, half-size 2.0 (= the table's 0.8-cell tile) |
| Venom Dust | `이팩트\particle3` sprite looping at the cell (effect 171: size 80, rising, `repeat: true`) | `particle3.spr/.act` | **Wired** — new looping-sprite unit mode |
| Demonstration (Alchemist bomb) | `이팩트\데몬스트레이션` sprite (effect 302) | `.spr/.act` | **Wired** — same looping-sprite mode |
| Hunter traps ×10 (Skid/Ankle/Land Mine/Blast/Shockwave/Sandman/Flasher/Freezing/Claymore/Talkie Box) | **RSM 3D prop models**, one per trap type | `data\model\외부소품\트랩01.rsm`–`05`, `03_2`–`03_6` (all 10 present) | **DONE 2026-07-26** — runtime RSM prop spawning shipped. Models preload into the map's shared geometry buffer at load; a spawned trap is an ordinary `ModelInstruction`. Mapping is 1:1 (Ankle=01, Skid=02, Land Mine=03, Freezing=03_2, Blast=03_3, Sandman=03_4, Flasher=03_5, Shockwave=03_6, Claymore=04, Talkie=05). Scaled to `TRAP_PROP_SCALE` 0.25 — eyeballed, no scale in the table. The "baked into vertex buffers" claim that deferred this was **wrong**; see the section below |

New machinery this pass:

- `UnitPulse` on `UnitCylinderSpec` — breathing radius scale, driven by an
  unwrapped `age` (not `spin`, which wraps at `TAU` and would step the pulse).
  Culling uses the pulse's widest extent.
- `UnitGroundQuad` — one ground-aligned quad per unit cell, lifted 0.6 world
  units off the terrain so it cannot z-fight the ground mesh.
- Looping sprite units in `SpriteEffects`: a spawn tagged with the unit's
  entity ID never expires on a client timer (`lifetime_ms` → `u32::MAX`) and
  wraps its ACT clock, because `render_action_frame` clamps to the last frame
  instead of looping. `remove_unit` is called from **both** `RemoveSkillUnit`
  and entity removal, mirroring `EffectHolder`.
- `UnitPointLight` — light-only companion registered under the unit's entity
  id for bodies that can't render their own light (`UnitGroundQuad`, looping
  sprites). Caught in review: those recipes declared `light:` that nothing
  consumed — `UnitCylinders` and the STR path self-register, these didn't.

### Song/dance areas — RESOLVED 2026-07-26: almost nothing here is E2 work

Investigated because roBrowser marks the `27x_ground` range **Tofix**. Three
rounds of asset hunting produced two wrong answers before the real one, so the
conclusions are recorded with the evidence that settled them.

**1. The skills mostly do not create ground units.** In the renewal
`db/re/skill_db.conf` this server uses, only **2 of 20** song/dance skills declare
a `Unit:` block: `CG_MOONLIT` (`Id: 0xb5`, `Layout: 4`, `UF_ENSEMBLE`) and
`CG_HERMODE`. Every `BA_*`, `DC_*` and `BD_*` is `SkillType: Self` /
`SkillInfo: { Song: true }`, so Hercules never sends `AddSkillUnit` for them and 18
`unit_recipe.rs` entries would have been **dead code that could never fire**. The
`UNT_*` song ids do exist in the enum and are referenced from `skill.c`'s
pre-renewal / `onplace_timer` paths, which is what makes the enum look like it
implies ground units. **Check `skill_db.conf` for a `Unit:` block before hunting
any skill-unit asset.**

**2. The status side is 15 skills, not 18, and E4 cannot just absorb them.**
Of the 18: `BA_DISSONANCE` and `DC_UGLYDANCE` declare **no `StatusChange` at all**
(splash damage / SP drain), and `BD_LULLABY` maps to the shared **`SC_SLEEP`**,
which the client already renders via `sleep.str`. The other 15 do carry per-song
statuses with real icons (`SI_WHISTLE`, `SI_POEMBRAGI`, …), so they already reach
the buff bar from the M1-010 work — songs are **not** invisible today.

Wiring per-song *entity* visuals is **new plumbing, not configuration**. The E4
machinery does not fit, in three specific ways:
  - `update_status_effect_visual` is keyed only on `body_state` / `health_state`
    (opt1/opt2 bitfields) via `status_effect_asset`. Song statuses arrive on a
    different channel — `NetworkEvent::StatusChange { index, .. }` — which today
    feeds only the buff bar and has **no visual path at all**.
  - It loads **`.str` scripts** via `effect_loader`. The song textures are bare
    flat images with no `.str`, so they need a procedural renderer like the E1
    recipes.
  - It is **single-slot per entity** (`active_status_effects: HashMap<EntityId, _>`).
    A Bard/Dancer party stacks several songs at once; one slot cannot represent that.

**3. `CG_MOONLIT` and `CG_HERMODE` are DONE 2026-07-26 — both recovered.** An
earlier revision of this section called their visuals "not recoverable". **That was
wrong, and the mistake was not checking roBrowser's `SkillUnit.js`** — it is keyed
by *unit id*, so Moonlit (`0xb5`) and Hermode (`0xb9`) live there, nowhere near the
`27x_ground` song range whose `Tofix` note I had wrongly generalised from memory.

- **`CG_HERMODE` → `EF_BOTTOM_HERMODE` (517), which is an explicitly EMPTY entry**
  commented `(Nothing)`. Its only presentation is the `517_music` variant: a sound,
  `attachedEntity: true`. **Drawing nothing is the authentic behaviour**, so the
  recipe is sound-only and a test (`hermode_is_deliberately_audible_only`) pins it
  so a later pass doesn't "fix" the absent visual.
- **`CG_MOONLIT` → effect 394**, a `FlatColorTile`: no texture, a flat translucent
  **salmon** quad, colour verbatim `0xff8abb` at alpha `0.6`, `uSize 0.5` over
  ±1.0 vertices = one full cell → `half_size` `GAT_TILE_SIZE / 2` (2.5), where Land
  Protector's 2.0 is its narrower 0.8-cell tile. The original does not pulse.

**Two traps this turned up.** First, Hercules names effect 394 **`EF_SPHEREWIND2`**,
which has nothing to do with the skill — the table's own comment is "Moonlit water
mill/sheltering bliss". Same family as the nine-of-eighteen wrong skill-id guesses:
**never take an effect from its constant name.** Second, roBrowser's
`'NNN_ground' // Tofix` entries are *guesses at an id it never implemented*, but the
id itself is usually right — 291/292/293/294/370/405/522 all validate against
Hercules' `EF_BOTTOM_*`. So a `Tofix` marker means "no renderer here", **not** "no
information here"; follow the number into `EffectTable.js` anyway. That is exactly
what turned Moonlit from "unrecoverable" into a verbatim colour value.

Both sounds are real and audited: `data\wav\effect\달빛세레나데.wav` and
`data\wav\effect\헤르모드의 지팡이.wav`, confirmed through
`all_mapped_skill_effect_assets_exist` (which does cover `presentation.sound`).
**Neither has been seen or heard live yet.**

#### Sizing yardstick: is this a ground texture?

Real ground textures here are **64×64** (`ring_red.tga`, elemental fields) to
**128×128** (`aaa copy.bmp`, Land Protector). Two candidate families were measured
against that and both are icons:

1. `data\sprite\아이템\{skill_constant}.spr` — exists for all 36
   Bard/Dancer/ensemble/Clown-Gypsy skills and looks like a perfect hit. Every one
   is a single **26×26** frame: a skill icon.
2. `data\texture\effect\{skillname}.tga` — uniformly **32×32, 4140 bytes**, and so
   are `efst_abyss_slayer.tga` (a known status indicator) and
   `unlimitedhummingvoice.tga` (a buff). This is the **EFST** family. It is the
   right art for the *status* layer, and was briefly mis-recorded here as the
   ground assets. The three songs with no English-named TGA are exactly the three
   with no dedicated status — good internal confirmation.

Both were settled by reading image headers through korangar's loader: many entries
are **DES-encrypted** and `tools/grf_list.py` skips them silently, while korangar
can read them (`loaders/archive/native/mixcrypt.rs`).

#### A new (partial) authority: the exe's effect-name pool

`client/2019-06-05fRagexe_patched.exe` (see the reference tree, `/Volumes/T7/GitHub/RO`)
contains a pool of effect `.str` names around offsets **7458420–7461300**, ordered
by `EffectId` **in runs**. Calibrated against `db/constants.conf` it resolves
cleanly inside a run — e.g. `poison.str` is **`EF_POISONATTACK` (192)**, *not*
`EF_POISON` (335); `Deffender.str` is `EF_DEFFENDER` (213), not `EF_DEFENDER` (222);
and `moonlight_1/2/3.str` is **`EF_LEVEL99/_2/_3` (200–202)**, the level-99 aura,
**not** `CG_MOONLIT`. It also *skips* `EF_PETRIFYATTACK` (195) and `EF_CURSEATTACK`
(196), independently confirming this plan's finding that `stone.str` / `curse.str`
do not exist.

**Its limit, which matters:** ordering holds within a run but resets between them
(`strip_weapon`→`strip_helm` is 269→272, then the next entry `shield_charge` is
246). So it can confirm a mapping but cannot be interpolated. Used that way it
shows **ids 273–294 have no `.str` entry at all** — the `EF_BOTTOM_*` song effects
are exe-hardcoded procedural effects over flat textures, the same standing as the
`Lockon` / `Beginspell` cast circles.

Two further notes: `EF_BOTTOM_*` (277–294) maps to the 18 units by **name, not
index** (Dissonance is 1st there, 9th in `UnitId`), and
`clif_getareachar_skillunit` sends only `unit_id`, never an effect id — so any
unit→visual mapping is necessarily client-side knowledge.

**Related discovery:** the reference client's `data.ini` loads two GRFs korangar
never registers — `renewal2021.grf` (UI textures) and `resources2021.grf` (~30k
item/accessory sprites) — at *higher* priority than `data.grf`. Likely home of
missing item icons.

### Batch 2 live pass — COMPLETE 6/6, 2026-07-24

| Unit | Live |
|---|---|
| Volcano | PASS after two size corrections (below) |
| Deluge | PASS — blue palette swap of Volcano, which is exactly the original's design |
| Violent Gale | PASS — yellow palette swap, same shape |
| Land Protector | PASS — authentic flat floor tile (121 units at Lv5), draws **under** the player via the depth-tested ground-decal pass (2026-07-24, see below). The interim low square glow it replaced is gone. |
| Venom Dust | PASS — 15 units, each `body=looping-sprite` with its `particle3` sprite; first live proof of the looping-sprite path. Poison confirmed applying (porings take damage), though the status itself has **no entity visual** yet |
| Demonstration | PASS 2026-07-24 — fire field renders on the ground (looping-sprite path). See the NOFOOTSET trap below: the cast is rejected with the misleading "Skill level is not high enough" unless aimed at open ground clear of characters |

**Demonstration trap — "Skill level is not high enough" is authentic server
behavior, not a level or client bug.** Demonstration's unit has `UF_NOFOOTSET`
and the live server config has `skill_nofootset: 1` (`BL_PC` is bit 1, so it
applies to players). At cast-end `skill_castend_pos` runs `check_unit_range2`,
which scans ~a 5×5 area (`get_unit_range` 1 + layout radius) around the target
cell for **any** `BL_CHAR` — **including the caster** (`check_unit_range2_sub`
only excludes a corpse and an Emperium under Demonstration). If one is found it
rejects with `USESKILL_FAIL_LEVEL`, whose client string is the misleading
"Skill level is not high enough." (`skill.c:12091`, `battle/skill.conf:141`).
The client makes it worse: for a Ground skill clicked **on a monster**,
`resolve_pending_cast` returns `CastEntityTile` and places the field on that
monster's own cell — guaranteed footset fail. **Placement rule:** arm the
skill, then left-click **bare ground 4–5 cells away** from your own character
and from any mob. Same family as Venom Dust's interval-only behavior — an
overloaded Hercules failure code, verified in source.

**Draw-order limit — RESOLVED 2026-07-24 with a depth-tested ground-decal
pass.** The problem: `EffectInstruction` carries only screen-space corners and
renders in `passes/postprocessing/effect.rs`, so **every** effect composites on
top of the whole scene, entities included. Vertical bodies (Sanctuary, Fire
Wall, the elemental cones) get away with it — a glow in front of the character
reads fine and matches the original. A ground-parallel quad did not: Land
Protector's tiles landed on top of the player sprite.

**The fix (live-verified — tiles now draw under the character):** a new
depth-tested ground-decal pass, built by generalizing the walk indicator to N
textured instances.

- `GroundDecalInstruction` (`graphics/instruction.rs`) keeps four **world-space**
  corners + uv + color + `Arc<Texture>` — unlike `EffectInstruction`, which
  discards depth.
- `EffectRenderer::render_ground_decal` (`renderer/effect.rs`) is the world-side
  entry; `UnitGroundQuad::render` calls it instead of `render_effect_world_quad`.
- `passes/forward/ground_decal.rs` (`ForwardGroundDecalDrawer`) draws in the
  forward pass **after** the walk indicator and **before** entities, so terrain
  occludes decals and entities compose over them. Depth-**tested** (reverse-Z
  `Greater`) but **not** depth-writing (translucent). Shaders
  `ground_decal_bindless.slang` / `ground_decal.slang` (emissive flat quad,
  corners from the instance buffer). Batches by texture — Land Protector's 121
  tiles share one texture, so it is a single instanced draw on both the bindless
  (Metal) and GL fallback paths; no bindless dependency.
- Land Protector's recipe (`world/unit_recipe.rs`) is restored from the interim
  low square glow to the authentic `UnitBody::GroundQuad` (`half_size 2.0` =
  0.8-cell tile, translucent so the magic pattern reads over the floor).

This pass is the shared prerequisite the plan flagged for **future ground
effects and DM prop markers** — reuse `render_ground_decal` for those.

**The scaling bug — read this before adding any per-cell unit.** The effect
table's `PropertyGround` sizes are in the effect's **own world units, not
cells**. Reading `top 3.0 / bottom 1.0` as cells and multiplying by
`GAT_TILE_SIZE` made each cone 6 cells wide; Hercules `Layout: 3` is a **7×7
square, so the server sends 49 separate `AddSkillUnit` packets** and the field
rendered as one solid block of fire. Corrected to a per-cell cone at roughly
cell pitch (`bottom 2.0 / top 5.0 / height 10.0`, alpha 0.45, 12 sides — the
quad count is paid 49× per field). Sanctuary/Magnus in batch 1 were already at
this scale, which is the tell: **treat table sizes as world units.**

**Server behaviors that look like client bugs — they are not:**

- *"Volcano does no damage."* `NoDamage: true`. It grants `SC_VOLCANO`:
  renewal Lv5 = +30 ATK/MATK and +20% Fire damage
  (`skill_enchant_eff[]`) to anyone standing in it. Verify via the stats
  window, not by hitting things.
- *"It never goes away."* `SkillData1` Lv5 = 300000 ms = **5 minutes**.
- *"The other fields won't cast."* `skill.c:18732`: *"The official
  implementation makes them fail to appear when casted on top of ANYTHING."*
  A 7×7 Volcano blankets the area for 5 minutes, so an overlapping Deluge
  produces zero units and looks like a failed cast. `@warp` to a fresh map
  between field casts.
- *"Nothing happens when I press the key."* All four need a **Blue Gemstone**;
  Land Protector needs a **Yellow** one too (`@item 717 20`, `@item 715 20`).
  Rejections arrive as `ZC_ACK_TOUSESKILL` and print to chat — easy to miss
  in combat spam. `@monsterignore` also stops mobs interrupting the ~2 s cast.

Remaining: none — all four Sage fields, Land Protector, Venom Dust, and
Demonstration are live-verified (batch 2 closed 6/6, 2026-07-24).

### iRO Wiki visual brief (acceptance language, not asset table)

Source: [Wizard](https://irowiki.org/wiki/Wizard) + linked skill pages (2026-07-23).
Wiki has **no GRF filenames** — use for “what should it look like,” not “which file.”

| Skill | Wiki intent |
|---|---|
| Napalm Beat | Psychokinetic hit on target; Ghost; AoE damage split |
| Frost Diver | Stream of frigid ice → target; freeze chance |
| Fire Ball | Fireball travel; splash AoE at impact |
| Jupitel | Crackling lightning **ball**; multi-hit animation; knockback |
| Earth Spike | Ground under **one** target rises into spikes |
| Heaven's Drive | Ground rises in **5×5**; multi-hit by level |
| Soul Strike | Ghost orbs travel — **live OK** |

Earth Spike / Heaven's Drive / Jupitel: wiki notes damage is often one bundle
despite multi-hit animation.

Note the two anchor styles differ. Napalm Beat and Soul Strike land on the
target entity; Earth Spike and Heaven's Drive rise out of the ground; Fire Ball,
Frost Diver, and Jupitel travel and then burst.

## Status-effect entity tints (landed + live-verified 2026-07-25)

Closes the "a poisoned mob looks untouched while losing HP" gap left open by the
E2 passes. Driven by the `opt1` / `opt2` flags the client already receives per
entity — note the field names are swapped relative to their contents:
`body_state` holds opt1, `health_state` holds opt2. `Common::status_tint()` in
`world/entity/mod.rs` is the whole table.

**A multiplicative tint is not sufficient, and this was the main finding.**
Multiplying a sprite by grey only ever *darkens* it — a petrified Poring read as
"standing in shadow", not as stone. Draining colour needs the sprite mixed
toward its own luminance, which is a per-pixel operation. So `StatusTint` carries
two knobs:

| Knob | Effect | Used by |
|---|---|---|
| `color` | component-wise multiply | freeze (cyan), poison (violet), stun (yellow) — genuine hue shifts |
| `desaturation` | mix toward Rec. 709 luminance, 0 = untouched | petrification (0.95), curse (0.6), blind (0.35) |

`desaturation` rides in the entity `InstanceData`'s existing `padding` slot, so
the instance buffer did not grow. The fragment shader drains hue **before** the
tint multiplies in — order matters, or the tint darkens a colour that is then
greyscaled. Both `entity.slang` and `entity_bindless.slang` carry the change;
**only the bindless path has been run** (macOS Metal), so the plain path shares
the same untested-on-GL/WSL caveat as the ground-decal pass.

**Petrification has two server phases and must render as two.** `SC_STONE`
starts as `OPT1_STONEWAIT` and Hercules deliberately lets the target keep
walking and attacking through it (`unit.c:1304` exempts it from the movement
block); only when the wait timer expires does `status.c:12456` call
`stop_walking` + `stop_attack` and flip to `OPT1_STONE`. Tinting both phases
identically produced a fully-grey mob wandering around. Now `STONEWAIT` gets a
faint 0.3 drain and `STONE` snaps to 0.95 — live-confirmed as reading correctly.
The animation-pause logic already made this distinction (`OPT1_STONE |
OPT1_FREEZE`, no `STONEWAIT`); the tint table was the odd one out.

**Sprite freeze extended to stun and sleep (2026-07-26).** The pause list was
honouring only half the server's rule: Hercules blocks movement for **every**
`opt1` state except `STONEWAIT` and `BURNING` (`unit.c:1304`), so a stunned or
sleeping mob was standing still server-side while its sprite kept bobbing
through its idle loop. Both call sites now go through
`status_freezes_animation`, and `STONEWAIT` remains excluded — pinned by a test,
because it is exactly the kind of exception that gets "tidied" into the list
later. Live-verified with a Hammerfall stun.

**Trap — a failed Stone Curse reports "Skill level is not high enough".** Same
overloaded `USESKILL_FAIL_LEVEL` family as the Demonstration NOFOOTSET trap
above, different emitter: `skill.c:8325` sends cause 0 when the petrify roll
misses, and the roll is `skill_lv*4+20` percent — **60% at level 10**, so ~4
casts in 10 report a level error on a maxed skill. `skill.c:8306` sends the same
cause 0 for an `MD_BOSS` target. Tell: a failed roll above level 5 does **not**
consume the Red Gemstone. The client now disambiguates the handful of cause-0
emitters verified to mean "resisted" (`skill_failed_text` in
`korangar-networking`); see that function's comment before adding more, since
most of the ~60 other cause-0 sites really are unmet conditions.

## Runtime RSM prop spawning — LANDED 2026-07-26 (was the last engine follow-up)

**The scoping below was wrong, and the correction is the useful part.** It
claimed models are baked into map vertex buffers at load, making this a new
pipeline. Direct inspection showed otherwise: map objects are **not** baked —
they render per frame via `Map::render_objects` → `Object::render_geometry`,
each emitting a `ModelInstruction` with its own model matrix. Only the *ground*
uses prebuilt sub-meshes. What is built once at load is the shared geometry
buffer; a `ModelInstruction` is just a draw range into it plus a transform.

So the real constraint was only: **a prop's geometry must be in that buffer.**
The map loader now loads the ten trap models into the same buffer and texture set
as the map's own objects, and a spawned trap is an ordinary `ModelInstruction`.
No new pass, no new pipeline, no buffer rebuild, correct lighting and depth for
free. Lesson: re-derive a "substantial" estimate from the code before pricing it.

**Live-verified:** model renders at the placed cell, Sandman applies sleep, prop
removed on both trigger and expiry.

**Shadow question RESOLVED 2026-07-26 — there was no bug.** The trap *does* cast
a shadow; it lands **directly beneath the model**, where the trap itself hides
it. That is the expected result for a small, near-flat prop sitting flush on the
ground under a near-overhead light. Lighting is correct too — the prop visibly
*receives* the player's shadow, which is what first showed the main pass and
shading were fine.

Worth keeping because the diagnosis was cheap and repeatable: scaling the prop
up 6× and logging the shadow-pass instruction count settled it in one pass
(`props=1 instructions_after_props=15`, i.e. the prop reaches the shadow pass and
produces draw instructions). Ruled out along the way: degenerate transform
(identity rotation, unit scale), shadow instruction wiring (props share the
object instruction buffer, batch count and partition camera), and missing baked
lighting (`ModelVertex` carries RSM colour and normals, lit by the same shader
path as any object).

**Trap caveat for future probes:** an `eprintln!` in the render loop fires every
frame and floods stderr hard enough to tank the frame rate — it looked like the
client had hung. Gate any render-loop logging behind a one-shot or a frame
counter.

DM prop placement can now reuse this path.

### Original scoping (kept for the record — the estimate it gave was wrong)

**Why it is the real fix, not a workaround.** All ten Hunter traps are RSM
*models*, not textures or sprites — the assets are present and identified
(`data\model\외부소품\트랩01.rsm`–`05`, `03_2`–`03_6`, see the batch-2 table
above). Every other persistent unit renders through a texture or sprite path, so
none of them apply here. Today a trap arrives from the server, misses
`MAPPED_UNIT_IDS`, and draws nothing at all: the placement animation plays, the
unit is live and functional server-side, and the ground stays empty.
Live-confirmed 2026-07-26 with Sandman.

**Why it is substantial.** Map models are baked into map vertex buffers at load
time. There is no path to introduce one after that, so this is not a recipe
entry — it needs:

1. A **runtime spawn path** for an RSM model outside the baked map geometry.
2. **Lifetime management** — traps are removed by `RemoveSkillUnit`, can expire,
   and can be destroyed; the prop has to follow that lifecycle.
3. **Non-disturbance** of the existing baked-map rendering, which is the part
   most likely to regress and the reason this was deferred rather than rushed.

**Why it is worth doing properly.** The same path serves **DM prop placement**,
which is a campaign feature rather than a fidelity one — so the work pays for
itself twice, and a throwaway trap-only hack would have to be rebuilt.

**Interim option, if traps are needed before the pipeline exists:** reuse the
depth-tested ground-decal drawer (`render_ground_decal`, shipped 2026-07-24 for
Land Protector) to put a flat marker quad on the trap's cell. Cheap and makes
traps visible and testable immediately, but it would be **our design, not the
original's look** — same standing as the Frost Diver spray below. Do not let an
interim marker close this item.

## Ground-skill aiming footprint — WIRED 2026-07-26, NOT YET LIVE-VERIFIED

While a ground-targeted skill is armed, the cursor now draws the **skill's real
area** on the ground instead of a single tile. Recovered the same way as
everything else here: from the authoritative source, not by eye.

**Where the shapes come from.** `world/skill_layout.rs` mirrors Hercules'
`skill_init_unit_layout` (`src/map/skill.c:21422`):

- **Square layouts** expand from `skill_db.conf`'s `Layout: N` to `(2N+1)²`,
  read out of the existing `docs/skills.json` export (the same field the hover
  tooltip already prints). Per-level tables are honoured — Land Protector is
  `[3,3,4,4,5,5,6,6,7,7]`, so its footprint grows 7×7 → 15×15 with level.
- **The fifteen `Layout: -1` skills** carry the cell lists hardcoded in that C
  function: Sanctuary (21), Magnus and Gospel (33), Grand Cross and Grand
  Darkness (29), Fog Wall (15), Kaensin (24), Wall of Thorns (16), Fire Mantle
  (8), Venom Dust (5), Tatamigaeshi (4/8/12 by level band).
- **Four wall-shaped skills are direction-dependent** — Fire Wall, Ice Wall,
  Earth Strain, Fire Rain. `facing_direction` ports `map_calc_dir`
  (`src/map/map.c:2943`) so the client picks the same one of eight orientations
  the server will.

**Rendering.** `Map::render_skill_footprint` stamps one decal per cell through
`render_ground_decal` — the depth-tested drawer built for Land Protector — using
each tile's own corner heights so the shape rides terrain, and skipping
unwalkable cells because the server will not place units there either. Driven
from `render_skill_aiming_footprint` in `lib.rs` off the armed `PendingSkill`.

**This also closes half of the "silent out-of-range ground cast" follow-up**: the
footprint tints red when `is_within_skill_range` fails. The cast is still *sent*
— there is still no client-side block on that path — but the player can now see
why it will fail.

**Two traps worth recording, both found by checking rather than reading:**

1. **Nine of eighteen skill ids taken from the Hercules constant names were
   wrong.** Land Protector is **288** (not 219), Earth Strain **2216** (not
   2019), Fire Mantle **8403**, Kaensin **535**, Gospel **369**. Always resolve
   ids through `docs/skills.json`. `every_custom_shape_id_is_a_custom_layout_in_the_export`
   now fails the build if any of the fifteen stops exporting `-1`.
2. **`MAX_SQUARE_LAYOUT` is 7 (15×15), not 5** — `skill.h:55`. An initial clamp
   at 5 would have drawn Land Protector at Lv9-10 as 11×11 while the server
   placed 15×15. It is the only skill in the db that exceeds 5, so this would
   only have surfaced at max level.

**Two skills Hercules groups into custom-shape branches are deliberately
excluded.** `NPC_EVILLAND` and `MH_POISON_MIST` sit beside Sanctuary and Venom
Dust in the C `switch`, but that whole branch only runs for skills whose layout
is `-1`; our `skill_db` gives Evil Land a real `Layout: 1` and Poison Mist no
layout at all, so both correctly take the generic path.

**Verification status.** Shapes were diffed mechanically against the C arrays
(all 12 static shapes match on cell set and count; all four directional families
match across all 8 directions) and the checks are pinned as tests — 10 in the
module, 242 in the lib, GRF audit green. **Nothing here has been seen on screen
yet.** Open questions for the live pass:

- Colour and alpha (`IN_RANGE` / `OUT_OF_RANGE` in `render_skill_aiming_footprint`)
  are guesses. The recurring note from batch 1 is that ground effects always want
  more brightness than they first get.
- **Does a large footprint read as a shape or as a solid slab?** Storm Gust is 81
  cells and Land Protector Lv10 is 225. This is the same failure mode batch 2 hit
  when 49 overlapping units rendered as one block; per-cell tiles may need a gap,
  or an edge-only treatment.

## Frost Diver travel spray — OUR DESIGN, not recovered fidelity

Flagged explicitly because everything else in this document is reverse-engineered
from the original. **Effect 27 carries no particle data.** The recovered table
entry is just `file: effect/ice`, `attachedEntity: false` — no count, no spread,
no rate. A dedicated `frostdiver.str` was probed for and does not exist in the
GRF (only `freeze.str` / `freezed.str`, which are the freeze *status*, effect
28). So the original really is a single travelling ice texture, and we were
already rendering exactly that.

It read as a generic blob, so the head now drags five shards (`trail_shards()` in
`skill_recipe.rs`) that lag 0.05→0.31 of the flight, spray across the travel
axis, and taper in size 0.78→0.34 and alpha 0.70→0.22. Only the head carries a
point light — six overlapping lights stack into one blown-out glare. Travel light
intensity also raised 36 → 58, the recurring "ground effects need to be brighter"
note from the E2 passes.

Anyone chasing fidelity later: **do not treat these numbers as authentic.** They
are tuned by eye. Same standing as the song/dance units, which roBrowser also
leaves as `Tofix`.

## The method — recovering any original-client visual (repeatable runbook)

How the E1/E2 mappings were derived. Use this for any other skill, status, or
unit visual we want authentic.

**1. Sources.** `MrAntares/roBrowserLegacy` on GitHub — a from-scratch
reimplementation whose DB tables reproduce the original exe's hardcoded
mappings (comments often cite the C++ client's own values):

| File | Gives you |
|---|---|
| `src/DB/Skills/SkillEffect.js` | skill ID → effect IDs per phase (`effectId`, `hitEffectId`, `beforeHitEffectId`, `groundEffectId`, `…OnCaster`) |
| `src/DB/Effects/EffectConst.js` | `EF_*` name → numeric effect ID |
| `src/DB/Effects/EffectTable.js` | effect ID → component list: `type` (STR / 3D / SPR / RSM / CYLINDER / QuadHorn / FUNC) + files, wavs, sizes, durations |
| `src/DB/Skills/SkillUnit.js` (+ `SkillUnitConst.js`) | persistent unit ID → effect ID |
| `src/DB/Monsters/AttackEffectTable.js` | monster attack visuals |
| `src/Renderer/EffectManager.js`, `src/Renderer/Effects/*.js` | how each `type` renders and which path prefix each field implies |

**2. Path conventions** (from EffectManager): `spriteName`/`SPR` →
`data\sprite\이팩트\<name>` · `STR` `file` → `data\texture\effect\<name>.str`
(`%d` + `rand: [a,b]` = random variant) · `RSM` → `data\model\<path>.rsm` ·
plain `file`/`textureName` → `data\texture\effect\…`.

**3. Decode Korean filenames from raw bytes.** The JS files hold CP949 bytes
that render as mojibake — `curl` the raw file and decode bytes as CP949
(`b'\xc0\xcc\xc6\xd1\xc6\xae'` = `이팩트`, `\xbf\xdc\xba\xce\xbc\xd2\xc7\xb0\x5c\xc6\xae\xb7\xa6` = `외부소품\트랩`).
Never trust the rendered text.

**4. Verify every file against our GRFs** with `tools/grf_list.py` (never
`get_files_with_extension`), then pin it in the
`all_mapped_skill_effect_assets_exist` audit so it can't rot.

**5. Cross-check the IDs three ways** before trusting them: roBrowser
`EffectConst` ↔ Hercules `doc/effect_list.md` ↔ our `ragnarok-packets`
`EffectId` discriminants; skill IDs against `Hercules/db/*/skill_db.conf`;
unit IDs against our `UnitId` enum. (All agreed for E1/E2.)

**6. License discipline** (CLAUDE.md §5): take *data* — which files, sizes,
timings — and write our own renderers. Do not port roBrowser code.

**7. Known pitfalls.** Entries marked `Tofix`/`Todo`/commented-out are
roBrowser's gaps, not the client's truth — cross-reference the GRF for the
real asset (that's how Napalm's unimplemented `EF_NAPALMBEAT` 32 was recovered
as `폭발1–8.tga`, the GRF's only 8-frame explosion cycle). Some `SkillEffect`
entries are roBrowser approximations rather than exe truth. And everything
needs a live pass: procedural translucency that looks right in theory washes
out against lit terrain (additive alphas want ~0.5–0.8), and authored STRs
beat procedural geometry when both exist (Safety Wall).

## Tooling

Both live in `tools/`, promoted from scratch work so they survive the session:

| Tool | Use |
|---|---|
| `tools/grf_list.py` | Dump a GRF file table (v2xx, CP949 names). **Use instead of `get_files_with_extension`.** |
| `tools/grf_extract.py` | Extract a subtree to a folder. Sizing works; see the encryption limit below. |

```sh
./tools/grf_list.py korangar/data.grf 'texture\effect'
./tools/grf_extract.py korangar/data.grf 'data\sprite\이팩트\' korangar/archive --dry-run
```

### Extraction is blocked on GRF decryption

`data.grf` entry flags break down as **1 (plain): 105,308 · 3 (ENCRYPT_MIXED):
34,228 · 5 (ENCRYPT_HEADER): 11,150 · 2: 244 · 0: 132**. So ~45k entries are
encrypted, and the classic effect folder is among the worst hit — a dry run over
`data\sprite\이팩트\` reports **293 of 427 entries skipped**.

The Python extractor does not implement GRF DES and skips those rather than
writing corrupt files. **Korangar already decrypts them** in
`src/loaders/archive/native/mixcrypt.rs` (`decrypt_file`, called from
`native/mod.rs:107`).

**So do not reimplement DES in Python.** Drive `GameFileLoader::get()` — which
decrypts transparently — from a small Rust binary or an `#[ignore]`d test, and
write the bytes out to `korangar/archive/`. That is the correct extraction path
for the bundling work.

## Assets are already local

`korangar/korangar/data.grf` (3.0 GB) and `rdata.grf` (292 MB) are **real local
copies**, not symlinks. `client/game_archives.ron` also references
`../../../RO/client/renewal2021.grf` and `resources2021.grf` by relative path —
those two must be copied in before the tree is self-contained for distribution.

Note: `data.grf` contents are Gravity's copyrighted assets. Bundling them is the
normal arrangement for a private friends server, but it is asset redistribution,
distinct from the repo's own "no upstream **code**" rule in CLAUDE.md §5.
