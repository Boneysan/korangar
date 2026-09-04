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

# Finder closes the window on exit, so a message the player never sees is the
# same as no message. Hold the window open on every failure path.
fail() {
    printf '\n  Seal Cascade cannot start: %s\n\n  %s\n\n' "$1" "$2"
    printf '  Press return to close this window. '
    read -r _ 2>/dev/null || true
    exit 1
}

[ -f ./korangar ] || fail 'the game program is not in this folder.' \
    'Keep Play next to the korangar program. Download the macOS folder again if it has gone.'

# Anything that arrives via a browser, Drive or AirDrop carries
# com.apple.quarantine, and an unsigned quarantined binary dies on a dialog
# with no Open button. Setup clears the whole folder; clear the binary here
# too, so a player who went straight to Play is not stuck.
xattr -d com.apple.quarantine ./korangar 2>/dev/null || true
chmod +x ./korangar 2>/dev/null || true

missing=''
for name in data.grf rdata.grf renewal2021.grf resources2021.grf lua_files.7z; do
    [ -f "$name" ] || missing="$missing $name"
done
[ -z "$missing" ] && [ -d ./BGM ] || fail "the game data is missing ($(echo $missing) )." \
    'Run Setup first. It merges in the big Assets download.'

[ -d ./archive ] || fail 'the archive folder is missing.' \
    'Download the macOS folder again -- archive ships inside it.'

[ -f ./client/server.ron ] || fail 'client/server.ron is missing, so the game does not know which server to join.' \
    'Download the macOS folder again -- server.ron ships already filled in, and is not something you write.'

# One cheap spot-check of the big half. Drive truncates large downloads and
# resumes them badly, and every later symptom of that looks like a bug in the
# game rather than a bad download. lua_files.7z is 3 MB, so this costs nothing;
# Verify checks all 3.7 GB when it matters.
if [ -f SHA256SUMS-assets ]; then
    expected=$(awk 'tolower($2) ~ /(^|\/)lua_files\.7z$/ { print $1; exit }' SHA256SUMS-assets)
    if [ -n "$expected" ]; then
        actual=$(shasum -a 256 lua_files.7z | cut -d' ' -f1)
        [ "$actual" = "$expected" ] || fail 'lua_files.7z does not match the checksum list.' \
            'That download is damaged. Download Assets again, then double-click Verify.'
    fi
fi

exec ./korangar
