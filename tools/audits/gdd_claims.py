#!/usr/bin/env python3
"""Check the code names cited by GDD task rows against both repositories.

Batch 0 of docs/plans/gdd-verification-audit.md. Each F row in
docs/plans/gdd-improvement-plan.md and each audit row in
docs/plans/gdd-next-slices.md cites code in backticks. This reports, per row:

- missing:   the name occurs nowhere in the searched source;
- test-only: every Rust occurrence sits inside a `#[cfg(test)]` module;
- files:     cited paths that do not exist.

It is a triage tool. A name that exists is not proof the feature works; a
missing or test-only name is a reason to look first.

    tools/audits/gdd_claims.py              # table of rows with findings
    tools/audits/gdd_claims.py --all        # every row, including clean ones
    tools/audits/gdd_claims.py --row F23    # one row in detail
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

KORANGAR = Path(__file__).resolve().parents[2]
HERCULES = KORANGAR.parent / "Hercules"
PLANS = (
    KORANGAR / "docs/plans/gdd-improvement-plan.md",
    KORANGAR / "docs/plans/gdd-next-slices.md",
)
SOURCE_ROOTS = (
    (KORANGAR / "korangar/src", (".rs",)),
    (KORANGAR / "korangar-networking", (".rs",)),
    (KORANGAR / "korangar-interface/src", (".rs",)),
    (KORANGAR / "ragnarok-packets", (".rs",)),
    (KORANGAR / "tools", (".py", ".sh")),
    (HERCULES / "src", (".c", ".h")),
    (HERCULES / "npc", (".txt",)),
    (HERCULES / "db", (".conf",)),
    (HERCULES / "conf", (".conf",)),
    (HERCULES / "tools", (".py", ".sh")),
)
ROW = re.compile(r"^\| (?P<id>F\d{2}|S\d+(?:–S\d+)?|\d{1,2}) [^|]*\|(?P<body>.*)$")
BACKTICK = re.compile(r"`([^`]+)`")
IDENTIFIER = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*$")
# Words in backticks that are values or commands, not code names.
NOT_CODE = re.compile(r"^(?:[A-Z][a-z]+|[a-z]+|[0-9a-f]{7,40})$")


@dataclass
class Row:
    id: str
    source: str
    names: list[str] = field(default_factory=list)
    paths: list[str] = field(default_factory=list)
    missing: list[str] = field(default_factory=list)
    test_only: list[str] = field(default_factory=list)
    missing_paths: list[str] = field(default_factory=list)


def test_spans(text: str) -> list[tuple[int, int]]:
    """Byte spans of `#[cfg(test)] mod name { ... }` blocks, brace-matched."""
    spans = []
    for attribute in re.finditer(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{", text):
        depth, index = 1, attribute.end()
        while depth and index < len(text):
            depth += {"{": 1, "}": -1}.get(text[index], 0)
            index += 1
        spans.append((attribute.start(), index))
    return spans


def load_sources() -> list[tuple[Path, str, list[tuple[int, int]]]]:
    """(path, text, test-module spans) for every source file."""
    files = []
    for root, suffixes in SOURCE_ROOTS:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if path.suffix not in suffixes or "target" in path.parts or "__pycache__" in path.parts:
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            files.append((path, text, test_spans(text) if path.suffix == ".rs" else []))
    return files


def rows() -> list[Row]:
    found = []
    for plan in PLANS:
        for line in plan.read_text(encoding="utf-8").splitlines():
            match = ROW.match(line)
            if not match:
                continue
            row = Row(match.group("id"), plan.name)
            for cited in BACKTICK.findall(match.group("body")):
                cited = cited.strip().rstrip("()").removeprefix("@").removeprefix("/")
                if "/" in cited and " " not in cited:
                    row.paths.append(cited)
                elif IDENTIFIER.match(cited) and not NOT_CODE.match(cited) and len(cited) >= 6:
                    row.names.append(cited.split("::")[-1])
            row.names = sorted(set(row.names))
            row.paths = sorted(set(row.paths))
            found.append(row)
    return found


def path_exists(cited: str) -> bool:
    cited = cited.split(":")[0]
    if "<" in cited or "[" in cited:
        return True  # a pattern or runtime file name, not a source path
    if cited.startswith("client/"):
        return True  # written at runtime next to the executable (e.g. client/keybindings.ron)
    for base in (KORANGAR, KORANGAR / "korangar", KORANGAR / "korangar/src", HERCULES, KORANGAR.parent):
        if (base / cited).exists():
            return True
    return False


def check(found: list[Row], sources: list[tuple[Path, str, list[tuple[int, int]]]]) -> None:
    cache: dict[str, tuple[int, int]] = {}
    for row in found:
        for name in row.names:
            if name not in cache:
                pattern = re.compile(r"\b" + re.escape(name) + r"\b")
                production = tests = 0
                for path, text, spans in sources:
                    for match in pattern.finditer(text):
                        if any(start <= match.start() < end for start, end in spans):
                            tests += 1
                        else:
                            production += 1
                cache[name] = (production, tests)
            production, tests = cache[name]
            if production == 0 and tests == 0:
                row.missing.append(name)
            elif production == 0:
                row.test_only.append(name)
        row.missing_paths = [path for path in row.paths if not path_exists(path)]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--all", action="store_true", help="list clean rows too")
    parser.add_argument("--row", help="show one row in detail, e.g. F23")
    args = parser.parse_args()

    found = rows()
    if not found:
        print("error: no task rows parsed", file=sys.stderr)
        return 2
    check(found, load_sources())

    if args.row:
        for row in found:
            if row.id == args.row:
                print(f"{row.id} ({row.source})")
                print(f"  cited names ({len(row.names)}): {', '.join(row.names) or '-'}")
                print(f"  cited paths ({len(row.paths)}): {', '.join(row.paths) or '-'}")
                print(f"  missing: {', '.join(row.missing) or '-'}")
                print(f"  test-only: {', '.join(row.test_only) or '-'}")
                print(f"  missing paths: {', '.join(row.missing_paths) or '-'}")
        return 0

    flagged = 0
    print(f"{'row':6} {'plan':26} {'names':>5} {'missing':40} {'test-only':30} missing paths")
    for row in found:
        bad = row.missing or row.test_only or row.missing_paths
        flagged += bool(bad)
        if bad or args.all:
            print(
                f"{row.id:6} {row.source:26} {len(row.names):>5} {', '.join(row.missing)[:40]:40} "
                f"{', '.join(row.test_only)[:30]:30} {', '.join(row.missing_paths)}"
            )
    print(f"\n{len(found)} rows, {flagged} with findings")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
