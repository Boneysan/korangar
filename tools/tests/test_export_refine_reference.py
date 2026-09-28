import unittest

from export_refine_reference import MAX_USEFUL_REFINE_LEVEL, parse_weapon_level_chances


def sample_refine_db(weapon_level_4_rates: str = "Rates: {}") -> str:
    """A minimal but structurally real refine_db.conf: four WeaponLevel
    blocks, each with a RefineryUISettings array (to prove the parser does
    not mistake its `Level: [..]` entries for `Rates.Lv{N}` ones) followed by
    a Rates block. Only WeaponLevel4 varies per test, to keep each test
    focused on one block without repeating all four every time.
    """
    return f"""
WeaponLevel1: {{
    StatsPerLevel: 200
    RefineryUISettings: (
        {{
            Level: [1, 7]
            Items: {{
                Phracon: {{ Type: "REFINE_CHANCE_TYPE_NORMAL" Cost: 50 }}
            }}
        }},
    )
    Rates: {{
        Lv8: {{
            NormalChance: 60
            EnrichedChance: 90
        }}
    }}
}}
WeaponLevel2: {{
    Rates: {{
        Lv9: {{ NormalChance: 20 }}
    }}
}}
WeaponLevel3: {{
    Rates: {{
    }}
}}
WeaponLevel4: {{
{weapon_level_4_rates}
}}
"""


class RefineReferenceExportTests(unittest.TestCase):
    def test_unlisted_levels_default_to_100_percent(self):
        chances = parse_weapon_level_chances(sample_refine_db("Rates: {}"))

        # refine_db.conf's own documented convention: "Refine levels that use
        # default values need not be listed."
        self.assertEqual(chances[4], {level: 100 for level in range(1, MAX_USEFUL_REFINE_LEVEL + 1)})

    def test_listed_levels_override_the_default(self):
        chances = parse_weapon_level_chances(sample_refine_db())

        self.assertEqual(chances[1][8], 60)
        self.assertEqual(chances[2][9], 20)
        # Everything else in weapon level 1 still defaults, including levels
        # both below and above the one explicit override.
        self.assertEqual(chances[1][7], 100)
        self.assertEqual(chances[1][9], 100)

    def test_refinery_ui_settings_level_field_is_not_mistaken_for_a_rate(self):
        # WeaponLevel1's RefineryUISettings has `Level: [1, 7]` before the
        # Rates block. If the parser searched the whole block for `Lv\d+:`
        # instead of scoping to the Rates sub-block, it could misread "7" from
        # that array as a rate-bearing level and silently invent a Lv7 entry.
        chances = parse_weapon_level_chances(sample_refine_db())

        self.assertEqual(chances[1][7], 100)

    def test_comments_do_not_leak_into_parsed_values(self):
        chances = parse_weapon_level_chances(
            sample_refine_db(
                """
    Rates: {
        Lv10: {
            NormalChance: 9 // was 19 before a balance pass
        }
    }
    """
            )
        )

        self.assertEqual(chances[4][10], 9)

    def test_levels_past_the_useful_cap_are_not_exported(self):
        # skill_weaponrefine never lets the caster go past +10 regardless of
        # skill level, so a Lv11+ entry (real for armor upgrades, which this
        # exporter does not cover) must not appear in the result at all.
        chances = parse_weapon_level_chances(
            sample_refine_db(
                """
    Rates: {
        Lv11: { NormalChance: 18 }
    }
    """
            )
        )

        self.assertNotIn(11, chances[4])
        self.assertEqual(len(chances[4]), MAX_USEFUL_REFINE_LEVEL)

    def test_missing_weapon_level_block_raises(self):
        with self.assertRaises(ValueError):
            parse_weapon_level_chances("WeaponLevel1: { Rates: {} }")

    def test_missing_rates_block_raises(self):
        with self.assertRaises(ValueError):
            parse_weapon_level_chances(
                "WeaponLevel1: {}\nWeaponLevel2: { Rates: {} }\nWeaponLevel3: { Rates: {} }\nWeaponLevel4: { Rates: {} }"
            )


if __name__ == "__main__":
    unittest.main()
