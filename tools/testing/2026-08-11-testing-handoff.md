# Testing additions handoff — August 11, 2026

This is the implementation handoff for the testing improvements completed on
August 11, 2026. Read this before changing the headless tester, packet fallback
policy, or the integration-run archive scripts.

The batch changed only the **Korangar** repository. The sibling **Hercules**
working tree was used as the protocol authority and live test server, but no
tracked Hercules file was changed.

## Final verified state

The suite registers **136 scenarios**. A fully green run is:

- 135 passed;
- 1 expected skip (`skills-novice`, with one exact reviewed reason);
- 0 flaky passes;
- 0 failed;
- 0 unexpected skips;
- 0 known failures;
- 0 unknown/fallback packet headers;
- 0 packet deserialization failures.

This result was reproduced in normal order and with shuffle seed `20260810`.
The normal run handled 170 distinct incoming packet headers; the shuffled run
handled 171. Both sent 63 distinct outgoing headers and had empty `unknown` and
`deserialization_failures` arrays.

The skill sweep still proves **protocol liveness**, not gameplay correctness.
Derived skill expectations remain measured and reported, but are not enforced.

## 1. Skill silence allowlist cleanup

File: `korangar-networking/examples/headless-tester/scenarios/skills.rs`

Removed from `allowlisted()`:

- `CR_DEVOTION`;
- `CR_PROVIDENCE`;
- `CG_MARIONETTE`.

All three reliably answer when the sweep aims them at the real partner fixture:

- Devotion returns partner-directed failure feedback;
- Providence produces a partner buff and follow-up effect evidence;
- Marionette returns partner-directed failure feedback.

An explicit server refusal is valid liveness evidence. These entries were
dangerous to retain because an allowlist entry turns future silence into an
accepted result.

Retained:

- `TK_MISSION`: answered for Taekwon and Soul Linker, but stayed silent for Star
  Gladiator in the same full run;
- `HT_SPRINGTRAP`: stayed silent in both jobs that expose it;
- the other existing entries whose setup requirements the sweep still cannot
  provide.

Do not remove an allowlist entry because it answered once. Verify it across all
jobs that expose the skill and across normal plus shuffled full runs. The end-of-
run “load-bearing in SOME jobs” report exists specifically to expose this trap.

## 2. Reviewed packets and the zero-unknown gate

Three recurring headers were traced to Hercules source and given typed packet
models in `ragnarok-packets/src/lib.rs`:

| Header | Packet model | Hercules meaning | Handling decision |
|---|---|---|---|
| `0x0078` | `FakeNpcDialogAnchorPacket` | `clif_sendfakenpc`, a 55-byte synthetic class-111 dialog anchor | Registered no-op. Publishing `AddEntity` would create a phantom actor. |
| `0x00CD` | `GmKickResponsePacket` | `ZC_ACK_DISCONNECT_CHARACTER`, a 3-byte GM kick result | Registered no-op. The kicked connection owns the visible disconnect flow. |
| `0x0191` | `TalkieBoxMessagePacket` | `ZC_TALKBOX_CHATCONTENTS`, 27 bytes at PACKETVER 20220406 | Registered no-op. No client surface consumes trap-local text yet. |

`reviewed_unknown_packets_match_hercules_20220406_layouts` constructs exact wire
bytes and verifies all three layouts. Their registrations live in
`korangar-networking/src/packet_versions/version_20220406.rs`.

The headless tester now has an explicit reviewed-unknown baseline:

```rust
const REVIEWED_UNKNOWN_PACKET_HEADERS: &[u16] = &[];
```

It is deliberately empty because every header observed in the two full runs now
has a model. `Ledger::unexpected_unknown()` subtracts only explicitly reviewed
headers and returns sorted `(header, count)` pairs. After the console summary and
JSON result are written, the executable fails if either condition is true:

1. any packet failed deserialization;
2. any unknown/fallback header is not in the reviewed baseline.

Do not add a header to `REVIEWED_UNKNOWN_PACKET_HEADERS` merely to make a run
green. Identify it against the PACKETVER 20220406 Hercules source, add an exact
layout test, then either register a real event handler or document why a typed
no-op is correct. If a genuinely opaque packet must be temporarily accepted,
name it in the baseline with a findings entry and an owner for removal.

## 3. Runner and CI regression coverage

### Archive contract

`tools/testing/run-suite.sh` now has executable regression coverage. Its contract
is:

| Run outcome | Console artifact | JSON artifact | Exit behavior |
|---|---|---|---|
| Complete full pass | `*.log` | `*.log.json` when the wrapper owns the path | tester status |
| Complete full failure | `*.log` | `*.log.json` | nonzero tester status, evidence retained |
| Interrupted/no summary | `*.partial` | not promoted | interruption status |
| Complete targeted run | `*.scoped` | `*.scoped.json` | tester status |
| Recovered retry under strict mode | `*.log`, containing `FLAKY-PASS` | outcome remains `flaky-pass` | nonzero |
| Unexpected skip | `*.log`, containing `UNEXPECTED-SKIP` | outcome remains `unexpected-skip` | nonzero |

Only complete full runs become `*.log`, because `tools/audits/flaky.py` consumes
that glob. A targeted or interrupted run must never enter the cross-run sample.
Completion is determined by the literal `=== Summary` marker, **not** by exit
status; this is why a complete red run is promoted while an interrupted run is
not. Do not replace that distinction with an exit-zero check.

When a caller supplies `--results-json`, it owns that path. In particular,
`run-integration-tests.sh` keeps the console archive under `runs/*.log` while
the structured result remains `target/headless-results.json`; it does not also
create a neighboring `*.log.json`.

The implementation adds `HEADLESS_CARGO` as a narrow command-injection seam.
It exists for `tools/testing/test-run-suite.sh`; production runs should leave it
unset. The test uses `tools/testing/fixtures/fake-headless-cargo.sh` to exercise
the archive state machine without starting Hercules or spending an hour on the
live suite.

The GitHub Actions workflow now runs:

```sh
tools/testing/test-run-suite.sh
```

after the headless harness unit tests.

### Gate semantics

The scenario exit calculation was extracted into `gate_failure_count()` and is
unit-tested. Ordinary failures and unexpected skips always count. A recovered
retry counts when `--fail-on-flaky` is active. The disposable integration runner
always supplies `--fail-on-flaky`.

### Bounded server shutdown

During live validation, Hercules `map-server` occasionally ignored `SIGTERM`
after the skill-heavy run and blocked cleanup indefinitely. The integration
runner now:

1. sends `SIGTERM` only to the exact three PIDs it started;
2. waits up to ten seconds for graceful shutdown;
3. logs a warning and sends `SIGKILL` only to an exact still-running owned PID;
4. waits for those PIDs before auditing the database fixture.

This preserves normal graceful shutdown while preventing CI from hanging after
the tester has already produced its result.

## 4. How to reproduce the validation

Run these from the Korangar repository root.

Fast checks that require no live server:

```sh
cargo test -p ragnarok-packets
cargo test -p korangar-networking --example headless-tester
tools/testing/test-run-suite.sh
bash -n tools/testing/run-suite.sh \
  tools/testing/run-integration-tests.sh \
  tools/testing/test-run-suite.sh \
  tools/testing/fixtures/fake-headless-cargo.sh
git diff --check
```

Verified results for this batch were 52 packet tests, 7 harness tests, and a
green runner regression script. Two pre-existing compiler warnings remain in
`scenarios/items.rs` and `context.rs`; neither is introduced by this batch.

For a self-contained live run, start MariaDB and let the integration script
create a unique database, temporary Hercules import configuration, accounts, and
characters:

```sh
HERCULES_DIR=../Hercules tools/testing/run-integration-tests.sh
HERCULES_DIR=../Hercules tools/testing/run-integration-tests.sh --shuffle 20260810
```

Set `INTEGRATION_SKIP_BUILD=1` only when the Hercules binaries are already built
for PACKETVER 20220406. Override `INTEGRATION_DB_PORT` when MariaDB is not on
3306. The runner refuses to start if ports 6900, 6121, or 5121 are already in
use, always enables strict flaky handling, writes
`target/headless-results.json`, audits fixture cleanup, drops its unique
database, and restores the prior ignored Hercules import files.

For an already-running manually managed Hercules stack, use:

```sh
tools/testing/run-suite.sh
tools/testing/run-suite.sh --shuffle 20260810
tools/testing/run-suite.sh --scenario skills-mage
```

## 5. File map for this batch

| File | Purpose |
|---|---|
| `ragnarok-packets/src/lib.rs` | Three packet models and exact PACKETVER 20220406 layout tests. |
| `korangar-networking/src/packet_versions/version_20220406.rs` | Typed no-op registrations. |
| `korangar-networking/examples/headless-tester/ledger.rs` | Reviewed-vs-unexpected unknown calculation and unit test. |
| `korangar-networking/examples/headless-tester/main.rs` | Zero-unknown gate and explicit scenario gate calculation. |
| `korangar-networking/examples/headless-tester/scenarios/skills.rs` | Evidence-based allowlist cleanup. |
| `tools/testing/run-suite.sh` | Injectable cargo command for runner regression testing. |
| `tools/testing/test-run-suite.sh` | Archive/gate contract regression suite. |
| `tools/testing/fixtures/fake-headless-cargo.sh` | Deterministic fake tester process used only by the runner tests. |
| `tools/testing/run-integration-tests.sh` | Bounded owned-PID Hercules shutdown. |
| `.github/workflows/tests.yml` | Runs the archive regression suite in CI. |
| `tools/testing/headless_test_plan.md` | Canonical policy and runner behavior. |
| `tools/testing/headless_findings.md` | Evidence for the allowlist decisions and packet investigation. |

## 6. What this batch did not claim

- It did not prove that all 983 skill casts have correct gameplay effects. The
  liveness sweep still accepts explicit server refusals and reviewed silence.
- It did not add a graphical presentation for Talkie Box text or the synthetic
  dialog anchor.
- It did not exercise the graphical client’s UI/state consumption layer.
- It did not change tracked Hercules source, scripts, configuration, or SQL.
- Derived skill expectations were still report-only in *this* handoff; a later
  depth batch (same day) enforced them with reviewed exemptions — see
  [headless-next-steps.md](headless-next-steps.md) for that work and the
  forward P0–P7 priority list agents should follow next.
