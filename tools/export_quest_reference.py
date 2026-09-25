#!/usr/bin/env python3
"""Export tracked Hercules quest names, explicit hunt targets, and static NPC references.

When the source file is absent or locally modified, the exporter uses its
tracked HEAD version and marks that fallback in the generated manifest. This
keeps unrelated local edits out of generated data and prevents implying the
static DB is the live SQL quest table.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from export_job_skills import HERCULES, source_revision
from export_skill_info import _parse_value, _strip_comments, _tokenize, parse_skill_db

ROOT = Path(__file__).resolve().parent.parent
QUEST_DB = HERCULES / "db/quest_db.conf"
MOB_DB = HERCULES / "db/re/mob_db.conf"
BESTIARY = ROOT / "docs/bestiary.v1.json"
OUTPUT = ROOT / "docs/quests.v1.json"
REVIEWED_NPC_ROUTES = ROOT / "tools/quest_npc_routes.json"
NPC_QUEST_DIRS = (HERCULES / "npc/re/quests", HERCULES / "npc/custom")
NPC_HEADER = re.compile(r"^\s*([A-Za-z0-9_]+),\s*(-?\d+),\s*(-?\d+),\s*(-?\d+)\s+script\s+([^\s,]+)\s+[^,\n]+,\s*\{")
QUEST_CALL = re.compile(r"\b(setquest|questprogress|completequest|erasequest)\s*\(?\s*(\d+)")


def read_quest_source() -> tuple[str, bool, str | None]:
    status = subprocess.run(
        ["git", "-C", str(HERCULES), "status", "--porcelain", "--", "db/quest_db.conf"],
        text=True,
        check=True,
        capture_output=True,
    ).stdout
    if QUEST_DB.is_file() and not status.strip():
        return QUEST_DB.read_text(encoding="utf-8", errors="replace"), False, None
    try:
        text = subprocess.check_output(
            ["git", "-C", str(HERCULES), "show", "HEAD:db/quest_db.conf"], text=True, stderr=subprocess.PIPE
        )
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError("quest_db.conf is absent and no tracked HEAD version is available") from error
    return text, True, "locally_modified" if QUEST_DB.is_file() else "missing"


def parse_rooted_quest_db(text: str) -> list[dict[str, Any]]:
    tokens = list(_tokenize(_strip_comments(text)))
    for index, (_, value) in enumerate(tokens):
        if value == "quest_db":
            if index + 2 >= len(tokens) or tokens[index + 1][1] != ":":
                raise ValueError("quest_db root has no colon")
            rows, _ = _parse_value(tokens, index + 2)
            if not isinstance(rows, list) or not all(isinstance(row, dict) for row in rows):
                raise ValueError("quest_db root is not a list of records")
            return rows
    raise ValueError("quest_db root not found")


def extract_npc_quest_references(text: str, source_path: str, quest_ids: set[int]) -> dict[int, list[dict[str, Any]]]:
    """Map literal quest-state calls to their nearest static NPC declaration."""
    result: dict[int, list[dict[str, Any]]] = {}
    current_npc: dict[str, Any] | None = None
    npc_depth: int | None = None
    depth = 0
    for line_number, line in enumerate(_strip_comments(text).splitlines(), 1):
        code = line
        header = NPC_HEADER.match(code)
        if header and depth == 0:
            map_name, x, y, _direction, name = header.groups()
            current_npc = {
                "name": name,
                "map_name": map_name,
                "x": int(x),
                "y": int(y),
                "source_path": source_path,
                "script_line": line_number,
                "uses": [],
            }
            npc_depth = depth + 1
        if current_npc is not None and npc_depth is not None and depth >= npc_depth:
            for call in QUEST_CALL.finditer(code):
                quest_id = int(call.group(2))
                if quest_id not in quest_ids:
                    continue
                record = result.setdefault(quest_id, [])
                key = (current_npc["name"], current_npc["map_name"], current_npc["x"], current_npc["y"])
                existing = next((npc for npc in record if (npc["name"], npc["map_name"], npc["x"], npc["y"]) == key), None)
                if existing is None:
                    existing = {**current_npc, "source_line": line_number, "uses": []}
                    record.append(existing)
                use = call.group(1)
                if use not in existing["uses"]:
                    existing["uses"].append(use)
        in_string = False
        escaped = False
        for character in code:
            if escaped:
                escaped = False
            elif character == "\\" and in_string:
                escaped = True
            elif character == '"':
                in_string = not in_string
            elif not in_string and character == "{":
                depth += 1
            elif not in_string and character == "}":
                depth -= 1
        if npc_depth is not None and depth < npc_depth:
            current_npc = None
            npc_depth = None
    for records in result.values():
        records.sort(key=lambda npc: (npc["map_name"], npc["name"], npc["x"], npc["y"]))
    return result


def npc_identity_at_line(text: str, target_line: int) -> tuple[str, str, int, int] | None:
    """Return the static NPC enclosing a source line, if there is one."""
    depth = 0
    npc_depth: int | None = None
    identity: tuple[str, str, int, int] | None = None
    for line_number, code in enumerate(_strip_comments(text).splitlines(), 1):
        if line_number == target_line:
            return identity if npc_depth is not None and depth >= npc_depth else None
        header = NPC_HEADER.match(code)
        if header and depth == 0:
            map_name, x, y, _direction, name = header.groups()
            identity = (name, map_name, int(x), int(y))
            npc_depth = depth + 1
        in_string = False
        escaped = False
        for character in code:
            if escaped:
                escaped = False
            elif character == "\\" and in_string:
                escaped = True
            elif character == '"':
                in_string = not in_string
            elif not in_string and character == "{":
                depth += 1
            elif not in_string and character == "}":
                depth -= 1
        if npc_depth is not None and depth < npc_depth:
            identity = None
            npc_depth = None
    return None


def apply_reviewed_npc_routes(
    npc_references: dict[int, list[dict[str, Any]]],
    routes: list[dict[str, Any]],
    quest_ids: set[int],
    source_texts: dict[str, str],
) -> None:
    """Annotate only reviewed NPC/quest pairs whose cited source calls still exist."""
    seen: set[tuple[int, str, str, str, int, int, str]] = set()
    for route in routes:
        try:
            quest_id = route["quest_id"]
            source_path = route["source_path"]
            npc_name = route["npc_name"]
            map_name = route["map_name"]
            x = route["x"]
            y = route["y"]
            role = route["role"]
            source_lines = route["source_lines"]
            evidence = route["evidence"]
        except (KeyError, TypeError) as error:
            raise ValueError(f"malformed reviewed quest NPC route: {route!r}") from error
        if not isinstance(quest_id, int) or quest_id not in quest_ids:
            raise ValueError(f"reviewed NPC route references unknown quest {quest_id!r}")
        if (
            not isinstance(source_path, str)
            or not source_path.strip()
            or not isinstance(npc_name, str)
            or not npc_name.strip()
            or not isinstance(map_name, str)
            or not map_name.strip()
            or not isinstance(x, int)
            or not isinstance(y, int)
            or x < 0
            or y < 0
            or not isinstance(role, str)
        ):
            raise ValueError(f"quest {quest_id} reviewed NPC route has invalid identity/location fields")
        key = (quest_id, source_path, npc_name, map_name, x, y, role)
        if key in seen:
            raise ValueError(f"duplicate reviewed NPC route: {key!r}")
        seen.add(key)
        if role not in {"offer", "turn_in"}:
            raise ValueError(f"quest {quest_id} has unsupported reviewed NPC role {role!r}")
        if not isinstance(evidence, str) or not evidence.strip():
            raise ValueError(f"quest {quest_id} reviewed NPC route has no evidence note")
        if not isinstance(source_lines, list) or not source_lines or not all(isinstance(line, int) and line > 0 for line in source_lines):
            raise ValueError(f"quest {quest_id} reviewed NPC route has invalid source lines")

        source_text = source_texts.get(source_path)
        if source_text is None:
            raise ValueError(f"reviewed NPC route source is not loaded: {source_path!r}")
        required_calls = {"offer": {"setquest"}, "turn_in": {"completequest", "erasequest"}}[role]
        source_code = _strip_comments(source_text).splitlines()
        observed_calls = set()
        for line_number in source_lines:
            if line_number > len(source_code):
                raise ValueError(f"quest {quest_id} reviewed NPC route cites missing line {line_number}")
            expected_npc = (npc_name, map_name, x, y)
            if npc_identity_at_line(source_text, line_number) != expected_npc:
                raise ValueError(f"quest {quest_id} reviewed NPC route line {line_number} is outside its declared NPC")
            observed_calls.update(
                call.group(1)
                for call in QUEST_CALL.finditer(source_code[line_number - 1])
                if int(call.group(2)) == quest_id
            )
        if not observed_calls.intersection(required_calls):
            raise ValueError(f"quest {quest_id} reviewed NPC route no longer has its cited {role} call")

        matching = next((npc for npc in npc_references.get(quest_id, []) if (
            npc["source_path"], npc["name"], npc["map_name"], npc["x"], npc["y"]
        ) == (source_path, npc_name, map_name, x, y)), None)
        if matching is None:
            raise ValueError(f"quest {quest_id} reviewed NPC route does not match an extracted script reference")
        matching["reviewed_role"] = role
        matching["reviewed_source_lines"] = source_lines
        matching["review_evidence"] = evidence


def build() -> dict[str, object]:
    quest_text, used_head_fallback, fallback_reason = read_quest_source()
    raw_quests = parse_rooted_quest_db(quest_text)
    bestiary = json.loads(BESTIARY.read_text(encoding="utf-8"))
    monsters = {int(row["id"]): row for row in bestiary["entries"]}
    mob_records = parse_skill_db(MOB_DB.read_text(encoding="utf-8", errors="replace"))
    mob_constants = {row["SpriteName"]: int(row["Id"]) for row in mob_records if "SpriteName" in row and "Id" in row}
    quest_ids = {int(row["Id"]) for row in raw_quests if isinstance(row.get("Id"), int)}
    npc_references: dict[int, list[dict[str, Any]]] = {}
    npc_source_texts: dict[str, str] = {}
    for directory in NPC_QUEST_DIRS:
        if not directory.is_dir():
            continue
        for script_path in sorted(directory.rglob("*.txt")):
            relative_path = script_path.relative_to(HERCULES).as_posix()
            source_text = script_path.read_text(encoding="utf-8", errors="replace")
            npc_source_texts[relative_path] = source_text
            references = extract_npc_quest_references(source_text, relative_path, quest_ids)
            for quest_id, records in references.items():
                npc_references.setdefault(quest_id, []).extend(records)

    reviewed_manifest = json.loads(REVIEWED_NPC_ROUTES.read_text(encoding="utf-8"))
    if reviewed_manifest.get("schema_version") != 1 or not isinstance(reviewed_manifest.get("entries"), list):
        raise ValueError("reviewed quest NPC route manifest has an unsupported schema")
    apply_reviewed_npc_routes(npc_references, reviewed_manifest["entries"], quest_ids, npc_source_texts)

    entries = []
    seen: set[int] = set()
    for row in raw_quests:
        quest_id = row.get("Id")
        name = row.get("Name")
        if not isinstance(quest_id, int) or not isinstance(name, str) or not name.strip():
            raise ValueError(f"invalid quest record: expected integer Id and nonempty Name, got {row!r}")
        if quest_id in seen:
            raise ValueError(f"duplicate quest ID {quest_id}")
        seen.add(quest_id)

        targets = []
        for target in row.get("Targets", []) or []:
            if not isinstance(target, dict):
                raise ValueError(f"quest {quest_id} has a malformed target")
            raw_mob_id = target.get("MobId", 0)
            if isinstance(raw_mob_id, str):
                mob_id = mob_constants.get(raw_mob_id)
                if mob_id is None:
                    raise ValueError(f"quest {quest_id} target has unknown mob constant {raw_mob_id}")
            elif isinstance(raw_mob_id, int):
                mob_id = raw_mob_id or None
            else:
                raise ValueError(f"quest {quest_id} target has invalid MobId {raw_mob_id!r}")
            raw_level = target.get("Level")
            level_range = None
            if isinstance(raw_level, list) and len(raw_level) == 2 and all(isinstance(value, int) for value in raw_level):
                level_range = raw_level
            elif raw_level is not None:
                raise ValueError(f"quest {quest_id} target has unsupported Level value {raw_level!r}")

            map_name = target.get("MapName")
            if map_name is not None and not isinstance(map_name, str):
                raise ValueError(f"quest {quest_id} target has invalid MapName {map_name!r}")
            monster = monsters.get(mob_id) if mob_id is not None else None
            targets.append({
                "mob_id": mob_id,
                "monster_name": monster["name"] if monster else ("Any monster" if mob_id is None else f"Monster {mob_id}"),
                "monster_data_known": monster is not None if mob_id is not None else None,
                "count": int(target.get("Count", 0)),
                "level_range": level_range,
                "map_name": map_name,
                "source_record": f"quest {quest_id} target {len(targets) + 1}",
            })
        entries.append({
            "id": quest_id,
            "name": name,
            "targets": targets,
            "npc_references": npc_references.get(quest_id, []),
            "source_record": f"quest {quest_id}",
        })

    entries.sort(key=lambda entry: (str(entry["name"]).casefold(), int(entry["id"])))
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "source": "db/quest_db.conf",
        "npc_sources": ["npc/re/quests", "npc/custom"],
        "source_file_available_in_worktree": QUEST_DB.is_file(),
        "used_tracked_head_fallback": used_head_fallback,
        "source_fallback_reason": fallback_reason,
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check generated-data drift")
    args = parser.parse_args()
    try:
        payload = json.dumps(build(), indent=1, ensure_ascii=False) + "\n"
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise SystemExit(f"cannot export quest reference data: {error}") from error
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != payload:
            print(f"stale: {OUTPUT.relative_to(ROOT)} — re-run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"up to date: {len(json.loads(payload)['entries'])} quests")
        return 0
    OUTPUT.write_text(payload, encoding="utf-8")
    data = json.loads(payload)
    target_count = sum(len(entry["targets"]) for entry in data["entries"])
    npc_count = sum(bool(entry["npc_references"]) for entry in data["entries"])
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(data['entries'])} quests, {target_count} hunt targets, {npc_count} with static NPC references)")
    if data["used_tracked_head_fallback"]:
        print(f"note: quest_db.conf {data['source_fallback_reason']}; used tracked HEAD snapshot (not live SQL data)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
