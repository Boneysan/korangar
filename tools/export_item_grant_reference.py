#!/usr/bin/env python3
"""Index literal item grants found in active NPC scripts as acquisition clues."""
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
OUTPUT = ROOT / "docs/item-script-grants.v1.json"
GRANT = re.compile(r"\b(getitem|getitembound|rentitem)\s*\(?\s*([A-Za-z0-9_]+)\s*,\s*(\d+)\b", re.I)
CONSUME = re.compile(r"\bdelitem\s*\(?\s*([A-Za-z0-9_]+)\s*,\s*(\d+)\b", re.I)
NPC = re.compile(r"^\s*([A-Za-z0-9_]+),\s*(-?\d+),\s*(-?\d+),\s*(-?\d+)\s+(?:script|shop|cashshop|trader)\s+([^\s,]+)")


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL).strip()
        dirty = bool(subprocess.check_output(["git", "-C", str(HERCULES), "status", "--porcelain"], text=True).strip())
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def build() -> dict[str, Any]:
    item_entries = json.loads((ROOT / "docs/items.v1.json").read_text(encoding="utf-8"))["entries"]
    items = {item["aegis_name"].lower(): item for item in item_entries}
    items_by_id = {str(item["id"]): item for item in item_entries}
    entries = []
    consumptions = []
    files = loaded_script_files(HERCULES, MANIFEST)
    ignored = 0
    for path in files:
        relpath = path.relative_to(HERCULES).as_posix()
        source = _strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        npc = None
        for line_number, line in enumerate(source.splitlines(), 1):
            declaration = NPC.match(line)
            if declaration:
                name, x, y, _direction, internal = declaration.groups()
                npc = {"name": name, "internal_name": internal, "map": "", "x": int(x), "y": int(y)}
                # Determine map from declaration's location prefix (captured separately below).
                match = re.match(r"^\s*([A-Za-z0-9_]+),", line)
                if match:
                    npc["map"] = match.group(1)
            for call in GRANT.finditer(line):
                function, constant, raw_amount = call.groups()
                item = items.get(constant.lower())
                if item is None:
                    ignored += 1
                    continue
                entries.append({
                    "item_id": int(item["id"]), "item_name": item.get("name") or item["aegis_name"],
                    "amount": int(raw_amount), "grant_kind": function.lower(),
                    "npc_clue": npc, "condition_status": "script_context_unreviewed",
                    "source": {"path": relpath, "line": line_number},
                })
            for call in CONSUME.finditer(line):
                constant, raw_amount = call.groups()
                item = items.get(constant.lower()) or items_by_id.get(constant)
                if item is None:
                    ignored += 1
                    continue
                consumptions.append({
                    "item_id": int(item["id"]), "item_name": item.get("name") or item["aegis_name"],
                    "amount": int(raw_amount), "npc_clue": npc,
                    "source": {"path": relpath, "line": line_number},
                })
    entries.sort(key=lambda row: (row["item_id"], row["source"]["path"], row["source"]["line"], row["grant_kind"]))
    revision, dirty = source_revision()
    consumptions.sort(key=lambda row: (row["item_id"], row["source"]["path"], row["source"]["line"]))
    return {"schema_version": 1, "source_revision": revision, "source_worktree_dirty": dirty, "mode": "renewal", "loaded_script_files": len(files), "unresolved_literal_item_calls": ignored, "entries": entries, "consumptions": consumptions}


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
        print(f"wrote {OUTPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
