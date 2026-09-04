#!/usr/bin/env bash
# Regression tests for run-suite.sh's evidence/archive contract.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
runner="$here/run-suite.sh"
fake_cargo="$here/fixtures/fake-headless-cargo.sh"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/korangar-run-suite.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT

fail() {
    echo "run-suite test failed: $*" >&2
    exit 1
}

assert_count() {
    expected="$1"
    directory="$2"
    pattern="$3"
    actual="$(find "$directory" -maxdepth 1 -type f -name "$pattern" | wc -l | tr -d ' ')"
    [ "$actual" = "$expected" ] || fail "expected $expected $pattern file(s) in $directory, found $actual"
}

run_case() {
    name="$1"
    mode="$2"
    shift 2
    case_dir="$test_root/$name"
    mkdir -p "$case_dir/runs"

    # Credentials are passed explicitly, and HERCULES_DIR is aimed at nothing,
    # so the case cannot pick up a real operator-credentials.conf. Without this
    # the test only passes on a machine that happens to have live credentials
    # beside the repo: run-suite.sh exits 2 when it cannot build a --password
    # argument, long before the fake cargo runs. It passed locally and failed
    # in CI for exactly that reason. What is under test here is the archive
    # contract, not credential resolution.
    set +e
    HEADLESS_RUNS_DIR="$case_dir/runs" HEADLESS_CARGO="$fake_cargo" HEADLESS_FAKE_MODE="$mode" \
        HERCULES_DIR="$test_root/no-hercules" \
        "$runner" --password test-password --partner-password test-partner-password "$@" \
        > "$case_dir/output" 2>&1
    case_status=$?
    set -e
}

run_case complete-red red
[ "$case_status" = 7 ] || fail "complete red run returned $case_status, expected 7"
assert_count 1 "$case_dir/runs" '*.log'
assert_count 1 "$case_dir/runs" '*.log.json'
assert_count 0 "$case_dir/runs" '*.partial'
grep -q '"outcome":"fail"' "$case_dir"/runs/*.log.json || fail "red run lost its JSON artifact"

run_case interrupted interrupted
[ "$case_status" = 130 ] || fail "interrupted run returned $case_status, expected 130"
assert_count 1 "$case_dir/runs" '*.partial'
assert_count 0 "$case_dir/runs" '*.log'

run_case targeted pass --scenario smoke
[ "$case_status" = 0 ] || fail "targeted run returned $case_status, expected 0"
assert_count 1 "$case_dir/runs" '*.scoped'
assert_count 1 "$case_dir/runs" '*.scoped.json'
assert_count 0 "$case_dir/runs" '*.log'

run_case flaky flaky
[ "$case_status" = 1 ] || fail "strict flaky run returned $case_status, expected 1"
assert_count 1 "$case_dir/runs" '*.log'
grep -q 'FLAKY-PASS' "$case_dir"/runs/*.log || fail "recovered retry was not archived as FLAKY-PASS"
grep -q '"outcome":"flaky-pass"' "$case_dir"/runs/*.log.json || fail "flaky JSON outcome was not preserved"

run_case unexpected-skip unexpected-skip
[ "$case_status" = 1 ] || fail "unexpected skip returned $case_status, expected 1"
assert_count 1 "$case_dir/runs" '*.log'
grep -q 'UNEXPECTED-SKIP' "$case_dir"/runs/*.log || fail "unexpected skip was not archived as a failure"

echo "run-suite archival tests passed"
