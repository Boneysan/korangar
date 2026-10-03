#!/usr/bin/env python3
"""Export manually reviewed item exchange paths with source-anchor validation."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from export_item_grant_reference import source_revision
from generate_navigation_graph import loaded_script_files


ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
MANIFEST = HERCULES / "npc/re/scripts_main.conf"
REVIEWS = ROOT / "tools" / "item_exchange_reviews.json"
OUTPUT = ROOT / "docs" / "item-exchanges.v1.json"


def build() -> dict[str, Any]:
    review_data = json.loads(REVIEWS.read_text(encoding="utf-8"))
    active_files = loaded_script_files(HERCULES, MANIFEST)
    active_paths = {path.relative_to(HERCULES).as_posix(): path for path in active_files}
    item_data = json.loads((ROOT / "docs" / "items.v1.json").read_text(encoding="utf-8"))
    item_names = {entry["id"]: entry.get("name") or entry["aegis_name"] for entry in item_data["entries"]}
    npc_data = json.loads((ROOT / "docs" / "npcs.v1.json").read_text(encoding="utf-8"))
    entries = []
    seen_ids: set[str] = set()

    for review in review_data["entries"]:
        exchange_id = review["id"]
        if not review_data.get("reviewed_by") or not review_data.get("reviewed_on") or not review_data.get("review_method"):
            raise ValueError("item exchange review manifest is missing reviewer, date, or method")
        if exchange_id in seen_ids:
            raise ValueError(f"duplicate reviewed exchange ID: {exchange_id}")
        seen_ids.add(exchange_id)
        source_path = review["source"]["path"]
        source_file = active_paths.get(source_path)
        if source_file is None:
            raise ValueError(f"reviewed exchange {exchange_id} cites a script not loaded by scripts_main.conf: {source_path}")
        source_lines = source_file.read_text(encoding="utf-8", errors="replace").splitlines()
        for line_number in review["source"]["lines"]:
            if line_number < 1 or line_number > len(source_lines):
                raise ValueError(f"reviewed exchange {exchange_id} cites missing source line {source_path}:{line_number}")
        source_text = "\n".join(source_lines)
        for literal in review["required_source_literals"]:
            if literal not in source_text:
                raise ValueError(f"reviewed exchange {exchange_id} source anchor no longer matches: {literal}")

        npc_source_lines = [line for line in source_lines if "script\t" in line and review["npc"]["internal_name"] in line]
        expected_decl = f'{review["npc"]["map"]},{review["npc"]["x"]},{review["npc"]["y"]},'
        if not any(line.lstrip().startswith(expected_decl) for line in npc_source_lines):
            raise ValueError(f"reviewed exchange {exchange_id} NPC declaration location changed")
        matching_npcs = [
            npc for npc in npc_data["entries"]
            if npc["map"].lower() == review["npc"]["map"].lower()
            and npc["x"] == review["npc"]["x"]
            and npc["y"] == review["npc"]["y"]
            and (npc["internal_name"] == review["npc"]["internal_name"] or npc["name"] == review["npc"]["internal_name"])
        ]
        if len(matching_npcs) != 1:
            raise ValueError(f"reviewed exchange {exchange_id} does not resolve to exactly one exported NPC")
        review["npc"]["npc_id"] = matching_npcs[0]["id"]

        for collection in (review.get("inputs", []),):
            for item in collection:
                if item["item_id"] not in item_names:
                    raise ValueError(f"reviewed exchange {exchange_id} references missing item {item['item_id']}")
                item["item_name"] = item_names[item["item_id"]]
        for outcome in review.get("outcomes", []):
            if not outcome.get("item_ids"):
                raise ValueError(f"reviewed exchange {exchange_id} has an empty outcome")
            for item_id in outcome["item_ids"]:
                if item_id not in item_names:
                    raise ValueError(f"reviewed exchange {exchange_id} references missing outcome item {item_id}")
            outcome["items"] = [{"item_id": item_id, "item_name": item_names[item_id]} for item_id in outcome["item_ids"]]

        entry = {key: value for key, value in review.items() if key != "required_source_literals"}
        entry["reviewed_by"] = review.get("reviewed_by") or review_data["reviewed_by"]
        entry["reviewed_on"] = review.get("reviewed_on") or review_data["reviewed_on"]
        entry["evidence_state"] = "conditional" if entry.get("conditions") else "verified"
        entry["source"]["reviewed_lines"] = list(entry["source"]["lines"])
        entries.append(entry)

    revision, dirty = source_revision()
    entries.sort(key=lambda entry: entry["id"])
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "reviewed_by": review_data["reviewed_by"],
        "reviewed_on": review_data["reviewed_on"],
        "review_method": review_data["review_method"],
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for stale output without writing")
    args = parser.parse_args()
    try:
        rendered = json.dumps(build(), indent=2, ensure_ascii=False) + "\n"
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"item exchange review export failed: {error}", file=sys.stderr)
        return 1
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {len(json.loads(rendered)['entries'])} reviewed exchanges")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(json.loads(rendered)['entries'])} reviewed exchanges)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
