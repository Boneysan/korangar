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
