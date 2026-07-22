# Classic effect fidelity — bringing authentic RO skill effects to Korangar

| | |
|---|---|
| **Status** | 2026-07-23: **all seven skills rebuilt from the reverse-engineered original-client effect table** (see "The authoritative mapping" below). Code green (194 lib tests + GRF asset audit); needs a live GUI pass. |
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

- **Cast bar for menu/priest skills** — packets verified healthy via the new
  `[skill-cast]` diagnostic (`KORANGAR_PACKET_LOG`): warp 1000 ms, Magnus
  2005 ms all arrive as `SkillCast` events. Whether the bar *renders* during
  those casts is still unconfirmed live.
- Warp-selection window close-without-Cancel leaves server menuskill state.
- Batch 2 units: Hunter traps (`ef_trap_*` string keys in the effect table),
  Sage ground fields (Volcano/Deluge/Violent Gale/Land Protector →
  `EF_BOTTOM_*`), Venom Dust (`particle3` sprite loop needs persistent
  sprite-effect support), song/dance areas.

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
