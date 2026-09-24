import unittest

from export_status_reference import index_status_change_skills


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


if __name__ == "__main__":
    unittest.main()
