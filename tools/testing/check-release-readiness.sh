#!/usr/bin/env bash
# Mechanical integrity check for the QW-105 release-readiness package.
# This is read-only: it never stages, rewrites, resets, commits, or publishes.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
client_repo="$(cd "$here/../.." && pwd)"
hercules_repo="${HERCULES_DIR:-$(cd "$client_repo/../Hercules" && pwd)}"
runbook="$client_repo/docs/plans/qwen3-playtest-runbook.md"
report="$client_repo/docs/reports/release-readiness-2026-09-16.md"

for required in "$runbook" "$report" \
    "$client_repo/tools/testing/runs/20260916-232255.scoped" \
    "$client_repo/tools/testing/runs/20260916-232810.scoped" \
    "$client_repo/tools/testing/runs/20260916-233018.scoped" \
    "$client_repo/tools/testing/runs/20260916-231914.scoped"; do
    [ -e "$required" ] || { echo "FAIL - missing release artifact: $required" >&2; exit 1; }
done

expected_client="866a8bac4d0055e63e2944d91b0931e9f078d5f0"
expected_hercules="0e9cc355c738704bf7d7e4c43db59c6fa5a886b7"
[ "$(git -C "$client_repo" rev-parse HEAD)" = "$expected_client" ] || {
    echo "FAIL - Korangar base revision changed; refresh the report hashes." >&2
    exit 1
}
[ "$(git -C "$hercules_repo" rev-parse HEAD)" = "$expected_hercules" ] || {
    echo "FAIL - Hercules base revision changed; refresh the report hashes." >&2
    exit 1
}

for required_text in \
    '## Revision and rollback boundary' \
    'No reset, commit, push, upload, or publication is' 'authorized' \
    '## Open acceptance gates' \
    'QW-006' 'QW-046' 'QW-052' 'QW-055' 'QW-056' 'QW-062' 'QW-063' \
    'QW-072' 'QW-079' 'QW-085' 'QW-101' 'QW-102' 'QW-103' 'QW-104' 'QW-105'; do
    rg -Fq "$required_text" "$report" || {
        echo "FAIL - release report is missing: $required_text" >&2
        exit 1
    }
done

for unchecked_id in $(rg -o '^-[[:space:]]\[ \][[:space:]]\*\*QW-[0-9]+' "$runbook" | sed -E 's/.*(QW-[0-9]+)$/\1/'); do
    rg -Fq "$unchecked_id" "$report" || {
        echo "FAIL - unchecked runbook card is absent from release report: $unchecked_id" >&2
        exit 1
    }
done

git -C "$client_repo" diff --check
git -C "$hercules_repo" diff --check
unchecked="$(rg -c '^-[[:space:]]\[ \][[:space:]]\*\*QW-' "$runbook")"
echo "OK - release readiness package is internally consistent (${unchecked} unchecked runbook cards; external gates remain explicit)."
