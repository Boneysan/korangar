# Implementation Plan — M1 P0 Verification (E3.1)

| | |
|---|---|
| **Status** | In progress — macOS live pass started 2026-07-10 |
| **Milestone** | M1 — Playable solo |
| **Parent** | [PROJECT_PLAN.md](../PROJECT_PLAN.md) E3 |
| **Depends on** | M0 complete |

**Recent client UX (not a substitute for this checklist):** minimap + Towninfo POIs,
player blip, party/compass marks, open/close preference, hotbar F1–F9 + CD overlay,
HUD (zeny/EXP), msgstringtable for `ZC_MSG`, sit (Insert; Home compatibility alias), dialog input/close,
shops + **EN itemInfo names** (tool dealer checked 2026-07-10), identify/trade/storage
protocol MVPs. Still run every P0 row live.

## 1. Scope

Systematically verify every P0 (and high-value 🔶 P1) row in PROJECT_PLAN §2 against
live Hercules_RO (`PACKETVER=20220406`). File defects; fix in E3.2.

## 2. Environment

```bash
# DB + servers (MariaDB must be running)
cd ~/GitHub/Hercules_RO && ./athena-start start
# expect listeners on 6900 / 6121 / 5121

# Client (WSL — use the GL/D3D12 launcher)
cd ~/GitHub/korangar && ./run-wsl.sh
```

Test account (from M0): `korangar` / `korangar` (or create a fresh novice).

Enable packet logging when debugging: `KORANGAR_PACKET_LOG=1` if supported, or the
in-client packet inspector (`debug` feature).

## 3. Checklist

Mark each item ✅ / ❌ / 🔶 and note the defect.

### Account & session (P0)
- [x] Login with valid credentials — macOS 2026-07-10 (`korangar`; `loginlog` rcode 100)
- [ ] Login failure shows a visible message (bad password / already online)
- [x] Character select lists existing chars — `test` displayed and selectable on macOS
- [ ] Character create works
- [ ] Character delete (if used) fails safely
- [x] Map load after select (no black world / freeze) — entered `int_land`; map TCP connection established
- [ ] Clean logout / disconnect

### Core gameplay (P0)
- [x] Click-to-move, pathing (~10 tiles) — full route live-verified on macOS 2026-07-10
- [x] Sit / stand — Home compatibility binding live-verified on macOS 2026-07-10
- [x] Basic melee attack + damage numbers — Poring live test: sword cursor, approach, hit/miss and damage numbers, moving-target chase; no freeze/disconnect
- [ ] Skill damage numbers (if character has a damaging skill)
- [x] Item pickup from ground — labels English live-verified macOS 2026-07-10 (M1-001); re-check pickup path if needed
- [ ] Stats window + stat allocation (success path)
- [ ] Skill tree opens; skill use (self / target / ground if available)
- [ ] Hotbar use
- [ ] Death → respawn window
- [ ] Weight footer updates in inventory (pick up / drop items; soft/hard color)

### Items & economy (P0)
- [x] Inventory open / use / equip / unequip / drop — open, tooltips, drop, drag, reorder, split live-verified macOS 2026-07-10 (M1-001–003)
- [ ] NPC shop buy
- [ ] NPC shop sell
- [x] NPC shop **English names** (tool dealer — live 2026-07-10; EN `itemInfo` overlay)
- [ ] Item identify (magnifier / double-click unidentified) — protocol landed; verify live
- [ ] Kafra storage open / store / retrieve / close — protocol landed; verify live

### Social (P0)
- [ ] Public chat send + receive

### NPC & world (P0)
- [ ] NPC dialog: mes + next + close
- [ ] NPC dialog: menu choices
- [ ] **NPC number input** (E3.3) — e.g. scripts that call `input .@n`
- [ ] **NPC string input** (E3.3) — e.g. scripts that call `input .@s$`
- [ ] Warp / map change
- [ ] Kafra **save point** (dialog-only; storage window is E4.4)

### Status / feedback (recent work)
- [ ] Buff bar shows timed status (Blessing / `@useskill` / consumable)
- [ ] Buff expires and tile/summary clears
- [ ] Rejection messages in chat (skill fail / known ZC_MSG) — see party-whisper loose end

## 4. NPC input smoke scripts

If no convenient stock NPC uses `input`, temporarily add under
`Hercules_RO/npc/custom/`:

```c
// npc/custom/test_input.txt
prontera,150,180,4	script	InputTester	4_F_KAFRA1,{
	mes "Number test";
	next;
	input .@n;
	mes "You entered: " + .@n;
	next;
	mes "String test";
	next;
	input .@s$;
	mes "You entered: " + .@s$;
	close;
}
```

`@reloadscript` (GM) or restart map-server after adding.

## 5. Defect log

| ID | Area | Severity | Notes | Status |
|---|---|---|---|---|
| M1-001 | Ground items | P0 | Poring drops rendered as `[][][]` on hover. Root cause: startup loaded only Korean `iteminfo.lub`; external English-table link was broken. Durable fix: 13,182 English names from Hercules-backed `docs/items.json` compiled into the client. | ✅ Live-verified macOS 2026-07-10 (ground items show English) |
| M1-002 | Inventory UX | P0 | Hovering inventory items showed no identifying text/tooltip. `ResourceMetadata.name` was already populated; `ItemBox::lay_out` only rendered texture/amount. Fix: register `layout.add_tooltip(&item.metadata.name, …)` on hover (same pattern as `SkillBox`). Applies to inventory, equipment, and storage slots that share `ItemBox`. Framework tooltip delay is ~1s. | ✅ Live-verified macOS 2026-07-10 (hover ~1s shows EN name) |
| M1-003 | Inventory actions | P0 | Drop, drag-to-equip, ground drop, in-inventory reorder, and split (partial drop: half / off 1 / drop all) via right-click menu. `CZ_ITEM_THROW2` 0x0363 + 0x00AF ack; end-of-frame UI event flush for drag. | ✅ Live-verified macOS 2026-07-10 |

## 5.1 Live session notes

- **2026-07-10, macOS:** Hercules login/char/map/api built with PACKETVER
  20220406. Login, existing-character selection, `int_land` load, movement, and
  Home sit/stand passed. Client remained connected to map port 5121.
- Click-to-move completed a route of at least ten tiles without stopping,
  teleporting, or disconnecting.
- Basic melee combat passed against a Poring: hover changed to the sword cursor,
  click approached and attacked, hit/miss and damage numbers rendered, and the
  player chased the moving target without freezing or disconnecting.
- Poring-drop testing found unreadable ground labels plus missing inventory
  tooltips and drop/split actions (M1-001 through M1-003).
- Original Insert sit/stand still needs a keyboard that exposes Insert; Home is
  the verified macOS/laptop compatibility path.

## 6. Exit criteria (E3.1 done)

- Every P0 row above marked ✅ or filed as a defect with repro steps.
- No framing desync / silent hang during a 30-minute mixed session.
- Hand off to E3.2 (fix) then E3.6 (novice → first job, 2-hour stability).
