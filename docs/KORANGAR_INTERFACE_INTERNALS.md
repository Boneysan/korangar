# Korangar Interface Internals

**Purpose**: Deep technical documentation of the `korangar-interface` crate and its integration in Korangar. This is the retained-mode UI framework used for all windows, HUD elements, and DM tools.

**Audience**: Game engine engineers who need to extend the UI framework, add custom components, understand reactivity and layout, or debug UI behavior.

**Key crates**:
- `korangar-interface/` (core framework)
- `korangar-interface/macros/` (proc macros for `window!`, components)
- `korangar-interface/component-macros/`
- Integration in `korangar/src/interface/`

## High-Level Architecture

Korangar uses a **retained** UI model (not immediate mode like Dear ImGui):

- State lives in `ClientState` (reactive via `rust_state`).
- UI is described declaratively with macros.
- Layout is resolved every frame.
- Elements are stored and can hold local state via `ElementStore`.
- Rendering is decoupled into `GameInterfaceRenderer`.

Core traits:
- `Element<App>` — anything that can be laid out and rendered.
- `Window<ClientState>` — top-level containers.
- `CustomWindow<ClientState>` — the trait implemented by concrete windows (e.g. `StatusBarWindow`).

Data flow:
```
ClientState (RustState paths)
    → Window constructor (holds paths)
    → to_window() → window! { ... } macro → tree of Elements
    → Resolver (layout pass)
    → GameInterfaceRenderer (draw pass)
    → Events flow back via EventQueue
```

## Directory Structure

```
korangar-interface/
├── src/
│   ├── lib.rs                 # Core types, prelude, WindowStore
│   ├── application.rs         # Application trait (Color, Size, TextLayouter, etc.)
│   ├── element/
│   │   ├── mod.rs
│   │   ├── state.rs           # StateElement trait + store
│   │   ├── id.rs
│   │   └── store.rs
│   ├── layout/
│   │   ├── mod.rs
│   │   ├── resolver.rs        # The heart of layout
│   │   ├── area.rs
│   │   ├── alignment.rs
│   │   └── tooltip.rs
│   ├── window/
│   │   ├── mod.rs             # CustomWindow, Window traits
│   │   ├── store.rs
│   │   └── anchor.rs
│   ├── components/            # Built-in components (button, split, text, etc.)
│   ├── event/                 # EventQueue, handlers
│   └── theme.rs
└── macros/
    ├── src/
    │   ├── lib.rs
    │   ├── window.rs          # window! macro
    │   ├── element.rs
    │   └── helper.rs
```

## The `window!` and Component Macros

The declarative syntax lives in macros.

Example usage (from real code):

```rust
window! {
    title: "Status Effects",
    class: WindowClass::StatusBar,
    theme: InterfaceThemeType::InGame,
    elements: (
        split! {
            children: ( ... )
        },
        // ...
    )
}
```

The `window!` macro (in `macros/src/window.rs`) expands to a struct implementing `Window` that holds child elements and layout info.

Component macros (e.g. `split!`, `text!`, `button!`) are generated via `interface_component_macros` and `interface_components`.

**Extending**:
- To add a new component, implement `Element<App>` and provide a macro entry point.
- See `components/button.rs` for a relatively simple example.

## Layout System (The Resolver)

The core layout logic is in `layout/resolver.rs`:

```rust
pub struct Resolver<'a, App: Application> {
    available_area: PartialArea,
    used_height: f32,
    gaps: f32,
    text_layouter: &'a App::TextLayouter,
}
```

Key methods:
- `with_height(height)` — allocates vertical space, advances the cursor.
- `push_available_area()` — returns remaining space (used by `split!` etc.).
- `get_text_dimensions()` — measures text for wrapping/height calculation.

Layout happens in two phases per element:
1. `create_layout_info(...)` — compute sizes (can be recursive).
2. `lay_out(...)` — actually place text, rectangles, etc. into `WindowLayout`.

`WindowLayout` accumulates draw commands (text, colored rects, etc.).

**Important concepts**:
- `Area` / `PartialArea` — left, top, width, height.
- Gaps are inserted between children.
- Text wrapping uses `OverflowBehavior`.
- Alignment (Horizontal/Vertical) controls positioning within allocated space.

**How to extend layout**:
- Subclass behavior by implementing custom `Element` that uses `Resolver` directly.
- For complex containers, look at `split.rs` or `scroll_view.rs`.

## Element System & Reactivity

Elements implement:

```rust
pub trait Element<App: Application> {
    type LayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<App>,
        store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<App>,
    ) -> Self::LayoutInfo;

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    );
}
```

**State binding**:
- Use `rust_state::Path` to bind to `ClientState`.
- `StateElement` derive on state structs + `client_state().foo()` paths.
- In `create_layout_info`, call `state.get(&path)` to read current values.
- Mutations happen outside (in `lib.rs` event handlers) via `follow_mut`.

`ElementStore` / `ElementStoreMut` provide per-element local storage (e.g. for cached text measurements, scroll positions).

`ErasedElement` is used to store heterogeneous element trees.

## Window Management

`CustomWindow` is the public API:

```rust
pub trait CustomWindow<App> {
    fn window_class() -> Option<WindowClass>;
    fn to_window<'a>(self) -> impl Window<App> + 'a;
}
```

Windows are stored in `WindowStore`.

`GameInterfaceRenderer` (in Korangar) owns the actual drawing and calls into the resolved layout.

**DM usage note**: DM windows should live in `src/interface/windows/dm/` and use `dm_state` paths for isolation.

## Themes & Styling

Themes are defined with `ThemePathGetter` derives.

See `state/theme/interface.rs` and `state/theme/world.rs`.

The `theme!()` macro provides access during layout.

## Event Handling

Events flow through `EventQueue`.

Common patterns:
- `on_click` closures in button macros.
- `SetToTrue`, `Toggle` helpers in the prelude.

Input is translated in `src/input/` into `InputReport`, which feeds the interface.

## Performance Characteristics

- Layout is O(n) per frame for visible elements (n = number of active UI nodes).
- Text measurement can be expensive — hence caching in `ElementStore`.
- Heavy use of `UnsafeCell` in some places for interior mutability during layout (historical reason).
- Debug builds have extra inspectors that walk the entire element tree.

For high-frequency HUD elements (buff bars, initiative), keep node count low and avoid deep nesting.

## How to Extend the Framework

1. **New component**:
   - Implement `Element`.
   - Add a macro in `macros/`.
   - Export via prelude.

2. **New layout behavior**:
   - Create a custom resolver helper or container element.
   - Look at `scroll_view.rs` for scrolling logic.

3. **Custom reactivity**:
   - Use `PartialEqDisplaySelector` or custom `Selector` for derived values (see `lib.rs`).

4. **DM-specific widgets**:
   - Prefer composing existing components.
   - Use paths into `dm_state` (see `DM_CLIENT_IMPLEMENTATION.md`).

**Rebaseability warning**: Changes to `korangar-interface` itself should be minimal and upstream-friendly when possible. DM-specific logic belongs in `korangar/src/dm/` and `windows/dm/`.

## Debugging UI

- Enable the `debug` feature → `ClientStateInspector`, `ThemeInspector`.
- Packet inspector can help correlate UI changes with network events.
- `KORANGAR_PACKET_LOG` + UI state inspector for end-to-end tracing.

## Cross References

- `CLIENT_SYSTEMS_OVERVIEW.md` (UI section)
- `UI_FRAMEWORK_EXTENSION.md` (practical "how to add a window")
- `specs/hud-edit-mode.md`
- `specs/dm-ui-window-template.md`
- `GRAPHICS_PIPELINE.md` (interface rendering pass)
- Actual windows: `src/interface/windows/status_bar.rs`, `dialog.rs`, `chat.rs`

This document + reading the source of `layout/resolver.rs`, a component, and one window should give an engineer a solid mental model for extending the UI layer.
