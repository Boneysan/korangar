# Test-intent audit — 2026-10-02

**Question:** does every GDD slice built so far have automated tests, and do those
tests check what the slice is *for* (not only that it compiles or repeats its own code)?

**Answer: no, not every slice, and not every behaviour.** This lists what is tested,
how well, what I fixed in this audit, and what no automated test can cover.

Evidence labels: **Intent-tested** = a named test fails if the behaviour regresses;
**Partly** = the core rule is tested but part of the claim is not; **Not tested** = no
automated test; **Live only** = needs a person at the client or a real fight.
I read the *bodies* of a sample of tests (chat link defanging, party sender
authentication, HUD combat fade, ground-loot filter) and they assert real intent; for
the rest I judged from test names and the code they exercise, so treat those verdicts
as "looks right", not "proven".

## Client slices

| Slice | Intent-tested | Not covered |
|---|---|---|
| **F20** shared marks / goals | `state/party.rs`: authenticated sender, versioned and bounded messages, per-sender markers, expiry, owner-only clear, goal caps, cleared on leave | The world label/outline on a marked monster, `/mark` and `/goal` through the real server, the HUD "Tonight" block — **live only** |
| **F22** party frames | roster, distance label, healer layout, leave cleanup | The click handlers (`JumpToPartyMember`, `OpenPartyMemberTarget`, `NavigateToPartyMember` in `lib.rs`) run inside the whole app — **live only**. *The plan row named `PartyFrameClick::…`; that identifier never existed. Corrected.* |
| **F23** chat | tab filtering, timestamps, link forging/defanging (body read: rejects fake ids, over-refine, over-slots, NAVI in chat) | Unread indicators |
| **F24** hotbar / sets | key binding round trip, reserved/conflicting chords rejected, input dispatch, sets persist per character. **New:** `state/equipment_plan.rs` (8 tests) | Hotbar row visibility — live only |
| **F25** HUD profiles | every profile yields a valid layout; combat fade and restore (body read) | How it looks — live only |
| **F26** loot | filter + wishlist never hide cards/wishlisted items (body read); settings round trip. **New:** weight thresholds (4 tests) | The drop-alert cue/toast/chat-tab firing on a ground drop (inline in `lib.rs`); Guide card↔monster links not re-verified here |
| **F27** refine reference | Guide tests for odds pages; exporter `--check` | Seen on screen — live only |
| **F32** effect density | `effect_density_never_thins_your_own_or_hostile_visuals` — real behaviour | "Telegraphs ignore density" is a **source-text** test (`cast_telegraph_pass_ignores_…`): it fails if the words appear in the function, not if behaviour changes. Weak guard. |
| **F35** audio cues | rate limiter, tick wrap, distinct wavs. **New:** the dangerous-cast rule (3 tests) | The other four triggers (interrupt, quest, ping, card) are inline in `lib.rs`; wav files existing in the GRF was checked by hand once, no test; a machine with no audio device — live only |
| **F28** trade | 14 state tests for the guard, the zeny fix and the final summary, each mutation-checked; `trade-partner-change-after-lock` pins the server facts it relies on | Item-grid drag not built; the window and a real two-client transfer — live only. *This slice's first test run found a real display bug (partner zeny shown as an item), which the older two trade tests could not have.* |
| **F37** provenance | Guide tests per new rule and for the revision line; `check_reference_drift.py` run | The drift tool itself has no unit tests (it was proven by running it) |
| Today: recovery HUD, DMJ channel, quest outlines, chests, clues | unit tests for each + headless scenarios `recovery-state-packet`, `dm-dmj-echo`, `dm-flag-channel` (each with a negative control) | The windows on screen — live only |

## Server and campaign slices

| Slice | Intent-tested | Not covered |
|---|---|---|
| **F14** monster AI profiles | `check_mob_ai_profiles.py` validates the file against the loader's real bounds and, in `check-campaign.sh`, requires the server's own boot line. **New:** 12 Python tests (every case that was first proven by hand, plus a mutation check). `check_mob_skill_families.py` | **Behaviour** (does an Orc Skeleton actually step back after three hits?) — needs a fight; **live only**. `mob.c` has no unit harness. |
| **F15** Eddga + elites | config validity; boot log | Spawn count of the elite, the fight, solo vs party — **live only**. No scenario exercises `f15_elites.txt`. |
| **F17 / F19** campaign Acts II–IV | `dm-beat-table`, `dm-story-beats`, `dm-golden-beats` sweep every beat menu; load check, walk audit, cell audit | **The finale's logic** (readiness matrix, once-only commit, volunteer-must-be-present) has **no automated behavioural test**. The runbook (tests A–J) is written and has **never been run**. This is the largest gap in the campaign. |
| **F39** party quest credit | `party-quest-interaction-credit` (passes in the full suite) | Disconnect and dead-member cases; no real quest uses it yet |
| **S10** party sync | `dm-party-offline-replay`, `-alternate-character`, `-offline-transitions`, `-reward-isolation`, `-recreation-isolation` + two end-of-run SQL audits | Concurrent multi-character play, failed-replay repair |
| **F36** GM metrics (today) | `gm-metrics` scenario (opt-in default off, counts, `off` stops counting — negative control, baseline diff, labels, reports); `drop_sim.py` 18 tests incl. refusal on a non-stock setting | The GM-only gate (no non-GM test account exists), pathing/stuck loops and UI usage (not built) |

## Mob behaviour (F14 / F15) — deepened

Before: config validators and a boot-log line only; nothing watched a monster behave. Now
`scenarios/mobs.rs` (phase 10) drives real monsters on the real server and judges what they
do, each against a **control** (same monster, same attacks, on a map with no profile):

| Scenario | What it proves | Control |
|---|---|---|
| `mob-skirmisher-orc-skeleton` | After 3 hits the skeleton steps one cell away, never before the 3rd hit, no two steps within the 3 s cooldown, no more steps than hits allow | Same fight on `prt_fild08`: zero steps |
| `mob-skirmisher-elite` | The elite uses *its own* numbers (2 hits, 4 s cooldown) | Same, no profile: zero steps |
| `mob-ranged-keeper-orc-archer` | The archer backs off a cell per 1.5 s until it is about 5 cells away | Stock archer on `prt_fild08`: stays |
| `mob-coward-poring` | A Poring flees only once it is down to 35% (never before 33 of 50 damage) | Same Poring on `prt_fild07`: never flees |
| `mob-eddga-pilot-skills` | Eddga casts Magnum (cast bar ~2 s, 15 s apart) and never Teleport | — |
| `mob-elite-population` | `f15_elites.txt` keeps exactly 2 elites, and holds at 2 across a further timer tick | — |
| `mob-elite-rollback-switch` | `mob_pilot_version 0` removes the elites and none respawn; back to 1 they return | Both directions |

Things the tests taught (none was visible from config alone):
- The elite is spawned by a **60 s timer**, so it does not exist for the first minute after boot.
- The skirmisher step needs an open cell farther from the player; in a corridor it falls
  through to stock AI **by design**, so the profile cases retry at up to three spots.
- The elite has 90 DEF; an unarmed character deals zero to it, so the test gives its attacker strength.
- Still **not** tested: `AvoidHazards` pathing, Raydric profiles, Eddga's Meteor (needs 20% damage first).

## The finale (F19) — deepened

`scenarios/finale.rs` drives the real NPCs through real dialogue with a policy-based
driver and decides every outcome from the server's own flags:
readiness matrix (bare and generous worlds, exact statuses), preparation bits 1/3/7/15/31,
declining every group leaves the Shared Seal at "0 so far", seven missing-prerequisite
cases (including the fallen Queen and a contradictory record), a full commit for **each of
the five endings** (cost shown before the question, "wait" commits nothing, exactly one
`dm_finale_*` flag, the campaign completes, 1,000,000 EXP **once**, a repeat visit changes
and grants nothing), two conversations racing for different endings, a volunteer who is
offline (the choice pauses, nothing is consumed, then it commits) and Keeper Lysandra as
volunteer, resume after both characters relog, and Loki's briefing.
**Writing the first of these found a real bug:** `@dmpreset` aborted with "run_script:
infinity loop !" (instruction limit) and answered nothing; it needed `freeloop`.

## The encyclopedia

| Layer | State |
|---|---|
| Review loop | About 2,340 units done; E0 (evidence-label rules) passes; E7 (the live client pass) blocked; one stronger-model audit due |
| Citation check | **29 reviews quote text that is not on their cited lines.** Diagnosed: not wrong quotes, but drifted line numbers. Only 1 of 32 drifted sources could be re-anchored provably (fixed). The rest moved unevenly (e.g. the Moonlight Flower header is cited 127 lines from where it is, while its body quotes are 4–23 lines the other way), so a machine fix could silently cite a different NPC's lines. **They need a person.** |
| Ratchet | New `test_encyclopedia_citations.py`: no *new* broken citation can enter, and a fixed one must be removed from the baseline. Mutation-checked. |
| Drift | `check_reference_drift.py`: 17 of 30 exporters are stale against today's server (the rebuilt arcs, new scripts, quest and monster data). Re-export after the server work is committed. |
| Guide data | 50 Guide tests + reference-data consistency checks; E0 asserts no runtime clue is labelled reviewed |
| Not covered | That an explanation is *correct*: only the verifier sessions and audits address that; and the Guide on screen (E7) |

## Fixed in this audit

1. **A real bug in F24.** `equip_named_set` chose the first unworn copy and tested "already worn" separately for each id, so a set with **two identical items** (a pair of rings) queued the same copy twice and never the second, or called both satisfied by one worn copy. The decision now lives in `plan_equipment_set`, which counts copies, with tests for exactly those cases. (Source-confirmed from the old loop; the old code was not run.)
2. **No tests for the weight rule** the F26 row said was tested. Extracted `is_soft_overweight` / `is_hard_overweight` and pinned both boundaries.
3. **The dangerous-cast cue's rule was buried in a 100-line event handler.** Extracted `is_dangerous_cast` and tested its edges.
4. **The F14 safeguard was only ever proven by hand.** Now 12 automated tests.
5. **F22's plan row cited identifiers that do not exist.** Corrected.

## What no automated test can settle

A person at the client has to look at: the recovery line, quest-log routes/outlines/clues,
Guide chest and rule pages, party markers and frames, audio cues, effect density,
combat fading, the HUD profiles. And someone has to play: Eddga, the elite orc, the
finale. Until then these are "automated tests pass", not "works".
