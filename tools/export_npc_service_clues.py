#!/usr/bin/env python3
"""Index literal Hercules service-function call sites in active NPC scripts.

A call site is a clue that an NPC offers a player service (storage, guild
storage, pushcart rental, teleport, repair, refine-UI access, stat/skill
reset, divorce). It is not proof of the exact conditions or fee — that
requires the hand-reviewed records in npc_service_reviews.json. This mirrors
export_item_grant_reference.py's discipline: index first, review second.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from export_skill_info import _strip_comments
from generate_navigation_graph import loaded_script_files

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
MANIFEST = HERCULES / "npc/re/scripts_main.conf"
OUTPUT = ROOT / "docs/npc-service-clues.v1.json"

NPC_DECL = re.compile(r"^([A-Za-z0-9_]+),(-?\d+),(-?\d+),(-?\d+)\t(script|shop|cashshop|trader)\t(.+)")

# Literal Hercules script-command call sites that signal a specific service.
# Both the parenthesized command form (`openstorage()`) and the bare command
# form (`openstorage;`) are used across these scripts, so each pattern
# tolerates an optional argument list.
SERVICE_PATTERNS: dict[str, re.Pattern[str]] = {
    "storage": re.compile(r"\bopenstorage\b\s*(\([^)]*\))?"),
    "guild_storage": re.compile(r"\bguildopenstorage\b\s*(\([^)]*\))?"),
    "cart_rental": re.compile(r"\bsetcart\b\s*(\([^)]*\))?"),
    "repair_single": re.compile(r"(?<![A-Za-z_])repair\s*\([^)]*\)"),
    "repair_all": re.compile(r"\brepairall\b\s*(\([^)]*\))?"),
    "refine_ui_access": re.compile(r"\bopenrefineryui\b\s*(\(\s*\))?"),
    "navigation_trigger": re.compile(r"\bnavigateto\b\s*\("),
    "divorce": re.compile(r"\bdivorce\b\s*(\([^)]*\))?"),
    "skill_reset": re.compile(r"\bresetskill\b\s*(\([^)]*\))?"),
    "stat_reset": re.compile(r"\bresetstatus\b\s*(\([^)]*\))?"),
    "kafra_menu": re.compile(r"callfunc\s*\(?\s*\"F_Kafra\""),
    "kafra_storage_helper": re.compile(r"callfunc\s*\(?\s*\"F_KafStor\""),
    "kafra_teleport_helper": re.compile(r"callfunc\s*\(?\s*\"F_KafTele\""),
    "kafra_cart_helper": re.compile(r"callfunc\s*\(?\s*\"F_KafCart\""),
    "repair_shared_function": re.compile(r"callfunc\s*\(?\s*\"repairmain\""),
}


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL).strip()
        dirty = bool(subprocess.check_output(["git", "-C", str(HERCULES), "status", "--porcelain"], text=True).strip())
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def nearest_npc_declaration(lines: list[str], line_index: int) -> dict[str, Any] | None:
    """Scan upward from a call-site line for the nearest NPC/script declaration."""
    for candidate in range(line_index, max(-1, line_index - 500), -1):
        match = NPC_DECL.match(lines[candidate])
        if match:
            map_name, x, y, _direction, declared_type, raw_rest = match.groups()
            raw_name = raw_rest.split("\t", 1)[0].strip()
            display_name = raw_name.split("::", 1)[0].split("#", 1)[0].strip() or raw_name
            internal_name = raw_name.split("::", 1)[1].strip() if "::" in raw_name else raw_name
            return {
                "map": map_name,
                "x": int(x),
                "y": int(y),
                "declared_type": declared_type,
                "display_name": display_name,
                "internal_name": internal_name,
                "declaration_line": candidate + 1,
            }
    return None


def build() -> dict[str, Any]:
    files = loaded_script_files(HERCULES, MANIFEST)
    entries: list[dict[str, Any]] = []
    for path in files:
        relpath = path.relative_to(HERCULES).as_posix()
        raw_text = path.read_text(encoding="utf-8", errors="replace")
        text = _strip_comments(raw_text)
        lines = text.splitlines()
        for service_kind, pattern in SERVICE_PATTERNS.items():
            for match in pattern.finditer(text):
                line_number = text.count("\n", 0, match.start()) + 1
                npc = nearest_npc_declaration(lines, line_number - 1)
                entries.append({
                    "service_kind": service_kind,
                    "call_text": match.group(0).strip(),
                    "npc_clue": npc,
                    "source": {"path": relpath, "line": line_number},
                    "status": "unreviewed_service_call_clue",
                })
    entries.sort(key=lambda row: (row["service_kind"], row["source"]["path"], row["source"]["line"]))
    revision, dirty = source_revision()
    by_kind: dict[str, int] = {}
    for entry in entries:
        by_kind[entry["service_kind"]] = by_kind.get(entry["service_kind"], 0) + 1
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "loaded_script_files": len(files),
        "service_kinds_indexed": sorted(SERVICE_PATTERNS),
        "call_site_counts_by_kind": dict(sorted(by_kind.items())),
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != generated:
            print(f"stale generated file: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUTPUT.relative_to(ROOT)} is current")
    else:
        OUTPUT.write_text(generated, encoding="utf-8")
        payload = json.loads(generated)
        print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(payload['entries'])} service-call clues, {payload['call_site_counts_by_kind']})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
