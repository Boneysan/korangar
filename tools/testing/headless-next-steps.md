# Headless testing — prioritized next steps

**Audience:** humans and coding agents (Claude, Codex, Cursor, etc.).
**Read this when:** improving the headless suite, deciding what test to write
next, or about to claim the suite is "complete".

| | |
|---|---|
| **Status** | **Acceptance on `4e14101c`+** — full **147/1/0** green; shuffle 146/1 with intermittent `incoming-damage` (Desert Wolf path hardened; scoped green) |
| **Resume first** | P5 client work only if product asks; optional re-shuffle after Desert Wolf approach harden |
| **Canonical plan** | [headless_test_plan.md](headless_test_plan.md) |
| **Latest ship handoff** | [2026-08-11-testing-handoff.md](2026-08-11-testing-handoff.md) |
| **CI cleanup handoff** | [2026-08-11-ci-cleanup-handoff.md](2026-08-11-ci-cleanup-handoff.md) (historical; local gates + push done) |
| **P0 pause handoff** | [2026-08-11-p0-p4-pause-handoff.md](2026-08-11-p0-p4-pause-handoff.md) (historical) |
| **Meaning of green** | [../../docs/plans/testing-completeness.md](../../docs/plans/testing-completeness.md) |
| **Findings log** | [headless_findings.md](headless_findings.md) |
| **Draft PR** | https://github.com/Boneysan/korangar/pull/2 |

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

**Registered scenarios:** **148+** (includes `quest-log-multi`; was 147 before that, 136 at morning baseline).

### Verified live baseline

| Run | Archive / note | Result |
|---|---|---|
| Full on `4e14101c` (post walk harden) | `runs/20260811-210533.log` | **147 pass**, 1 expected-skip, 0 fail, **0 unmet / 0 exemptions**, 173 in / 66 out / **0 unknown** |
| Shuffle `20260810` on same HEAD | `runs/20260811-220535.log` | **146 pass**, **1 fail** (`incoming-damage`: natural mob one-shot → Desert Wolf retry used hard `walk_to` — path now multi-cell + warp; scoped **PASS** `20260811-230914.scoped`), 1 expected-skip, **0 unmet**, 174 in / 66 out / **0 unknown** |
| Prior full (empty exemptions + golden 1–10 + quest-log-multi) | `runs/20260811-190011.log` | 147 pass; 171 in / 66 out |
| Prior shuffle | `runs/20260811-200007.log` | 146 pass, 1 fail incoming-damage (first walk harden) |

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
| GitHub Actions green on latest HEAD | Confirm in PR checks |

### P7 — Process rules

Zero-unknown, exact expected-skips, allowlist cull discipline, archive semantics, never headless-green = client verified.

---

## 4. Suggested work order

```text
Optional: re-shuffle after Desert Wolf approach harden (full already green on 4e14101c)
  → Watch PR #2 multi-scenario CI
  → P5 client only if product asks
  → Homun/pet/guild/cart scenarios only when client implements them
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
