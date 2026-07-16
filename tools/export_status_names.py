#!/usr/bin/env python3
"""Export Hercules status-icon constants to `docs/status_effects.json`.

Why: the buff bar renders the raw server status index ("10:218s"), so players
cannot tell what is buffed (M1-010). This produces an index -> English name map
the client embeds.

Source is Hercules' own `db/constants.conf` (GPL, in-tree) rather than the
official client's efst/stateicon tables, which keeps us clear of the
No-Upstream-IP rule in CLAUDE.md.

Names are humanised from the constant (`SI_BLESSING` -> "Blessing"). Many RO
constants are run-together compounds that humanise badly (`SI_TWOHANDQUICKEN` ->
"Twohandquicken"), so OVERRIDES below fixes the ones a player actually sees.
Anything not overridden still beats a bare integer, and overrides can be added
incrementally.

Usage:
    tools/export_status_names.py [--check]

    --check  exit 1 if docs/status_effects.json is stale (for CI)
"""

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CONSTANTS = REPO.parent / "Hercules" / "db" / "constants.conf"
OUTPUT = REPO / "docs" / "status_effects.json"

# Constants whose humanised form is wrong or unreadable. Keyed by the constant
# name minus the `SI_` prefix. Extend freely — anything absent falls back to the
# humaniser.
OVERRIDES = {
    "TWOHANDQUICKEN": "Two-Hand Quicken",
    "INC_AGI": "Increase AGI",
    "DEC_AGI": "Decrease AGI",
    "POSTDELAY": "Skill Delay",
    "SLOWPOISON": "Slow Poison",
    "POISONREACT": "Poison React",
    "ENCHANTPOISON": "Enchant Poison",
    "LEXAETERNA": "Lex Aeterna",
    "LEXDIVINA": "Lex Divina",
    "WEAPONPERFECT": "Weapon Perfection",
    "MAGICROD": "Magic Rod",
    "AUTOGUARD": "Auto Guard",
    "REFLECTSHIELD": "Reflect Shield",
    "DEVOTION": "Devotion",
    "PROVIDENCE": "Providence",
    "AUTOSPELL": "Auto Spell",
    "STEELBODY": "Steel Body",
    "EXPLOSIONSPIRITS": "Explosion Spirits",
    "CHASEWALK": "Chase Walk",
    "ENERGYCOAT": "Energy Coat",
    "MAXIMIZEPOWER": "Maximize Power",
    "ADRENALINE": "Adrenaline Rush",
    "ADRENALINE2": "Adrenaline Rush (Party)",
    "TRUESIGHT": "True Sight",
    "WINDWALK": "Wind Walk",
    "MELTDOWN": "Melting Down",
    "CARTBOOST": "Cart Boost",
    "ASSUMPTIO": "Assumptio",
}


def humanise(constant: str) -> str:
    """`SI_INC_AGI` -> "Inc Agi". Best-effort; see OVERRIDES."""
    stem = constant.removeprefix("SI_")
    if stem in OVERRIDES:
        return OVERRIDES[stem]
    return " ".join(part.capitalize() for part in stem.split("_") if part)


def parse_constants(path: Path) -> dict[int, str]:
    if not path.is_file():
        sys.exit(f"error: constants file not found: {path}")

    pattern = re.compile(r"^\s*(SI_[A-Z0-9_]+):\s*(-?\d+)\s*,?\s*$")
    by_index: dict[int, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = pattern.match(line)
        if not match:
            continue
        constant, raw_index = match.group(1), int(match.group(2))
        # Negative/sentinel entries aren't real icons.
        if raw_index < 0:
            continue
        # First definition wins; later aliases would otherwise clobber the
        # canonical name for a shared index.
        by_index.setdefault(raw_index, humanise(constant))
    return by_index


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed JSON is stale")
    args = parser.parse_args()

    by_index = parse_constants(CONSTANTS)
    if not by_index:
        sys.exit("error: parsed zero SI_ constants — has constants.conf changed format?")

    payload = json.dumps({str(k): by_index[k] for k in sorted(by_index)}, indent=2, ensure_ascii=False) + "\n"

    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.is_file() else ""
        if current != payload:
            print(f"stale: {OUTPUT} — re-run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"up to date: {len(by_index)} status names")
        return 0

    OUTPUT.write_text(payload, encoding="utf-8")
    print(f"wrote {OUTPUT} ({len(by_index)} status names, {len(OVERRIDES)} overridden)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
