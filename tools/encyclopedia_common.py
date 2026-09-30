#!/usr/bin/env python3
"""Shared paths and source helpers for the encyclopedia unit workflow.

The unit tools (queue, packet, checks, CLI) read Hercules source through these
helpers so that every excerpt a model sees and every citation it writes is
checked against the same loaded-script scope.
"""
from __future__ import annotations

import json
import re
from functools import lru_cache
from pathlib import Path
from typing import Any

from generate_navigation_graph import loaded_script_files

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
SCRIPT_MANIFEST = HERCULES / "npc/re/scripts_main.conf"
PLANS = ROOT / "docs/plans"
QUEUE = PLANS / "encyclopedia-unit-queue.json"
LEDGER = PLANS / "encyclopedia-qwen3-progress.json"
JOURNAL = PLANS / "encyclopedia-qwen3-journal.md"
TEMPLATES = PLANS / "encyclopedia-unit-templates.md"
DISPOSITIONS = ROOT / "tools/encyclopedia_dispositions.json"
STAGING = ROOT / ".encyclopedia-staging"

EVIDENCE_STATES = ("verified", "conditional", "configured_estimate", "source_clue", "not_reviewed", "unknown")
REVIEWED_STATES = ("verified", "conditional", "configured_estimate")
DISPOSITION_RESULTS = ("verified", "conditional", "configured_estimate", "unknown", "absent", "not_applicable")

NPC_HEADER = re.compile(
    r"^\s*(?P<map>[A-Za-z0-9_@\-]+)(?:,\s*(?P<x>-?\d+),\s*(?P<y>-?\d+)(?:,\s*(?P<dir>-?\d+))?)?\t+"
    r"(?P<type>script|shop|cashshop|pointshop|itemshop|trader|marketshop)\t+(?P<name>[^\t]+)\t"
)
FUNCTION_HEADER = re.compile(r"^\s*function\t+script\t+(?P<name>[^\t]+)\t")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    json.loads(path.read_text(encoding="utf-8"))


@lru_cache(maxsize=1)
def loaded_scripts() -> dict[str, Path]:
    """Loaded NPC scripts keyed by their Hercules-relative path."""
    return {path.relative_to(HERCULES).as_posix(): path for path in loaded_script_files(HERCULES, SCRIPT_MANIFEST)}


def hercules_path(relpath: str) -> Path | None:
    """Resolve a citation path. NPC scripts must be loaded; src/db/conf/doc files must exist."""
    if relpath.startswith("npc/"):
        return loaded_scripts().get(relpath)
    if relpath.split("/", 1)[0] in {"src", "db", "conf", "doc"}:
        path = HERCULES / relpath
        return path if path.is_file() else None
    return None


@lru_cache(maxsize=256)
def source_lines(relpath: str) -> tuple[str, ...]:
    path = hercules_path(relpath)
    if path is None:
        raise ValueError(f"not a loaded script or known Hercules source file: {relpath}")
    return tuple(path.read_text(encoding="utf-8", errors="replace").splitlines())


def numbered(relpath: str, start: int, end: int) -> str:
    """1-based inclusive line range, one `LINE| text` row per source line."""
    lines = source_lines(relpath)
    end = min(end, len(lines))
    width = len(str(end))
    return "\n".join(f"{number:>{width}}| {lines[number - 1]}" for number in range(start, end + 1))


def npc_blocks(relpath: str) -> list[dict[str, Any]]:
    """Top-level NPC and function declarations with their 1-based line spans."""
    lines = source_lines(relpath)
    starts = []
    for index, line in enumerate(lines):
        header = NPC_HEADER.match(line)
        function = FUNCTION_HEADER.match(line)
        if header:
            starts.append((index, {
                "kind": header.group("type"),
                "map": header.group("map"),
                "x": int(header.group("x")) if header.group("x") else None,
                "y": int(header.group("y")) if header.group("y") else None,
                "name": header.group("name").strip(),
            }))
        elif function:
            starts.append((index, {"kind": "function", "map": None, "x": None, "y": None, "name": function.group("name").strip()}))
    blocks = []
    for position, (index, info) in enumerate(starts):
        end = starts[position + 1][0] if position + 1 < len(starts) else len(lines)
        # Trim trailing blank and comment-only lines so the next file section header stays out.
        while end > index + 1 and (not lines[end - 1].strip() or lines[end - 1].strip().startswith("//")):
            end -= 1
        blocks.append({**info, "path": relpath, "start": index + 1, "end": end})
    return blocks


def block_at(relpath: str, line: int) -> dict[str, Any] | None:
    for block in npc_blocks(relpath):
        if block["start"] <= line <= block["end"]:
            return block
    return None


@lru_cache(maxsize=1)
def function_index() -> dict[str, dict[str, Any]]:
    """Global `function script` helpers across loaded scripts, keyed by lowercase name."""
    index: dict[str, dict[str, Any]] = {}
    for relpath in loaded_scripts():
        for block in npc_blocks(relpath):
            if block["kind"] == "function":
                index.setdefault(block["name"].lower(), block)
    return index


@lru_cache(maxsize=1)
def items_by_key() -> dict[str, dict[str, Any]]:
    """Exported items keyed by lowercase AegisName and by decimal ID string."""
    entries = read_json(ROOT / "docs/items.v1.json")["entries"]
    table: dict[str, dict[str, Any]] = {}
    for entry in entries:
        table[entry["aegis_name"].lower()] = entry
        table[str(entry["id"])] = entry
    return table


def resolve_item(token: str) -> dict[str, Any] | None:
    return items_by_key().get(token.strip().strip('"').lower())


@lru_cache(maxsize=1)
def skills_by_key() -> dict[str, dict[str, Any]]:
    table: dict[str, dict[str, Any]] = {}
    for entry in read_json(ROOT / "docs/skills.json"):
        table[entry["Name"].upper()] = entry
        table[str(entry["Id"])] = entry
    return table


def slug(*parts: str) -> str:
    text = "-".join(str(part) for part in parts if str(part))
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
