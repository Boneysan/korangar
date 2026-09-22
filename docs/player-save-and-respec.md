# Player save, load, and respec

Approved 2026-09-15.

## Save / load

- **Menu:** Save here, Warp to save.
- **Chat:** `@save` / `@load` (player group 0).
- **NPC:** `Seal Cascade Checkpoint` at Prontera (151, 191).
- Death respawns at the last savepoint (`pc_respawn`).
- Invalid map: `@load` still respects `nowarpto` / `nowarp` flags.

Live: Save here then Warp to save returned to the pinned cell.

## Respec

- **Unlock** on the skill tree: queue **+** ranks or **−** refunds.
- **Lock** sends `CZ_UPGRADE_SKILLLEVEL` (`0x0112`) for queued ranks.
- **−** on a locked skill sends `@refundskill <id>` (`pc_skilldown`).
- **Reset skills** / `@resetskill` refunds the whole tree.

Live: locked ranks survived logout/login. Headless `skill-lock-relog` passed.
