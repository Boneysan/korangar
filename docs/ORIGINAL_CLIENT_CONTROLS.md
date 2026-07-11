# Original Client Control Compatibility

Behavioral baseline: [iRO Wiki — Basic Game Control](https://irowiki.org/wiki/Basic_Game_Control)
(page revision dated 2025-01-10). The goal is original-client capability;
custom campaign controls must use non-conflicting bindings.

## Compatibility rules

- Normal RO play is mouse-driven. Do not add WASD character movement.
- Preserve original shortcuts before assigning custom or debug shortcuts.
- A shortcut must behave consistently on macOS, Windows, Linux, and WSL.
- Text entry owns ordinary keys while focused; global gameplay keys are limited
  to bindings that the original client intentionally keeps active.
- Unsupported windows should not silently steal their official shortcut. Leave
  the binding reserved until the window exists.

## Observed failures before this rework

These are the concrete failures seen during the 2026-07-10 macOS bring-up and
the audit of controls previously added under WSL:

- Hercules had been configured for PACKETVER 20220406 without a clean rebuild.
  The running binaries still reported 20190605, so the client and server were
  wire-incompatible even though `config.log` looked correct.
- `athena-start` left stale PID files after its background processes were
  reaped. The launcher appeared to succeed while no service was listening on
  ports 6900, 6121, 5121, or 7121. The client accepted the Login click but gave
  no visible feedback because its TCP connection failed before authentication.
- Earlier packet registration gaps could silently wedge login, character list,
  or map handoff. The generated packet-length fallback table and modeled login
  refusal packets now prevent unknown known-length packets from desynchronizing
  the stream.
- The initial macOS render guard avoided an AppKit startup panic but could leave
  a permanently blank window when the first redraw was discarded. The guard
  now re-requests redraws and `resumed()` explicitly starts the redraw loop.
- Several WSL-era shortcuts replaced original-client controls instead of
  extending them: Alt+Z opened friends rather than party, Alt+H opened the
  custom HUD rather than friends, Alt+M toggled the minimap rather than opening
  macros, and F10 was treated as a tenth hotbar slot rather than chat height.
- Sit/stand existed on Insert (with Home as a laptop compatibility alias), but
  this and other isolated additions were not managed as an original-client
  compatibility set. The matrix below is now the source of truth.

## Current matrix

| Capability | Original control | Current status |
|---|---|---|
| Move / continuous move | Left click / hold | Implemented |
| NPC / monster interaction | Left click | Implemented |
| Camera rotate | Hold right mouse and drag | Implemented |
| Camera zoom | Mouse wheel | Implemented |
| Reset horizontal camera | Double right click | Implemented |
| Sit / stand | Insert, `/sit`, `/stand` | Implemented |
| Main hotbar | F1–F9 | Implemented; F10 no longer stolen |
| Basic information | Alt+V | Implemented |
| Inventory | Alt+E | Implemented |
| Equipment | Alt+Q | Implemented |
| Skills | Alt+S | Implemented |
| Party | Alt+Z | Implemented |
| Friends | Alt+H | Implemented |
| Options/audio | Alt+O | Implemented (audio window) |
| Minimap display cycle | Ctrl+Tab | Partial: visible/hidden; opacity modes pending |
| Close ordinary windows | F11 | Implemented; retains basic info and chat |
| Chat height | F10 | Pending |
| Hotbar page cycle | F12 | Pending |
| Chat focus/send | Type, Enter | Partial; cross-platform IME/text audit pending |
| Party / guild chat | Ctrl+Enter / Alt+Enter | Pending |
| Camera height | Shift+wheel, Ctrl+Shift+wheel | Pending |
| Non-wheel zoom | Ctrl+right drag | Pending |
| Full camera reset | Shift+double right click | Pending |
| Player context menu | Right click player | Pending |
| Auto-follow | Shift+right click player | Pending |
| Item/skill details | Right click | Partial |
| Item link in chat | Shift+left click item | Pending |
| Fast item transfer | Alt+right click item | Pending |
| One-click identify | Ctrl+right click item | Implemented in UI; modifier path pending |
| Drag/drop inventory, equipment, storage, trade, skills | Mouse drag | Partial |
| Screenshot | Print Screen | Pending |
| Emote/macro windows | Alt+L / Alt+M | Pending; bindings reserved |
| Quest, guild, cart and specialized windows | Official Alt shortcuts | Pending with their windows |
| `/where`, `/help`, audio/effect and social commands | Slash commands | Partial |

## Implementation order

1. Input semantics: modifiers, camera variants, F10/F12, chat focus and channel
   sends, macOS IME/text delivery.
2. Existing-window parity: every official shortcut for a window already in the
   client, plus close/cycle behavior.
3. Interaction parity: player context actions, follow, item modifiers, and
   complete drag/drop paths.
4. Commands and settings: original slash commands backed by real client/server
   state rather than cosmetic messages.
5. Missing systems/windows: quest, guild, cart, pet, homunculus, mercenary,
   macro/emote, world map, chatroom, and instance interfaces.
6. Platform verification: exercise the same checklist against native macOS,
   native Windows, Linux, and WSL with PACKETVER 20220406.
