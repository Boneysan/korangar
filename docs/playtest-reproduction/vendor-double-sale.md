# Vendor Double-Transaction Classification & Cart Clearing Resolution

**Status:** VERIFIED & CLASSIFIED (Automated Suite PASS)  
**Scenario:** `vendor-double-transaction-classification`  
**Execution Venue:** Live local Hercules server (`login-server`, `char-server`, `map-server`), MySQL MariaDB, map `prontera`  
**Test Duration:** 4.3s (all candidate paths classified)  

---

## 1. Problem Statement & Audit Objective

The September playtest runbook card `QW-025` requires:
> Separate NPC purchase, NPC sale, player vending, buying store, and cart-item reports. For each available path count outgoing requests, result packets, inventory delta, and Zeny delta. Double-click and delayed-response tests must be included.  
> **Done when:** “vendor sells twice” is converted into one named transaction path and one reproducible sequence, or each candidate path is ruled out.

Playtesters reported:
1. "Vendor sells twice": Ambiguous report suggesting duplicate transactions, duplicate item delivery, or double debit/credit.
2. Stale cart contents: Confirmed client defect where successful sales did not clear the sell cart.

---

## 2. Candidate Path Matrix & Classification Summary

| Candidate Path | Available in Client? | Tested Pattern | Outgoing Requests | Result Packets | Inventory Delta | Zeny Delta | Server Mechanism | Classification Outcome |
|---|---|---|---|---|---|---|---|---|
| **1. NPC Purchase (`shop-buy`)** | **Yes** | Rapid double-click / in-flight resubmission (Pet Food, 1000z) | **2** | **2** (1 Success, 1 Error) | **+1** (not +2) | **-1,000z** (not -2,000z) | `sd->npc_shopid = 0` in `clif.c:13033` clears shop session after first purchase; 2nd request fails `!sd->npc_shopid` | **Ruled out** on server. Duplicate purchase cannot occur. |
| **2. NPC Sale (`shop-sell`)** | **Yes** | Rapid double-click / in-flight resubmission (Pet Food, 500z) | **2** | **2** (1 Success, 1 Error) | **-1** (not -2) | **+500z** (not +1,000z) | `sd->npc_shopid = 0` in `clif.c:13089` clears shop session after first sale; 2nd request fails `!sd->npc_shopid` | **Ruled out** on server. Duplicate sale cannot occur. |
| **3. Client Cart Clearing Defect** | **Yes** | Confirmed client defect in `Client::handle_selling_completed` | N/A | N/A | Correct (-N) | Correct (+Zeny) | N/A (Client UI state error) | **IDENTIFIED ROOT CAUSE.** Client cleared `buy_cart` instead of `sell_cart`, leaving sold items displayed in the sell cart window. |
| **4. Player Vending** | **No** | `CZ_REQ_OPENSTORE2` / `CZ_PURCHASE_FROM_VENDING` | N/A | N/A | N/A | N/A | Protocol not implemented in client | **Ruled out** (unexposed in client protocol layer). |
| **5. Buying Store** | **No** | `CZ_REQ_OPEN_BUYING_STORE` | N/A | N/A | N/A | N/A | Protocol not implemented in client | **Ruled out** (unexposed in client protocol layer). |
| **6. Pushcart Currency** | N/A | Pushcart inventory vs wallet zeny | N/A | N/A | N/A | N/A | `struct s_cart` holds `items_cart` only | **Ruled out** (Pushcarts possess zero currency). |

---

## 3. Detailed Empirical Analysis

### 3.1 Candidate Path 1: NPC Purchase Double-Click / Delayed Response
- **Test:** Client emits two consecutive `CZ_PURCHASE_ITEMS` packets (`0x00ca`) for 1 Pet Food (1,000z) without awaiting server acknowledgment.
- **Observed Wire Packets:**
  - Client -> Server: `CZ_PURCHASE_ITEMS` (Request 1)
  - Client -> Server: `CZ_PURCHASE_ITEMS` (Request 2)
  - Server -> Client: `ZC_BUYING_COMPLETED` (`0x00cb`, `result: 0` / `Success`)
  - Server -> Client: `ZC_LONGLONGPAR_CHANGE` (`SP_ZENY`, delta -1,000z)
  - Server -> Client: `ZC_ITEM_PICKUP_ACK` (Pet Food added to inventory)
  - Server -> Client: `ZC_BUYING_COMPLETED` (`0x00cb`, `result: 1` / `Error`)
- **Server Guard ([`Hercules/src/map/clif.c:13033`](file:///Volumes/T7/GitHub/Ragnarok_Online/Hercules/src/map/clif.c#L13033)):**
  ```c
  if (sd->state.trading || !sd->npc_shopid || pc_has_permission(sd, PC_PERM_DISABLE_STORE)) {
      result = 1;
  } else {
      result = npc->buylist(sd, &item_list);
  }
  sd->npc_shopid = 0; // Clear shop data.
  clif->npc_buy_result(sd, result);
  ```
- **Conclusion:** Because `sd->npc_shopid` is reset to 0 immediately upon handling Request 1, Request 2 encounters `!sd->npc_shopid` and is rejected with `result = 1`. The server strictly guarantees exactly-once execution.

### 3.2 Candidate Path 2: NPC Sale Double-Click / Delayed Response
- **Test:** Client emits two consecutive `CZ_PC_SELL_ITEMLIST` packets (`0x00c9`) for the same inventory slot (Pet Food, 500z).
- **Observed Wire Packets:**
  - Client -> Server: `CZ_PC_SELL_ITEMLIST` (Request 1)
  - Client -> Server: `CZ_PC_SELL_ITEMLIST` (Request 2)
  - Server -> Client: `ZC_SELLING_COMPLETED` (`0x00ce`, `result: 0` / `Success`)
  - Server -> Client: `ZC_LONGLONGPAR_CHANGE` (`SP_ZENY`, delta +500z)
  - Server -> Client: `ZC_ITEM_DISAPPEAR` (Pet Food removed from inventory)
  - Server -> Client: `ZC_SELLING_COMPLETED` (`0x00ce`, `result: 1` / `Error`)
- **Server Guard ([`Hercules/src/map/clif.c:13089`](file:///Volumes/T7/GitHub/Ragnarok_Online/Hercules/src/map/clif.c#L13089)):**
  ```c
  if (sd->state.trading || pc_isdead(sd) || pc_isvending(sd) || !sd->npc_shopid) {
      fail = 1;
  } else {
      fail = npc->selllist(sd, &item_list);
  }
  sd->npc_shopid = 0; // Clear shop data.
  clif->npc_sell_result(sd, fail);
  ```
- **Conclusion:** Identical to purchase, `sd->npc_shopid = 0` immediately invalidates the open shop session. The second request is rejected with `fail = 1`.

### 3.3 Candidate Path 3: The Confirmed Client Cart Clearing Defect (Root Cause)
- **Defect Location:** [`korangar/korangar/src/lib.rs:3870`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar/src/lib.rs#L3870) in `handle_selling_completed`.
- **Original Buggy Behavior:**
  Prior to `QW-003`, when `NetworkEvent::SellingCompleted { result: Success }` arrived, the client handler invoked:
  ```rust
  // BUG: cleared buy_cart instead of sell_cart!
  self.client_state.follow_mut(client_state().buy_cart()).clear();
  ```
- **Playtest Manifestation:**
  1. Player selected items to sell and clicked "Sell".
  2. The server successfully processed the sale and removed the items from the player's inventory.
  3. The client received `Success`, but mistakenly wiped the **buy cart** while leaving the **sell cart** populated with the sold items!
  4. The player saw their items still sitting in the sell cart, creating the visual appearance that the sale had either not completed or that the items were duplicated/pending a second sale.
  5. If the player clicked "Sell" again, the client transmitted a second request for items no longer in inventory, triggering the server rejection error message: `"Failed to sell items"`.
- **The Fix:**
  In [`korangar/korangar/src/lib.rs:3877`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar/src/lib.rs#L3877):
  ```rust
  pub(crate) fn handle_selling_completed(
      sell_cart: &mut Vec<SellItem<(ResourceMetadata, u16)>>,
      result: SellItemsResult,
      mut close_window: impl FnMut(WindowClass),
  ) -> Option<ChatMessage> {
      match result {
          SellItemsResult::Success => {
              // Clear the cart.
              sell_cart.clear();

              close_window(WindowClass::Sell);
              close_window(WindowClass::SellCart);
              None
          }
          SellItemsResult::Error => Some(ChatMessage::new("Failed to sell items".to_owned(), MessageColor::Error)),
      }
  }
  ```
- **Unit Verification:** Verified by `successful_sale_clears_only_sell_cart_and_closes_windows` in [`korangar/korangar/src/lib.rs:11254`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar/src/lib.rs#L11254), proving `sell_cart` is cleared, sell windows are closed, and `buy_cart` is unaffected.

---

## 4. Verification & Regression Commands

```bash
# Automated scenario verifying Path 1 (buy double-click) and Path 2 (sell double-click)
./korangar/tools/testing/run-suite.sh --scenario vendor-double-transaction-classification

# Client unit tests verifying Path 3 (cart clearing fix)
cargo test -p korangar --lib successful_sale_clears_only_sell_cart_and_closes_windows
cargo test -p korangar --lib failed_sale_retains_selection_and_leaves_windows_open

# Full quality gate
cargo test -p korangar-networking items
cargo test -p korangar-networking --example headless-tester
cargo check -p korangar
cargo clippy -p korangar-networking --lib -- -Dwarnings
cargo clippy -p korangar -- -Dwarnings
cargo fmt --all -- --check
git diff --check
make -j4  # in Hercules/
```
