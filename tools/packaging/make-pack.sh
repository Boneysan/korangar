#!/usr/bin/env bash
# Assemble the friends pack: a folder to zip and drop on Google Drive.
#
#   tools/packaging/make-pack.sh --server 100.x.y.z
#   tools/packaging/make-pack.sh --server ro.example.com --skip-assets   # update pack
#
# Produces (see docs/plans/friends-distribution.md):
#
#   dist/Windows/     the ~82 MB half that changes every build
#   dist/Assets/      the ~3.7 GB half that almost never changes
#
# Friends drop both into ONE folder, so data.grf ends up beside Play.bat.
# Splitting them is the whole point: a client update must not cost 3.7 GB.
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
redist_url="https://aka.ms/vs/17/release/vc_redist.x64.exe"
redist="tools/packaging/windows/VC_redist.x64.exe"

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

# The client links the MSVC C++ runtime dynamically and cannot do otherwise:
# `+crt-static` fails to link because the vendored mach-dxcompiler (wgpu's DX12
# shader compiler) is a prebuilt static lib built against the dynamic CRT
# (`undefined symbol: __declspec(dllimport) nearbyint`). So the redistributable
# has to ship, or a friend on a fresh Windows gets a DLL dialog with no context.
# It is a Microsoft-redistributable component; ~25 MB, gitignored, fetched once.
if [ ! -f "$redist" ]; then
    echo "==> fetching the Visual C++ redistributable (once, ~25 MB)"
    curl -fsSL --max-time 300 -o "$redist" "$redist_url" \
        || die "could not download $redist_url -- fetch it by hand to $redist"
fi

windows="$out/Windows"
assets="$out/Assets"
rm -rf "$windows"
mkdir -p "$windows/client"

echo "==> Windows/"
cp "$exe" "$windows/korangar.exe"
cp tools/packaging/windows/Play.bat tools/packaging/windows/Play.ps1 \
   tools/packaging/windows/Setup.bat tools/packaging/windows/Setup.ps1 \
   tools/packaging/windows/Verify.bat tools/packaging/windows/Verify.ps1 \
   "tools/packaging/windows/READ ME FIRST.txt" "$redist" "$windows/"
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
    echo "==> Assets/ (~3.7 GB, slowest step)"
    mkdir -p "$assets"
    cp korangar/data.grf korangar/rdata.grf korangar/lua_files.7z "$assets/"
    for grf in renewal2021.grf resources2021.grf; do
        [ -f "$extra_grf_dir/$grf" ] || die "$extra_grf_dir/$grf not found (override with --assets-from)"
        cp "$extra_grf_dir/$grf" "$assets/"
    done
    rm -rf "$assets/BGM"
    cp -R korangar/BGM "$assets/BGM"
    # The 3.7 GB half is the one that arrives subtly incomplete, so it needs the
    # verifier more than the client folder does.
    cp tools/packaging/windows/Verify.bat tools/packaging/windows/Verify.ps1 "$assets/"
fi

[ -d "$assets" ] || die "$assets does not exist -- run once without --skip-assets"

# Integrity manifests, so a friend can tell a good download from a corrupted or
# swapped one. Drive downloads truncate, resume badly and get re-shared, and
# without these the only symptom is the client failing in some unrelated way.
#
# ONE MANIFEST PER HALF, AND THE NAMES MUST DIFFER. The two halves are merged
# into a single folder by design, so a shared `SHA256SUMS` meant Explorer
# offering Replace-or-Skip and whichever copy lost taking its half's coverage
# with it -- either Play.ps1's lua_files.7z check ("SHA256SUMS does not list
# lua_files.7z", a dead end on a perfectly good download) or every client file
# going unverified. Distinct names survive the merge and Verify.ps1 reads both.
#
# Always rewritten, even under --skip-assets: a manifest that lags the folder it
# describes is worse than none, because it reports a good download as corrupt.
# The pack that shipped 2026-08-17 did exactly that.
write_manifest() {
    local dir="$1"
    local name="$2"
    [ -d "$dir" ] || return 0
    echo "==> $dir/$name"
    (
        cd "$dir"
        rm -f SHA256SUMS "$name"
        # Sorted for a stable file, and excluding the manifest itself.
        find . -type f ! -name 'SHA256SUMS*' -print0 \
            | LC_ALL=C sort -z \
            | xargs -0 shasum -a 256 > "$name"
    )
}

write_manifest "$windows" SHA256SUMS-client
write_manifest "$assets" SHA256SUMS-assets

# A pack that leaks credentials is worse than no pack. login_settings.ron holds
# a real username and password in plaintext, so this is an assertion, not a
# comment: fail loudly rather than upload it.
leaked="$(find "$out" -name 'login_settings.ron' -o -name 'window_cache*.ron' 2>/dev/null || true)"
[ -z "$leaked" ] || die "personal files reached the pack: $leaked"

grep -q '\.\.' "$windows/client/game_archives.ron" && die "game_archives.ron still points outside the pack"

# THE CONTENTS CHECKLIST. Every entry here is a file whose absence a friend
# discovers instead of us: a missing launcher is a folder they cannot start, a
# missing READ ME is a support call, a missing redistributable is a DLL dialog.
# Add to this list whenever the pack gains a file -- that is the whole point of
# it, and it is cheaper than remembering.
require() {
    [ -e "$1" ] || die "the pack is missing $1 -- see the contents checklist in $0"
}

for f in korangar.exe Play.bat Play.ps1 Setup.bat Setup.ps1 Verify.bat Verify.ps1 \
         "READ ME FIRST.txt" VC_redist.x64.exe SHA256SUMS-client \
         archive client/server.ron client/game_archives.ron; do
    require "$windows/$f"
done

for f in data.grf rdata.grf renewal2021.grf resources2021.grf lua_files.7z \
         BGM SHA256SUMS-assets Verify.bat Verify.ps1; do
    require "$assets/$f"
done

# The launchers name these; a rename on one side only is silent until a friend
# hits it. Cheap to pin, so pin it.
grep -q 'SHA256SUMS-assets' "$windows/Play.ps1" \
    || die "Play.ps1 does not read SHA256SUMS-assets -- the manifest names have drifted"
grep -q 'SHA256SUMS-assets' "$windows/Setup.ps1" \
    || die "Setup.ps1 does not read SHA256SUMS-assets -- the manifest names have drifted"

# BGM is loaded off the filesystem (case-insensitively), not out of a GRF, so a
# manifest that stops at the top level silently leaves 345 MB unverified. That
# also shipped in the 2026-08-17 pack.
bgm_lines="$(grep -c 'BGM/' "$assets/SHA256SUMS-assets" || true)"
[ "$bgm_lines" -gt 100 ] || die "SHA256SUMS-assets lists only $bgm_lines BGM files -- the manifest is not covering the folder"

echo
echo "pack ready:"
du -sh "$windows" 2>/dev/null || true
du -sh "$assets" 2>/dev/null || true
echo
echo "  server:  $address:$port"
echo "  client:  $(grep -c . "$windows/SHA256SUMS-client") files"
echo "  assets:  $(grep -c . "$assets/SHA256SUMS-assets") files"
echo
echo "zip Windows/ and upload it; upload Assets/ as a folder."
echo "Do NOT re-compress the GRFs -- they are already compressed, so zipping"
echo "Assets costs a long wait and saves almost nothing."
