# Korangar Client Systems Overview

This document maps the major systems in the Korangar client (the base for this fork). It serves as a "table of contents" for deep dives, helping us understand the architecture to improve it—especially for the Seal Cascade DM/campaign features (tabletop tools, in-world visuals, custom UI, while staying rebaseable).

**Project context (from CLAUDE.md, DM_INTERFACE.md, etc.):**
- Base: Upstream Korangar (wgpu renderer, retained UI via korangar-interface, packet-by-deserialization networking).
- This fork: Custom DM/player UI for friends-group D&D campaign. Prioritize DM tools (dice, initiative, hazards, encounters, journal) over generic RO features.
- Keep custom code isolated (e.g., `interface/windows/dm/`, `dm/` state) for easy rebases.
- No upstream IP; use official client data via GRFs + System/ (see RO_OFFICIAL_CLIENT_STRUCTURE.md).
- Packet strategy: 20190605 base + length fallbacks; model packets only when contents are needed for UI/events.
- Graphics: Modern real-time lighting/shadows (see GRAPHICS_PIPELINE.md).

**Current documentation state:**
- High-level: `SOFTWARE_DESIGN.md` (crates, state machine, protocol, assets).
- Graphics: `GRAPHICS_PIPELINE.md` (passes, lighting/shadows deep dive).
- Official client data/assets: `RO_OFFICIAL_CLIENT_STRUCTURE.md` (GRFs, System/ Lua, maps, sprites, etc.).
- DM design: `DM_INTERFACE.md`, `FEATURE_ROADMAP.md`, `PROJECT_PLAN.md` (E7 tasks).
- Other: Scattered notes in plans/, CLAUDE.md, handover docs.
- Code is the ultimate source (well-structured but needs more prose maps for onboarding/improvement).

**What to map in depth (prioritized for this project):**
We need maps that answer:
- Data flow (how packets/data → state → UI/render).
- Extension points (where to hook DM features without breaking base).
- Relation to official client (what data/behavior to replicate or improve).
- Gotchas (e.g., rebase risks, GL/WSL limits, packet desyncs).
- Improvement paths (performance, modern UI, DM-specific).

Priorities (based on DM focus + common pain points like packets, maps, UI):
1. **World / Maps / Entities / Effects** (map display, in-world DM tools).
2. **Interface / Windows / UI System** (custom DM windows).
3. **Networking / Events / Packets** (data availability for UI/DM).
4. **State Management** (reactive ClientState for UI/DM state).
5. **Loaders / Library / Assets** (data access for customs).
6. Others (Input/Camera, Renderer details, Settings, overall loop).

Below is an audit of major systems with current state, key files/entry points, and what a deep map should cover. Use this to decide next docs (e.g., `WORLD_MAPS.md`, `INTERFACE_UI.md`).

## 1. Graphics / Rendering (Well-started)
**Purpose:** Multi-pass forward renderer with real-time lighting, shadows (SDSM + point), tiled light culling, effects, map/ground/models/entities.
**Why important for DM:** In-world visuals (hazard telegraphs, markers, effects for scenes/cutscenes, entity rendering for initiative/downed states).
**Current docs:** `GRAPHICS_PIPELINE.md` (high-level passes, bind groups, deep-dive on lighting/shadows, uniforms, night maps, improvement paths). GL backend quirks are covered in the root `CLAUDE.md`.
**Key files:**
- `src/graphics/` (engine.rs, mod.rs, passes/ for light_culling/forward/directional_shadow/point_shadow/sdsm/postprocessing, instruction.rs).
- `src/lib.rs` (render_geometry, render passes dispatch, uniforms).
- `src/world/` (map/, light/, effect/, particles/, cameras/).
- Shaders: `shaders/passes/` + `modules/` (Slang).
- `src/renderer/` (game_interface.rs, interface.rs, effect.rs).
**What a deep map should include (beyond current doc):**
- Full pass pipeline with data flow diagrams (prepare/upload/dispatch).
- Exact bind group layouts and resource management.
- Entity/ground/model rendering details (culling with KDTree/frustum, animation integration).
- Effects/particles system (how quest effects, skill units, overheads work).
- Camera integration (player/debug/start cameras, projection).
- Debug/visualization tools and how they hook in.
- Relation to official client (how RO map data + effects map to this).
- Extension for DM: Adding custom in-world markers/telegraphs without breaking performance.
- Gotchas: GL limitations, bindless support, MSAA, performance with many lights/entities.
**Improvement focus:** Lighting (as discussed), effect quality, map object placement fidelity, DM-specific visuals (e.g., initiative bars in-world?).

## 2. World / Maps / Entities (High priority for map display + DM)
**Purpose:** Loads and simulates the game world: maps (ground + objects), entities (player/NPC/monster with sprites/animations), effects/particles, pathing, collision, objects/lights/sounds.
**Why critical:** "Map display" is here. For DM: Hazard telegraphs (reuse skill units?), entity states (downed, initiative), quest effects/markers, in-world narration/spotlights.
**Current docs:** Partial in RO_OFFICIAL_CLIENT_STRUCTURE.md (map file formats from official client: .rsw/.gnd/.gat, resources). Some in SOFTWARE_DESIGN.md (loaders). GRAPHICS_PIPELINE covers rendering side. No full runtime map.
**Key files:**
- `src/world/` (map/mod.rs + lighting.rs + vertices.rs + water_plane.rs; entity/mod.rs; effect/mod.rs; particles/mod.rs; object/mod.rs; model/; animation/; cameras/; pathing.rs; light/mod.rs; sound/; ground_item.rs; library/ for identities).
- `src/loaders/map/`, `loaders/model/`, `loaders/sprite/`, `loaders/effect/`, `loaders/animation/`.
- `src/lib.rs` (AddEntity, map loading, quest effects).
- `src/graphics/passes/` (forward for models/entities, picker for interaction).
**Data flow:** GRF map files → loaders → Map struct (ground tiles, objects via KDTree) → entities (Common + type-specific: Player/Npc/Monster) → rendering + simulation (pathing, animation_state, effects).
**What a deep map should include:**
- Map loading pipeline (RSW for resources/objects/lights/effects/sounds, GND for ground mesh, GAT for collision/height/picking). How offsets are applied.
- Entity system: AddEntity handling, Npc/Player/Monster creation (job_id → sprite via library), Common (position, animation, fade), quest effects (AddQuestEffectPacket → particle icons).
- Animation: How .act/.spr drive states (idle/walk/attack/die), integration with entities.
- Effects/Particles: Quest icons (quest_*.bmp markers with !/?), skill units, overheads, how they attach to entities/positions.
- Pathing/Collision/Picking: KDTree for objects, GAT for tiles/walkability, frustum culling.
- Cameras: Player (orbital?), debug, start; transitions.
- Relation to official: How RO map data (from H:\RO GRFs) populates this; quest effects/markers from client data.
- DM extensions: In-world hazard rendering (reuse effect paths), entity states for downed/initiative, map pings/markers.
- Gotchas: Missing sprites fallback, entity culling hiding markers, map change clearing.
**Improvement focus:** Better quest/instruction markers (as in your recent issue), DM in-world visuals, entity fidelity for campaign (e.g., custom monster sizes).

## 3. Interface / Windows / UI Framework (High priority for custom DM UI)
**Purpose:** Retained-mode UI built on korangar-interface crate. Windows, components (buttons, text, item/skill boxes), state binding (RustState/StateElement paths), themes, input handling for UI.
**Why critical:** DM_INTERFACE.md calls for many custom windows (campaign board, decision ledger, check console, initiative tracker, encounter panel, hazard board, scene director, rewards, session HUD, player dice cards, journal, etc.). Existing windows (chat, dialog, inventory, skill_tree, hotbar, status_bar, maps) are base to extend.
**Current docs:** `KORANGAR_INTERFACE_INTERNALS.md` (deep dive on layout resolver, Element trait, macros, stores, themes). Plus `UI_FRAMEWORK_EXTENSION.md` (practical patterns), DM_CLIENT_IMPLEMENTATION.md, and specs.
**Key files:**
- `src/interface/` (mod.rs, windows/ (many .rs + skill_tree/ sub), components/ (item_box, skill_box), cursor/, resource.rs).
- `src/state/` (for paths like client_state().hotbar(), status_effects()).
- `src/lib.rs` (interface.open_window, event handling for UI).
- `korangar-interface/` crate (windows, elements, layout, theme; CustomWindow trait, window! macro).
- Themes in `state/theme/`.
**Data flow:** ClientState (reactive) → window constructors (e.g., StatusBarWindow::new(path)) → to_window() building with korangar-interface macros → layout/render via GameInterfaceRenderer.
**What a deep map should include:**
- UI framework basics (retained vs immediate, state paths for reactivity, themes, input modes).
- Existing windows: Structure (e.g., dialog.rs for NPC, chat.rs, inventory/equipment/stats/skill_tree for core, hotbar, status_bar (recently added for buffs), maps window).
- Components: How item/skill boxes work (drag-drop, tooltips), buttons, text, etc.
- How to add DM windows: Isolation in dm/, state in ClientState, event queue for @dm commands.
- Dialog/NPC system (how text/choices flow from packets).
- Relation to official: Replicating or improving official UI (e.g., modern inventory vs bare grid).
- DM specifics: Command palette for @dm, structured chat for [DMJ], custom HUD elements (dice cards, initiative bar).
- Gotchas: Window classes for closing, focus, debug features.
**Improvement focus:** Build DM console/windows (E7), modernize core windows (inventory, dialog per roadmap), HUD edit mode.

## 4. Networking / Events / Packets (Critical priority)
**Purpose:** Connects to servers, receives packets → NetworkEvents → state/UI updates. Framing, version dispatch, fallbacks.
**Why critical:** "Framing is now automatic" via length fallbacks (from Hercules tables). Many features blocked on packets (party, quests, effects, chat). DM tools rely on chat + quest packets; future custom packets.
**Current docs:** `docs/PACKET_EVENTS_CATALOG.md` (full NetworkEvent catalogue, producing packets, registration, handler machinery, DM chat/quest flows, verbatim structs and dispatch). Also SOFTWARE_DESIGN.md (§5), plans/packet-gap-*.md, protocol/ notes.
**Key files:**
- `src/networking/mod.rs`
- `korangar-networking/` (lib.rs, event.rs, packet_versions/version_20220406.rs + lengths_*.rs, handler).
- `ragnarok-packets/` (definitions, QuestEffectPacket, etc.).
- `src/lib.rs` (network event handling loop, register packets).
**Data flow:** Network packets → PacketHandler → NetworkEvent (AddEntity, AddQuestEffect, ChatMessage, etc.) → match in lib.rs → mutate ClientState or UI.
**What a deep map should include:**
- See the complete, up-to-date reference in `docs/PACKET_EVENTS_CATALOG.md`.
- Packet registration (register_*, register_noop, length fallbacks).
- Key events and their handlers (AddEntity → world, QuestEffect → particles, chat for DM).
- Version specifics (20190605 base).
- Relation to official + Hercules (what packets server sends for DM/commands/quests).
- Gaps (from FEATURE_ROADMAP: status/buffs, damage, party, etc.).
- DM: How @commands flow (chat packets), structured echoes. Full implementation bridge in `docs/DM_CLIENT_IMPLEMENTATION.md`.
- Gotchas: Desyncs, unknown packets, reconnect.
**Improvement focus:** Promote noops to handlers for DM-needed data (quests, effects, chat). Custom packets for Phase B DM state.

## 5. State Management (ClientState)
**Purpose:** Central reactive state (hotbar, inventory, skills, entities list, chat, dialogs, themes, localization, status_effects, DM state).
**Why important:** UI binds to paths (e.g., client_state().status_effects()); DM state (initiative, flags, downed) lives here.
**Current docs:** `STATE_MANAGEMENT_GUIDE.md` (full reactivity explanation + how to add state). Also status_effects.rs as example, DM_CLIENT_IMPLEMENTATION.md for DM state pattern.
**Key files:** `src/state/mod.rs` + sub (hotbar.rs, inventory.rs, skills.rs, status_effects.rs (recent), localization/, theme/, character_slots.rs), ClientStatePathExt.
**Data flow:** NetworkEvents + input → mutate state → UI re-renders via StateElement.
**What a deep map should include:**
- Full ClientState structure.
- How paths work (RustState derive for reactivity).
- Existing state (e.g., status_effects for buffs as template).
- Adding DM state (isolated).
- Relation to UI binding.
**Improvement focus:** DM state modules, reactive journal/initiative.

## 6. Loaders / Library / Assets
**Purpose:** Load from GRFs + overrides (sprites, models, textures, effects, animations, fonts, maps); Library for queryable data (item names, skill info, job/npc identities).
**Why important:** Powers everything; custom DM data (quests, items) via System/ + merges.
**Current docs:** plans/asset-pipeline.md + RO_OFFICIAL_CLIENT_STRUCTURE.md. `STATE_MANAGEMENT_GUIDE.md` and `UI_FRAMEWORK_EXTENSION.md` cover related state/UI binding patterns.
**Key files:** `src/loaders/` (mod.rs, async/, gamefile/, map/, sprite/, model/, effect/, animation/, texture/, font/, server/client_info.rs), `src/world/library/` (mod.rs + item_*, skill_*, job_identity.rs, etc.).
**What a deep map should include:**
- GRF + folder + 7z loading.
- Async request system.
- Library tables (how Lua is executed for data).
- Relation to official data (itemInfo_EN.lua, npcidentity.lub, etc.).
- DM: Syncing customs.
**Improvement focus:** Better English data loading, custom asset packing.

## 7. Input / Camera / Controls
**Purpose:** Input events/modes, cameras (player orbital, debug, start), pathing, picking (world + UI).
**Why important:** DM free-cam, better controls, map pings, assist targeting.
**Current docs:** `INPUT_CAMERA_SYSTEMS.md` (full internals of InputSystem, MouseInputMode, Camera trait, PlayerCamera, picking, pathing + guidance for WASD/gamepad). Also `plans/modern-mechanics.md` for future designs.
**Key files:** `src/input/`, `src/world/cameras/`, `src/lib.rs` (input handling).
**Map needs:** Input modes, camera transitions, picking for DM markers.

## 8. Other / Supporting
- **Renderer** (`src/renderer/`): Game vs interface rendering separation.
- **Settings** (`src/settings/`): Per-category (graphic has lighting/shadows), persistence.
- **Overall Loop** (`src/lib.rs` + main.rs): State machine, frame update/render, network polling.
- **Audio/Video**: Lower priority.
- **Debug tools**: Packet inspector, profilers, inspectors (huge for diagnosis).

## Recommended Documentation Roadmap (Status)
We have made significant progress on the deep dives. Current status (after filling the lighter areas):

1. **High-level map**: This file + `SOFTWARE_DESIGN.md`.
2. **Deep dives** (completed or strong):
   - `WORLD_MAPS_ENTITIES.md`
   - `GRAPHICS_PIPELINE.md`
   - `PACKET_EVENTS_CATALOG.md`
   - `LOADERS_ASSET_PIPELINE_INTERNALS.md`
   - `INPUT_CAMERA_SYSTEMS.md`
   - `KORANGAR_INTERFACE_INTERNALS.md`
   - `MAIN_LOOP_RENDERER_PERFORMANCE.md`
   - `STATE_MANAGEMENT_GUIDE.md`
   - `UI_FRAMEWORK_EXTENSION.md`
   - `DEBUG_AUDIO_VIDEO_SUBSYSTEMS.md`
3. **DM-specific**: `DM_CLIENT_IMPLEMENTATION.md` + specs (excellent coverage).
4. **Improvement guides**: Strong across the board. Most deep dives now include concrete extension points, gotchas, and code patterns.

All previously "still lighter" areas have dedicated deep-dive documentation.

**Next steps?**
- DM server functions are now mapped in `docs/DM_SERVER_FUNCTIONS.md` (primary request completed, with command list, script modules, state vars, client integration, and ties to DM_INTERFACE.md).
- Create one of the high-priority deep dives (e.g., start `docs/WORLD_MAPS_ENTITIES.md` with map loading, entity/quest effects, and DM visuals/hazards/telegraphs)?
- Or create a quick "packet events catalog" (mapping DM commands + responses to chat/quest packets) or "UI window extension guide" first?
- Or audit a specific area with more code inspection (e.g., current Korangar quest effect or chat handling for @dm emulation)?
- Focus on DM client integration (generating commands from planned DM windows, parsing [DMJ] echoes, etc.)?

This gives the full picture for improving Korangar against the real server mechanics. Let me know the priority!