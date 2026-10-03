import tempfile
import unittest
from pathlib import Path

from generate_kafra_services import build, merge

FUNCTIONS = """function\tscript\tF_KafTele\t{
\tif (@wrpD$[.@j] == "Geffen") warp "geffen", 120, 39;
\telse if (@wrpD$[.@j] == "Izlude") { if (RENEWAL) warp "izlude", 128, 98; else warp "izlude", 91, 105; }
\tend;
}

function\tscript\tF_KafSet\t{
\tif (strnpcinfo(NPC_MAP) == "prontera") {
\t\tsetarray @wrpD$, "Izlude", "Geffen";
\t\tsetarray @wrpP, 600, 1200;
\t} else if (strnpcinfo(NPC_MAP) == "izlude") {
\t\tsetarray @wrpD$, "Geffen";
\t\tsetarray @wrpP, 1200;
\t} else if (strnpcinfo(NPC_MAP) == "geffen") {
\t\tsetarray @wrpD$, "Izlude";
\t\tsetarray @wrpP, 1200;
\t} else if (strnpcinfo(NPC_MAP) == "veins") {
\t\tsetarray @wrpD$, "Geffen";
\t\tsetarray @wrpP, 2200;
\t}
\treturn;
}
"""

KAFRAS = """prontera,146,89,0\tscript\tKafra Employee::kaf_prontera\t4_F_KAFRA1,{
\tcallfunc "F_KafSet";
\tcallfunc "F_Kafra",5,0,1,40,800;
}

geffen,120,62,0\tscript\tKafra Employee::kaf_geffen\t4_F_KAFRA3,{
\tcallfunc "F_KafSet";
\tcallfunc "F_Kafra",5,3,1,40,800;
}

-\tscript\t::kaf_izlude\tFAKE_NPC,{
\tcallfunc "F_KafSet";
\tcallfunc "F_Kafra",5,0,1,40,820;
}

izlude,128,148,6\tduplicate(kaf_izlude)\tKafra Employee#iz\t4_F_KAFRA1

veins,208,128,5\tscript\tCool Event Corp. Staff::CoolEventCorpStaffVeins\t4_M_ZONDAMAN,{
\tcallfunc "F_KafSet";
\tcallfunc "F_ZondaStaff", 0, "in the town of Veins.";
}
"""


def write_tree(root: Path) -> None:
    (root / "npc/kafras").mkdir(parents=True)
    (root / "npc/re").mkdir(parents=True)
    (root / "db").mkdir()
    (root / "npc/kafras/functions_kafras.txt").write_text(FUNCTIONS, encoding="utf-8")
    (root / "npc/kafras/kafras.txt").write_text(KAFRAS, encoding="utf-8")
    (root / "npc/re/scripts_main.conf").write_text(
        'npc: "npc/kafras/functions_kafras.txt"\nnpc: "npc/kafras/kafras.txt"\n', encoding="utf-8"
    )
    (root / "db/map_index.txt").write_text("prontera 1\ngeffen 2\nizlude 3\nveins 4\n", encoding="utf-8")


class KafraServiceTests(unittest.TestCase):
    def build_fixture(self) -> dict[str, dict]:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir).resolve()
            write_tree(root)
            return {service["id"]: service for service in build(root)}

    def test_teleporting_kafra_gets_every_table_destination_with_its_fare_and_renewal_cell(self):
        services = self.build_fixture()

        izlude = services["service-kafra-prontera-izlude"]
        self.assertEqual(izlude["from"], {"map": "prontera", "x": 146, "y": 89})
        self.assertEqual(izlude["to"], {"map": "izlude", "x": 128, "y": 98})
        self.assertEqual(izlude["requirements"], "Costs 600 zeny, or one Free Ticket for Kafra Transportation.")
        self.assertIn("Talk to the Kafra Employee,", izlude["action"])
        self.assertEqual(services["service-kafra-prontera-geffen"]["to"], {"map": "geffen", "x": 120, "y": 39})

    def test_a_kafra_whose_menu_has_no_teleport_adds_nothing(self):
        services = self.build_fixture()

        self.assertNotIn("service-kafra-geffen-izlude", services)

    def test_a_duplicate_of_a_teleporting_template_teleports_from_its_own_map(self):
        services = self.build_fixture()

        hop = services["service-kafra-izlude-geffen"]
        self.assertEqual(hop["from"], {"map": "izlude", "x": 128, "y": 148})
        self.assertIn("duplicate of kaf_izlude", hop["source"])

    def test_zonda_staff_without_the_teleport_menu_adds_nothing(self):
        services = self.build_fixture()

        self.assertNotIn("service-kafra-veins-geffen", services)

    def test_merge_replaces_only_generated_kafra_entries(self):
        manifest = {
            "services": [
                {"id": "service-kafra-old-stale"},
                {"id": "service-izlude-byalan-ferry"},
            ]
        }

        merged = merge(manifest, [{"id": "service-kafra-prontera-izlude"}])

        self.assertEqual(
            [service["id"] for service in merged["services"]],
            ["service-izlude-byalan-ferry", "service-kafra-prontera-izlude"],
        )


if __name__ == "__main__":
    unittest.main()
