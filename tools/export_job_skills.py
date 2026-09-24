#!/usr/bin/env python3
"""Export verified Hercules job skill trees and prerequisite links.

The player Guide uses this reference to link a job directly to its skills.
Inheritance is expanded from `db/re/skill_tree.conf`; names and IDs are checked
against the active renewal skill DB and job constants so stale references fail
generation instead of silently becoming dead links.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from export_skill_info import _parse_value, _strip_comments, _tokenize, parse_skill_db

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
CONSTANTS = HERCULES / "db/constants.conf"
SKILL_TREE = HERCULES / "db/re/skill_tree.conf"
SKILL_DB = HERCULES / "db/re/skill_db.conf"
OUTPUT = ROOT / "docs/job-skills.v1.json"
JOB_CONSTANT = re.compile(r"^\s*Job_([A-Za-z0-9_]+):\s*(\d+)\s*$")

# Skill-tree labels predate/ differ from the public job constant spelling.
JOB_ALIASES = {
    "Swordsman": "Swordman",
    "Magician": "Mage",
    "Super_Novice": "SuperNovice",
    "Swordsman_High": "Swordman_High",
    "Magician_High": "Mage_High",
    "Baby_Novice": "Baby",
    "Baby_Swordsman": "Baby_Swordman",
    "Baby_Magician": "Baby_Mage",
    "Rune_Knight_Trans": "Rune_Knight_T",
    "Warlock_Trans": "Warlock_T",
    "Ranger_Trans": "Ranger_T",
    "Arch_Bishop_Trans": "Arch_Bishop_T",
    "Mechanic_Trans": "Mechanic_T",
    "Guillotine_Cross_Trans": "Guillotine_Cross_T",
    "Royal_Guard_Trans": "Royal_Guard_T",
    "Sorcerer_Trans": "Sorcerer_T",
    "Minstrel_Trans": "Minstrel_T",
    "Wanderer_Trans": "Wanderer_T",
    "Sura_Trans": "Sura_T",
    "Genetic_Trans": "Genetic_T",
    "Shadow_Chaser_Trans": "Shadow_Chaser_T",
    "Baby_Rune_Knight": "Baby_Rune",
    "Baby_Arch_Bishop": "Baby_Bishop",
    "Baby_Guillotine_Cross": "Baby_Cross",
    "Baby_Royal_Guard": "Baby_Guard",
    "Baby_Shadow_Chaser": "Baby_Chaser",
    "Expanded_Super_Novice": "Super_Novice_E",
    "Expanded_Super_Baby": "Super_Baby_E",
}


def parse_skill_tree(text: str) -> dict[str, dict[str, Any]]:
    """Parse top-level `Job: { ... }` records using the skill exporter parser."""
    # skill_tree.conf terminates its tuple-valued inheritance declarations
    # with semicolons; the shared libconfig tokenizer otherwise needs no change.
    tokens = list(_tokenize(_strip_comments(text).replace(";", " ")))
    jobs: dict[str, dict[str, Any]] = {}
    index = 0
    while index < len(tokens):
        if tokens[index][0] not in ("name", "digitname") or index + 2 >= len(tokens) or tokens[index + 1][1] != ":":
            raise ValueError(f"unexpected token in skill tree near {tokens[index:index + 4]!r}")
        job_name = tokens[index][1]
        if job_name in jobs:
            raise ValueError(f"duplicate job skill tree {job_name}")
        if tokens[index + 2][1] != "{":
            raise ValueError(f"job {job_name} has no record body")
        record, index = _parse_value(tokens, index + 2)
        if not isinstance(record, dict):
            raise ValueError(f"job {job_name} record is not an object")
        jobs[job_name] = record
    return jobs


def job_ids(text: str) -> dict[str, int]:
    result: dict[str, int] = {}
    for line in text.splitlines():
        match = JOB_CONSTANT.match(line)
        if match:
            name, value = match.group(1), int(match.group(2))
            result.setdefault(name, value)
    return result


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL).strip()
        dirty = bool(subprocess.check_output(["git", "-C", str(HERCULES), "status", "--porcelain"], text=True).strip())
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def expand_job_skills(
    job_name: str,
    trees: dict[str, dict[str, Any]],
    skill_ids_by_name: dict[str, int],
    cache: dict[str, dict[str, dict[str, Any]]],
    visiting: set[str] | None = None,
) -> dict[str, dict[str, Any]]:
    if job_name in cache:
        return cache[job_name]
    visiting = set() if visiting is None else visiting
    if job_name in visiting:
        raise ValueError(f"inheritance cycle in skill tree at {job_name}")
    record = trees.get(job_name)
    if record is None:
        raise ValueError(f"unknown inherited skill-tree job {job_name}")
    visiting.add(job_name)
    expanded: dict[str, dict[str, Any]] = {}
    parents = record.get("inherit", [])
    if isinstance(parents, str):
        parents = [parents]
    if not isinstance(parents, list):
        raise ValueError(f"invalid inheritance list for {job_name}")
    for parent in parents:
        expanded.update(expand_job_skills(parent, trees, skill_ids_by_name, cache, visiting))

    raw_skills = record.get("skills", {})
    if not isinstance(raw_skills, dict):
        raise ValueError(f"invalid skills table for {job_name}")
    for skill_name, raw in raw_skills.items():
        if skill_name not in skill_ids_by_name:
            raise ValueError(f"{job_name} references unknown skill {skill_name}")
        if isinstance(raw, int):
            level, minimum_job_level, prereq_source = raw, 0, {}
        elif isinstance(raw, dict):
            level = raw.get("MaxLevel")
            minimum_job_level = raw.get("MinJobLevel", 0)
            prereq_source = {key: value for key, value in raw.items() if key not in ("MaxLevel", "MinJobLevel")}
        else:
            raise ValueError(f"invalid skill entry {job_name}.{skill_name}")
        if not isinstance(level, int) or level <= 0 or not isinstance(minimum_job_level, int):
            raise ValueError(f"invalid skill level for {job_name}.{skill_name}")
        prerequisites = []
        for prerequisite, required_level in sorted(prereq_source.items()):
            if prerequisite not in skill_ids_by_name:
                raise ValueError(f"{job_name}.{skill_name} references unknown prerequisite {prerequisite}")
            if not isinstance(required_level, int) or required_level <= 0:
                raise ValueError(f"invalid prerequisite level for {job_name}.{skill_name}.{prerequisite}")
            prerequisites.append({"skill_id": skill_ids_by_name[prerequisite], "name": prerequisite, "level": required_level})
        expanded[skill_name] = {
            "skill_id": skill_ids_by_name[skill_name],
            "name": skill_name,
            "max_level": level,
            "minimum_job_level": minimum_job_level,
            "prerequisites": prerequisites,
        }
    visiting.remove(job_name)
    cache[job_name] = expanded
    return expanded


def build() -> dict[str, Any]:
    constants = job_ids(CONSTANTS.read_text(encoding="utf-8", errors="replace"))
    trees = parse_skill_tree(SKILL_TREE.read_text(encoding="utf-8", errors="replace"))
    skill_records = parse_skill_db(SKILL_DB.read_text(encoding="utf-8", errors="replace"))
    skill_ids_by_name = {row["Name"]: int(row["Id"]) for row in skill_records if "Name" in row and "Id" in row}
    if len(skill_ids_by_name) != sum(1 for row in skill_records if "Name" in row and "Id" in row):
        raise ValueError("duplicate skill names in skill_db.conf")

    cache: dict[str, dict[str, dict[str, Any]]] = {}
    entries = []
    used_ids: set[int] = set()
    for tree_name in trees:
        if tree_name == "Job_Name":
            continue
        constant_name = JOB_ALIASES.get(tree_name, tree_name)
        if constant_name not in constants:
            raise ValueError(f"no Hercules job ID for skill-tree job {tree_name} (tried {constant_name})")
        job_id = constants[constant_name]
        if job_id in used_ids:
            raise ValueError(f"multiple skill trees map to job ID {job_id}")
        used_ids.add(job_id)
        skills = expand_job_skills(tree_name, trees, skill_ids_by_name, cache)
        entries.append({
            "job_id": job_id,
            "tree_name": tree_name,
            "skills": sorted(skills.values(), key=lambda row: (row["name"], row["skill_id"])),
        })
    entries.sort(key=lambda row: row["job_id"])
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "skill_tree_source": "db/re/skill_tree.conf",
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for generated-data drift")
    args = parser.parse_args()
    for source in (CONSTANTS, SKILL_TREE, SKILL_DB):
        if not source.exists():
            raise SystemExit(f"required Hercules source missing: {source}")
    try:
        data = build()
        rendered = json.dumps(data, indent=1, ensure_ascii=False) + "\n"
    except (ValueError, KeyError, IndexError) as error:
        raise SystemExit(f"cannot export job skills: {error}") from error
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != rendered:
            raise SystemExit(f"{OUTPUT.relative_to(ROOT)} is stale — re-run {Path(__file__).name}")
        print(f"up to date: {OUTPUT.relative_to(ROOT)}")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(data['entries'])} jobs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
