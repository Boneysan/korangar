#!/usr/bin/env python3
"""Export the tracked Hercules quest DB's names and explicit hunt targets.

When the source file is absent or locally modified, the exporter uses its
tracked HEAD version and marks that fallback in the generated manifest. This
keeps unrelated local edits out of generated data and prevents implying the
static DB is the live SQL quest table.
"""

from __future__ import annotations

import argparse
import json
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


def build() -> dict[str, object]:
    quest_text, used_head_fallback, fallback_reason = read_quest_source()
    raw_quests = parse_rooted_quest_db(quest_text)
    bestiary = json.loads(BESTIARY.read_text(encoding="utf-8"))
    monsters = {int(row["id"]): row for row in bestiary["entries"]}
    mob_records = parse_skill_db(MOB_DB.read_text(encoding="utf-8", errors="replace"))
    mob_constants = {row["SpriteName"]: int(row["Id"]) for row in mob_records if "SpriteName" in row and "Id" in row}

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
        entries.append({"id": quest_id, "name": name, "targets": targets, "source_record": f"quest {quest_id}"})

    entries.sort(key=lambda entry: (str(entry["name"]).casefold(), int(entry["id"])))
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "source": "db/quest_db.conf",
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
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(data['entries'])} quests, {target_count} hunt targets)")
    if data["used_tracked_head_fallback"]:
        print(f"note: quest_db.conf {data['source_fallback_reason']}; used tracked HEAD snapshot (not live SQL data)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
