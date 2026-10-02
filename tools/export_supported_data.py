#!/usr/bin/env python3
"""Run the currently reproducible Hercules-backed exporters in order.

This is intentionally named "supported": it generates versioned parallel
reference data, but it does not regenerate the legacy bestiary/items/cards
files. See docs/specs/encyclopedia-data.md for the G1 contract and remaining
coverage gaps.

Usage:
    tools/export_supported_data.py [--check]

`--check` runs each exporter in its non-mutating drift-check mode.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
EXPORTERS = (
    "export_skill_info.py",
    "export_job_names.py",
    "export_job_skills.py",
    "export_job_bonuses.py",
    "export_refine_reference.py",
    "export_server_rules.py",
    "export_status_names.py",
    "export_status_reference.py",
    "extend_bestiary_export.py",
    "export_item_reference.py",
    "export_crafting_reference.py",
    "export_item_grant_reference.py",
    "export_npc_reference.py",
    "export_item_exchange_reviews.py",
    "export_npc_service_clues.py",
    "export_npc_service_reviews.py",
    "export_map_flags.py",
    "export_bestiary_reference.py",
    "export_spawn_rectangles.py",
    "export_boss_behavior_reviews.py",
    "export_scripted_spawn_reviews.py",
    "export_quest_reference.py",
    "export_skill_formula_reviews.py",
    "export_encyclopedia_dispositions.py",
    "generate_navigation_graph.py",
    "export_rumors_reference.py",
    "export_encyclopedia_coverage.py",
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for drift without writing generated files")
    args = parser.parse_args()

    for name in EXPORTERS:
        command = [sys.executable, str(ROOT / "tools" / name)]
        if args.check:
            command.append("--check")
        print(f"==> {' '.join(command)}", flush=True)
        result = subprocess.run(command, cwd=ROOT, check=False)
        if result.returncode:
            print(f"failed: {name} (exit {result.returncode})", file=sys.stderr)
            return result.returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
