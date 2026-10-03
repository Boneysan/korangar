#!/usr/bin/env python3
"""Write the one file a model reads for a unit, and the tool-drafted units.

A packet holds the unit's rules and template, pinned examples copied from the
live review file, numbered source excerpts, helper functions the excerpt calls,
and lookup tables for item, quest, and skill IDs. A model should not need to
open any other file.
"""
from __future__ import annotations

import json
import re
from typing import Any

from encyclopedia_common import (
    ROOT,
    TEMPLATES,
    function_index,
    numbered,
    read_json,
    resolve_item,
    skills_by_key,
    source_lines,
)
from encyclopedia_checks import REVIEW_FILES, existing_records

PACKET_LINE_LIMIT = 900
HELPER_LINE_LIMIT = 250
CALLFUNC = re.compile(r"\bcallfunc\s*\(?\s*\"([^\"]+)\"", re.I)
ITEM_TOKEN = re.compile(r"\b(?:getitem|delitem|countitem|getitembound|checkweight|rentitem)\s*\(?\s*([A-Za-z0-9_]+)", re.I)
QUEST_TOKEN = re.compile(r"\b(?:setquest|completequest|erasequest|changequest|checkquest|questprogress|isbegin_quest)\s*\(?\s*(\d{3,6})", re.I)
TOOL_DRAFTED = {"e3_dispose", "e5_absence"}


def template_section(kind: str) -> str:
    text = TEMPLATES.read_text(encoding="utf-8")
    match = re.search(rf"<!-- kind: {re.escape(kind)} -->\n(.*?)<!-- end -->", text, re.S)
    return match.group(1).strip() if match else ""


def pinned_examples(kind: str) -> str:
    section = template_section(kind)
    match = re.search(r"^examples:\s*(.+)$", section, re.M)
    if not match or kind not in REVIEW_FILES:
        return ""
    wanted = [part.strip() for part in match.group(1).split(",")]
    entries = {entry["id"]: entry for entry in read_json(ROOT / REVIEW_FILES[kind])["entries"]}
    blocks = []
    for entry_id in wanted:
        if entry_id in entries:
            shown = {k: v for k, v in entries[entry_id].items() if k not in ("reviewed_by", "reviewed_on")}
            blocks.append(f"```json\n{json.dumps(shown, indent=2, ensure_ascii=False)}\n```")
    return "\n\n".join(blocks)


def helper_scope(scope: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], list[str]]:
    """Global helper functions called from the excerpt, when small enough to include."""
    extra, notes, seen = [], [], set()
    for part in scope:
        if not part["path"].startswith("npc/"):
            continue
        lines = source_lines(part["path"])
        for number in range(part["start"], part["end"] + 1):
            for name in CALLFUNC.findall(lines[number - 1]):
                key = name.lower()
                if key in seen:
                    continue
                seen.add(key)
                helper = function_index().get(key)
                if helper is None:
                    notes.append(f"`callfunc \"{name}\"` (line {number}): not a loaded script function; treat its effect as unknown.")
                elif helper["end"] - helper["start"] + 1 > HELPER_LINE_LIMIT:
                    notes.append(f"`{name}` at {helper['path']}:{helper['start']} is {helper['end'] - helper['start'] + 1} lines, too large to include; treat its effect as unknown.")
                else:
                    extra.append({"path": helper["path"], "start": helper["start"], "end": helper["end"], "helper": name})
    return extra, notes


def lookups(scope: list[dict[str, Any]]) -> list[str]:
    items: dict[str, str] = {}
    quests: dict[int, str] = {}
    for part in scope:
        lines = source_lines(part["path"])
        for number in range(part["start"], min(part["end"], len(lines)) + 1):
            code = lines[number - 1].split("//", 1)[0]
            for token in ITEM_TOKEN.findall(code):
                item = resolve_item(token)
                items[token] = f"{item['id']} = {item.get('name') or item['aegis_name']}" if item else "not an exported item (a variable or a typo)"
            for token in QUEST_TOKEN.findall(code):
                quests[int(token)] = ""
    rows = []
    if items:
        rows.append("Items named in the excerpt (token -> ID = name):")
        rows += [f"- `{token}` -> {value}" for token, value in sorted(items.items())]
    if quests:
        titles = {entry["id"]: entry.get("name") or entry.get("title") or "" for entry in read_json(ROOT / "docs/quests.v1.json")["entries"]}
        rows.append("Quest IDs in the excerpt:")
        rows += [f"- {quest_id}: {titles.get(quest_id, 'not in quest_db')}" for quest_id in sorted(quests)]
    return rows


def translator_anchor() -> str:
    lines = (ROOT / "tools/export_item_reference.py").read_text(encoding="utf-8").splitlines()
    start = next(i for i, line in enumerate(lines) if line.startswith("def translate_simple_effect("))
    end = next(i for i in range(start + 1, len(lines)) if lines[i].startswith("def ") or lines[i].startswith("@"))
    tail = next(i for i in range(end - 1, start, -1) if lines[i].strip() == "return None")
    return (
        f"`translate_simple_effect` is tools/export_item_reference.py:{start + 1}-{end}. "
        f"Add your `match = re.fullmatch(...)` block before the final `return None` at line {tail + 1}.\n\n"
        + "\n".join(f"{n + 1:>5}| {lines[n]}" for n in range(tail - 16, tail + 2))
    )


def subject_lines(unit: dict[str, Any]) -> list[str]:
    subject = unit["subject"]
    kind = unit["kind"]
    if kind == "e1_exchange":
        return [f"NPC header: internal_name `{subject['npc']}`, map `{subject['map']}`, x {subject['x']}, y {subject['y']} ({subject['path']}:{subject['line']})"]
    if kind == "e2_stock":
        return [f"- `{d['name']}` ({d['type']}) {d['path']}:{d['line']}" for d in subject["declarations"]]
    if kind == "e4_formula":
        skills = skills_by_key()
        return [f"- {name} = skill ID {skills[name]['Id']}" + ("" if skills[name]["Id"] in subject["skill_ids"] else " (already reviewed; do not include)")
                for name in subject["skill_names"] if name in skills]
    if kind in ("e5_quest", "e5_absence"):
        return [f"Quest IDs: {', '.join(str(q) for q in subject['quest_ids'][:40])}" + (" ..." if len(subject["quest_ids"]) > 40 else "")]
    if kind == "e6_boss":
        return [f"Monster ID {subject['monster_id']}, sprite `{subject['sprite_name']}`"]
    if kind == "e6_zone":
        return [f"Zone `{subject['zone']}`"]
    if kind == "e3_translator":
        rows = [f"Cluster key `{subject['key']}`: {len(subject['item_ids'])} item(s). First examples:"]
        for member in subject["members"][:6]:
            item = resolve_item(str(member["item_id"]))
            rows.append(f"- item {member['item_id']} {item.get('name') if item else ''}: `{member['statement']}` ({member['path']}:{member['line']})")
        return rows
    return [json.dumps(subject)[:400]]


def build_packet(unit: dict[str, Any], draft_path: str) -> tuple[str, list[dict[str, Any]]]:
    """Packet text, and helper ranges that become part of the unit's citable scope."""
    extra: list[dict[str, Any]] = []
    notes: list[str] = []
    if unit["kind"] in ("e1_exchange", "e5_quest"):
        extra, notes = helper_scope(unit["scope"])
    scope = [part for part in unit["scope"] if not part.get("helper")] + extra
    parts = [
        f"# Unit {unit['id']}",
        f"Package {unit['package']}, kind `{unit['kind']}`, tier {unit['tier']}. Why this unit exists: {unit['reason']}.",
        f"Write your draft to `{draft_path}`. Then run `python3 tools/encyclopedia_unit.py validate`.",
        "## Subject",
        "\n".join(subject_lines(unit)),
        "## Rules",
        template_section("common"),
        f"## Template for `{unit['kind']}`",
        template_section(unit["kind"]),
    ]
    examples = pinned_examples(unit["kind"])
    if examples:
        parts += ["## Accepted examples from the review file", examples]
    if unit["kind"] == "e3_translator":
        parts += ["## Where the translator rule goes", translator_anchor()]
    already = existing_records(unit)
    if already:
        parts += ["## Already accepted in this scope",
                  "These records already cover the lines they cite. Do not review them again; their lines count as covered. "
                  "Account only for the item or quest calls they do not cite.",
                  "\n".join(f"- `{r['id']}`: {r.get('title') or r.get('statement', '')} (lines "
                            f"{sorted({n for s in (r.get('sources') or [r.get('source', {})]) for n in s.get('lines', [])})})" for r in already)]
    table = lookups(scope)
    if table:
        parts += ["## Lookups", "\n".join(table)]
    if notes:
        parts += ["## Helpers not included", "\n".join(f"- {note}" for note in notes)]
    parts.append("## Source excerpts (cite only these line numbers)")
    shown = scope
    if unit["kind"] in TOOL_DRAFTED and len(scope) > 20:
        shown = scope[:20]
        parts.append(f"The tool drafted this unit from {len(scope)} excerpts; the first 20 are shown.")
    for part in shown:
        label = f" — helper `{part['helper']}`" if part.get("helper") else ""
        parts.append(f"### {part['path']}:{part['start']}-{part['end']}{label}\n```\n{numbered(part['path'], part['start'], part['end'])}\n```")
    return "\n\n".join(parts) + "\n", extra


def verify_packet(unit: dict[str, Any], claims: list[str]) -> str:
    """What the verifier sees: sources and numbered claims, never the drafter's template or notes."""
    parts = [
        f"# Verify unit {unit['id']}",
        "You did not write these claims. Read the source excerpts, then judge each claim on its own.",
        "For each claim answer `supported` (the cited source shows it), `unsupported` (the source does not show it, "
        "or shows something else), or `partly` (part of it is not shown). Give the line number you relied on.",
        "Then list anything the source shows that the claims leave out: a cost, a failure path, a random roll, "
        "a timer, a prerequisite, or a branch that grants nothing.",
        "Write `verdict.json` next to this file, exactly in this shape:",
        '```json\n{"claims": [{"n": 1, "verdict": "supported", "line": 123, "note": ""}], "omissions": []}\n```',
        "## Claims",
        *[f"{index}. {claim}" for index, claim in enumerate(claims, 1)],
        "## Source excerpts",
    ]
    for part in unit["scope"]:
        parts.append(f"### {part['path']}:{part['start']}-{part['end']}\n```\n{numbered(part['path'], part['start'], part['end'])}\n```")
    return "\n\n".join(parts) + "\n"


SKIP_KEYS = {"id", "unit", "package", "path", "lines", "required_source_literals", "reviewed_by", "reviewed_on",
             "review_method", "source_line", "source_lines", "subject", "sprite_name", "npc"}


def claims_of(draft: dict[str, Any]) -> list[str]:
    """Every checkable sentence in a draft, flattened for the verifier."""
    claims: list[str] = []

    def walk(value: Any, prefix: str) -> None:
        if isinstance(value, dict):
            for key, inner in value.items():
                if key not in SKIP_KEYS:
                    walk(inner, f"{prefix}{key}: ")
        elif isinstance(value, list):
            for inner in value:
                walk(inner, prefix)
        elif isinstance(value, str) and len(value) > 12:
            claims.append(prefix + value)

    for review in draft.get("reviews", []):
        for entry in review.get("inputs", []):
            item = resolve_item(str(entry.get("item_id")))
            claims.append(f"The player gives {entry.get('amount')} x {item.get('name') if item else entry.get('item_id')} (item {entry.get('item_id')}).")
        for outcome in review.get("outcomes", []):
            names = [resolve_item(str(i)).get("name") if resolve_item(str(i)) else str(i) for i in outcome.get("item_ids", [])]
            claims.append(f"The player receives {outcome.get('amount')} x {' or '.join(names)} ({outcome.get('selection')}).")
        for key in ("summons", "hp_threshold_behaviors"):
            for entry in review.get(key, []):
                claims.append(f"{key}: {json.dumps({k: v for k, v in entry.items() if k not in SKIP_KEYS}, ensure_ascii=False)}")
        walk({k: v for k, v in review.items() if k not in ("inputs", "outcomes", "summons", "hp_threshold_behaviors")}, "")
    for record in draft.get("dispositions", []):
        claims.append(f"[{record.get('result')}] {record.get('statement')}")
        for condition in record.get("conditions", []):
            claims.append(f"condition: {condition}")
    return claims


# Tool-drafted units --------------------------------------------------------

def tool_draft(unit: dict[str, Any], model: str) -> dict[str, Any]:
    if unit["kind"] == "e3_dispose":
        records = []
        for member in unit["subject"]["members"]:
            line = source_lines(member["path"])[member["line"] - 1].strip()
            records.append({
                "id": f"item_{member['item_id']}_{member['field'].lower()}_untranslated",
                "unit": unit["id"],
                "package": "E3",
                "subject": {"kind": "item", "ids": [member["item_id"]]},
                "result": "unknown",
                "statement": f"The {member['field']} of item {member['item_id']} is not summarized: the translator does not interpret the quoted statement ({unit['subject']['key']}).",
                "conditions": [],
                "sources": [{"path": member["path"], "lines": [member["line"]], "required_source_literals": [line]}],
            })
        return {"unit": unit["id"], "reviews": [], "dispositions": records}
    if unit["kind"] == "e5_absence":
        from encyclopedia_queue import quest_db_lines

        db = quest_db_lines()
        records = []
        for quest_id in unit["subject"]["quest_ids"]:
            records.append({
                "id": f"quest_{quest_id}_absent_from_loaded_scripts",
                "unit": unit["id"],
                "package": "E5",
                "subject": {"kind": "quest", "ids": [quest_id]},
                "result": "absent",
                "statement": f"Quest {quest_id} has a quest database record, but its number appears in no loaded NPC script and no item script, so nothing in that scope starts, checks, or completes it.",
                "conditions": [],
                "sources": [{"path": "db/quest_db.conf", "lines": [db[quest_id]], "required_source_literals": [f"Id: {quest_id}"]}],
            })
        return {"unit": unit["id"], "reviews": [], "dispositions": records}
    raise ValueError(f"{unit['kind']} is not tool-drafted")
