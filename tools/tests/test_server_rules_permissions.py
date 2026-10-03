#!/usr/bin/env python3
"""Unit tests verifying server rule permissions and GM-only command gates (F36).

Asserts that administrative and observability commands (@metrics, @dm)
are strictly protected by the server permission model and never granted to
ordinary players (group 0).
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
HERCULES = ROOT.parent / "Hercules"
GROUPS_CONF = HERCULES / "conf/groups.conf"
METRICS_SCRIPT = HERCULES / "npc/custom/korangar_metrics.txt"


class ServerRulesPermissionsTests(unittest.TestCase):
    def test_group_zero_has_level_zero(self):
        """Default player group 0 must have level 0 (non-GM)."""
        text = GROUPS_CONF.read_text(encoding="utf-8", errors="replace")
        player_group_match = re.search(r"\bid\s*:\s*0\b([\s\S]*?)(?=\n\s*\},)", text)
        self.assertIsNotNone(player_group_match, "group 0 must exist in conf/groups.conf")
        player_group = player_group_match.group(1)

        level_match = re.search(r"\blevel\s*:\s*(\d+)", player_group)
        self.assertIsNotNone(level_match, "group 0 must declare a level")
        self.assertEqual(int(level_match.group(1)), 0, "group 0 must have level 0")

    def test_group_zero_does_not_grant_gm_commands(self):
        """Group 0 must strictly NOT be granted @metrics, @dm, or dangerous GM commands."""
        text = GROUPS_CONF.read_text(encoding="utf-8", errors="replace")
        player_group_match = re.search(r"\bid\s*:\s*0\b([\s\S]*?)(?=\n\s*\},)", text)
        player_group = player_group_match.group(1)

        forbidden_commands = ("metrics", "dm", "item", "item2", "warp", "kill", "alive", "speed", "reloadatcommand")
        for cmd in forbidden_commands:
            self.assertFalse(
                re.search(rf"\b{cmd}\s*:\s*true\b", player_group, re.I),
                f"group 0 must not be granted @{cmd}"
            )

    def test_metrics_command_requires_gm_level(self):
        """@metrics must be bound with level >= 1, ensuring non-GM accounts are rejected."""
        self.assertTrue(METRICS_SCRIPT.is_file(), "korangar_metrics.txt must exist")
        text = METRICS_SCRIPT.read_text(encoding="utf-8", errors="replace")
        bind_match = re.search(r'bindatcmd\s*\(\s*"metrics"\s*,\s*[^,]+,\s*(\d+)\s*,\s*(\d+)', text)
        self.assertIsNotNone(bind_match, "@metrics must be registered via bindatcmd")
        min_level, max_level = int(bind_match.group(1)), int(bind_match.group(2))
        self.assertGreaterEqual(min_level, 1, "@metrics minimum GM level must be at least 1")
        self.assertGreaterEqual(max_level, min_level, "@metrics max level must be >= min level")

    def test_group_zero_has_free_resets_and_loot_commands(self):
        """Group 0 must explicitly have free reset (@streset, @skreset, @refundskill) and loot commands (S2/GDD §7.5)."""
        text = GROUPS_CONF.read_text(encoding="utf-8", errors="replace")
        player_group_match = re.search(r"\bid\s*:\s*0\b([\s\S]*?)(?=\n\s*\},)", text)
        player_group = player_group_match.group(1)

        required_commands = ("streset", "skreset", "refundskill", "autoloot", "alootid", "autopickup")
        for cmd in required_commands:
            self.assertTrue(
                re.search(rf"\b{cmd}\s*:\s*true\b", player_group, re.I),
                f"group 0 must explicitly be granted @{cmd}"
            )

    def test_party_exp_sharing_rules(self):
        """Party EXP rules must configure 25% even-share bonus and 30-level share spread (S3/GDD §13.5)."""
        inter_server = (HERCULES / "conf/common/inter-server.conf").read_text(encoding="utf-8", errors="replace")
        share_level_match = re.search(r"\bparty_share_level\s*:\s*(\d+)", inter_server)
        self.assertIsNotNone(share_level_match, "party_share_level must be defined in conf/common/inter-server.conf")
        self.assertEqual(int(share_level_match.group(1)), 30, "party_share_level must be 30")

        battle_import = (HERCULES / "conf/import/battle.conf").read_text(encoding="utf-8", errors="replace")
        even_share_match = re.search(r"\bparty_even_share_bonus\s*:\s*(\d+)", battle_import)
        self.assertIsNotNone(even_share_match, "party_even_share_bonus must be defined in conf/import/battle.conf")
        self.assertEqual(int(even_share_match.group(1)), 25, "party_even_share_bonus must be 25%")

    def test_progression_rules_exported_cleanly(self):
        """First and Second Job progression rules must be present in docs/server-rules.v1.json."""
        import json
        rules_path = ROOT / "docs/server-rules.v1.json"
        self.assertTrue(rules_path.is_file(), "server-rules.v1.json must exist")
        data = json.loads(rules_path.read_text(encoding="utf-8"))
        rule_ids = {entry["id"] for entry in data.get("entries", [])}
        self.assertIn("first-job-progression", rule_ids)
        self.assertIn("second-job-progression", rule_ids)


if __name__ == "__main__":
    unittest.main()
