import copy
import json
import re
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import encyclopedia_queue
from encyclopedia_checks import check_draft, check_sources
from encyclopedia_common import TEMPLATES, block_at, read_json
from encyclopedia_packets import claims_of, tool_draft
from encyclopedia_unit import e0_checks

SHOPS = "npc/re/merchants/shops.txt"
AIRSHIPS = "npc/airports/airships.txt"


def template_json(kind: str) -> dict:
    text = TEMPLATES.read_text(encoding="utf-8")
    section = re.search(rf"<!-- kind: {kind} -->\n(.*?)<!-- end -->", text, re.S).group(1)
    blocks = re.findall(r"```json\n(.*?)\n```", section, re.S)
    return json.loads(blocks[-1])


def unit_for(kind: str, scope: list, subject: dict, unit_id: str = "T-unit") -> dict:
    return {"id": unit_id, "package": unit_id.split("-")[0], "kind": kind, "scope": scope, "subject": subject}


def meltz_unit() -> dict:
    block = block_at(AIRSHIPS, 949)
    return unit_for(
        "e1_exchange",
        [{"path": AIRSHIPS, "start": block["start"], "end": block["end"]}],
        {"npc": "Apple Merchant#air01", "map": "airplane_01", "x": 50, "y": 66, "path": AIRSHIPS, "line": block["start"]},
        "E1-test",
    )


def meltz_review() -> dict:
    entry = next(e for e in read_json(Path(__file__).resolve().parents[1] / "item_exchange_reviews.json")["entries"]
                 if e["id"] == "airplane_01_meltz_apple_juice")
    review = copy.deepcopy(entry)
    review["id"] = "test_meltz_copy"
    return review


class SourceCheckTests(unittest.TestCase):
    def test_literal_must_be_on_a_cited_line(self):
        sources = [{"path": SHOPS, "lines": [808], "required_source_literals": ["sellitem Arrow;"]}]
        errors = check_sources("x", sources)
        self.assertTrue(any("literal not found on any cited line" in e for e in errors))

    def test_every_cited_line_needs_a_quote(self):
        sources = [{"path": SHOPS, "lines": [808, 809], "required_source_literals": ["sellitem Arrow;"]}]
        self.assertTrue(any("no literal quotes it" in e for e in check_sources("x", sources)))

    def test_citation_outside_the_packet_is_refused(self):
        sources = [{"path": SHOPS, "lines": [809], "required_source_literals": ["sellitem Arrow;"]}]
        scope = [{"path": SHOPS, "start": 900, "end": 950}]
        self.assertTrue(any("outside this unit's packet" in e for e in check_sources("x", sources, scope)))

    def test_unloaded_or_invented_path_is_refused(self):
        sources = [{"path": "npc/made_up.txt", "lines": [1], "required_source_literals": ["x"]}]
        self.assertTrue(any("not a loaded script" in e for e in check_sources("x", sources)))

    def test_exact_quote_on_its_line_passes(self):
        sources = [{"path": SHOPS, "lines": [809], "required_source_literals": ["sellitem Arrow;"]}]
        self.assertEqual(check_sources("x", sources), [])


class ExchangeReviewTests(unittest.TestCase):
    def draft(self, review: dict) -> dict:
        return {"unit": "E1-test", "reviews": [review], "dispositions": []}

    def errors(self, review: dict) -> list:
        # Coverage is checked separately; the Meltz NPC has other branches.
        with mock.patch("encyclopedia_checks.uncovered", return_value=[]):
            return check_draft(self.draft(review), meltz_unit())

    def test_accepted_example_passes(self):
        self.assertEqual(self.errors(meltz_review()), [])

    def test_wrong_input_amount_is_caught(self):
        review = meltz_review()
        review["inputs"][0]["amount"] = 5
        self.assertTrue(any("but the quoted delitem uses 3" in e for e in self.errors(review)))

    def test_dropped_cost_is_caught(self):
        review = meltz_review()
        review["inputs"] = review["inputs"][:1]
        self.assertTrue(any("is not listed in `inputs`" in e for e in self.errors(review)))

    def test_invented_reward_is_caught(self):
        review = meltz_review()
        review["outcomes"][0]["item_ids"] = [501]
        self.assertTrue(any("has no quoted `getitem` literal" in e for e in self.errors(review)))

    def test_npc_must_match_the_header(self):
        review = meltz_review()
        review["npc"]["x"] = 51
        self.assertTrue(any("npc.x must be 50" in e for e in self.errors(review)))

    def test_evidence_state_is_left_to_the_exporter(self):
        review = meltz_review()
        review["evidence_state"] = "verified"
        self.assertTrue(any("remove `evidence_state`" in e for e in self.errors(review)))


class CoverageTests(unittest.TestCase):
    def test_uncited_item_call_is_reported(self):
        unit = meltz_unit()
        with mock.patch("encyclopedia_checks.existing_records", return_value=[]):
            errors = check_draft({"unit": "E1-test", "reviews": [], "dispositions": []}, unit)
            self.assertTrue(any(e.startswith("coverage:") for e in errors))

    def test_formula_unit_must_account_for_every_skill(self):
        unit = unit_for("e4_formula", [{"path": "src/map/battle.c", "start": 1, "end": 2}], {"skill_ids": [1, 2], "skill_names": []}, "E4-test")
        from encyclopedia_checks import uncovered
        self.assertTrue(uncovered({"reviews": [{"skill_ids": [1]}], "dispositions": []}, unit))


class TemplateFixtureTests(unittest.TestCase):
    """The examples in the templates file are what a small model copies. They must pass."""

    def test_stock_disposition_example_validates(self):
        record = template_json("e2_stock")
        record["unit"] = "E2-test"
        block = block_at(SHOPS, 807)
        unit = unit_for("e2_stock", [{"path": SHOPS, "start": block["start"], "end": block["end"]}],
                        {"declarations": [{"name": "Tool Dealer#alb", "path": SHOPS, "line": 807}]}, "E2-test")
        self.assertEqual(check_draft({"unit": "E2-test", "reviews": [], "dispositions": [record]}, unit), [])

    def test_boss_example_validates(self):
        review = template_json("e6_boss")
        unit = unit_for("e6_boss", [{"path": "db/re/mob_skill_db.conf", "start": 24806, "end": 24830}],
                        {"monster_id": 1511, "sprite_name": "AMON_RA"}, "E6-test")
        errors = check_draft({"unit": "E6-test", "reviews": [review], "dispositions": []}, unit)
        self.assertEqual([e for e in errors if "already exists" not in e], [])


class ToolDraftTests(unittest.TestCase):
    def test_untranslated_scripts_are_recorded_as_unknown_not_reviewed(self):
        unit = next(u for u in encyclopedia_queue.e3_units() if u["kind"] == "e3_dispose")
        draft = tool_draft(unit, "test")
        self.assertTrue(draft["dispositions"])
        self.assertEqual({d["result"] for d in draft["dispositions"]}, {"unknown"})
        errors = check_draft(draft, unit)
        self.assertEqual([e for e in errors if "already exists" not in e], [])

    def test_claims_spell_out_items_for_the_verifier(self):
        claims = claims_of({"reviews": [meltz_review()], "dispositions": []})
        self.assertIn("The player gives 3 x Apple (item 512).", claims)
        self.assertTrue(any(c.startswith("The player receives 1 x Apple Juice") for c in claims))


class QueueTests(unittest.TestCase):
    def test_rebuild_keeps_progress_of_existing_units(self):
        fake = [encyclopedia_queue.unit("E4-a", "E4", "e4_formula", {"skill_ids": [1]}, [], "r", tier="local")]
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "queue.json"
            path.write_text(json.dumps({"schema_version": 1, "units": [dict(fake[0], state="done", attempts=1, history=["h"])]}))
            with mock.patch.object(encyclopedia_queue, "QUEUE", path), \
                    mock.patch.dict(encyclopedia_queue.BUILDERS, {"E4": lambda: copy.deepcopy(fake)}):
                queue = encyclopedia_queue.rebuild(["E4"])
        self.assertEqual(queue["units"][0]["state"], "done")
        self.assertEqual(queue["units"][0]["history"], ["h"])

    def test_live_pass_stays_blocked(self):
        self.assertEqual(encyclopedia_queue.e7_units()[0]["state"], "blocked")


class EvidenceLabelTests(unittest.TestCase):
    def test_e0_rules_hold_on_the_current_exports(self):
        failed = [text for ok, text in e0_checks() if not ok]
        self.assertEqual(failed, [])


if __name__ == "__main__":
    unittest.main()
