# DM Interface — Native Tabletop Tooling

| | |
|---|---|
| **Status** | Feature design |
| **Architecture** | [SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) |
| **Feature roadmap** | [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) |
| **Work plan** | [PROJECT_PLAN.md](PROJECT_PLAN.md) E7 |
| **Implementation guide** | [DM_CLIENT_IMPLEMENTATION.md](DM_CLIENT_IMPLEMENTATION.md) — code layout, Phase A transport, state, parser, visuals, first slices |

This document owns the custom DM/player tabletop interface for the Seal Cascade
campaign engine. The base client architecture and protocol strategy stay in
[SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md).

## 9. DM Interface — native tabletop tooling  ⭐ *flagship custom feature*

### 9.1 Context — what the server already has
Hercules_RO ships a complete DM-run campaign engine (`npc/custom/dm_campaign/`,
"Seal Cascade": 19 arcs / 4 acts, documented in `CAMPAIGN.md`). It is driven
entirely through **chat-bound `@commands`** and NPC-style `select()` menus, with
output as `dispbottom`/announce text. The full server-side surface:

| Domain | Commands (script file) | Mechanics |
|---|---|---|
| Session | `@dm mode/status/reset/cleanup` (`dm_console.txt`) | Party-locked DnD mode, `$dm_active_party`, state lifetimes table |
| Story | `@dmbeat`, `@dmstory`, `@dmdecide`, `@dmflag`, `@dmquest` (`dm_beats/decisions/flags/quests.txt`) | Beat director (numbered beats per act), exclusive-choice decision registry, cross-arc flag dependency map, quest start/complete |
| Dice | `@dm check <player\|party> <stat> <DC> [adv\|dis]`, `@roll` (players, GM level 0), `@dminspire`, `@dminitiative` (`dm_checks.txt`) | d20 + stat/10 vs DC; nat-20/nat-1; advantage/disadvantage; inspiration tokens; AGI-sorted party initiative |
| Combat | `@dmencounter`, `@dmscale`, `@dmbloodied` (`dm_combat.txt`) | Encounter registry, %-based MVP stat scaling, bloodied watcher |
| Life/death | `@dmdown`, `@dmrevive`, `@dmdownrule` (`dm_downed.txt`) | Downed/death-save rules |
| Environment | `@dmhazard`, `@dmsymptom`, `@dmtrap` (+`dm_puzzles.txt` APIs) | Pulsing map hazards, symptom events, traps, sequential puzzles & riddles |
| Scene | `@dmscene`, `@dmcutscene`, `@dmspotlight`, `@dmsay`, `@dmsecret` (`dm_scene/voice.txt`) | Cutscene blocking, spotlighting, NPC-voice narration, secret whispers |
| Rewards | `@dmreward`, `@dm exp challenge <minor\|standard\|major>`, `@dmexprate`, `@dmpoints`, `@dmlevels` (`dm_rewards.txt`) | Arc-scaled loot/EXP presets for non-combat play |
| Logistics | `@dmwarp`, `@dmrecall`, `@dminstance` | Warp-to-NPC, party recall, private instances |
| Log | `@dmlog`, `@dmrecap` (`dm_session_log.txt`) | Session journal & recap |

Supporting assets: `planning/` (dm-live-table.md, mvp-party-balance-checker.md,
dm-tooling.md, campaign_quest_journal_entries.lua), `tools/campaign_quest_merge.py`
(merges campaign quests into the client's `OngoingQuestInfoList` lua),
`tools/campaign-preflight.sh`, `tools/verify_boss_levels.py`.

### 9.2 Problem
Running a session today means the DM types ~30 different chat commands from
memory and reads results as flat chat text, juggling `CAMPAIGN.md` and the
planning docs on a second screen. Players experience dice rolls, downed states,
and initiative as chat spam. A custom client can turn this into real tooling.

### 9.3 Design — two personas, one transport

**Transport (Phase A — zero server-protocol changes):** DM windows are
*command generators*: every button emits the corresponding `@dm…` chat message
through the existing chat packet path, and a response parser consumes the
server's text replies. To make parsing robust, add an opt-in structured echo to
the server scripts: when `$dm_client_mode` is on, emit a parallel
machine-readable line (e.g. `[DMJ]{"t":"check","who":"Wynne","roll":17,…}`)
that the client consumes and suppresses from chat. Script-only change, no
packets touched.

**Transport (Phase B — structured):** dedicated custom packets in
`ragnarok-packets` + a Hercules plugin for high-frequency state (initiative
order, hazard ticks, encounter HP) once Phase A proves the UX.

**DM-side windows** (visible only when the account's GM level ≥ `DM_Config.gm_level`):
1. **Campaign board** — acts → arcs → beats tree (static data imported from
   `CAMPAIGN.md`/planning lua at build time); click a beat → `@dmbeat` sequence;
   shows started/complete state from `@dm status` / `@dmquest` output.
2. **Decision ledger** — `@dm decide status` rendered as the branch table incl.
   cross-arc callback warnings (from the §9.1 flag dependency map).
3. **Check console** — stat/DC/adv-dis pickers per player or party → `@dm check`;
   results rendered as dice cards; inspiration token counters (`@dminspire`).
4. **Initiative tracker** — `@dminitiative` result as an ordered, reorderable
   turn list overlay.
5. **Encounter panel** — spawn palette (arc boss + stand-ins with mob IDs from
   CAMPAIGN.md), `@dmscale` slider with the balance-checker starting
   percentages, bloodied indicator, manual-completion fallback button.
6. **Hazard & trap board** — active pulses, `@dmhazard`/`@dmtrap`/`@dmsymptom`
   toggles with map-coordinate pickers.
7. **Scene director** — `@dmscene`/`@dmcutscene`/`@dmspotlight` controls plus a
   narration composer for `@dmsay` (NPC voice) / `@dmsecret` (targeted whisper).
8. **Rewards drawer** — challenge-EXP presets (minor/standard/major), arc-level
   loot via `@dmreward`, exp-rate and points controls.
9. **Session HUD** — `@dm mode` toggle, active party, state-lifetime warnings
   after restart/relog, `@dmrecap` viewer, warp/recall shortcuts.

**Player-side UI** (all players, no GM level):
1. **Dice cards** — `@roll` and DM-initiated checks render as animated d20
   result cards (roll, modifier, DC, pass/fail, nat-20/nat-1 flair) instead of
   chat lines.
2. **Initiative bar** — current turn order + whose turn it is.
3. **Downed overlay** — death-save state, revive prompts.
4. **Inspiration indicator** — token count near the hotbar.
5. **Campaign journal** — native quest-log view of the campaign quests (IDs
   20000–20234) using the same data `tools/campaign_quest_merge.py` maintains;
   supersedes the official client's `OngoingQuestInfoList` lua pipeline.
6. **Hazard telegraphs** — pulse warnings drawn in-world at the hazard center
   (client already renders skill units; reuse that path in Phase B).

### 9.4 Implementation notes
- Client code lives in a dedicated module (`korangar/src/interface/windows/dm/…`,
  `korangar/src/dm/` for state + parser) to stay rebaseable on upstream.
- Static campaign data (arc/beat/mob tables) is generated from the Hercules_RO
  repo at build time by a small tool — single source of truth stays server-side.
- Server gating is authoritative: the client UI is convenience; every command is
  still permission-checked by `DM_RequireDM` on the server.
- The `@roll` command is already bound at GM level 0 — player dice cards work
  from day one of Phase A.
- Sequencing: player-side dice cards + journal first (works during any play),
  DM console second (needs a real session to validate), Phase B packets last.
