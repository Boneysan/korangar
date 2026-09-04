#!/bin/sh
# Check a download against the SHA256SUMS-* lists shipped beside it.
#
# macOS ships `shasum`, so unlike the Windows side this is a thin wrapper
# rather than a hand-rolled hasher -- but a friend still needs something
# double-clickable, and `shasum -c` alone prints one line per file, which for
# 260 files buries the two lines that matter.
#
# TWO manifests, deliberately named apart:
#
#   SHA256SUMS-client   the small half -- korangar, launchers, archive, client
#   SHA256SUMS-assets   the big half -- the GRFs, lua_files.7z, BGM
#
# The two halves merge into ONE folder by design, so a shared name meant
# whichever copy lost took its half's coverage with it. Different names mean
# both survive the merge and this script checks everything at once.

set -u

here=$(cd "$(dirname "$0")" && pwd) || exit 1
cd "$here" || exit 1

hold() {
    printf '\n  Press return to close this window. '
    read -r _ 2>/dev/null || true
}

found=''
for name in SHA256SUMS-client SHA256SUMS-assets; do
    [ -f "$name" ] && found="$found $name"
done

if [ -z "$found" ]; then
    printf '\n  No checksum list here, so there is nothing to check against.\n'
    printf '  Run this inside the folder you downloaded, next to a SHA256SUMS- file.\n'
    hold
    exit 1
fi

bad=0
ok=0

for name in $found; do
    printf '\n  Checking against %s ...\n' "$name"

    # shasum prints "FAILED" for a mismatch and "FAILED open or read" for a
    # missing file; both are worth showing, and nothing else is.
    output=$(shasum -a 256 -c "$name" 2>&1)
    good=$(printf '%s\n' "$output" | grep -c ': OK$')
    ok=$((ok + good))

    printf '%s\n' "$output" | grep -v ': OK$' | grep -E 'FAILED|WARNING' | while read -r line; do
        printf '  %s\n' "$line"
    done

    failures=$(printf '%s\n' "$output" | grep -c 'FAILED')
    bad=$((bad + failures))
done

printf '\n'

if [ "$bad" -eq 0 ]; then
    printf '  All %s files match. This download is intact.\n' "$ok"
    hold
    exit 0
fi

printf '  %s good, %s bad.\n' "$ok" "$bad"
printf '  Download the affected folder again from the shared Drive folder.\n'
printf '  Names ending .grf, lua_files.7z or starting BGM/ are the big Assets\n'
printf '  download; anything else is the small macOS one.\n'
hold
exit 1
