#!/usr/bin/env python3
"""Re-anchor encyclopedia reviews whose cited line numbers have drifted.

`encyclopedia_unit.py citations` reports reviews whose quoted source text is not
on the lines they cite. On 2026-10-02 all 29 were drift, not wrong quotes: each
quoted text exists in the cited file, a few lines (or, for `mob_skill_db.conf`,
about a hundred) away from where the review says. This moves the cited lines to
where the text is, and only when that is provable:

  * every cited line of a source moves by the SAME offset;
  * the smallest offset that makes the existing validator (`check_sources`)
    pass completely is used, and two offsets of the same size with opposite
    signs are refused as ambiguous;
  * nothing is changed unless that source then passes the validator.

It edits only the `"lines": [...]` array of each fixed source, as text, so the
rest of each review file keeps its formatting. Dry run unless `--apply`.

    python3 tools/encyclopedia_reanchor.py            # report
    python3 tools/encyclopedia_reanchor.py --apply    # write, then re-run the citation check
"""

from __future__ import annotations

import argparse
import re
import sys
from typing import Any

import encyclopedia_checks as checks
import encyclopedia_unit as unit

WINDOW = 800
LINES_ARRAY = re.compile(r'"lines":\s*\[[^\]]*\]')


def review_files() -> list[str]:
    return list(unit.REVIEW_FILES.values()) + ["tools/npc_service_reviews.json", "tools/map_runtime_flag_reviews.json"]


def entry_sources(entry: dict[str, Any]) -> list[dict[str, Any]]:
    if entry.get("sources"):
        return entry["sources"]
    if "source" in entry:
        return [dict(entry["source"], required_source_literals=entry.get("required_source_literals"))]
    return []


def literal_errors(label: str, source: dict[str, Any]) -> list[str]:
    return [error for error in checks.check_sources(label, [source]) if "literal not found" in error]


def find_offset(label: str, source: dict[str, Any], length: int) -> tuple[int | None, str]:
    """The smallest offset that makes the whole source valid, or why none was chosen."""
    lines = source["lines"]
    for size in range(1, WINDOW + 1):
        passing = []
        for offset in (-size, size):
            shifted = [number + offset for number in lines]
            if min(shifted) < 1 or max(shifted) > length:
                continue
            if not checks.check_sources(label, [dict(source, lines=shifted)]):
                passing.append(offset)
        if len(passing) == 1:
            return passing[0], "ok"
        if len(passing) == 2:
            return None, f"ambiguous: both {-size:+d} and {size:+d} pass"
    return None, f"no offset within +/-{WINDOW} makes every quote sit on a cited line"


TOLERANCE = 8


def occurrences(text: list[str], literal: str) -> list[int]:
    span = literal.count("\n") + 1
    return [n for n in range(1, len(text) + 1) if literal in "\n".join(text[n - 1:n - 1 + span])]


def anchor_on_literals(label: str, source: dict[str, Any]) -> tuple[list[int] | None, str]:
    """Rebuild the cited lines from where each quoted literal actually is.

    The validator demands that every cited line be quoted and every quote sit on
    a cited line, so the line list is determined by the literals; the only
    ambiguity is a literal that appears several times. That is settled by the
    shift that the file's UNIQUE literals show (a block that moved 127 lines
    moved all of its text), then by the nearest occurrence to the shifted citations.
    """
    text = checks.source_lines(source["path"])
    cited = list(source["lines"])
    literals = [lit.strip() for lit in source["required_source_literals"]]
    found = {lit: occurrences(text, lit) for lit in literals}
    if any(not hits for hits in found.values()):
        return None, "a quoted literal is not in the file at all"

    unique = [hits[0] for hits in found.values() if len(hits) == 1]
    votes: dict[int, int] = {}
    for line in unique:
        for number in cited:
            votes[line - number] = votes.get(line - number, 0) + 1
    if unique:
        # The delta supported by the most unique literals, within a small tolerance.
        def support(delta: int) -> int:
            return sum(1 for line in unique if any(abs(line - (number + delta)) <= TOLERANCE for number in cited))

        best = max(votes, key=lambda delta: (support(delta), -abs(delta)))
        delta = best
    else:
        delta = 0

    # Every line that carries a quote, anywhere in the file.
    quoted: set[int] = set()
    for literal, hits in found.items():
        span = literal.count("\n") + 1
        for hit in hits:
            quoted.update(range(hit, hit + span))
    # Move each cited line, one by one and in order, to the nearest unused quoted
    # line. The count never changes: a review that cited three lines still cites three.
    lines: list[int] = []
    used: set[int] = set()
    for number in sorted(cited):
        wanted = number + delta
        candidates = sorted((line for line in quoted if line not in used), key=lambda line: (abs(line - wanted), line))
        if not candidates or abs(candidates[0] - wanted) > TOLERANCE:
            return None, f"cited line {number} has no quoted line within {TOLERANCE} of {wanted} (shift {delta:+d})"
        used.add(candidates[0])
        lines.append(candidates[0])
    lines.sort()
    errors = checks.check_sources(label, [dict(source, lines=lines)])
    if errors:
        return None, "still invalid after anchoring: " + errors[0][:90]
    return lines, f"shift {delta:+d}"


def plan() -> list[dict[str, Any]]:
    found = []
    for name in review_files():
        for entry in unit.read_json(unit.ROOT / name)["entries"]:
            for index, source in enumerate(entry_sources(entry)):
                label = f"{entry['id']} sources[{index}]"
                if not literal_errors(label, source):
                    continue
                length = len(checks.source_lines(source["path"]))
                offset, why = find_offset(label, source, length)
                new_lines = None
                if offset is None:
                    new_lines, why = anchor_on_literals(label, source)
                found.append({"file": name, "id": entry["id"], "index": index, "path": source["path"], "lines": source["lines"], "offset": offset, "new_lines": new_lines, "why": why})
    return found


def format_like(original: str, numbers: list[int], text: str, start: int) -> str:
    """The new array in the layout the old one had: inline, or one number per line."""
    if "\n" not in original:
        return '"lines": [' + ", ".join(str(number) for number in numbers) + "]"
    line_start = text.rfind("\n", 0, start) + 1
    indent = text[line_start:start]
    inner = indent + "  "
    return '"lines": [\n' + ",\n".join(inner + str(number) for number in numbers) + "\n" + indent + "]"


def apply(changes: list[dict[str, Any]]) -> None:
    for name in sorted({change["file"] for change in changes}):
        path = unit.ROOT / name
        text = path.read_text(encoding="utf-8")
        for change in changes:
            if change["file"] != name or (change["offset"] is None and change["new_lines"] is None):
                continue
            start = text.index(f'"id": "{change["id"]}"')
            nxt = text.find('"id": "', start + 1)
            end = nxt if nxt != -1 else len(text)
            matches = list(LINES_ARRAY.finditer(text, start, end))
            if change["index"] >= len(matches):
                raise SystemExit(f"cannot find source {change['index']} of {change['id']} in {name}")
            target = matches[change["index"]]
            numbers = change["new_lines"] if change["new_lines"] is not None else [number + change["offset"] for number in change["lines"]]
            new = format_like(target.group(0), numbers, text, target.start())
            text = text[: target.start()] + new + text[target.end():]
        path.write_text(text, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()

    changes = plan()
    fixable = [change for change in changes if change["offset"] is not None or change["new_lines"] is not None]
    stuck = [change for change in changes if change["offset"] is None and change["new_lines"] is None]
    for change in fixable:
        how = f"offset {change['offset']:+d}" if change["offset"] is not None else f"anchored on its literals ({change['why']})"
        print(f"fix   {change['id']:<46} {change['path'].split('/')[-1]:<24} {how}")
    for change in stuck:
        print(f"STUCK {change['id']:<46} {change['path'].split('/')[-1]:<24} {change['why']}")
    print(f"{len(fixable)} source(s) can be re-anchored, {len(stuck)} need a person.")
    if not args.apply:
        return 0
    apply(fixable)
    print("applied; re-running the citation check:")
    return unit.cmd_citations(argparse.Namespace())


if __name__ == "__main__":
    sys.exit(main())
