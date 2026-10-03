#!/usr/bin/env python3
"""Export manually reviewed boss summon/HP-threshold behavior with source-anchor validation.

Reviews live in tools/boss_behavior_reviews.json. Each entry cites exact
db/re/mob_skill_db.conf line numbers and literal substrings that must still be
present in the live Hercules source; summon candidates and HP-threshold skill
names are cross-checked against the versioned bestiary/skill exports so a
renamed sprite or skill cannot silently go stale. Nothing here is inferred:
NPC_SUMMONMONSTER/NPC_SUMMONSLAVE's val0-val4 candidate list and SkillLevel-as-amount
semantics, and every MSC_* cast-condition meaning, were confirmed by reading
src/map/mob.c and src/map/skill.c directly (see review_method).
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
MOB_SKILL_DB = HERCULES / "db/re/mob_skill_db.conf"
REVIEWS = ROOT / "tools/boss_behavior_reviews.json"
OUTPUT = ROOT / "docs/boss-behavior.v1.json"


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
        raise ValueError("boss behavior review manifest is missing reviewer, date, or method")

    mob_skill_text = MOB_SKILL_DB.read_text(encoding="utf-8", errors="replace")
    mob_skill_lines = mob_skill_text.splitlines()

    bestiary = json.loads((ROOT / "docs/bestiary.v1.json").read_text(encoding="utf-8"))
    monsters_by_id = {int(entry["id"]): entry for entry in bestiary["entries"]}

    skills = json.loads((ROOT / "docs/skills.json").read_text(encoding="utf-8"))
    skill_names = {row["Name"] for row in skills if "Name" in row}

    seen_ids: set[str] = set()
    entries = []
    for review in review_data["entries"]:
        entry_id = review.get("id")
        if not isinstance(entry_id, str) or not entry_id.strip() or entry_id in seen_ids:
            raise ValueError(f"invalid or duplicate boss behavior review ID: {entry_id!r}")
        seen_ids.add(entry_id)

        monster_id = review.get("monster_id")
        monster = monsters_by_id.get(monster_id)
        if monster is None:
            raise ValueError(f"boss behavior review {entry_id} references unknown monster_id {monster_id}")
        if monster["sprite_name"] != review.get("sprite_name"):
            raise ValueError(f"boss behavior review {entry_id} sprite_name mismatch: {review.get('sprite_name')!r} != {monster['sprite_name']!r}")

        for source in review.get("sources", []):
            if source.get("path") != "db/re/mob_skill_db.conf":
                raise ValueError(f"boss behavior review {entry_id} cites an unexpected source path: {source.get('path')}")
            for line_number in source.get("lines", []):
                if not isinstance(line_number, int) or line_number < 1 or line_number > len(mob_skill_lines):
                    raise ValueError(f"boss behavior review {entry_id} cites missing source line {line_number}")
            for literal in source.get("required_source_literals", []):
                if literal not in mob_skill_text:
                    raise ValueError(f"boss behavior review {entry_id} source anchor no longer matches: {literal!r}")
        if not review.get("sources"):
            raise ValueError(f"boss behavior review {entry_id} has no source references")

        for summon in review.get("summons", []):
            candidates = summon.get("summon_candidates", [])
            names = summon.get("summon_candidate_names", [])
            if not candidates:
                raise ValueError(f"boss behavior review {entry_id} has an empty summon candidate list")
            for candidate_id in candidates:
                if candidate_id not in monsters_by_id:
                    raise ValueError(f"boss behavior review {entry_id} summon references unknown monster id {candidate_id}")
            if len(names) != len(candidates):
                raise ValueError(f"boss behavior review {entry_id} summon_candidate_names length does not match summon_candidates")
            for line_number in summon.get("source_lines", []):
                if not isinstance(line_number, int) or line_number < 1 or line_number > len(mob_skill_lines):
                    raise ValueError(f"boss behavior review {entry_id} summon cites missing source line {line_number}")

        for behavior in review.get("hp_threshold_behaviors", []):
            skill_name = behavior.get("skill")
            if skill_name not in skill_names:
                raise ValueError(f"boss behavior review {entry_id} references unknown skill {skill_name!r}")
            threshold = behavior.get("threshold_pct")
            if not isinstance(threshold, int) or not (0 <= threshold <= 100):
                raise ValueError(f"boss behavior review {entry_id} has an invalid threshold_pct for skill {skill_name!r}")
            line_number = behavior.get("source_line")
            if not isinstance(line_number, int) or line_number < 1 or line_number > len(mob_skill_lines):
                raise ValueError(f"boss behavior review {entry_id} HP-threshold behavior cites missing source line {line_number}")

        entry = {
            "id": entry_id,
            "monster_id": monster_id,
            "sprite_name": review["sprite_name"],
            "monster_name": monster.get("name"),
            "title": review["title"],
            "evidence_state": review.get("evidence_state", "verified"),
            "summons": [
                {**summon, "summon_candidates": [
                    {"monster_id": mid, "name": monsters_by_id[mid]["name"]} for mid in summon["summon_candidates"]
                ]}
                for summon in review.get("summons", [])
            ],
            "hp_threshold_behaviors": review.get("hp_threshold_behaviors", []),
            "conditions": review.get("conditions", []),
            "sources": review["sources"],
            "reviewed_by": review.get("reviewed_by") or review_data["reviewed_by"],
            "reviewed_on": review.get("reviewed_on") or review_data["reviewed_on"],
            "review_method": review.get("review_method") or review_data["review_method"],
        }
        # Drop the now-redundant candidate-name list; the resolved objects above carry it.
        for summon in entry["summons"]:
            summon.pop("summon_candidate_names", None)
        entries.append(entry)

    entries.sort(key=lambda entry: entry["monster_id"])
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
        print(f"boss behavior review export failed: {error}", file=sys.stderr)
        return 1
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {len(json.loads(rendered)['entries'])} reviewed bosses")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(json.loads(rendered)['entries'])} reviewed bosses)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
