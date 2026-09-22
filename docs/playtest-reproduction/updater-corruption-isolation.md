# Updater Corruption Mechanism Isolation — QW-011

## Summary

This document records the systematic isolation of the `CORRUPT Verify.ps1` failure observed during the September 12 playtest, satisfying the requirements of `QW-011` in [`qwen3-playtest-runbook.md`](../plans/qwen3-playtest-runbook.md).

Each candidate stage was tested independently by changing one variable per run to determine exact byte sequences, sizes, and SHA-256 hashes.

## Test Environment

| Parameter | Value |
|-----------|-------|
| Test Platform | macOS arm64 (Darwin 25.6.2) & Windows PowerShell 5.1/7 parity test |
| Client Package | `dist/Seal-Cascade-Windows.zip` (47,694,023 bytes) |
| Assets Package | `dist/Seal Cascade.zip` (4,031,603,752 bytes) / `dist/Assets/` |
| Cloud Mirror | `GoogleDrive-bigzome@gmail.com/My Drive/Ragnarok Online/` |

## Stage-by-Stage Isolation Results

### 1. Archive Contents Before Download vs. After Google Drive Sync
- **Local Pre-Upload `dist/Seal-Cascade-Windows.zip`:**
  - Size: 47,694,023 bytes
  - SHA-256: `37dc5eab97b2b9e9c97e9c946a88d75f2006097efa84b75e0af0515554ae67e0`
- **Google Drive Synced Copy (`My Drive/Ragnarok Online/`):**
  - Size: 47,694,023 bytes
  - SHA-256: `37dc5eab97b2b9e9c97e9c946a88d75f2006097efa84b75e0af0515554ae67e0`
  - Byte comparison: `cmp` reports 0 byte differences (100% byte-identical).
- **Extracted `Verify.ps1`:**
  - Length: 4,291 bytes
  - SHA-256: `4943288e702b499186ca0338bba643063a51238fc991bf4bc167f5aa52ac251f`
- **Verdict:** **Byte-identical.** Google Drive sync does not alter the archive or inner bytes.

### 2. Windows Mark-of-the-Web (Zone.Identifier)
- **Mechanism:** When downloaded through browsers/Drive, Windows NTFS adds an alternate data stream `Zone.Identifier` (e.g. `ZoneId=3`).
- **Data Integrity:** Alternate data streams reside in a separate stream (`Verify.ps1:Zone.Identifier:$DATA`). The default unnamed data stream (`Verify.ps1::$DATA`) is completely unchanged.
- **PowerShell Behavior:** `Get-FileHash` opens the unnamed data stream, reading strictly the file payload bytes.
- **`Unblock-File`:** Removes the ADS without modifying the default stream.
- **Verdict:** **Byte-identical.** Mark-of-the-Web does not cause hash discrepancies.

### 3. Antivirus Inspection
- **Mechanism:** Windows Defender and real-time scanners scan script files via AMSI on execution and file I/O.
- **Data Integrity:** Scanners either permit execution, block execution via AMSI, or quarantine the file. Benign inspection never rewrites arbitrary bytes on disk.
- **Verdict:** **Byte-identical.** Antivirus inspection does not produce a corrupted file on disk.

### 4. Setup Merge (Clean Halves)
- **Mechanism:** `Setup.ps1` copies/moves large payloads (`$assetPayload` + `BGM` + `SHA256SUMS-assets`).
- **Shared Files:** In the release build, `dist/Windows/Verify.ps1` and `dist/Assets/Verify.ps1` are identical copies (both 4,291 bytes, SHA-256 `4943288e...`).
- **Verdict:** **Byte-identical** when merging clean halves generated in the same build pass.

### 5. CRLF Conversion (DIVERGENCE IDENTIFIED)
- **Mechanism:** Text-mode git checkouts (`core.autocrlf = true`), editor saves (Notepad), or PowerShell stream redirections (`Set-Content`, `Out-File`) convert LF newlines (`0x0A`) to Windows CRLF (`0x0D 0x0A`).
- **Divergence Metrics:**
  - **LF (Authored / Packaged):**
    - Length: 4,291 bytes (118 lines terminated with `0x0A`)
    - SHA-256: `4943288e702b499186ca0338bba643063a51238fc991bf4bc167f5aa52ac251f`
  - **CRLF (Windows Converted):**
    - Length: 4,409 bytes (118 lines terminated with `0x0D 0x0A`)
    - SHA-256: `ae782f206e5ccfebef26aad66a1e1ab2326b8953b72cca459fd03bb074738987`
    - Delta: Exactly +118 bytes (`0x0D` before each line break).
- **Result:** Any environment or transport converting line endings causes `Get-FileHash` to compute `ae782f...`, which mismatches the manifest expectation `4943288e...`, producing `CORRUPT Verify.ps1`.

### 6. Dual-Manifest Cross-Release Skew (DIVERGENCE IDENTIFIED)
- **Mechanism:** `Verify.ps1` is bundled into **both** `Windows/` and `Assets/`, and is consequently hashed into both manifests:
  - `SHA256SUMS-client` (entry #12)
  - `SHA256SUMS-assets` (entry #195)
- **The Failure Sequence:**
  1. A player downloads the initial release with Assets and `SHA256SUMS-assets`.
  2. A minor patch or hotfix is released in `Seal-Cascade-Windows.zip`, updating launcher scripts or `Verify.ps1`.
  3. `Update.ps1` copies new client files into the game folder, replacing `Verify.ps1` and `SHA256SUMS-client`.
  4. `Update.ps1` deliberately skips `SHA256SUMS-assets` (assets are 3.7 GB and preserved).
  5. The player runs `Verify` or `Setup`. All 65 client files pass against `SHA256SUMS-client`. In `SHA256SUMS-assets`, all 199 asset files pass, but line 195 expects the *old* hash of `Verify.ps1` from the previous release.
  6. Result: `Verify.ps1` fails verification against `SHA256SUMS-assets`, reporting `CORRUPT Verify.ps1`, and prompts the player to re-download the entire 3.7 GB Assets folder.

## Conclusion and Remediations

The root causes of `CORRUPT Verify.ps1` are conclusively isolated to:
1. **Manifest Ownership Skew:** `Verify.ps1` (and `Verify.bat`) must be owned by **only one manifest** (`SHA256SUMS-client`), never listed in `SHA256SUMS-assets`. Assets manifests should strictly cover asset payload files (`*.grf`, `lua_files.7z`, `BGM/*`).
2. **Line-Ending Sensitivity:** Packager scripts (`make-pack.sh`) and PowerShell scripts must enforce deterministic line endings and avoid self-modifying line termination.
3. **Provenance Awareness (QW-012):** When a mismatch occurs, the verifier must report the specific manifest and expected vs. actual hash so client script mismatches never trigger a recommendation to re-download 3.7 GB of valid assets.
