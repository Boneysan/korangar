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
- [x] Death → respawn window — killed on `moc_fild22`; Respawn returned to the saved Payon point on macOS 2026-07-11
- [ ] Weight footer updates in inventory (pick up / drop items; soft/hard color)

### Items & economy (P0)
- [x] Inventory open / use / equip / unequip / drop — open, tooltips, drop, drag, reorder, split live-verified macOS 2026-07-10 (M1-001–003)
- [ ] NPC shop buy
- [ ] NPC shop sell
- [x] NPC shop **English names** (tool dealer — live 2026-07-10; EN `itemInfo` overlay)
- [ ] Item identify (magnifier / double-click unidentified) — protocol landed; verify live
- [x] Kafra storage open / store / retrieve / close — UI grid live 2026-07-11 (stock Kafra uses `close2` then `openstorage`: click dialog **Close** first; see [storage-window.md](../storage-window.md))

### Social (P0)
- [ ] Public chat send + receive

### NPC & world (P0)
- [x] NPC dialog: mes + next + close — live via Kafra 2026-07-11
- [x] NPC dialog: menu choices — live via Kafra teleport 2026-07-11
- [ ] **NPC number input** (E3.3) — e.g. scripts that call `input .@n`
- [ ] **NPC string input** (E3.3) — e.g. scripts that call `input .@s$`
- [x] Warp / map change — Kafra teleport live-verified 2026-07-11
- [x] Kafra **save point** (dialog-only; storage window is E4.4) — live-verified 2026-07-11

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
| M1-004 | Campaign map placement | P0 | Arc 19 warped players and placed its encounter at non-walkable void cells `moc_fild22,150,150` / `155,150`, producing black surroundings, no destination marker, and no movement. Deep audit proved the map, lighting, terrain, and pathing data valid. Moved the rift/boss/hazards to walkable `(170,140)` and the choice NPC to `(175,140)`. | ✅ Live-verified macOS 2026-07-11 |

## 5.1 Live session notes

- **2026-07-11, macOS:** release build and full `cargo test --all-features`
  baseline passed (271 unit tests plus 2 compile-fail doc tests). The GM/DM
  panel opened with Ctrl+O, all six tabs (DM, Beats, Character, Items, Combat,
  Travel) rendered and switched correctly, and Ctrl+O continued to work while
  chat retained keyboard focus.
- **2026-07-11 map asset audit:** compared all 1,156 maps enabled by Hercules
  with `data.grf` + `rdata.grf`. 274 lack at least one same-named core `.rsw`,
  `.gnd`, or `.gat` asset. `moc_fild22` has all three, narrowing M1-004 to a
  deeper load/render issue rather than absent core map data.
- **2026-07-11 campaign coordinate audit:** decrypted and parsed each available
  RSW plus its referenced GND/GAT. `moc_fild22` has 62,367 walkable cells, but
  the Arc 19 points `(150,150)` and `(155,150)` were both void cells. Moving
  onto nearby ground restored the destination marker. The automated scan of
  all 60 static `DM_WarpParty` calls found 21 additional non-walkable targets
  for follow-up.
- **2026-07-11 campaign teleport remediation:** the audit was expanded across
  all Hercules static `warp`, warp-portal, and `DM_WarpParty` destinations. It
  discovered 5,677 destinations total: 371 cannot be checked with the current
  GRFs and 79 were non-walkable in available maps. The 21 actionable campaign
  destinations were moved to their nearest walkable cells; the focused scan
  now passes all 59 statically analyzable DM party warps with zero unsafe
  destinations. Dynamic GM-entered coordinates remain runtime-validated.
  The complete unresolved stock/legacy and missing-asset inventory is preserved
  in [teleport-audit-2026-07-11.md](../reports/teleport-audit-2026-07-11.md).
- **2026-07-11 Arc 19 coordinate recheck:** after `@reloadscript`, warping to
  `moc_fild22,170,140` showed visible ground, normal movement, and the
  destination marker. The Central Choice appeared on the ground at `(175,140)`
  and was interactable. M1-004 closed.
- **2026-07-11 death/respawn:** a high-level enemy killed `test` on
  `moc_fild22`; the respawn window appeared and Respawn returned the character
  to the previously saved Payon point.
- **2026-07-11, macOS:** Kafra teleport (dialog + menu + warp) and Kafra **save
  point** live-verified. DM mode on/off + `[DM]` chat feedback fixed earlier same
  session (`0x017F` + `dm_console` `@` bridge). Kafra **storage** grid opens after
  dialog Close (`close2` → `openstorage`); drag/store path wired.
- **Also same day:** client `DisplayBottomMessagePacket` (0x017F) for `dispbottom`;
  local `→ @cmd` echo; storage window grid + `0x0B44` item-added.
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
