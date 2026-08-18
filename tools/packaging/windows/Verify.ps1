#Requires -Version 5.1
# Check a download against the SHA256SUMS shipped beside it.
#
# Windows has no sha256sum, so the manifest make-pack.sh writes is unreadable to
# the people it is for without this. Run it if the game behaves strangely after a
# download: Drive truncates large files, resumes badly, and a 3.7 GB asset folder
# is exactly the kind of thing that arrives subtly incomplete.
#
# Windows PowerShell 5.1 compatible, pure ASCII, no BOM -- see Play.ps1.

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here

$manifest = Join-Path $here 'SHA256SUMS'

if (-not (Test-Path -LiteralPath $manifest)) {
    Write-Host ''
    Write-Host '  No SHA256SUMS here, so there is nothing to check against.' -ForegroundColor Yellow
    Write-Host '  Run this inside the folder you downloaded, next to SHA256SUMS.'
    Write-Host ''
    exit 1
}

$bad = 0
$missing = 0
$ok = 0

foreach ($line in Get-Content -LiteralPath $manifest) {
    if ($line -notmatch '^([0-9a-fA-F]{64})\s+\.?[\\/]?(.+)$') { continue }

    $expected = $Matches[1]
    $relative = $Matches[2] -replace '/', '\'
    $path = Join-Path $here $relative

    if (-not (Test-Path -LiteralPath $path)) {
        Write-Host ("  MISSING  " + $relative) -ForegroundColor Red
        $missing = $missing + 1
        continue
    }

    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash

    if ($actual -ne $expected) {
        Write-Host ("  CORRUPT  " + $relative) -ForegroundColor Red
        $bad = $bad + 1
    } else {
        $ok = $ok + 1
    }
}

Write-Host ''

if ($bad -eq 0 -and $missing -eq 0) {
    Write-Host ("  All $ok files match. This download is intact.") -ForegroundColor Green
    Write-Host ''
    exit 0
}

Write-Host ("  $ok good, $bad corrupt, $missing missing.") -ForegroundColor Red
Write-Host '  Download the affected folder again from the shared Drive folder.'
Write-Host ''
exit 1
