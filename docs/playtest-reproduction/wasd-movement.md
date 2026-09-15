# WASD Movement Protocol — QW-027 LAN trace

## Problem Statement

Reported: Internet/WAN WASD movement rubber-bands in ways click-to-move does
not on LAN. This document records the first redundant/stale request and
correction mechanism, using the packets this checkout actually sends.

## Test Environment

| Parameter | Value |
|-----------|-------|
| Korangar | `7d40510f` plus the QW-027 working tree (`input/wasd.rs`, `KORANGAR_WASD_TRACE` fields) |
| Hercules | `8b850e4e5` (`agent/map-teleport-safety`), map-server on `127.0.0.1:5121` |
| Packet version | `20220406` |
| Map / route | `prontera` start `(155, 180)` |
| Character | headless GM `korangar` / `test` |
| Scenario | `wasd-lan-trace` |

## Packet structures (source-confirmed)

Do not use the Phase-0 template's `CZ_MOVE_PLAYER` / `0x0073`. This fork's
movement request is:

- Client → server: `RequestPlayerMovePacket` header `0x035F`
  (`ragnarok-packets` `RequestPlayerMovePacket`, bytes proven by
  `request_player_move_packet_matches_20220406_opcode`:
  `[0x5F, 0x03, …]`).
- Server → walking client: `PlayerMovePacket` header `0x0087`
  (`origin`, `destination`, `starting_timestamp`).
- Server → stop: `EntityStopMovePacket` / `ZC_STOPMOVE` header `0x0088`.

WASD and click-to-move use the same `0x035F`. They differ in *how many* are
sent and *when*.

## Client send policy (source-confirmed + automated)

`korangar/src/input/wasd.rs` is what `apply_keyboard_move` runs:

- Throttle: 200 ms between `0x035F` sends.
- Press and hold both name the longest straight walkable path, up to 15 cells
  (`max_walk_path` is 17 stock in `Hercules/conf/map/battle/client.conf`).
- A held key is Silent until the direction changes or the character is within
  1 cell of the stored dest.
- Key release names one cell ahead of the client's tile, never the tile
  underfoot.
- After a warp/knockback that leaves the stored dest more than 1 cell away, a
  still-held key is Silent (stale intent). The client does not repath.

`KORANGAR_WASD_TRACE=1` now logs `press` / `extend` / `stop` / `click`, the
`0x0087` origin/dest, client-believed tile, correction cells, dest_delta, ack
delay, and `slide` / `stop-move` / `change-map` interrupts (including whether
held intent is stale).

The historical one-cell-then-15-cell pair at the start of every walk is gone:
`historical_one_cell_then_path_pair_is_gone` asserts a press sends 15 cells and
the next hold 200 ms later is Silent.

## Route and LAN matrix (observed)

Live run: `./tools/testing/run-suite.sh --scenario wasd-lan-trace`
(2026-09-14, 16.6 s, PASS). Headless cannot press W; it sends the `0x035F`
sequence the WASD policy emits.

### Route A — east along `y = 180`

| Case | Request (`0x035F`) | `0x0087` acks | Origin → dest | Notes |
|------|--------------------|---------------|---------------|-------|
| Click-to-move | `(165, 180)` (10 east) | 1 | `(155, 180)` → `(165, 180)` | dest_delta 0, correction vs start 0 |
| WASD press | `(170, 180)` (15 east) | 1 | `(155, 180)` → `(170, 180)` | dest_delta 0; no second ack |
| Stop-ahead | `(157, 180)` while 15-cell walk in flight | 1 | `(158, 180)` → `(157, 180)` | dest_delta 0; 1-cell **backward** snap |
| Duplicate click dest | two `(165, 180)` | 2 | `(155, 180)` → `(165, 180)` then `(156, 180)` → `(165, 180)` | second ack is a redundant correction |
| Far warp `(155, 170)` then stale dest `(170, 180)` | 1 | 0 | — | server dropped (path longer than `max_walk_path`) |
| Near warp `(155, 182)` then stale dest `(170, 180)` | 1 | 1 | `(155, 182)` → `(170, 180)` | server honors a nearby stale dest; client policy would stay Silent |

Stop-ahead correction vs the in-flight dest `(170, 180)` was chebyshev 12: the
server origin was the cell it had actually reached, not the long dest the
client still believed.

A prior run of the same scenario with `@useskill 678 5 self` (`NPC_WIDESTUN`)
also produced `EntityStopMove` (`0x0088`) at `(156, 180)` while the 15-cell
walk was in flight. That command was removed from the checked-in scenario
because it emitted unmodeled header `0x0A41` and failed the packet gate.

## First redundant / stale / correction mechanism

Identified on LAN:

1. **Redundant request:** a second `RequestPlayerMovePacket` (`0x035F`) while a
   walk is already in flight — stop-ahead, duplicate click, or a nearby stale
   dest after warp — is accepted and answered with another `PlayerMovePacket`
   (`0x0087`) whose origin is the server's current tile. That origin is the
   snap. Click-to-move does not send that second packet, so it has no mid-walk
   snap.
2. **Visible LAN snap:** stop-ahead walked the character from `(158, 180)`
   **back** to `(157, 180)` (one cell of rubber-band).
3. **Stale held intent (client):** after an authoritative position jump more
   than 1 cell from the stored dest, `decide_keyboard_move` stays Silent. A
   held W will not resume until a fresh press or a direction change. QW-034
   is the follow-up that should clear that intent.

The historical press-then-extend pair is not the current bug.

## Network profiles

| Profile | Status | Evidence |
|---------|--------|----------|
| LAN, ~0–5 ms, 0% loss | Observed | table above; extra `0x035F` is sufficient to snap even with no added delay |
| 75 / 150 / 250 ms + jitter | BLOCKED | `dnctl`/`pfctl` require sudo; char-server advertises the map port, so a userspace delay proxy was not inserted |
| 1–3% loss | BLOCKED | same: no packet-loss injector without sudo |
| Seated graphical `KORANGAR_WASD_TRACE` | Not captured | no interactive client session this run; tracer fields are in the client and match the LAN `0x035F`/`0x0087` sequence |

WAN is expected to make the same extra `0x0087` origin older relative to the
client's believed tile, which is why the playtest saw it on the internet and
not on LAN for ordinary holds. LAN already shows the stop-ahead snap.

## Actual vs expected

| Action | Expected | Observed on LAN |
|--------|----------|-----------------|
| Hold WASD / 15-cell `0x035F` | One path, one `0x0087` | One ack, dest exact, no one-cell pair |
| Click-to-move | One `0x035F`, one `0x0087` | One ack, dest exact, correction 0 |
| Key release / stop-ahead | Stop without walking backward | Extra `0x0087` from `(158,180)` to `(157,180)` — 1-cell reverse |
| Duplicate dest | Second request ignored or coalesced | Two `0x0087` acks; second origin had already stepped |
| Warp + held dest | Client must not resend the old dest | Client policy Silent; server still honors a nearby stale dest if one is sent |

## Next (not this card)

- QW-033: coalesce so stop/extend do not restart from a behind origin.
- QW-034: clear `keyboard_move_target` on `0x0088`, slide, warp, stun.
- QW-035: repeat this matrix at 150 ms once a delay injector exists.
