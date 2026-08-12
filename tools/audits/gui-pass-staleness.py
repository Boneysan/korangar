#!/usr/bin/env python3
"""Has the code under a closed GUI-pass block moved since somebody looked at it?

WHY THIS EXISTS. The GUI live pass (docs/plans/gui-verification-pass.md) is the
only instrument this project has for boundary 5, event -> pixel. It is also the
only one whose results do not decay: a headless scenario re-runs every suite, an
audit re-runs every commit, and a GUI row is a human writing **PASS** in a
markdown table once and the file saying "do not re-walk closed Blocks A-D".

That advice had already expired when this was written. Block A closed on
2026-08-04 having found twelve bugs across the social windows; by 2026-08-12
**eleven** commits had touched those same files, and three of them changed the
exact behaviours its rows asserted -- the party roster on leaving, the trade
window telling the server it was dismissed, and where the whisper channel
points. No mechanism said so. Nothing was wrong with the pass; the problem is
that a one-shot observation has no way to notice that its subject moved.

This does not verify anything. It cannot: only eyes on a screen can close a GUI
row. What it does is tell you **which closed rows you are no longer entitled to
trust**, which is the difference between "we verified that" and "we verified
that, once, against code that has since changed eleven times".

    tools/audits/gui-pass-staleness.py            # report; exit 1 if any block is stale
    tools/audits/gui-pass-staleness.py --list     # every block with its commits
    tools/audits/gui-pass-staleness.py --report   # never fail, just print

DELIBERATELY NOT IN CI. Every commit to a UI file would redden it until somebody
wrote a line, and this repo's own rule is that a warning which is always present
is a warning nobody reads (see the KNOWN_INTERMITTENT note in scenarios/
skills.rs). Run it when planning a GUI session, so the queue starts from what is
actually unverified rather than from what was unverified a week ago.

HOW TO CLEAR A STALE BLOCK. Two honest answers, and "ignore it" is not one:

  1. Re-walk the affected rows and update `verified` below to today.
  2. Read the commits and decide they cannot affect those rows -- then move
     `reviewed_through` to the newest of them and say why. That is the same
     contract as observer-parity.baseline: classify, do not silence.

The path lists are deliberately narrow and deliberately incomplete. Mapping a
block to `korangar/src/lib.rs` would make every block permanently stale, since
that file is touched constantly and is 8,000 lines of everything; a block would
then be telling you nothing. Narrow paths mean this can miss a change that
reaches a window through the event plumbing. It is a tripwire on the obvious
half, not proof of the rest.
"""

import argparse
import pathlib
import subprocess
import sys

# block | what it verified | date it was closed | reviewed_through (commit) | paths it depends on
#
# `reviewed_through` is empty until somebody reads the commits and decides they
# do not affect the block, at which point it holds the newest commit they read
# and the reason goes in the comment above the row.
BLOCKS = [
    {
        "name": "cheap-queue 1-4",
        "covers": "map-zone refusal message, item names, ground footprint, support walk-into-range",
        "verified": "2026-07-31",
        "reviewed_through": "",
        "paths": [
            "korangar/src/world/skill_layout.rs",
            "korangar/src/world/library/item_info.rs",
        ],
    },
    {
        "name": "observer rows",
        "covers": "two-seat appearance parity: weapon, shield, hair, dye, ammunition",
        "verified": "2026-08-02",
        "reviewed_through": "",
        "paths": [
            "korangar/src/world/entity/mod.rs",
        ],
    },
    {
        "name": "Block A (N1-N19)",
        "covers": "social windows: chat, whisper, friends, party, trade, class labels",
        "verified": "2026-08-04",
        "reviewed_through": "",
        "paths": [
            "korangar/src/interface/windows/chat.rs",
            "korangar/src/interface/windows/friend_list.rs",
            "korangar/src/interface/windows/friend_request.rs",
            "korangar/src/interface/windows/party.rs",
            "korangar/src/interface/windows/party_invite.rs",
            "korangar/src/interface/windows/trade.rs",
            "korangar/src/interface/windows/character_overview.rs",
            "korangar/src/state/friends.rs",
            "korangar/src/state/party.rs",
            "korangar/src/state/trade.rs",
            "korangar/src/world/library/job_name.rs",
        ],
    },
    {
        "name": "Block B (N20-N23)",
        "covers": "Sage / Wizard seat: Auto Spell picker, cast visuals",
        "verified": "2026-08-04",
        "reviewed_through": "",
        "paths": [
            "korangar/src/interface/windows/auto_spell.rs",
            "korangar/src/world/skill_recipe.rs",
        ],
    },
    {
        "name": "Block C (N15/N19/N25)",
        "covers": "instance window, NPC weapon refine, item names in dialogs",
        "verified": "2026-08-05",
        "reviewed_through": "",
        "paths": [
            "korangar/src/interface/windows/instance.rs",
            "korangar/src/interface/windows/repair_weapon.rs",
            "korangar/src/interface/windows/dialog.rs",
            "korangar/src/state/instance.rs",
        ],
    },
    {
        "name": "Block D (N26)",
        "covers": "Priest support skills — eleven of them had no visual at all",
        "verified": "2026-08-06",
        "reviewed_through": "",
        "paths": [
            "korangar/src/world/skill_recipe.rs",
            "korangar/src/world/sprite_effect.rs",
            "korangar/src/world/special_effect.rs",
        ],
    },
    {
        "name": "Block E (Moonlit)",
        "covers": "ensemble ground unit: tile texture, alpha, sound — Hermode still OPEN",
        "verified": "2026-08-08",
        "reviewed_through": "",
        "paths": [
            "korangar/src/world/unit_recipe.rs",
            "korangar/src/world/skill_unit_registry.rs",
        ],
    },
]


def git(repo: pathlib.Path, *arguments: str) -> str:
    result = subprocess.run(["git", "-C", str(repo), *arguments], capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise SystemExit(f"error: git {' '.join(arguments)} failed: {result.stderr.strip()}")
    return result.stdout


def changes_behaviour(repo: pathlib.Path, commit: str, paths: list[str]) -> bool:
    """Did this commit change anything in `paths` beyond whitespace?

    A repo-wide `cargo fmt --all` touches sixty files and changes no behaviour
    in any of them, and one such commit would mark every block below stale at
    once. A tool that cries wolf on a formatting pass is a tool that gets
    ignored -- this repo says so itself, about always-present warnings.

    Checked with `-w` rather than assumed: the 2026-08-11 formatting pass
    (`d5eb977a`) was the obvious candidate for an ignore-list entry, and it
    turns out to carry real edits to `state/party.rs`, `unit_recipe.rs`,
    `item_info.rs` and three others. It is correctly reported. Naming it as
    formatting-only would have been a false rationale in a baseline, which is
    the mistake this directory has on record twice.
    """
    numstat = git(repo, "show", "-w", "--numstat", "--format=", commit, "--", *paths)
    return any(line.strip() for line in numstat.splitlines())


def commits_since(repo: pathlib.Path, since: str, after_commit: str, paths: list[str]) -> list[str]:
    """Commits touching `paths` after the block was verified.

    `after_commit` wins when it is set: a reviewed-through revision is a
    stronger statement than a date, and dates are only what the pass document
    happens to record.
    """
    range_argument = [f"{after_commit}..HEAD"] if after_commit else [f"--since={since}"]
    # `--no-merges`: `git show` on a merge prints no diff, so a merge would be
    # reported with nothing to read and could never be assessed.
    lines = git(repo, "log", "--oneline", "--no-decorate", "--no-merges", *range_argument, "--", *paths)
    return [line for line in lines.splitlines() if line.strip() and changes_behaviour(repo, line.split()[0], paths)]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--list", action="store_true", help="print every block, including the fresh ones")
    parser.add_argument("--report", action="store_true", help="never exit non-zero")
    arguments = parser.parse_args()

    repo = pathlib.Path(__file__).resolve().parents[2]

    # A path list that has gone stale would silently shrink this to nothing,
    # which is the failure mode the tool is about.
    missing = [path for block in BLOCKS for path in block["paths"] if not (repo / path).exists()]
    if missing:
        raise SystemExit(f"error: these BLOCKS paths no longer exist, so their block checks nothing: {missing}")

    print(f"GUI-pass staleness — {len(BLOCKS)} closed blocks, against HEAD")

    stale = 0
    for block in BLOCKS:
        commits = commits_since(repo, block["verified"], block["reviewed_through"], block["paths"])
        anchor = block["reviewed_through"] or f"verified {block['verified']}"
        if not commits:
            if arguments.list:
                print(f"  fresh    {block['name']:22} ({anchor}) — {block['covers']}")
            continue

        stale += 1
        print(f"\n  STALE    {block['name']:22} ({anchor})")
        print(f"           {block['covers']}")
        print(f"           {len(commits)} commit(s) have touched its files since:")
        for line in commits:
            print(f"             {line}")

    if stale == 0:
        print("  every closed block is still standing on the code it was verified against")
        return 0

    print(
        f"\n{stale} of {len(BLOCKS)} closed blocks are older than the code they cover.\n"
        "A PASS mark is a statement about one revision. Either re-walk the affected\n"
        "rows and move `verified` forward, or read the commits, decide they cannot\n"
        "reach those rows, and set `reviewed_through` with the reason — the same\n"
        "contract as observer-parity.baseline. Do not simply carry the PASS."
    )
    return 0 if arguments.report else 1


if __name__ == "__main__":
    sys.exit(main())
