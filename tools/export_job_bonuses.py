#!/usr/bin/env python3
"""Export verified per-job-level stat bonuses from Hercules' job_db2.txt."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from export_job_skills import CONSTANTS, HERCULES, job_ids, source_revision

ROOT = Path(__file__).resolve().parent.parent
BONUS_DB = HERCULES / "db/job_db2.txt"
OUTPUT = ROOT / "docs/job-bonuses.v1.json"
STATS = {1: "STR", 2: "AGI", 3: "VIT", 4: "INT", 5: "DEX", 6: "LUK"}


def parse_job_bonuses(text: str, known_job_ids: set[int]) -> list[dict[str, object]]:
    entries = []
    seen: set[int] = set()
    for line_number, source_line in enumerate(text.splitlines(), 1):
        line = source_line.split("//", 1)[0].strip()
        if not line:
            continue
        try:
            fields = [int(field.strip()) for field in line.split(",")]
        except ValueError as error:
            raise ValueError(f"line {line_number}: expected comma-separated integers") from error
        job_id, *bonuses = fields
        if job_id not in known_job_ids:
            raise ValueError(f"line {line_number}: job ID {job_id} has no Job_ constant")
        if job_id in seen:
            raise ValueError(f"line {line_number}: duplicate job bonus row for {job_id}")
        if not bonuses or len(bonuses) > 99:
            raise ValueError(f"line {line_number}: invalid job-level bonus schedule length {len(bonuses)}")
        if any(bonus not in range(7) for bonus in bonuses):
            raise ValueError(f"line {line_number}: stat bonus code must be between 0 and 6")

        totals = {stat: 0 for stat in STATS.values()}
        milestones = []
        for level, bonus in enumerate(bonuses, 1):
            if bonus == 0:
                continue
            stat = STATS[bonus]
            totals[stat] += 1
            milestones.append({"job_level": level, "stat": stat})
        entries.append({
            "job_id": job_id,
            "max_job_level": len(bonuses),
            "total_bonuses": {stat: value for stat, value in totals.items() if value},
            "bonus_levels": milestones,
        })
        seen.add(job_id)
    if not entries:
        raise ValueError("no job bonus records found")
    entries.sort(key=lambda row: int(row["job_id"]))
    return entries


def build() -> dict[str, object]:
    constants = job_ids(CONSTANTS.read_text(encoding="utf-8", errors="replace"))
    entries = parse_job_bonuses(BONUS_DB.read_text(encoding="utf-8", errors="replace"), set(constants.values()))
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "source": "db/job_db2.txt",
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check generated-data drift")
    args = parser.parse_args()
    if not CONSTANTS.exists() or not BONUS_DB.exists():
        raise SystemExit(f"required Hercules source missing: {BONUS_DB}")
    try:
        data = build()
        rendered = json.dumps(data, indent=1, ensure_ascii=False) + "\n"
    except ValueError as error:
        raise SystemExit(f"cannot export job bonuses: {error}") from error
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != rendered:
            raise SystemExit(f"{OUTPUT.relative_to(ROOT)} is stale — re-run {Path(__file__).name}")
        print(f"up to date: {OUTPUT.relative_to(ROOT)}")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(data['entries'])} jobs)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
