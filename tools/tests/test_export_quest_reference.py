import unittest

from export_quest_reference import extract_npc_quest_references


class QuestNpcReferenceTests(unittest.TestCase):
    def test_maps_literal_quest_calls_to_nearest_static_npc_and_coordinates(self):
        script = """prontera,100,200,4\tscript\tFirst NPC\t4_F_KAFRA,{
    setquest(1001);
    if (questprogress(1001) == 1) completequest(1001);
}
prontera,300,400,4\tscript\tSecond NPC\t4_M_01,{
    setquest 1002;
}
setquest(1001);
"""

        refs = extract_npc_quest_references(script, "npc/re/quests/test.txt", {1001, 1002})

        self.assertEqual(len(refs[1001]), 1)
        first = refs[1001][0]
        self.assertEqual((first["name"], first["map_name"], first["x"], first["y"]), ("First", "prontera", 100, 200))
        self.assertEqual(first["source_line"], 2)
        self.assertEqual(first["uses"], ["setquest", "questprogress", "completequest"])
        self.assertEqual((refs[1002][0]["name"], refs[1002][0]["source_line"]), ("Second", 6))

    def test_does_not_assign_calls_outside_npc_blocks(self):
        script = """prontera,10,20,4\tscript\tFirst NPC\t4_F_KAFRA,{
    end;
}
function\tscript\tSomeFunction\t{
    setquest(1001);
    return;
}
"""

        self.assertEqual(extract_npc_quest_references(script, "test.txt", {1001}), {})


if __name__ == "__main__":
    unittest.main()
