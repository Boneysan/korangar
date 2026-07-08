# Targeted Spec — DM UI Window Template & Isolation Pattern

**Parents**: [DM_CLIENT_IMPLEMENTATION.md](../DM_CLIENT_IMPLEMENTATION.md) §1+2, [CLIENT_SYSTEMS_OVERVIEW.md](../CLIENT_SYSTEMS_OVERVIEW.md) §3 (Interface), [FEATURE_ROADMAP.md](../FEATURE_ROADMAP.md) §8.2 UI principles + E7, korangar-interface usage in existing windows.

**Purpose**: Provide a concrete, copy-pasteable template + rules so every DM window (`campaign_board`, `check_console`, `initiative_bar`, `hazard_board`, etc.) is implemented consistently, stays isolated for rebaseability, and follows the modern + DM-specific UX principles.

## 1. Isolation Rules (Enforced)

- Source lives **only** in:
  - `src/dm/` (state, parser, commands, data models — zero rendering or wgpu).
  - `src/interface/windows/dm/` (windows + pure interface elements).
- Never add DM fields to core `ClientState` top-level without the `dm_` prefix or a `dm: DmState` wrapper.
- New `WindowClass` variants for DM windows should be non-`#[cfg(debug)]` if they are production DM features.
- All DM windows must be **gated** (GM level check or `$dm_client_mode` echo) or only constructable from DM chrome.
- Use paths for reactivity: `client_state().dm_state().initiative()` etc.
- Static campaign data → generated at build (never inline).

## 2. WindowClass Additions

In `korangar/src/interface/windows/mod.rs`:

```rust
pub enum WindowClass {
    // ... existing
    DmCampaignBoard,
    DmCheckConsole,
    DmInitiative,
    DmHazardBoard,
    // ...
    // Player facing (no "Dm" prefix if visible to all)
    DiceCards,
    CampaignJournal,
}
```

Export the windows:

```rust
pub use self::dm::campaign_board::DmCampaignBoardWindow;
// etc. (or re-export from a dm mod)
```

Add `mod dm;` under the windows module if using a subdirectory (recommended for isolation).

## 3. Basic DM Window Template

Create `korangar/src/interface/windows/dm/check_console.rs`:

```rust
use korangar_interface::prelude::*;
use crate::state::{ClientState, client_state, ClientStatePathExt};
use crate::interface::windows::WindowClass;

pub struct DmCheckConsoleWindow;

impl CustomWindow<ClientState> for DmCheckConsoleWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::DmCheckConsole)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        window! {
            title: "DM Check Console",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,   // or DM-specific later
            closable: true,
            resizable: true,
            elements: (
                // Header / mode
                split! {
                    children: (
                        text! { text: "Stat" },
                        // dropdown or buttons for str/agi/...
                    )
                },

                // DC picker + adv/dis
                field! { /* numeric */ },

                // Targets (party list from dm_state or manual)
                list! {
                    // bind to client_state().dm_state().active_party()
                },

                button! {
                    text: "Roll Check",
                    on_click: || {
                        // Capture values from state or local vars
                        // Emit via a provided emitter or queue event
                        // For now: self.dm_emitter.check(target, stat, dc, adv);
                        // In practice, the window will hold or receive the emitter
                    }
                },

                // Results history (dice cards preview or list)
                list! {
                    // bind dm_state.last_checks
                },
            )
        }
    }
}
```

**Key patterns from existing code**:
- Use `split!`, `field!`, `button!`, `list!`, `text!` from `korangar_interface::prelude`.
- Bind reactive data via `client_state().xxx()` paths (see `status_bar.rs`, `chat.rs`, `dialog.rs` for examples).
- `ChatWindow` / `DialogWindow` show how to wrap state + messages.
- `StatusBarWindow` (recent) is a great simple HUD example.

For HUD-style (initiative bar, inspiration indicator):
- Implement as a non-closable overlay or use special layout in game interface.
- Participate in future HUD edit mode.

## 4. Opening DM Windows

From `lib.rs` (on map load, menu action, or hotkey):

```rust
// Example
if dm_mode_active {
    self.interface.open_window(DmCheckConsoleWindow);
    self.interface.open_window(DiceCardsOverlay);  // player facing
}
```

Use `close_window_with_class(WindowClass::DmCheckConsole)` for cleanup.

Add DM windows to the "in-game" set that opens after map load (see current hotbar + status_bar + chat opening).

## 5. Passing DM Context to Windows

Options (choose one or hybrid):

A. **Global via ClientState** (preferred for reactivity):
   - Window takes a path: `DmCheckConsole::new(client_state().dm_state())`
   - Inside: `state.get(&self.dm_path).last_checks`

B. **Command Emitter injection** (for action windows):
   - Window stores `Option<DmCommandEmitter>` or a callback.
   - On button: emitter.send(...)

C. For very isolated: use an event queue in `dm/` that lib.rs drains to networking.

## 6. Styling & Principles (from roadmap §8.2)

- **P2/P3**: Context-gate. DM windows only when GM or in session. Player dice cards only on relevant events.
- **P4/P5**: Place near relevant area (dice cards near hotbar/chat; initiative top or side).
- **P7**: Reuse tiles, buttons, lists. One widget system.
- **P10**: Bound lists (recent 8–12 checks, max 8 hazards shown).
- **P13**: Confirm on dangerous DM actions (`@dm reset` etc.).
- Use `MessageColor::Information` or custom DM colors for feedback.
- For dice flair: nat-20 green/gold, nat-1 red.

## 7. File Checklist for a New DM Window

1. `src/dm/` — add any new types or parser updates.
2. `src/interface/windows/dm/xxx.rs` — the window.
3. Update `src/interface/windows/mod.rs`:
   - `mod dm;`
   - `pub use self::dm::xxx::DmXxxWindow;`
   - Add `WindowClass::DmXxx`
4. Wire open/close in `lib.rs` (minimal).
5. Add to `ClientState` paths if new state needed (behind `dm_state`).
6. Document in `DM_CLIENT_IMPLEMENTATION.md` and update E7 task.
7. Add to debug inspector or a DM menu if appropriate.

## 8. Example for Player-Facing Dice Cards

Can be a persistent small overlay (not full window) or a stack of temporary cards spawned near the character or in a dedicated corner window.

Reuse `particle_holder` ideas or pure interface elements that animate on `dm_state.last_checks` changes.

## 9. Verification

- Window opens cleanly on a DM account.
- Buttons emit correctly formatted chat (visible in packet inspector as GlobalMessagePacket).
- `[DMJ]` results update state and appear in the window (and optionally as nice chat entries).
- Rebase test: upstream change to `windows/mod.rs` or `ClientState` should only require small merge in the DM files.
- No impact on non-DM play sessions.

This template + the Phase A parser spec + party packets spec give a complete starting kit for the first DM windows (check console + dice cards are recommended first slices).

Cross-reference when implementing: look at `status_bar.rs` (recent, clean, path-based) and `dialog.rs` (menu/choice handling).