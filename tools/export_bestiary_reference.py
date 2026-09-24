#!/usr/bin/env python3
"""Export a versioned, source-linked Hercules monster reference.

Writes `docs/bestiary.v1.json` as a parallel artifact; the legacy
`docs/bestiary.json` and its DM consumers are unchanged. Scripted spawn
conditions and exact spawn locations are not inferred here.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

from export_item_reference import HERCULES, ROOT, build as build_items
from export_skill_info import parse_skill_db
from extend_bestiary_export import parse_mob_skill_db
from generate_navigation_graph import loaded_script_files, map_names


MOB_SOURCE = HERCULES / "db/re/mob_db.conf"
MOB_SKILL_SOURCE = HERCULES / "db/re/mob_skill_db.conf"
SPAWN_MANIFEST = HERCULES / "npc/re/scripts_main.conf"
MAP_INDEX = HERCULES / "db/map_index.txt"
OUTPUT = ROOT / "docs/bestiary.v1.json"
STATIC_SPAWN = re.compile(
    r"^\s*([A-Za-z0-9_]+),-?\d+,-?\d+,-?\d+,-?\d+\s+"
    r"(monster|boss_monster)\s+.+?(?:,\s*|\s+)(\d+),\s*(\d+)\b"
)
FIELDS = (
    "Name", "JName", "Lv", "Hp", "Sp", "Exp", "JExp", "AttackRange", "Attack",
    "Def", "Mdef", "Stats", "ViewRange", "ChaseRange", "Size", "Race", "Element",
    "MoveSpeed", "AttackDelay", "AttackMotion", "DamageMotion", "MvpExp",
)
FIELD_NAMES = {
    "Name": "name",
    "JName": "jname",
    "Lv": "level",
    "Hp": "hp",
    "Sp": "sp",
    "Exp": "base_exp",
    "JExp": "job_exp",
    "AttackRange": "attack_range",
    "Attack": "attack",
    "Def": "defense",
    "Mdef": "magic_defense",
    "Stats": "stats",
    "ViewRange": "view_range",
    "ChaseRange": "chase_range",
    "Size": "size",
    "Race": "race",
    "Element": "element",
    "MoveSpeed": "move_speed",
    "AttackDelay": "attack_delay",
    "AttackMotion": "attack_motion",
    "DamageMotion": "damage_motion",
    "MvpExp": "mvp_exp",
}


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(
            ["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL
        ).strip()
        dirty = bool(subprocess.check_output(["git", "-C", str(HERCULES), "status", "--porcelain"], text=True).strip())
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def normalize_element(value: Any) -> Any:
    if isinstance(value, list) and len(value) == 2:
        element, level = value
        if isinstance(element, str) and element.startswith("Ele_"):
            return {"type": element.removeprefix("Ele_"), "level": level}
    return value


def parse_static_spawns() -> tuple[dict[int, list[dict[str, Any]]], dict[str, int]]:
    """Collect loaded static mob directives, exposing maps but never cells."""
    known_maps = map_names(MAP_INDEX)
    files = loaded_script_files(HERCULES, SPAWN_MANIFEST)
    aggregate: dict[int, dict[str, dict[str, Any]]] = defaultdict(dict)
    unknown_map_records = 0
    record_count = 0

    for path in files:
        in_block_comment = False
        for line_number, raw in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), start=1):
            line = raw
            if in_block_comment:
                if "*/" not in line:
                    continue
                line = line.split("*/", 1)[1]
                in_block_comment = False
            while "/*" in line:
                before, after = line.split("/*", 1)
                if "*/" not in after:
                    line = before
                    in_block_comment = True
                    break
                line = before + after.split("*/", 1)[1]
            line = line.split("//", 1)[0]
            if not line.strip():
                continue
            match = STATIC_SPAWN.match(line)
            if not match:
                continue
            map_name, directive, raw_mob_id, _raw_amount = match.groups()
            mob_id = int(raw_mob_id)
            if map_name not in known_maps:
                unknown_map_records += 1
                continue
            if mob_id <= 0:
                continue
            source = f"{path.relative_to(HERCULES).as_posix()}:{line_number}"
            record_count += 1
            per_map = aggregate[mob_id]
            if map_name not in per_map:
                per_map[map_name] = {
                    "map": map_name,
                    "kind": "boss" if directive == "boss_monster" else "normal",
                    "spawn_records": 0,
                    "source": [],
                }
            region = per_map[map_name]
            region["spawn_records"] += 1
            if source not in region["source"]:
                region["source"].append(source)

    return (
        {mob_id: sorted(regions.values(), key=lambda region: (region["map"], region["kind"])) for mob_id, regions in aggregate.items()},
        {
            "active_script_files_scanned": len(files),
            "static_spawn_records": record_count,
            "records_with_unknown_maps": unknown_map_records,
            "records_with_unknown_mob_ids": 0,
        },
    )


def build() -> dict[str, Any]:
    mobs = parse_skill_db(MOB_SOURCE.read_text(encoding="utf-8", errors="replace"))
    if not mobs:
        raise ValueError(f"no monster records parsed from {MOB_SOURCE}")
    by_id: dict[int, dict[str, Any]] = {}
    by_sprite: dict[str, dict[str, Any]] = {}
    for mob in mobs:
        if "Id" not in mob or "SpriteName" not in mob:
            raise ValueError("monster record missing Id or SpriteName")
        mob_id = int(mob["Id"])
        sprite = mob["SpriteName"]
        if mob_id in by_id:
            raise ValueError(f"duplicate monster ID {mob_id} in {MOB_SOURCE}")
        if sprite in by_sprite:
            raise ValueError(f"duplicate monster SpriteName {sprite!r} in {MOB_SOURCE}")
        by_id[mob_id] = mob
        by_sprite[sprite] = mob

    skills = parse_mob_skill_db()
    spawn_regions_by_mob, spawn_coverage = parse_static_spawns()
    unknown_skill_sprites = sorted(set(skills) - set(by_sprite))
    if unknown_skill_sprites:
        raise ValueError(f"mob skills reference unknown sprites: {unknown_skill_sprites[:10]}")
    unknown_mob_records = sum(
        region["spawn_records"]
        for mob_id, regions in spawn_regions_by_mob.items()
        if mob_id not in by_id
        for region in regions
    )
    spawn_coverage["records_with_unknown_mob_ids"] = unknown_mob_records
    spawn_regions_by_mob = {mob_id: regions for mob_id, regions in spawn_regions_by_mob.items() if mob_id in by_id}
    spawn_coverage["mobs_with_static_spawn_regions"] = len(spawn_regions_by_mob)
    spawn_coverage["mobs_without_static_spawn_regions"] = len(by_id) - len(spawn_regions_by_mob)

    item_payload, _ = build_items()
    drops_by_mob: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for item in item_payload["entries"]:
        for drop in item["drops_from"]:
            mob_id = drop["monster_id"]
            if mob_id not in by_id:
                raise ValueError(f"item {item['id']} drop references unknown monster ID {mob_id}")
            drops_by_mob[mob_id].append({
                "item_id": item["id"],
                "aegis_name": item["aegis_name"],
                "name": item.get("name"),
                "rate_per_10000": drop["rate_per_10000"],
                "kind": drop["kind"],
                "source_record": drop["source_record"],
            })

    mob_path = MOB_SOURCE.relative_to(HERCULES).as_posix()
    skill_path = MOB_SKILL_SOURCE.relative_to(HERCULES).as_posix()
    entries: list[dict[str, Any]] = []
    for mob_id in sorted(by_id):
        mob = by_id[mob_id]
        sprite = mob["SpriteName"]
        entry: dict[str, Any] = {
            "id": mob_id,
            "sprite_name": sprite,
            "source": {"path": mob_path, "record": f"Id={mob_id}"},
            "skills_source": {"path": skill_path, "record": f"SpriteName={sprite}"} if skills.get(sprite) else None,
            "skills": [
                {
                    "skill_name": skill["Skill"],
                    "level": skill["Level"],
                    "rate": skill["Rate"],
                    "delay_ms": skill["Delay"],
                }
                for skill in skills.get(sprite, [])
            ],
            "drops": sorted(drops_by_mob[mob_id], key=lambda drop: (drop["item_id"], drop["kind"])),
            "spawn_regions": spawn_regions_by_mob.get(mob_id, []),
        }
        for field in FIELDS:
            if field in mob:
                value = normalize_element(mob[field]) if field == "Element" else mob[field]
                if field in ("Size", "Race") and isinstance(value, str):
                    value = value.removeprefix("Size_").removeprefix("RC_")
                entry[FIELD_NAMES[field]] = value
        mode = mob.get("Mode") or {}
        if isinstance(mode, dict):
            entry["modes"] = sorted(key for key, enabled in mode.items() if enabled is True)
        entries.append(entry)

    revision, dirty = source_revision()
    skill_count = sum(bool(entry["skills"]) for entry in entries)
    drop_count = sum(bool(entry["drops"]) for entry in entries)
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "sources": [mob_path, skill_path, *item_payload["sources"]],
        "coverage": {
            "monsters": len(entries),
            "with_skills": skill_count,
            "with_drops": drop_count,
            "spawn_regions": spawn_coverage,
            "scripted_spawn_conditions": "not_exported",
            "exact_spawn_locations": "not_exported",
        },
        "entries": entries,
    }


def render(payload: dict[str, Any]) -> str:
    return json.dumps(payload, indent=1, ensure_ascii=False) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check bestiary.v1.json without writing it")
    args = parser.parse_args()
    try:
        payload = build()
    except (OSError, ValueError, StopIteration) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    rendered = render(payload)
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print(f"stale: {OUTPUT} — re-run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"up to date: {payload['coverage']['monsters']} monsters")
        return 0

    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT} ({payload['coverage']['monsters']} monsters)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
