#!/usr/bin/env bash
# Assemble the friends pack: a folder to zip and drop on Google Drive.
#
#   tools/packaging/make-pack.sh --server 100.x.y.z
#   tools/packaging/make-pack.sh --server ro.example.com --skip-assets   # update pack
#   tools/packaging/make-pack.sh --server 100.x.y.z --merged             # + one zip
#   tools/packaging/make-pack.sh --server 100.x.y.z --os macos --merged  # the Mac pack
#
# Produces (see docs/plans/friends-distribution.md):
#
#   dist/Windows/     the ~82 MB half that changes every build   (--os windows, default)
#   dist/macOS/       the same half for Apple Silicon            (--os macos)
#   dist/Assets/      the ~3.7 GB half that almost never changes (shared by both)
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
do_merged=0
os="windows"
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
        --merged) do_merged=1; shift ;;
        --os) os="${2:-}"; shift 2 ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done

[ -n "$server" ] || die "--server is required. Friends never edit server.ron, so it ships filled in."

# One pack per OS. The Assets half is shared -- it is the same 3.7 GB of
# Gravity artwork either way -- so only the small half is OS-specific.
case "$os" in
    windows)
        target="x86_64-pc-windows-msvc"
        exe_name="korangar.exe"
        half_name="Windows"
        ;;
    macos)
        # Apple Silicon only. An Intel Mac needs x86_64-apple-darwin and a
        # `lipo` step; say so in the READ ME rather than shipping a binary
        # that dies with "Bad CPU type in executable".
        target="aarch64-apple-darwin"
        exe_name="korangar"
        half_name="macOS"
        ;;
    *) die "unknown --os: $os (expected windows or macos)" ;;
esac

address="${server%%:*}"
port="${server#*:}"
[ "$port" = "$server" ] && port=6900

if [ "$do_build" -eq 1 ]; then
    echo "==> building $target (this is slow the first time)"
    if [ "$os" = "windows" ]; then
        # Homebrew LLVM is keg-only, so clang-cl/llvm-lib are not on PATH by
        # default and three build scripts fail without them.
        export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
        cargo xwin build --release --target "$target" --bin korangar \
            --features unicode --xwin-include-atl
    else
        cargo build --release --target "$target" --bin korangar --features unicode
    fi
fi

exe="target/$target/release/$exe_name"
[ -f "$exe" ] || die "$exe not found. Pass --build, or see docs/plans/friends-distribution.md section 9."

# The client links the MSVC C++ runtime dynamically and cannot do otherwise:
# `+crt-static` fails to link because the vendored mach-dxcompiler (wgpu's DX12
# shader compiler) is a prebuilt static lib built against the dynamic CRT
# (`undefined symbol: __declspec(dllimport) nearbyint`). So the redistributable
# has to ship, or a friend on a fresh Windows gets a DLL dialog with no context.
# It is a Microsoft-redistributable component; ~25 MB, gitignored, fetched once.
if [ "$os" = "windows" ] && [ ! -f "$redist" ]; then
    echo "==> fetching the Visual C++ redistributable (once, ~25 MB)"
    curl -fsSL --max-time 300 -o "$redist" "$redist_url" \
        || die "could not download $redist_url -- fetch it by hand to $redist"
fi

windows="$out/$half_name"
assets="$out/Assets"
rm -rf "$windows"
mkdir -p "$windows/client"

echo "==> $half_name/"
cp "$exe" "$windows/$exe_name"

if [ "$os" = "windows" ]; then
    cp tools/packaging/windows/Play.bat tools/packaging/windows/Play.ps1 \
       tools/packaging/windows/Setup.bat tools/packaging/windows/Setup.ps1 \
       tools/packaging/windows/Verify.bat tools/packaging/windows/Verify.ps1 \
       "tools/packaging/windows/READ ME FIRST.txt" "$redist" "$windows/"
else
    cp tools/packaging/macos/Play.command tools/packaging/macos/Setup.command \
       tools/packaging/macos/Verify.command \
       "tools/packaging/macos/READ ME FIRST.txt" "$windows/"
    # A .command without the execute bit opens in TextEdit, which looks exactly
    # like "nothing happened". zip preserves the mode; Finder's unzip restores
    # it. The binary needs it for the same reason.
    chmod +x "$windows"/*.command "$windows/$exe_name"
fi

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
fi

[ -d "$assets" ] || die "$assets does not exist -- run once without --skip-assets"

# The 3.7 GB half is the one that arrives subtly incomplete, so it needs the
# verifier more than the client folder does. Both OSes' verifiers ship, because
# Assets is downloaded once and shared by every friend whatever they run.
#
# Refreshed even under --skip-assets: a verifier that lags the script it was
# copied from is the same class of bug as a manifest that lags its folder.
cp tools/packaging/windows/Verify.bat tools/packaging/windows/Verify.ps1 \
   tools/packaging/macos/Verify.command "$assets/"
chmod +x "$assets/Verify.command"

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

if [ "$os" = "windows" ]; then
    for f in korangar.exe Play.bat Play.ps1 Setup.bat Setup.ps1 Verify.bat Verify.ps1 \
             "READ ME FIRST.txt" VC_redist.x64.exe SHA256SUMS-client \
             archive client/server.ron client/game_archives.ron; do
        require "$windows/$f"
    done
else
    for f in korangar Play.command Setup.command Verify.command \
             "READ ME FIRST.txt" SHA256SUMS-client \
             archive client/server.ron client/game_archives.ron; do
        require "$windows/$f"
    done

    # A launcher without the execute bit opens in TextEdit. Silent, and it
    # looks like the download is broken.
    for f in Play.command Setup.command Verify.command korangar; do
        [ -x "$windows/$f" ] || die "$windows/$f is not executable -- Finder would open it as text"
    done
fi

for f in data.grf rdata.grf renewal2021.grf resources2021.grf lua_files.7z \
         BGM SHA256SUMS-assets Verify.bat Verify.ps1 Verify.command; do
    require "$assets/$f"
done

# The launchers name these; a rename on one side only is silent until a friend
# hits it. Cheap to pin, so pin it.
if [ "$os" = "windows" ]; then
    launcher="Play.ps1"; setup="Setup.ps1"
else
    launcher="Play.command"; setup="Setup.command"
fi
grep -q 'SHA256SUMS-assets' "$windows/$launcher" \
    || die "$launcher does not read SHA256SUMS-assets -- the manifest names have drifted"
grep -q 'SHA256SUMS-assets' "$windows/$setup" \
    || die "$setup does not read SHA256SUMS-assets -- the manifest names have drifted"

# BGM is loaded off the filesystem (case-insensitively), not out of a GRF, so a
# manifest that stops at the top level silently leaves 345 MB unverified. That
# also shipped in the 2026-08-17 pack.
bgm_lines="$(grep -c 'BGM/' "$assets/SHA256SUMS-assets" || true)"
[ "$bgm_lines" -gt 100 ] || die "SHA256SUMS-assets lists only $bgm_lines BGM files -- the manifest is not covering the folder"

# The first-time download: ONE zip a friend unpacks and plays from, with no
# merging step of their own. Decision S3's "optional merged first-time zip".
#
# Built here rather than by hand, so it inherits the contents checklist above --
# a hand-assembled upload is exactly the artifact nobody checks. Updates still
# use the small Windows half; that is the whole reason the split exists.
if [ "$do_merged" -eq 1 ]; then
    # Named per OS so both packs can sit in dist/ at once. Windows keeps the
    # bare name it has always had, because that link is already shared.
    if [ "$os" = "windows" ]; then
        merged_name="Seal Cascade"
    else
        merged_name="Seal Cascade (macOS)"
    fi

    merged="$out/$merged_name"
    echo "==> $merged/"
    rm -rf "$merged" "$out/$merged_name.zip"
    mkdir -p "$merged"
    # -c clones on APFS, so staging 3.7 GB costs no space and no time.
    cp -Rc "$windows/." "$merged/"
    cp -Rc "$assets/." "$merged/"

    # Both manifests have to survive the merge -- that is the entire reason
    # they are named apart (S12). If this ever fails, the rename regressed.
    require "$merged/SHA256SUMS-client"
    require "$merged/SHA256SUMS-assets"

    # The two halves deliberately SHARE a few files -- the verifier ships in
    # both, so a friend can check the big download before merging it. A shared
    # file must therefore be byte-identical, or the merge silently keeps one
    # copy and the other half's manifest now describes a file that is not there.
    for shared in $(cd "$windows" && find . -type f ! -name 'SHA256SUMS*' | sed 's|^\./||'); do
        [ -f "$assets/$shared" ] || continue
        cmp -s "$windows/$shared" "$assets/$shared" \
            || die "$shared differs between the two halves -- merging would break one manifest"
    done

    # Count DISTINCT paths, not the sum of the two manifests: a file listed in
    # both is one file on disk after the merge. Summing happened to be right
    # for Windows, where two shared files cancelled the two manifest files
    # exactly, and was off by one the moment the macOS half shared only one.
    merged_files="$(find "$merged" -type f | wc -l | tr -d ' ')"
    distinct="$(cat "$windows/SHA256SUMS-client" "$assets/SHA256SUMS-assets" | cut -c67- | sort -u | grep -c .)"
    expected_files="$(( distinct + 2 ))"
    [ "$merged_files" = "$expected_files" ] \
        || die "merged folder has $merged_files files, the manifests describe $distinct distinct paths (+2 manifests)"

    # Storing the already-compressed payload instead of deflating it: the GRFs,
    # the 7z and the mp3s do not shrink, and compressing 3.7 GB of them costs
    # many minutes to save nothing. The exe and archive/ still compress.
    echo "==> $out/$merged_name.zip"
    ( cd "$out" && zip -r -X -q -n .grf:.7z:.mp3 "$merged_name.zip" "$merged_name" -x '*.DS_Store' )
fi

echo
echo "pack ready:"
du -sh "$windows" 2>/dev/null || true
du -sh "$assets" 2>/dev/null || true
echo
echo "  server:  $address:$port"
echo "  client:  $(grep -c . "$windows/SHA256SUMS-client") files"
echo "  assets:  $(grep -c . "$assets/SHA256SUMS-assets") files"
echo
if [ "$do_merged" -eq 1 ]; then
    du -sh "$out/$merged_name.zip" 2>/dev/null || true
    echo
    echo "first-time download: upload '$merged_name.zip' -- one file, they unzip and play."
    echo "updates: zip $half_name/ on its own (80 MB), so nobody re-downloads 3.7 GB."
else
    echo "zip $half_name/ and upload it; upload Assets/ as a folder."
    echo "Do NOT re-compress the GRFs -- they are already compressed, so zipping"
    echo "Assets costs a long wait and saves almost nothing."
fi
