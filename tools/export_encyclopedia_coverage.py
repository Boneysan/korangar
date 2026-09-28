#!/usr/bin/env python3
"""Build a coverage report from the encyclopedia's current versioned exports."""

from __future__ import annotations

import argparse
from enum import Enum
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "docs" / "encyclopedia-coverage.v1.json"

VERSIONED = {
    "monsters": ROOT / "docs" / "bestiary.v1.json",
    "items": ROOT / "docs" / "items.v1.json",
    "cards": ROOT / "docs" / "cards.v1.json",
    "job_skills": ROOT / "docs" / "job-skills.v1.json",
    "job_bonuses": ROOT / "docs" / "job-bonuses.v1.json",
    "statuses": ROOT / "docs" / "status-effects.v1.json",
    "quests": ROOT / "docs" / "quests.v1.json",
    "npcs": ROOT / "docs" / "npcs.v1.json",
    "item_exchanges": ROOT / "docs" / "item-exchanges.v1.json",
    "map_flags": ROOT / "docs" / "map-flags.v1.json",
    "crafting": ROOT / "docs" / "crafting.v1.json",
    "item_grants": ROOT / "docs" / "item-script-grants.v1.json",
    "refine": ROOT / "docs" / "refine.v1.json",
    "server_rules": ROOT / "docs" / "server-rules.v1.json",
    "npc_service_clues": ROOT / "docs" / "npc-service-clues.v1.json",
    "npc_service_reviews": ROOT / "docs" / "npc-service-reviews.v1.json",
    "boss_behavior": ROOT / "docs" / "boss-behavior.v1.json",
    "skill_formula_reviews": ROOT / "docs" / "skill-formula-reviews.v1.json",
    "scripted_spawn_reviews": ROOT / "docs" / "scripted-spawn-reviews.v1.json",
}

class EvidenceState(str, Enum):
    VERIFIED = "verified"
    CONDITIONAL = "conditional"
    CONFIGURED_ESTIMATE = "configured_estimate"
    SOURCE_CLUE = "source_clue"
    NOT_REVIEWED = "not_reviewed"
    UNKNOWN = "unknown"


EVIDENCE_STATES = {
    EvidenceState.VERIFIED.value: "Source or reviewed control flow establishes this claim for the exported server revision.",
    EvidenceState.CONDITIONAL.value: "The claim is supported and its known condition or branch is shown.",
    EvidenceState.CONFIGURED_ESTIMATE.value: "A configured value is shown with its runtime-modifier caveat.",
    EvidenceState.SOURCE_CLUE.value: "A source reference suggests a relationship but its player-accessible path is unreviewed.",
    EvidenceState.NOT_REVIEWED.value: "Relevant source exists, but its behavior has not been semantically reviewed.",
    EvidenceState.UNKNOWN.value: "The value is unavailable, dynamic, or not extracted; give a reason when known.",
}


def read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read {path.relative_to(ROOT)}: {error}") from error


def entries(data: dict) -> list[dict]:
    return data.get("entries", [])


def dimension(observed: int, total: int, basis: str) -> dict:
    return {"observed": observed, "total": total, "basis": basis}


def build_report() -> dict:
    data = {key: read_json(path) for key, path in VERSIONED.items()}
    revisions = {item.get("source_revision") for item in data.values()}
    modes = {item.get("mode") for item in data.values()}
    if len(revisions) != 1 or None in revisions:
        raise ValueError(f"versioned encyclopedia exports have mismatched revisions: {sorted(str(x) for x in revisions)}")
    if len(modes) != 1:
        raise ValueError(f"versioned encyclopedia exports have mismatched modes: {sorted(str(x) for x in modes)}")

    monsters = entries(data["monsters"])
    items = entries(data["items"])
    cards = entries(data["cards"])
    quests = entries(data["quests"])
    npcs = entries(data["npcs"])
    exchanges = entries(data["item_exchanges"])
    flags = entries(data["map_flags"])
    runtime_flags = data["map_flags"].get("runtime_clues", [])
    runtime_flag_reviews = data["map_flags"].get("runtime_reviews", [])
    statuses = entries(data["statuses"])
    grants = data["item_grants"].get("entries", [])
    consumptions = data["item_grants"].get("consumptions", [])
    recipes = entries(data["crafting"])
    skill_data = read_json(ROOT / "docs" / "skills.json")
    status_icons = read_json(ROOT / "docs" / "status_effects.json")
    job_skills = entries(data["job_skills"])
    job_bonuses = entries(data["job_bonuses"])
    rules = entries(data["server_rules"])
    refine = data["refine"]
    npc_service_clues = entries(data["npc_service_clues"])
    npc_service_reviews = entries(data["npc_service_reviews"])
    boss_behaviors = entries(data["boss_behavior"])
    skill_formula_reviews = entries(data["skill_formula_reviews"])
    scripted_spawn_reviews = entries(data["scripted_spawn_reviews"])

    monster_spawn_rows = [entry for entry in monsters if entry.get("spawn_regions")]
    monster_script_rows = [entry for entry in monsters if entry.get("scripted_spawn_references")]
    item_groups = {group.get("source", {}).get("record") for item in items for group in item.get("contained_in_groups", [])}
    combo_records = {
        combo.get("source", {}).get("record"): combo
        for entry in [*items, *cards]
        for combo in entry.get("combos", [])
    }
    all_status_mechanics = [status for entry in statuses for status in entry.get("statuses", [])]
    linked_status_icon_rows = sum(bool(entry.get("statuses")) for entry in statuses)
    quest_npc_relations = [npc for quest in quests for npc in quest.get("npc_references", [])]
    quest_reward_candidates = [candidate for quest in quests for candidate in quest.get("item_reward_candidates", [])]
    reviewed_quest_npcs = [npc for npc in quest_npc_relations if npc.get("reviewed_role")]
    verified_quest_rewards = [npc for npc in quest_npc_relations if npc.get("verified_reward")]
    reviewed_quest_item_rewards = [
        reward for quest in quests for reward in (quest.get("flow_review") or {}).get("verified_item_rewards", [])
    ]
    npc_offers = [offer for npc in npcs for offer in npc.get("offers", [])]
    exchange_item_ids = {
        item["item_id"] for exchange in exchanges for item in exchange.get("inputs", [])
    } | {
        item_id for exchange in exchanges for outcome in exchange.get("outcomes", []) for item_id in outcome.get("item_ids", [])
    }
    reviewed_service_roles = {
        (exchange["npc"]["npc_id"], exchange["npc"].get("service_role"))
        for exchange in exchanges if exchange["npc"].get("service_role")
    }
    statuses_with_call_sites = [status for status in all_status_mechanics if status.get("code_call_sites")]
    monster_skills = [skill for monster in monsters for skill in monster.get("skills", [])]
    call_sites = [site for status in all_status_mechanics for site in status.get("code_call_sites", [])]

    categories = {
        "monsters": {
            "records": len(monsters),
            "configured_skill_records": len(monster_skills),
            "skill_trigger_translations": dimension(sum(skill.get("trigger_translation_status") == "translated" for skill in monster_skills), len(monster_skills), "configured monster skill records with documented trigger state, condition, chance, and target semantics"),
            "skill_trigger_partial_records": sum(skill.get("trigger_translation_status") != "translated" for skill in monster_skills),
            "configured_drop_coverage": dimension(sum(bool(m.get("drops")) for m in monsters), len(monsters), "monster records with at least one configured drop row"),
            "static_spawn_coverage": dimension(len(monster_spawn_rows), len(monsters), "monster records with at least one loaded static spawn placement"),
            "script_spawn_coverage": dimension(len(monster_script_rows), len(monsters), "monster records with at least one indexed script-spawn clue"),
            "script_spawn_clues": sum(len(m.get("scripted_spawn_references", [])) for m in monsters),
            "reviewed_boss_behavior_records": len(boss_behaviors),
            "reviewed_boss_summon_groups": sum(len(b.get("summons", [])) for b in boss_behaviors),
            "reviewed_scripted_spawn_groups": len(scripted_spawn_reviews),
        },
        "items": {
            "records": len(items),
            "translated_effects": dimension(sum(i.get("effect_status") == "translated" for i in items), len(items), "records with a supported natural-language effect summary"),
            "noncard_records": sum(i.get("type") != "IT_CARD" for i in items),
            "noncard_translated_effects": dimension(sum(i.get("effect_status") == "translated" and i.get("type") != "IT_CARD" for i in items), sum(i.get("type") != "IT_CARD" for i in items), "non-card item records with a supported natural-language effect summary"),
            "drop_sources": dimension(sum(bool(i.get("drops_from")) for i in items), len(items), "records linked to at least one configured monster drop"),
            "literal_shop_sources": dimension(sum(bool(i.get("shops")) for i in items), len(items), "records linked to at least one literal loaded shop offer"),
            "container_sources": dimension(sum(bool(i.get("contained_in_groups")) for i in items), len(items), "records linked to at least one indexed container outcome"),
            "verified_quest_reward_sources": len(reviewed_quest_item_rewards),
            "script_grant_clues": len(grants),
            "script_consumption_clues": len(consumptions),
            "items_linked_to_reviewed_exchange": len(exchange_item_ids),
        },
        "cards": {
            "records": len(cards),
            "translated_effects": dimension(sum(c.get("effect_status") == "translated" for c in cards), len(cards), "card records with a supported natural-language effect summary"),
            "drop_sources": dimension(sum(bool(c.get("drops_from")) for c in cards), len(cards), "records linked to at least one configured monster drop"),
            "combo_membership_links": sum(len(c.get("combos", [])) for c in cards),
        },
        "combos_and_groups": {
            "combos": len(combo_records),
            "translated_combo_effects": dimension(sum(c.get("effect_status") == "translated" for c in combo_records.values()), len(combo_records), "distinct combo records with translated script summaries"),
            "item_groups": len(item_groups - {None}),
        },
        "skills_and_jobs": {
            "skill_records": len(skill_data) if isinstance(skill_data, list) else 0,
            "job_skill_trees": len(job_skills),
            "job_bonus_schedules": len(job_bonuses),
            "reviewed_formula_records": len(skill_formula_reviews),
            "skills_covered_by_reviewed_formulas": len({skill["skill_id"] for review in skill_formula_reviews for skill in review.get("skill_ids", [])}),
        },
        "statuses": {
            "icon_records": len(status_icons),
            "icon_rows_linked_to_server_status": dimension(linked_status_icon_rows, len(statuses), "status icon rows with one or more server status mechanics linked"),
            "linked_server_statuses": len(all_status_mechanics),
            "literal_call_site_clues": len(call_sites),
            "statuses_with_call_sites": len(statuses_with_call_sites),
        },
        "quests": {
            "records": len(quests),
            "quest_records_with_hunt_targets": dimension(sum(bool(q.get("targets")) for q in quests), len(quests), "quest records with at least one exported hunt target"),
            "hunt_target_rows": sum(len(q.get("targets", [])) for q in quests),
            "npc_stage_relations": len(quest_npc_relations),
            "quests_with_reviewed_flow": dimension(sum(bool(q.get("flow_review")) for q in quests), len(quests), "quest records with source-anchored reviewed flow notes"),
            "reviewed_npc_stage_relations": dimension(len(reviewed_quest_npcs), len(quest_npc_relations), "NPC-stage relations with an authored reviewed role"),
            "verified_npc_rewards": len(verified_quest_rewards),
            "verified_quest_item_rewards": dimension(len(reviewed_quest_item_rewards), len(quest_reward_candidates), "nearby literal item-grant candidates whose complete quest branch was manually reviewed"),
            "quests_with_verified_item_rewards": dimension(sum(bool((q.get("flow_review") or {}).get("verified_item_rewards")) for q in quests), len(quests), "quest records with at least one source-reviewed item reward"),
            "nearby_item_reward_clues": len(quest_reward_candidates),
        },
        "npcs_and_services": {
            "declarations": len(npcs),
            "map_known_declarations": dimension(sum(bool(n.get("map_known")) for n in npcs), len(npcs), "declarations whose map identifier resolves in the static map index"),
            "literal_shop_offers": len(npc_offers),
            "reviewed_exchange_records": len(exchanges),
            "reviewed_service_roles": {"count": len(reviewed_service_roles), "evidence_state": "conditional", "basis": "distinct service roles cited by source-reviewed exchange records"},
            "literal_service_call_clues": len(npc_service_clues),
            "reviewed_service_records": {"count": len(npc_service_reviews), "of_indexed_call_clues": len(npc_service_clues), "basis": "source-reviewed service records (storage/kafra/refine-UI/repair/divorce/reset/navigation); one record may cover several indexed call-site clues, e.g. the shared Kafra function"},
        },
        "maps_and_rules": {
            "map_flag_directives": len(flags),
            "effective_static_map_flags": dimension(sum(bool(f.get("effective_static")) for f in flags), len(flags), "map/flag declarations selected as effective in loaded static file order"),
            "maps_with_flag_directives": len({f.get("map") for f in flags}),
            "runtime_map_flag_call_clues": dimension(len(runtime_flags), len(runtime_flags), "loaded-script setmapflag/removemapflag calls with a literal MF_ flag; call execution and conditions are unreviewed"),
            "runtime_map_flag_clues_with_literal_map": dimension(sum(bool(clue.get("map")) for clue in runtime_flags), len(runtime_flags), "runtime call clues whose first argument is a literal map name"),
            "source_reviewed_runtime_map_flag_behaviors": dimension(len(runtime_flag_reviews), len(runtime_flag_reviews), "runtime map-flag call groups whose loaded-script control flow and conditions were reviewed; live map state is not observed"),
            "server_rule_entries": len(rules),
            "element_matchup_tables": dimension(sum(str(rule.get("id", "")).startswith("element-matchup-") for rule in rules), 10, "complete defender-element tables with four defense levels and ten attacking elements"),
            "weapon_size_adjustment_tables": dimension(sum(rule.get("id") == "weapon-size-adjustments" for rule in rules), 1, "configured weapon damage size table covering three target sizes and documented weapon types"),
        },
        "crafting_and_mechanics": {
            "production_and_conversion_recipes": len(recipes),
            "refinement_weapon_levels": len(refine.get("weapon_levels", [])),
        },
    }

    return {
        "schema_version": 1,
        "source_revision": next(iter(revisions)),
        "mode": next(iter(modes)),
        "evidence_states": EVIDENCE_STATES,
        "counting_policy": "Counts describe exported records or explicitly named subsets, not completeness of player-facing explanations. Clues, reviewed facts, configured values, and runtime observations are separate states.",
        "categories": categories,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the report is stale")
    args = parser.parse_args()
    try:
        expected = json.dumps(build_report(), indent=2, ensure_ascii=False) + "\n"
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != expected:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {OUTPUT.relative_to(ROOT)}")
        return 0
    OUTPUT.write_text(expected, encoding="utf-8")
    print(f"wrote {OUTPUT} from {len(VERSIONED)} versioned exports")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
