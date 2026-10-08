# Implementation plan — playtest 2026-10-06

Handoff plan for the findings in [playtest-2026-10-06.md](playtest-2026-10-06.md). Written
2026-10-07. The named slices below are implemented. None are live-verified. P7 still
waits on a live `KORANGAR_MOVE_TRACE=1` capture. Do not start a new slice unless the
owner names one.

## Before you start

- korangar `85534f49` on `agent/playtest-fixes-2026-10-05`. Hercules `73aab7bdb` on
  `agent/flinch-stop-and-sit-regen`. Both were pushed 2026-10-07. A later edit to these
  plan docs is local until the owner asks for another commit.
- `KORANGAR_MOVE_TRACE` (`trace_move_correction` and its three call sites) is inside
  `85534f49`. Leave it alone. It does not name a rubber-banding cause.
- Validation order (workspace `CLAUDE.md`): `cargo fmt --all -- --check` → narrowest test
  filter → `cargo check -p korangar` → `cargo test -p korangar --lib` → live client check.
- Status words: *implemented and compiles* / *automated tests pass* / *live-verified*. Every
  slice below ends at live-verified. Without that, it is not done.
- Hercules: rebuild with `Hercules/dev.sh build`, restart with `./dev.sh restart && ./dev.sh wait`.
  Script-only changes need `@reloadscript` or a restart. Item 50002 and the floor-1
  `iz_ac01` map caches load only when the map-server process starts, so those need a restart.

## Wave 1 — confirmed causes, small fixes

### P0. Escape closes windows, then opens the menu — *implemented, automated tests pass*

- **Owner decision 2026-10-07:** Escape closes the top window. Once nothing closable is
  left, it opens the menu. This reverses the menu-only rule in `04aba3bd`. That commit also
  fixed the double-event bug that made the old version close two windows per press.
- **Done:** `Interface::close_top_window_except` plus the pure helper
  `top_closable_index_except` in `korangar-interface/src/lib.rs` (4 tests in `tests::`).
  `ESCAPE_KEEPS_OPEN` (HUD, Minimap, MonsterTarget, BossTarget, PartyHealer) and the
  `InputEvent::Escape` arm are in `korangar/src/lib.rs`.
- **Passed:** fmt check, `cargo test -p korangar-interface --lib tests::` (5/5),
  `cargo check -p korangar`, `cargo test -p korangar --lib escape` (includes
  `one_escape_press_is_one_escape_event`).
- **Remaining, live:** open Inventory + Skills + Emotes, then press Escape repeatedly.
  Each press closes one window. The HUD, minimap, and target frame stay. The next press
  opens the menu, and the one after closes it. Escape while typing in chat only leaves
  the text box. Escape during a cast still cancels the cast first.

### P1. Potion count does not decrement — *implemented, automated tests pass*

- **Evidence (source-confirmed):** Hercules `pc_useitem` deletes the item with
  `pc->delitem(..., 1 /* no packet */ ...)` and reports the remaining count in
  `ZC_USE_ITEM_ACK` 0x01C8 (`clif->useitemack(sd, n, amount-1, true)`, `pc.c:5598`,
  `pc.c:5628`). The handler at `korangar-networking/src/packet_versions/version_20220406.rs:1003`
  acts only on `result != 0 && amount == 0`.
- **Second bug in the same handler (source-confirmed):** a successful ack is broadcast to
  the whole area (`clif_useitemack`: `clif->send(..., AREA)`; only failures go to the user
  alone). The handler ignores `entity_id`, so when a nearby player uses their *last* stack
  of anything, your client removes one item from *your* inventory slot with that index.
  Expect this to show up as "items vanish in a group".
- **Change:** the networking crate does not know the local account id, so carry
  `entity_id` and `amount` in the event and let the client filter: act only when
  `entity_id` is the local player. On success, set the slot's amount to `packet.amount`
  and remove the slot at 0. Look for an existing set-amount path before adding one. On
  failure (`result == 0`), change nothing.
- **Test:** start the slot at 5 and feed a successful ack with `amount: 2`. Expect 2. That
  fails both the current "do nothing" and a naive "remove one" (4). Second test: an ack
  with another player's `entity_id` and `amount: 0` leaves your slot unchanged.
- **Live:** use 3 of 5 Novice Potions. The count reads 2. Then use the last one.
  A nearby player's last item must not change your bag.
- **Done 2026-10-07:** `NetworkEvent::UseItemAck` carries `entity_id`, the remaining
  `amount`, and `success`. `apply_use_item_ack` sets the local player's slot to that
  count and removes it at 0. Another player's ack and a failed use leave the bag alone.
  The headless `use_consumable` scenario now waits for this ack instead of a delete packet.
- **Passed:** `cargo fmt --all -- --check`, `cargo test -p korangar-networking --lib use_item_ack`
  (6/6), `cargo check -p korangar`, `cargo check -p korangar-networking --example headless-tester`,
  `cargo test -p korangar --lib` (755 passed, 18 ignored).
- **Not live.** The headless `use_consumable` scenario was not run against a server.

### P2. Clicks on the UI make the character walk — *implemented, automated tests pass*

Reported as: closing a window with X, clicking an item to equip it, adding to the hotbar,
and possibly emotes "attacking".

- **Evidence (hypothesis, strong):** the hold-to-move branch in `korangar/src/lib.rs`
  (search `should_reissue_hold_mouse_move`) issues `PlayerMove` on any frame where the left
  button is down, the interface is not hovered, and the cursor is on a tile. It never asks
  where the press began. If a click closes or relayouts a window while the button is
  still down, the next frame sees bare ground.
- **Alternative cause to rule out:** mouse mode read from the previous frame (memory
  `interface-frame-mouse-mode-staleness`): `is_interface_hovered` is this frame's layout,
  but some gates read last frame's applied mode. A double-click equip, where the
  inventory never closes, is not explained by the main hypothesis alone.
- **Confirm first:** run with `KORANGAR_PACKET_LOG` or a temporary `client_log!` and close a
  window with X. Expect a `PlayerMove` in the frame after the close.
- **Change:** record whether the current left press started over the interface (on
  `MouseButton::Left` press). Hold-to-move only continues a press that started on the
  world. Clear the flag on release.
- **Test:** extract the decision into a pure function, as was done for
  `should_reissue_hold_mouse_move`. A press that starts over the UI and is then held over a
  tile yields no move. A press that starts on a tile and is held yields moves.
- **Live:** close Inventory with X, double-click a weapon to equip it, and drag a potion
  to the hotbar. The character must not move. Click-and-hold on the ground still walks.
  Re-test the emote window: if "emote makes the character attack" persists, open a
  separate slice for it.
- **Done 2026-10-07:** a left press records whether it began on the world
  (`left_press_started_on_world`). `hold_walk_allowed` lets the hold path continue
  only that press. Releasing the button clears the flag. The press frame itself is
  unchanged: a click the interface does not own can still walk or attack.
- **Passed:** `cargo fmt --all -- --check`, `cargo test -p korangar --lib hold_walk` (1/1),
  `cargo test -p korangar --lib` (756 passed, 18 ignored).
- **Not live.**

### P3. Own headgear never renders — *implemented, automated tests pass*

- **Evidence (source-confirmed):** `Player::get_entity_part_files`
  (`korangar/src/world/entity/mod.rs`) composed body, head, weapon, and shield and
  never called `push_headgear_part_files`. `Common::get_entity_part_files`, used for
  remote players, does call it. The headgear ids are present: `EntityData::from_character`
  copies `accessory`, `accessory2`, and `accessory3`. `refresh_entity_headgear_layers` for
  self (`ChangeLook`) goes through the same method, so equipping a hat mid-session failed
  too. Character-select cards already called `push_headgear_part_files_for_views`.
- **Done 2026-10-07:** `Player::get_entity_part_files` delegates to
  `self.common.get_entity_part_files(...)`. The stale "Stored but not yet drawn" note on
  `Common::accessory` now says headgear is drawn and that palettes, the robe, and the
  alternate body style are still stored only.
- **Passed:** `cargo fmt --all -- --check`, `cargo check -p korangar`,
  `cargo test -p korangar --lib the_local_player_lists_the_same_hat` (1/1),
  `cargo test -p korangar --lib` (757 passed, 18 ignored). The test builds a player and a
  `Common` from the same hatted entity with no GRF: one accessory name and one sprite
  path. The two part lists match, and the hat path is last (weapon and shield empty).
- **Not live.** Log in wearing a hat (all 8 facings and sitting). Unequip and re-equip it.

### P4. Number-key attack skills ignore the Tab target — *implemented, automated tests pass*

- **Evidence (source-confirmed):** the `SkillType::Attack` hotbar branch resolved the
  target only from `input_report.mouse_target` through `resolve_pending_cast`. The Tab
  target (`self.targeted_monster`, set by `CycleMonsterTarget`) was never consulted, so
  the skill armed and waited for a click. Space (`AttackTarget`) already attacks that
  monster and was not this bug.
- **Done 2026-10-07:** `attack_skill_key_target` picks the hovered entity, then a Tab
  target that `is_targetable_monster` still accepts (alive, not fading, not hidden),
  then arms. The cast still goes through `cast_or_path_entity_skill`. A click while the
  skill is already armed still uses `resolve_pending_cast` and does not fire at the Tab
  target by itself.
- **Passed:** `cargo fmt --all -- --check`, `cargo test -p korangar --lib an_attack_skill_key`
  (1/1), `cargo check -p korangar`, `cargo test -p korangar --lib` (758 passed, 18 ignored).
- **Not live.** Tab to a Poring, press the Firebolt key with the cursor over empty ground,
  and it casts. Out of range, it walks into range. A dead or faded Tab target still arms
  and waits.

### P5. Navigation line remains after you arrive — *implemented, automated tests pass*

- **Evidence (source-confirmed):** `refresh_navigation_marker` rebuilt breadcrumbs and
  nothing cleared `minimap.navigation_target` when the player reached the cell. The
  route is not refreshed on each step, so the check also runs after `update_entities`.
- **Before coding (done in P6b):** `state/breadcrumb.rs` on closed PR #10 is a quest HUD.
  That tree has no `navigation_target`, so it did not clear the minimap line on arrival.
- **Done 2026-10-07:** `navigation_has_arrived` is true on the same map within 3 tiles
  (Chebyshev) of a fixed cell. `clear_navigation_on_arrival` then clears the target,
  marker, and breadcrumbs, and toasts "Arrived". A different map, a map-level target
  with no cell, and a party-member route (`follows_party_member`) stay up. Setting a
  destination you are already standing on does not also toast "no walkable route".
- **Passed:** `cargo fmt --all -- --check`, `cargo test -p korangar --lib navigation_arrives`
  (1/1), `cargo check -p korangar`, `cargo test -p korangar --lib` (759 passed, 18 ignored).
- **Not live.** Navigate to an Izlude NPC. The line disappears within 3 tiles and the
  toast says "Arrived". A route to another map stays up until you are on that map and
  close to the cell. A party-member route stays up.

## Wave 2 — reproduce or instrument before changing code

### P6. Save button is missing — *implemented and compiles*

- **Cause (source-confirmed):** "Save here" / "Warp to save" (`@save` / `@load`) were added in
  `8f8cbeee`, on `agent/bump-hercules-pin` (PR #10). That PR was closed unmerged on 2026-09-28,
  so the buttons never reached `main` or this branch. The server side was never the problem:
  group 0 has `save`/`load` in `Hercules/conf/groups.conf`, and `ACMD(save)` is unconditional.
- **Done 2026-10-07:** the two buttons are back in the Menu's Adventure section
  (`interface/windows/menu.rs`), sending the same `InputEvent::SendMessage` text as before.
  Fmt and `cargo check -p korangar` pass. No unit test: it is pure wiring, and the live check
  below is the evidence.
- **Live:** Menu → Save here prints "Your save point has been changed." Walk away, then
  Warp to save returns you to that cell. Dying respawns you there.
- The old branch also had a Save/respec row in the Commands window. Leave it out unless
  the owner wants it.

### P6b. Audit what closed PR #10 had that this branch lacks — *audit handed over, nothing ported*

- `git log HEAD..origin/agent/bump-hercules-pin` is 36 commits from merge-base `e7321150`.
  `main` is an ancestor of this branch (16 newer UI commits). Equivalents below are on
  `main` unless the row says otherwise. Checked by behaviour, not by filename.
- PR #10 still has the old potion handler (P1) and the missing self-headgear call (P3).
  Neither fix is on that branch.
- **Port nothing from this table without a yes.** One feature per later slice, with tests.

| Feature | PR files | On `main` / this branch | Recommendation |
|---|---|---|---|
| Menu "Save here" / "Warp to save" | `interface/windows/menu.rs` | Restored in uncommitted P6 | already replaced |
| Commands-window Save / `@resetskill` | `interface/windows/commands.rs` | Absent. P6 already says leave it out | drop |
| WASD tap and hold | `input/wasd.rs` | Same path; this copy is 157 lines newer | already replaced |
| Graphics API remembered across launches | `tools/packaging/windows/Play.ps1` (`client/graphics-api.txt`) | Same behaviour | already replaced |
| Outdated-client window | pack version on login | `korangar-networking` "out of date" text and the login popup | already replaced |
| Class name on the character slot | character select | `CharacterSlots::set_class_name` | already replaced |
| Space attacks the Tab target | `AttackTarget` | Default Space binding and `attack_targeted_monster`. This is a basic attack, not P4 | already replaced |
| Party share buttons match the server | `PartyShareOptions` | Handler and `party.share_experience` | already replaced |
| Refuse party invites | party window | "Refuse all party invites" | already replaced |
| Unusable gear drawn muted | `equip_presentation.rs`, `equipment_eligibility.rs` | Identical file blobs | already replaced |
| Campaign `[DMJ]` journal | `dm/parser.rs` | `state/dm_journal.rs` (keeps every `[DMJ]` line out of chat) | already replaced |
| Hunt text in the journal | `hunt_schema.rs`, `journal_slice.rs` | Quest log takes hunt rows from the server packets | already replaced |
| Warp routes | `world/library/warp_graph.rs` | `world/navigation.rs` (`NavigationGraph`) | already replaced |
| Hidden-chest list | `world/library/chest.rs` | `hidden_chests.rs` and Adventure Guide routes | already replaced |
| Click sound does not play on a drag | `ui_sounds.rs` plus the stale mouse-mode fix | `click_sound_tests` covers a window drag | already replaced |
| Sitting HP and SP numbers | `heal_number_appearance` | `DisplayPlayerHealEffect` still has `heal_type`, and the handler drops it. `HealNumber` is always green at one position, so HP and SP sit on top of each other | port |
| Quest HUD chip (track, directions, primary) | `state/breadcrumb.rs`, `tracked_objective.rs` | Quest log and the minimap route exist. The always-on HUD chip does not. It did not clear `navigation_target` (that field is absent on the PR) | port, as Wave 3 item 4 |
| Walk-over `@autopickup` settings row | Game Settings query | No client control. Headless `autopickup-radius` and `autopickup-party-override` cover the server. `@autoloot` in Game Settings is a different command | drop |
| Chest unopened / available / opened art | `chest_discovery.rs` | The Guide says opened state is not sent to the client | drop |
| Screen-space click picking | `input/target.rs` | The GPU picker chooses the entity | drop |
| Activation / rejection UI sound gate | `ui_sounds.rs` | In-game windows stay silent; only the login screens click | drop |
| Drop or trade an exact count | `quantity.rs`, `QuantityWindow` | Trade 1 or the whole stack, drop the whole stack, split 1 / half / a typed amount | drop |
| Area loot (vacuum within 4 cells) | `area_loot.rs` | Absent | drop |
| Structured combat log | `combat_chat.rs` | Floating combat text | drop |
| Stable per-member party colors | `party_colors.rs` | Absent. Party blips are a separate open bug (B4) | drop |
| September player guide | `docs/PLAYER_GUIDE.md` | Absent, and written against that branch | drop |

Docs-only commits on that branch (patch reports, runbooks, session notes) are not features.

### P7. Rubber-banding — *investigated 2026-10-07, no cause named, no code change*

- The trace is already in `korangar/src/lib.rs` (`trace_move_correction`) and was left
  untouched. It logs only when `KORANGAR_MOVE_TRACE` is set and the Chebyshev gap is more
  than 1 tile. Call sites: `EntityMove` (`move`), `PlayerMove` (`own-move`),
  `EntityStopMove` (`stop`).
- **Source-confirmed:** this Hercules checkout is `agent/flinch-stop-and-sit-regen` at
  `2439c440e` (2026-10-05 18:00), clean. `unit_set_walkdelay` adds
  `STOPWALKING_FLAG_FIXPOS` for `BL_PC`, and `clif_fixpos` sends `ZC_STOPMOVE` 0x0088 to
  `AREA`, which includes the player. The client applies `EntityStopMove` to the entity
  list, and the local player is in that list, so a stop for yourself is applied. The
  reading that the client drops its own stop is false.
- **Not the playtest server.** The local `map-server` binary is dated 2026-10-05 18:51.
  Nothing is running. The newest log is `Hercules/log/server-20261005-185119.log`
  (shutdown 19:24). There is no 2026-10-06 log on this machine, and the playtest notes
  name only client pack `20261005`. Which server binary the friends session used is still
  unknown. No `[move-trace]` line is in the repo.
- `CLAUDE.md` §3b's "no correction over 1 tile in 220" is an earlier session, not this
  playtest. WASD sends a move and waits for the server path; it does not move the sprite
  first. A corner-cut can still make Hercules walk a different route (`input/wasd.rs`);
  that guard is not a measured snap from 2026-10-06.
- No movement code changed. A fix still waits on a `KORANGAR_MOVE_TRACE=1` run against
  the flinch-stop server: WASD, click-walk, under attack, and the Academy.
  B8 is a separate, source-confirmed floor-1 cache mismatch (see B8 below). Client-side
  flinch prediction stays rejected.

### P8. Novice Academy warps (iz_ac01 / iz_ac02) and "wonky movement" — *implemented, not live*

- **Evidence (source-confirmed):** stock `Hercules/npc/re/warps/cities/izlude.txt`. Both
  iz_ac01 stairs landed on iz_ac02 `124,46`. That cell is inside `#to_ac1f01` at `126,48`,
  whose span of 2 covers x 124–128, y 46–50 (`npc.c`). Arrival runs `npc_touch_areanpc`
  (`clif.c`), so going upstairs warped straight back to iz_ac01 `78,28`.
- **Done 2026-10-07:** left stair (`78,25`, and the `_a`–`_d` copies) now lands on
  `126,45`. Right stair (`122,25`, and the copies) now lands on `131,45`. Both cells are
  walkable on `maps/re/iz_ac02.mcache` and on the four copies, and both sit outside
  `#to_ac1f01` (x 124–128, y 46–50) and `#to_ac1f02` (x 130–134, y 46–50). `132,45` is a
  wall, so the right stair cannot use its own center column. Downstairs landings
  `78,28` and `122,28` were already one cell outside (`y` 23–27) and were left alone.
  Noted in korangar `CLAUDE.md` §3b. `./map-server --script-check --load-script
  npc/re/warps/cities/izlude.txt` exits 0 (a broken copy of the same file exits 1).
  The running servers were not restarted, so the live map has not reloaded.
- **Live:** take each stair both ways. Upstairs stays on iz_ac02, one cell south of the
  matching stair. One step north goes back down. The two stairs no longer share a cell.

### B8. Walking the Academy — *investigated 2026-10-07, no code change*

Deeper look after P8. The stair landing was not the walking bug.

- **Floor 1 cache is stale (source-confirmed).** `use_grf` is false, so the server
  walks `maps/re/iz_ac01.mcache`. The client walks `data.grf` `iz_ac01.gat`. Same
  size, 220×200, and 1,321 cells differ. 1,266 are type 0 on the client and walls
  in the cache: four blocks on the sides of the hall, x 76–94 and x 105–123, at
  y 45–70 (411 and 398 cells) and y 77–89 (195 and 194 cells), plus thin strips
  at y 38 and y 22–23. All 228 cells where those blocks touch the agreed hall have
  height difference 0, so they are the same floor. 55 cells run the other way
  (server floor, client wall), in a patch around x 94–123, y 36–51. The `_a`–`_d`
  GAT type grids match the base map, and their caches match the stale base cache.
  `iz_ac02.mcache` matches its GAT (0 differences), including 54 type-5 cells.
  September's rebuild (`3b1ca95a9`) replaced floor 2 only. Floor 1's cache was last
  touched in git in 2018, when the dimensions already matched, so it was left.
- **Clicks past 17 steps are dropped (source-confirmed, every map).**
  `InputEvent::PlayerMove` sends the cursor cell. `unit_walk_toxy` returns failure
  when `path_len > max_walk_path`, and `conf/map/battle/client.conf` sets that to
  17. WASD already stops at 15 (`HELD_PATH`). Holding the button resends the same
  far cell. On the server-legal floor 1, from (97, 74), 6,308 of 6,794 cells are
  more than 17 steps away. Floor 2's long north hall (1,472 cells, 97 tiles long)
  has 976 cells past 17 from its center.
- **Floor 2 rooms do not connect by walking (source-confirmed).** 12 walk islands
  plus a 479-cell border the server refuses (last row and column are never
  passable). Cream Puff's `warp()` calls are the classroom links. The downstairs
  stair spans still cover the south foyer except x=129. Removing those warp cells
  does not join any rooms. Gat types 2/4/6 are not involved. Diagonal corner checks
  already match Hercules.
- **Done 2026-10-07, not live.** `maps/re/iz_ac01.mcache` and `_a`–`_d` were
  rebuilt with the mapcache plugin from `korangar/korangar/data.grf`. The new
  base cache is 220×200, its checksum matches, and it differs from the 2018
  cache in 1,321 cells: 1,266 walls (type 1) became floor (type 0), and 55
  floors became walls. Inside the four side blocks that was 411, 398, 195, and
  194 cells, and none of those flipped the other way. The four copies are
  byte-for-byte the same cell grid as the base. A map-server restart loads
  them. `@reloadscript` does not.
- **Done 2026-10-07, automated tests pass, not live.** A click or a held
  mouse button no longer names a cell Hercules will drop. `click_walk_destination`
  walks the route and keeps the furthest step `unit_walk_toxy` accepts: 17
  when the straight shot is clear, 14 when `OFFICIAL_WALKPATH` sees a wall
  (`conf/map/battle/client.conf`, `max_walk_path: 17`). A short click, including
  a short walk around a corner, is unchanged. A click into a solid wall walks
  up to the wall. The held button compares that clamped cell, so it does not
  re-send the same cell every 200 ms. WASD is unchanged.

### P9. Party location does not update on the map — *implemented, automated tests pass*

Both surfaces were wrong, for different reasons.

- **Minimap dots vanished after anyone changed maps (source-confirmed).** Hercules
  sends `ZC_GROUP_LIST` on every map change, and that packet has no tile.
  `set_roster` replaced every row and dropped the cell. The server only resends
  `0x0107` when that member takes another step, so a standing member's green dot
  stayed gone. A same-map roster refresh now keeps the cell (and the HP/SP the
  roster also does not carry). A map change still drops the old cell. `0x0107`
  with x or y `-1` (`u16::MAX` on the wire) clears the dot; that is the leave-map
  signal, not a tile. Dots still require **Show party members** (on by default)
  and only appear for members on your map. Your own red blip is unchanged.
- **Navigate did not follow (source-confirmed).** The party-window button copied
  one cell and never moved it. The route now stores that member's account id and
  updates the cell and map when `0x0107` or the roster arrives. Arrival still
  does not clear a party route. If they log out or leave the party, the route
  clears.
- **World Map required Navigate to show anyone off your current map.** The atlas
  now draws a green dot on a town where another online member is standing, and
  the detail line adds `Elsewhere: Name (map)` for members who are not on the
  selected map. Neither needs the Navigate button. A field or dungeon that is
  not one of the atlas towns is named in that line and has no dot.

### P10. Smaller reproductions

Checked 2026-10-07. One fix. The other three do not reproduce from source.

- **Trade window (does not reproduce).** The window opens once, at trade start,
  and it is resizable. After the first layout a resizable window keeps its
  stored height, so later text does not resize it. Confirm sends the commit
  and does not change the buttons. Both sides locking appends "FINAL - if you
  confirm" to the text. That makes the preferred content taller, and it does
  not change a window that already has a size.
- **Sitting head (source-confirmed, implemented).** `head_direction` is saved
  from `ZC_CHANGE_DIRECTION` and never read, so it is not the spin. Sit plays
  action group 2 on a loop. Idle already holds motion 0, which is the forward
  head; sit was cycling motions 1 and 2. On the novice, knight, and mage bodies
  those three sit motions share one seated sprite. The head ACT (hairs 1 and
  15, both sexes) uses sprite 0 on motion 0 and sprite 1 on the later motions.
  A sitting player now holds motion 0. Walking still advances. Not live.
- **`@skreset` (does not reproduce).** Permanent skills go to level 0 and the
  points are refunded. Quest skills stay, because `quest_skill_reset` is 0.
  Wedding, spirit, and permanent-granted skills stay, and Basic Skill stays on
  anyone who is not a novice. The server sends a full skill list and the
  client replaces its list, so a listed skill cannot keep an old level. No
  playtest before/after was captured. The headless scenario that expects
  SM_BASH at 0 was not re-run.
- **Emotes (does not reproduce).** The label and the bubble use the same id,
  and the bubble plays that id as the `emotion.act` action. The GRF file has
  98 actions, one per id. Neighboring actions use different sprites. The dice
  at actions 58–62 and the signal bars at 71–72 match the ids the name table
  was checked against.

## Wave 3 — scoped UI and data work

Each item is its own slice. Suggested order:

1. **Professor Hun is unnamed.** — *implemented, not live (2026-10-07).*
   `Hun#0` and `Hun#a`–`Hun#d` replace `Criatura Academy Staff` in
   `Hercules/npc/re/jobs/novice/academy.txt`. The `#` suffixes and the cell
   `izlude,122,207` stay. The dialogue already calls him Hun. The file is
   CP-949; the edit was a byte replace of that ASCII name only.
   `./script-checker npc/re/jobs/novice/academy.txt` exited 0 with no warning.
   A too-long name in a probe file still prints `npc_parsename` and exits 1,
   so a silent pass means Hun was accepted. A running server needs
   `@reloadscript` or a restart before players see the name.
2. **Quest-giver icons.** — *implemented, not live (2026-10-07).* `showevent` is
   the wrong call: it needs an attached player, so `OnInit` does nothing, and no
   NPC script uses it. `questinfo` already sends the same packet (`0x0446`) per
   player on map entry and when quest state changes. Campaign arc hubs already
   mark a quest that is not started (`QINFO_QUEST` state 0, `!`) and one that is
   in progress (state 1, `?`). The novice chain only had the in-progress mark, so
   a new player saw no icon until they had already accepted. Academy now also
   marks not-started on the Wounded Swordsman (21001), the island captain (21002),
   Lumin (7471), Izlude Carocc after Lumin is done (7472, base level 1–14), and
   Hun (7473, novice, base level 1–14). The sailor keeps a `?` only while 21002
   is in progress, because he does not give that quest. `data.grf` has
   `quest_0_1.bmp` through `quest_10_1.bmp`. The same script-check exited 0.
   Not live.
3. **NPC names always shown** (setting, default on), instead of only on hover.
   — *implemented, not live (2026-10-07).* `show_npc_names` defaults on, including
   for an older settings file that omits the field. Game settings → Map and routes
   → Show NPC names. While it is on, the client asks for each NPC's name and draws
   it over the head (the unique `#` suffix is stripped). A name behind the camera
   or off the screen is skipped, and the cursor label is not also drawn for that
   NPC. Players, monsters, warps, and ground items still use the cursor label.
   Turn it off and names go back to hover only.
4. **Quest tracker on the HUD.** — *implemented, not live (2026-10-07).* Tracked
   quests are listed in the order the server sent them. The first is the primary:
   the title is `Primary · …` and gold. Each row has Navigate, which uses the
   existing route when the quest has a map cell, and Untrack, which is the same
   control as the quest log. A quest with no cell says "No map location" and does
   not invent one. A newly seen quest is already auto-tracked. *First Step Towards
   a New World* is not in the client quest table, so this slice does not give it a
   marker; the test puts a quest of that name with no cell on the HUD as primary
   and checks that it has no route. The encyclopedia import that would add the
   cell is Wave 4. The HUD's saved default of 260×110 grew to 280 so the new rows
   fit. A height the player chose is left alone. Navigate does not set the route
   by itself.
5. **Current level on the HUD.** — *implemented, not live (2026-10-07).* A Level
   row sits above HP and reads the player's base level. Job level is not shown.
6. **Tab-target frame shows monster level and HP.** — *already present
   (2026-10-07), no new code.* The target frame prints the bestiary level (`Lv N`),
   element, race, and size. An elite keeps the Elite badge and does not print the
   base monster's level. Hercules sends -1 HP until a normal monster is damaged
   (and always for a boss). That sentinel prints "HP full", or "HP hidden" for an
   MVP. A damaged monster prints `HP current/max`. The word "unhurt" was not added.
7. **Hotbar.** — *implemented, not live (2026-10-07).* Dragging an inventory item
   onto a slot already replaced that slot, including a consumable. A slot whose
   sprite has not loaded can now be dragged too; previously it had no click
   handlers. Right-click → Add to hotbar is on identified usable stacks. If that
   item is already on the bar, that slot is replaced; otherwise it takes the first
   empty slot. A full bar that does not already hold it still says to drag onto a
   slot. Skill right-click uses the same replace-or-first-empty rule, so changing
   the chosen level updates the existing slot instead of filling a second one.
   Gear does not get the menu button. Depends on P2 so the drag does not walk.
8. **Inventory shows equipped items.** — *implemented, not live (2026-10-07).*
   A worn item draws `E` on its inventory slot, using the same equipped check the
   Equipped tab already used. The tab itself was already there.
9. **Window text.** — *crafting odds only (2026-10-07), not live.* The odds body
   was `Shrink`, so a long line became unreadably small and did not wrap. It now
   stays at 14px, wraps, and sits in a scroll view inside the existing window.
   Other windows were not resized to their content. Windows built from state
   already scroll. A custom window that still shrinks its text, or whose saved
   size is shorter than its contents, still needs a drag on the corner.
10. **Hotbar chrome.** — *implemented, not live (2026-10-07).* The key name
    (`1`, `C1`, `A1`, `S1`) is drawn to the left of the button instead of over
    the icon. The lock control is the short label "Lock" at the top of the bar,
    with a tooltip. Number keys still work while it is locked.
11. **Window transparency setting.** — *implemented, not live (2026-10-07).*
    Game settings → HUD layout → Window opacity steps through 100%, 75%, 50%,
    and 25%. It is stored with the window layout, so an older cache stays solid.
    Combat HUD fade still multiplies on top for the hotbar, status bar, and
    target frame.
12. **Music default.** — *no code change (2026-10-07).* A missing music value
    already loads as 50%. A saved 100% stays 100%, because that file cannot be
    told apart from a player who chose full volume. No migration.

## Wave 4 — owner yes on 2026-10-07

The one-page proposal gate is closed. Do one slice at a time.
- GPS: route drawn on the minimap, World Map art with town labels and click-through,
  NPCs marked on the map, NPC location verification.
  **World Map art** — *implemented, automated tests pass, not live (2026-10-07).*
  The window draws the official 1280×1024 sheets (`worldmap.jpg`, the Amatsu and
  Brasilis sheets, the dimension sheet, Midgard North, and Far-Star). Town click
  boxes are the rectangles in `worldviewdata_table.lub`. A click still only sets
  a route. Jawaii is not on those sheets, so it stays a button on the sheet row.
  The sampled minimap route and party dots were already present and were left
  alone.
  **NPC marks and location verification** — *implemented, automated tests pass,
  not live (2026-10-07).* A Towninfo facility stays on the minimap, the world-map
  route buttons, and the Adventure Guide only when a non-warp, non-story NPC
  from the server export stands on that exact cell and the role matches (kafra
  name or a `/kafras/` script, guide, tool, weapon, armor, smith or a `/refine/`
  script, inn, styling). Empty cells are dropped, including the renewal Izlude
  outdoor kafra and shops. A server NPC whose visible name is exactly one of
  those facility roles is added at its declared cell, which is how indoor dealers
  such as the Izlude Tool Dealer appear. `is_town_map` still means the map has
  any Towninfo row. 93 Towninfo rows are kept and 108 dealers are added. The
  sampled minimap route was left as the breadcrumb dots. That closes the GPS
  bullet.
- New-player path: job trainers (or one all-job trainer) in Izlude with job quests,
  directions from the Academy, a newbie guide, "which town for my class", and trainers
  listed in the Adventure Guide.
  **First-job guide** — *implemented, automated tests pass, not live (2026-10-07).*
  The six renewal instructors already stand on the Academy's second floor, and the
  town guild quests already change the job. The Adventure Guide now says which town
  each first job uses and routes to those declared cells. Novice, and a search for
  trainer, which town, newbie, first job, or academy, lists all six. Swordman is
  Izlude (`izlude_in` 74,172; trainer `iz_ac02` 60,51, not the sign at 62,51). Mage
  is Geffen, Archer is Payon, Acolyte is the Prontera church (Father Mareusis),
  Merchant is Alberta, and Thief is the Morocc pyramid (guide, then the guildsman).
  No new NPC was added. The stock Job Master in `npc/custom/jobmaster.txt` stays
  commented out: it skips the quests and it stands in Prontera, not Izlude.
- Adventure Guide: make Item Search player-facing, import the irowiki quests (the MCP
  server was down on 2026-10-07, so reconnect it first), and more readable class docs.
  **Izlude quest chain** — *implemented, automated tests pass, not live (2026-10-07).*
  iRO Wiki's Izlude category is six short pages (First Time Talking, First Step,
  Cool Drink, Gift From a Guide Staff, Hold Your Breath, Novice Skill Quest).
  The renewal script is the authority for cells and rewards; the wiki's older
  reward numbers were not copied. Quest 7472, "First step towards a new world",
  routes to Hun at izlude 122,207. Cool drink (7473) stays with Hun. The
  guide-staff hunt (7474–7477) names the airship board 179,75, the arena board
  207,167, and the Prontera field board 45,94, and turns in to Information Staff
  at 120,207. Hold your breath (15001) is Instructor Argos at izlude 140,260.
  The wreck quests the wiki does not describe are included because they come
  first: Escape the Wreck (21001) turns in to Captain Carocc at int_land 78,103,
  and The first battle (21002) turns in to the Sailor at int_land 58,69.
  int_land and iz_int are not in the navigation graph, so those two show the
  cell without a drawn route.
  **Item Search** — *implemented, automated tests pass, not live (2026-10-07).*
  An item page now leads with the drop list, then what the item does (jobs, where it
  is worn, combat numbers, the effect, buy and sell prices, and who sells it, with a
  route to the shop when the map is in the navigation graph). Crafting, quest rewards,
  script grants, turn-ins, and exchanges stay, but the file paths and reviewer notes
  are folded source lines. Drop rates stay the database figures. No shop or rate was
  invented.
  **Class docs** — *implemented, automated tests pass, not live (2026-10-07).*
  A job page now leads with the stat cap, weight, HP, SP, and weapon ASPD, then the
  job-level bonuses, then the EXP table, then the skill list. EXP milestones, HP, SP,
  and each weapon's ASPD are one line each. The numbers are the same server tables.
  The EXP and skill-tree citations fold as source lines. No class data was invented,
  and no second-job or third-job NPC was added.
  **Guide readability** — *implemented, automated tests pass, not live (2026-10-07).*
  Quest, map, NPC, service, and monster pages now lead with the player fact: who offers
  the quest, the map's rules and exits, what an NPC sells, and a monster's attack range,
  skills, and drops. File paths, reviewer names, raw skill records, and export tokens
  fold as source lines. NPC search rows say Shop, Cash shop, Trader, Warp, or NPC, and
  a service kind shows spaces instead of underscores. Skill pages name the skill, its
  requirements in words, the classes that learn it, and the reviewed formula; the file
  paths fold. Status pages name the status and the rules in words; the server constant
  and call sites fold. No drops, shops, cells, formulas, or status effects were invented.
  The coverage page stays the audit scorecard.
- Skills: refund one skill point from the skill tree. *implemented, automated tests pass,
  not live (2026-10-07).* The skill tree shows a minus beside a job skill that has a
  spent rank or a queued point. A queued point is removed on the client. A spent point
  sends `@refundskill`, which ordinary players could already use. The server returns one
  skill point unless another learned skill still needs this rank. It also refuses quest
  skills while they cannot be learned with points, wedding, spirit, and guild skills,
  permanent grants, Basic Skill after leaving the novice job, and Trick Dead after
  leaving the novice job. High Novice and Baby still count as novice. The bottom arrows
  still only choose a cast level. A cast level that would sit above the new rank is
  dropped when the skill list updates. The map-server was rebuilt and must be restarted
  before those refusals apply. Not live.
- Systems:
  **Rechargeable potion** — *implemented, script-check passed, not live (2026-10-07).*
  Item 50002, Rechargeable Potion, is a healing item the player keeps
  (`KeepAfterUse`). Using it restores 30% HP and 30% SP, then it waits 10
  seconds. The request did not name amounts; 30% is below a consumed Yggdrasil
  Seed (50%) and the wait is the item `Delay` of 10000 ms. Every Kafra menu
  sells one for 1000 zeny when the inventory does not already hold one.
  Storage, cart rental, and the free warp ticket are unchanged. The client
  name is the bundled item table; the icon stays the missing-item image.
  Storing or trading the potion lets that character buy another. The item
  database loads at map-server start, so this needs a restart, and the client
  name needs a client rebuild.
  **Kafra teleport at 800 zeny** — *implemented, script-check passed, not live
  (2026-10-07).* `F_KafSet` sets every public destination to 800 after the
  town tables, so the menu text and the charge match. A free warp ticket still
  skips the charge. Guild-castle Kafras set their own 200 zeny price and do
  not call `F_KafSet`. Storage and cart fees are unchanged. Needs
  `@reloadscript` or a map-server restart.
  **DM command polish** — *implemented, script-check passed, not live
  (2026-10-07).* `@dm` and `@dm help` stay the short command list. The
  arc01–arc19 flag shortcuts moved to `@dm help flags`. The flag handlers
  were not rewritten.
  **Party ping polish** — *implemented, compiles, not live (2026-10-07).*
  The party window shows the same six pings and Share current route. The
  buttons stay visible and disable until the character is in a party. Ping
  lifetime and the clear-on-map-change behavior were left as they are.
  **In-game menu world theme** — *implemented, compiles, not live
  (2026-10-07).* The escape menu was already on the in-game theme. Its group
  headings used a fixed blue-gray. They now use that theme's window title
  color, so a saved in-game theme colors the headings with the other world
  windows. Login and character select stay on the menu theme.
  **Graphics filters** — *implemented, automated tests pass, not live
  (2026-10-07).* Graphics Settings gains Color filter: Off, Warm, Cool, Night,
  and Mono. Off is an identity grade, so an older settings file stays
  unchanged. The grade is a multiply and a saturation on the existing screen
  blit, in linear color, before the frame is encoded. Multisampling,
  supersampling, screen-space AA, and texture filtering were already present
  and were not duplicated. The grade numbers are tested. The picture was not
  seen in a client.

## Quick live checks (no slice unless they fail)

- The framerate limit (`LimitFramerate`) caps frames in Graphics Settings.
- "Recovery paused in combat" is intended: the server reports block 4 (combat) through
  `ZC_RECOVERY_STATE` 0x0EFD. Confirm the HUD line reads that way to a player.

## Status log

| Slice | Status | Date |
|---|---|---|
| P0 Escape | implemented, automated tests pass, not live | 2026-10-07 |
| P1 Potion count | implemented, automated tests pass, not live | 2026-10-07 |
| P6 Save button restored | implemented and compiles, not live | 2026-10-07 |
| P6b PR #10 audit | handed over, nothing ported | 2026-10-07 |
| P2 UI click walks | implemented, automated tests pass, not live | 2026-10-07 |
| P3 Own headgear | implemented, automated tests pass, not live | 2026-10-07 |
| P4 Attack skill on Tab target | implemented, automated tests pass, not live | 2026-10-07 |
| P5 Navigation arrival | implemented, automated tests pass, not live | 2026-10-07 |
| P7 Rubber-banding | investigated, no cause named, no code change | 2026-10-07 |
| P8 Academy warps | implemented, not live | 2026-10-07 |
| B8 Academy walking | floor-1 caches rebuilt; click-to-move capped at the server path limit; automated tests pass, not live | 2026-10-07 |
| P9 Party location | implemented, automated tests pass, not live | 2026-10-07 |
| P10 Smaller reproductions | sitting head implemented, automated tests pass, not live; trade, skill reset, and emotes do not reproduce | 2026-10-07 |
| W3.1 Professor Hun | implemented, not live | 2026-10-07 |
| W3.2 Quest-giver icons | implemented, not live | 2026-10-07 |
| W3.3 NPC names | implemented, automated tests pass, not live | 2026-10-07 |
| W3.4 Quest tracker | implemented, automated tests pass, not live | 2026-10-07 |
| W3.5 HUD level | implemented, compiles, not live | 2026-10-07 |
| W3.6 Monster target level and HP | already present, no code change | 2026-10-07 |
| W3.7 Hotbar consumables | implemented, automated tests pass, not live | 2026-10-07 |
| W3.8 Equipped mark | implemented, compiles, not live | 2026-10-07 |
| W3.9 Crafting odds text | implemented, compiles, not live | 2026-10-07 |
| W3.10 Hotbar labels and Lock | implemented, compiles, not live | 2026-10-07 |
| W3.11 Window opacity | implemented, automated tests pass, not live | 2026-10-07 |
| W3.12 Music default | no migration; new files are already 50% | 2026-10-07 |
| W4 GPS World Map art | implemented, automated tests pass, not live | 2026-10-07 |
| W4 GPS facility marks | implemented, automated tests pass, not live | 2026-10-07 |
| W4 First-job guide | implemented, automated tests pass, not live | 2026-10-07 |
| W4 Izlude quest chain | implemented, automated tests pass, not live | 2026-10-07 |
| W4 Item Search | implemented, automated tests pass, not live | 2026-10-07 |
| W4 Class docs | implemented, automated tests pass, not live | 2026-10-07 |
| W4 Guide readability | implemented, automated tests pass, not live; skill and status pages included | 2026-10-07 |
| W4 Skill refund | implemented, automated tests pass, not live | 2026-10-07 |
| W4 Rechargeable potion | implemented, script-check passed, not live | 2026-10-07 |
| W4 Kafra 800z | implemented, script-check passed, not live | 2026-10-07 |
| W4 DM help | implemented, script-check passed, not live | 2026-10-07 |
| W4 Party pings | implemented, compiles, not live | 2026-10-07 |
| W4 Menu theme | implemented, compiles, not live | 2026-10-07 |
| W4 Color filter | implemented, automated tests pass, not live | 2026-10-07 |
