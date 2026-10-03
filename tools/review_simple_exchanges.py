#!/usr/bin/env python3
"""Add source-proven item exchanges that are a single removal and a single grant.

A branch qualifies only when one literal delitem and one literal getitem sit in
the same brace branch, and nothing between them can skip the grant (close, end,
menu, loop, call, or another branch). Same-NPC calls in other branches are not
paired. The existing exchange exporter then checks the NPC declaration and the
literal source text.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

from export_skill_info import _strip_comments
from generate_navigation_graph import loaded_script_files

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
MANIFEST = HERCULES / "npc/re/scripts_main.conf"
REVIEWS = ROOT / "tools/item_exchange_reviews.json"
NPC_HEADER = re.compile(
    r"^\s*([A-Za-z0-9_]+),\s*(-?\d+),\s*(-?\d+),\s*(-?\d+)\s+script\s+(.+?)\s+[A-Za-z0-9_]+(?:,|\{)"
)
ITEM_CALL = re.compile(r"\b(getitem|delitem)\s*\(?\s*([A-Za-z_][A-Za-z0-9_]*)\s*,\s*(\d+)", re.I)
IF_LINE = re.compile(r"^\s*(if|else\s+if)\s*\((.*)\)\s*(\{)?\s*(.*?)\s*$", re.I)
ELSE_LINE = re.compile(r"^\s*else\s*(\{)?\s*$", re.I)
SKIP = re.compile(r"\b(close|end|select|callfunc|callsub|for|while|switch|rand|return)\b", re.I)
IF_LINE_START = re.compile(r"^\s*(?:else\s+)?if\b", re.I)
ALLOWED_IF = re.compile(r"^\s*(?:else\s+)?if\b.*\)\s*(?:\{.*|(?:close|end)\s*;)\s*$", re.I)
BRACELESS_ELSE = re.compile(r"^\s*else\s*$", re.I)


def body_uses_unbraced_branches(lines: list[str]) -> bool:
    for line in lines:
        if BRACELESS_ELSE.match(line):
            return True
        if IF_LINE_START.match(line) and not ALLOWED_IF.match(line):
            return True
    return False


def npc_blocks(text: str) -> list[tuple[re.Match[str], int, int]]:
    lines = text.splitlines()
    starts = [index for index, line in enumerate(lines) if NPC_HEADER.match(line)]
    blocks = []
    for position, start in enumerate(starts):
        end = starts[position + 1] if position + 1 < len(starts) else len(lines)
        header = NPC_HEADER.match(lines[start])
        if header:
            blocks.append((header, start, end))
    return blocks


def _pop_closed(stack: list[tuple[int, str, int]], brace: int) -> None:
    while stack and stack[-1][2] >= brace:
        stack.pop()


def branches(lines: list[str]) -> list[dict[str, Any]]:
    """Group literal item calls that share one brace branch."""
    brace = 0
    stack: list[tuple[int, str, int]] = []
    next_id = 1
    grouped: dict[tuple[int, ...], list[dict[str, Any]]] = {}
    conditions: dict[tuple[int, ...], list[str]] = {}
    for offset, raw in enumerate(lines):
        line = raw.strip()
        if not line:
            continue
        closes = len(re.findall(r"\}", line))
        for _ in range(closes):
            brace -= 1
            _pop_closed(stack, brace)
        remainder = line.replace("}", "").strip()
        else_header = ELSE_LINE.match(remainder)
        if_header = IF_LINE.match(remainder)
        if else_header or (if_header and if_header.group(1).lower().startswith("else")):
            if stack:
                stack.pop()
        if if_header and "{" in remainder:
            stack.append((next_id, if_header.group(2).strip(), brace))
            next_id += 1
            brace += remainder.count("{")
        elif else_header and "{" in remainder:
            stack.append((next_id, "the preceding branch was not taken", brace))
            next_id += 1
            brace += remainder.count("{")
        elif "{" in remainder:
            brace += remainder.count("{")
        call = ITEM_CALL.search(remainder)
        if call is None:
            continue
        key = tuple(item[0] for item in stack)
        grouped.setdefault(key, []).append({
            "kind": call.group(1).lower(),
            "constant": call.group(2),
            "amount": int(call.group(3)),
            "offset": offset,
            "text": call.group(0).rstrip(";") + ";",
        })
        conditions.setdefault(key, [item[1] for item in stack])
    return [{"conditions": conditions.get(key, []), "calls": calls} for key, calls in grouped.items()]


def proven_exchange(calls: list[dict[str, Any]], lines: list[str]) -> dict[str, Any] | None:
    removals = [call for call in calls if call["kind"] == "delitem"]
    grants = [call for call in calls if call["kind"] == "getitem"]
    if len(removals) != 1 or len(grants) != 1 or len(calls) != 2:
        return None
    first, second = sorted(calls, key=lambda call: call["offset"])
    between = "\n".join(lines[first["offset"] + 1:second["offset"]])
    if SKIP.search(between):
        return None
    return {"removal": removals[0], "grant": grants[0]}


def slug(map_name: str, npc_name: str, removal: str, grant: str) -> str:
    raw = f"{map_name}_{npc_name}_{removal}_for_{grant}".lower()
    return re.sub(r"[^a-z0-9]+", "_", raw).strip("_")[:80]


def discover(files: list[Path], items_by_aegis: dict[str, dict[str, Any]]) -> tuple[list[dict[str, Any]], int]:
    reviews = []
    rejected = 0
    for path in files:
        raw_lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        stripped = _strip_comments("\n".join(raw_lines)).splitlines()
        # Comment stripping must keep line numbers aligned. _strip_comments removes
        # comment text but not whole lines in this codebase; fall back to raw if not.
        if len(stripped) != len(raw_lines):
            stripped = raw_lines
        relpath = path.relative_to(HERCULES).as_posix()
        for header, start, end in npc_blocks("\n".join(stripped)):
            body = stripped[start + 1:end]
            if body_uses_unbraced_branches(body):
                if any(ITEM_CALL.search(line) for line in body):
                    rejected += 1
                continue
            if any(re.search(r"\b(select|switch)\s*\(", line) for line in body):
                if any(ITEM_CALL.search(line) for line in body):
                    rejected += 1
                continue
            proven = []
            for branch in branches(body):
                exchange = proven_exchange(branch["calls"], body)
                if exchange is None:
                    if branch["calls"]:
                        rejected += 1
                    continue
                removal_item = items_by_aegis.get(exchange["removal"]["constant"].lower())
                grant_item = items_by_aegis.get(exchange["grant"]["constant"].lower())
                if removal_item is None or grant_item is None:
                    rejected += 1
                    continue
                map_name, x, y, _direction, internal = header.groups()
                base = start + 1
                reviews.append({
                    "id": slug(map_name, internal, exchange["removal"]["constant"], exchange["grant"]["constant"]),
                    "title": f"{internal} exchanges {removal_item.get('name') or exchange['removal']['constant']} for {grant_item.get('name') or exchange['grant']['constant']}",
                    "npc": {
                        "name": internal.split("#", 1)[0],
                        "internal_name": internal,
                        "map": map_name,
                        "x": int(x),
                        "y": int(y),
                        "service_role": "Source-proven single-branch item exchange",
                    },
                    "inputs": [{"item_id": int(removal_item["id"]), "amount": exchange["removal"]["amount"]}],
                    "outcomes": [{
                        "label": grant_item.get("name") or exchange["grant"]["constant"],
                        "item_ids": [int(grant_item["id"])],
                        "amount": exchange["grant"]["amount"],
                        "selection": "single_option",
                    }],
                    "conditions": [
                        *branch["conditions"],
                        "The removal and the grant are the only item calls in this brace branch, and no close, end, menu, loop, or call sits between them.",
                    ],
                    "source": {
                        "path": relpath,
                        "lines": sorted({
                            start + 1,
                            base + exchange["removal"]["offset"] + 1,
                            base + exchange["grant"]["offset"] + 1,
                        }),
                    },
                    "required_source_literals": [
                        exchange["removal"]["text"],
                        exchange["grant"]["text"],
                    ],
                })
                proven.append(exchange)
            if not proven and any(ITEM_CALL.search(line) for line in body):
                rejected += 1
    return reviews, rejected


def refusal_reason(body: list[str]) -> str:
    """Why this NPC body is not a braced one-item exchange. Empty if it has no item call."""
    if not any(ITEM_CALL.search(line) for line in body):
        return ""
    text = "\n".join(body)
    if body_uses_unbraced_branches(body):
        return "braceless"
    if re.search(r"\bswitch\s*\(", text):
        return "switch"
    if re.search(r"\bselect\s*\(", text):
        return "menu"
    if re.search(r"\b(?:callfunc|callsub)\b", text):
        return "call"
    if re.search(r"\b(?:for|while|rand)\s*\(", text):
        return "loop-or-random"
    return "multi-item"


def candidate_rows(files: list[Path]) -> list[dict[str, Any]]:
    rows = []
    for path in files:
        raw_lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        stripped = _strip_comments("\n".join(raw_lines)).splitlines()
        if len(stripped) != len(raw_lines):
            stripped = raw_lines
        relpath = path.relative_to(HERCULES).as_posix()
        for header, start, end in npc_blocks("\n".join(stripped)):
            body = stripped[start + 1:end]
            reason = refusal_reason(body)
            if not reason:
                continue
            if reason == "multi-item":
                groups = [branch for branch in branches(body) if branch["calls"]]
                if groups and all(proven_exchange(branch["calls"], body) for branch in groups):
                    continue
            map_name, x, y, _direction, internal = header.groups()
            rows.append({
                "id": slug(map_name, internal, "branch", str(start + 1)),
                "reason": reason,
                "path": relpath,
                "line": start + 1,
                "map": map_name,
                "x": int(x),
                "y": int(y),
                "npc": internal,
            })
    rows.sort(key=lambda row: (row["path"], row["line"], row["id"]))
    return rows


def merge(existing: dict[str, Any], found: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], int]:
    seen = {entry["id"] for entry in existing["entries"]}
    literals = {
        (entry["source"]["path"], literal)
        for entry in existing["entries"]
        for literal in entry.get("required_source_literals", [])
    }
    added = []
    for review in found:
        if review["id"] in seen:
            continue
        if any((review["source"]["path"], literal) in literals for literal in review["required_source_literals"]):
            continue
        seen.add(review["id"])
        added.append(review)
    return added, len(found) - len(added)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="append proven exchanges into the review file")
    parser.add_argument("--candidates", type=Path, help="write refused NPC branches as a candidate queue and exit")
    args = parser.parse_args()
    if args.candidates is not None:
        files = loaded_script_files(HERCULES, MANIFEST)
        rows = candidate_rows(files)
        payload = {"schema_version": 1, "rule": "refused by the braced one-item exchange prover", "entries": rows}
        args.candidates.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {args.candidates} ({len(rows)} candidates)")
        return 0
    items = json.loads((ROOT / "docs/items.v1.json").read_text(encoding="utf-8"))["entries"]
    by_aegis = {item["aegis_name"].lower(): item for item in items}
    files = loaded_script_files(HERCULES, MANIFEST)
    found, rejected = discover(files, by_aegis)
    existing = json.loads(REVIEWS.read_text(encoding="utf-8"))
    added, skipped = merge(existing, found)
    print(f"proven branches {len(found)}; new {len(added)}; already reviewed {skipped}; other item branches left open {rejected}")
    if not args.write:
        return 0
    existing["entries"].extend(added)
    existing["reviewed_on"] = "2026-09-30"
    REVIEWS.write_text(json.dumps(existing, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {REVIEWS.relative_to(ROOT)} ({len(existing['entries'])} reviews)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
