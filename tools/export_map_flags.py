#!/usr/bin/env python3
"""Export literal mapflag directives from the active Hercules NPC script tree."""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from export_skill_info import _strip_comments
from generate_navigation_graph import INCLUDE, SCRIPT_FILE

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
MANIFEST = HERCULES / "npc/re/scripts_main.conf"
OUTPUT = ROOT / "docs/map-flags.v1.json"
RUNTIME_REVIEWS = ROOT / "tools/map_runtime_flag_reviews.json"
DESCRIPTIONS = {
    "nomemo": "Blocks saving a Warp Portal memo point; also blocks marriage warp skills.",
    "noteleport": "Blocks player teleportation such as Teleport and Fly Wing.",
    "pairship_endable": "Enables the server's pairship end behavior for this map.",
    "pairship_startable": "Enables the server's pairship start behavior for this map.",
    "nightenabled": "Allows the server night setting to affect this map.",
    "nobranch": "Blocks Dead Branch use.",
    "nopenalty": "Disables the map's EXP death penalty.",
    "nosave": "Disables normal save-point behavior for this map; a configured save destination may be supplied.",
    "pvp": "Enables player-versus-player rules.",
    "nowarpto": "Blocks warping to this map through Warp Portal and related warp methods.",
    "zone": "Assigns this map to the named map zone.",
    "pvp_noguild": "Disables guild-based PvP relationships on this map.",
    "nowarp": "Blocks warping out of this map through warp skills and commands.",
    "pvp_nightmaredrop": "Enables the PvP item-drop behavior configured for this map.",
    "reset": "Allows Neuralizer use on this map.",
    "noicewall": "Blocks Ice Wall placement.",
    "gvg_castle": "Marks this map as a Guild-versus-Guild castle map.",
    "town": "Marks the map as a town for map behavior and related systems.",
    "monster_noteleport": "Prevents ordinary monsters from teleporting on this map.",
    "noreturn": "Blocks Return and similar return-warp skills from this map.",
    "noskill": "Blocks skill use on this map, subject to server exceptions.",
    "battleground": "Enables Battleground map behavior; a mode value may refine the rules.",
    "gvg_dungeon": "Marks this map as a Guild-versus-Guild dungeon map.",
    "pvp_noparty": "Disables party-based PvP relationships on this map.",
    "gvg": "Enables Guild-versus-Guild combat behavior.",
    "notomb": "Disables MVP tomb creation on this map.",
}


def ordered_script_files(root: Path, entry: Path) -> list[Path]:
    visited: set[Path] = set()
    files: list[Path] = []

    def visit(config: Path) -> None:
        config = config.resolve()
        if config in visited or not config.is_file():
            return
        visited.add(config)
        for raw in config.read_text(encoding="utf-8", errors="replace").splitlines():
            line = raw.split("//", 1)[0].strip()
            if not line:
                continue
            include = INCLUDE.match(line)
            if include:
                visit(root / include.group(1))
                continue
            for match in SCRIPT_FILE.finditer(line):
                path = (root / match.group(1)).resolve()
                if path.is_file() and path not in files:
                    files.append(path)

    visit(entry)
    return files


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
    entries = []
    runtime_clues = []
    files = ordered_script_files(HERCULES, MANIFEST)
    winners: dict[tuple[str, str], dict[str, Any]] = {}
    override_counts: dict[tuple[str, str], int] = {}
    for path in files:
        relpath = path.relative_to(HERCULES).as_posix()
        source = _strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        for call in re.finditer(r"\b(setmapflag|removemapflag)\s*\(([^;]*?)\)\s*;", source, re.IGNORECASE | re.DOTALL):
            args = call.group(2)
            parts = [part.strip() for part in args.split(",", 2)]
            if len(parts) < 2 or not re.fullmatch(r"MF_[A-Za-z0-9_]+", parts[1], re.IGNORECASE):
                continue
            map_match = re.fullmatch(r"\"([^\"]+)\"", parts[0])
            value = parts[2] if len(parts) > 2 else ""
            runtime_clues.append({
                "operation": call.group(1).lower(),
                "map": map_match.group(1) if map_match else None,
                "map_expression": parts[0],
                "flag": parts[1],
                "value": value,
                "evidence_state": "source_clue",
                "source": {"path": relpath, "line": source.count("\n", 0, call.start()) + 1},
            })
        for line_number, raw in enumerate(source.splitlines(), 1):
            fields = raw.strip().split(None, 3)
            if len(fields) < 3 or fields[1].lower() != "mapflag":
                continue
            key = (fields[0].lower(), fields[2].lower())
            entry = {
                "map": fields[0],
                "flag": fields[2],
                "value": fields[3].strip() if len(fields) > 3 else "",
                "description": DESCRIPTIONS.get(fields[2].lower(), "Map behavior flag; see the source record for its configured value."),
                "effective_static": True,
                "source": {"path": relpath, "line": line_number},
            }
            if key in winners:
                winners[key]["effective_static"] = False
                override_counts[key] = override_counts.get(key, 1) + 1
            winners[key] = entry
            entries.append(entry)
    for entry in entries:
        key = (entry["map"].lower(), entry["flag"].lower())
        entry["effective_static"] = entry is winners[key]
        if entry["effective_static"] and key in override_counts:
            entry["overridden_directive_count"] = override_counts[key]
    entries.sort(key=lambda row: (row["map"].lower(), row["flag"].lower(), not row["effective_static"], row["source"]["path"], row["source"]["line"]))
    revision, dirty = source_revision()
    runtime_clues.sort(key=lambda row: (row["map"] or "~", row["flag"].lower(), row["source"]["path"], row["source"]["line"]))
    reviews = json.loads(RUNTIME_REVIEWS.read_text(encoding="utf-8"))
    loaded = {path.relative_to(HERCULES).as_posix(): path for path in files}
    seen_reviews: set[str] = set()
    runtime_reviews = []
    for review in reviews["entries"]:
        review_id = review.get("id")
        if not isinstance(review_id, str) or not review_id or review_id in seen_reviews:
            raise ValueError(f"invalid or duplicate runtime map-flag review ID: {review_id!r}")
        seen_reviews.add(review_id)
        if review.get("evidence_state") != "conditional" or not review.get("flags") or not review.get("conditions"):
            raise ValueError(f"runtime map-flag review {review_id} is missing its conditional scope")
        checked_sources = []
        for source_ref in review.get("sources", []):
            source_path = source_ref.get("path")
            source_file = loaded.get(source_path)
            if source_file is None:
                raise ValueError(f"runtime map-flag review {review_id} cites a script not loaded by scripts_main.conf: {source_path}")
            source_lines = source_file.read_text(encoding="utf-8", errors="replace").splitlines()
            cited_lines = source_ref.get("lines", [])
            if not cited_lines or any(not isinstance(line, int) or line < 1 or line > len(source_lines) for line in cited_lines):
                raise ValueError(f"runtime map-flag review {review_id} cites an invalid line in {source_path}")
            full_text = "\n".join(source_lines)
            for literal in source_ref.get("required_source_literals", []):
                if literal not in full_text:
                    raise ValueError(f"runtime map-flag review {review_id} source anchor changed: {literal}")
            checked_sources.append({"path": source_path, "lines": cited_lines})
        if not checked_sources:
            raise ValueError(f"runtime map-flag review {review_id} has no source citations")
        runtime_reviews.append({key: value for key, value in review.items() if key != "sources"} | {"sources": checked_sources})
    runtime_reviews.sort(key=lambda review: review["id"])
    return {"schema_version": 1, "source_revision": revision, "source_worktree_dirty": dirty, "mode": "renewal", "loaded_script_files": len(files), "entries": entries, "runtime_clues": runtime_clues, "runtime_reviews": runtime_reviews}


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
        data = json.loads(generated)
        print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(data['entries'])} static directives, {len(data['runtime_clues'])} runtime call clues)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
