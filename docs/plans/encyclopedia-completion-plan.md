# Encyclopedia completion plan

Adapted on 2026-09-30 from [encyclopedia-local-model-plan.md](encyclopedia-local-model-plan.md). That file remains the rule for how one unit is traced, checked, and committed. This file is the queue. `tools/encyclopedia_unit.py queue` turns it into `docs/plans/encyclopedia-unit-queue.json`, and a local model works that queue through [encyclopedia-procedures.md](encyclopedia-procedures.md): one packet, one draft, a separate verifier session, then the tool accepts and commits. The model does not choose a unit. `tools/encyclopedia_loop.py` is not this plan. It only proves a braced one-item exchange, its queue is empty, and running it again does not advance E0–E7.

Live client testing stays deferred until the source reviews below have a disposition. Do not mark E7 passed from a build or a static export.

## Current disposition

Counts are from the exports on 2026-09-30. Hercules tracked revision `cc83ca152`. A clue is not a disposition.

| Phase | Already dispositioned | Still open |
|---|---|---|
| E0 | Loaded-script manifest: 653 scripts. `export_loaded_script_manifest.py --check` passes. | Per-claim state labels are not on every Guide field. |
| E1 | 87 reviewed exchanges, 302 linked items. 107 of 107 literal service calls are E2, not E1. | 906 NPC bodies with item calls refused by the prover: 333 switch, 292 braceless, 147 menu, 109 multi-item, 19 loop or random, 6 call. Other `getitem` / `delitem` clues that are not exchanges. |
| E2 | 14 service records cover all 107 indexed call sites. | Ordinary barter, cash, and trader stock that is not one of those calls. |
| E3 | 3,665 of 12,171 non-card items, 635 of 1,012 cards, and 269 of 436 combos have a supported summary. | 2,035 item and card scripts the translator refuses, in 191 clusters by first refused statement. |
| E4 | 12 formula records, 50 of 1,170 skills. Element and size tables exist. | The other skills, and status duration, chance, cure, and interruption. |
| E5 | 52 flow records cover 124 of 3,172 quests. 40 of 41 nearby grant candidates are reviewed. | 3,048 quests. |
| E6 | 11 bosses, 43 scripted-spawn groups, the Poring War flag cycle (source only). | Map-zone overrides and any live map state. |
| E7 | Release build and the encyclopedia load test have passed in earlier sessions. | The fresh-account journeys. Deferred. |

## Queue as built (2026-09-30)

`encyclopedia_unit.py queue` built 2,341 units. **Local** units fit a small model's packet. **Strong** units are too large or branchy (packet over 900 lines, a skillratio case with branches or `#ifdef`, a quest reached only indirectly) and wait for `next --tier strong`.

| Package | Local | Strong | Unit |
|---|---|---|---|
| E0 | 1 | 0 | Evidence-label rule check (`e0`); passes. |
| E1 | 742 | 164 | One refused NPC body; every item call must be covered. |
| E3 | 191 | 0 | 102 translator clusters on a documented command, 89 tool-drafted `unknown` clusters (control flow or undocumented). |
| E2 | 35 | 0 | Up to eight shop or trader declarations from one file. |
| E4 | 66 | 147 | One `battle_calc_skillratio` case group. |
| E5 | 427 | 469 | One chain of quest IDs sharing NPC blocks, plus one tool-drafted unit for 644 quests whose number appears in no loaded NPC or item script. |
| E6 | 96 | 2 | One MVP without a boss review, or one map zone. |
| E7 | 0 | 1 | Blocked until the user ends the live-test deferral. |

Rebuild the queue after Hercules source changes. A rebuild keeps unit state; a unit whose source moved keeps its record if it was done or blocked.

## What one unit is

Use the Qwen plan's nine steps for every unit. Stop after one accepted unit and checkpoint the ledger. A unit is one of these, never "the rest of the phase":

1. One NPC script body, or one shared helper, for an item exchange.
2. One shop, barter, or cash declaration family.
3. One item-script pattern whose semantics are copied from Hercules `doc/item_bonus.md` or `doc/script_commands.txt`, plus a failing example and a test that a wrong shape stays untranslated.
4. One skill or status family, read in `battle.c` or `status.c`, with the formula and one worked example.
5. One quest chain from offer to turn-in, including branches that grant nothing.
6. One boss script or one map-zone family.
7. The E7 live pass, once, after the source units that the journeys depend on have dispositions.

After two attempts with no accepted record, mark that unit blocked and take the next ID. Do not widen the unit to make it pass.

## Order

Work in this order. Do not start E7 early.

1. **E0 label sample (done by `encyclopedia_unit.py e0`).** Pick one item, one quest, one NPC, one map flag, one spawn, and one formula already in the Guide. Confirm each shows indexed, reviewed, or unknown, and that a clue is not shown as confirmed. Fix only a label that is wrong. Exit: those six screens or data records agree with the evidence enum.
2. **E1 candidate list (built by `queue`).** Run `python3 tools/review_simple_exchanges.py --candidates docs/plans/encyclopedia-e1-candidates.json`. Each refused NPC gets `id`, `path`, `line`, `npc`, and `reason` (`braceless`, `switch`, `menu`, `call`, `loop-or-random`, or `multi-item`). Do not author reviews in this unit. Exit: the procedures file's E1 candidate steps pass.
3. **E1 reviews, path order.** Take candidate IDs in path and line order. Each accepted review must pass `export_item_exchange_reviews.py`. A branch that is only a quest reward with no returned item stays a quest unit, not an exchange.
4. **E3 clusters, then leftovers.** Group the 2,035 untranslated item scripts by the first statement the translator rejects. A cluster becomes a translator unit only when the command is documented. A branch, random roll, or unknown command is a disposition of `unknown` with the quoted statement, not a guessed summary.
5. **E2 stock families.** Inventory loaded `shop`, `trader`, `barter`, and cash declarations that are not already in the 107 service calls. Review one declaration family per unit.
6. **E4 skill families.** Prefer a skill whose `battle.c` case is a single ratio expression not already in `tools/skill_formula_reviews.json`. Do not copy a ratio from a wiki or from the skill name.
7. **E5 chains.** High-use boards and job changes first, then quest ID order. One chain per unit. A shared helper may cover many quest IDs only after that helper is fully traced.
8. **E6 bosses and zones.** One boss script or one map-zone file per unit. Configured values stay `configured`. Runtime results stay unknown until someone sees them.
9. **E7, one unit.** Fresh non-DM account. Three journeys: item to source to route, skill to status to cure, quest to turn-in. Record what was watched and what disagreed. This unit is blocked until the user ends the live-test deferral.

## Scale

These are pacing estimates, not exit gates. The exit is the roadmap checklist, not a count of passes.

- E1: on the order of 100 units if a unit keeps closing about 10 branches.
- E3: a few pattern units, then one disposition per remaining script that does not share a pattern.
- E4: on the order of 200 units if several skills still share a case.
- E5: several hundred units at one chain each. This is the bulk.
- E2 and E6: open until their inventories exist; do not guess a count before unit 5 and unit 8.
- E7: one unit.

Repeating `encyclopedia_loop.py` is not one of these units.

## Session checkpoint

After each unit, set the ledger `next_action` to the next candidate ID. Print the unit ID, files changed, the exporter or test that passed, and the next ID. If the session ends mid-unit, the unit stays `active` and the next session resumes that ID. Do not open a second phase while an `active` unit is unfinished.
