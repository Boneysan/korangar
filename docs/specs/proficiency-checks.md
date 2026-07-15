# Targeted Spec — Proficiencies & Mechanical Checks (E7.16, future work)

**Parents**: PROJECT_PLAN.md E7.16, DM_INTERFACE.md §9 (dice group),
`plans/modern-mechanics.md` §2 (skill-check dialogue), Hercules
`planning/mechanics-and-quests.md` §8. **Depends on**: nothing hard
(extends the `@dm check` design before `dm_checks.txt` is written — cheaper
to build it right the first time); E7.2 dice cards render the results.
**Related**: `atb-structured-rounds.md` (same surface-the-engine philosophy).

**Status**: future work — captured 2026-07-15.

**Purpose**: Make `@dm check` a real tabletop skill system instead of a bare
stat roll, without inventing a new point economy: RO's **stat points** are
the ability-score layer, RO's **job skill points** are the proficiency
layer, and check outcomes get real in-engine consequences (pass a stealth
check → you are actually hidden).

## Check formula (supersedes the `d20 + stat/10` sketch in DM_INTERFACE.md)

```
d20 + ability_mod + proficiency_bonus   vs DC     (nat 20 / nat 1 kept)
ability_mod       = min(base_stat / 15, 8)        // integer division
proficiency_bonus = (skill_lv + 1) / 2, cap +5    // 0 if no qualifying skill
```

### Why /15, not /10 — bounded accuracy vs 100+ stats

RO stats don't stop at 99: gear, buffs, and the server's `max_parameter`
push totals well past 100 (renewal-era caps are 120–130). At `stat/10` a
late-campaign character rolls at +10 to +13 *before* proficiency — every
DC on a sane ladder (10/15/20/25) becomes automatic and the d20 stops
mattering. Dividing by 15 and capping at +8 compresses the entire 1–130
range into the 5e bounded-accuracy band, so one DC ladder works for the
whole campaign and the die stays the dominant term:

| Base stat | 1–14 | 15–29 | 30–44 | 45–59 | 60–74 | 75–89 | 90–104 | 105–119 | 120+ |
|---|---|---|---|---|---|---|---|---|---|
| Mod | +0 | +1 | +2 | +3 | +4 | +5 | +6 | +7 | +8 |

Worst-to-best case: fresh party (stat 30, no proficiency) hits DC 15 on a
13+; endgame specialist (stat 120, maxed skill) is +13 and still needs a
17+ for a DC 30 "nearly impossible." That is exactly the 5e progression.

### Base stats only; gear grants advantage, not numbers

Read the **base** stat (Hercules script params `Str`/`Agi`/`Vit`/`Int`/
`Dex`/`Luk` — the values stat-reset scripts assign), *not*
`readparam(bStr)` totals. Rationale: totals swing ±20 with every Blessing
tick or gear swap, which makes DCs unpredictable mid-session. Instead,
relevant equipment or buffs let the DM grant **advantage** (the `adv` flag
already in the `@dm check` design) — the 5e answer to modifier stacking.
Inspiration tokens (`@dminspire`) stay the player-side advantage source.

## Proficiency = the class skill tree they already built

No proficiency picks, no DM bookkeeping: `getskilllv()` on a mapping table.
Spending job points on Hide *is* training Stealth. Starter mapping (tune at
session zero for the party's actual classes — coverage across all RO jobs
is deliberately out of scope):

| Check type | Qualifying skills (highest level wins) |
|---|---|
| Stealth | TF_HIDE, AS_CLOAKING |
| Sleight of hand | TF_STEAL |
| Perception | AC_OWL (Owl's Eye), AC_VULTURE |
| Athletics | SM_SWORD/weapon mastery lines |
| Arcana | SA_ADVANCEDBOOK, MG_SRECOVERY (any caster mastery) |
| Medicine | AL_HEAL, NV_FIRSTAID (everyone qualifies at +1) |
| Social (persuade/intimidate/deceive) | see the dedicated section below |
| Lore (monster knowledge) | stat + skill vary by mob race — see below |

Fallback: no qualifying skill → proficiency 0, stat-only roll. Every check
stays possible for everyone; specialists get +1..+5.

## Social checks — stat by approach (no CHA needed)

RO has no charisma stat, so social checks don't get one fixed ability.
Instead the DM picks the stat from **how the player plays the attempt**
(the 5e variant rule of remixing skills and abilities):

| Approach | Stat |
|---|---|
| Argue with logic, evidence, lore | INT |
| Charm, flatter, fast-talk, bluff | LUK (fortune favors the likeable) |
| Intimidate with physical presence | STR |
| Stone-faced stubbornness, endure a negotiation | VIT |
| Haggle a price | INT or LUK, player's pick |

This is self-balancing: every class has a social angle that matches its
fantasy, nobody is locked out of social scenes, and players must describe
*how* they persuade before dice come out. Per-class flavor + qualifying
proficiency skills (classic classes; tune to the party at session zero):

| Class line | Social angle | Stat | Qualifying skill (proficiency) |
|---|---|---|---|
| Swordsman/Knight | Goad, challenge, loom | STR | SM_PROVOKE (Provoke — literally trained goading) |
| Crusader | Righteous conviction | VIT | CR_TRUST (Faith) |
| Mage/Wizard | Overwhelm with theory | INT | — stat-only |
| Sage | Lecture, cite sources | INT | SA_ADVANCEDBOOK |
| Archer/Hunter | Plainspoken frontier honesty | VIT or LUK | — stat-only |
| Bard/Dancer | Perform, charm a crowd | LUK | BA_MUSICALLESSON / DC_DANCINGLESSON |
| Acolyte/Priest | Pastoral comfort, appeal to faith | INT or VIT | AL_ANGELUS (inspiring hymn) |
| Monk | Silent, unnerving composure | VIT | — stat-only |
| Merchant/Blacksmith/Alchemist | Haggle, talk shop | INT or LUK | MC_DISCOUNT / MC_OVERCHARGE (trained haggling); MC_LOUD (Crazy Uproar) for bellowed intimidation |
| Thief/Assassin | Fast-talk, veiled threat | LUK | — stat-only (reputation is the DM's to grant as advantage) |
| Rogue | Streetwise menace, scam artistry | LUK | RG_GANGSTER (Gangster's Paradise → intimidation), RG_COMPULSION (Compulsion Discount → haggling) |

Ambiguous or unroleplayed attempts default to **LUK**.

## Lore checks — stat by mob race (monster identification)

`@dm check party lore <mob_id>` drives the bestiary's tiered reveal
(tier table in `bestiary-journal.md`: Identity at DC, Combat at +5,
Scholar at +10/nat 20; DC from mob level + boss bump). Like social
checks, the stat — and the qualifying proficiency skill — depends on the
*subject*, giving every class a knowledge niche:

| Mob race | Framing | Stat | Qualifying skill |
|---|---|---|---|
| Undead, Demon | Holy scholarship | INT | AL_DEMONBANE (Priest/Crusader line) |
| Brute, Insect, Fish | Field craft | DEX or INT | HT_BEASTBANE (Hunter) |
| Formless, constructs | Arcana | INT | SA_ADVANCEDBOOK (Sage) |
| Plant | Herbalism | INT | AM_LEARNINGPOTION (Alchemist) |
| Demi-human, Angel | Worldliness | INT or LUK | — stat-only |
| Dragon | Legend-lore | INT | — stat-only (DM may grant advantage for relevant story beats) |

Fallback INT, stat-only. Same nat-1 misinformation toggle as the
bestiary spec — wrong fact rendered client-side, never stored.

## Mechanical consequences (the payoff)

On success, `dm_checks.txt` applies the engine state matching the fiction,
using the same hooks as E7.14:

| Check passed | Engine effect |
|---|---|
| Stealth | `sc_start SC_HIDE` (or SC_CLOAKING for move-while-hidden at high margin) for `10s + 5s × margin`; works on non-Thief classes — "you press into the shadows" |
| Perception | `enablenpc` the hidden warp/NPC/trap marker (mechanics-and-quests §8 secret-passage pattern) |
| Athletics/Strength | `enablenpc` blocked-door warp; or `unitwalk` an obstacle mob aside |
| Lockpicking (Sleight) | same enablenpc pattern; on nat 1, fire the trap's `OnTouch` |

Failure consequences stay DM narration (plus the trap case above). Group
checks: `@dm check party <type> <DC>` already rolls everyone — pass if
half the party passes (5e group-check rule), echo per-player results for
the dice cards.

## Implementation notes

- Server: all of this lives in the planned `dm_checks.txt` (same file as
  `@dminitiative`, see `initiative-encounter-panel.md`). Mapping table as
  script arrays; `[DMJ]{"t":"check","who":…,"roll":…,"mod":…,"prof":…,
  "dc":…,"ok":…,"nat":…}` echo feeds dice cards (E7.2) unchanged.
- Client: no new windows. The E7.6 check console gains a per-player derived
  modifier readout (stat mod + proficiency source, e.g. "Wynne — Stealth +8
  (AGI 92 → +6, Hide 3 → +2)" — compute live from the party data the DM
  can already see); dialogue-embedded checks
  (modern-mechanics §2) consume the same formula server-side.
- `@dm profcheck <player>` debug command: prints every check type's derived
  modifier for one character — the session-zero tuning tool.

## Open questions (decide at session zero)

- ~~Persuasion/social stat~~ — **resolved 2026-07-15**: stat-by-approach
  (section above); LUK default. A per-character `dm_social_stat` var stays
  as a fallback only if approach-picking proves too fiddly at the table.
- Should nat 20 upgrade the mechanical effect (SC_CLOAKING instead of
  SC_HIDE, longer duration) rather than just flavor? (Leaning yes.)
- Advantage stacking rule when both gear and inspiration apply (5e: they
  don't stack — recommend adopting that).

**MVP first**: formula change + mapping table for the party's actual
classes + stealth/perception consequences only. Athletics/lockpicking
consequences and the console readout come after the first session proves
the numbers.
