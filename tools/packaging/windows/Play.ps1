#Requires -Version 5.1
# Seal Cascade launcher checks.
#
# Run by Play.bat, never double-clicked (see the comment there).
#
# TARGETS WINDOWS POWERSHELL 5.1 -- the one that ships in the box. Play.bat
# invokes `powershell`, not `pwsh`, because a friend's machine has 5.1 and may
# not have 7. So: no ternaries, no `??`, no `&&`, no three-argument Join-Path,
# and nothing newer than PowerShell 3.0 cmdlets. Keep this file **pure ASCII**
# with no BOM: 5.1 decodes a BOM-less script as ANSI, so a stray dash or quote
# from a text editor turns into mojibake in the messages below.
#
# This runs on EVERY launch, so it only does the cheap checks -- Setup.ps1 is
# where the full hash sweep lives. What is cheap enough to repeat is whatever
# fails in a way a friend cannot read:
#
#   1. No AVX2. The client is built for x86-64-v3, so an old CPU dies with
#      STATUS_ILLEGAL_INSTRUCTION before it can draw anything. No window, no
#      message, no log -- the launcher is the only place this can be said.
#   2. No Visual C++ runtime. A DLL dialog at best, silence at worst.
#   3. They opened the OS folder without copying Assets in beside it, so the
#      GRFs are missing. The client says nothing helpful.
#   4. The working directory is not the folder holding the game. Every path the
#      client reads is CWD-relative, and `Client::init` responds by trying to
#      `cd korangar` and then giving up -- a checkout heuristic that means
#      nothing on a friend's machine.
#
# None of those is worth a support call, and each is one existence check.

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here

function Say($text)  { Write-Host ("  " + $text) }
function Good($text) { Write-Host ("  " + $text) -ForegroundColor Green }

function Fail($what, $fix) {
    Write-Host ''
    Write-Host "  Seal Cascade cannot start: $what" -ForegroundColor Red
    Write-Host ''
    Write-Host "  $fix"
    Write-Host ''
    exit 1
}

Write-Host ''
Write-Host '  Seal Cascade' -ForegroundColor Cyan
Write-Host ''
$packVersionFile = Join-Path $here 'VERSION'
if (Test-Path -LiteralPath $packVersionFile) {
    $packVersion = (Get-Content -LiteralPath $packVersionFile -Raw).Trim()
    Say ('Pack version ' + $packVersion)
} else {
    Say 'Pack version unknown (no VERSION file -- this is an old download).'
}
Say 'Checking this folder before starting. Nothing is sent anywhere.'
Say 'If a line sits still, it is still working -- large files take a minute.'
Write-Host ''

Say 'Looking for korangar.exe...'
if (-not (Test-Path -LiteralPath (Join-Path $here 'korangar.exe'))) {
    Fail 'korangar.exe is not in this folder.' `
         'Keep Play.bat and korangar.exe together -- do not move one out on its own.'
}

Good 'korangar.exe is here'

# The client is compiled for x86-64-v3 (AVX2, FMA, BMI). Without it the process
# dies on its first vectorised instruction and a friend sees nothing at all, so
# this check has to come before anything slower. 40 is
# PF_AVX2_INSTRUCTIONS_AVAILABLE from winnt.h -- PowerShell 5.1 runs on .NET
# Framework, which has no System.Runtime.Intrinsics, so this P/Invoke is the
# only way to ask. It answers false on Windows 7 regardless; the client needs
# Windows 10 anyway.
Say 'Checking your processor...'
$avx2 = $null
try {
    $signature = '[DllImport("kernel32.dll")] public static extern bool IsProcessorFeaturePresent(uint feature);'
    $native = Add-Type -MemberDefinition $signature -Name 'SealCascadeCpu' -Namespace 'SealCascade' -PassThru
    $avx2 = $native::IsProcessorFeaturePresent(40)
} catch {
    $avx2 = $null
}

if ($avx2 -eq $false) {
    Fail 'this PC''s processor does not support AVX2.' `
         ("The game needs an Intel Core from 2013 (4th generation) or newer, or`n" +
          "  any AMD Ryzen. Budget chips -- Celeron, Pentium Silver/Gold, Atom --`n" +
          "  do not have AVX2 even when they are recent.`n`n" +
          "  There is no way around this one. See READ ME FIRST.txt.")
}

if ($avx2 -ne $false) {
    Good 'Processor is fine.'
}

# Only ask if the pack ships the installer -- its presence is what says this
# build has a runtime dependency at all.
Say 'Checking the Visual C++ runtime...'
if (Test-Path -LiteralPath (Join-Path $here 'VC_redist.x64.exe')) {
    $runtime = Join-Path (Join-Path $env:WINDIR 'System32') 'VCRUNTIME140_1.dll'
    if (-not (Test-Path -LiteralPath $runtime)) {
        Fail 'the Microsoft Visual C++ runtime is not installed.' `
             ("Double-click Setup in this folder -- it installs the runtime for you`n" +
              "  from the copy that ships here. Nothing to download.")
    }
}

Good 'Runtime is fine.'

# The Assets download is separate and enormous, so this is the likely miss.
Say 'Looking for game data...'
$assets = @('data.grf', 'rdata.grf', 'renewal2021.grf', 'resources2021.grf', 'lua_files.7z')
$missing = @($assets | Where-Object { -not (Test-Path -LiteralPath (Join-Path $here $_)) })

if ($missing.Count -gt 0) {
    Fail "the game data is missing ($($missing -join ', '))." `
         "Double-click Setup in this folder. It finds the Assets download and moves it in for you."
}

Good 'Game data is here.'

# The two halves of the pack ship SEPARATELY NAMED manifests on purpose: they
# are merged into one folder, and a shared name would mean one silently
# replacing the other -- taking either the lua check below or the client half's
# coverage with it. lua_files.7z lives in the assets half.
$manifest = Join-Path $here 'SHA256SUMS-assets'
if (-not (Test-Path -LiteralPath $manifest)) {
    Fail 'SHA256SUMS-assets is missing, so the game data cannot be checked.' `
         'Double-click Setup, or copy SHA256SUMS-assets in from the Assets folder. Without it a swapped lua_files.7z would be trusted.'
}

$luaRelative = 'lua_files.7z'
$luaPath = Join-Path $here $luaRelative
$expected = $null
foreach ($line in Get-Content -LiteralPath $manifest) {
    if ($line -notmatch '^([0-9a-fA-F]{64})\s+\.?[\\/]?(.*)$') { continue }
    $name = $Matches[2] -replace '\\', '/'
    if ($name -eq $luaRelative -or $name.EndsWith('/' + $luaRelative)) {
        $expected = $Matches[1].ToUpperInvariant()
        break
    }
}

if ($null -eq $expected) {
    Fail 'SHA256SUMS-assets does not list lua_files.7z.' `
         'Re-download Assets. The manifest must name lua_files.7z.'
}

Say 'Spot-checking lua_files.7z (a few seconds)...'
$actual = (Get-FileHash -LiteralPath $luaPath -Algorithm SHA256).Hash.ToUpperInvariant()
if ($actual -ne $expected) {
    Fail 'lua_files.7z does not match the checksum list.' `
         'The file was swapped or the download is corrupt. Run Verify, then copy a fresh lua_files.7z and SHA256SUMS-assets from Assets.'
}
Good 'lua_files.7z matches.'

if (-not (Test-Path -LiteralPath (Join-Path $here 'archive'))) {
    Fail 'the archive folder is missing.' `
         'Re-download this OS folder from the shared Drive folder; archive must sit next to korangar.exe.'
}

if (-not (Test-Path -LiteralPath (Join-Path (Join-Path $here 'client') 'server.ron'))) {
    Fail 'client\server.ron is missing, so the game does not know which server to join.' `
         'Re-download this OS folder -- server.ron ships already filled in, and is not something you write.'
}

# Files from Drive arrive tagged as downloaded; clearing the tag on our own
# files makes Windows less suspicious. Harmless if the tag is not there.
Say 'Clearing Windows download flags...'
Get-ChildItem -LiteralPath $here -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { @('.exe', '.bat', '.ps1', '.ron', '.txt') -contains $_.Extension } |
    Unblock-File -ErrorAction SilentlyContinue
Good 'Ready.'

Write-Host ''
Write-Host '  Starting Seal Cascade...' -ForegroundColor Green
Say 'The window can take a minute to appear, especially the first time.'
Say 'You can close this console once the game is open.'
Write-Host ''
Start-Process -FilePath (Join-Path $here 'korangar.exe') -WorkingDirectory $here
