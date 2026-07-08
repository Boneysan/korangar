# Official Ragnarok Online Client Structure

**Source:** The official RO client install at `H:\RO\client\` (Windows) / `/mnt/h/RO/client/` (WSL mount from the root `H:\RO`).  
**Client version:** 2019-06-05 (PACKETVER 20190605)  
**Main executable:** `2019-06-05fRagexe_patched.exe`  
**Purpose of this document:** Provide a practical map of where everything lives in the real client so developers can understand how the official RO client organizes data, assets, configuration, and runtime behavior. This is grounded in direct inspection of *your* H:\RO install.

This is derived from direct filesystem inspection of your actual `H:\RO` (via WSL mount) + Korangar's loading code + standard RO client conventions.

**Scope note:** This document focuses on the *client* portion (`H:\RO\client\`). The full `H:\RO\` root also contains the Hercules server (`server/`), Laragon web stack (`laragon.exe`, `www/`, `etc/`, `bin/`), and supporting files. A high-level inventory of the entire `H:\RO` already exists in `docs/PROJECT_PLAN.md` §1.

## 1. Top-Level Layout (of the Client)

```
H:\RO\client\
├── *.exe                          # Launchers, patchers, the game itself
├── *.grf                          # Main asset archives (see below)
├── System/                        # Client-side Lua/Lub databases (critical!)
├── BGM/                           # Background music (MP3s)
├── AI/ + AI_sakray/               # Homunculus / mercenary AI scripts (Lua)
├── NavigationData/                # Warp / pathfinding data
├── Skin/                          # Official UI skin resources
├── PatchClient/                   # Patcher UI and related
├── data/                          # Loose files (mainly clientinfo.xml here)
├── Replay/                        # Replay-related
├── *.ini, *.xml, *.txt            # Configs, patch lists, tips
├── DLLs (D3DX, bink, mss, etc.)   # Rendering, audio, video, protection
└── ...
```

**Sizes (approximate from inspection):**
- `data.grf`: ~3.1 GB (primary assets)
- `rdata.grf`: ~293 MB
- `resources2021.grf`: ~66 MB
- `renewal2021.grf`: ~7 MB
- `System/`: ~57 MB (databases)
- `BGM/`: ~345 MB

## 2. GRF Archives — The Core Asset Store

The client loads archives in the order defined in `data.ini`:

```
[Data]
1=renewal2021.grf
2=resources2021.grf
3=data.grf
4=rdata.grf
```

Korangar mirrors this in `client/game_archives.ron`.

**Virtual layout inside the GRFs** (use backslashes in paths; names often use Korean encoding / EUC-KR):

The GRF stores a file table; plain `strings` often yields limited results because filenames live in a compressed table. The most reliable map comes from what the official client (and thus Korangar) actually requests.

**Common roots (observed in loaders):**
- `data\sprite\` — All 2D sprite + animation data (.spr + .act pairs)
- `data\texture\` — Textures (UI elements, ground, effects, etc.)
- `data\model\` — 3D models (.rsm, .rsm2)
- `data\luafiles514\` — Lua-compiled data tables (the "brain" for many systems)
- `data\map\` — Map files (.rsw, ground, gat, lightmaps)
- `effect\` / `data\texture\effect\` — Particle / skill effect definitions (.str files)

**Precise paths loaded by the client / Korangar (authoritative):**

**Sprites & Animations**
- `data\sprite\{job_or_mob_path}.spr` and `.act`  (e.g. `data\sprite\인간족\남\1_남.spr`, `data\sprite\몬스터\poring.spr`)
- Player bodies/heads: `data\sprite\인간족\남\...`, `data\sprite\인간족\여\...`, head dirs like `머리통\`
- Mobs: `data\sprite\몬스터\{monster_id_name}`
- NPCs: `data\sprite\npc\...`
- Items: `data\sprite\아이템\...`

**Lua Data Tables (very important for names, skills, quests)**
- `data\luafiles514\lua files\datainfo\iteminfo.lub`
- `data\luafiles514\lua files\datainfo\jobidentity.lub`
- `data\luafiles514\lua files\datainfo\npcidentity.lub`
- `data\luafiles514\lua files\skillinfoz\skillid.lub`
- `data\luafiles514\lua files\skillinfoz\skillinfolist.lub`
- `data\luafiles514\lua files\skillinfoz\jobinheritlist.lub`
- `data\luafiles514\lua files\mapskydata\mapskydata.lub`

**Textures & Effects**
- `data\texture\{path}`
- `data\texture\effect\{name}`
- `effect\{prefix}{name}` (for .str effect definitions)

**Models & Maps**
- `data\model\{model_file}.rsm`
- `data\{mapname}.rsw`
- `data\{ground_file}`
- `data\{gat_file}`

Korangar mirrors these exact paths in `GameFileLoader`, `SpriteLoader`, `ActionLoader`, `ModelLoader`, `TextureLoader`, `EffectLoader`, and the `world/library/*` modules.

### Binary Formats (for Porting / Custom Loaders)

#### GRF Archive Format (data.grf etc.)
- Header (fixed 0x200 version for this client):
  - Magic: "Master of Magic"
  - File table offset (u32, from start of file)
  - Seed / scramble
  - File count (after decompression)
  - Version: 0x200
- File table is Zlib-compressed.
- Each entry (`FileTableRow`):
  - `file_name` (variable, null-terminated, case-insensitive, uses `\`)
  - `compressed_size`, `compressed_size_aligned`, `uncompressed_size`
  - `offset` (from after header)
  - `flags` (u8): bit 0 = mixed crypto, bit 1 = DES, bit 2 = zlib?
- Loading in Korangar: `NativeArchive::from_path` → parse header → seek to table → decompress → populate HashMap<String, FileTableRow> (lowercased keys).
- Access: `get_file_by_path` seeks + reads + decrypts/decompresses per-file (mixcrypt + zlib).
- Korangar tip: FolderArchive and 7z overrides take precedence (inserted at front of search list).

#### Sprite Format (.spr)
```rust
// From ragnarok-formats/src/sprite.rs
pub struct SpriteData {
    pub signature: Signature<b"SP">,
    pub version: Version<MinorFirst>,
    pub palette_image_count: u16,
    pub rgba_image_count: Option<u16>, // >= 1.2
    pub palette_image_data: Vec<PaletteImageData>,
    pub rgba_image_data: Vec<RgbaImageData>,
    pub palette: Option<Palette>,      // >= 1.1
}
```
- Korangar `SpriteLoader` parses this, converts to RGBA textures (premultiplied alpha), caches up to 4k sprites / 256MB.
- Fallback: `npc\missing.spr`.

#### Action / Animation Format (.act)
```rust
pub struct ActionsData {
    pub signature: Signature<b"AC">,
    pub version: Version<MinorFirst>,
    pub action_count: u16,
    pub actions: Vec<Action>,   // each Action has Vec<Frame>
    pub events: Vec<Event>,     // >= 2.1
    pub delays: Option<Vec<f32>>, // >= 2.2 per-action
}
```
- `ActionLoader` loads alongside `.spr`.
- Frames contain sprite index, offsets, scale, rotation, color, sounds.
- Used by `AnimationState` to drive entity playback (idle, walk, attack, die, etc.).
- Attack timing is in specific action indices per job/mob.

#### Map Formats
See detailed breakdown in §4 below. Key structs are in `ragnarok-formats/src/map.rs`:
- `MapData` (GRSW): ground/gat refs + `MapResources` (objects + lights + sounds + effects) + quadtree.
- `GroundData` (GRGN): tiles with 4-corner heights + surfaces (UVs + lightmap + color).
- `GatData` (GRAT): per-tile heights + `TileFlags` (WALKABLE | WATER | SNIPABLE | CLIFF).

Parsing uses `parse_generic_data` (versioned ByteConvertable).

#### Effect Format (.str) - Full Technical Spec

Effects are stored as `.str` files (signature `STRM`).

From `ragnarok-formats/src/effect.rs`:

```rust
pub struct EffectData {
    pub signature: Signature<b"STRM">,
    pub version: Version<MajorFirst>,
    pub frames_per_second: u32,
    pub max_key: u32,
    pub layer_count: u32,
    pub layers: Vec<LayerData>,
}

pub struct LayerData {
    pub texture_count: i32,
    pub texture_names: Vec<TextureName>,  // 128-byte strings
    pub frame_count: i32,
    pub frames: Vec<Frame>,
}

pub struct Frame {
    pub frame_index: i32,
    pub frame_type: i32,          // 0=Basic, 1=Morphing
    pub offset: Vector2<f32>,
    pub uv: [f32; 8],
    pub xy: [f32; 8],
    pub texture_index: f32,
    pub animation_type: i32,
    pub delay: f32,
    pub angle: f32,
    pub color: [f32; 4],
    pub source_blend_factor: i32,
    pub destination_blend_factor: i32,
    pub mt_present: i32,          // multi-texture flag
}
```

Korangar mapping (loaders/effect/mod.rs + world/effect.rs):
- Parses to `Effect` with `Vec<Layer>`, each `Layer` has textures + `Vec<Frame>`.
- `FrameType::Basic` / `Morphing`.
- Blend factors mapped to wgpu.
- Rendered with particle system, animation driven by client tick.
- Loaded as `effect\{prefix}{name}` or `data\texture\effect\...`.

Current status: Loader + basic rendering exists. Full keyframe morphing, multi-texture, advanced blending may be partial. Great for skill hits, magic effects, @dm scene visuals.

Example usage in client: skill cast effects reference these by name from skill DB or hard-coded.

#### Lua Data Tables (mlua + GameFileLoader)
All loaded via:
```rust
let state = Lua::load_from_game_files(game_file_loader, &["data\\luafiles514\\..."]);
let tbl = state.globals().get::<mlua::Table>("tbl")?;  // or SKILL_INFO_LIST, etc.
```
Common tables and Korangar usage:
- `iteminfo.lub` → `ItemInfo` (identified/unidentified names, resources)
- `jobidentity.lub` + `npcidentity.lub` → `JobIdentity`
- `skillinfolist.lub` + `skillid.lub` + `jobinheritlist.lub` → skills + requirements + tree
- `skilltreeview.lub` + scaffolding → UI tabs
- `mapskydata.lub` → sky (partial)
- System/ loose .lub/.lua (especially itemInfo_EN.lua) are authoritative for English data on this install.

Encoding fix: many tables use EUC-KR; Korangar has `fix_encoding`.

**Full Skill Tree Lua Structures (detailed)**

The client uses a combination of official `luafiles514\skillinfoz\` tables + Korangar scaffolding for the skill tree UI (tabs + slots per job).

**Core tables loaded (in `world/library/skill_tree.rs`):**

- `JOB_INHERIT_LIST` / `JOB_INHERIT_LIST2` (from jobinheritlist.lub): `{ [jobid] = parent_jobid }` for inheritance (2nd list first).
- `SKID` (skillid.lub): `{ ["SKILL_NAME"] = id }`
- `SKILL_TREEVIEW_FOR_JOB` (skilltreeview.lub): `{ [jobid] = { [slot_index] = skillid, ... } }` — the visual layout per job. Slots are 0-based, laid out in 7-column grid in UI.
- `JOBID` table for all job constants.

**Scaffolding (local `archive/data/lua-scaffolding/` , executed in Lua state):**

`skill-tree-tab-name.lua`:
- Defines `JobSkillTab[job_id] = { tab0_name, tab1_name, ... }` via `ChangeSkillTabName`.
- `GET_SKILL_TAB_NAME(job_id)` returns combined table [0]= "Novice", [1..]= base or override ("First", "Second"...).
- Handles inheritance of tab names.

`skill-tree-tab-for-job.lua`:
- `GET_TAB_FOR_JOB(job_id)`: Complex if/elseif on job ranges (JT_NOVICE, < KNIGHT, Taekwon, Supernovice range, 2nd/3rd/4th classes, baby/trans, etc.) returning tab index 0-4.
- Used to know how many tabs a job has.

**Runtime in Korangar (`evaluate_job` + `SkillTreeLayout`):**
```lua
-- conceptual
SKILL_TREEVIEW_FOR_JOB = {
  [4001] = { [0] = 1, [1] = 2, ... }  -- jobid -> slot -> skid
}
```
- Recurse inherit lists, merge skills into tabs without duplicates.
- Build `Vec<SkillTabLayout>` { name, skills: HashMap<usize, SkillId> }
- UI (skill_tree/tabs.rs): 7 columns, dynamic rows from max slot, drag to hotbar.

The official client hardcodes some of `GET_TAB_FOR_JOB` logic in `skillinfo_f.lub`; scaffolding overrides for flexibility.

This structure is what populates the in-game skill tree window per job, including inheritance from novice → 1st → 2nd etc.

**Quadtree Culling Details**

In `.rsw` (MapData):

```rust
pub quadtree: Option<QuadTreeData>,  // >= 2.1
pub struct QuadTreeData {
    pub max: [f32; 3], min: [f32; 3], half_size: [f32; 3], center: [f32; 3],
    pub children: Vec<QuadTreeData>,  // up to 4
}
```

Parsing (`FromBytes` in ragnarok-formats/map.rs): DFS stack simulation, depth max 5, 4 children per level. Builds tree bottom-up, reverses children for correct order. Root at nodes[0].

Purpose in official client: Spatial partitioning of the map for frustum culling of objects/lights/effects, faster than checking every placed model.

**In Korangar:**
- Parsed (stored in MapData).
- **Not heavily used for culling.** Instead:
  - Objects get AABBs, inserted into a `KDTree` (in `world/map/mod.rs` and collision kdtree).
  - Culling: `cull_objects_with_frustum` / `cull_objects_in_sphere` using `object_kdtree.query(&frustum, ...)` or sphere.
  - Frustum from camera view-projection.
  - Also used for debug bounding box rendering and light culling.
- Quadtree present for compatibility / future official-style culling or editor use. Current impl uses KDTree for objects + explicit frustum tests in lib.rs render passes (world, shadows).

This is why some maps feel fast even with many placed objects — KDTree + early frustum discard.

**How the Client Renders Placed Map Objects**

From `.rsw` `resources.objects` (Vec<ObjectData>):

```rust
pub struct ObjectData {
    pub name: Option<String>,
    pub model_name: String,   // .rsm path
    pub _node_name: String,
    pub transform: Transform, // position, rotation (quat?), scale
}
```

**Loading pipeline** (`loaders/map/mod.rs:load`):
1. Parse RSW → `map_data.resources.objects`.
2. `apply_map_offset(ground_data, &mut resources)` — shifts everything so map origin matches GAT/GND tile grid center (width/2, height/2 in world units).
3. For each object:
   - Cache Model by (model_name, reverse_order from negative scale product).
   - `model_loader.load(...)` accumulates vertices/indices into builder.
   - Create `Object { name, model_name, model, transform }`.
   - Compute AABB from model + transform.
4. Build `KDTree` from (key, AABB) pairs for fast queries.
5. In `Map` struct: stores `objects: SimpleSlab<Object>`, `object_kdtree`.

**Rendering** (`world/map/mod.rs` + lib.rs):
- `cull_objects_with_frustum(camera, enabled)` → uses kdtree.query(frustum) → ResourceSet of visible ObjectKeys.
- `render_objects(instructions, object_set, animation_timer, camera)`:
  - For each visible key: `object.render_geometry(instructions, ...)` 
  - Pushes `ModelInstruction { model_matrix: transform.as_matrix(), index/vertex offsets from submeshes, texture_index, ... }`
- Ground is separate (its own submeshes + vertex buffer).
- Models from objects are batched into the map's overall ModelBuffer/Textures.
- Animation: `animation_timer_ms` passed for animated models (e.g. some flags, water wheels?).
- Transparency, distance sorting handled in render pass.
- Debug: bounding boxes, wireframe, etc. via render_options.

Placed objects (buildings, trees, signs from the .rsw resource list) are thus instanced 3D models positioned exactly as authored in the map, culled spatially, and rendered alongside the terrain ground mesh and entities.

This matches the official client's scene graph for maps. Korangar approximates it with KDTree instead of (or in addition to) the quadtree for performance.

These sections give the low-level details needed to match or improve rendering, UI, and culling in Korangar when working with official map/skill data from H:\RO.

### Full H:\RO Root Technical Inventory (for Asset & Data Pipeline)

```
H:\RO\
├── client/                     # Primary source for Korangar assets (see above)
│   ├── *.grf (4 files, load order in data.ini)
│   ├── System/
│   │   ├── itemInfo_EN.lua     # Best English item DB (use this!)
│   │   ├── OngoingQuestInfoList_True_EN.lub
│   │   ├── LuaFiles514/
│   │   └── ...
│   ├── BGM/                    # MP3s, referenced by map or BGM scripts
│   ├── data/                   # Loose (clientinfo.xml)
│   └── Skin/, NavigationData/, AI/, ...
├── server/                     # Hercules_RO (PACKETVER 20190605)
│   ├── npc/custom/dm_campaign/ # Your D&D content (@dm* commands)
│   ├── conf/import/            # battle.conf, etc. (packet_obfuscation:0)
│   ├── db/                     # item_db, mob_db, skill_db (source of truth for customs)
│   └── ...
├── laragon.exe + www/ + etc/ + bin/ + data/  # Web stack (FluxCP, control panel, etc.)
├── HerculesRO-client.zip
└── setup-wsl-portforward.ps1
```

**Porting guidance:**
- GRFs + System/ loose files → GameFileLoader + FolderArchive overrides.
- To improve English: prefer itemInfo_EN.lua / _EN.lub in loaders.
- Animations: full support via Sprite + Action loaders + AnimationState.
- Maps: GND + GAT + RSW + models already implemented; add more effect/sky support from the formats.
- For DM tools: pull quest data from OngoingQuestInfoList_True_EN.lub and server-side scripts.

This document is intended as a living technical reference for extending Korangar while staying compatible with the 2019-06-05 client data at H:\RO.

### Maps (Detailed)

RO maps are **not** single files. A map name (e.g. "prontera", "izlude", "geffen") refers to a small set of files that together describe the 3D scene, collision, and environment.

All paths are under `data\` inside the GRFs (e.g. `data\prontera.rsw`).

**The three core files per map:**

1. **`.rsw` (MapData / "GRSW")** — The map scene file.
   - Header + version (major.minor, plus build version in newer).
   - References:
     - `ground_file` → the `.gnd` file
     - `gat_file` → the `.gat` file
   - Environment:
     - `water_settings` (water level, wave height/speed/pitch, texture cycling interval)
     - `light_settings` (sun longitude/latitude, diffuse + ambient color, shadow alpha)
   - Bounds (top/bottom/left/right in older versions)
   - **Resources** (the interesting part):
     - Placed **Objects**: model_name (.rsm), transform (position, rotation, scale). These are the buildings, trees, signs, etc.
     - **LightSources**, **SoundSources**, **EffectSources** (with name, position, parameters).
   - **QuadTree** (for culling — 5 levels of 4 children, bounding boxes).

2. **`.gnd` (GroundData / "GRGN")** — The ground/terrain mesh.
   - Dimensions (width × height in tiles) + zoom factor.
   - List of texture names used by this ground.
   - Lightmap data (baked lighting textures).
   - Surfaces: UV coordinates (4 u/v per surface), texture index, lightmap index, vertex color.
   - GroundTiles: 4 corner heights + indices into surfaces (top, north, east faces).
   - The ground is rendered as a big grid of textured quads with per-vertex heights → hills, valleys, ramps.

3. **`.gat` (GatData / "GRAT")** — Walkability / collision grid.
   - Width × height (in "GAT tiles").
   - Each `Tile`:
     - 4 corner heights (southwest, southeast, northwest, northeast)
     - Flags (bitfield): WALKABLE, WATER, SNIPABLE, CLIFF
   - **Important constants** (in Korangar): `GAT_TILE_SIZE = 5.0` world units per tile.
   - Used by:
     - Server for movement validation and pathfinding.
     - Client for cursor height, picking, and some visual cues.
     - Korangar generates debug tile meshes and picker buffers from it.

**Additional map-related data:**

- Water planes: generated at runtime from GND heights + water_settings in RSW (animated texture cycling).
- Placed models: loaded on demand via the ModelLoader using the names from .rsw resources.
- Sky / atmosphere: referenced via `mapskydata.lub` (still TODO in some places).
- Lighting: directional light from the RSW light settings + point lights from LightSources + lightmaps on ground.

**How the official client (and Korangar) assembles a map at runtime:**

1. Load `{map}.rsw` → get ground/gat names + list of objects + env settings.
2. Load the referenced `.gnd` → build ground mesh (vertices + surfaces + textures).
3. Load the referenced `.gat` → build collision grid + tile debug data.
4. For each object in resources: load the `.rsm` model + place it with the given transform.
5. Generate water if present.
6. Apply lights, effects, sounds from the resources list.
7. (Client also loads minimap image, etc.)

**In Korangar specifically** (`loaders/map/mod.rs`, `world/map/`):
- Heavy use of vertex buffer generation from ground + GAT tiles.
- TextureSetBuilder for ground textures (with video support for some animated ones).
- Submesh splitting for older GPUs without bindless.
- Picker buffers for mouse-to-world (using GAT tiles).
- Models are instanced/added via the normal ModelLoader.
- Map is cached (see `maps cacheable` in project history) for near-instant reloads on change.
- Debug inspectors exist for map data (in `#[cfg(feature = "debug")]`).

**Examples of maps** (common ones inside your `data.grf`):
- prontera, izlude, geffen, payon, morocc, alberta, aldebaran, etc.
- Newer/renewal maps may be in the 2021 GRFs.

**Bringing maps / map data over from H:\RO:**

- They come "for free" once you have the GRFs loaded (data.grf + rdata + renewal/resources).
- If you want to override a specific map (custom geometry, added objects, fixed lighting), place files in `korangar/archive/data/` with the same names:
  ```
  korangar/archive/data/prontera.rsw
  korangar/archive/data/prontera.gnd   (or whatever the ground_file inside says)
  korangar/archive/data/prontera.gat
  ```
- Local FolderArchive takes precedence in the loader search order.
- For completely custom maps you would also need to update server-side .gat/.rsw equivalents for collision.

This map system is one of the more complex parts of the client because it mixes 2.5D ground + full 3D models + baked lighting + grid-based collision.

## 3. System/ — Client-Side Databases (Extremely Important)

This directory provides **override / authoritative** data that the client uses for names, descriptions, quests, UI strings, etc. It is often more important than what's packed in the GRFs for custom servers.

Key files (many have English variants):

| File                              | Purpose                                      | English variant          |
|-----------------------------------|----------------------------------------------|--------------------------|
| `itemInfo_EN.lua` / `itemInfo*.lub` | Item names, descriptions, slots, icons      | Primary English source   |
| `OngoingQuestInfoList_True_EN.lub` | Quest journal data (heavily used by campaign) | Yes (True_EN)           |
| `MsgString.lub`                   | UI strings, messages                         | -                        |
| `Towninfo.lub` / `Towninfo_EN.lub`| Town / warp info                             | Yes                      |
| `achievement_list*.lub`           | Achievements                                 | _EN                      |
| `CheckAttendance*.lub`            | Daily rewards / attendance                   | _EN                      |
| `PetEvolutionCln*.lub`            | Pet evolution data                           | _E                       |
| `ShadowTable.lub`                 | Shadow rendering table                       | -                        |
| `RecommendedQuestInfoList*.lub`   | Recommended quests                           | _EN                      |
| `LuaFiles514/`                    | Some core tables (MsgString, OptionInfo)     | -                        |
| Fonts/                            | Custom Korean/English fonts                  | -                        |

**How the official client uses it:** The exe + GRF data can be overridden by files in `System/` (especially when `readfolder` is set in clientinfo).

Korangar currently loads item/skill data via the `luafiles514` paths inside the GRFs. The `_EN` files are a major source for proper English text.

## 4. Sprites, Animations & Visuals

This is the core of "how the game looks and moves."

- **.spr files**: Contain the actual image frames + palette.
- **.act files**: Define animation sequences (which frames play when, timing, sound events, attack frames, etc.).

**Animation categories:**
- Player jobs (`인간족\...`): Body + head separate. Different `.act` actions for idle, walk, attack (basic + skill), sit, pickup, etc.
- Mobs (`몬스터\...`): One `.spr` + `.act` per monster. Contains walk, attack, hit, death, etc.
- Effects: Many skill and hit visuals are driven by `.str` (structured effect) files + sprite/texture references.
- Attack visuals are driven by the action indices in the `.act` + any attached effect sprites.

Korangar:
- `SpriteLoader` loads `data\sprite\{path}.spr`
- `ActionLoader` loads `data\sprite\{path}.act`
- `AnimationState` + entity code picks the right action (including attack states).

## 5. Other Major Components

### Audio
- `BGM/`: MP3 files (numbered). Korangar can point at this directory.

### AI / Homunculus
- `AI/` and `AI_sakray/USER_AI/`: Lua scripts for homunculus behavior.
- Includes documentation HTML.

### Navigation & Warps
- `NavigationData/`: Contains warp lists and path data (Korean text in the sample file).

### Patching & Updates
- `PatchClient/`, `patch*.txt`, `rsu-*.exe`, `Patchup_RE.exe`.
- The client has a full patcher infrastructure.

### Configuration
- `data.ini`: GRF load order (very useful).
- `clientinfo.xml` / `sclientinfo.xml`: Server address, port, version, langtype, `passwordencrypt`, `readfolder`, etc.
- `RagnarokKR.ini`, `dinput.ini`, etc.
- User `savedata/` (OptionInfo.lua, etc.) — not present in this snapshot but standard.

### Replay
- `RagnarokReplay.exe` + `Replay/` folder.

### Skin / UI Theming
- `Skin/`: Official skin resources + manual.

## 6. Runtime Behavior Highlights

- The client is a classic Win32 + DirectX (or compatible) app using custom GRF format.
- Data loading order: GRFs (in `data.ini` order) + loose files from `System/` and `data/` when allowed.
- Packet version is tied to the exe (here 20190605). Server must match `PACKETVER`.
- Most "game data" (items, skills, quests, strings) lives in the Lua/Lub tables rather than being hardcoded.
- Sprites and actions are the foundation of all character/mob animation.

## 7. Korangar Interaction Map

Korangar tries to replicate the important parts:

- `GameFileLoader` + archives: Replaces the GRF + loose file system.
- `world/library/*.rs` (item_info, skill_information, job_identity, etc.): Re-implements loading from the `luafiles514` paths.
- `loaders/sprite/` + `loaders/action/`: Load `.spr`/`.act`.
- Local `korangar/archive/data/`: Acts as an override layer (fonts, languages, textures, lua-scaffolding, models, sprites).
- `sclientinfo.xml` in the local archive.
- Campaign-specific data (e.g. `OngoingQuestInfoList_True_EN.lub`) is maintained separately.

Current practical setup (as of recent work):
- GRFs from H:\RO are symlinked or copied.
- English item/quest data is available in `System/`.
- Local overrides prepared for sprites.

## 8. Tips for Exploration & Reverse Engineering

- Use a GRF editor (GRF Editor on Windows) to browse/extract without the game running. This is the best way to get a full file list.
- `strings` on the .grf files gives only partial results (the filename table is stored compressed toward the end of the file). Use a real GRF reader for exhaustive listing.
- `itemInfo_EN.lua` (and other `_EN` files in System/) are among the best sources for readable English data.
- The patched exe + `clientinfo.xml` (version 55, langtype 1) tells you the exact protocol version.
- Many features (quests, skills, UI strings) can be understood by looking at the corresponding .lub and the server script (Hercules) that feeds it.
- For animations (attacks, mobs, etc.): open a `.act` + `.spr` pair in ActOR or BrowEdit. The action indices map directly to what the client's (and Korangar's) `AnimationState` selects (idle, walk, attack, die...).
- To see exactly what the real client loads, search the Korangar source for `data\\` strings — they mirror the official paths 1:1.

## 9. Next Steps / Open Questions

- Extract specific sprite/animation sets (mobs for attack behaviors, particular jobs for player attacks, effect .str + sprites) into `korangar/archive/data/sprite/` for local inspection or overrides? (We have prepared the directory skeleton with typical folders.)
- Improve Korangar's item/skill loaders to prefer the `_EN` / English variants from System/ when present?
- Expand effect `.str` system or UI skin format documentation next?
- Produce a "Korangar vs Official Client load matrix" table showing every path we currently emulate.
- Use this map to decide what else to bring over from H:\RO (animations, specific textures, BGM, NavigationData, etc.).

Recent attempts to dump raw filenames via `strings` on data.grf confirmed that a proper GRF reader (or extraction) is required for a complete listing. The paths the real client actually asks for (captured in Korangar's code) are the practical map.

This document should grow as we explore more specific areas (skill effects, map rendering, UI layout, etc.).

---

## 10. Broader H:\RO Install Context (Full Root Map)

`H:\RO` (mounted at `/mnt/h/RO` in WSL) is your complete Windows-side RO + server development environment. The detailed map in this document focuses on the **game client** (`client/` subdirectory), because that's where the assets, animations, map data, English translations, etc. live that Korangar consumes.

Here is the actual top-level layout of the full `H:\RO`:

```
H:\RO\
├── HerculesRO-client.zip          # Packaged client distribution (3.8 GB)
├── laragon.exe                    # Laragon stack launcher (PHP + MySQL + Apache/Nginx)
├── setup-wsl-portforward.ps1      # WSL networking helper
├── client/                        # ← The actual RO game client (3.9 GB)
│   ├── 2019-06-05fRagexe_patched.exe   # Main patched client (PACKETVER 20190605)
│   ├── *.grf (data.grf, rdata.grf, renewal2021.grf, resources2021.grf)
│   ├── System/                    # Lua/Lub data tables + English variants
│   ├── BGM/
│   ├── AI/ + AI_sakray/
│   ├── NavigationData/
│   ├── Skin/
│   ├── PatchClient/
│   └── ... (DLLs, configs, etc.)
├── server/                        # Hercules_RO (621 MB)
│   ├── char-server.exe, map-server.exe, login-server.exe
│   ├── npc/custom/dm_campaign/    # Seal Cascade D&D content
│   ├── conf/, db/, src/, etc.
│   └── athena-start, cache/, etc.
├── bin/                           # Laragon binaries (httpd, mysqld, php, etc.)
├── www/                           # Web root (index.php, etc.)
├── etc/                           # Configs for apache, nginx, php, mariadb, ssl
├── data/                          # MariaDB data + other runtime data
├── usr/                           # Additional Laragon components
└── tmp/                           # Temporary files
```

**Sizes (from direct inspection):**
- `client/`: ~3.9 GB (main source of GRFs, sprites, maps, English item/quest data)
- `server/`: ~621 MB
- `HerculesRO-client.zip`: ~3.8 GB

This full root is referenced throughout the project (see `docs/PROJECT_PLAN.md` §1 for the original detailed inventory table, and `CLAUDE.md` for WSL development notes).

When we copy or reference things "from H:\RO", we are almost always pulling from:
- `client/` → GRFs, `System/itemInfo_EN.lua`, sprites/animations, BGM, etc.
- `server/` → Reference for custom NPC scripts, packet expectations, campaign mechanics.

### Current State of Mapping
- **Detailed "where everything is" for the game client** → This document (`RO_OFFICIAL_CLIENT_STRUCTURE.md`), including GRF virtual paths, `.rsw`/`.gnd`/`.gat` map format, sprite animation system, System/ tables, etc.
- **High-level inventory of the full `H:\RO`** → `docs/PROJECT_PLAN.md` §1.
- **Korangar-specific loading** → Cross-referenced throughout this doc and in the codebase (GameFileLoader, world/library/*, loaders/*).

If you want a deeper "server map" (detailed breakdown of `server/npc/custom/dm_campaign/`, conf/import/, db/ structure, how the DM commands work, etc.) or a more exhaustive file-by-file tree of the entire H:\RO, let me know and I'll build it out in the same style.

---

## How to Use This Document to Build Korangar Features

1. **Identify the data source** — almost everything is in one of:
   - GRF (via `data\\...` path)
   - System/ loose .lua/.lub (prefer _EN variants)
   - Local `korangar/archive/data/` overrides

2. **Add the path** to the appropriate loader or `Lua::load_from_game_files` call.
3. **Parse with existing ragnarok-formats** types (or extend ByteConvertable).
4. **Wire into state / renderer** (ClientState, Library, AnimationState, etc.).
5. **Test against live H:\RO client** (use the exact GRFs + System/ from your install).

### Korangar Implementation Status Table (vs Official Client Formats)

This is a gap analysis for building out Korangar using H:\RO data.

| Format / System          | Official Client Location          | Korangar Support                          | Notes / Gaps for DM Work |
|--------------------------|-----------------------------------|-------------------------------------------|----------------------------|
| GRF Archives             | *.grf (data/rdata/renewal...)    | Full (NativeArchive + Folder + 7z)       | Load order matches data.ini. Good. |
| Sprite (.spr) + Action (.act) | data\sprite\...                 | Full (SpriteLoader + ActionLoader + AnimationState) | Attack indices per job/mob mostly working. Verify per-H:\RO jobs. |
| Map (.rsw + .gnd + .gat) | data\*.rsw etc.                  | Good (MapLoader, ground + models + GAT tiles) | Water/sky/lightmaps partial. Custom DM maps possible via overrides. |
| Effect (.str)            | effect\*.str or data\texture\effect | Partial (EffectLoader + basic Layer/Frame render) | Keyframe morphing, MT present, advanced blends incomplete. High value for @dmscene / skills. |
| Item DB (names, desc)    | System/itemInfo_EN.lua + GRF iteminfo.lub | Partial (loads iteminfo.lub; EN not preferred) | Drive from itemInfo_EN.lua for English. Critical for inventory/journal. |
| Skill DB / Tree          | luafiles514/skillinfoz + System  | Good (skill_information, skill_tree, requirements) | Scaffolding for tabs. |
| Quests / Campaign        | System/OngoingQuestInfoList_True_EN.lub | Partial (client uses for journal)       | Native journal (E7.3) needs this + server merge tool. |
| Lua tables (general)     | data\luafiles514\... + System    | Many (job, npc, skill, mapsky, etc. via mlua) | fix_encoding used. Add more for UI strings. |
| Models (.rsm)            | data\model\...                   | Full (ModelLoader)                       | Good for map objects. |
| Textures / UI            | data\texture\...                 | Full (TextureLoader, cache)              | Good. |
| BGM                      | BGM/*.mp3 (loose)                | Supported via audio engine               | Point to H:\RO/client/BGM. |
| Server DB (item/skill/mob) | server/db/*.txt + .conf        | Not parsed by client (client uses its DB) | For DM parity: sync customs via tools/campaign_quest_merge.py etc. Use for @dmreward etc. |
| NPC Scripts (DM commands)| server/npc/custom/dm_campaign/*.txt | Not parsed (client emulates via chat @dm) | Client-side DM windows generate @dm chat. Parse responses or use [DMJ] echo for structured. |
| Quest / Flag state       | Server memory + client quest packets | Quest packets modeled (0x0AFF etc.), no full DM state | For initiative/hazard/encounter: Phase A chat parse or Phase B packets. |

**Status legend**: Full = production usable; Good = works for core; Partial = basic works, needs work; Not = client doesn't handle (server authoritative).

Use this to prioritize: e.g., finish .str for visuals, EN item loader, map resources for immersive DM scenes.

### Server-Side DB / NPC Cross-Reference (for DM Command Parity)

The "real client" at H:\RO works with the Hercules server at H:\RO\server.

**DB formats (server/db/)** (from direct ls):
- item_db2.conf, pre-re/item_db.conf, re/item_db.conf : Classic item DB format (tab or comma separated fields for ID, Name, Type, Price, Sell, Weight, ATK:DEF, Range, Slots, Job, Upper, Loc, WeaponLv, EquipLv, Refineable, View, Script, OnEquip, OnUnequip).
- Other important: abra_db.txt, achievement_rank_db.conf, attendance_db.conf, captcha_db.conf, cashshop_db.conf, castle_db.conf, clans.conf, constants.conf, create_arrow_db.txt, elemental_db.txt, elemental_skill_db.txt, guild_skill_tree.txt, homun_skill_tree.txt, homunculus_db2.conf, item_options.conf, job_db2.txt, magicmushroom_db.txt, map_index.txt.
- For customs (Seal Cascade items, mobs, skills): Edit the appropriate .conf/.txt (pre-re or re), reload server. Client display requires matching updates in System/itemInfo_EN.lua (and GRF if packed).
- Cross sync: Your tools/campaign_quest_merge.py and verify scripts keep client quest/journal in sync with server.

**NPC / DM Scripts (server/npc/custom/dm_campaign/)** (and related in custom/):
- Scripts: dm_console.txt, dm_checks.txt, dm_beats/decisions/flags/quests.txt, dm_combat.txt, dm_downed.txt, dm_rewards.txt, dm_scene/voice.txt, dm_session_log.txt, etc.
- Commands (@dm*, @roll at GM0): Use Hercules script language (mes, select, dispbottom, announce, set, if, etc.).
- State variables: $dm_active_party, $dm_client_mode, beat/flag/quest tables, encounter state.
- Output: text + optional structured [DMJ] JSON when $dm_client_mode (for client parsing without new packets).
- Cross-ref with client: Client sends chat for commands; parses results for dice cards, initiative, journal. Use the EN quest list for display. Server is authoritative for all DM state (beats, decisions, downed, initiative).

**For DM tools in Korangar**:
- Client-side: Generate chat packets for @dm commands (no server change needed for Phase A).
- Server: Validates (party/GM), mutates state, outputs text + [DMJ] echoes.
- Robustness: Scripts can be updated to emit machine-readable lines when client mode on.
- Packet: 20190605 + length fallbacks in Korangar (no desync on unknowns).
- Quest/flag sync: Use the merge tool + client journal.

This allows native UI (check console, initiative tracker, encounter panel, campaign board) while server remains the single source of truth.

Keep this doc in sync as you add loaders — it is the canonical reference for the 2019-06-05 client data at H:\RO.

---

**Hexdump + Field-by-Field Walk (GRF Header - Entry Point for All Map/Effect/Asset Files)**

Direct hexdump from the start of your `/mnt/h/RO/client/data.grf` (common for all GRF access, including every .rsw/.gnd/.gat and .str):

```
00: 4d 61 73 74 65 72 20 6f 66 20 4d 61 67 69 63 00   "Master of Magic\0"
10: 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e af 52
20: b9 c0 00 00 00 00 1d 4e 02 00 00 02 00 00 28 89
30: bd 97 30 aa 24 0f 01 41 d1 1e 2a cd f8 70 e0 67
```

**Field mapping (from Korangar NativeArchive + ragnarok-formats):**
- 0x00-0x0F: Magic "Master of Magic"
- Scramble/seed bytes follow.
- File table offset (u32), compressed size, file count, version 0x200.
- Table is zlib compressed; contains name + offset/size/flags for every asset (maps, effects, sprites...).
- To walk a real map: find "data\prontera.rsw" (or similar) in table → seek + read bytes → parse as MapData (GRSW at start, version, ground/gat strings, resources list, quadtree).

Same for .str (look for STRM sig after table lookup).

This is the on-disk reality the official client and Korangar use for the entire asset pipeline from your H:\RO GRFs.

---

## Using This Document as a Diagnostic Guide & Functional True North for Korangar

**Core strength:** This is an excellent reference for the *data model and asset pipeline* that the real 2019-06-05 client uses. Most "Korangar doesn't look/act right" issues stem from:
- Wrong or incomplete parsing of a format (GRF entry, .rsw/.gnd/.gat, .spr/.act, .str, Lua tables).
- Incorrect transformation / composition (map offsets, object transforms, animation action indices).
- Missing inheritance or fallback logic (skill tree, job sprites, item names from EN data).

When you hit a problem:
1. Identify the symptom (bad model position on map? wrong skill icon? animation not playing attack? quest not showing?).
2. Trace back to the data source using the "Precise paths" and "Binary Formats" sections.
3. Compare Korangar code (linked throughout) against the official structures described here.
4. Use Korangar's built-in tools while testing:
   - Packet inspector (for live traffic vs. expected packets from server).
   - Render options (frustum culling, wireframe, bounding boxes, light culling).
   - Debug map inspector / state windows.
   - `KORANGAR_PACKET_LOG=1` + packet history.
5. Replicate the real client's behavior by following the exact loading/rendering steps documented (e.g., "apply_map_offset then build KDTree from object AABBs" for maps; "evaluate inherits then merge SKILL_TREEVIEW slots" for skill tree).

**Covered very well (high confidence as true north):**
- GRF loading & file table.
- Map composition & rendering (rsw/gnd/gat + placed objects + quadtree vs. actual KDTree culling).
- Sprite + action animation system (including attack timing via action indices).
- Effect (.str) layer/frame structure.
- Skill tree Lua (SKILL_TREEVIEW_FOR_JOB + scaffolding + inheritance).
- Item/skill/quest data sources (especially English variants in System/).
- Server DB/NPC cross-references for DM command behavior.

**Areas that are good but benefit from live verification:**
- Exact animation frame interpolation / timing (client tick vs. real exe).
- Full lighting model + lightmap application on ground/objects.
- UI/dialog/quest state machines (client-only flows).
- Edge cases in renewal vs. pre-renewal data or specific job sprites.

**How to keep it functional:**
- When adding a feature to Korangar, note the corresponding section here.
- When something breaks, open the official client + packet inspector side-by-side and cross-check against the documented structures.
- For completely new behaviors (custom DM UI), use the server scripts + this doc to decide what data the client needs to expose.

This document + Korangar's debug features + your live H:\RO server/client should let you diagnose "why doesn't Korangar match the real client here?" and implement the correct behavior.

If a specific class of issue keeps coming up (e.g., "animations feel off", "map objects are in the wrong place after warp", "skill tree tabs are wrong for a renewal job"), tell me and we can add even more targeted diagnostic steps or hexdump examples.

## Troubleshooting Guide: Missing NPC / Quest Marker ("!") with Instruction Text in Starting / Tutorial Area

### Symptom
- In the very first / starting area (e.g., novice grounds or initial map with a teleport/warp at the end), an NPC is supposed to appear and give instructions (text/dialogue).
- Official / earlier Korangar behavior: NPC visible + black-outlined "!" marker (quest indicator) above or at the location.
- Current Korangar build: No visible NPC sprite/entity at the spot, but the instruction text still appears.
- The "!" is a guide/quest marker, not part of the NPC body sprite.

This is a common "visual guidance" element to direct new players to the exit/teleport.

### How the Official Client Handles It (from H:\RO data + packets)

**Data in the client (GRFs + System/):**
- Quest/instruction markers use dedicated textures:
  `data\texture\유저인터페이스\minimap\quest_{effect_id}_{variant}.bmp`
  (e.g., `quest_1_1.bmp`). These contain the "!" (and ? variants) with built-in black outlines for screen visibility/contrast.
- NPC bodies (when present) are normal sprites loaded from:
  `data\sprite\npc\{name}` where `name` comes from `npcidentity.lub` (or `jobidentity.lub`) looked up by the `job_id` in the entity's packet data.
- Korean paths are the norm for UI/quest assets (`유저인터페이스` = user interface).

**Packets involved:**
- `AddEntity` (various headers) — sends the NPC as an entity with `job_id`, `position`, `entity_id`, etc. Type is derived from job_id range (typically 46-999 or 10000-19999 → Npc).
- `QuestEffectPacket` (header `0x0446`, 14 bytes):
  ```rust
  pub struct QuestEffectPacket {
      pub entity_id: EntityId,
      pub position: TilePosition,
      pub effect: QuestEffect,   // selects quest_{id}_*.bmp
      pub color: QuestColor,     // Yellow/Orange/Green/Purple tint
  }
  ```
  - `effect` id picks the texture variant.
  - Sent by server when the spot/NPC should show a marker (quest active, guide mode, etc.).
  - Can attach to a real NPC entity or a special/warp entity (job 45).

**Rendering in official client:**
- Body sprite: Loaded via sprite system (`data\sprite\npc\...` or monster path), rendered as part of entity (with animation data from .act).
- Marker: Separate 2D billboard sprite. World position = entity pos (or packet position) + small Y offset (~25 units). Projected to screen via camera view-projection, drawn with `render_sprite` (scaled ~30 units, tinted by color). The black outline comes from the .bmp asset.
- Text: Independent server output (chat packets, `EntityMessagePacket`, `NpcDialogPacket`, or area script trigger). Not tied to the visual marker.
- Markers can appear without a full body sprite (e.g., on warps or guide points).

**Server side (H:\RO\server reference):**
- The NPC or marker position is often a warp trigger or static entity in the map script.
- Server sends the entity + QuestEffectPacket based on player progress or always in tutorial areas.
- Text comes from the script (`dispbottom`, `mes`, announce, etc.).

See sections:
- "Quest / Instruction Markers" (textures + packet).
- "Binary Formats" (QuestEffectPacket, entity data).
- "Maps" (how positions and resources are handled).
- "Server-Side DB / NPC Cross-Reference".
- "Using This Document as a Diagnostic Guide".

### Why It Breaks in Korangar (Common Causes)

Korangar replicates the mechanism (see `world/particles/mod.rs`, `lib.rs` network event handling, texture loader):

1. **Quest icon creation** (`QuestIcon::new`):
   - Loads texture with the exact Korean path above.
   - Stores world position from packet + offset.
   - Previously used `.unwrap()` on load → silent failure if texture not found.

2. **Attachment & filtering**:
   - Icons stored by `entity_id` in `ParticleHolder::quest_icons`.
   - Old render logic:
     ```rust
     entities.iter()
         .filter_map(|entity| self.quest_icons.get(&entity.get_entity_id()))
         .for_each(|icon| icon.render(...));
     ```
     - Only drew icons if the entity was currently in the live `entities` list.
     - If the base entity (NPC or warp) was never added, culled, or filtered, the marker disappeared.

3. **Entity creation** (`AddEntity` handler in `lib.rs`):
   - Creates `Entity::Npc` via `Npc::new(...)` (checks map position validity via `get_world_position`).
   - Loads sprite parts via `get_entity_part_files` → `npc\{name}` or `몬스터\...`.
   - For warps/hidden (job 45/111): intentionally empty parts (no sprite, to avoid "missing" shadow).
   - Entity must survive to render time (not removed on map change, etc.).

4. **Texture / path issues**:
   - Korean paths + backslashes must resolve in loaded GRFs (data.grf + others).
   - UI textures sometimes in renewal/resources GRFs.
   - Lowercasing + path normalization in `TextureLoader` / `GameFileLoader` must succeed.

5. **Timing / packet order**:
   - QuestEffectPacket can arrive before/after AddEntity.
   - Marker position comes from packet (not always entity position at runtime).
   - Text packets are separate.

6. **Other**:
   - Map bounds / position invalid in current GAT/GND.
   - Quest effect cleared on map change without re-sending.
   - Rendering pass order (icons are in bottom_interface_renderer after world but use projected screen coords).

Result: Entity missing (or filtered) → no icon rendered, even if packet arrived. Text still shows because it's a separate server message.

### Diagnostic Steps (using this doc + Korangar tools)

1. **Packet capture** (primary tool):
   - Enable packet inspector or `KORANGAR_PACKET_LOG=1`.
   - Walk the area.
   - Look for:
     - `AddEntity` with job_id in NPC range (or 45 for warp) + matching position.
     - `QuestEffectPacket` (0x0446) with effect id (usually small number for !) and the entity_id/position.
   - Compare to what the official client receives (run side-by-side if possible).

2. **Texture verification**:
   - Confirm `유저인터페이스\minimap\quest_*.bmp` files exist in your loaded GRFs (use GRF Editor on Windows side).
   - Check logs for texture load failures.

3. **Entity presence**:
   - Use debug state inspector or render options (show entities, bounding boxes).
   - See if the entity_id from the packet appears in the live entities list.
   - Check `entity_type` (Npc vs Warp vs Hidden).

4. **Map data**:
   - Dump the relevant .rsw (via GRF tools) and look for objects/resources near the teleport position.
   - Verify GAT/GND allows the position (`get_world_position` succeeds).

5. **Quest state**:
   - Check if `AddQuestEffect` / `RemoveQuestEffect` events fire.
   - Look at `ParticleHolder.quest_icons` (debug) and whether icons are attached.

6. **Cross-check with official**:
   - Run the real client from H:\RO.
   - Note exact visual (sprite + marker), any overhead text, and packet traffic if captured.
   - Use the "Precise paths" and "Binary Formats" sections here to map what you see.

### Fixes Applied (to match official behavior)

In `korangar/src/world/particles/mod.rs`:

```rust
// Before (only for entities present in list)
entities.iter()
    .filter_map(|entity| self.quest_icons.get(&entity.get_entity_id()))
    .for_each(...);

// After (all active icons from packet data)
self.quest_icons.values()
    .for_each(|quest_icon| quest_icon.render(...));
```

- Quest icons now render unconditionally from the positions stored when the `QuestEffectPacket` arrived.
- This handles cases where the "NPC" is a warp, the body sprite isn't loaded, or entity timing differs.
- Matches official client (marker can stand alone as guide visual).

Also made texture loading safe:

```rust
let texture = match texture_loader.get_or_load(
    &format!("유저인터페이스\\minimap\\quest_{}_{}.bmp", effect_id, 1),
    ImageType::Color,
) {
    Ok(t) => t,
    Err(_) => { /* debug warn + return None */ }
};
```

- Prevents silent loss of the marker if path has encoding/GRF issues.

These changes ensure the `!` (with its asset-provided black outline) appears at the packet-provided position when the server intends it, replicating the official guide experience.

### Prevention & Similar Issues
- Always render effect markers from packet data (position is authoritative).
- Make asset loads for UI/quest elements robust (Korean paths, optional GRFs).
- For guide spots, treat warps + attached QuestEffect as valid "visual only" cases.
- Test with full GRF set from H:\RO (data + rdata + 2021 files) + English System/ data.
- When server scripts change (e.g., new instruction NPCs), ensure both AddEntity + QuestEffectPacket are sent.

Add similar examples to this guide as they arise. The rest of the document (especially packet formats, asset paths, map/object rendering, and the diagnostic workflow) is the reference material.

---

**Maintained for the Korangar + Hercules_RO "Seal Cascade" setup.**  
Last updated based on direct inspection of the full H:\RO install (2026-07).