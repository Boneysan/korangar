# Zeny Transaction Verification & Persistence Matrix

**Status:** VERIFIED (Automated Suite PASS)  
**Scenario:** `zeny-persistence-transaction-matrix`  
**Execution Venue:** Live local Hercules server (`login-server`, `char-server`, `map-server`), MySQL MariaDB (`127.0.0.1:3306`), maps `prontera` and `prt_fild08`  
**Test Suite Time:** 25.0s (all 7 sections passing)  

---

## 1. Problem Statement & Architecture Audit

The September playtest runbook card `QW-024` requires:
> Query `character.zeny`; do not invent cart currency. Test logout/reconnect, server restart, player trade, NPC buy, NPC sell, player vending purchase, and buying-store sale if the campaign exposes it. Record both participants where applicable, client display, database value, inventory delta, and one server log entry.  
> **Done when:** every supported path balances exactly once and unsupported “cart Zeny” language has been removed.

### 1.1 Refutation of the "Cart Zeny" Myth
Previous documentation hypothetically proposed "Cart Transactions" where players deposit or withdraw Zeny to/from a Pushcart (`sd->cart`). In Ragnarok Online and Hercules:
- Source code in [`Hercules/src/map/pc.h`](file:///Volumes/T7/GitHub/Ragnarok_Online/Hercules/src/map/pc.h) defines `struct s_cart` as holding `struct item items_cart[MAX_CART]` only.
- In [`Hercules/src/common/mmo.h`](file:///Volumes/T7/GitHub/Ragnarok_Online/Hercules/src/common/mmo.h), `struct mmo_charstatus` contains `int zeny` (character wallet) and `struct item cart[MAX_CART]`. There is no cart currency field anywhere in the Hercules codebase or official Gravity RO packet architecture.
- In [`Hercules/conf/map/logs.conf:70`](file:///Volumes/T7/GitHub/Ragnarok_Online/Hercules/conf/map/logs.conf#L70):
  ```
  // Please note that moving items from inventory to cart and back is not logged by design.
  ```
- **Verdict:** Pushcarts store inventory items only. All speculative language regarding "cart Zeny deposits/withdrawals" is false and permanently removed.

### 1.2 Authoritative Database Schema & Precision
1. **Character Table (`ragnarok.char`):**
   - Column: `` `zeny` int(10) unsigned NOT NULL DEFAULT 0 ``.
   - Limit: Capped in server logic by [`Hercules/src/common/mmo.h:230`](file:///Volumes/T7/GitHub/Ragnarok_Online/Hercules/src/common/mmo.h#L230): `#define MAX_ZENY INT_MAX` (2,147,483,647).
2. **Transaction Log Table (`ragnarok.zenylog`):**
   - Schema:
     - `id int(11) NOT NULL auto_increment PRIMARY KEY`
     - `time datetime NULL`
     - `char_id int(11) NOT NULL DEFAULT 0`
     - `src_id int(11) NOT NULL DEFAULT 0`
     - `type enum('T','V','P','M','S','N','D','C','A','E','I','B','K','4','5') NOT NULL DEFAULT 'S'`
     - `amount int(11) NOT NULL DEFAULT 0`
     - `map varchar(11) NOT NULL`
   - Logging configuration: In [`Hercules/conf/map/logs.conf`](file:///Volumes/T7/GitHub/Ragnarok_Online/Hercules/conf/map/logs.conf), `log_zeny: 0` disables logging. Overriding with `log_zeny: 1` in `Hercules/conf/import/logs.conf` enables transaction-level logging to `zenylog`.
3. **Transaction Type Codes:**
   - `'S'` — NPC Shop transaction (`LOG_TYPE_NPC`, buy or sell).
   - `'T'` — Player Trade transaction (`LOG_TYPE_TRADE`).
   - `'A'` — Admin / GM command (`LOG_TYPE_COMMAND`, e.g. `@zeny`).
   - `'N'` — Script transaction (`LOG_TYPE_SCRIPT`).

---

## 2. Client Initialization Bug Isolation & Resolution

### The Issue
During initial map connection, the server sends character stats (`pc_calcstatus`), but does **not** emit a redundant `SP_ZENY` (`ZC_LONGLONGPAR_CHANGE` / `0x09CB`) update because starting currency was already transmitted by the character server during character selection (`HC_CHAR_SELECT` / `0x0071` carrying `CharacterInformation.money`).

Prior to `QW-024`:
1. `Player::new` in [`korangar/src/world/entity/mod.rs`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar/src/world/entity/mod.rs) hardcoded `zeny: 0`.
2. `TestContext::new` in [`korangar-networking/examples/headless-tester/context.rs`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar-networking/examples/headless-tester/context.rs) initialized `context.zeny = 0` and did not populate it from `info.money`.

As a consequence, the client UI and headless test harness reported 0 Zeny on fresh login until an artificial probe or transaction occurred.

### The Fix
1. Populated `zeny: character_information.money.max(0) as u32` in `Player::new` ([`korangar/src/world/entity/mod.rs:1974`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar/src/world/entity/mod.rs#L1974)).
2. Populated `context.zeny = info.money.max(0) as u32` in `TestContext::new` ([`korangar-networking/examples/headless-tester/context.rs:386`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar-networking/examples/headless-tester/context.rs#L386)).
3. Both the graphical client and headless automation now immediately reflect accurate wallet balances upon character selection.

---

## 3. Live Transaction Verification Matrix

Automated verification was executed via `./korangar/tools/testing/run-suite.sh --scenario zeny-persistence-transaction-matrix`. All paths balanced exactly once with zero drift across client display, MySQL character database, and MySQL transaction audit logs.

### Summary Table

| Path | Participant(s) | Pre Zeny (Client / DB) | Action | Post Zeny (Client / DB) | Zeny Delta | Inv Delta | Server `zenylog` Entry |
|---|---|---|---|---|---|---|---|
| **1. Initial Alignment** | Primary (`test`, CID 150000) | `1,000,000` / `1,000,000` | Sync check | `1,000,000` / `1,000,000` | `0` | `0` | Baseline |
| **2. NPC Buy** | Primary (`test`, CID 150000) | `1,000,000` / `1,000,000` | Buy 1 Pet Food (ID 537) from Groomer | `999,000` / `999,000` | `-1,000` | `+1` | `char_id: 150000, src_id: 150000, type: 'S', amount: -1000, map: 'prontera'` |
| **3. NPC Sell** | Primary (`test`, CID 150000) | `999,000` / `999,000` | Sell 1 Pet Food (ID 537) to Groomer | `999,500` / `999,500` | `+500` | `-1` | `char_id: 150000, src_id: 150000, type: 'S', amount: 500, map: 'prontera'` |
| **4. Player Trade** | Primary (`test`, CID 150000)<br>Partner (`HeadlessTwo`, CID 150018) | Primary: `500,000` / `500,000`<br>Partner: `100,000` / `100,000` | Primary trades 50,000z to Partner | Primary: `450,000` / `450,000`<br>Partner: `150,000` / `150,000` | Primary: `-50,000`<br>Partner: `+50,000` | `0`<br>`0` | Primary: `char_id: 150000, src_id: 150018, type: 'T', amount: -50000`<br>Partner: `char_id: 150018, src_id: 150000, type: 'T', amount: 50000` |
| **5. Logout / Reconnect** | Primary (`test`, CID 150000) | `450,000` / `450,000` | Logout, check offline DB, reconnect | `450,000` / `450,000` | `0` | `0` | Preserved offline in MySQL `char.zeny`; restored at character select |
| **6. Server Restart** | Primary (`test`, CID 150000) | `462,345` / `462,345` | Set balance, stop server, start server, wait ready, reconnect | `462,345` / `462,345` | `0` | `0` | Preserved across full Hercules shutdown and reboot; restored at login |
| **7. Vending / Store Boundaries** | N/A | N/A | Protocol audit | N/A | N/A | N/A | Unexposed packet layer boundary verified (no player vending/buying-store in scope) |

---

## 4. Path Details & Observed Telemetry

### Section 1: Initial State & DB Alignment
- **Execution:** Character logged into map server. Baseline established at 1,000,000z.
- **Client Display:** `1,000,000`
- **Database Query:** `SELECT zeny FROM \`char\` WHERE char_id = 150000` -> `1000000`
- **Outcome:** Exact 1:1 match.

### Section 2: NPC Buy
- **Location:** Prontera `(218, 209)` adjacent to Pet Groomer (`prontera 218 211`).
- **Packet Flow:**
  - Client -> Server: `CZ_REQ_NAME` / `CZ_START_DIALOG` (`0x0090`) to Groomer entity.
  - Server -> Client: `ZC_ASK_BUY_OR_SELL` (`0x00c7`).
  - Client -> Server: `CZ_SELECT_BUY_OR_SELL` (`0x00c8`, option `Buy`).
  - Server -> Client: `ZC_OPEN_SHOP` (`0x00c9`) listing Pet Food (537) at price 1,000z.
  - Client -> Server: `CZ_PURCHASE_ITEMS` (`0x00ca`) with count 1.
  - Server -> Client: `ZC_BUYING_COMPLETED` (`0x00cb`, result `Success`).
  - Server -> Client: `ZC_LONGLONGPAR_CHANGE` (`0x09CB`, `SP_ZENY` = 999,000).
- **Recorded Database Value:** `999,000`
- **Recorded `zenylog` Row:**
  ```sql
  SELECT id, time, char_id, src_id, type, amount, map FROM `zenylog` WHERE char_id = 150000 ORDER BY id DESC LIMIT 1;
  -- Output:
  -- id: 8 | time: 2026-09-14 17:12:58 | char_id: 150000 | src_id: 150000 | type: 'S' | amount: -1000 | map: 'prontera'
  ```

### Section 3: NPC Sell
- **Location:** Prontera `(218, 209)`.
- **Packet Flow:**
  - Client -> Server: `CZ_START_DIALOG` (`0x0090`).
  - Server -> Client: `ZC_ASK_BUY_OR_SELL` (`0x00c7`).
  - Client -> Server: `CZ_SELECT_BUY_OR_SELL` (`0x00c8`, option `Sell`).
  - Server -> Client: `ZC_SELL_ITEM_LIST` (`0x00cc`) quoting 1 Pet Food at 500z.
  - Client -> Server: `CZ_SELL_ITEMS` (`0x00cd`) with inventory index and quantity 1.
  - Server -> Client: `ZC_SELLING_COMPLETED` (`0x00ce`, result `Success`).
  - Server -> Client: `ZC_LONGLONGPAR_CHANGE` (`0x09CB`, `SP_ZENY` = 999,500).
- **Recorded Database Value:** `999,500`
- **Recorded `zenylog` Row:**
  ```sql
  -- Output:
  -- id: 9 | time: 2026-09-14 17:13:00 | char_id: 150000 | src_id: 150000 | type: 'S' | amount: 500 | map: 'prontera'
  ```

### Section 4: Player Trade
- **Location:** `prt_fild08 (287, 338)` (Primary) and `(286, 338)` (Partner).
- **Starting Balances:**
  - Primary (`test`, CID 150000): 500,000z
  - Partner (`HeadlessTwo`, CID 150018): 100,000z
- **Packet Flow:**
  - Primary -> Server: `CZ_REQ_EXCHANGE_ITEM` (`0x00e4`).
  - Partner -> Server: `CZ_ACK_EXCHANGE_ITEM` (`0x00e6`, accept).
  - Primary -> Server: `CZ_ADD_EXCHANGE_ITEM` (`0x00e8`, `InventoryIndex(65534)`, amount 50,000).
  - Both -> Server: `CZ_CONCLUDE_EXCHANGE_ITEM` (`0x00eb`, Ok).
  - Both -> Server: `CZ_EXEC_EXCHANGE_ITEM` (`0x00ed`, Commit).
  - Server -> Both: `ZC_EXEC_EXCHANGE_ITEM` (`0x00ee`, success).
  - Server -> Primary: `ZC_LONGLONGPAR_CHANGE` (450,000).
  - Server -> Partner: `ZC_LONGLONGPAR_CHANGE` (150,000).
- **Ending Balances:**
  - Primary: Client `450,000`, Database `450,000` (exact -50,000z delta).
  - Partner: Client `150,000`, Database `150,000` (exact +50,000z delta).
- **Recorded `zenylog` Rows:**
  ```sql
  -- Primary:
  -- id: 13 | time: 2026-09-14 17:13:08 | char_id: 150000 | src_id: 150018 | type: 'T' | amount: -50000 | map: 'prt_fild08'
  -- Partner:
  -- id: 14 | time: 2026-09-14 17:13:08 | char_id: 150018 | src_id: 150000 | type: 'T' | amount: 50000 | map: 'prt_fild08'
  ```

### Section 5: Logout & Reconnect Persistence
- **Pre-Logout Zeny:** 450,000z.
- **Action:** Primary sent `CZ_REQUEST_QUIT` (`0x018a`), awaited `ZC_NOTIFY_QUIT` (`0x018b`), and disconnected map socket.
- **Offline Database State:** `SELECT zeny FROM \`char\` WHERE char_id = 150000` returned `450,000`.
- **Reconnect State:** Character connected through login and char servers. `CharacterInformation.money` initialized client `context.zeny` to `450,000`. Database query confirmed `450,000`.

### Section 6: Server Restart Persistence
- **Pre-Restart Setup:** Delta of +12,345 applied, resulting in 462,345z. Character explicitly saved and logged out.
- **Database Value Before Restart:** `462,345`.
- **Action:** Executed `./Hercules/dev.sh restart && ./Hercules/dev.sh wait`.
  - Servers stopped via `athena-start stop`.
  - Servers restarted via `athena-start start`.
  - Readiness verified: `map-server listening on port 5121 (5s)`.
- **Database Value During / Post Restart:** `SELECT zeny FROM \`char\` WHERE char_id = 150000` returned `462,345`.
- **Reconnect After Server Boot:** Client connected, entering map server. Client display `462,345`, MySQL database `462,345`.

### Section 7: Player Vending, Buying Store, and Cart Boundaries
- **Vending / Buying Store:** The current client protocol layer does not implement player vending packets (`CZ_REQ_OPENSTORE2`, `CZ_REQ_OPEN_BUYING_STORE`). In [`docs/PROJECT_PLAN.md`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/docs/PROJECT_PLAN.md), player vending is scheduled for Milestone 3 (E5.6).
- **Cart Currency:** Formally refuted and ruled out. RO carts hold item slots only (`items_cart`), possessing zero currency fields.

---

## 5. Verification Commands

The following commands verify the scenario, code formatting, and type-safety:
```bash
# Run integration scenario (PASS in 25.0s)
./korangar/tools/testing/run-suite.sh --scenario zeny-persistence-transaction-matrix

# Unit and integration suite tests
cargo test -p korangar-networking items
cargo test -p korangar-networking --example headless-tester

# Compiler and linter gates
cargo check -p korangar
cargo clippy -p korangar-networking --lib -- -Dwarnings
cargo clippy -p korangar -- -Dwarnings
cargo fmt --all -- --check
git diff --check

# Hercules build gate
make -j4  # in Hercules/
```
