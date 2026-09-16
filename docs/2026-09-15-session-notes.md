# Session notes — 2026-09-15

## Where the project got to

- Advanced the September playtest queue through hotbar editing, exact quantity
  flows, trade/drop/loot scenarios, vendor comparison foundations, campaign
  data models, recovery/rules changes, presentation models, and chest/cosmetics
  planning.
- Added focused headless scenarios for the implemented transaction and movement
  paths and recorded their evidence in the QW runbook.
- Added Hercules support for quest-location exports, campaign savepoint access,
  player skill reset/refund, playtest EXP values, expanded carrying capacity,
  and recovery/respawn behavior.
- Recorded approved progression decisions and the remaining cosmetics/chest
  decisions.

## End-of-day audit correction

A senior pass compared the plan, source references, and done conditions. The
code is not yet feature complete. Several checked cards represented isolated
models or fixtures rather than wired player-facing features. The affected tasks
were reopened and the specific gaps are recorded in
`plans/code-completeness-audit-2026-09-15.md`.

Highest-priority finding: sitting/respawn recovery currently bypasses the normal
poison/overweight suppression checks, so QW-071/QW-072 are the next code tasks.
Campaign routing/HUD, durable reconciliation, Combat chat, party colors, UI
sounds, unified equipment eligibility, and chest rendering also still require
production integration.

Validation at handoff: Hercules `make -j2` passed; hunt artifacts and Python
syntax checks passed; Korangar format/check and 445 library tests passed (17
archive-dependent tests ignored). Strict Korangar Clippy failed on 43 unused/dead
code findings, consistent with the incomplete-integration audit.

## Next

1. Correct and server-test recovery suppression/timer lifecycle (QW-071/072).
2. Finish and validate weight, respec, and EXP rule boundaries (QW-073–079).
3. Wire the campaign, presentation, and chest models into real state/UI/data
   paths (QW-048–063, QW-080–089).
4. Run QW-100, then Windows, internet, Arc I, and final rules gates.
