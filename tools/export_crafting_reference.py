#!/usr/bin/env python3
"""Export structured weapon/cooking production and arrow conversion recipes."""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
PRODUCE = HERCULES / "db/produce_db.txt"
ARROWS = HERCULES / "db/create_arrow_db.txt"
OUTPUT = ROOT / "docs/crafting.v1.json"


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL).strip()
        dirty = bool(subprocess.check_output(["git", "-C", str(HERCULES), "status", "--porcelain"], text=True).strip())
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def rows(path: Path):
    for number, raw in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        line = raw.split("//", 1)[0].strip()
        if line:
            yield number, [value.strip() for value in line.split(",")]


def build() -> dict[str, Any]:
    items_path = ROOT / "docs/items.v1.json"
    items = json.loads(items_path.read_text(encoding="utf-8"))["entries"]
    names = {int(item["id"]): item.get("name") or item["aegis_name"] for item in items}
    skills = json.loads((ROOT / "docs/skills.json").read_text(encoding="utf-8"))
    skill_names = {int(skill["Id"]): skill.get("Name", f"Skill {skill['Id']}") for skill in skills}
    entries = []
    for number, fields in rows(PRODUCE):
        if len(fields) < 4 or not all(value.lstrip("-").isdigit() for value in fields[:4]):
            continue
        output_id, item_level, skill_id, skill_level = map(int, fields[:4])
        materials = []
        for pos in range(4, len(fields) - 1, 2):
            if not fields[pos].isdigit() or not fields[pos + 1].lstrip("-").isdigit():
                continue
            item_id, amount = int(fields[pos]), int(fields[pos + 1])
            if item_id:
                materials.append({"item_id": item_id, "item_name": names.get(item_id, f"Unknown item {item_id}"), "amount": amount, "required": amount > 0})
        entries.append({"kind": "production", "output_id": output_id, "output_name": names.get(output_id, f"Unknown item {output_id}"), "output_amount": 1, "item_level": item_level, "skill_id": skill_id, "skill_name": skill_names.get(skill_id, f"Skill {skill_id}"), "skill_level": skill_level, "materials": materials, "source": {"path": "db/produce_db.txt", "record": f"line {number}"}})
    for number, fields in rows(ARROWS):
        if len(fields) < 3 or not fields[0].isdigit():
            continue
        source_id = int(fields[0])
        for pos in range(1, len(fields) - 1, 2):
            if not fields[pos].isdigit() or not fields[pos + 1].isdigit():
                continue
            output_id, amount = int(fields[pos]), int(fields[pos + 1])
            if output_id and amount:
                entries.append({"kind": "arrow_conversion", "output_id": output_id, "output_name": names.get(output_id, f"Unknown item {output_id}"), "output_amount": amount, "materials": [{"item_id": source_id, "item_name": names.get(source_id, f"Unknown item {source_id}"), "amount": 1, "required": True}], "source": {"path": "db/create_arrow_db.txt", "record": f"line {number}"}})
    revision, dirty = source_revision()
    return {"schema_version": 1, "source_revision": revision, "source_worktree_dirty": dirty, "mode": "renewal", "entries": entries}


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
        print(f"wrote {OUTPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
