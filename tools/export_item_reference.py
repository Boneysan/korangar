#!/usr/bin/env python3
"""Export versioned item/card reference data from the configured Hercules DB.

The legacy `items.json` and `cards.json` have no reproducible original
exporter. This emits parallel v1 files so those consumers remain untouched:
`docs/items.v1.json` and `docs/cards.v1.json`.

Only a small allowlisted set of unconditional script effects is translated.
Conditional or unsupported scripts remain marked as untranslated.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from functools import lru_cache
from pathlib import Path
from typing import Any

from export_skill_info import _strip_comments, parse_skill_db
from generate_navigation_graph import loaded_script_files, map_names


ROOT = Path(__file__).resolve().parent.parent
HERCULES = ROOT.parent / "Hercules"
ITEM_SOURCES = (HERCULES / "db/re/item_db.conf", HERCULES / "db/item_db2.conf")
MOB_SOURCE = HERCULES / "db/re/mob_db.conf"
COMBO_SOURCE = HERCULES / "db/re/item_combo_db.conf"
ITEM_GROUP_SOURCE = HERCULES / "db/re/item_group.conf"
SKILL_DB_SOURCE = HERCULES / "db/re/skill_db.conf"
ITEMS_OUTPUT = ROOT / "docs/items.v1.json"
CARDS_OUTPUT = ROOT / "docs/cards.v1.json"
SCRIPT_FIELDS = ("Script", "OnEquipScript", "OnUnequipScript", "OnRentalStartScript", "OnRentalEndScript")
SCRIPT_FIELD = re.compile(r"\b(" + "|".join(SCRIPT_FIELDS) + r"):" + r"\s*<\".*?\">", re.S)
ITEM_GROUP_HEADER = re.compile(r"^\s*([A-Za-z0-9_]+)\s*:\s*\(", re.M)
ITEM_GROUP_ENTRY = re.compile(r'\s*(?:\(\s*"([^"\\]+)"\s*,\s*(\d+)\s*\)|"([^"\\]+)")\s*,?')
NPC_LOCATION = re.compile(r"^\s*([A-Za-z0-9_]+),(-?\d+),(-?\d+),(-?\d+)\s*$")
SHOP_COMMAND = re.compile(r"^\s*sellitem\s+([^;]+);", re.M)
NPC_MANIFEST = HERCULES / "npc/re/scripts_main.conf"

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
    raw_scripts: list[str] = []
    def preserve_script(match: re.Match[str]) -> str:
        raw_scripts.append(match.group(0).split('<"', 1)[1][:-2])
        return f'{match.group(1)}: "__SCRIPT_PRESENT__"'
    scrubbed, count = SCRIPT_FIELD.subn(preserve_script, text)
    starts = len(re.findall(r"\b(?:" + "|".join(SCRIPT_FIELDS) + r"):" + r"\s*<\"", text))
    if starts != count:
        raise ValueError(f"could not safely delimit every script value in {path}: {starts} starts, {count} complete values")
    rows = parse_skill_db(scrubbed)
    expected_scripts = sum(
        sum(row.get(field) == "__SCRIPT_PRESENT__" for field in SCRIPT_FIELDS) for row in rows
    )
    # Item DB headers include five commented script-field examples which are
    # matched by the syntax scanner but intentionally ignored by the parser.
    if len(raw_scripts) == expected_scripts + len(SCRIPT_FIELDS) and raw_scripts[0].strip().startswith("Script\n"):
        raw_scripts = raw_scripts[len(SCRIPT_FIELDS):]
    scripts_align = len(raw_scripts) == expected_scripts
    # Script fields occur in the same order as records and field keys. Attach
    # summaries only when every present field is covered by the conservative
    # translator below; never turn a partial interpretation into a full one.
    cursor = 0
    for row in rows if scripts_align else []:
        fields = [field for field in SCRIPT_FIELDS if row.get(field) == "__SCRIPT_PRESENT__"]
        scripts = raw_scripts[cursor:cursor + len(fields)]
        cursor += len(fields)
        summaries = [summary for summary in (translate_simple_effect(script) for script in scripts) if summary]
        if fields and len(summaries) == len(fields):
            row["__effect_summary"] = "; ".join(summaries)
    if scripts_align and cursor != len(raw_scripts):
        raise ValueError(f"script fields could not be associated with records in {path}")
    return rows


def _number(value: str) -> str | None:
    if re.fullmatch(r"-?\d+", value.strip()):
        return str(int(value.strip()))
    match = re.fullmatch(r"rand\(\s*(-?\d+)\s*,\s*(-?\d+)\s*\)", value.strip(), re.I)
    return f"{match.group(1)}–{match.group(2)}" if match else None


# Equip slots whose refine level combos commonly scale a bonus by. Restricted to
# slots with a single, unambiguous player-facing name; anything else is left
# untranslated rather than guessed at.
_REFINE_SLOT_LABELS = {
    "EQI_HAND_R": "the weapon", "EQI_HAND_L": "the weapon (or shield/offhand)",
    "EQI_HEAD_TOP": "the headgear", "EQI_ARMOR": "the armor", "EQI_SHOES": "the shoes",
    "EQI_GARMENT": "the garment", "EQI_ACC_L": "the accessory", "EQI_ACC_R": "the accessory",
}


def _parse_refine_scaled_value(expr: str) -> tuple[int, int, str] | None:
    """Recognize `<base> + (getequiprefinerycnt(<slot>) * <coef>)` and bare-refine forms.

    Only literal integer base/coefficient forms against a known equip slot are
    accepted; anything else (nested expressions, unknown slots, non-integer
    coefficients) returns None so the caller leaves the script untranslated.
    """
    compact = expr.strip()

    # Handle patterns like: 20+getrefine()/2, getrefine()*2, 3*getrefine(), (getrefine()-5)
    # Note: getrefine() is shorthand for getequiprefinerycnt(EQI_HAND_R) in many contexts

    # Pattern: base + refine_expression (e.g., "20+getrefine()/2")
    match = re.fullmatch(
        r"(-?\d+)\s*\+\s*(.+)", compact, re.I
    )
    if match:
        base = int(match.group(1))
        refine_expr = match.group(2).strip()
        parsed = _parse_refine_expression(refine_expr)
        if parsed:
            return (base, parsed[0], parsed[1])
        return None

    # Direct refine expression without base
    parsed = _parse_refine_expression(compact)
    if parsed:
        return (0, parsed[0], parsed[1])

    return None


def _parse_refine_expression(expr: str) -> tuple[int, str] | None:
    """Parse a refine-scaled expression like getrefine()*2 or (getrefine()-5)*2."""
    compact = expr.strip()

    # Pattern with parentheses: (getrefine()-5)*2 or ((getrefine()-5)*2)
    match = re.fullmatch(
        r"\(\s*getrefine\(\)\s*([+-])\s*(-?\d+)\s*\)\s*\*\s*(-?\d+)",
        compact, re.I
    )
    if match:
        op, subtracted, coef = match.groups()
        # (getrefine() - 5) * 2 becomes base_offset = -10, coef = 2
        subtracted_val = int(subtracted)
        coef_val = int(coef)
        base_offset = -subtracted_val * coef_val if op == "-" else subtracted_val * coef_val
        return (coef_val, "EQI_HAND_R")  # getrefine() defaults to weapon slot

    # Pattern: getrefine() * coef or getrefine()*coef
    match = re.fullmatch(
        r"getrefine\(\)\s*\*\s*(-?\d+)",
        compact, re.I
    )
    if match:
        coef = int(match.group(1))
        return (coef, "EQI_HAND_R")

    # Pattern: coef * getrefine()
    match = re.fullmatch(
        r"(-?\d+)\s*\*\s*getrefine\(\)",
        compact, re.I
    )
    if match:
        coef = int(match.group(1))
        return (coef, "EQI_HAND_R")

    # Pattern: getrefine() with optional division: getrefine()/2 or getrefine() / 2
    match = re.fullmatch(
        r"getrefine\(\)\s*/\s*(-?\d+)",
        compact, re.I
    )
    if match:
        divisor = int(match.group(1))
        # Division makes it non-integer for many refine levels; skip for safety
        return None

    # Pattern: (getrefine()-5) without multiplier - returns getrefine()-5 which can be negative
    match = re.fullmatch(
        r"\(\s*getrefine\(\)\s*([+-])\s*(-?\d+)\s*\)",
        compact, re.I
    )
    if match:
        op, subtracted = match.groups()
        # This form is ambiguous for direction (can flip sign across refine range)
        return None

    return None


def _refine_scaled_phrase(base: int, coef: int, slot_key: str) -> str | None:
    """Render a parsed refine-scaled value, rejecting forms whose sign can flip
    across the possible refine range (0-20), since that direction is not a
    single unambiguous claim."""
    if coef == 0:
        return None
    # getrefine() defaults to EQI_HAND_R (weapon) - use that slot label
    slot_label = _REFINE_SLOT_LABELS.get(slot_key, f"refined equipment ({slot_key})")

    # If base is non-zero and has opposite sign to coef, the effect direction could flip
    # depending on refine level. This is too ambiguous for a single claim.
    if base != 0 and ((base > 0) != (coef > 0)):
        return None

    if base == 0:
        direction = "increases" if coef >= 0 else "reduces"
        return f"{direction} by {abs(coef)}% per refine level of {slot_label}"

    direction = "increases" if base >= 0 else "reduces"
    return f"{direction} by {abs(base)}%, plus {abs(coef)}% per refine level of {slot_label}"


def translate_simple_effect(script: str) -> str | None:
    """Translate only complete scripts made of known unconditional commands."""
    body = _strip_comments(script).strip()
    if not body:
        return None
    conditional_effects: dict[str, tuple[str, str]] = {}
    scale_effects: dict[str, tuple[str, int, int, int]] = {}
    level_scale = re.compile(
        r"if\s*\(\s*BaseLevel\s*>\s*(\d+)\s*\)\s*\{\s*bonus\s+(bAtk|bBaseAtk|bMatk)\s*,\s*\(?\s*\(?\s*\(?\s*BaseLevel\s*-\s*(\d+)\s*\)?\s*/\s*(\d+)\s*\)?\s*\*\s*(\d+)\s*\)?\s*;\s*\}",
        re.I | re.S,
    )

    def capture_level_scale(match: re.Match[str]) -> str:
        threshold, stat, formula_threshold, divisor, amount = match.groups()
        if threshold != formula_threshold or int(divisor) <= 0:
            return match.group(0)
        key = f"__LEVEL_SCALE_{len(scale_effects)}__"
        scale_effects[key] = (stat, int(threshold), int(divisor), int(amount))
        return key + ";"

    body = level_scale.sub(capture_level_scale, body)

    # Preserve simple one-command branches. Nested/multi-command branches remain
    # untranslated because their full control flow has not been interpreted.
    simple_if = re.compile(
        r"if\s*\(\s*(?P<condition>[^{};]+?)\s*\)\s*(?:\{\s*(?P<effect>bonus(?:2|3|4)?\s+[^;]+;)\s*\}|(?P<inline>bonus(?:2|3|4)?\s+[^;]+))",
        re.I | re.S,
    )

    def capture_simple_if(match: re.Match[str]) -> str:
        match_dict = match.groupdict()
        effect = match_dict["effect"] or match_dict["inline"]
        return capture_condition_proxy(match.group("condition"), effect)

    def capture_condition_proxy(condition: str, effect: str) -> str:
        key = f"__CONDITIONAL_EFFECT_{len(conditional_effects)}__"
        conditional_effects[key] = (condition.strip(), effect.strip().rstrip(";"))
        return key + ";"

    body = simple_if.sub(capture_simple_if, body)
    statements = [part.strip() for part in body.split(";") if part.strip()]
    if not statements or re.search(r"\b(if|else|for|while|callfunc|callsub|autobonus)\b", body, re.I):
        return None
    effects: list[str] = []
    cures = {
        "SC_POISON": "Poison", "SC_SILENCE": "Silence", "SC_BLIND": "Blind",
        "SC_CONFUSION": "Confusion", "SC_ILLUSION": "Hallucination", "SC_BLEEDING": "Bleeding",
        "SC_CURSE": "Curse", "SC_FEAR": "Fear", "SC_STUN": "Stun",
        "SC_FREEZE": "Freeze", "SC_STONE": "Stone Curse", "SC_SLEEP": "Sleep",
        "SC_BURNING": "Burning", "SC_FROSTBITE": "Frostbite", "SC_DEEPSLEEP": "Deep Sleep",
    }
    status_names = {
        **cures,
        "SC_BLESSING": "Blessing", "SC_INC_AGI": "Increase Agility", "SC_WINDWALK": "Wind Walk",
        "SC_MAGNIFICAT": "Magnificat", "SC_GLORIA": "Gloria", "SC_ASSUMPTIO": "Assumptio",
        "SC_KYRIE": "Kyrie Eleison", "SC_IMPOSITIO": "Impositio Manus", "SC_ANGELUS": "Angelus",
        "SC_CONCENTRATION": "Concentration", "SC_TWOHANDQUICKEN": "Two-Hand Quicken",
        "SC_ONEHAND": "One-Hand Quicken", "SC_ADRENALINE": "Adrenaline Rush",
        "SC_OVERTHRUST": "Overthrust", "SC_MAXIMIZEPOWER": "Maximize Power",
        "SC_ENCPOISON": "Enchant Poison", "SC_ASPDPOTION0": "ASPD Potion",
        "SC_ASPDPOTION1": "Frenzy Drink", "SC_ASPDPOTION2": "Berserk Potion",
        "SC_ASPDPOTION3": "Speed Potion", "SC_STRFOOD": "STR Food", "SC_AGIFOOD": "AGI Food",
        "SC_VITFOOD": "VIT Food", "SC_INTFOOD": "INT Food", "SC_DEXFOOD": "DEX Food",
        "SC_LUKFOOD": "LUK Food", "SC_ATKPOTION": "ATK Potion", "SC_MATKPOTION": "MATK Potion",
        "SC_FOOD_STR": "STR food", "SC_FOOD_AGI": "AGI food", "SC_FOOD_VIT": "VIT food",
        "SC_FOOD_INT": "INT food", "SC_FOOD_DEX": "DEX food", "SC_FOOD_LUK": "LUK food",
    }
    for statement in statements:
        parenthesized = re.fullmatch(r"bonus\((b[A-Za-z0-9_]+)\s*,\s*(-?\d+)\)", statement, re.I)
        if parenthesized:
            statement = f"bonus {parenthesized.group(1)},{parenthesized.group(2)}"
        if statement in conditional_effects:
            condition, nested_effect = conditional_effects[statement]
            condition_label = describe_item_condition(condition)
            translated = translate_simple_effect(nested_effect)
            if condition_label is None or translated is None:
                return None
            effects.append(f"{condition_label}: {translated}")
            continue
        if statement in scale_effects:
            stat, threshold, divisor, amount = scale_effects[statement]
            label = {"batk": "ATK", "bbaseatk": "base ATK", "bmatk": "MATK"}.get(stat.lower())
            if label is None:
                return None
            effects.append(f"increases {label} by {amount} for each full {divisor} Base Levels above {threshold}")
            continue
        match = re.fullmatch(r"bonus\s+(bAtkRate|bMatkRate|bStr|bAgi|bVit|bInt|bDex|bLuk|bAllStats|bMaxHP|bMaxSP|bMaxHPrate|bMaxSPrate|bHit|bFlee|bFlee2|bCritical|bAtk|bMatk|bDef|bMdef|bAspdRate|bCritAtkRate|bLongAtkRate|bSPrecovRate|bHPrecovRate|bUseSPrate|bDelayrate)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            labels = {
                "batkrate": "physical damage", "bmatkrate": "magic damage", "bstr": "STR", "bagi": "AGI",
                "bvit": "VIT", "bint": "INT", "bdex": "DEX", "bluk": "LUK", "bmaxhp": "maximum HP",
                "ballstats": "all six primary stats (STR, AGI, VIT, INT, DEX, and LUK)",
                "bmaxsp": "maximum SP", "bmaxhprate": "maximum HP", "bmaxsprate": "maximum SP",
                "bhit": "HIT", "bflee": "FLEE", "bflee2": "Perfect Dodge", "bcritical": "critical chance",
                "batk": "ATK", "bmatk": "MATK", "bdef": "equipment DEF", "bmdef": "equipment MDEF",
                "baspdrate": "attack speed", "bcritatkrate": "critical damage",
                "blongatkrate": "ranged (long-range) physical damage", "bsprecovrate": "SP regeneration rate",
                "bhprecovrate": "HP regeneration rate", "busesprate": "the SP cost of skills",
                "bdelayrate": "attack delay",
            }
            key, value = match.group(1).lower(), int(match.group(2))
            unit = "%" if key in {
                "batkrate", "bmatkrate", "bmaxhprate", "bmaxsprate", "baspdrate", "bcritical",
                "bcritatkrate", "blongatkrate", "bsprecovrate", "bhprecovrate", "busesprate", "bdelayrate",
            } else ""
            direction = "increases" if value >= 0 else "reduces"
            effects.append(f"{direction} {labels[key]} by {abs(value)}{unit}")
            continue
        match = re.fullmatch(
            r"bonus\s+(bBaseAtk|bAspd|bSpeedRate|bSpeedAddRate|bVariableCastrate)\s*,\s*(-?\d+)",
            statement,
            re.I,
        )
        if match:
            key, value = match.group(1).lower(), int(match.group(2))
            labels = {
                "bbaseatk": ("basic attack power", ""),
                "baspd": ("attack speed", ""),
                "bspeedrate": ("movement speed", "%"),
                "bspeedaddrate": ("movement speed", "%"),
                "bvariablecastrate": ("variable cast time of all skills", "%"),
            }
            label, unit = labels[key]
            direction = "increases" if value >= 0 else "reduces"
            note = ""
            if key == "bspeedrate":
                note = " (only the highest bonus applies)"
            effects.append(f"{direction} {label} by {abs(value)}{unit}{note}")
            continue
        match = re.fullmatch(
            r"bonus\s+(bUnbreakableWeapon|bUnbreakableArmor|bUnbreakableHelm|bUnbreakableShield|bUnbreakableGarment|bUnbreakableShoes)\s*,\s*-?\d+",
            statement,
            re.I,
        )
        if match:
            slot = {
                "bunbreakableweapon": "weapon",
                "bunbreakablearmor": "armor",
                "bunbreakablehelm": "helm",
                "bunbreakableshield": "shield",
                "bunbreakablegarment": "garment",
                "bunbreakableshoes": "shoes",
            }[match.group(1).lower()]
            effects.append(f"the equipped {slot} cannot be broken")
            continue
        match = re.fullmatch(r"bonus\s+bUnbreakable\s*,\s*(-?\d+)", statement, re.I)
        if match:
            value = int(match.group(1))
            direction = "reduces" if value >= 0 else "increases"
            effects.append(f"{direction} the chance equipped items break by {abs(value)}%")
            continue
        match = re.fullmatch(r"bonus\s+(bBreakWeaponRate|bBreakArmorRate)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            # rate is compared against rnd()%10000 in skill_break_equip (src/map/skill.c), so
            # value/100 is the percent chance; the roll happens on a physical attack that hits.
            bonus, raw_value = match.groups()
            value = int(raw_value)
            target = "the target's weapon" if bonus.lower() == "bbreakweaponrate" else "the target's armor"
            chance = abs(value) / 100
            if value >= 0:
                effects.append(f"on a physical attack, has a {chance:g}% chance to break {target}")
            else:
                effects.append(f"reduces an existing chance to break {target} by {chance:g}%")
            continue
        match = re.fullmatch(r"bonus2\s+(bMagicAddRace|bExpAddRace)\s*,\s*(RC_[A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            bonus, race_constant, raw_value = match.groups()
            race = race_constant[3:].replace("DemiPlayer", "player").replace("_", " ").lower()
            value = int(raw_value)
            direction = "increases" if value >= 0 else "reduces"
            if bonus.lower() == "bmagicaddrace":
                effects.append(f"{direction} magic damage against {race} targets by {abs(value)}%")
            else:
                effects.append(f"{direction} EXP gained from {race} targets by {abs(value)}%")
            continue
        match = re.fullmatch(r"bonus2\s+bSkillAtk\s*,\s*([A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            skill, raw_value = match.groups()
            value = int(raw_value)
            direction = "increases" if value >= 0 else "reduces"
            effects.append(f"{direction} {_skill_display_name(skill)} damage by {abs(value)}%")
            continue
        match = re.fullmatch(r"bonus2\s+bSkillUseSP\s*,\s*([A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            skill, raw_value = match.groups()
            value = int(raw_value)
            direction = "reduces" if value >= 0 else "increases"
            effects.append(f"{direction} the SP cost of {_skill_display_name(skill)} by {abs(value)}")
            continue
        match = re.fullmatch(r"bonus2\s+bSkillCooldown\s*,\s*([A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            skill, raw_value = match.groups()
            value = int(raw_value)
            # Negative cooldown values mean "increases duration" in seconds
            direction = "increases" if value >= 0 else "reduces"
            effects.append(f"{direction} the cooldown of {_skill_display_name(skill)} by {abs(value)} ms")
            continue
        bare_itemskill = re.fullmatch(r"itemskill\s+([A-Za-z0-9_]+)\s*,\s*(\d+)", statement, re.I)
        called_itemskill = re.fullmatch(
            r"itemskill\(\s*([A-Za-z0-9_]+)\s*,\s*(\d+)(?:\s*,\s*([A-Za-z0-9_]+(?:\s*\|\s*[A-Za-z0-9_]+)*))?\s*\)",
            statement,
            re.I,
        )
        match = bare_itemskill or called_itemskill
        if match:
            skill, level, *flags = match.groups()
            phrase = _itemskill_phrase(skill, level, flags[0] if flags else None)
            if phrase is None:
                return None
            effects.append(phrase)
            continue
        match = re.fullmatch(r"bonus2\s+bSkillAtk\s*,\s*([A-Za-z0-9_]+)\s*,\s*(.+)", statement, re.I)
        if match:
            skill, raw_expr = match.groups()
            parsed = _parse_refine_scaled_value(raw_expr)
            phrase = _refine_scaled_phrase(*parsed) if parsed else None
            if phrase is None:
                return None
            effects.append(f"{_skill_display_name(skill)} damage {phrase}")
            continue
        match = re.fullmatch(r"bonus\s+(bAtk|bMatk|bAspdRate|bAtkRate|bMatkRate|bCritAtkRate|bLongAtkRate)\s*,\s*(.+)", statement, re.I)
        if match:
            bonus_name, raw_expr = match.groups()
            parsed = _parse_refine_scaled_value(raw_expr)
            phrase = _refine_scaled_phrase(*parsed) if parsed else None
            if phrase is None:
                return None
            labels = {
                "batk": "ATK", "bmatk": "MATK", "baspdrate": "attack speed", "batkrate": "physical damage",
                "bmatkrate": "magic damage", "bcritatkrate": "critical damage",
                "blongatkrate": "ranged (long-range) physical damage",
            }
            effects.append(f"{labels[bonus_name.lower()]} {phrase}")
            continue
        match = re.fullmatch(r"bonus2\s+(bAddSize|bMagicAddSize|bSubSize)\s*,\s*(Size_[A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            bonus, size_constant, raw_value = match.groups()
            size = size_constant.removeprefix("Size_").lower()
            value = int(raw_value)
            if bonus.lower() == "bsubsize":
                direction = "reduces damage from" if value >= 0 else "increases damage from"
                phrase = f"{direction} {size} targets by {abs(value)}%"
            else:
                direction = "increases" if value >= 0 else "reduces"
                damage_type = "magic" if bonus.lower() == "bmagicaddsize" else "physical"
                phrase = f"{direction} {damage_type} damage against {size} targets by {abs(value)}%"
            effects.append(phrase)
            continue
        match = re.fullmatch(r"bonus\s+(bHealPower|bHealPower2|bAddItemHealRate)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            bonus, raw_value = match.groups()
            value = int(raw_value)
            labels = {
                "bhealpower": "healing from skills you cast on yourself",
                "bhealpower2": "healing received from other players' skills",
                "badditemhealrate": "HP recovered from healing items",
            }
            direction = "increases" if value >= 0 else "reduces"
            effects.append(f"{direction} {labels[bonus.lower()]} by {abs(value)}%")
            continue
        match = re.fullmatch(r"bonus\s+(bHPDrainValue|bSPDrainValue)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            bonus, raw_value = match.groups()
            value = int(raw_value)
            resource = "HP" if bonus.lower() == "bhpdrainvalue" else "SP"
            direction = "recovers" if value >= 0 else "reduces the recovery by"
            effects.append(f"on a weapon attack against a monster, {direction} {abs(value)} {resource}")
            continue
        match = re.fullmatch(r"bonus2\s+bAddRace\s*,\s*(RC_[A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            race = match.group(1)[3:].replace("DemiPlayer", "player").replace("_", " ").lower()
            rate = int(match.group(2))
            direction = "increases" if rate >= 0 else "reduces"
            effects.append(f"{direction} damage against {race} targets by {abs(rate)}%")
            continue
        match = re.fullmatch(r"bonus2\s+bSubRace\s*,\s*(RC_[A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            race = match.group(1)[3:].replace("DemiPlayer", "player").replace("_", " ").lower()
            rate = int(match.group(2))
            direction = "reduces" if rate >= 0 else "increases"
            effects.append(f"{direction} damage from {race} targets by {abs(rate)}%")
            continue
        match = re.fullmatch(r"bonus2\s+bAddRaceTolerance\s*,\s*(RC_[A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            race = match.group(1)[3:].replace("DemiPlayer", "player").replace("_", " ").lower()
            rate = int(match.group(2))
            direction = "increases" if rate >= 0 else "reduces"
            effects.append(f"{direction} tolerance against {race} targets by {abs(rate)}%")
            continue
        match = re.fullmatch(r"bonus\s+bIgnoreDefRace\s*,\s*(RC_[A-Za-z0-9_]+)", statement, re.I)
        if match:
            race = match.group(1)[3:].replace("DemiPlayer", "player").replace("_", " ").lower()
            effects.append(f"ignores target DEF against {race} targets")
            continue
        match = re.fullmatch(r"bonus\s+bAtkEle\s*,\s*(Ele_[A-Za-z0-9_]+)", statement, re.I)
        if match:
            element = _element_display_name(match.group(1))
            effects.append(f"gives the player's attacks the {element} element")
            continue
        match = re.fullmatch(r"bonus\s+bDefEle\s*,\s*(Ele_[A-Za-z0-9_]+)", statement, re.I)
        if match:
            element = _element_display_name(match.group(1))
            effects.append(f"gives the player's defense the {element} element")
            continue
        match = re.fullmatch(r"bonus2\s+(bAddEle|bMagicAddEle|bMagicAtkEle|bSubEle)\s*,\s*(Ele_[A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            bonus, element_constant, raw_value = match.groups()
            element = _element_display_name(element_constant)
            value = int(raw_value)
            key = bonus.lower()
            if key == "bsubele":
                direction = "reduces damage from" if value >= 0 else "increases damage from"
                effect_text = f"{direction} {element} element attacks by {abs(value)}%"
            else:
                direction = "increases" if value >= 0 else "reduces"
                damage_type = {
                    "baddele": "physical",
                    "bmagicaddele": "magic",
                    "bmagicatkele": "elemental magic",
                }.get(key, "physical")
                if key == "bmagicatkele":
                    effect_text = f"{direction} {element}-element magic damage by {abs(value)}%"
                else:
                    effect_text = f"{direction} {damage_type} damage against {element} element targets by {abs(value)}%"
            effects.append(effect_text)
            continue
        match = re.fullmatch(r"bonus3\s+bAutoSpell\s*,\s*([A-Za-z0-9_]+)\s*,\s*(\d+)\s*,\s*(\d+)", statement, re.I)
        if match:
            skill, level, rate = match.groups()
            effects.append(f"on a normal attack, has a {int(rate) / 10:g}% chance to cast {_skill_display_name(skill)} Lv {level}")
            continue
        match = re.fullmatch(r"bonus4\s+bAutoSpell\s*,\s*([A-Za-z0-9_]+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*0", statement, re.I)
        if match:
            skill, level, rate = match.groups()
            effects.append(f"on a normal attack, has a {int(rate) / 10:g}% chance to cast {_skill_display_name(skill)} Lv {level}")
            continue
        match = re.fullmatch(r"bonus3\s+bAutoSpellWhenHit\s*,\s*([A-Za-z0-9_]+)\s*,\s*(\d+)\s*,\s*(\d+)", statement, re.I)
        if match:
            skill, level, rate = match.groups()
            effects.append(f"when hit by a direct attack, has a {int(rate) / 10:g}% chance to cast {_skill_display_name(skill)} Lv {level} on the attacker")
            continue
        match = re.fullmatch(r"bonus4\s+bAutoSpellWhenHit\s*,\s*([A-Za-z0-9_]+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*0", statement, re.I)
        if match:
            skill, level, rate = match.groups()
            effects.append(f"when hit by a direct attack, has a {int(rate) / 10:g}% chance to cast {_skill_display_name(skill)} Lv {level} on the attacker")
            continue
        match = re.fullmatch(r"bonus4\s+bAutoSpellOnSkill\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)\s*,\s*(\d+)\s*,\s*(\d+)", statement, re.I)
        if match:
            trigger_skill, skill, level, rate = match.groups()
            effects.append(f"when using {_skill_display_name(trigger_skill)}, has a {int(rate) / 10:g}% chance to cast {_skill_display_name(skill)} Lv {level}")
            continue
        match = re.fullmatch(r"bonus2\s+bAddEff\s*,\s*(Eff_[A-Za-z0-9_]+)\s*,\s*(\d+)", statement, re.I)
        if match:
            effect, rate = match.groups()
            effects.append(f"on attack, has a {int(rate) / 100:g}% chance to inflict {_effect_display_name(effect)}")
            continue
        match = re.fullmatch(r"bonus2\s+bAddEff2\s*,\s*(Eff_[A-Za-z0-9_]+)\s*,\s*(\d+)", statement, re.I)
        if match:
            effect, rate = match.groups()
            effects.append(f"on attack, has a {int(rate) / 100:g}% chance to inflict {_effect_display_name(effect)} on the wearer")
            continue
        match = re.fullmatch(r"bonus2\s+bAddEffWhenHit\s*,\s*(Eff_[A-Za-z0-9_]+)\s*,\s*(\d+)", statement, re.I)
        if match:
            effect, rate = match.groups()
            effects.append(f"when hit by physical damage, has a {int(rate) / 100:g}% chance to inflict {_effect_display_name(effect)} on the attacker")
            continue
        match = re.fullmatch(r"bonus2\s+bResEff\s*,\s*(Eff_[A-Za-z0-9_]+)\s*,\s*(-?\d+)", statement, re.I)
        if match:
            effect, rate = match.groups()
            value = int(rate) / 100
            direction = "increases" if value >= 0 else "reduces"
            effects.append(f"{direction} tolerance to {_effect_display_name(effect)} by {abs(value):g}%")
            continue
        # Match itemheal with literal values or rand(X,Y) expressions.
        # The simple [^,]+ pattern doesn't work because rand() contains commas.
        match = re.fullmatch(
            r"itemheal\s+(rand\(\s*-?\d+\s*,\s*-?\d+\)|-?\d+)\s*,\s*(rand\(\s*-?\d+\s*,\s*-?\d+\)|-?\d+)",
            statement,
            re.I,
        )
        if match:
            hp, sp = (_number(value) for value in match.groups())
            if hp is None or sp is None:
                return None
            if hp != "0":
                effects.append(f"Restores {hp} HP")
            if sp != "0":
                effects.append(f"restores {sp} SP")
            continue
        match = re.fullmatch(r"percentheal\s+(-?\d+)\s*,\s*(-?\d+)", statement, re.I) or re.fullmatch(
            r"percentheal\(\s*(-?\d+)\s*,\s*(-?\d+)\s*\)", statement, re.I
        )
        if match:
            hp, sp = map(int, match.groups())
            if hp: effects.append(f"restores {hp}% HP" if hp > 0 else f"reduces HP by {abs(hp)}%")
            if sp: effects.append(f"restores {sp}% SP" if sp > 0 else f"reduces SP by {abs(sp)}%")
            continue
        match = re.fullmatch(r"sc_end\s+(SC_[A-Z0-9_]+)", statement, re.I)
        if match and match.group(1).upper() in cures:
            effects.append(f"Cures {cures[match.group(1).upper()]}")
            continue
        match = re.fullmatch(
            r"sc_start\s+(SC_[A-Z0-9_]+)\s*,\s*(\d+)\s*,\s*(-?\d+)(?:\s*,\s*(\d+))?(?:\s*,\s*(SCFLAG_[A-Z0-9_]+))?",
            statement,
            re.I,
        ) or re.fullmatch(
            r"sc_start\(\s*(SC_[A-Z0-9_]+)\s*,\s*(\d+)\s*,\s*(-?\d+)(?:\s*,\s*(\d+))?(?:\s*,\s*(SCFLAG_[A-Z0-9_]+))?\s*\)",
            statement,
            re.I,
        )
        if match:
            status, duration_ms, value1, raw_rate, flag = match.groups()
            status = status.upper()
            # script_commands.txt names the constant and the tick unit, not every status title.
            label = status_names.get(status) or f"status {status}"
            duration_ms = int(duration_ms)
            duration = f"{duration_ms / 1000:g} seconds" if duration_ms % 1000 else f"{duration_ms // 1000} seconds"
            rate_note = f" at {int(raw_rate) / 100:g}% chance" if raw_rate is not None else " (no explicit chance limit)"
            value_note = f", value {value1}" if int(value1) != 0 else ""
            flag_note = ""
            if flag and flag.upper() != "SCFLAG_NONE":
                flag_note = f", flag {flag.upper()}"
            effects.append(f"Applies {label} for {duration}{value_note}{rate_note}{flag_note}")
            continue
        match = re.fullmatch(r"getitem\s+(-?\d+)\s*,\s*(\d+)", statement, re.I)
        if match:
            item_id, amount = int(match.group(1)), int(match.group(2))
            effects.append(f"grants {amount} {_item_display_name(item_id)}")
            continue
        match = re.fullmatch(
            r"getitem(?:\(\s*|\s+)([A-Za-z_][A-Za-z0-9_]*)\s*,\s*(\d+)\s*\)?",
            statement,
            re.I,
        )
        if match:
            token, amount = match.group(1), int(match.group(2))
            label = _item_names_by_aegis().get(token)
            if label is None:
                return None
            effects.append(f"grants {amount} {label}")
            continue
        match = re.fullmatch(r"rentitem\s+(-?\d+)\s*,\s*(\d+)", statement, re.I)
        if match:
            item_id, seconds = int(match.group(1)), int(match.group(2))
            effects.append(f"rents {_item_display_name(item_id)} for {seconds} seconds")
            continue
        match = re.fullmatch(r"getrandgroupitem\s+(-?\d+)\s*,\s*(\d+)", statement, re.I)
        if match:
            group_id, amount = int(match.group(1)), int(match.group(2))
            effects.append(f"grants {amount} random item from {_item_display_name(group_id)}")
            continue
        match = re.fullmatch(r"packageitem(?:\(\s*\))?", statement, re.I)
        if match:
            effects.append("grants this item's package contents")
            continue
        match = re.fullmatch(r"skill\s+([A-Za-z0-9_]+)\s*,\s*(\d+)", statement, re.I)
        if match:
            skill, level = match.groups()
            effects.append(f"grants {_skill_display_name(skill)} at level {level}")
            continue
        match = re.fullmatch(
            r"specialeffect\(\s*(EF_[A-Z0-9_]+)\s*,\s*AREA\s*,\s*playerattached\(\)\s*\)",
            statement,
            re.I,
        )
        if match:
            # script_commands.txt: the number is the effect, AREA shows it to everyone,
            # and a unit id displays it on that unit. The effect list is not in this file.
            effects.append(
                f"displays special effect {match.group(1).upper()} to everyone on the unit from playerattached()"
            )
            continue
        match = re.fullmatch(r"pet\s+([A-Za-z_][A-Za-z0-9_]*)", statement, re.I) or re.fullmatch(
            r"pet\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)", statement, re.I
        )
        if match:
            # script_commands.txt: pet id numbers live in pet_db.conf, which is not this excerpt.
            effects.append(f"makes the pet catching cursor appear for pet ID {match.group(1).upper()}")
            continue
        return None
    return "; ".join(effects) if effects else None


@lru_cache(maxsize=1)
def _skill_display_names() -> dict[str, str]:
    rows = parse_skill_db(SKILL_DB_SOURCE.read_text(encoding="utf-8", errors="replace"))
    return {
        str(row["Name"]): str(row.get("Description") or row["Name"])
        for row in rows
        if isinstance(row.get("Name"), str)
    }


@lru_cache(maxsize=1)
def _item_names_by_id() -> dict[int, str]:
    names: dict[int, str] = {}
    pattern = re.compile(r"Id:\s*(\d+)(?:(?!\n\s*Id:)[\s\S]){0,800}?\n\s*Name:\s*\"([^\"]*)\"")
    for path in ITEM_SOURCES:
        if not path.is_file():
            continue
        for match in pattern.finditer(path.read_text(encoding="utf-8", errors="replace")):
            names[int(match.group(1))] = match.group(2)
    return names


def _item_display_name(item_id: int) -> str:
    return _item_names_by_id().get(item_id, f"item {item_id}")


@lru_cache(maxsize=1)
def _item_names_by_aegis() -> dict[str, str]:
    """Map an item constant to its database Name. script_commands.txt calls that constant the item name."""
    names: dict[str, str] = {}
    pattern = re.compile(
        r'AegisName:\s*"([^"]+)"(?:(?!\n\s*Id:)[\s\S]){0,400}?\n\s*Name:\s*"([^"]*)"'
    )
    for path in ITEM_SOURCES:
        if not path.is_file():
            continue
        for match in pattern.finditer(path.read_text(encoding="utf-8", errors="replace")):
            names[match.group(1)] = match.group(2) or match.group(1)
    return names


def _skill_display_name(constant: str) -> str:
    return _skill_display_names().get(constant, constant.replace("_", " ").title())


# itemskill() flag bits from Hercules enum itemskill_flag (src/map/script.h).
_ITEMSKILL_FLAG_NOTES = (
    ("ISF_CHECKCONDITIONS", "after checking the skill's conditions and paying its costs"),
    ("ISF_INSTANTCAST", "instantly"),
    ("ISF_CASTONSELF", "on yourself, without a target cursor"),
)


def _itemskill_phrase(skill: str, level: str, flags: str | None) -> str | None:
    phrase = f"grants access to {_skill_display_name(skill)} at level {level}"
    if not flags:
        return phrase
    present = {part.strip().upper() for part in flags.split("|") if part.strip()}
    known = {name for name, _note in _ITEMSKILL_FLAG_NOTES}
    if not present or not present <= known:
        return None
    notes = [note for name, note in _ITEMSKILL_FLAG_NOTES if name in present]
    return f"{phrase}, {', '.join(notes)}"


def _effect_display_name(constant: str) -> str:
    known = {
        "Eff_Stun": "Stun", "Eff_Sleep": "Sleep", "Eff_Poison": "Poison", "Eff_Curse": "Curse",
        "Eff_Stone": "Stone Curse", "Eff_Freeze": "Freeze", "Eff_Blind": "Blindness",
        "Eff_Silence": "Silence", "Eff_Confusion": "Confusion", "Eff_Bleeding": "Bleeding",
        "Eff_Fear": "Fear", "Eff_Cold": "Cold", "Eff_Burning": "Burning", "Eff_Frostbite": "Frostbite",
    }
    return known.get(constant, constant.removeprefix("Eff_").replace("_", " ").title())


def _element_display_name(constant: str) -> str:
    return {
        "Ele_Neutral": "Neutral", "Ele_Water": "Water", "Ele_Earth": "Earth", "Ele_Fire": "Fire",
        "Ele_Wind": "Wind", "Ele_Poison": "Poison", "Ele_Holy": "Holy", "Ele_Dark": "Shadow",
        "Ele_Ghost": "Ghost", "Ele_Undead": "Undead",
    }.get(constant, constant.removeprefix("Ele_").replace("_", " ").title())


def describe_item_condition(condition: str) -> str | None:
    """Describe only literal equip conditions that have clear player meaning."""
    compact = re.sub(r"\s+", "", condition)
    comparison = r"(>=|>|==)"
    match = re.fullmatch(rf"BaseLevel{comparison}(\d+)", compact, re.I)
    if match:
        operator, value = match.groups()
        phrase = {">=": "at least", ">": "above", "==": "exactly"}[operator]
        return f"At Base Level {phrase} {value}"
    match = re.fullmatch(rf"JobLevel{comparison}(\d+)", compact, re.I)
    if match:
        operator, value = match.groups()
        phrase = {">=": "at least", ">": "above", "==": "exactly"}[operator]
        return f"At Job Level {phrase} {value}"
    match = re.fullmatch(rf"getrefine\(\){comparison}(\d+)", compact, re.I)
    if match:
        operator, value = match.groups()
        phrase = {">=": "at least", ">": "above", "==": "exactly"}[operator]
        return f"At refine level {phrase} +{value}"
    match = re.fullmatch(r"BaseJob==Job_([A-Za-z0-9_]+)", compact, re.I)
    if match:
        job = match.group(1).replace("_", " ")
        return f"For the {job} job"
    match = re.fullmatch(r"BaseClass==Job_([A-Za-z0-9_]+)", compact, re.I)
    if match:
        job_class = match.group(1).replace("_", " ")
        return f"For the {job_class} class"
    match = re.fullmatch(rf"readparam\(b(Str|Agi|Vit|Int|Dex|Luk)\){comparison}(\d+)", compact, re.I)
    if match:
        stat, operator, value = match.groups()
        phrase = {">=": "at least", ">": "above", "==": "exactly"}[operator]
        return f"With {stat.upper()} {phrase} {value}"
    return None


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


def read_item_groups() -> list[dict[str, Any]]:
    """Parse item-group memberships without treating repeat counts as drop rates."""
    text = ITEM_GROUP_SOURCE.read_text(encoding="utf-8", errors="replace")
    # Preserve quoted item names while removing source comments.
    cleaned = _strip_comments(text)
    groups: list[dict[str, Any]] = []
    position = 0
    while header := ITEM_GROUP_HEADER.search(cleaned, position):
        name = header.group(1)
        depth = 1
        cursor = header.end()
        in_string = False
        escaped = False
        while cursor < len(cleaned) and depth:
            char = cleaned[cursor]
            if in_string:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
            elif char == '"':
                in_string = True
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
            cursor += 1
        if depth:
            raise ValueError(f"unterminated item group {name!r} in {ITEM_GROUP_SOURCE}")
        body = cleaned[header.end():cursor - 1]
        entries: list[tuple[str, int]] = []
        offset = 0
        while offset < len(body):
            if not body[offset:].strip(" \t\r\n,"):
                break
            entry = ITEM_GROUP_ENTRY.match(body, offset)
            if not entry:
                raise ValueError(f"could not parse item group {name!r} near {body[offset:offset + 80]!r}")
            item_name, repeat, singleton = entry.groups()
            count = int(repeat) if repeat else 1
            if count <= 0:
                raise ValueError(f"non-positive selection weight in item group {name!r}: {count}")
            entries.append((item_name or singleton, count))
            offset = entry.end()
        groups.append({"name": name, "entries": entries})
        position = cursor
    return groups


def read_literal_npc_shops(
    item_by_aegis: dict[str, int], item_by_id: dict[int, tuple[dict[str, Any], Path]]
) -> tuple[dict[int, list[dict[str, Any]]], dict[str, int]]:
    """Index literal stock declared by loaded map shops and trader scripts."""
    locations: dict[int, list[dict[str, Any]]] = defaultdict(list)
    known_maps = map_names(HERCULES / "db/map_index.txt")
    script_files = loaded_script_files(HERCULES, NPC_MANIFEST)
    scanned_npcs = 0
    literal_offers = 0
    unresolved = 0
    dynamic_stock_skipped = 0
    loaded_sellitem_sources: set[str] = set()
    trader_sellitem_sources: set[str] = set()

    def add_offer(item_key: str, shop: dict[str, Any], source: str) -> None:
        nonlocal literal_offers, unresolved
        item_key = item_key.strip().strip('"')
        if item_key.startswith("ID") and item_key[2:].isdigit():
            item_id = int(item_key[2:])
        elif item_key.isdigit():
            item_id = int(item_key)
        else:
            item_id = item_by_aegis.get(item_key, 0)
        if item_id not in item_by_id:
            unresolved += 1
            return
        locations[item_id].append({**shop, "source": source})
        literal_offers += 1

    for path in script_files:
        text = _strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        path_record = path.relative_to(HERCULES).as_posix()
        loaded_sellitem_sources.update(
            f"{path_record}:{text.count(chr(10), 0, match.start()) + 1}" for match in SHOP_COMMAND.finditer(text)
        )
        lines = text.splitlines(keepends=True)
        offsets: list[int] = []
        offset = 0
        for line in lines:
            offsets.append(offset)
            offset += len(line)
        for line_number, line in enumerate(text.splitlines(), start=1):
            fields = line.split("\t")
            if len(fields) < 4:
                continue
            location_match = NPC_LOCATION.match(fields[0])
            shop_type = fields[1].strip()
            if not location_match or shop_type not in ("shop", "cashshop", "trader"):
                continue
            map_name, raw_x, raw_y, _direction = location_match.groups()
            if map_name not in known_maps:
                continue
            scanned_npcs += 1
            npc_name = fields[2].strip()
            tail = "\t".join(fields[3:]).strip()
            common = {
                "npc_name": npc_name,
                "map": map_name,
                "x": int(raw_x),
                "y": int(raw_y),
                "currency": "Kafra points" if shop_type == "cashshop" else "Zeny",
                "shop_type": shop_type,
            }
            if shop_type in ("shop", "cashshop"):
                # Static shop declarations encode item IDs and prices in the header.
                header = tail.splitlines()[0]
                _sprite, separator, stock = header.partition(",")
                if not separator:
                    continue
                for offer in stock.split(","):
                    item_key, colon, raw_price = offer.strip().partition(":")
                    if not colon or not raw_price.strip().lstrip("-").isdigit():
                        continue
                    price = int(raw_price)
                    add_offer(item_key, {
                        **common,
                        "currency": "Kafra points" if shop_type == "cashshop" else "Zeny",
                        "price": None if price == -1 else price,
                        "uses_item_db_price": price == -1,
                    }, f"{path_record}:{line_number}")
                continue

            # A trader's literal sellitem commands add its configured stock.
            opening = tail.find("{")
            if opening < 0:
                continue
            absolute_open = offsets[line_number - 1] + text.splitlines()[line_number - 1].find("{")
            depth = 1
            cursor = absolute_open + 1
            in_string = False
            escaped = False
            while cursor < len(text) and depth:
                char = text[cursor]
                if in_string:
                    if escaped:
                        escaped = False
                    elif char == "\\":
                        escaped = True
                    elif char == '"':
                        in_string = False
                elif char == '"':
                    in_string = True
                elif char == "{":
                    depth += 1
                elif char == "}":
                    depth -= 1
                cursor += 1
            if depth:
                raise ValueError(f"unterminated trader script block: {path_record}:{line_number}")
            body = text[absolute_open + 1:cursor - 1]
            trader_type_match = re.search(r"\btradertype\s*\(\s*(NST_[A-Z_]+)\s*\)\s*;", body)
            trader_type = trader_type_match.group(1) if trader_type_match else "NST_ZENY"
            if trader_type in ("NST_BARTER", "NST_EXPANDED_BARTER", "NST_CUSTOM"):
                skipped_count = len(list(SHOP_COMMAND.finditer(body)))
                dynamic_stock_skipped += skipped_count
                continue
            currency = "Cash points" if trader_type == "NST_CASH" else "Zeny"
            for match in SHOP_COMMAND.finditer(body):
                command_offset = absolute_open + 1 + match.start()
                command_source = f"{path_record}:{text.count(chr(10), 0, command_offset) + 1}"
                trader_sellitem_sources.add(command_source)
                args = [value.strip() for value in match.group(1).split(",")]
                item_key = args[0]
                if not re.fullmatch(r"(?:ID\d+|\d+|[A-Za-z0-9_]+)", item_key):
                    dynamic_stock_skipped += 1
                    continue
                raw_price = args[1] if len(args) > 1 else "-1"
                if not re.fullmatch(r"-?\d+", raw_price):
                    dynamic_stock_skipped += 1
                    continue
                if len(args) > 3 or (len(args) > 2 and not args[2].isdigit()):
                    dynamic_stock_skipped += 1
                    continue
                price = int(raw_price)
                add_offer(item_key, {
                    **common,
                    "shop_type": "cash trader" if trader_type == "NST_CASH" else shop_type,
                    "currency": currency,
                    "price": None if price == -1 else price,
                    "uses_item_db_price": price == -1,
                    "quantity": int(args[2]) if len(args) > 2 and args[2].isdigit() else None,
                }, command_source)

    for offers in locations.values():
        offers.sort(key=lambda offer: (offer["map"], offer["npc_name"], offer["source"]))
    return locations, {
        "loaded_script_files": len(script_files),
        "literal_shop_npcs_scanned": scanned_npcs,
        "literal_shop_offers": literal_offers,
        "sellitem_commands_in_loaded_scripts": len(loaded_sellitem_sources),
        "sellitem_commands_in_indexed_traders": len(trader_sellitem_sources),
        "sellitem_commands_outside_indexed_traders": len(loaded_sellitem_sources - trader_sellitem_sources),
        "sellitem_commands_outside_indexed_trader_sources": sorted(loaded_sellitem_sources - trader_sellitem_sources),
        "unresolved_literal_item_references": unresolved,
        "dynamic_or_nonliteral_stock_entries_skipped": dynamic_stock_skipped,
        "indexed_shop_types": ["shop", "cashshop", "trader", "cash trader"],
        "barter_custom_and_runtime_added_stock": "not_exported",
    }


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
    item_by_id: dict[int, tuple[dict[str, Any], Path]] = merged
    for item_id, (row, path) in merged.items():
        aegis_name = row.get("AegisName")
        if not isinstance(aegis_name, str) or not aegis_name:
            raise ValueError(f"item {item_id} in {path} has no AegisName")
        if aegis_name in item_by_aegis:
            raise ValueError(f"duplicate AegisName {aegis_name!r} across merged item sources")
        item_by_aegis[aegis_name] = item_id

    combo_text = COMBO_SOURCE.read_text(encoding="utf-8", errors="replace")
    combo_raw_scripts: list[str] = []
    def preserve_combo_script(match: re.Match[str]) -> str:
        combo_raw_scripts.append(match.group(0).split('<"', 1)[1][:-2])
        return f'{match.group(1)}: "__SCRIPT_PRESENT__"'
    scrubbed_combos, combo_scripts = SCRIPT_FIELD.subn(preserve_combo_script, combo_text)
    combo_starts = len(re.findall(r"\b(?:" + "|".join(SCRIPT_FIELDS) + r"):" + r"\s*<\"", combo_text))
    if combo_starts != combo_scripts:
        raise ValueError(f"could not safely delimit every combo script: {combo_starts} starts, {combo_scripts} complete values")
    combo_rows = parse_skill_db(scrubbed_combos)
    combos_by_item: dict[int, list[dict[str, Any]]] = defaultdict(list)
    belongs_to_combos_by_item: dict[int, list[str]] = defaultdict(list)
    # The database header includes a commented example entry with a Script
    # field; it is not returned by the config parser.
    if len(combo_raw_scripts) == len(combo_rows) + 1:
        combo_raw_scripts = combo_raw_scripts[1:]
    if len(combo_raw_scripts) != len(combo_rows):
        raise ValueError("could not associate combo effects with combo records")
    for index, combo in enumerate(combo_rows, start=1):
        names = combo.get("Items")
        if not isinstance(names, list) or len(names) < 2 or any(not isinstance(name, str) for name in names):
            raise ValueError(f"invalid Items list in {COMBO_SOURCE}, combo_db[{index}]")
        unknown_names = [name for name in names if name not in item_by_aegis]
        if unknown_names:
            raise ValueError(f"unknown combo item names in combo_db[{index}]: {unknown_names}")
        members = [
            {"id": item_id, "aegis_name": name, "name": item_by_id[item_id][0].get("Name")}
            for name in names
            for item_id in [item_by_aegis[name]]
        ]
        combo_link = {
            "members": members,
            "effect_status": "translated" if translate_simple_effect(combo_raw_scripts[index - 1]) else "scripted_not_translated",
            **({"effect_summary": translate_simple_effect(combo_raw_scripts[index - 1])}
               if translate_simple_effect(combo_raw_scripts[index - 1]) else {}),
            "source": {"path": COMBO_SOURCE.relative_to(HERCULES).as_posix(), "record": f"combo_db[{index}]"},
        }
        combo_name = "_".join(sorted(names))
        for member in members:
            combos_by_item[member["id"]].append(combo_link)
            belongs_to_combos_by_item[member["id"]].append(combo_name)

    group_contents_by_item: dict[int, list[dict[str, Any]]] = defaultdict(list)
    contained_in_by_item: dict[int, list[dict[str, Any]]] = defaultdict(list)
    group_path = ITEM_GROUP_SOURCE.relative_to(HERCULES).as_posix()
    for group in read_item_groups():
        container_id = item_by_aegis.get(group["name"])
        if container_id is None:
            raise ValueError(f"item group container {group['name']!r} is missing from merged item DB")
        total_weight = sum(weight for _, weight in group["entries"])
        if total_weight <= 0:
            continue
        container = {
            "id": container_id,
            "aegis_name": group["name"],
            "name": item_by_id[container_id][0].get("Name"),
        }
        source = {"path": group_path, "record": group["name"]}
        group_weights: dict[int, int] = {}
        for member_name, weight in group["entries"]:
            if member_name.startswith("ID") and member_name[2:].isdigit():
                member_id = int(member_name[2:])
            else:
                member_id = item_by_aegis.get(member_name, 0)
            if member_id not in item_by_id:
                raise ValueError(f"item group {group['name']!r} references unknown item {member_name!r}")
            group_weights[member_id] = group_weights.get(member_id, 0) + weight
        for member_id, weight in group_weights.items():
            item = item_by_id[member_id][0]
            group_contents_by_item[container_id].append({
                "item": {"id": member_id, "aegis_name": item["AegisName"], "name": item.get("Name")},
                "selection_weight": weight,
                "total_weight": total_weight,
                "selection_chance_percent": round(weight * 10000 / total_weight) / 100,
                "source": source,
            })
            contained_in_by_item[member_id].append({
                "container": container,
                "selection_weight": weight,
                "total_weight": total_weight,
                "selection_chance_percent": round(weight * 10000 / total_weight) / 100,
                "source": source,
            })

    shops_by_item, shop_coverage = read_literal_npc_shops(item_by_aegis, item_by_id)

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
            **({"combos": combos_by_item[item_id]} if item_id in combos_by_item else {}),
            **({"belongs_to_combos": sorted(belongs_to_combos_by_item[item_id])} if item_id in belongs_to_combos_by_item else {}),
            **({"group_contents": group_contents_by_item[item_id]} if item_id in group_contents_by_item else {}),
            **({"contained_in_groups": contained_in_by_item[item_id]} if item_id in contained_in_by_item else {}),
            **({"shops": shops_by_item[item_id]} if item_id in shops_by_item else {}),
            "effect_status": (
                "translated" if row.get("__effect_summary") else
                "scripted_not_translated" if any(row.get(field) == "__SCRIPT_PRESENT__" for field in SCRIPT_FIELDS) else
                "no_script_field"
            ),
            **({"effect_summary": row["__effect_summary"]} if row.get("__effect_summary") else {}),
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
        "sources": [
            path.relative_to(HERCULES).as_posix()
            for path in (*ITEM_SOURCES, MOB_SOURCE, COMBO_SOURCE, ITEM_GROUP_SOURCE)
        ] + ["src/map/itemdb.c:itemdb_searchrandomid", "npc/re/scripts_main.conf (loaded shop source manifest)"],
        "shop_coverage": shop_coverage,
    }
    return ({**metadata, "entries": item_entries}, {**metadata, "entries": card_entries})


def render(payload: dict[str, Any]) -> str:
    return json.dumps(payload, indent=2, ensure_ascii=False) + "\n"


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
