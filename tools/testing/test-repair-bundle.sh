#!/usr/bin/env bash
# Test harness for QW-013: small repair bundle and atomic repair safety.
# Exercises:
#   1. Corrupt repair bundle input leaves installation completely unchanged.
#   2. One missing and one corrupt small file are repaired without touching valid GRFs.
#   3. Post-repair verification passes cleanly.
#   4. macOS Repair.command parity.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PWSH="${PWSH:-pwsh}"

if ! command -v "$PWSH" >/dev/null 2>&1; then
    echo "ERROR: $PWSH not found on PATH. Required for testing PowerShell repair scripts." >&2
    exit 1
fi

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/repair-bundle-test.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

REPAIR_PS1="$REPO_ROOT/tools/packaging/windows/Repair.ps1"
VERIFY_PS1="$REPO_ROOT/tools/packaging/windows/Verify.ps1"
REPAIR_CMD="$REPO_ROOT/tools/packaging/macos/Repair.command"

echo "=== Running QW-013 Repair Bundle Test Suite ==="
echo "Tmp dir: $TMP_ROOT"
echo "PowerShell: $("$PWSH" --version)"

calc_sha256() {
    shasum -a 256 "$1" | awk '{print toupper($1)}'
}

# -----------------------------------------------------------------------------
# Setup baseline installation and repair source
# -----------------------------------------------------------------------------
INSTALL_DIR="$TMP_ROOT/install"
REPAIR_SRC="$TMP_ROOT/repair_bundle"
mkdir -p "$INSTALL_DIR/client" "$REPAIR_SRC"

# Baseline files in installation
echo "korangar binary bytes" > "$INSTALL_DIR/korangar.exe"
echo "server ron" > "$INSTALL_DIR/client/server.ron"
echo "Play batch" > "$INSTALL_DIR/Play.bat"
echo "Play powershell" > "$INSTALL_DIR/Play.ps1"
echo "Verify batch" > "$INSTALL_DIR/Verify.bat"
cp "$VERIFY_PS1" "$INSTALL_DIR/Verify.ps1"
cp "$REPAIR_PS1" "$INSTALL_DIR/Repair.ps1"

# Large asset files (mock GRFs)
echo "BIG DATA GRF CONTENT 1234567890" > "$INSTALL_DIR/data.grf"
echo "BIG RDATA GRF CONTENT 0987654321" > "$INSTALL_DIR/rdata.grf"

# Baseline repair bundle
echo "Play batch" > "$REPAIR_SRC/Play.bat"
echo "Play powershell" > "$REPAIR_SRC/Play.ps1"
echo "Verify batch" > "$REPAIR_SRC/Verify.bat"
cp "$VERIFY_PS1" "$REPAIR_SRC/Verify.ps1"
cp "$REPAIR_PS1" "$REPAIR_SRC/Repair.ps1"

# Generate SHA256SUMS-repair
{
    echo "$(calc_sha256 "$REPAIR_SRC/Play.bat")  Play.bat"
    echo "$(calc_sha256 "$REPAIR_SRC/Play.ps1")  Play.ps1"
    echo "$(calc_sha256 "$REPAIR_SRC/Verify.bat")  Verify.bat"
    echo "$(calc_sha256 "$REPAIR_SRC/Verify.ps1")  Verify.ps1"
    echo "$(calc_sha256 "$REPAIR_SRC/Repair.ps1")  Repair.ps1"
} > "$REPAIR_SRC/SHA256SUMS-repair"

# Write client and asset manifests for the installation
{
    echo "$(calc_sha256 "$INSTALL_DIR/korangar.exe")  korangar.exe"
    echo "$(calc_sha256 "$INSTALL_DIR/client/server.ron")  client/server.ron"
    echo "$(calc_sha256 "$INSTALL_DIR/Play.bat")  Play.bat"
    echo "$(calc_sha256 "$INSTALL_DIR/Play.ps1")  Play.ps1"
    echo "$(calc_sha256 "$INSTALL_DIR/Verify.bat")  Verify.bat"
    echo "$(calc_sha256 "$INSTALL_DIR/Verify.ps1")  Verify.ps1"
    echo "$(calc_sha256 "$INSTALL_DIR/Repair.ps1")  Repair.ps1"
} > "$INSTALL_DIR/SHA256SUMS-client"

{
    echo "$(calc_sha256 "$INSTALL_DIR/data.grf")  data.grf"
    echo "$(calc_sha256 "$INSTALL_DIR/rdata.grf")  rdata.grf"
} > "$INSTALL_DIR/SHA256SUMS-assets"

echo "PASS: Baseline fixtures created."

# -----------------------------------------------------------------------------
# Test 1: Corrupt repair bundle input leaves installation unchanged
# -----------------------------------------------------------------------------
echo ""
echo "--- Test 1: Corrupt repair bundle input leaves installation unchanged ---"
CORRUPT_REPAIR_SRC="$TMP_ROOT/corrupt_repair"
cp -R "$REPAIR_SRC" "$CORRUPT_REPAIR_SRC"

# Corrupt Verify.ps1 inside the repair bundle
echo "# CORRUPTED REPAIR BYTES" >> "$CORRUPT_REPAIR_SRC/Verify.ps1"

# Make installation have a missing Play.bat and corrupt Verify.ps1
INSTALL_T1="$TMP_ROOT/install_t1"
cp -R "$INSTALL_DIR" "$INSTALL_T1"
rm -f "$INSTALL_T1/Play.bat"
echo "# CORRUPT IN INSTALL" > "$INSTALL_T1/Verify.ps1"

# Record all file hashes in installation before repair attempt
BEFORE_RECORD="$TMP_ROOT/t1_before.txt"
( cd "$INSTALL_T1" && find . -type f | LC_ALL=C sort | xargs shasum -a 256 > "$BEFORE_RECORD" )

set +e
OUT_T1="$("$PWSH" -File "$REPAIR_PS1" -TargetDirectory "$INSTALL_T1" -RepairSource "$CORRUPT_REPAIR_SRC" 2>&1)"
CODE_T1=$?
set -e

if [ "$CODE_T1" -eq 0 ]; then
    echo "FAILED: Test 1 expected repair failure due to corrupt repair bundle input, got exit 0."
    echo "$OUT_T1"
    exit 1
fi

echo "$OUT_T1" | grep -q "Repair input is corrupted" || {
    echo "FAILED: Test 1 did not report corrupt repair input error."
    echo "$OUT_T1"
    exit 1
}

# Verify installation was 100% untouched
AFTER_RECORD="$TMP_ROOT/t1_after.txt"
( cd "$INSTALL_T1" && find . -type f | LC_ALL=C sort | xargs shasum -a 256 > "$AFTER_RECORD" )

if ! cmp -s "$BEFORE_RECORD" "$AFTER_RECORD"; then
    echo "FAILED: Test 1 failed -- installation files changed despite corrupt repair input!"
    diff -u "$BEFORE_RECORD" "$AFTER_RECORD"
    exit 1
fi
echo "PASS: Test 1 verified corrupt repair input leaves installation 100% unchanged."

# -----------------------------------------------------------------------------
# Test 2: One missing and one corrupt small file repaired without touching GRFs
# -----------------------------------------------------------------------------
echo ""
echo "--- Test 2: One missing and one corrupt file repaired without touching GRFs ---"
INSTALL_T2="$TMP_ROOT/install_t2"
cp -R "$INSTALL_DIR" "$INSTALL_T2"

# Introduce defects:
# 1. missing small file: Play.bat
rm -f "$INSTALL_T2/Play.bat"
# 2. corrupt small file: Verify.ps1
echo "# DAMAGED VERIFIER SCRIPT" > "$INSTALL_T2/Verify.ps1"

# Record GRF hashes before repair
GRF_DATA_HASH_BEFORE="$(calc_sha256 "$INSTALL_T2/data.grf")"
GRF_RDATA_HASH_BEFORE="$(calc_sha256 "$INSTALL_T2/rdata.grf")"

OUT_T2="$("$PWSH" -File "$REPAIR_PS1" -TargetDirectory "$INSTALL_T2" -RepairSource "$REPAIR_SRC" 2>&1)" || {
    echo "FAILED: Test 2 repair failed."
    echo "$OUT_T2"
    exit 1
}

echo "$OUT_T2" | grep -q "Repair successful" || {
    echo "FAILED: Test 2 did not report Repair successful."
    echo "$OUT_T2"
    exit 1
}

# Verify Play.bat was restored
[ -f "$INSTALL_T2/Play.bat" ] || {
    echo "FAILED: Test 2 missing Play.bat was not restored."
    exit 1
}
EXPECTED_PLAY_HASH="$(calc_sha256 "$REPAIR_SRC/Play.bat")"
ACTUAL_PLAY_HASH="$(calc_sha256 "$INSTALL_T2/Play.bat")"
if [ "$EXPECTED_PLAY_HASH" != "$ACTUAL_PLAY_HASH" ]; then
    echo "FAILED: Test 2 Play.bat hash mismatch: $EXPECTED_PLAY_HASH != $ACTUAL_PLAY_HASH"
    exit 1
fi

# Verify Verify.ps1 was repaired
EXPECTED_VERIFY_HASH="$(calc_sha256 "$REPAIR_SRC/Verify.ps1")"
ACTUAL_VERIFY_HASH="$(calc_sha256 "$INSTALL_T2/Verify.ps1")"
if [ "$EXPECTED_VERIFY_HASH" != "$ACTUAL_VERIFY_HASH" ]; then
    echo "FAILED: Test 2 Verify.ps1 hash mismatch: $EXPECTED_VERIFY_HASH != $ACTUAL_VERIFY_HASH"
    exit 1
fi

# Verify GRFs were completely untouched
GRF_DATA_HASH_AFTER="$(calc_sha256 "$INSTALL_T2/data.grf")"
GRF_RDATA_HASH_AFTER="$(calc_sha256 "$INSTALL_T2/rdata.grf")"

if [ "$GRF_DATA_HASH_BEFORE" != "$GRF_DATA_HASH_AFTER" ]; then
    echo "FAILED: Test 2 data.grf was modified during repair!"
    exit 1
fi
if [ "$GRF_RDATA_HASH_BEFORE" != "$GRF_RDATA_HASH_AFTER" ]; then
    echo "FAILED: Test 2 rdata.grf was modified during repair!"
    exit 1
fi

# Run Verify.ps1 on repaired installation to verify entire directory passes
OUT_VERIFY="$("$PWSH" -File "$INSTALL_T2/Verify.ps1" 2>&1)" || {
    echo "FAILED: Test 2 post-repair verification failed."
    echo "$OUT_VERIFY"
    exit 1
}
echo "$OUT_VERIFY" | grep -q "All 9 files match. This download is intact." || {
    echo "FAILED: Test 2 post-repair verification did not report all 9 files match."
    echo "$OUT_VERIFY"
    exit 1
}
echo "PASS: Test 2 verified one missing and one corrupt file repaired without touching GRFs."

# -----------------------------------------------------------------------------
# Test 3: macOS Repair.command parity
# -----------------------------------------------------------------------------
echo ""
echo "--- Test 3: macOS Repair.command parity ---"
INSTALL_T3="$TMP_ROOT/install_t3"
REPAIR_SRC_MAC="$TMP_ROOT/repair_src_mac"
mkdir -p "$INSTALL_T3/client" "$REPAIR_SRC_MAC"

echo "korangar binary" > "$INSTALL_T3/korangar"
echo "Play command" > "$INSTALL_T3/Play.command"
echo "Verify command" > "$INSTALL_T3/Verify.command"
echo "DATA GRF" > "$INSTALL_T3/data.grf"

echo "Play command" > "$REPAIR_SRC_MAC/Play.command"
echo "Verify command" > "$REPAIR_SRC_MAC/Verify.command"

{
    echo "$(calc_sha256 "$REPAIR_SRC_MAC/Play.command")  Play.command"
    echo "$(calc_sha256 "$REPAIR_SRC_MAC/Verify.command")  Verify.command"
} > "$REPAIR_SRC_MAC/SHA256SUMS-repair"

# Introduce defects in T3
rm -f "$INSTALL_T3/Play.command"
echo "CORRUPT VERIFY" > "$INSTALL_T3/Verify.command"

# Run Repair.command
OUT_T3="$("$REPAIR_CMD" "$INSTALL_T3" "$REPAIR_SRC_MAC" 2>&1)" || {
    echo "FAILED: Test 3 Repair.command failed."
    echo "$OUT_T3"
    exit 1
}

[ -f "$INSTALL_T3/Play.command" ] || {
    echo "FAILED: Test 3 Play.command was not restored."
    exit 1
}
ACTUAL_MAC_PLAY="$(calc_sha256 "$INSTALL_T3/Play.command")"
EXPECTED_MAC_PLAY="$(calc_sha256 "$REPAIR_SRC_MAC/Play.command")"
if [ "$ACTUAL_MAC_PLAY" != "$EXPECTED_MAC_PLAY" ]; then
    echo "FAILED: Test 3 Play.command hash mismatch."
    exit 1
fi
echo "PASS: Test 3 verified macOS Repair.command parity."

echo ""
echo "=== All QW-013 Repair Bundle Tests Passed! ==="
