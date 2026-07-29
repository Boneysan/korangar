#!/usr/bin/env bash
#
# Observer-parity audits — static checks over both source trees.
#
# Answers one question in nine places: does state that reaches a client
# out-of-band from the spawn packet have a recovery mechanism? See
# docs/plans/observer-parity-audits.md (the greps and their rationale) and
# docs/plans/observer-parity-harness.md (why these boundaries, and in what
# order to fix them).
#
# An audit does NOT pass by returning nothing. It passes when every hit is
# CLASSIFIED — recorded in observer-parity.baseline with a comment saying why
# it is safe. This script therefore diffs findings against that baseline:
#
#   new finding      -> exit 1. Triage it, then either fix it or add it to the
#                       baseline WITH a rationale comment.
#   vanished finding -> exit 2. Something was fixed (or moved). Re-run with
#                       --update and commit the smaller baseline.
#
# Usage:
#   ./observer-parity.sh              # check against the baseline
#   ./observer-parity.sh --list       # print findings, no comparison
#   ./observer-parity.sh --update     # rewrite the baseline (keeps comments)
#
# Needs no running server and no build. Safe to run in CI.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
korangar_root="$(cd "$script_dir/../.." && pwd)"
hercules_dir="${HERCULES_DIR:-$(cd "$korangar_root/.." && pwd)/Hercules}"
baseline="$script_dir/observer-parity.baseline"

mode="check"
case "${1:-}" in
    --list) mode="list" ;;
    --update) mode="update" ;;
    --help | -h)
        sed -n '3,26p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
        exit 0
        ;;
    "") ;;
    *)
        echo "unknown argument: $1 (try --help)" >&2
        exit 64
        ;;
esac

client_lib="$korangar_root/korangar/src/lib.rs"
entity_mod="$korangar_root/korangar/src/world/entity/mod.rs"
entity_data="$korangar_root/korangar-networking/src/entity.rs"
packets="$korangar_root/ragnarok-packets/src/lib.rs"
version_map="$korangar_root/korangar-networking/src/packet_versions/version_20220406.rs"
clif="$hercules_dir/src/map/clif.c"

for required in "$client_lib" "$entity_mod" "$entity_data" "$packets" "$version_map"; do
    if [[ ! -f "$required" ]]; then
        echo "missing client source: $required" >&2
        exit 70
    fi
done

# Pull the `pub <name>` field list out of a struct or the variant list out of
# an enum. Both are `awk` over a brace-delimited block, so one helper does.
struct_fields() { # <file> <struct name>
    awk -v name="$2" '$0 ~ "^pub struct " name " *\\{", /^\}/' "$1" |
        grep -oE '^    pub [a-z_0-9]+' | awk '{ print $2 }' | sort -u
}

enum_variants() { # <file> <enum name>
    awk -v name="$2" '$0 ~ "^pub enum " name " *\\{", /^\}/' "$1" |
        grep -oE '^    [A-Z][A-Za-z0-9]*,?$' | tr -d ' ,' | sort -u
}

emit() { # <audit id> <finding text>
    printf '%s|%s\n' "$1" "$2"
}

findings() {
    # --- A1: entity-keyed handlers, as a tripwire on the count ---------------
    # 40 sites share the shape that silently dropped ammunition; all are
    # currently classified by recovery mechanism in the audits doc. The count
    # is the signal: a new one means a new handler to triage.
    emit A1 "entity-keyed-find-sites=$(grep -c 'find(|entity| entity.get_entity_id()' "$client_lib" || true)"

    # --- A2: Common state absent from EntityData ----------------------------
    # `AddEntity` rebuilds an on-screen entity from EntityData alone, so any
    # Common field not re-derivable from it is wiped on rebuild.
    comm -23 <(struct_fields "$entity_mod" Common) <(struct_fields "$entity_data" EntityData) |
        while read -r field; do emit A2 "common-field-not-in-entitydata=$field"; done

    # --- A3: appearance state gated on Entity::Player ------------------------
    # Remote players are built as Entity::Npc and use Common, so anything
    # reachable only through Player is invisible to every observer.
    grep -oE 'if let Self::Player\([^)]*\)' "$entity_mod" | sort -u |
        while read -r hit; do emit A3 "player-gated=$hit"; done

    # --- A9: local-player path vs remote-entity path asymmetry ---------------
    # The local player is mutated through `this_entity()` bindings; every other
    # entity is built and mutated in the `AddEntity` arm. The two paths must
    # leave an entity in the same state, and nothing makes them — they are
    # separate code that drifts. `set_in_safe_zone` reached both paths but its
    # companion `refresh_neutral_stance` reached only one, so remote armed
    # players kept the battle stance on town maps (found 2026-07-29).
    #
    # Only *mutating* methods are compared; getters are noise. A hit is a
    # mutation one path performs and the other neither performs nor derives
    # from `EntityData` at construction — deriving it is a valid classification.
    local mutator='^(set_|refresh_|revive$|clear_|move_from_to$|inherit_)'

    local_path_mutators() {
        grep -oE '\b(player|local_player)\.[a-z_]+\(' "$client_lib" |
            sed 's/.*\.//; s/(//' | grep -E "$mutator" | sort -u
    }

    # Capture the AddEntity arm without depending on which arm follows it.
    add_entity_arm_mutators() {
        awk '/NetworkEvent::AddEntity \{/ { f = 1 }
             f && /^                NetworkEvent::/ && !/AddEntity/ { f = 0 }
             f' "$client_lib" |
            grep -oE '\bnpc\.[a-z_]+\(' | sed 's/npc\.//; s/(//' | grep -E "$mutator" | sort -u
    }

    comm -23 <(local_path_mutators) <(add_entity_arm_mutators) |
        while read -r method; do emit A9 "local-only-mutator=$method"; done
    comm -13 <(local_path_mutators) <(add_entity_arm_mutators) |
        while read -r method; do emit A9 "addentity-only-mutator=$method"; done

    # --- A6 is a diagnosability rule, not a grep; A7 is the harness. --------

    # --- B2a: sprite-change look types with no NetworkEvent ------------------
    comm -23 <(enum_variants "$packets" SpriteChangeType) \
        <(awk '/register\(\|packet: SpriteChangePacket\|/,/^    \}\)\?;/' "$version_map" |
            grep -oE 'SpriteChangeType::[A-Za-z][A-Za-z0-9]*' | sed 's/.*:://' | sort -u) |
        while read -r variant; do emit B2a "look-type-unmapped=$variant"; done

    # --- B2b: packets the server sends and the client deliberately drops -----
    grep -oE 'register_noop::<[A-Za-z0-9_]+>' "$version_map" |
        sed 's/register_noop::<//; s/>//' | sort -u |
        while read -r packet; do emit B2b "noop=$packet"; done

    # --- B2c: spawn-packet appearance the EntityData boundary discards -------
    # The widest gap found 2026-07-29: the wire carries the full appearance and
    # this struct keeps 17 of 27 fields.
    comm -23 <(struct_fields "$packets" EntityAppearPacket) <(struct_fields "$entity_data" EntityData) |
        while read -r field; do emit B2c "spawn-field-dropped=$field"; done

    # --- Server-side audits (skipped without the Hercules tree) -------------
    if [[ ! -f "$clif" ]]; then
        emit SKIP "hercules-tree-not-found=$hercules_dir"
        return
    fi

    # --- A4: AREA broadcasts issued before map->addblock --------------------
    # An AREA send walks the map's block list, so anything broadcast before the
    # character joins it reaches nobody.
    awk '/^static void clif_parse_LoadEndAck\(/,/^\}/' "$clif" |
        awk '/map->addblock/ { found = 1 }
             !found && /clif->(changelook|sendlook)\(/ { gsub(/^[ \t]+|[ \t]+$/, ""); print }' |
        sort -u | while read -r hit; do emit A4 "before-addblock=$hit"; done

    # --- A5: enter-view re-sends that cannot transmit "none" ----------------
    # Harmless for any attribute the spawn packet carries (set_unit_idle runs
    # unconditionally just above) — which is why each hit needs a rationale
    # rather than a fix. See the corrected A5 entry in the audits doc.
    awk '/^static void clif_getareachar_unit\(/,/^\}/' "$clif" |
        awk '/refreshlook\(/ && prev ~ /if *\(/ {
                 guard = prev; line = $0
                 gsub(/^[ \t]+|[ \t]+$/, "", guard)
                 match(line, /LOOK_[A-Z_0-9]+/)
                 print substr(line, RSTART, RLENGTH) " guarded-by " guard
             }
             { prev = $0 }' |
        sort -u | while read -r hit; do emit A5 "guarded-resend=$hit"; done

    # --- A8: wholesale writes to sd->vd ------------------------------------
    # Every fork-added view_data field inherits this lifetime problem: a
    # memcpy over the whole struct zeroes it, and the other branch never
    # re-assigns it. This is how disguise permanently zeroed vd->ammo.
    grep -rhoE 'memcpy\(&sd->vd[^;]*|memset\(&sd->vd[^;]*|sd->vd = [^;]*' "$hercules_dir"/src/map/*.c |
        sed 's/[ \t]*$//' | sort -u |
        while read -r hit; do emit A8 "wholesale-vd-write=$hit"; done
}

current="$(findings | sort)"

case "$mode" in
    list)
        printf '%s\n' "$current"
        exit 0
        ;;
    update)
        # Rewrite in place rather than regenerating: the rationale comments are
        # interleaved with the findings they explain, and that adjacency IS the
        # audit. So keep every comment where it is, keep findings that still
        # occur, drop the ones that don't, and append genuinely new hits under a
        # banner so they cannot be mistaken for classified ones.
        tmp="$(mktemp)"
        trap 'rm -f "$tmp"' EXIT

        if [[ -f "$baseline" ]]; then
            while IFS= read -r line; do
                case "$line" in
                    '#'* | '') printf '%s\n' "$line" ;;
                    *) printf '%s\n' "$current" | grep -Fxq -- "$line" && printf '%s\n' "$line" ;;
                esac
            done <"$baseline" >"$tmp"
            previous="$(grep -v '^#' "$baseline" | grep -v '^[[:space:]]*$' | sort)"
        else
            {
                printf '# Classified observer-parity findings. Every line without a leading `#`\n'
                printf '# is a hit that has been triaged. The comments ARE the audit.\n'
            } >"$tmp"
            previous=""
        fi

        added="$(comm -23 <(printf '%s\n' "$current") <(printf '%s\n' "$previous"))"
        if [[ -n "$added" ]]; then
            {
                printf '#\n'
                printf '# --- NEEDS CLASSIFICATION (appended by --update) ---------------------\n'
                printf '# Write a "# why:" above each of these and move it next to its audit,\n'
                printf '# then delete this banner. A hit with no rationale is not classified,\n'
                printf '# and an audit with unclassified hits has not passed.\n'
                printf '%s\n' "$added"
            } >>"$tmp"
        fi

        cp "$tmp" "$baseline"
        echo "baseline updated: $baseline"
        if [[ -n "$added" ]]; then
            echo "$(printf '%s\n' "$added" | wc -l | tr -d ' ') new hit(s) appended — classify them before committing."
        fi
        exit 0
        ;;
esac

if [[ ! -f "$baseline" ]]; then
    echo "no baseline at $baseline — run with --update to create one" >&2
    exit 70
fi

known="$(grep -v '^#' "$baseline" | grep -v '^[[:space:]]*$' | sort)"
new="$(comm -23 <(printf '%s\n' "$current") <(printf '%s\n' "$known"))"
gone="$(comm -13 <(printf '%s\n' "$current") <(printf '%s\n' "$known"))"

status=0

if [[ -n "$new" ]]; then
    echo "UNCLASSIFIED findings — triage each, then fix it or baseline it with a rationale:"
    printf '%s\n' "$new" | sed 's/^/  + /'
    status=1
fi

if [[ -n "$gone" ]]; then
    echo "STALE baseline entries — these no longer occur (fixed? moved?). Re-run --update:"
    printf '%s\n' "$gone" | sed 's/^/  - /'
    [[ $status -eq 0 ]] && status=2
fi

if [[ $status -eq 0 ]]; then
    echo "observer-parity: all $(printf '%s\n' "$current" | wc -l | tr -d ' ') findings classified."
fi

# A finding can be classified and still be a bug. Those are marked `# OPEN:` in
# the baseline and reprinted on every run, so a green exit never reads as
# "nothing wrong here" — only as "nothing NEW here".
open_items="$(grep '^# OPEN:' "$baseline" || true)"
if [[ -n "$open_items" ]]; then
    echo
    echo "Classified but still open ($(printf '%s\n' "$open_items" | wc -l | tr -d ' ') items):"
    printf '%s\n' "$open_items" | sed 's/^# OPEN:/  !/'
fi

exit $status
