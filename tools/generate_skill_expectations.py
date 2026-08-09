#!/usr/bin/env python3
"""Derive, from Hercules' own `skill_db.conf`, what each skill should be seen to do.

WHY THIS EXISTS. The 39-job skill sweep passes a skill when *any* observable
response arrives. That is a **liveness** check, and a good one — it is what
caught unregistered and misparsed packets. It is not a correctness check, and
the number it produces ("skills: 47 scenarios, green") reads like one. Measured
2026-08-09 across 983 casts: 36% of observations were the server *refusing* the
skill, 26% were passive skills never cast at all, and only 25 of the 403 skills
the sweep touches were checked against an outcome specific to that skill.

Worse, the matcher stops at the FIRST event it recognises and `SkillCast` is
checked first — so `cast` does not mean "the skill worked", it means "a cast bar
started and we stopped looking". That was 12% of observations.

THE FIX, and the precedent for it: traps were once held to the same loose
standard, and tightening them to "prove the unit was placed" turned zero
assertions into 38. `skill_db.conf` already states what most skills do, so the
expectation can be *derived* rather than hand-maintained — the same principle as
tools/generate_skill_states.py and tools/generate_message_table.py, and for the
same reason: a hand-written table drifts and nobody notices.

  Unit: block        -> an AddSkillUnit must arrive
  StatusChange: SC_X -> that status must be gained
  damaging + Attack  -> damage must arrive

A skill with none of these keeps the current loose standard, which is honest:
the database does not say what it should do, so neither can we.

Usage:
  tools/generate_skill_expectations.py [--hercules DIR] [--check]

`--check` prints the coverage delta and writes nothing — use it to see how much
of the sweep this would actually tighten before committing to it.
"""

import argparse
import pathlib
import re
import sys

# Hercules groups skills into one big libconfig list; entries are `Id:`/`Name:`
# pairs followed by optional blocks. Parsing is deliberately shallow — we only
# need the presence of a few keys, not a real libconfig reader.
ENTRY = re.compile(r"\n\tId:\s*(\d+)\s*\n\tName:\s*\"([A-Z][A-Z0-9_]+)\"")
NON_PLAYER = ("NPC_", "MER_", "HVAN_", "MOB_", "EL_", "GD_")


def entries(text: str):
    """(id, name, body) for every skill, body running to the next entry."""
    marks = list(ENTRY.finditer(text))
    for index, match in enumerate(marks):
        end = marks[index + 1].start() if index + 1 < len(marks) else len(text)
        yield int(match.group(1)), match.group(2), text[match.start() : end]


def block(body: str, key: str) -> str | None:
    """The `{...}` following `key:`, brace-matched."""
    match = re.search(rf"\n\t*{key}:\s*\{{", body)
    if not match:
        return None
    start = body.index("{", match.start())
    depth, index = 0, start
    while index < len(body):
        if body[index] == "{":
            depth += 1
        elif body[index] == "}":
            depth -= 1
            if depth == 0:
                return body[start : index + 1]
        index += 1
    return None


def expectation(body: str) -> tuple[str, str | None]:
    """What this skill's own database entry says it must be seen to do."""
    # A `Unit:` block means a skill unit is placed — the strongest signal there
    # is, and already proven to work as an assertion (38 trap assertions).
    if block(body, "Unit") is not None:
        return "Unit", None

    types = block(body, "SkillType") or ""
    offensive = "Enemy: true" in types

    # **Who gains the status matters, and getting this wrong is worse than not
    # deriving it at all.** `StatusChange:` on an enemy-targeted skill lands on
    # the TARGET — Frost Diver freezes the monster — while the sweep watches the
    # caster. Only self/friend-targeted skills promise the caster a status.
    status = re.search(r'\n\tStatusChange:\s*"(SC_[A-Z0-9_]+)"', body)
    if status and not offensive:
        return "Status", status.group(1)

    # Damage needs an enemy target AND a damaging hit type. `Hit:` alone is
    # carried by plenty of no-damage skills, and the attack flag in skill_db is
    # spelled `Enemy`, not `Attack`.
    hit = re.search(r'\n\tHit:\s*"(\w+)"', body)
    if offensive and hit and hit.group(1) in ("BDT_NORMAL", "BDT_SKILL", "BDT_MULTIHIT"):
        return "Damage", None

    return "Any", None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    here = pathlib.Path(__file__).resolve().parent
    parser.add_argument("--hercules", type=pathlib.Path, default=here.parents[1] / "Hercules")
    parser.add_argument("--check", action="store_true", help="report coverage and write nothing")
    parser.add_argument(
        "--out",
        type=pathlib.Path,
        default=here.parent / "korangar-networking/examples/headless-tester/scenarios/skill_expectations.rs",
    )
    args = parser.parse_args()

    path = args.hercules / "db/re/skill_db.conf"
    if not path.is_file():
        raise SystemExit(f"error: {path} not found; is --hercules correct?")

    rows, tally = [], {}
    for skill_id, name, body in entries(path.read_text(errors="replace")):
        if name.startswith(NON_PLAYER):
            continue
        kind, detail = expectation(body)
        tally[kind] = tally.get(kind, 0) + 1
        if kind != "Any":
            rows.append((skill_id, name, kind, detail))

    total = sum(tally.values())
    print(f"player skills in skill_db: {total}")
    for kind, count in sorted(tally.items(), key=lambda item: -item[1]):
        print(f"  {count:5}  {100 * count / total:5.1f}%  {kind}")
    print(f"\nderivable expectations (everything but 'Any'): {len(rows)}")

    if args.check:
        return 0

    lines = [
        "// @generated by tools/generate_skill_expectations.py — do not edit by hand.",
        "//",
        "// What each skill's own `skill_db.conf` entry says it must be seen to do, so",
        "// the sweep can assert the specific outcome instead of accepting any response.",
        "// Skills whose entry says nothing useful are absent and keep the loose standard.",
        "//",
        "// Regenerate after any skill_db change: tools/generate_skill_expectations.py",
        "",
        "/// The observable a skill's database entry promises.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum Expected {",
        "    /// `Unit:` — an `AddSkillUnit` must arrive.",
        "    Unit,",
        "    /// `StatusChange:` — the caster must gain this status.",
        "    Status(&'static str),",
        "    /// An attacking skill with a damaging `Hit:`.",
        "    Damage,",
        "}",
        "",
        "/// (skill id, skill name, expectation). Sorted by id.",
        "pub static SKILL_EXPECTATIONS: &[(u16, &str, Expected)] = &[",
    ]
    for skill_id, name, kind, detail in sorted(rows):
        value = f'Expected::Status("{detail}")' if kind == "Status" else f"Expected::{kind}"
        lines.append(f'    ({skill_id}, "{name}", {value}),')
    lines.append("];")

    args.out.write_text("\n".join(lines) + "\n")
    print(f"wrote {args.out} ({len(rows)} entries)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
