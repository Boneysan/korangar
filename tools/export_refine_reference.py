#!/usr/bin/env python3
"""Export Whitesmith/Mechanic self-refine (`WS_WEAPONREFINE`) odds from Hercules.

GDD 11.6: "Before an attempt, the UI must clearly show success chance if the
server exposes it, required materials, cost, and exact failure consequence."
`WeaponRefineWindow` currently shows neither -- this is the data half of
closing that gap.

This is deliberately narrow: it covers only the skill-triggered self-refine
path (`skill_weaponrefine`, `src/map/skill.c`), which is what
`RequestWeaponRefinePacket` (wire header 0x0222) actually drives on this
server. It is NOT the newer NPC-driven "Refinery UI" system in the same
`refine_db.conf` file (`RefineryUISettings`) -- that system is disabled here
(`conf/map/battle/feature.conf: enable_refinery_ui: false`) and unreachable
through this packet, so exporting its cost/blessing/failure-behavior data
would document a path players cannot take.

Per `skill_weaponrefine`:
- Material consumed (exactly 1, by the weapon's own `WeaponLv` 1-4): Phracon,
  Emveretarcon, Oridecon, Oridecon.
- `refine_db.conf`'s `WeaponLevel<N>.Rates.Lv<target>.NormalChance` is a
  percentage (default 100 for any unlisted level). Hercules multiplies it by
  10 for its per-thousand roll, then adds `+5 * (job_level - 50)` per-thousand
  for every class except Mechanic (Transcendent), which gets a flat `+100`.
  Those bonuses are respectively 0.5 percentage points per job level and 10
  percentage points. This exporter records the base percentages and bonus
  units separately so the client can calculate the live character's result.
- On failure: `pc->delitem` -- the weapon is unequipped and destroyed
  outright, not downgraded.
- The skill cannot refine at or above its own skill level, or above +10,
  whichever is lower (`item->refine >= sd->menuskill_val || item->refine >= 10`).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from export_job_skills import HERCULES, source_revision

ROOT = Path(__file__).resolve().parent.parent
REFINE_DB = HERCULES / "db/re/refine_db.conf"
OUTPUT = ROOT / "docs/refine.v1.json"

# `ditem->wlv` (weapon level, 1-4) -> AegisName of the material `skill_weaponrefine`
# searches the caster's inventory for. Hardcoded to Hercules' own `material[]`
# array in `skill_weaponrefine`, not derived from `refine_db.conf` -- that file
# has no such table, because official RO's NPC-driven Refinery UI (which the
# file mostly documents) takes its material list from a different, disabled
# path (`RefineryUISettings.Items`).
MATERIAL_BY_WEAPON_LEVEL = {
    1: "Phracon",
    2: "Emveretarcon",
    3: "Oridecon",
    4: "Oridecon",
}

# The skill can never usefully refine past +10 (`item->refine >= 10` in
# `skill_weaponrefine`), regardless of the caster's own skill level.
MAX_USEFUL_REFINE_LEVEL = 10


def parse_weapon_level_chances(text: str) -> dict[int, dict[int, int]]:
    """`{weapon_level: {target_refine_level: percent_chance}}` for levels 1-10.

    Any level `refine_db.conf` does not list for a given weapon level
    defaults to 100, per that file's own documented convention ("Refine
    levels that use default values need not be listed").
    """
    # Strip `//` comments the same way every other line-based exporter in
    # this tree does, so a rate commented out for testing is not read live.
    lines = [line.split("//", 1)[0] for line in text.splitlines()]
    stripped = "\n".join(lines)

    chances: dict[int, dict[int, int]] = {}
    for weapon_level in range(1, 5):
        block_match = re.search(rf"^WeaponLevel{weapon_level}:\s*\{{", stripped, re.MULTILINE)
        if not block_match:
            raise ValueError(f"WeaponLevel{weapon_level} block not found")
        block_text = _matching_brace_block(stripped, block_match.end() - 1)

        rates_match = re.search(r"\bRates:\s*\{", block_text)
        if not rates_match:
            raise ValueError(f"WeaponLevel{weapon_level}.Rates block not found")
        rates_text = _matching_brace_block(block_text, rates_match.end() - 1)

        levels = {level: 100 for level in range(1, MAX_USEFUL_REFINE_LEVEL + 1)}
        for level_match in re.finditer(r"\bLv(\d+):\s*\{", rates_text):
            level = int(level_match.group(1))
            if level > MAX_USEFUL_REFINE_LEVEL:
                continue
            level_text = _matching_brace_block(rates_text, level_match.end() - 1)
            normal_match = re.search(r"\bNormalChance:\s*(\d+)", level_text)
            if normal_match:
                levels[level] = int(normal_match.group(1))
        chances[weapon_level] = levels
    return chances


def _matching_brace_block(text: str, open_brace_index: int) -> str:
    """The text strictly between the `{` at `open_brace_index` and its match."""
    if text[open_brace_index] != "{":
        raise ValueError(f"expected '{{' at index {open_brace_index}")
    depth = 0
    for index in range(open_brace_index, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[open_brace_index + 1 : index]
    raise ValueError("unterminated block: no matching '}' found")


def build() -> dict[str, object]:
    chances = parse_weapon_level_chances(REFINE_DB.read_text(encoding="utf-8", errors="replace"))
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "source": "db/re/refine_db.conf (WeaponLevel1-4.Rates), src/map/skill.c (skill_weaponrefine material list and formula)",
        "scope": "WS_WEAPONREFINE self-refine only (packet 0x0222); the NPC-driven Refinery UI in the same file is disabled on this server",
        "max_useful_refine_level": MAX_USEFUL_REFINE_LEVEL,
        "job_level_bonus_per_job_level_from_50_per_mille": 5,
        "mechanic_transcendent_flat_bonus_percent": 10,
        "on_failure": "weapon is unequipped and destroyed",
        "weapon_levels": [
            {
                "weapon_level": weapon_level,
                "material": MATERIAL_BY_WEAPON_LEVEL[weapon_level],
                # Percent chance to advance FROM (level - 1) TO level, using a
                # caster with no job-level bonus (job_level 50). The client
                # adds the job-level term itself, since that depends on the
                # live character, not static data.
                "base_chance_percent_by_target_level": chances[weapon_level],
            }
            for weapon_level in range(1, 5)
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check generated-data drift")
    args = parser.parse_args()
    if not REFINE_DB.exists():
        raise SystemExit(f"required Hercules source missing: {REFINE_DB}")
    try:
        data = build()
        rendered = json.dumps(data, indent=1, ensure_ascii=False) + "\n"
    except ValueError as error:
        raise SystemExit(f"cannot export refine reference: {error}") from error
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != rendered:
            raise SystemExit(f"{OUTPUT.relative_to(ROOT)} is stale — re-run {Path(__file__).name}")
        print(f"up to date: {OUTPUT.relative_to(ROOT)}")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} (4 weapon levels x {MAX_USEFUL_REFINE_LEVEL} levels)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
