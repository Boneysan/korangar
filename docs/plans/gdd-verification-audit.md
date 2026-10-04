# GDD verification audit

**Started 2026-10-03.** Owner request: check every GDD v0.2 task against what was claimed,
because reviews on 2026-10-03 kept finding "Built" rows that were partial, unwired, or broken.
This plan governs that audit; [gdd-improvement-plan.md](gdd-improvement-plan.md) (F01–F39),
[gdd-next-slices.md](gdd-next-slices.md) (slices 1–15, S1–S10) and GDD Appendix E are the claims
being checked. Update a row's verdict here, then correct the claim at its source.

## Why: what the 2026-10-03 reviews found

| Pattern | Example | Caught by |
|---|---|---|
| Claim names code that does not exist | F23 `show_chat_timestamps` toggle, `<GUIDE:id>` links | grep for cited symbols |
| Built, never wired to a player | F30 crafting formulas, F31 commission assign/complete, F16 recap fields | clippy dead-code |
| Data silently defeats the code | Pilot `ClearSkills` removed every MVP's minion summons | reading the data the code loads |
| A fix changes what tests observe | ViewData elite (class 1152) broke two elite scenarios and `@metrics` counts | running the scenarios |
| Work dropped between branches | 36 commits on closed PR #10, incl. 2026-09-18 live-confirmed fixes | `git cherry` + symbol check |
| CI never ran the real thing | Hercules builds skipped for weeks; client built only beside Hercules | reading CI logs |

The common cause: "Built" was written from the code author's side. Nothing checked that a player
can reach the feature, that the shipped data lets it work, or that a test would fail if it broke.

## Method: five checks per task

Each task gets a verdict from the strongest evidence reached. Use the CLAUDE.md evidence words.

1. **Claim inventory.** List every concrete claim in the row: symbols, files, numbers, behaviors.
2. **Source and wiring.** Each cited symbol exists, and something in the *production* path calls
   it: a window, menu entry, key binding, packet handler, NPC, or atcommand. Test-only callers do
   not count.
3. **Data and config.** The data the code needs is loaded in this server's configuration, and
   nothing loaded after it overrides it (ClearSkills, import configs, ViewData, map flags).
4. **Test meaning.** A test exists that would fail if the behavior broke (mutation check: break
   it, see red). Run it. Headless for server behavior; unit tests for client logic.
5. **Runtime.** Server behavior: a headless scenario against the dev server. Client UI: a line in
   the live GUI checklist ([gui-verification-pass.md](gui-verification-pass.md)), run with the
   owner. Nothing is "Verified" on source reading alone.

**Verdicts:** `Verified` (checks 1–5 pass) · `Verified (unseen)` (1–4 pass, GUI not yet watched)
· `Partial` (named gaps) · `Claim wrong` (the row overstates; correct it) · `Broken` (fix first).

## Tooling to build first (Batch 0)

- **Claim checker** `tools/audits/gdd_claims.py`: parse F01–F39 and S rows, extract every
  backticked identifier, report missing symbols and symbols with only test callers. One pass
  triages all rows and replaces guesswork about where to look.
- **Dead-code map:** list every `allow(dead_code)` / `cfg_attr(not(test), …)` and map it to its
  F row (the 2026-10-03 clippy pass marked F30, F31, F16 this way).
- **Scenario index:** which headless scenarios cover which rows, and which are absent from CI's
  paired list (the elite scenarios were never in CI).
- **Safe all-scripts load:** the CI "extratest" mirror must use a throwaway database; the
  2026-10-03 run against `herc_re_db` left orphan barter rows and inert `$` variables.

## Order: player impact first

| Batch | Scope | Why first | Main checks |
|---|---|---|---|
| 0 | Tooling above; decide PR #10 (port or record as dropped) | Triage for everything else | Claim checker run |
| 1 | Server gameplay: F14, F15, F16, F39, S3, S4, S8, S10, F21, F34 | Changes what players fight and earn; silent failures | Data/config, headless runtime |
| 2 | Combat and input: F09, F10, F11, F12, F13, slices 2, 3, 10, 13 | Every fight uses it | Wiring, unit tests, GUI checklist |
| 3 | Party and social: F20, F22, F23, F24, F25, slices 11, 12, 14 | Multiplayer paths, hard to test alone | Two-client headless, GUI checklist |
| 4 | Economy: F26–F31, F27 refine, trade (F28), slice 7 | Item loss/duplication risk | Server handlers, SQL paths, headless trade |
| 5 | Guide and data: F01–F08, F37, slices 5, 6, 8, 9 | Large but display-only | Claim checker, exporter `--check`, Guide tests |
| 6 | Polish and ops: F32, F33, F35, F36, F38, slices 1, 4, 15 | Lowest risk | Wiring, GUI checklist |

Each batch ends with: verdicts recorded below, claims corrected in the plan and Appendix E, fixes
filed as their own slices (not mixed into the audit), and the batch's headless scenarios added to
CI where missing.

## Already known going in

| Row | Finding | Status |
|---|---|---|
| F16 | Pilot MVPs never summoned escorts (`ClearSkills`) | Fixed 2026-10-03 (Hercules `35964619a`, scenario `mob-eddga-summons-escorts`) |
| F15 | ViewData elite broke two elite scenarios and `@metrics spawns` | Fixed 2026-10-03 (korangar `6ecd527b`, Hercules `35964619a`) |
| F14 | Aggressor flag lost on mode recalculation (latent) | Fixed 2026-10-03 |
| S8/F39 | Online hand-in could take equipped copies | Fixed 2026-10-03 |
| F23 | No chat timestamp toggle; no `<GUIDE:id>` links | Claim corrected in the plan 2026-10-03 |
| F30 | Formulas not shown in any window | Partial — open |
| F31 | No assign/complete action in the board window | Partial — open |
| F16 | Recap tracks damage taken/interrupts; toast shows only damage dealt | Partial — open |
| PR #10 | Live-confirmed 2026-09-18 fixes never reached `main` | Settled 2026-10-03: 7 ported + WASD merged (below) |
| F04 | Stat view modes not wired to any window | Claim wrong — open |
| F03 | ~~Build planner projections not shown~~ — wrong: shown via `state/build_planner.rs` (Batch 5) | Closed |
| CI | PR gate covers 10/206 scenarios; Batch 1 scenarios only weekly | Open |

## Batch 0 results (2026-10-03)

**Claim checker** (`tools/audits/gdd_claims.py`, 93 rows): every cited name exists except
F22 `PartyMemberLocationUpdate`, F23 `show_chat_timestamps`/`is_message_visible_in_tab`, and F24
`RequestEquipItem`/`SavedEquipmentSets`. Existing names prove little on their own, which is why
the dead-code map matters more.

**Dead-code map.** Removing the file-wide `#![allow(dead_code)]` from six GDD modules shows code
nothing in the client calls:

| Row | Unused outside tests | Meaning |
|---|---|---|
| F04 | `StatViewMode`, `SimpleStatDescription`, `DetailedStatRow`, `AdvancedStatMetrics`, `CharacterStatsInput` | Simple/Detailed/Advanced stat modes are not wired to any window |
| F03 | `calculate_projections`, `can_increase_stat` | Unused duplicates only: the planner window shows HP/SP, HIT/FLEE, DEF, ATK and CRIT deltas from `state/build_planner.rs` (corrected in Batch 5) |
| F02 | `skill_cooldown`, `skill_after_cast_*_delay`, `skill_prerequisites`, `skill_status_change`, `skill_source` | Timing helpers unused; check whether the Guide reads the same data another way |
| F30 | the whole formula module | No window shows a success chance |
| F04/F10 | `is_unsupported` in `item_bonus.rs` | Minor |
| Encyclopedia | 21 unread fields/methods in `reference_data.rs` | Data loaded but not shown; triage in Batch 5 |

The blanket allowances should be replaced by item-level ones with a reason, so the compiler
reports new unused code again.

**Scenario index.** The pull-request gate runs 10 of 206 headless scenarios (smoke, registration,
connection, character select, three quest scenarios, `skills-mage`, `trade-reject`,
`party-lifecycle`). Every Batch 1 scenario (monster AI, MVPs, elites, EXP sharing, death recovery,
quest credit, discovery, DM replay) runs only in the weekly full run on `main`. A green PR says
nothing about them.

**Disposable runner works locally (2026-10-03).** MariaDB login `korangar_it`@`127.0.0.1`, limited
to `korangar_integration_*` databases, password in git-ignored `target/integration-db.env`
(`set -a; . target/integration-db.env; set +a` before `tools/testing/run-integration-tests.sh`).
The dev server must be stopped first (same ports). It cleaned up every database it made. Five of
Batch 1's seven dev-server failures were only fixtures the dev server does not load; run
fixture scenarios through this runner, never against `herc_re_db`.

**PR #10 — settled 2026-10-03 (owner chose option 1).** Ported onto korangar
`agent/review-fixes` (local, not pushed), each re-applied against today's `main`:

| Fix | Commit | Notes |
|---|---|---|
| Windows pack Vulkan/DX12 choice | `a17b5a2f` | Clean cherry-pick |
| No click sound on drag start | `b735fca2` | 2 tests |
| Space attacks the Tab target | `e54146fa` | Rebindable `AttackTarget`; shares Space with the debug camera |
| Sell list: whole ammo stacks; sell cart cleared after a sale | `9f8e6ee6` | Equipped items were already refused server-side (Hercules `8b850e4e5`); PR #10's filter was test-only. The cart bug was new |
| Pack version at login, "out of date" popup, Update scripts | `4f361bf0` | **Server check is off**: local `conf/import/login-server.conf` lacks `check_client_version: true` |
| Class name on character slots | `252c5b3e`..`16675621` | |
| Vendor "cannot wear" marker with reason tooltip | `dabf385b` | Generator moved to `tools/generate_equipment_eligibility.py`; in `generated-drift.sh` |
| WASD: one held path, corner rule, wall slide | `9e52e852` | Corner rule from `path.c` `chk_dir`; mutation-checked |

Dropped: the pickup toggle and party privacy (option 2, not chosen), PR #10's quest tracker and
recovery slices (`main` rebuilt both), the quest-name table, and its docs/runbooks. None of the
ports has been seen live; each needs a line in the GUI pass.

## Verdict log

| Row | Verdict | Evidence | Gaps / fixes filed | Date |
|---|---|---|---|---|
| F14 | Verified (unseen) | Boot log loads 29 profiles; `check_mob_skill_families.py` passes; 6 mob-AI scenarios pass (skirmisher ×2, keeper ×2, coward, raydric) | **Fabre `Aggressor` on `prt_fild08` is now really aggressive** (it was inert until the 10-03 mode fix) and swarms the novice field; it breaks `party-quest-credit` (A/B: passes with the profile removed). Design call needed | 2026-10-03 |
| F15 | Verified (unseen) | `mob-eddga-pilot-skills`, `-meteor-and-enrage`, `mob-elite-population`, `mob-elite-rollback-switch` pass | No live fight watched | 2026-10-03 |
| F16 | Partial | Eddga runtime-verified (`mob-eddga-summons-escorts`). Data check of all 11 MVPs: every cast ≥1 s is `Cancelable`, enrage at 30/40(50)/80% HP, no teleport, escorts restored | The other 10 MVPs are data-checked only, never fought; client recap fields still partial (see above) | 2026-10-03 |
| F21 / S3 | Verified | `party-experience-sharing` passes: solo 117/87, 31-cell pair 72/53 each, 30-level boundary in, 31-level and other map out; `party_even_share_bonus: 25`, `party_share_level: 30` tracked in config | — | 2026-10-03 |
| F34 / S4 | Verified | `death-recovery-save-point` passes; `death-recovery-ten-kill-threshold` **fixed and 6/6 green** (was flaky since `af45e4ab`); danger warning wired with boundary tests | Root cause was the test, not the server: a killing hit sends damage then death back to back, the loop returned on the damage, and the next `flush()` discarded the death, so it attacked a corpse. `af45e4ab`'s `@str 60` made one-hit kills the norm. The quest-Spore helper had the same latent flaw; both fixed | 2026-10-03 |
| S8 | Verified, test broken | Shared kill credit passes (mixed ownership, 30/31 cells, solo after leave) **with the Fabre profile removed** | `party-quest-credit` fails as shipped: Fabres swarm the test character on `prt_fild08` (see F14) | 2026-10-03 |
| F39 | Partial | **First ever run**: `party-quest-interaction-credit` passes under the disposable runner (mixed ownership, 30/31 range, party leave) | Disconnect and dead-member cases untested; no quest uses the commands yet | 2026-10-03 |
| S10 | Verified | All 5 `dm-party-*` scenarios pass under the disposable runner (offline replay and transitions, recreation and reward isolation, alternate character) | — | 2026-10-03 |
| F09 | Verified (unseen) | Every cited function has a production caller (`hovered_skill_range`, `pending_skill_cursor_state`, `stippled_footprint_cells`, quickcast/hold-aim settings in the window) | GUI pass | 2026-10-03 |
| F10 | Fixed, unseen | Throttle wired at the hold-mouse site. **Defect:** it re-sent an unchanged destination every 200 ms mid-walk; Hercules answers each with a new `PlayerMove` (`unit.c` `change_walk_target`) — the WASD stutter mechanism. Fixed `f5703669` (test expectation changed on that source evidence) | Live check: hold the mouse on a far tile | 2026-10-03 |
| F11 | Claim wrong (tests) | Prediction, echo check and rollback are wired (`SkillFailed` paths) | The row says unit tests cover prediction and rollback; only `server_echo_repeats_prediction` is tested. `predict_local_motion` and `rollback_predicted_motion` are `Client` methods with no test | 2026-10-03 |
| F12 | Verified (unseen) | `format_monster_target_summary` (status chips, cast cue, target-of-target) is the production summary and is tested; `BossTargetWindow` opened from the client | GUI pass | 2026-10-03 |
| F13 | Fixed, unseen | Waypoints, zoom, route accents wired. **Gap:** portal and spawn-region layers had settings the minimap read but no toggle in Game Settings; added `8e702427` | Hiding portals also hides the tracked route's portal accent (tooltip says so) | 2026-10-03 |
| Slices 2, 3, 10, 13 | Verified (unseen) | Cast footprint uses `level_invariant_skill_footprint` in the render path; Tab cycling, `TimedBufferedAction`, keybinding table wired; 32 Batch 2 tests pass | GUI pass | 2026-10-03 |
| F20 | Verified (unseen) | `/mark`, `/goal` wired; marked monsters drawn in the world (`lib.rs` target-marker pass); `Tonight:` goals block | Multi-client pass | 2026-10-03 |
| F22 | Verified (unseen), names wrong | Navigate/jump to member, healer layout and member location exist under other names | Row cites `PartyMemberLocationUpdate`, which does not exist; multi-client pass | 2026-10-03 |
| F23 | Claim wrong — corrected | Tabs filter (`chat_message_matches_tab`); official-form `<ITEM>…<INFO>id</INFO></ITEM>` links validated and defanged; timestamps always on | No timestamp toggle, no tab unread indicators, no `<GUIDE:id>` links. Row rewritten in the plan | 2026-10-03 |
| F24 | Fixed, unseen | Four hotbar rows; equipment sets save/equip/delete (`/saveset`, `/equip`). **Defect:** Mouse4/Mouse5 chords were accepted but never read, and could not be captured; fixed `db1544c4` (mutation-checked) | Row cites `RequestEquipItem` / `SavedEquipmentSets`, which do not exist under those names | 2026-10-03 |
| F25 | Verified (unseen) | Seven profiles applied in `cache.rs`; combat fade; lock state | GUI pass | 2026-10-03 |
| Slice 11 | Verified (unseen) | Six ping kinds over `[KORANGAR-PING:v2]`, ready check wired; 56 party/chat/HUD tests pass | Multi-client pass. Slices 12 and 14 not traced in this batch | 2026-10-03 |
| F01 | Verified (unseen) | `StatPreviewSelector` drives the Stats window tooltips | GUI pass | 2026-10-04 |
| F02 | Verified (unseen) | Prerequisites, cooldowns and delays reach the Guide and hotbar tooltips from the exported skill data (`adventure_guide.rs`, `skill_box.rs`) | The `skill_info.rs` timing helpers are unused duplicates of that path | 2026-10-04 |
| F03 | Verified (unseen) | Build planner window (from the Stats window) shows level/stat sliders and HP/SP, HIT/FLEE, soft DEF/MDEF, status ATK/MATK, CRIT deltas computed in `state/build_planner.rs` | Batch 0's "projections not shown" was wrong; `calculate_projections` is an unused duplicate | 2026-10-04 |
| F04 | Claim wrong | — | No window offers Simple/Detailed/Advanced modes; `world/stat_view.rs` has no production caller. The row describes code, not a feature | 2026-10-04 |
| F05 | Verified | Route services, locks and access notes exercised by Guide tests and `generate_navigation_graph.py --check` (export drift clean 2026-10-03) | Live route walk | 2026-10-04 |
| F06 | Verified (unseen) | `MapSpawnDetails` used by the world map and Guide; danger threshold wired (Batch 1) | GUI pass | 2026-10-04 |
| F07 | Verified (unseen) | `broad_spawn_rectangles_for_map` feeds the minimap spawn layer (toggle added in Batch 2) | GUI pass | 2026-10-04 |
| F08 | Verified (unseen) | `alias_targets` feeds skill, status, monster, item, job and map search | GUI pass | 2026-10-04 |
| F37 | Verified (unseen) | `server-rules.v1.json` embedded and rendered by the Guide (`adventure_guide.rs`); export drift clean | GUI pass | 2026-10-04 |
| Slices 5, 6, 8, 9 | Verified (unseen) | Navigation graph, world-map atlas, skill tooltips and Guide window wired; all their exports pass `export_supported_data.py --check` | GUI pass | 2026-10-04 |
| F32 | Verified (unseen) | `effect_density` read at the effect spawn points; cycle button in Game Settings; source-level test keeps telegraphs/cast bars exempt | GUI pass in a crowded fight | 2026-10-04 |
| F33 | Verified (unseen) | `render_tooltips` flips each tooltip to the roomier side of the cursor; `handle_drag` constrains windows | Screenshot matrix | 2026-10-04 |
| F35 | Verified (unseen) | Five cues played from `lib.rs` via `state/audio_cues.rs` | Never heard; fit by ear pending | 2026-10-04 |
| F36 | Verified | `@metrics` exercised by `mob-elite-population` (passes, name-based count since 10-03) | No real-session use yet | 2026-10-04 |
| F38 | Historical | The 2026-10-03 regression figures are a point-in-time record, not a standing check | The headless tester has 38 clippy errors CI never sees (lint runs without `--examples`) | 2026-10-04 |
| Slices 1, 4, 15 | Verified (unseen) | `ToastQueue` drives HUD notices; character delete with typed-name confirmation; High Contrast and Deuteranopia are in the theme lists and load real palettes (`state/theme/interface.rs`) | GUI pass | 2026-10-04 |

## Open work (collected 2026-10-04, after Batches 0–6)

One list of everything the audit left open. Tick an item here when it is done and say where.

**Economy safety (Batch 4, in progress on `agent/batch4-economy`)**
- [x] Loot fixture registered (Hercules `16222684f`); five loot scenarios added (`autopickup-respects-drop-owner` new; four ported from PR #10). All pass; the ownership test fails against a planted bug (`pc_takeitem` ignoring the owner window). `loot-pickup-multi-pile` failed once in ~6 runs (a pile granted to nobody, not lost); it now reports positions on failure. Watch it.
- [x] Item conservation is a gate: the 2026-10-04 full run's only difference was an unlogged starting Cotton Shirt, now an explicit exception; any other difference fails the run.
- [ ] Move `log_zeny: 1` from the untracked local `conf/import/logs.conf` into tracked config, so a fresh server logs zeny.
- [ ] Decide a retention policy for `picklog`: monster-drop rows (`M`) alone were 2.5 million on the dev database.
- [ ] Optional hardening: the split handler copies the source row's `id` into the new half; set it to 0 so the char-server's matching never has to cope with a shared id.
- [ ] Remaining Batch 4 rows: F26 autoloot settings, F27 refine odds against `refine_db`, F28 live two-client trade, F29, F30 (no window shows crafting odds), F31 (no assign/complete action).

**Campaign rewards (added 2026-10-04; outside the GDD rows, so no batch covered it)**
- [ ] Inventory every call to `DM_GivePartyItem`, `DM_GivePartyZeny`, `DM_PartyExp`, `DM_QueueGrant` and the treasure/quest grants across the 19 arcs; confirm each sits behind a once-only flag.
- [ ] Check `DM_QueueGrant` cannot pay an offline member twice (reconnect, party replay, re-join).
- [ ] Headless scenarios that try to claim twice: re-talk, reconnect mid-reward, party replay. The item-conservation audit cannot catch this class: a repeated reward is logged correctly every time.
- [ ] F27 refine odds: compare the export directly against `db/re/refine_db.conf` (the drift check only proves the export matches its own generator).

**Claims that are wrong or partial (decide: build, or correct the row)**
- [ ] F04 stat view modes: no window offers them. Build the mode switch or drop `world/stat_view.rs` and rewrite the row.
- [ ] F23: no timestamp toggle, no tab unread indicators, no `<GUIDE:id>` links (row corrected in PR #14). Scope Guide links if wanted.
- [ ] F11: prediction and rollback have no tests; extract them from `Client` into testable functions.
- [ ] F16: 10 of 11 pilot MVPs are data-checked only; add one fight scenario per MVP (or a sweep).
- [ ] F16 recap: damage taken and interrupts are tracked but not shown.
- [ ] F39: disconnect and dead-member cases untested; no quest uses `partycompletequest` yet.
- [ ] F22 and F24 rows cite names that do not exist (`PartyMemberLocationUpdate`, `RequestEquipItem`, `SavedEquipmentSets`); correct the text.

**Live checks nobody can automate**
- [ ] GUI verification pass, including section 7 (the PR #10 ports, WASD corner/slide, Mouse4/Mouse5, hold-mouse movement).
- [ ] Two-client session: party marks, goals, frames, pings, ready check, trade.
- [ ] Listen to the five F35 audio cues.
- [ ] Run the Windows pack on Windows (never done), including the Vulkan/DX12 choice.
- [ ] Pack version gate: the client sends its version but the server check is off; turn it on with the next pack.

**CI and code hygiene**
- [ ] The PR gate runs 10 of ~207 scenarios; add a fast economy/Batch 1 subset.
- [ ] Fix the headless tester's 38 clippy errors and lint examples in CI.
- [ ] Replace the six file-wide `#![allow(dead_code)]` with item-level allowances, and remove the unused duplicates found here (`skill_info.rs` timing helpers, `calculate_projections`, possibly `stat_view.rs`).
- [ ] Triage the 21 unread fields/methods in `dm/reference_data.rs`.
- [ ] New in the 2026-10-04 full run: `finale-loki-briefing` (dialogue did not finish in 160 steps) and `mob-coward-poring` (Poring never fled after a 235-damage hit). 199 passed, 2 failed, 1 expected skip.
- [x] `skills-professor` failed in the 2026-10-02 full run; it passes in the 2026-10-04 disposable run (85.8 s). Watch for a recurrence.
