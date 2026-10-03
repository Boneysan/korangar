#!/usr/bin/env python3
"""Export selected effective Hercules rates and progression rules for the Guide.

Reads only reviewed numeric/boolean settings from the configuration include
trees. Includes and imports are applied in source order, so later import files
override stock values exactly where Hercules loads them. Script-derived death
recovery values are read from the dedicated Korangar NPC script.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from export_job_skills import HERCULES, source_revision
from export_skill_info import _strip_comments

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "docs/server-rules.v1.json"
INCLUDE = re.compile(r'^\s*@include\s+"([^"]+)"')
IMPORT = re.compile(r'^\s*import\s*:\s*"([^"]+)"')
SETTING = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(true|false|-?\d+)\b")


class ExportError(ValueError):
    pass


def ordered_config_files(root_file: Path) -> list[Path]:
    """Expand libconfig include/import directives in their textual order."""
    ordered: list[Path] = []
    active: set[Path] = set()

    def visit(path: Path) -> None:
        path = path.resolve()
        if path in active:
            raise ExportError(f"configuration include cycle at {path.relative_to(HERCULES)}")
        if not path.is_file():
            raise ExportError(f"configuration source missing: {path}")
        active.add(path)
        ordered.append(path)
        cleaned = _strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        for line in cleaned.splitlines():
            match = INCLUDE.match(line) or IMPORT.match(line)
            if match:
                target = (HERCULES / match.group(1)).resolve()
                visit(target)
        active.remove(path)

    visit(root_file)
    return ordered


def effective_settings(root_file: Path) -> dict[str, dict[str, object]]:
    """Return last-wins scalar settings with their exact source locations."""
    values: dict[str, dict[str, object]] = {}
    for path in ordered_config_files(root_file):
        cleaned = _strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        for line_number, line in enumerate(cleaned.splitlines(), 1):
            match = SETTING.match(line)
            if not match:
                continue
            key, raw = match.groups()
            value: object = raw == "true" if raw in ("true", "false") else int(raw)
            values[key] = {
                "value": value,
                "source": {
                    "path": path.relative_to(HERCULES).as_posix(),
                    "record": f"line {line_number}: {key}",
                },
            }
    return values


def required(settings: dict[str, dict[str, object]], key: str) -> dict[str, object]:
    try:
        return settings[key]
    except KeyError as error:
        raise ExportError(f"required setting {key!r} is absent from the effective include tree") from error


def value(settings: dict[str, dict[str, object]], key: str) -> int:
    result = required(settings, key)["value"]
    if not isinstance(result, int) or isinstance(result, bool):
        raise ExportError(f"setting {key!r} is not an integer")
    return result


def sources(*settings: dict[str, object]) -> list[dict[str, str]]:
    return [setting["source"] for setting in settings]  # type: ignore[list-item]


def parse_element_adjustments(path: Path) -> dict[str, dict[int, dict[str, int]]]:
    """Parse defender/level/attacker rates from Hercules attr_fix.conf."""
    text = _strip_comments(path.read_text(encoding="utf-8", errors="replace"))
    result: dict[str, dict[int, dict[str, int]]] = {}
    position = 0
    while match := re.search(r"(?m)^(Ele_[A-Za-z0-9_]+)\s*:\s*\{", text[position:]):
        defender = match.group(1)
        opening = position + match.end() - 1
        depth = 1
        cursor = opening + 1
        while cursor < len(text) and depth:
            if text[cursor] == "{":
                depth += 1
            elif text[cursor] == "}":
                depth -= 1
            cursor += 1
        if depth:
            raise ExportError(f"unclosed elemental adjustment block for {defender}")
        body = text[opening + 1:cursor - 1]
        levels: dict[int, dict[str, int]] = {}
        for level_match in re.finditer(r"Lv(\d+)\s*:\s*\{([^{}]*)\}", body, re.S):
            level = int(level_match.group(1))
            attackers = {name: int(rate) for name, rate in re.findall(r"(Ele_[A-Za-z0-9_]+)\s*:\s*(-?\d+)", level_match.group(2))}
            if level in levels or not attackers:
                raise ExportError(f"duplicate or empty elemental adjustment level {defender} Lv{level}")
            levels[level] = attackers
        if defender in result or set(levels) != {1, 2, 3, 4}:
            raise ExportError(f"duplicate defender or incomplete levels in attr_fix.conf: {defender}")
        result[defender] = levels
        position = cursor
    elements = set(result)
    if len(elements) != 10 or any(set(rows) != elements for levels in result.values() for rows in levels.values()):
        raise ExportError("attr_fix.conf did not resolve a complete 10-element matrix")
    return result


def parse_weapon_size_adjustments(path: Path) -> dict[str, dict[str, int]]:
    """Parse the active weapon-type by target-size table and its documented columns."""
    text = path.read_text(encoding="utf-8", errors="replace")
    columns = re.search(r"(?m)^//\s*Unarmed,\s*(.+)$", text)
    if not columns:
        raise ExportError("size_fix.txt weapon column documentation is missing")
    weapons = ["Unarmed", *[column.strip() for column in columns.group(1).split(",")]]
    sizes = ("Small", "Medium", "Large")
    rows = []
    for line in text.splitlines():
        match = re.match(r"^\s*([\d,\s]+)\s*//\s*Size:\s*(Small|Medium|Large)\s*$", line)
        if match:
            rows.append((match.group(2), [int(value) for value in match.group(1).split(",")]))
    table = {size: values for size, values in rows}
    if len(rows) != 3 or set(table) != set(sizes) or any(len(values) != len(weapons) for values in table.values()):
        raise ExportError(f"size_fix.txt must define three rows with {len(weapons)} documented weapon columns")
    return {size: dict(zip(weapons, table[size], strict=True)) for size in sizes}


def parse_level_penalty(path: Path) -> dict[str, dict[str, dict[int, int]]]:
    """Parse type/race/diff/rate rows from Hercules level_penalty.conf."""
    text = re.sub(r"/\*.*?\*/", "", path.read_text(encoding="utf-8", errors="replace"), flags=re.S)
    text = re.sub(r"//.*", "", text)
    rows = re.findall(
        r'\{\s*type:\s*"(\w+)"\s*,\s*race:\s*"(\w+)"\s*,\s*diff:\s*(-?\d+)\s*,\s*rate:\s*(\d+)\s*\}', text
    )
    result: dict[str, dict[str, dict[int, int]]] = {}
    for kind, race, diff, rate in rows:
        if kind not in {"EXP_PENALTY_RATE", "ITEM_DROP_PENALTY_RATE"} or race not in {"RC_NonBoss", "RC_Boss"}:
            raise ExportError(f"unexpected level_penalty row: {kind} {race}")
        table = result.setdefault(kind, {}).setdefault(race, {})
        if int(diff) in table:
            raise ExportError(f"duplicate level_penalty row: {kind} {race} {diff}")
        table[int(diff)] = int(rate)
    if text.count("type:") != len(rows) or set(result) != {"EXP_PENALTY_RATE", "ITEM_DROP_PENALTY_RATE"}:
        raise ExportError("level_penalty.conf has rows this parser does not understand")
    return result


def derived_stat_formula_rule(battle: dict[str, dict[str, object]]) -> dict[str, object]:
    """Authored from a source review of status.c / skill.c (Renewal build).

    The arithmetic below is restated, not parsed: it was read from the C
    expressions named in `sources`, and the client's `stat_formulas` module
    mirrors the same expressions and is unit-tested against hand-computed
    values. Only the cast scale is read from configuration.
    """
    scale = required(battle, "vcast_stat_scale")
    scale_value = value(battle, "vcast_stat_scale")
    return {
        "id": "derived-stat-formulas",
        "category": "Mechanics",
        "title": "Derived stat formulas (players)",
        "summary": "How the server turns base stats into HIT, FLEE, DEF, MDEF, CRIT, ATK, MATK and cast time on this Renewal build. Equipment, cards, skills and status effects add to or scale these afterwards.",
        "details": [
            "HIT = Base Level + DEX + floor(LUK / 3) + 175.",
            "FLEE = Base Level + AGI + floor(LUK / 5) + 100.",
            "Soft DEF = (Base Level + VIT) / 2 + AGI / 5, with the fractions added first and the total rounded down once.",
            "Soft MDEF = INT + Base Level / 4 + (DEX + VIT) / 5, with the fractions added first and the total rounded down once.",
            "Critical = (10 + floor(LUK x 10 / 3)) tenths of a percent, about 1% plus a third of a percent per LUK.",
            "Perfect dodge = (LUK + 10) tenths of a percent.",
            "Status ATK = STR + DEX / 5 + LUK / 3 + Base Level / 4, rounded down once. Bows, instruments, whips and guns swap STR and DEX.",
            "Status MATK = INT + floor(INT / 2) + floor(DEX / 5) + floor(LUK / 3) + floor(Base Level / 4).",
            f"Variable cast time: DEX and INT remove sqrt((DEX x 2 + INT) / {scale_value}) of the variable part of a cast (a square root, not a straight line, so the first points matter most). At DEX x 2 + INT of {scale_value} or more, no variable cast remains.",
            "A skill with no configured fixed cast time is split 20% fixed and 80% variable; a configured fixed time is kept as the fixed part, and a negative one means no fixed part. Stats do not reduce the fixed part; equipment and status bonuses can.",
            "Not covered here: Max HP/SP, ASPD, the equipment DEF/MDEF bonuses, and how items and statuses modify these values.",
            "These were confirmed from the source and are unit-tested in the client, not observed on a live server.",
        ],
        "sources": [
            {"path": "src/map/status.c", "record": "status_calc_misc (HIT, FLEE, DEF2, MDEF2, CRI, FLEE2)"},
            {"path": "src/map/status.c", "record": "status_base_atk and status_base_matk (Renewal)"},
            {"path": "src/map/skill.c", "record": "skill_vfcastfix (variable and fixed cast)"},
            scale["source"],  # type: ignore[list-item]
        ],
    }


def stat_point_rule() -> dict[str, object]:
    """Built from export_stat_rules (imported lazily: that module imports this one)."""
    from export_stat_rules import build as build_stat_rules

    rules = build_stat_rules()
    table: list[int] = rules["points_at_level"]  # type: ignore[assignment]
    jobs: list[dict[str, object]] = rules["jobs"]  # type: ignore[assignment]
    extra = rules["upper_class_extra_points"]

    def cost(value: int) -> int:
        return 2 + (value - 1) // 10 if value < 100 else 16 + 4 * ((value - 100) // 5)

    milestones = [level for level in (1, 10, 25, 50, 75, 99, 130, 150, 175) if level <= len(table)]
    totals = "; ".join(f"level {level}: {table[level - 1]:,}" for level in milestones)
    gains = "; ".join(f"level {level} to {level + 1}: +{table[level] - table[level - 1]}" for level in (1, 50, 98, 150) if level < len(table))
    by_cap: dict[int, list[str]] = {}
    for job in jobs:
        by_cap.setdefault(job["max_stats"], []).append(job["parameters_group"])  # type: ignore[arg-type]
    caps = "; ".join(
        f"{cap}: {', '.join(sorted(set(groups)))}" for cap, groups in sorted(by_cap.items())
    )
    upper_names = ", ".join(str(job["name"]).replace("_", " ") for job in jobs if job["upper"])
    return {
        "id": "stat-points",
        "category": "Mechanics",
        "title": "Stat points: totals, costs and caps",
        "summary": "How many stat points a character has at each level, what each point costs, the extra points upper classes get, and each class's stat cap.",
        "details": [
            f"Total stat points at a level (cumulative, before any are spent): {totals}.",
            f"A level-up grants the difference between two levels' totals, for example {gains}.",
            f"Raising a stat from v to v + 1 costs 2 + (v - 1) / 10 points below 100 and 16 + 4 x ((v - 100) / 5) from 100 (integer division): 9 to 10 costs {cost(9)}, 98 to 99 costs {cost(98)}, 99 to 100 costs {cost(99)}, 100 to 101 costs {cost(100)}. Taking one stat from 1 to 99 costs {sum(cost(v) for v in range(1, 99)):,} points in all.",
            f"Resetting stats gives back the level's total, plus {extra} extra points for upper classes: {upper_names}.",
            f"Stat cap by parameter group (from unit_parameters_db.conf, each job's ParametersGroup in job_db.conf): {caps}. The battle.conf setting max_parameter is not the player cap.",
            "A job with no job_db.conf block (mounted and cosmetic forms) has no entry here.",
            "These were confirmed from the source and are unit-tested in the client, not observed on a live server.",
        ],
        "sources": [
            {"path": "db/re/statpoint.txt", "record": "cumulative points by level"},
            {"path": "db/re/unit_parameters_db.conf", "record": "MaxStats per parameter group"},
            {"path": "db/re/job_db.conf", "record": "ParametersGroup per job"},
            {"path": "src/map/pc.c", "record": "pc_readdb, pc_gets_status_point, pc_resetstate, pc_need_status_point, pc_jobid2mapid"},
            {"path": "conf/map/battle/exp.conf", "record": "use_statpoint_table: true"},
        ],
    }


def max_hp_sp_rule() -> dict[str, object]:
    """Built from export_job_tables (imported lazily: it imports this package's helpers)."""
    from export_job_tables import build as build_job_tables

    tables = build_job_tables()
    jobs = {job["job_id"]: job for job in tables["jobs"]}  # type: ignore[index]

    def hp(job_id: int, level: int, vit: int) -> int:
        job = jobs[job_id]
        value = tables["hp_tables"][job["hp_table"]][level - 1]  # type: ignore[index]
        if job["super_novice"] and level >= 99:
            value += 2000
        if job["expanded_super_novice"] and level >= 150:
            value += 2000
        if job["upper"]:
            value += value * 25 // 100
        elif job["baby"]:
            value = value * 70 // 100
        return value + value * vit // 100

    def sp(job_id: int, level: int, int_stat: int) -> int:
        job = jobs[job_id]
        value = tables["sp_tables"][job["sp_table"]][level - 1]  # type: ignore[index]
        if job["upper"]:
            value += value * 25 // 100
        elif job["baby"]:
            value = value * 70 // 100
        return value + value * int_stat // 100

    return {
        "id": "max-hp-sp",
        "category": "Mechanics",
        "title": "Max HP and SP: class tables",
        "summary": "A class's base max HP and SP come from per-class tables by level, adjusted for upper and baby classes, then scaled by VIT and INT. Equipment and status effects add to the result.",
        "details": [
            "Base max HP = the class table value at your Base Level; plus 2000 for a Super Novice at level 99 or higher (Expanded Super Novice: another 2000 at level 150); then plus 25% for upper classes or times 70% for baby classes; then plus 1% per VIT. Each step uses whole numbers.",
            "Base max SP = the class table value at your Base Level; plus 25% for upper classes or times 70% for baby classes; then plus 1% per INT. Super Novices get no flat SP bonus.",
            f"Example, level 99 with VIT 99: Knight {hp(7, 99, 99):,} base HP; Lord Knight (upper) {hp(4008, 99, 99):,}; Baby Swordsman (baby) {hp(4024, 99, 99):,}. Level 99 Wizard with INT 99: {sp(9, 99, 99):,} base SP.",
            "The tables are the ones in job_db.conf, read through the server's own loader: a class with no table of its own inherits from another, and levels a table does not list are filled in from an average increment. Open a job's page for its HP and SP at several levels.",
            "Equipment bonuses, skills and status effects (for example Increase HP, Berserk) change the final value afterwards and are not included here. A top-ranked Taekwon over level 90 gets triple HP and SP from a live ranking this client cannot see.",
            "Confirmed from the source and compared with what this server computed for 26 characters: 18 across first, second, upper and third classes at level 99 or below, and 8 made for the purpose (baby, Super Novice, Super Baby, Expanded Super Novice at level 150, two third classes at level 175, and baby jobs at levels 151 and 161). Every value matched. Not compared: gear and status modifiers, and the ranked-Taekwon bonus.",
            "Known data defect on this server: the Baby Kagerou and Baby Oboro tables fall to 1 from level 161 to 175, so a level-161 Baby Kagerou has 1 max HP and 1 max SP (observed).",
        ],
        "sources": [
            {"path": "db/re/job_db.conf", "record": "HPTable, SPTable, Inherit, InheritHP, InheritSP per job"},
            {"path": "src/map/status.c", "record": "status_read_job_db_sub (table loader), status_get_base_maxhp, status_get_base_maxsp"},
            {"path": "db/re/unit_parameters_db.conf", "record": "MaxHP caps applied to table values"},
        ],
    }


def aspd_rule() -> dict[str, object]:
    from export_job_tables import build as build_job_tables
    from export_stat_rules import build as build_stat_rules

    tables = build_job_tables()
    stat_jobs = {job["job_id"]: job for job in build_stat_rules()["jobs"]}  # type: ignore[index]
    by_cap: dict[int, set[str]] = {}
    for job in tables["jobs"]:  # type: ignore[index]
        by_cap.setdefault(job["max_aspd"], set()).add(stat_jobs[job["job_id"]]["parameters_group"])
    caps = "; ".join(f"{cap}: {', '.join(sorted(groups))}" for cap, groups in sorted(by_cap.items()))
    return {
        "id": "aspd",
        "category": "Mechanics",
        "title": "Attack speed (ASPD)",
        "summary": "Attack speed starts from AGI, DEX and the class's value for the weapon, is then changed by skills, equipment, statuses and mounts, and cannot pass the class's cap.",
        "details": [
            "1. Base ASPD = floor(196 + sqrt(DEX x DEX / 5 + AGI x AGI / 2) / 4 + (P + S) x AGI / 200) - min(B, 200). For bows, instruments, whips and guns, DEX counts as DEX x DEX / 7 instead of DEX x DEX / 5.",
            "B is the class's value for the weapon type, plus its Shield value when a shield is worn (a shield slows you down), plus a quarter of the second weapon's value when dual wielding. Each job's page lists its values.",
            "P is the passive skill bonus: Advanced Book with a book gives (level - 1) / 2 + 1; Single Action gives (level + 1) / 2; Plagiarism gives its level; Musical Lesson with an instrument gives its level. They need their weapon: Musical Lesson does nothing bare-handed.",
            "S is the status bonus: Center, Awakening and Berserk Potions give +4, +6 and +9, and only the strongest potion counts, they do not add up. Two-Hand Quicken, One-Hand Quicken, Spear Quicken and Adrenaline Rush give +7 (the largest of that family, not a sum), Berserk +15, Madness Cancel +20. S adds to P.",
            "2. Rate: a mounted Knight's ASPD is multiplied by (500 + 100 x Cavalier Mastery level) / 1000, so riding without Cavalier Mastery halves it and level 5 removes the penalty. A Rune Knight on a Dragon uses (750 + 50 x Dragon Training level) / 1000.",
            "3. Percentages close that share of the gap between your ASPD and 195: equipment 'ASPD +x%' (Windhawk +5, Katar of Speed +3, Muramasa +8, Doom Slayer -40), and statuses such as Two-Hand Quicken, Adrenaline Rush and Spear Quicken (+10 each), Increase AGI (+its level), or the slowing ones (Don't Forget Me, Steel Body). Whole numbers, rounded toward zero. Quagmire switches off the Quicken-family bonus in S.",
            "4. Attack motion in milliseconds = 10 x (200 - ASPD), then flat equipment 'ASPD +n' bonuses take 10 ms off per point (Cursed Mad Bunny +3, Masamune +2, Spoon +10), and a few statuses take a fixed amount off.",
            "5. The result cannot be faster than the class's MaxASPD: 100 ms at 190, 70 ms at 193. By parameter group: " + caps + ".",
            "Equipment bonuses that are a chance on hit (Jewel Ring) or need a refine level (the Fashion Shoes and Berry Hat's +2 needs +12) add nothing otherwise.",
            "Confirmed from the source and compared with what this server reported in 115 measurements: bare-handed and every weapon type, shield, dual wield, three stat levels, baby, upper and third classes, the four passive skills, equipment percentages and flat bonuses, the three potions, Two-Hand Quicken, Adrenaline Rush, Increase AGI, a mounted Knight at every Cavalier Mastery level, both class caps (190 and 193) and a Dancer's whip. Every value matched. Read from the source but not compared: the Dragon mount, Spear and One-Hand Quicken, the Berserk and Madness Cancel statuses, Quagmire, the slowing statuses, and the statuses that take a fixed amount off.",
        ],
        "sources": [
            {"path": "src/map/status.c", "record": "status_base_amotion_pc (Renewal ASPD) and the amotion assembly in status_calc_bl_main"},
            {"path": "db/re/job_db.conf", "record": "BaseASPD per weapon per job"},
            {"path": "db/re/unit_parameters_db.conf", "record": "MaxASPD per parameter group"},
            {"path": "src/config/renewal.h", "record": "RENEWAL_ASPD"},
        ],
    }


def level_penalty_rule(path: Path) -> dict[str, object]:
    table = parse_level_penalty(path)

    def listed(kind: str) -> str:
        rows = sorted(table[kind]["RC_NonBoss"].items(), key=lambda row: row[0])
        return "; ".join(f"{diff:+d}: {rate}%" for diff, rate in rows)

    return {
        "id": "level-difference-modifiers",
        "category": "Mechanics",
        "title": "Level difference EXP and drop modifiers",
        "summary": "Monster level minus player Base Level can scale EXP and item drop rates, but this server applies a modifier only at the exact differences listed here.",
        "details": [
            "Difference = monster level minus the player's Base Level. EXP uses the player receiving the EXP; drop rates use the monster's top-credited attacker (MVP, else second, else third).",
            f"EXP modifier (applies to base and job EXP) at these exact differences: {listed('EXP_PENALTY_RATE')}.",
            f"Item drop-rate modifier at these exact differences: {listed('ITEM_DROP_PENALTY_RATE')}.",
            "Any other difference takes 100%: the server reads each row at its exact difference and does not carry a value across the gaps or beyond the last row. A monster 17 or more levels above, or any gap such as -5, is therefore unmodified.",
            "Bosses have no separate penalty rows (the boss row lists only difference 0 at 100%); the same non-boss rows apply to them.",
            "This describes how the server loads and looks up the table. It was confirmed from the source, not observed in play; an @mobinfo check against a monster at a listed and an unlisted level difference in a live session would confirm it.",
        ],
        "sources": [
            {"path": path.relative_to(HERCULES).as_posix(), "record": "level_penalty_db rows"},
            {"path": "src/map/pc.c", "record": "pc_level_penalty_mod exact-difference lookup; pc_read_level_penalty_db_sub loader"},
            {"path": "src/map/mob.c", "record": "drop modifier from the top-credited attacker's level"},
            {"path": "src/config/renewal.h", "record": "RENEWAL_EXP and RENEWAL_DROP"},
        ],
    }


def dm_campaign_rule(battle: dict[str, dict[str, object]]) -> dict[str, object]:
    timeout_ms = value(battle, "campaign_combat_timeout_ms")
    sit_interval_ms = value(battle, "campaign_sit_recovery_interval_ms")
    sit_percent = value(battle, "campaign_sit_recovery_percent")
    respawn_percent = value(battle, "campaign_respawn_percent")
    fill_ms = value(battle, "campaign_respawn_fill_ms")
    weight_mult = value(battle, "campaign_max_weight_multiplier")
    return {
        "id": "dm-campaign-rules",
        "category": "Progression",
        "title": "DM Campaign progression and recovery",
        "summary": "Settings for combat recovery, sitting regeneration, respawn, weight capacity, and party story synchronization active during DM Session campaign play.",
        "details": [
            f"Campaign combat timeout: {timeout_ms} ms ({timeout_ms // 1000} seconds) without taking or dealing damage before leaving combat state.",
            f"Rest and sit recovery: sitting out of combat recovers {sit_percent}% of max HP and max SP every {sit_interval_ms} ms ({sit_interval_ms // 1000} seconds).",
            f"Campaign respawn: defeated characters respawn with {respawn_percent}% HP and SP, filling to 100% linearly over {fill_ms} ms ({fill_ms // 1000} seconds).",
            f"Exploration weight capacity: maximum weight capacity is multiplied by {weight_mult}x during campaign sessions.",
            "Party story synchronization: characters in an active campaign party (dm_campaign_checkpoint_member) catch up to the party's journaled story quests and beat flags (dm_campaign_party_event) upon joining or map change. Personal choices and past rewards are never overwritten.",
        ],
        "sources": sources(
            required(battle, "campaign_combat_timeout_ms"),
            required(battle, "campaign_sit_recovery_interval_ms"),
            required(battle, "campaign_sit_recovery_percent"),
            required(battle, "campaign_respawn_percent"),
            required(battle, "campaign_respawn_fill_ms"),
            required(battle, "campaign_max_weight_multiplier"),
        ) + [
            {"path": "src/map/combat_state.c", "record": "campaign combat timeout, sit recovery, and respawn fill"},
            {"path": "src/map/party.c", "record": "party_campaign_catchup_others and party_campaign_push_others"},
        ],
    }


def view_distance_rule(battle: dict[str, dict[str, object]]) -> dict[str, object]:
    area = value(battle, "area_size")
    dead_area = value(battle, "dead_area_size")
    return {
        "id": "view-distance-and-aoe",
        "category": "Mechanics",
        "title": "View distance and area radius",
        "summary": "Entity view radius and despawn margin configured for modern high-resolution client display.",
        "details": [
            f"Active entity view radius (area_size): {area} cells ({area * 2}-cell diameter), configured in conf/import/battle.conf to prevent pop-in on modern high-resolution displays (overriding the stock 14-cell client.conf default).",
            f"Despawn margin (dead_area_size): {dead_area} cells, keeping snap-dodge margin and entity tracking consistent with the expanded view radius.",
            f"Shared party quest kill credit uses the same {area}-cell area radius around the defeated monster.",
        ],
        "sources": sources(
            required(battle, "area_size"),
            required(battle, "dead_area_size"),
        ) + [
            {"path": "conf/map/battle/client.conf", "record": "stock area_size: 14 default"},
            {"path": "src/map/mob.c", "record": "party quest objective update within AREA_SIZE"},
        ],
    }


def natural_recovery_rule(battle: dict[str, dict[str, object]]) -> dict[str, object]:
    hp_ms = value(battle, "natural_healhp_interval")
    sp_ms = value(battle, "natural_healsp_interval")
    return {
        "id": "natural-recovery-and-weight",
        "category": "Mechanics",
        "title": "Natural recovery and weight thresholds",
        "summary": "Standing and sitting HP/SP regeneration intervals and the 50%/90% overweight capacity thresholds.",
        "details": [
            f"Natural HP recovery interval: {hp_ms} ms ({hp_ms // 1000} seconds) while standing.",
            f"Natural SP recovery interval: {sp_ms} ms ({sp_ms // 1000} seconds) while standing.",
            f"Sitting doubles natural recovery frequency (halving the tick interval to {hp_ms // 2000} seconds for HP and {sp_ms // 2000} seconds for SP). Moving or attacking resets the interval.",
            "Overweight 50% limit: when inventory weight reaches or exceeds 50% of maximum capacity, natural HP and SP regeneration stops completely.",
            "Overweight 90% limit: when inventory weight reaches or exceeds 90% of maximum capacity, normal attacks and skill casting are disabled.",
        ],
        "sources": sources(
            required(battle, "natural_healhp_interval"),
            required(battle, "natural_healsp_interval"),
        ) + [
            {"path": "src/map/status.c", "record": "natural heal calculation and sitting double rate"},
            {"path": "src/map/pc.h", "record": "pc_isoverhealweight (50%) and pc_is90overweight (90%) macros"},
        ],
    }


def progression_limits_rule(battle: dict[str, dict[str, object]]) -> dict[str, object]:
    max_param = value(battle, "max_parameter")
    multi_up = bool(required(battle, "multi_level_up")["value"])
    return {
        "id": "progression-limits",
        "category": "Progression",
        "title": "Multi-level-up and progression limits",
        "summary": "Level advancement gating, baseline parameter limits, and server combat mode.",
        "details": [
            f"Multi-level-up: {'enabled' if multi_up else 'disabled (multi_level_up: false)'}. A single EXP gain from a monster or quest cannot advance a character by more than one level; excess EXP is retained toward the next level up to 99%.",
            f"Base parameter limit: {max_param}. Player classes override this with per-job MaxStats from unit_parameters_db.conf (e.g. 130 for Third Classes, 125 for Super Novice and Extended Classes, 80 for Baby Classes).",
            "PvP and War of Emperium: WoE guild castle battle modules are excluded from the server's battle include tree (this is a PvE and DM campaign server).",
        ],
        "sources": sources(
            required(battle, "multi_level_up"),
            required(battle, "max_parameter"),
        ) + [
            {"path": "conf/map/battle.conf", "record": "WoE guild battle includes commented out"},
            {"path": "db/re/unit_parameters_db.conf", "record": "per-job MaxStats parameter caps"},
        ],
    }


def equipment_refinement_rule() -> dict[str, object]:
    return {
        "id": "equipment-refinement",
        "category": "Mechanics",
        "title": "Equipment refinement odds and mechanics",
        "summary": "Safe limits, materials, Zeny costs, failure consequences, and stat bonuses for Armor and Weapons.",
        "details": [
            "Safe refine limits (100% success): Armor +4; Weapon Level 1 +7; Weapon Level 2 +6; Weapon Level 3 +5; Weapon Level 4 +4.",
            "Failure consequence: refining past the safe limit carries a risk of permanent destruction — on failure, the item is destroyed outright and lost forever.",
            "NPC Blacksmith costs: Armor uses 1 Elunium (2,000 Zeny); Weapon Lv 1 uses 1 Phracon (50 Zeny); Weapon Lv 2 uses 1 Emveretarcon (200 Zeny); Weapon Lv 3 uses 1 Oridecon (5,000 Zeny); Weapon Lv 4 uses 1 Oridecon (20,000 Zeny).",
            "Whitesmith self-refine (WS_WEAPONREFINE): consumes 1 ore material with 0 Zeny cost; adds +0.5% success chance per job level above 50 (+10% flat for Mechanic Transcendent), but has no safe refine limit (can fail even at +1).",
            "Stat bonuses: Armor gains +1 DEF per level (Lv 1–4), +2 DEF per level (Lv 5–8), +3 DEF per level (Lv 9–10). Weapons gain flat ATK/MATK per level (+2 for Lv1, +3 for Lv2, +5 for Lv3, +7 for Lv4), plus random ATK bonuses on unsafe levels.",
        ],
        "sources": [
            {"path": "db/re/refine_db.conf", "record": "Armors and WeaponLevel1-4 Rates and StatsPerLevel"},
            {"path": "npc/merchants/refine.txt", "record": "refinemain materials, fees, and failure destruction"},
            {"path": "src/map/skill.c", "record": "skill_weaponrefine formula and material list"},
        ],
    }


def require_source(path: str, needle: str) -> None:
    """A rule that quotes a script or source file must be able to point at it."""
    target = HERCULES / path
    if not target.is_file() or needle not in target.read_text(encoding="utf-8", errors="replace"):
        raise ExportError(f"{path} no longer contains {needle!r}; review the rule text that quotes it")


def card_socketing_rule() -> dict[str, object]:
    require_source("src/map/pc.c", "pc_insert_card")
    require_source("src/map/battle.c", "battle_calc_cardfix")
    return {
        "id": "card-socketing-and-binding",
        "category": "Mechanics",
        "title": "Card socketing and slot binding mechanics",
        "summary": "Rules for inserting monster cards into slotted equipment, permanent binding, and bonus stacking.",
        "details": [
            "Slotted equipment contains between 1 and 4 card slots depending on the item definition.",
            "Double-clicking a card opens the socketing interface to insert it into an eligible equipment piece with an open slot.",
            "Permanent slot binding: under standard server rules, socketing a card binds it permanently into the item. Cards cannot be removed or extracted without specialized separation services.",
            "Damage modifier stacking: identical percentage bonuses stack additively with each other (e.g. two 20% cards = +40%), while different modifier categories stack multiplicatively (Size × Element × Race × Boss).",
            "Target validation: weapons, armors, shields, garments, shoes, and accessories only accept cards matching their specific equipment slot type.",
        ],
        "sources": [
            {"path": "src/map/pc.c", "record": "pc_insert_card and pc_can_insert_card_into"},
            {"path": "src/map/battle.c", "record": "battle_calc_cardfix card bonus damage calculations"},
        ],
    }


def first_job_progression_rule() -> dict[str, object]:
    require_source("src/map/pc.c", "pc_jobchange")
    return {
        "id": "first-job-progression",
        "category": "Progression",
        "title": "First Job combat archetypes and stat priorities",
        "summary": "Core combat identities, stat investment guidance, and career paths for the 6 primary First Job classes.",
        "details": [
            "Swordsman (Melee Tank / Striker): Focuses on STR and VIT (or AGI for two-hand speed). Key skills: Bash, Magnum Break, Provoke, Endure, Increase HP Recovery. Advances to Knight or Crusader.",
            "Mage (Elemental Ranged Artillery): Focuses on INT and DEX. Casts Fire, Cold, and Lightning bolts, Fire Wall, and Stone Curse. Relies on variable cast reduction and elemental advantage. Advances to Wizard or Sage.",
            "Archer (Precision Physical Ranged): Focuses on DEX and AGI. Specializes in bows, elemental arrows, Owl's Eye, Vulture's Eye, Double Strafe, and Arrow Shower. Advances to Hunter, Bard, or Dancer.",
            "Thief (Evasion & Rapid Melee): Focuses on AGI and STR with moderate DEX. Utilizes Double Attack, Improve Dodge, Steal, and Hiding. Advances to Assassin or Rogue.",
            "Acolyte (Divine Support & Undead Purging): Focuses on INT, VIT, and DEX. Core foundation of party gameplay with Heal, Blessing, Increase AGI, Angelus, and Holy Light. Advances to Priest or Monk.",
            "Merchant (Economy & Logistics): Focuses on STR, VIT, and DEX (or LUK for forging). Unique capabilities include Pushcart, Discount, Overcharge, Vending, Mammonite, and Item Appraisal. Advances to Blacksmith or Alchemist.",
            "Job Advancement: Novices become eligible for First Job change upon reaching Job Level 10. Advancing to Second Job requires Job Level 40 minimum (Job Level 50 strongly recommended for complete skill points).",
        ],
        "sources": [
            {"path": "src/map/pc.c", "record": "pc_jobchange class transition handling"},
            {"path": "db/re/job_db.conf", "record": "First and Second Job stat allocations and skill trees"},
        ],
    }


def second_job_progression_rule() -> dict[str, object]:
    require_source("src/map/pc.c", "pc_jobchange")
    return {
        "id": "second-job-progression",
        "category": "Progression",
        "title": "Second Job specializations and combat branch paths",
        "summary": "Combat specializations, distinctive build branches, and primary stat and skill synergies for the 12 classic Second Job classes.",
        "details": [
            "Knight & Crusader (Swordsman branches): Knight excels in mobile burst (Bowling Bash, Pierce, Brandish Spear, Two-Hand Quicken) with STR/AGI or STR/VIT. Crusader excels in holy defense and sacrifice (Grand Cross, Shield Charge, Holy Cross, Devotion, Auto Guard) with STR/VIT/INT.",
            "Wizard & Sage (Mage branches): Wizard specializes in devastating area denial (Storm Gust, Meteor Storm, Lord of Vermilion, Quagmire) with high INT/DEX. Sage specializes in counter-magic and combat casting (Free Cast, Cast Cancel, Dispell, Land Protector, Endow elements).",
            "Hunter & Bard/Dancer (Archer branches): Hunter brings long-range sustained DPS and tactical zoning (Blitz Beat, Ankle Snare, Claymore Trap, Arrow Repel) with DEX/AGI/LUK. Bard & Dancer provide party area songs and ensembles (Poem of Bragi, Service for You, Dissonance, Frost Joker) scaling with DEX/INT/VIT.",
            "Assassin & Rogue (Thief branches): Assassin deals lethal dual-wielding and katar critical burst (Sonic Blow, Grimtooth, Cloaking, Katar Mastery) with AGI/STR/LUK. Rogue provides utility, strip combat, and skill copying (Snatch, Strip Weapon/Shield/Armor, Tunnel Drive, Plagiarism) with DEX/AGI/STR.",
            "Priest & Monk (Acolyte branches): Priest acts as the party's essential anchor (Sanctuary, Resurrection, Magnificat, Kyrie Eleison, Gloria, Lex Aeterna, Aspersio) with INT/VIT/DEX. Monk channels divine martial arts into devastating combo strikes and burst (Triple Attack, Combo Finish, Steel Body, Asura Strike / Guillotine Fist) with STR/INT/DEX.",
            "Blacksmith & Alchemist (Merchant branches): Blacksmith masters weapon forging and melee party buffs (Adrenaline Rush, Weapon Perfection, Overthrust, Hammer Fall) with STR/AGI or DEX/LUK. Alchemist commands potion brewing, homunculus creation, and chemical warfare (Demon Demonstration, Acid Terror, Aid Potion, Pharmacy) with INT/DEX/LUK.",
            "Career Progression: Second Job classes unlock at Job Level 40-50, granting 49 additional skill points. They represent the core combat identities for mid-to-endgame party composition and dungeon delves.",
        ],
        "sources": [
            {"path": "src/map/pc.c", "record": "pc_jobchange handles advancement from First Job to Second Job branches"},
            {"path": "db/re/job_db.conf", "record": "Second Job stat modifiers, weapon ASPD tables, and HP/SP pools"},
        ],
    }


def config_precedence_rule() -> dict[str, object]:
    root = HERCULES / "conf/map/battle.conf"
    order = [path.relative_to(HERCULES).as_posix() for path in ordered_config_files(root)]
    imports = [name for name in order if name.startswith("conf/import/")]
    if not imports:
        raise ExportError("conf/map/battle.conf loads no conf/import file; review the precedence rule")
    return {
        "id": "config-precedence",
        "category": "Provenance",
        "title": "Where server values come from (import precedence)",
        "summary": "Every rate and rule in this Guide is the effective value: stock settings, then this server's overrides applied in the order Hercules loads them.",
        "details": [
            "Hercules reads its stock configuration first. Files in conf/import/ are read last and replace stock values, so a number you see here is the import value when one exists.",
            "Battle settings are read in this order (later wins): " + " -> ".join(order),
            f"{len(order)} files take part; {len(imports)} of them are server overrides ({', '.join(imports)}).",
            "The Guide does not read the live server. It shows what was exported at the Hercules revision named under Source configuration, so a server that changed afterwards can differ until the data is re-exported.",
        ],
        "sources": [{"path": name, "record": "battle settings load order"} for name in order[:1] + imports],
    }


def discovery_scope_rule() -> dict[str, object]:
    path = "npc/custom/korangar_discovery.txt"
    require_source(path, "never their party peers")
    return {
        "id": "discovery-scope",
        "category": "Provenance",
        "title": "What discovery does and does not unlock",
        "summary": "Reference data is open from the start; discovery only records your own encounters.",
        "details": [
            "Mechanical reference data (monsters, items, skills, maps) is searchable without having met the monster or visited the map.",
            "The server keeps a discovery ledger of encounter milestones and visited maps. It is scoped to your account and is never shared with party members.",
            "Discovery adds history and badges to Guide entries. It does not hide or reveal mechanical information.",
            "Campaign (DM session) unlocks are separate and are not part of the account ledger.",
        ],
        "sources": [{"path": path, "record": "account discovery ledger and its scope comment"}],
    }


def quest_guidance_rule() -> dict[str, object]:
    require_source("src/map/quest.c", "quest")
    return {
        "id": "quest-guidance",
        "category": "Provenance",
        "title": "Where quest guidance comes from",
        "summary": "Route buttons and quest outlines are client features built from exported data; the server does not decide what guidance you see.",
        "details": [
            "The server sends a quest's kill objectives and progress. It does not send item turn-in lists, NPC locations or routes.",
            "Item requirements, turn-in locations and outlines for campaign contracts come from data exported from the server's own scripts and bundled with the client.",
            "Guidance and routing are your client's settings. Nothing about them is stored on the server.",
        ],
        "sources": [{"path": "src/map/quest.c", "record": "quest packets carry kill objectives only"}],
    }


def dm_mode_rule() -> dict[str, object]:
    console = "npc/custom/dm_campaign/shared/dm_console.txt"
    common = "npc/custom/dm_campaign/shared/dm_common.txt"
    require_source(console, "A session with no party would unlock the campaign for every solo player.")
    require_source(common, "$dm_mode && $dm_active_party > 0")
    require_source("src/map/mob.c", "mob_dm_mode_should_suppress")
    return {
        "id": "dm-mode",
        "category": "Provenance",
        "title": "DM mode: what it switches on",
        "summary": "A DM turns a campaign session on for one party. Campaign content reacts only to that party.",
        "details": [
            "Turning DM mode on needs a party; the session is bound to that party.",
            "Campaign NPCs and story state respond only while DM mode is on and only to members of the party the session is bound to. Everyone else sees ordinary NPCs.",
            "While any session is on, map-spawned MVP and boss monsters are held back server-wide (re-checked every 10 seconds). This applies to the whole server, not only the DM's party.",
            "Turning DM mode off ends the session; story progress is kept for the party.",
        ],
        "sources": [
            {"path": console, "record": "@dm mode on / off"},
            {"path": common, "record": "DM_SessionAllows"},
            {"path": "src/map/mob.c", "record": "mob_dm_mode_should_suppress"},
        ],
    }


def build() -> dict[str, object]:
    battle = effective_settings(HERCULES / "conf/map/battle.conf")
    inter = effective_settings(HERCULES / "conf/common/inter-server.conf")
    recovery_path = HERCULES / "npc/custom/korangar_death_recovery.txt"
    groups_path = HERCULES / "conf/groups.conf"
    groups_text = _strip_comments(groups_path.read_text(encoding="utf-8", errors="replace"))
    player_group = re.search(r"\bid\s*:\s*0\b([\s\S]*?)(?=\n\s*\},)", groups_text)
    if not player_group:
        raise ExportError("default player group (id 0) is missing from conf/groups.conf")
    player_commands = player_group.group(1)
    required_commands = ("autopickup", "autoloot", "alootid", "autoloottype", "noautolootid", "skreset", "streset", "refundskill")
    missing_commands = [name for name in required_commands if not re.search(rf"\b{re.escape(name)}\s*:\s*true\b", player_commands)]
    if missing_commands:
        raise ExportError(f"default player group commands are missing or disabled: {', '.join(missing_commands)}")
    drops_path = HERCULES / "conf/map/battle/drops.conf"
    drops = effective_settings(drops_path)
    if not recovery_path.is_file():
        raise ExportError(f"recovery script missing: {recovery_path}")
    recovery_text = _strip_comments(recovery_path.read_text(encoding="utf-8", errors="replace"))
    recovery: dict[str, int | bool] = {}
    for field in ("enabled", "kills_required", "refund_percent", "save_radius"):
        match = re.search(rf"\.{field}\s*=\s*(true|false|-?\d+)\s*;", recovery_text)
        if not match:
            raise ExportError(f"recovery setting .{field} is missing or not a literal")
        raw = match.group(1)
        recovery[field] = raw == "true" if raw in ("true", "false") else int(raw)

    fields = {
        "base_exp_rate": required(battle, "base_exp_rate"),
        "job_exp_rate": required(battle, "job_exp_rate"),
        "quest_exp_rate": required(battle, "quest_exp_rate"),
        "item_rate_common": required(battle, "item_rate_common"),
        "item_rate_common_boss": required(battle, "item_rate_common_boss"),
        "item_rate_heal": required(battle, "item_rate_heal"),
        "item_rate_heal_boss": required(battle, "item_rate_heal_boss"),
        "item_rate_use": required(battle, "item_rate_use"),
        "item_rate_use_boss": required(battle, "item_rate_use_boss"),
        "item_rate_equip": required(battle, "item_rate_equip"),
        "item_rate_equip_boss": required(battle, "item_rate_equip_boss"),
        "item_rate_card": required(battle, "item_rate_card"),
        "item_rate_card_boss": required(battle, "item_rate_card_boss"),
        "item_rate_mvp": required(battle, "item_rate_mvp"),
        "item_rate_adddrop": required(battle, "item_rate_adddrop"),
        "item_rate_add_chain": required(battle, "item_rate_add_chain"),
        "item_rate_treasure": required(battle, "item_rate_treasure"),
        "item_logarithmic_drops": required(battle, "item_logarithmic_drops"),
        "party_even_share_bonus": required(battle, "party_even_share_bonus"),
        "party_share_level": required(inter, "party_share_level"),
        "death_penalty_type": required(battle, "death_penalty_type"),
        "death_penalty_base": required(battle, "death_penalty_base"),
        "death_penalty_job": required(battle, "death_penalty_job"),
        "zeny_penalty": required(battle, "zeny_penalty"),
    }
    recovery_details = {
        "path": recovery_path.relative_to(HERCULES).as_posix(),
        "record": "OnInit recovery settings",
    }
    attr_fix_path = HERCULES / "db/re/attr_fix.conf"
    elemental_adjustments = parse_element_adjustments(attr_fix_path)
    level_penalty_path = HERCULES / "db/re/level_penalty.conf"
    size_fix_path = HERCULES / "db/re/size_fix.txt"
    size_adjustments = parse_weapon_size_adjustments(size_fix_path)
    element_names = {
        "Ele_Neutral": "Neutral", "Ele_Water": "Water", "Ele_Earth": "Earth",
        "Ele_Fire": "Fire", "Ele_Wind": "Wind", "Ele_Poison": "Poison",
        "Ele_Holy": "Holy", "Ele_Dark": "Shadow", "Ele_Ghost": "Ghost", "Ele_Undead": "Undead",
    }
    element_rules = [
        {
            "id": f"element-matchup-{defender.removeprefix('Ele_').lower()}",
            "category": "Mechanics",
            "title": f"{element_names[defender]} element attack matchups",
            "summary": f"Configured damage adjustment against {element_names[defender]} defenses by attacking element and defense level.",
            "details": [
                f"Defense element level {level}: " + "; ".join(
                    f"{element_names[attacker]} attacks {rate}%"
                    for attacker, rate in sorted(attacks.items(), key=lambda row: element_names[row[0]])
                ) + "."
                for level, attacks in sorted(levels.items())
            ] + [
                "100% is the base elemental adjustment; values below or above 100% reduce or increase this table component of damage.",
                "Final damage can also change through skill, card, equipment, status, map, and other server modifiers; this table alone is not a complete damage formula.",
            ],
            "sources": [{"path": attr_fix_path.relative_to(HERCULES).as_posix(), "record": defender}],
        }
        for defender, levels in sorted(elemental_adjustments.items(), key=lambda row: element_names[row[0]])
    ]
    penalty_type = value(battle, "death_penalty_type")
    if penalty_type not in (0, 1, 2):
        raise ExportError(f"unsupported death_penalty_type {penalty_type}; review the player-facing description")
    penalty_basis = {
        0: "EXP death penalty is disabled by configuration.",
        1: "Death penalty rates apply to EXP earned in the current level.",
        2: "Death penalty rates apply to total stored EXP.",
    }[penalty_type]
    exp_loss_basis = "current-level EXP" if penalty_type == 1 else "total stored EXP"
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "sources": [
            "conf/map/battle.conf and its ordered includes/imports",
            "conf/common/inter-server.conf and its ordered imports",
            recovery_path.relative_to(HERCULES).as_posix(),
            groups_path.relative_to(HERCULES).as_posix(),
            drops_path.relative_to(HERCULES).as_posix(),
            attr_fix_path.relative_to(HERCULES).as_posix(),
            size_fix_path.relative_to(HERCULES).as_posix(),
            level_penalty_path.relative_to(HERCULES).as_posix(),
        ],
        "entries": [
            {
                "id": "player-loot-and-resets",
                "category": "Mechanics",
                "title": "Player loot and reset commands",
                "summary": "Default player accounts have the listed pickup, auto-loot, and respec commands.",
                "details": [
                    "Group 0 (the default player group) can use @autopickup, @autoloot, @alootid, @autoloottype, and @noautolootid.",
                    f"Automatic ground pickup starts at {value(drops, 'autopickup_radius')} cells; parties impose this configured radius. Player pickup reservations, party sharing, map flags, weight, and inventory capacity still apply.",
                    f"Auto-loot drop-rate checks {'include' if drops['autoloot_adjust']['value'] else 'do not include'} player drop-rate bonuses and penalties.",
                    "Group 0 can use @streset and @skreset for free stat and skill resets, plus @refundskill to refund eligible skills.",
                    "The command permissions and defaults are configured values; per-character auto-loot selections and pickup settings can differ at runtime.",
                ],
                "sources": [
                    {"path": groups_path.relative_to(HERCULES).as_posix(), "record": "groups[0].commands"},
                    *sources(required(drops, "autopickup_radius"), required(drops, "autoloot_adjust")),
                    {"path": "src/map/atcommand.c", "record": "@streset, @skreset, and @refundskill handlers"},
                ],
            },
            {
                "id": "experience-and-drops",
                "category": "Rates",
                "title": "Experience and drop modifiers",
                "summary": "Configured base, job, and quest EXP rates are shown separately from item drop modifiers.",
                "details": [
                    f"Base EXP: {value(battle, 'base_exp_rate')}%.",
                    f"Job EXP: {value(battle, 'job_exp_rate')}%.",
                    f"Quest EXP: {value(battle, 'quest_exp_rate')}%.",
                    f"Common-item drop modifier: {value(battle, 'item_rate_common')}%; boss common-item modifier: {value(battle, 'item_rate_common_boss')}%.",
                    f"Healing-item modifier: {value(battle, 'item_rate_heal')}%; boss healing-item modifier: {value(battle, 'item_rate_heal_boss')}%.",
                    f"Usable-item modifier: {value(battle, 'item_rate_use')}%; boss usable-item modifier: {value(battle, 'item_rate_use_boss')}%.",
                    f"Equipment modifier: {value(battle, 'item_rate_equip')}%; boss equipment modifier: {value(battle, 'item_rate_equip_boss')}%.",
                    f"Card drop modifier: {value(battle, 'item_rate_card')}%; boss card modifier: {value(battle, 'item_rate_card_boss')}%.",
                    f"MVP inventory-drop modifier: {value(battle, 'item_rate_mvp')}%.",
                    f"Script bonus drops: {value(battle, 'item_rate_adddrop')}%; item-chain drops: {value(battle, 'item_rate_add_chain')}%; treasure drops: {value(battle, 'item_rate_treasure')}%.",
                    f"Logarithmic drop scaling: {'enabled' if battle['item_logarithmic_drops']['value'] else 'disabled'}.",
                    "These are server modifiers, not final per-monster drop chances; the item’s exported drop rate remains a separate value.",
                ],
                "sources": sources(*(fields[key] for key in (
                    "base_exp_rate", "job_exp_rate", "quest_exp_rate", "item_rate_common", "item_rate_common_boss",
                    "item_rate_heal", "item_rate_heal_boss", "item_rate_use", "item_rate_use_boss", "item_rate_equip",
                    "item_rate_equip_boss", "item_rate_card", "item_rate_card_boss", "item_rate_mvp", "item_rate_adddrop",
                    "item_rate_add_chain", "item_rate_treasure", "item_logarithmic_drops",
                ))),
            },
            {
                "id": "party-experience",
                "category": "Party",
                "title": "Party experience sharing",
                "summary": "Eligible even-share parties receive the configured bonus before EXP is divided.",
                "details": [
                    f"Even-share bonus: {value(battle, 'party_even_share_bonus')}% per additional eligible member.",
                    f"Maximum member level spread: {value(inter, 'party_share_level')} levels.",
                    "Party EXP sharing is map-scoped; this configuration does not set a cell-distance limit.",
                    "The 30-level spread is a playtest candidate, not a final balance decision.",
                ],
                "sources": sources(fields["party_even_share_bonus"], fields["party_share_level"], {
                    "source": {"path": "src/map/pc.c", "record": "party EXP eligibility and same-map check"}
                }),
            },
            {
                "id": "death-and-recovery",
                "category": "Progression",
                "title": "Death penalty and recovery",
                "summary": "Death-penalty rates and the Korangar recovery rule, sourced from effective configuration.",
                "details": [
                    penalty_basis,
                    f"Configured base EXP penalty rate: {value(battle, 'death_penalty_base') / 100:.2f}%" + (f" of {exp_loss_basis}." if penalty_type else " (inactive while the penalty type is disabled)."),
                    f"Configured job EXP penalty rate: {value(battle, 'death_penalty_job') / 100:.2f}%" + (f" of {exp_loss_basis}." if penalty_type else " (inactive while the penalty type is disabled)."),
                    f"Configured Zeny death penalty rate: {value(battle, 'zeny_penalty') / 100:.2f}%.",
                    f"Recovery enabled: {'yes' if recovery['enabled'] else 'no'}.",
                    f"When enabled, recovery returns {recovery['refund_percent']}% of measured EXP losses after {recovery['kills_required']} eligible same-map monster kills or arrival within {recovery['save_radius']} cells of the save point.",
                    "Map flags, class exemptions, or a zero actual loss can prevent a death penalty and therefore leave nothing to recover.",
                ],
                "sources": sources(*(fields[key] for key in (
                    "death_penalty_type", "death_penalty_base", "death_penalty_job", "zeny_penalty",
                ))) + [
                    {"path": recovery_details["path"], "record": recovery_details["record"]},
                    {"path": "src/map/pc.c", "record": "death penalty calculation and map/job exemptions"},
                ],
            },
            *element_rules,
            level_penalty_rule(level_penalty_path),
            derived_stat_formula_rule(battle),
            stat_point_rule(),
            max_hp_sp_rule(),
            aspd_rule(),
            {
                "id": "weapon-size-adjustments",
                "category": "Mechanics",
                "title": "Weapon damage by target size",
                "summary": "Configured size adjustments for the weapon damage component, by weapon type and target size.",
                "details": [
                    f"{size} targets: " + "; ".join(f"{weapon} {rate}%" for weapon, rate in values.items()) + "."
                    for size, values in size_adjustments.items()
                ] + [
                    "100% is full weapon-component damage; a lower value reduces that component.",
                    "This table is specifically the weapon size adjustment from size_fix.txt; it is not a universal multiplier for every skill or the complete final damage formula.",
                ],
                "sources": [
                    {"path": size_fix_path.relative_to(HERCULES).as_posix(), "record": "Small, Medium, and Large rows; documented weapon columns"},
                    {"path": "src/map/status.c", "record": "status_read_job_db loads db/re/size_fix.txt"},
                    {"path": "src/map/battle.c", "record": "weapon damage calculation uses target size adjustment"},
                ],
            },
            dm_campaign_rule(battle),
            view_distance_rule(battle),
            natural_recovery_rule(battle),
            progression_limits_rule(battle),
            equipment_refinement_rule(),
            card_socketing_rule(),
            first_job_progression_rule(),
            second_job_progression_rule(),
            config_precedence_rule(),
            discovery_scope_rule(),
            quest_guidance_rule(),
            dm_mode_rule(),
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check generated-data drift")
    args = parser.parse_args()
    try:
        data = build()
    except (OSError, ExportError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    rendered = json.dumps(data, indent=1, ensure_ascii=False) + "\n"
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != rendered:
            print(f"stale: {OUTPUT.relative_to(ROOT)} — re-run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"up to date: {len(data['entries'])} server-rule entries")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(data['entries'])} server-rule entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
