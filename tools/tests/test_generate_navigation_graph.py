import tempfile
import unittest
from pathlib import Path

from generate_navigation_graph import parse_authored_services, parse_warps


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
