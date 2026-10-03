#!/usr/bin/env python3
"""Export searchable, source-linked active map NPC declarations."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from export_skill_info import _strip_comments
from generate_navigation_graph import loaded_script_files, map_names


ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
MANIFEST = HERCULES / "npc/re/scripts_main.conf"
OUTPUT = ROOT / "docs/npcs.v1.json"
LOCATION = re.compile(r"^\s*([A-Za-z0-9_]+),(-?\d+),(-?\d+),(-?\d+)\s*$")
KINDS = {"script", "warp", "shop", "cashshop", "trader"}


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL).strip()
        dirty = subprocess.run(
            ["git", "-C", str(HERCULES), "diff-index", "--quiet", "HEAD"],
            capture_output=True,
        ).returncode != 0
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def build() -> dict[str, Any]:
    known_maps = map_names(HERCULES / "db/map_index.txt")
    entries: list[dict[str, Any]] = []
    files = loaded_script_files(HERCULES, MANIFEST)
    unknown_maps = 0
    for path in files:
        relative = path.relative_to(HERCULES).as_posix()
        text = _strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        for line_number, line in enumerate(text.splitlines(), start=1):
            fields = line.split("\t")
            if len(fields) < 3 or fields[1].strip() not in KINDS:
                continue
            location = LOCATION.fullmatch(fields[0])
            if not location:
                continue
            map_name, raw_x, raw_y, _direction = location.groups()
            if map_name not in known_maps:
                unknown_maps += 1
            raw_name = fields[2].strip()
            display_name = raw_name.split("::", 1)[0].split("#", 1)[0].strip() or raw_name
            internal_name = raw_name.split("::", 1)[1].strip() if "::" in raw_name else ""
            sprite = fields[3].split(",", 1)[0].strip() if len(fields) > 3 and fields[1].strip() != "warp" else ""
            entries.append({
                "id": len(entries),
                "name": raw_name,
                "display_name": display_name,
                "internal_name": internal_name,
                "map": map_name,
                "x": int(raw_x),
                "y": int(raw_y),
                "declared_type": fields[1].strip(),
                "sprite": sprite,
                "map_known": map_name in known_maps,
                "source": {"path": relative, "line": line_number},
                "offers": [],
            })
    entries.sort(key=lambda entry: (entry["map"], entry["display_name"].lower(), entry["source"]["path"], entry["source"]["line"]))
    # IDs are stable for a given sorted source snapshot and used only within
    # this generated directory.
    for entry_id, entry in enumerate(entries):
        entry["id"] = entry_id
    item_file = ROOT / "docs/items.v1.json"
    if item_file.is_file():
        item_entries = json.loads(item_file.read_text(encoding="utf-8")).get("entries", [])
        offer_index: dict[tuple[str, int, int, str], list[dict[str, Any]]] = {}
        for item in item_entries:
            for offer in item.get("shops", []):
                key = (offer["map"].lower(), int(offer["x"]), int(offer["y"]), offer["npc_name"].lower())
                offer_index.setdefault(key, []).append({
                    "item_id": int(item["id"]),
                    "item_name": item.get("name") or item["aegis_name"],
                    "currency": offer["currency"],
                    "shop_type": offer["shop_type"],
                    "price": offer.get("price"),
                    "uses_item_db_price": offer["uses_item_db_price"],
                    "source": offer["source"],
                })
        for entry in entries:
            names = {entry["name"].lower(), entry["display_name"].lower()}
            if entry["internal_name"]:
                names.add(entry["internal_name"].lower())
            for name in names:
                entry["offers"].extend(offer_index.get((entry["map"].lower(), entry["x"], entry["y"], name), []))
            unique = { (offer["item_id"], offer["source"]): offer for offer in entry["offers"] }
            entry["offers"] = sorted(unique.values(), key=lambda offer: (offer["item_name"].lower(), offer["item_id"]))
    revision, dirty = source_revision()
    counts: dict[str, int] = {}
    for entry in entries:
        kind = entry["declared_type"]
        counts[kind] = counts.get(kind, 0) + 1
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "sources": [MANIFEST.relative_to(HERCULES).as_posix(), "loaded NPC script files"],
        "coverage": {
            "loaded_script_files": len(files),
            "map_based_declarations": len(entries),
            "declarations_by_type": counts,
            "declarations_on_unknown_maps": unknown_maps,
            "npcs_with_literal_shop_offers": sum(bool(entry["offers"]) for entry in entries),
            "literal_shop_offers_linked": sum(len(entry["offers"]) for entry in entries),
            "notes": "Only active loaded static map declarations are indexed; dynamically created NPCs and script behavior are not inferred.",
        },
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check output without writing")
    args = parser.parse_args()
    payload = build()
    rendered = json.dumps(payload, indent=1, ensure_ascii=False) + "\n"
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print("stale: docs/npcs.v1.json (run tools/export_npc_reference.py)", file=sys.stderr)
            return 1
        print(f"up to date: {len(payload['entries'])} NPC declarations")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT} ({len(payload['entries'])} NPC declarations)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
