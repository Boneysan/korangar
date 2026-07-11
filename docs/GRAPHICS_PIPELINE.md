# Korangar Graphics Pipeline

This document provides both a high-level overview and deep technical details of Korangar's rendering architecture. It is intended as a reference for understanding the current implementation and for improving subsystems (especially the real-time lighting engine).

**Key characteristics:**
- Built on **wgpu** (Vulkan primary on Linux/WSL, Metal/DX12 elsewhere; OpenGL fallback with limitations).
- Shaders written in **Slang** (compiled to SPIR-V at build time via `slangc` in `build.rs`).
- Modern multi-pass forward renderer with **tiled light culling**, cascaded directional shadows, point light shadows, and SDSM (Sample Distribution Shadow Maps).
- Heavy use of bindless resources where the adapter supports it.
- Designed for many dynamic lights + real-time shadows while staying compatible with the 2019 RO data set.

## High-Level Architecture

### Core Components

| Component              | Location                          | Responsibility |
|------------------------|-----------------------------------|--------------|
| `GraphicsEngine`       | `src/graphics/engine.rs`         | Owns device/queue/surface, creates all pass contexts + drawers/dispatchers, manages frame lifecycle. |
| `GlobalContext`        | `src/graphics/` (via engine)     | Global bind group + resources (uniforms, samplers, shadow maps, etc.). |
| `RenderInstruction`    | `src/graphics/instruction.rs`    | CPU-side collection of everything that needs to be drawn this frame (models, entities, lights, UI, etc.). |
| Pass Contexts          | `src/graphics/passes/*/mod.rs`   | Bind group layouts and resources specific to a render/compute pass. |
| Drawers / Dispatchers  | `src/graphics/passes/*/`         | Actual pipelines + `dispatch()` / `draw()` logic. |
| `PointLightManager`    | `src/world/light/mod.rs`         | Collects, scores, and manages dynamic point lights (shadow assignment, fading). |
| Shaders                | `shaders/` (Slang)               | All GPU code. Organized into `modules/` (reusable) and `passes/` (entry points). |

### Per-Frame Flow (High Level)

1. **Collect** (`lib.rs` + world systems)
   - `point_light_manager.prepare()`
   - Map, entities, effects, particles, UI, etc. push instructions into `RenderInstruction`.

2. **Prepare + Upload**
   - Each context/drawer implements `Prepare`: `prepare(device, instructions)` then `upload(device, staging_belt, encoder)`.
   - Global uniforms, per-pass data, resource tables are staged.

3. **Dispatch Passes** (in `render_geometry` / main render functions)
   - SDSM passes (compute: compute/reduce partitions and bounds)
   - Shadow passes:
     - Directional shadow (cascades)
     - Point shadows (for lights that need them)
   - Light culling (compute): tiles the screen and builds `tile_light_indices`
   - Forward pass (main scene + entities + ground + water + effects)
   - Post-processing (FXAA, WBOIT resolve, etc.)
   - Interface / UI (separate render pass)
   - Screen blit / present

4. **Present**

The pipeline is deliberately split so that expensive work (shadows, culling) can be done early, and the forward pass can be relatively lightweight thanks to pre-culled lights.

## Bind Group Strategy (Important for Lighting Work)

From `graphics/mod.rs`:

```text
Set 0: Global Bindings          (view matrices, camera, ambient, point_light_count, etc.)
Set 1: Pass Bindings            (e.g. shadow matrices for a specific shadow pass)
Set 2: Dispatcher / Drawer      (e.g. tile_light_indices buffer, indirection tables)
Set 3: Resource Bindings        (per-map or per-model texture sets, bindless where possible)
```

This layout is consistent across most passes. When adding new lighting features (more light types, clustered culling, etc.), you will almost always touch Set 0 (global uniforms) and Set 2 (the culling/dispatch data).

## Lighting & Shadows Deep Dive

This dedicated section expands on the real-time lighting and shadowing system. It is the foundation for "real lighting" in Korangar and the primary area for future enhancements (dynamic time-of-day, more lights, better quality, performance, etc.).

The system combines:
- Per-map baked directional + ambient lighting from official RO map data.
- Dynamic point lights with tiled culling.
- Real-time shadow mapping (directional via SDSM + point light cube shadows).
- Evaluation in the forward pass using culled lights + shadow visibility.

### Exact Uniform Structs (CPU/GPU)

**GlobalUniforms** (`korangar/src/graphics/mod.rs`):
```rust
#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct GlobalUniforms {
    view_projection: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    inverse_view: [[f32; 4]; 4],
    inverse_projection: [[f32; 4]; 4],
    inverse_view_projection: [[f32; 4]; 4],
    indicator_positions: [[f32; 4]; 4],
    indicator_color: [f32; 4],
    ambient_color: [f32; 4],
    camera_position: [f32; 4],
    forward_size: [u32; 2],
    interface_size: [u32; 2],
    pointer_position: [u32; 2],
    animation_timer: f32,
    point_light_count: u32,
    enhanced_lighting: u32,
    shadow_method: u32,
    shadow_detail: u32,
    use_sdsm: u32,
}
```
- `ambient_color`: from current map.
- `point_light_count`: number of active point lights (with + without shadows).
- `enhanced_lighting`: 0 = classic (no/minimal point lights), 1 = full.
- `shadow_method`: 0 = basic SampleCmp, 1 = PCF, 2 = PCF+PCSS.
- `shadow_detail`: controls tap count / quality.
- `use_sdsm`: enables Sample Distribution Shadow Maps for directional.

**DirectionalLightUniforms** (`korangar/src/graphics/mod.rs`):
```rust
#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct DirectionalLightUniforms {
    view_projection: [[f32; 4]; 4],
    color: [f32; 4],
    direction: [f32; 4],
}
```

**DirectionalLightPartition** (per-cascade data):
```rust
#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct DirectionalLightPartition {
    view_projection: [[f32; 4]; 4],
    interval_end: f32,
    world_space_texel_size: f32,
    near_plane: f32,
    far_plane: f32,
}
```

**PointLightData** (uploaded for active lights):
```rust
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct PointLightData {
    position: [f32; 4],
    color: [f32; 4],
    range: f32,
    texture_index: i32,  // 0 = no shadow, else 1-based cube index
    padding: [u32; 2],
}
```

**TileLightIndices** (output of culling, input to forward):
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct TileLightIndices {
    indices: [u32; 256],
}
```

These are prepared in `GlobalContext::prepare` / `upload` and bound via the global bind group.

**Map Lighting Source** (`korangar/src/world/map/lighting.rs`):
```rust
pub struct Lighting {
    ambient_color: Color,
    diffuse_color: Color,
    light_latitude: f32,
    light_longitude: f32,
}

impl Lighting {
    pub fn new(settings: LightSettings) -> Self { ... }
    pub fn ambient_light_color(&self) -> Color { ... }
    pub fn directional_light(&self) -> (Vector3<f32>, Color) {
        let rotation_around_x = Matrix3::from_angle_x(Deg(-self.light_latitude));
        let rotation_around_y = Matrix3::from_angle_y(Deg(self.light_longitude));
        let light_direction = rotation_around_y * (rotation_around_x * Vector3::new(0.0, 1.0, 0.0));
        (light_direction, self.diffuse_color)
    }
}
```
This is populated from the map's RSW `LightSettings` (see `ragnarok-formats`).

### Shader Paths

- **Globals + shared lighting math**: `korangar/shaders/modules/globals.slang`, `forward.slang`, `directional_shadow.slang`, `sdsm.slang`, `point_shadow.slang`, `depth_texture.slang`
- **Light culling compute**: `korangar/shaders/passes/light_culling/light_culling.slang`
- **Directional + SDSM**: `korangar/shaders/passes/directional_shadow/*.slang` + `sdsm/*.slang`
- **Point shadows**: `korangar/shaders/passes/point_shadow/*.slang`
- **Forward evaluation (shadows + lighting + BRDF)**: 
  - `korangar/shaders/passes/forward/model_bindless.slang`
  - `korangar/shaders/passes/forward/entity_bindless.slang`
  - `korangar/shaders/passes/forward/indicator.slang`
  - `korangar/shaders/modules/forward.slang` (core helpers)

In the forward shaders, lighting is evaluated after base material sampling and before post-effects.

### Exact Shader Code: Shadow Sampling + Lighting/BRDF

**Shadow helpers** (excerpts from `korangar/shaders/modules/forward.slang`):

```slang
[ForceInline]
public func get_pcf_shadow(...) -> float { ... }  // bilinear or Gaussian PCF via SampleCmp

[ForceInline]
public func get_pcf_pcss_shadow(...) -> float {
    // Blocker search using unfiltered reads
    let blocker = find_blocker(...);
    ...
    let penumbra = ...;
    return pcf_filter(..., penumbra, ...);
}

[ForceInline]
public func find_blocker(...) -> float2 { ... }  // samples kernel, averages blocker depths

[ForceInline]
public func pcf_filter(...) -> float { ... }
```

**Core lighting + BRDF** (from `korangar/shaders/passes/forward/model_bindless.slang`, simplified):

```slang
let normal = normalize(input.normal);
var ambient_light_contribution = global_uniforms.ambient_color.rgb;

let light_direction = normalize(-directional_light.direction.xyz);
let light_percent = max(dot(light_direction, normal), 0.0);

// ... bias shadow_coords using normal/light offsets + depth_bias ...

var visibility: float = 1.0;
switch (global_uniforms.shadow_method) {
    case 0: visibility = shadow_maps.SampleCmp(...);
    case 1: visibility = get_pcf_shadow(...);
    case 2: visibility = get_pcf_pcss_shadow(...);
}
visibility *= shadow_translucence.SampleLevel(...).r;

let directional_light_contribution = directional_light.color.rgb * light_percent * visibility;

var point_light_contribution = float3(0.0);
for (var index = 0; index < light_count; index++) {
    let light = point_lights[light_index];
    let light_dir = normalize(...);
    let light_percent = max(dot(light_dir, normal), 0.0);
    let dist = length(...);
    var vis = 1.0;
    if (light.texture_index != 0) {
        // depth compare against point_shadow_maps_unfiltered
    }
    let atten = calculate_attenuation(dist, light.range);
    point_light_contribution += (light.color.rgb * intensity) * light_percent * atten * vis;
}

let base_color = diffuse_color.rgb * input.color;
let light_contribs = saturate(ambient + directional + point);
var color = base_color.rgb * light_contribs;

if (global_uniforms.enhanced_lighting == 0) {
    color = color_balance(color, -0.01, 0.0, 0.0);
}
fragment_color = float4(color, diffuse_color.a);
```

This is a simple diffuse (Lambert) model. `calculate_attenuation` is quadratic falloff clamped to range.

Tile lights come from the culling pass output:
```slang
let light_count = light_count_texture.Load(...);
for (...) {
    let light_index = tile_light_indices[tile_index].indices[index];
    ...
}
```

### How Night Maps Differ in the Data

Night (or dim) maps are **not** handled by any runtime time-of-day simulation. All lighting parameters come from the map's static data:

- In the official RO data (inside GRFs), each map's `.rsw` file contains a `LightSettings` block with:
  - `ambient_color` (often very dark for night maps)
  - `diffuse_color` (weaker sun/moon tint)
  - `light_latitude` / `light_longitude` (different sun elevation/azimuth; night maps typically have lower/negative effective light)
- Korangar loads these verbatim into `Lighting` (see `world/map/lighting.rs:Lighting::new`).
- When a map loads, `map.ambient_light_color()` and `map.directional_light()` return the baked values for *that map only*.
- "Night maps" in the dataset simply ship with darker/weaker values in their RSW. Examples include certain versions of maps at night or custom low-light areas.
- Dynamic elements (point lights from torches, effects) are layered on top via the point light system.

In the forward shaders, `global_uniforms.ambient_color` comes straight from the current map's data. There is no global "night factor" or sun angle animation.

### Making Shadow Count / Resolution / Night Behavior More Dynamic

**Current state (mostly static/init-time):**
- Shadowed point light count: hardcoded `const NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS: usize = 6;` in `lib.rs`. Used for buffer sizing and `create_point_light_set`.
- Resolution: chosen at `GlobalContext` creation time from `ShadowResolution` (see `graphics/settings.rs`). Textures are created once; runtime changes require context recreation.
- Night: purely data-driven per map. No overrides or animation.

**Concrete steps to make them dynamic:**

1. **Shadow count (number of point lights with shadows)**:
   - Replace the const with a runtime value (e.g., from `GraphicsSettings` or a new cap).
   - Dynamically size the cube array in `create_point_shadow_textures` and the `PointLightData` buffer (currently sized for 128).
   - Pass the active count via `GlobalUniforms.point_light_count` (already done) and update shader array bounds or use dynamic loops.
   - In `PointLightManager`, make `create_point_light_set(max)` take a parameter instead of the const.
   - Update bind groups and `GlobalContext` on change (similar to resolution).

2. **Shadow resolution**:
   - Add a `set_shadow_resolution` method that recreates the relevant `AttachmentTexture`s and `CubeArrayTexture` (directional + point).
   - Invalidate/recreate dependent views, samplers, and partition data.
   - Re-upload any dependent uniforms (world_space_texel_size etc. come from the camera, which can be updated).
   - For SDSM, the texel size calculations adapt automatically.
   - Expose in the graphics settings UI and call from `lib.rs` when the setting changes (see existing `set_shadow_resolution` skeleton).

3. **Night / time-of-day behavior**:
   - Keep per-map base `Lighting` but allow runtime overrides (e.g., a `TimeOfDay` struct or scalar `night_factor`).
   - In `lib.rs` (when building `RenderInstruction` / uniforms):
     ```rust
     let (dir, color) = map.directional_light();
     let ambient = map.ambient_light_color() * (night_factor > 0.0 ? 0.3 : 1.0);
     let dir_color = color * sun_intensity(night_factor);
     ```
   - Animate `light_latitude/longitude` or blend to a "moon" direction/color for true night.
   - Drive the factor from server packets (for DM campaign control) or client time.
   - For full dynamism, allow overriding the entire `DirectionalLightInstruction` and ambient per-frame.
   - Update the directional shadow camera with the (possibly animated) direction every frame.
   - Expose live controls in render options or a new DM debug panel.

These changes touch:
- `src/world/light/mod.rs` + `src/world/map/lighting.rs`
- `src/graphics/mod.rs` (GlobalContext + buffers)
- `src/lib.rs` (uniform + instruction building + LightingMode)
- Shaders (if you need dynamic array sizes instead of constants)
- Settings + UI

See `docs/GRAPHICS_PIPELINE.md` sections on "How to Improve the Lighting System" and the uniform/shader excerpts above for the exact call sites. Start with making the point-light shadow count a setting and thread it through `create_point_light_set`.

This gives a complete, code-accurate reference for working on the lighting system.

When you want to extend or fix lighting, these are the touch points:

1. **Add more lights or better culling**
   - Extend `PointLightManager`
   - Modify the light culling compute shader (or replace with clustered)
   - Update `tile_light_indices` layout and the readers in forward shaders

2. **Better shadows**
   - Tweak SDSM compute passes (`sdsm/`)
   - Change shadow resolution / partition count (also update shader constants)
   - Improve PCF or add variance/soft shadows (requires changes in both shadow and forward shaders + possibly extra bindings)

3. **New lighting features** (IBL, area lights, etc.)
   - Usually means new global uniforms or a new bind group entry
   - New module in `shaders/modules/`
   - Possibly a new compute pre-pass

4. **Performance**
   - The light culling dispatch size is calculated in `calculate_dispatch_size`.
   - Bindless vs non-bindless paths exist for older hardware.
   - Many passes have separate "bindless" shader variants.

## Shader Organization (Slang)

- `modules/`: Reusable code (globals, matrix math, transform, lighting helpers, forward lighting functions, shadow sampling, etc.).
- `passes/`: Concrete entry points. Each sub-folder usually contains variants for:
  - Model vs Entity
  - Bindless vs non-bindless
  - Debug / indicator / wireframe variants

Compilation happens in `build.rs` → SPIR-V is embedded or loaded at runtime via `ShaderCompiler`.

See `korangar/shaders/README.md` for the basic folder rules.

## Useful Debug Tools

- Render Options window (`interface/windows/render_options.rs`):
  - Toggle individual lighting components
  - Show light culling count buffer
  - Frustum culling, bounding boxes, etc.
- Debug inspectors for lights, maps, effects.
- The various debug_* drawers in postprocessing.

## Current Limitations / Gotchas (as of 2026)

- Light culling is tiled (screen-space 16×16). No clustered or 3D culling yet.
- Number of shadowed point lights is a compile-time constant in several places.
- GL backend has restrictions (no texture arrays in some ways, MSAA resolve issues historically).
- Some raw depth reads for shadows were added specifically to work around GL limitations (separate non-comparison bindings).

## Next Steps for Documentation / Work

If you're improving lighting:

1. Start in `world/light/mod.rs` + the light culling shader.
2. Trace through `RenderInstruction` → forward shader.
3. Look at how `GlobalUniforms` and the tile buffer are bound.
4. Test changes with the light culling debug buffer visible.

### Aspirational implementation order (Seal Cascade fork)

The project roadmap authorizes major lighting improvements, but they should land
in dependency order:

1. Add non-destructive artistic profiles over existing RSW lighting.
2. Add interpolated, recoverable campaign scene-light overrides.
3. Establish representative screenshot/performance regression scenes.
4. Move the world scene to a linear floating-point HDR target; add exposure and
   tone mapping before bloom.
5. Add opt-in emissive materials and restrained HDR-driven bloom.
6. Prototype subtle contact shadows/SSAO and reject it if sprite halos or dirty
   painted textures outweigh the depth benefit.
7. Unify lighting/shadow/AO/bloom settings into backend-aware quality presets.
8. Generalize scene transitions into weather and time-of-day only after the
   campaign controls are stable.

Do not assume the existing sRGB path is incorrect; it already uses sRGB texture
formats and surface-aware presentation. Do not prioritize full PBR, IBL,
clustered lighting, or shadow-selection rewrites without captured evidence that
the current diffuse/tiled/hysteresis design is the limiting factor.

### Broader modern-graphics extension map

The aspirational graphics program remains feasible in this engine because it
maps onto existing systems rather than requiring replacement:

| Improvement | Current extension point |
|---|---|
| Sky, distance/height fog, weather grading | `MapSkyData`, global uniforms, forward/post passes |
| Better water | `WaterWaveDrawer`, `WaterInstruction`, forward wave shader |
| Soft/lit/emissive particles | particle renderer, depth texture, point-light manager |
| SMAA/upscaling/sharpening | existing post-processing and screen-blit pass chain |
| Selection/target outlines | picker ID/depth buffers plus a focused post/forward pass |
| Decals and hazard telegraphs | bounded render instructions and a focused forward/decal drawer |
| Cinematic camera and obstruction | existing camera abstraction, picker/collision data |
| Movement/animation smoothing | entity presentation state after authoritative network updates |
| Wind and precipitation | model vertex uniforms plus particle/effect systems |
| Optional texture/material overlays | `GameFileLoader`, texture cache, bindless resource sets |
| Transparency polish | existing WBOIT accumulation/revealage path |
| LOD/culling and budgets | map KD-tree/frustum culling, instruction collection, pass timings |
| Screenshot/regression tools | offscreen targets, SSAA, debug render options |

Each slice must retain classic assets and settings as a fallback, define Low to
Ultra cost, include a reduced-motion/effects option where relevant, and pass
comparison scenes before becoming a default. The preferred architectural shape
is a small data/state addition plus a new or extended context/drawer/pass—not a
parallel renderer.

## Sprite And Entity Lighting

RO `SPR`/`ACT` entities are not rendered as unlit overlays. Players, monsters,
NPCs, equipment parts, and ground-item sprites participate in the forward
lighting pipeline through `shaders/passes/forward/entity*.slang`.

### Current behavior

Sprites currently:

- receive map ambient and directional lighting;
- receive cascaded directional-shadow visibility;
- receive tile-culled dynamic point lights and point-shadow visibility;
- apply per-entity color and alpha modulation;
- split opaque and transparent pixels, with transparent output using WBOIT;
- cast alpha-tested directional and point-light shadows through the dedicated
  entity shadow drawers.

Because legacy sprite art has no geometry normals, normal maps, or material
properties, the vertex shader synthesizes a camera-relative normal across the
billboard width. Camera code also supplies per-frame-part depth and curvature.
The shader offsets fragment depth as if the upright card had shallow volume.
This improves body/hair/equipment overlap, depth testing, lighting variation,
and shadow placement without changing `SPR`/`ACT` assets.

### Strengths

- Sprites inhabit the same ambient, sun, spell, torch, and shadow environment
  as map geometry.
- The curved-normal/depth approximation adds dimension while retaining complete
  legacy asset compatibility.
- Both bindless and non-bindless shader paths implement the same model.
- Sprite transparency participates in the existing order-independent
  transparency system.

### Known limitations and validation needs

- **Synthetic shape:** every painted surface on a sprite shares the same broad
  billboard-curvature assumption. Faces, cloth, armor, and weapons can appear
  cylindrical or change implausibly as the camera rotates.
- **Double shading:** RO art already contains painted highlights and shadows.
  Strong Lambert lighting can darken faces, distort class colors, or reduce
  pixel-art readability.
- **Limited range:** ambient + directional + point contributions are saturated
  before multiplying sprite color, and point-light intensity is currently a
  fixed shader constant (`10.0`). Multiple bright lights can clip detail rather
  than produce a graded response. The planned HDR pipeline removes the range
  limitation but does not replace sprite-specific tuning.
- **Cutout shadows:** sprite shadow passes are primarily alpha-tested; partial
  transparency does not produce nuanced volumetric/translucent shadowing.
- **Grounding:** a billboard can still appear detached at its feet even while
  receiving and casting technically correct shadows.
- **Point-light convention audit:** the entity shader constructs its point-light
  vector as adjusted surface position minus light position. Do not change the
  sign by inspection alone because synthesized normal conventions may compensate.
  Capture controlled front/back/left/right/above tests for both sprites and map
  models, then correct only if the illuminated side is observably inconsistent.
- **Worst-case scenes:** validate pale and dark costumes, faces, translucent
  wings, multi-part equipment, large bosses, ground items, nighttime maps, and
  multiple colored spell lights.

### Design goal

Do not pursue physically realistic sprite materials as the default. The target
is to ground sprites in the scene while preserving authored colors, faces,
silhouettes, animation timing, and pixel-art readability.

### Aspirational sprite-lighting plan

1. **Regression rig:** fixed camera and sprite/model subjects with controllable
   front/back/left/right/above directional and point lights; screenshot and GPU
   baselines across primary backends.
2. **Lighting modes:** expose `Classic` (ambient/map tint), `Soft` (wrapped or
   half-Lambert), and `Enhanced` (full current response), plus advanced custom
   tuning. Preserve an unambiguous legacy fallback.
3. **Soft default evaluation:** test whether wrapped diffuse response preserves
   painted detail better than the current full Lambert response before changing
   defaults.
4. **Luminance-preserving response:** allow colored scene light to tint and
   ground sprites without crushing or clipping the artwork's authored value
   structure.
5. **Configurable curvature:** tune synthesized normals separately for players,
   NPCs, ordinary monsters, large bosses, and ground items if captured scenes
   show one curve cannot serve all categories.
6. **Readable ambient floor:** provide a sprite-specific minimum/readability
   control for dark scenes. Essential characters and combat silhouettes remain
   visible without flattening the environment.
7. **Contact grounding:** prototype a subtle foot/contact shadow or sprite-safe
   contact-depth treatment before adopting heavy scene-wide AO.
8. **Optional emissive overlays:** custom assets may add eyes, runes, weapons,
   ghosts, seals, or boss-phase emission. Legacy sprites require no new maps.
9. **Coverage-aware shadows:** evaluate stable dithering/coverage for soft sprite
   edges and reject it if animation noise outweighs the benefit.
10. **Lighting-independent outlines:** target/ally/hostile/interactable/DM
    selection outlines use picker/depth information and icon/shape cues so
    gameplay remains readable in dark or color-limited scenes.

All modes must define reduced-effects behavior, avoid color-only meaning, work
with opaque and WBOIT sprite pixels, and remain mechanically identical. Server
authority, `SPR`/`ACT` timing, and entity coordinates are unchanged.

### Feasibility tiers

The current wgpu/Slang forward renderer is sufficient for the modernization
program. These tiers describe implementation risk, not product priority.

**High feasibility — incremental extensions:**

- artistic map profiles and campaign lighting;
- height/distance fog and color grading;
- optional emissive overlays;
- picker/depth-based hover and target outlines;
- batched ground telegraphs (rings, cones, paths);
- weather particles, effect budgets, and reduced-effects controls;
- clean-HUD and supersampled screenshots;
- sharpening, quality presets, and pass/GPU diagnostics.

**Medium feasibility — focused renderer projects:**

- floating-point HDR scene targets, exposure, and tone mapping;
- HDR-driven bloom;
- subtle SSAO/contact shadows;
- depth-aware water and soft particles;
- SMAA and optional spatial upscaling;
- projected decals;
- camera obstruction handling;
- authoritative movement/animation presentation interpolation;
- wind-reactive optional assets, LOD, dynamic resolution, and weather surface
  reactions.

**Experimental or evidence-gated:**

- temporal AA (motion vectors, jitter, history, disocclusion, sprite ghosting);
- screen-space reflections;
- full dynamic day/night across legacy maps;
- volumetric fog/light shafts;
- full PBR conversion and IBL;
- clustered lighting, deferred rendering, ray tracing, virtualized geometry, or
  engine replacement.

The final group is technically possible in varying degrees but offers poor
value until captured scenes demonstrate a limitation that the current
diffuse/tiled/forward architecture cannot solve more simply.

### Dependency order

```text
Artistic profiles
    -> Campaign lighting and fog
        -> Weather and time transitions

HDR scene target
    -> Exposure and tone mapping
        -> Emissive materials
        -> Bloom

Reliable scene depth
    -> Contact shadows / SSAO
    -> Soft particles
    -> Improved water
    -> Depth-aware outlines and obstruction presentation

Stable authoritative presentation state
    -> Movement interpolation
    -> Animation smoothing
    -> Cinematic camera tools
```

Do not implement a dependent effect first with temporary color/depth/state
assumptions that will be discarded by its prerequisite.

### Cross-backend requirements

Vulkan, Metal, and DX12 are the primary modernization targets. OpenGL fallback
is the limiting backend and must receive a safe reduced path rather than block
all progress.

Every graphics slice must define:

- adapter/capability checks and backend-aware defaults;
- a cheap fallback or clean disable path;
- bounded textures, buffers, passes, and per-frame work;
- behavior without bindless resources where applicable;
- Slang translation validation on supported backends;
- Low/Medium/High/Ultra cost and visible-quality expectations;
- stable UI compositing and mechanically complete reduced-effects behavior.

Examples: HDR can fall back to the current SDR path; AO can disable; bloom can
use smaller buffers; shadowed-light count can shrink; telegraphs retain simple
geometry even when richer decals/effects are unavailable.

### Realistic delivery shape

- **Small-to-medium slices:** artistic profiles, basic campaign lighting, fog,
  outlines, screenshots, diagnostics, and unified presets.
- **Medium renderer projects:** HDR/tone mapping, emissive+bloom, AO, improved
  particles, water, decals, and presentation interpolation.
- **Long experiments:** TAA, volumetrics, sophisticated reflections, or broad
  weather/time simulation.

Engineering time, art direction, legacy-asset behavior, and cross-backend
validation are the practical constraints. The engine architecture itself is not
a blocker.

This document will be updated as the pipeline evolves. The best "source of truth" remains the code + the Slang shaders themselves.

---

**Related files to explore:**

- `src/graphics/mod.rs` (constants, uniforms, Prepare trait)
- `src/graphics/engine.rs`
- `src/lib.rs` (search for `render_geometry`, `point_light`, `light_culling`)
- `src/world/light/mod.rs`
- `src/world/map/mod.rs` (how lights are registered from maps)
- `shaders/modules/forward.slang` + `light_culling.slang`
- `shaders/passes/light_culling/`, `sdsm/`, `forward/`

Feel free to ask for deeper dives on any specific pass or for help adding new sections here.
