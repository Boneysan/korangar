#!/usr/bin/env python3
"""Build the encyclopedia unit queue from loaded Hercules source.

Every unit names its package, kind, subject, the exact source ranges a model
may read (`scope`), and a tier. `local` units fit a small model's context;
`strong` units are too large or too branchy and wait for a stronger model or a
person. Rebuilding keeps the state, attempts, and history of existing units.

Order follows docs/plans/encyclopedia-completion-plan.md: E0, E1, E3, E2, E4,
E5, E6, E7. Within a package, units are sorted by path and line, except E3,
where translator clusters come before dispositions.
"""
from __future__ import annotations

import re
from collections import defaultdict
from typing import Any, Callable

from encyclopedia_common import (
    HERCULES,
    QUEUE,
    ROOT,
    block_at,
    loaded_scripts,
    npc_blocks,
    read_json,
    skills_by_key,
    slug,
    source_lines,
    write_json,
)

PACKAGE_ORDER = ("E0", "E1", "E3", "E2", "E4", "E5", "E6", "E7")
LOCAL_LINE_LIMIT = 400
SHOP_TYPES = {"shop", "cashshop", "pointshop", "itemshop", "trader", "marketshop"}
QUEST_CALL = re.compile(
    r"\b(setquest|completequest|erasequest|changequest|checkquest|questprogress|isbegin_quest|questinfo|questactive|showevent)"
    r"\s*\(?\s*(\d{3,6})\b",
    re.I,
)
CASE_LABEL = re.compile(r"^(\s*)case\s+([A-Z][A-Z0-9_]+)\s*:\s*(?://.*)?$")
DEFAULT_LABEL = re.compile(r"^(\s*)default\s*:")


def unit(
    unit_id: str,
    package: str,
    kind: str,
    subject: dict[str, Any],
    scope: list[dict[str, Any]],
    reason: str,
    tier: str | None = None,
) -> dict[str, Any]:
    size = sum(part["end"] - part["start"] + 1 for part in scope)
    return {
        "id": unit_id,
        "package": package,
        "kind": kind,
        "tier": tier or ("local" if size <= LOCAL_LINE_LIMIT else "strong"),
        "state": "todo",
        "attempts": 0,
        "subject": subject,
        "scope": scope,
        "size_lines": size,
        "reason": reason,
        "history": [],
    }


def record_span(relpath: str, anchor: int) -> dict[str, Any]:
    """libconfig record around a 1-based anchor line: from its `{` to the matching `}`."""
    lines = source_lines(relpath)

    def braces(number: int) -> tuple[int, int]:
        text = re.sub(r'"[^"]*"', "", lines[number - 1].split("//", 1)[0])
        return text.count("{"), text.count("}")

    start, depth = anchor, 0
    while start > 1:
        start -= 1
        opened, closed = braces(start)
        depth += closed - opened
        if depth < 0:
            break
    depth = 0
    for number in range(start, len(lines) + 1):
        text = re.sub(r'"[^"]*"', "", lines[number - 1].split("//", 1)[0])
        depth += text.count("{") - text.count("}")
        if depth <= 0 and number > start:
            return {"path": relpath, "start": start, "end": number}
    return {"path": relpath, "start": start, "end": len(lines)}


# E0 -------------------------------------------------------------------------

def e0_units() -> list[dict[str, Any]]:
    return [unit(
        "E0-label-sample", "E0", "e0_check", {"check": "evidence labels follow the evidence enum"}, [],
        "Run `encyclopedia_unit.py e0`. The tool checks the rule; there is nothing to draft.", tier="local",
    )]


# E1 -------------------------------------------------------------------------

def e1_units() -> list[dict[str, Any]]:
    from review_simple_exchanges import candidate_rows

    rows = candidate_rows(list(loaded_scripts().values()))
    units = []
    for row in rows:
        block = block_at(row["path"], row["line"])
        if block is None:
            continue
        units.append(unit(
            f"E1-{row['id']}", "E1", "e1_exchange",
            {"npc": row["npc"], "map": row["map"], "x": row["x"], "y": row["y"], "path": row["path"], "line": row["line"]},
            [{"path": row["path"], "start": block["start"], "end": block["end"]}],
            f"refused by the braced one-item prover: {row['reason']}",
        ))
    return units


# E2 -------------------------------------------------------------------------

def e2_units(chunk: int = 8) -> list[dict[str, Any]]:
    by_file: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for relpath in loaded_scripts():
        for block in npc_blocks(relpath):
            if block["kind"] in SHOP_TYPES:
                by_file[relpath].append(block)
    units = []
    for relpath in sorted(by_file):
        blocks = by_file[relpath]
        for offset in range(0, len(blocks), chunk):
            group = blocks[offset:offset + chunk]
            first = group[0]
            units.append(unit(
                f"E2-{slug(relpath, str(first['start']))}", "E2", "e2_stock",
                {"declarations": [
                    {"name": b["name"], "type": b["kind"], "map": b["map"], "x": b["x"], "y": b["y"], "path": relpath, "line": b["start"]}
                    for b in group
                ]},
                [{"path": relpath, "start": b["start"], "end": b["end"]} for b in group],
                f"{len(group)} stock declaration(s) not covered by the 107 literal service calls",
            ))
    return units


# E3 -------------------------------------------------------------------------

ITEM_SCRIPT_SOURCES = ("db/re/item_db.conf", "db/item_db2.conf")
SCRIPT_FIELD = re.compile(r"\b(Script|OnEquipScript|OnUnequipScript|OnRentalStartScript|OnRentalEndScript):\s*<\"(.*?)\">", re.S)
CONTROL = re.compile(r"^\s*(if|else|for|while|switch|callfunc|callsub|autobonus[23]?|set|\.@|@)\b", re.I)


def item_scripts() -> dict[int, list[dict[str, Any]]]:
    """Raw script fields per item ID with their first line number."""
    found: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for relpath in ITEM_SCRIPT_SOURCES:
        path = HERCULES / relpath
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        id_positions = [(m.start(), int(m.group(1))) for m in re.finditer(r"^\s*Id:\s*(\d+)", text, re.M)]
        for match in SCRIPT_FIELD.finditer(text):
            before = [item_id for position, item_id in id_positions if position < match.start()]
            if not before:
                continue
            line = text.count("\n", 0, match.start(2)) + 1
            found[before[-1]].append({"field": match.group(1), "script": match.group(2), "path": relpath, "line": line})
    return found


def first_rejected(script: str, line: int) -> tuple[str, str, int]:
    """Cluster key, quoted statement, and line of the first statement the translator refuses."""
    from export_item_reference import translate_simple_effect

    for piece in script.split(";"):
        statement = " ".join(piece.split())
        if not statement:
            continue
        control = CONTROL.match(statement)
        if control:
            key = "control:" + control.group(1).lower().rstrip("23")
        elif translate_simple_effect(statement + ";") is not None:
            continue
        else:
            words = statement.replace("(", " ").replace(",", " ").split()
            is_bonus = words[0].lower().startswith("bonus") and len(words) > 1
            key = " ".join(words[:2]) if is_bonus else words[0]
            arguments = statement.split(None, 1)[1].split(",", 1)[-1] if is_bonus and "," in statement else ""
            if arguments and not re.fullmatch(r"[\sA-Za-z0-9_,\-\"]*", arguments):
                key += " (expression)"
        offset = script[: script.find(piece.strip()[:30])].count("\n") if piece.strip() else 0
        return key, statement, line + offset
    return "whole-script", " ".join(script.split())[:160], line


def documented(key: str) -> tuple[str, int] | None:
    """Where Hercules documents a command or bonus, or None."""
    if key.startswith("control:") or key == "whole-script":
        return None
    name = key.split()[-1]
    candidates = [("doc/item_bonus.md", re.compile(rf"\b{re.escape(name)}\b"))] if key.lower().startswith("bonus") else []
    candidates.append(("doc/script_commands.txt", re.compile(rf"^\*{re.escape(name)}\b", re.M)))
    for relpath, pattern in candidates:
        path = HERCULES / relpath
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        match = pattern.search(text)
        if match:
            return relpath, text.count("\n", 0, match.start()) + 1
    return None


def e3_units() -> list[dict[str, Any]]:
    items = read_json(ROOT / "docs/items.v1.json")["entries"]
    open_ids = {entry["id"] for entry in items if entry["effect_status"] == "scripted_not_translated"}
    scripts = item_scripts()
    clusters: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for item_id in sorted(open_ids):
        for field in scripts.get(item_id, []):
            from export_item_reference import translate_simple_effect

            if translate_simple_effect(field["script"]) is not None:
                continue
            key, statement, line = first_rejected(field["script"], field["line"])
            clusters[key].append({"item_id": item_id, "field": field["field"], "path": field["path"], "line": line, "statement": statement})
            break
    translator, dispose = [], []
    for key, members in sorted(clusters.items(), key=lambda pair: (-len(pair[1]), pair[0])):
        doc = documented(key)
        examples = members[:6]
        scope = [{"path": m["path"], "start": m["line"], "end": m["line"]} for m in examples]
        subject = {"key": key, "item_ids": [m["item_id"] for m in members], "members": members}
        if doc:
            scope.insert(0, {"path": doc[0], "start": doc[1], "end": doc[1] + 30})
            translator.append(unit(
                f"E3-translate-{slug(key)}", "E3", "e3_translator", {**subject, "doc": {"path": doc[0], "line": doc[1]}},
                scope, f"{len(members)} item script(s) first stop at documented `{key}`", tier="local",
            ))
        else:
            dispose.append(unit(
                f"E3-unknown-{slug(key)}", "E3", "e3_dispose", subject,
                [{"path": m["path"], "start": m["line"], "end": m["line"]} for m in members],
                f"{len(members)} item script(s) first stop at `{key}`, which is control flow or undocumented", tier="local",
            ))
    return translator + dispose


# E4 -------------------------------------------------------------------------

def skillratio_groups() -> list[dict[str, Any]]:
    relpath = "src/map/battle.c"
    lines = source_lines(relpath)
    start = next(i for i, line in enumerate(lines) if line.startswith("static int battle_calc_skillratio("))
    end = next(i for i in range(start + 1, len(lines)) if lines[i].startswith("}"))
    groups: list[dict[str, Any]] = []
    index = start
    while index < end:
        label = CASE_LABEL.match(lines[index])
        if not label or label.group(2).startswith("BF_"):
            index += 1
            continue
        indent, names, first = label.group(1), [label.group(2)], index
        index += 1
        while index < end and (CASE_LABEL.match(lines[index]) or not lines[index].strip()):
            follow = CASE_LABEL.match(lines[index])
            if follow:
                names.append(follow.group(2))
            index += 1
        body_start = index
        while index < end:
            text = lines[index]
            same_case = CASE_LABEL.match(text) and CASE_LABEL.match(text).group(1) == indent
            same_default = DEFAULT_LABEL.match(text) and DEFAULT_LABEL.match(text).group(1) == indent
            closing = text.strip() == "}" and len(text) - len(text.lstrip()) < len(indent)
            if same_case or same_default or closing:
                break
            index += 1
        body = [line.strip() for line in lines[body_start:index]]
        statements = [line for line in body if line and not line.startswith("//") and line != "break;"]
        single = len(statements) == 1 and re.match(r"^skillratio\s*[+\-*/]?=", statements[0]) is not None
        preprocessor = any(line.startswith("#") for line in statements)
        groups.append({"names": names, "start": first + 1, "end": index, "single_ratio": single and not preprocessor, "preprocessor": preprocessor})
    return groups


def other_case_sites(names: list[str]) -> list[int]:
    lines = source_lines("src/map/battle.c")
    wanted = set(names)
    return [i + 1 for i, line in enumerate(lines) if (m := CASE_LABEL.match(line)) and m.group(2) in wanted]


def e4_units() -> list[dict[str, Any]]:
    reviewed = {skill_id for entry in read_json(ROOT / "tools/skill_formula_reviews.json")["entries"] for skill_id in entry["skill_ids"]}
    skills = skills_by_key()
    units = []
    for group in skillratio_groups():
        ids = [skills[name]["Id"] for name in group["names"] if name in skills]
        open_ids = [skill_id for skill_id in ids if skill_id not in reviewed]
        if not open_ids:
            continue
        scope = [{"path": "src/map/battle.c", "start": group["start"], "end": group["end"]}]
        for site in other_case_sites(group["names"]):
            if not group["start"] <= site <= group["end"]:
                scope.append({"path": "src/map/battle.c", "start": site, "end": site + 25})
        tier = "local" if group["single_ratio"] and group["end"] - group["start"] < 20 else "strong"
        units.append(unit(
            f"E4-{slug(group['names'][0])}", "E4", "e4_formula",
            {"skill_names": group["names"], "skill_ids": open_ids},
            scope,
            "single skillratio expression" if group["single_ratio"] else "skillratio case with branches or preprocessor blocks",
            tier=tier,
        ))
    units.sort(key=lambda u: (u["tier"] != "local", u["scope"][0]["start"]))
    return units


# E5 -------------------------------------------------------------------------

def quest_db_lines() -> dict[int, int]:
    lines = source_lines("db/quest_db.conf")
    return {int(m.group(1)): i + 1 for i, line in enumerate(lines) if (m := re.match(r"^\s*Id:\s*(\d+)", line))}


def e5_units() -> list[dict[str, Any]]:
    quests = read_json(ROOT / "docs/quests.v1.json")["entries"]
    open_ids = {entry["id"] for entry in quests if not entry.get("flow_review")}
    db_lines = quest_db_lines()
    calls: dict[int, set[tuple[str, int]]] = defaultdict(set)
    tokens: set[str] = set()
    for relpath in loaded_scripts():
        lines = source_lines(relpath)
        for number, line in enumerate(lines, 1):
            code = line.split("//", 1)[0]
            tokens.update(re.findall(r"\b\d{3,6}\b", code))
            for match in QUEST_CALL.finditer(code):
                quest_id = int(match.group(2))
                if quest_id in open_ids:
                    block = block_at(relpath, number)
                    if block:
                        calls[quest_id].add((relpath, block["start"]))
    for fields in item_scripts().values():
        for field in fields:
            tokens.update(re.findall(r"\b\d{3,6}\b", field["script"]))
    parent: dict[Any, Any] = {}

    def find(node: Any) -> Any:
        parent.setdefault(node, node)
        while parent[node] != node:
            parent[node] = parent[parent[node]]
            node = parent[node]
        return node

    for quest_id, blocks in calls.items():
        for block in blocks:
            parent[find(("q", quest_id))] = find(("b", block))
    chains: dict[Any, dict[str, set]] = defaultdict(lambda: {"quests": set(), "blocks": set()})
    for quest_id, blocks in calls.items():
        root = find(("q", quest_id))
        chains[root]["quests"].add(quest_id)
        chains[root]["blocks"].update(blocks)
    units = []
    for chain in chains.values():
        quest_ids = sorted(chain["quests"])
        scope = []
        for relpath, start in sorted(chain["blocks"]):
            block = block_at(relpath, start)
            scope.append({"path": relpath, "start": block["start"], "end": block["end"]})
        for quest_id in quest_ids:
            if quest_id in db_lines:
                scope.append(record_span("db/quest_db.conf", db_lines[quest_id]))
        units.append(unit(
            f"E5-chain-{quest_ids[0]}", "E5", "e5_quest", {"quest_ids": quest_ids}, scope,
            f"{len(quest_ids)} quest ID(s) set or checked by {len(chain['blocks'])} NPC block(s)",
        ))
    units.sort(key=lambda u: (u["tier"] != "local", u["scope"][0]["path"], u["scope"][0]["start"]))
    uncalled = sorted(open_ids - set(calls))
    absent = [quest_id for quest_id in uncalled if str(quest_id) not in tokens]
    indirect = [quest_id for quest_id in uncalled if str(quest_id) in tokens]
    if absent:
        units.append(unit(
            "E5-absent-from-loaded-scripts", "E5", "e5_absence", {"quest_ids": absent},
            [{"path": "db/quest_db.conf", "start": db_lines[q], "end": db_lines[q]} for q in absent if q in db_lines],
            f"{len(absent)} quest ID(s) whose number appears in no loaded NPC script and no item script", tier="local",
        ))
    for quest_id in indirect:
        units.append(unit(
            f"E5-indirect-{quest_id}", "E5", "e5_quest", {"quest_ids": [quest_id], "indirect": True},
            [record_span("db/quest_db.conf", db_lines[quest_id])] if quest_id in db_lines else [],
            "quest number appears in loaded scripts but never as a literal quest-command argument", tier="strong",
        ))
    return units


# E6 -------------------------------------------------------------------------

def e6_units() -> list[dict[str, Any]]:
    reviewed = {entry["monster_id"] for entry in read_json(ROOT / "tools/boss_behavior_reviews.json")["entries"]}
    mob_lines = source_lines("db/re/mob_db.conf")
    skill_lines = source_lines("db/re/mob_skill_db.conf")
    skill_blocks = {m.group(1): i + 1 for i, line in enumerate(skill_lines) if (m := re.match(r"^\t([A-Z0-9_]+): \{", line))}
    units = []
    for index, line in enumerate(mob_lines):
        mvp = re.match(r"^\s*MvpExp:\s*(\d+)", line)
        if not mvp or int(mvp.group(1)) == 0:
            continue
        record = record_span("db/re/mob_db.conf", index + 1)
        text = "\n".join(mob_lines[record["start"] - 1:record["end"]])
        mob_id = int(re.search(r"^\s*Id:\s*(\d+)", text, re.M).group(1))
        sprite = re.search(r'^\s*SpriteName:\s*"([^"]+)"', text, re.M).group(1)
        if mob_id in reviewed:
            continue
        scope = [record]
        if sprite in skill_blocks:
            scope.append(record_span("db/re/mob_skill_db.conf", skill_blocks[sprite] + 1))
        units.append(unit(f"E6-boss-{mob_id}", "E6", "e6_boss", {"monster_id": mob_id, "sprite_name": sprite}, scope, "MVP without a boss-behavior review"))
    zone_lines = source_lines("db/re/map_zone_db.conf")
    for index, line in enumerate(zone_lines):
        zone = re.match(r'^\s*name:\s*"([^"]+)"', line)
        if zone:
            units.append(unit(
                f"E6-zone-{slug(zone.group(1))}", "E6", "e6_zone", {"zone": zone.group(1)},
                [record_span("db/re/map_zone_db.conf", index + 1)], "map-zone family: configured restrictions",
            ))
    return units


# E7 -------------------------------------------------------------------------

def e7_units() -> list[dict[str, Any]]:
    live = unit(
        "E7-fresh-account-journeys", "E7", "e7_live", {"journeys": ["item-source-route", "skill-status-cure", "quest-turn-in"]}, [],
        "Fresh non-DM account, running server. A person must watch it.", tier="strong",
    )
    live["state"] = "blocked"
    live["blocker"] = "Live testing was deferred on 2026-09-30 (roadmap E7 row). Unblock only when the user ends the deferral."
    return [live]


BUILDERS: dict[str, Callable[[], list[dict[str, Any]]]] = {
    "E0": e0_units, "E1": e1_units, "E2": e2_units, "E3": e3_units,
    "E4": e4_units, "E5": e5_units, "E6": e6_units, "E7": e7_units,
}


def rebuild(packages: list[str]) -> dict[str, Any]:
    """Recompute the named packages. Existing unit state and history are kept."""
    queue = read_json(QUEUE) if QUEUE.is_file() else {"schema_version": 1, "units": []}
    existing = {u["id"]: u for u in queue["units"]}
    kept = [u for u in queue["units"] if u["package"] not in packages]
    fresh: list[dict[str, Any]] = []
    for package in packages:
        for new in BUILDERS[package]():
            old = existing.get(new["id"])
            if old:
                for field in ("state", "attempts", "history", "blocker", "accepted_files", "audit"):
                    if field in old:
                        new[field] = old[field]
            fresh.append(new)
        # A unit that disappeared was either finished elsewhere or its source moved. Keep finished ones.
        fresh_ids = {u["id"] for u in fresh}
        for old in existing.values():
            if old["package"] == package and old["id"] not in fresh_ids and old["state"] in ("done", "blocked"):
                fresh.append(old)
    units = kept + fresh
    units.sort(key=lambda u: PACKAGE_ORDER.index(u["package"]))
    queue["units"] = units
    write_json(QUEUE, queue)
    return queue
