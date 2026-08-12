# Headless testing — prioritized next steps

**Audience:** humans and coding agents (Claude, Codex, Cursor, etc.).
**Read this when:** improving the headless suite, deciding what test to write
next, or about to claim the suite is "complete".

| | |
|---|---|
| **Status** | **P0–P4/P6 advanced 2026-08-11 night** — full+shuffle green; P1 exemptions shrunk; CI gates local-green |
| **Resume first** | Commit/push CI batch, optional full re-suite after P1, remaining P5 client art |
| **Canonical plan** | [headless_test_plan.md](headless_test_plan.md) |
| **Latest ship handoff** | [2026-08-11-testing-handoff.md](2026-08-11-testing-handoff.md) |
| **P0 pause handoff** | [2026-08-11-p0-p4-pause-handoff.md](2026-08-11-p0-p4-pause-handoff.md) (historical; P0 closed below) |
| **Meaning of green** | [../../docs/plans/testing-completeness.md](../../docs/plans/testing-completeness.md) |
| **Findings log** | [headless_findings.md](headless_findings.md) |

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
| Skill expectations | Non-exempt `Unmet` rows fail the run |
| Expected skips | Exact name + reason only (`skills-novice` today) |
| Archives | Complete full → `runs/*.log`; targeted → `*.scoped`; interrupted → `*.partial` |

**Registered scenarios:** **147** (was 136 at morning baseline).

### Verified live baseline (2026-08-11 evening)

| Run | Archive | Result | Packets |
|---|---|---|---|
| Full `--scenario all` | `runs/20260811-161233.log` | **146 pass**, 0 flaky, 0 fail, **1 expected-skip** (`skills-novice`), 0 unexpected-skip | 171 in / 66 out / **0 unknown** / 0 deser fail |
| Shuffle `--shuffle 20260810` | `runs/20260811-171208.log` | **146 pass**, 0 flaky, 0 fail, **1 expected-skip**, 0 unexpected-skip | 173 in / 66 out / **0 unknown** / 0 deser fail |

Expectations (enforced): normal order **635** casts (264 met / 362 refused / 6 blocked / 3 reviewed-exemption unmet, 0 unexpected); shuffle **634** casts (265 met / 362 refused / 6 blocked / residual unmet printed as SG_FEEL only). Fixture cleanup clean both runs.

Load-bearing silence allowlist (do not cull from one job’s answer alone): `HT_REMOVETRAP` (silent on Rogue/Stalker — no trap placer), `TK_MISSION` (job-dependent).

**Integration DB for local macOS:** TCP admin `korangar_int` / `korangar_int`
(Homebrew `root` is often socket-only). Fixture emails are `a@a.com` so
`DeleteCharacterPacket` succeeds. Char import sets `deletion.delay: 0`.

```sh
HERCULES_DIR=../Hercules \
INTEGRATION_DB_ADMIN=korangar_int \
INTEGRATION_DB_ADMIN_PASSWORD=korangar_int \
INTEGRATION_SKIP_BUILD=1 \
  tools/testing/run-integration-tests.sh

# Shuffle replay:
# ... run-integration-tests.sh --shuffle 20260810

# Multi-scenario selector:
# --scenario "smoke,connection-state,skills-mage"
```

Fast checks (no live server):

```sh
cargo test -p ragnarok-packets
cargo test -p korangar-networking --example headless-tester
tools/testing/test-run-suite.sh
```

---

## 2. What shipped in the depth batches (2026-08-11)

1. Partner-suffix expectation match; expectation **enforcement** with exemptions
2. Negative scenarios: trade-reject/invalid, identify-cancel, equip-wrong-job,
   shop-close, use-drop-failures, storage-persistence
3. Session: connection-state, character-select-invalid
4. channeling-start-stop (wire API)
5. dm-golden-beats (arcs 1–3 + `@dmstatus`)
6. `prepare_skill_cast` + Soul Link partner job mapping
7. AL_CRUCIS / MC_IDENTIFY expectation honesty (no longer exempted)
8. HT_REMOVETRAP entity-target setup; **silence allowlist retained** for
   Rogue/Stalker (no placer skill) after full-run evidence
9. `NotifySkillUnitGraffitiPacket` (`0x01C9`)
10. Integration: email `a@a.com`, deletion delay 0, free-slot character create
11. Comma-separated `--scenario` lists
12. `ACTION_COVERAGE` manifest unit test

---

## 3. Priority status

### P0 — Full live validation

| Item | Status |
|---|---|
| Scoped new scenarios | **Done** green |
| Lifecycle / provision / email / free-slot fixes | **Done** green |
| Full `--scenario all` | **Done** green — `runs/20260811-161233.log` |
| Shuffle `20260810` | **Done** green — `runs/20260811-171208.log` |

### P1 — Expectation exemptions

| Skill | Status |
|---|---|
| AL_CRUCIS | **Closed** — no-damage-effect meets Status for this skill |
| MC_IDENTIFY | **Closed** — identify-list = blocked / met path |
| HT_REMOVETRAP | **Improved** — entity target when placer exists; **silence allowlist kept** for Rogue/Stalker |
| RG_CLEANER | **Closed** — refuse is `fail-feedback` → `Refused`; multi-cell graffiti + paint brush setup |
| SG_FEEL | **Closed** — observes `FeelRequest` (0x0253) as blocked/met modal |
| SA_SPELLBREAKER | **Closed** — partner mid-cast; always targets partner (ally refuse is honest `Refused`, never pupa Unmet) |

`EXPECTATION_EXEMPTIONS` is now **empty**.

### P2 — Negative / boundary scenarios

| Item | Status |
|---|---|
| shop/use-drop/storage/trade negatives | **Done** |
| connection-state / character-select-invalid | **Done** |
| channeling-start-stop | **Done** |
| multi-scenario CLI | **Done** |

### P3 — Sweep correctness / flaky

Re-ran `tools/audits/flaky.py tools/testing/runs/*.log` after P0 full+shuffle
archives (2026-08-11):

- **Job-dependent silence (load-bearing allowlist):** `HT_REMOVETRAP`
  (Rogue/Stalker), `HT_SPRINGTRAP` (Sniper), `TK_MISSION` (Star Gladiator)
- **Cross-run intermittent silence:** `SL_SMA` still appears; keep in
  `KNOWN_INTERMITTENT`
- **Scenario duration spikes:** many short scenarios show ~7× worst/median
  (connection jitter); not treated as product defects
- **3rd-class jobs:** still intentionally out of scope

`KNOWN_INTERMITTENT`: `MG_NAPALMBEAT`, `HP_BASILICA`, `SL_SMA`

Do **not** cull allowlist entries from a single job’s answer — flaky.py’s
“silent in some jobs only” section is the authority.

### P4 — Campaign golden beats

`dm-golden-beats` now runs first non-warp story beat of arcs **1–6** (full Act I
plus first Act II arc) and asserts `@dmstatus` answers. Soft `@dmflag` touch
keeps flag tooling warm. Expand further only with content-reviewed rows.

### P5 — Outside headless (partial)

| Item | Status |
|---|---|
| Ice Wall walkability | **Done** at unit level (`ice_wall_cells_are_not_walkable`) + headless `ice-wall-blocks-cells` |
| Spirit sphere **state** | **Done** — `ZC_SPIRITS` → entity field; headless `spirit-spheres` |
| Spirit sphere **render** | **Open** — deliberate asset task; state only today |
| Quest UI | **Open** — not a headless concern; no quest window yet |
| Full GUI pass | **Open** — manual client session |

### P6 — CI (local gates green; push pending)

Local gates verified after cleanup:

- `cargo fmt --all --check`
- `cargo clippy -- -Dwarnings`
- `cargo clippy --all-features -- -Dwarnings`
- `HERCULES_DIR=../Hercules tools/audits/generated-drift.sh` (generators now
  rustfmt their output)
- packet/harness/`korangar --lib` unit tests; `tools/testing/test-run-suite.sh`

Still open until someone commits/pushes: draft PR #2 title/description update,
GitHub Actions green on the new revision. See
[2026-08-11-ci-cleanup-handoff.md](2026-08-11-ci-cleanup-handoff.md).

### P7 — Process rules (reinforced)

Standing contracts (do not regress):

1. Zero unknown packets / zero deserialization failures
2. Exact expected-skip name+reason only
3. Allowlist cull only after normal **and** shuffled evidence across every
   exposing job (`flaky.py` “silent in some jobs only”)
4. Archive semantics: complete full → `*.log`; scoped → `*.scoped`; interrupted → `*.partial`
5. Never report headless-green as client-verified gameplay

Documented here and in the testing handoffs so agents cannot rediscover weakened
gates as “improvements”.

---

## 4. Suggested work order for the next agent

```text
Commit + push CI/testing batch; update draft PR #2 title/description
  → Full suite + shuffle to reconfirm empty EXPECTATION_EXEMPTIONS + arcs 1–6 golden
  → P5 spirit-sphere render / quest UI when product priority asks
```

---

## 5. File map

| Path | Role |
|---|---|
| `korangar-networking/examples/headless-tester/` | Suite |
| `…/scenarios/skills.rs` | Sweep, allowlist, exemptions, prepare_skill_cast |
| `…/scenarios/mod.rs` | Registration + ACTION_COVERAGE |
| `tools/testing/run-integration-tests.sh` | Disposable DB + Hercules |
| `tools/audits/flaky.py` | Cross-run inconsistency |
| `ragnarok-packets` `NotifySkillUnitGraffitiPacket` | 0x01C9 graffiti unit |

---

## 6. When this document is stale

Update when a P-item closes, exemption list changes, scenario count changes, or
full-suite acceptance numbers are reconfirmed.
