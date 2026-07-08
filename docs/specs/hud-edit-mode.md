# Targeted Spec — HUD Edit Mode (Foundational for Phase 2 UI)

**Parents**: FEATURE_ROADMAP.md §8 (Foundation), DM_CLIENT_IMPLEMENTATION.md (HUD elements must participate), CLIENT_SYSTEMS_OVERVIEW.md, SOFTWARE_DESIGN.md, plans/modern-mechanics.md (UI trickery context).

**Why critical**: Every modern widget (party frames, buff bars, initiative, dice cards, toasts, nameplates, cast bars, quest trackers) depends on a flexible, per-character, resizable/scalable HUD layout system. Without it, DM tools and "feels modern" MVP will feel bolted-on.

**Current state (deep dive)**:
- Windows use `korangar_interface` with fixed layouts.
- Hotbar, StatusBar, Chat are opened on map load.
- No drag/scale/lock/persist system yet.
- Cameras and input support basic zoom/rotate but no edit mode overlay.
- Persistence exists for general settings but not per-HUD profiles.

## Architecture

**Core**: Extend `korangar_interface` layout resolver + add a global `HudLayout` state.

- `HudEditMode` state flag in `ClientState` or game settings.
- When active: Render draggable/resizeable "frames" over every HUD element.
- Each element registers a `HudElementId` + default rect + constraints.
- Layout stored per character (or account) in settings (similar to `interface_settings.ron` but versioned HUD profiles).

**Elements to support** (start with core + DM):
- Hotbar
- Status bar (buffs)
- Chat
- Minimap
- Party frames (future)
- Initiative bar (DM)
- Dice cards / toasts
- Quest tracker
- DM-specific HUD (session info)

**Interaction**:
- Drag: Move the element's anchor/rect.
- Resize handles: Scale (respect min/max).
- Right-click: Lock, reset to default, opacity slider, scale.
- Snap to grid or other elements.
- Profiles: "Default", "Combat", "DM", "Minimal" + import/export.

## Implementation Steps

1. **State**:
   - Add `hud_layout: HudLayout` to ClientState (or under interface_settings).
   - `HudLayout` : HashMap<HudElementId, HudElementConfig> { rect: Rect, scale: f32, locked: bool, visible: bool, ... }

2. **Registration**:
   - Each HUD window/element implements a trait or provides metadata on creation.
   - Example in hotbar/status_bar: register default position.

3. **Edit Mode UI**:
   - New debug or in-game mode (toggle via F-key or settings).
   - Overlay grid + resize boxes using existing picker or new 2D overlay pass.
   - Use `MouseInputMode::HudEdit` or similar.

4. **Persistence**:
   - Serialize to client settings (ron or similar).
   - Load on character select / map enter.
   - Per-character if possible (tie to character name/id).

5. **Rendering**:
   - Most HUDs already rendered in interface pass.
   - When editing: Draw frames around them; allow interaction before normal input.
   - Scale/position applied at layout time.

6. **DM Integration**:
   - DM elements (initiative, hazard indicators) participate automatically.
   - Free-cam or spectator may hide certain HUDs.

## Dependencies & Risks

- Depends on: korangar-interface layout system maturity, existing window positioning.
- Risks: Performance (many elements), persistence format stability, conflict with fixed RO layouts (respect P8: keep familiar conventions).
- Server: None (pure client).

## Testing

- Toggle edit, drag/resize hotbar + new DM bar, save profile, relog, verify positions.
- Multiple profiles switchable.
- Locked elements ignore drag.

See also: modern-mechanics.md for related UI trickery, buff-bar-slice for widget patterns.

This is the substrate for "HUD edit mode" listed as MVP foundation.
