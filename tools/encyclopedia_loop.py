#!/usr/bin/env python3
"""Run the source-proof encyclopedia loop.

The only automatic phase is the single-branch item exchange prover. It stops
when every loaded NPC branch has either been accepted or rejected by that
proof rule. Quest flows, skill formulas, and the deferred live client pass
are reported and left open. This script does not mark the encyclopedia complete.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPORT = ROOT / "docs/plans/encyclopedia-loop-report.json"
PYTHON = sys.executable


def run(script: str, *args: str) -> None:
    completed = subprocess.run([PYTHON, str(ROOT / "tools" / script), *args], check=False)
    if completed.returncode != 0:
        raise SystemExit(f"{script} failed with status {completed.returncode}")


def main() -> int:
    run("review_simple_exchanges.py", "--write")
    run("export_item_exchange_reviews.py")
    run("export_encyclopedia_coverage.py")
    exchanges = json.loads((ROOT / "docs/item-exchanges.v1.json").read_text(encoding="utf-8"))
    coverage = json.loads((ROOT / "docs/encyclopedia-coverage.v1.json").read_text(encoding="utf-8"))
    report = {
        "loop": "encyclopedia_loop.py",
        "automatic_phase": "single-branch delitem plus getitem with braced conditions and no skip between the calls",
        "reviewed_exchanges": len(exchanges["entries"]),
        "quests_with_reviewed_flow": coverage["categories"]["quests"]["quests_with_reviewed_flow"],
        "live_client_pass": "deferred until the source reviews that this loop cannot prove are done",
        "not_completed_by_this_loop": [
            "NPC branches with menus, switches, braceless if/else, calls, or more than one item on either side",
            "Quest flows that are not already in tools/quest_flow_reviews.json",
            "Skill formulas beyond tools/skill_formula_reviews.json",
            "Fresh-account Adventure Guide journeys",
        ],
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
