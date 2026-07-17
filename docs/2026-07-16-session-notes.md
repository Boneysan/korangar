# Session notes — 2026-07-15/16 (E3.1 GUI pass)

The M1 P0 checklist driven by hand against a live server, start to finish. It
took the pass from 17 verified rows to **32 of 34**, fixed **6 client bugs**,
and filed **6 more** with root causes.

Every fix had the same shape: **the packets were already correct and
headless-green; the client's handling of them was wrong.** That is the blind
spot the 106-scenario suite has by construction, and this session turned it from
an argument into six data points.

## Bugs fixed (all live-verified the same session)

| ID | Symptom | Root cause |
|---|---|---|
| M1-005 | Dialogue box read as empty | `window!` for Dialog declared no size floor, so it collapsed — **and the collapsed size persisted to `window_cache.ron`**, so it reopened broken every session |
| M1-007 | No hide/cloak visuals at all | `StateChangePacket` (`0x0229`) was `register_noop`; `OPTION_HIDE` never reached the UI. Knock-on: hide-gated skills (`RG_RAID`) looked broken with no way to diagnose |
| M1-010 | Buff bar showed `10:218s` | No index → name mapping. Now resolves from Hercules' own `db/constants.conf` (699 names) |
| M1-011 | "Skill Delay" stuck forever | `apply()` treated `duration_ms == 0` as *infinite*. Zero means **already over**; Hercules signals permanent with `INFINITE_DURATION = -1` → `u32::MAX`, already handled |
| M1-012 | Cancelled buffs never cleared | Statuses **start** on `0x0983` but **end** on `0x0196` — and `0x0196` was `register_noop` |
| M1-013 | Character list empty at 3 characters | Hercules sends a second, **empty** `0x0B72` at exactly 3 chars as an end-of-pagination marker; `set_characters` cleared all slots before re-adding, so the terminator wiped the list |

## Bugs filed (open)

| ID | Pri | Note |
|---|---|---|
| M1-006 | P0 | No skill-targeting mode — hover-then-press is the only way to aim. Needs a design call (cancel semantics), not just code |
| M1-008 | P1 | Partial Wizard slice: retained skill IDs prove packet routing and Storm Gust renders, but Thunderstorm/modern spell STRs expose missing renderer support (visible light, no geometry). Broader `DisplaySpecialEffectPacket`/ground-skill coverage also remains open; mind the No-Upstream-IP rule |
| M1-009 | P1 | No gear stats or comparison. **Data is already in the binary** (`items.json` has Atk/Def/Slots/EquipLv; `DmItem` embeds it) — but it lives under `src/dm/`, which rule 4 isolates for rebaseability |
| M1-010 | P1 | Kept open for the **icon** half (needs artwork); names are done |
| M1-014 | P2 | Character delete is right-click-only *and* unconfirmed — fixing discoverability alone would make accidental deletion easier |
| M1-015 | P2 | Stuck at server select after a failed login. **Observed once, not reproduced** — root cause is a lead, not a finding |

## Why headless missed all of it

Worth internalising, because it is structural rather than an oversight:

- **Noop packets emit no `NetworkEvent`**, so there is nothing for a scenario to
  assert on. M1-007 and M1-012 were both invisible for this reason while every
  skill sweep passed green.
- **No scenario asserts on buff-bar contents** — M1-010/M1-011 sat in plain sight.
- **Fixtures clean up after themselves.** `character-create-delete` never holds 3
  characters at once, so M1-013 could not occur. It needed an account that had
  *accumulated* characters — what a person does and a fixture doesn't.
- **Fresh context per scenario.** `bad-password` never carries a prior successful
  login into a failure, which is exactly the state M1-015 needs.

## Two bugs stacked on one feature

M1-011 was **masking** M1-012. While every status was wrongly treated as
infinite, nothing expired, so a missing end packet made no observable
difference. Fixing expiry is what exposed it. Only sequential live testing
surfaces that ordering — a one-shot audit would have found the first and
declared victory.

## Test-environment lessons

- **Do not hand a tester a polluted character.** Aborting the headless suite
  mid-run left `test` as a level-99 Rogue with a hotbar full of skills from
  other jobs (Blacksmith, Acolyte, Whitesmith). Every key did nothing —
  correctly — and cost real time chasing a bug that did not exist.
- **macOS eats F1–F9** by default (`com.apple.keyboard.fnState` unset). Use
  **Fn+F1**, or enable standard function keys. Not a client bug.
- **Insert does not sit on this Mac** — laptop keyboards lack the key. **Home**
  is the verified path.
- **`@item` cannot make unidentified items.** `Hercules/npc/custom/identify_test.txt`
  adds an **Identify Test NPC at `prontera,164,200`** using `getitem2`.
  Deliberately separate from the Dialogue Test NPC — headless `dialogue-choice`
  asserts that NPC's menu exactly and picks by index.
- **Stock shops are `trader` NPCs, not `shop`.** Searching for `shop` finds only
  `npc/custom/itemmall.txt`, which is commented out. Sell was verified at the
  Payon Weapon Dealer (`payon_in01,15,119`).

## Docs corrected

The pass kept surfacing stale claims, each of which had already misled someone:

- `FEATURE_ROADMAP.md` §8.3 listed `StatusChangeSequencePacket` as pending
  *after* it was promoted, and `DisplaySkillCooldownPacket` / `UseSkillSuccessPacket`
  as "still noop" when both were already handled.
- The Phase 1 note claimed buff bar and weight footer had no coverage; both are
  now hand-verified.
- `M1-p0-verification.md` §4 told you to write an `input` test NPC that already
  existed, and its environment block had WSL-only paths for a macOS pass.

## Regression run + a caught server bug

Re-ran all 106 scenarios after the fixes — the first full run since the 07-13
gate. One failure: `dm-quest-lifecycle`, timing out on `QuestAdded`.

**It was not a client regression.** The three promoted packets are unrelated to
quests, and no quest file changed in the client commits. The diagnostic path:

1. `QuestAdded` comes from `QuestNotificationPacket1/4` — untouched today.
2. `git diff` over the day's commits: zero quest files.
3. The DM command *executed and reported success* ("Quest 20001 started for 1
   party member(s)") but the quest never landed in the char's table → the fault
   is server-side quest-add, not client packet handling.
4. The map-server log: `db/quest_db.conf:16164 - syntax error`. Quest_db failed
   to load, so `setquest()` silently no-oped for every campaign quest.

Root cause: Hercules `d28ffb666` (post-gate off-by-one MobId fixes) injected a
`//` comment into a **single-line** quest entry —
`Targets: ( { MobId: 1366 // Lava Golem /* … */ Count: 20 }, )`. `//` runs to
end of line, eating `Count: 20 }` and the closing `)`; libconfig then reported
the error at the next structure it reached (line 16164, which looked innocent).
Fixed in Hercules `aa40e2053`; server now loads all 3172 quest entries and the
scenario passes.

**Lesson worth keeping:** a single failure in the first run after a gap is not
automatically a flake *or* a regression from the current change. Ask first
whether the failing subsystem is even touched by the diff. Here it plainly was
not, which pointed straight at the intervening server commits rather than
today's client work.

## Still open

- **Clean logout** — the last checklist row needing a human.
- The `test` character was rebuilt into a clean, well-equipped **Knight** for
  future manual passes (see the character-rebuild note below); it does not affect
  the suite, which reshapes the job per scenario.

---

# M1-006 skill-targeting — built and live-verified (later 2026-07-16)

After the E3.1 pass closed and both repos were pushed, this follow-on session
built the **skill-targeting mode** filed as M1-006, live-verified it, and filed
two new findings. Uncommitted at time of writing; see the M1-006 row in
`plans/M1-p0-verification.md` for the canonical detail.

## What it does

Pressing a targeted hotbar skill with **no valid target under the cursor** now
*arms* it: the cursor becomes the attack reticle and the next left-click picks
the target. Per skill type:

- **Attack** — hover-instant fast path if already over a target, else arm.
- **Ground/Trap** — **always** arm, so you aim and place the AoE with the click.
- **Support** — hovered entity, else self (kept the original fallback).
- **SelfCast** — unchanged instant self-cast.

Cancel = right-click (primary) or Escape (Escape clears the target *before* it
opens the menu). Pressing another skill re-arms (swap); a chat line names the
armed skill. Core decision logic is `resolve_pending_cast`, a pure function with
6 unit tests; the armed state clears on map disconnect so it can't leak across a
relogin. All in `korangar/src/lib.rs` (rule-4 note: this is core input, not DM).

## The design conventions (user-approved)

Cancel via right-click **and** Escape; a second skill **swaps**; empty ground
**fizzles** for entity-targeted (stays armed, never walks) and **casts at the
cell** for ground-targeted. These are the genre-standard RO conventions.

## What live testing taught — three rounds, three real lessons

The feature "worked" in code immediately; the value was in what live iteration
surfaced that headless never could:

1. **Support arming broke self-cast.** First cut armed Support skills too. But you
   can't reliably click your *own* sprite (the picker returns the tile under your
   feet), so Heal became uncastable. Reverted Support to hovered-entity-else-self.
   Lesson: self-target support skills must not require a click-target.

2. **Ground instant-cast gave no way to aim.** The hybrid "instant-cast if over a
   valid target" is wrong for ground skills — the cursor is *always* over a valid
   tile, so it dropped the AoE at wherever the cursor sat with no aiming step. This
   read to the tester as "Storm Gust isn't selectable." Fix: ground skills *always*
   arm.

3. **The big one — test-character skill pruning.** Storm Gust wouldn't cast or even
   announce. Root cause was **not** the client: Hercules' `pc_calc_skilltree`
   **prunes on login** any skill whose tree prerequisites aren't met (and any skill
   outside the job's tree entirely). Granting a curated cross-tier / cross-job skill
   list via SQL left only the first-tier, prereq-less skills (Fire Bolt, Cold Bolt)
   — everything else was silently stripped, so the client correctly showed them as
   unlearned/dimmed and refused to cast. **You must grant a complete, valid job
   tree.** The `test` char (150000) is now a Wizard with the full Mage+Wizard tree.
   This cost the most time and is the reusable trap — recorded in the
   `ro-test-environment-traps` memory.

## M1-008 made all of this hard to see

Skills draw **no cast animation** (M1-008), so a correctly-sent cast looks
identical to one that did nothing — which is why "spells don't cast on ground"
looked like a targeting bug when it wasn't. Added temporary `eprintln` + a
"Casting X" chat line purely to prove the packet was sent (it was), then removed
both once confirmed. Kept only the **"Aiming X"** arm line, which is genuinely
useful (identifies the armed skill, makes swaps visible) and low-frequency.
Damage numbers already confirm offensive casts.

## New findings filed

- **M1-016 (P2)** — emotes rendered only as a chat line, no animated emoticon
  over the entity. Resolved later that day: the GRF assets were present, but
  self-emote echoes could not resolve because the local player lives outside
  the nearby-entity list. Name and render lookup now include the local player
  (and recently dead entities). A first live check then found the animation at
  the character's feet because the loader normalizes ACT frames to their bottom
  edge. The emote anchor now uses each entity's current composed sprite height,
  so players, small mobs, and bosses get distinct overhead placement. An
  81-entry scrollable picker was also added to the menu with an `Alt+L`
  shortcut, removing the need to type `/e <id>`. Live verification confirmed
  a picker-selected emote animates above the player. See §5.
- **Cast interruption (feature request, not a bug)** — the tester repeatedly
  wanted right-click/Esc to abort an *in-progress* cast. RO cancels a cast by
  *moving*; there is no cancel packet. This is a separate feature from targeting
  (M1-006's cancel only clears an *armed*, pre-cast skill) — build it on its own.

---

# M1-016 emote bubbles + picker — completed and live-verified

This follow-on closed the emote defect found during the GUI pass and added the
player-facing selection UI requested during live testing.

## What was actually broken

The assets and packet pipeline were already valid. An ignored archive test now
confirms both `data\\sprite\\이팩트\\emotion.spr` and `.act` exist in the
configured GRFs. The failure had two client-side layers:

1. **Self-emotes could not resolve their owner.** Hercules broadcasts the
   sender's `DisplayEmotion` packet back to the sender. Korangar stores the
   local player at `this_entity()`, outside `client_state().entities()`, but the
   emote name and render paths searched only the latter. The result was the
   paired symptom `Someone: delighted` plus no animation.
2. **The first working animation was grounded at the character's feet.** The
   generic animation loader normalizes each ACT frame to its bottom edge. Using
   the entity's ground position as the emote position therefore put the image
   underneath the actor instead of above its head.

## Implementation

- `world/emote.rs`
  - Resolves bubble owners across nearby entities, the local player, and
    recently dead entities.
  - Holds the shared 81-entry `EMOTION_NAMES` table used by chat and the picker.
  - Anchors each bubble at `entity position + current visual height + 2 units`.
  - Keeps `KORANGAR_EMOTE_DEBUG=1` diagnostics; the live trace showed 98 ACT
    actions/delays and successful action-2 frame submission.
- `world/animation/mod.rs` exposes `current_frame_world_height`, calculated from
  the current composed frame, renderer sprite scale, and entity scale.
- `world/entity/mod.rs` exposes `get_visual_height(camera)`. This is deliberately
  per entity and per current frame: a Poring, player, baby character, and tall
  boss do not share a hard-coded player offset. A 14-unit fallback is used only
  while that entity's animation data is still loading.
- `lib.rs` resolves local emote chat labels through `player_name`, includes the
  local/dead collections in rendering, and handles picker requests with the same
  `request_emotion` network call used by `/e <id>`.
- `interface/windows/emote.rs` adds a scrollable three-column palette containing
  all 81 named wire emotes. It is exported/registered in `windows/mod.rs`, has a
  persisted 520×480 default in `windows/cache.rs`, and is linked from the main
  in-game menu.
- `input/event.rs` adds `UseEmotion` and `ToggleEmoteWindow`; `input/mod.rs`
  binds the original-client-style `Alt+L` shortcut. The typed `/emotion <id>` /
  `/e <id>` path remains available.
- `loaders/gamefile/mod.rs` contains the ignored real-GRF asset regression test:
  `cargo test -p korangar loads_emote_assets -- --ignored` (run from the nested
  `korangar/` runtime directory so it sees the configured archives).

## Validation and live result

- `cargo check -p korangar` — passed.
- `cargo test -p korangar --lib` — **75 passed, 0 failed, 2 ignored**.
- `cargo build --release --bin korangar` — passed (existing dead-code warnings
  only).
- `git diff --check` — passed.
- macOS live check, 2026-07-16:
  - `/e 2` first proved the sheet loaded and the animation rendered, exposing
    the ground-anchor bug.
  - After dynamic-height anchoring and the picker landed, selecting an emote from
    the UI visibly animated it **above the player's character**.

## Honest remaining verification boundary

The same dynamic-height code runs for players, NPCs, mobs, and recently dead
entities, so mob placement is not based on an assumed player height. A mob/NPC
emote was not separately forced during this live session. Full visual
ID-to-action alignment across all 81 labels (especially dice and flags) also
remains optional polish, as recorded in `FEATURE_ROADMAP.md`; it is not part of
the closed M1-016 rendering defect.

## Resume state for the next agent

- The user closed the Korangar client after the successful live check.
- A fresh release binary containing these changes was built at
  `target/release/korangar`.
- The local Hercules login/char/map/API servers were still listening on
  6900/6121/5121/7121 when this handoff was written.
- All emote and picker changes are currently **uncommitted working-tree
  changes**. Preserve them; inspect `git status` before doing unrelated work.
- M1-016 is closed. The most relevant optional follow-up is a forced mob/NPC
  emote across a few sprite sizes and a spot-check of dice/flag ID alignment.

---

# M1-008 Wizard spell visual slice — implemented, live tuning in progress

The follow-up Wizard test refined the earlier M1-008 diagnosis. Fire Bolt,
Cold Bolt, Soul Strike, Thunderstorm, and Storm Gust do not depend on `0x01F3`
for their primary hit animation. Hercules sends all five through
`DisplaySkillEffectAndDamagePacket` (`ZC_NOTIFY_SKILL2`, `0x01DE`), including
the skill ID, caster, target, level, hit count, and timing. Korangar was
discarding the skill ID while converting that packet into a generic floating
damage event, so the renderer had no way to distinguish a spell from a basic
attack. Storm Gust's damage pulses follow this same path; its `0x86` skill unit
is the generic `Dummyskill` unit and does not identify Storm Gust by itself.

Implemented a contained first slice:

- `NetworkEvent::DamageEffect` now retains an optional `skill_id`. Basic attack
  packet handlers set it to `None`; `0x01DE` sets it to the packet's skill ID.
- The client maps the locally tested IDs — Soul Strike 13, Cold Bolt 14,
  Fire Bolt 19, Thunderstorm 21, and Storm Gust 89 — to shipped STR assets in
  the configured GRFs.
- Effects spawn at the struck entity's actual world position, with a matching
  temporary point light. Existing attack animation and damage-number behavior
  remain unchanged.
- The configured client data does not expose one-to-one classic
  `firebolt.str`/`coldbolt.str`/`soulstrike.str`/`stormgust.str` files. This
  first slice therefore uses close shipped STR effects; exact legacy-client
  recreation still requires a dedicated legacy effect renderer/mapping.
- `DisplaySpecialEffectPacket` and `NotifyGroundSkillPacket` remain broader
  M1-008 work. This slice deliberately does not claim full skill coverage.

Validation before live testing: `cargo check -p korangar`, release build, and
`git diff --check` all passed (existing dead-code warnings only). The rebuilt
client was launched for live checks of the Wizard spells.

First live result: Storm Gust rendered its full animation, proving the retained
skill ID, target position, and effect spawn path. The initially selected modern
nested STRs for Soul Strike, Cold Bolt, and Fire Bolt produced their point
lights but no visible geometry; Korangar's current STR renderer does not cover
the newer geometry features those files use. They were replaced with older
root-level, renderer-compatible effects (`strangelights.str`, `chill.str`, and
`meteor1.str`). Thunderstorm's first `cloudh.str` substitution also rendered
nothing. A temporary `storm_min.str` substitution proved the Thunderstorm
packet mapping by visibly playing Storm Gust, but was not retained as the final
look. The next candidate is the shipped
`lightningstrike\\lightningstrike.str`, selected specifically for lightning
semantics. Live verification found that it also produces no visible geometry.
The temporary Storm Gust mapping was removed rather than shipping a visibly
incorrect animation; Thunderstorm remains explicitly open.

## Live compatibility result and required follow-up

- **Storm Gust (89):** `storm_min.str` renders a complete visible animation.
- **Thunderstorm (21):** the event/skill-ID path is proven because temporarily
  mapping it to `storm_min.str` displayed the Storm Gust animation at the hit.
  Both `cloudh.str` and the semantically correct
  `lightningstrike\\lightningstrike.str` produced no visible geometry with the
  current renderer. Thunderstorm was therefore removed from the production
  mapping pending renderer support.
- **Soul Strike / Cold Bolt / Fire Bolt:** their first modern nested STR choices
  produced point lights but no visible geometry. Older root-level candidates
  are currently mapped, but this session did not record a conclusive live
  result for those replacements; do not mark them verified without retesting.

The next implementation thread is in `loaders/effect/mod.rs`,
`world/effect/mod.rs`, and `renderer/effect.rs`, not packet handling. Compare a
working legacy file (`storm_min.str`) against `lightningstrike.str` and the
modern nested files at parsed-frame/render-instruction level. In particular,
audit frame/morph interpolation, multi-texture handling, coordinate/origin
assumptions, and supported blend/animation modes. Add a renderer-level fixture
that asserts visible draw instructions for the lightning file before restoring
skill ID 21. The wider `DisplaySpecialEffectPacket` / ground-skill mapping is
still separate work.

---

# M1-008 root cause 2: the STR morph interpolator — fixed

The renderer-side follow-up predicted above is done. Comparing the parsed
frame tables (new ignored diagnostic
`loaders::effect::diagnostics::reports_str_frame_structure`, run with
`cargo test -p korangar str_frame_structure -- --ignored --nocapture`)
explained every live symptom:

- **STR key frames are basic/morphing pairs.** A basic frame (type 0) sets
  all fields absolutely; a morphing frame (type 1) sharing its key holds
  *per-key deltas* added onto the base each key, and its animation type +
  `delay` field drive texture cycling (type 2 stop-at-end, type 3 wrap,
  type 4 reverse). Reference semantics cross-checked against roBrowser's
  open-source `StrEffect.js` (algorithm only; no code copied).
- `lightningstrike\lightningstrike.str` layer 1 is exactly: base@0, all-zero
  morph@0 with animation type 3 / delay 0.4 across **21 textures**, final
  base@53. Its whole animation is morph-driven texture cycling.
- Korangar's `Layer::interpolate_frame` ignored frame types entirely: it
  lerped *between list entries two apart* (assuming strict pairing), its
  key→frame map yielded `None` between the first two keys, and
  `texture_index` was truncated to `usize` at load. Result: base+morph files
  drew nothing (or a degenerate quad), while `storm_min.str` survived only
  because it has a dense basic key on nearly every frame.

**Fix (`world/effect/mod.rs`, `loaders/effect/mod.rs`):** scan-based
selection (last basic ≤ key = base, adjacent same-key morph = deltas,
reference hide rules), fractional key index for smooth deltas, `texture_index`
kept as `f32` with the four texture-animation modes implemented, new
`AnimationType::Type4`, and the render-time texture bound check fixed from
`>` to `>=` (was an off-by-one panic risk). The buggy precomputed index map
is deleted. Five unit fixtures in `world::effect::frame_at_tests` encode the
shapes of the real files (lightningstrike cycle, cloudh delta accumulation,
storm_min static density, lone frames, clamp/reverse texture modes); full
`cargo test -p korangar --lib` green (80 tests).

**Mapping update (`wizard_skill_effect`):** Thunderstorm 21 restored →
`lightningstrike\lightningstrike.str`; Fire Bolt 19 →
`crescivebolt\crescivebolt_hit\crescivebolt_hit.str` and Soul Strike 13 →
`new_soulexpansion\new_soulexpansion_hit\new_soulexpansion_hit.str` (hit
effects of those spells' modern upgrades); Cold Bolt 14 →
`frostbite\frostbite_finish\frostbite_finish.str` (59-key icy shard burst;
`frostbite.str` itself is a 600-key/10-second loop, rejected). All five
mapped files parse with 0 unconsumed bytes and use only now-supported field
values (anim 0/2/3, mt 0, standard D3D blends). TGA textures confirmed
supported by the texture loader.

**Still open in M1-008:** live GUI verification of the five mappings (client
rebuilt + launched for it), `DisplaySpecialEffectPacket` (`0x01F3`) /
`NotifyGroundSkillPacket` breadth, and mt_present ≠ 0 files if any appear.

---

# M1-008 follow-up: the classic effect library was here all along

A direct GRF file-table scan (scratchpad Python lister, all four configured
archives) found the **complete classic effect library in `data.grf`**:
`thunderstorm.str`, `stormgust.str` (with `storm_min.str` as its official
"simplified effects" variant), `firehit1/2/3.str`, `icecrash.str`,
`lightning.str`, `spell.str` (cast circle), `firewall.str`, `meteor*.str`,
`lord.str`, `sanctuary.str`, and ~290 more root-level classic files.

**Listing gap (trap for audit tools):** `GameFileLoader::get_files_with_extension`
reported only 26 root-level STRs while the GRF tables hold 296 — direct
`FileLoader::get` loads them all fine, so runtime behavior is unaffected, but
**any audit that enumerates via `get_files_with_extension` under-reports GRF
contents.** This is why the earlier session concluded the classic files were
absent. Not yet root-caused.

**Classic remap (`wizard_skill_effect`),** wiring cross-checked against
roBrowser's effect table (semantics only): Thunderstorm 21 →
`thunderstorm.str`; Storm Gust 89 → `stormgust.str`; Fire Bolt 19 → random
`firehit1/2/3.str` per hit (EF_FIREHIT `rand: [1,3]`, via
`rand_aes::tls::rand_range_u32`); Cold Bolt 14 → `icecrash.str` (classic
attached ice-shatter; the original Cold Bolt itself was code-drawn, no STR
exists); Soul Strike 13 stays on `new_soulexpansion_hit.str` (also code-drawn
originally). All six files parse with 0 unconsumed bytes and use only
supported field values (frame types 0/1, anim 0/3, mt 0, blend 5/7).

# Warp-portal vortex on map-transfer points

Warp entities (job 45) carry no sprite in the original client either — it
draws a **rotating blue vortex** (textured cylinders of
`effect\ring_blue.tga`) at the warp position. Implemented:

- `EffectRenderer::render_effect_world_quad` — projects four *world-space*
  corners to clip space directly (the existing `render_effect` only offsets
  screen-space corners around one projected point), reusing the existing
  effect pass and `EffectInstruction` path unchanged.
- `world/effect/portal.rs` — `PortalVortex` (`EffectBase`): two nested
  counter-rotating cylinders (20 segments each, outer r4.0→3.4 h11, inner
  r2.4→2.0 h16, additive SrcAlpha/One, cell = 5 world units), spawned in the
  `AddEntity` handler for `EntityType::Warp` via `effect_holder.add_unit`,
  torn down by `remove_unit` in `RemoveEntity` (added unconditionally at the
  end of that handler) and by the existing `effect_holder.clear()` on map
  change. Sizes/spin/alpha are first-guess values pending live tuning.

# Mob emote chat text silenced

The `DisplayEmotion` handler printed "<name>: <emote>" for every emote as
lazy-load fallback feedback. Now that bubbles render, the line only prints
for **player** entities (kept as chat-log convenience); monster and NPC
emotes are bubble-only, matching the original client and keeping scripted
mob emotes from spamming the log.

All three changes: `cargo test -p korangar --lib` green (80), release
rebuilt, client relaunched for the live pass. Remaining live checks: five
spells' looks, portal size/rotation feel, mob emote silence.
