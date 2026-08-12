# CI cleanup handoff — August 11, 2026

This document records the **uncommitted** cleanup started after the testing
batch in `2026-08-11-testing-handoff.md`. It is intentionally a stopping point,
not a claim that the branch is ready to merge.

## Repository and pull request state

- Repository: `Boneysan/korangar`
- Branch: `agent/platform-connectivity-controls`
- Upstream: `origin/agent/platform-connectivity-controls`
- Last committed and pushed revision: `f1d37603` (`Harden headless packet and
  runner gates`)
- Draft PR: <https://github.com/Boneysan/korangar/pull/2>
- PR title is still the stale `Complete native combat animation runtime`.
- The PR description has **not** been updated.
- There are 72 modified tracked files and no cleanup commit for this session.
- Do not discard or reset the worktree. It contains the formatter pass, the
  paired-integration repair, and the partial Clippy cleanup described below.
- The sibling `Hercules` checkout was clean when this work stopped. Its pinned
  revision remains `c07c4b235ce213c446d0694449ac6b7d069e08a5`.
- The disposable MariaDB instance and temporary CI artifacts used during
  diagnosis were shut down and moved to Trash. No Hercules server from the
  smoke run remains. A pre-existing Homebrew MariaDB service is still running
  from `/opt/homebrew/var/mysql`; it was not started, stopped, or modified as
  part of cleanup.

The checks currently shown on GitHub belong to `f1d37603`, before any of this
uncommitted cleanup. Their state is:

| Check | State at handoff | Meaning |
|---|---|---|
| Formatting | failed | Reproduced locally and addressed by `cargo fmt --all`. |
| Korangar + pinned Hercules | failed | Root cause found and repaired locally; smoke scenario now passes. |
| Run cargo clippy | failed | Default-feature failures were repaired locally; all-features verification is unfinished. |
| Build | passed | Result for `f1d37603`. |
| Tests | passed | Result for `f1d37603`. |
| Nix checks | passed | Result for `f1d37603`. |

## Changes currently in the worktree

### 1. Repository-wide Rust formatting

`cargo fmt --all` was required because the long-lived branch had broad format
drift. It touched roughly 60 Rust files, including generated Rust tables. The
current formatter check passes:

```sh
cargo fmt --all --check
```

Most of the 1,484 additions and 1,433 deletions reported by `git diff --stat`
are line wrapping and import ordering. Review with whitespace ignored as well
as normally before committing:

```sh
git diff --check
git diff --stat
git diff -w
```

### 2. Paired Korangar/Hercules integration startup

File: `tools/testing/run-integration-tests.sh`

The failed Actions artifact showed that the generated Hercules import files
placed `@include` directives inline inside braces. The Linux Hercules config
parser rejected all four files as syntax errors, fell back to the default
`ragnarok` database credentials, and then failed authentication.

The local repair writes multiline blocks for:

- `login_configuration.account.ipban`;
- `char_configuration`;
- `map_configuration`;
- `inter_configuration.log`.

The two diagnostic log calls were also changed from the non-portable
`tail -40` form to:

```sh
tail -n 40 -- "$server_log_dir"/*.log
```

This was reproduced against the pinned Hercules checkout and a disposable
MariaDB instance on port 33307:

```text
smoke: 1 passed, 0 flaky, 0 failed
fixture cleanup: clean
```

The successful scoped artifact was
`tools/testing/runs/20260811-075051.scoped` (normally ignored).

### 3. Default-feature Clippy cleanup

The exact CI command now passes locally:

```sh
cargo clippy -- -Dwarnings
```

The first CI error was two intentionally retained but currently unconstructed
`AttenuationFunction` variants. Clearing it exposed 50 additional warnings in
the long-lived branch. The current worktree addresses them with:

- small type aliases for pending inventory data and map-audit teleport points;
- removal or feature-gating of unused imports and re-exports;
- `Default` derivation for `SpriteLightingMode`;
- Clippy-safe condition chains, iteration, ASCII checking, option checks,
  argument conversions, and whitespace splitting;
- narrowly placed `clippy::too_many_arguments` annotations, consistent with
  existing project style;
- narrowly placed `dead_code` annotations for staged state, animation, and
  effect APIs that are intentionally present but not yet connected;
- test-only gates for `AL_PNEUMA` and `SOUL_STRIKE_ORB_TEXTURE`.

The judgment-heavy part is the `dead_code` treatment. Before committing, review
each new annotation and choose deliberately among keeping the staged API,
wiring it into production, gating it to tests, or removing it. In particular,
review the staged audio attenuation choices, party/trade/storage UI getters,
weapon-layer helpers, `SoulStrikeOrbs`, sprite effect recipes, and special-
effect shape diagnostics. Also verify that removing the `SoulStrikeOrbs` and
`ItemStats` re-exports does not conflict with the intended public module API.

## Verification completed before stopping

The following checks passed during this session:

- `cargo fmt --all --check`;
- `cargo clippy -- -Dwarnings` after the current default-feature lint edits;
- `git diff --check`;
- `tools/testing/test-run-suite.sh`;
- Bash syntax checks for the testing scripts;
- `cargo test -p ragnarok-packets`: 52 passed;
- `cargo test -p korangar-networking --example headless-tester`: 7 passed;
- `cargo test -p korangar --lib`: 277 passed, 17 ignored;
- pinned-Hercules `smoke`: 1 passed with clean fixture cleanup.

The three Cargo test commands were run **before** the final Clippy-driven source
edits. Those edits are small, but the tests must be rerun before commit.

## Unfinished work — resume here

### Closed later the same day (night session)

1. **All-features Clippy** now passes locally:

   ```sh
   cargo clippy --all-features -- -Dwarnings
   ```

   Fixes included cfg-gated `cgmath::One` for debug-only matrices and
   `print_debug!` format args under the debug feature.

2. **Generated-file parity** restored. Generators
   `tools/generate_skill_states.py` and `tools/generate_skill_expectations.py`
   now rustfmt their output after write. Drift audit is clean:

   ```sh
   HERCULES_DIR=../Hercules tools/audits/generated-drift.sh
   ```

3. Review the non-format changes, especially every new lint annotation and the
   two removed re-exports:

   ```sh
   git diff -w
   rg -n "#\[(allow\(dead_code|allow\(clippy::too_many_arguments|cfg\(test\))" \
     korangar korangar-audio korangar-networking
   ```

4. Rerun the fast validation after the final lint changes:

   ```sh
   cargo fmt --all --check
   cargo clippy -- -Dwarnings
   cargo clippy --all-features -- -Dwarnings
   cargo test -p ragnarok-packets
   cargo test -p korangar-networking --example headless-tester
   cargo test -p korangar --lib
   tools/testing/test-run-suite.sh
   bash -n tools/testing/run-suite.sh \
     tools/testing/run-integration-tests.sh \
     tools/testing/test-run-suite.sh \
     tools/testing/fixtures/fake-headless-cargo.sh
   git diff --check
   ```

5. Run at least the pinned-Hercules smoke scenario again after the final edits.
   A disposable MariaDB service is required. The verified form used this
   session was equivalent to:

   ```sh
   HERCULES_DIR=../Hercules \
   INTEGRATION_SKIP_BUILD=1 \
   INTEGRATION_DB_PORT=<mariadb-port> \
     tools/testing/run-integration-tests.sh --scenario smoke
   ```

6. Confirm that Korangar contains only this intended batch and Hercules is
   still clean. Then commit and push the Korangar branch. A suitable commit
   subject is:

   ```text
   Align formatting and paired integration CI
   ```

7. Update the existing draft PR; do not open a second PR and do not mark it
   ready. Suggested title:

   ```text
   Advance Korangar runtime, DM tooling, and integration testing
   ```

   The description should identify this as a long-lived draft integration
   branch, summarize runtime/DM/protocol/headless-testing scope, link or refer
   to `2026-08-11-testing-handoff.md`, state the 136-scenario result (135 pass,
   one expected skip, zero flaky/fail/unknown), list the fast-test totals, and
   explicitly state that skill-sweep liveness is not gameplay-correctness proof
   and graphical UI verification remains separate.

8. Watch the new GitHub checks created by the push. The minimum completion bar
   for this cleanup is green Formatting, both Clippy commands, Build, Tests,
   Nix, and `Korangar + pinned Hercules`. Treat any new failure as part of this
   cleanup rather than updating the PR to imply it is complete.

## Testing work after the cleanup

Once the branch is back to a clean, green baseline, continue the testing plan
rather than treating this CI repair as the end state.

**Canonical forward list:** [headless-next-steps.md](headless-next-steps.md)
(P0 live re-validate, P1 shrink expectation exemptions, P2 negative scenarios,
P3–P7 sweep/campaign/client/CI/process rules).

High-value reminders that remain true:

- retain the zero-unknown-packet and zero-deserialization-failure gates;
- keep full-run, shuffled-run, scoped-run, flaky, and interrupted archive
  behavior under regression coverage;
- remove skill-silence allowlist entries only after normal and shuffled
  evidence across every job exposing the skill;
- rerun the generated-drift audit whenever the pinned Hercules revision or
  packet/skill databases change.

Ship contracts: [2026-08-11-testing-handoff.md](2026-08-11-testing-handoff.md).
This file only describes unfinished CI/PR cleanup layered on top of that work.
