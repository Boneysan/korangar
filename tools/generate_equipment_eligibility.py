#!/usr/bin/env python3
"""Generate equipment eligibility table from Hercules item databases.

Extracts job, level, sex, and slot requirements from db/re/item_db.conf
and db/item_db2.conf for all equippable items.

Writes korangar/src/world/library/equipment_eligibility.tsv, which the buy
window reads to mark gear the current character cannot wear. Ported from the
Hercules branch agent/map-teleport-safety (tools/gen-equipment-eligibility.py)
so it sits with the other generators and tools/audits/generated-drift.sh.

Usage:
  tools/generate_equipment_eligibility.py [HERCULES_DIR]          # write the table
  tools/generate_equipment_eligibility.py [HERCULES_DIR] --check  # exit 1 if stale
"""
from __future__ import annotations

import argparse
import os
import sys

KORANGAR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_positional = [argument for argument in sys.argv[1:] if not argument.startswith("-")]
HERCULES = os.path.abspath(_positional[0] if _positional else os.path.join(KORANGAR, os.pardir, "Hercules"))
sys.path.insert(0, os.path.join(HERCULES, "tools"))
from utils import libconf  # noqa: E402  (Hercules' own libconfig reader)

ITEM_DB_CONF = os.path.join(HERCULES, "db", "re", "item_db.conf")
ITEM_DB2_CONF = os.path.join(HERCULES, "db", "item_db2.conf")
OUTPUT_TSV = os.path.join(KORANGAR, "korangar", "src", "world", "library", "equipment_eligibility.tsv")

SCHEMA_VERSION = 1

JOB_NAME_TO_IDS = {
    "Novice": [0, 23, 4001, 4023, 4045],
    "Swordsman": [1, 4002, 4024],
    "Swordman": [1, 4002, 4024],
    "Knight": [7, 4008, 4030],
    "Crusader": [14, 4015, 4037],
    "Magician": [2, 4003, 4025],
    "Mage": [2, 4003, 4025],
    "Wizard": [9, 4010, 4032],
    "Sage": [16, 4017, 4039],
    "Archer": [3, 4004, 4026],
    "Hunter": [11, 4012, 4034],
    "Bard": [19, 4020, 4042],
    "Dancer": [20, 4021, 4043],
    "Acolyte": [4, 4005, 4027],
    "Priest": [8, 4009, 4031],
    "Monk": [15, 4016, 4038],
    "Merchant": [5, 4006, 4028],
    "Blacksmith": [10, 4011, 4033],
    "Alchemist": [18, 4019, 4041],
    "Thief": [6, 4007, 4029],
    "Assassin": [12, 4013, 4035],
    "Rogue": [17, 4018, 4040],
    "Taekwon": [4046],
    "Star_Gladiator": [4047],
    "Soul_Linker": [4048],
    "Gunslinger": [24],
    "Rebellion": [4215],
    "Ninja": [25],
    "Kagerou": [4211, 4212],
    "Summoner": [4218],
    "Gangsi": [4049],
    "Death_Knight": [4050],
    "Dark_Collector": [4051],
}


def parse_item(it: dict) -> tuple[int, str, str, int, str, int, int, int, int] | None:
    loc = it.get("Loc")
    if loc is None:
        return None
    if isinstance(loc, int):
        loc = "EQP_SHOES" if loc == 64 else ("EQP_COSTUME_FLOOR" if loc == 16384 else str(loc))
    elif isinstance(loc, list):
        loc = "|".join(str(x) for x in loc)
    else:
        loc = str(loc)

    # Sex
    gender = it.get("Gender")
    if gender == "SEX_MALE":
        sex = "male"
    elif gender == "SEX_FEMALE":
        sex = "female"
    else:
        sex = "any"

    # Level
    elv = it.get("EquipLv")
    min_lv = 0
    max_lv = 0
    if isinstance(elv, int):
        min_lv = elv
    elif isinstance(elv, list) and len(elv) >= 1:
        min_lv = elv[0]
        if len(elv) >= 2:
            max_lv = elv[1]

    # Jobs
    job_spec = it.get("Job")
    jobs = []
    if isinstance(job_spec, dict):
        if not job_spec.get("All", False):
            job_set = set()
            for k, v in job_spec.items():
                if v and k in JOB_NAME_TO_IDS:
                    job_set.update(JOB_NAME_TO_IDS[k])
            jobs = sorted(list(job_set))

    job_str = "|".join(str(j) for j in jobs)
    upper = it.get("Upper", 0)
    if not isinstance(upper, int):
        upper = 0
    wlv = it.get("WeaponLv", 0) or 0
    slots = it.get("Slots", 0) or 0
    item_id = int(it["Id"])
    return (item_id, job_str, sex, min_lv, loc, max_lv, upper, wlv, slots)


def build_manifest() -> str:
    items_by_id: dict[int, tuple] = {}

    for conf_path in (ITEM_DB_CONF, ITEM_DB2_CONF):
        if not os.path.exists(conf_path):
            continue
        with open(conf_path, "r", encoding="utf-8", errors="replace") as f:
            cfg = libconf.load(f)
        for it in cfg.get("item_db", []):
            if "Id" not in it:
                continue
            parsed = parse_item(it)
            if parsed is not None:
                items_by_id[parsed[0]] = parsed

    lines = [f"# schema={SCHEMA_VERSION}"]
    for item_id in sorted(items_by_id.keys()):
        row = items_by_id[item_id]
        lines.append("\t".join(str(val) for val in row))
    lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Generate Hercules equipment eligibility manifest")
    parser.add_argument("hercules", nargs="?", help="Hercules checkout (default: ../Hercules)")
    parser.add_argument("--check", action="store_true", help="verify manifest is up to date")
    args = parser.parse_args()

    content = build_manifest()

    if args.check:
        if not os.path.exists(OUTPUT_TSV):
            print(f"Error: {OUTPUT_TSV} does not exist.", file=sys.stderr)
            sys.exit(1)
        with open(OUTPUT_TSV, "r", encoding="utf-8") as f:
            existing = f.read()
        if existing != content:
            print("Error: equipment_eligibility.tsv is out of date.", file=sys.stderr)
            sys.exit(1)
        print("OK: equipment_eligibility.tsv is up to date.")
        sys.exit(0)

    os.makedirs(os.path.dirname(OUTPUT_TSV), exist_ok=True)
    with open(OUTPUT_TSV, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    row_count = content.count("\n") - 1
    print(f"Wrote {row_count} equipment eligibility rows to {OUTPUT_TSV}")


if __name__ == "__main__":
    main()
