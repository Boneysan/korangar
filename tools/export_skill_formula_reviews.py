#!/usr/bin/env python3
"""Export manually reviewed skill/status damage-formula derivations with source-anchor validation.

Unlike the NPC-script reviewed-data files, sources here are engine C source
(src/map/*.c) and db/re/skill_db.conf / conf/map/battle/skill.conf -- files
that are always present regardless of scripts_main.conf's loaded-script
manifest, so no loaded-script check applies. Every citation is still
re-validated against the live sibling Hercules checkout on each run.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from export_job_skills import source_revision
from export_skill_info import SKILL_DB, parse_skill_db

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
REVIEWS = ROOT / "tools" / "skill_formula_reviews.json"
OUTPUT = ROOT / "docs" / "skill-formula-reviews.v1.json"

REQUIRED_ENTRY_FIELDS = ("id", "title", "skill_ids", "reviewed_by", "reviewed_on", "review_method", "evidence_state", "formula", "worked_example", "conditions", "sources")
VALID_EVIDENCE_STATES = {"verified", "conditional", "configured_estimate", "source_clue", "not_reviewed", "unknown"}


def load_skill_names() -> dict[int, str]:
    text = SKILL_DB.read_text(encoding="utf-8", errors="replace")
    records = parse_skill_db(text)
    names: dict[int, str] = {}
    for record in records:
        skill_id = record.get("Id")
        name = record.get("Name")
        if isinstance(skill_id, int) and isinstance(name, str):
            names[skill_id] = name
    return names


def build() -> dict[str, Any]:
    review_data = json.loads(REVIEWS.read_text(encoding="utf-8"))
    skill_names = load_skill_names()
    entries: list[dict[str, Any]] = []
    seen_ids: set[str] = set()
    seen_skill_ids: set[int] = set()

    for review in review_data["entries"]:
        entry_id = review.get("id")
        if not isinstance(entry_id, str) or not entry_id.strip() or entry_id in seen_ids:
            raise ValueError(f"invalid or duplicate reviewed skill-formula ID: {entry_id!r}")
        seen_ids.add(entry_id)
        for field in REQUIRED_ENTRY_FIELDS:
            if field not in review:
                raise ValueError(f"reviewed skill-formula {entry_id} is missing required field: {field}")
        if review["evidence_state"] not in VALID_EVIDENCE_STATES:
            raise ValueError(f"reviewed skill-formula {entry_id} has unknown evidence_state: {review['evidence_state']}")
        skill_ids = review["skill_ids"]
        if not isinstance(skill_ids, list) or not skill_ids:
            raise ValueError(f"reviewed skill-formula {entry_id} has no skill_ids")
        for skill_id in skill_ids:
            if not isinstance(skill_id, int) or skill_id not in skill_names:
                raise ValueError(f"reviewed skill-formula {entry_id} references unknown skill ID {skill_id!r}")
            if skill_id in seen_skill_ids:
                raise ValueError(f"skill ID {skill_id} is claimed by more than one reviewed skill-formula entry; merge the reviewed coverage explicitly")
            seen_skill_ids.add(skill_id)
        if not review["conditions"]:
            raise ValueError(f"reviewed skill-formula {entry_id} has no conditions/caveats listed")

        source_refs = []
        for source in review["sources"]:
            path = source.get("path")
            lines = source.get("lines")
            literals = source.get("required_source_literals")
            if not isinstance(path, str) or not path or not isinstance(lines, list) or not lines:
                raise ValueError(f"reviewed skill-formula {entry_id} has malformed source reference")
            source_file = HERCULES / path
            if not source_file.is_file():
                raise ValueError(f"reviewed skill-formula {entry_id} cites a source file that does not exist: {path}")
            text = source_file.read_text(encoding="utf-8", errors="replace")
            source_lines = text.splitlines()
            for line_number in lines:
                if not isinstance(line_number, int) or line_number < 1 or line_number > len(source_lines):
                    raise ValueError(f"reviewed skill-formula {entry_id} cites missing source line {path}:{line_number}")
            if not isinstance(literals, list) or not literals:
                raise ValueError(f"reviewed skill-formula {entry_id} has no required_source_literals for {path}")
            for literal in literals:
                if literal not in text:
                    raise ValueError(f"reviewed skill-formula {entry_id} source anchor no longer matches in {path}: {literal!r}")
            source_refs.append({"path": path, "lines": lines})
        if not source_refs:
            raise ValueError(f"reviewed skill-formula {entry_id} has no source references")

        entry = {
            "id": entry_id,
            "title": review["title"],
            "skill_ids": [{"skill_id": skill_id, "skill_name": skill_names[skill_id]} for skill_id in skill_ids],
            "evidence_state": review["evidence_state"],
            "reviewed_by": review["reviewed_by"],
            "reviewed_on": review["reviewed_on"],
            "review_method": review["review_method"],
            "formula": review["formula"],
            "worked_example": review["worked_example"],
            "conditions": review["conditions"],
            "sources": source_refs,
        }
        if "per_skill" in review:
            per_skill = review["per_skill"]
            if not isinstance(per_skill, list) or not per_skill:
                raise ValueError(f"reviewed skill-formula {entry_id} has a malformed per_skill breakdown")
            for row in per_skill:
                skill_id = row.get("skill_id")
                if skill_id not in skill_ids:
                    raise ValueError(f"reviewed skill-formula {entry_id} per_skill entry references a skill_id not in skill_ids: {skill_id!r}")
                if row.get("skill_name") != skill_names[skill_id]:
                    raise ValueError(f"reviewed skill-formula {entry_id} per_skill entry names skill {skill_id} as {row.get('skill_name')!r}, but skill_db.conf has {skill_names[skill_id]!r}")
            entry["per_skill"] = per_skill
        entries.append(entry)

    revision, dirty = source_revision()
    entries.sort(key=lambda entry: entry["id"])
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "review_scope": "Manually reviewed per-level damage/heal formula derivations, traced against engine C source and skill_db.conf. Not a full skill catalogue -- see docs/plans/encyclopedia-roadmap.md E4 for remaining scope.",
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for stale output without writing")
    args = parser.parse_args()
    try:
        rendered = json.dumps(build(), indent=2, ensure_ascii=False) + "\n"
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"skill formula review export failed: {error}", file=sys.stderr)
        return 1
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {len(json.loads(rendered)['entries'])} reviewed skill-formula entries")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(json.loads(rendered)['entries'])} reviewed skill-formula entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
