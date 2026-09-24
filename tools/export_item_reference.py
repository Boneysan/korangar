#!/usr/bin/env python3
"""Export versioned item/card reference data from the configured Hercules DB.

The legacy `items.json` and `cards.json` have no reproducible original
exporter. This emits parallel v1 files so those consumers remain untouched:
`docs/items.v1.json` and `docs/cards.v1.json`.

Raw Hercules item scripts are code, not player-facing effect descriptions.
They are deliberately omitted; entries with scripts are explicitly marked
`effect_status: "scripted_not_translated"`.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from export_skill_info import parse_skill_db


ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
ITEM_SOURCES = (HERCULES / "db/re/item_db.conf", HERCULES / "db/item_db2.conf")
MOB_SOURCE = HERCULES / "db/re/mob_db.conf"
ITEMS_OUTPUT = ROOT / "docs/items.v1.json"
CARDS_OUTPUT = ROOT / "docs/cards.v1.json"
SCRIPT_FIELDS = ("Script", "OnEquipScript", "OnUnequipScript", "OnRentalStartScript", "OnRentalEndScript")
SCRIPT_FIELD = re.compile(r"\b(" + "|".join(SCRIPT_FIELDS) + r"):" + r"\s*<\".*?\">", re.S)

ITEM_FIELDS = (
    "Type", "Buy", "Sell", "Weight", "Atk", "Matk", "Def", "Range", "Slots",
    "Job", "Gender", "Loc", "WeaponLv", "EquipLv", "Refine",
)


def read_records(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        raise ValueError(f"required Hercules source is missing: {path}")
    text = path.read_text(encoding="utf-8", errors="replace")
    # Scripts use Hercules' <" ... "> syntax, which is not libconfig syntax.
    # Replace each complete code value before passing the remaining data to
    # the shared parser; track only presence, never expose it as prose.
    scrubbed, count = SCRIPT_FIELD.subn(lambda match: f'{match.group(1)}: "__SCRIPT_PRESENT__"', text)
    starts = len(re.findall(r"\b(?:" + "|".join(SCRIPT_FIELDS) + r"):" + r"\s*<\"", text))
    if starts != count:
        raise ValueError(f"could not safely delimit every script value in {path}: {starts} starts, {count} complete values")
    return parse_skill_db(scrubbed)


def check_unique(records: list[dict[str, Any]], field: str, path: Path) -> None:
    values = [row.get(field) for row in records]
    if any(value is None for value in values):
        raise ValueError(f"missing {field} in {path}")
    duplicate_counts = Counter(values)
    if any(count > 1 for count in duplicate_counts.values()):
        duplicates = sorted(value for value, count in duplicate_counts.items() if count > 1)
        raise ValueError(f"duplicate {field} values in {path}: {duplicates[:20]}")


def source_revision() -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(
            ["git", "-C", str(HERCULES), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL
        ).strip()
        dirty = bool(subprocess.check_output(["git", "-C", str(HERCULES), "status", "--porcelain"], text=True).strip())
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def build() -> tuple[dict[str, Any], dict[str, Any]]:
    merged: dict[int, tuple[dict[str, Any], Path]] = {}
    for path in ITEM_SOURCES:
        rows = read_records(path)
        check_unique(rows, "Id", path)
        for row in rows:
            # Hercules loads item_db2 after the renewal DB; it intentionally
            # overrides matching IDs and can introduce server-only records.
            merged[int(row["Id"])] = (row, path)

    mobs = read_records(MOB_SOURCE)
    check_unique(mobs, "Id", MOB_SOURCE)
    check_unique(mobs, "SpriteName", MOB_SOURCE)
    item_by_aegis: dict[str, int] = {}
    for item_id, (row, path) in merged.items():
        aegis_name = row.get("AegisName")
        if not isinstance(aegis_name, str) or not aegis_name:
            raise ValueError(f"item {item_id} in {path} has no AegisName")
        if aegis_name in item_by_aegis:
            raise ValueError(f"duplicate AegisName {aegis_name!r} across merged item sources")
        item_by_aegis[aegis_name] = item_id

    drops_by_item: dict[int, list[dict[str, Any]]] = defaultdict(list)
    unresolved: list[str] = []
    mob_path = MOB_SOURCE.relative_to(HERCULES).as_posix()
    for mob in mobs:
        for drop_kind, field in (("normal", "Drops"), ("mvp", "MvpDrops")):
            for aegis_name, raw_rate in (mob.get(field) or {}).items():
                rate = raw_rate[0] if isinstance(raw_rate, list) and raw_rate else raw_rate
                item_id = item_by_aegis.get(aegis_name)
                if item_id is None:
                    unresolved.append(f"{mob.get('SpriteName')}:{field}:{aegis_name}")
                    continue
                if not isinstance(rate, int) or isinstance(rate, bool) or rate < 0:
                    raise ValueError(f"invalid drop rate for {mob.get('SpriteName')} -> {aegis_name}: {rate!r}")
                drops_by_item[item_id].append({
                    "monster_id": int(mob["Id"]),
                    "sprite_name": mob["SpriteName"],
                    "rate_per_10000": rate,
                    "kind": drop_kind,
                    "source_record": f"{mob_path}:Id={mob['Id']}",
                })
    if unresolved:
        raise ValueError(f"{len(unresolved)} dangling mob drop references; examples: {unresolved[:10]}")

    item_path = ITEM_SOURCES[0].relative_to(HERCULES).as_posix()
    item_entries: list[dict[str, Any]] = []
    card_entries: list[dict[str, Any]] = []
    for item_id in sorted(merged):
        row, source_path = merged[item_id]
        entry: dict[str, Any] = {
            "id": item_id,
            "aegis_name": row["AegisName"],
            "name": row.get("Name"),
            "source": {
                "path": source_path.relative_to(HERCULES).as_posix(),
                "record": f"Id={item_id}",
            },
            "drops_from": sorted(drops_by_item[item_id], key=lambda drop: (drop["monster_id"], drop["kind"])),
            "effect_status": "scripted_not_translated"
            if any(row.get(field) == "__SCRIPT_PRESENT__" for field in SCRIPT_FIELDS)
            else "no_script_field",
        }
        for field in ITEM_FIELDS:
            if field in row and row[field] != "__SCRIPT_PRESENT__":
                entry[field.lower()] = row[field]
        item_entries.append(entry)
        if row.get("Type") == "IT_CARD":
            card_entries.append(entry)

    revision, dirty = source_revision()
    metadata = {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "sources": [path.relative_to(HERCULES).as_posix() for path in (*ITEM_SOURCES, MOB_SOURCE)],
    }
    return ({**metadata, "entries": item_entries}, {**metadata, "entries": card_entries})


def render(payload: dict[str, Any]) -> str:
    return json.dumps(payload, indent=1, ensure_ascii=False) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check both generated files without writing them")
    args = parser.parse_args()
    try:
        items, cards = build()
    except (OSError, ValueError, StopIteration) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    outputs = ((ITEMS_OUTPUT, items), (CARDS_OUTPUT, cards))
    if args.check:
        stale = [path for path, payload in outputs if not path.is_file() or path.read_text(encoding="utf-8") != render(payload)]
        if stale:
            for path in stale:
                print(f"stale: {path} — re-run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"up to date: {len(items['entries'])} items, {len(cards['entries'])} cards")
        return 0

    for path, payload in outputs:
        path.write_text(render(payload), encoding="utf-8")
    print(f"wrote {ITEMS_OUTPUT} ({len(items['entries'])} items)")
    print(f"wrote {CARDS_OUTPUT} ({len(cards['entries'])} cards)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
