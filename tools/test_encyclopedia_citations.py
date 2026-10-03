"""A ratchet on the encyclopedia's citation quality.

`encyclopedia_unit.py citations` fails for 29 reviews written against line
numbers that have since moved (2026-10-02; see docs/plans/missing-files-sweep).
Fixing them is review work that needs a person, and until it is done the check
cannot simply be made to fail the build. This makes it a ratchet instead:

  * a review that is NOT in the baseline and has a broken citation FAILS, so no
    new drift enters unnoticed (a new review, or an old one whose source moved);
  * a review that IS in the baseline and now passes FAILS, until it is removed
    from `encyclopedia_citation_baseline.json`, so the list only ever shrinks.

    cd tools && python3 -m unittest test_encyclopedia_citations
"""

import json
import unittest
from pathlib import Path

import encyclopedia_checks as checks
import encyclopedia_unit as unit

BASELINE = Path(__file__).with_name("encyclopedia_citation_baseline.json")


def broken_reviews() -> set[str]:
    files = list(unit.REVIEW_FILES.values()) + ["tools/npc_service_reviews.json", "tools/map_runtime_flag_reviews.json"]
    broken = set()
    for name in files:
        for entry in unit.read_json(unit.ROOT / name)["entries"]:
            sources = entry.get("sources") or (
                [dict(entry["source"], required_source_literals=entry.get("required_source_literals"))] if "source" in entry else []
            )
            if sources and any("literal not found" in error for error in checks.check_sources(entry["id"], sources)):
                broken.add(f"{name}:{entry['id']}")
    return broken


class Citations(unittest.TestCase):
    def test_no_review_has_a_newly_broken_citation(self):
        baseline = set(json.loads(BASELINE.read_text(encoding="utf-8"))["broken"])
        new = sorted(broken_reviews() - baseline)
        self.assertEqual(new, [], f"these reviews quote text that is not on their cited lines and are not in the known baseline: {new}")

    def test_the_baseline_only_lists_reviews_that_are_still_broken(self):
        baseline = set(json.loads(BASELINE.read_text(encoding="utf-8"))["broken"])
        fixed = sorted(baseline - broken_reviews())
        self.assertEqual(fixed, [], f"these now pass; remove them from {BASELINE.name}: {fixed}")


if __name__ == "__main__":
    unittest.main()
