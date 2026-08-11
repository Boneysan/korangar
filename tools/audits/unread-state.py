#!/usr/bin/env python3
"""Find state the client stores and never reads.

WHY THIS EXISTS. The recurring failure in this fork is *"the data arrives and
nothing displays it"*, and it has four stages: wire -> handler -> surface
lifetime -> draw order. Five instances are on record. Stage three is a field
that a handler dutifully writes and **nothing ever reads**, and it is invisible
to everything else we have:

  * the headless suite asserts the WIRE, so it is green — the packet is correct;
  * `event-routing.py` asserts the arm is not empty, so it is green — the arm
    stores a value;
  * the compiler is silent, because a `pub` field with a setter is "used".

`spirit_spheres` sat in exactly that hole: ZC_SPIRITS is modelled, registered,
evented, routed and stored, and Monk spheres have never rendered. It was found by
hand, by an audit aimed at something else entirely. This is that grep, made
repeatable.

WHAT IT IS NOT. It catches stage three only. A field read exclusively by dead
code still passes, and so does the popup drawn behind character select (stage
four). Closing those needs the routing extraction in
`docs/plans/testing-completeness.md` §2c — but run this FIRST, because it tells
you how common the class actually is before anyone commits to a 2,755-line
refactor.

READING A FIELD IS NOT JUST `.field`. This codebase passes field *identifiers*
into macros (`stat_row!(.., bonus_strength, ..)`) and into generated state paths.
Counting only `.field` reported 87 fields, and the entire `Player.bonus_*`
cluster was a FALSE POSITIVE — those are drawn by the stats window. A bare
identifier occurrence that is not the declaration counts as a read; that took it
to 42, all classifiable.

Usage:
  tools/audits/unread-state.py            # exits non-zero on an unclassified hit
  tools/audits/unread-state.py --list     # every hit, ignoring the baseline
"""

import argparse
import pathlib
import re
import sys

BASELINE = pathlib.Path(__file__).with_name("unread-state.baseline")
ROOT = pathlib.Path(__file__).resolve().parents[2] / "korangar" / "src"

STRUCT = re.compile(r"\npub struct (\w+) \{(.*?)\n\}", re.S)
FIELD = re.compile(r"\n\s*(?:pub |pub\(crate\) )?(\w+):\s")


def sources() -> list[tuple[pathlib.Path, str]]:
    return [(path, path.read_text(errors="replace")) for path in sorted(ROOT.rglob("*.rs"))]


def line_at(text: str, index: int) -> str:
    return text[text.rfind("\n", 0, index) + 1 : text.find("\n", index)]


def unread(text: str, field: str) -> bool:
    """True when nothing anywhere reads this field."""
    escaped = re.escape(field)
    assignment = re.compile(r"\.%s\s*(=[^=]|\+=|-=|\*=|/=)" % escaped)

    for hit in re.finditer(r"\.%s\b" % escaped, text):
        if not assignment.search(line_at(text, hit.start())):
            return False

    # Bare identifier: macro arguments and generated state paths. Skipping lines
    # that look like `field:` drops the declaration and struct-literal init,
    # which are not reads.
    declaration = re.compile(r"%s\s*:" % escaped)
    for hit in re.finditer(r"(?<![\w.])%s(?![\w:])" % escaped, text):
        if not declaration.search(line_at(text, hit.start())):
            return False

    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--list", action="store_true", help="print every hit and ignore the baseline")
    arguments = parser.parse_args()

    files = sources()
    if not files:
        raise SystemExit(f"error: no sources under {ROOT}")
    whole = "\n".join(text for _, text in files)

    findings = []
    for path, text in files:
        for match in STRUCT.finditer(text):
            name = match.group(1)
            for field in FIELD.findall(match.group(2)):
                if unread(whole, field):
                    findings.append(f"{name}.{field}")

    findings = sorted(set(findings))
    print(f"unread-state audit — {len(files)} files, {len(findings)} field(s) stored and never read")

    if arguments.list:
        for hit in findings:
            print(f"  {hit}")
        return 0

    if not BASELINE.is_file():
        raise SystemExit(f"error: {BASELINE} not found")
    baseline_text = BASELINE.read_text()
    classified = {line.strip() for line in baseline_text.splitlines() if line.strip() and not line.startswith("#")}

    new = [hit for hit in findings if hit not in classified]
    gone = sorted(classified - set(findings))

    # Reprinted every run: classified, and still wrong.
    for line in baseline_text.splitlines():
        if line.startswith("# OPEN:"):
            print("  !" + line[len("# OPEN:") :])

    if gone:
        print(f"\nSTALE baseline entries ({len(gone)}) — these are read now. Remove them:")
        for hit in gone:
            print(f"  - {hit}")
    if new:
        print(f"\nUNCLASSIFIED ({len(new)}) — each is state the client stores and never reads.")
        print("  Fix it, or add it to the baseline with a rationale above the line.")
        for hit in new:
            print(f"  + {hit}")

    if new or gone:
        return 1
    print("  no unclassified hits")
    return 0


if __name__ == "__main__":
    sys.exit(main())
