# Targeted Spec — ATB Structured Rounds (E7.14, future work)

**Parents**: PROJECT_PLAN.md E7.14, DM_INTERFACE.md §9.3, FEATURE_ROADMAP.md §8,
`plans/modern-mechanics.md`. **Depends on**: E7.5 (initiative tracker), E7.1
(`[DMJ]` echo).

**Status**: future work — captured 2026-07-15, build after E7.5.

**Purpose**: A DM-toggleable Active Time Battle mode for dramatic encounters.
Instead of pure real-time combat or strict D&D turns, each combatant has a
visible charge bar; when it fills you get a short free-action window while
everyone else is movement/action-locked. Real-time RO combat stays the default
— this mode is opt-in per encounter to make boss beats and story fights feel
like the table holding its breath, not an MMO grind.

## Why ATB fits this engine (the "hidden timing system")

Hercules already gates every unit action through per-unit timestamps — RO
combat *is* an ATB system without the visualization. In
`src/map/unit.h` (`struct unit_data`):

- `canact_tick` — earliest tick the unit may act (skills/attacks); set by
  cast delays and skill aftercast.
- `canmove_tick` — earliest tick the unit may walk;
  `unit_can_move()` (`src/map/unit.c:1216`) refuses movement until then, and
  `unit_set_walkdelay()` (`src/map/unit.c:1366`) is the API that pushes it
  forward (damage walk-delay uses it today).
- `attackabletime` — next auto-attack, driven by ASPD (`adelay`/`amotion`).

So "your ATB bar is full" is literally `gettick() >= canact_tick`, and the
charge rate is the same AGI/ASPD stat economy players already build around.
We are not adding a timing system; we are surfacing and orchestrating one.

### Existing enforcement hooks (no core patch needed for MVP)

| Need | Hercules mechanism | Access |
| --- | --- | --- |
| Hard-lock a player off-turn | `sd->block_action` bitfield, checked in `unit_can_move()` and the attack/skill/item paths | `setpcblock(PCBLOCK_MOVE\|PCBLOCK_ATTACK\|PCBLOCK_SKILL\|PCBLOCK_USEITEM, true)` script builtin (flags in `src/map/script.h:563`) |
| Release the active player | same | `setpcblock(..., false)` |
| Freeze mobs off-turn | `unitstop()` builtin + `SC_STOP` status (movement-block list in `unit_can_move()`), or `setunitdata(UDT_MODE, …)` to strip aggressive/canattack bits | script builtins (`src/map/script.c`) |
| Turn timer / round ticks | NPC `sleep`/`addtimer` in `dm_combat.txt` | script |
| Order + whose-turn broadcast | E7.1 `[DMJ]` echo | script |

`@dmscene` (E7.9) needs the same `setpcblock` machinery — build the freeze
helper once in `dm_common.txt` and both features consume it.

## Design

**Round flow (server-driven, script-only Phase A):**

1. DM triggers `@dminitiative atb [window_seconds]` (default 12s).
2. Script snapshots party + registered encounter mobs, computes charge order
   from AGI + d20 (same roll as the planned E7.5 list), applies
   `setpcblock` to all players and `SC_STOP` to all mobs, and emits
   `[DMJ]{"t":"atb_start","order":[…],"window":12}`.
3. Active combatant's locks are released for the window
   (`[DMJ]{"t":"atb_turn","who":…,"until":tick}`); they act in real time —
   move, attack, skill, item — then locks re-apply. Mobs get their window
   too: unfreeze + normal AI for the window, then re-freeze (this is what
   makes it ATB rather than menu combat — positioning still matters).
4. Repeat down the order; `@dminitiative stop` (or encounter completion)
   clears all blocks. **Cleanup must be unconditional** — `@dmreset`/
   `@dmcleanup` and player logout hooks must clear `PCBLOCK_*` and `SC_STOP`
   or a crash mid-round leaves the party soft-locked.

**Client (Korangar, `interface/windows/dm/` + HUD):**

- ATB strip: one charge bar per combatant, ordered; fills over the window,
  flashes on whose-turn. Consumes `atb_start`/`atb_turn`/`atb_stop` from the
  `[DMJ]` parser into `DmState` (same pattern as the E7.5 tracker —
  this widget *is* the initiative bar with a time axis).
- Off-turn feedback: dim the hotbar + a lock icon near the character, so a
  blocked click reads as "not your turn," not "the game broke."
- Dice cards (E7.2) fire on the `@dm check` results the DM calls for during
  windows — the two features compose into the full tabletop loop.

**Out of scope (deliberately):** action-point budgets, per-turn move-range
limits, strict one-action turns. RO characters are balanced around ASPD and
sustained DPS; a free-action time window preserves that economy, a turn
economy erases it.

## Phasing

- **MVP (Phase A, script + client widget):** fixed windows, party +
  encounter-registry mobs only, `setpcblock`/`SC_STOP` enforcement, `[DMJ]`
  sync, ATB strip UI. No core changes.
- **Later (Phase B):** charge rates scaled from live AGI/ASPD instead of
  fixed windows; client-side bar prediction between `[DMJ]` events; custom
  packets if script-echo latency shows at the table; DM per-combatant
  window overrides from the encounter panel (E7.8).

## Open questions (decide at build time)

- Interrupts/reactions: does the DM get a "pause the round" button (a
  Basilica-style global hold) for narration mid-round?
- Death mid-round: interaction with `@dmdown` death saves — downed players
  should probably keep their slot and roll saves in it.
- Whether mobs share one collective window per round (faster pacing, DM
  drives them) vs. individual slots (purer ATB, slower).
