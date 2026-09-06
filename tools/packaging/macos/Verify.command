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

fmt_size() {
    awk -v b="$1" 'BEGIN {
        if (b >= 1073741824) printf "%.1f GB", b / 1073741824
        else if (b >= 1048576) printf "%.0f MB", b / 1048576
        else if (b >= 1024) printf "%.0f KB", b / 1024
        else printf "%d B", b
    }'
}

for name in $found; do
    printf '\n  Checking against %s ...\n' "$name"
    printf '  Each file is named before it is hashed. Large GRFs can take a minute.\n'

    total=$(awk '/^[0-9a-fA-F]{64}/ { c++ } END { print c+0 }' "$name")
    n=0
    while read -r hash path rest; do
        case $hash in
            [0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]*) ;;
            *) continue ;;
        esac
        [ -n "$path" ] || continue
        path=${path#./}
        n=$((n + 1))
        if [ ! -e "$path" ]; then
            printf '  [%s/%s] MISSING  %s\n' "$n" "$total" "$path"
            bad=$((bad + 1))
            continue
        fi
        bytes=$(stat -f %z "$path")
        size=$(fmt_size "$bytes")
        if [ "$bytes" -gt 104857600 ]; then
            printf '  [%s/%s] %s (%s) -- large file, please wait...\n' "$n" "$total" "$path" "$size"
        else
            printf '  [%s/%s] %s (%s)\n' "$n" "$total" "$path" "$size"
        fi
        actual=$(shasum -a 256 "$path" | awk '{ print $1 }')
        if [ "$actual" = "$hash" ]; then
            ok=$((ok + 1))
        else
            printf '  CORRUPT  %s\n' "$path"
            bad=$((bad + 1))
        fi
    done < "$name"
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
