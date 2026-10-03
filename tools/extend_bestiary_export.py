#!/usr/bin/env python3
"""Extend docs/bestiary.json with Element/Race/Size/Mode/Skills.

The original exporter that produced bestiary.json was never committed, so
this is a fresh, narrowly-scoped script: it MERGES into the existing file
rather than regenerating it, preserving every already-shipped field
(PhysDPS/MagicDPS/DropsCount/HasMvpDrops/MvpExp etc. -- consumed by
src/dm/loot.rs and the shipped Bestiary Journal window) exactly as-is, and
only adds the fields the tiered lore-check reveal needs
(docs/specs/proficiency-checks.md, docs/specs/bestiary-journal.md):

- Element / Race / Size, from db/re/mob_db.conf (Identity tier)
- Mode flags (Aggressive/Looter/Assist/Boss/...), from the same file
- Skills (id/level/rate/delay), joined from db/re/mob_skill_db.conf by
  SpriteName (Combat tier's "notable skills" line)

Usage: python3 tools/extend_bestiary_export.py
Re-run after mob_db.conf / mob_skill_db.conf changes, then rebuild
(include_str! embeds bestiary.json at compile time).
"""

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
HERCULES = REPO.parent / "Hercules"
MOB_DB = HERCULES / "db" / "re" / "mob_db.conf"
MOB_SKILL_DB = HERCULES / "db" / "re" / "mob_skill_db.conf"
BESTIARY = REPO / "docs" / "bestiary.json"

MODE_FLAGS = (
    "Aggressive", "Angry", "Assist", "Boss", "CanAttack", "CanMove",
    "CastSensorChase", "CastSensorIdle", "ChangeChase", "ChangeTargetChase",
    "ChangeTargetMelee", "Detector", "Looter", "NoKnockback", "Plant",
)


def parse_mob_db():
    """Id -> {Size, Race, Element: {type, level}, Mode: [flags]}."""
    text = MOB_DB.read_text(encoding="utf-8", errors="replace")
    blocks = re.findall(r"^\{\n(.*?)\n\},", text, re.M | re.S)
    out = {}
    for b in blocks:
        m = re.search(r"^\tId:\s*(\d+)\s*$", b, re.M)
        if not m:
            continue
        mid = int(m.group(1))
        if mid in out:
            raise ValueError(f"duplicate monster ID {mid} in {MOB_DB}")
        entry = {}
        sm = re.search(r'Size:\s*"Size_(\w+)"', b)
        if sm:
            entry["Size"] = sm.group(1)
        rm = re.search(r'Race:\s*"RC_(\w+)"', b)
        if rm:
            entry["Race"] = rm.group(1)
        em = re.search(r'Element:\s*\(\s*"Ele_(\w+)"\s*,\s*(\d+)\s*\)', b)
        if em:
            # BestiaryMonster.element is Option<String> (src/dm/data.rs) --
            # "<Type> <Level>" matches how RO players/wikis conventionally
            # write it (e.g. "Water 1"), keeps the level without a schema
            # change, and stays a plain string so deserialization can't break.
            entry["Element"] = f"{em.group(1)} {em.group(2)}"
        modeblock = re.search(r"Mode:\s*\{([^}]*)\}", b, re.S)
        if modeblock:
            flags = [f for f in MODE_FLAGS
                     if re.search(rf"\b{f}:\s*true\b", modeblock.group(1))]
            if flags:
                entry["Mode"] = flags
        out[mid] = entry
    return out


def parse_mob_skill_db(include_triggers=False, path=MOB_SKILL_DB):
    """SpriteName -> [{Skill, Level, Rate, Delay}].

    Structure is `mob_skill_db:( { SPRITE: { SKILL: { fields } } } )` --
    one wrapping brace puts sprites at depth 1, skills at depth 2, and
    skill fields at depth 3. The header doc-comment (a /* */ block) uses
    "{"/"}" in its illustrative example text, so block comments must be
    stripped before depth-counting or they desync the whole parse.
    """
    text = path.read_text(encoding="utf-8", errors="replace")
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    lines = text.splitlines()
    out = {}
    depth = 0
    sprite = None
    skill = None
    cur = {}
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("//") or not stripped:
            continue
        opens = stripped.count("{")
        closes = stripped.count("}")

        if depth == 1 and stripped.endswith("{") and ":" in stripped:
            sprite = stripped.split(":", 1)[0].strip()
            depth += opens - closes
            continue
        elif depth == 2 and stripped.endswith("{") and ":" in stripped:
            skill = stripped.split(":", 1)[0].strip()
            cur = {}
            depth += opens - closes
            continue
        elif depth == 3:
            fm = re.match(r"(\w+):\s*(.+)", stripped)
            if fm:
                key, val = fm.group(1), fm.group(2).strip('"')
                if key == "ClearSkills":
                    cur[key] = val.lower() == "true"
                elif key in ("SkillLevel", "Rate", "Delay", "CastTime", "ConditionData", "val0"):
                    cur[key] = int(val) if val.lstrip("-").isdigit() else val
                elif include_triggers and key in ("SkillState", "SkillTarget", "CastCondition", "Cancelable"):
                    clean = val.strip('"')
                    cur[key] = clean.lower() == "true" if key == "Cancelable" and clean.lower() in ("true", "false") else clean
            depth += opens - closes
            if depth == 2:  # skill block closed
                if cur.get("ClearSkills"):
                    # `mob_skill_db_libconfig_sub_skill` drops every skill
                    # loaded so far for this monster and adds nothing itself.
                    out.setdefault(sprite, []).append({"ClearSkills": True})
                    continue
                row = {
                    "Skill": skill,
                    "Level": cur.get("SkillLevel", 1),
                    "Rate": cur.get("Rate", 1 if include_triggers else 0),
                    "Delay": cur.get("Delay", 0),
                }
                if include_triggers:
                    row.update({
                        "SkillState": cur.get("SkillState", "MSS_ANY"),
                        "SkillTarget": cur.get("SkillTarget", "MST_TARGET"),
                        "CastCondition": cur.get("CastCondition", "MSC_ALWAYS"),
                        "ConditionData": cur.get("ConditionData", 0),
                        "Value0": cur.get("val0", 0),
                        "CastTime": cur.get("CastTime", 0),
                        "Cancelable": cur.get("Cancelable", False),
                    })
                out.setdefault(sprite, []).append(row)
            continue
        else:
            depth += opens - closes
            if depth == 1:
                sprite = None
    return out


def apply_skill_layers(layers):
    """Effective skills per sprite after loading ``layers`` in server order.

    Each layer is a ``parse_mob_skill_db`` result. Rows append to a monster's
    list; a ``ClearSkills`` marker empties it first (Hercules
    ``mob_skill_db_libconfig_sub_skill``).
    """
    out = {}
    for layer in layers:
        for sprite, rows in layer.items():
            for row in rows:
                if row.get("ClearSkills"):
                    out[sprite] = []
                else:
                    out.setdefault(sprite, []).append(row)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail without writing if bestiary.json would change")
    args = parser.parse_args()

    mob_fields = parse_mob_db()
    skills = apply_skill_layers([parse_mob_skill_db()])
    print(f"mob_db.conf: {len(mob_fields)} entries parsed")
    print(f"mob_skill_db.conf: {len(skills)} sprites with skills")

    bestiary = json.loads(BESTIARY.read_text(encoding="utf-8"))
    ids = [monster.get("Id") for monster in bestiary]
    if None in ids:
        raise SystemExit(f"{BESTIARY} contains an entry without Id")
    if len(ids) != len(set(ids)):
        duplicates = sorted({monster_id for monster_id in ids if ids.count(monster_id) > 1})
        raise SystemExit(f"{BESTIARY} contains duplicate monster IDs: {duplicates}")
    before = {
        "element": sum(1 for m in bestiary if m.get("Element")),
        "skills": sum(1 for m in bestiary if m.get("Skills")),
        "mode": sum(1 for m in bestiary if m.get("Mode")),
    }

    for m in bestiary:
        extra = mob_fields.get(m["Id"], {})
        for key in ("Size", "Race", "Element", "Mode"):
            if key in extra:
                m[key] = extra[key]
        sprite_skills = skills.get(m.get("SpriteName", ""))
        if sprite_skills and not m.get("Skills"):
            m["Skills"] = sprite_skills

    after = {
        "element": sum(1 for m in bestiary if m.get("Element")),
        "skills": sum(1 for m in bestiary if m.get("Skills")),
        "mode": sum(1 for m in bestiary if m.get("Mode")),
    }
    print(f"Element coverage: {before['element']} -> {after['element']} / {len(bestiary)}")
    print(f"Skills coverage:  {before['skills']} -> {after['skills']} / {len(bestiary)}")
    print(f"Mode coverage:    {before['mode']} -> {after['mode']} / {len(bestiary)}")

    rendered = json.dumps(bestiary, indent=1, ensure_ascii=False) + "\n"
    if args.check:
        current = BESTIARY.read_text(encoding="utf-8")
        if current != rendered:
            print(f"stale: {BESTIARY} — re-run {Path(__file__).name}", file=sys.stderr)
            raise SystemExit(1)
        print(f"up to date: {len(bestiary)} monsters")
        return

    BESTIARY.write_text(rendered, encoding="utf-8")
    print(f"wrote {BESTIARY}")


if __name__ == "__main__":
    main()
