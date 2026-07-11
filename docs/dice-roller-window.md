# Dice Roller Window

GUI front-end for the Hercules `@roll` atcommand. Lets players roll dice without
typing chat commands. Added 2026-07-11 (Seal Cascade fork).

`@roll` is a **level-0** command (`bindatcmd("roll", …, 0, 99, 1)` in
`Hercules/npc/custom/dm_campaign/shared/dm_console.txt`), so every player can use
this window. `@roll hidden` requires GM level 60+ and is enforced **server-side** —
the client always offers the toggle; a non-GM who enables it just gets a permission
message back.

## What it does

- **Standard**: d4, d6, d8, d10, d12, d20, d100 — each sends `@roll 1dX`.
- **Common**: 2d6, 3d6, 4d6, 2d20 — each sends `@roll NdX`.
- **Custom**: a text field for any `NdX+mod` expression (e.g. `3d8+2`); Enter or the
  Roll button sends `@roll <expr>` and clears the field.
- **Hidden (GM only)**: a toggle; when on, every roll from this window (buttons and
  custom) is sent as `@roll hidden …` instead of `@roll …`.

## Opening it

- **Ctrl+D** — works even while the chat box is focused (same always-works input
  path as Ctrl+O; see `Hercules/planning/dm-mode-troubleshooting.md` Symptom D).
- **Menu → Dice Roller** (Escape menu).

## How rolls are sent

Each button/field carries a `ClickHandler` closure (the blanket
`impl ClickHandler for F: Fn` in `korangar-interface/src/event/handler.rs`) that
reads the `hidden` bool from window state at click time and queues
`InputEvent::SendMessage { text: "@roll …" }` — the same event the GM/DM panel uses.
The server runs `@roll` and broadcasts the result (public) or self-messages it
(hidden, via `0x017F` — see `dm-atcommand-feedback.md`).

## Files (checklist for rebuild / re-merge)

All client-side; rebuild with `cargo build --release -p korangar` and restart.

- [ ] `korangar/src/interface/windows/dice.rs` — the window + `DiceWindowState`
      (`custom_text`, `hidden`) + `roll()` / `custom_roll()` click-handler builders.
- [ ] `korangar/src/interface/windows/mod.rs` — `mod dice;`, re-export
      `DiceWindow`/`DiceWindowState`, and `WindowClass::Dice`.
- [ ] `korangar/src/interface/windows/cache.rs` — default placement arm for
      `WindowClass::Dice` (the class match is exhaustive; a missing arm is a build
      error `E0004`).
- [ ] `korangar/src/state/mod.rs` — import `DiceWindowState`, the `dice_window`
      field, its `::default()`, and inclusion in the state struct assembly.
- [ ] `korangar/src/input/event.rs` — `InputEvent::ToggleDiceWindow`.
- [ ] `korangar/src/input/mod.rs` — Ctrl+D binding in `push_game_action_keys`
      (always-works path, like Ctrl+O — NOT `handle_keyboard_input`, which would
      double-toggle when chat is unfocused).
- [ ] `korangar/src/lib.rs` — `ToggleDiceWindow` handler
      (`open_window(DiceWindow::new(client_state().dice_window()))`).
- [ ] `korangar/src/interface/windows/menu.rs` — Dice Roller menu button.

## Verify

```bash
strings target/release/korangar | grep 'Dice Roller'
```

In game: **Ctrl+D** → window opens (even with chat focused) → click **d20** → chat
shows `→ @roll 1d20` then the public roll result. Toggle **Hidden**, click **d20** →
result reported only to you (requires GM 60+).

Related: `dm-atcommand-feedback.md` (0x017F feedback, Ctrl+O), `commands.rs`
(GM/DM panel this mirrors), `Hercules/planning/dm-tooling.md` (`@roll` semantics).
