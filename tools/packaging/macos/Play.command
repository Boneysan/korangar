#!/bin/sh
# Seal Cascade launcher (macOS).
#
# Finder launches a .command with the working directory set to /, and EVERY
# path the client reads is relative to the working directory. Starting the
# binary by double-clicking it therefore fails in a way that looks like
# missing game data. That is the whole reason this file exists.
#
# Kept to POSIX sh: /bin/sh on macOS is bash 3.2 in POSIX mode, and a friend's
# machine is not the place to discover a bashism.

set -u

here=$(cd "$(dirname "$0")" && pwd) || exit 1
cd "$here" || exit 1

say()  { printf '  %s\n' "$1"; }
good() { printf '  OK  %s\n' "$1"; }

# Finder closes the window on exit, so a message the player never sees is the
# same as no message. Hold the window open on every failure path.
fail() {
    printf '\n  Seal Cascade cannot start: %s\n\n  %s\n\n' "$1" "$2"
    printf '  Press return to close this window. '
    read -r _ 2>/dev/null || true
    exit 1
}

printf '\n  Seal Cascade\n\n'
if [ -f "$here/VERSION" ]; then
    say "Pack version $(tr -d '[:space:]' < "$here/VERSION")"
else
    say 'Pack version unknown (no VERSION file -- this is an old download).'
fi
say 'Checking this folder before starting. Nothing is sent anywhere.'
printf '\n'

# Cheapest possible check, and the most fundamental: an Intel Mac cannot run
# this binary at all. See Setup.command for why `uname -m` alone is not enough.
say 'Checking this Mac...'
if [ "$(uname -m 2>/dev/null)" != "arm64" ] \
    && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" != "1" ]; then
    fail 'this Mac has an Intel processor, and this build is Apple Silicon only.' \
        'Tell the host you have an Intel Mac -- an Intel build is possible, it is just not in this download.'
fi
good 'Apple Silicon'

[ -f ./korangar ] || fail 'the game program is not in this folder.' \
    'Keep Play next to the korangar program. Download the macOS folder again if it has gone.'

# Anything that arrives via a browser, Drive or AirDrop carries
# com.apple.quarantine, and an unsigned quarantined binary dies on a dialog
# with no Open button. Setup clears the whole folder; clear the binary here
# too, so a player who went straight to Play is not stuck.
say 'Clearing macOS download warnings on the game...'
xattr -d com.apple.quarantine ./korangar 2>/dev/null || true
chmod +x ./korangar 2>/dev/null || true
good 'game program is ready'

say 'Looking for game data...'
missing=''
for name in data.grf rdata.grf renewal2021.grf resources2021.grf lua_files.7z; do
    [ -f "$name" ] || missing="$missing $name"
done
[ -z "$missing" ] && [ -d ./BGM ] || fail "the game data is missing ($(echo $missing) )." \
    'Run Setup first. It merges in the big Assets download.'
good 'game data is here'

[ -d ./archive ] || fail 'the archive folder is missing.' \
    'Download the macOS folder again -- archive ships inside it.'

[ -f ./client/server.ron ] || fail 'client/server.ron is missing, so the game does not know which server to join.' \
    'Download the macOS folder again -- server.ron ships already filled in, and is not something you write.'
good 'server settings are here'

# One cheap spot-check of the big half. Drive truncates large downloads and
# resumes them badly, and every later symptom of that looks like a bug in the
# game rather than a bad download. lua_files.7z is 3 MB, so this costs nothing;
# Verify checks all 3.7 GB when it matters.
if [ -f SHA256SUMS-assets ]; then
    say 'Spot-checking lua_files.7z (a few seconds)...'
    expected=$(awk 'tolower($2) ~ /(^|\/)lua_files\.7z$/ { print $1; exit }' SHA256SUMS-assets)
    if [ -n "$expected" ]; then
        actual=$(shasum -a 256 lua_files.7z | cut -d' ' -f1)
        [ "$actual" = "$expected" ] || fail 'lua_files.7z does not match the checksum list.' \
            'That download is damaged. Download Assets again, then double-click Verify.'
        good 'lua_files.7z matches'
    fi
fi

printf '\n'
say 'Starting the game.'
say 'The window can take a minute to appear, especially the first time.'
say 'Leave this open until you see it. You can close this afterwards.'
printf '\n'

exec ./korangar
