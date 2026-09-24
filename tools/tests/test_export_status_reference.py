import unittest

from export_status_reference import extract_status_call_sites, index_status_change_skills


class StatusReferenceExportTests(unittest.TestCase):
    def test_indexes_only_explicit_skill_database_status_fields(self):
        index = index_status_change_skills(
            [
                {"Id": 20, "Name": "SKILL_Z", "Description": "Zed", "StatusChange": "SC_ADAPTATION"},
                {"Id": 10, "Name": "SKILL_A", "StatusChange": "SC_ADAPTATION"},
                {"Id": 30, "Name": "SKILL_NONE", "StatusChange": "SC_NONE"},
                {"Id": 40, "Name": "SKILL_NO_FIELD"},
                {"Id": 50, "StatusChange": "SC_MISSING_NAME"},
            ]
        )

        self.assertEqual([skill["name"] for skill in index["SC_ADAPTATION"]], ["SKILL_A", "SKILL_Z"])
        self.assertNotIn("SC_NONE", index)
        self.assertNotIn("SC_MISSING_NAME", index)
        self.assertEqual(index["SC_ADAPTATION"][0]["source"], {"path": "db/re/skill_db.conf", "record": "SKILL_A"})

    def test_extracts_only_literal_status_call_sites_and_preserves_line_numbers(self):
        source = '''
// sc_start(NULL, bl, SC_FAKE, 100, 0, 1000, 0);
sc_start(
    NULL,
    bl,
    SC_BLESSING,
    100, nested(1, 2), 1000, 0
);
sc_start4(NULL, bl, SC_STUN, 15, 0, 0, 0, 0, 1000, 0);
sc_start(NULL, bl, skill->get_sc_type(SM_PROVOKE), 100, 1, 1000, 0);
const char *text = "sc_start(NULL, bl, SC_NOT_A_CALL, 100, 0, 0, 0);";
/* multiline comments\nsc_start(NULL, bl, SC_IGNORED, 100, 0, 0, 0); */
'''

        calls = extract_status_call_sites("src/map/sample.c", source)

        self.assertEqual(calls["SC_BLESSING"], [{"path": "src/map/sample.c", "line": 3}])
        self.assertEqual(calls["SC_STUN"], [{"path": "src/map/sample.c", "line": 9}])
        self.assertNotIn("SC_FAKE", calls)
        self.assertNotIn("SC_NOT_A_CALL", calls)
        self.assertNotIn("SC_IGNORED", calls)
        self.assertNotIn("SC_PROVOKE", calls)


if __name__ == "__main__":
    unittest.main()
