#!/bin/sh
# Seal Cascade script repair for macOS.
set -eu

here=$(cd "$(dirname "$0")" && pwd) || exit 1
source_dir="${2:-$here}"
dest="${1:-}"

if [ -z "$dest" ]; then
    if [ -e "$here/korangar" ] || [ -e "$here/korangar.exe" ] || [ -e "$here/data.grf" ]; then
        dest="$here"
    else
        parent=$(dirname "$here")
        if [ -e "$parent/korangar" ] || [ -e "$parent/korangar.exe" ] || [ -e "$parent/data.grf" ]; then
            dest="$parent"
        fi
    fi
fi

if [ -z "$dest" ] || [ ! -d "$dest" ]; then
    printf '\n  Could not determine target game installation folder.\n'
    printf '  Run: ./Repair.command /path/to/game\n\n'
    exit 1
fi

manifest=""
for m in SHA256SUMS-repair SHA256SUMS-client; do
    if [ -f "$source_dir/$m" ]; then
        manifest="$source_dir/$m"
        break
    fi
done

if [ -z "$manifest" ]; then
    printf '\n  No repair manifest found in %s\n\n' "$source_dir"
    exit 1
fi

printf '\n  ============================================\n'
printf '   Seal Cascade - script repair\n'
printf '  ============================================\n'
printf '  Validating repair bundle integrity before making changes...\n'

# 1. Validate repair source
while read -r hash path rest; do
    case $hash in
        [0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]*) ;;
        *) continue ;;
    esac
    [ -n "$path" ] || continue
    path=${path#./}
    case "$path" in
        *.grf|*.7z|korangar|korangar.exe) continue ;;
    esac

    src_file="$source_dir/$path"
    if [ ! -f "$src_file" ]; then
        printf '  ERROR: Repair bundle missing %s. Aborting without changes.\n\n' "$path"
        exit 1
    fi
    actual=$(shasum -a 256 "$src_file" | awk '{print tolower($1)}')
    expected=$(echo "$hash" | tr '[:upper:]' '[:lower:]')
    if [ "$actual" != "$expected" ]; then
        printf '  ERROR: Repair bundle file %s is corrupt. Aborting without changes.\n\n' "$path"
        exit 1
    fi
done < "$manifest"

printf '  Repair bundle is intact. Checking installation in %s ...\n' "$dest"

# 2. Identify and stage repairs
repaired=0
while read -r hash path rest; do
    case $hash in
        [0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]*) ;;
        *) continue ;;
    esac
    [ -n "$path" ] || continue
    path=${path#./}
    case "$path" in
        *.grf|*.7z|korangar|korangar.exe) continue ;;
    esac

    src_file="$source_dir/$path"
    target_file="$dest/$path"
    expected=$(echo "$hash" | tr '[:upper:]' '[:lower:]')

    needs_repair=0
    if [ ! -f "$target_file" ]; then
        printf '  missing: %s\n' "$path"
        needs_repair=1
    else
        actual=$(shasum -a 256 "$target_file" | awk '{print tolower($1)}')
        if [ "$actual" != "$expected" ]; then
            printf '  corrupt: %s\n' "$path"
            needs_repair=1
        fi
    fi

    if [ "$needs_repair" -eq 1 ]; then
        tmp_file="$dest/$path.repair-tmp"
        mkdir -p "$(dirname "$tmp_file")"
        cp "$src_file" "$tmp_file"
        
        # Verify staged bytes
        staged=$(shasum -a 256 "$tmp_file" | awk '{print tolower($1)}')
        if [ "$staged" != "$expected" ]; then
            rm -f "$tmp_file"
            printf '  ERROR: Staged verification failed for %s. Aborting.\n\n' "$path"
            exit 1
        fi

        mv -f "$tmp_file" "$target_file"
        chmod +x "$target_file" 2>/dev/null || true
        printf '  repaired: %s\n' "$path"
        repaired=$((repaired + 1))
    fi
done < "$manifest"

printf '\n  Repair complete: %s file(s) restored.\n' "$repaired"
printf '  Large game assets (GRFs, BGM) were untouched.\n\n'
exit 0
