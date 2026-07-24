#!/usr/bin/env python3
"""Export Hercules skill data to `docs/skills.json` for client hover tooltips.

Why: hovering a skill showed only its name, so players could not see what it
costs, what it targets, how long its ground field lasts, or which reagent it
consumes. That last one matters — a Sage field that silently fails for want of
a Blue Gemstone looks like a client bug, and cost us half a session once.

The official `skilldescript.lub` in data.grf would be nicer prose, but ours is
from a `kr_ro1_live` build: the body text is Korean, only the skill names in
parentheses are English. Generating from the server's own skill_db instead is
factual rather than flavourful, but always matches the server the client is
actually connected to.

Source is `Hercules/db/re/skill_db.conf` (libconfig, not JSON — see the parser
below). Renewal only: the server is built with RENEWAL (`src/config/renewal.h`).

Usage:
    tools/export_skill_info.py [--check]

    --check  exit 1 if docs/skills.json is stale (for CI)
"""

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SKILL_DB = REPO.parent / "Hercules" / "db" / "re" / "skill_db.conf"
OUTPUT = REPO / "docs" / "skills.json"

# Fields worth putting in a tooltip. Everything else in skill_db (splash,
# knockback, plagiarism flags, weapon restrictions, …) is deliberately dropped:
# a tooltip that fills the screen is worse than one that answers the question.
SCALAR_FIELDS = ["Id", "Name", "Description", "MaxLevel", "AttackType"]
LEVELLED_FIELDS = ["Range", "CastTime", "FixedCastTime", "NumberOfHits", "SkillData1"]
# Element is usually a plain string but six skills vary it by level
# (TK_SEVENWIND and friends), so it gets the same levelled treatment.
LEVELLED_TEXT_FIELDS = ["Element"]


class ParseError(Exception):
    pass


def _strip_comments(text):
    """Remove `//` and `/* */` comments, leaving quoted strings untouched.

    The file has both styles — a `/* */` banner at the top and `//` notes
    throughout — and `//` also appears inside the ASCII-art header, so this has
    to be a real state machine rather than a per-line split.
    Newlines are preserved so error offsets still point at a sensible line.
    """
    out = []
    index = 0
    length = len(text)
    in_string = False

    while index < length:
        char = text[index]

        if in_string:
            out.append(char)
            if char == "\\" and index + 1 < length:
                out.append(text[index + 1])
                index += 2
                continue
            if char == '"':
                in_string = False
            index += 1
            continue

        if char == '"':
            in_string = True
            out.append(char)
            index += 1
            continue

        if char == "/" and index + 1 < length:
            if text[index + 1] == "/":
                while index < length and text[index] != "\n":
                    index += 1
                continue
            if text[index + 1] == "*":
                end = text.find("*/", index + 2)
                block = text[index : end + 2 if end != -1 else length]
                out.append("\n" * block.count("\n"))
                index = length if end == -1 else end + 2
                continue

        out.append(char)
        index += 1

    return "".join(out)


# libconfig scalars. Numbers may carry `_` digit separators (`1_100`) and may
# be hex (`0x81`); both appear in this file.
_TOKEN = re.compile(
    r"""
      (?P<string>"(?:[^"\\]|\\.)*")
    | (?P<hex>0[xX][0-9a-fA-F_]+)
    # Hercules uses identifiers that start with a digit — the weapon-type keys
    # `1HSwords`, `2HAxes` and friends. Requiring a *letter* after the digits
    # keeps `1_100` (a number with separators) out of this branch.
    | (?P<digitname>\d+[A-Za-z][A-Za-z0-9_]*)
    | (?P<number>-?\d[\d_]*)
    | (?P<bool>\btrue\b|\bfalse\b)
    | (?P<name>[A-Za-z_][A-Za-z0-9_]*)
    | (?P<punct>[{}()\[\],:])
    """,
    re.VERBOSE,
)


def _tokenize(text):
    position = 0
    length = len(text)
    while position < length:
        if text[position].isspace():
            position += 1
            continue
        match = _TOKEN.match(text, position)
        if match is None:
            raise ParseError(f"unexpected character {text[position]!r} at offset {position}")
        position = match.end()
        yield match.lastgroup, match.group()


def _parse_value(tokens, index):
    kind, text = tokens[index]

    if text == "{":
        return _parse_block(tokens, index)
    if text == "[" or text == "(":
        closing = "]" if text == "[" else ")"
        items = []
        index += 1
        while tokens[index][1] != closing:
            if tokens[index][1] == ",":
                index += 1
                continue
            value, index = _parse_value(tokens, index)
            items.append(value)
        return items, index + 1

    if kind == "string":
        return text[1:-1], index + 1
    if kind == "hex":
        return int(text.replace("_", ""), 16), index + 1
    if kind == "number":
        return int(text.replace("_", "")), index + 1
    if kind == "bool":
        return text == "true", index + 1
    # A bare identifier used as a value (rare, e.g. an enum-ish token).
    return text, index + 1


def _parse_block(tokens, index):
    """Parse `{ Key: value ... }` starting at the opening brace."""
    assert tokens[index][1] == "{"
    index += 1
    block = {}
    while tokens[index][1] != "}":
        if tokens[index][1] == ",":
            index += 1
            continue
        key = tokens[index][1]
        if tokens[index + 1][1] != ":":
            context = " ".join(text for _, text in tokens[max(0, index - 8) : index + 4])
            raise ParseError(f"expected ':' after {key!r} at token {index}; near: …{context}…")
        value, index = _parse_value(tokens, index + 2)
        block[key] = value
    return block, index + 1


def parse_skill_db(text):
    tokens = list(_tokenize(_strip_comments(text)))
    # File shape: `skill_db: ( { ... }, { ... } )`
    start = next(i for i, (_, t) in enumerate(tokens) if t == "(")
    entries, _ = _parse_value(tokens, start)
    return [entry for entry in entries if isinstance(entry, dict)]


def levelled(value, kind=int, missing=0):
    """Normalise a scalar-or-per-level field into one shape.

    skill_db writes `Range: 9` but `CastTime: { Lv1: 500, ... }` — the same
    field can be either. A handful of skills do it with *strings* too
    (`TK_SEVENWIND` changes element per level), so this takes the value type.
    Emitting one shape means the client never has to care.
    Returns `{"flat": v}` or `{"levels": [v1, v2, ...]}`.
    """
    if value is None:
        return None
    if isinstance(value, dict):
        levels = {}
        for key, entry in value.items():
            match = re.fullmatch(r"Lv(\d+)", key)
            if match and isinstance(entry, kind):
                levels[int(match.group(1))] = entry
        if not levels:
            return None
        return {"levels": [levels.get(level, missing) for level in range(1, max(levels) + 1)]}
    if isinstance(value, kind):
        return {"flat": value}
    return None


def target_kind(skill):
    """Human phrase for who/what the skill is aimed at."""
    flags = skill.get("SkillType") or {}
    for flag, label in (
        ("Place", "Ground target"),
        ("Trap", "Trap"),
        ("Self", "Self"),
        ("Friend", "Ally"),
        ("Enemy", "Single target"),
        ("Passive", "Passive"),
    ):
        if flags.get(flag):
            return label
    return None


def requirement_items(skill):
    """Reagents the skill consumes, as readable names.

    `Blue_Gemstone` -> `Blue Gemstone`. Underscore-to-space is correct for
    these and avoids depending on the item DB just to render a tooltip line.
    """
    items = (skill.get("Requirements") or {}).get("Items") or {}
    return [name.replace("_", " ") for name in items]


def convert(skill):
    row = {}
    for field in SCALAR_FIELDS:
        if field in skill:
            row[field] = skill[field]
    for field in LEVELLED_FIELDS:
        value = levelled(skill.get(field))
        if value is not None:
            row[field] = value
    for field in LEVELLED_TEXT_FIELDS:
        value = levelled(skill.get(field), kind=str, missing="")
        if value is not None:
            row[field] = value

    requirements = skill.get("Requirements") or {}
    sp_cost = levelled(requirements.get("SPCost"))
    if sp_cost is not None:
        row["SPCost"] = sp_cost

    items = requirement_items(skill)
    if items:
        row["Items"] = items

    kind = target_kind(skill)
    if kind:
        row["Target"] = kind

    # Ground-unit footprint. Hercules square layouts are (2N+1)^2 cells
    # (skill.c, skill_init_unit_layout). Layout -1 is a custom shape, so the
    # client must not print a square size for it.
    unit = skill.get("Unit") or {}
    layout = levelled(unit.get("Layout"))
    if layout is not None:
        row["Layout"] = layout

    return row


def build():
    text = SKILL_DB.read_text(encoding="utf-8", errors="replace")
    skills = parse_skill_db(text)
    rows = [convert(skill) for skill in skills if "Id" in skill]
    rows.sort(key=lambda row: row["Id"])
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="exit 1 if the export is stale")
    args = parser.parse_args()

    if not SKILL_DB.exists():
        raise SystemExit(f"skill_db not found: {SKILL_DB}")

    rows = build()
    rendered = json.dumps(rows, indent=1, ensure_ascii=False) + "\n"

    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != rendered:
            raise SystemExit(f"{OUTPUT} is stale — re-run {Path(__file__).name}")
        print(f"up to date: {len(rows)} skills")
        return

    OUTPUT.write_text(rendered, encoding="utf-8")
    with_items = sum(1 for row in rows if "Items" in row)
    print(f"wrote {OUTPUT} ({len(rows)} skills, {with_items} with item requirements)")


if __name__ == "__main__":
    main()
