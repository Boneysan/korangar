#!/usr/bin/env python3
"""Export manually reviewed NPC service records with source-anchor validation.

Mirrors export_item_exchange_reviews.py's discipline: every reviewed claim
must cite loaded source lines whose text still matches at build time. The
Kafra and repairmain shared-function service entries additionally cross-check
against the mechanically generated docs/npc-service-clues.v1.json so their
location lists cannot drift from the live script set without the build
failing.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

from export_item_grant_reference import source_revision
from generate_navigation_graph import loaded_script_files

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
MANIFEST = HERCULES / "npc/re/scripts_main.conf"
REVIEWS = ROOT / "tools" / "npc_service_reviews.json"
CLUES = ROOT / "docs" / "npc-service-clues.v1.json"
NPCS = ROOT / "docs" / "npcs.v1.json"
OUTPUT = ROOT / "docs" / "npc-service-reviews.v1.json"

KAFRA_MENU_ARGS = re.compile(r'callfunc\s*\(?\s*"F_Kafra"\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)')
REPAIRMAIN_ARGS = re.compile(r'callfunc\s*\(?\s*"repairmain"\s*,\s*_?\(?\s*"([^"]+)"')


def validate_sources(entry_id: str, sources: list[dict[str, Any]], active_paths: dict[str, Path]) -> None:
    for source in sources:
        path = source["path"]
        lines = source["lines"]
        if not isinstance(path, str) or not path or not isinstance(lines, list) or not lines:
            raise ValueError(f"reviewed service {entry_id} has malformed source reference {source!r}")
        source_file = active_paths.get(path)
        if source_file is None:
            raise ValueError(f"reviewed service {entry_id} cites a script not loaded by scripts_main.conf: {path}")
        text = source_file.read_text(encoding="utf-8", errors="replace")
        source_lines = text.splitlines()
        for line_number in lines:
            if not isinstance(line_number, int) or line_number < 1 or line_number > len(source_lines):
                raise ValueError(f"reviewed service {entry_id} cites missing source line {path}:{line_number}")
        for literal in source.get("required_source_literals", []):
            if literal not in text:
                raise ValueError(f"reviewed service {entry_id} source anchor no longer matches in {path}: {literal!r}")


def resolve_npc(entry_id: str, npc: dict[str, Any], npc_entries: list[dict[str, Any]]) -> int:
    matches = [
        candidate for candidate in npc_entries
        if candidate["map"].lower() == npc["map"].lower()
        and candidate["x"] == npc["x"]
        and candidate["y"] == npc["y"]
        and (candidate["internal_name"] == npc["internal_name"] or candidate["name"] == npc["internal_name"])
    ]
    if len(matches) != 1:
        raise ValueError(f"reviewed service {entry_id} NPC {npc['internal_name']!r} does not resolve to exactly one exported NPC ({len(matches)} matches)")
    return matches[0]["id"]


def build_kafra_locations(active_paths: dict[str, Path]) -> list[dict[str, Any]]:
    """Re-derive every F_Kafra call site's per-location menu variant and fees directly from source."""
    clue_data = json.loads(CLUES.read_text(encoding="utf-8"))
    locations = []
    for clue in clue_data["entries"]:
        if clue["service_kind"] != "kafra_menu":
            continue
        path = clue["source"]["path"]
        line = clue["source"]["line"]
        source_file = active_paths.get(path)
        if source_file is None:
            raise ValueError(f"kafra_menu clue cites a script not loaded by scripts_main.conf: {path}")
        text_lines = source_file.read_text(encoding="utf-8", errors="replace").splitlines()
        call_line = text_lines[line - 1]
        match = KAFRA_MENU_ARGS.search(call_line)
        if not match:
            raise ValueError(f"kafra_menu clue at {path}:{line} no longer matches the expected 5-argument F_Kafra call shape: {call_line!r}")
        welcome_msg, menu_variant, info_flag, storage_fee, cart_fee = (int(value) for value in match.groups())
        npc = clue["npc_clue"] or {}
        locations.append({
            "map": npc.get("map"),
            "x": npc.get("x"),
            "y": npc.get("y"),
            "internal_name": npc.get("internal_name"),
            "display_name": npc.get("display_name"),
            "welcome_message_variant": welcome_msg,
            "menu_variant": menu_variant,
            "info_menu_flag": info_flag,
            "storage_fee_zeny": storage_fee,
            "cart_fee_zeny": cart_fee,
            "source": {"path": path, "line": line},
        })
    locations.sort(key=lambda row: (row["map"] or "", row["x"] or 0, row["y"] or 0))
    return locations


def build_repair_locations(active_paths: dict[str, Path]) -> list[dict[str, Any]]:
    """Re-derive every repairmain call site's displayed repairman name directly from source."""
    clue_data = json.loads(CLUES.read_text(encoding="utf-8"))
    locations = []
    for clue in clue_data["entries"]:
        if clue["service_kind"] != "repair_shared_function":
            continue
        path = clue["source"]["path"]
        line = clue["source"]["line"]
        source_file = active_paths.get(path)
        if source_file is None:
            raise ValueError(f"repair_shared_function clue cites a script not loaded by scripts_main.conf: {path}")
        text_lines = source_file.read_text(encoding="utf-8", errors="replace").splitlines()
        call_line = text_lines[line - 1]
        match = REPAIRMAIN_ARGS.search(call_line)
        if not match:
            raise ValueError(f"repair_shared_function clue at {path}:{line} no longer matches the expected repairmain call shape: {call_line!r}")
        npc = clue["npc_clue"] or {}
        locations.append({
            "map": npc.get("map"),
            "x": npc.get("x"),
            "y": npc.get("y"),
            "internal_name": npc.get("internal_name"),
            "display_name": npc.get("display_name"),
            "spoken_name": match.group(1),
            "source": {"path": path, "line": line},
        })
    locations.sort(key=lambda row: (row["map"] or "", row["x"] or 0, row["y"] or 0))
    return locations


def build() -> dict[str, Any]:
    review_data = json.loads(REVIEWS.read_text(encoding="utf-8"))
    if not review_data.get("reviewed_by") or not review_data.get("review_method"):
        raise ValueError("npc service review manifest is missing reviewer or method")
    active_files = loaded_script_files(HERCULES, MANIFEST)
    active_paths = {path.relative_to(HERCULES).as_posix(): path for path in active_files}
    npc_data = json.loads(NPCS.read_text(encoding="utf-8"))
    npc_entries = npc_data["entries"]

    kafra_locations = build_kafra_locations(active_paths)
    repair_locations = build_repair_locations(active_paths)

    entries = []
    seen_ids: set[str] = set()
    for review in review_data["entries"]:
        entry_id = review.get("id")
        if not isinstance(entry_id, str) or not entry_id.strip() or entry_id in seen_ids:
            raise ValueError(f"invalid or duplicate reviewed service ID: {entry_id!r}")
        seen_ids.add(entry_id)
        for field in ("title", "service_kind", "reviewed_on"):
            if not isinstance(review.get(field), str) or not review[field].strip():
                raise ValueError(f"reviewed service {entry_id} is missing required field {field!r}")
        if not isinstance(review.get("conditions"), list) or not review["conditions"]:
            raise ValueError(f"reviewed service {entry_id} has no conditions")
        sources = review.get("sources", [])
        if not sources:
            raise ValueError(f"reviewed service {entry_id} has no source references")
        validate_sources(entry_id, sources, active_paths)

        entry: dict[str, Any] = {
            "id": entry_id,
            "title": review["title"],
            "service_kind": review["service_kind"],
            "evidence_state": review.get("evidence_state", "conditional"),
            "reviewed_by": review_data["reviewed_by"],
            "reviewed_on": review["reviewed_on"],
            "review_method": review_data["review_method"],
            "conditions": review["conditions"],
            "sources": [{"path": s["path"], "lines": s["lines"]} for s in sources],
        }
        if "npc" in review:
            entry["npc"] = dict(review["npc"])
            entry["npc"]["npc_id"] = resolve_npc(entry_id, review["npc"], npc_entries)
        if "route_table" in review:
            entry["route_table"] = review["route_table"]
        if "route_table_note" in review:
            entry["route_table_note"] = review["route_table_note"]
        if entry_id == "kafra_employee_core_services":
            entry["locations"] = kafra_locations
            entry["location_count"] = len(kafra_locations)
        if entry_id == "repairmain_shared_function":
            entry["locations"] = repair_locations
            entry["location_count"] = len(repair_locations)
        entries.append(entry)

    entries.sort(key=lambda entry: entry["id"])
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "reviewed_by": review_data["reviewed_by"],
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
        print(f"npc service review export failed: {error}", file=sys.stderr)
        return 1
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {len(json.loads(rendered)['entries'])} reviewed NPC services")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(json.loads(rendered)['entries'])} reviewed NPC services)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
