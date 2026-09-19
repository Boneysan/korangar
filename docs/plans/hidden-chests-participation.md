# Hidden chest participation (QW-090)

**Decision (2026-09-15):** keep **per-character** discoveries. No party-share of opened chests.

## Inventory

- **Act I:** 38 chests in five regions (Prontera, Geffen, Morroc, Payon, Alberta/Izlude). `DM_TreasureFound` grants field notes, Marks, and a regional hat when a region is complete. Discovery is stored on the character (`DM_NOTES_*`).
- **Outside Act I:** 108 stock achievement chests. `DM_TreasureFound` returns 0 for those ids; they stay the stock achievement hide-on-complete behavior.

No implementation change. A later task is needed only if the user picks shared/party participation.
