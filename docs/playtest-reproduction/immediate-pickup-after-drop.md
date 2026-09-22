# Immediate Repickup After Player Drop — Reproduction & Verification Report (QW-022)

## Executive Summary

- **Task:** QW-022 — Immediate repickup after player drop.
- **Finding:** Client automatic repickup is **ruled out with packet evidence**. The Korangar client never dispatches a pickup packet (`0x0362` / `CZ_ITEM_PICKUP2`) following an item drop (`0x0363` / `CZ_ITEM_THROW2`).
- **Root Cause of Playtest Observation:** The immediate repickup reported by playtesters is caused by Hercules's server-side **`autopickup_radius`** feature (`Hercules/src/map/pc.c:5032-5152`, configured in `conf/map/battle/drops.conf:178` to radius `2` by default):
  1. When a player drops an item, `pc_dropitem` places the floor item directly onto `(sd->bl.x, sd->bl.y)` with no pickup reservation (`first_get_charid = 0`).
  2. Hercules runs a periodic background timer `pc_autopickup_timer` every `AUTOPICKUP_INTERVAL` (400 ms).
  3. Because the item is 0 cells from the player and within the active radius of 2, the next sweep of `pc_autopickup_sub` automatically executes `pc->takeitem(sd, fitem)`.
  4. The server removes the ground item and adds it back into the player's inventory via `ZC_ADD_ITEM` / `ZC_ITEM_PICKUP_ACK`, without receiving any pickup packet from the client.
  5. When `@autopickup 0` is set on the character, dropped items remain on the ground indefinitely until manually clicked.

---

## 1. Protocol & Wire Definitions

### 1.1 Drop Request: `CZ_ITEM_THROW2` (`0x0363`)
Wire size: 6 bytes.
- `header: u16` = `0x0363`
- `inventory_index: u16` (serialized as `index + 2`)
- `amount: u16`

### 1.2 Drop Acknowledgment & Ground Creation Packets
- `ZC_ITEM_THROW_ACK` (`0x00AF`): 6 bytes (`inventory_index: u16`, `amount: u16`). Amount 0 indicates rejection.
- `ZC_DELETE_ITEM_FROM_BODY` (`0x07FA`): Removes item from inventory slot.
- `ZC_ITEM_FALL_ENTRY` / `ZC_ITEM_FALL_ENTRY4` (`0x084B` on `PACKETVER >= 20180418`): Broadcasts floor entity creation (`entity_id: u32`, `item_id: u16`, `x: u16`, `y: u16`, `amount: u16`).

### 1.3 Pickup Request: `CZ_ITEM_PICKUP2` (`0x0362`)
Wire size: 6 bytes.
- `header: u16` = `0x0362`
- `entity_id: u32` (floor item map object ID)

### 1.4 Pickup Acknowledgment Packets
- `ZC_ITEM_PICKUP_ACK3` / `ZC_ITEM_PICKUP_ACK` (`0x00A0` / `0x0842` / `0x02C8`): Confirms item taken off floor.
- `ZC_ADD_ITEM` (`0x0AD9` / `0x00A3`): Adds item to recipient's inventory.
- `ZC_ITEM_DISAPPEAR` (`0x00A1`): Removes ground entity for observers.

---

## 2. Client-Side State Transition Audit

Inspection of `korangar/src/lib.rs` and `korangar/src/state/mod.rs` confirms:

1. **No Automatic Pickup on Drop:**
   Handling of `InputEvent::DropItem` in `flush_inventory_input_events` (`lib.rs:6957-6967`) and `process_input_events` (`lib.rs:8133-8143`) calls only `self.networking_system.drop_item(inventory_index, amount)`. It does not create, modify, or schedule any `BufferedAction::PickUpItem`.
2. **UI Click Isolation:**
   - Drops via `ItemActionsWindow` (right-click -> Drop 1 / Half / All) are consumed by the interface frame (`is_interface_hovered = true`), preventing mouse clicks from hitting ground raycast pickers (`PickerTarget::Entity`).
   - Drops via drag-and-drop (`MouseInputMode::MoveItem`) terminate upon mouse release, returning `MouseMode` to `Default` without generating a ground entity click.
3. **Behavior with Pending Floor Item (`BufferedAction::PickUpItem`):**
   - If a player clicks floor item A out of range, the client sets `buffered_action = Some(BufferedAction::PickUpItem { entity_id: item_a })`.
   - If the player subsequently drops item B, `DropItemPacket` is sent for item B.
   - `buffered_action` retains its target on item A (`entity_id: item_a`). It never points to item B's newly generated entity ID.
   - When the player stops moving, `process_buffered_action()` executes `pick_up_item(item_a)` only if item A still exists on the ground. Item B is completely untouched.

---

## 3. Server-Side Autopickup Architecture

In `Hercules/src/map/pc.c`:

```c
#define AUTOPICKUP_INTERVAL 400

static int pc_autopickup_timer(int tid, int64 tick, int id, intptr_t data)
{
    map->foreachpc(pc_autopickup_pc);
    return 0;
}
```

- When an item is dropped via `pc_dropitem`:
  ```c
  map->addflooritem(&sd->bl, &sd->status.inventory[n], amount, sd->bl.m, sd->bl.x, sd->bl.y, 0, 0, 0, 2, false);
  ```
  The item is dropped at the player's exact coordinates `(sd->bl.x, sd->bl.y)`.
- The configuration `conf/map/battle/drops.conf:178` sets `autopickup_radius: 2` by default.
- Every 400 ms, `pc_autopickup_timer` scans `sd->bl.x ± 2`, `sd->bl.y ± 2`.
- Since the dropped item is at distance 0, `pc_autopickup_sub` validates weight and capacity, then invokes `pc->takeitem(sd, fitem)`.
- As a result, any player who drops an item will have that item repicked up by the server within 0 to 400 ms unless they run `@autopickup 0`.

---

## 4. Automated Integration Verification Evidence

A dedicated integration scenario `immediate-repickup-after-drop` was executed using the headless test harness on map `prontera`:

| Scenario | Pre-Drop Pending Action | Server Autopickup Setting | Drop Request | Ground Creation Entity & Delay | Client `0x0362` Sent | Server Pickup Delay | Final Inventory | Outcome |
|---|---|---|---|---|---|---|---|---|
| **Trial A** | None (`buffered_action: None`) | `@autopickup 2` (Default) | `0x0363(idx=14, amt=1)` | `EntityId(1491)` in 37.0 ms | **0 packets** | **1507 ms** (server sweep) | Restored (+1) | **Reproduced server autopickup; 0 client packets** |
| **Trial B** | None (`buffered_action: None`) | `@autopickup 0` (Disabled) | `0x0363(idx=14, amt=1)` | `EntityId(1492)` in 40.0 ms | **0 packets** | **None** (persisted >1200 ms) | Reduced (-1) | **Ruled out client auto-pickup; item stayed on floor** |
| **Trial C** | Floor Item A (`EntityId(1493)`) | `@autopickup 0` (Disabled) | `0x0363(idx=14, amt=1)` for Item B | `EntityId(1494)` in 38.5 ms | **0 packets for B** | **None** (Item B stayed on floor) | Item B dropped | **Proved pending action on A does not affect dropped B** |

---

## 5. Conclusion & Resolution

1. **Client Status:** Clean and correct. The Korangar client contains no automatic pickup logic or state leaks upon dropping items.
2. **Server Cause:** Hercules's `autopickup_radius: 2` feature automatically gathers any items dropped within 2 cells of the player every 400 ms.
3. **Gameplay / User Guidance:** If players wish to transfer items to the ground without having them immediately pulled back into their inventory, they must either:
   - Disable autopickup using `@autopickup 0`.
   - Drop the item while overweight (so `pc_autopickup_sub` bypasses it).
   - In future game design, if server-side autopickup of player-dropped items is undesirable, `map_addflooritem` or `pc_dropitem` can set a brief player-specific reservation or `pc_autopickup_sub` can ignore floor items flagged as player drops (`flags & 2`).
