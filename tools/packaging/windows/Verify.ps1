#Requires -Version 5.1
# Check a download against the SHA256SUMS-* lists shipped beside it.
#
# Windows has no sha256sum, so the manifests make-pack.sh writes are unreadable
# to the people they are for without this. Run it if the game behaves strangely
# after a download: Drive truncates large files, resumes badly, and a 3.7 GB
# asset folder is exactly the kind of thing that arrives subtly incomplete.
#
# TWO manifests, deliberately named apart:
#
#   SHA256SUMS-client   the small half -- exe, launchers, archive\, client\
#   SHA256SUMS-assets   the big half -- the GRFs, lua_files.7z, BGM\
#
# They used to share the name SHA256SUMS, which was a bug: the two halves are
# merged into ONE folder by design, so Explorer offered Replace-or-Skip and
# whichever copy lost took its half's coverage with it. Different names mean
# both survive the merge and this script can check everything at once.
#
# Checks whichever it finds, so it is still useful run inside a half that has
# not been merged yet.
#
# Windows PowerShell 5.1 compatible, pure ASCII, no BOM -- see Play.ps1.

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here

$manifestNames = @('SHA256SUMS-client', 'SHA256SUMS-assets')
$found = @($manifestNames | Where-Object { Test-Path -LiteralPath (Join-Path $here $_) })

if ($found.Count -eq 0) {
    Write-Host ''
    Write-Host '  No checksum list here, so there is nothing to check against.' -ForegroundColor Yellow
    Write-Host '  Run this inside the folder you downloaded, next to a SHA256SUMS-* file.'
    Write-Host ''
    exit 1
}

if ($found.Count -eq 1) {
    Write-Host ''
    Write-Host ("  Only " + $found[0] + " is here, so only that half gets checked.") -ForegroundColor Yellow
    Write-Host '  That is expected before you have run Setup.'
}

$bad = 0
$missing = 0
$ok = 0

foreach ($manifestName in $found) {
    $manifest = Join-Path $here $manifestName

    Write-Host ''
    Write-Host ("  Checking against " + $manifestName + " ...") -ForegroundColor Cyan
    Write-Host '  Each file is named before it is hashed. Large GRFs can take a minute.'

    $entries = @()
    foreach ($line in Get-Content -LiteralPath $manifest) {
        if ($line -notmatch '^([0-9a-fA-F]{64})\s+\.?[\\/]?(.+)$') { continue }
        $entries = $entries + $line
    }
    $total = $entries.Count
    $n = 0

    foreach ($line in $entries) {
        if ($line -notmatch '^([0-9a-fA-F]{64})\s+\.?[\\/]?(.+)$') { continue }

        $expected = $Matches[1]
        $relative = $Matches[2] -replace '/', '\'
        $path = Join-Path $here $relative
        $n = $n + 1
        $prefix = '  [' + $n.ToString() + '/' + $total.ToString() + '] ' + $relative

        if (-not (Test-Path -LiteralPath $path)) {
            Write-Host ($prefix + ' -- MISSING') -ForegroundColor Red
            $missing = $missing + 1
            continue
        }

        $bytes = [long](Get-Item -LiteralPath $path).Length
        $sizeText = if ($bytes -ge 1073741824) {
            ([math]::Round(($bytes / 1073741824), 1).ToString() + ' GB')
        } elseif ($bytes -ge 1048576) {
            ([math]::Round(($bytes / 1048576), 0).ToString() + ' MB')
        } else {
            ([math]::Round(($bytes / 1024), 0).ToString() + ' KB')
        }

        if ($bytes -gt 104857600) {
            Write-Host ($prefix + ' (' + $sizeText + ') -- large file, please wait...')
        } else {
            Write-Host ($prefix + ' (' + $sizeText + ')')
        }

        $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash

        if ($actual -ne $expected) {
            Write-Host ("  CORRUPT  " + $relative) -ForegroundColor Red
            $bad = $bad + 1
        } else {
            $ok = $ok + 1
        }
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
Write-Host '  Names ending .grf, lua_files.7z or starting BGM\ are the big Assets'
Write-Host '  download; anything else is the small Windows one.'
Write-Host ''
exit 1
