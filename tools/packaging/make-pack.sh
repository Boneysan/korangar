#!/usr/bin/env bash
# Assemble the friends pack: a folder to zip and drop on Google Drive.
#
#   tools/packaging/make-pack.sh --server 100.x.y.z
#   tools/packaging/make-pack.sh --server ro.example.com --skip-assets   # update pack
#
# Produces (see docs/plans/friends-distribution.md):
#
#   dist/Windows/     the ~50 MB half that changes every build
#   dist/Assets/      the ~3.6 GB half that almost never changes
#
# Friends drop both into ONE folder, so data.grf ends up beside Play.bat.
# Splitting them is the whole point: a client update must not cost 3.6 GB.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && cd .. && pwd)"
cd "$repo_root"

server=""
out="dist"
# The 2021 GRFs are NOT in this tree; game_archives.ron reaches out to them with
# a relative path that is meaningless inside a pack.
extra_grf_dir="/Volumes/T7/GitHub/RO/client"
skip_assets=0
do_build=0
target="x86_64-pc-windows-msvc"

die() { printf '\nerror: %s\n\n' "$1" >&2; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --server) server="${2:-}"; shift 2 ;;
        --out) out="${2:-}"; shift 2 ;;
        --assets-from) extra_grf_dir="${2:-}"; shift 2 ;;
        --skip-assets) skip_assets=1; shift ;;
        --build) do_build=1; shift ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done

[ -n "$server" ] || die "--server is required. Friends never edit server.ron, so it ships filled in."

address="${server%%:*}"
port="${server#*:}"
[ "$port" = "$server" ] && port=6900

if [ "$do_build" -eq 1 ]; then
    # Homebrew LLVM is keg-only, so clang-cl/llvm-lib are not on PATH by
    # default and three build scripts fail without them.
    export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
    echo "==> building $target (this is slow the first time)"
    cargo xwin build --release --target "$target" --bin korangar \
        --features unicode --xwin-include-atl
fi

exe="target/$target/release/korangar.exe"
[ -f "$exe" ] || die "$exe not found. Pass --build, or see docs/plans/friends-distribution.md section 9."

windows="$out/Windows"
assets="$out/Assets"
rm -rf "$windows"
mkdir -p "$windows/client"

echo "==> Windows/"
cp "$exe" "$windows/korangar.exe"
cp tools/packaging/windows/Play.bat tools/packaging/windows/Play.ps1 \
   tools/packaging/windows/Verify.bat tools/packaging/windows/Verify.ps1 "$windows/"
cp -R korangar/archive "$windows/archive"

# Written, never copied: the working copy points outside the pack.
cat > "$windows/client/game_archives.ron" <<'RON'
(
    archives: [
        "data.grf",
        "rdata.grf",
        "renewal2021.grf",
        "resources2021.grf",
        "archive/",
    ],
)
RON

cat > "$windows/client/server.ron" <<RON
(
    address: "$address",
    port: $port,
    name: "Seal Cascade",
)
RON

if [ "$skip_assets" -eq 0 ]; then
    echo "==> Assets/ (~3.6 GB, slowest step)"
    mkdir -p "$assets"
    cp korangar/data.grf korangar/rdata.grf korangar/lua_files.7z "$assets/"
    for grf in renewal2021.grf resources2021.grf; do
        [ -f "$extra_grf_dir/$grf" ] || die "$extra_grf_dir/$grf not found (override with --assets-from)"
        cp "$extra_grf_dir/$grf" "$assets/"
    done
    cp -R korangar/BGM "$assets/BGM"
    # The 3.6 GB half is the one that arrives subtly incomplete, so it needs the
    # verifier more than the client folder does.
    cp tools/packaging/windows/Verify.bat tools/packaging/windows/Verify.ps1 "$assets/"
fi

# Integrity manifests, so a friend can tell a good download from a corrupted or
# swapped one. Drive downloads truncate, resume badly and get re-shared, and
# without these the only symptom is the client failing in some unrelated way.
#
# One manifest per folder rather than one for the pack, because the two halves
# are downloaded separately and updated on completely different schedules.
write_manifest() {
    local dir="$1"
    [ -d "$dir" ] || return 0
    echo "==> $dir/SHA256SUMS"
    (
        cd "$dir"
        # Sorted for a stable file, and excluding the manifest itself.
        find . -type f ! -name 'SHA256SUMS' -print0 \
            | LC_ALL=C sort -z \
            | xargs -0 shasum -a 256 > SHA256SUMS
    )
}

write_manifest "$windows"
[ "$skip_assets" -eq 0 ] && write_manifest "$assets"

# A pack that leaks credentials is worse than no pack. login_settings.ron holds
# a real username and password in plaintext, so this is an assertion, not a
# comment: fail loudly rather than upload it.
leaked="$(find "$out" -name 'login_settings.ron' -o -name 'window_cache*.ron' 2>/dev/null || true)"
[ -z "$leaked" ] || die "personal files reached the pack: $leaked"

grep -q '\.\.' "$windows/client/game_archives.ron" && die "game_archives.ron still points outside the pack"

echo
echo "pack ready:"
du -sh "$windows" 2>/dev/null || true
[ "$skip_assets" -eq 0 ] && du -sh "$assets" 2>/dev/null || true
echo
echo "zip Windows/ and upload it; upload Assets/ as a folder."
echo "Do NOT re-compress the GRFs -- they are already compressed, so zipping"
echo "Assets costs a long wait and saves almost nothing."
