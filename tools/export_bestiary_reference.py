#!/usr/bin/env python3
"""Export a versioned, source-linked Hercules monster reference.

Writes `docs/bestiary.v1.json` as a parallel artifact; the legacy
`docs/bestiary.json` and its DM consumers are unchanged. Loaded static spawn
directive centers, spreads, listed amounts, and source lines are recorded;
scripted conditions and exact randomized runtime cells are not inferred.
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
from export_skill_info import _strip_comments, parse_skill_db
from extend_bestiary_export import parse_mob_skill_db
from generate_navigation_graph import loaded_script_files, map_names


MOB_SOURCE = HERCULES / "db/re/mob_db.conf"
MOB_SKILL_SOURCE = HERCULES / "db/re/mob_skill_db.conf"
SPAWN_MANIFEST = HERCULES / "npc/re/scripts_main.conf"
MAP_INDEX = HERCULES / "db/map_index.txt"
OUTPUT = ROOT / "docs/bestiary.v1.json"
STATIC_SPAWN = re.compile(
    r"^\s*([A-Za-z0-9_]+),(-?\d+),(-?\d+),(-?\d+),(-?\d+)\s+"
    r"(monster|boss_monster)\s+.+?(?:,\s*|\s+)(\d+),\s*(\d+)\b"
)
SCRIPT_SPAWN_CALL = re.compile(r"\b(areamonster|monster|boss_monster)\s*\(")
INSTANCE_MAP_ASSIGNMENT = re.compile(r'\.@map\$\s*=\s*instance_mapname\(\s*"([A-Za-z0-9_@]+)"\s*\)\s*;')
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


def monster_skill_condition(skill: dict[str, Any]) -> str:
    condition = skill.get("cast_condition", "MSC_ALWAYS")
    data = skill.get("condition_data", 0)
    value0 = skill.get("value0", 0)
    descriptions = {
        "MSC_ALWAYS": "no additional condition",
        "MSC_MYHPLTMAXRATE": f"the monster's HP is at or below {data}%",
        "MSC_MYHPINRATE": f"the monster's HP is between {data}% and {value0}%",
        "MSC_FRIENDHPLTMAXRATE": f"a nearby ally's HP is at or below {data}%",
        "MSC_FRIENDHPINRATE": f"a nearby ally's HP is between {data}% and {value0}%",
        "MSC_MYSTATUSON": f"the monster has status {data}",
        "MSC_MYSTATUSOFF": f"the monster does not have status {data}",
        "MSC_FRIENDSTATUSON": f"a nearby ally has status {data}",
        "MSC_FRIENDSTATUSOFF": f"a nearby ally does not have status {data}",
        "MSC_ATTACKPCGT": f"more than {data} units are attacking the monster",
        "MSC_ATTACKPCGE": f"at least {data} units are attacking the monster",
        "MSC_SLAVELT": f"the monster has fewer than {data} active slaves",
        "MSC_SLAVELE": f"the monster has {data} or fewer active slaves",
        "MSC_CLOSEDATTACKED": "the monster is hit by a melee attack",
        "MSC_LONGRANGEATTACKED": "the monster is hit by a ranged attack",
        "MSC_AFTERSKILL": "after the monster uses a skill" if data == 0 else f"after the monster uses skill {data}",
        "MSC_SKILLUSED": "after a skill is used on the monster" if data == 0 else f"after skill {data} is used on the monster",
        "MSC_CASTTARGETED": "while a skill is being cast on the monster",
        "MSC_RUDEATTACKED": "after repeated rude attacks",
        "MSC_MASTERHPLTMAXRATE": f"the monster master's HP is below {data}%",
        "MSC_MASTERATTACKED": "the monster's master is attacked",
        "MSC_ALCHEMIST": "the monster was summoned by an Alchemist-class character",
        "MSC_SPAWN": "when the monster spawns",
        "MSC_MAGICATTACKED": "after the monster takes magic damage",
    }
    return descriptions.get(condition, f"untranslated server condition {condition} (condition data: {data}; val0: {value0})")


def explain_monster_skill(skill: dict[str, Any]) -> dict[str, Any]:
    state = skill.get("skill_state", "MSS_ANY")
    state_text = {
        "MSS_ANY": "while alive",
        "MSS_IDLE": "while idle with no target",
        "MSS_WALK": "while walking",
        "MSS_LOOT": "while looting or walking to loot",
        "MSS_DEAD": "while dying",
        "MSS_BERSERK": "while attacking after entering combat",
        "MSS_ANGRY": "while attacking after being attacked",
        "MSS_RUSH": "while chasing after being attacked",
        "MSS_FOLLOW": "while following an enemy before being attacked",
        "MSS_ANYTARGET": "while alive with a target",
    }.get(state, f"in server state {state}")
    target = skill.get("skill_target", "MST_TARGET")
    target_text = {
        "MST_TARGET": "current target",
        "MST_RANDOM": "random enemy in range",
        "MST_SELF": "self",
        "MST_FRIEND": "random nearby ally (self if none is found)",
        "MST_MASTER": "master (ally if no master is found)",
        "MST_AROUND": "random ground cell within 4 cells",
        **{
            f"MST_AROUND{i}": (
                f"random ground cell within {i} cells"
                if i <= 4
                else f"random ground cell within {i - 4} cells when the current target is in skill range"
            )
            for i in range(1, 9)
        },
    }.get(target, f"server target {target}")
    rate = skill.get("rate")
    rate_text = f"{rate / 100:.2f}% chance" if isinstance(rate, int) and rate > 0 else "chance not configured as a positive literal rate"
    condition_text = monster_skill_condition(skill)
    condition_sentence = "No extra condition beyond the monster state." if condition_text == "no additional condition" else f"Condition: {condition_text}."
    skill["trigger_summary"] = f"May be attempted {state_text}. {condition_sentence} Configured chance: {rate_text}."
    skill["target_summary"] = f"Target: {target_text}."
    skill["trigger_translation_status"] = "translated" if not condition_text.startswith("untranslated") and state in {
        "MSS_ANY", "MSS_IDLE", "MSS_WALK", "MSS_LOOT", "MSS_DEAD", "MSS_BERSERK", "MSS_ANGRY", "MSS_RUSH", "MSS_FOLLOW", "MSS_ANYTARGET"
    } and target in ({"MST_TARGET", "MST_RANDOM", "MST_SELF", "MST_FRIEND", "MST_MASTER", "MST_AROUND"} | {f"MST_AROUND{i}" for i in range(1, 9)}) else "partial"
    return skill


def parse_static_spawns() -> tuple[dict[int, list[dict[str, Any]]], dict[str, int]]:
    """Collect loaded static mob directive placement centers and rectangular spreads."""
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
            map_name, raw_x, raw_y, raw_xs, raw_ys, directive, raw_mob_id, raw_amount = match.groups()
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
                    "listed_monsters": 0,
                    "placements": [],
                    "source": [],
                }
            region = per_map[map_name]
            region["spawn_records"] += 1
            amount = int(raw_amount)
            region["listed_monsters"] += amount
            region["placements"].append({
                "x": int(raw_x),
                "y": int(raw_y),
                "random_map_cell": int(raw_x) == 0 and int(raw_y) == 0,
                "x_spread": int(raw_xs),
                "y_spread": int(raw_ys),
                "amount": amount,
                "source": source,
            })
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


def _split_script_arguments(value: str) -> list[str]:
    arguments: list[str] = []
    start = 0
    depth = 0
    quoted = False
    escaped = False
    for index, char in enumerate(value):
        if quoted:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif char == "," and depth == 0:
            arguments.append(value[start:index].strip())
            start = index + 1
    arguments.append(value[start:].strip())
    return arguments


def _mask_quoted_strings(value: str) -> str:
    """Keep offsets/newlines while hiding command-like text inside strings."""
    output = list(value)
    quoted = escaped = False
    for index, char in enumerate(value):
        if quoted:
            if char != "\n":
                output[index] = " "
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
            output[index] = " "
    return "".join(output)


def parse_scripted_spawn_references(mobs: list[dict[str, Any]]) -> tuple[dict[int, list[dict[str, Any]]], dict[str, int]]:
    """Index direct loaded-script spawn calls as event-driven source clues."""
    known_maps = map_names(MAP_INDEX)
    by_name: dict[str, int] = {}
    for mob in mobs:
        mob_id = int(mob["Id"])
        for key in (mob.get("SpriteName"), mob.get("AegisName")):
            if isinstance(key, str):
                by_name[key.upper()] = mob_id
    by_id: dict[int, list[dict[str, Any]]] = defaultdict(list)
    files = loaded_script_files(HERCULES, SPAWN_MANIFEST)
    calls = resolved = literal_map = literal_cells = literal_amount = map_associated = approximate_template = 0
    skipped_unknown_mob = 0
    for path in files:
        text = _strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        search_text = _mask_quoted_strings(text)
        path_record = path.relative_to(HERCULES).as_posix()
        for match in SCRIPT_SPAWN_CALL.finditer(search_text):
            calls += 1
            depth = 1
            cursor = match.end()
            quoted = escaped = False
            while cursor < len(text) and depth:
                char = text[cursor]
                if quoted:
                    if escaped: escaped = False
                    elif char == "\\": escaped = True
                    elif char == '"': quoted = False
                elif char == '"': quoted = True
                elif char == "(": depth += 1
                elif char == ")": depth -= 1
                cursor += 1
            if depth:
                continue
            args = _split_script_arguments(text[match.end():cursor - 1])
            kind = match.group(1)
            mob_index = 6 if kind == "areamonster" else 4
            if len(args) <= mob_index:
                continue
            mob_token = args[mob_index].strip()
            normalized = mob_token.removeprefix("MD_").upper()
            mob_id = by_name.get(mob_token.upper()) or by_name.get(normalized)
            if mob_id is None and normalized.startswith("G_"):
                mob_id = by_name.get(normalized[2:])
            if mob_id is None:
                skipped_unknown_mob += 1
                continue
            resolved += 1
            map_match = re.fullmatch(r'"([A-Za-z0-9_]+)"', args[0])
            map_name = map_match.group(1) if map_match else None
            template_match = re.fullmatch(r'instance_mapname\(\s*"([A-Za-z0-9_]+)"\s*\)', args[0])
            map_template = template_match.group(1) if template_match else None
            map_known = map_name in known_maps if map_name else False
            map_template_approximate = False
            if map_template is None and args[0].strip() == ".@map$":
                search_start = max(0, match.start() - 1000)
                assignments = list(INSTANCE_MAP_ASSIGNMENT.finditer(text, search_start, match.start()))
                if assignments:
                    map_template = assignments[-1].group(1)
                    map_template_approximate = True
            if map_known or map_template:
                map_associated += 1
            if map_template_approximate:
                approximate_template += 1
            if map_known:
                literal_map += 1
            cell_args = args[1:5] if kind == "areamonster" else args[1:3]
            cells = [int(value) if re.fullmatch(r"-?\d+", value) else None for value in cell_args]
            cell_known = map_known and all(value is not None for value in cells)
            if cell_known:
                literal_cells += 1
            amount_index = mob_index + 1
            amount = int(args[amount_index]) if len(args) > amount_index and re.fullmatch(r"\d+", args[amount_index]) else None
            if amount is not None:
                literal_amount += 1
            line = text.count("\n", 0, match.start()) + 1
            record: dict[str, Any] = {
                "spawn_kind": kind,
                "map": map_name if map_known else None,
                "map_template": map_template,
                "map_template_approximate": map_template_approximate,
                "map_expression": args[0],
                "coordinates": cells,
                "amount": amount,
                "source": f"{path_record}:{line}",
                "availability": "scripted_or_event_driven",
            }
            by_id[mob_id].append(record)
    for references in by_id.values():
        references.sort(key=lambda item: item["source"])
    return by_id, {
        "direct_spawn_calls": calls,
        "calls_resolved_to_monster_db": resolved,
        "calls_with_known_map": literal_map,
        "calls_with_literal_map_and_cells": literal_cells,
        "calls_associated_with_map_or_instance_template": map_associated,
        "calls_with_approximate_instance_template": approximate_template,
        "calls_with_literal_amount": literal_amount,
        "calls_with_unresolved_monster_token": skipped_unknown_mob,
        "trigger_and_runtime_population": "not_inferred",
    }


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

    skills = parse_mob_skill_db(include_triggers=True)
    spawn_regions_by_mob, spawn_coverage = parse_static_spawns()
    scripted_spawns_by_mob, scripted_spawn_coverage = parse_scripted_spawn_references(mobs)
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
                explain_monster_skill({
                    "skill_name": skill["Skill"],
                    "level": skill["Level"],
                    "rate": skill["Rate"],
                    "delay_ms": skill["Delay"],
                    "skill_state": skill["SkillState"],
                    "skill_target": skill["SkillTarget"],
                    "cast_condition": skill["CastCondition"],
                    "condition_data": skill["ConditionData"],
                    "value0": skill["Value0"],
                    "cast_time_ms": skill["CastTime"],
                    "cancelable": skill["Cancelable"],
                })
                for skill in skills.get(sprite, [])
            ],
            "drops": sorted(drops_by_mob[mob_id], key=lambda drop: (drop["item_id"], drop["kind"])),
            "spawn_regions": spawn_regions_by_mob.get(mob_id, []),
            "scripted_spawn_references": scripted_spawns_by_mob.get(mob_id, []),
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
        "sources": [mob_path, skill_path, SPAWN_MANIFEST.relative_to(HERCULES).as_posix(), *item_payload["sources"]],
        "coverage": {
            "monsters": len(entries),
            "with_skills": skill_count,
            "with_drops": drop_count,
            "spawn_regions": spawn_coverage,
            "scripted_spawn_references": scripted_spawn_coverage,
            "scripted_spawn_conditions": "call-site indexed; execution conditions and runtime population are not inferred",
            "placement_directive_areas": "exported_as_configured_centers_and_spreads; runtime cells are randomized",
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
