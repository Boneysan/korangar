#!/usr/bin/env python3
"""Export the base/job EXP-to-next-level tables and which job uses which.

Sources (Hercules):
  db/re/exp_group_db.conf   named EXP groups, `MaxLevel` and the `Exp` array
  db/re/job_db.conf         each job's BaseExpGroup / JobExpGroup
  src/map/pc.c              `pc_check_job_name`: the server's own job-name -> id table
  src/common/class*.h       `JOB_ENUM_VALUE(NAME, id, ...)`: the ids (class_special.h
                            holds the gender-split jobs such as Bard and Dancer)

Why the C tables: job_db.conf names jobs ("Swordsman", "Magician",
"Rune_Knight_Trans") by the server's own spelling, which does not match the
`Job_*` constants in constants.conf for 29 of them. The server resolves the
names through `pc_check_job_name`, so this does too, and fails if any job or
group does not resolve rather than guessing.

Per `pc_read_exp_db_sub_class`, an `Exp` array holds `MaxLevel - 1` values: the
EXP needed to advance *from* level n to n+1 is `exp[n - 1]`.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

from export_job_skills import HERCULES, source_revision

ROOT = Path(__file__).resolve().parent.parent
EXP_GROUPS = HERCULES / "db/re/exp_group_db.conf"
JOB_DB = HERCULES / "db/re/job_db.conf"
PC_SOURCE = HERCULES / "src/map/pc.c"
CLASS_HEADERS = sorted((HERCULES / "src/common").glob("class*.h"))
OUTPUT = ROOT / "docs/exp-tables.v1.json"


class ExportError(ValueError):
    pass


def strip_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//.*", "", text)


def matching_brace(text: str, opening: int) -> int:
    depth = 0
    for index in range(opening, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return index
    raise ExportError("unbalanced braces")


def named_blocks(text: str, allow_duplicates: bool = False) -> dict[str, list[str]] | dict[str, str]:
    """Top-level `Name: { ... }` blocks of a libconfig body, in order.

    With `allow_duplicates`, every name maps to the list of its bodies.
    """
    blocks: dict = {}
    position = 0
    while match := re.search(r"(?m)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*\{", text[position:]):
        opening = position + match.end() - 1
        closing = matching_brace(text, opening)
        name = match.group(1)
        body = text[opening + 1 : closing]
        if allow_duplicates:
            blocks.setdefault(name, []).append(body)
        elif name in blocks:
            raise ExportError(f"duplicate block {name}")
        else:
            blocks[name] = body
        position = closing + 1
    return blocks


def parse_groups(section: str) -> dict[str, dict[str, Any]]:
    groups: dict[str, dict[str, Any]] = {}
    for name, body in named_blocks(section).items():
        max_level = re.search(r"MaxLevel\s*:\s*(\d+)", body)
        exp = re.search(r"Exp\s*:\s*\[([^\]]*)\]", body, re.S)
        if not max_level or not exp:
            raise ExportError(f"EXP group {name} has no MaxLevel or Exp array")
        values = [int(value) for value in re.findall(r"\d+", exp.group(1))]
        # pc_read_exp_db_sub_class reads at most MaxLevel - 1 values.
        if len(values) < 1 or len(values) > int(max_level.group(1)) - 1:
            raise ExportError(f"EXP group {name}: {len(values)} values for MaxLevel {max_level.group(1)}")
        groups[name] = {"max_level": int(max_level.group(1)), "exp": values}
    return groups


def parse_exp_groups() -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    text = strip_comments(EXP_GROUPS.read_text(encoding="utf-8", errors="replace"))
    sections = named_blocks(text)
    if set(sections) != {"base_exp_group_db", "job_exp_group_db"}:
        raise ExportError(f"unexpected sections in exp_group_db.conf: {sorted(sections)}")
    return parse_groups(sections["base_exp_group_db"]), parse_groups(sections["job_exp_group_db"])


def job_name_ids() -> dict[str, int]:
    class_text = "\n".join(path.read_text(encoding="utf-8", errors="replace") for path in CLASS_HEADERS)
    enum_ids = {name: int(value) for name, value in re.findall(r"JOB_ENUM_VALUE\(\s*([A-Z0-9_]+)\s*,\s*(\d+)\s*,", class_text)}
    pc_text = PC_SOURCE.read_text(encoding="utf-8", errors="replace")
    start = pc_text.index("static int pc_check_job_name")
    table = pc_text[start : pc_text.index("};", start)]
    names: dict[str, int] = {}
    for name, constant in re.findall(r'\{\s*"([A-Za-z0-9_]+)"\s*,\s*JOB_([A-Z0-9_]+)\s*\}', table):
        if constant not in enum_ids:
            raise ExportError(f"pc_check_job_name names JOB_{constant}, which no class*.h defines")
        if name in names:
            raise ExportError(f"duplicate job name {name}")
        names[name] = enum_ids[constant]
    if len(names) < 100:
        raise ExportError(f"only {len(names)} job names parsed from pc_check_job_name")
    return names


def parse_jobs(
    base_groups: dict[str, dict[str, Any]], job_groups: dict[str, dict[str, Any]]
) -> list[dict[str, Any]]:
    text = strip_comments(JOB_DB.read_text(encoding="utf-8", errors="replace"))
    blocks = named_blocks(text, allow_duplicates=True)
    blocks.pop("Job_Name", None)
    ids = job_name_ids()
    jobs: list[dict[str, Any]] = []
    for name, bodies in blocks.items():
        # job_db.conf declares a few jobs twice (Baby_Novice). That is only
        # harmless for EXP if every copy names the same groups.
        pairs = {
            (
                (re.search(r'BaseExpGroup\s*:\s*"([^"]+)"', body) or [None, None])[1],
                (re.search(r'JobExpGroup\s*:\s*"([^"]+)"', body) or [None, None])[1],
            )
            for body in bodies
        }
        if len(pairs) != 1:
            raise ExportError(f"job {name} is declared more than once with different EXP groups: {sorted(pairs, key=str)}")
        ((base_name, job_name),) = pairs
        base = re.match(r"(.+)", base_name) if base_name else None
        job = re.match(r"(.+)", job_name) if job_name else None
        if name not in ids:
            raise ExportError(f"job_db.conf job {name} is not in pc_check_job_name")
        if not base or not job:
            raise ExportError(f"job {name} has no BaseExpGroup/JobExpGroup")
        if base.group(1) not in base_groups or job.group(1) not in job_groups:
            raise ExportError(f"job {name} names an unknown EXP group ({base.group(1)} / {job.group(1)})")
        jobs.append({"job_id": ids[name], "name": name, "base_group": base.group(1), "job_group": job.group(1)})
    seen = [job["job_id"] for job in jobs]
    if len(seen) != len(set(seen)):
        raise ExportError("two job_db.conf blocks resolve to the same job id")
    return sorted(jobs, key=lambda job: job["job_id"])


def build() -> dict[str, Any]:
    base_groups, job_groups = parse_exp_groups()
    jobs = parse_jobs(base_groups, job_groups)
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "source": [
            "db/re/exp_group_db.conf",
            "db/re/job_db.conf",
            "src/map/pc.c (pc_check_job_name)",
            "src/common/class*.h (JOB_ENUM_VALUE)",
        ],
        "rule": "exp[n - 1] is the EXP needed to advance from level n to n + 1; a group holds MaxLevel - 1 values.",
        "base_groups": base_groups,
        "job_groups": job_groups,
        "jobs": jobs,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for drift without writing")
    args = parser.parse_args()
    try:
        content = build()
    except (OSError, ValueError, KeyError) as error:
        raise SystemExit(f"cannot export EXP tables: {error}") from error
    payload = json.dumps(content, indent=1) + "\n"
    summary = f"{len(content['base_groups'])} base groups, {len(content['job_groups'])} job groups, {len(content['jobs'])} jobs"
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
