# UI Framework Extension Guide

**Audience**: Game engine engineers and DM UI implementers.

**Purpose**: How the retained-mode UI system (`korangar-interface` + `src/interface/`) works, how state binding works, and concrete patterns for adding or modifying windows/components — especially for isolated DM features.

## High-Level Architecture

Korangar uses a **retained-mode** UI built on top of a custom `korangar-interface` crate.

- **Core concepts**:
  - `CustomWindow<ClientState>` trait (in `korangar-interface/src/window/mod.rs`).
  - `window! { ... }` declarative macro for layout.
  - Reactive binding via `rust_state::Path` + `StateElement` / `RustState` derives on `ClientState`.
  - `ClientState` is the single source of truth. UI reads via paths like `client_state().hotbar()`.
  - Rendering happens in `src/renderer/interface.rs` (separate from world rendering).

**Data flow for a window**:
1. Event or map load triggers `interface.open_window(SomeWindow::new(...))`.
2. Window implements `to_window()` which builds a tree of elements using macros.
3. Elements bind to paths in `ClientState`.
4. On frame: `interface.process_events()` + layout resolution + `GameInterfaceRenderer`.
5. Mutations go through `client_state.follow_mut(path)` — changes are applied at end of frame via `client_state.apply()`.

## Key Files

- `korangar-interface/src/`:
  - `window/` — `CustomWindow`, `Window` trait.
  - `element/state.rs` — `StateElement`.
  - `layout/` — resolver, area, alignment.
  - `components/` — button, text, split, field, etc.
  - `macros/` — the `window!`, `split!`, `text!`, etc. macros.
- `korangar/src/interface/`:
  - `windows/` — concrete windows (e.g. `status_bar.rs`, `dialog.rs`, `chat.rs`).
  - `mod.rs` — `WindowClass` enum + exports.
  - `components/item_box.rs`, `skill_box.rs` — reusable drag-drop + tooltip pieces.
- `korangar/src/state/mod.rs` — `ClientState` definition + path helpers (`ClientStatePathExt`).
- `korangar/src/lib.rs` — `self.interface.open_window(...)`, window management on events.

## How to Add a New Window (DM or Core)

See also `docs/specs/dm-ui-window-template.md` and `docs/DM_CLIENT_IMPLEMENTATION.md` for isolation rules.

1. **Create the window file** (e.g. `src/interface/windows/dm/my_feature.rs`):

```rust
use korangar_interface::prelude::*;
use crate::state::ClientState;
use crate::interface::windows::WindowClass;

pub struct MyFeatureWindow;

impl CustomWindow<ClientState> for MyFeatureWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::MyFeature)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        window! {
            title: "My Feature",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! { text: "Hello" },
                button! {
                    text: "Do Thing",
                    on_click: || {
                        // Capture state or use paths
                    }
                },
            )
        }
    }
}
```

2. **Register it**:
   - Add `mod my_feature;` (or under `dm/`) in `src/interface/windows/mod.rs`.
   - `pub use self::dm::my_feature::MyFeatureWindow;`
   - Add variant to `WindowClass` enum.

3. **Open it** from `lib.rs` or a menu:
   ```rust
   self.interface.open_window(MyFeatureWindow);
   ```

4. **Bind reactive state**:
   - Add field to `ClientState` (use `#[hidden_element]` for DM state).
   - Pass path: `MyFeatureWindow::new(client_state().my_feature_state())`
   - Use `state.get(&self.path)` inside layout methods if needed (see `chat.rs` for examples).

## Reusable Components & Patterns

- **Item/Skill boxes**: See `components/item_box.rs` and `skill_box.rs`. Support drag-drop via `MouseInputMode::MoveItem`.
- **Tooltips**: Use `TooltipExt` and `add_tooltip`.
- **Themes**: Defined in `state/theme/`. Use `InterfaceThemeType::InGame` or `Menu`.
- **WindowClass**: Used for `close_window_with_class`, focus, debug inspectors.
- **DM isolation**: Put DM windows in `windows/dm/`. Keep logic in `src/dm/`. Use `dm_state` sub-struct.

## Common Gotchas

- Windows are recreated on open — store transient state in the window struct or in `ClientState`.
- Layout is resolved every frame in `create_layout_info`.
- Focus and input modes matter (see `input/mode.rs`).
- For HUD elements that participate in edit mode, they must register layout metadata (see `specs/hud-edit-mode.md`).
- Debug windows are `#[cfg(feature = "debug")]`.

## How to Extend the Framework Itself

- New component: Add to `korangar-interface/src/components/`.
- New layout primitive: Extend `layout/resolver.rs`.
- New macro sugar: Add in `korangar-interface/macros/`.
- Always keep changes minimal if you want to stay close to upstream.

## Practical Improvement Examples

- **Add a DM command palette**: Create window that lists @dm verbs (from generated data), on select calls emitter that builds `GlobalMessagePacket`.
- **Modernize inventory**: Extend `inventory.rs` window + reuse `item_box` component. Add tabs by filtering `ClientState` inventory list.
- **Custom HUD element**: Implement `CustomWindow`, make it non-closable, position via edit mode.

For full DM window patterns, see the template spec.

This + reading the source of a few existing windows (start with `status_bar.rs` and `dialog.rs`) should give an engineer enough to make safe, rebaseable improvements.
