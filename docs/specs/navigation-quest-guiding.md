# Navigation and quest guiding — GDD §8.4 / next slices 5–6

**Parent:** [GDD](../GDD.md) §§8–9 and [next-slices plan](../plans/gdd-next-slices.md). This replaces the older ribbon-first proposal. Guidance is a selected destination, a full map-to-map itinerary, a regional atlas, and a sampled walkable breadcrumb trail on the current minimap. The client never moves or teleports the player automatically. Quest tracker, party pings, portal labels, and population regions can consume the same route data later.

## Current hooks

- `src/interface/windows/dialog.rs` preserves dialog text and now parses valid short/full `<NAVI>` targets into labeled route buttons. Map names/coordinates are syntactically checked; route selection additionally requires the current map/cell to be valid or a verified graph path to the destination. Hercules `src/map/clif.c` emits `map,x,y` followed by optional mode/service/window/monster fields; mode/service behavior is intentionally not interpreted yet.
- `src/state/minimap.rs` and `src/interface/windows/minimap.rs` draw player, party, compass, Towninfo POIs, the next route exit, and sampled walkable breadcrumb points. `src/interface/windows/maps.rs` lays out a regional atlas with selectable locations and graph-derived reachability/route highlighting.
- `Hercules/npc/re/warps/**` contains static `warp` lines. Loaded script manifests, custom warps, and travel-service NPCs must also be inspected; text in an unloaded script is not a playable route.

## Graph artifact and build step

Generate a deterministic UTF-8 JSON artifact with `schema_version`, Hercules source revision, `maps` keyed by canonical map name, and directed `edges`. Each edge has stable ID, source map/cell/activation area, destination map/cell, kind (`walk_warp`, `service`, `conditional`), source file/line, and availability (`always`, named requirement, or `unknown`). Keep one row per actual portal even if multiple portals connect the same maps. Include only routes loaded by the active renewal script manifests. Exclude WoE, disabled scripts, temporary instance names, GM warps, and inferred reverse routes. Add reviewed service edges separately, with price/prerequisite metadata; do not parse arbitrary NPC control flow as an unconditional route.

The generator validates map names against `map_index`, coordinates against the map cache where available, duplicate IDs, missing destinations, and source provenance. It emits a coverage report: playable map count, connected components, parsed/unsupported lines, and manually authored edges. A broken edge fails generation or is marked unusable; it never becomes a route. The client embeds the artifact with a schema check and shows “route unavailable” for missing/unknown targets. Regeneration is coupled to the Hercules revision used for a client pack.

## Routing and UI

For a target `(map, x, y)`, run a directed shortest-path search over usable map edges. Begin with hop count; prefer a reachable exit cell on the current map as the tie-breaker. The graph search does not claim to know intra-map travel time. The atlas shows authored regional placement and only draws a connection when the generated graph can route between its endpoints. On the current map, use walkable-path data to draw a bounded set of breadcrumb points from the player to the next exit; recalculate at every map transition. On the destination map the target gets a cell or broad area marker. If the expected warp fails or the player goes elsewhere, recalculate from the actual server map. A route never initiates movement or teleport by itself.

Parse `<NAVI>label<INFO>map,x,y[,mode,services,show_window,monster_id]</INFO></NAVI>` into a typed target, preserving the human label. Invalid coordinates, unknown maps, or destinations without a verified directed graph route leave readable dialog text and do not replace the active route. On click, set one active target; same-map targets show the marker immediately, cross-map targets show the next exit. A player can clear/replace guidance without losing the dialog text. Live focus order, route accuracy, and guidance disable controls remain acceptance work.

Quest objectives use the same target type only when the server or curated quest data supplies a location. The client consumes Hercules hunting notifications/progress and routes identified monster objectives only to loaded static spawn-map regions also present in the navigation graph; these are broad regions, not exact cells. Item turn-in requirements link to the matching Guide item/card record and can route to graph-known maps for verified exported drop-source monsters. Do not invent a precise destination from a quest title or inferred NPC location. Other non-hunt/story objectives and NPC locations remain unlinked until explicit server/curated location data exists.

## Acceptance fixtures

1. Regenerate twice and compare byte-identical output; edit a loaded warp and detect drift.
2. Route Prontera to `prt_fild08` and Izlude to one accessible dungeon. Check that each suggested exit exists at the generated cell on the live 20220406 server. `navigation-warp-traversal` proves the Prontera route's live server traversal in both directions. `navigation-izlude-ferry-service` now follows the real Izlude Sailor menu, exact 150 zeny fare, transfer to `izlu2dun`, and the walk-cache-derived route into `iz_dun00`. The ferry is an explicitly authored conditional service edge with reviewed script provenance; NPC/service hops are distinct from walk warps and display their action/requirement.
3. Reject an unreachable map, unloaded/WoE route, temporary instance, and malformed NAVI tag without a false arrow.
4. Click a same-map and a cross-map NAVI link; warp normally, deviate once, relog once, and confirm the displayed next exit follows actual location.
5. Disable guidance, resize UI, and verify text remains readable and clickable state is apparent by more than color.

The [next-slices plan](../plans/gdd-next-slices.md) holds ownership and sequencing. The atlas now surfaces account visit/sync state and selected-destination route details (outgoing connections, hop count, next portal/service cell and action). The graph exporter parses multiword static warp names and merges validated authored services; it currently reports 538 maps and 3,185 directed edges (3,183 static walk warps and two reviewed Izlude/Byalan Sailor services), with no unsupported static warp rows. Remaining GDD §9.10 work includes broader conditional/scripted transport coverage, population shading, portal labels, and personal/party waypoints on this graph.
