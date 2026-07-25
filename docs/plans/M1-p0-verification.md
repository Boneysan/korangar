# Implementation Plan — M1 P0 Verification (E3.1)

| | |
|---|---|
| **Status** | **34 of 34 rows verified** — macOS live pass 2026-07-10 → 2026-07-16; rejection-messages closed 2026-07-22 (unit + headless). Clean logout verified live 2026-07-16. The 2026-07-15 GUI sitting fixed several client bugs and filed more — see §5. |
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

**macOS** (where the current live pass runs — see [MACOS_WORKFLOW.md](../MACOS_WORKFLOW.md)):

```bash
# DB first — MariaDB no longer autostarts on the Mac; start it by hand.
# Use `run`, NOT `start` (`brew services start` re-registers autostart).
brew services run mariadb

cd /Volumes/T7/GitHub/Ragnarok_Online/Hercules && ./athena-start start
# expect listeners on 6900 / 6121 / 5121 (+ api 7121)

# Client — no env vars needed; wgpu picks Metal natively
cd /Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar
cargo run --release --bin korangar
```

**WSL** (the other dev machine — paths differ; see the WSL section in the root `CLAUDE.md`):

```bash
cd ~/GitHub/Hercules_RO && ./athena-start start
cd ~/GitHub/korangar && ./run-wsl.sh   # GL/D3D12 launcher — do NOT plain `cargo run`
```

If MariaDB is restarted while Hercules is running, **restart Hercules too** — the
servers survive but hold dead connections (their `MYSQL_OPT_RECONNECT` is deprecated).
A healthy stack shows ~6 `ragnarok` rows in
`select id, host, command from information_schema.processlist;`, not 1.

Test account (from M0): `korangar` / `korangar` (or create a fresh novice).

Enable packet logging when debugging: `KORANGAR_PACKET_LOG=1` if supported, or the
in-client packet inspector (`debug` feature).

## 3. Checklist

Mark each item ✅ / ❌ / 🔶 and note the defect.

> **This checklist is the GUI axis only.** An unchecked row here does **not** mean
> "untested" — most of these rows are already covered by a green scenario in the
> 106-scenario headless suite (`tools/testing/headless_test_plan.md`, acceptance passed
> 2026-07-13). Headless links the same `ragnarok-packets` / `korangar-networking` crates
> as the real client, so headless-green means **the wire protocol and event mapping are
> correct**; what is unproven is only the `korangar/src/` UI/state layer — i.e. does the
> window render, and is it clickable. Rows below are annotated with their covering
> scenario (`headless: name`) so nobody re-reads them as "never tested". Rows with **no**
> headless annotation are the ones with genuinely no automated coverage — mostly
> UI-only concerns headless cannot reach by construction.

### Account & session (P0)
- [x] Login with valid credentials — macOS 2026-07-10 (`korangar`; `loginlog` rcode 100)
- [x] Login failure shows a visible message (bad password / already online) —
      live-verified macOS 2026-07-15 (server-side refuse). **GUI re-verified 2026-07-22:**
      wrong password shows a red status line on the **login form**
      ("Incorrect username or password") — no separate Error popup. Fixed stale
      half-open login sockets so retries are never a silent no-op. *headless:
      `bad-password` covers the refusal packet.*
- [x] Character select lists existing chars — `test` displayed and selectable on macOS
- [x] Character create works — live-verified macOS 2026-07-15: created `yoyo`, which
      persisted correctly in slot 2. (Also validated in M0 2026-07-08.) **Creating a 3rd
      character exposed M1-013** — the list then rendered empty. *headless:
      `character-create-delete`.*
- [x] Character delete (if used) fails safely — live-verified macOS 2026-07-15: deleted
      `yoyo`, gone from the DB. **Delete is a right-click context menu** on the character
      slot (`character_selection.rs:336`), not a visible button — see M1-014.
      *headless: `character-create-delete`, `character-delete-after-play`.*
- [x] Map load after select (no black world / freeze) — entered `int_land`; map TCP connection established
- [x] Clean logout / disconnect — live-verified macOS 2026-07-16: logged in, logged out
      cleanly back to the login/character-select screen, no hang or stuck state, client
      process stayed healthy. *headless: `logout-relogin` (incl. chat-marker proofs).*

### Core gameplay (P0)
- [x] Click-to-move, pathing (~10 tiles) — full route live-verified on macOS 2026-07-10
- [x] Sit / stand — Home compatibility binding live-verified on macOS 2026-07-10; re-confirmed 2026-07-15. **Insert does not work on this Mac** (laptop keyboards don't expose it) — Home is the macOS path.
- [x] Basic melee attack + damage numbers — Poring live test: sword cursor, approach, hit/miss and damage numbers, moving-target chase; no freeze/disconnect
- [x] Skill damage numbers (if character has a damaging skill) — live-verified macOS
      2026-07-15: Rogue `RG_RAID` after Hiding damaged the mob and rendered damage
      numbers. **No skill animation played — see M1-008.** *headless: `skills-*` (39
      job-class sweeps) + `attack-kill` / `incoming-damage` cover the damage-event wire
      path.*
- [x] Item pickup from ground — labels English live-verified macOS 2026-07-10 (M1-001); re-check pickup path if needed
- [x] Stats window + stat allocation (success path) — live-verified macOS 2026-07-15: points adjust correctly. *headless: `stat-skill-points`.*
- [x] Skill tree opens; skill use (self / target / ground if available) — live-verified macOS 2026-07-15:
      self-cast (Hiding) and self-AoE (Raid) both fire. **Targeted skills require the
      mouse to already hover the target — see M1-006.** *headless: `skills-novice` …
      `skills-soul-linker` — **39 job-class sweeps**, plus `teleport-select` /
      `teleport-cancel` for the skill-menu path.*
- [x] Hotbar use — live-verified macOS 2026-07-15: Fn+F1 cast Hiding (macOS needs Fn, or enable standard function keys — F1-F9 are media keys by default). *headless: `hotkeys`.*
- [x] Death → respawn window — killed on `moc_fild22`; Respawn returned to the saved Payon point on macOS 2026-07-11
- [x] Weight footer updates in inventory (pick up / drop items; soft/hard color) —
      live-verified macOS 2026-07-15: weight changes on pickup. *No headless coverage —
      footer rendering is UI-only, so this row had no other safety net.*

### Items & economy (P0)
- [x] Inventory open / use / equip / unequip / drop — open, tooltips, drop, drag, reorder, split live-verified macOS 2026-07-10 (M1-001–003)
- [x] NPC shop buy — live-verified macOS 2026-07-15: bought an arrow; dialogue and shop flow behaved. *headless: `shop-buy-sell` (full cart flow, `0x0B77` wire).*
- [x] NPC shop sell — live-verified macOS 2026-07-15: sold a Cotton Shirt to the Payon Weapon Dealer (`payon_in01,15,119`). **Note:** stock Hercules shops are `trader` NPCs, not the legacy `shop` type — searching for `shop` finds only `npc/custom/itemmall.txt`, which is commented out in `scripts_custom.conf`. Traders default to `NST_ZENY`, so they both sell and buy. *headless: `shop-buy-sell`.*
- [x] NPC shop **English names** (tool dealer — live 2026-07-10; EN `itemInfo` overlay)
- [x] Item identify (magnifier / double-click unidentified) — live-verified macOS
      2026-07-15: gear identified correctly. **Fixture:** `@item` only grants *identified*
      items, so `Hercules/npc/custom/identify_test.txt` adds an **Identify Test NPC at
      `prontera,164,200`** handing out 4 unidentified pieces via `getitem2` (identify flag
      0). Deliberately a separate NPC — the headless `dialogue-choice` scenario asserts the
      Dialogue Test NPC's menu exactly and picks by index. *headless: `identify`.*
- [x] Kafra storage open / store / retrieve / close — UI grid live 2026-07-11 (stock Kafra uses `close2` then `openstorage`: click dialog **Close** first; see [storage-window.md](../storage-window.md))

### Social (P0)
- [x] Public chat send + receive — live-verified macOS 2026-07-15: GUI chat box sends and displays. *headless:
      `smoke` / `logout-relogin` `say()` a marker and await the `ChatMessage` echo.*

### NPC & world (P0)
- [x] NPC dialog: mes + next + close — live via Kafra 2026-07-11
- [x] NPC dialog: menu choices — live via Kafra teleport 2026-07-11
- [x] **NPC number input** (E3.3) — live-verified macOS 2026-07-15 via Dialogue Test NPC (`prontera,160,200`). *headless: `dialogue-number`.*
- [x] **NPC string input** (E3.3) — live-verified macOS 2026-07-15 via Dialogue Test NPC (`prontera,160,200`). *headless: `dialogue-string`.*
- [x] Warp / map change — Kafra teleport live-verified 2026-07-11
- [x] Kafra **save point** (dialog-only; storage window is E4.4) — live-verified 2026-07-11
- [x] **Job change + appearance** (E3.4) — live-verified macOS 2026-07-15: `@job` changed
      class and the sprite updated correctly, no silhouette. *headless: `gm-job` covers the
      `ChangeJob` wire path only — the **sprite** is UI-only and unproven. A silhouette
      means the sprite failed to resolve, not that job change failed — see the
      "Classic mob silhouettes — jobname.lub overlay" section of
      [2026-07-13-session-notes.md](../2026-07-13-session-notes.md). This row was
      previously untracked here despite E3.4 existing in
      [PROJECT_PLAN.md](../PROJECT_PLAN.md).*

### Status / feedback (recent work)
- [x] Buff bar shows timed status (Blessing / `@useskill` / consumable) — live-verified
      macOS 2026-07-15: `@useskill 34 10 test` produced `Effects: 46 10:218s` — i.e.
      `SI_POSTDELAY` (46) and **`SI_BLESSING` (10) counting down from 218s**. Wire data and
      timer are correct. **Displays raw indices instead of names/icons — see M1-010.**
      *No headless coverage — the bar is a UI widget, so this row had no other safety net.*
- [x] Buff expires and tile/summary clears — live-verified macOS 2026-07-15: Blessing
      ran out and cleared on its own. **Zero-duration effects did *not* clear — see
      M1-011**, fixed same day. *No headless coverage (UI-only).*
- [x] Rejection messages in chat (skill fail / known ZC_MSG) — closed 2026-07-22.
      Path was already wired (`ZC_ACK_TOUSESKILL` 0x0110 → red `ChatMessage` via
      `skill_failed_text`; `ZC_MSG` 0x0291 / `ZC_MSG_COLOR` 0x09CD → `MessageTable` →
      `MsgStringTable` → chat). Prior live coverage: login-failure message (2026-07-15),
      friend/party rejection chat (`friend-reject`, `party-reject-block`). **Skill-fail
      specifically:** headless `skill-fail-rejection` PASSed 2026-07-22 (Mage Fire Bolt
      at 0 SP → rejection chat). Unit fixtures: `skill_fail_0x0110_surfaces_chat_message`,
      `skill_fail_basic_skill_gate_has_readable_text`, `message_table_0x0291_*`,
      `message_table_color_0x09cd_*` in `korangar-networking`.

## 4. NPC input smoke scripts

**Already done — nothing to add.** `Hercules/npc/custom/headless_dialog_test.txt`
(added 2026-07-12, `5c421f162`) provides **Dialogue Test NPC** at **`prontera,160,200`**,
and it is already loaded via `npc/scripts_custom.conf`. Its menu covers four of the
rows in §3 in one place:

| Menu choice | Covers |
|---|---|
| Linear Dialogue | `mes` + `next` + `close` |
| Number Input | **NPC number input** (E3.3) |
| String Input | **NPC string input** (E3.3) |
| Dialogue Warp | dialogue-driven `warp` (`close2` → `warp payon,150,150`) |

Just walk to it — it's a short walk from the Prontera spawn, well inside
`max_walk_path`. If you edit the script, `@reloadscript` (GM) or restart map-server.

## 4.1 Run sheet — the remaining GUI pass in one sitting

§3 is grouped by category; this is the same rows in **execution order**, arranged to
avoid needless relogging and walking. Nearly every row is already headless-green, so
unless noted you are only confirming **the window renders and responds** — not whether
the protocol works. The genuinely uncovered rows are marked ⚠️: nothing else tests them.

Prereqs: `brew services run mariadb` → `./athena-start start` → client. Char `test`
(level 50, Admin group 99, spawns in Prontera).

**A. At the login / char screen** (do first — needs repeated login)
1. Bad password → **login failure message visible**.
2. Create a throwaway character → **char create**.
3. Delete that character → **char delete fails safely**.

**B. In Prontera, standing still** (log in as `test`)
4. Type in chat → **public chat send + receive**.
5. Open stats → allocate a point → **stats window + stat allocation**.
6. Open skill tree → use a skill → **skill tree + skill use**.
7. Drag a skill to F1, press it → **hotbar use**.
8. ⚠️ Pick up / drop an item → **weight footer updates** (soft/hard colour).
9. ⚠️ `@useskill 34 10 test` (Blessing, `SC_BLESSING`) → **buff bar shows the tile**…
10. ⚠️ …wait for it to lapse → **buff expires and the tile/summary clears**.
11. Unidentified item → magnifier → **item identify**.

**C. Prontera NPCs** (a short walk each)
12. Tool dealer → **shop buy** and **shop sell** (EN names already verified).
13. **Dialogue Test NPC at `prontera,160,200`** (see §4) → Number Input → **NPC number
    input**; String Input → **NPC string input**. Leave *Dialogue Warp* for last — it
    sends you to Payon.
14. `@job` across a few classes → **job change + appearance** (E3.4). A silhouette is a
    *sprite-resolution* failure, not a job-change failure.

**D. In the field**
15. Attack a mob with a damaging skill → **skill damage numbers render**.
16. Force a skill failure → **rejection message in chat**. ✅ Closed 2026-07-22
    (headless `skill-fail-rejection` + unit fixtures; see §3 row).

**E. Closing**
17. Keep playing to ~30 minutes total → exit criterion **no framing desync / silent
    hang**. B–D usually covers most of this.
18. Log out cleanly → **clean logout / disconnect**.

File anything broken in §5 with repro steps (that is the E3.1 exit bar — rows may exit as
a filed defect, not only as ✅), then hand to E3.2.

## 5. Defect log

| ID | Area | Severity | Notes | Status |
|---|---|---|---|---|
| M1-001 | Ground items | P0 | Poring drops rendered as `[][][]` on hover. Root cause: startup loaded only Korean `iteminfo.lub`; external English-table link was broken. Durable fix: 13,182 English names from Hercules-backed `docs/items.json` compiled into the client. | ✅ Live-verified macOS 2026-07-10 (ground items show English) |
| M1-002 | Inventory UX | P0 | Hovering inventory items showed no identifying text/tooltip. `ResourceMetadata.name` was already populated; `ItemBox::lay_out` only rendered texture/amount. Fix: register `layout.add_tooltip(&item.metadata.name, …)` on hover (same pattern as `SkillBox`). Applies to inventory, equipment, and storage slots that share `ItemBox`. Framework tooltip delay is ~1s. | ✅ Live-verified macOS 2026-07-10 (hover ~1s shows EN name) |
| M1-003 | Inventory actions | P0 | Drop, drag-to-equip, ground drop, in-inventory reorder, and split (partial drop: half / off 1 / drop all) via right-click menu. `CZ_ITEM_THROW2` 0x0363 + 0x00AF ack; end-of-frame UI event flush for drag. | ✅ Live-verified macOS 2026-07-10 |
| M1-004 | Campaign map placement | P0 | Arc 19 warped players and placed its encounter at non-walkable void cells `moc_fild22,150,150` / `155,150`, producing black surroundings, no destination marker, and no movement. Deep audit proved the map, lighting, terrain, and pathing data valid. Moved the rift/boss/hazards to walkable `(170,140)` and the choice NPC to `(175,140)`. | ✅ Live-verified macOS 2026-07-11 |
| M1-005 | NPC dialogue window | P0 | **Dialogue box renders collapsed to its minimum size.** The `mes` text is present but the window does not size to its content, so it reads as an empty box until manually resized. Found on the Prontera "Vandez" NPC, macOS 2026-07-15. Layer: **client UI** (`interface/windows/dialog.rs`) — headless `dialogue-linear` / `dialogue-choice` are green, so the wire data is correct and only layout is wrong. Repro: talk to any `mes`-using NPC; observe the box at minimum height with text clipped. Fixed 2026-07-15: added `minimum_width: 400.0` / `minimum_height: 280.0` to the `window!` in `interface/windows/dialog.rs` (it declared no size floor at all), matching the seeded `WindowClass::Dialog` default in `cache.rs`. Also purged the poisoned 392x185 entry from `client/window_cache.ron` — the collapsed size had persisted, so it reopened broken every session. | ✅ Live-verified macOS 2026-07-15 — dialogue sizes correctly and closes normally |
| M1-006 | Skill targeting | P0 | **No skill-targeting mode.** Targeted skills fire only if the mouse already hovers the target at the instant of the keypress (`lib.rs:4086` requires `PickerTarget::Entity`; `4095` requires `PickerTarget::Tile`). The original client's press-skill → reticle-cursor → click-target flow does not exist: `MouseInputMode` has only `RotateCamera`, `Walk`, `MoveItem`, `MoveSkill`. Player-visible symptom: "skills don't target". Layer: **client UI/input**. Invisible to headless, which calls `cast_skill(id, level, entity_id)` with an entity id and never touches the picker. Repro: put any targeted skill on the hotbar, press its key without hovering a mob — nothing happens, no feedback. | ✅ Fixed + live-verified macOS 2026-07-16. Added a `PendingSkill` armed-target state (`lib.rs`): pressing a targeted skill with no valid target under the cursor arms it, the cursor becomes the attack reticle, and the next left-click picks the target. **Per-type behavior:** *Attack* — hover-instant fast path, else arm; *Ground/Trap* — **always** arm so the AoE is aimed and placed by the click (instant-casting at the cursor gave no way to aim — the "Storm Gust isn't selectable" symptom); *Support* — kept its hovered-entity-else-self cast (self-buffs can't be aimed by clicking your own sprite); *SelfCast* — unchanged instant self-cast. **Cancel:** right-click (primary) or Escape (Escape cancels the target before it opens the menu). **Swap:** pressing another skill re-arms; a chat line names the armed skill so the swap is visible. `resolve_pending_cast` is a pure, unit-tested mapping (6 tests). Cleared on map disconnect so it can't leak across relogin. **Test-char trap found:** granting cross-tree/prereq-less skills gets them pruned on login (`pc_calc_skilltree`) → they read as unlearned/dimmed and won't cast; a valid full job tree is required. **Confirmed separate & still open:** skills draw no cast animation (M1-008), and there is no in-progress-cast interruption (RO cancels a cast by moving) — neither is part of skill *targeting*. |
| M1-009 | No gear stats or comparison | P1 | **Fixed 2026-07-22:** shared `world/library/item_stats.rs` embeds Hercules `docs/items.json` (outside `src/dm/`) and inventory/equipment/storage hover tooltips show ATK/MATK/DEF/slots/req Lv/weight plus **vs equipped** deltas when the slot already has gear. | ✅ Code complete — live GUI confirm recommended |
| M1-010 | Buff bar shows raw indices, not names | P1 | **Names 2026-07-15/16; icon monograms 2026-07-22:** each line is `[Bl+] Blessing 218s` — two-letter monogram + role tag (`+` buff / `−` debuff / `·` utility). **Effect descriptions + shared-icon disambiguation 2026-07-24:** a curated `status_description` renders an indented second line (`MAXIMUM_DISPLAYED_EFFECTS` 8 → 5 to pay for the height). Two problems had to be solved to make ground fields readable. (1) **The icon is lossy** — `SC_VOLCANO`, `SC_DELUGE` and `SC_VIOLENTGALE` all declare `Icon: "SI_GROUNDMAGIC"` (112) in `db/re/sc_config.conf`, so all three rendered as one meaningless name. New `world/skill_unit_registry.rs` tracks every live skill unit's `UnitId` + position (same lifetime points as `EffectHolder`) and names the field the player is standing in. (2) **The values were zero** — `StatusChangePacket` carries `value: [u32; 3]` but all three registrations dropped it, *and* Hercules' `status_get_val_flag()` had no case for these statuses so it sent `val1 = 1, val2 = 0` regardless. Forwarded client-side **and** patched server-side (`Hercules/src/map/status.c`, a fork delta — see CLAUDE.md §3b). Live: `+30 ATK & MATK`, `+15% Max HP`, `+15 Flee`. Also **synthesises a Land Protector hint** (`[ME·] Magnetic Earth / Ground magic suppressed here`) — that skill grants no status at all because it acts on the ground, so nothing otherwise told the player their magic was suppressed. Real GRF SC sprites still deferred (needs licensed artwork). | 🟡 Names + monograms + descriptions live-verified 2026-07-24; real sprites deferred |
| M1-011 | "Skill Delay" never clears | P0 | **A zero-duration status stuck on the buff bar permanently.** Reported live 2026-07-15: after un-hiding, `Hiding` cleared correctly but `Skill Delay` remained forever. Root cause is client-side: `StatusEffects::apply` treated `duration_ms == 0` as *infinite* (`expires_at = None`), and `tick()` retains timerless effects by design. But zero duration means **already over**, not forever — Hercules signals genuinely-permanent with `INFINITE_DURATION = -1` (`src/common/mmo.h`), which arrives as `u32::MAX` and was already handled correctly. `SC_POSTDELAY` is the case that bites: `skill.c:6616` fires it via a **direct `clif->status_change`** with `delay_fix()` as the duration and **no `sc_start`** — so there is no server-side timer and **no end packet ever arrives**; the client alone must expire it. A skill with no after-cast delay sends 0/0, so the icon hung forever. Fixed 2026-07-15: expiry now keys off `remaining_ms == u32::MAX` only. 3 regression tests (zero-duration expires, infinite retained until server end, timed expires on schedule). Layer: **client UI/state**. Invisible to headless — no scenario asserts on buff-bar contents. | ✅ Live-verified macOS 2026-07-15 — Skill Delay no longer sticks; it refreshes per cast and expires. Hiding (real 300s duration, `SkillData1 Lv10: 300000`) still counts down correctly, and Cloaking's `INFINITE_DURATION` path is unaffected |
| M1-012 | Cancelled buffs never cleared | P0 | **A buff cancelled early kept its timer on screen forever.** Live 2026-07-15: un-hiding cleared the sprite (M1-007) but left `Hiding` counting down. Root cause is an asymmetry: statuses **start** on `0x0983` (handled) but **end** on **`0x0196`** (`clif_status_change_end` → `status_change_endType`, `state = 0`), and `0x0196` (`StatusChangeSequencePacket`) was **`register_noop`** — parsed for framing, then discarded. The client's `remove(index)` path existed and was correct; the packet simply never reached it. So a buff could only vanish when its own client-side timer ran out — Blessing looked fine because it ran its full 240s; cancelling early has no timer to save it. **The name hid it:** §8.3 listed `StatusChangeSequencePacket` as pending work, but nothing suggested it was *how buffs end*. Also masked by [M1-011]: while everything was wrongly infinite, nothing expired and the missing end packet went unnoticed — fixing expiry exposed this. Fixed 2026-07-15: promoted to a real handler → `NetworkEvent::StatusChange { gained: false }`; layout test asserts the 9-byte wire shape against Hercules' `packet_status_change_end`. Layer: **shared crate**. Invisible to headless — no scenario asserts on buff-bar contents. | ✅ Live-verified macOS 2026-07-15 — timer clears on un-hide |
| M1-013 | Character list wiped at exactly 3 characters | P0 | **The character select rendered empty once the account had 3 characters.** Live 2026-07-15: creating a 3rd character (`yoyo`) emptied the list; every relogin then showed nothing. No data loss — all characters were always intact in SQL. Cause: `char.c:2161` — *"send empty packet if chars count is 3, for trigger final code in client"* — Hercules sends the list, then a **second, empty `0x0B72`** as an end-of-pagination marker the official client depends on. The two are indistinguishable by content, and `CharacterSlots::set_characters` cleared all slots before re-adding, so the terminator wiped the list a moment after it was built. Only reproducible at **exactly 3** characters, which is why character select passed all day at 2. Fixed 2026-07-15: an empty list is ignored as the terminator; safe for a genuinely empty account since slots start `None`. 3 tests. Layer: **client state**. Invisible to headless — `character-create-delete` cleans up after itself and never holds 3 characters at once. Note `PLATFORM_BRINGUP.md` records this code path breaking once before (PACKETVER mismatch) — same symptom, different cause. | ✅ Live-verified macOS 2026-07-15 — list renders with 3 characters |
| M1-014 | Character delete is hidden behind right-click | P2 | **Fixed 2026-07-22:** hover tooltip documents Left-click play / Right-click Delete·Switch; delete is **two-step** (`Delete {name}…` → `Really delete {name}?` → Yes, delete). Layer: **client UI**. | ✅ Code complete — live GUI confirm recommended |
| M1-015 | Stuck at server select after a failed login | P2 | **Closed 2026-07-22 (live):** failed login shows status on the login form; successful login with a **sole** character server auto-enters it (no select flash). Multi-server stacks still show select with a 30s AUTH countdown and return to login on expiry/handoff failure instead of silent reconnect loops. Connect paths force-clear half-open sockets so retries work. | ✅ Live-verified macOS 2026-07-22 |
| M1-008 | Skills have no visual effect | P1 | **Partial — ongoing with Phase E.** Wizard kit live 2026-07-17; E1 code-drawn Mage/Wizard effects + 0x01F3 `special_effect_recipe` 2026-07-22. **E2 units all closed live** (both batches + Hunter traps as runtime RSM props, 2026-07-26). **Status effects now have entity visuals** (2026-07-25/26): opt1/opt2 tints incl. a shader desaturation path for petrification, plus attached looping status STRs for stun/sleep/poison/silence. Cast circles are wired (`Lockon` + `Beginspell`) though not live-checked. Still open: catalog-wide effect-ID coverage (~79 mapped of 1124 variants — a long tail, harvest via `KORANGAR_PACKET_LOG`), sound retry, and E4's stun/sleep poses + refresh variants. See [animation-fidelity.md](animation-fidelity.md) §6. | 🟡 Expanded; catalog incomplete by design |
| M1-007 | Hide/cloak has no visuals | P0 | **`StateChangePacket` (`0x0229`) is `register_noop`** (`version_20220406.rs:709`), so the client discards every option change. `OPTION_HIDE` (0x02) / `OPTION_CLOAK` (0x04) / `OPTION_INVISIBLE` (0x40) never reach the UI: no sprite transparency, no icon, no buff-bar entry (nothing maps `SC_HIDING`). The player cannot tell whether Hiding is active. Knock-on: hide-gated skills appear broken — `RG_RAID` is `Self: true` and requires `State: "Hiding"`, so it silently refuses when not hidden, with no way to diagnose. Server state is correct (`pc_ishiding` reads the same mask); this is purely the client dropping the packet. Layer: **shared crate + client UI**. Invisible to headless by construction: a noop packet emits no `NetworkEvent` to assert on. Fixed 2026-07-15: promoted to a real handler → `NetworkEvent::StateChange`; `EntityOption` bitflags added to `ragnarok-packets` (values checked against `mmo.h`, `is_concealed()` mirrors the `pc_ishiding` mask, unknown bits truncated so a newer server can't panic us); `Common.option` stores it and `Common::render` multiplies the existing `fade_state` alpha by 0.3 when concealed — no renderer change. 4 regression tests in `ragnarok-packets`. | ✅ Live-verified macOS 2026-07-15 — hide shows a visible cue, and Hide → Raid then worked |
| M1-017 | Client crashes on logout | P1 | **Logging out closes the client entirely instead of returning to character select.** Reported live 2026-07-22. Panic: `called \`Option::unwrap()\` on a \`None\` value` at `rust-state/src/state.rs:409:38` (`State::get` on a safe selector / incorrect `ManuallyAssertExt`). **ROOT CAUSE (two layers):** (1) Skill Tree stored `this_player().manually_asserted().skill_points()` and other safe paths that unwrap when `this_player` is gone after `entities().clear()`. (2) **Primary live trigger:** `NetworkEvent::LoggedOut` called `skill_tree().clear()` *while Skill Tree was still open*; `tabs().index(n).skills().manually_asserted()` then panicked on the next layout before `MapServerDisconnected` closed windows (can be a later frame). Stats-only logout was clean; Skill-Tree-only crashed. **FIXED 2026-07-22:** close all windows on `LoggedOut` *before* clearing skill_tree/hotbar; Skill Tree uses optional `this_player().skill_points()` and optional tab skills path (`try_get` / empty layout if missing) in `interface/windows/skill_tree/{mod,tabs,slot,state}.rs` + `lib.rs`. Layer: **client UI/state**. Invisible to headless. | ✅ Live-verified macOS 2026-07-22 — Skill Tree open → Log out returns to character select |
| M1-016 | Emotes show no bubble, only chat text | P2 | **Root cause:** the local player is stored at `this_entity()`, separately from `client_state().entities()`, but the emote name and render paths searched only the latter. The server echoes the sender's emote, so a self-emote became `Someone: delighted` and the bubble had no entity position. Fixed by resolving emote owners across nearby, local-player, and recently-dead entity collections; local chat labels use `player_name`. Verified the configured GRFs contain `data\\sprite\\이팩트\\emotion.spr/.act` with an ignored asset regression test. First GUI smoke check confirmed the animation loaded and rendered, but exposed a second issue: the generic animation loader normalizes each ACT frame to its bottom edge, so drawing at the entity's ground anchor put the emote under the character. The anchor now derives from each entity's current composed sprite-frame height plus a small gap, adapting automatically to players, small mobs, and tall bosses. Added a scrollable 81-entry emote palette, available from the in-game menu or `Alt+L`; buttons send the same packet as `/e <id>`. Layer: **client UI/loaders**. | ✅ Live-verified macOS 2026-07-16 — picker emote animates above the player |

## 5.1 Live session notes

- **2026-07-22, macOS — login / server-select UX:** wrong password paints a red
  status line on the login form (popup path removed as noisy). Valid login with
  one Hercules character server skips server select and lands on character
  select (`[login] sole character server 'Hercules' — auto-entering`). Char
  handoff failures bounce back to login with a status message. Networking
  `connect_to_*` now tears down ClosingManually/stale Connected so a second
  attempt is never a silent no-op.
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
