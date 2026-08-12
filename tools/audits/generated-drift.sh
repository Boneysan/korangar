#!/usr/bin/env bash
#
# Does every @generated file in the tree still match its generator?
#
# WHY THIS EXISTS. Four files here are generated from Hercules' own data, and a
# generated file that has silently stopped matching its generator is this
# repo's most-repeated failure:
#
#   * `messages_*.h` and `packets<year>_len_*.h` are banner-marked "do not commit
#     manual changes" and carry fork deltas anyway — a regeneration drops them
#     and SAYS NOTHING. `generate_message_table.py` grew a `CORRECTED_UPSTREAM`
#     sentinel for exactly that.
#   * The symptom is never an error. It is plausible text (an Yggdrasil Berry
#     reporting "Content has been saved in [SaveData_ExMacro5]") or a packet the
#     client silently mis-frames.
#
# Nothing checked the other direction — that the file IN THIS TREE is what the
# generator produces TODAY. It found drift on its first run: the length table
# was generated before the three fork packets (0x0EFE, 0x0EFF, 0x0F00) gained
# their entries in Hercules' hand-maintained `src/common/packets_len.h`, so
# every future regeneration would have shown a diff nobody could explain.
#
# Read-only: each file is snapshotted, regenerated, compared, and restored —
# including on failure. Safe to run on a dirty tree, and it does not use git, so
# an uncommitted edit to a generated file is preserved rather than clobbered.
#
#   tools/audits/generated-drift.sh
#
# Needs the sibling Hercules tree; override with HERCULES_DIR=/path/to/Hercules.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
hercules="${HERCULES_DIR:-$(cd "$repo/../Hercules" 2>/dev/null && pwd || true)}"
cd "$repo"

if [ -z "$hercules" ] || [ ! -d "$hercules" ]; then
    echo "SKIP|hercules-tree-not-found — set HERCULES_DIR"
    exit 0
fi

scratch="$(mktemp -d)"
restore() {
    # Unconditional, and before any exit path: a generator that half-wrote a
    # file must not leave it that way.
    for entry in "${generated[@]}"; do
        path="${entry%%|*}"
        [ -f "$scratch/$(basename "$path")" ] && cp "$scratch/$(basename "$path")" "$repo/$path"
    done
    rm -rf "$scratch"
}

# path | how to regenerate it.
# The three CLIs disagree on how to take the Hercules path (positional, positional
# plus variant, and `--hercules`); that is upstream style drift, not worth
# unifying here, but it is why each line spells its command out in full.
generated=(
    "korangar-networking/src/packet_versions/lengths_20220406.rs|tools/generate_packet_lengths.sh $hercules 20220406 main"
    "korangar-networking/src/packet_versions/skill_states.rs|python3 tools/generate_skill_states.py $hercules"
    "korangar/src/world/library/msgstringtable.rs|python3 tools/generate_message_table.py $hercules main"
    "korangar-networking/examples/headless-tester/scenarios/skill_expectations.rs|python3 tools/generate_skill_expectations.py --hercules $hercules"
)

trap restore EXIT

# **Two of the four generators rustfmt their own output, so this audit is only
# honest when rustfmt can apply `rustfmt.toml`.**
#
# That file uses options which are nightly-only (`wrap_comments`,
# `format_strings`, `imports_granularity`, …), and stable rustfmt does not
# error on them — it prints "can't set ... unstable" to stderr and formats
# with everything else, producing output that is valid, plausible, and
# different. Every line then reads as drift.
#
# It cost a red CI run to find, and the failure was maximally misleading: two
# files reported DRIFTED with real-looking diffs (a comment unwrapped, a tuple
# collapsed onto one line) against a Hercules checkout that was byte-identical
# to the pinned revision. Nothing had drifted. The runner had simply resolved
# `rustfmt` to stable, because `rust-toolchain.toml` only applies inside the
# repo and the toolchain had been installed from outside it.
#
# Probed rather than version-sniffed: this asks whether the options in *this*
# `rustfmt.toml` are actually being applied, so it keeps working if a future
# stable rustfmt stabilises them, and it fails if someone adds a new unstable
# option to the file.
if [ -f "$repo/rustfmt.toml" ] && command -v rustfmt >/dev/null 2>&1; then
    rejected="$(printf 'fn probe() {}\n' | rustfmt --config-path="$repo/rustfmt.toml" --emit stdout 2>&1 | grep -ci 'unstable' || true)"
    if [ "$rejected" -gt 0 ]; then
        echo "generated-drift audit — CANNOT RUN"
        echo "  $(rustfmt --version) ignores $rejected option(s) in rustfmt.toml as unstable."
        echo "  Two generators rustfmt their output, so every file would report DRIFTED"
        echo "  for the formatter rather than for a real change. Use the toolchain pinned"
        echo "  in rust-toolchain.toml ($(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$repo/rustfmt.toml" 2>/dev/null)$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$repo/rust-toolchain.toml"))."
        echo "  In CI: install that toolchain WITH rustfmt, from inside the repo directory."
        exit 2
    fi
fi

for entry in "${generated[@]}"; do
    cp "$repo/${entry%%|*}" "$scratch/$(basename "${entry%%|*}")"
done

echo "generated-drift audit — ${#generated[@]} generated files, Hercules at $hercules"
findings=0
for entry in "${generated[@]}"; do
    path="${entry%%|*}"
    command="${entry#*|}"
    snapshot="$scratch/$(basename "$path")"

    if ! output="$(eval "$command" 2>&1)"; then
        # A generator that refuses to run is itself a finding — the message-table
        # one exits non-zero WITHOUT WRITING when its upstream sentinel reverts,
        # which is the loudest signal in this whole area.
        echo "  GENERATOR FAILED  $path"
        printf '%s\n' "$output" | sed 's/^/      /'
        findings=$((findings + 1))
        continue
    fi

    if cmp -s "$snapshot" "$repo/$path"; then
        echo "  ok        $path"
    else
        echo "  DRIFTED   $path"
        diff -u "$snapshot" "$repo/$path" | sed -n '4,24p' | sed 's/^/      /'
        findings=$((findings + 1))
    fi
done

if [ "$findings" -ne 0 ]; then
    echo
    echo "$findings generated file(s) do not match their generator. Regenerate and commit,"
    echo "or find out what changed in Hercules — a stale generated file fails silently."
    exit 1
fi
echo "  no drift"
