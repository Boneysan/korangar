#!/usr/bin/env python3
"""Mechanical checks for an encyclopedia unit draft.

These checks are stricter than the exporters on purpose. A small model can
write a plausible record that cites the right file but the wrong line, or an
item amount that the cited call does not use. Every check here returns plain
error strings; an empty list means the draft may go to the verifier.
"""
from __future__ import annotations

import re
from typing import Any

from encyclopedia_common import (
    DISPOSITION_RESULTS,
    EVIDENCE_STATES,
    REVIEWED_STATES,
    hercules_path,
    read_json,
    resolve_item,
    ROOT,
    source_lines,
)

ITEM_CALL = re.compile(r"\b(getitem|delitem)\s*\(?\s*([A-Za-z0-9_]+)\s*,\s*([^;),]+)", re.I)
QUEST_CALL = re.compile(r"\b(setquest|completequest|erasequest|changequest)\s*\(?\s*(\d{3,6})\b", re.I)
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def in_scope(path: str, line: int, scope: list[dict[str, Any]] | None) -> bool:
    if scope is None:
        return True
    return any(part["path"] == path and part["start"] <= line <= part["end"] for part in scope)


def check_sources(label: str, sources: Any, scope: list[dict[str, Any]] | None = None) -> list[str]:
    """Each source must be a real, loaded path; each literal must sit on one of its cited lines."""
    errors: list[str] = []
    if not isinstance(sources, list) or not sources:
        return [f"{label}: `sources` must be a non-empty list"]
    for index, source in enumerate(sources):
        where = f"{label} sources[{index}]"
        if not isinstance(source, dict) or not isinstance(source.get("path"), str):
            errors.append(f"{where}: needs `path`, `lines`, and `required_source_literals`")
            continue
        path = source["path"]
        if hercules_path(path) is None:
            errors.append(f"{where}: {path} is not a loaded script or an existing Hercules src/db/conf/doc file")
            continue
        lines = source.get("lines")
        literals = source.get("required_source_literals")
        if not isinstance(lines, list) or not lines or not all(isinstance(n, int) for n in lines):
            errors.append(f"{where}: `lines` must be a non-empty list of line numbers")
            continue
        if not isinstance(literals, list) or not literals or not all(isinstance(t, str) and t.strip() for t in literals):
            errors.append(f"{where}: `required_source_literals` must be a non-empty list of exact source text")
            continue
        text = source_lines(path)
        for number in lines:
            if not 1 <= number <= len(text):
                errors.append(f"{where}: {path}:{number} does not exist")
            elif not in_scope(path, number, scope):
                errors.append(f"{where}: {path}:{number} is outside this unit's packet; cite only lines you were given")
        quoted: set[int] = set()
        for literal in literals:
            span = literal.strip().count("\n") + 1
            starts = [n for n in lines if 1 <= n <= len(text) and literal.strip() in "\n".join(text[n - 1:n - 1 + span])]
            if not starts:
                errors.append(f"{where}: literal not found on any cited line: {literal!r}")
            for start in starts:
                quoted.update(range(start, start + span))
        for number in lines:
            if 1 <= number <= len(text) and number not in quoted:
                errors.append(f"{where}: {path}:{number} is cited but no literal quotes it; quote each cited line")
    return errors


def check_common(label: str, record: dict[str, Any], state_field: str, allowed: tuple[str, ...]) -> list[str]:
    errors = []
    if not isinstance(record.get("id"), str) or not re.fullmatch(r"[a-z0-9_]+", record.get("id", "")):
        errors.append(f"{label}: `id` must be lowercase letters, digits, and underscores")
    state = record.get(state_field)
    if state not in allowed:
        errors.append(f"{label}: `{state_field}` must be one of {', '.join(allowed)}; got {state!r}")
    conditions = record.get("conditions", [])
    if not isinstance(conditions, list) or not all(isinstance(c, str) and c.strip() for c in conditions):
        errors.append(f"{label}: `conditions` must be a list of sentences")
    if state in ("conditional", "configured_estimate") and not conditions:
        errors.append(f"{label}: a {state} record must list its conditions")
    return errors


def check_disposition(label: str, record: dict[str, Any], unit: dict[str, Any] | None) -> list[str]:
    errors = check_common(label, record, "result", DISPOSITION_RESULTS)
    if not isinstance(record.get("statement"), str) or len(record["statement"].strip()) < 12:
        errors.append(f"{label}: `statement` must say, in one sentence, what the source shows")
    subject = record.get("subject")
    if not isinstance(subject, dict) or not subject.get("kind") or not (subject.get("ids") or subject.get("name")):
        errors.append(f"{label}: `subject` needs `kind` and `ids` or `name`")
    if unit is not None and record.get("unit") != unit["id"]:
        errors.append(f"{label}: `unit` must be {unit['id']!r}")
    errors += check_sources(label, record.get("sources"), unit["scope"] if unit else None)
    return errors


# Per-kind review checks ----------------------------------------------------

def _literal_calls(sources: list[dict[str, Any]]) -> list[tuple[str, str, str]]:
    calls = []
    for source in sources:
        for literal in source.get("required_source_literals", []):
            for match in ITEM_CALL.finditer(literal):
                calls.append((match.group(1).lower(), match.group(2), match.group(3).strip()))
    return calls


def check_e1_review(label: str, review: dict[str, Any], unit: dict[str, Any]) -> list[str]:
    # The exchange exporter derives evidence_state from `conditions`; the draft must not set it.
    errors = check_common(label, {**review, "evidence_state": "verified"}, "evidence_state", ("verified",))
    if "evidence_state" in review:
        errors.append(f"{label}: remove `evidence_state`; the exporter sets it from `conditions`")
    if not isinstance(review.get("title"), str) or len(review["title"].strip()) < 8:
        errors.append(f"{label}: `title` must name the exchange")
    npc = review.get("npc", {})
    subject = unit["subject"]
    for field, expected in (("internal_name", subject["npc"]), ("map", subject["map"]), ("x", subject["x"]), ("y", subject["y"])):
        if npc.get(field) != expected:
            errors.append(f"{label}: npc.{field} must be {expected!r} (from the NPC header); got {npc.get(field)!r}")
    source = review.get("source")
    sources = [dict(source, required_source_literals=review.get("required_source_literals"))] if isinstance(source, dict) else None
    errors += check_sources(label, sources, unit["scope"])
    if errors:
        return errors
    calls = _literal_calls(sources)
    removed = {}
    granted = {}
    for command, token, amount in calls:
        item = resolve_item(token)
        if item is None:
            errors.append(f"{label}: {command} {token} does not resolve to an exported item")
            continue
        table = removed if command == "delitem" else granted
        table[item["id"]] = amount
    for index, entry in enumerate(review.get("inputs", [])):
        item_id, amount = entry.get("item_id"), entry.get("amount")
        if item_id not in removed:
            errors.append(f"{label}: inputs[{index}] item {item_id} has no quoted `delitem` literal")
        elif str(amount) != removed[item_id]:
            errors.append(f"{label}: inputs[{index}] amount {amount} but the quoted delitem uses {removed[item_id]}")
    claimed_inputs = {entry.get("item_id") for entry in review.get("inputs", [])}
    for item_id in removed:
        if item_id not in claimed_inputs:
            errors.append(f"{label}: quoted delitem of item {item_id} is not listed in `inputs`")
    outcomes = review.get("outcomes")
    if not isinstance(outcomes, list) or not outcomes:
        errors.append(f"{label}: `outcomes` must list what the player receives")
        return errors
    claimed_outputs = set()
    for index, outcome in enumerate(outcomes):
        for item_id in outcome.get("item_ids", []):
            claimed_outputs.add(item_id)
            if item_id not in granted:
                errors.append(f"{label}: outcomes[{index}] item {item_id} has no quoted `getitem` literal")
            elif len(outcome["item_ids"]) == 1 and str(outcome.get("amount")) != granted[item_id]:
                errors.append(f"{label}: outcomes[{index}] amount {outcome.get('amount')} but the quoted getitem uses {granted[item_id]}")
        if outcome.get("selection") not in ("single_option", "player_choice", "random", "all"):
            errors.append(f"{label}: outcomes[{index}].selection must be single_option, player_choice, random, or all")
    for item_id in granted:
        if item_id not in claimed_outputs:
            errors.append(f"{label}: quoted getitem of item {item_id} is not listed in `outcomes`")
    return errors


def check_e4_review(label: str, review: dict[str, Any], unit: dict[str, Any]) -> list[str]:
    errors = check_common(label, review, "evidence_state", REVIEWED_STATES)
    skill_ids = review.get("skill_ids")
    if not isinstance(skill_ids, list) or not skill_ids:
        errors.append(f"{label}: `skill_ids` must be a non-empty list")
    else:
        extra = sorted(set(skill_ids) - set(unit["subject"]["skill_ids"]))
        if extra:
            errors.append(f"{label}: skill_ids {extra} are not in this unit")
    for field in ("title", "formula", "worked_example", "review_method"):
        if not isinstance(review.get(field), str) or len(review[field].strip()) < 12:
            errors.append(f"{label}: `{field}` must be a full sentence")
    if not review.get("conditions"):
        errors.append(f"{label}: list at least one condition or caveat (the exporter requires it)")
    if isinstance(review.get("worked_example"), str) and not re.search(r"\d", review["worked_example"]):
        errors.append(f"{label}: `worked_example` must compute a number")
    errors += check_sources(label, review.get("sources"), unit["scope"])
    return errors


def check_e5_review(label: str, review: dict[str, Any], unit: dict[str, Any]) -> list[str]:
    errors = check_common(label, review, "evidence_state", REVIEWED_STATES)
    quest_ids = review.get("quest_ids")
    if not isinstance(quest_ids, list) or not quest_ids:
        errors.append(f"{label}: `quest_ids` must be a non-empty list")
    else:
        extra = sorted(set(quest_ids) - set(unit["subject"]["quest_ids"]))
        if extra:
            errors.append(f"{label}: quest_ids {extra} are not in this unit")
    for field in ("title", "review_method"):
        if not isinstance(review.get(field), str) or len(review[field].strip()) < 12:
            errors.append(f"{label}: `{field}` must be a full sentence")
    if not review.get("conditions"):
        errors.append(f"{label}: list each stage, prerequisite, and branch as a condition sentence")
    errors += check_sources(label, review.get("sources"), unit["scope"])
    return errors


def check_e6_review(label: str, review: dict[str, Any], unit: dict[str, Any]) -> list[str]:
    errors = check_common(label, review, "evidence_state", REVIEWED_STATES)
    if review.get("monster_id") != unit["subject"]["monster_id"]:
        errors.append(f"{label}: monster_id must be {unit['subject']['monster_id']}")
    if review.get("sprite_name") != unit["subject"]["sprite_name"]:
        errors.append(f"{label}: sprite_name must be {unit['subject']['sprite_name']!r}")
    for field in ("summons", "hp_threshold_behaviors"):
        if not isinstance(review.get(field), list):
            errors.append(f"{label}: `{field}` must be a list (empty when the boss has none)")
    errors += check_sources(label, review.get("sources"), unit["scope"])
    return errors


REVIEW_CHECKS = {
    "e1_exchange": check_e1_review,
    "e4_formula": check_e4_review,
    "e5_quest": check_e5_review,
    "e6_boss": check_e6_review,
}


# Coverage: nothing in scope may be silently skipped -------------------------

def cited_lines(draft: dict[str, Any]) -> set[tuple[str, int]]:
    cited = set()
    for record in draft.get("reviews", []) + draft.get("dispositions", []):
        sources = record.get("sources") or ([record["source"]] if isinstance(record.get("source"), dict) else [])
        for source in sources:
            for number in source.get("lines", []):
                cited.add((source.get("path"), number))
    return cited


def existing_records(unit: dict[str, Any]) -> list[dict[str, Any]]:
    """Accepted reviews and dispositions that already cite a line inside this unit's scope."""
    records = []
    files = [REVIEW_FILES[unit["kind"]]] if unit["kind"] in REVIEW_FILES else []
    for name in files + ["tools/encyclopedia_dispositions.json"]:
        for entry in read_json(ROOT / name)["entries"]:
            if cited_lines({"reviews": [entry]}) & scope_lines(unit):
                records.append(entry)
    return records


def scope_lines(unit: dict[str, Any]) -> set[tuple[str, int]]:
    return {(part["path"], n) for part in unit["scope"] for n in range(part["start"], part["end"] + 1)}


def uncovered(draft: dict[str, Any], unit: dict[str, Any]) -> list[str]:
    kind = unit["kind"]
    cited = cited_lines(draft) | cited_lines({"reviews": existing_records(unit)})
    errors = []
    if kind in ("e1_exchange", "e5_quest"):
        pattern = ITEM_CALL if kind == "e1_exchange" else QUEST_CALL
        for part in unit["scope"]:
            if not part["path"].startswith("npc/"):
                continue
            lines = source_lines(part["path"])
            for number in range(part["start"], part["end"] + 1):
                code = lines[number - 1].split("//", 1)[0]
                if pattern.search(code) and (part["path"], number) not in cited:
                    errors.append(f"coverage: {part['path']}:{number} `{code.strip()[:70]}` is in scope but no review or disposition cites it")
    elif kind == "e4_formula":
        claimed = {skill_id for review in draft.get("reviews", []) for skill_id in review.get("skill_ids", [])}
        claimed |= {int(i) for d in draft.get("dispositions", []) for i in d.get("subject", {}).get("ids", []) if str(i).isdigit()}
        missing = sorted(set(unit["subject"]["skill_ids"]) - claimed)
        if missing:
            errors.append(f"coverage: skill IDs {missing} have no review or disposition")
    elif kind == "e2_stock":
        for declaration in unit["subject"]["declarations"]:
            if (declaration["path"], declaration["line"]) not in cited:
                errors.append(f"coverage: declaration {declaration['name']} at line {declaration['line']} is not cited")
    elif kind in ("e6_boss", "e6_zone") and not (draft.get("reviews") or draft.get("dispositions")):
        errors.append("coverage: the unit needs one review or one disposition")
    return errors


def check_draft(draft: Any, unit: dict[str, Any]) -> list[str]:
    if not isinstance(draft, dict):
        return ["draft.json must be one JSON object"]
    errors = []
    if draft.get("unit") != unit["id"]:
        errors.append(f"draft `unit` must be {unit['id']!r}")
    reviews = draft.get("reviews", [])
    dispositions = draft.get("dispositions", [])
    if not isinstance(reviews, list) or not isinstance(dispositions, list):
        return errors + ["`reviews` and `dispositions` must be lists"]
    if not reviews and not dispositions:
        remaining = uncovered(draft, unit)
        if remaining or unit["kind"] not in ("e1_exchange", "e5_quest", "e2_stock"):
            return errors + ["the draft has no reviews and no dispositions"] + remaining
        return errors
    if reviews and unit["kind"] not in REVIEW_CHECKS:
        errors.append(f"a {unit['kind']} unit records dispositions only; move each review into `dispositions`")
    seen = set()
    for index, review in enumerate(reviews):
        label = f"reviews[{index}] {review.get('id', '?')}"
        if review.get("id") in seen:
            errors.append(f"{label}: duplicate id")
        seen.add(review.get("id"))
        if unit["kind"] in REVIEW_CHECKS:
            errors += REVIEW_CHECKS[unit["kind"]](label, review, unit)
    for index, record in enumerate(dispositions):
        label = f"dispositions[{index}] {record.get('id', '?')}"
        if record.get("id") in seen:
            errors.append(f"{label}: duplicate id")
        seen.add(record.get("id"))
        errors += check_disposition(label, record, unit)
    errors += collisions(reviews, dispositions, unit)
    if not errors:
        errors += uncovered(draft, unit)
    return errors


REVIEW_FILES = {
    "e1_exchange": "tools/item_exchange_reviews.json",
    "e4_formula": "tools/skill_formula_reviews.json",
    "e5_quest": "tools/quest_flow_reviews.json",
    "e6_boss": "tools/boss_behavior_reviews.json",
}


def collisions(reviews: list[dict[str, Any]], dispositions: list[dict[str, Any]], unit: dict[str, Any]) -> list[str]:
    errors = []
    if reviews and unit["kind"] in REVIEW_FILES:
        existing = read_json(ROOT / REVIEW_FILES[unit["kind"]])["entries"]
        taken = {entry["id"] for entry in existing}
        for review in reviews:
            if review.get("id") in taken:
                errors.append(f"review id {review.get('id')!r} already exists in {REVIEW_FILES[unit['kind']]}")
        if unit["kind"] == "e4_formula":
            claimed = {skill_id for entry in existing for skill_id in entry["skill_ids"]}
            for review in reviews:
                for skill_id in set(review.get("skill_ids", [])) & claimed:
                    errors.append(f"skill {skill_id} is already covered by another reviewed formula")
    if dispositions:
        taken = {entry["id"] for entry in read_json(ROOT / "tools/encyclopedia_dispositions.json")["entries"]}
        for record in dispositions:
            if record.get("id") in taken:
                errors.append(f"disposition id {record.get('id')!r} already exists")
    return errors


def check_date(value: Any) -> bool:
    return isinstance(value, str) and DATE.match(value) is not None


__all__ = ["check_draft", "check_sources", "check_disposition", "REVIEW_FILES", "EVIDENCE_STATES", "check_date"]
