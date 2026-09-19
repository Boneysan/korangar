# Korangar Player Guide

A practical "how do I…" reference for playing on this fork. This is written
for players, not developers — for the technical side, start at
[README.md](README.md) instead.

Everything here reflects the controls as implemented in
`korangar/src/input/mod.rs` and `korangar/src/input/event.rs`; if something
here stops matching what you see in-game, the source is the tie-breaker.

## Movement

- **Click-to-move**: left-click a spot on the ground to walk there. Holding
  the button down keeps re-issuing the destination toward your cursor.
- **WASD movement** (optional, on by default — toggle it in
  **Game Settings**): W/A/S/D walks relative to the camera, not the
  character's facing. Click-to-move still works even with WASD on.
- **Camera**: right-click and drag to rotate; scroll wheel to zoom;
  double-right-click resets the rotation.

## Targeting and combat

- **Click to attack**: left-click a monster to walk into range and start
  attacking it. With **Auto attack** on (Game Settings, on by default), you
  keep attacking that target automatically until it dies, you move away, or
  you pick a new target — you don't need to keep re-clicking.
- **Tab-targeting**: press **Tab** (configurable — see below) to cycle
  through visible, living hostile monsters by distance, nearest first. It
  wraps around and skips players and hidden entities.
- **Attack your Tab-target**: press **Space** (configurable) to attack
  whatever you last Tab-selected, without needing to click or hover it. This
  is a separate, deliberate press rather than something that fires the
  instant Tab selects a target — so cycling through several monsters to look
  around doesn't engage all of them.
- **Skills also use your Tab-target.** If your mouse isn't hovering a
  specific entity when you press a skill's hotbar key, an attack or support
  skill targets whatever you last Tab-selected instead of defaulting to
  yourself (support) or failing to fire (attack). Hovering an entity always
  wins over the Tab-target.
- **Both target-related bindings are configurable and can be disabled**, in
  **Game Settings**:
  - *Hostile target cycle binding*: Tab → ~ (tilde) → Q → Disabled.
  - *Attack target binding*: Space → F → R → Disabled.
- **Ground/area skills** always need an explicit click on a tile (or on an
  entity, which targets its tile) — they never use the Tab-target.
- **Out-of-range casts walk you in first** instead of silently failing —
  official Hercules just drops an out-of-range cast with no feedback; this
  fork walks you into range and then casts.
- **Sitting** (Insert, or Home as an alternate) restores 25% of your maximum
  HP and SP every 10 seconds. **Respawning** restores 50% immediately, then
  fills the rest over the next 10 seconds — taking damage cancels that fill.
  Combat, being overweight, and certain status effects report why recovery
  is currently blocked instead of silently applying a partial tick.

## Skills and the hotbar

- Skills sit on a **27-slot hotbar**, arranged as three 9-slot rows:
  - Row 1: **F1–F9** or the **number row 1–9**.
  - Row 2 (slots 10–18): hold **Ctrl** + F1–F9 or 1–9.
  - Row 3 (slots 19–27): hold **Alt** + F1–F9 or 1–9.
- Drag a learned skill from the Skill Tree window onto a hotbar slot to
  assign it; drag it back off (or right-click the slot) to clear it.
- Items can also be dragged onto the hotbar and used from there the same way.
- The number-row hotbar keys only fire while no chat/text box has focus (so
  you can still type digits in chat); **F1–F9 always work**, even while
  typing.

## Inventory, equipment, and items

- **Alt+E**: Inventory. **Alt+Q**: Equipment. **Alt+S**: Skill Tree.
  **Alt+A**: Stats. **Alt+V**: Character overview.
- **Drag** an item to equip it (to its equipment slot), to move it between
  inventory/storage/trade, or to reorder your inventory display.
- **Double-click** an inventory item to quick-equip it (or quick-unequip it
  from the Equipment window); unidentified items are identified instead, and
  usable items (potions, etc.) are used.
- **Right-click** an inventory item to open its actions popup (use / equip /
  drop). Item names always appear in messages — never a raw item number.
- Vendor buy lists mute and red-tint items your character cannot use, with a
  tooltip explaining why (job, level, sex, or slot restriction); you can
  still buy them, just not equip them.
- **Automatic pickup** (Game Settings) sweeps loot within two tiles straight
  into your bag; in a party it applies to everyone and the server keeps the
  setting across sessions.

## Quests

- **Ctrl+Q**: open the quest journal (works even while chat is focused).
- Each quest entry can be expanded/collapsed, **pinned** to the top of the
  journal for the session, and **tracked** on the HUD.
- **Track on HUD / Untrack** puts that quest's next objective (with remaining
  count, direction, and distance) on the always-visible HUD breadcrumb and
  remembers your choice for next login. If nothing is manually tracked, the
  client auto-selects your first incomplete quest.
- The HUD breadcrumb also marks the next portal or destination on the
  minimap; for cross-map objectives, the journal shows the full route while
  the HUD shows only the next leg. It never auto-walks or completes anything
  for you — it's guidance, not an autopilot.
- HUD breadcrumb controls: collapse/expand, hide/show, and scale/opacity
  adjustments live as buttons on the HUD widget itself.

## Party and social

- **Alt+Z** (or **Alt+P**): Party window. **Alt+H**: Friend list
  (**Alt+Shift+H** toggles the custom zeny/EXP HUD instead).
- Party member colors on the minimap/roster can be locally recolored per
  member without touching the server's actual party.
- A party leader can kick members, promote a new leader, and set the three
  share rules (item/EXP/etc.) from the party window.
- "Go to" a party member warps you to them through the same command path a
  GM would use, without needing GM rights.

## Trading, storage, and shops

- **Left-click another player** to open a target window with **Whisper** and
  **Trade** buttons. Requesting a trade lets both sides accept/reject, add
  items or zeny, then lock (OK) and commit.
- Storage and shop buy/sell windows work the same drag-and-drop way as your
  inventory. Weight limits are enforced consistently across inventory,
  storage, trade, and cart, including exact-fit boundaries.

## Windows and settings

| Key | Window |
|---|---|
| Escape | Menu |
| Alt+E | Inventory |
| Alt+Q | Equipment |
| Alt+S | Skill Tree |
| Alt+A | Stats |
| Alt+V | Character overview |
| Alt+Z / Alt+P | Party |
| Alt+H | Friend list |
| Alt+Shift+H | Zeny/EXP HUD |
| Alt+L | Emote palette |
| Ctrl+Q | Quest log |
| Ctrl+D | Dice roller |
| Ctrl+O | DM/GM commands panel |
| Ctrl+S | Game settings |
| Ctrl+I | Interface settings |
| Ctrl+G | Graphics settings |
| Alt+O or Ctrl+A | Audio settings |
| Ctrl+Tab | Toggle minimap visibility |
| Ctrl+H | Show/hide the whole UI (e.g. for screenshots) |
| Ctrl+W | Close the top window |
| F11 | Close all ordinary windows (keeps chat + basic info) |
| Alt+Enter | Toggle fullscreen |

**Game Settings** (Ctrl+S) also holds: Auto attack, Show minimap, WASD
movement, the two Tab-target key bindings above, and Automatic pickup.

## Full keybind quick-reference

| Input | Action |
|---|---|
| Left-click | Move / select / interact / attack |
| Left-click + drag | Continuous move toward cursor |
| Double left-click | Quick-equip/unequip, or context action |
| Right-click | Rotate camera, or cancel an armed skill / an in-progress cast |
| Double right-click | Reset camera rotation |
| Scroll wheel | Zoom camera |
| W / A / S / D | Camera-relative movement (if WASD movement is on) |
| Insert / Home | Sit / stand |
| Tab (configurable) | Cycle Tab-target |
| Space (configurable) | Attack current Tab-target |
| F1–F9, 1–9 | Hotbar row 1 |
| Ctrl + F1–F9 / 1–9 | Hotbar row 2 |
| Alt + F1–F9 / 1–9 | Hotbar row 3 |

## See also

- [ORIGINAL_CLIENT_CONTROLS.md](ORIGINAL_CLIENT_CONTROLS.md) — the
  compatibility policy behind these bindings (developer-facing).
- [DM_INTERFACE.md](DM_INTERFACE.md) — the DM/tabletop tools (dice, bestiary
  journal, loot generator, encounter panel) beyond ordinary play.
- [storage-window.md](storage-window.md), [player-save-and-respec.md](player-save-and-respec.md) —
  deeper notes on specific systems mentioned above.
