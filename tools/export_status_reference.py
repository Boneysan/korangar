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
C_STATUS_CALL = re.compile(r"\bsc_start4?\s*\(")
STATUS_NAME = re.compile(r"SC_[A-Z0-9_]+")


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


def mask_c_comments_and_strings(text: str) -> str:
    """Blank C comments and literals while preserving offsets and line numbers."""
    result = list(text)
    index = 0
    state = "code"
    while index < len(text):
        current = text[index]
        following = text[index + 1] if index + 1 < len(text) else ""
        if state == "code":
            if current == "/" and following == "/":
                result[index] = result[index + 1] = " "
                index += 2
                state = "line_comment"
                continue
            if current == "/" and following == "*":
                result[index] = result[index + 1] = " "
                index += 2
                state = "block_comment"
                continue
            if current == '"':
                result[index] = " "
                state = "string"
            elif current == "'":
                result[index] = " "
                state = "character"
        elif state == "line_comment":
            if current == "\n":
                state = "code"
            else:
                result[index] = " "
        elif state == "block_comment":
            if current == "*" and following == "/":
                result[index] = result[index + 1] = " "
                index += 2
                state = "code"
                continue
            if current != "\n":
                result[index] = " "
        else:
            if current == "\\" and index + 1 < len(text):
                if current != "\n":
                    result[index] = " "
                if following != "\n":
                    result[index + 1] = " "
                index += 2
                continue
            if current == ('"' if state == "string" else "'"):
                result[index] = " "
                state = "code"
            elif current != "\n":
                result[index] = " "
        index += 1
    return "".join(result)


def split_c_arguments(masked: str, open_paren: int) -> list[str] | None:
    depth = 1
    start = open_paren + 1
    arguments: list[str] = []
    for index in range(start, len(masked)):
        char = masked[index]
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                arguments.append(masked[start:index].strip())
                return arguments
        elif char == "," and depth == 1:
            arguments.append(masked[start:index].strip())
            start = index + 1
    return None


def extract_status_call_sites(path: str, text: str) -> dict[str, list[dict[str, object]]]:
    """Find call sites with a literal SC_* third argument; dynamic types stay unknown."""
    masked = mask_c_comments_and_strings(text)
    references: dict[str, list[dict[str, object]]] = defaultdict(list)
    for match in C_STATUS_CALL.finditer(masked):
        open_paren = masked.find("(", match.start())
        arguments = split_c_arguments(masked, open_paren)
        if arguments is None or len(arguments) < 3:
            continue
        status = re.sub(r"\s+", "", arguments[2])
        if not STATUS_NAME.fullmatch(status):
            continue
        references[status].append({"path": path, "line": masked.count("\n", 0, match.start()) + 1})
    return references


def collect_status_call_sites() -> dict[str, list[dict[str, object]]]:
    references: dict[str, list[dict[str, object]]] = defaultdict(list)
    for path in sorted((HERCULES / "src/map").glob("*.c")):
        relative = path.relative_to(HERCULES).as_posix()
        for status, rows in extract_status_call_sites(relative, path.read_text(encoding="utf-8", errors="replace")).items():
            references[status].extend(rows)
    return {status: sorted(rows, key=lambda row: (str(row["path"]), int(row["line"]))) for status, rows in references.items()}


def build() -> dict[str, object]:
    icon_names: dict[str, str] = json.loads(STATUS_NAMES.read_text(encoding="utf-8"))
    sc_ids, si_ids = parse_constants(CONSTANTS.read_text(encoding="utf-8", errors="replace"))
    status_config = parse_records(STATUS_CONFIG.read_text(encoding="utf-8", errors="replace"))
    skills = parse_skill_db(SKILL_DB.read_text(encoding="utf-8", errors="replace"))
    skills_by_name = {skill["Name"]: skill for skill in skills if "Name" in skill and "Id" in skill}
    skills_by_status = index_status_change_skills(skills)
    code_call_sites = collect_status_call_sites()
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
        if constant in code_call_sites:
            status_record["code_call_sites"] = code_call_sites[constant]
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
        "source": ["db/constants.conf", "db/re/sc_config.conf", "db/re/skill_db.conf", "src/map/*.c sc_start/sc_start4 literal call sites"],
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
