#!/usr/bin/env python3
"""Export verified non-story rumors for the Adventure Guide.

Non-story rumors are account-wide typed discoveries (GDD §§9.5-9.7, 9.10)
derived from NPC dialogue, notice boards, signs, books, and exploration leads.
They provide map notes, leads on monsters or items, and general world clues
without creating kill-count obligations or revealing story campaign spoilers.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from export_item_grant_reference import source_revision

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "docs" / "rumors.v1.json"

AUTHORED_RUMORS: list[dict[str, Any]] = [
    {
        "id": 1,
        "title": "Unusual Sea Creatures of Byalan",
        "category": "Creatures",
        "text": "Adventurers and sailors at Izlude speak of bizarre marine life and walking starfish lurking in the sunken caves beneath Byalan Island.",
        "map_name": "izlude",
        "coordinates": [198, 213],
        "source_location": "Izlude Sailor by the ferry dock",
        "related_monster_id": 1068,
        "related_item_id": None,
        "is_story_spoiler": False,
        "evidence_state": "verified",
    },
    {
        "id": 2,
        "title": "Morroc Merchant Ant Jaw Demand",
        "category": "Commerce",
        "text": "Desert merchants gathering near Morroc are offering high bounties for sturdy Ant Jaws gathered from the subterranean chambers of Ant Hell.",
        "map_name": "morroc",
        "coordinates": [160, 145],
        "source_location": "Morroc Merchant Notice Board",
        "related_monster_id": 1094,
        "related_item_id": 907,
        "is_story_spoiler": False,
        "evidence_state": "verified",
    },
    {
        "id": 3,
        "title": "Payon Cave Whispering Depths",
        "category": "Dungeons",
        "text": "Villagers in Payon warn young travelers that mournful whispers echo from the deeper subterranean levels of Payon Cave where the restless dead wander.",
        "map_name": "payon",
        "coordinates": [155, 230],
        "source_location": "Payon Village Elder lore",
        "related_monster_id": 1015,
        "related_item_id": None,
        "is_story_spoiler": False,
        "evidence_state": "verified",
    },
    {
        "id": 4,
        "title": "Geffen Tower Magical Metals",
        "category": "Research",
        "text": "Wizards atop the Geffen Tower study the alchemical properties of Oridecon and Phracon, claiming pure minerals amplify enchanted bladecraft.",
        "map_name": "geffen",
        "coordinates": [120, 110],
        "source_location": "Geffen Mage Guild notice",
        "related_monster_id": None,
        "related_item_id": 984,
        "is_story_spoiler": False,
        "evidence_state": "verified",
    },
    {
        "id": 5,
        "title": "Sunken Ship of Treasure Island",
        "category": "Exploration",
        "text": "Seasoned sailors along the Alberta docks trade rumors of an eerie ghost ship stranded off the coast of Alberta, filled with cursed chests and undead buccaneers.",
        "map_name": "alberta",
        "coordinates": [188, 142],
        "source_location": "Alberta Harbor Sailor tales",
        "related_monster_id": 1071,
        "related_item_id": None,
        "is_story_spoiler": False,
        "evidence_state": "verified",
    },
    {
        "id": 6,
        "title": "Prontera Culvert Infestation",
        "category": "Bounty",
        "text": "The Prontera City Guard recruits novice adventurers to clear the sewer network west of the capital, reporting aggressive thief bugs and oversized vermin.",
        "map_name": "prt_fild05",
        "coordinates": [270, 212],
        "source_location": "Knight Recruitment Officer",
        "related_monster_id": 1017,
        "related_item_id": None,
        "is_story_spoiler": False,
        "evidence_state": "verified",
    },
    {
        "id": 7,
        "title": "Ancient Ruins of Glast Heim",
        "category": "Folklore",
        "text": "Scholars whisper of a fallen kingdom northwest of Geffen known as Glast Heim, where crumbling battlements and forgotten archives guard long-lost relics.",
        "map_name": "glast_01",
        "coordinates": [200, 200],
        "source_location": "Geffen Scholar journal notes",
        "related_monster_id": None,
        "related_item_id": None,
        "is_story_spoiler": False,
        "evidence_state": "verified",
    },
    {
        "id": 8,
        "title": "Trans-Continental Flying Airship",
        "category": "Travel",
        "text": "Engineers from Schwarzwald and Midgarts maintain regular international airship flights departing from the Izlude and Yuno air terminals.",
        "map_name": "izlude",
        "coordinates": [205, 75],
        "source_location": "Izlude Airport Receptionist",
        "related_monster_id": None,
        "related_item_id": None,
        "is_story_spoiler": False,
        "evidence_state": "verified",
    },
]


def build() -> dict[str, Any]:
    revision, dirty = source_revision()
    seen_ids = set()
    for entry in AUTHORED_RUMORS:
        entry_id = entry["id"]
        if entry_id in seen_ids:
            raise ValueError(f"duplicate rumor id: {entry_id}")
        seen_ids.add(entry_id)
        if entry["is_story_spoiler"]:
            raise ValueError(f"rumor {entry_id} must not contain story campaign spoilers")
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "entries": AUTHORED_RUMORS,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for drift without writing generated files")
    args = parser.parse_args()

    content = build()
    payload = json.dumps(content, indent=2) + "\n"

    if args.check:
        if not OUTPUT.exists():
            print(f"missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        existing = OUTPUT.read_text(encoding="utf-8")
        if existing != payload:
            print(f"drift detected: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {len(content['entries'])} rumors", flush=True)
        return 0

    OUTPUT.write_text(payload, encoding="utf-8")
    print(f"wrote: {OUTPUT.relative_to(ROOT)} ({len(content['entries'])} rumors)", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
