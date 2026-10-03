#!/usr/bin/env python3
"""Export the stat-point rules: the level table, per-point cost, the extra
points for upper classes, and each job's maximum stat.

Sources (Hercules):
  db/re/statpoint.txt            cumulative points by level (read by pc_readdb)
  db/re/unit_parameters_db.conf  parameter groups and MaxStats, with Inherit
  db/re/job_db.conf              each job's ParametersGroup
  src/map/pc.c                   pc_readdb (statpoint loader), pc_gets_status_point,
                                 pc_resetstate (+52 for upper classes),
                                 pc_need_status_point (cost), pc_jobid2mapid
  src/map/map.h                  MAPID_* definitions (which carry JOBL_UPPER)
  conf/map/battle/exp.conf       use_statpoint_table

Findings this encodes, each read from the C code:
  * `statp[level]` is the *cumulative* total at that level: data line n is read
    into statp[n] (the first line is level 1).
  * With `use_statpoint_table` on, a level-up gives statp[level+1] - statp[level].
  * `battle_config.max_parameter` is NOT the player cap. The cap is the job's
    `MaxStats` from unit_parameters_db.conf (default 99 when no group sets one).
  * Upper classes (JOBL_UPPER: transcendent, High Novice, trans third classes)
    get 52 extra points on a stat reset.

The exporter fails, rather than guesses, when the C code no longer contains the
expressions it restates.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

from export_exp_tables import (
    ExportError,
    job_name_ids,
    named_blocks,
    strip_comments,
)
from export_job_skills import HERCULES, source_revision
from export_server_rules import effective_settings

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "docs/stat-rules.v1.json"
STATPOINT = HERCULES / "db/re/statpoint.txt"
PARAMETERS = HERCULES / "db/re/unit_parameters_db.conf"
JOB_DB = HERCULES / "db/re/job_db.conf"
PC_SOURCE = HERCULES / "src/map/pc.c"
MAP_HEADER = HERCULES / "src/map/map.h"
CLASS_HEADERS = sorted((HERCULES / "src/common").glob("class*.h"))
MAX_LEVEL = 175  # src/common/mmo.h; statpoint.txt is read up to this many rows

COST_EXPRESSION = "(low < 100) ? (2 + (low - 1) / 10) : (16 + 4 * ((low - 100) / 5))"
UPPER_BONUS_EXPRESSION = "((sd->job & JOBL_UPPER) != 0 ? 52 : 0)"
GETS_STATUS_POINT_EXPRESSION = "return (pc->statp[level+1] - pc->statp[level]);"


def parse_statpoints() -> list[int]:
    """Read statpoint.txt the way pc_readdb does: every non-comment line is a row."""
    rows: list[int] = []
    for number, line in enumerate(STATPOINT.read_text(encoding="utf-8", errors="replace").splitlines(), start=1):
        if line.startswith("//"):
            continue
        if len(rows) >= MAX_LEVEL:
            break
        match = re.match(r"\s*(\d+)\s*$", line)
        # pc_readdb would read a blank or non-numeric line as 0 and shift every
        # later level, so refuse to export a file that does that.
        if not match:
            raise ExportError(f"statpoint.txt line {number} is not a plain number: {line!r}")
        rows.append(int(match.group(1)))
    if len(rows) != MAX_LEVEL:
        raise ExportError(f"statpoint.txt has {len(rows)} rows; expected {MAX_LEVEL}")
    if any(later < earlier for earlier, later in zip(rows, rows[1:])):
        raise ExportError("statpoint.txt is not non-decreasing")
    return rows


def parse_parameter_groups() -> dict[str, int]:
    text = strip_comments(PARAMETERS.read_text(encoding="utf-8", errors="replace"))
    groups = named_blocks(text)
    if "Base" not in groups or len(groups) < 5:
        raise ExportError(f"unit_parameters_db.conf parsed to unexpected groups: {sorted(groups)}")
    resolved: dict[str, int] = {}

    def resolve(name: str, trail: tuple[str, ...] = ()) -> int:
        if name in resolved:
            return resolved[name]
        if name in trail:
            raise ExportError(f"parameter group inheritance loop at {name}")
        body = groups[name]
        own = re.search(r"MaxStats\s*:\s*(\d+)", body)
        inherit = re.search(r'Inherit\s*:\s*"([^"]+)"', body)
        if own:
            resolved[name] = int(own.group(1))
        elif inherit:
            if inherit.group(1) not in groups:
                raise ExportError(f"{name} inherits unknown group {inherit.group(1)}")
            resolved[name] = resolve(inherit.group(1), trail + (name,))
        else:
            # status.c: entry.max_stats = 99 when neither the group nor a parent sets it.
            resolved[name] = 99
        return resolved[name]

    for name in groups:
        resolve(name)
    return resolved


def mapid_upper_flags() -> dict[str, bool]:
    """Which MAPID_* names carry JOBL_UPPER, resolving MAPID_* references."""
    text = strip_comments(MAP_HEADER.read_text(encoding="utf-8", errors="replace"))
    # Members are either `MAPID_X = <flags | MAPID_Y>,` or bare `MAPID_X,` (an
    # auto-incremented base class, which carries no JOBL_ flag).
    definitions = {
        name: (expression or "")
        for name, expression in re.findall(r"(?m)^\s*(MAPID_[A-Z0-9_]+)\s*(?:=\s*([^,\n]+))?\s*,", text)
    }

    def upper(name: str, trail: tuple[str, ...] = ()) -> bool:
        expression = definitions[name]
        if "JOBL_UPPER" in expression:
            return True
        if name in trail:
            raise ExportError(f"MAPID definition loop at {name}")
        return any(
            upper(reference, trail + (name,))
            for reference in re.findall(r"\bMAPID_[A-Z0-9_]+\b", expression)
            if reference in definitions and reference != name
        )

    return {name: upper(name) for name in definitions}


def job_constant_ids() -> dict[str, int]:
    class_text = strip_comments("\n".join(path.read_text(encoding="utf-8", errors="replace") for path in CLASS_HEADERS))
    return {name: int(value) for name, value in re.findall(r"JOB_ENUM_VALUE\(\s*([A-Z0-9_]+)\s*,\s*(\d+)\s*,", class_text)}


def upper_job_ids() -> set[int]:
    """Jobs whose pc_jobid2mapid result carries JOBL_UPPER.

    pc_jobid2mapid is partly an X-macro: the hand-written cases cover the
    gender-split jobs (class_special.h), `#include "common/class.h"` expands to
    `case JOB_<name>: return MAPID_<name>;` for every other job, and
    class_hidden.h jobs return -1 (no map id, so no flags).
    """
    pc_text = PC_SOURCE.read_text(encoding="utf-8", errors="replace")
    start = pc_text.index("static int pc_jobid2mapid")
    body = pc_text[start : pc_text.index("\n}\n", start)]
    macro_start = body.index("#define JOB_ENUM_VALUE")
    explicit_body = body[:macro_start]
    if '#include "common/class.h"' not in body or "return MAPID_ ## name;" not in body:
        raise ExportError("pc_jobid2mapid no longer expands class.h as `case JOB_name: return MAPID_name;`")

    flags = mapid_upper_flags()
    ids = job_constant_ids()
    upper: set[int] = set()
    covered = 0

    pending: list[str] = []
    for line in explicit_body.splitlines():
        pending += re.findall(r"case\s+JOB_([A-Z0-9_]+)\s*:", line)
        result = re.search(r"return\s+(MAPID_[A-Z0-9_]+)\s*;", line)
        if result:
            if result.group(1) not in flags:
                raise ExportError(f"pc_jobid2mapid returns {result.group(1)}, which map.h does not define")
            for label in pending:
                if label not in ids:
                    raise ExportError(f"pc_jobid2mapid names JOB_{label}, which no class*.h defines")
                covered += 1
                if flags[result.group(1)]:
                    upper.add(ids[label])
            pending = []

    class_text = strip_comments((HERCULES / "src/common/class.h").read_text(encoding="utf-8", errors="replace"))
    for name, value in re.findall(r"JOB_ENUM_VALUE\(\s*([A-Z0-9_]+)\s*,\s*(\d+)\s*,", class_text):
        mapid = f"MAPID_{name}"
        if mapid not in flags:
            raise ExportError(f"class.h job {name} has no {mapid} in map.h")
        covered += 1
        if flags[mapid]:
            upper.add(int(value))

    if covered < 100 or not upper:
        raise ExportError(f"pc_jobid2mapid resolved {covered} jobs and {len(upper)} upper jobs; the parser lost the table")
    return upper


def parse_jobs(groups: dict[str, int], upper_ids: set[int]) -> list[dict[str, Any]]:
    blocks = named_blocks(strip_comments(JOB_DB.read_text(encoding="utf-8", errors="replace")), allow_duplicates=True)
    blocks.pop("Job_Name", None)
    ids = job_name_ids()
    jobs: list[dict[str, Any]] = []
    for name, bodies in blocks.items():
        picked = {(re.search(r'ParametersGroup\s*:\s*"([^"]+)"', body) or [None, None])[1] for body in bodies}
        if len(picked) != 1 or None in picked:
            raise ExportError(f"job {name} has no single ParametersGroup: {sorted(picked, key=str)}")
        (group,) = picked
        if name not in ids:
            raise ExportError(f"job_db.conf job {name} is not in pc_check_job_name")
        if group not in groups:
            raise ExportError(f"job {name} names unknown parameter group {group}")
        jobs.append(
            {
                "job_id": ids[name],
                "name": name,
                "parameters_group": group,
                "max_stats": groups[group],
                "upper": ids[name] in upper_ids,
            }
        )
    return sorted(jobs, key=lambda job: job["job_id"])


def check_source_expressions() -> None:
    pc_text = PC_SOURCE.read_text(encoding="utf-8", errors="replace")
    squashed = re.sub(r"\s+", " ", pc_text)
    for label, expression in (
        ("per-point cost (pc_need_status_point)", COST_EXPRESSION),
        ("upper-class bonus (pc_resetstate)", UPPER_BONUS_EXPRESSION),
        ("level-up gain (pc_gets_status_point)", GETS_STATUS_POINT_EXPRESSION),
    ):
        if expression not in squashed:
            raise ExportError(f"pc.c no longer contains the {label} expression this exporter restates: {expression}")
    if "pc->statp[i]=stat;" not in squashed or "pc->statp[0] = 45;" not in squashed:
        raise ExportError("pc.c no longer reads statpoint.txt into statp[1..] as this exporter assumes")


def build() -> dict[str, Any]:
    check_source_expressions()
    battle = effective_settings(HERCULES / "conf/map/battle.conf")
    use_table = battle.get("use_statpoint_table", {}).get("value")
    if use_table is not True:
        raise ExportError(f"use_statpoint_table is {use_table!r}; the exported table is only the rule when it is true")
    groups = parse_parameter_groups()
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "source": [
            "db/re/statpoint.txt",
            "db/re/unit_parameters_db.conf",
            "db/re/job_db.conf",
            "src/map/pc.c (pc_readdb, pc_gets_status_point, pc_resetstate, pc_need_status_point, pc_jobid2mapid)",
            "src/map/map.h (MAPID_*)",
            "conf/map/battle/exp.conf (use_statpoint_table)",
        ],
        "rule": "points_at_level[n - 1] is the cumulative stat points a character has at level n; a level-up from n to n + 1 grants points_at_level[n] - points_at_level[n - 1].",
        "points_at_level": parse_statpoints(),
        "upper_class_extra_points": 52,
        "cost_to_raise_from": {
            "below_100": "2 + (value - 1) / 10",
            "from_100": "16 + 4 * ((value - 100) / 5)",
            "note": "integer division; the cost of the point that takes a stat from `value` to `value + 1`",
        },
        "parameter_groups": groups,
        "jobs": parse_jobs(groups, upper_job_ids()),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for drift without writing")
    args = parser.parse_args()
    try:
        content = build()
    except (OSError, ValueError, KeyError, StopIteration) as error:
        raise SystemExit(f"cannot export stat rules: {error}") from error
    payload = json.dumps(content, indent=1) + "\n"
    summary = f"{len(content['points_at_level'])} levels, {len(content['jobs'])} jobs, {sum(job['upper'] for job in content['jobs'])} upper"
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != payload:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {summary}")
        return 0
    OUTPUT.write_text(payload, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({summary})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
