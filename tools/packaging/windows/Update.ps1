#Requires -Version 5.1
# Seal Cascade client update: find an existing install and copy this small
# pack over it, keeping the 3.7 GB game data.
#
# Run by Update.bat from the unzipped Seal-Cascade-Windows.zip -- never
# double-clicked (a .ps1 double-click opens Notepad).
#
# TARGETS WINDOWS POWERSHELL 5.1. No ternaries, no `??`, no `&&`, no
# three-argument Join-Path. Pure ASCII, no BOM. See Play.ps1.

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here
$here = (Get-Item -LiteralPath $here).FullName

$keepFiles = @(
    'data.grf',
    'rdata.grf',
    'renewal2021.grf',
    'resources2021.grf',
    'lua_files.7z',
    'SHA256SUMS-assets'
)
$keepDirs = @('BGM')

function Say($text)  { Write-Host ("  " + $text) }
function Good($text) { Write-Host ("  " + $text) -ForegroundColor Green }
function Warn($text) { Write-Host ("  " + $text) -ForegroundColor Yellow }

function Fail($what, $fix) {
    Write-Host ''
    Write-Host ("  Update stopped: " + $what) -ForegroundColor Red
    Write-Host ''
    foreach ($line in $fix) { Write-Host ("  " + $line) }
    Write-Host ''
    exit 1
}

function Format-Size([long]$bytes) {
    if ($bytes -ge 1073741824) {
        return ([math]::Round(($bytes / 1073741824), 1).ToString() + ' GB')
    }
    if ($bytes -ge 1048576) {
        return ([math]::Round(($bytes / 1048576), 0).ToString() + ' MB')
    }
    if ($bytes -ge 1024) {
        return ([math]::Round(($bytes / 1024), 0).ToString() + ' KB')
    }
    return ($bytes.ToString() + ' B')
}

function Same-Path($a, $b) {
    if ([string]::IsNullOrEmpty($a) -or [string]::IsNullOrEmpty($b)) { return $false }
    return (([string]$a).TrimEnd('\') -eq ([string]$b).TrimEnd('\'))
}

function Test-Install($path) {
    if ([string]::IsNullOrEmpty($path)) { return $false }
    if (-not (Test-Path -LiteralPath $path)) { return $false }
    if (-not (Test-Path -LiteralPath (Join-Path $path 'data.grf'))) { return $false }
    $exe = Join-Path $path 'korangar.exe'
    $play = Join-Path $path 'Play.bat'
    if (-not (Test-Path -LiteralPath $exe) -and -not (Test-Path -LiteralPath $play)) { return $false }
    return $true
}

function Skip-DirName($name) {
    $n = $name.ToLowerInvariant()
    if ($n -eq 'windows' -or $n -eq 'windows.old') { return $true }
    if ($n -eq 'program files' -or $n -eq 'program files (x86)' -or $n -eq 'programdata') { return $true }
    if ($n -eq '$recycle.bin' -or $n -eq 'system volume information' -or $n -eq 'recovery') { return $true }
    if ($n -eq 'perflogs' -or $n -eq 'msocache' -or $n -eq 'config.msi' -or $n -eq 'intel') { return $true }
    if ($n -eq 'appdata' -or $n -eq 'application data' -or $n -eq 'local settings') { return $true }
    if ($n -eq 'windowsapps' -or $n -eq '$windows.~bt' -or $n -eq '$windows.~ws') { return $true }
    if ($n -eq 'node_modules' -or $n -eq '.git' -or $n -eq '.svn') { return $true }
    if ($n -eq 'temp' -or $n -eq 'tmp' -or $n -eq 'cache' -or $n -eq 'caches') { return $true }
    return $false
}

function Is-DriveRoot($path) {
    if ([string]::IsNullOrEmpty($path)) { return $false }
    $p = $path.TrimEnd('\')
    if ($p.Length -eq 2 -and $p[1] -eq ':') { return $true }
    return $false
}

function Is-Link($item) {
    if ($null -eq $item) { return $false }
    try {
        return [bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
    } catch {
        return $false
    }
}

function Add-Found([ref]$list, $path) {
    if (-not (Test-Install $path)) { return }
    if (Same-Path $path $here) { return }
    foreach ($existing in $list.Value) {
        if (Same-Path $existing $path) { return }
    }
    $list.Value = $list.Value + $path
}

# $dir itself, then nested folders up to $depth (0 = only $dir).
function Search-Tree($dir, $depth, [ref]$list) {
    if ($depth -lt 0) { return }
    if ([string]::IsNullOrEmpty($dir)) { return }
    if (-not (Test-Path -LiteralPath $dir -ErrorAction SilentlyContinue)) { return }
    Add-Found $list $dir
    if ($depth -eq 0) { return }
    $children = Get-ChildItem -LiteralPath $dir -Directory -ErrorAction SilentlyContinue
    foreach ($child in $children) {
        if (Skip-DirName $child.Name) { continue }
        if (Is-Link $child) { continue }
        # Users\ is covered by the dedicated profile scan; skip it at a
        # drive root so C:\ does not walk every profile twice.
        if ((Is-DriveRoot $dir) -and ($child.Name -eq 'Users')) { continue }
        Search-Tree $child.FullName ($depth - 1) $list
    }
}

function Search-Around($dir, $depth, $label, [ref]$list, [ref]$seen) {
    if ([string]::IsNullOrEmpty($dir)) { return }
    if (-not (Test-Path -LiteralPath $dir -ErrorAction SilentlyContinue)) { return }
    $already = $false
    foreach ($s in $seen.Value) {
        if (Same-Path $s $dir) { $already = $true }
    }
    if ($already) { return }
    $seen.Value = $seen.Value + $dir
    Say ('  ' + $label)
    Search-Tree $dir $depth $list
}

Write-Host ''
Write-Host '  ============================================' -ForegroundColor Cyan
Write-Host '   Seal Cascade - update' -ForegroundColor Cyan
Write-Host '  ============================================' -ForegroundColor Cyan
Say 'This copies the new program into your existing game folder.'
Say 'It will not replace the big artwork files (the .grf files, BGM,'
Say 'lua_files.7z). Your characters stay on the server.'
Write-Host ''

if (-not (Test-Path -LiteralPath (Join-Path $here 'korangar.exe'))) {
    Fail 'korangar.exe is not in this folder.' @(
        'Unzip Seal-Cascade-Windows.zip and run Update from THAT folder,',
        'not from inside the zip and not from your old game folder.'
    )
}

if (Test-Path -LiteralPath (Join-Path $here 'data.grf')) {
    Fail 'this folder already has the game data in it.' @(
        'Update is meant to run from the small unzipped update, then copy',
        'into your existing install. This folder already looks like a full',
        'game. If you meant to play, double-click Play instead.'
    )
}

Say 'Looking for an existing install (a folder that already has data.grf)...'
Say 'Same folder as this zip first, then outward, then other drives.'
Say 'This can take a minute. Each place is printed as it is searched.'
Write-Host ''

$found = @()
$seenDirs = @()

# 1. The folder this zip was unzipped into, then the directory it sits in
#    (Downloads, Desktop, a USB stick, ...), several folders deep so
#    "Seal Cascade" next to the zip or one or two folders under it is found.
Search-Around $here 1 'this unzipped folder' ([ref]$found) ([ref]$seenDirs)

$parent = Split-Path -Parent $here
Search-Around $parent 4 'same directory as this zip (and folders under it)' ([ref]$found) ([ref]$seenDirs)

# 2. The Windows user folder (Desktop, Downloads, Documents, Games, ...),
#    even when the zip itself is on another drive. Do this before walking
#    up to C:\Users, so the profile is searched 4 deep instead of 3.
if (-not [string]::IsNullOrEmpty($env:USERPROFILE)) {
    Search-Around $env:USERPROFILE 4 ('your user folder: ' + $env:USERPROFILE) ([ref]$found) ([ref]$seenDirs)
}
$desktop = [Environment]::GetFolderPath('Desktop')
Search-Around $desktop 3 ('Desktop: ' + $desktop) ([ref]$found) ([ref]$seenDirs)
$docs = [Environment]::GetFolderPath('MyDocuments')
Search-Around $docs 3 ('Documents: ' + $docs) ([ref]$found) ([ref]$seenDirs)

# 3. Walk outward: each parent folder, looking down into its other children.
$cursor = $parent
while (-not [string]::IsNullOrEmpty($cursor)) {
    $next = Split-Path -Parent $cursor
    if ([string]::IsNullOrEmpty($next) -or (Same-Path $next $cursor)) { break }
    $cursor = $next
    $outwardDepth = 3
    if (Is-DriveRoot $cursor) { $outwardDepth = 4 }
    Search-Around $cursor $outwardDepth ('outward: ' + $cursor) ([ref]$found) ([ref]$seenDirs)
}

# 4. Other local disks and USB drives (game on D: / E: while the zip is on C:).
$homeDrive = [System.IO.Path]::GetPathRoot($here)
$disks = @()
try {
    $disks = @(Get-CimInstance -ClassName Win32_LogicalDisk -ErrorAction Stop | Where-Object { $_.DriveType -eq 2 -or $_.DriveType -eq 3 })
} catch {
    try {
        $disks = @(Get-WmiObject -Class Win32_LogicalDisk -ErrorAction Stop | Where-Object { $_.DriveType -eq 2 -or $_.DriveType -eq 3 })
    } catch {
        $disks = @()
    }
}

foreach ($disk in $disks) {
    $root = $disk.DeviceID + '\'
    if (Same-Path $root $homeDrive) { continue }
    Search-Around $root 4 ('drive ' + $root) ([ref]$found) ([ref]$seenDirs)
}

# A single match can stop being an array in PowerShell 5.1; wrap so .Count
# is 1, not the length of the path string.
$found = @($found)

$dest = $null
if ($found.Count -eq 0) {
    Write-Host ''
    Warn 'Could not find a game folder automatically.'
    Say 'It is a folder that already has data.grf next to Play.bat.'
    Say 'Looked next to this zip, outward up the disk, your user folder,'
    Say 'and other local / USB drives. Network drives are skipped.'
    Write-Host ''
    $typed = Read-Host '  Paste the full path to your game folder (or Enter to cancel)'
    if ([string]::IsNullOrEmpty($typed)) {
        Fail 'no game folder was chosen.' @(
            'Find the folder you already play from (it has data.grf in it),',
            'run Update again, and paste that path when asked.'
        )
    }
    $typed = $typed.Trim().Trim('"')
    if (-not (Test-Install $typed)) {
        Fail ('that path is not a Seal Cascade install: ' + $typed) @(
            'It needs data.grf and Play.bat (or korangar.exe) in the same folder.'
        )
    }
    $dest = (Get-Item -LiteralPath $typed).FullName
} elseif ($found.Count -eq 1) {
    $dest = $found[0]
    Good ('Found it: ' + $dest)
} else {
    Write-Host ''
    Say 'Found more than one install:'
    $i = 1
    foreach ($path in $found) {
        Say ('  ' + $i.ToString() + '  ' + $path)
        $i = $i + 1
    }
    Write-Host ''
    $pick = Read-Host '  Type the number to update'
    $index = 0
    if (-not [int]::TryParse($pick, [ref]$index)) {
        Fail 'that was not a number.' @('Run Update again and type one of the numbers listed.')
    }
    if ($index -lt 1 -or $index -gt $found.Count) {
        Fail 'that number is not on the list.' @('Run Update again and pick one of the numbers shown.')
    }
    $dest = $found[$index - 1]
}

if (Same-Path $dest $here) {
    Fail 'the install it found is this folder.' @(
        'Run Update from the unzipped small zip, not from the game itself.'
    )
}

Write-Host ''
Say ('Will copy from:  ' + $here)
Say ('            to:  ' + $dest)
Say 'Keeping your existing .grf files, BGM, lua_files.7z, and personal'
Say 'settings in client (login, window layout, graphics).'
Write-Host ''
$answer = Read-Host '  Copy the update in? [Y/n]'
if (-not ($answer -eq '' -or $answer -eq 'y' -or $answer -eq 'Y')) {
    Fail 'cancelled.' @('Nothing was changed.')
}

Write-Host ''
Say 'Copying. Each name is printed before it moves.'

$copied = 0
$skipped = 0
$items = Get-ChildItem -LiteralPath $here -Force
foreach ($item in $items) {
    $name = $item.Name
    if ($name -eq '.DS_Store' -or $name -eq 'Thumbs.db') { continue }

    $skip = $false
    foreach ($keep in $keepFiles) {
        if ($name -eq $keep) { $skip = $true }
    }
    foreach ($keep in $keepDirs) {
        if ($name -eq $keep) { $skip = $true }
    }
    if ($skip) {
        Say ('  skip (game data): ' + $name)
        $skipped = $skipped + 1
        continue
    }

    $to = Join-Path $dest $name
    if ($item.PSIsContainer) {
        Say ('  ' + $name + '\  (merging folder)')
        if (-not (Test-Path -LiteralPath $to)) {
            New-Item -ItemType Directory -Path $to | Out-Null
        }
        # Copy contents into the existing folder. Copy-Item of the folder
        # itself onto a dest that already has that name can nest archive\archive.
        Copy-Item -LiteralPath (Join-Path $item.FullName '*') -Destination $to -Recurse -Force
    } else {
        $sizeText = Format-Size ([long]$item.Length)
        if ([long]$item.Length -gt 10485760) {
            Say ('  ' + $name + ' (' + $sizeText + ') -- this can take a few seconds.')
        } else {
            Say ('  ' + $name + ' (' + $sizeText + ')')
        }
        Copy-Item -LiteralPath $item.FullName -Destination $to -Force
    }
    $copied = $copied + 1
}

Write-Host ''
Good ('Copied ' + $copied.ToString() + ' item(s). Left your game data alone (' + $skipped.ToString() + ' skipped).')
Say ''
Say 'Next: open your game folder and double-click Verify, then Play.'
Say ('Game folder: ' + $dest)
Write-Host ''

$play = Read-Host '  Start the game now? [Y/n]'
if ($play -eq '' -or $play -eq 'y' -or $play -eq 'Y') {
    $exe = Join-Path $dest 'korangar.exe'
    if (Test-Path -LiteralPath $exe) {
        Say 'Starting. The window can take a minute to appear.'
        Start-Process -FilePath $exe -WorkingDirectory $dest
    }
}
