# Phase 1 progress — 2026-09-23

## Status

Phase 1 remains **partial**. This handoff first recorded code and data present in the working trees on 2026-09-23, before any build or test run.

**Re-verified 2026-09-27:** each item below exists in source. The client library suite passes (413 passed, 17 ignored). Headless live scenarios now cover graph portal traversal, the Izlude ferry-to-dungeon route, inventory split/storage/order, account discovery isolation, and DM late-join/offline replay (see [gdd-next-slices.md](gdd-next-slices.md) for per-slice evidence). GUI and visual acceptance has still not been run for any item.

## Implemented in the working trees

- **Navigation graph:** `tools/generate_navigation_graph.py` reads the active Hercules script manifests and emits `korangar/data/navigation_graph.json`. As of 2026-09-27, the artifact contains 538 maps and 3,189 directed edges: 3,183 static walk warps plus six reviewed NPC service edges (Izlude/Byalan Sailor, and the Izlude/Alberta <-> Malangdo cat fleet added 2026-09-27), each conditional service edge marked with its zeny requirement. Other dynamic or conditional transports are not represented as guaranteed routes.
- **World Map:** The in-game menu opens a centered, clickable regional atlas with 27 town and destination nodes. Reachable atlas connections are filtered through the generated graph. Selecting a node sets a route; it does not teleport the player. The atlas highlights the current route and the current/target locations where represented.
- **Per-map guidance:** The client finds a shortest-hop portal route, marks the next portal on the minimap, and computes a walkable tile path from the player to that exit. The minimap draws sampled breadcrumb points and a distinct exit marker. A map transition triggers route recalculation for the next leg. The client does not auto-walk.
- **Quest tracker preference:** Track/Untrack selections persist per character in `client/game_settings.ron` and are restored against the server's active quest list. Quest state and objective progress remain server-authoritative.
- **Inventory management (refreshed 2026-09-24):** Inventory and storage expose shared name search and server-item-type categories; inventory sorting, drag arrangement, server-persisted slot ordering, and per-character item protections are present. The client sends a dedicated split-stack packet, and Hercules validates eligibility/amount/capacity before moving quantity into a separate slot. Targeted disposable-server tests pass valid, invalid, full-capacity, partial storage round-trip, inventory ordering, and live Iron Arrow/Card item-type cases. Quest-item/favorite/recent-loot filters are explicitly deferred pending reliable metadata/history contracts; GUI visuals remain unverified.
- **DM campaign catch-up:** Hercules has party-join, quest-log, and map-load catch-up hooks. They copy stored current status for the explicitly listed campaign quests and story flags to a joining or returning character. Status catch-up does not replay past rewards. See the [party quest-sync spec](../specs/dm-party-quest-sync.md) for limits.

## Still open for Phase 1

- GUI/visual acceptance for the World Map, portal selection, walkable breadcrumbs, inventory behavior, and quest preference restoration. Builds pass, and headless live coverage exists for portal/Izlude traversal, inventory split/storage, and DM late-join catch-up.
- Expand and validate route data for travel-service NPCs and conditional transports beyond the Izlude/Byalan Sailor and the Malangdo cat fleet (both added). Account-wide visited badges and destination detail panels now exist (slices 6 and S7). Live traversal of the Malangdo hop is not yet verified, same as its Guide/atlas visual acceptance.
- Supply destinations for NPC/story quest objectives that have no explicit location data. Valid `<NAVI>` links, hunt objectives, and item turn-ins already route.
- Complete Adventure Guide data coverage for GDD §9.5 (status prose and cures, conditional skill effects, authored spawn conditions) and fresh-account live acceptance. The Guide window itself exists.
- Add status and cast detail to the monster target frame, which already shows HP and level/element/race/size.
- GUI visual/usability acceptance and broader relog/drag/drop acceptance for inventory/storage. Quest-item/favorite/recent-loot filters are deferred (see the refreshed slice 7 row in `gdd-next-slices.md`); they are not live acceptance failures.
- Add personal/shared party waypoints and party location display beyond current-map minimap markers.
- Continue Phase 1 live acceptance and resolve any failures found there.

## Relevant references

- [GDD Phase 1 and implementation audit](../GDD.md#phase-1---foundational-ux--partial--paths-in-87-910-1017)
- [Implementation plan and acceptance gates](gdd-next-slices.md)
- [Navigation and quest guiding](../specs/navigation-quest-guiding.md)
- [DM party quest synchronization](../specs/dm-party-quest-sync.md)
- [20 September playtest acceptance list](playtest-2026-09-20.md)
