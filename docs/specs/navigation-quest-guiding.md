# Targeted Spec — Navigation & Quest Guiding System

**Parents**: FEATURE_ROADMAP.md (Navigation & quests section), CLIENT_SYSTEMS_OVERVIEW.md, WORLD_MAPS_ENTITIES.md (map loading + KDTree), modern-mechanics.md §7 (3D world integration), DM_CLIENT_IMPLEMENTATION (map pings/hazards).

**Scope**: Cross-map quest breadcrumbs, clickable <NAVI> links in dialog, enhanced minimap/world map with tracking, in-world glowing paths/ribbons, custom waypoints/pings (DM + party).

**Current base**:
- Basic maps window with static warp list (no dynamic pathing).
- Pathing exists (`world/pathing.rs`, collision KDTree in loaders/map).
- Minimap via maps window or in-game (limited).
- Quest effects already provide map markers.
- TilePosition / WorldPosition well modeled.
- No NAVI parsing, no multi-map graph, no 3D ribbons yet.

## Architecture

**Data**:
- Precompute or load a world graph of maps + warps (from NavigationData/ or server data + custom DM warps).
- Per-quest objectives can have target (map, x, y) from quest packets or [DMJ] or parsed dialog.

**Layers**:
1. **Path computation**: A* or similar across map graph using warp edges + intra-map pathing.
2. **Minimap / World map**: Render path segments, arrows at edges, objective radius.
3. **In-world**: Ground ribbons (using existing tile/model or particle ribbons) + 3D floating markers.
4. **Dialog integration**: Parse `<NAVI>[text]<INFO>map,x,y</INFO></NAVI>` in NpcDialogPacket / dialog text. Make clickable → set active quest target or immediate warp indicator.
5. **DM pings**: Special markers ("Danger", "Move here") via QuestEffect or new light packets; shared via party or [DMJ].

**Components**:
- New `NavigationSystem` in world or loaders.
- `QuestTracker` state (extends quest handling).
- UI: Enhanced MapsWindow + in-game minimap widget + QuestTrackerHUD.
- Rendering: Extend forward pass or post for ribbons (reuse water/ground techniques or new decal).

## Implementation Outline

1. **Graph**:
   - Load warp data (hardcode common or parse from client NavigationData + RSW?).
   - For DM: Allow dynamic addition of temporary warps/instances via commands.

2. **Pathfinding**:
   - Reuse `path_finder` for intra-map.
   - High-level: BFS/A* over map nodes connected by warps.
   - Result: sequence of (map, positions or warp points).

3. **Dialog Parsing**:
   - In dialog window or a pre-processor: Regex or custom parser for NAVI tags.
   - On click: `set_active_navigation_target(map, x, y)` → update tracker + render.

4. **HUD / Rendering**:
   - QuestTrackerHUD: List of active with progress + "Guide" button.
   - In-world: For current target, draw path using collision mesh or simplified lines. Use existing effect system or new ribbon geometry.
   - Minimap: Overlay path lines/arrows.

5. **DM Extensions**:
   - `@dm ping x y "Danger"` → QuestEffect + special particle + [DMJ] to party.
   - Free-cam (future) helps DM place pings accurately.

## Packets / Events

- Existing: Quest packets (promote more per backlog), ChangeMap, AddQuestEffect (already great for markers).
- New events if needed: `SetNavigationTarget`, `QuestObjectiveUpdate`.
- No new packets initially (use chat/[DMJ] for custom objectives).

## Phasing

- MVP: Single-map breadcrumbs + basic NAVI click in dialog + minimap marker.
- Phase 2: Full cross-map paths, ribbons, party-shared pings.
- Ties directly to campaign journal (E7.3).

## Risks

- Performance: Pathing across many maps; precompute where possible.
- Data accuracy: Client warps vs server reality (use server as truth via packets when possible).
- Visuals: Making ribbons look good without new shaders (reuse existing passes).

**See**: WORLD_MAPS_ENTITIES.md for map/object loading/KDTree reuse, GRAPHICS for decal/ground effects, DM_CLIENT for hazard telegraphs (similar spatial rendering).

This completes the navigation item in the roadmap.
