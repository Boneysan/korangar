# Loaders & Asset Pipeline Internals

**Purpose**: Detailed internals of asset loading, GRF handling, async loading, and the Library system. This goes beyond the high-level decisions in `plans/asset-pipeline.md` and the reference mapping in `RO_OFFICIAL_CLIENT_STRUCTURE.md`.

**Audience**: Engine engineers who need to add new asset types, optimize loading, integrate custom DM content, or debug missing resources.

## Overall Architecture

Assets are loaded through two main paths:

1. **Synchronous / direct loaders** (`TextureLoader`, `SpriteLoader`, etc.) — used for immediate needs.
2. **AsyncLoader** (`src/loaders/async/mod.rs`) — the primary system for gameplay. Uses a rayon thread pool (1 thread currently) and request/callback model.

All loaders ultimately read from `GameFileLoader` (the archive abstraction).

Key crates/files:
- `korangar/src/loaders/`
  - `gamefile/` — archive management
  - `async/` — request system
  - `archive/` — folder + native + 7zip backends
  - Individual loaders: `sprite/`, `model/`, `texture/`, `map/`, `font/`, `animation/`, etc.
- `src/world/library/` — queryable data (items, skills, jobs) loaded from Lua/GRF.
- `src/state/` uses async requests heavily.

## Archive / GameFile System

`GameFileLoader` (in `loaders/gamefile/`) manages multiple archives with priority order.

Archives can be:
- GRF files (via `archive/native/`)
- 7z files
- Loose folder (`archive/folder/`)
- Built-in `korangar/archive/data/`

Priority is defined in `client/game_archives.ron` (lowest to highest in the array; loader inserts at front).

Loading a file:
```rust
let data = game_file_loader.get("data/sprite/...")
    .ok_or(LoadError::FileNotFound)?;
```

Mixcrypt (native GRF encryption) is handled in `archive/native/mixcrypt.rs`.

**Extension points**:
- Add new archive backend by implementing the `Archive` trait.
- Custom overrides go in the highest-priority entry (usually a folder or the built-in archive).

## Async Loading Pattern

`AsyncLoader` is the central coordinator.

Core idea:
- Request by `LoaderId` (e.g. `AnimationData(EntityId)`, `Map(String)`).
- If already loaded/cached → return immediately.
- Otherwise schedule work on the thread pool.
- Store `LoadStatus::Loading` / `Completed` / `Failed`.
- Caller (usually entity or window) polls or receives `Arc<...>` later.

Example from code (animation):

```rust
pub fn request_animation_data_load(...) -> Option<Arc<AnimationData>> {
    if let Some(cached) = ... { return Some(cached); }

    self.request_load(LoaderId::AnimationData(entity_id), move || {
        // heavy work: load sprite + actions → build AnimationData
        let animation_data = animation_loader.load(...)?;
        Ok(LoadableResource::AnimationData(animation_data))
    });
    None  // will be available on next frame(s)
}
```

The actual work is done in closures passed to `request_load`.

**Pending loads** live in a `Mutex<HashMap>`.

When work completes, the main thread picks it up in the next update and inserts into the appropriate loader cache.

## Individual Loaders

### TextureLoader
- Supports PNG, BMP, TGA, JPG.
- Builds texture atlases / sets via `TextureSetBuilder`.
- Used by almost everything (UI, sprites, models, effects).
- Fallbacks defined as constants (`FALLBACK_*.png` etc.).

### SpriteLoader + ActionLoader
- `.spr` + `.act` pairs.
- `Sprite` contains frames + palette.
- `Actions` contain animation sequences (with delays, sounds, etc.).
- Heavily used for entities and skills.

### ModelLoader
- `.rsm` / `.rsm2` (Ragnarok model format).
- Produces meshes with vertex data for wgpu.
- Normals are smoothed in some cases (`smoothing.rs`).

### MapLoader
- Parses RSW (objects + lights), GND (ground mesh), GAT (walkability).
- Builds collision KDTree for picking/pathing.
- Water planes, lighting data, etc. are produced here.
- See `WORLD_MAPS_ENTITIES.md` for high-level usage.

### AnimationLoader
- Combines sprite + action data into `AnimationData`.
- Used by the entity animation system.
- Caches per entity part list.

### FontLoader
- Bitmap fonts + color span parsing.
- Supports colored text via `^RRGGBB` codes (RO style).
- `color_span_iterator.rs` for rich text.

### Effect / Video / etc.
- `.str` effect files.
- Video via `korangar-video` (IVF?).

## The Library System

`src/world/library/mod.rs` + submodules.

Purpose: Provide fast, typed lookups for game data (item names, skill requirements, job identities, etc.).

Many entries are populated from Lua files extracted from GRF (or overrides in `korangar/archive/data/lua-scaffolding/`).

Example:
- `ItemNameKey { item_id, is_identified }` → `ItemName`
- Job identities for NPC sprites.

**DM relevance**: Campaign quests, custom items, and monster data are merged here (see `tools/campaign_quest_merge.py` mentioned in project docs).

To add custom DM data:
- Place files in the highest-priority archive.
- Or extend the library at runtime after loading.

## Caching & Resource Lifetime

- Most loaders have internal `HashMap` caches (keyed by path or composite key).
- `ResourceMetadata` is used for UI (name + texture) and is populated asynchronously.
- `Arc<...>` is used heavily so data can be shared between world entities, UI, and renderer without cloning.

## Error Handling & Fallbacks

Every loader has graceful degradation:
- Missing sprite → `npc\\missing.spr`
- Missing model → `missing.rsm`
- etc.

See constants in `loaders/mod.rs`.

`LoadError` enum in `loaders/error.rs`.

## Performance & Threading Notes

- Async loader uses a single worker thread (`num_threads(1)`).
- Heavy work (decompression, parsing large models) is off the main thread.
- Main thread only does the final insertion and GPU upload.
- Texture uploads happen on the render thread via `TextureLoader`.
- Profiling hooks (`#[cfg(feature = "debug")]`) exist for load times.

For very large maps or many entities, the pending load map can grow — this is why `request_*` methods return `Option` (data may arrive later).

## Extension Points for Engineers

1. **New asset type**:
   - Create `XxxLoader` similar to `SpriteLoader`.
   - Register in `AsyncLoader`.
   - Add a `LoaderId` variant.
   - Add a `LoadableResource` arm.
   - Expose request method.

2. **Custom DM content pipeline**:
   - Use folder archive with highest priority.
   - Populate `Library` entries at startup.
   - For dynamic data, use `[DMJ]` echoes + parser updates in `dm/` module.

3. **Optimizations**:
   - Increase worker threads (careful with CPU on WSL).
   - Preload critical assets on map change.
   - Add LRU eviction for very long sessions.

4. **Debugging missing assets**:
   - Use the state inspector on `ClientState`.
   - Packet log + async loader debug prints.
   - Check archive priority order.

## Cross References

- `plans/asset-pipeline.md` (high-level decisions, GRF order)
- `RO_OFFICIAL_CLIENT_STRUCTURE.md` (what lives in System/, itemInfo, etc.)
- `WORLD_MAPS_ENTITIES.md` (map loading specifics)
- `DM_CLIENT_IMPLEMENTATION.md` (custom content for campaign)
- `CLIENT_SYSTEMS_OVERVIEW.md` (loaders section)

Reading `loaders/async/mod.rs` + `loaders/gamefile/mod.rs` + one concrete loader (e.g. `sprite/mod.rs`) + the Library will give a complete picture.
