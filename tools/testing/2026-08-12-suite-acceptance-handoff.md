# Handoff — headless suite acceptance closed (2026-08-12)

| | |
|---|---|
| **Branch** | `agent/platform-connectivity-controls` |
| **Suite HEAD** | `e4d6e6d5` (Desert Wolf approach harden) + doc reconcile |
| **PR** | https://github.com/Boneysan/korangar/pull/2 |

## What closed

Headless **planned depth** is done. Do not re-open for allowlist culls,
cast-only skill expansion, or duration jitter.

| Deliverable | Evidence |
|---|---|
| Full acceptance | `runs/20260811-210533.log` — 147 pass / 1 skip / 0 fail / 0 unmet / 0 unknown |
| Shuffle | `runs/20260811-220535.log` — 146 pass; 1 intermittent `incoming-damage` (fixed path) |
| Scoped after fix | `runs/20260811-230914.scoped` — PASS |
| Empty `EXPECTATION_EXEMPTIONS` | FeelRequest, partner Spellbreaker, etc. closed |
| Golden arcs 1–10 | `dm-golden-beats` with `A0N:` progress tokens |
| Quest wire | `dm-quest-lifecycle` + `quest-log-multi` |
| PR CI | Multi-scenario list (not smoke-only) in `integration.yml` |

## Code notes (already on branch)

- `incoming-damage`: shared multi-cell + warp approach for natural *and* Desert Wolf paths (`combat.rs`)
- CI: PR runs short multi-scenario gate; schedule/manual still `all`

## Doc map (updated this handoff)

| File | Role |
|---|---|
| [headless-next-steps.md](headless-next-steps.md) | Suite status, archives, non-goals, resume → GUI |
| [gui-verification-pass.md](../../docs/plans/gui-verification-pass.md) | GUI open-only table at top; full findings inline |
| [plans/README.md](../../docs/plans/README.md) | Index: suite closed vs GUI in progress |
| [work-backlog.md](../../docs/plans/work-backlog.md) | §1 reconciled to open-only; §2+ features |
| [RESUME-HERE.md](../../docs/RESUME-HERE.md) | Two-track table |

## What is *not* closed (intentionally separate)

1. **GUI live pass** — Hermode, N20 Auto Spell; see open-only table  
2. **Client features** — quest journal UI, spirit sphere render, cast circles  
3. **Optional** — re-shuffle on `e4d6e6d5` for clean double-green; GitHub Actions check on PR  

## Run commands (unchanged)

```sh
HERCULES_DIR=../Hercules \
INTEGRATION_DB_ADMIN=korangar_int \
INTEGRATION_DB_ADMIN_PASSWORD=korangar_int \
INTEGRATION_SKIP_BUILD=1 \
  tools/testing/run-integration-tests.sh

# Shuffle:
# ... run-integration-tests.sh --shuffle 20260810
```
