import tempfile
import unittest
from pathlib import Path

import json

from generate_navigation_graph import access_notes, parse_authored_services, parse_warps


class NavigationGraphTests(unittest.TestCase):
    def test_parses_static_warps_with_multiword_names_and_comments(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "npc" / "warps.txt"
            source.parent.mkdir()
            source.write_text(
                "louyang,129,121,0\twarp\tStorage Warp#1\t1,1,lou_in02,203,161 // exit\n",
                encoding="utf-8",
            )

            edges, unsupported, errors = parse_warps(root, [source], {"louyang", "lou_in02"})

        self.assertEqual(unsupported, [])
        self.assertEqual(errors, [])
        self.assertEqual(len(edges), 1)
        self.assertEqual(edges[0]["from"], {"map": "louyang", "x": 129, "y": 121, "width": 1, "height": 1})
        self.assertEqual(edges[0]["to"], {"map": "lou_in02", "x": 203, "y": 161})
        self.assertEqual(edges[0]["source"], "npc/warps.txt:1")

    def test_still_reports_unknown_static_warp_destinations(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "warps.txt"
            source.write_text("prontera,10,10,0\twarp\tTest Warp\t1,1,missing,20,20\n", encoding="utf-8")

            edges, unsupported, errors = parse_warps(root, [source], {"prontera"})

        self.assertEqual(edges, [])
        self.assertEqual(unsupported, [])
        self.assertEqual(len(errors), 1)
        self.assertIn("unknown map 'prontera' -> 'missing'", errors[0])

    def test_parses_reviewed_conditional_npc_service_edges(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "services.json"
            path.write_text(
                '{"services":[{"id":"service-test","availability":"conditional",'
                '"action":"Talk to the Sailor.","requirements":"150 zeny",'
                '"from":{"map":"izlude","x":197,"y":205},'
                '"to":{"map":"izlu2dun","x":107,"y":50},"source":"npc/test.txt:1"}]}',
                encoding="utf-8",
            )

            edges, errors = parse_authored_services(path, {"izlude", "izlu2dun"})

        self.assertEqual(errors, [])
        self.assertEqual(len(edges), 1)
        self.assertEqual(edges[0]["kind"], "npc_service")
        self.assertEqual(edges[0]["availability"], "conditional")
        self.assertEqual(edges[0]["requirements"], "150 zeny")

    def test_rejects_service_edges_with_unknown_maps(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "services.json"
            path.write_text(
                '{"services":[{"id":"service-test","availability":"always",'
                '"action":"Talk.","from":{"map":"missing","x":1,"y":1},'
                '"to":{"map":"prontera","x":2,"y":2},"source":"npc/test.txt:1"}]}',
                encoding="utf-8",
            )

            edges, errors = parse_authored_services(path, {"prontera"})

        self.assertEqual(edges, [])
        self.assertEqual(len(errors), 1)
        self.assertIn("unknown from map 'missing'", errors[0])


if __name__ == "__main__":
    unittest.main()


def edge(source: str, target: str) -> dict:
    return {"from": {"map": source}, "to": {"map": target}}


class AccessNoteTests(unittest.TestCase):
    def test_each_unreachable_map_gets_a_reason(self):
        edges = [edge("prontera", "izlude"), edge("gefenia01", "gefenia02"), edge("gld_dun01", "gld_dun02")]
        entrances = {"gefenia01": [{"npc": "Geffenia Warp", "npc_map": "geffen", "source": "a.txt:1"}]}

        notes = access_notes(edges, entrances)

        self.assertNotIn("izlude", notes)
        self.assertEqual(notes["gefenia01"]["kind"], "unreviewed_script_entrance")
        self.assertEqual(notes["gefenia02"]["kind"], "behind_unreviewed_entrance")
        self.assertEqual(notes["gefenia02"]["via"], "gefenia01")
        self.assertEqual(notes["gld_dun02"]["kind"], "no_known_entrance")


class ServiceLockTests(unittest.TestCase):
    def write(self, root: Path, service: dict) -> Path:
        path = root / "services.json"
        path.write_text(json.dumps({"services": [service]}), encoding="utf-8")
        return path

    def service(self, **extra) -> dict:
        return {
            "id": "service-a-b",
            "availability": "conditional",
            "action": "Talk.",
            "source": "a.txt:1",
            "from": {"map": "a", "x": 1, "y": 1},
            "to": {"map": "b", "x": 2, "y": 2},
            **extra,
        }

    def test_a_lock_with_steps_is_kept(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            path = self.write(Path(temp_dir), self.service(locked_by="Quest.", unlock_steps=["Do it."]))
            edges, errors = parse_authored_services(path, {"a", "b"})

        self.assertEqual(errors, [])
        self.assertEqual(edges[0]["unlock_steps"], ["Do it."])

    def test_a_lock_without_steps_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            path = self.write(Path(temp_dir), self.service(locked_by="Quest."))
            edges, errors = parse_authored_services(path, {"a", "b"})

        self.assertEqual(edges, [])
        self.assertIn("a lock needs both", errors[0])


class EntranceConditionTests(unittest.TestCase):
    SCRIPT = (
        "tha_t02,227,163,0\tscript\t3rdf_warp#tt\tWARPNPC,1,1,{\n"
        "OnTouch:\n"
        "\tif (checkhiding())\n"
        "\t\tend;\n"
        "\tif (thana_tower == 0) warp \"tha_t02\",227,158;\n"
        "\telse warp \"tha_t03\",219,159;\n"
        "}\n"
        "tha_t01,149,78,4\tscript\tGuide\t4_M_ZONDAMAN,{\n"
        "\tthana_tower = 1;\n"
        "}\n"
        "tha_t02,231,161,5\tscript\tEntrance Guide\t4_M_ZONDAMAN,{\n"
        "\tthana_tower = 10;\n"
        "}\n"
    )

    def test_conditions_and_the_npcs_that_satisfy_them_are_recorded(self):
        from generate_navigation_graph import note_variables, script_entrances, variable_setters

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "thana.txt"
            source.write_text(self.SCRIPT, encoding="utf-8")
            entrances = script_entrances(root, [source])
            setters = variable_setters(root, [source])

        tower = entrances["tha_t03"][0]
        self.assertEqual(tower["npc"], "3rdf_warp")
        self.assertIn("thana_tower == 0", tower["conditions"])
        self.assertNotIn("checkhiding()", " ".join(tower["conditions"]))
        ranked = note_variables([{"conditions": ["thana_tower >= 1"]}], setters)["thana_tower"]
        self.assertEqual((ranked[0]["npc"], ranked[0]["sets"]), ("Guide", "= 1"))


class FareValidationTests(unittest.TestCase):
    def test_a_negative_fare_is_rejected(self):
        service = {
            "id": "service-a-b",
            "availability": "conditional",
            "action": "Talk.",
            "source": "a.txt:1",
            "fare_zeny": -5,
            "from": {"map": "a", "x": 1, "y": 1},
            "to": {"map": "b", "x": 2, "y": 2},
        }
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "services.json"
            path.write_text(json.dumps({"services": [service]}), encoding="utf-8")
            edges, errors = parse_authored_services(path, {"a", "b"})

        self.assertEqual(edges, [])
        self.assertIn("fare_zeny", errors[0])
