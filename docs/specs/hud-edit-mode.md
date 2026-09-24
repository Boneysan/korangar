# Targeted Spec — HUD Edit Mode (Foundational for Phase 2 UI)

**Parents**: [GDD §10.17](../GDD.md#1017-implementation-path-added-v02), [next-slices plan](../plans/gdd-next-slices.md) row 14, DM_CLIENT_IMPLEMENTATION.md, CLIENT_SYSTEMS_OVERVIEW.md.

**Scope for slice 14**: Lock/unlock, edge snapping, named layouts, reset, and combat fading for existing movable windows and the new quest tracker. Toasts and combat telegraphs can ship earlier using their fixed defaults.

**Current state (2026-09-24)**:
- Windows move, resize, and persist anchor/size through versioned `WindowCache`.
- Lock/unlock, Classic/Modern layouts, a per-character custom layout slot, and reset are exposed in Game Settings.
- Optional 8/16/32px screen-grid snapping can be cycled in Game Settings and is persisted; old caches default to snapping off.
- Combat-only fading is not implemented yet. Interface scale is already a player setting.

## Architecture

**Core**: Extend `WindowCache` and the existing window controls first; change the shared layout resolver only if a specific required interaction cannot be implemented there.

- `HudEditMode` state flag in `ClientState` or game settings.
- When active: Render draggable/resizeable "frames" over every HUD element.
- Each element registers a `HudElementId` + default rect + constraints.
- Layout stored per character ID in versioned profiles, migrated from the existing cache. A missing/corrupt profile falls back to a usable default.

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
- Optional screen-grid snap at 8/16/32px. Snapping to other elements remains future work.
- Profiles: shipped Classic and Modern layouts plus named custom layouts; import/export can follow after local save/load works.

## Implementation Steps

1. **State**:
   - Version a `WindowCache` profile keyed by character ID and layout name. Store anchor, size, locked, visibility, combat-only flag and non-combat opacity. Persist snap-grid selection separately from per-character layouts.
   - Migrate one existing cache into the default profile without losing positions.

2. **Registration**:
   - Give participating windows stable IDs and default/minimum bounds; include hotbar, status bar, chat, minimap, party, tracker, and DM HUD.

3. **Edit Mode UI**:
   - Expose an in-game Edit HUD action after the keybinding-table slice. Show visible handles, lock/reset commands, and snap-to-edge feedback.

4. **Persistence**:
   - Save/restore profiles through the existing cache path; use character ID rather than mutable character name. Clamp windows back onscreen after resolution or scale changes.

5. **Combat fade**:
   - Define combat as damage dealt or received in the last five seconds (playtest value). Flagged windows fade to a configurable opacity outside combat; interactive controls remain accessible.

6. **DM Integration**:
   - DM elements (initiative, hazard indicators) participate automatically.
   - Free-cam or spectator may hide certain HUDs.

## Dependencies & Risks

- Depends on: quest tracker and keybinding table for full coverage.
- Risks: cache migration, offscreen windows after resolution changes, DM windows that bypass normal registration.
- Server: None (pure client).

## Testing

- Toggle edit, drag/resize hotbar, tracker, and DM bar; save, relog, and verify positions.
- Switch Classic/Modern/custom profiles and change resolution/UI scale.
- Grid tests cover off/8/16/32px cycling, negative offsets, and old-cache migration. Still required: visually verify dragged windows align to screen coordinates and remain stable after relog, resolution/UI-scale changes, and profile switching; combat fade restores on damage and expires after its timer.

See also: modern-mechanics.md for related UI trickery, buff-bar-slice for widget patterns.

This is the substrate for "HUD edit mode" listed as MVP foundation.
