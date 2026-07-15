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

import json
import re
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


def parse_mob_skill_db():
    """SpriteName -> [{Skill, Level, Rate, Delay}].

    Structure is `mob_skill_db:( { SPRITE: { SKILL: { fields } } } )` --
    one wrapping brace puts sprites at depth 1, skills at depth 2, and
    skill fields at depth 3. The header doc-comment (a /* */ block) uses
    "{"/"}" in its illustrative example text, so block comments must be
    stripped before depth-counting or they desync the whole parse.
    """
    text = MOB_SKILL_DB.read_text(encoding="utf-8", errors="replace")
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
                if key in ("SkillLevel", "Rate", "Delay", "CastTime"):
                    cur[key] = int(val) if val.lstrip("-").isdigit() else val
            depth += opens - closes
            if depth == 2:  # skill block closed
                out.setdefault(sprite, []).append({
                    "Skill": skill,
                    "Level": cur.get("SkillLevel", 1),
                    "Rate": cur.get("Rate", 0),
                    "Delay": cur.get("Delay", 0),
                })
            continue
        else:
            depth += opens - closes
            if depth == 1:
                sprite = None
    return out


def main():
    mob_fields = parse_mob_db()
    skills = parse_mob_skill_db()
    print(f"mob_db.conf: {len(mob_fields)} entries parsed")
    print(f"mob_skill_db.conf: {len(skills)} sprites with skills")

    bestiary = json.loads(BESTIARY.read_text())
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

    BESTIARY.write_text(json.dumps(bestiary, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {BESTIARY}")


if __name__ == "__main__":
    main()
