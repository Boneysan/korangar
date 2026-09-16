# Campaign journal, tracker, and routing — implementation plan

**Owners:** QW-050–053 and QW-056–063  
**Related specs:** `../specs/campaign-quest-journal.md`,
`../specs/navigation-quest-guiding.md`

## Goal

Turn the current quest-location tables and pure breadcrumb/warp models into a
player-facing journal, tracked-objective HUD, minimap guidance, and deterministic
cross-map route system. Guidance remains advisory and never walks or enters a
portal for the player.

## Sources of truth

- Quest identity/targets: Hercules `db/quest_db.conf`.
- Campaign authored meaning and reveal rules: versioned sidecar under
  `npc/custom/dm_campaign/`, referenced by quest id.
- NPC/turn-in coordinates: `tools/export_quest_locations.py` plus explicit
  authored overrides when scripts cannot be inferred safely.
- Hunt contracts: `tools/gen-hunts.py`.
- Warp edges: a new Hercules exporter over enabled warp scripts and approved
  custom/instance edge declarations.
- Runtime progress: quest and inventory packets; never infer server completion
  solely from carried items.

## Generated packs

Produce versioned, reproducible artifacts for:

1. Objective definitions: type, target ids/names, counts, source/rank, party vs
   personal semantics, optional/required/DM-triggered flags.
2. Guidance: revealed steps, NPC/object, readable area, coordinates, turn-in.
3. Warp graph: source/destination map and coordinates, one-way, level/quest
   gate, instance/dynamic/uncertain flags, readable map names.

Every generator gets `--check`, reference validation, stable sorting, and a
schema/source hash. Rust loads these through `Library`; remove embedded fixture
strings after real packs are available.

## Runtime state

Add production state to `ClientState`:

- `TrackedObjectiveState`: character-scoped quest/objective selection and
  collapse/hide/scale/opacity/position/guidance settings.
- `NavigationState`: current destination, computed route legs, graph revision,
  and reason guidance is unavailable.

Wire construction, `ClientState` initialization, selector paths, and character
reset together. Persist user presentation settings in `GameSettings`; key
tracked selection by server/account/character so characters do not leak state.

## Event flow

- Quest list/add/update/remove refreshes journal and validates tracked selection.
- Inventory add/remove/use refreshes Collect counts only.
- Party/campaign reconciliation refreshes party-state labels.
- Map load, authoritative position correction, teleport, Fly Wing, death,
  respawn, and graph revision recompute the route.
- Completion/hidden target removes stale markers immediately.

Use the existing `QuestLogState`, `CampaignQuestTable`, `MinimapState` dynamic
markers, quest log window, and map-change event loop. Do not create a second
quest log or minimap.

## UI

1. Extend `interface/windows/quest_log.rs` with typed objectives, You carry,
   source/rank, party/personal explanation, return NPC, and full readable route.
2. Add one non-intercepting breadcrumb HUD window showing the current objective,
   remaining amount, current leg, direction, and distance. Only visible controls
   capture clicks.
3. Add collapse/hide/guide/journal controls and supported scale/opacity/position.
4. Add one quest marker/current-portal marker to `MinimapState`; reuse existing
   rendering and marker expiry/removal conventions.
5. World portal highlighting is a later substep of the same route state, not a
   separate route engine.

## Route algorithm

- Validate graph coordinates and known map names at load.
- Use deterministic Dijkstra/A* with documented costs and stable map/coordinate
  tie-breaking.
- Exclude unavailable edges; penalize uncertain/gated edges and label them.
- Dynamic instance edges carry a revision and disappear atomically.
- Route output is ordered legs with source portal, destination map, and final
  objective. Never show an arrow to coordinates on a different map.

## Tests and acceptance

- Parser/generator failures, all six objective types, hidden/revealed story
  steps, personal/party counts, pickup/drop/use, quest removal.
- Route fixtures: reachable, unreachable, alternate equal cost, one-way, gated,
  dynamic instance, dangling coordinate, wrong portal and graph revision.
- State integration: reconnect, character switch, map change, late join, manual
  selection preservation, completed selection advance.
- GUI at supported laptop sizes/scales, overlapping dialogue/target/party/hotbar.
- Live Prontera → `prt_maze02`: intended route, wrong exit, teleport, death and
  respawn, with each recomputation logged.

## Done

- No `breadcrumb`, `hunt_schema`, `journal_slice`, or `warp_graph` production
  type is dead code.
- Rockers and Rumors and Omens at the Fountain meet their acceptance examples
  using generated/authored data, not UI hard-codes.
- Generators, unit/integration tests, live route pass, and strict Clippy pass.

