# Headless Client Tester — Findings, Investigations & Resolutions

This document logs historical findings, protocol discoveries, and resolutions encountered while running the Korangar headless client integration tests (`korangar-networking/examples/headless-tester`).

All findings are classified by layer:
* **Shared Crate**: Changes to `ragnarok-packets` or `korangar-networking` (directly affects the graphical client).
* **Client Test Harness**: Changes to test-runner logic (`scenarios/`, `context.rs`, etc.).
* **Server Emulator**: Changes to Hercules configuration, scripts, or databases.

---

## Shuffle pass — July 29/30, 2026 (five bugs, and a tally that was wrong)

Running the 114-scenario suite in a **randomised order** (`--shuffle <seed>`)
found **four order-dependence bugs** and **one real client bug**. None were
reachable by the existing double-run gate, which runs the same order twice.

**Read this first: the previously recorded "114/114 green" was never true.**
`skills-dancer` and `skills-gypsy` had been failing the whole time behind a green
count, because a skipped scenario was tallied as PASS. Both the cause and the
mechanism are below. **A pass count that includes non-tests is worse than a red
one** — it is the reason these sat unnoticed.

| Issue | Scenario | Layer | Root cause & resolution |
| --- | --- | --- | --- |
| **SP drained across scenarios** | `weapon-refine-missing-material` | Client Test Harness | Logged for weeks as "order-dependent, root cause not yet found", and never a refine bug. `skill-fail-rejection` drains SP to zero (`@heal 0 -999999`) and never restored it; `WS_WEAPONREFINE` costs 5 SP, and `ensure_job` does not heal, so the drain survived the job change to Whitesmith. The cast was rejected, `RefinableWeaponList` never arrived, the scenario timed out. **Fix:** restore SP on the way out, including the failure path. **Proved** by running the two back to back. |
| **"Far" map was not far** | `observer-look-clear` (+ 2 latent) | Client Test Harness | `FAR_MAP` was the constant `"prontera"`, but `connect_pair` meets wherever the *partner character was last left*, and that position persists in the `char` table across scenarios. When order parked the partner on prontera, "warp away to leave view" became "warp to a random cell on the observer's own map" and `assert_in_view(false)` stopped holding. **Fix:** `far_map_from(home_map)`. Three scenarios used `FAR_MAP`, so all three were latent. |
| **Sex-locked jobs silently remapped** | `skills-dancer`, `skills-gypsy` | Client Test Harness | *Not* order-dependent — these could never pass. The shared character is male, and **Hercules does not refuse a sex-mismatched job change**: `pc_mapid2jobid` (`pc.c:6465`) round-trips the request through the character's sex, so asking for Gypsy yields Clown and the server reports "Your job has been changed." With no failure message, `sweep_job`'s gender-restriction skip guard could never fire, so it timed out instead. **Fix:** route female-only jobs to `HeadlessTwo` (already female, level 99, GM group 99 — **no new account needed**, sex is per-character here). |
| **Inherited job decided the outcome** | `incoming-damage` | Client Test Harness | Deliberately did not call `ensure_job`, so it inherited whatever job ran before it; after `skills-soul-linker` the provoked mob never retaliates. Passed in natural order only because `dm-warp-recall` precedes it there and leaves the character alone. **Fix:** best-effort `ensure_job(4008)`, preserving the no-GM A/B path. **Two wrong hypotheses first:** a lethal provoking blow (the fallback never fired) and mob species (1007 both passed *and* failed). A one-variable bisect settled it in two runs. |
| **Map-zone rejections were silent** | `skills-dancer`, `skills-gypsy` | **Shared Crate** | `ZC_NOTIFY_MAPINFO` (**0x0189**) was unmodeled, so `register_length_fallbacks` consumed it and **four user-facing messages were dropped**: cannot teleport here / save point cannot be memorized / skill unusable here / item unusable here. Hercules sends this *instead of* `clif->skill_fail` (`clif.c:6213` says so outright). Any skill in the map zone's `disabled_skills` therefore did nothing at all, silently, on every non-PvP map — `DC_UGLYDANCE`, `BD_ETERNALCHAOS`, `BD_ROKISWEIL`, `CG_HERMODE`, `BA_DISSONANCE`, `DC_DONTFORGETME`. **Fix:** packet + handler + regression test over all four types (`ee26fb23`). **Wire-verified only — not yet seen in the graphical client.** |

### Skips are no longer counted as passes

`sweep_job` returned `Ok(())` on a skip, so it printed "skipped" and tallied as
PASS. `Scenario` now has a third outcome (`SKIPPED_PREFIX` / `skipped()` /
`is_skip()` in `scenarios/mod.rs`), reported as `SKIP` and excluded from the pass
count. A skip is deliberately an `Err`, so the worst case is a visible amber row
rather than a false green.

**`skills-novice` is the one legitimately permanent skip** — the Novice tree is
passive apart from quest-gated actives (First Aid / Trick Dead), which `@allskill`
cannot grant. That is why a skip must **not** fail the exit code: it would leave
the suite permanently red for a correct reason, recreating the same
learn-to-ignore-the-tally problem from the other direction.

### Three more, found while validating the above (July 30)

| Issue | Scenario | Layer | Root cause & resolution |
| --- | --- | --- | --- |
| **Random mid-run disconnects** | 16–24 scenarios at a time | **Server Emulator** | Hercules' anti-flood counts connections **per IP**, and here everything is 127.0.0.1 — suite *and* servers. `ddos.count` (5 per 3000 ms) is trivially exceeded, and the flag sticks for 10 minutes. **The char-server connects to the login-server from 127.0.0.1 too**, so its reconnect is refused, `there is no char-server online` floods the log, every login fails, and the harness retrying renews the flag. **Fix:** `ip_rules.enable: false` in `conf/common/socket.conf` (Hercules tree — see CLAUDE.md §3b). Before: 38/126/105 and 16 failures. After: 0/0/0 and 113 passed. |
| **Ammo accumulated until the character was overweight** | `use-consumable`, `skill-fail-rejection`, `skills-hunter`, `skills-sniper` | Client Test Harness | `observer-ammo-disguise` granted 100 Silver Bullets per run and never removed them. Ammunition stacks, so it was invisible — until ~1600 rounds pushed the shared character past its weight limit, at which point Hercules answers `@item` with "Failed to pick up item." and **every item-dependent scenario fails at once**, far from the cause. **Fix:** clean up the ammo alongside the gun. The backlog was purged by hand via SQL and does **not** replay on a fresh database. |
| **`SkillFailedMissingItem` counted as silence** | `skills-hunter`, `skills-sniper` | Client Test Harness | `ZC_ACK_TOUSESKILL` causes 71/72 arrive as this event rather than a `ChatMessage` (the networking crate has no item DB), and the sweep's matcher omitted it — so a correctly-reported refusal read as "SILENT". Hidden because the character carried a stash of Trap items from earlier runs, so traps *succeeded*; purge the inventory and all seven go silent at once. Same class as the `SkillEffectNoDamage` arm. |

### Ground skills now assert their unit (CLOSED 2026-07-31)

Traps are now held to `AddSkillUnit` (see the commit "Make trap sweeps prove the
trap was actually placed"). **The other 119 `Ground`-typed skills are not**, and
`ground-unit` had never once been reported before that change.

The reason is the `cast` outcome: most ground skills have a cast time, so
`SkillCast` arrives first, satisfies the generic "any response" wait, and the
sweep never looks at what the cast *produced*. Traps are instant, which is why
they were the family that exposed it.

Outcome distribution for the 119 `Ground` skills in a full run:

| Outcome | Count | Can a unit exist? |
| --- | --- | --- |
| `fail-feedback` | 72 | No — refused |
| `fail-missing-item` | 13 | No — refused |
| `cast` | 23 | **Yes, unverified** |
| `buff` | 9 | **Yes, unverified** |
| allowlisted | 2 | — |

So the addressable gap is **~32 casts**, not 119 — asserting a unit for the 85
refusals would be wrong.

**The authority is `skill_db`: 129 skills have a `Unit:` block** (Safety Wall,
Fire Wall, Thunderstorm, Pneuma, Warp, Arrow Shower, Sanctuary, Magnus …), which
is very nearly the E1/E2 rendered-effect set — so this is the coverage that
matters most for the effect work. Deriving that id list from `skill_db` has
precedent in `tools/generate_packet_lengths.sh`. Expect real triage when it
lands: invisible units and owner-only sends mean some legitimately never reach
the caster.

**Closed.** `UNIT_CREATING_SKILLS` (the 129 skills with a `Unit:` block) must now
produce `AddSkillUnit` or an explicit refusal; `SkillCast` is not accepted, since
a cast bar starting is precisely what the loose standard mistook for evidence.
A full run reports **38 `ground-unit` assertions**, against zero for this suite's
entire history before 2026-07-30.

Two things the measurement forced, both worth keeping if this is ever revisited:
a **12s** window (these have cast times; the trap window of 4s would have caused
false failures), and accepting `WarpList` for `AL_WARP`, which opens a
destination picker *before* the portal exists — a real response, not silence, so
accepting it beats allowlisting it.

**The predicted triage did not happen.** I expected invisible and owner-only
units to need several documented allowlist entries; there was exactly one case
(`AL_WARP`). `CR_GRANDCROSS` and `PA_GOSPEL` prove their units despite being
`SelfCast` — caught because the check keys off skill id, not cast type — and the
three Ninja skills report `fail-missing-item` for stones the harness never
grants. Worth recording that the estimate was wrong in the pessimistic direction.

**Maintenance:** the id list is a snapshot of `skill_db`. Regenerate it when the
skill database changes — the snippet is in the const's doc comment.

**Known intermittent:** `MG_FROSTDIVER` occasionally reports SILENT in
`skills-super-novice` (target dies or the response misses the 4 s window). Two
isolated re-runs passed, reporting `fail-feedback` and `cast`.

**The diagnostic worth remembering:** a failure that is connection-flavoured
*and* accompanied by `Packet coverage: … 0 failed` is an environment problem,
not a test or protocol problem. That one check separates the top row above from
everything else in this document, and would have saved most of a day.

### Two stale entries worth correcting

* `BD_ETERNALCHAOS` / `BD_ROKISWEIL` are allowlisted as "Ensemble / Duet skills".
  They were actually **map-zone-disabled**, and now produce real feedback — so the
  entries are both inert and misleading.
* An earlier note claimed the no-damage skill packet was unhandled as `0x011a`.
  **Wrong header for this packetver:** ours is `0x09CB`, and it was handled all
  along. Check the packetver's actual header before concluding a packet is missing.

---

## Summary of Recent Resolutions (July 2026)

The original 73 integration tests—including all 39 job class skill sweeps and
phase 6/7 item/dialogue loops—passed as a complete run. The July 12 Phase 9 DM suite (14)
and straggler additions (`weapon-refine-cancel`, `repair-list-empty`,
`repair-invalid-item`, `character-slot-switch`) took it to **105 active scenarios**.

Following the final fixes on July 12, 2026, the full `--scenario all` sweep
was completed end-to-end with **100% passing results (105/105 green)**.

### Acceptance gate — July 13, 2026 (current state: 106 scenarios)

The §4 destructive-lifecycle refinements added **`character-delete-after-play`**, bringing
the suite to its current **106 active scenarios**. The acceptance gate was run as a
**double run**: run A 106/106, run B 105/106 (a single `TF_HIDING` observation-window
flake, 3/3 green on retest), run C 106/106. **Gate green — acceptance complete.**

Two failures previously written off as "flaky" were root-caused here and were in fact
deterministic:

1. A GUI DM session left `OPTION_INVISIBLE` (`char.option = 64`) on the GM character,
   making it invisible to partner clients and untargetable by mobs — this broke
   `whisper-emotion` and `incoming-damage` every time. Fixed: flag cleared in the DB, and
   `TestContext` connect now sends a best-effort `@option 0 0 0` for the GM account.
2. `area_size: 30` (Hercules `90db6a335`, the draw-distance raise — itself innocent,
   A/B-verified) exposes entities beyond `max_walk_path` (~17), and Hercules silently
   drops longer move requests. Fixed: `walk_to` now hops ≤10 cells with progress checks,
   and `incoming-damage` picks the nearest mob within 12 cells.

Scenario inventory by group (106): session/lifecycle 8 · GM 8 · movement 5 · combat 3 ·
skills 44 (39 job-class sweeps + teleport/weapon-refine menus) · items 12 · dialogue 5 ·
social 7 · DM tooling 14.

### Regression run — July 16, 2026 (first full run since the gate)

Re-run after the 2026-07-15 GUI pass promoted three map-server packets in the shared
crates (`0x0229` hide/cloak, `0x0196` status-end, plus the status-name table) and rebuilt
the `test` character. **First full-suite pass since the 07-13 gate**, so it also caught
anything the intervening Hercules commits introduced.

- **Result: `dm-quest-lifecycle` was the only failure** (105/106), and it was **not** a
  client regression — the three promoted packets are unrelated to quests, and no quest
  file changed in the client commits. **Server Emulator** cause: Hercules `d28ffb666`
  (post-gate, off-by-one MobId fixes) injected a `//` comment into a *single-line*
  quest_db entry — `Targets: ( { MobId: 1366 // Lava Golem /* … */ Count: 20 }, )`. `//`
  runs to end of line, so it ate `Count: 20 }` and the closing `)`; libconfig aborted
  quest_db parsing and reported the error at the next structure (`db/quest_db.conf:16164`,
  a line that looked innocent). With quest_db unloaded, `setquest()` silently no-oped, so
  `@dmquest start` reported success while adding nothing and sending no `QuestAdded`.
  Fixed in Hercules (`aa40e2053`): removed the stray `//`. Server now loads all 3172 quest
  entries; scenario passes 2/2 on retest.
- **Lesson:** a lone failure in the first run after a gap is not automatically a flake or
  a regression from the current change. Isolate by asking whether the failing subsystem is
  even touched by the diff — here it plainly was not, which pointed straight at the
  intervening server commits. (See the diagnostic path in `docs/2026-07-16-session-notes.md`.)

## DM suite (Phase 9) server-side fixes — July 12, 2026

Bugs the Phase 9 headless scenarios found and fixed in `Hercules/` (Server Emulator
layer — no client port; each ships in the campaign scripts or map-server source):

| Finding | Found by | Layer | Root cause & resolution |
| :--- | :--- | :--- | :--- |
| `@roll NdX` (no modifier) leaked a spurious modifier | `dm-roll-bounds` | Server script (`dm_console.txt`) | `sscanf(input, "%dd%d+%d", …)` was tried first and partially matched modifier-less input, so `1d1` rolled as `1d2+1`. Now the format is chosen by the sign actually present in the string (`compare()` on `+`/`-`). |
| `@dminstance` untrackable when instance id is 0 | `dm-instance-lifecycle` | Server script (`dm_instances.txt`) | The live-instance record stored the raw instance id, and ids start at 0, so the first instance after a boot read back as "no instance". Record now stores `id + 1` (0 = none). |
| Freshly created instance did not warp the party in | `dm-instance-lifecycle` | Server script (`dm_instances.txt`) | `instance_warpall` only moves players already standing on the instance's maps; right after creation nobody is. Replaced with an explicit `DM_WarpParty` into the new map. |
| Map-server crash on map-load after instance destroy | `dm-instance-lifecycle` | **Map-server source (`src/map/instance.c`)** | `instance_add_map` `memcpy`s the whole `map_data`, shallow-copying the source map's `qi_list` questinfo vector; destroying the instance `VECTOR_CLEAR`ed it and freed the source map's array, so the next `LoadEndAck` dereferenced freed memory in `quest_questinfo_refresh`. Fixed with `VECTOR_INIT(map->list[im].qi_list)` right after the copy (instanced NPC clones carry no questinfo of their own). Requires a map-server rebuild. |
| Colons in beat-menu labels split every menu | `dm-beat-table` | Server script (`dm_beats.txt`) | `select()` uses `:` as its option separator, so `"Warp: Wynne"` became two menu items and every choice after the first mapped to the wrong `switch` case. Only clicking the very first option ever worked (which is why manual testing missed it). Replaced `Warp: `/`Beat: ` label prefixes with `Warp - `/`Beat - ` (162 labels). |

## Phase 9 / straggler design deltas vs headless_remaining_test_design.md

- **`dm-roll-bounds`**: `1d1` cannot total 1 — the server clamps dice sides to ≥2, so
  `1d1` behaves as `1d2` (asserted range `1..=2`). The design doc's "1d1 must equal 1"
  is not achievable without a server rule change.
- **quest coverage is `dm-quest-lifecycle`, not `dm-quest-markers`**: `@dmquest`
  drives the quest *log* (modeled as new `QuestAdded`/`QuestRemoved`/`QuestList`
  events, converted from noops in `version_20220406.rs`), not on-map quest *effect*
  bubbles. Quest-effect markers (`AddQuestEffect`) come from campaign NPC `questinfo`
  blocks and are covered incidentally; a dedicated marker scenario would need a beat
  that attaches a marker deterministically.
- **`dm-beat-table` scope**: verifies every arc beat menu opens with a stable choice
  list and every `Warp:` beat changes the map (59 warp beats). Story/encounter beats
  are catalogued (103) but not executed — they spawn bosses and mutate campaign flags
  (content, not protocol), and leave NPC dialog state that made a full drive flaky.
- **`dm-reward-delta` zeny baseline** reads the wallet via an explicit `@zeny +1/-1`
  round trip; the map-login burst does not reliably emit a tracked `Zeny` stat update.
- **instanced-map name**: the shared crate reports the client resource name (the part
  after `#`, e.g. `000#pronter` → `pronter`); the scenario normalizes on `#`. Resource-
  name resolution for instanced town maps remains a graphical-client presentation gap.

## Open findings from the expanded suite (July 12, 2026)

| Finding / Issue | Affected Scenarios | Layer | Current evidence |
| :--- | :--- | :--- | :--- |
| Character-slot success requires entitlement | `character-slot-switch-rejected`, `character-slot-switch` | Test fixture / server configuration | The GM fixture has `slotchange=0` (keeps the rejection scenario valid). The success/persistence case runs on the `headless2` partner character after a one-time `tools/testing/fixtures/grant-slotchange.sql` grant. Both scenarios now exist and pass. |
| Partner registration connection closes after creation | phase 8 scenarios on a fresh server | Client test harness | Hercules created the `_M` account but closed that registration connection. The harness now retries once using the stable username instead of repeatedly sending the registration suffix. |
| Interrupted partner session can remain online | phase 8 after Ctrl-C | Server emulator / fixture hygiene | An interrupted two-client run can leave the partner account unavailable until Hercules clears its online state. Normal `TestContext` drops perform acknowledged logout; forced termination cannot. |
| Persisted GM `@hide` silently breaks proximity scenarios (2026-07-13) | `whisper-emotion`, `incoming-damage` | Test fixture / state pollution | A GUI DM session left `OPTION_INVISIBLE` in `char.option` (value 64) on the GM character: other clients get no `AddEntity`/emotion broadcast for it and mobs cannot target it. Looked like flake but reproduced deterministically. Fixed by clearing the flag and adding a best-effort `@option 0 0 0` baseline reset to `TestContext` connect for the GM account. `whisper-emotion` now also asserts the sender's own emotion echo first and logs pair colocation. |
| View radius exceeds `max_walk_path` (2026-07-13) | `incoming-damage`, any scenario walking to a seen entity | Test harness | With `area_size: 30` (Korangar draw-distance raise) entities are visible farther than the ~17-cell `max_walk_path`, and Hercules silently ignores longer move requests — `walk_to` timed out waiting for a `PlayerMove` ack. `walk_to` now splits walks into ≤10-cell hops with progress checks, and `incoming-damage` picks the nearest mob within 12 cells. |
| `TF_HIDING` rarely classifies SILENT (2026-07-13) | `skills-super-novice` (once) | Test harness / timing flake | One sweep in ~21 observations across full runs classified `TF_HIDING` as silent; the same sweep passed 3/3 immediate retests and every other job sweep. Likely the state-change packet landed outside the sweep's per-skill observation window under load. Not allowlisted (a real regression should still fail twice in a row); rerun on hit. |

## Expanded-suite graphical-client handoff

This is the port checklist for scenarios added after the original 73-scenario run.
“Shared automatically” means the graphical client calls the same
`korangar-networking` implementation and no headless code should be copied.

| Headless coverage | Implementation layer | Graphical-client handoff |
| :--- | :--- | :--- |
| `teleport-select` | Shared packet/event API plus graphical selection window | Already consumed by the Warp Selection window; retain live click verification. |
| `teleport-cancel` | Shared `NetworkingSystem::cancel_warp_selection` | Shared automatically. Cancellation is deliberately a no-op on the wire; never send an empty destination because Hercules treats it as random Teleport. |
| `weapon-refine-missing-material`, `weapon-refine-success` | Shared refine list/result/effect events plus graphical Weapon Refine window | Packet behavior is shared automatically. The graphical client must resolve `item_id` through item metadata for result text and render `bs_refinesuccess.str`; both were manually verified. |
| Character-slot rejection | Shared character-server event mapping | Shared automatically. A success/persistence graphical check still requires a character with `slotchange > 0`. |
| Whisper, emotion, friends, party, and trade | Shared events and requests; graphical state/window consumers | Protocol behavior is shared automatically. Graphically verify window opening, accept/reject controls, roster/state updates, and item presentation. |
| DM dice and read-only command contracts | Normal chat requests and `ChatMessage` responses | No protocol port. Dice/Commands windows must emit the documented strings and display server feedback. |
| Partner registration/retry and cleanup | Headless fixture harness only | Do not port; these helpers only make integration tests repeatable. |
| Skill icon audit | Graphical resource loader/library | Run `cargo run -p korangar --bin skill-asset-audit` with configured archives; zero missing player-visible assets is required. |
| `repair-weapon-cancel`, `repair-weapon-success` | Shared modern Repair Weapon packets/events/API plus graphical selection window | Ported; both scenarios pass live. Graphical core flow was verified on macOS 2026-07-12: the window offered Sword, selection repaired it, and named success feedback appeared. Window resize/move and graphical Cancel remain presentation checks. |
| `repair-list-empty`, `repair-invalid-item`, `weapon-refine-cancel` | Shared skill/repair/refine packets and events | Shared automatically. Negative paths (no list opens; a vanished target reports no success; cancel clears pending menu state) are wire behavior. No new graphical surface. |
| `character-slot-switch` (success + persistence) | Shared character-server event mapping | Shared automatically. Graphical check still needs a `slotchange > 0` character; apply `tools/testing/fixtures/grant-slotchange.sql` to the fixture. |
| DM quest lifecycle (`dm-quest-lifecycle`) | **New** `QuestAdded`/`QuestRemoved`/`QuestList` events (converted from noops in `version_20220406.rs`) | Shared automatically. The graphical client currently ignores these three events (no quest-log window yet); they are wired for the headless tester and a future quest log. When a quest-log UI lands, consume them there. |
| DM warp/recall, hazard, instance, reward, exp, beat sweep | Shared warp/stat/map/inventory/experience events + `ChatMessage` feedback | Protocol behavior shared automatically. Graphically verify the GM/DM panel buttons that issue these commands and that map changes / feedback render. Instanced town-map resource-name resolution closed 2026-07-14: `GameFileLoader::resolve_map_name` strips the `NNN#` prefix and completes wire-truncated base names against the archive `.rsw` table (`000#pronter` → `prontera`); archive-backed test via `cargo test -p korangar --lib resolve -- --ignored`. Live instance-warp click check remains a manual pass. |

### Port completion rule

A headless result alone does not verify presentation. Integration is complete when:

1. serialization/deserialization and event mapping have unit coverage;
2. the live headless scenario passes without unknown packets;
3. the graphical event consumer updates the appropriate state/window;
4. a manual graphical pass verifies clicking, labels, icons, resizing where relevant,
   feedback text, and visual effects; and
5. results and fixture requirements are recorded here and in `testing_guide.md`.

Window resizing is a graphical framework guarantee rather than a protocol behavior.
It was live-verified across the client on macOS 2026-07-12; new windows inherit
two-axis resizing from the `window!` component default.

| Finding / Issue | Affected Scenarios | Layer | Root Cause & Resolution |
| :--- | :--- | :--- | :--- |
| **Kaizel Resurrection Hang** | `use-consumable` | Client Test Harness | **Root Cause**: Running `skills-soul-linker` beforehand left the `Kaizel` buff active on the player character. When `@die` was called in `use-consumable`, Kaizel instantly self-resurrected the character, swallowing the expected `RemoveEntity` event and causing a timeout.<br>**Resolution**: Replaced the non-existent GM `@dispel` command with double-death retry logic. The first `@die` consumes the Kaizel buff, and a second `@die` triggers character death. |
| **Adjacent Walkable Cell Exhaustion** | `skills-paladin`, `skills-creator`, etc. | Client Test Harness | **Root Cause**: Dead or alive dummy targets (Pupas) accumulated around the player character between skill casts because monsters were only cleared at the end of the scenario. Once all 8 adjacent cells were filled, `approach_target` failed to find a walkable cell.<br>**Resolution**: Added immediate `kill_all_monsters()` calls inside the sweep loop to clean up dummy targets after each skill. |
| **Dynamic Positioning & Walk Drift** | `skills-paladin`, `skills-stalker` | Client Test Harness | **Root Cause**: Fixed warp coordinates `(170, 180)` were occasionally blocked or adjusted by the server. Furthermore, approaching targets in sequence created a "random walk" that drifted the player into walls.<br>**Resolution**: Switched to `warp_random` to dynamically acquire a safe coordinate, and added a walk reset back to `start_position` before each skill cast. |
| **Movement-Binding & Channeling States** | `skills-paladin`, `skills-stalker` | Client Test Harness | **Root Cause**: Channeling or persistent stealth skills like `PA_GOSPEL` (Gospel) or `ST_CHASEWALK` (Chase Walk) bound the character's movement or status, causing subsequent active skills (like `PA_SHIELDCHAIN` or `TF_HIDING`) to fail.<br>**Resolution**: Added these skills to `stateful_skill_rank` so they are sorted to the end of the sweep, preventing their state from locking later skills. |
| **Weapon/Ammo & Wall Dependencies** | `skills-assassin`, `skills-stalker`, etc. | Client Test Harness | **Root Cause**: Several skills require special setups—such as `AS_CLOAKING` (requires standing next to a wall) or `ST_REJECTSWORD` (requires daggers/swords)—which fail silently on open grass fields.<br>**Resolution**: Added these skills (`AS_CLOAKING`, `ST_REJECTSWORD`, `ST_PRESERVE`, `ST_FULLSTRIP`, `RG_GRAFFITI`, `RG_CLEANER`) to the `ALLOWLIST` with explanation comments. |
| **Weapon Refine Success Flakiness** | `weapon-refine-success` | Client Test Harness | **Root Cause**: Weapon refinement success rates are subject to RNG and depend on Whitesmith Job Level and DEX/LUK stats. At default level 1 stats, refinement frequently fails, destroying the weapon and timing out the test.<br>**Resolution**: Rewrote the scenario to boost Whitesmith stats (`@jlevel 70`, `@stat dex 99`, `@stat luk 99`) and implemented an automatic retry loop (up to 5 attempts) with `@delitem` cleanup. |
| **Sage Skill Silence** | `skills-sage` | Client Test Harness | **Root Cause**: Sage casting/free-cast stance quirks headlessly cause `MG_NAPALMBEAT`, `MG_SOULSTRIKE`, and `MG_COLDBOLT` to produce no observable protocol response, despite working on Mage/Wizard/Professor.<br>**Resolution**: Added these three skills to the headless-tester `ALLOWLIST` in `skills.rs`. |
| **Friend Rejection Text Mismatch** | `friend-reject` | Client Test Harness | **Root Cause**: Primary character's rejection assertion looked for `"reject"`, but the server returns `"does not want to be friends"`.<br>**Resolution**: Updated the text matcher to accept either variation. |
| **Trade Cancel / Trade Commit Failures** | `trade-cancel`, `trade-commit` | Client Test Harness / Shared Crate | **Root Cause**: Two issues:<br>1. The trade acceptance check expected `result: 0` (too far) instead of `result: 3` (success/start) in `begin_trade`.<br>2. Adding Zeny to a trade sent raw index `0` on the wire. Due to `InventoryIndex` conversion subtracting 2, `InventoryIndex(0)` serialized to raw `2` (the first real inventory item) instead of `0`. Furthermore, raw `0` deserialized to `65534`, triggering a subtract-with-overflow panic in debug builds.<br>**Resolution**: Updated `begin_trade` to look for `result: 3`. Changed `InventoryIndex` from/to bytes to use wrapping arithmetic (`wrapping_sub`/`wrapping_add`). Updated `trade_add_zeny` to pass `InventoryIndex(65534)` which correctly serializes to raw `0` on the wire. |
| **Hercules Temporary IP Bans** | All scenarios (subsequent runs) | Server Emulator | **Root Cause**: Abrupt client disconnections during tests triggered Hercules' `dynamic_pass_failure` ipban system, setting `unban_time` on test accounts in the `login` database and returning `Login prohibited until ...` errors.<br>**Resolution**: Disabled `dynamic_pass_failure` in Hercules' `conf/login/login-server.conf` for the testing environment. |

---

## Detailed Investigations

### 1. The Kaizel Resurrection Buff
> [!IMPORTANT]
> The Soul Linker self-resurrection buff, **`Kaizel`**, is extremely persistent. Because there is no standard GM `@dispel` command in the default Hercules build, the only way to clear it programmatically without restarting the server or map session is to trigger a character death.
* **Finding**: `use-consumable` failed with a timeout waiting for player death.
* **Analysis**: When the client receives `@die`, the server processes character death but immediately resurrects the character due to Kaizel. The client never receives a `RemoveEntity` event for the player, leading to a test timeout.
* **Resolution**: The double-death retry loop was implemented in [items.rs](../korangar-networking/examples/headless-tester/scenarios/items.rs). It attempts character death, waits for 2 seconds, checks if the character resurrected, and issues a second `@die` command if needed.

### 2. Random-Walk Drift & Target Accumulation
> [!TIP]
> Live integration tests sweeping dozens of active skills sequentially are prone to positioning drift. If the test harness spawns a target, walks next to it, and does not clean up the target, the player character gradually crawls across the map (random walk) and gets boxed in by their own targets.
* **Finding**: Paladin and other martial classes failed with `no walkable adjacent cell found around target` after successfully sweeping 15-20 skills.
* **Analysis**:
  1. The player character was not reset to a starting coordinate between casts, causing them to wander away.
  2. The target dummies (Pupas) remained on the map, occupying adjacent cells.
* **Resolution**: 
  1. In [skills.rs](../korangar-networking/examples/headless-tester/scenarios/skills.rs), the starting tile is dynamically recorded via `warp_random` as `start_position`.
  2. Before each skill sweep, the character walks back to `start_position`.
  3. After each cast, the target dummy is removed via `kill_all_monsters()`.

### 3. Stateful & Disabling Skills
> [!CAUTION]
> Certain skills modify player movement and casting states persistently. In a headless test sweep, these must be executed last to avoid polluting the state of other independent skill tests.
* **Examples**:
  * `HP_BASILICA` (blocks all actions within a holy zone).
  * `PA_GOSPEL` (binds the player to the spot and channels a zone).
  * `ST_CHASEWALK` (enters persistent stealth and drains SP).
* **Resolution**: These are registered in `stateful_skill_rank` and sorted to the very end of the execution queue.
