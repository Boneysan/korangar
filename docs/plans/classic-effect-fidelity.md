# Classic effect fidelity — bringing authentic RO skill effects to Korangar

| | |
|---|---|
| **Status** | Backend landed 2026-07-22 — skill mapping + asset extraction next |
| **Branch** | `agent/platform-connectivity-controls` |
| **Parent** | [animation-fidelity.md](animation-fidelity.md) §6 Phase E |
| **Trigger** | Phase E1 skills work but "don't look right" — particles travel, but they don't read as RO spells |

## The core finding

RO effects come in **two distinct families**, and Korangar only supports one of them.

| Family | Location | Used by | Korangar support |
|---|---|---|---|
| **STR scripts** | `data\texture\effect\*.str` | Ground / AoE spells | ✅ Full — `EffectLoader`, `EffectAsset::Fixed` |
| **Sprite effects** | `data\sprite\이팩트\*.spr` + `.act` | Classic single-target spells | ❌ **None in the effect path** |

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

Current procedural E1 recipes to be replaced (`world/skill_recipe.rs`):

| Skill | Current stand-in |
|---|---|
| 11 Napalm Beat | `DamageTargetEffect::NapalmBeat` + `ring_blue.tga` |
| 13 Soul Strike | `ProjectileRecipe::SoulStrikeOrbs` + `pok1.tga` |
| 15 Frost Diver | `TravelBall(FrostDiver)` + `ice.tga` |
| 17 Fire Ball | `TravelBall(FireBall)` + `fire_blast.bmp` |
| 84 Jupitel Thunder | `TravelBall(Jupitel)` + `번개4.bmp` |
| 90 Earth Spike | `DamageTargetEffect::EarthSpike` |
| 91 Heaven's Drive | `DamageTargetEffect::HeavensDrive` |

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

### Suggested first move

Prove the path end-to-end on **one** skill before mapping the other six:
point skill 13 (Soul Strike) at `이팩트\soule` — the one mapping backed by an
actual filename match — and check that a real sprite animation plays at the
target. Anchoring is the likely first bug: the emote work found the generic
animation loader normalizes every ACT frame to its bottom edge, which drew
emotes *under* the character until the anchor was derived from the composed
frame height instead.

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
