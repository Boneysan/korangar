# Reference data the client loads but does not show

Triage from the GDD verification audit (2026-10-04, Batch 0 dead-code map). Replacing the
file-wide `#![allow(dead_code)]` in `korangar/src/dm/reference_data.rs` exposed 21 fields and two
methods nothing reads. None of the structs uses `deny_unknown_fields`, so a field could be deleted
without breaking the export, but each is either audit provenance or something the Guide could
show. Every struct now carries an `expect(dead_code, reason = …)`: the compiler flags it the day a
field gains a reader, and flags any new unread field immediately.

## Provenance: keep in the export, never display

| Struct | Unread fields | Why |
|---|---|---|
| `ReferencePilotSkillLayer` | `skill_source` | Which Hercules file the pilot layer came from |
| `ReferenceSpawnRegion` | `kind` | Exporter classification |
| `ReferenceGrantNpcClue` | `internal_name` | NPC script name, not a display name |
| `ReferenceRuntimeMapFlagClue` | `evidence_state` | Review state |
| `ReferenceRuntimeMapFlagReview` | `id`, `flags`, `review_method` | Review record |
| `ReferenceItemGroupContainer` | `source` | Source file |
| `ReferenceSkillFormulaReview` | `reviewed_by`, `reviewed_on`, `review_method` | Review record |
| `ReferenceQuestFlowReview` | `id` | Review record id |

## Could be shown in the Guide (a display decision, not a defect)

| Struct | Unread fields | What a player would gain |
|---|---|---|
| `ReferenceScriptedSpawn` | `availability` | When a scripted spawn is present |
| `ReferenceCraftingRecipe` / `ReferenceCraftingEntry` | `skill_id`, `combos`, entry payload | Which skill crafts it; ingredient combinations |
| `ReferenceItemComboRecipe` | `name`, `members`, `script`, `source` | Item set (combo) names, pieces and effect |
| `ReferenceExchangeOutcome` | `condition` | What an NPC exchange requires |
| `ReferenceItemShop` | `quantity` | Stock limits |
| `ReferenceSkill` | `attack_type` | Physical / magic / misc |
| `ReferenceNpcServiceReview` | `locations` | Where a service NPC stands (read by tests only) |
| `ReferenceNpcOffer` | `shop_type` | Zeny, points or item-currency shop |
| `ReferenceStatJob` | `name` | Job name on stat tables |
| `ReferenceQuestRewardCandidate` | `status` | Whether a reward is confirmed |

## Unused methods

- `ReferenceItem::matches_query`: superseded by the Guide's search (`alias_targets` and its own filters).
- `ReferenceStatJob::points_at_level`: stat points are read from the exp/stat tables directly.
