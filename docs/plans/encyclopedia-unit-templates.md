# Encyclopedia unit templates

`tools/encyclopedia_unit.py packet` copies one section of this file into each
unit packet. Do not read this whole file during a unit; the packet already
contains the part you need.

Every draft is one JSON object at `.encyclopedia-staging/<unit-id>/draft.json`:

```json
{"unit": "<unit id>", "reviews": [], "dispositions": []}
```

## Rules for every kind

<!-- kind: common -->
1. Read only the packet. Cite only line numbers that appear in its excerpts.
2. For every source, `lines` lists line numbers and `required_source_literals`
   quotes text copied exactly from those lines. Each cited line needs a quote,
   and each quote must be on a cited line. Copy text, do not retype it.
3. Write what the source does, not what the dialogue says. When dialogue and
   script disagree, say so in a condition.
4. When a branch depends on something the packet does not show (a variable set
   elsewhere, a random roll, a live server state), say that in a condition. Do
   not guess the value.
5. If you cannot tell what the source does, write a disposition with
   `"result": "unknown"` and quote the line that stops you. That is a correct
   answer. A confident guess is not.
6. Do not set `reviewed_by` or `reviewed_on`; `accept` stamps them.
7. IDs are lowercase letters, digits and underscores, unique, and start with
   the map, monster, or subject name.

Disposition shape (used by every kind):

```json
{
  "id": "alb2trea_tool_dealer_stock",
  "unit": "<unit id>",
  "package": "E2",
  "subject": {"kind": "npc", "name": "Tool Dealer#alb"},
  "result": "verified",
  "statement": "One sentence saying what the source establishes.",
  "conditions": [],
  "sources": [{"path": "npc/...", "lines": [807], "required_source_literals": ["exact text"]}]
}
```

`result` is one of `verified`, `conditional` (list conditions),
`configured_estimate` (a configured value; list conditions), `unknown` (quote
the blocking line), `absent` (you searched the declared scope and it is not
there), or `not_applicable` (say why and where it belongs instead).
`subject.kind` is one of `npc`, `item`, `quest`, `skill`, `monster`, `zone`.
<!-- end -->

## E1 item exchange (`e1_exchange`)

<!-- kind: e1_exchange -->
The packet is one NPC body that the automatic prover refused. Your job is to
account for **every** `getitem` and `delitem` line in it. `validate` fails if
one is left uncited.

- A branch where the player gives items (or Zeny) and receives items is an
  exchange: write a review.
- A grant with no cost that belongs to a quest step is not an exchange: write a
  disposition with `"result": "not_applicable"`, `"subject": {"kind": "npc",
  "name": "<npc>"}`, and a statement naming the quest variable that gates it.
- A grant or removal whose outcome you cannot pin down (random table, amount in
  a variable set elsewhere): disposition `"result": "unknown"`, quoting it.

Review shape (goes into `tools/item_exchange_reviews.json`):

```json
{
  "id": "map_npc_short_name",
  "title": "Who makes what",
  "npc": {"name": "<display name>", "internal_name": "<exact header name>", "map": "<map>", "x": 0, "y": 0, "service_role": "short role"},
  "inputs": [{"item_id": 512, "amount": 3}],
  "outcomes": [{"label": "Apple Juice", "item_ids": [531], "amount": 1, "selection": "single_option"}],
  "conditions": ["Every check the branch makes before the grant, one per sentence."],
  "source": {"path": "npc/...", "lines": [949, 950, 951]},
  "required_source_literals": ["delitem Apple,3;", "delitem Empty_Bottle,1;", "getitem Apple_Juice,1;"]
}
```

- `npc.internal_name`, `map`, `x`, `y` must equal the NPC header; the packet
  prints them.
- Every `delitem` you quote must be one input, with the same amount. Every
  `getitem` you quote must be in one outcome, with the same amount. The packet
  lists each item's ID.
- `selection`: `single_option` (the only result), `player_choice` (a menu
  picks one), `random` (a roll picks one), or `all` (every listed item).
- Zeny costs (`Zeny -= N`) go in `conditions`, quoted in the literals.
- No `evidence_state`: records with conditions are shown as conditional.

examples: airplane_01_meltz_apple_juice, prt_in_sir_gray_claymore_craft
<!-- end -->

## E2 stock declarations (`e2_stock`)

<!-- kind: e2_stock -->
The packet holds up to eight `shop`, `cashshop`, `pointshop`, `itemshop`,
`trader`, or `marketshop` declarations. Write **one disposition per
declaration**, citing its header line. Record only dispositions.

- `verified`: fixed stock, always available. A `trader` with no `tradertype`
  sells for Zeny at the item database price; say so.
- `conditional`: stock or availability depends on something (a `tradertype`
  currency, an `OnInit` branch, a `disablenpc`, an event). List each one.
- `unknown`: the stock is added from a variable or another script not in the
  packet. Quote the line.

Example (npc/re/merchants/shops.txt):

```json
{
  "id": "alb2trea_tool_dealer_alb_stock",
  "unit": "<unit id>",
  "package": "E2",
  "subject": {"kind": "npc", "name": "Tool Dealer#alb"},
  "result": "verified",
  "statement": "Tool Dealer#alb is a trader with no tradertype, so it sells its ten OnInit sellitem entries for Zeny at item-database prices.",
  "conditions": [],
  "sources": [{"path": "npc/re/merchants/shops.txt", "lines": [807, 808, 809], "required_source_literals": ["alb2trea,87,65,5\ttrader\tTool Dealer#alb\t4_M_01,{", "OnInit:", "sellitem Arrow;"]}]
}
```

Tabs inside a quote are written as `\t` in JSON.
<!-- end -->

## E3 translator pattern (`e3_translator`)

<!-- kind: e3_translator -->
The packet shows one documented command, its Hercules documentation, and item
scripts whose first untranslated statement uses it. You add one narrow rule to
`translate_simple_effect` in `tools/export_item_reference.py` and one test.

1. Copy the meaning from the documentation excerpt only. Not from the item
   name, a wiki, or the variable names.
2. Add one `match = re.fullmatch(...)` block beside the other command blocks in
   the statement loop (the packet prints where). Match only the literal shape
   of the examples. Anything else must still fall through to `return None`.
3. Add a test in `tools/tests/test_export_item_effect_phrases.py` that asserts
   the positive script's summary and that the negative script returns `None`.
4. Write the draft:

```json
{
  "unit": "<unit id>",
  "translator": {
    "doc_source": {"path": "doc/item_bonus.md", "lines": [120], "required_source_literals": ["exact doc text"]},
    "positive_script": "bonus2 bSubEle,Ele_Fire,10;",
    "expected_summary": "exact string the translator now returns",
    "negative_script": "a similar shape the rule must not accept;",
    "item_ids": [1234]
  }
}
```

`validate` imports the edited translator and checks both scripts, and that the
test file contains the negative script. `accept` re-runs the item exporter and
fails unless every listed item is now translated.

If the documentation does not fully define the behavior (a chance, a stack
rule, an unknown unit), stop and run `block` with that sentence.
<!-- end -->

## E3 untranslated scripts (`e3_dispose`)

<!-- kind: e3_dispose -->
The tool drafted this unit. Each item gets an `unknown` disposition quoting the
first statement the translator does not interpret. Run `validate`, then
`accept`. Do not edit the draft. If `validate` fails, run `block` with the
first error line.
<!-- end -->

## E4 skill formula (`e4_formula`)

<!-- kind: e4_formula -->
The packet holds one `case` group from `battle_calc_skillratio` in
`src/map/battle.c`, plus any other `case` label for the same skills elsewhere
in that file. Write one review for the group, or `unknown` dispositions if the
case cannot be followed.

- `formula`: what the case adds to or sets on `skillratio`, in words and as an
  expression, and that the result is a percentage multiplier. Only describe
  code you can see.
- `worked_example`: pick a skill level and compute the number, step by step.
- `conditions`: every `if`, `#ifdef RENEWAL`, and every other `case` site the
  packet shows for these skills, with what it changes. If another site changes
  damage and you did not trace it, say "not traced here".
- `per_skill`: one sentence per skill ID when they differ; `[]` otherwise.
- `evidence_state`: `verified` only when there are no conditions; otherwise
  `conditional`.

```json
{
  "id": "weapon_skillratio_skill_name",
  "title": "Skill Name damage ratio",
  "skill_ids": [0],
  "review_method": "Read the case in battle_calc_skillratio and the listed other case sites.",
  "evidence_state": "conditional",
  "formula": "...",
  "per_skill": [],
  "worked_example": "At skill level 5: 100 + 5*40 = 300%.",
  "conditions": ["..."],
  "sources": [{"path": "src/map/battle.c", "lines": [0], "required_source_literals": ["case XX_NAME:"]}]
}
```

examples: weapon_skillratio_ac_shower, magic_skillratio_wizard_elemental_spells
<!-- end -->

## E5 quest chain (`e5_quest`)

<!-- kind: e5_quest -->
The packet holds every NPC block that sets, changes, erases, or completes the
unit's quest IDs, plus their quest database records. Account for **every**
`setquest`, `changequest`, `erasequest` and `completequest` line; `validate`
fails if one is uncited.

- One review per journey. `conditions` walk the journey in order: who offers
  it, prerequisites, each stage, each branch (including branches that grant
  nothing), rewards, reset or repeat, and time limits from the quest record.
- A quest line you cannot follow gets an `unknown` disposition quoting it.
- `verified_item_rewards` lists only rewards whose `getitem` you quoted on the
  completion path: `[{"item_id": 0, "amount": 1}]`. Omit it otherwise.

```json
{
  "id": "map_npc_quest_short_name",
  "title": "...",
  "quest_ids": [0],
  "review_method": "Traced the offer, stages and turn-in in the packet's NPC blocks.",
  "evidence_state": "conditional",
  "conditions": ["..."],
  "sources": [{"path": "npc/...", "lines": [0], "required_source_literals": ["setquest 0;"]}]
}
```

examples: dicastes_registration_7184, mora_raffle_researcher_5028
<!-- end -->

## E5 absent quests (`e5_absence`)

<!-- kind: e5_absence -->
The tool drafted this unit. Each quest ID in it has a quest database record,
but its number does not appear in any loaded NPC script or item script. Each
gets an `absent` disposition citing its `Id:` line. Run `validate`, then
`accept`.
<!-- end -->

## E6 boss behavior (`e6_boss`)

<!-- kind: e6_boss -->
The packet holds the monster's `mob_db` record and its `mob_skill_db` block.
Write one review. Configured values are configured: say "configured", not
"observed".

- `summons`: one entry per `NPC_SUMMONSLAVE` or `NPC_SUMMONMONSTER` skill, with
  the trigger (`SkillState`, `CastCondition`, `ConditionData`), candidates
  when the packet shows them, `amount` from `SkillLevel`, and `Rate` as a
  chance out of 10000.
- `hp_threshold_behaviors`: one entry per `MSC_MYHPLTMAXRATE` row.
- Monsters with neither get empty lists and a condition saying so.
- `evidence_state`: `configured_estimate` for configured rows.

Example (tested fixture, Amon Ra):

```json
{
  "id": "amon_ra_heal_at_half_hp",
  "monster_id": 1511,
  "sprite_name": "AMON_RA",
  "title": "Amon Ra heals itself below half HP",
  "evidence_state": "configured_estimate",
  "summons": [],
  "hp_threshold_behaviors": [{"skill": "AL_HEAL", "target": "self", "threshold_pct": 50, "effect": "Casts level-10 Heal on itself when its HP is at or below 50%.", "source_line": 24815}],
  "conditions": ["Rate 10000 of 10000 per check while idle, with a 10000 ms delay between casts; these are configured values."],
  "sources": [{"path": "db/re/mob_skill_db.conf", "lines": [24806, 24807, 24808, 24809, 24810, 24811, 24814, 24815], "required_source_literals": ["AMON_RA: {", "AL_HEAL: {", "SkillState: \"MSS_IDLE\"", "SkillLevel: 10", "Rate: 10000", "Delay: 10000", "CastCondition: \"MSC_MYHPLTMAXRATE\"", "ConditionData: 50"]}]
}
```
<!-- end -->

## E6 map zone (`e6_zone`)

<!-- kind: e6_zone -->
The packet holds one zone from `db/re/map_zone_db.conf`. Write one
disposition with `"result": "configured_estimate"`, `"subject": {"kind":
"zone", "name": "<zone>"}`. `conditions` list each disabled skill, disabled
item, and map flag the zone sets, quoted. Say which zone it inherits from if
it has `inherit`. Do not claim which maps use it unless the packet shows it.
<!-- end -->
