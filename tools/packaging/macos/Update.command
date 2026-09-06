#!/bin/sh
# Seal Cascade client update (macOS): find an existing install and copy this
# small pack over it, keeping the 3.7 GB game data.
#
# Run from the unzipped Seal-Cascade-macOS.zip. POSIX sh; see Play.command.

set -u

here=$(cd "$(dirname "$0")" && pwd) || exit 1
cd "$here" || exit 1

say()  { printf '  %s\n' "$1"; }
good() { printf '  OK  %s\n' "$1"; }

fail() {
    printf '\n  Update stopped: %s\n\n' "$1"
    shift
    for line in "$@"; do printf '  %s\n' "$line"; done
    printf '\n  Press return to close this window. '
    read -r _ 2>/dev/null || true
    exit 1
}

hold() {
    printf '\n  Press return to close this window. '
    read -r _ 2>/dev/null || true
}

is_install() {
    [ -d "$1" ] || return 1
    [ -f "$1/data.grf" ] || return 1
    [ -f "$1/korangar" ] || [ -f "$1/Play.command" ] || return 1
    return 0
}

skip_name() {
    case $1 in
        System|Applications|Library|private|usr|bin|sbin|cores|Preboot|Recovery|VM|.Spotlight-V100|Network)
            return 0 ;;
        node_modules|.git|.svn|.Trash|.Trashes|Caches|Containers)
            return 0 ;;
    esac
    return 1
}

add_found() {
    [ -n "$1" ] || return 0
    [ "$1" = "$here" ] && return 0
    is_install "$1" || return 0
    grep -Fxq "$1" "$list" 2>/dev/null && return 0
    printf '%s\n' "$1" >> "$list"
}

# $1 itself, then nested folders up to $2 (0 = only $1).
# POSIX sh has no `local`: $1/$2 stay per-call, but assigning depth= would
# leak into the caller and skip later siblings. Use the positional params.
search_tree() {
    [ "$2" -lt 0 ] && return 0
    [ -d "$1" ] || return 0
    add_found "$1"
    [ "$2" -eq 0 ] && return 0
    for child in "$1"/*; do
        [ -d "$child" ] || continue
        [ -L "$child" ] && continue
        name=$(basename "$child")
        skip_name "$name" && continue
        # Users on a volume is scanned via $HOME instead, so a search of
        # Macintosh HD does not walk every profile.
        if [ "$name" = "Users" ]; then
            case $1 in
                /Volumes/*) continue ;;
            esac
        fi
        child=$(cd "$child" && pwd) || continue
        search_tree "$child" $(($2 - 1))
    done
}

seen_file=""

search_around() {
    dir=$1
    depth=$2
    label=$3
    [ -n "$dir" ] || return 0
    [ -d "$dir" ] || return 0
    dir=$(cd "$dir" && pwd) || return 0
    grep -Fxq "$dir" "$seen_file" 2>/dev/null && return 0
    printf '%s\n' "$dir" >> "$seen_file"
    say "  $label"
    search_tree "$dir" "$depth"
}

printf '\n  Seal Cascade update\n\n'
say 'This copies the new program into your existing game folder.'
say 'It will not replace the big artwork files. Your characters stay'
say 'on the server.'
printf '\n'

[ -f "$here/korangar" ] || fail 'the game program is not in this folder.' \
    'Unzip Seal-Cascade-macOS.zip and run Update from THAT folder.'

if [ -f "$here/data.grf" ]; then
    fail 'this folder already has the game data in it.' \
        'Update is meant to run from the small unzipped update.' \
        'If you meant to play, double-click Play instead.'
fi

say 'Looking for an existing install (a folder that already has data.grf)...'
say 'Same folder as this zip first, then outward, then other volumes.'
say 'This can take a minute. Each place is printed as it is searched.'
printf '\n'

list="/tmp/seal-cascade-update-$$"
seen_file="/tmp/seal-cascade-update-seen-$$"
trap 'rm -f "$list" "$seen_file"' EXIT INT HUP
: > "$list"
: > "$seen_file"

# 1. The unzipped folder, then the directory it sits in (Downloads, Desktop,
#    a USB stick, ...), several folders deep so a game next to the zip or a
#    couple of folders under it is found.
search_around "$here" 1 'this unzipped folder'
parent=$(cd "$here/.." && pwd) || parent=''
search_around "$parent" 4 'same directory as this zip (and folders under it)'

# 2. Home folder (Desktop, Downloads, Documents, Games, ...) even when the
#    zip itself is on another volume. Do this before walking up through
#    /Users, so home is searched 4 deep instead of 3.
if [ -n "${HOME:-}" ]; then
    search_around "$HOME" 4 "your home folder: $HOME"
    search_around "$HOME/Desktop" 3 "Desktop: $HOME/Desktop"
    search_around "$HOME/Documents" 3 "Documents: $HOME/Documents"
    search_around "$HOME/Downloads" 3 "Downloads: $HOME/Downloads"
fi

# 3. Walk outward: each parent folder, looking down into its other children.
#    Stop at / and /Volumes -- those are covered by $HOME and the
#    other-volume pass.
cursor=$parent
prev=''
while [ -n "$cursor" ] && [ "$cursor" != "$prev" ]; do
    prev=$cursor
    cursor=$(cd "$cursor/.." && pwd) || break
    [ "$cursor" = "/" ] && break
    [ "$cursor" = "/Volumes" ] && break
    search_around "$cursor" 3 "outward: $cursor"
done

# 4. Other mounted volumes (game on an external disk while the zip is on
#    Macintosh HD). Include the zip's own volume from the mount root so a
#    game at /Volumes/T7/Games is found even if the zip is nested deeper
#    than the outward walk looked.
here_vol=$(df "$here" | awk 'NR==2 { print $1 }')
for vol in /Volumes/*; do
    [ -d "$vol" ] || continue
    [ -L "$vol" ] && continue
    vol_dev=$(df "$vol" 2>/dev/null | awk 'NR==2 { print $1 }')
    [ -n "$vol_dev" ] || continue
    if [ "$vol_dev" = "$here_vol" ]; then
        search_around "$vol" 4 "this disk: $vol"
    else
        search_around "$vol" 4 "other volume: $vol"
    fi
done
nfound=0
if [ -s "$list" ]; then
    nfound=$(grep -c . "$list")
fi

dest=''
if [ "$nfound" -eq 0 ]; then
    printf '\n'
    say 'Could not find a game folder automatically.'
    say 'It is a folder that already has data.grf next to Play.'
    say 'Looked next to this zip, outward up the disk, your home folder,'
    say 'and other volumes (external drives).'
    printf '\n  Paste the full path to your game folder (or return to cancel): '
    read -r typed 2>/dev/null || typed=''
    [ -n "$typed" ] || fail 'no game folder was chosen.' \
        'Find the folder you already play from and run Update again.'
    typed=$(cd "$typed" && pwd) || fail 'that path could not be opened.' \
        'Check it and run Update again.'
    is_install "$typed" || fail 'that path is not a Seal Cascade install.' \
        'It needs data.grf and Play (or korangar) in the same folder.'
    dest=$typed
elif [ "$nfound" -eq 1 ]; then
    dest=$(sed -n '1p' "$list")
    good "found it: $dest"
else
    printf '\n'
    say 'Found more than one install:'
    i=1
    while IFS= read -r path; do
        say "  $i  $path"
        i=$((i + 1))
    done < "$list"
    printf '\n  Type the number to update: '
    read -r pick 2>/dev/null || pick=''
    dest=$(sed -n "${pick}p" "$list")
    [ -n "$dest" ] || fail 'that number is not on the list.' \
        'Run Update again and pick one of the numbers shown.'
fi

[ "$dest" != "$here" ] || fail 'the install it found is this folder.' \
    'Run Update from the unzipped small zip, not from the game itself.'

printf '\n'
say "Will copy from:  $here"
say "            to:  $dest"
say 'Keeping your existing .grf files, BGM, lua_files.7z, and personal'
say 'settings in client (login, window layout, graphics).'
printf '\n  Copy the update in? [Y/n] '
read -r answer 2>/dev/null || answer=''
case $answer in
    ''|Y|y) ;;
    *) fail 'cancelled.' 'Nothing was changed.' ;;
esac

printf '\n'
say 'Copying. Each name is printed before it moves.'

copied=0
skipped=0
for item in "$here"/* "$here"/.[!.]*; do
    [ -e "$item" ] || continue
    name=$(basename "$item")
    case $name in
        .DS_Store|Thumbs.db) continue ;;
        data.grf|rdata.grf|renewal2021.grf|resources2021.grf|lua_files.7z|SHA256SUMS-assets|BGM)
            say "  skip (game data): $name"
            skipped=$((skipped + 1))
            continue
            ;;
    esac

    if [ -d "$item" ]; then
        say "  $name/  (merging folder)"
        mkdir -p "$dest/$name"
        cp -R "$item/." "$dest/$name/" || fail "could not copy $name." \
            'Check that the game folder is not read-only.'
    else
        say "  $name"
        cp "$item" "$dest/$name" || fail "could not copy $name." \
            'Check that the game folder is not read-only.'
    fi
    copied=$((copied + 1))
done

xattr -dr com.apple.quarantine "$dest" 2>/dev/null || true
chmod +x "$dest/korangar" "$dest"/*.command 2>/dev/null || true

printf '\n'
good "Copied $copied item(s). Left your game data alone ($skipped skipped)."
say ''
say 'Next: open your game folder, right-click Verify -> Open, then Play.'
say "Game folder: $dest"
printf '\n  Start the game now? [Y/n] '
read -r play 2>/dev/null || play=''
case $play in
    ''|Y|y)
        say 'Starting. The window can take a minute to appear. Leave this open until then.'
        cd "$dest" || exit 1
        exec ./korangar
        ;;
esac
hold
