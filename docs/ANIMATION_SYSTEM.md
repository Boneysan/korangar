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

`Common::get_entity_part_files` (entity/mod.rs) assembles this list. Korangar's
`AnimationLoader` currently pre-merges the pairs into one `AnimationData`.
The native client retains separate resources and composes them at render time;
runtime composition is therefore the long-term fidelity architecture.
Monsters/NPCs are generally a single pair (`몬스터\{name}`, `npc\{name}`),
but their action layouts still need to be audited by actor family.

The **weapon layer** rules (all verified against the GRFs by
`weapon-sprite-audit`):

- The weapon *name* comes from the appearance class (`weapon_resource_suffix`):
  단검/검/양손검/창/양손창/도끼/양손도끼/클럽/활/너클/악기/채찍/책/카타르_카타르/
  guns/수리검, plus the dual-wield pairs 단검_단검, 검_검, 도끼_도끼, 단검_검,
  단검_도끼, 검_도끼 (views 25..=30). Two-handed weapons have their **own**
  sprites — never reuse the one-handed name.
- Classic rods/staves (views 10/23) ship no generic class weapon layer in the
  configured archives, so Korangar adds no generic layer for them. Per-item
  models remain a separate unresolved path.
- The weapon *folder* is usually the body-sprite folder. The archive probes
  show Priest files under `프리스트`, Royal Guard files under `로얄가드`, and
  no class-named weapon layer for the transcendent second classes (plus Shadow
  Chaser). Korangar currently resolves the latter to base-class folders
  (어쌔신크로스→어세신, 스토커→로그, 로드나이트→기사, ...); native lookup
  confirmation is still required.
- A layer is only requested if the exact `.spr` exists (`push_weapon_part_file`)
  — a missing combination must render no weapon, not the fallback sprite.
- The GRFs also ship per-item weapon sprites (numbered, e.g. `기사_남_1530`)
  and `_검광` "sword glow" trail variants. **Not currently rendered** — see
  Known gaps.

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
Assassin left/right-hand combination still occur upstream and need their own
normalization path.

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
   wrap. Korangar's `SoundToken` prevents repeated playback while one frame is
   displayed, but it can miss an event when a slow frame crosses multiple ACT
   motions. It must be replaced by a crossed-motion cursor. Native `.wav`
   events are body-owned; `"atk"` is searched separately at `0x008A9500`.

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

| Tool / test | What it does |
|---|---|
| `cargo run --release --bin weapon-sprite-audit` (from `korangar/korangar/`) | Probes every job folder × sex × candidate weapon name against the GRFs; also dumps the archives' own 인간족 listing. |
| `cargo run --release --bin animation-audit` (from `korangar/korangar/`) | Parses every player job's body/head/weapon ACTs; reports per-direction frame-count mismatches and body attack events. It inventories data; it does not prove runtime selection. |
| `PYTHONPATH=/tmp/ro_binary_tools python3 tools/native_skill_state_audit.py` (from `korangar/korangar/`) | Hash-checks the reference Ragexe and exhaustively emulates `0x009B4B30` over the full `u16` skill-ID domain. This regenerates pipeline spec section 18's source actor-state catalog. Requires `pefile` and `capstone`. |
| `native_motion::tests` (fast suite) | Pin actor-update cadence, wall deadlines, duplicate-motion occurrences, ping-pong/cycle order, cross-action steps, random branches, and job-specific programs. |
| `reports_knight_attack_frame_structure` (ignored, `--nocapture`) | Parses real body/weapon/head ACTs; prints per-action frame counts, delays, event placement. Point it at other jobs when extending. |
| `loads_classic_weapon_layers_for_roster` (ignored) | Asserts the exact weapon SPR/ACT paths for the effect roster exist. |
| `all_mapped_skill_effect_assets_exist` (ignored) | Walks every typed skill recipe and every declared random variant, then asserts each referenced STR/texture/wav ships. |
| `probes_classic_skill_sound_candidates` (ignored, `--nocapture`) | Existence probes for classic sound names. |
| `weapon_action_tests`, `weapon_layer_tests` (fast suite) | Pin the job×weapon action table and the suffix/folder mappings. |

**Important**: `GameFileLoader::get_files_with_extension` under-reports the
classic GRF (verified: the 기사 folder listed 3 files, probes found 16).
Discover assets by probing candidate paths with `file_exists` (lowercase the
path — `file_exists` does not normalize case the way `get` does).

## 7. Known gaps / open questions

The execution plan for closing these is
[plans/animation-fidelity.md](plans/animation-fidelity.md).

- **Per-item weapon sprites** (`기사_남_1530.spr` etc.) and **`_검광` sword
  trail layers** exist in the GRFs but are not rendered. The trail is a
  visible part of classic attack presentation for many weapons.
- **Attack selector inputs**: raw item-to-view resolution and Assassin
  left/right-hand combination precede the recovered `0x009A2DB0` matrix and
  are not modeled yet.
- **Impact scheduler completeness**: the ACT-derived pending-impact boundary
  and `dMotion / 288.0` target clock are implemented. Add exact damage-type
  reaction guards, per-hit cadence, packet-fixture/golden-timeline coverage,
  and remaining skill-specific timing branches.
- **State/status presentation depth**: request-time ownership and precedence
  for Trick Dead, SU Hide/SU Stoop, PK neutral, movement/Hurt, casting, and
  stone/freeze opt1 holds are implemented. The remaining body/health/option
  values still need their native tint, attached effect, pose, sound, refresh,
  and removal recipes; damage-type-specific reaction guards also remain.
- **Crossed frame events**: replace displayed-frame sound polling with the
  body event cursor used by the native client.
- **Runtime layer composition and attach rules**: stop permanently flattening
  player parts; recover special motion-0 attachment cases, layer ordering,
  shields, cosmetics, and dynamic swaps.
- Several classic skill sounds were not found under any probed name — list in
  the 2026-07-17 session notes.
- Freeze1/Freeze2 action groups are not selected as generic status poses;
  stone/freeze currently preserve the recovered body-state ACT clock hold.
  Remaining per-status visual branches and most skill presentation recipes are
  not yet complete.
- Complete skill, persistent-unit, status, and monster presentation coverage
  is specified and tracked in
  [`specs/combat-animation-pipeline.md`](specs/combat-animation-pipeline.md).
