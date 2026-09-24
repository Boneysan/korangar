import tempfile
import unittest
from pathlib import Path

from generate_navigation_graph import parse_warps


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


if __name__ == "__main__":
    unittest.main()
