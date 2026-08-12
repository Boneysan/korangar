# Headless testing — prioritized next steps

**Audience:** humans and coding agents (Claude, Codex, Cursor, etc.).
**Read this when:** improving the headless suite, deciding what test to write
next, or about to claim the suite is "complete".

| | |
|---|---|
| **Status** | **Headless suite acceptance CLOSED (2026-08-12)** — full **148 pass / 1 skip / 0 fail** over **149** registered, ~59 min; 0 flaky, 0 retried, 0 unknown packets; expectations enforced with **0 unmet / 0 exemptions**. HEAD `7d38d12e`+ |
| **Resume first** | **GUI / client**, not more suite grinding — [gui-verification-pass.md](../../docs/plans/gui-verification-pass.md) (Block E Hermode, N20 Auto Spell). Optional re-shuffle after Desert Wolf harden |
| **Canonical plan** | [headless_test_plan.md](headless_test_plan.md) |
| **Meaning of green** | [../../docs/plans/testing-completeness.md](../../docs/plans/testing-completeness.md) |
| **Findings log** | [headless_findings.md](headless_findings.md) |
| **GUI live queue** | [../../docs/plans/gui-verification-pass.md](../../docs/plans/gui-verification-pass.md) — separate from headless; open rows at top of that file |
| **Draft PR** | https://github.com/Boneysan/korangar/pull/2 |
| **Historical handoffs** | [2026-08-11-testing-handoff.md](2026-08-11-testing-handoff.md), [ci-cleanup](2026-08-11-ci-cleanup-handoff.md), [p0-p4-pause](2026-08-11-p0-p4-pause-handoff.md) |

> [!IMPORTANT]
> **A green headless run means the wire protocol and event mapping work.**
> It does **not** mean skills have correct gameplay effects, that the graphical
> client draws anything, or that pathfinding works. Never report headless-green
> as "verified working in the client".

---

## 1. Current baseline (do not regress)

| Gate | Contract |
|---|---|
| Scenario outcomes | Failures and unexpected skips fail the run |
| Flaky recovery | Recovered connection retry = `FLAKY-PASS`; fails under `--fail-on-flaky` |
| Packet deserialization | Any failure fails the run |
| Unknown packets | Any header not in the (empty) reviewed baseline fails the run |
| Skill expectations | Non-exempt `Unmet` rows fail the run (`EXPECTATION_EXEMPTIONS` is **empty**) |
| Expected skips | Exact name + reason only (`skills-novice` today) |
| Archives | Complete full → `runs/*.log`; targeted → `*.scoped`; interrupted → `*.partial` |

**Registered scenarios:** **149**, counted from a real run's `headless-results.json`
on 2026-08-12 rather than incremented by hand — the hand-maintained figures had
drifted to 139/136/135 across `CLAUDE.md`, `testing_guide.md` and `docs/README.md`
while the tree grew by thirteen. Count it, do not carry it forward.

### Verified live baseline

| Run | Archive / note | Result |
|---|---|---|
| **Full on `3f704c26`+ (two-cycle hotkeys, kick confirmation)** | `runs/20260812-104814.log` | **148 pass** of 149, 1 expected-skip, 0 fail, **0 flaky / 0 retried**, **0 unmet / 0 exemptions**, 172 in / 66 out / **0 unknown**; ~59 min |
| Full on `4e14101c` (post walk harden) | `runs/20260811-210533.log` | **147 pass**, 1 expected-skip, 0 fail, **0 unmet / 0 exemptions**, 173 in / 66 out / **0 unknown** |
| Shuffle `20260810` on same HEAD | `runs/20260811-220535.log` | **146 pass**, **1 fail** (`incoming-damage` Desert Wolf hard walk — fixed in `e4d6e6d5`); 1 expected-skip, **0 unmet**, 174 in / 66 out / **0 unknown** |
| Scoped `incoming-damage` after `e4d6e6d5` | `runs/20260811-230914.scoped` | **PASS** |
| Prior full (empty exemptions + golden 1–10 + quest-log-multi) | `runs/20260811-190011.log` | 147 pass; 171 in / 66 out |

Load-bearing silence allowlist (do not cull from one job’s answer alone):
`HT_REMOVETRAP` (Rogue/Stalker), `HT_SPRINGTRAP`, `TK_MISSION`.

**Integration DB for local macOS:** TCP admin `korangar_int` / `korangar_int`
(Homebrew `root` is often socket-only). Fixture emails are `a@a.com`. Char
import sets `deletion.delay: 0`.

```sh
HERCULES_DIR=../Hercules \
INTEGRATION_DB_ADMIN=korangar_int \
INTEGRATION_DB_ADMIN_PASSWORD=korangar_int \
INTEGRATION_SKIP_BUILD=1 \
  tools/testing/run-integration-tests.sh

# Shuffle:
# ... run-integration-tests.sh --shuffle 20260810

# CI-like multi-scenario slice:
# ... run-integration-tests.sh --scenario \
#   "smoke,connection-state,character-select-invalid,dm-quest-lifecycle,quest-log-multi,dm-golden-beats,skills-mage,trade-reject,party-lifecycle"
```

Fast checks (no live server):

```sh
cargo test -p ragnarok-packets
cargo test -p korangar-networking --example headless-tester
tools/testing/test-run-suite.sh
HERCULES_DIR=../Hercules tools/audits/generated-drift.sh
```

---

## 1a. Meaning review, 2026-08-12 — what a green run did *not* cover

A read-through of the suite and the audit layer after the acceptance close.
Five findings, all fixed on the same day. The suite itself held up well; three
of the five were in the layer *around* it, which is where they have tended to
be.

| # | Finding | Fix |
|---|---|---|
| 1 | **`observer-parity.sh` was RED and nobody knew** — three unclassified `register_noop` findings from the typed-no-op batch had been sitting there. Two turned out to be dropped features whose stated reason did not survive checking (below) | Classified in the baseline; both gaps filed as `OPEN:` |
| 2 | **None of the five standing audits ran in CI**, while `tools/audits/README.md` said "in CI, on every commit". That is what let #1 sit | New [`audits.yml`](../../.github/workflows/audits.yml): all five on every push and PR, with the pinned Hercules checked out so none self-skips |
| 3 | **The `hotkeys` scenario asserted nothing** — connect, send, pump, `Ok(())`, under a comment claiming it "verified" the hotkey. Only a dropped connection could redden it, and `ACTION_COVERAGE` pointed `set_hotkey_data` at it as covered | Rewritten as a real round trip: rotating probe → relog → assert the slot out of `ZC_SHORTCUT_KEY_LIST` |
| 4 | **`ACTION_COVERAGE` was a one-way gate.** It checked that every row names a real scenario, never that every real action has a row — so a new `pub fn` on `NetworkingSystem` could land untested with the suite green | `every_public_action_has_a_coverage_row` + `no_coverage_row_names_a_vanished_action`, both verified failing |
| 5 | **A job sweep that cast nothing would have passed.** The gate is "no skill was silent"; zero casts satisfies it. `@allskill` is best-effort and the tree wait only counts *entries*, passives included | `sweep_job` now fails on zero casts and prints `N cast, M passive` per job, so a per-job floor is derivable from the archives |

**The two dropped features found behind #1** — both are `register_noop` with a
rationale that reads fine and is not true:

- **`GmKickResponsePacket` (0x00CD)** is the *only* confirmation the kicking GM
  gets. `ACMD(kick)` prints nothing on success and `clif_GM_kick`'s whole
  feedback path is `clif->GM_kickack(sd, 1)`. So a DM types `@kick`, the target
  vanishes, and the client says nothing — and a *failed* kick is
  indistinguishable from a lost command. The registration comment says "the
  kicked target owns the disconnect flow", which is true of 0x0081 and is the
  other half of the interaction. DM tooling is this fork's stated priority.
- **`TalkieBoxMessagePacket` (0x0191)** is noop'd because the text is
  "rendered at the trap rather than in a chat window". Nothing in the tree
  renders it anywhere: the only Talkie Box reference in `korangar/src` is the
  trap's own prop model. The prop draws; the message it exists to carry does
  not.

Same shape as `BD_ETERNALCHAOS` sitting in the skill allowlist for months under
a false reason. **Check the claim in a rationale; do not inherit it.**

Also corrected: three doc comments had collapsed onto one function in
`skills.rs` through refactoring, and the surviving one still said the
expectation verdicts "never fail" — a day after the gate went in. A stale
comment on a gate is how the next session decides it may add exemptions.

## 2. What shipped (2026-08-11)

1. Expectation **enforcement**; `EXPECTATION_EXEMPTIONS` emptied (FeelRequest, partner Spellbreaker, Cleaner refuse)
2. Negatives, session, channeling, multi-scenario CLI
3. `dm-golden-beats` arcs **1–10** with `A0N:` progress token assertions on start beats
4. `dm-quest-lifecycle` + **`quest-log-multi`** (two quests, QuestList after relog)
5. Integration fixture + multiline Hercules import fixes for CI
6. Generators rustfmt output; local fmt/clippy/all-features Clippy green
7. PR integration workflow: pull_request runs a **short multi-scenario list**, not smoke-only; schedule/manual still `all`
8. Draft PR #2 title/description refreshed; commit pushed

---

## 3. Priority status

### P0 — Full live validation

| Item | Status |
|---|---|
| Full + shuffle (pre-P1 empty) | **Done** green |
| Full after empty exemptions + golden 1–10 + quest-log-multi | **Done** green — `runs/20260811-190011.log` |
| Full after walk harden (`4e14101c`) | **Done** green — `runs/20260811-210533.log` |
| Shuffle after same | **Done** with 1 intermittent fail: natural-mob one-shot → Desert Wolf hard `walk_to` (now shared multi-cell + warp helper); scoped green |

### P1 — Expectation exemptions

| Skill | Status |
|---|---|
| All residual named exemptions | **Closed** — list empty |

### P2 — Negatives | **Done**

### P3 — Flaky

Allowlist job-split + `KNOWN_INTERMITTENT` documented. Do not cull allowlist from one job.

### P4 — Campaign golden beats

Arcs **1–10** with status token + non-zero progress on start beats. Further arcs only content-reviewed.

### P5 — Outside headless

| Item | Status |
|---|---|
| Ice Wall walkability | Done (unit + headless) |
| Spirit sphere state | Done |
| Spirit sphere render / quest UI / GUI | **Open** (client) |

### Homunculus / pet / cart / guild

**Deferred.** No meaningful headless client surface for merchant cart, homun, guild hall, or pet taming beyond incidental shop pet-food. Add scenarios only when those systems are implemented in the client.

### P6 — CI

| Item | Status |
|---|---|
| Local fmt/clippy/drift/tests | **Done** |
| Commit + push + PR #2 refresh | **Done** |
| PR multi-scenario gate (not smoke-only) | **Done** in workflow |
| GitHub Actions green on latest HEAD | Confirm in PR checks when convenient |

### P7 — Process rules

Zero-unknown, exact expected-skips, allowlist cull discipline, archive semantics, never headless-green = client verified.

### Explicit non-goals (do not grind)

- Cull silence allowlist (`HT_SPRINGTRAP`, `TK_MISSION`, …) without better setup  
- Expand skill sweep to every residual “cast only” skill  
- Treat duration jitter as failures  

---

## 4. Suggested work order

```text
Headless suite: CLOSED for planned depth — do not re-open without a product need
  → GUI live pass: docs/plans/gui-verification-pass.md (open rows at top)
  → Watch PR #2 multi-scenario CI
  → Homun/pet/guild/cart headless scenarios only when client implements them
```

---

## 5. File map

| Path | Role |
|---|---|
| `examples/headless-tester/` | Suite |
| `…/scenarios/skills.rs` | Sweep, allowlist, empty exemptions |
| `…/scenarios/dm.rs` | DM + golden + quest-log-multi |
| `tools/testing/run-integration-tests.sh` | Disposable DB + Hercules |
| `.github/workflows/integration.yml` | PR multi-scenario vs schedule `all` |
| `tools/audits/flaky.py` | Cross-run inconsistency |

---

## 6. When this document is stale

Update when scenario count changes, full-suite acceptance is reconfirmed, CI
scope changes, or exemption/allowlist policy changes.
