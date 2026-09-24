# Phase 1 progress — 2026-09-23

## Status

Phase 1 remains **partial**. This handoff records code and data present in the working trees as of today. No build, automated tests, server script checks, or live-client acceptance were run for this update. Treat each item as implemented in source, not as confirmed working in a session.

## Implemented in the working trees

- **Navigation graph:** `tools/generate_navigation_graph.py` reads the active Hercules script manifests and emits `korangar/data/navigation_graph.json`. The current artifact contains 537 maps and 3,176 directed static-warp edges, with source locations and coverage information. Dynamic or conditional transports are not represented as guaranteed routes.
- **World Map:** The in-game menu opens a centered, clickable regional atlas with 27 town and destination nodes. Reachable atlas connections are filtered through the generated graph. Selecting a node sets a route; it does not teleport the player. The atlas highlights the current route and the current/target locations where represented.
- **Per-map guidance:** The client finds a shortest-hop portal route, marks the next portal on the minimap, and computes a walkable tile path from the player to that exit. The minimap draws sampled breadcrumb points and a distinct exit marker. A map transition triggers route recalculation for the next leg. The client does not auto-walk.
- **Quest tracker preference:** Track/Untrack selections persist per character in `client/game_settings.ron` and are restored against the server's active quest list. Quest state and objective progress remain server-authoritative.
- **Inventory management:** Inventory and storage expose shared name search and server-item-type categories; inventory sorting, drag arrangement, server-persisted slot ordering, and per-character item protections are present. The client sends a dedicated split-stack packet, and Hercules validates eligibility/amount/capacity before moving quantity into a separate slot. Quest-item/favorite/recent-loot categories and live behavior remain unverified.
- **DM campaign catch-up:** Hercules has party-join, quest-log, and map-load catch-up hooks. They copy stored current status for the explicitly listed campaign quests and story flags to a joining or returning character. Status catch-up does not replay past rewards. See the [party quest-sync spec](../specs/dm-party-quest-sync.md) for limits.

## Still open for Phase 1

- Build and live acceptance for the World Map, portal selection, walkable breadcrumbs, inventory behavior, quest preference restoration, and late-join catch-up.
- Expand and validate route data for travel-service NPCs and conditional transports; add visited/discovered state and destination detail panels.
- Make quest objectives and Hercules `<NAVI>` dialog links supply actionable map destinations.
- Add the player Adventure Guide and data-generation coverage described by GDD §9.5.
- Add the monster target frame and related status detail.
- Live acceptance for server-authoritative stack splitting, category filtering, storage transfers, drag/drop, and relog persistence.
- Add personal/shared party waypoints and party location display beyond current-map minimap markers.
- Continue Phase 1 live acceptance and resolve any failures found there.

## Relevant references

- [GDD Phase 1 and implementation audit](../GDD.md#phase-1---foundational-ux--partial--paths-in-87-910-1017)
- [Implementation plan and acceptance gates](gdd-next-slices.md)
- [Navigation and quest guiding](../specs/navigation-quest-guiding.md)
- [DM party quest synchronization](../specs/dm-party-quest-sync.md)
- [20 September playtest acceptance list](playtest-2026-09-20.md)
