#!/usr/bin/env python3
"""Export broad spawn rectangles and density classifications from static monster placements.

Writes `docs/spawn-rectangles.v1.json` as a versioned reference artifact. Broad rectangles
represent general population regions without revealing exact runtime spawn cells or timers.

Usage:
    tools/export_spawn_rectangles.py [--check]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
BESTIARY_FILE = ROOT / "docs/bestiary.v1.json"
OUTPUT_FILE = ROOT / "docs/spawn-rectangles.v1.json"


def build() -> dict[str, Any]:
    if not BESTIARY_FILE.is_file():
        raise FileNotFoundError(f"Missing bestiary reference: {BESTIARY_FILE}")

    bestiary = json.loads(BESTIARY_FILE.read_text(encoding="utf-8"))
    rectangles: list[dict[str, Any]] = []
    maps: set[str] = set()
    monsters: set[int] = set()

    for monster in bestiary.get("entries", []):
        mob_id = int(monster["id"])
        mob_name = monster.get("name", "")

        for region in monster.get("spawn_regions", []):
            map_name = region["map"]
            for placement in region.get("placements", []):
                x = int(placement["x"])
                y = int(placement["y"])
                xs = int(placement["x_spread"])
                ys = int(placement["y_spread"])
                amount = int(placement["amount"])
                is_map_wide = bool(placement.get("random_map_cell", False) or (x == 0 and y == 0))

                if is_map_wide:
                    rect_x = 0
                    rect_y = 0
                    rect_w = 0
                    rect_h = 0
                    density = "high" if amount >= 20 else ("medium" if amount >= 6 else "low")
                else:
                    rect_x = max(0, x - xs)
                    rect_y = max(0, y - ys)
                    rect_w = max(1, 2 * xs + 1)
                    rect_h = max(1, 2 * ys + 1)
                    density = "high" if amount >= 15 else ("medium" if amount >= 5 else "low")

                rectangles.append({
                    "map": map_name,
                    "monster_id": mob_id,
                    "monster_name": mob_name,
                    "x": rect_x,
                    "y": rect_y,
                    "width": rect_w,
                    "height": rect_h,
                    "density": density,
                    "is_map_wide": is_map_wide,
                })
                maps.add(map_name)
                monsters.add(mob_id)

    rectangles.sort(key=lambda r: (r["map"], r["monster_id"], r["is_map_wide"], r["x"], r["y"]))

    return {
        "version": 1,
        "sources": ["docs/bestiary.v1.json"],
        "disclosure_guarantee": (
            "broad population bounding boxes and qualitative density only; "
            "exact respawn cells and timers are not revealed"
        ),
        "summary": {
            "total_rectangles": len(rectangles),
            "maps_covered": len(maps),
            "monsters_covered": len(monsters),
        },
        "entries": rectangles,
    }


def render(payload: dict[str, Any]) -> str:
    return json.dumps(payload, indent=1, ensure_ascii=False) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for drift without writing generated files")
    args = parser.parse_args()

    try:
        payload = build()
    except Exception as error:  # noqa: BLE001
        print(f"error: {error}", file=sys.stderr)
        return 1

    rendered = render(payload)
    if args.check:
        if not OUTPUT_FILE.is_file() or OUTPUT_FILE.read_text(encoding="utf-8") != rendered:
            print(f"stale: {OUTPUT_FILE} — re-run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"up to date: {payload['summary']['total_rectangles']} spawn rectangles")
        return 0

    OUTPUT_FILE.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT_FILE} ({payload['summary']['total_rectangles']} spawn rectangles)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
