#!/usr/bin/env python3
"""Export server status metadata and linked skills without inferring prose."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

from export_job_skills import CONSTANTS, HERCULES, source_revision
from export_skill_info import _parse_value, _strip_comments, _tokenize, parse_skill_db
from export_status_names import OUTPUT as STATUS_NAMES

ROOT = Path(__file__).resolve().parent.parent
STATUS_CONFIG = HERCULES / "db/re/sc_config.conf"
SKILL_DB = HERCULES / "db/re/skill_db.conf"
OUTPUT = ROOT / "docs/status-effects.v1.json"
STATUS_CONSTANT = re.compile(r"^\s*(SC_[A-Z0-9_]+):\s*(-?\d+)\s*,?\s*$")
ICON_CONSTANT = re.compile(r"^\s*(SI_[A-Z0-9_]+):\s*(-?\d+)\s*,?\s*$")


def parse_records(text: str) -> dict[str, dict[str, Any]]:
    tokens = list(_tokenize(_strip_comments(text)))
    records: dict[str, dict[str, Any]] = {}
    index = 0
    while index < len(tokens):
        if tokens[index][0] not in ("name", "digitname") or index + 2 >= len(tokens) or tokens[index + 1][1] != ":":
            raise ValueError(f"unexpected status record token near {tokens[index:index + 4]!r}")
        name = tokens[index][1]
        if name in records:
            raise ValueError(f"duplicate status record {name}")
        value, index = _parse_value(tokens, index + 2)
        if not isinstance(value, dict):
            raise ValueError(f"status record {name} is not a block")
        records[name] = value
    return records


def parse_constants(text: str) -> tuple[dict[str, int], dict[str, int]]:
    statuses: dict[str, int] = {}
    icons: dict[str, int] = {}
    for line in text.splitlines():
        status = STATUS_CONSTANT.match(line)
        icon = ICON_CONSTANT.match(line)
        if status:
            name, value = status.group(1), int(status.group(2))
            if value >= 0:
                if value in statuses.values():
                    raise ValueError(f"duplicate nonnegative status ID {value}")
                statuses[name] = value
        if icon:
            name, value = icon.group(1), int(icon.group(2))
            if value >= 0:
                icons[name] = value
    return statuses, icons


def index_status_change_skills(skills: list[dict[str, Any]]) -> dict[str, list[dict[str, object]]]:
    """Index explicit skill_db StatusChange fields without inferring other sources."""
    by_status: dict[str, list[dict[str, object]]] = defaultdict(list)
    for skill in skills:
        status = skill.get("StatusChange")
        skill_id, name = skill.get("Id"), skill.get("Name")
        if (
            not isinstance(status, str)
            or not status.startswith("SC_")
            or status == "SC_NONE"
            or not isinstance(skill_id, int)
            or not isinstance(name, str)
        ):
            continue
        by_status[status].append(
            {
                "id": skill_id,
                "name": name,
                "description": str(skill.get("Description", "")),
                "source": {"path": "db/re/skill_db.conf", "record": name},
            }
        )
    return {status: sorted(rows, key=lambda row: (int(row["id"]), str(row["name"]))) for status, rows in by_status.items()}


def build() -> dict[str, object]:
    icon_names: dict[str, str] = json.loads(STATUS_NAMES.read_text(encoding="utf-8"))
    sc_ids, si_ids = parse_constants(CONSTANTS.read_text(encoding="utf-8", errors="replace"))
    status_config = parse_records(STATUS_CONFIG.read_text(encoding="utf-8", errors="replace"))
    skills = parse_skill_db(SKILL_DB.read_text(encoding="utf-8", errors="replace"))
    skills_by_name = {skill["Name"]: skill for skill in skills if "Name" in skill and "Id" in skill}
    skills_by_status = index_status_change_skills(skills)
    status_by_icon: dict[int, list[dict[str, object]]] = defaultdict(list)

    for constant, config in sorted(status_config.items()):
        if constant not in sc_ids:
            continue
        icon_constant = config.get("Icon")
        icon_id = si_ids.get(icon_constant) if isinstance(icon_constant, str) else None
        if icon_id is None or str(icon_id) not in icon_names:
            continue

        flags = config.get("Flags") or {}
        calc_flags = config.get("CalcFlags") or {}
        skill_name = config.get("Skill")
        skill = skills_by_name.get(skill_name) if isinstance(skill_name, str) else None
        status_record: dict[str, object] = {
            "constant": constant,
            "id": sc_ids[constant],
            "flags": sorted(name for name, enabled in flags.items() if enabled is True),
            "calculation_flags": sorted(name for name, enabled in calc_flags.items() if enabled is True),
            "source": {"path": "db/re/sc_config.conf", "record": constant},
        }
        if isinstance(icon_constant, str):
            status_record["icon_constant"] = icon_constant
        if skill:
            status_record["associated_skill"] = {
                "id": int(skill["Id"]),
                "name": str(skill["Name"]),
                "description": str(skill.get("Description", "")),
                "source": {"path": "db/re/skill_db.conf", "record": str(skill["Name"])},
            }
        if constant in skills_by_status:
            status_record["status_change_skills"] = skills_by_status[constant]
        status_by_icon[icon_id].append(status_record)

    entries = [
        {
            "id": int(icon_id),
            "name": name,
            "statuses": sorted(status_by_icon.get(int(icon_id), []), key=lambda row: str(row["constant"])),
            "source": {"path": "db/constants.conf", "record": f"SI icon {icon_id}"},
        }
        for icon_id, name in sorted(icon_names.items(), key=lambda row: int(row[0]))
    ]
    if len(entries) != len(icon_names) or not any(entry["statuses"] for entry in entries):
        raise ValueError("status export did not reconcile the icon-name index and sc_config records")

    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "source": ["db/constants.conf", "db/re/sc_config.conf", "db/re/skill_db.conf"],
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check generated-data drift")
    args = parser.parse_args()
    try:
        payload = json.dumps(build(), indent=1, ensure_ascii=False) + "\n"
    except (OSError, ValueError, KeyError, StopIteration) as error:
        raise SystemExit(f"cannot export status reference data: {error}") from error
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != payload:
            print(f"stale: {OUTPUT.relative_to(ROOT)} — re-run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"up to date: {len(json.loads(payload)['entries'])} status icons")
        return 0
    OUTPUT.write_text(payload, encoding="utf-8")
    data = json.loads(payload)
    linked = sum(bool(entry["statuses"]) for entry in data["entries"])
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({linked}/{len(data['entries'])} icons linked to server statuses)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
