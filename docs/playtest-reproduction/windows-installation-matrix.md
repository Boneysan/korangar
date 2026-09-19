# Windows Installation Matrix Test Report (QW-014)

**Environment:** macOS arm64 running PowerShell Core 7.6.4 (Windows PowerShell 5.1 target scripts)
**Date:** 2026-09-13

| Case | Scenario | Expected Behavior | Observed Result | Large Assets Untouched? |
|---|---|---|---|---|
| 1 | Clean install | Assets merged into Windows folder, manifests verified | PASS (exit 0, all files verified) | Yes (cleanly copied into place) |
| 2 | Previous-friend-build upgrade | Update.ps1 updates client files, preserves GRFs and user configs | PASS (game data skipped, user settings kept) | Yes (100% byte-identical before & after) |
| 3 | Missing small file | Verify detects [Client] MISSING; Repair restores file | PASS (repaired Play.bat, verification clean) | Yes (100% byte-identical) |
| 4 | Corrupt small file | Verify detects [Client] CORRUPT; Repair restores file | PASS (repaired Troubleshoot.bat, verification clean) | Yes (100% byte-identical) |
| 5 | Conflicting shared file | Stale asset verifier ignored; non-verifier conflict isolated | PASS (Assets identified as responsible half) | Yes (GRFs untouched) |
| 6 | Interrupted repair | Corrupted input fails safely, cleans staging, leaves install untouched | PASS (installation 100% byte-identical) | Yes (100% byte-identical) |
| 7 | Interrupted GRF | Verify flags [Assets] CORRUPT; asset restored without recopying others | PASS (only damaged GRF replaced, others untouched) | Yes (valid GRFs untouched) |
| 8 | Rerunning Setup after success | Setup skips already-present assets, runs manifest verification | PASS (100% byte-identical, no recopy) | Yes (0 bytes copied) |
