# Entity animation system

How Korangar plays actor (player/monster/NPC) animations, and how that maps
onto the original client's data and behavior. This is the compact actor-system
reference. The full packet-to-skill presentation contract is in
[`specs/combat-animation-pipeline.md`](specs/combat-animation-pipeline.md).

Native claims here target `../../RO/client/2019-06-05fRagexe_patched.exe`
(SHA-256 `61663a6f3bca42e992e3d61418b9508db57e6ba18cc0069e297eb3730d4d825d`).
Claims derived only from GRF data, Korangar, or a third-party implementation
are identified as such. Last native research pass: 2026-07-17.

## 1. Building blocks: SPR + ACT pairs, layered

Every visible actor is a stack of sprite layers. Each layer is one SPR
(images) + ACT (choreography) pair:

| Layer | Path pattern (players) |
|---|---|
| Body | `인간족\몸통\{sex}\{job}_{sex}` |
| Head | `인간족\머리통\{sex}\{hair}_{sex}` |
| Weapon | `인간족\{weapon folder}\{folder}_{sex}_{weapon name}` |
| Shield | Ragexe `0x7C46C0`: job token is compound `job\\job`. Class views 1–4 → `방패\{job}\{job}_{sex}_{가드\|버클러\|쉴드\|미러쉴드}`; view ≥ 5 prefers `방패\{job}\{job}_{sex}_{view}_방패` when the special-job check at `0x9A2430` allows (we always probe). Not under `인간족\`. |

`Common::get_entity_part_files` (entity/mod.rs) assembles this list. Korangar
keeps per-layer SPR/ACT resources in `AnimationData.layers` and composes them
at render time (`compose_frame`): the body owns the clock; secondary layers
use `CActRes::get_motion` motion-0 fallback (`native_layer_motion_index`).
Action-global billboard AABBs are measured once at load (`action_layouts`) so
proportions stay stable across a motion. Head (and any secondary) attach
points are applied at compose: `offset += -child_attach + body_attach` using
the body motion's attach and the secondary layer's selected motion attach.
**Shield draw order** is per camera-relative facing: dirs `2..=5` paint the
shield before the body (back half); otherwise after the weapon (front).
**Partial swaps** (`with_weapon_layer` / `with_head_layer` / `with_shield_layer`)
reload one layer without re-fetching body/head delays. Weapon vs shield paths
are identified by content (`인간족\{job}\…` vs `방패\…`), not fixed indices.
Monsters/NPCs are generally a single pair (`몬스터\{name}`, `npc\{name}`),
but their action layouts still need to be audited by actor family.

**Phase C (runtime layer composition) closed 2026-07-18** — see
[plans/animation-fidelity.md](plans/animation-fidelity.md) §4.
**Phase D (weapon visual completeness) DONE 2026-07-22** — per-item weapons,
dual-wield combine, `_검광` trails; live GUI signed off (all 8 rows PASS
2026-07-21). Note the idle draw rule this surfaced: a weapon/shield `.act`
**blanks its Idle/Walk frames** (sprite `-1`) — those are the *unarmed* stand —
so an armed player must stand in the **ReadyFight** action (group 4) to render
gear, relaxing to Idle only on town/safe maps. Remaining player-visual work is
hat/accessories. Next engine work is Phase E.

The **weapon layer** rules (all verified against the GRFs by
`weapon-sprite-audit`):

- Hercules `PACKETVER ≥ 4` puts the **raw item nameid** on LOOK_WEAPON /
  LOOK_SHIELD (`clif_get_weapon_view`). Class views still appear on older
  paths and for expansion IDs after `GetRealWeaponId`.
- Appearance → class view: `weapon_view_from_appearance` (class +
  `native_real_weapon_id`, or `weapon_view_from_item_id` for item IDs).
  Assassin dual-wield left+right combine via `effective_weapon_view` into
  views 25..=30 before attack selection.
- **Native path builders** (Ragexe `0x007C4F90` weapon, `0x007C4B30` trail;
  act/spr wrappers `0x009B8A30` / `0x009B8B80` and trail wrappers that pass
  trail index 0/1). Formats under the job folder:
  - weapon: `\%s_%s%s.%s` → `{job}_{sex}{suffix}.spr` (suffix from class
    name table or Lua `ReqWeaponNameByClassNum`)
  - trail: `\%s_%s%s%s.%s` → `{job}_{sex}{suffix}_검광.spr` (or `_발광`)
- Sprite probe order (`push_weapon_part_file`): exact per-item path
  (`{folder}_{sex}_{itemId}`) → dual class pair when off-hand is a weapon →
  single class suffix (`weapon_resource_suffix`) → none. Never the
  placeholder. Dual off-hand becomes a second weapon layer when the combined
  pair was not used.
- **`_검광` trails** follow the native switch at `0x00976590` /
  table `0x00976EC0`: class views 1–7, 16–18, 25–30 only. Per-item bases
  still probe `{base}_검광` when that SPR exists (e.g. Mjolnir 1530).
- Class suffixes: 단검/검/양손검/창/양손창/도끼/양손도끼/클럽/활/너클/악기/
  채찍/책/카타르_카타르/guns/수리검, plus dual pairs 단검_단검…검_도끼
  (views 25..=30). Two-handed weapons have their **own** sprites.
- Classic rods/staves (views 10/23) ship no generic class weapon layer in the
  configured archives; per-item rods still load when the numbered SPR exists.
- The weapon *folder* is usually the body-sprite folder. The archive probes
  show Priest files under `프리스트`, Royal Guard files under `로얄가드`, and
  no class-named weapon layer for the transcendent second classes (plus Shadow
  Chaser). Korangar currently resolves the latter to base-class folders
  (어쌔신크로스→어세신, 스토커→로그, 로드나이트→기사, ...); native lookup
  confirmation is still required.
- A layer is only requested if the exact `.spr` exists — a missing combination
  must render no weapon, not the fallback sprite.

## 2. ACT structure

An ACT file contains:

- **Actions**: a flat list; every 8 consecutive actions are the same motion in
  the 8 camera directions. So *action group* = `action_index / 8`.
- **Motions** (frames) per action: each has sprite clips (which SPR images,
  offsets, zoom, mirror, color), optional attach points (head↔body
  alignment), and an optional **event id**.
- **Events**: a list of names — `"atk"` (generic attack impact) or a `.wav`
  filename (played spatially when the frame shows).
- **Delays**: per action, the natural per-frame duration factor.

Player action groups (`AnimationActionType::action_base_offset`):

| Group | Action | Group | Action |
|---|---|---|---|
| 0 | Idle | 7 | Freeze1 |
| 1 | Walk | 8 | Die |
| 2 | Sit | 9 | Freeze2 |
| 3 | Pickup | 10 | Attack2 |
| 4 | ReadyFight | 11 | Attack3 |
| 5 | Attack1 | 12 | Skill |
| 6 | Hurt | | |

Monsters/NPCs: 0 Idle, 1 Walk, 2 Attack, 3 Hurt, 4 Die — no ReadyFight, no
extra attack actions.

Example GRF data (Knight male body, from
`reports_knight_attack_frame_structure`):
Attack1 has 5 frames, Attack2 has 9 (sound event on frame 4 →
`attack_twohand_sword.wav`), Attack3 has 8 (sound event on frame 5 →
`attack_spear.wav`), Hurt has 3 (frame 1 → `player_metal.wav`). The weapon
and head ACTs happen to mirror this body's frame counts. That is not a global
rule: the full archive audit finds many player layer mismatches.

## 3. Which attack action plays

The recovered classic sprite-player state function (`CPc::SetState`,
`0x009779A0`) sends both normal
and critical basic attacks through the same job/sex/weapon selector at
`0x009A2DB0`. False selects flat Attack2 (`0x50`); true selects Attack3
(`0x58`). Critical damage type does not itself select Attack3.

Do not substitute the adjacent `CGrannyPc` function at `0x00977430` for this
table. Korangar's actors use SPR/ACT (`CPc`), not Granny-model playback. The
classic path also overrides state-2 selection for jobs `4046`, `4047`, `4048`,
`4225`, `4226`, `4238`, `4239`, `4241`, `4243`, and `4244`: 70% of requests
select Attack3 and 30% select Attack2. State 9 still uses the ordinary exact
job/sex/weapon selector.

The selector normalizes expansion weapon appearances with Lua
`GetRealWeaponId`. Its exact family predicates and job-ID rows are now mirrored
by `native_player_attack_action` and unit-tested. Raw item-to-view lookup and
Assassin left/right-hand combination are modeled by
`weapon_view_from_appearance` / `effective_weapon_view` (Phase D).

`0x00991580` returns the attack-event position, not a playback-speed factor.
It is `5.85` for male Merchant Attack2, `5.75` for Thief Attack2, `3.0` for
Assassin-family dual-wield Attack3, `5.85` for male Novice-family Attack3, and
`6.0` otherwise. Native impact scheduling multiplies this position by the
current ACT delay and 24 ms. Skill damage has a separate skill-ID → actor-state
lookup; see sections 7.8 and 18 of the pipeline specification.

## 4. Timing: how frames advance

The native core actor update is at `0x008AC2E0`:

```text
act_time = elapsed_milliseconds / 24.0
raw_motion = floor(act_time / actor_delay)
```

`actor_delay` starts from the body ACT action delay and can be changed by
actor timing factors. Playback mode 0 wraps `raw_motion` by the **body** motion
count. One-shot mode 1 clamps to the body's last motion after one cycle.
Missing delay entries default to `4.0` (`CActRes` accessor `0x004F5280`).

Higher actor states can install a second clock above this ACT clock. The
helpers at `0x008BA040` and `0x008B9E90` define a motion range or five exact
`(action, motion)` pairs, a cadence measured in **actor update calls**, a
wall-clock deadline, and hold/cycle behavior. The chosen motion bypasses the
normal ACT-derived motion until that program ends. Korangar implements every
program used by generic dispatcher states 16..51 plus the classic-player
state-7 programs in `animation/native_motion.rs`. The exact table is section
7.9 of the pipeline specification.

Korangar uses `delay × 24 ms` for unmodified ACT playback and defaults missing
delays to 4.0. Native source attacks also use this natural ACT clock; packet
`sMotion` does not stretch their frames. Korangar now follows that source rule.

Target reaction timing is different. At the scheduled impact, native code
computes `reaction_cycles = dMotion / 288.0`. Above zero and at or below one
cycle, it speeds the Hurt clock by changing the effective delay: player-job
actors use `ACT_delay × reaction_cycles`, while non-player actors use
`reaction_cycles` directly. `0x009A3400` defines player-job as raw job
`0..=30` or `4001..=5999`. Above one cycle it plays at natural ACT speed, then
holds the last one-shot motion until that cycle threshold. The native setter
rejects a zero delay. Korangar now follows this reaction clock once Hurt
begins, and begins it at the queued ACT-derived impact boundary rather than at
packet receipt.

For mismatched layers, the native client sends the same body-owned action and
motion index to each separate ACT resource. `CActRes::get_motion`
(`0x004F5360`) returns motion 0 when a non-empty secondary action is too short.
Consequently a short layer falls back to its first motion; it does not hold its
last motion or wrap independently. Extra motions in a longer secondary layer
are unreachable. Empty motions stay in the body timeline even if they draw no
clips. Korangar's pre-merge now emulates those index rules, but retaining
layers through rendering is still needed for complete fidelity.

STR/effect advancement is currently capped at 1/15 s per application frame to
avoid a synchronous first-load stall skipping an effect. That is a Korangar
workaround, not a recovered native timing rule.

## 5. Current combat presentation loop

For a damage packet (`ZC_NOTIFY_ACT` 0x08C8 / skill damage 0x01DE):

1. **Attacker**: Korangar rotates toward the target and plays its selected
   source action on natural ACT timing. Basic attacks use the recovered native
   weapon selector. Every skill resolves the exhaustive `u16` skill catalog,
   classic-player intercept, shared flat action, and any higher motion program
   before impact timing is derived. Completion follows the recovered higher
   state, not the semantic appearance of the ACT group: only states
   `2/4/5/7/9/12/45` request state zero, while classic-player state 51 first
   becomes state 2. All other completed states hold for an explicit event.
2. **Impact/target**: Korangar queues a due target phase at
   `round(actor[0x24C] × ACT_delay × 24 ms)` (plus actor-specific projectile
   constants), then applies numbers or miss, hit effects/sounds, and Hurt
   together. Hurt uses the `dMotion / 288.0` rule above. Caster and travel
   tracks launch immediately with the source action. Non-death entity removal,
   map change, and disconnect cancel affected queued work; death keeps the
   number/effect at the actor's last position but suppresses Hurt. Existing
   per-skill effect, projectile, and audio branches are typed recipes; the
   unmapped catalog, cadence variants, and cleanup branches remain recipe
   work. None changes the resolved source actor-state clock.
3. **Sound events**: the native scanner (`0x008AC860`) examines the body ACT
   and walks every body motion crossed since the prior update, including loop
   wrap. Korangar matches that with `FrameEventCursor` on `AnimationState`
   (`AnimationData::take_crossed_events` / `collect_crossed_events`):
   - cursor identity is the playback start tick + flat action base offset;
   - one-shot actions clamp at the final motion so a held last frame produces
     no new crossings (image only; sound does not re-fire);
   - looping actions continue the raw motion index so wrap re-fires cycle
     events;
   - a slow application frame that jumps several motions delivers every
     authored event on the crossed frames;
   - stall recovery is deliberately bounded to one full cycle so an extreme
     hitch cannot flood the audio mixer (native walks unbounded);
   - higher motion programs fire once per `step_serial` occurrence (duplicate
     authored motions fire again; a terminal held step does not).
   Native `.wav` events are body-owned; `"atk"` is searched separately at
   `0x008A9500` for impact scheduling. When a weapon-swing body ACT has an
   empty event table (male Assassin / Assassin Cross in classic GRFs), the
   cursor emits one synthetic `ActionEvent::Attack` at the native attack-
   event marker so `weapon_sound` still plays; authored events suppress it.

Request-time actor input is now packet/status owned:

- actor `+0x16C` is `SI_TRICKDEAD` (status 29). Gain installs a held
  death-looking pose but is not real death; both it and actual death reject
  later movement/action requests;
- classic-player `+0x114` is `SI_SUHIDE` (933). It displays native state 48
  and rejects only native state-7 skill actions. `SI_SU_STOOP` (893) displays
  state 47 underneath it;
- actor `+0x2C0` is packet `isPKModeON`. It is preserved on spawn/state change
  and selects ReadyFight when a classic player returns to logical neutral;
- `ZC_STATE_CHANGE` option/body/health/PK fields are applied atomically.
  Stone/freeze opt1 pauses the ACT clock without stealing movement, casting,
  or future action ownership;
- movement and Hurt no longer cancel one another, and movement can coexist
  with a cast (Free Cast). Explicit `ZC_DISPEL`, failed skill acknowledgement,
  skill execution, expiry, Trick Dead, or death ends the cast. A terminal skill
  result clears the cast before its source actor pose is request-guarded.

All previously supported skill effects, projectiles, and sounds now resolve
through `world/skill_recipe.rs`. Each skill independently declares
successful-caster, damage-caster, launch/projectile, scheduled target/hit, and
ground phases. Missing effect tracks do not suppress audio/projectiles, random
variants are enumerable for asset audits, and an unmapped ID returns an
explicit empty presentation contract while still using the exhaustive native
skill-to-actor-state resolver. The current mapped IDs are cataloged in section
8.1 of the full pipeline specification; this registry boundary does not imply
complete RO skill coverage.

## 6. Diagnostics and tests

### 6.1 How to run (fast suite)

From the repo root (`korangar/`):

```bash
# Full client lib unit suite
cargo test -p korangar --lib

# Phase C composition + gear (recommended smoke after animation edits)
cargo test -p korangar --lib runtime_compose_tests
cargo test -p korangar --lib classic_shield
cargo test -p korangar --lib native_shield_paths
cargo test -p korangar --lib weapon_layer_tests
cargo test -p korangar --lib frame_event_cursor_tests
```

GRF-backed diagnostics (need official client archives configured):

```bash
cd korangar
cargo test -p korangar --lib probes_original_client_shield_paths -- --ignored --nocapture
cargo test -p korangar --lib loads_classic_weapon_layers_for_roster -- --ignored --nocapture
cargo run --release --bin weapon-sprite-audit
cargo run --release --bin animation-audit
```

Live GUI (manual, Phase D — DONE 2026-07-21, all 8 rows PASS): provision with
headless `provision-effect-roster` (Knight gets sword/spear + Guard/Shield
items), then check EffectKnight sword+Guard idle/walk/attack and facing-away
body covering the Guard. Full results + method in
[plans/phase-d-live-verification.md](plans/phase-d-live-verification.md).

### 6.2 Catalog

| Tool / test | What it does |
|---|---|
| `cargo run --release --bin weapon-sprite-audit` (from `korangar/korangar/`) | Probes every job folder × sex × candidate weapon name against the GRFs; also dumps the archives' own 인간족 listing. |
| `cargo run --release --bin animation-audit` (from `korangar/korangar/`) | Parses every player job's body/head/weapon ACTs; reports per-direction frame-count mismatches and body attack events. It inventories data; it does not prove runtime selection. |
| `PYTHONPATH=/tmp/ro_binary_tools python3 tools/native_skill_state_audit.py` (from `korangar/korangar/`) | Hash-checks the reference Ragexe and exhaustively emulates `0x009B4B30` over the full `u16` skill-ID domain. This regenerates pipeline spec section 18's source actor-state catalog. Requires `pefile` and `capstone`. |
| `native_motion::tests` (fast suite) | Pin actor-update cadence, wall deadlines, duplicate-motion occurrences, ping-pong/cycle order, cross-action steps, random branches, and job-specific programs. |
| `frame_event_cursor_tests` (fast suite, Phase B) | Pin the crossed-motion event cursor: multi-motion jump, loop wrap, held final frame, one-cycle stall bound, motion-program `step_serial`, and playback-identity reset. |
| **`runtime_compose_tests` (fast suite, Phase C)** | See §6.3. |
| `weapon_action_tests`, `weapon_layer_tests` (fast suite) | Pin the job×weapon action table, suffix/folder mappings, and Ragexe shield path candidates (`native_shield_paths_match_ragexe_sprintf_forms`). |
| `state::inventory::tests::classic_shield_item_ids_map_to_view_sprites` | Guard/Buckler/Shield/Mirror item IDs → views 1–4; non-shield IDs return `None`. |
| `reports_knight_attack_frame_structure` (ignored, `--nocapture`) | Parses real body/weapon/head ACTs; prints per-action frame counts, delays, event placement. Point it at other jobs when extending. |
| `loads_classic_weapon_layers_for_roster` (ignored) | Asserts the exact weapon SPR/ACT paths for the effect roster exist. |
| `probes_original_client_shield_paths` / `probes_shield_sprite_paths` (ignored) | GRF existence probes for nested `방패\{job}\{job}_{sex}_{suffix}` forms. |
| `all_mapped_skill_effect_assets_exist` (ignored) | Walks every typed skill recipe and every declared random variant, then asserts each referenced STR/texture/wav ships. |
| `probes_classic_skill_sound_candidates` (ignored, `--nocapture`) | Existence probes for classic sound names. |

### 6.3 Phase C unit tests (`runtime_compose_tests`)

| Test | Pins |
|---|---|
| `compose_uses_body_motion_count_and_motion_zero_fallback` | Body owns clock; secondary `get_motion` falls back to motion 0 (C1). |
| `child_attach_aligns_to_body_attach_point` | `offset += -child + body` attach delta (C2). |
| `attach_uses_body_motion_when_secondary_falls_back_to_motion_zero` | Attach uses body motion's point when secondary is on motion 0 (C2). |
| `weapon_layer_swap_preserves_body_and_head_paths` | Partial weapon swap does not reload body/head (C4). |
| `shield_layer_swap_preserves_weapon` | Shield equip/unequip keeps weapon; weapon unequip keeps shield (C5). |
| `shield_draw_order_follows_view_direction` | Dirs 2–5: shield before body; dirs 0,1,6,7: shield last (C3). |
| `weapon_path_from_parts_skips_shield_at_index_two` | Weapon path is never `방패\…` when shield sits at parts[2]. |
| `weapon_swap_with_shield_path_arg_does_not_clobber_sword` | Mis-fed shield path must not replace sword; dual-equip restore (C5 closeout). |
| `weapon_layers_preserve_trail_and_offhand` | Multi weapon-family swap keeps shield; trails drop with base (D). |
| `item_ids_map_to_classic_weapon_views` / dual-combine tests | Item→view + Assassin L/R → 25..=30 (D). |
| `per_item_candidates_precede_class_suffix` | Path order: item id before class (D). |

**Live checklist (Phase C closed 2026-07-18):** EffectSinX dual-wield + head
attach; EffectKnight spear/sword attack hold + sound; sword+Guard idle/attack;
Guard covered by body when facing away.

**Live checklist (Phase D — DONE 2026-07-21, all 8 rows PASS):** full results in
[plans/phase-d-live-verification.md](plans/phase-d-live-verification.md)
(Mjolnir 1530, dual daggers, trail allowlist, C4/C5 regressions, bow/mace). **Next
task for Claude/Codex/any agent is Phase E** (skill/status recipes,
[plans/animation-fidelity.md](plans/animation-fidelity.md) §6).

**Important**: `GameFileLoader::get_files_with_extension` under-reports the
classic GRF (verified: the 기사 folder listed 3 files, probes found 16).
Discover assets by probing candidate paths with `file_exists` (lowercase the
path — `file_exists` does not normalize case the way `get` does).

## 7. Known gaps / open questions

The execution plan for closing these is
[plans/animation-fidelity.md](plans/animation-fidelity.md).

- ~~**Phase D live GUI**~~ → **DONE 2026-07-21**, all 8 rows PASS —
  **[plans/phase-d-live-verification.md](plans/phase-d-live-verification.md)**.
- ~~**Per-item weapon sprites** / **`_검광` trails** / **item→view + dual
  combine**~~ — Phase D code closed 2026-07-18, live-verified 2026-07-21.
- **Impact scheduler completeness**: the ACT-derived pending-impact boundary
  and `dMotion / 288.0` target clock are implemented. Add exact damage-type
  reaction guards, per-hit cadence, packet-fixture/golden-timeline coverage,
  and remaining skill-specific timing branches.
- **State/status presentation depth**: request-time ownership and precedence
  for Trick Dead, SU Hide/SU Stoop, PK neutral, movement/Hurt, casting, and
  stone/freeze opt1 holds are implemented. The remaining body/health/option
  values still need their native tint, attached effect, pose, sound, refresh,
  and removal recipes; damage-type-specific reaction guards also remain.
- **Player cosmetics / hat stack**: body+head+weapon+shield compose is closed
  (Phase C). Bottom/mid/top accessories and mounts are not layered yet.
- **Special motion-0 attach edge cases**: generic attach is C2; any rare
  native specials beyond body-clock + motion-0 fallback remain unproven.
- Several classic skill sounds were not found under any probed name — list in
  the 2026-07-17 session notes.
- Freeze1/Freeze2 action groups are not selected as generic status poses;
  stone/freeze currently preserve the recovered body-state ACT clock hold.
  Remaining per-status visual branches and most skill presentation recipes are
  not yet complete.
- Complete skill, persistent-unit, status, and monster presentation coverage
  is specified and tracked in
  [`specs/combat-animation-pipeline.md`](specs/combat-animation-pipeline.md).
