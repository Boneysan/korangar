#!/bin/sh
# Seal Cascade first-run setup (macOS): do everything, then say it is ready.
#
# The friend runs ONE thing and is either playing or holding a sentence that
# says exactly what to do next. Every check exists because its failure would
# otherwise be silent or unreadable:
#
#   processor   this build is arm64 only; on an Intel Mac the binary dies with
#               "bad CPU type in executable", which names nothing to act on
#   quarantine  an unsigned binary from Drive dies on a dialog with no Open
#               button, and nothing explains why
#   assets      the 3.7 GB half downloads separately and is the likely miss
#   hashes      Drive truncates big files, and every later symptom looks like
#               a bug in the game instead of a bad download

set -u

here=$(cd "$(dirname "$0")" && pwd) || exit 1
cd "$here" || exit 1

say()  { printf '  %s\n' "$1"; }
good() { printf '  OK  %s\n' "$1"; }

fail() {
    printf '\n  Setup stopped: %s\n\n' "$1"
    shift
    for line in "$@"; do printf '  %s\n' "$line"; done
    printf '\n  Press return to close this window. '
    read -r _ 2>/dev/null || true
    exit 1
}

printf '\n  Seal Cascade setup\n\n'
say 'This runs once. It unblocks the download, merges in the game data,'
say 'checks the files, and starts the game.'
printf '\n'

# 0. The processor. Checked first because nothing later matters if the binary
#    cannot execute at all, and because the native failure -- "bad CPU type in
#    executable" -- tells a player nothing.
#
#    `uname -m` alone is NOT enough: a Terminal running under Rosetta on an
#    Apple Silicon Mac also reports x86_64, and that machine is perfectly
#    capable. hw.optional.arm64 is 1 on Apple Silicon whether or not this shell
#    is being translated, so it is the honest answer to "is this an Intel Mac".
if [ "$(uname -m 2>/dev/null)" != "arm64" ] \
    && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" != "1" ]; then
    fail 'this Mac has an Intel processor, and this build is Apple Silicon only.' \
        'Nothing you can install fixes this -- the game has to be rebuilt for' \
        'Intel. Tell the host you have an Intel Mac (Apple menu > About This' \
        'Mac); an Intel build is possible, it just is not in this download.'
fi
good 'Apple Silicon'

# 1. Quarantine. Everything from Drive carries it, including this script; the
#    player had to right-click -> Open just to get here. Clearing the whole
#    folder means Play works by double-click from now on.
if xattr -dr com.apple.quarantine "$here" 2>/dev/null; then
    good 'download unblocked'
else
    say 'could not clear the quarantine flag; carrying on'
fi

chmod +x "$here/korangar" "$here"/*.command 2>/dev/null || true

[ -f "$here/korangar" ] || fail 'the game program is not in this folder.' \
    'Download the macOS folder again from the shared Drive folder.'

# 2. The asset half. Look where a person would plausibly have put it.
payload='data.grf rdata.grf renewal2021.grf resources2021.grf lua_files.7z BGM SHA256SUMS-assets'

have_all=1
for name in $payload; do
    [ -e "$here/$name" ] || have_all=0
done

if [ "$have_all" -eq 1 ]; then
    good 'game data already in place'
else
    source=''
    for candidate in "$here/Assets" "$here/../Assets" "$HOME/Downloads/Assets"; do
        if [ -d "$candidate" ] && [ -f "$candidate/data.grf" ]; then
            source=$(cd "$candidate" && pwd)
            break
        fi
    done

    [ -n "$source" ] || fail 'the game data (Assets) is not here.' \
        'There are two downloads in the shared Drive folder, and this is' \
        'only the small one. You still need the big one:' \
        '' \
        '    Assets    about 3.7 GB - artwork, maps and music' \
        '' \
        'Download it, unzip it if it arrived zipped, put the Assets folder' \
        'next to this one, and run Setup again.' \
        '' \
        'Setup looked in this folder, the folder above it, and Downloads.'

    good "found it: $source"

    # Same filesystem means a rename is instant and costs no extra disk.
    # Across volumes there is no cheap move, so copy and let them delete the
    # download themselves.
    from_dev=$(stat -f %d "$source")
    to_dev=$(stat -f %d "$here")

    if [ "$from_dev" = "$to_dev" ]; then
        say 'Moving the game data into this folder (this is quick).'
    else
        say 'Copying the game data into this folder. It is 3.7 GB, so this'
        say 'takes a few minutes. Leave this window open.'
    fi

    for name in $payload; do
        if [ -e "$here/$name" ]; then
            say "  already here, skipping: $name"
            continue
        fi
        [ -e "$source/$name" ] || fail "$name is missing from the Assets folder." \
            'That download is incomplete. Download Assets again from the' \
            'shared Drive folder, then run Setup again.'

        say "  $name"
        if [ "$from_dev" = "$to_dev" ]; then
            mv "$source/$name" "$here/$name" || fail "could not move $name into place." \
                'Check that this folder is not read-only, then run Setup again.'
        else
            cp -R "$source/$name" "$here/$name" || fail "could not copy $name into place." \
                'Check that there is enough free disk space, then run Setup again.'
        fi
    done

    good 'game data is in place'
fi

# 3. Checksums. The whole point of shipping the manifests.
for manifest in SHA256SUMS-client SHA256SUMS-assets; do
    [ -f "$here/$manifest" ] || continue
    say "checking $manifest (this takes a moment)"
    if shasum -a 256 -c "$here/$manifest" > /dev/null 2>&1; then
        good "$manifest matches"
    else
        fail "some files do not match $manifest." \
            'The download is damaged. Double-click Verify to see which files,' \
            'then download that folder again from the shared Drive folder.'
    fi
done

# 4. Tailscale. Not fatal -- Setup should still leave a playable folder if it
# is merely not installed yet. Checked here because the failure lands one
# screen late: login is a separate connection and succeeds, and it is character
# select that fails, which reads as a broken game rather than a missing VPN.
if [ -x /Applications/Tailscale.app/Contents/MacOS/Tailscale ] \
    || [ -x /usr/local/bin/tailscale ] || [ -x /opt/homebrew/bin/tailscale ] \
    || command -v tailscale > /dev/null 2>&1; then
    good 'Tailscale is installed -- make sure it is signed in and connected'
else
    say 'Tailscale is NOT installed.'
    say ''
    say 'The game server is not on the public internet: it runs on the host'\''s'
    say 'computer, and Tailscale is how you reach it. Install it from'
    say 'https://tailscale.com/download, sign in, and accept the host'\''s invite.'
    say ''
    say 'You can play once that is done. Without it you will reach the login'
    say 'screen and then fail when you pick a character.'
fi

printf '\n'
good 'Ready. Starting the game -- from now on, just double-click Play.'
printf '\n'

exec "$here/korangar"
