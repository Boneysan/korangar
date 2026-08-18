#!/usr/bin/env bash
# Fail if a known credential is committed.
#
# Written 2026-08-17, after the first security audit found the server's *admin*
# password sitting in tracked files of a PUBLIC repository. A generic entropy
# scanner would drown this codebase in false positives -- it is full of packet
# hex, GRF hashes and Korean asset names -- so this checks for the specific
# strings we know are credentials, which is the class that actually bit us.
#
# When a credential is rotated, the new one must not be added here. Add the
# *old* one, so re-committing it fails.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

# Known credentials. Split so this file does not itself trip the scan.
patterns=(
    "headless2""pw"
    "headless3""pw"
    # Username-as-password pair from the published admin account. Not a generic
    # "korangar" match — that word is the project name.
    "(2000000, 'korangar', 'korangar'"
)

status=0
known=0
baseline="tools/audits/committed-secrets.baseline"

for pattern in "${patterns[@]}"; do
    # Tracked files only: a developer's untracked local config is their business.
    # Tracked files only, and only paths not already recorded as known debt.
    while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        path="${hit%%:*}"

        if grep -qxF "$path:$pattern" "$baseline"; then
            known=$((known + 1))
            continue
        fi

        printf '\nNEW CREDENTIAL COMMITTED: %s\n  %s\n' "$pattern" "$hit"
        status=1
    done <<< "$(git grep -n --fixed-strings -- "$pattern" -- . ':!tools/audits/committed-secrets.sh' ':!tools/audits/committed-secrets.baseline' ':!docs/plans/security-audit.md' ':!docs/plans/security-audit-2.md' ':!docs/plans/security-audit-3.md' ':!docs/plans/security-audit-4.md' 2>/dev/null || true)"
done

if [ "$status" -ne 0 ]; then
    cat <<'MSG'

This repository is public. A credential in a tracked file is published, and
rewriting history does not unpublish it -- rotate the credential instead, then
add the OLD value to the pattern list above so it cannot come back.

MSG
    exit 1
fi

if [ "$known" -gt 0 ]; then
    printf 'committed-secrets: no NEW leaks, but %d known site(s) remain -- see %s\n' "$known" "$baseline"
    printf 'These are the admin password in a public repository. Rotate it.\n'
else
    echo "committed-secrets: clean, and the baseline is empty"
fi
