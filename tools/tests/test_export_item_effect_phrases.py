import unittest

from export_item_reference import translate_simple_effect


class ItemEffectPhraseTests(unittest.TestCase):
    def test_bare_itemskill_stays_a_grant(self):
        self.assertEqual(
            translate_simple_effect("itemskill AL_TELEPORT,1;"),
            "grants access to Teleport at level 1",
        )

    def test_itemskill_flags_use_the_hercules_enum_notes(self):
        text = translate_simple_effect(
            "itemskill(AL_BLESSING, 10, ISF_INSTANTCAST | ISF_CASTONSELF);"
        )
        self.assertIn("Blessing", text)
        self.assertIn("level 10", text)
        self.assertIn("instantly", text)
        self.assertIn("on yourself, without a target cursor", text)
        self.assertNotIn("checking the skill's conditions", text)

    def test_sc_start_flag_uses_the_documented_rate(self):
        self.assertEqual(
            translate_simple_effect("sc_start SC_FREEZE,10000,0,2500,SCFLAG_NONE;"),
            "Applies Freeze for 10 seconds at 25% chance",
        )
        self.assertEqual(
            translate_simple_effect("sc_start SC_ATTHASTE_POTION1,1800000,4;"),
            "Applies status SC_ATTHASTE_POTION1 for 1800 seconds, value 4 (no explicit chance limit)",
        )
        self.assertIsNone(translate_simple_effect("sc_start SC_FREEZE,10000,0,2500,SCFLAG_NONE,1;"))

    def test_unknown_itemskill_flag_stays_untranslated(self):
        self.assertIsNone(translate_simple_effect("itemskill(AL_BLESSING, 10, ISF_NOT_A_FLAG);"))

    def test_parenthesized_percentheal_and_status_apply(self):
        text = translate_simple_effect(
            "percentheal(0, 5); itemskill(AL_BLESSING, 5, ISF_INSTANTCAST | ISF_CASTONSELF);"
        )
        self.assertIn("restores 5% SP", text)
        self.assertIn("Blessing", text)

    def test_documented_bonuses_and_item_grants(self):
        self.assertEqual(
            translate_simple_effect("bonus bUnbreakableHelm,1;"),
            "the equipped helm cannot be broken",
        )
        self.assertEqual(
            translate_simple_effect("bonus bBaseAtk,5; bonus bSpeedRate,10;"),
            "increases basic attack power by 5; increases movement speed by 10% (only the highest bonus applies)",
        )
        self.assertEqual(translate_simple_effect("getitem 501,10;"), "grants 10 Red Potion")
        self.assertEqual(
            translate_simple_effect("packageitem();"),
            "grants this item's package contents",
        )
        self.assertIn("Heal", translate_simple_effect("skill AL_HEAL,1;"))

    def test_map_gated_itemskill_stays_untranslated(self):
        self.assertIsNone(
            translate_simple_effect('if(strcharinfo(PC_MAP)=="job3_war02") { itemskill WL_FROSTMISTY,5; }')
        )


if __name__ == "__main__":
    unittest.main()
