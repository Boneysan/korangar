#!/usr/bin/env python3
"""Export structured weapon/cooking production, arrow conversion recipes, and item combos."""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
PRODUCE = HERCULES / "db/produce_db.txt"
ARROWS = HERCULES / "db/create_arrow_db.txt"
COMBOS = HERCULES / "db/re/item_combo_db.conf"
OUTPUT = ROOT / "docs/crafting.v1.json"


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL).strip()
        # Use diff-index instead of status --porcelain to ignore untracked files.
        # This ensures source_worktree_dirty reflects only tracked-source changes,
        # not new tooling or other uncommitted untracked items.
        dirty = subprocess.run(
            ["git", "-C", str(HERCULES), "diff-index", "--quiet", "HEAD"],
            capture_output=True,
            text=True,
        ).returncode != 0
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def rows(path: Path):
    for number, raw in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        line = raw.split("//", 1)[0].strip()
        if line:
            yield number, [value.strip() for value in line.split(",")]


def parse_combo_name(item_names: list[str]) -> str:
    """Generate a unique name for a combo from its member item names."""
    return "_".join(sorted(item_names))


def parse_combos() -> tuple[dict[str, Any], dict[str, list[str]]]:
    """Parse Hercules item_combo_db.conf and return combo entries with reverse links."""
    content = COMBOS.read_text(encoding="utf-8", errors="replace")

    # Find all { ... } blocks containing Items arrays
    block_pattern = r'\{([^}]+)\}'
    combos = {}
    items_to_combos: dict[str, list[str]] = {}  # aegis_name -> list of combo names

    for match in re.finditer(block_pattern, content):
        block = match.group(1)

        # Extract Items array - looks like: Items: ["item1", "item2"]
        items_match = re.search(r'Items:\s*\[([^\]]*)\]', block)
        if not items_match:
            continue

        items_str = items_match.group(1)
        # Parse individual item names from the array
        item_names = re.findall(r'"([^"]+)"', items_str)
        if len(item_names) < 2:
            continue

        # Extract Script content - looks like: Script: <"...">
        script_match = re.search(r'Script:\s*<["\']?([^"\']*)["\']?>', block, re.DOTALL)
        script_content = script_match.group(1).strip() if script_match else ""

        # Generate combo name from sorted item names
        combo_name = parse_combo_name(item_names)

        combos[combo_name] = {
            "members": item_names,
            "script": script_content
        }

        # Build reverse lookup: for each member, track which combos it belongs to
        for item_name in item_names:
            if item_name not in items_to_combos:
                items_to_combos[item_name] = []
            items_to_combos[item_name].append(combo_name)

    return combos, items_to_combos


def build() -> dict[str, Any]:
    items_path = ROOT / "docs/items.v1.json"
    items_data = json.loads(items_path.read_text(encoding="utf-8"))
    items = items_data["entries"]
    names = {int(item["id"]): item.get("name") or item["aegis_name"] for item in items}
    aegis_from_id = {int(item["id"]): item["aegis_name"] for item in items}

    # Build reverse lookup: aegis_name -> item_id
    item_ids_by_aegis = {item["aegis_name"]: int(item["id"]) for item in items if "aegis_name" in item}

    skills = json.loads((ROOT / "docs/skills.json").read_text(encoding="utf-8"))
    skill_names = {int(skill["Id"]): skill.get("Name", f"Skill {skill['Id']}") for skill in skills}

    entries = []

    # Parse and add combo entries first, with their reverse links to items
    combos, items_to_combos = parse_combos()

    for combo_name, combo_data in combos.items():
        combo_entry = {
            "kind": "combo",
            "name": combo_name,
            "members": combo_data["members"],
            "script": combo_data["script"],
            "source": {"path": "db/re/item_combo_db.conf", "record": f"parsed from config"}
        }
        entries.append(combo_entry)

    # Now add production and arrow entries with combo links from items
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
        output_aegis = aegis_from_id.get(output_id, "")
        entry = {"kind": "production", "output_id": output_id, "output_name": names.get(output_id, f"Unknown item {output_id}"), "output_amount": 1, "item_level": item_level, "skill_id": skill_id, "skill_name": skill_names.get(skill_id, f"Skill {skill_id}"), "skill_level": skill_level, "materials": materials, "source": {"path": "db/produce_db.txt", "record": f"line {number}"}}
        # Add combo links if any member items belong to combos
        all_member_aegis = [aegis_from_id.get(int(m["item_id"]), "") for m in materials]
        all_member_aegis.append(output_aegis)
        linked_combos = []
        seen_combos = set()
        for aegis in all_member_aegis:
            if aegis in items_to_combos:
                for combo_name in items_to_combos[aegis]:
                    if combo_name not in seen_combos:
                        seen_combos.add(combo_name)
                        linked_combos.append(combo_name)
        if linked_combos:
            entry["combos"] = sorted(linked_combos)
        entries.append(entry)

    for number, fields in rows(ARROWS):
        if len(fields) < 3 or not fields[0].isdigit():
            continue
        source_id = int(fields[0])
        for pos in range(1, len(fields) - 1, 2):
            if not fields[pos].isdigit() or not fields[pos + 1].isdigit():
                continue
            output_id, amount = int(fields[pos]), int(fields[pos + 1])
            if output_id and amount:
                entry = {"kind": "arrow_conversion", "output_id": output_id, "output_name": names.get(output_id, f"Unknown item {output_id}"), "output_amount": amount, "materials": [{"item_id": source_id, "item_name": names.get(source_id, f"Unknown item {source_id}"), "amount": 1, "required": True}], "source": {"path": "db/create_arrow_db.txt", "record": f"line {number}"}}
                # Add combo links for both source and output
                linked_combos = []
                seen_combos = set()
                for item_id in [source_id, output_id]:
                    aegis = aegis_from_id.get(item_id, "")
                    if aegis in items_to_combos:
                        for combo_name in items_to_combos[aegis]:
                            if combo_name not in seen_combos:
                                seen_combos.add(combo_name)
                                linked_combos.append(combo_name)
                if linked_combos:
                    entry["combos"] = sorted(linked_combos)
                entries.append(entry)

    # The item exporter now includes belongs_to_combos, so we don't modify items.v1.json
    # here. We just use it for reading combo links to add to production/arrow entries.

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
