import tempfile
import unittest
from pathlib import Path

from extend_bestiary_export import apply_skill_layers, parse_mob_skill_db

STOCK = """mob_skill_db:(
{
\tEDDGA: {
\t\tSM_MAGNUM: {
\t\t\tSkillLevel: 5
\t\t}
\t\tNPC_POWERUP: {
\t\t\tSkillLevel: 1
\t\t}
\t}
\tPORING: {
\t\tNPC_EMOTION: {
\t\t\tSkillLevel: 1
\t\t}
\t}
}
)
"""

PILOT = """mob_skill_db:(
{
\tEDDGA: {
\t\tNPC_EMOTION: {
\t\t\tClearSkills: true
\t\t}
\t\tWZ_METEOR: {
\t\t\tSkillLevel: 10
\t\t}
\t}
}
)
"""


class MobSkillLayerTests(unittest.TestCase):
    def parse(self, text: str) -> dict:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "skills.conf"
            path.write_text(text, encoding="utf-8")
            return parse_mob_skill_db(include_triggers=True, path=path)

    def test_clear_skills_is_a_reset_marker_not_a_skill(self):
        pilot = self.parse(PILOT)

        self.assertEqual(pilot["EDDGA"][0], {"ClearSkills": True})
        self.assertEqual(pilot["EDDGA"][1]["Skill"], "WZ_METEOR")

    def test_a_later_layer_clears_then_replaces_only_its_own_monsters(self):
        effective = apply_skill_layers([self.parse(STOCK), self.parse(PILOT)])

        self.assertEqual([row["Skill"] for row in effective["EDDGA"]], ["WZ_METEOR"])
        self.assertEqual([row["Skill"] for row in effective["PORING"]], ["NPC_EMOTION"])

    def test_without_the_pilot_layer_the_stock_skills_stand(self):
        effective = apply_skill_layers([self.parse(STOCK)])

        self.assertEqual([row["Skill"] for row in effective["EDDGA"]], ["SM_MAGNUM", "NPC_POWERUP"])


if __name__ == "__main__":
    unittest.main()
