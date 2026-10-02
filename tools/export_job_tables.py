#!/usr/bin/env python3
"""Export per-job base HP/SP tables, base ASPD tables, weight and ASPD cap.

This is a faithful port of Hercules' `status_read_job_db_sub` (src/map/status.c),
because a job's tables are not simply the arrays in job_db.conf: 95 of 128 jobs
have none of their own and inherit, and the loader regenerates missing levels
with `base + average_increment * level`, capped by the group's `MaxHP`. The
steps run in the C order, on integer arithmetic with C's truncation toward zero:

  1. BaseExpGroup (its MaxLevel bounds how far tables are generated)
  2. Inherit      - copy the parent's weight, ASPD table, HP and SP tables, and
                    regenerate each tail
  3. ParametersGroup
  4. InheritHP, InheritSP
  5. Weight, BaseASPD
  6. the job's own HPTable, SPTable

Blocks are processed in file order, including the duplicate `Baby_Novice`.

It also derives the class flags `status_get_base_maxhp` branches on (upper,
baby, Super Novice, Expanded Super Novice) by evaluating the real `MAPID_*`
values, and cross-checks the upper set against export_stat_rules.

The result is source-confirmed only. It has not been compared against values a
live server computed; see the Guide entry's wording.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from pathlib import Path
from typing import Any

from export_exp_tables import (
    ExportError,
    job_name_ids,
    matching_brace,
    parse_exp_groups,
    strip_comments,
)
from export_job_skills import HERCULES, source_revision
from export_stat_rules import job_constant_ids, upper_job_ids

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "docs/job-tables.v1.json"
JOB_DB = HERCULES / "db/re/job_db.conf"
PARAMETERS = HERCULES / "db/re/unit_parameters_db.conf"
MAP_HEADER = HERCULES / "src/map/map.h"
MAPDEFINES = HERCULES / "src/map/mapdefines.h"
MMO_HEADER = HERCULES / "src/common/mmo.h"
PC_SOURCE = HERCULES / "src/map/pc.c"
STATUS_SOURCE = HERCULES / "src/map/status.c"
PLAYER_CONF = HERCULES / "conf/map/battle/player.conf"
MAX_LEVEL = 175

WEAPONS = (
    "Fist", "Dagger", "Sword", "TwoHandSword", "Spear", "TwoHandSpear", "Axe", "TwoHandAxe", "Mace",
    "TwoHandMace", "Rod", "Bow", "Knuckle", "Instrument", "Whip", "Book", "Katar", "Revolver", "Rifle",
    "GatlingGun", "Shotgun", "GrenadeLauncher", "FuumaShuriken", "TwoHandRod", "Shield",
)


def trunc_div(numerator: int, denominator: int) -> int:
    """C integer division: truncates toward zero."""
    return int(math.trunc(numerator / denominator))


def ordered_blocks(text: str) -> list[tuple[str, str]]:
    blocks: list[tuple[str, str]] = []
    position = 0
    while match := re.search(r"(?m)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*\{", text[position:]):
        opening = position + match.end() - 1
        closing = matching_brace(text, opening)
        blocks.append((match.group(1), text[opening + 1 : closing]))
        position = closing + 1
    return blocks


def number(text: str) -> int:
    return int(text.replace("_", ""))


# --- the numeric MAPID values the HP formula branches on -------------------

def jobl_constants() -> dict[str, int]:
    text = strip_comments(MMO_HEADER.read_text(encoding="utf-8", errors="replace"))
    return {name: int(value, 0) for name, value in re.findall(r"#define\s+(JOBL_[A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+|\d+)", text)}


def mapid_values() -> dict[str, int]:
    """Evaluate map.h's MAPID enums, including auto-incremented bare members."""
    constants = jobl_constants()
    text = strip_comments(MAP_HEADER.read_text(encoding="utf-8", errors="replace"))
    values: dict[str, int] = {}
    for enum_body in re.findall(r"enum\s*\{(.*?)\}\s*;", text, flags=re.S):
        previous = -1
        for member in enum_body.split(","):
            member = member.strip()
            if not member:
                continue
            match = re.fullmatch(r"(MAPID_[A-Z0-9_]+)\s*(?:=\s*(.+))?", member, flags=re.S)
            if not match:
                continue
            name, expression = match.groups()
            if expression is None:
                previous += 1
                values[name] = previous
                continue
            total = 0
            for term in (part.strip() for part in expression.split("|")):
                if term in constants:
                    total |= constants[term]
                elif term in values:
                    total |= values[term]
                elif re.fullmatch(r"0x[0-9A-Fa-f]+|\d+", term):
                    total |= int(term, 0)
                else:
                    raise ExportError(f"cannot evaluate {term!r} in {name}")
            values[name] = total
            previous = total
    return values


def mask_constants() -> dict[str, int]:
    text = MAPDEFINES.read_text(encoding="utf-8", errors="replace")
    base = int(re.search(r"#define\s+MAPID_UPPERMASK\s+(0x[0-9A-Fa-f]+)", text).group(1), 0)  # type: ignore[union-attr]
    return {"UPPER": base}


def job_mapids() -> dict[int, int]:
    """job id -> numeric mapid, from pc_jobid2mapid (explicit cases + class.h X-macro)."""
    values = mapid_values()
    ids = job_constant_ids()
    pc_text = PC_SOURCE.read_text(encoding="utf-8", errors="replace")
    start = pc_text.index("static int pc_jobid2mapid")
    body = pc_text[start : pc_text.index("\n}\n", start)]
    explicit = body[: body.index("#define JOB_ENUM_VALUE")]
    result: dict[int, int] = {}
    pending: list[str] = []
    for line in explicit.splitlines():
        pending += re.findall(r"case\s+JOB_([A-Z0-9_]+)\s*:", line)
        match = re.search(r"return\s+(MAPID_[A-Z0-9_]+)\s*;", line)
        if match:
            for label in pending:
                result[ids[label]] = values[match.group(1)]
            pending = []
    class_text = strip_comments((HERCULES / "src/common/class.h").read_text(encoding="utf-8", errors="replace"))
    for name, value in re.findall(r"JOB_ENUM_VALUE\(\s*([A-Z0-9_]+)\s*,\s*(\d+)\s*,", class_text):
        result[int(value)] = values[f"MAPID_{name}"]
    return result


# --- unit parameter groups: MaxHP caps and MaxASPD -------------------------

def parse_unit_groups() -> dict[str, dict[str, Any]]:
    text = strip_comments(PARAMETERS.read_text(encoding="utf-8", errors="replace"))
    groups: dict[str, dict[str, Any]] = {}
    for name, body in ordered_blocks(text):
        inherit = re.search(r'Inherit\s*:\s*"([^"]+)"', body)
        parent = groups[inherit.group(1)] if inherit else None
        if inherit and parent is None:
            raise ExportError(f"parameter group {name} inherits {inherit.group(1)} before it is defined")
        caps: list[tuple[int, int]] | None = None
        maxhp = re.search(r"MaxHP\s*:\s*\{([^{}]*)\}", body)
        if maxhp:
            caps = sorted((int(level), number(value)) for level, value in re.findall(r"Lv(\d+)\s*:\s*([\d_]+)", maxhp.group(1)))
            if caps and caps[-1][0] < MAX_LEVEL:
                caps[-1] = (MAX_LEVEL, caps[-1][1])  # "extend highest level all the way to cap"
        elif parent:
            caps = parent["maxhp_caps"]
        if not caps:
            raise ExportError(f"parameter group {name} has no MaxHP caps")
        aspd = re.search(r"MaxASPD\s*:\s*(\d+)", body)
        groups[name] = {
            "maxhp_caps": caps,
            "max_aspd": int(aspd.group(1)) if aspd else (parent["max_aspd"] if parent else None),
        }
        if groups[name]["max_aspd"] is None:
            raise ExportError(f"parameter group {name} has no MaxASPD")
    return groups


def cap_entry(groups: dict[str, dict[str, Any]], group: str, level: int) -> tuple[int, int]:
    for max_level, value in groups[group]["maxhp_caps"]:
        if level <= max_level:
            return max_level, value
    return groups[group]["maxhp_caps"][-1]


# --- the loader ------------------------------------------------------------

class Tables:
    def __init__(self) -> None:
        self.hp = [0] * (MAX_LEVEL + 2)
        self.sp = [0] * (MAX_LEVEL + 2)
        self.aspd: dict[str, int] = {}
        self.weight = 0
        self.group: str | None = None
        self.max_level = 0


def max_sp_setting() -> int:
    match = re.search(r"(?m)^\s*max_sp\s*:\s*(\d+)", PLAYER_CONF.read_text(encoding="utf-8", errors="replace"))
    if not match:
        raise ExportError("player.conf has no max_sp")
    return int(match.group(1))


def generate_hp(table: list[int], start: int, base: int, average: int, max_level: int, groups, group: str) -> None:
    max_cap_level, cap = cap_entry(groups, group, 1)
    level = start
    while level <= max_level:
        if level > max_cap_level:
            max_cap_level, cap = cap_entry(groups, group, level)
        table[level] = min(base + average * level, cap)
        level += 1


def inherit_hp(child: Tables, parent: Tables, groups) -> None:
    i = 1
    while i <= MAX_LEVEL and parent.hp[i]:
        child.hp[i] = parent.hp[i]
        i += 1
    base = child.hp[1] if i > 1 else 35
    if i > 2:
        if i >= MAX_LEVEL + 1:
            i = MAX_LEVEL
        average = trunc_div(child.hp[i] - base, i - 1)
    else:
        average = 5
    generate_hp(child.hp, i, base, average, child.max_level, groups, child.group)


def inherit_sp(child: Tables, parent: Tables, max_sp: int) -> None:
    i = 1
    while i <= MAX_LEVEL and parent.sp[i]:
        child.sp[i] = parent.sp[i]
        i += 1
    base = child.sp[1] if i > 1 else 10
    if i > 2:
        if i >= MAX_LEVEL + 1:
            i = MAX_LEVEL
        average = trunc_div(child.sp[i] - base, i - 1)
    else:
        average = 1
    while i <= child.max_level:
        child.sp[i] = min(base + average * i, max_sp)
        i += 1


def own_hp_table(tables: Tables, values: list[int], groups) -> None:
    max_cap_level, cap = cap_entry(groups, tables.group, 1)
    level = 0
    while level <= MAX_LEVEL and level < len(values):
        if level > max_cap_level:
            max_cap_level, cap = cap_entry(groups, tables.group, level)
        level += 1
        tables.hp[level] = min(values[level - 1], cap)
    base = tables.hp[1] if level > 0 else 35
    if level > 2:
        if level >= MAX_LEVEL + 1:
            level = MAX_LEVEL
        average = trunc_div(tables.hp[level] - base, level)
    else:
        average = 5
    level += 1
    while level <= tables.max_level:
        if level > max_cap_level:
            max_cap_level, cap = cap_entry(groups, tables.group, level)
        tables.hp[level] = min(base + average * level, cap)
        level += 1


def own_sp_table(tables: Tables, values: list[int], max_sp: int) -> None:
    level = 0
    while level <= MAX_LEVEL and level < len(values):
        level += 1
        tables.sp[level] = min(values[level - 1], max_sp)
    base = tables.sp[1] if level > 0 else 10
    if level > 2:
        if level >= MAX_LEVEL + 1:
            level = MAX_LEVEL
        average = trunc_div(tables.sp[level] - base, level)
    else:
        average = 1
    level += 1
    while level <= tables.max_level:
        tables.sp[level] = min(base + average * level, max_sp)
        level += 1


def names_in(body: str, key: str) -> list[str] | None:
    match = re.search(r"\b" + key + r'\s*:\s*\(([^)]*)\)', body)
    return re.findall(r'"([^"]+)"', match.group(1)) if match else None


def number_list(body: str, key: str) -> list[int] | None:
    match = re.search(r"\b" + key + r"\s*:\s*\[([^\]]*)\]", body, re.S)
    return [int(value) for value in re.findall(r"\d+", match.group(1))] if match else None


def load_jobs() -> tuple[dict[int, Tables], dict[int, str]]:
    base_groups, _ = parse_exp_groups()
    groups = parse_unit_groups()
    names = job_name_ids()
    max_sp = max_sp_setting()
    text = strip_comments(JOB_DB.read_text(encoding="utf-8", errors="replace"))
    state: dict[int, Tables] = {}
    labels: dict[int, str] = {}

    for name, body in ordered_blocks(text):
        if name == "Job_Name":
            continue
        if name not in names:
            raise ExportError(f"job_db.conf job {name} is not in pc_check_job_name")
        job_id = names[name]
        tables = state.setdefault(job_id, Tables())
        labels[job_id] = name

        base_group = re.search(r'BaseExpGroup\s*:\s*"([^"]+)"', body)
        if not base_group or base_group.group(1) not in base_groups:
            raise ExportError(f"job {name} has no usable BaseExpGroup")
        tables.max_level = base_groups[base_group.group(1)]["max_level"]

        for parent_name in names_in(body, "Inherit") or []:
            parent = state.get(names[parent_name])
            if parent is None:
                raise ExportError(f"{name} inherits {parent_name} before it is loaded")
            tables.weight = parent.weight
            tables.aspd = dict(parent.aspd)
            tables.group = parent.group
            inherit_hp(tables, parent, groups)
            inherit_sp(tables, parent, max_sp)

        group = re.search(r'ParametersGroup\s*:\s*"([^"]+)"', body)
        if group:
            if group.group(1) not in groups:
                raise ExportError(f"job {name} names unknown parameter group {group.group(1)}")
            tables.group = group.group(1)
        if tables.group is None:
            raise ExportError(f"job {name} has no ParametersGroup")

        for parent_name in names_in(body, "InheritHP") or []:
            inherit_hp(tables, state[names[parent_name]], groups)
        for parent_name in names_in(body, "InheritSP") or []:
            inherit_sp(tables, state[names[parent_name]], max_sp)

        weight = re.search(r"\bWeight\s*:\s*(\d+)", body)
        if weight:
            tables.weight = int(weight.group(1))
        elif not tables.weight:
            tables.weight = 20000

        aspd_block = re.search(r"BaseASPD\s*:\s*\{([^{}]*)\}", body)
        if aspd_block:
            for weapon, value in re.findall(r"([A-Za-z]+)\s*:\s*(\d+)", aspd_block.group(1)):
                if weapon not in WEAPONS:
                    raise ExportError(f"job {name} has BaseASPD for unknown weapon {weapon}")
                tables.aspd[weapon] = int(value)

        hp_values = number_list(body, "HPTable")
        if hp_values is not None:
            own_hp_table(tables, hp_values, groups)
        sp_values = number_list(body, "SPTable")
        if sp_values is not None:
            own_sp_table(tables, sp_values, max_sp)
    return state, labels


def check_source_shape() -> None:
    """Fail if the C code no longer has the shapes this port restates."""
    status = re.sub(r"\s+", " ", STATUS_SOURCE.read_text(encoding="utf-8", errors="replace"))
    for needle in (
        "val += val * 25 / 100; //Trans classes get a 25% hp bonus",
        "val = val * 70 / 100; //Baby classes get a 30% hp penalty",
        "val += val * st->vit / 100; // +1% per each point of VIT",
        "val += val * st->int_ / 100;",
        "val += 2000; //Supernovice lvl99 hp bonus.",
        "val += 2000; //Extented Supernovice lvl150 hp bonus.",
        "status->dbs->HP_table[idx][level] = min(base + avg_increment * level, maxhp->value);",
        "temp = (float)(sqrt(temp) * 0.25f) + 0xc4;",
        "temp = st->dex * st->dex / 5.0f + st->agi * st->agi * 0.5f;",
        "temp = st->dex * st->dex / 7.0f + st->agi * st->agi * 0.5f;",
    ):
        if re.sub(r"\s+", " ", needle) not in status:
            raise ExportError(f"status.c no longer contains an expression this exporter relies on: {needle}")


def build() -> dict[str, Any]:
    check_source_shape()
    state, labels = load_jobs()
    groups = parse_unit_groups()
    mapids = job_mapids()
    values = mapid_values()
    constants = jobl_constants()
    upper_mask = mask_constants()["UPPER"]
    third_mask = constants["JOBL_THIRD"] | upper_mask

    # Cross-check: the numeric JOBL_UPPER set must equal the name-based set.
    numeric_upper = {job for job, mapid in mapids.items() if mapid & constants["JOBL_UPPER"]}
    if numeric_upper != upper_job_ids():
        raise ExportError("numeric JOBL_UPPER set disagrees with export_stat_rules")

    unique: dict[tuple[int, ...], str] = {}
    hp_tables: dict[str, list[int]] = {}
    sp_tables: dict[str, list[int]] = {}

    def intern(kind: str, table: list[int], target: dict[str, list[int]]) -> str:
        key = (hash(kind),) + tuple(table)
        if key not in unique:
            unique[key] = f"{kind}-{len(target) + 1}"
            target[unique[key]] = table
        return unique[key]

    jobs: list[dict[str, Any]] = []
    for job_id in sorted(state):
        tables = state[job_id]
        hp = tables.hp[1 : tables.max_level + 1]
        sp = tables.sp[1 : tables.max_level + 1]
        if any(value <= 0 for value in hp) or any(value <= 0 for value in sp):
            bad = [(level + 1, value) for level, value in enumerate(hp) if value <= 0][:3]
            raise ExportError(f"{labels[job_id]} has a non-positive table entry (HP {bad}); the loader port produced a value the server could not use")
        mapid = mapids[job_id]
        jobs.append(
            {
                "job_id": job_id,
                "name": labels[job_id],
                "max_level": tables.max_level,
                "hp_table": intern("hp", hp, hp_tables),
                "sp_table": intern("sp", sp, sp_tables),
                "weight_base": tables.weight,
                "max_aspd": groups[tables.group]["max_aspd"],  # type: ignore[index]
                "base_aspd": {weapon: tables.aspd[weapon] for weapon in WEAPONS if weapon in tables.aspd},
                "upper": bool(mapid & constants["JOBL_UPPER"]),
                "baby": bool(mapid & constants["JOBL_BABY"]) and not mapid & constants["JOBL_UPPER"],
                "super_novice": (mapid & upper_mask) == values["MAPID_SUPER_NOVICE"],
                "expanded_super_novice": (mapid & third_mask) == values["MAPID_SUPER_NOVICE_E"],
            }
        )
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "source": [
            "db/re/job_db.conf",
            "db/re/unit_parameters_db.conf",
            "src/map/status.c (status_read_job_db_sub, status_get_base_maxhp, status_get_base_maxsp, status_base_amotion_pc)",
            "src/map/pc.c (pc_jobid2mapid)",
            "src/map/map.h, src/map/mapdefines.h, src/common/mmo.h (MAPID_*, JOBL_*)",
            "conf/map/battle/player.conf (max_sp)",
        ],
        "rule": "hp_table[n - 1] and sp_table[n - 1] are the class base values at level n, before the upper/baby/Super Novice adjustment, VIT/INT percent, equipment and status bonuses.",
        "hp_tables": hp_tables,
        "sp_tables": sp_tables,
        "jobs": jobs,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for drift without writing")
    args = parser.parse_args()
    try:
        content = build()
    except (OSError, ValueError, KeyError, IndexError, StopIteration, AttributeError) as error:
        raise SystemExit(f"cannot export job tables: {error!r}") from error
    payload = json.dumps(content, separators=(",", ":")) + "\n"
    summary = f"{len(content['jobs'])} jobs, {len(content['hp_tables'])} HP tables, {len(content['sp_tables'])} SP tables"
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != payload:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {summary}")
        return 0
    OUTPUT.write_text(payload, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({summary}, {OUTPUT.stat().st_size // 1024} KiB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
