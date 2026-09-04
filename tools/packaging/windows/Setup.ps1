#Requires -Version 5.1
# Seal Cascade first-run setup: do everything, then say it is ready.
#
# Run by Setup.bat, never double-clicked (a .ps1 double-click opens Notepad).
#
# TARGETS WINDOWS POWERSHELL 5.1 -- the one in the box. No ternaries, no `??`,
# no `&&`, no three-argument Join-Path, nothing newer than PowerShell 3.0
# cmdlets. Keep this file PURE ASCII with no BOM: 5.1 decodes a BOM-less script
# as ANSI, so a stray dash or quote becomes mojibake in the messages below.
#
# The whole point is that a friend runs ONE thing and is either playing or
# holding a sentence that explains exactly what to do next. Every check here
# exists because its failure would otherwise be silent or unreadable:
#
#   AVX2      the client is built for x86-64-v3, so an older CPU dies with
#             STATUS_ILLEGAL_INSTRUCTION and no window ever appears
#   runtime   a missing VC++ runtime is a DLL dialog, or nothing at all
#   assets    the 3.7 GB half downloaded separately is the likely miss
#   hashes    Drive truncates big files, and every later symptom looks like
#             a bug in the game instead of a bad download

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here

$assetPayload = @(
    'data.grf',
    'rdata.grf',
    'renewal2021.grf',
    'resources2021.grf',
    'lua_files.7z'
)

function Say($text)  { Write-Host ("  " + $text) }
function Good($text) { Write-Host ("  " + $text) -ForegroundColor Green }
function Warn($text) { Write-Host ("  " + $text) -ForegroundColor Yellow }

function Fail($what, $fix) {
    Write-Host ''
    Write-Host ("  Setup stopped: " + $what) -ForegroundColor Red
    Write-Host ''
    foreach ($line in $fix) { Write-Host ("  " + $line) }
    Write-Host ''
    exit 1
}

function Step($number, $text) {
    Write-Host ''
    Write-Host ("  [" + $number + "/5] " + $text) -ForegroundColor Cyan
}

Write-Host ''
Write-Host '  ============================================' -ForegroundColor Cyan
Write-Host '   Seal Cascade - setup' -ForegroundColor Cyan
Write-Host '  ============================================' -ForegroundColor Cyan
Say 'This runs once. It checks your PC, merges in the game data,'
Say 'and verifies every file. Nothing is sent anywhere.'

# ---------------------------------------------------------------- 1. the CPU
#
# IsProcessorFeaturePresent is the only AVX2 query available to PowerShell 5.1
# -- it runs on .NET Framework, which has no System.Runtime.Intrinsics. 40 is
# PF_AVX2_INSTRUCTIONS_AVAILABLE from winnt.h. It reports false on Windows 7
# even for a capable CPU, which is fine: the client needs Windows 10 anyway.
Step 1 'Checking your processor'

$avx2 = $null
try {
    $signature = '[DllImport("kernel32.dll")] public static extern bool IsProcessorFeaturePresent(uint feature);'
    $native = Add-Type -MemberDefinition $signature -Name 'SealCascadeCpu' -Namespace 'SealCascade' -PassThru
    $avx2 = $native::IsProcessorFeaturePresent(40)
} catch {
    $avx2 = $null
}

$cpuName = 'your processor'
try {
    $cpuName = (Get-CimInstance -ClassName Win32_Processor -ErrorAction Stop | Select-Object -First 1).Name.Trim()
} catch {
    # Not worth failing setup over a cosmetic name.
}

if ($avx2 -eq $false) {
    Fail ($cpuName + ' does not support AVX2.') @(
        'The game is compiled for processors with AVX2, which means an Intel',
        'Core from 2013 (4th generation, "Haswell") or newer, or any AMD Ryzen.',
        '',
        'Some budget chips sold well after 2013 -- Celeron, Pentium Silver and',
        'Gold, Atom -- do not have AVX2 either, even though they are recent.',
        '',
        'There is no setting that works around this. The game would close the',
        'instant it started, with no message at all, so Setup stops here',
        'instead. Sorry -- you will need a different PC for this one.',
        '',
        'Tell the host what this said. They can confirm it from your CPU name.'
    )
}

if ($null -eq $avx2) {
    Warn 'Could not read your CPU features. Continuing anyway.'
    Warn 'If the game closes instantly with no message, your CPU is too old.'
} else {
    Good ($cpuName + ' supports AVX2.')
}

# ------------------------------------------------------- 2. the C++ runtime
#
# Only relevant when the pack ships the redistributable. A statically linked
# client has no such dependency, and then there is nothing here to do -- so the
# presence of the installer beside us IS the question of whether to ask.
Step 2 'Checking the Visual C++ runtime'

$redist = Join-Path $here 'VC_redist.x64.exe'

if (-not (Test-Path -LiteralPath $redist)) {
    Good 'Not needed -- this build has no separate runtime to install.'
} else {
    $system32 = Join-Path $env:WINDIR 'System32'
    $probe = Join-Path $system32 'VCRUNTIME140_1.dll'

    if (Test-Path -LiteralPath $probe) {
        Good 'Already installed.'
    } else {
        Say 'The Microsoft Visual C++ runtime is missing. The game cannot start'
        Say 'without it. It is a standard Microsoft component and it ships in'
        Say 'this folder, so there is nothing to download.'
        Say ''
        Say 'Installing it now. Windows may ask for permission.'

        $exit = -1
        try {
            $process = Start-Process -FilePath $redist -ArgumentList '/install', '/passive', '/norestart' -Wait -PassThru
            $exit = $process.ExitCode
        } catch {
            $exit = -1
        }

        # 0 = installed, 1638 = a newer one is already there, 3010 = wants a reboot.
        if ($exit -eq 0 -or $exit -eq 1638) {
            Good 'Installed.'
        } elseif ($exit -eq 3010) {
            Warn 'Installed, but Windows wants a restart before the game will run.'
        } else {
            Fail ('the Visual C++ runtime could not be installed (code ' + $exit + ').') @(
                'Double-click VC_redist.x64.exe in this folder and follow the',
                'prompts, then run Setup again.',
                '',
                'If it refuses, you may not be an administrator on this PC.'
            )
        }
    }
}

# --------------------------------------------------------- 3. the game data
Step 3 'Finding the game data'

function Test-AssetFolder($path) {
    if ([string]::IsNullOrEmpty($path)) { return $false }
    if (-not (Test-Path -LiteralPath $path)) { return $false }
    return (Test-Path -LiteralPath (Join-Path $path 'data.grf'))
}

$alreadyHere = $true
foreach ($name in $assetPayload) {
    if (-not (Test-Path -LiteralPath (Join-Path $here $name))) { $alreadyHere = $false }
}

if ($alreadyHere) {
    Good 'Already in place.'
} else {
    $parent = Split-Path -Parent $here
    $downloads = Join-Path $env:USERPROFILE 'Downloads'

    $candidates = @(
        (Join-Path $here 'Assets'),
        (Join-Path $parent 'Assets'),
        (Join-Path $downloads 'Assets')
    )

    # Drive folders arrive with all sorts of names, and a zip that was unpacked
    # twice nests one inside another. So after the obvious spots, look for any
    # nearby folder that simply HAS a data.grf in it.
    foreach ($root in @($parent, $downloads, $here)) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        $children = Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue
        foreach ($child in $children) {
            $candidates = $candidates + $child.FullName
            $inner = Get-ChildItem -LiteralPath $child.FullName -Directory -ErrorAction SilentlyContinue
            foreach ($deeper in $inner) { $candidates = $candidates + $deeper.FullName }
        }
    }

    $source = $null
    foreach ($candidate in $candidates) {
        if (Test-AssetFolder $candidate) { $source = $candidate; break }
    }

    if ($null -eq $source) {
        Fail 'the game data (the Assets download) is nowhere to be found.' @(
            'There are two downloads in the shared Drive folder, and this is',
            'only the small one. You still need the big one:',
            '',
            '    Assets    about 3.7 GB - artwork, maps and music',
            '',
            'Download it, unzip it if it arrived zipped, put the Assets folder',
            'next to this one, and run Setup again.',
            '',
            'Setup looked in this folder, the folder above it, and Downloads.'
        )
    }

    Good ('Found it: ' + $source)

    # Same volume means a move is instant and costs no extra disk. Across
    # volumes there is no such thing as a cheap move, so copy and let them
    # delete the download themselves.
    $sameVolume = $false
    try {
        $fromRoot = [System.IO.Path]::GetPathRoot($source)
        $toRoot = [System.IO.Path]::GetPathRoot($here)
        $sameVolume = ($fromRoot -eq $toRoot)
    } catch {
        $sameVolume = $false
    }

    if ($sameVolume) {
        Say 'Moving the game data into this folder (this is quick).'
    } else {
        Say 'Copying the game data into this folder. It is 3.7 GB, so this'
        Say 'takes a few minutes. Leave the window open.'
    }

    $items = @()
    foreach ($name in $assetPayload) { $items = $items + $name }
    $items = $items + 'BGM'
    $items = $items + 'SHA256SUMS-assets'

    foreach ($name in $items) {
        $from = Join-Path $source $name
        $to = Join-Path $here $name

        if (-not (Test-Path -LiteralPath $from)) {
            if (Test-Path -LiteralPath $to) { continue }
            Fail ($name + ' is missing from the Assets folder.') @(
                'That download is incomplete. Download Assets again from the',
                'shared Drive folder, then run Setup again.'
            )
        }

        if (Test-Path -LiteralPath $to) {
            Say ('  already here, skipping: ' + $name)
            continue
        }

        Say ('  ' + $name)
        if ($sameVolume) {
            Move-Item -LiteralPath $from -Destination $to
        } else {
            Copy-Item -LiteralPath $from -Destination $to -Recurse
        }
    }

    Good 'Game data is in place.'
}

# ------------------------------------------------------------ 4. the hashes
Step 4 'Checking every file'

function Test-Manifest($manifestPath) {
    $result = New-Object psobject -Property @{ Ok = 0; Bad = 0; Missing = 0; Names = @() }

    foreach ($line in Get-Content -LiteralPath $manifestPath) {
        if ($line -notmatch '^([0-9a-fA-F]{64})\s+\.?[\\/]?(.+)$') { continue }

        $expected = $Matches[1].ToUpperInvariant()
        $relative = $Matches[2] -replace '/', '\'
        $path = Join-Path $here $relative

        if (-not (Test-Path -LiteralPath $path)) {
            $result.Missing = $result.Missing + 1
            $result.Names = $result.Names + ('MISSING  ' + $relative)
            continue
        }

        $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToUpperInvariant()
        if ($actual -ne $expected) {
            $result.Bad = $result.Bad + 1
            $result.Names = $result.Names + ('CORRUPT  ' + $relative)
        } else {
            $result.Ok = $result.Ok + 1
        }
    }

    return $result
}

$manifests = @('SHA256SUMS-client', 'SHA256SUMS-assets')
$totalOk = 0
$problems = @()
$checked = 0

foreach ($manifestName in $manifests) {
    $manifestPath = Join-Path $here $manifestName
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        Fail ($manifestName + ' is missing, so the files cannot be checked.') @(
            'Download that half again from the shared Drive folder. Without the',
            'checksum list a truncated download would be trusted, and every',
            'symptom afterwards would look like a bug in the game.'
        )
    }

    Say ('Reading ' + $manifestName + ' ...')
    $checked = $checked + 1
    $outcome = Test-Manifest $manifestPath
    $totalOk = $totalOk + $outcome.Ok
    foreach ($name in $outcome.Names) { $problems = $problems + $name }
}

if ($problems.Count -gt 0) {
    Write-Host ''
    foreach ($problem in $problems) { Write-Host ("    " + $problem) -ForegroundColor Red }
    Fail ($problems.Count.ToString() + ' file(s) are missing or damaged.') @(
        'Those files did not download correctly. Download the half they belong',
        'to again from the shared Drive folder -- the big Assets one if the',
        'names above are .grf or BGM, this small one otherwise -- and run',
        'Setup again.'
    )
}

Good ($totalOk.ToString() + ' files checked, all correct.')

# --------------------------------------------------------- 5. the mark of the web
Step 5 'Clearing the downloaded-file warnings'

Get-ChildItem -LiteralPath $here -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { @('.exe', '.bat', '.ps1', '.ron', '.txt') -contains $_.Extension } |
    Unblock-File -ErrorAction SilentlyContinue

Good 'Done.'

Write-Host ''
Write-Host '  ============================================' -ForegroundColor Green
Write-Host '   Ready to play.' -ForegroundColor Green
Write-Host '  ============================================' -ForegroundColor Green
Say 'From now on just double-click Play.'
Say ''
Say 'At the login screen, type a name ending in _m or _f -- for example'
Say 'BobSmith_m -- and a password you do not use anywhere else. There is'
Say 'no sign-up; typing a new name creates the account.'
Write-Host ''

$answer = Read-Host '  Start the game now? [Y/n]'
if ($answer -eq '' -or $answer -eq 'y' -or $answer -eq 'Y') {
    Start-Process -FilePath (Join-Path $here 'korangar.exe') -WorkingDirectory $here
}
