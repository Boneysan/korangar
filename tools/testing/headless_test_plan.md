# Headless Client — Full Test Plan

Canonical test plan for automated protocol/gameplay testing via the headless client
(`korangar-networking/examples/headless-tester.rs`). The goal: exercise as much of the
client↔server protocol as possible without graphics, at high speed, and catch packet
mapping errors, skill/combat bugs, and server-script regressions early.

**Companion documents:**
- [headless_findings.md](headless_findings.md) — bug log + port-back tracking (fill this in whenever a scenario fails)
- [headless_mock_client_plan.md](headless_mock_client_plan.md) — original design doc (implementation status updated there)
- [testing_guide.md](testing_guide.md) — overall project testing reference
- [headless_remaining_test_design.md](headless_remaining_test_design.md) — implementation-ready design for the remaining lifecycle, social, DM, repair, and skill-menu coverage

---

## 1. Why this catches bugs for the main client

The headless tester links the **same crates the graphical client uses**:

| Layer | Crate/Location | Shared with main client? |
|---|---|---|
| Packet structs, headers, serialization | `ragnarok-packets` | ✅ identical code |
| Framing, handlers, `NetworkEvent` mapping | `korangar-networking` | ✅ identical code |
| Event → game state handling | `korangar/src/networking/`, `korangar/src/lib.rs` | ❌ headless has its own minimal loop |

Consequences:
- A packet layout/framing/event-mapping bug found headlessly is **automatically fixed for
  the main client** when fixed in the shared crates — no porting step, but it MUST get a
  regression unit test in `ragnarok-packets` (see testing_guide.md §3).
- A bug in how the *main client consumes* an event (UI/state layer) cannot be caught
  headlessly — but headless runs establish "the wire data is correct", which isolates
  such bugs to `korangar/src/` quickly.
- Server-side bugs (Hercules scripts, DB, config) show up as wrong/missing events;
  fixes land in `Hercules/` and need no client port.

Every finding gets classified into one of those layers in the findings log.

### Coverage baseline (2026-07-11)
- `NetworkingSystem` exposes **~65 client actions** (login flow, movement, combat, skills,
  items, storage, trade, party, friends, dialogue, shops, hotkeys, stats).
- `NetworkEvent` has **108 variants**; `version_20220406.rs` registers **131 handlers**
  producing events plus **42 no-ops**; unmodeled known-length packets are consumed by the
  length-fallback table (`lengths_20220406.rs`).
- Anything consumed by the fallback produces no event — Phase 10 measures how often that
  happens so we know what to model next.

---

## 2. Prerequisites (verified working 2026-07-11)

- MariaDB running (`brew services start mariadb`), Hercules built with
  `--enable-packetver=20220406`.
- Test account `korangar`/`korangar` exists with **`group_id = 99`** in the `login` table
  (verified — all `@`/`@dm*` commands available; the `@dm*` suite binds at GM level 1+).
- Server start **from a script** must redirect stdio or the calling shell blocks on the
  inherited pipes and a timeout kills the servers:
  ```bash
  ./athena-start start > log/athena-start.out 2>&1 < /dev/null
  ```
- Smoke test (Phase 1 core path) passes:
  `cargo run --example headless-tester -p korangar-networking` → exit 0.

---

## 3. Test phases

Phases are ordered so each builds on machinery from the previous one. Each scenario lists
the **actions** (NetworkingSystem calls / GM commands) and the **assertions**
(`NetworkEvent`s that must arrive, with field checks). Any `expect`/timeout failure exits
nonzero and produces a findings entry.

### Phase 1 — Session lifecycle ✅ (smoke test implemented)

| Scenario | Actions | Assert |
|---|---|---|
| Happy path | login → char list → select → map connect → `map_loaded` → chat marker | `LoginServerConnected`, `CharacterList`, `CharacterSelected`, `UpdateClientTick`, `ChatMessage` echo — **implemented** |
| Bad password | login with wrong password | `LoginServerConnectionFailed`, exit 1 — **implemented** |
| Missing character | `--character NoSuchChar` | clean failure, exit 1 — **implemented** |
| Character create/delete | `create_character` in empty slot, then `delete_character` | `CharacterCreated` (name/slot match), `CharacterDeleted`; duplicate name → `CharacterCreationFailed` |
| Slot switch | `switch_character_slot(a, b)` | `CharacterSlotSwitched`, char list reflects swap |
| Logout/relogin | `log_out`, full reconnect | `LoggedOut`, second session succeeds |
| Respawn | `@die` via chat, then `respawn` | `UpdateEntityHealth` (0 hp), `ResurrectPlayer`/`ChangeMap` to save point |

### Phase 2 — GM command channel (bootstrap for everything below)

Build `send_gm_command(cmd) -> await feedback` once, reuse everywhere. Hercules replies to
atcommands via `ServerMessagePacket`/`DisplayBottomMessagePacket` → both surface as
`ChatMessage` with `MessageColor::Server`. **Failure mode to detect:** if the account
lacks GM rights the command echoes back as ordinary chat (`OverheadMessagePacket`) —
treat that as a hard test-setup error.

| Scenario | Actions | Assert |
|---|---|---|
| Job change | `@job 4010` (High Wizard — NOT 4001, that's Novice High) | `ChangeJob { job_id: 4010 }` |
| Leveling | `@blevel 98` (to 99), `@jlevel 69` (to 70) | `UpdateStat` base/job level, `GainedExperience` |
| Grant skills | `@allskill` | `SkillTree` event, tree non-empty for current job |
| Stat set | `@str +N` / `@allstats` | `UpdateStat` per stat |
| Item grant | `@item 501 5` (Red Potion) | `IventoryItemAdded` [sic — enum typo] / `ItemObtained`, count 5 |
| Zeny | `@zeny 100000` | `UpdateStat` zeny |
| Warp | `@warp prontera 155 180` | `ChangeMap { map_name: "prontera" }`, position matches |
| Monster spawn | `@monster PORING 1` | `AddEntity` with monster job id 1002 |
| Cleanup | `@killmonster` | `RemoveEntity` for spawned ids |

### Phase 3 — Movement & world state

| Scenario | Actions | Assert |
|---|---|---|
| Basic walk | `player_move(WorldPosition)` ~10 tiles away | `PlayerMove { origin, destination }`, destination == request |
| Walk into wall | request unwalkable target | server clamps path or no move — record actual behavior (documents pathing contract) |
| Cross-map warp | `warp_to_map("geffen", pos)` (GM char) | `ChangeMap`, then fresh `AddEntity` stream for new map |
| Entity awareness | `@monster` 3 spawns, walk away/back | `AddEntity`/`RemoveEntity` as view range changes |
| Entity details | `entity_details(id)` on spawned monster | `UpdateEntityDetails { name }` matches mob db |
| Sit/stand | `player_sit` / `player_stand` | `PlayerSitDown` / `PlayerStandUp` with own entity id |
| Tick sync | `request_client_tick` | `UpdateClientTick` monotonic |

### Phase 4 — Combat (melee)

Bootstrap: `@job 4008` (Lord Knight), `@blevel 98`, `@allskill`, `@monster PORING 1`.

| Scenario | Actions | Assert |
|---|---|---|
| Basic attack | `player_attack(target)` | `DamageEffect { source, target, amount > 0 }` |
| Kill & rewards | attack until dead | `RemoveEntity { reason: Died }`, `GainedExperience` |
| Health tracking | `@monster` something tanky, attack | `UpdateEntityHealth` decreasing monotonically |
| Attack failure | attack out-of-range/dead entity | `AttackFailed` |
| Incoming damage | `@monster` aggressive mob, stand still | `DamageEffect` with player as target; `UpdateStat` hp |

### Phase 5 — Skill sweep (the packet-mapping goldmine)

This is the highest-value phase: iterate jobs × skills and verify every cast produces
correctly-mapped packets. Skill metadata ground truth is
`Hercules/db/re/skill_db.conf` (`SkillType`, `MaxLevel`, `Range`, `AttackType`) — parse it
(or generate a Rust table the way `tools/generate_packet_lengths.sh` generates lengths)
so the sweep is data-driven rather than hand-listed.

**Per-skill procedure** (after `@job <id>`, `@allskill`, `@warp` to an open field,
`@monster PORING 1` for enemy-target skills):

| SkillType | Cast via | Assert |
|---|---|---|
| Enemy | `cast_skill(id, max_level, target)` | `SkillCast` (cast bar) if cast time > 0, then `DamageEffect`/`UseSkillAck`-derived events; `SkillCooldown` if defined |
| Place (e.g. Storm Gust = **89**, not 0x00B8) | `cast_ground_skill(id, level, TilePosition)` | `SkillCast`, `AddSkillUnit` for each ground cell, later `RemoveSkillUnit` |
| Self / Friend (buffs, heals) | `cast_skill` on self entity id | `StatusChange` (buff icon) and/or `HealEffect`; buff expiry → `StatusChange` clear |
| Channeling (e.g. arrow barrage types) | `cast_channeling_skill` + `stop_channeling_skill` | start/stop events, no desync after stop |
| Passive | skip (assert present in `SkillTree`) | — |

**Job coverage order** (each exercises different packet families):
1. High Wizard 4010 — ground AoEs, cast times, elements (fire/ice/lightning)
2. High Priest 4009 — buffs, heals, resurrection, status changes on others
3. Lord Knight 4008 — melee skills, self-buffs, HP-cost skills
4. Sniper 4012 — ranged, traps (`AddSkillUnit` trap units), falcon
5. Assassin Cross 4013 — dual wield, poison statuses, hiding (entity visibility packets)
6. Whitesmith 4011 / Creator 4015 — item-consuming skills, homunculus-adjacent packets
7. First/second classes spot-checks (skill tree shape at low job levels)

**What failures look like:** cast succeeds server-side (visible in server log / mob HP
drops) but no client event → unregistered or fallback-consumed packet; event arrives with
garbage fields → layout mismatch in `ragnarok-packets`; client desyncs after specific
skill → framing/length error. All three are exactly the bugs that hit the main client, and
all three get fixed in shared crates.

**Expected-failure ledger:** some skills legitimately fail headlessly (require ammo,
specific weapon, catalyst items — bootstrap with `@item` where feasible, otherwise record
in an allowlist with the reason). `ToUseSkillSuccessPacket`/failure feedback should map to
an observable event; if a skill fails *silently*, that's itself a finding.

### Phase 6 — Items, inventory, economy

Bootstrap: `@item` for consumables/equipment, `@zeny`.

| Scenario | Actions | Assert |
|---|---|---|
| Inventory load | fresh map login | `SetInventory` matches DB contents |
| Use consumable | `use_item` Red Potion | `HealEffect`, `InventoryItemRemoved` (count −1) |
| Equip/unequip | `@item 1101` (Sword), `request_item_equip` | `UpdateEquippedPosition`; wrong-job equip → failure feedback |
| Drop & pickup | `drop_item`, walk off/back, `pick_up_item` | `InventoryItemRemoved` + `AddGroundItem`, then `EntityPickUpItem` + `ItemObtained` |
| Identify | `@item 617` (Old Violet Box → unidentified gear) + Magnifier | `ItemIdentifyList`, `ItemIdentified` |
| Weight limit | `@item 501 <huge count>` | `CriticalWeightPercent` |
| Storage | `@storage`, `move_item_to_storage` / back / `close_storage` | `SetStorage`, `StorageItemAdded`/`Removed`, `StorageAmount`, `StorageClosed` |
| NPC shop buy | find shop NPC (via `AddEntity` in prontera), `select_buy_or_sell`, `purchase_items` | `OpenShop` (list non-empty), `BuyingCompleted`, zeny decreases |
| NPC shop sell | `sell_items` | `SellItemList`, `SellingCompleted` |
| Stat/skill points | `request_stat_up`, `level_up_skill` | `UpdateStat`, `UpdateSkill` |
| Hotkeys | `set_hotkey_data` round trip (relogin) | `SetHotkeyData` persisted |

### Phase 7 — NPC dialogue

Target: a stable vanilla NPC (e.g. any Kafra — located via `AddEntity` after warping to
prontera) plus one campaign NPC from `dm_campaign` for script coverage.

| Scenario | Actions | Assert |
|---|---|---|
| Linear dialogue | `start_dialog` → `next_dialog`* → `close_dialog` | `OpenDialog` (text non-empty), `AddNextButton`, `AddCloseButton` |
| Choice menu | `choose_dialog_option(n)` | `AddChoiceButtons` (options parse correctly — this validates the `:`-separated menu packet), branch text differs per choice |
| Number input | NPC with number prompt | `NpcRequestNumberInput`, `submit_dialog_number` accepted |
| String input | NPC with string prompt | `NpcRequestStringInput`, `submit_dialog_string` accepted |
| Dialogue warp | Kafra teleport service | `ChangeMap` |

### Phase 8 — Multi-client: party, trade, social

Spawn **two** `NetworkingSystem`s in one process (accounts `korangar` + a second test
account; create it via the `_M` suffix trick or SQL — document in prerequisites when done).
This is the only way to test the peer-to-peer packet families:

| Scenario | Actions | Assert |
|---|---|---|
| Party lifecycle | A: `create_party`, `invite_to_party(B)`; B: `accept_party_invite` | A: `CreatePartyResult`, `PartyMemberAdded`; B: `PartyInvite`, `PartyList` |
| Party telemetry | B walks / takes damage | A sees `PartyMemberPosition`, `PartyMemberHealth` |
| Party chat | `send_party_chat_message` | other side gets `PartyChatMessage` |
| Leave/kick | `leave_party` | `PartyMemberRemoved` both sides |
| Invite block | B: `set_party_invitation_block(true)`, A invites | A gets `PartyInviteResult` rejection |
| Whisper | `send_whisper_message` A→B | B: `WhisperReceived`; to offline name → A: `WhisperResult` failure |
| Friends | `add_friend`, accept, `remove_friend`, relogin | `FriendRequest`, `FriendAdded`, `SetFriendList`, `FriendOnlineStatus`, `FriendRemoved` |
| Trade full loop | request → accept → add item + zeny → ok → commit | `TradeRequest`, `TradeStart`, `TradePartnerItem`, `TradeAddItemResult`, `TradeLocked`, `TradeCompleted`; inventory deltas on both sides |
| Trade cancel | cancel mid-trade | `TradeCancelled`, no item loss |
| Emotion | `request_emotion` | other client: `DisplayEmotion` |

### Phase 9 — DM campaign command suite

The fork's raison d'être. Commands bound in `Hercules/npc/custom/dm_campaign/shared/`
(`bindatcmd`, GM level 1+): `@dm`, `@dmreward`, `@dmflag`, `@dmquest`, `@dmbeat`,
`@dmguide`, `@dmstory`, `@dmcleanup`, `@dmwarp`, `@dmrecall`, `@dminstance`, `@dmmode`,
`@dmhazard`, `@dmexp`, `@dmreset`, `@dmstatus`, `@roll`.

This automates the manual DM-mode testing that paused 2026-07-11 (see memory notes /
dm-mode-testing-status):

| Scenario | Actions | Assert |
|---|---|---|
| Console feedback | each `@dm*` with no/bad args | usage text via `ChatMessage` (Server color) — proves `DisplayBottomMessagePacket` path per command |
| `@roll` | `@roll 2d6+3` etc. | result message, value within bounds |
| `@dmwarp`/`@dmrecall` | warp self/party | `ChangeMap`; with Phase 8 second client: both members move |
| `@dmbeat` | trigger campaign beats | `ChangeMap` to beat destination (cross-check with map-asset-audit walkability list) |
| `@dmflag`/`@dmquest` | set flag, query status | `@dmstatus` output reflects change; `AddQuestEffect`/`RemoveQuestEffect` where beats attach quest markers |
| `@dmreward` | grant reward table entry | `ItemObtained`/`IventoryItemAdded`, matches `dm_rewards.txt` |
| `@dmhazard` | arm hazard, stand in it | periodic `DamageEffect`/`StatusChange` matching configured interval/damage |
| `@dminstance` | create/enter/cleanup instance | `ChangeMap` to instanced map name, `@dmcleanup` removes it |
| `@dmexp` | exp grant | `GainedExperience` with expected amount |

### Phase 10 — Protocol coverage & robustness (continuous)

Run alongside every phase, not a separate pass:

- **Unknown-packet ledger**: `NetworkingSystem::spawn_with_callback` gives a hook on every
  packet; additionally `KORANGAR_PACKET_LOG=1` dumps hex. Record every packet consumed by
  the length-fallback during a full run → the list of "server sends it, client ignores it"
  headers, ranked by frequency. That is the packet-modeling backlog.
- **Desync detection**: any framing error / connection drop mid-scenario is a P0 finding
  (main client would hard-desync too). The scenario name + preceding packet hex go in the
  findings log.
- **Timing**: flag events that arrive > N s after the triggering action (server lag vs
  lost packet disambiguation).
- **Soak**: loop Phases 2–6 for 30+ min on one session — catches slow desyncs, tick drift,
  and buffer leaks.

---

## 4. Harness & runner design

Extend `headless-tester.rs` incrementally (keep the current smoke test as scenario
`smoke`, the default):

```
cargo run --example headless-tester -p korangar-networking -- --scenario <name> [--timeout 60]
```

- One `async` helper per phase; scenarios are **plain Rust functions** (type-checked
  against the packet structs) — no YAML/JSON runner unless scenario count outgrows this.
- Core utility: `await_event!(matcher, timeout, stage_name)` — poll loop with deadline
  that reports the stage and the last 20 events on failure (context for the findings log).
- `--scenario all` runs everything and prints a per-scenario PASS/FAIL table; exit code =
  number of failures.
- Wrapper script `tools/run-integration-tests.sh`: start MariaDB check → start Hercules
  (with the stdio redirect from §2) → poll ports 6900/6121/5121 → run `--scenario all` →
  `athena-start stop` → propagate exit code. CI-ready.

---

## 5. Bug documentation & port-back workflow

**Every scenario failure or anomaly gets an entry in [headless_findings.md](headless_findings.md) before it gets fixed.** The template there enforces the fields
needed to route the fix:

1. **Classify the layer** (shared crate / main-client-only / server) — this determines
   whether a port-back step exists at all.
2. Shared-crate fixes (`ragnarok-packets`, `korangar-networking`): fix + **mandatory unit
   test** per testing_guide.md §3 → main client gets it for free. Mark "port-back: N/A
   (shared code)".
3. Server fixes (scripts/config/DB): fix in `Hercules/`, re-run `check-campaign.sh`,
   note whether the main client also needs a behavior change.
4. Main-client-only implications (headless proved the wire is fine, so the bug is in
   `korangar/src/` event handling): the finding documents the exact events + payloads so
   the client-side fix can be written and manually verified per testing_guide.md §6
   checklist.
5. A finding is **closed** only when: fix landed + regression test exists (or manual
   checklist item added) + headless scenario re-run green.

---

## 6. Status & priorities

| Phase | Status | Priority |
|---|---|---|
| 1 Session lifecycle | ✅ core implemented (smoke) — create/delete/slot-switch pending | follow-up |
| 2 GM channel | ✅ implemented and green | done |
| 3 Movement/world | ✅ implemented and green | done |
| 4 Combat melee | ✅ implemented and green | done |
| 5 Skill sweep | ✅ 39 job sweeps implemented and green | done |
| 6 Items/economy | ✅ implemented and green | done |
| 7 NPC dialogue | ✅ implemented and green | done |
| 8 Multi-client social | not started (needs 2nd account) | P2 |
| 9 DM campaign suite | not started | **P1 — fork-critical, resumes paused DM testing** |
| 10 Coverage ledger | ✅ implemented | done |

Suggested build order: **2 → 10 (callback) → 3 → 4 → 5 → 9 → 6 → 7 → 8**.
