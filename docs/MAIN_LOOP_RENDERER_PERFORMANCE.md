# Main Loop, Renderer Separation & Performance Characteristics

**Purpose**: End-to-end view of how a frame is produced, how the world renderer and interface renderer are separated, and performance considerations. This is the "big picture" that ties the other deep dives together.

**Audience**: Engine engineers who want to add new passes, optimize, understand frame timing, or debug performance/stutter.

## High-Level Frame Structure

The core of the game lives in `korangar/src/lib.rs` in the `Client` struct.

Main update methods (called from the winit event loop or a fixed-tick driver):

1. `update_client_state()`
2. `update_settings()`
3. Input processing
4. Network event handling (`handle_network_events`)
5. User input processing (`process_user_events`)
6. World simulation (entities, particles, cameras, pathing)
7. Rendering preparation
8. Actual GPU submission

The loop is roughly:

```rust
// Per frame
self.update_client_state();
self.update_settings();

// Handle network
self.handle_network_events(client_tick);

// Process input
let input_report = self.input_system.update_delta(...);
self.process_user_events(&input_report, ...);

// Simulate world
self.update_world(client_tick, delta_time);

// Prepare render instructions
self.prepare_render_instructions(...);

// Submit
self.graphics_engine.render_frame(...);
```

## Renderer Separation

There are two largely independent rendering paths:

### 1. World / 3D Renderer (`src/graphics/` + `src/renderer/game_interface.rs` ?)

- Multi-pass forward renderer.
- Bind groups 0–3 (globals, per-pass, per-draw, etc.).
- Tiled light culling (compute).
- SDSM (or PSSM) directional shadows.
- Point light cube shadows.
- Opaque + transparent (WBOIT?) forward pass.
- Post-processing (FXAA, etc.).
- See `GRAPHICS_PIPELINE.md` for the full pass list and shader structure.

Key types:
- `GraphicsEngine`
- `RenderInstruction` / batches
- `GlobalUniforms`, `DirectionalLightUniforms`, etc.

### 2. Interface / 2D Renderer

- Completely separate from the 3D world.
- Uses its own `GameInterfaceRenderer` (or similar).
- Renders after or composited with the world.
- Uses the layout system from `korangar-interface` (resolved `WindowLayout` → draw commands).
- Can be high-quality (SSAA) independently of world settings.

See `UI_FRAMEWORK_EXTENSION.md` and `KORANGAR_INTERFACE_INTERNALS.md` for details.

**Why separate?**
- Different coordinate systems and pipelines.
- UI needs to stay crisp even when world is low-res or heavily post-processed.
- Easier to reason about focus and input capture.

In the frame, the world is rendered first, then UI is drawn on top (or into a separate attachment).

## ClientState Apply & Timing

Very important for correctness:

```rust
// In update_client_state
self.client_state.follow_mut(...).tick(...);
self.client_state.apply()?;   // <--- mutations become visible
```

`apply()` is called **after** most simulation but **before** final render decisions in some paths.

Settings changes (vsync, MSAA, shadow res, etc.) must be applied between presenting the previous frame and acquiring the next swapchain image. The code has explicit comments about DX12 requirements.

## Performance Characteristics (Observed + Architectural)

**Positive / well designed**:
- Async loading off main thread.
- Compute light culling (tiled).
- Instance + bindless where supported (with GL fallbacks).
- Smoothed values + velocity culling for expensive effects.
- Separate interface renderer.

**Known costs / bottlenecks**:
- Map loading (RSW/GND parsing + building KDTree + buffers) can be heavy.
- Texture uploads on first use.
- Text measurement in UI (mitigated by caching).
- Shadow map rendering (especially many point lights).
- Entity animation data requests.
- Full re-layout of complex windows every frame.

**GL / WSL specifics** (see CLAUDE.md):
- No `TEXTURE_BINDING_ARRAY` → bindless fallbacks.
- MSAA resolve produces black → forced to `Msaa::Off`.
- Software rendering risk if not using the run script.

**Frame timing**:
- The game uses `ClientTick` from the server for authoritative timing.
- `game_timer` reconciles server ticks with local time.
- Many animations and particles are driven by `client_tick`.

## Adding New Work to the Frame

Recommended order (to avoid bugs):

1. Network events → mutate state
2. Input → mutate state + trigger actions
3. World simulation (entities, particles, cameras)
4. UI layout (uses final state)
5. Prepare render instructions (world + interface)
6. GPU submit

If you add a new compute pass or post effect, insert it in the graphics engine's frame submission, not in the high-level `lib.rs` loop.

## Debug & Profiling Hooks

- `#[cfg(feature = "debug")] korangar_debug::profile`
- `Profiler` measurements in loaders and hot paths.
- Packet inspector, state inspector, theme inspector, frame inspector.
- `KORANGAR_PACKET_LOG` env var.

## Cross References

- `GRAPHICS_PIPELINE.md` (detailed passes)
- `CLIENT_SYSTEMS_OVERVIEW.md` (high-level loop mention)
- `SOFTWARE_DESIGN.md`
- `KORANGAR_INTERFACE_INTERNALS.md`
- `UI_FRAMEWORK_EXTENSION.md`

For a complete picture, read the `update_*` methods in `lib.rs` around the frame, the `GraphicsEngine::render_frame` path, and how `RenderInstruction` is populated.
