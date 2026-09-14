#Requires -Version 5.1
# Seal Cascade script repair: verify and restore launch, setup, update, and
# verifier scripts and their manifest into an existing installation.
#
# Repairs into a temporary name first, verifies the staged bytes against the
# repair manifest, then replaces each damaged or missing file atomically.
#
# Targets Windows PowerShell 5.1, pure ASCII, no BOM.

param(
    [string]$TargetDirectory = '',
    [string]$RepairSource = ''
)

$ErrorActionPreference = 'Stop'
$runningScript = $MyInvocation.MyCommand.Path
$here = Split-Path -Parent $runningScript

function Say($text)  { Write-Host ("  " + $text) }
function Good($text) { Write-Host ("  " + $text) -ForegroundColor Green }
function Warn($text) { Write-Host ("  " + $text) -ForegroundColor Yellow }

function Fail($what, $fix) {
    Write-Host ''
    Write-Host ("  Repair stopped: " + $what) -ForegroundColor Red
    Write-Host ''
    foreach ($line in $fix) { Write-Host ("  " + $line) }
    Write-Host ''
    exit 1
}

function Same-Path($a, $b) {
    if ([string]::IsNullOrEmpty($a) -or [string]::IsNullOrEmpty($b)) { return $false }
    return (([string]$a).TrimEnd('\').ToLowerInvariant() -eq ([string]$b).TrimEnd('\').ToLowerInvariant())
}

# -----------------------------------------------------------------------------
# 1. Locate Repair Source
# -----------------------------------------------------------------------------
$sourceDir = $here
if (-not [string]::IsNullOrEmpty($RepairSource)) {
    if (-not (Test-Path -LiteralPath $RepairSource)) {
        Fail ("Repair source does not exist: " + $RepairSource) @('Provide a valid folder containing the repair files.')
    }
    $sourceDir = (Resolve-Path -LiteralPath $RepairSource).Path
}

$manifestNames = @('SHA256SUMS-repair', 'SHA256SUMS-client')
$repairManifest = $null
foreach ($m in $manifestNames) {
    $mp = Join-Path $sourceDir $m
    if (Test-Path -LiteralPath $mp) {
        $repairManifest = $mp
        break
    }
}

if ($null -eq $repairManifest) {
    Fail 'No repair manifest found in the repair source.' @(
        'The repair folder must contain SHA256SUMS-repair or SHA256SUMS-client.',
        'Folder checked: ' + $sourceDir
    )
}

# -----------------------------------------------------------------------------
# 2. Validate Repair Source Input (Never repair from corrupt inputs)
# -----------------------------------------------------------------------------
Write-Host ''
Write-Host '  ============================================' -ForegroundColor Cyan
Write-Host '   Seal Cascade - script repair' -ForegroundColor Cyan
Write-Host '  ============================================' -ForegroundColor Cyan
Say 'Validating repair bundle integrity before making any changes...'

$repairEntries = @()
foreach ($line in Get-Content -LiteralPath $repairManifest) {
    if ($line -notmatch '^([0-9a-fA-F]{64})\s+\.?[\\/]?(.+)$') { continue }
    $expectedHash = $Matches[1].ToUpperInvariant()
    $rel = $Matches[2] -replace '/', '\'
    if ($rel.StartsWith('.\')) { $rel = $rel.Substring(2) }
    
    # Exclude non-script payloads if reading from SHA256SUMS-client
    $leaf = Split-Path -Leaf $rel
    if ($leaf.EndsWith('.exe') -or $leaf.EndsWith('.grf') -or $leaf.EndsWith('.7z')) {
        continue
    }

    $sourcePath = Join-Path $sourceDir $rel
    if (-not (Test-Path -LiteralPath $sourcePath)) {
        Fail ('Repair input is missing ' + $rel) @(
            'The repair bundle itself is incomplete.',
            'No files in your game installation were changed.'
        )
    }

    $actualSourceHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($actualSourceHash -ne $expectedHash) {
        Fail ('Repair input is corrupted: ' + $rel) @(
            'The repair bundle file has been damaged in transit.',
            'Expected: ' + $expectedHash,
            'Actual:   ' + $actualSourceHash,
            'No files in your game installation were changed.'
        )
    }

    $repairEntries = $repairEntries + [PSCustomObject]@{
        Relative     = $rel
        ExpectedHash = $expectedHash
        SourcePath   = $sourcePath
    }
}

if ($repairEntries.Count -eq 0) {
    Fail 'No repairable script entries found in manifest.' @('Check the repair manifest.')
}

Good ('Repair bundle is intact (' + $repairEntries.Count.ToString() + ' files validated).')

# -----------------------------------------------------------------------------
# 3. Locate Target Game Installation
# -----------------------------------------------------------------------------
$destDir = ''
if (-not [string]::IsNullOrEmpty($TargetDirectory)) {
    if (-not (Test-Path -LiteralPath $TargetDirectory)) {
        Fail ("Target directory does not exist: " + $TargetDirectory) @('Specify an existing game folder.')
    }
    $destDir = (Resolve-Path -LiteralPath $TargetDirectory).Path
} else {
    # Check if here or parent is the game folder
    if ((Test-Path -LiteralPath (Join-Path $here 'korangar.exe')) -or (Test-Path -LiteralPath (Join-Path $here 'data.grf'))) {
        $destDir = $here
    } else {
        $parent = Split-Path -Parent $here
        if (-not [string]::IsNullOrEmpty($parent)) {
            if ((Test-Path -LiteralPath (Join-Path $parent 'korangar.exe')) -or (Test-Path -LiteralPath (Join-Path $parent 'data.grf'))) {
                $destDir = $parent
            }
        }
    }
}

if ([string]::IsNullOrEmpty($destDir)) {
    Fail 'Could not determine target game installation folder.' @(
        'Pass -TargetDirectory <path> to specify your game folder.',
        'Example: powershell -File Repair.ps1 -TargetDirectory "C:\Games\Seal Cascade"'
    )
}

Say ('Target game folder: ' + $destDir)

# -----------------------------------------------------------------------------
# 4. Identify Files That Need Repair
# -----------------------------------------------------------------------------
$filesToRepair = @()
foreach ($entry in $repairEntries) {
    $targetPath = Join-Path $destDir $entry.Relative
    if (-not (Test-Path -LiteralPath $targetPath)) {
        $filesToRepair = $filesToRepair + $entry
        Say ('  missing: ' + $entry.Relative)
        continue
    }

    $actualHash = (Get-FileHash -LiteralPath $targetPath -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($actualHash -ne $entry.ExpectedHash) {
        $filesToRepair = $filesToRepair + $entry
        Say ('  corrupt: ' + $entry.Relative)
    }
}

if ($filesToRepair.Count -eq 0) {
    Good 'All scripts and manifests already match the repair bundle. No repairs needed.'
    exit 0
}

Say ('Found ' + $filesToRepair.Count.ToString() + ' file(s) needing repair.')

# -----------------------------------------------------------------------------
# 5. Repair into Temporary Names, Verify Staged Bytes, Replace Atomically
# -----------------------------------------------------------------------------
$stagedFiles = @()
try {
    foreach ($item in $filesToRepair) {
        $tmpName = $item.Relative + '.repair-tmp'
        $tmpPath = Join-Path $destDir $tmpName
        
        $tmpDir = Split-Path -Parent $tmpPath
        if (-not (Test-Path -LiteralPath $tmpDir)) {
            New-Item -ItemType Directory -Path $tmpDir | Out-Null
        }

        # Copy into temporary name
        Copy-Item -LiteralPath $item.SourcePath -Destination $tmpPath -Force
        $stagedFiles = $stagedFiles + $tmpPath

        # Verify staged bytes immediately
        $stagedHash = (Get-FileHash -LiteralPath $tmpPath -Algorithm SHA256).Hash.ToUpperInvariant()
        if ($stagedHash -ne $item.ExpectedHash) {
            throw ("Staged file verification failed for " + $item.Relative)
        }
    }

    # All staged files verified. Now replace each target file atomically.
    foreach ($item in $filesToRepair) {
        $tmpPath = Join-Path $destDir ($item.Relative + '.repair-tmp')
        $targetPath = Join-Path $destDir $item.Relative

        # If target file is the running script itself, avoid in-place overwrite
        if (Same-Path $targetPath $runningScript) {
            Warn ('  skipping currently running script: ' + $item.Relative)
            Remove-Item -LiteralPath $tmpPath -Force -ErrorAction SilentlyContinue
            continue
        }

        # Atomically move/replace temporary file into final destination
        Move-Item -LiteralPath $tmpPath -Destination $targetPath -Force
        Good ('  repaired: ' + $item.Relative)
    }
} catch {
    # Clean up any leftover temporary files on error
    foreach ($tmp in $stagedFiles) {
        if (Test-Path -LiteralPath $tmp) {
            Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
        }
    }
    Fail ('Repair operation aborted: ' + $_) @(
        'An error occurred while copying or staging repair files.',
        'Staged temporary files were cleaned up. Original files preserved.'
    )
}

Write-Host ''
Good ('Repair successful! ' + $filesToRepair.Count.ToString() + ' file(s) restored.')
Say 'Large game assets (GRFs, BGM) were untouched.'
Say 'You can now run Verify.bat or Play.bat in your game folder.'
Write-Host ''
exit 0
