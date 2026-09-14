# Selling Equipped Item — Phase 0 Reproduction Template

## Problem Statement

**From the plan (line 54-55):**
> Reproduce selling an equipped item and determine whether the server accepts it or the client sends the wrong inventory index.

**Reported behavior:** Player equipped an item, then attempted to sell it through a vendor. The expected behavior is that equipped items cannot be sold (server should reject), but there's uncertainty about whether:
1. Server incorrectly accepts the sell (security flaw)
2. Client sends incorrect inventory index when item is equipped

## Test Environment

| Parameter | Value |
|-----------|-------|
| Client Build | korangar HEAD (commit hash) |
| Server Build | Hercules hercules-2025.09 branch |
| Map | midgard 1 (near vendor NPC) |
| Character Weight | Normal (under 30%) |

## Reproduction Steps

### Test Case 1: Equip then Sell
```
1. Login character with normal inventory weight
2. Equip an item (e.g., weapon, armor, accessory)
3. Approach a vending NPC (e.g., kafra, town vendor)
4. Open the vendor window
5. Select "Sell Items" or "Buy/Sell"
6. Attempt to sell the equipped item
7. Note: Any error message, packet sequence, and server response
```

### Test Case 2: Sell While Equipped (Direct)
```
1. Login character with item in inventory (not equipped)
2. Equip the item
3. Immediately try to sell withoutunequipping first
4. Observe if UI allows selecting equipped items
```

## Expected Behavior

| Action | Expected Result |
|--------|-----------------|
| Equipped item shown in vendor list | Should be filtered/grayed out or hidden |
| Attempt to sell equipped item | Server should reject with error |
| Inventory index sent for equipped item | Must match actual unequipped slot |

## What to Capture

### Client-Side Evidence
1. **UI behavior:** Does the equipped item appear in the vendor list?
   - If yes: Is it selectable? Grayed out? Visible but non-interactable?
2. **Inventory window:** Check if equipped status is indicated
3. **Packet capture (KORANGAR_PACKET_LOG=1):**
   ```
   Packet: CZ_ITEM_SELL (0x0076) or vendor sell packet
   - inventory_index: ???
   - amount: ???
   ```

### Server-Side Evidence
1. **Hercules logs:** Enable debug level for map/clif
2. **Key log locations to check:**
   - `clif_parse_VendingListReq` or similar vendor sell handler
   - Inventory access validation when item is equipped
3. **Server response packet:** What error code if any?
4. **Log excerpt format:**
   ```
   [Debug] clif.c:XXXXX - vend list request for inventory index XXX
   [Warning] ... or [Error] if rejection occurs
   ```

## Key Packets to Capture

### CZ_ITEM_SELL / Vendor Sell Request
```
struct CZ_ITEM_SELL {
    int16 packetType;        // Vendor sell packet ID
    uint8 inventory_index;   // The inventory slot being sold
    uint32 amount;           // Number of items to sell
} __attribute__((packed));
```

### ZC_ACK_SELL / Server Response
```
struct ZC_ACK_SELL {
    int16 packetType;        // Acknowledgment packet ID
    uint8 result;            // 0=failed, 1=success
    // May include error code or reason
} __attribute__((packed));
```

## Failure Scenarios to Document

| Symptom | Question |
|---------|----------|
| Server accepts sell of equipped item | Is this a security hole? Does server validate equipped status? |
| Client sends wrong inventory index | What index does client send vs actual slot? |
| Item sells but server rejects | Packet-level rejection with what cause? |
| UI hides equipped items | Expected behavior - document the filter logic |

## Analysis Criteria

The issue is **confirmed** when:
1. Reproduction steps work consistently (3+ trials)
2. Server packet log shows actual inventory index sent
3. Server response either accepts (security concern) or rejects with specific error
4. Determine root cause: client bug (wrong index) vs server bug (no validation)

## References

-korangar/src/network/client/packet.rs - Sell item handling
- korangar/src/interface/vendor.rs - Vendor UI logic
- Hercules/src/map/clif.c - `clif_parse_Vending*` handlers
- Hercules/src/map/inventory.h - Equipped status flags
- Item database: What inventory slot types are considered "equipped"?

## Notes from Previous Sessions

From 2026-07-24 session notes, no equipped-item selling evidence found.
This requires live testing with an equipped item being sold.
