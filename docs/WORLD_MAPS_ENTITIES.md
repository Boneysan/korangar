# World, Maps, and Entities in Korangar

This document provides a deep technical map of how Korangar handles maps, world rendering, entities, effects, particles, and quest-related visuals. It is written to support improving the client — particularly for DM/campaign features such as in-world hazard telegraphs, guide markers, entity states, and visuals driven by server-side DM functions.

It cross-references:
- Official client data formats (from `docs/RO_OFFICIAL_CLIENT_STRUCTURE.md` and the GRFs/System/ at H:\RO).
- Korangar source (loaders, world, graphics, lib.rs).
- DM server functions (from `docs/DM_SERVER_FUNCTIONS.md` — server scripts start hazards/beats that the client must visualize).
- Graphics pipeline (`docs/GRAPHICS_PIPELINE.md` — rendering passes, culling, effects).

## 1. Map Loading and Rendering (RSW / GND / GAT + Placed Objects)

Maps are the foundation of the world. Loading is driven by the map name sent by the server on warp (via `ChangeMap` event or initial login).

### Loading Pipeline
- Entry point: `NetworkEvent::ChangeMap` (or initial map load) in `korangar/src/lib.rs`.
- `MapLoader::load(resource_file, ...)` in `korangar/src/loaders/map/mod.rs`.
- Uses `parse_generic_data` (from `ragnarok-formats`) to read three core files from the GRF (paths use backslashes, lowercased in search):

  1. **.rsw (MapData / "GRSW" signature)** — Scene descriptor.
     - Versioned (MajorFirst).
     - References: `ground_file` (GND), `gat_file` (GAT).
     - Environment: `water_settings`, `light_settings` (ambient, diffuse, latitude/longitude).
     - `resources: MapResources` — list of placed objects, lights, sounds, effects.
     - `quadtree` (for culling, depth 5, 4 children per node).
     - From official client: exactly matches Gravity's RSW format. See `ragnarok-formats/src/map.rs:MapData`.

  2. **.gnd (GroundData / "GRGN")** — Ground mesh.
     - Dimensions (width/height in tiles), zoom.
     - Texture list, lightmaps, surfaces (UVs, texture/lightmap indices, vertex colors).
     - `ground_tiles`: 4 corner heights per tile + surface indices (top/north/east).
     - Built into vertex/index buffers for rendering.
     - Water plane generated from GND + RSW water settings (TODO: full GND v2.6+ water support).

  3. **.gat (GatData / "GRAT")** — Collision / height grid + picking.
     - Width/height in GAT tiles (GAT_TILE_SIZE = 5.0 world units).
     - Per-tile: 4 corner heights + `TileFlags` (WALKABLE, WATER, SNIPABLE, CLIFF).
     - Used for:
       - Server pathfinding/movement validation (shared data).
       - Client picking (tile picker buffers).
       - Debug tile rendering.
     - `generate_tile_vertices` creates debug/picker geometry.

- **Objects (placed models from RSW)**:
  - `map_data.resources.objects` (Vec<ObjectData> with model_name, transform).
  - `apply_map_offset` shifts everything so map origin aligns with GAT/GND center.
  - Models loaded via `ModelLoader` (cached by name + winding from scale sign).
  - Each becomes an `Object` with AABB.
  - Collected into `SimpleSlab<ObjectKey, Object>` + `object_kdtree` (KDTree for frustum/sphere culling).
  - Similar for `light_sources` (KDTree of spheres) and sound/effect sources.

- **Final Map struct** (`korangar/src/world/map/mod.rs`):
  - Stores tiles (from GAT), sub_meshes (ground), vertex/index buffers, texture_set (with videos for animated ground).
  - `objects`, `light_sources`, `sound_sources`, `effect_sources` (debug).
  - KDTrees for fast queries.
  - `lighting: Lighting` (from RSW).
  - Water plane.
  - Background music, videos.

- **Rendering** (see `GRAPHICS_PIPELINE.md` for full passes):
  - Ground: `render_ground` → model instructions (submeshes by texture or bindless).
  - Objects: `render_objects` (after culling) → per-object `render_geometry` (ModelInstruction with transform).
  - Water: animated texture cycling from RSW/GND.
  - Culling: `cull_objects_with_frustum` / `cull_objects_in_sphere` using object_kdtree (frustum from camera). Quadtree from RSW is parsed but Korangar primarily uses KDTree + explicit frustum tests.
  - Debug: tile overlays, pathing, bounding boxes (when render options enabled).

**Official client data tie-in**:
- All maps come from GRFs (data.grf + rdata + 2021 files). Use GRF Editor or Korangar's loaders to inspect.
- RSW/GND/GAT formats are stable across 2019/2021 data. See `ragnarok-formats/src/map.rs` for parsers (signatures GRSW/GRGN/GRAT, versioned fields).
- Placed objects reference models in `data\model\...` and textures.
- For DM: server scripts can start hazards that the client visualizes by placing temporary effects/objects or using quest markers at specific positions.

**Map change handling**:
- On `ChangeMap`: clear particles/effects/lights, truncate entities to player only, load new Map, add initial entities.

### Caching
Maps are cached (`Cacheable` impl) for fast reloads (important for DM iteration and large worlds).

## 2. Entity System (AddEntity → Npc/Player/Monster)

Entities represent living things in the world. Most are server-driven via packets.

### Adding Entities
- `NetworkEvent::AddEntity { entity_data }` in `korangar/src/lib.rs`.
- `entity_data` comes from `EntityData` (parsed from server packets: id, job_id, position, sex, etc.).
- `EntityType` derived from `job_id` (in `world/entity/mod.rs`):
  ```rust
  impl From<JobId> for EntityType {
      match job_id.0 {
          45 => Warp,
          111 => Hidden,
          0..=44 | 4000..=5999 => Player,
          46..=999 | 10000..=19999 => Npc,
          1000..=3999 | 20000..=29999 => Monster,
          _ => Npc,
      }
  }
  ```
- For NPCs: `Npc::new(...)` → creates `Entity::Npc` with `Common`.
  - Sprite path: `format!("npc\\{}", library.get::<JobIdentity>(job_id))` (from npcidentity.lub / jobidentity.lub).
- Players: body + head sprites under `인간족\...` (Korean paths in GRF).
- Monsters: `format!("몬스터\\{}", library.get::<JobIdentity>(job_id))`.
- Warps/Hidden: intentionally no sprite parts (to avoid misleading "missing" shadows); they are server triggers.
- Animation data loaded async via `request_animation_data_load` (pairs .spr + .act).
- Entity added to `ClientState.entities()` (or dead_entities).
- Inherit fade state if re-adding.

**Common** (shared base):
- Position (tile + world), direction, movement (pathing via PathFinder).
- Health, speed, animation_state, sound_state, fade.
- `get_entity_part_files` for sprite loading.

**Rendering**:
- Entities rendered in forward pass (after culling).
- Use animation_data for sprite frames.
- Status (HP bars, names) in overlays.
- For NPCs: green tint in debug, dialog cursor on hover.

**Removal**:
- `RemoveEntity` packet → remove from lists, handle death (fade out, etc.).

### Quest Effects / Markers on Entities
- `NetworkEvent::AddQuestEffect { quest_effect: QuestEffectPacket }`.
- Packet: `entity_id`, `position`, `effect` (id for texture), `color`.
- `ParticleHolder::add_quest_icon` creates `QuestIcon`:
  - Position = map world pos + (0,25,0) offset.
  - Texture: `유저인터페이스\\minimap\\quest_{effect_id}_1.bmp` (Korean path from GRF; falls back gracefully).
  - Color tint (Yellow/Orange/Green/Purple).
- Rendered in `particle_holder.render(...)` as projected sprite (30px scaled, with outline from asset).
- **Important recent change**: Now renders *all* active quest icons from packet data (not strictly filtered to current entities). This allows markers at locations even for warps/special entities or before/after sprite loads. Matches official client guide markers (e.g., "!" at teleport areas).
- Remove via `RemoveQuestEffect` or map change clear.
- Ties to official client: These are the standard quest "!" / "?" icons (see System/ quest data and GRF UI textures). Server (or client quest state) decides when to send.

**Entity types in rendering/culling**:
- Players/Monsters/NPCs participate in KDTree culling and entity rendering.
- Warps often get quest effects for visibility without full sprite.

**Skill tree data note** (related to entities/NPCs for completeness, though primarily UI):
Loaded via library from `data\\luafiles514\\lua files\\skillinfoz\\skilltreeview.lub` as `SKILL_TREEVIEW_FOR_JOB = { [jobid] = { [slot] = skillid, ... }, ... }` plus scaffolding for tabs/inheritance (JOB_INHERIT_LIST etc.). Used for player skill UI, but NPCs may reference job identities for visuals. Verified against H:\RO GRF (in GRF as skilltreeview.lub).

## 3. Effects, Particles, and Quest Icons (In-World DM Visuals)

Beyond entities, Korangar has a particle/effect system for dynamic visuals.

### Quest Icons / Markers (the "!" case)
- See above. Rendered as 2D sprites in world space (projected).
- Used for:
  - Quest givers (black outline "!" for visibility).
  - Instruction spots in starting areas / teleports.
  - DM guide markers (server can send QuestEffectPacket to highlight locations).
- In official client: These come from the same minimap quest textures and are shown above entities or at fixed positions for navigation/hints.
- DM tie-in: Server scripts (e.g., `dm_hunt_markers.txt`, beat scripts) can trigger these via quest effects or custom packets. Client renders them to telegraph important spots without needing full NPC bodies.

### Other Effects and Particles
- `EffectHolder` + `EffectRenderer`: Skill units, cast effects, hits, etc.
  - Loaded from `data\texture\effect\` or GRF effect files.
  - Animated via layers/frames (similar to .str effects).
- `ParticleHolder`:
  - Generic particles (heal numbers, misses, etc.).
  - Quest icons (as above).
- `world/effect/mod.rs`: Registers point lights for some effects.
- Rendering: After world geometry, before/ with interface overlays. Uses `animation_timer_ms` for timing.

**Hazard telegraphs / DM in-world visuals**:
- Server DM scripts start hazards (e.g., `@dmhazard`, beats that pulse pressure/status).
- Client visual: Often via quest effects (colored !/? markers at positions) + particles/effects at the center.
- Reuse skill unit rendering paths for pulsing areas.
- In Korangar: `AddQuestEffect` + particle system can place temporary effects at tile positions.
- For full DM: Client can also drive client-side previews or receive explicit effect packets from server scripts.

**Overhead / floating text**:
- `EntityMessagePacket` → overhead messages on entities.
- Used for NPC speech, damage, instructions.
- Rendered in entity status or separate overlays.

## 4. Direct Ties to DM Server Functions

Server (Hercules_RO `npc/custom/dm_campaign/`):
- Hazards/beats started in scripts (`dm_hazards.txt`, `dm_beats.txt`, `dm_combat.txt`, etc.).
- Use `@dmhazard`, `@dmtrap`, `@dmsymptom`, beat triggers that spawn effects or set quest flags.
- Can send `QuestEffectPacket` (or equivalent via existing packets) + entity updates to clients.
- Structured output via planned `[DMJ]` echoes or existing announce/dispbottom.

Client (Korangar) responsibilities for matching/improving official experience:
- **Receive and render**: When server sends `AddQuestEffect` for a hazard/beat location or NPC, show the icon/particle (using the exact quest_*.bmp textures from client data). This is how official client shows guide "!" and quest markers.
- **Drive from UI**: DM windows emit `@dmhazard ...`, `@dmscene ...` etc. as chat packets (see networking chat path). Server executes; client receives effects/entities and renders.
- **No full NPC body needed for markers**: Many guide spots use just the effect icon at a position (especially warps/teleports). The recent render change supports this.
- **In-world DM visuals**:
  - Hazard telegraphs: Positioned effects/particles at map coords (reuse skill unit or custom particle paths).
  - Scene/cutscene: `@dmscene` / `@dmspotlight` can trigger client-side camera/effect changes (or server-sent entities/effects).
  - Entity states for DM (downed, etc.): Extend entity rendering + quest effects.
- **Quest integration**: Many DM quests use the official quest system. Client journal + effects pull from quest packets + EN list (merged from server planning data).
- **Official client fidelity**: Use the same GRF textures/effects as the real client (H:\RO data). Markers have black outlines from the .bmp assets. Position + offset matches official projection.

**Example flow for a DM hazard**:
1. DM uses client UI → sends `@dmhazard <map> <x> <y> ...`.
2. Server script (dm_*.txt) activates hazard, sends QuestEffectPacket(s) or entity updates to party.
3. Korangar: AddQuestEffect → QuestIcon (or particle) at world position. Renders as ! / colored marker above the spot.
4. Visual telegraph + any associated text/effects.
5. Cleanup via RemoveQuestEffect or map change.

**Gaps / improvement opportunities**:
- Ensure all DM-started effects use the quest/particle system for consistency with official client.
- Client-side prediction or preview for DM tools (e.g., place temporary hazard marker before sending command).
- Better support for in-world DM elements (custom effect types, camera control for scenes).
- Tie entity quest effects to DM state (e.g., downed overlay on entities).
- Performance: Effects are in the particle render path after world culling.

See also:
- `DM_SERVER_FUNCTIONS.md` for server script side.
- `DM_INTERFACE.md` for planned client UI that drives these.
- `GRAPHICS_PIPELINE.md` for how effects/particles are rendered in passes.
- `RO_OFFICIAL_CLIENT_STRUCTURE.md` for quest textures and map formats from official data.
- Code: `src/world/particles/mod.rs` (QuestIcon), `src/lib.rs` (Add/RemoveQuestEffect), `src/loaders/map/mod.rs` (objects/effects from RSW), `src/world/entity/mod.rs` (Npc etc.).
- Packet side: `QuestEffectPacket` (0x0446) and `AddQuestEffect`/`RemoveQuestEffect` are documented in `docs/PACKET_EVENTS_CATALOG.md` (with full handler + downstream).

This map should let us replicate (and enhance) the official client's map/entity/effect behavior while adding DM-specific visuals. Update as we implement more DM features or discover edge cases from the live server.