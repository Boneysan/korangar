# Implementation Specs

**Parent hub**: [docs/README.md](../README.md)

This directory contains targeted, code-level implementation specifications. These are concrete guides meant to be followed when building specific features or promoting packets.

Use these alongside the higher-level plans in `../plans/`.

## Current Specs

See the main [Documentation Hub](../README.md) for the full categorized list and descriptions. All specs here are linked from the relevant roadmap and implementation documents.

- `buff-bar-slice.md` — Reference template for packet → event → state → widget promotion.
- `party-packets.md` — Party and whisper packet definitions + promotion plan.
- `dm-phase-a-chat-integration.md` — DM `[DMJ]` parser, command emitter, and integration.
- `dm-ui-window-template.md` — DM window isolation pattern and template.
- `hud-edit-mode.md` — HUD edit controls and named profiles over WindowCache.
- `navigation-quest-guiding.md` — Quest navigation graph, NAVI links, next-exit markers.
- `encyclopedia-data.md` — Adventure Guide source, schema, provenance, and knowledge policy.
- `monster-ai-pilot.md` — GDD tactical monster profiles and three pilot tiers.
- `gdd-combat-input.md` — GDD cast, selection, and input-buffer state transitions.
- `gdd-ui-wireframes.md` — GDD next-slice UI hierarchy and interaction states.
- `atb-structured-rounds.md` — Future work (E7.14): DM-toggleable ATB rounds on top of Hercules' `canact_tick`/`canmove_tick`/`setpcblock` gates.
- `campaign-quest-journal.md` — E7.3: quest_db.conf → embedded JSON pipeline, journal window + HUD tracker, consumes the wired Quest* events (last port-back row).
- `dm-party-quest-sync.md` — Character-specific story progress synchronized across enrolled DM Session party members, including safe reconnect replay.
- `bestiary-unlock-persistence.md` — Server-authoritative per-account player unlocks and login sync.
- `initiative-encounter-panel.md` — E7.5+E7.8: `@dminitiative`/`@dmencounter`/`@dmscale`/`@dmbloodied` server scripts + tracker/panel windows.
- `proficiency-checks.md` — Future work (E7.16): bounded-accuracy `@dm check` formula (handles 100+ stats), class-skill-tree proficiencies, checks with real engine consequences.

When adding a new spec:
1. Create the file here.
2. Add it to the hub in `../README.md`.
3. Link it from the relevant roadmap/plan document.
