# Adventure Guide data contract — GDD §9.5 / next slices 8–9

**Parent:** [GDD §9.5](../GDD.md#95-adventure-guide--the-in-game-encyclopedia) and [next-slices plan](../plans/gdd-next-slices.md). The target is comprehensive, searchable information about *this* server. Extraction is staged; a category is not “done” merely because a JSON filename exists.

## Build and provenance

Create one reproducible entry point under `korangar/tools/` that reads the sibling Hercules checkout, produces the category files in a fixed order, and supports a non-mutating `--check`. Record schema version, source revision, renewal/pre-renewal mode, generated timestamp outside byte-compared content, and source path/record for each entry. Packaged client data must match the server revision; a mismatched pack displays its data revision and treats rate/rule values as possibly stale.

`tools/export_skill_info.py` and `tools/extend_bestiary_export.py` are starting points. The original exporter for the current `bestiary.json`, `items.json`, and `cards.json` was not committed, so restore deterministic generation for those files before changing their shape. Preserve existing DM consumers (`src/dm/data.rs`, loot, Bestiary) with a migration or parallel versioned files. Reject duplicate IDs, dangling cross-links, and parse failures; report category counts and coverage. Authored text/overrides live in a separate reviewed file with an explicit source and may be localized later.

| Category | Source and first useful fields | Extraction limit to label |
|---|---|---|
| Monsters | `mob_db.conf`, `mob_skill_db.conf`, loaded spawn scripts: identity, level, stats, modes, skills, drops, spawn maps | Conditional spawns and scripted bosses need reviewed additions; exact spawn cells stay private. |
| Items and cards | `item_db.conf`, combo/group tables, reverse monster-drop index: stats, equipment rules, source links | Item `Script` text is code, not a player explanation. Effects need authored or mechanically verified prose. |
| Skills and jobs | `skill_db.conf`, `skill_tree.conf`, job/EXP tables: per-level costs, cast, range, prerequisites, bonuses | Damage formulas in C/script and conditional costs cannot be inferred from one DB field. |
| Maps, NPCs, quests | Navigation graph, loaded spawn/NPC scripts, `quest_db.conf`, Towninfo: names, connections, services, quest IDs, known giver/reward | Arbitrary quest steps, conditional NPC locations, and service prices require authored records or “unknown”. |
| Status and mechanics | `status_effects.json` currently supplies names; `sc_config.conf`, status/battle code, refine/element/size/level tables, live configs supply effects, cure sources, odds, rates | Existing names do not establish effect or cure behavior. Code-derived formulas require an explicit versioned implementation and validation, never a copied wiki value. |
| Server rules | Effective import configs and permissions: EXP/drop rates, party share, death, autoloot, respec, DM mode | Config precedence matters; do not read only the stock default file. |

## Client model and knowledge policy

Use stable `category:id` keys, normalized search aliases, display name, concise summary, source/provenance, and typed links. Search returns all matching entries with category labels; cross-links use IDs, not string matching. Unknown or unsupported fields are omitted or marked “not documented yet”; never silently show a default that may be false. A partial category may ship if its coverage is plainly labeled.

**Open Database is the friends-server baseline.** Every generated reference entry and verified build-relevant field is searchable from a fresh account: monster stats, element/race/size, skills and weaknesses, item/card effects and sources, drop rates, skill costs/prerequisites, jobs, maps, mechanics, and effective server rules. Search indexes names, aliases, effect terms, source monsters, and cross-linked IDs, with category filters. Unsupported or unverified fields say “not documented yet” rather than appearing locked. Campaign plot spoilers and DM-only actions are separate from mechanics. Account-wide discovery is a progress/history layer, never a prerequisite for reference information. A more restrictive knowledge mode would be a later explicit server-owner decision; it is not a dependency of this plan. The DM Bestiary's session-only kill list is not the authority for player discovery. No peer party message can grant an unlock.

First dedicated player window: monsters, items, cards, **and the already-exported skills**; search, result details, item → source monster → known map link, skill → prerequisite/effect links, empty/unknown states, and no DM controls. The existing searchable Bestiary can be opened through the unguarded Commands window and its Reveal all toggle is client-only; do not count it as the new player policy. Separate the views and gate DM-only controls on actual server permission or remove them from a player-accessible launcher. Then add jobs, maps/quests, status, mechanics, and server rules as each export passes its own coverage review. Navigate appears only when the graph has a verified location. A source revision and incomplete-data label are visible in the window.

## Acceptance

- Regenerate twice with identical bytes; `--check` catches a changed monster, item, skill, quest, and effective server rule.
- Existing DM views load after the export; IDs and drop links reconcile, with a report for records that do not.
- A new non-DM account searches “Hydra”, “Oridecon”, and a skill or effect term; traverses category links and can inspect verified rates, effects, and requirements before any encounter. Missing fields explain the coverage gap, not a hidden unlock.
- Two characters on one account share discovery badges/notes; another account does not. Reconnect restores the server snapshot without changing reference visibility.
- Compare a sample of displayed rates/formulas/refine odds with the live configured server before marking those fields complete.
