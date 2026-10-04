"""Tests for drop_sim.py: the arithmetic must match mob_dropitem, and the tool
must refuse rather than print odds for a configuration it does not model."""

import random
import unittest

import drop_sim


class EffectiveRate(unittest.TestCase):
    def test_a_stock_player_gets_the_database_rate(self):
        self.assertEqual(drop_sim.effective_rate(7000, 100, 9000), 7000)

    def test_the_level_modifier_scales_the_rate_with_integer_math(self):
        self.assertEqual(drop_sim.effective_rate(7000, 90, 9000), 6300)
        self.assertEqual(drop_sim.effective_rate(1234, 70, 9000), 863)  # 1234 * 70 // 100

    def test_a_rate_never_drops_below_one(self):
        self.assertEqual(drop_sim.effective_rate(1, 50, 9000), 1)

    def test_a_rate_is_capped_at_the_larger_of_its_base_and_the_threshold(self):
        self.assertEqual(drop_sim.effective_rate(5000, 200, 9000), 9000)
        self.assertEqual(drop_sim.effective_rate(9500, 200, 9000), 9500)


class LevelModifier(unittest.TestCase):
    TABLE = {"ITEM_DROP_PENALTY_RATE": {"RC_NonBoss": {-4: 90, 0: 100, 4: 90}, "RC_Boss": {0: 100}}}

    def test_only_an_exact_difference_row_applies(self):
        self.assertEqual(drop_sim.level_modifier(self.TABLE, False, 1, 5), 90)   # diff -4
        self.assertEqual(drop_sim.level_modifier(self.TABLE, False, 1, 6), 100)  # diff -5 has no row

    def test_a_boss_falls_back_to_the_non_boss_row(self):
        # pc_level_penalty_mod: a boss reads RC_Boss first and, when that row
        # is empty at the difference, carries on to RC_NonBoss. The old
        # expectation here (100) encoded "bosses use only their own table",
        # which the server never did.
        self.assertEqual(drop_sim.level_modifier(self.TABLE, True, 1, 5), 90)   # diff -4: no boss row
        self.assertEqual(drop_sim.level_modifier(self.TABLE, True, 1, 1), 100)  # diff 0: the boss row
        self.assertEqual(drop_sim.level_modifier(self.TABLE, True, 1, 6), 100)  # diff -5: neither

    def test_a_boss_row_wins_where_it_exists(self):
        table = {"ITEM_DROP_PENALTY_RATE": {"RC_NonBoss": {4: 90}, "RC_Boss": {4: 60}}}
        self.assertEqual(drop_sim.level_modifier(table, True, 5, 1), 60)
        self.assertEqual(drop_sim.level_modifier(table, False, 5, 1), 90)

    def test_the_real_table_matches_the_server_rule(self):
        table = drop_sim.rules.parse_level_penalty(drop_sim.LEVEL_PENALTY)
        self.assertEqual(drop_sim.level_modifier(table, False, 10, 20), 70)  # diff -10
        self.assertEqual(drop_sim.level_modifier(table, False, 10, 15), 100)  # diff -5, a gap
        self.assertEqual(drop_sim.level_modifier(table, True, 10, 20), 70)  # a boss at diff -10


class Probability(unittest.TestCase):
    def test_one_in_a_hundred_over_a_hundred_kills(self):
        self.assertAlmostEqual(drop_sim.chance_at_least_one(100, 100), 0.634, places=3)

    def test_a_certain_drop_comes_on_the_first_kill(self):
        self.assertEqual(drop_sim.kills_until_first(10000, random.Random(1)), 1)

    def test_the_median_is_near_the_expected_count(self):
        rng = random.Random(7)
        samples = sorted(drop_sim.kills_until_first(500, rng) for _ in range(4000))
        median = drop_sim.percentile(samples, 0.5)
        self.assertTrue(10 <= median <= 17, median)  # expected 20 kills, median ~ 0.69 * 20


class Refusal(unittest.TestCase):
    def stock(self):
        return {key: {"value": value} for key, value in drop_sim.STOCK.items()}

    def test_a_stock_configuration_is_accepted(self):
        self.assertEqual(drop_sim.unmodelled_settings(self.stock()), [])

    def test_a_changed_type_rate_is_named_and_refused(self):
        settings = self.stock()
        settings["item_rate_card"] = {"value": 200}
        problems = drop_sim.unmodelled_settings(settings)
        self.assertEqual(len(problems), 1)
        self.assertIn("item_rate_card", problems[0])

    def test_a_missing_setting_is_an_error_not_a_guess(self):
        settings = self.stock()
        del settings["drops_by_luk"]
        with self.assertRaises(drop_sim.SimError):
            drop_sim.unmodelled_settings(settings)

    def test_this_servers_real_configuration_is_modelled(self):
        settings = drop_sim.rules.effective_settings(drop_sim.HERCULES / "conf/map/battle.conf")
        self.assertEqual(drop_sim.unmodelled_settings(settings), [])


class Entries(unittest.TestCase):
    def test_plain_and_grouped_drop_entries_both_give_a_rate(self):
        self.assertEqual(drop_sim.drop_entries({"A": 100, "B": (250, "group")}), [("A", 100), ("B", 250)])

    def test_no_drops_is_empty(self):
        self.assertEqual(drop_sim.drop_entries(None), [])


class EndToEnd(unittest.TestCase):
    def test_poring_at_level_five_matches_the_hand_calculation(self):
        monsters = drop_sim.load_monsters()
        result = drop_sim.simulate(drop_sim.find_monster(monsters, "PORING"), 5, 100, 200, 1)
        jellopy = next(drop for drop in result["drops"] if drop["item"] == "Jellopy")
        self.assertEqual((result["level_modifier_percent"], jellopy["database_rate"], jellopy["effective_rate"]), (90, 7000, 6300))

    def test_an_unknown_monster_is_an_error(self):
        with self.assertRaises(drop_sim.SimError):
            drop_sim.find_monster(drop_sim.load_monsters(), "NO_SUCH_MONSTER")


if __name__ == "__main__":
    unittest.main()
