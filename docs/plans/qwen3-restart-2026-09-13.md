# Qwen3 restart baseline — 2026-09-13

This is the senior-developer reconciliation of Qwen3's first pass through the
September playtest runbook. It distinguishes code that exists from work that
actually satisfies the runbook's completion gate. The detailed, original task
records remain in [`qwen3-playtest-runbook.md`](qwen3-playtest-runbook.md).

## Resume point

**Start with QW-001.** Finish its forged equipped-sale regression matrix, then
close QW-006's real graphical/audio and source-route gaps. Skip QW-011, QW-014,
and QW-020 only after preserving their blockers, finish the partially written
QW-023 scenario, and continue with QW-024.

Do not discard the current worktree. None of this work has been committed, and
the modified files are the implementation under review.

## Reconciled totals

The runbook contains 82 task cards.

| Senior status | Count | Task IDs |
|---|---:|---|
| Done | 11 | QW-000, QW-002–005, QW-010, QW-012–013, QW-015, QW-021–022 |
| In progress | 3 | QW-001, QW-006, QW-023 |
| Blocked | 3 | QW-011, QW-014, QW-020 |
| Not started | 65 | QW-024–027, QW-030–039, QW-040–049, QW-050–063, QW-070–079, QW-080–090, QW-100–105 |

Qwen3 had self-reported 15 cards as done. Four of those reports did not satisfy
their own `Done when` gates: QW-001, QW-006, QW-011, and QW-014. QW-023 was
started after the last runbook write but was never recorded.

## What is complete

| Task | Accepted result | Main evidence |
|---|---|---|
| QW-000 | Removed unproved Orb/Water Ball mappings while retaining Increase AGI | Focused effect tests and Korangar check |
| QW-002 | Extracted and tested sell-list filtering, including stackable ammo quantities | Six focused networking tests plus library suite |
| QW-003 | Correct client sale-completion state and exactly-once ordinary sale behavior | Two state tests and live `shop-buy-sell` scenario |
| QW-004 | Removed the unused `CharacterSlots::class_name` accessor | Both strict Clippy configurations passed at the recorded point |
| QW-005 | Added Increase AGI packet, recipe, loader, target, audio, Heal-isolation, and archive tests | Focused tests, library suite, and ignored archive-backed audit |
| QW-010 | Compared the exact local/synced release inputs and did not reproduce corrupt verifier bytes | Recorded sizes, SHA-256 values, shared-file comparison, and merged verification |
| QW-012 | Added package-half provenance and single-manifest verifier ownership | Seven fixture cases passed |
| QW-013 | Added atomic small-file repair bundles without touching valid GRFs | Repair and verifier fixture suites passed |
| QW-015 | Built and locally verified a Windows hotfix candidate and upgrade path | `make-pack.sh`, full manifest verification, and simulated upgrade passed; upload correctly withheld |
| QW-021 | Proved the Blue Potion trade path healthy three times with a normal-item control | Two-client headless scenario; overweight remains the likely explanation for the original report, not a reproduced cause |
| QW-022 | Ruled out a client pickup request after drop and tied repickup to Hercules auto-pickup | Three packet-evidenced headless trials |

## Work that exists but is not complete

### QW-001 — authoritative equipped-sale rejection

The Hercules guard exists in `src/map/npc.c`, before the script-controlled shop
branch, and `make -j4` passed. The required direct forged/stale request tests for
equipped weapon, armor, costume, and ammo do not exist. QW-003 covers only a
normal unequipped sale and stale resubmission.

**Next concrete action:** extend the headless selling scenario or add a focused
server harness that directly submits each equipped inventory index, then assert
zero inventory and Zeny deltas for every rejection plus one successful control.

### QW-006 — Increase AGI live presentation

The two-client scenario proves packet routes, and tests prove recipe selection,
assets, loader, anchoring data, single-spawn configuration, and Heal isolation.
It does not prove visible pixels, audible playback, or attachment while moving:
a headless client cannot observe those boundaries. The evidence also conflates
skill-bar and skill-window activation and does not include an actual monster or
NPC source.

**Next concrete action:** run a graphical client with packet logging, capture
the distinct activation/source routes, record at least one rendered frame and
one audio observation per distinct presentation path, and retain the existing
headless route assertions as protocol evidence.

### QW-023 — sitting regeneration thresholds

Qwen3 added `sitting-regeneration-thresholds` to
`korangar-networking/examples/headless-tester/scenarios/movement.rs`. The example
compiles, but the scenario has not been run and its result document is still an
old blank template with unsupported placeholder values. The code currently
hard-codes timing and threshold expectations that still need to be tied to the
active Hercules config/source before the live run.

**Next concrete action:** fix the EOF whitespace, source-confirm the active
threshold/timer branches, update the scenario to derive or cite those values,
run it against the local server, and replace the template with exact observed
before/after values from at least three ticks per required case.

## Explicit blockers

| Task | Useful work already present | Missing external input |
|---|---|---|
| QW-011 | Archive, synced-drive, merge, CRLF, and manifest-ownership analysis | Native Windows browser download, mark-of-the-web, and antivirus-stage byte captures |
| QW-014 | Automated eight-case installation matrix passes under PowerShell 7 on macOS | Native Windows execution of all eight cases |
| QW-020 | Repository/history search and protocol template audit | Actual skill names reported as failing in the friends playtest |

These blockers do not prevent QW-023 and later independent tasks from
continuing. QW-011 and QW-014 must be revisited at the Windows release gates.

## Worktree snapshot

At reconciliation time:

- Korangar branch `agent/bump-hercules-pin`, base commit `4f9131b4`: 19 tracked
  files modified plus new plan, reproduction, repair, and packaging-test files;
  tracked diff was 2,478 insertions and 166 deletions.
- Hercules branch `agent/map-teleport-safety`, base commit `6067ed3b`: tracked
  changes in `src/map/clif.c` and `src/map/npc.c`; four untracked live log files.
- `cargo check -p korangar-networking --example headless-tester` passed with an
  `ACTION_COVERAGE` dead-code warning.
- `git diff --check` passes in Hercules and currently fails in Korangar at
  `korangar-networking/examples/headless-tester/scenarios/movement.rs:516`
  because of a new blank line at EOF.

The earlier passing-command lists are historical evidence from Qwen3's task
records, not a fresh full-suite validation of the combined final worktree. Run
the appropriate validation ladder again after closing QW-001, QW-006, and
QW-023.

## Restart rule

For each card, keep the checkbox unchecked until every `Done when` statement is
supported by source, automated, or genuinely observed evidence. A headless
packet result must never be relabeled as a rendered visual/audio observation.
After recording one card, update `NEXT` and begin the next runnable card in the
same session. Preserve blockers and continue independent work; do not stop at a
stage checkpoint or progress summary.
