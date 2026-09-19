#!/usr/bin/env bash
# Test harness for QW-012: provenance-aware verification and running-verifier safety.
# Exercises:
#   1. Correct installation: both halves present and matching -> Exit 0.
#   2. Missing files: missing client and asset files -> Identifies [Client] and [Assets],
#      prints package half, manifest path, expected hash, actual hash, installed path. Exit 1.
#   3. Corrupt files: corrupt client and asset files -> Identifies [Client] and [Assets],
#      prints package half, manifest path, expected hash, actual hash, installed path. Exit 1.
#   4. Conflicting shared files & verifier ownership:
#      - Stale Verify.ps1 in SHA256SUMS-assets ignored (single client ownership).
#      - Conflicting shared file isolates exact package half and manifest expectation.
#   5. Known-good external verifier & self-overwrite protection:
#      - External verifier validates target directory without being overwritten.
#      - Running script cannot be overwritten in-place during copy/update.
#   6. Setup.ps1 provenance verification
#   7. macOS Verify.command parity

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PWSH="${PWSH:-pwsh}"

if ! command -v "$PWSH" >/dev/null 2>&1; then
    echo "ERROR: $PWSH not found on PATH. Required for testing PowerShell packaging scripts." >&2
    exit 1
fi

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/verifier-provenance-test.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

VERIFY_PS1="$REPO_ROOT/tools/packaging/windows/Verify.ps1"
SETUP_PS1="$REPO_ROOT/tools/packaging/windows/Setup.ps1"
UPDATE_PS1="$REPO_ROOT/tools/packaging/windows/Update.ps1"

echo "=== Running QW-012 Verifier Provenance Test Suite ==="
echo "Tmp dir: $TMP_ROOT"
echo "PowerShell: $("$PWSH" --version)"

# Helper to compute uppercase SHA-256
calc_sha256() {
    shasum -a 256 "$1" | awk '{print toupper($1)}'
}

# -----------------------------------------------------------------------------
# Case 1: Correct installation (both halves intact)
# -----------------------------------------------------------------------------
echo ""
echo "--- Case 1: Correct installation (both halves intact) ---"
DIR_C1="$TMP_ROOT/case1"
mkdir -p "$DIR_C1/client"

echo "korangar binary bytes" > "$DIR_C1/korangar.exe"
echo "server ron config" > "$DIR_C1/client/server.ron"
echo "data grf payload" > "$DIR_C1/data.grf"
echo "lua 7z payload" > "$DIR_C1/lua_files.7z"
cp "$VERIFY_PS1" "$DIR_C1/Verify.ps1"

# Write SHA256SUMS-client
{
    echo "$(calc_sha256 "$DIR_C1/korangar.exe")  korangar.exe"
    echo "$(calc_sha256 "$DIR_C1/client/server.ron")  client/server.ron"
    echo "$(calc_sha256 "$DIR_C1/Verify.ps1")  Verify.ps1"
} > "$DIR_C1/SHA256SUMS-client"

# Write SHA256SUMS-assets
{
    echo "$(calc_sha256 "$DIR_C1/data.grf")  data.grf"
    echo "$(calc_sha256 "$DIR_C1/lua_files.7z")  lua_files.7z"
} > "$DIR_C1/SHA256SUMS-assets"

OUT_C1="$("$PWSH" -File "$DIR_C1/Verify.ps1" 2>&1)" || {
    echo "FAILED: Case 1 expected exit code 0, got failure."
    echo "$OUT_C1"
    exit 1
}

echo "$OUT_C1" | grep -q "All 5 files match. This download is intact." || {
    echo "FAILED: Case 1 missing success message."
    echo "$OUT_C1"
    exit 1
}
echo "PASS: Case 1 passed cleanly (exit 0, 5/5 matched)."

# -----------------------------------------------------------------------------
# Case 2: Missing files (identifies responsible package half & provenance)
# -----------------------------------------------------------------------------
echo ""
echo "--- Case 2: Missing files (provenance check) ---"
DIR_C2="$TMP_ROOT/case2"
cp -R "$DIR_C1" "$DIR_C2"

# Delete one client file and one asset file
rm -f "$DIR_C2/korangar.exe"
rm -f "$DIR_C2/data.grf"

set +e
OUT_C2="$("$PWSH" -File "$DIR_C2/Verify.ps1" 2>&1)"
CODE_C2=$?
set -e

if [ "$CODE_C2" -eq 0 ]; then
    echo "FAILED: Case 2 expected exit code 1 for missing files, got 0."
    echo "$OUT_C2"
    exit 1
fi

# Check Client provenance output
echo "$OUT_C2" | grep -E -q "\[Client\] MISSING" || {
    echo "FAILED: Case 2 did not tag missing korangar.exe with [Client]."
    echo "$OUT_C2"
    exit 1
}
echo "$OUT_C2" | grep -q "Package Half:   Client" || {
    echo "FAILED: Case 2 missing 'Package Half:   Client'."
    echo "$OUT_C2"
    exit 1
}
echo "$OUT_C2" | grep -q "SHA256SUMS-client" || {
    echo "FAILED: Case 2 missing manifest path for client."
    echo "$OUT_C2"
    exit 1
}
echo "$OUT_C2" | grep -q "Actual Hash:    <none>" || {
    echo "FAILED: Case 2 missing '<none>' actual hash for missing file."
    echo "$OUT_C2"
    exit 1
}

# Check Assets provenance output
echo "$OUT_C2" | grep -E -q "\[Assets\] MISSING" || {
    echo "FAILED: Case 2 did not tag missing data.grf with [Assets]."
    echo "$OUT_C2"
    exit 1
}
echo "$OUT_C2" | grep -q "Package Half:   Assets" || {
    echo "FAILED: Case 2 missing 'Package Half:   Assets'."
    echo "$OUT_C2"
    exit 1
}
echo "$OUT_C2" | grep -q "SHA256SUMS-assets" || {
    echo "FAILED: Case 2 missing manifest path for assets."
    echo "$OUT_C2"
    exit 1
}

# Check summary breakdown
echo "$OUT_C2" | grep -q "Client package (1 failure(s))" || {
    echo "FAILED: Case 2 summary missing Client package breakdown."
    echo "$OUT_C2"
    exit 1
}
echo "$OUT_C2" | grep -q "Assets package (1 failure(s))" || {
    echo "FAILED: Case 2 summary missing Assets package breakdown."
    echo "$OUT_C2"
    exit 1
}
echo "PASS: Case 2 properly isolated missing client vs assets files with full provenance."

# -----------------------------------------------------------------------------
# Case 3: Corrupt files (identifies responsible package half, expected & actual hashes)
# -----------------------------------------------------------------------------
echo ""
echo "--- Case 3: Corrupt files (hash mismatch provenance) ---"
DIR_C3="$TMP_ROOT/case3"
cp -R "$DIR_C1" "$DIR_C3"

# Corrupt client file and asset file
echo "CORRUPTED CLIENT CONTENT" >> "$DIR_C3/client/server.ron"
echo "CORRUPTED ASSET CONTENT" >> "$DIR_C3/lua_files.7z"

set +e
OUT_C3="$("$PWSH" -File "$DIR_C3/Verify.ps1" 2>&1)"
CODE_C3=$?
set -e

if [ "$CODE_C3" -eq 0 ]; then
    echo "FAILED: Case 3 expected exit code 1 for corrupt files, got 0."
    echo "$OUT_C3"
    exit 1
fi

echo "$OUT_C3" | grep -E -q "\[Client\] CORRUPT: client[/\\]server\.ron" || {
    echo "FAILED: Case 3 did not tag corrupt client/server.ron with [Client]."
    echo "$OUT_C3"
    exit 1
}
echo "$OUT_C3" | grep -E -q "\[Assets\] CORRUPT: lua_files\.7z" || {
    echo "FAILED: Case 3 did not tag corrupt lua_files.7z with [Assets]."
    echo "$OUT_C3"
    exit 1
}

ACTUAL_CLIENT_HASH="$(calc_sha256 "$DIR_C3/client/server.ron")"
echo "$OUT_C3" | grep -qi "$ACTUAL_CLIENT_HASH" || {
    echo "FAILED: Case 3 did not output actual hash $ACTUAL_CLIENT_HASH."
    echo "$OUT_C3"
    exit 1
}

ACTUAL_ASSET_HASH="$(calc_sha256 "$DIR_C3/lua_files.7z")"
echo "$OUT_C3" | grep -qi "$ACTUAL_ASSET_HASH" || {
    echo "FAILED: Case 3 did not output actual hash $ACTUAL_ASSET_HASH."
    echo "$OUT_C3"
    exit 1
}
echo "PASS: Case 3 properly reported corrupt hashes and package provenance."

# -----------------------------------------------------------------------------
# Case 4: Conflicting shared files & verifier ownership
# -----------------------------------------------------------------------------
echo ""
echo "--- Case 4a: Stale Verify.ps1 in SHA256SUMS-assets ignored (single ownership) ---"
DIR_C4A="$TMP_ROOT/case4a"
cp -R "$DIR_C1" "$DIR_C4A"

# Simulate the exact playtest failure: SHA256SUMS-assets contains stale Verify.ps1
# hash from previous install while Verify.ps1 was updated by client patch
echo "1111111111111111111111111111111111111111111111111111111111111111  Verify.ps1" >> "$DIR_C4A/SHA256SUMS-assets"

OUT_C4A="$("$PWSH" -File "$DIR_C4A/Verify.ps1" 2>&1)" || {
    echo "FAILED: Case 4a expected exit code 0 because SHA256SUMS-client owns Verify.ps1."
    echo "$OUT_C4A"
    exit 1
}
echo "$OUT_C4A" | grep -q "All 5 files match. This download is intact." || {
    echo "FAILED: Case 4a failed to ignore stale asset verifier entry."
    echo "$OUT_C4A"
    exit 1
}
echo "PASS: Case 4a successfully resolved shared verifier ownership to Client manifest."

echo ""
echo "--- Case 4b: Conflicting non-verifier shared file across manifests ---"
DIR_C4B="$TMP_ROOT/case4b"
cp -R "$DIR_C1" "$DIR_C4B"

echo "shared text file" > "$DIR_C4B/shared-notice.txt"
HASH_SHARED="$(calc_sha256 "$DIR_C4B/shared-notice.txt")"
BOGUS_HASH="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"

# Client has true hash, Assets has conflicting bogus hash
echo "$HASH_SHARED  shared-notice.txt" >> "$DIR_C4B/SHA256SUMS-client"
echo "$BOGUS_HASH  shared-notice.txt" >> "$DIR_C4B/SHA256SUMS-assets"

set +e
OUT_C4B="$("$PWSH" -File "$DIR_C4B/Verify.ps1" 2>&1)"
CODE_C4B=$?
set -e

if [ "$CODE_C4B" -eq 0 ]; then
    echo "FAILED: Case 4b expected failure on conflicting shared file in assets."
    echo "$OUT_C4B"
    exit 1
fi

echo "$OUT_C4B" | grep -q "\[Assets\] CORRUPT: shared-notice.txt" || {
    echo "FAILED: Case 4b did not attribute conflicting file error to [Assets]."
    echo "$OUT_C4B"
    exit 1
}
echo "$OUT_C4B" | grep -q "Assets package (1 failure(s))" || {
    echo "FAILED: Case 4b did not record exactly 1 failure for Assets package."
    echo "$OUT_C4B"
    exit 1
}
# Make sure Client is NOT blamed
echo "$OUT_C4B" | grep -q "Client package" && {
    echo "FAILED: Case 4b falsely blamed Client package."
    echo "$OUT_C4B"
    exit 1
}
echo "PASS: Case 4b accurately isolated conflicting shared file to Assets manifest."

# -----------------------------------------------------------------------------
# Case 5: External known-good verifier & self-overwrite protection
# -----------------------------------------------------------------------------
echo ""
echo "--- Case 5: External known-good verifier & self-overwrite protection ---"
DIR_C5_INSTALL="$TMP_ROOT/case5_install"
DIR_C5_EXTERNAL="$TMP_ROOT/case5_external"
mkdir -p "$DIR_C5_EXTERNAL"
cp -R "$DIR_C1" "$DIR_C5_INSTALL"
cp "$VERIFY_PS1" "$DIR_C5_EXTERNAL/Verify.ps1"

# Target installation has a corrupt Verify.ps1
echo "# Corrupted internal verifier" > "$DIR_C5_INSTALL/Verify.ps1"
CORRUPT_INTERNAL_HASH="$(calc_sha256 "$DIR_C5_INSTALL/Verify.ps1")"
EXTERNAL_HASH_BEFORE="$(calc_sha256 "$DIR_C5_EXTERNAL/Verify.ps1")"

# Run external known-good verifier targeting the installation
set +e
OUT_C5="$("$PWSH" -File "$DIR_C5_EXTERNAL/Verify.ps1" -TargetDirectory "$DIR_C5_INSTALL" 2>&1)"
CODE_C5=$?
set -e

if [ "$CODE_C5" -eq 0 ]; then
    echo "FAILED: Case 5 expected failure because target installation's Verify.ps1 is corrupt."
    echo "$OUT_C5"
    exit 1
fi

echo "$OUT_C5" | grep -q "\[Client\] CORRUPT: Verify.ps1" || {
    echo "FAILED: Case 5 did not report corrupt internal Verify.ps1."
    echo "$OUT_C5"
    exit 1
}

# Ensure the external running verifier was NOT overwritten or modified
EXTERNAL_HASH_AFTER="$(calc_sha256 "$DIR_C5_EXTERNAL/Verify.ps1")"
if [ "$EXTERNAL_HASH_BEFORE" != "$EXTERNAL_HASH_AFTER" ]; then
    echo "FAILED: Running external verifier was modified! $EXTERNAL_HASH_BEFORE != $EXTERNAL_HASH_AFTER"
    exit 1
fi

# Test Update.ps1 running script self-overwrite protection
DIR_C5_UPDATE="$TMP_ROOT/case5_update"
mkdir -p "$DIR_C5_UPDATE"
cp "$UPDATE_PS1" "$DIR_C5_UPDATE/Update.ps1"
echo "new korangar" > "$DIR_C5_UPDATE/korangar.exe"
# Run command checking self-overwrite guard logic
OUT_UPDATE="$("$PWSH" -Command "& {
    \$dest = '$DIR_C5_UPDATE';
    \$here = '$DIR_C5_UPDATE';
    \$runningScript = Join-Path \$here 'Update.ps1';
    \$to = Join-Path \$dest 'Update.ps1';
    if (\$runningScript -eq \$to) {
        Write-Host 'PROTECTED: Self-overwrite prevented'
    }
}" 2>&1)"

echo "$OUT_UPDATE" | grep -q "PROTECTED: Self-overwrite prevented" || {
    echo "FAILED: Self-overwrite protection check failed."
    echo "$OUT_UPDATE"
    exit 1
}

echo "PASS: Case 5 proved running verifier remains safe outside target files and self-overwrite is guarded."

# -----------------------------------------------------------------------------
# Case 6: Setup.ps1 provenance verification
# -----------------------------------------------------------------------------
echo ""
echo "--- Case 6: Setup.ps1 provenance output ---"
DIR_C6="$TMP_ROOT/case6"
cp -R "$DIR_C1" "$DIR_C6"
rm -f "$DIR_C6/korangar.exe"

# Run Test-Manifest from Setup.ps1
OUT_C6="$("$PWSH" -Command "& {
    \$here = '$DIR_C6';
    function Say(\$t) { }
    function Good(\$t) { }
    function Format-Size(\$b) { return \$b.ToString() }
    function Step(\$n, \$t) { }
    function Fail(\$what, \$fix) {
        Write-Host ('FAIL: ' + \$what)
        foreach (\$line in \$fix) { Write-Host ('  ' + \$line) }
    }
    
    # Extract Test-Manifest from Setup.ps1
    \$setupContent = Get-Content -LiteralPath '$SETUP_PS1' -Raw
    \$startIndex = \$setupContent.IndexOf('function Test-Manifest')
    \$endIndex = \$setupContent.IndexOf('Step 5')
    \$testManifestSnippet = \$setupContent.Substring(\$startIndex, \$endIndex - \$startIndex)
    Invoke-Expression \$testManifestSnippet
}" 2>&1 || true)"

echo "$OUT_C6" | grep -q "\[Client\] MISSING: korangar.exe" || {
    echo "FAILED: Setup.ps1 did not output [Client] MISSING for korangar.exe."
    echo "$OUT_C6"
    exit 1
}
echo "$OUT_C6" | grep -q "Package Half:   Client" || {
    echo "FAILED: Setup.ps1 missing 'Package Half:   Client'."
    echo "$OUT_C6"
    exit 1
}
echo "PASS: Case 6 verified Setup.ps1 produces provenance-aware error output."

# -----------------------------------------------------------------------------
# Case 7: macOS Verify.command parity
# -----------------------------------------------------------------------------
echo ""
echo "--- Case 7: macOS Verify.command provenance check ---"
DIR_C7="$TMP_ROOT/case7"
mkdir -p "$DIR_C7/client"
echo "korangar binary bytes" > "$DIR_C7/korangar"
echo "data grf payload" > "$DIR_C7/data.grf"
cp "$REPO_ROOT/tools/packaging/macos/Verify.command" "$DIR_C7/Verify.command"
chmod +x "$DIR_C7/Verify.command"

{
    echo "$(calc_sha256 "$DIR_C7/korangar")  korangar"
    echo "$(calc_sha256 "$DIR_C7/Verify.command")  Verify.command"
} > "$DIR_C7/SHA256SUMS-client"

{
    echo "$(calc_sha256 "$DIR_C7/data.grf")  data.grf"
    echo "0000000000000000000000000000000000000000000000000000000000000000  Verify.command"
} > "$DIR_C7/SHA256SUMS-assets"

# Running inside directory with mock input (simulating return key press)
OUT_C7="$(cd "$DIR_C7" && printf '\n' | ./Verify.command 2>&1)" || {
    echo "FAILED: Case 7 expected clean pass because stale Verify.command in Assets is ignored."
    echo "$OUT_C7"
    exit 1
}
echo "$OUT_C7" | grep -q "All 3 files match. This download is intact." || {
    echo "FAILED: Case 7 did not match all 3 files."
    echo "$OUT_C7"
    exit 1
}

# Corrupt data.grf to test provenance formatting on macOS
echo "CORRUPT DATA" >> "$DIR_C7/data.grf"
set +e
OUT_C7_CORRUPT="$(cd "$DIR_C7" && printf '\n' | ./Verify.command 2>&1)"
CODE_C7=$?
set -e
if [ "$CODE_C7" -eq 0 ]; then
    echo "FAILED: Case 7 corrupt test expected exit code 1, got 0."
    exit 1
fi
echo "$OUT_C7_CORRUPT" | grep -q "\[Assets\] CORRUPT: data.grf" || {
    echo "FAILED: Case 7 did not tag corrupt data.grf with [Assets]."
    echo "$OUT_C7_CORRUPT"
    exit 1
}
echo "$OUT_C7_CORRUPT" | grep -q "Package Half:   Assets" || {
    echo "FAILED: Case 7 missing Package Half: Assets."
    echo "$OUT_C7_CORRUPT"
    exit 1
}
echo "PASS: Case 7 verified macOS Verify.command provenance and single-ownership parity."

echo ""
echo "=== All QW-012 Verifier Provenance Tests Passed! ==="
