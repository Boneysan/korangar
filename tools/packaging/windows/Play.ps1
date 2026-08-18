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
# This exists because the two ways a friend's first launch fails both produce
# useless symptoms:
#
#   1. They opened the OS folder without copying Assets in beside it, so the
#      GRFs are missing. The client says nothing helpful.
#   2. The working directory is not the folder holding the game. Every path the
#      client reads is CWD-relative, and `Client::init` responds by trying to
#      `cd korangar` and then giving up -- a checkout heuristic that means
#      nothing on a friend's machine.
#
# Neither is worth a support call, and both are one file-existence check.

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here

function Fail($what, $fix) {
    Write-Host ''
    Write-Host "  Seal Cascade cannot start: $what" -ForegroundColor Red
    Write-Host ''
    Write-Host "  $fix"
    Write-Host ''
    exit 1
}

if (-not (Test-Path -LiteralPath (Join-Path $here 'korangar.exe'))) {
    Fail 'korangar.exe is not in this folder.' `
         'Keep Play.bat and korangar.exe together -- do not move one out on its own.'
}

# The Assets download is separate and enormous, so this is the likely miss.
$assets = @('data.grf', 'rdata.grf', 'renewal2021.grf', 'resources2021.grf', 'lua_files.7z')
$missing = @($assets | Where-Object { -not (Test-Path -LiteralPath (Join-Path $here $_)) })

if ($missing.Count -gt 0) {
    Fail "the game data is missing ($($missing -join ', '))." `
         "Copy everything from the Assets folder into THIS folder, so data.grf and lua_files.7z sit next to Play.bat, then try again."
}

$manifest = Join-Path $here 'SHA256SUMS'
if (-not (Test-Path -LiteralPath $manifest)) {
    Fail 'SHA256SUMS is missing, so the game data cannot be checked.' `
         'Copy SHA256SUMS from the Assets folder into THIS folder. Without it a swapped lua_files.7z would be trusted.'
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
    Fail 'SHA256SUMS does not list lua_files.7z.' `
         'Re-download Assets. The manifest must name lua_files.7z.'
}

$actual = (Get-FileHash -LiteralPath $luaPath -Algorithm SHA256).Hash.ToUpperInvariant()
if ($actual -ne $expected) {
    Fail 'lua_files.7z does not match SHA256SUMS.' `
         'The file was swapped or the download is corrupt. Copy a fresh lua_files.7z and SHA256SUMS from Assets.'
}

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
Get-ChildItem -LiteralPath $here -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { @('.exe', '.bat', '.ps1', '.ron') -contains $_.Extension } |
    Unblock-File -ErrorAction SilentlyContinue

Write-Host '  Starting Seal Cascade...' -ForegroundColor Green
Start-Process -FilePath (Join-Path $here 'korangar.exe') -WorkingDirectory $here
