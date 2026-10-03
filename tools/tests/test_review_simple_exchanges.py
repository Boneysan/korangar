import unittest

from review_simple_exchanges import body_uses_unbraced_branches, branches, proven_exchange, refusal_reason


HEDRICK = """
if(countitem(Mother_Letter)==0 || tu_acolyte01 != 19) close;
delitem Mother_Letter,1;
tu_acolyte01 = 20;
getitem Well_Baked_Cookie,1;
""".strip("\n").splitlines()

ZOOLOGIST = """
if (countitem(Disin_Delivery_Box) > 0) {
delitem Disin_Delivery_Box,1;
erasequest 8271;
getitem Novice_Potion,100;
}
""".strip("\n").splitlines()

SEPARATE = """
if (dew_drink == 7) {
delitem Palm_O,30;
}
if (dew_drink == 8) {
getitem Old_Violet_Box,1;
}
""".strip("\n").splitlines()

SKIPPED = """
if (ready) {
delitem Apple,1;
if (countitem(Empty_Bottle) == 0) close;
getitem Apple_Juice,1;
}
""".strip("\n").splitlines()


class SimpleExchangeTests(unittest.TestCase):
    def test_hedrick_is_one_exchange(self):
        found = branches(HEDRICK)
        self.assertEqual(len(found), 1)
        self.assertIsNotNone(proven_exchange(found[0]["calls"], HEDRICK))

    def test_zoologist_branch_is_one_exchange(self):
        proven = [proven_exchange(branch["calls"], ZOOLOGIST) for branch in branches(ZOOLOGIST)]
        self.assertEqual(sum(item is not None for item in proven), 1)

    def test_different_stages_are_not_one_exchange(self):
        proven = [proven_exchange(branch["calls"], SEPARATE) for branch in branches(SEPARATE)]
        self.assertTrue(all(item is None for item in proven))

    def test_refusal_reasons_follow_the_prover_rules(self):
        self.assertEqual(refusal_reason(["delitem Hammer,1;", "if (.@lv > 48)", "getitem Steel,30;", "else", "getitem Steel,5;"]), "braceless")
        self.assertEqual(refusal_reason(["switch(select(\"No.\", \"Yes.\")) {", "delitem Apple,1;", "getitem Juice,1;", "}"]), "switch")
        self.assertEqual(refusal_reason(["delitem Apple,1;", "getitem Juice,1;"]), "multi-item")

    def test_braceless_else_is_not_a_proven_body(self):
        body = ["delitem Hammer,1;", "if (.@joblvl > 48)", "getitem Steel,30;", "else", "getitem Steel,5;"]
        self.assertTrue(body_uses_unbraced_branches(body))

    def test_a_same_line_close_guard_is_allowed(self):
        self.assertFalse(body_uses_unbraced_branches(["if(countitem(Mother_Letter)==0) close;"]))

    def test_a_close_between_the_calls_is_rejected(self):
        proven = [proven_exchange(branch["calls"], SKIPPED) for branch in branches(SKIPPED)]
        self.assertTrue(all(item is None for item in proven))


if __name__ == "__main__":
    unittest.main()
