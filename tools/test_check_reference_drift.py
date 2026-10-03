"""Tests for check_reference_drift.py: how it reads each export's provenance and
which exporters it treats as checkable."""

import json
import tempfile
import unittest
from pathlib import Path

import check_reference_drift as drift


class Fixture(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        (self.root / "docs").mkdir()
        (self.root / "tools").mkdir()
        self._saved = (drift.ROOT, drift.TOOLS)
        drift.ROOT, drift.TOOLS = str(self.root), str(self.root / "tools")

    def tearDown(self):
        drift.ROOT, drift.TOOLS = self._saved
        self.dir.cleanup()

    def export(self, name, **fields):
        (self.root / "docs" / name).write_text(json.dumps(fields), encoding="utf-8")


class Provenance(Fixture):
    def test_a_plain_revision_and_its_dirty_flag_are_read(self):
        self.export("a.v1.json", source_revision="abc123", source_worktree_dirty=True)
        self.assertEqual(drift.provenance(), [("a.v1.json", "abc123", True)])

    def test_a_missing_dirty_flag_means_clean(self):
        self.export("a.v1.json", source_revision="abc123")
        self.assertEqual(drift.provenance(), [("a.v1.json", "abc123", False)])

    def test_the_manifest_that_records_both_repositories_is_read_by_its_hercules_half(self):
        self.export("manifest.v1.json", source_revision={"korangar": "k1", "hercules": "h1"})
        self.assertEqual(drift.provenance(), [("manifest.v1.json", "h1", False)])

    def test_files_without_a_revision_or_that_are_not_json_are_skipped(self):
        self.export("b.v1.json", schema_version=1)
        (self.root / "docs" / "c.v1.json").write_text("not json", encoding="utf-8")
        self.export("d.v1.json", source_revision=12345)  # not a string: ignored, never a crash
        self.assertEqual(drift.provenance(), [])


class Exporters(Fixture):
    def test_only_tools_that_have_a_check_mode_are_listed(self):
        (self.root / "tools" / "export_a.py").write_text('parser.add_argument("--check")', encoding="utf-8")
        (self.root / "tools" / "export_b.py").write_text("print('no check mode')", encoding="utf-8")
        (self.root / "tools" / "other.py").write_text('"--check"', encoding="utf-8")
        self.assertEqual([Path(path).name for path in drift.exporters()], ["export_a.py"])


class RealTree(unittest.TestCase):
    def test_every_real_export_that_records_a_revision_is_readable(self):
        rows = drift.provenance()
        self.assertGreater(len(rows), 20)
        self.assertTrue(all(isinstance(revision, str) and revision for _, revision, _ in rows))


if __name__ == "__main__":
    unittest.main()
