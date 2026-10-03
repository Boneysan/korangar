#!/usr/bin/env python3
"""Export reviewed instance-template resolutions for scripted (.@map$) monster spawns.

Reviews live in tools/scripted_spawn_reviews.json. Each entry re-derives one
NPC-script label's ".@map$ = instance_mapname(...)" assignment and confirms,
via same-execution-scope control flow (not the base bestiary exporter's
unscoped nearest-preceding-assignment-within-1000-characters heuristic), that
it is the single governing assignment for a group of monster()/areamonster()
calls previously indexed as "approximate" in bestiary.v1.json. This does not
resolve calls to a literal concrete map -- instance_mapname() templates are
bound to a dynamically-created per-instance map at runtime, which static
source review cannot observe. See each entry's "note" field.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
REVIEWS = ROOT / "tools/scripted_spawn_reviews.json"
OUTPUT = ROOT / "docs/scripted-spawn-reviews.v1.json"

LABEL_LINE = re.compile(r'^([A-Za-z_][A-Za-z0-9_]*):\s*$')
INSTANCE_MAP_ASSIGN = re.compile(r'\.@map\$\s*=\s*instance_mapname\(\s*"([A-Za-z0-9_@]+)"\s*\)\s*;')
SCRIPT_SPAWN_CALL = re.compile(r"\b(areamonster|monster|boss_monster)\s*\(\s*\.@map\$\s*,")


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL).strip()
        dirty = subprocess.run(
            ["git", "-C", str(HERCULES), "diff-index", "--quiet", "HEAD"],
            capture_output=True,
        ).returncode != 0
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def build() -> dict[str, Any]:
    review_data = json.loads(REVIEWS.read_text(encoding="utf-8"))
    if not review_data.get("reviewed_by") or not review_data.get("reviewed_on") or not review_data.get("review_method"):
        raise ValueError("scripted spawn review manifest is missing reviewer, date, or method")

    bestiary = json.loads((ROOT / "docs/bestiary.v1.json").read_text(encoding="utf-8"))
    monsters_by_id = {int(entry["id"]): entry for entry in bestiary["entries"]}

    file_cache: dict[str, list[str]] = {}
    seen_ids: set[str] = set()
    entries = []
    for review in review_data["entries"]:
        entry_id = review.get("id")
        if not isinstance(entry_id, str) or not entry_id.strip() or entry_id in seen_ids:
            raise ValueError(f"invalid or duplicate scripted spawn review ID: {entry_id!r}")
        seen_ids.add(entry_id)

        path = review.get("source_path")
        source_file = HERCULES / path if path else None
        if not path or source_file is None or not source_file.is_file():
            raise ValueError(f"scripted spawn review {entry_id} cites a script that does not exist: {path}")
        if path not in file_cache:
            file_cache[path] = source_file.read_text(encoding="utf-8", errors="replace").splitlines()
        lines = file_cache[path]

        label = review.get("label")
        label_line = review.get("label_line")
        assignment_line = review.get("assignment_line")
        template = review.get("instance_template")
        for name, value in (("label_line", label_line), ("assignment_line", assignment_line)):
            if not isinstance(value, int) or value < 1 or value > len(lines):
                raise ValueError(f"scripted spawn review {entry_id} has an invalid {name}: {value!r}")

        label_match = LABEL_LINE.match(lines[label_line - 1])
        if not label_match or label_match.group(1) != label:
            raise ValueError(f"scripted spawn review {entry_id} label no longer matches at {path}:{label_line}: {lines[label_line - 1]!r}")

        assign_match = INSTANCE_MAP_ASSIGN.search(lines[assignment_line - 1])
        if not assign_match or assign_match.group(1) != template:
            raise ValueError(f"scripted spawn review {entry_id} assignment no longer matches at {path}:{assignment_line}: {lines[assignment_line - 1]!r}")
        if not (label_line <= assignment_line):
            raise ValueError(f"scripted spawn review {entry_id} assignment line is not within its label body")

        call_lines = review.get("spawn_call_lines", [])
        mob_ids = review.get("spawn_monster_ids", [])
        if not call_lines or not mob_ids:
            raise ValueError(f"scripted spawn review {entry_id} has no spawn calls or monster IDs")
        for line_number in call_lines:
            if not isinstance(line_number, int) or line_number < 1 or line_number > len(lines):
                raise ValueError(f"scripted spawn review {entry_id} cites missing call line {line_number}")
            if line_number < assignment_line:
                raise ValueError(f"scripted spawn review {entry_id} call at line {line_number} precedes its governing assignment at {assignment_line}")
            if not SCRIPT_SPAWN_CALL.search(lines[line_number - 1]):
                raise ValueError(f"scripted spawn review {entry_id} line {line_number} is no longer a .@map$-argument spawn call: {lines[line_number - 1]!r}")
            # A later, different assignment between the governing one and this call would invalidate the confirmation.
            for between in range(assignment_line, line_number - 1):
                other = INSTANCE_MAP_ASSIGN.search(lines[between])
                if other and other.group(1) != template:
                    raise ValueError(f"scripted spawn review {entry_id} call at line {line_number} is preceded by a different reassignment at {path}:{between + 1}")
        for mob_id in mob_ids:
            if mob_id not in monsters_by_id:
                raise ValueError(f"scripted spawn review {entry_id} references unknown monster id {mob_id}")

        entries.append({
            "id": entry_id,
            "source": {"path": path, "label_line": label_line, "assignment_line": assignment_line},
            "label": label,
            "instance_template": template,
            "evidence_state": review.get("evidence_state", "conditional"),
            "verdict": review.get("verdict"),
            "note": review.get("note"),
            "spawn_monster_ids": mob_ids,
            "spawn_monster_names": sorted({monsters_by_id[mob_id]["name"] or monsters_by_id[mob_id]["sprite_name"] for mob_id in mob_ids}),
            "spawn_call_lines": call_lines,
            "spawn_count": review.get("spawn_count", len(call_lines)),
        })

    entries.sort(key=lambda entry: entry["id"])
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "reviewed_by": review_data["reviewed_by"],
        "reviewed_on": review_data["reviewed_on"],
        "review_method": review_data["review_method"],
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for stale output without writing")
    args = parser.parse_args()
    try:
        rendered = json.dumps(build(), indent=2, ensure_ascii=False) + "\n"
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"scripted spawn review export failed: {error}", file=sys.stderr)
        return 1
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {len(json.loads(rendered)['entries'])} reviewed scripted-spawn groups")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(json.loads(rendered)['entries'])} reviewed scripted-spawn groups)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
