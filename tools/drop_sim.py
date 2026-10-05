#!/usr/bin/env python3
"""Offline drop simulator for balance work (GDD 17.3, F36).

Scripts cannot read a monster's drop table, so this reads the same database the
server loads and applies the server's own rule from `mob_dropitem` (src/map/mob.c):

    chance = max(1, database_rate * level_modifier / 100)          # per 10,000
    chance = min(chance, max(database_rate, item_drop_bonus_max_threshold))

The per-item-type rates (`item_rate_*`) are applied by the server when it loads
the database, and the level modifier comes from `level_penalty.conf` at the exact
difference between monster level and the top attacker's level.

It models only the stock behaviour of this server. If any modifier it does not
model is configured (type rates, luck bonuses, size influence, PK mode,
logarithmic drops, zero-rate drops) it REFUSES and names the setting, rather
than print odds that are quietly wrong.

    python3 tools/drop_sim.py PORING --player-level 5 --kills 100 --seed 1
    python3 tools/drop_sim.py 1002 --json

MVP inventory rewards (`MvpDrops`) are listed but not simulated: the server
awards at most one per kill and the chance depends on the other entries.
"""

from __future__ import annotations

import argparse
import json
import random
import sys
from pathlib import Path
from typing import Any

import export_server_rules as rules
from export_skill_info import parse_skill_db

HERCULES = rules.HERCULES
MOB_SOURCES = [HERCULES / "db/re/mob_db.conf", HERCULES / "db/mob_db2.conf"]
LEVEL_PENALTY = HERCULES / "db/re/level_penalty.conf"

# Settings whose non-stock values this tool does not model, with the stock value.
STOCK = {
    "item_rate_common": 100, "item_rate_common_boss": 100,
    "item_rate_heal": 100, "item_rate_heal_boss": 100,
    "item_rate_use": 100, "item_rate_use_boss": 100,
    "item_rate_equip": 100, "item_rate_equip_boss": 100,
    "item_rate_card": 100, "item_rate_card_boss": 100,
    "item_logarithmic_drops": False,
    "drop_rate0item": False,
    "drops_by_luk": 0, "drops_by_luk2": 0,
    "mob_size_influence": False,
    "pk_mode": 0,
}


class SimError(ValueError):
    pass


def unmodelled_settings(settings: dict[str, dict[str, Any]]) -> list[str]:
    """Settings that differ from the stock value this tool assumes."""
    problems = []
    for key, stock in STOCK.items():
        if key not in settings:
            raise SimError(f"setting {key} was not found in the battle configuration; review drop_sim.py")
        value = settings[key]["value"]
        if value != stock:
            problems.append(f"{key} is {value!r}, this tool models {stock!r}")
    return problems


def level_modifier(table: dict[str, dict[str, dict[int, int]]], boss: bool, monster_level: int, player_level: int) -> int:
    """Item drop modifier in percent: the row at the exact level difference, else 100.

    `pc_level_penalty_mod` (src/map/pc.c) walks the races in order and takes
    the first row set at that difference. A boss starts at `RC_Boss`, and when
    that row is empty it carries on to `RC_NonBoss`; a non-boss never reads
    the boss row. This server's boss table lists only difference 0, so bosses
    take the non-boss rows everywhere else (as the Guide's level-difference
    rule says). The parser admits no other races.
    """
    rows = table["ITEM_DROP_PENALTY_RATE"]
    diff = monster_level - player_level
    if boss and diff in rows.get("RC_Boss", {}):
        return rows["RC_Boss"][diff]
    return rows.get("RC_NonBoss", {}).get(diff, 100)


def effective_rate(base: int, modifier: int, threshold: int) -> int:
    """The server's per-10,000 chance for a stock player, mirroring mob_dropitem."""
    rate = base
    if modifier != 100:
        rate = rate * modifier // 100
        if rate < 1:
            rate = 1
    rate = max(rate, 1)
    return min(rate, max(base, threshold))


def chance_at_least_one(rate: int, kills: int) -> float:
    return 1.0 - (1.0 - rate / 10000.0) ** kills


def kills_until_first(rate: int, rng: random.Random) -> int:
    kills = 1
    while rng.randrange(10000) >= rate:
        kills += 1
    return kills


def percentile(sorted_values: list[int], fraction: float) -> int:
    return sorted_values[min(len(sorted_values) - 1, int(fraction * len(sorted_values)))]


def load_monsters() -> list[dict[str, Any]]:
    monsters: list[dict[str, Any]] = []
    for source in MOB_SOURCES:
        if source.is_file():
            monsters.extend(parse_skill_db(source.read_text(encoding="utf-8", errors="replace")))
    if not monsters:
        raise SimError("no monster database found")
    return monsters


def find_monster(monsters: list[dict[str, Any]], query: str) -> dict[str, Any]:
    for monster in monsters:
        if str(monster.get("Id")) == query or str(monster.get("SpriteName", "")).lower() == query.lower() or str(monster.get("Name", "")).lower() == query.lower():
            return monster
    raise SimError(f"no monster matches {query!r} (use its id or sprite name)")


def drop_entries(block: Any) -> list[tuple[str, int]]:
    """`Name: rate` or `Name: (rate, "group")` entries, in database order."""
    entries = []
    for name, value in (block or {}).items():
        rate = value[0] if isinstance(value, (list, tuple)) else value
        entries.append((name, int(rate)))
    return entries


def simulate(monster: dict[str, Any], player_level: int | None, kills: int, trials: int, seed: int) -> dict[str, Any]:
    settings = rules.effective_settings(HERCULES / "conf/map/battle.conf")
    problems = unmodelled_settings(settings)
    if problems:
        raise SimError("this server has settings the simulator does not model, so its odds would be wrong:\n  " + "\n  ".join(problems))
    threshold = int(settings["item_drop_bonus_max_threshold"]["value"])
    boss = bool(monster.get("Mode", {}).get("Boss", False))
    level = int(monster.get("Lv", 1))
    table = rules.parse_level_penalty(LEVEL_PENALTY)
    modifier = 100 if player_level is None else level_modifier(table, boss, level, player_level)
    rng = random.Random(seed)

    drops = []
    for name, base in drop_entries(monster.get("Drops")):
        rate = effective_rate(base, modifier, threshold)
        samples = sorted(kills_until_first(rate, rng) for _ in range(trials))
        drops.append({
            "item": name,
            "database_rate": base,
            "effective_rate": rate,
            "effective_percent": rate / 100.0,
            "expected_kills_for_one": 10000.0 / rate,
            "chance_at_least_one": chance_at_least_one(rate, kills),
            "kills_for_one_p10_p50_p90": [percentile(samples, 0.10), percentile(samples, 0.50), percentile(samples, 0.90)],
        })
    return {
        "monster": {"id": monster.get("Id"), "sprite": monster.get("SpriteName"), "level": level, "boss": boss},
        "player_level": player_level,
        "level_modifier_percent": modifier,
        "kills": kills,
        "trials": trials,
        "seed": seed,
        "drops": drops,
        "mvp_rewards_not_simulated": [{"item": name, "database_rate": rate} for name, rate in drop_entries(monster.get("MvpDrops"))],
    }


def render(result: dict[str, Any]) -> str:
    monster = result["monster"]
    lines = [
        f"{monster['sprite']} (id {monster['id']}, level {monster['level']}{', boss' if monster['boss'] else ''})",
    ]
    if result["player_level"] is None:
        lines.append("Player level not given: no level modifier applied (100%).")
    else:
        lines.append(f"Top attacker level {result['player_level']}: item drop modifier {result['level_modifier_percent']}%.")
    at_least = "P(>=1 in %d)" % result["kills"]
    lines.append(
        f"{'item':<24}{'db':>7}{'now':>7}{'%':>8}{'1 in':>9}{at_least:>15}   kills for one (p10/p50/p90, {result['trials']} trials)"
    )
    for drop in result["drops"]:
        p10, p50, p90 = drop["kills_for_one_p10_p50_p90"]
        lines.append(
            f"{drop['item']:<24}{drop['database_rate']:>7}{drop['effective_rate']:>7}{drop['effective_percent']:>7.2f}%{drop['expected_kills_for_one']:>9.1f}"
            f"{drop['chance_at_least_one'] * 100:>14.1f}%   {p10}/{p50}/{p90}"
        )
    if result["mvp_rewards_not_simulated"]:
        lines.append("MVP rewards (not simulated): " + ", ".join(f"{m['item']} {m['database_rate']}" for m in result["mvp_rewards_not_simulated"]))
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("monster", help="monster id or sprite name")
    parser.add_argument("--player-level", type=int, help="the top attacker's base level (applies the level modifier)")
    parser.add_argument("--kills", type=int, default=100, help="kills for the P(at least one) column")
    parser.add_argument("--trials", type=int, default=2000, help="simulated runs per drop")
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    try:
        result = simulate(find_monster(load_monsters(), args.monster), args.player_level, args.kills, args.trials, args.seed)
    except (SimError, rules.ExportError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, indent=1) if args.json else render(result))
    return 0


if __name__ == "__main__":
    sys.exit(main())
