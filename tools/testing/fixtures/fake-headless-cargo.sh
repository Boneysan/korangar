#!/usr/bin/env bash
# Deterministic cargo stand-in for test-run-suite.sh. It only models the
# headless tester's observable contract: output, result JSON, and exit status.
set -euo pipefail

results_json=""
arguments=(${@+"$@"})
index=0
while [ "$index" -lt "${#arguments[@]}" ]; do
    if [ "${arguments[$index]}" = "--results-json" ]; then
        index=$((index + 1))
        results_json="${arguments[$index]}"
        break
    fi
    index=$((index + 1))
done

write_json() {
    outcome="$1"
    printf '{"schema_version":1,"scenarios":[{"outcome":"%s"}]}' "$outcome" > "$results_json"
}

case "${HEADLESS_FAKE_MODE:?HEADLESS_FAKE_MODE is required}" in
pass)
    write_json pass
    echo "=== Summary: 1 passed, 0 flaky-pass, 0 failed, 0 expected-skip, 0 unexpected-skip, 0 known-fail ==="
    exit 0
    ;;
red)
    write_json fail
    echo "[FAIL] example"
    echo "=== Summary: 0 passed, 0 flaky-pass, 1 failed, 0 expected-skip, 0 unexpected-skip, 0 known-fail ==="
    exit 7
    ;;
interrupted)
    echo "[Running] example"
    exit 130
    ;;
flaky)
    write_json flaky-pass
    echo "[FLAKY-PASS] example"
    echo "=== Summary: 0 passed, 1 flaky-pass, 0 failed, 0 expected-skip, 0 unexpected-skip, 0 known-fail ==="
    exit 1
    ;;
unexpected-skip)
    write_json unexpected-skip
    echo "[UNEXPECTED-SKIP] example"
    echo "=== Summary: 0 passed, 0 flaky-pass, 0 failed, 0 expected-skip, 1 unexpected-skip, 0 known-fail ==="
    exit 1
    ;;
*)
    echo "unknown HEADLESS_FAKE_MODE: $HEADLESS_FAKE_MODE" >&2
    exit 64
    ;;
esac
