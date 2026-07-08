# Input & Camera Systems

**Purpose**: Detailed internals of input handling, camera system, picking, and pathing. This moves beyond the high-level sketches in `plans/modern-mechanics.md`.

**Audience**: Engine engineers who want to implement WASD/action camera, gamepad support, improved picking, free-cam for DM, or custom controls.

## Input System Overview

Location: `korangar/src/input/`

Core types:
- `InputSystem`
- `InputReport` (what is consumed by the rest of the game each frame)
- `Key`
- `MouseInputMode` (context for what the mouse is currently doing)

### InputSystem Responsibilities

- Track raw winit events (keyboard, mouse buttons, scroll, characters, motion).
- Detect double-clicks (using `ClientTick` and `DOUBLE_CLICK_TIME_MS`).
- Compute deltas (mouse movement, scroll).
- Read the GPU picker value (`picker_value: Arc<AtomicU64>`) to know what is under the cursor (`mouse_target: PickerTarget`).
- Produce a clean `InputReport` once per frame in `update_delta`.

Key method:

```rust
pub fn update_delta(&mut self, client_tick: ClientTick) -> InputReport { ... }
```

This is called from the main game loop.

### MouseInputMode

Defined in `mode.rs`. This is crucial for context:

- Normal world interaction
- `MoveItem` (dragging inventory items)
- UI dragging
- Possibly more in the future (picking for DM pings, etc.)

The mode affects how raw clicks are interpreted in `lib.rs` (process_user_events).

### Integration with UI

The interface system receives the `InputReport` and translates it into higher-level events (`korangar_interface::event`).

UI has priority in many cases (focus, captured input).

## Camera System

All cameras live in `korangar/src/world/cameras/`.

Common trait:

```rust
pub trait Camera {
    fn camera_position(&self) -> Point3<f32>;
    fn focus_point(&self) -> Point3<f32>;
    fn generate_view_projection(&mut self, window_size: ScreenSize);
    fn view_projection_matrix(&self) -> Matrix4<f32>;
    fn view_direction(&self) -> Vector3<f32>;
    // ... billboard helpers, etc.
}
```

### Current Cameras

- **PlayerCamera** (`player.rs`): The normal RO-style orbital camera.
  - Focus point follows the player.
  - Distance + yaw (angle) are smoothed (`SmoothedValue`).
  - Fixed pitch (`CAMERA_PITCH = -55°`).
  - Zoom and rotate via mouse wheel / drag (right button?).

- **StartCamera**, **DebugCamera**: Used on login / debug.

- Shadow cameras:
  - `DirectionalShadowCamera`
  - `PointShadowCamera`
  - Partitioning for cascaded / tiled shadows (see `GRAPHICS_PIPELINE.md`).

- `SmoothedValue`: Simple spring-like smoothing used for camera movement.

Coordinate system note (from code):
- Left-handed, +Y up, +Z into screen for world.

### How Cameras Are Used

In `lib.rs` / renderer:
- Main player camera is updated based on player position + input.
- `generate_view_projection` is called with current window size.
- View/projection matrices are fed to shaders (globals bind group).
- Picking uses the inverse of the view-projection + raycast against collision.

## Picking

Two levels:

1. **GPU Picker** (for fast "what is under cursor?"):
   - Separate render pass that writes `EntityId` or other IDs into a texture.
   - Read back via `AtomicU64` (see `picker_target.rs`).
   - Extremely fast, happens every frame.

2. **CPU Collision / Pathing**:
   - `korangar-collision` crate (KDTree, AABB, etc.).
   - Used for:
     - Walkability (GAT)
     - Object picking
     - Path finding to click target
   - Built during map load in `MapLoader`.

`PickerTarget` enum tells you if you hit an entity, ground, UI, etc.

## Pathing & Movement

- `world/pathing.rs`
- Uses collision data + GAT tiles.
- Client does local path prediction, then sends `RequestPlayerMovePacket`.
- Server is authoritative.

For future WASD / action camera (see `modern-mechanics.md`):
- You will need to do raycasts from screen center (or reticle) each frame.
- Throttle movement packets (200ms suggested in the sketch).
- Convert 3D direction into tile coordinates.

## Current Input Processing (High Level)

In `lib.rs`:

1. `input_system.update_delta(client_tick)` → `InputReport`
2. `process_user_events(input_report, ...)` — handles:
   - UI focus first
   - World clicks → attack / move / pick up
   - Camera control (rotate, zoom)
   - Hotbar, skills, etc.
3. Mouse modes change behavior (e.g. while dragging an item).

## Extension Points

### Implementing Action Camera / WASD

See the sketch in `plans/modern-mechanics.md`, but here are the concrete places:

- Add `CameraMode` enum (or extend `PlayerCamera`).
- In input: detect WASD keys → compute movement vector.
- Rotate vector by camera yaw.
- Raycast to find target tile (use collision or map data).
- Send throttled `RequestPlayerMovePacket`.
- For combat: raycast from center for target selection on left click.
- Lock cursor when in action mode (`winit` cursor grab).

You will also need to modify how the player entity is oriented (face movement direction instead of click direction).

### Gamepad Support

Currently zero support.

- winit has gamepad events via `gilrs` or similar (not integrated yet).
- Map left stick → movement (when in action cam).
- Right stick → camera rotate.
- Face buttons → hotbar / attack.
- Triggers → radial menus or modifier layers (as sketched).

The `InputReport` would need to grow analog stick + button state.

### DM Free-Cam / Spectator

- Reuse or extend `DebugCamera`.
- Allow detaching focus point from player.
- Add speed controls.
- Probably a separate input mode.
- Useful for placing hazards / pings precisely.

### Custom Picking Modes

- Add new `MouseInputMode` variants.
- In `process_user_events`, check mode + `mouse_target` + buttons.
- For DM: "place marker here" mode that sends a command instead of normal move.

## Gotchas

- Camera pitch is currently hardcoded — many things (billboards, shadows) assume the -55° angle.
- Smoothed values have velocity thresholds for "fast movement" culling (used for some effects).
- Picker value is updated by the GPU — you may read stale data for one frame.
- Double-click detection uses server ticks, not wall time.
- Input buffer for characters is drained every frame (for chat).

## Performance Notes

- Input processing is very cheap.
- Picking readback is the only "expensive" part (but atomic load is fast).
- Camera math is negligible.
- For future WASD you will want to be careful not to raycast every single frame if possible.

## Cross References

- `plans/modern-mechanics.md` (aspirational designs for WASD, gamepad, dodge, etc.)
- `CLIENT_SYSTEMS_OVERVIEW.md` (Input / Camera section)
- `GRAPHICS_PIPELINE.md` (how cameras feed view/projection + billboards)
- `WORLD_MAPS_ENTITIES.md` (collision / picking usage)
- `specs/hud-edit-mode.md` (mouse modes during editing)
- `src/input/mode.rs` for current modes

Reading `input/mod.rs` + `world/cameras/player.rs` + `world/cameras/mod.rs` (the trait) + the picking usage in `lib.rs` will give you the full current picture.
