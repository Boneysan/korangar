#!/usr/bin/env python3
"""Find server events the client receives and then does nothing with.

THE GAP THIS FILLS. The headless suite links `ragnarok-packets` and
`korangar-networking` and **not** `korangar/src`, so it proves the wire data is
correct and says nothing about whether the client *uses* it. The client crate
has 300+ unit tests, but they exercise state and world logic through their own
APIs — the routing from `NetworkEvent` into that state is not covered by either
side. That routing is exactly where this project's most-repeated bug lives:

  "the data arrives and nothing displays it"

recorded four times, with four distinct stages (wire -> handler -> surface
lifetime -> draw order). This audit catches the **handler** stage, which is
where three of those four instances stopped.

It is deliberately dumb: an arm that is literally `=> {}` is an event the server
took the trouble to send and the client throws away. Some of those are correct —
hence the allowlist below, which follows the same rule as every other audit
here: **an audit passes when every hit is classified, not when it returns
nothing.**

What it cannot see: an arm that stores a value nobody reads, or a window that is
opened and then drawn underneath another. Those are stages three and four, and
they need the GUI pass.

Usage:
  tools/audits/event-routing.py            # exits non-zero on an unclassified drop
  tools/audits/event-routing.py --list     # show every empty arm, classified or not
"""

import argparse
import pathlib
import re
import sys

# Events the client is right to ignore, each with the reason. Anything not
# listed here that does nothing is a finding.
DELIBERATELY_IGNORED = {
    "EntityEffectState": "0x028A — modelled but its meaning is not established; needs a look before it earns a window",
    # Classified rather than left red, because an audit that always fails is one
    # nobody reads. The classification is not "this is fine" — it is the size of
    # the gap, recorded so a NEW drop still stands out:
    #
    #   The campaign depends on quests. 78 quest ids are registered in
    #   db/quest_db.conf, dm_quests.txt grants them for real via setquest(), and
    #   the scripts gate progression on questprogress() 290 times. All three
    #   events reach the client and stop here, so a DM can hand out a quest, the
    #   server records it, the gates work — and the player never sees any of it.
    #
    # Tracked in docs/plans/work-backlog.md §2. Needs a window, not a fix here.
    "QuestAdded": "quest system unbuilt client-side — see work-backlog §2",
    "QuestList": "quest system unbuilt client-side — see work-backlog §2",
    "QuestRemoved": "quest system unbuilt client-side — see work-backlog §2",
}


def empty_arms(source: str) -> set[str]:
    """`NetworkEvent::X { .. } => {}` — received, then dropped."""
    found = set(re.findall(r"NetworkEvent::(\w+)\s*\{[^}]*\}\s*=>\s*\{\s*\}", source))
    found |= set(re.findall(r"NetworkEvent::(\w+)\s*=>\s*\{\s*\}", source))
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    here = pathlib.Path(__file__).resolve().parent
    parser.add_argument("--korangar", type=pathlib.Path, default=here.parents[1])
    parser.add_argument("--list", action="store_true", help="show classified drops too")
    args = parser.parse_args()

    client = args.korangar / "korangar/src/lib.rs"
    events = args.korangar / "korangar-networking/src/event.rs"
    if not client.is_file() or not events.is_file():
        raise SystemExit("error: run this from the korangar checkout (or pass --korangar)")

    source = client.read_text(errors="replace")
    dropped = empty_arms(source)

    body = events.read_text(errors="replace")
    body = body[body.index("pub enum NetworkEvent") :]
    variants = set(re.findall(r"^\s{4}([A-Z]\w+)\s*[\{,]", body, re.M))
    referenced = set(re.findall(r"NetworkEvent::(\w+)", source))

    unclassified = sorted(dropped - DELIBERATELY_IGNORED.keys())
    classified = sorted(dropped & DELIBERATELY_IGNORED.keys())

    print(f"event-routing audit — {len(variants)} NetworkEvent variants, {len(variants & referenced)} referenced by the client")
    if args.list and classified:
        print(f"\n  classified drops ({len(classified)}):")
        for name in classified:
            print(f"    {name}: {DELIBERATELY_IGNORED[name]}")

    if not unclassified:
        print("  no unclassified drops")
        return 0

    print(f"\n  {len(unclassified)} EVENT(S) RECEIVED AND DISCARDED, unclassified:")
    for name in unclassified:
        print(f"    NetworkEvent::{name} => {{}}")
    print(
        "\n  Each is data the server sent that reaches the client and stops there.\n"
        "  Either use it, or add it to DELIBERATELY_IGNORED with the reason."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
