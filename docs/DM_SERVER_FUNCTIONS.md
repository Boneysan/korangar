# DM Server Functions Map — Hercules_RO Seal Cascade

**Source:** `/home/boneysan/GitHub/Hercules_RO/npc/custom/dm_campaign/` (and related in `npc/custom/`, `db/`, `conf/`)

**Purpose:** This is the server-side "DM console" and campaign engine for the Seal Cascade D&D campaign. All mechanics are driven by chat commands (`@dm*`, `@roll`) executed via Hercules NPC scripts. The client (Korangar) will generate these commands and consume results (text + planned structured `[DMJ]` echoes) for native UI.

**High-level architecture:**
- Main console NPC: `DM_Console` (binds `@dm`, subcommands, permission via `DM_RequireDM`).
- Shared libraries: `dm_*.txt` (included via `callfunc` or direct).
- Per-arc story: `act_XX/arc_*.txt` (use S_StartArc / turn-in patterns).
- State: Global `$dm_*` vars, per-char `dm_*` (inspiration, etc.), party-locked.
- Output: `dispbottom`, `announce`, `mes` for players; server-side logs.
- Client integration (Phase A): Client sends `@dm ...` as chat (via `GlobalMessagePacket`); all responses arrive as `ChatMessage` events (various server chat packets). See `docs/PACKET_EVENTS_CATALOG.md` for the chat family and outgoing path. Add `$dm_client_mode` + `[DMJ]JSON` echoes in scripts for machine-readable data. Visuals (hazards etc.) use `QuestEffectPacket`.
- No new packets initially; leverages existing chat + quest packets.

**Key entry points in scripts:**
- `dm_console.txt`: Command dispatcher.
- `dm_common.txt`: Config, require, party helpers, warp, join args, parse tier.
- Others: specialized consoles and registries.

## Command Console (dm_console.txt)

Binds at GM level (default 1, configurable in `DM_Config`).

```herc
bindatcmd("dm", "DM_Console::OnDM", 1, 99, 1);
bindatcmd("roll", "DM_Console::OnRoll", 0, 99, 1);  // GM0 for players
// plus @dmreward, @dmflag, @dmquest, @dmbeat, @dmstory, @dmcleanup, @dmwarp, @dmrecall, @dminstance, @dmmode, @dmhazard, @dmexp, @dmexprate, @dmpoints, @dmlevels, @dmreset, @dmstatus, @dmtrap, @dmsymptom, etc.
```

**OnDM dispatcher** (parses .@sub$ and calls sub handlers or S_*):

Main subs (from code):
- `help` → S_Help
- `reward` → S_Reward (or callfunc DM_Reward)
- `flag` → S_Flag / DM_FlagRegistry
- `quest` → S_Quest / DM_QuestRegistry
- `decide` → callfunc("DM_DecideCommand", ...)
- `beat` → callfunc("DM_BeatMenu")
- `story` / `globalstory` → DM_MapStory / DM_GlobalStory
- `spawn` / `holdspawn` / `release` / `holdclear` → encounter/spawn control
- `encounter` / `scale` / `bloodied` → DM_CombatConsole
- `cleanup` / `reset` / `status` / `mode` → session management
- `warp` / `recall` / `instance` → DM_WarpParty, etc.
- `hazard` / `symptom` / `trap` → hazard consoles
- `exp` / `exprate` / `points` / `levels` → rewards
- `novice` / `resetstat` / `resetskill` / `savepoint` → player utils

Many delegate to shared "Console" NPCs that provide menus.

**Permission:**
- `DM_Config` sets `.gm_level = 1` (Group 5 "Dungeon Master").
- `DM_IsDM(level)` / `DM_RequireDM(level)` checks `getgmlevel()`.
- Party-locked via `$dm_active_party`.

**Player-side:**
- `@roll` (bound at GM level 0).
- `@dm inspire` etc. via subcommands.

## Core Mechanics Modules (shared/*.txt)

### dm_checks.txt — Dice, Checks, Initiative, Inspiration
- `DM_StatConst(stat$)`: str→bStr, agi→bAgi, etc.
- `DM_RollCheck(stat_id, dc, mode$)`: d20 + (readparam(stat)/10) vs DC.
  - adv/dis: roll twice, keep high/low.
  - Inspiration: on adv, consume `dm_inspiration` token.
  - Nat 20/1 always pass/fail.
  - Sets temp vars `@dm_check_*` for callers.
  - Announces to map.
- `@dm check <player|party> <stat> <DC> [adv|dis]`
- `@dm inspire <player> [+n|spend|clear|set n]` or party.
- Initiative: `@dminitiative` — AGI-sorted party list.
- Used by: traps, hazards, combat, downed saves, player rolls.

### dm_beats.txt — Beat Director
- `DM_BeatMenu()` → Act I/II/III/IV → specific arc.
- `DM_BeatActN()` → arc menus.
- `DM_BeatArcXX()`: Warps party, sets flags/quests via helpers, starts hazards if applicable.
- Arcs 1-19 detailed in CAMPAIGN.md (hunting + story quests, villain choices, bosses for later arcs).
- Flags set for callbacks across arcs (e.g. `dm_arc04_cassell_unmasked`).

### dm_decisions.txt — Decision Ledger
- `DM_DecideCommand(...)` / `DM_DecisionsConsole`
- Registry of exclusive choices per arc.
- Cross-arc flag dependencies.
- `@dm decide ...`

### dm_flags.txt — Flag Registry
- `DM_FlagRegistry` / `S_Flag`
- Persistent flags: `setd`, `getd` on char or global.
- `@dm flag <name> [set|clear|check]`

### dm_quests.txt — Quest Registry
- `DM_QuestRegistry`
- Start/complete quests (IDs 20000+ for campaign).
- Ties to client `OngoingQuestInfoList`.
- `@dm quest ...`

### dm_combat.txt — Encounters
- `DM_CombatConsole`
- `@dm encounter`, `@dmscale`, `@dmbloodied`
- Spawn palette, MVP scaling, bloodied watcher.
- Hazards tied to beats.

### dm_downed.txt — Life/Death
- `@dmdown`, `@dmrevive`, `@dmdownrule`
- Death-save rules, inspiration interaction.

### dm_rewards.txt — Rewards / EXP
- `@dmreward`, `@dm exp challenge <minor|standard|major>`, `@dmexprate`, `@dmpoints`, `@dmlevels`
- Arc-scaled presets.

### Hazard Systems
- `dm_traps.txt`: `DM_TrapConsole` — `@dmtrap`
- `dm_symptoms.txt`: `DM_SymptomConsole` — `@dmsymptom`
- `dm_puzzles.txt`: Sequential puzzles/riddles.
- `dm_hunt_markers.txt`
- Pressure hazards started by specific beats (pulses with status effects).

### Scene / Narrative
- `dm_scene.txt`, `dm_voice.txt`: `@dmscene`, `@dmcutscene`, `@dmspotlight`, `@dmsay`, `@dmsecret`
- Narration, blocking, whispers.

### Session / Log
- `dm_session.txt`, `dm_session_log.txt`, `dm_console.txt` (mode/status/reset/cleanup)
- `@dmlog`, `@dmrecap`, `@dmmode`, `@dmstatus`
- Party instance, state lifetimes.

### Other
- `dm_rewards.txt`, `dm_onboarding.txt`, `dm_storyteller.txt`, `dm_instances.txt`
- `dm_common.txt`: Helpers (DM_WarpParty, DM_JoinArgs, DM_ParseTier, DM_CurrentParty, etc.)
- `dm_checks.txt` also has initiative.

## State Variables (key ones)
- `$dm_active_party`
- `$dm_client_mode` (for structured echoes)
- Per-char: `dm_inspiration`
- Flags: `dm_arcXX_*`, `dm_new_world_alliance`, etc. (see CAMPAIGN.md for full list per arc)
- Quest progress tied to official quest system (IDs 20000–20234)
- Encounter state: `@dm_enc_*` temps

## Campaign Structure (from CAMPAIGN.md)
- 19 arcs / 4 acts.
- Act I (1-5): Story-focused, no MVPs initially.
- Later acts: MVP bosses, 3-choice villain gates, escalating hazards.
- Quests: Hunting + story per arc.
- Flags for callbacks across arcs.
- Tools: `planning/`, `tools/campaign_quest_merge.py`, `verify_boss_levels.py`, etc. (on Windows side).

## Client (Korangar) Integration Points
- **Commands as generators**: DM windows emit `@dm <sub> <args>` as chat packets (see `src/networking` chat path).
- **Parse responses**: `dispbottom`/`announce` → chat log or structured parser. Add `[DMJ]{"t":"check", ...}` echoes in scripts when `$dm_client_mode`.
- **Quest data**: Use `OngoingQuestInfoList_True_EN.lub` (maintained by merge tool) for journal.
- **Visuals**: Quest effects for markers (see world/particles and quest packets). Reuse for DM hazards.
- **Packets needed**: Chat, quest list/add/update (0x0AFF etc. already partially modeled), entity effects.
- **DM-only UI**: GM level check on client (or trust server). See `DM_INTERFACE.md` for planned windows.

**Current gaps for full parity:**
- Structured echoes not yet in all scripts (add them).
- No native dice cards / tracker yet (client-side parse of rolls).
- High-frequency state (initiative order, hazard ticks) may need Phase B packets later.
- Client must suppress `[DMJ]` lines from chat when in client mode.

## How to Extend / Use for Korangar DM UI
1. Client window → format command string → send as chat (use existing chat send path).
2. Listen for responses via chat events or new `NetworkEvent`.
3. For visuals: Send quest effect packets from server scripts, or client-side render based on parsed state.
4. State sync: On map load / login, request status; use flags/quests.
5. Testing: Use Hercules `@dm` console + Korangar packet inspector.

**Source of truth for story/quests/flags:** `CAMPAIGN.md` + planning docs (synced from Windows Obsidian).

This map is derived from direct inspection of the scripts + CAMPAIGN.md. Update as scripts evolve.

For client-side mapping of these (e.g. how effects render, how to parse rolls into dice cards), see related deep dives like WORLD_MAPS_ENTITIES.md (quest effects, markers) and INTERFACE_UI_WINDOWS.md (DM windows).

See also:
- `DM_INTERFACE.md` for planned client UI.
- `PROJECT_PLAN.md` E7 for tasks.
- `RO_OFFICIAL_CLIENT_STRUCTURE.md` for quest data and effect markers from client assets.

**Next for this project:** 
- Add `[DMJ]` echoes to all relevant scripts.
- Implement client parser for rolls/checks.
- Build the DM windows that emit these commands and consume structured data. 

This gives the "functionally true north" for both server mechanics and client replication. Let me know if you want code snippets for client command generation, a specific script deep dive, or to start the WORLD_MAPS_ENTITIES.md now.