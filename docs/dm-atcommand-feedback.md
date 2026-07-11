# Atcommand / `dispbottom` Feedback in Korangar

Breadcrumbs for GM/DM chat feedback. Paired with Hercules
`planning/dm-mode-troubleshooting.md` (server-side `@dm mode` scope bug).

**Live-fixed:** 2026-07-11 · `PACKETVER` 20220406

---

## Problem summary

Hercules `dispbottom` and many atcommand replies use **`clif_disp_onlyself`**,
which sends packet **`0x017F`** (`ZC_GUILD_CHAT`-style self message), **not**
`0x008E` (`ServerMessagePacket` / player chat notify).

If the client only handles `0x008E`, you get:

- Commands may still **execute** on the map server
- **No** `[DM] …` / atcommand text in the chat window
- Length table may still list `(0x017F, -1)` so the stream does **not** desync

---

## Required client pieces

### 1. Packet definition

`ragnarok-packets/src/lib.rs`:

```rust
/// Self-only / guild-style display (`ZC_GUILD_CHAT` 0x017F).
/// Hercules: dispbottom / clif_disp_onlyself / clif_disp_message
#[header(0x017F)]
#[variable_length]
pub struct DisplayBottomMessagePacket {
    #[length_remaining]
    pub message: String,
}
```

Layout matches `ServerMessagePacket`: `<header>.W <len>.W <message>.?B` (+ NUL).

### 2. Map-server handler

`korangar-networking/src/packet_versions/version_20220406.rs` (near other chat
handlers):

```rust
packet_handler.register(|packet: DisplayBottomMessagePacket| NetworkEvent::ChatMessage {
    text: packet.message,
    color: MessageColor::Server,
})?;
```

Register **before** `register_length_fallbacks` so `0x017F` is not swallowed
as a no-op skip.

### 3. Local echo for `@` commands (UX)

`korangar/src/lib.rs` on `InputEvent::SendMessage`: before
`send_chat_message`, if `text.starts_with('@')`, push a chat line
`→ {text}` with `MessageColor::Information`.

Atcommands are not rebroadcast as public chat, so without this (and without
`0x017F`) the panel feels completely dead.

### 4. GM / DM panel

`korangar/src/interface/windows/commands.rs` — buttons emit
`InputEvent::SendMessage { text: "@dm mode on".to_string() }` etc.  
Menu: **GM / DM Commands** (`Ctrl+O`).

**Tabbed layout (2026-07-11).** The panel is split into 6 tabs — **DM · Beats ·
Character · Items · Combat · Travel** — because a DM needs many live controls.
The **Beats** tab (index 1) has one button per arc (1–19) grouped by act; each
sends `@dmbeat <arc>`, which the server (`dm_console.txt` `OnBeat`) parses to jump
straight into that arc's beat submenu. Adding/removing a tab means renumbering the
`selected_tab` indices in both the header buttons and the `either!` chain.
Implementation:

- `CommandsWindowState { selected_tab: u8 }` (new) — the window is now
  `CommandsWindow<A>` holding a path to it, opened in `lib.rs` via
  `CommandsWindow::new(client_state().commands_window())`. Wired into
  `ClientState` (`commands_window` field + `::default()` + struct assembly) and
  re-exported from `windows/mod.rs`.
- Header = a `split!` of 5 `button!`s; each `event` closure calls
  `state.update_value(selected_tab, index)`, and each is `disabled` (greyed = the
  active indicator) via `ComputedSelector` when `selected_tab == index`.
- Body = a nested `either!` chain (heterogeneous branches, `korangar-interface`
  `components/either.rs`) keyed on the same `ComputedSelector`; each tab body is a
  `fragment!` (vertical group) holding that tab's `text!`/`split!` rows.

To add a tab: bump the header buttons, extend the `either!` chain, and give the
new fragment its rows. To add a button to a tab: edit that fragment only.

**Ctrl+O must work while the chat box is focused.** A DM types an `@dm`
command (which leaves the chat box focused — Enter submits but does not
unfocus), then reaches for the panel. `process_user_events` only runs the full
`handle_keyboard_input` (menu shortcuts) when the interface is **not** focused,
so the Ctrl+O binding lives in `input/mod.rs` → `push_game_action_keys` (the
sit/hotbar path that runs in **both** focus branches), **not** in
`handle_keyboard_input` — keeping it in both would double-toggle when chat is
unfocused (open+close = nothing). Ctrl+O emits no printable char, so it does
not leak `o` into chat. Full write-up: Symptom D in
`Hercules/planning/dm-mode-troubleshooting.md`.

Chat format sent on the wire:

```text
{player_name} : @dm mode on
```

via `NetworkingSystem::send_chat_message` → `GlobalMessagePacket` **0x00F3**.

---

## Rebuild / verify

```bash
cd /path/to/korangar
cargo build --release -p korangar
# Quit any running client, then launch the new binary:
#   target/release/korangar
strings target/release/korangar | grep DisplayBottomMessagePacket
```

In game: `@dm help` or panel **DM help** should flood several `[DM]` lines.
If the server log shows `clif_disp_message: Truncated message` but chat is
empty → this packet is missing again.

---

## Related failures (not this packet)

| Symptom | Likely cause | Doc |
|---------|--------------|-----|
| Always “Mode is currently OFF” | `callsub` drops `.@atcmd_parameters$` on Hercules | [Hercules planning/dm-mode-troubleshooting.md](../../Hercules/planning/dm-mode-troubleshooting.md) |
| Kick on chat | Chat name prefix ≠ character name | Hercules `clif_process_chat_message` |
| No permission | Group level / `DM_RequireDM` | `dm_common.txt`, `groups.conf` |

---

## Do not regress

- Do not assume all server messages are `0x008E`.
- Do not “fix” silent atcommands only by local echo — still handle `0x017F`.
- After merge conflicts in `version_20220406.rs`, re-check `DisplayBottomMessagePacket` registration.
