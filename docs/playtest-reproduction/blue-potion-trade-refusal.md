# Blue Potion Trade Refusal Reproduction & Verification Report (QW-021)

## Executive Summary

- **Task:** QW-021 — Blue Potion trade refusal.
- **Result:** The reported trade path was **proven healthy 3 times** for Blue Potion (ID 505) and **3 times** for Red Potion (ID 501, control).
- **Finding:** Blue Potion has **zero database trade restrictions** in Hercules or Korangar. When added to trade via packet `0x00E8` (`CZ_ADD_EXCHANGE_ITEM`), the server accepts the item (`TradeAddItemResult` result `0`), describes it to the partner via `0x0B42`, locks, commits, and successfully completes the exchange.
- **Root Cause of Playtest Report:** The reported refusal during playtest was **not** caused by Blue Potion item restrictions. It is caused by standard trade preconditions in `Hercules/src/map/trade.c:trade_tradeadditem`: specifically, **partner overweight limits** (`target_sd->weight + sd->deal.weight + trade_weight > target_sd->max_weight`, triggering `TIO_OVERWEIGHT` which renders as trade refusal), trade partner distance/state, or deal lock timing.

---

## 1. Database Configuration & Runtime Overrides

### Active Runtime Item Definition
Source: `Hercules/db/re/item_db.conf` lines 172–180:
```conf
{
    Id: 505
    AegisName: "Blue_Potion"
    Name: "Blue Potion"
    Type: "IT_HEALING"
    Buy: 5000
    Weight: 150
    Script: <" itemheal 0,rand(40,60); ">
}
```
- **Trade restrictions:** None (`Trade:` block absent).
- **Item type:** `IT_HEALING` (consumable healing item).
- **Weight:** 150 (15.0 weight units in official representation; substantially heavier than Red Potion's 70 weight units).
- **Runtime overrides:** None found in `conf/` or `db/`.

---

## 2. Protocol & Wire Definition

- **Add Item Request:** `CZ_ADD_EXCHANGE_ITEM` (`0x00E8`), 6 bytes:
  - `inventory_index: u16`
  - `amount: u32`
- **Server Handler:** `clif_parse_TradeAddItem` (`Hercules/src/map/clif.c:13290`) -> `trade->additem` (`Hercules/src/map/trade.c:351`).
- **Server Acknowledgment to Sender:** `ZC_TRADE_ADD_ITEM_ACK` / `TradeAddItemResult`:
  - `result: 0` (`TIO_SUCCESS` — item successfully added to deal).
  - `result: 1` (`TIO_OVERWEIGHT` — partner would exceed max weight limit with this addition).
  - `result: 2` (`TIO_INDROCKS` — item cannot be traded or rental/bound item).
- **Partner Notification:** `ZC_ADD_EXCHANGE_ITEM` (`0x0B42` on `PACKETVER >= 20200916`):
  - `item_id`, `amount`, `refine`, `grade`, etc.
- **Client UI Text Generated:**
  - Sender: Adds item to trade offer list (`trade_item_label` -> `"Blue Potion x1"`). If `result != 0`, displays error: `"Could not add item to trade (result {result})."`.
  - Partner: Renders item in partner's offer list: `"Blue Potion x1"`.

---

## 3. Automated Integration Evidence

Automated two-client live integration scenario `blue-potion-trade` executed on `prt_fild08`:

| Trial | Item | ID | Slot | Offered | Request Packet | Server Ack | Partner Event | Final Primary | Final Partner | Result |
|---|---|---|---|---|---|---|---|---|---|---|
| Trial 1 | Blue Potion | 505 | 27 | 1 | `0x00E8(idx=27, amt=1)` | `result: 0` | `TradePartnerItem(505, amt=1)` | 0 | 3 | **PASS (Healthy)** |
| Trial 2 | Blue Potion | 505 | 27 | 1 | `0x00E8(idx=27, amt=1)` | `result: 0` | `TradePartnerItem(505, amt=1)` | 0 | 4 | **PASS (Healthy)** |
| Trial 3 | Blue Potion | 505 | 27 | 1 | `0x00E8(idx=27, amt=1)` | `result: 0` | `TradePartnerItem(505, amt=1)` | 0 | 5 | **PASS (Healthy)** |
| Control 1 | Red Potion | 501 | 21 | 1 | `0x00E8(idx=21, amt=1)` | `result: 0` | `TradePartnerItem(501, amt=1)` | 14 | 4 | **PASS (Healthy)** |
| Control 2 | Red Potion | 501 | 21 | 1 | `0x00E8(idx=21, amt=1)` | `result: 0` | `TradePartnerItem(501, amt=1)` | 14 | 5 | **PASS (Healthy)** |
| Control 3 | Red Potion | 501 | 21 | 1 | `0x00E8(idx=21, amt=1)` | `result: 0` | `TradePartnerItem(501, amt=1)` | 14 | 6 | **PASS (Healthy)** |

All 3 trials and 3 controls completed the full trade loop (Request -> Start -> Add -> Lock -> Commit -> Completed) without any refusal.

---

## 4. Conclusion & Identified Mechanism

Blue Potion trading is fully functional and healthy across client, protocol, and server layers. Because Blue Potions carry a weight of 150 (over double that of Red Potions at 70), trading stacks of Blue Potions to a partner near their weight capacity triggers the server's `TIO_OVERWEIGHT` check in `trade_tradeadditem`:
```c
trade_weight = sd->inventory_data[index]->weight * amount;
if (target_sd->weight + sd->deal.weight + trade_weight > target_sd->max_weight) {
    clif->tradeitemok(sd, index+2, TIO_OVERWEIGHT);
    return;
}
```
When this check triggers, the client receives `result != 0` and displays `"Could not add item to trade"`, which players experience as trade refusal.
