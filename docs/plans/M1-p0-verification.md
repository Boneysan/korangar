# Implementation Plan — M1 P0 Verification (E3.1)

| | |
|---|---|
| **Status** | Ready to run (servers available) — **next priority after 2026-07-10 UX slice** |
| **Milestone** | M1 — Playable solo |
| **Parent** | [PROJECT_PLAN.md](../PROJECT_PLAN.md) E3 |
| **Depends on** | M0 complete |

**Recent client UX (not a substitute for this checklist):** minimap + Towninfo POIs,
player blip, open/close preference, hotbar F1–F10 labels, msgstringtable for `ZC_MSG`,
sit (Insert), dialog input/close fixes. Still run every P0 row live.

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
- [ ] Login with valid credentials
- [ ] Login failure shows a visible message (bad password / already online)
- [ ] Character select lists existing chars
- [ ] Character create works
- [ ] Character delete (if used) fails safely
- [ ] Map load after select (no black world / freeze)
- [ ] Clean logout / disconnect

### Core gameplay (P0)
- [ ] Click-to-move, pathing (~10 tiles)
- [ ] Sit / stand
- [ ] Basic melee attack + damage numbers
- [ ] Skill damage numbers (if character has a damaging skill)
- [ ] Item pickup from ground
- [ ] Stats window + stat allocation (success path)
- [ ] Skill tree opens; skill use (self / target / ground if available)
- [ ] Hotbar use
- [ ] Death → respawn window
- [ ] Weight footer updates in inventory (pick up / drop items; soft/hard color)

### Items & economy (P0)
- [ ] Inventory open / use / equip / unequip / drop
- [ ] NPC shop buy
- [ ] NPC shop sell

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
| | | | | |

## 6. Exit criteria (E3.1 done)

- Every P0 row above marked ✅ or filed as a defect with repro steps.
- No framing desync / silent hang during a 30-minute mixed session.
- Hand off to E3.2 (fix) then E3.6 (novice → first job, 2-hour stability).
