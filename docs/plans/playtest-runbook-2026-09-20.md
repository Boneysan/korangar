# Execution runbook — 2026-09-20 playtest backlog

This is the operational companion to
[`playtest-adjustments-2026-09-20.md`](playtest-adjustments-2026-09-20.md).
The adjustment plan defines product intent and priority. This runbook defines
the order of work, the exact evidence each card must produce, and when the
executor may move to the next card. It uses the same loop, record format, and
working rules as [`qwen3-playtest-runbook.md`](qwen3-playtest-runbook.md); the
rules are restated here only where this backlog adds anchors or constraints.

Task IDs are `PT-###` so they cannot be confused with the earlier `QW-###`
queue. Several PT cards intentionally reuse evidence already produced by QW
cards; those cards say so and must not re-derive it.

## Current pointer

**NEXT: PT-000 — record the execution baseline**

**EXECUTION STATE: READY TO RESUME**

To start the loop, change `READY TO RESUME` to `RUNNING`. Only change `NEXT`
after the current card has a completed evidence record. When a card is done,
tick its checkbox, update `NEXT`, and immediately begin the next runnable card
in the same session without asking whether to continue. Stage boundaries are
not stopping points. A card that needs a human decision, a second live player,
Windows hardware, or an upload is recorded as `BLOCKED` with the exact missing
input and the loop moves to the next independent card.

## Continuous executor loop

```text
while EXECUTION STATE == RUNNING:
    read NEXT
    git status -sb in korangar/ and Hercules/; inspect the current diff
    read the card; locate every named symbol with rg before editing
    perform the card, or split it into ordered child cards (PT-010a, PT-010b …)
    run the card's validation ladder
    write the evidence record directly under the card
    mark DONE only when every clause of "Done when" is true
    otherwise mark IN PROGRESS (runnable work remains) or BLOCKED (external)
    select the next runnable card in queue order
    update NEXT
    continue immediately; do not end the response
```

A progress summary, a passing test, a completed card, a stage checkpoint,
context compaction, or a blocked card is **not** a terminal condition. A final
response is allowed only when every card is `DONE`, or every remaining card is
genuinely `BLOCKED` on something outside the repositories. Before any final
response, search this file for unchecked boxes and justify each one.

Before expected context compaction, write the exact current symbol, diff, last
command and result, and next command into the active card's record.

## Working rules specific to this backlog

The ten rules in `qwen3-playtest-runbook.md` § "Non-negotiable working rules"
apply unchanged. In addition:

1. **Anchors are resolved, not guessed.** Every symbol below was located in the
   checkout on 2026-09-20. Re-verify a line number with `rg` before editing —
   they drift — but do not substitute a "similar-looking" symbol.
2. **Two reports, one bug.** Where the plan says two reports share a root
   cause (window z-order; quantity dialog; late-join quest sync), the card for
   the root cause is the only card that may fix it. The dependent card is a
   verification card and must not carry a second fix.
3. **Live GUI evidence is required for every UI card.** The headless suite
   proves wire data only. `shop-buy-sell` already passes headlessly — that is
   exactly why the vendor bug is a UI bug, and why a headless pass cannot close
   any card in Stages 1, 2, 3, 4, or 6.
4. **A decision card produces a written decision, not code.** Stage 8 cards
   end with a recommendation and a `BLOCKED: awaiting DM decision` record; the
   executor does not pick a gameplay policy on the user's behalf.
5. **Do not touch the live server config or database while players are
   online** except through in-game GM commands. Tonight's session showed a raw
   SQL edit to an online character's persistent variable can be overwritten on
   the next autosave.

### Anchor table

| Area | Anchor (verified 2026-09-20) |
|---|---|
| Window stack | `korangar-interface/src/lib.rs`: `open_new_window` (~380, appends to `self.windows`), `Event::MoveWindowToTop` (~525, the only reorder), `lay_out_windows` (~545; hover pass iterates `.rev()` at ~665), `InterfaceFrame::render` (~733; draws in forward order), `InterfaceFrame::click` (~878), `close_top_window` (~458) |
| Char select / creation | `korangar/src/lib.rs` `InputEvent::OpenCharacterCreationWindow` (~8723); `interface/windows/character_creation.rs` `MINIMUM_NAME_LENGTH = 4` (~86), `disabled` selector (~119), Create button (~335) |
| Vendor buy | `interface/windows/buy.rs` `disabled_cutoff` (~224), `AddAction` (~321), `+1/+10/+100` (~369); `buy_cart.rs` Buy button (~351) → `InputEvent::BuyItems`; `lib.rs` `NetworkEvent::OpenShop` (~7349) opens `BuyWindow` then `BuyCartWindow` (~7374-7376); `korangar-networking/src/lib.rs` `purchase_items` (~1273) → `BuyItemsPacket` `0x00C8`; result `BuyItemsResultPacket` `0x00CA` |
| Vendor sell | `interface/windows/sell.rs` `disabled_cutoff` (~227), `+1/+10/+100` (~290-300) |
| Quantity dialog | `interface/windows/quantity.rs` — `QuantityWindow::new(_quantity_path)` ignores the path; buttons `−`/`+`/`All`/Confirm/Cancel; **no value readout**; `state/quantity.rs` `QuantityChooser` `increment` (~98), `decrement` (~107), `confirm` (~168); `lib.rs` handlers ~7905-7917 |
| Inventory order | `state/inventory.rs` `fill` (~20, replaces the list), `add_item` (~27, pushes), `remove_item` (~58), `reorder_display` (~87, manual order already exists) |
| Quest tracker | `lib.rs` opens `TrackedObjectiveWindow` once on world entry (~4871); `InputEvent::ToggleQuestTracking` (~10164) calls `refresh_tracked_quest_display` and never `open_window`; `interface/windows/tracked_objective.rs` `closable: true` (~316); `interface/windows/menu.rs` has no tracker toggle |
| Breadcrumb | `input/event.rs` `ToggleBreadcrumbCollapsed/Hidden/Guidance`, `BreadcrumbScale/Opacity` (~192-204); handlers `lib.rs` ~9349-9391 |
| Armed skill | `lib.rs` `PendingSkill` (~1350), `PendingCastResolution` (~1373), `resolve_skill_activation` (~1414), `resolve_pending_cast` (~1469), armed-click path (~11451-11486: fizzle re-arms), right-click cancel (~11516) |
| Status log | `state/combat_chat.rs` `"[Status-] Lost {status}"` (~315); `state/status_effects.rs` `status_name` (~32) formats unknown ids as `#{index}`; Hercules `src/map/status.h` `SI_BLANK = -1` (~952) → wire `0xFFFF` = 65535 |
| Steal stops attack | Hercules `src/map/unit.c` `unit_skilluse_id2`: `if (!temp) unit->stop_attack(src); // Stop attack on non-combo skills` — server-intended |
| Skill fail cause | Hercules `src/map/clif.h` `enum useskill_fail_cause` (official) and fork `enum skill_fail_reason`; client `ragnarok-packets` `SkillFailReason::from_wire`, test `wire_reasons_match_the_server_enum` |
| Skills | `SM_BASH` id 5, `AC_OWL` id 43 (passive, `status.c` ~1937 `dex += skill_lv`), `AL_BLESSING` id 34; `docs/skills.json` is the name→id source |
| Audio | `settings/audio.rs` `AudioSettings { mute_on_focus_loss, ui_sound_enabled, ui_sound_volume }` (~17); `korangar-audio/src/lib.rs` already exposes `set_background_music_volume` (~288), `set_sound_effect_volume` (~296), `set_spatial_sound_effect_volume` (~304) |
| Keys | `input/mod.rs`: `Escape` → `ToggleMenuWindow` (~303); `Alt+E` inventory (~307); `Alt+V` character overview (~311); `Ctrl+I` interface settings (~374) |
| Obstacle overlay | `world/map/mod.rs` `render_overlay_tiles` (~823, `#[cfg(feature = "debug")]`), `render_entity_pathing` (~855, debug), `is_walkable` (~1109) |
| DM quests | `Hercules/npc/custom/dm_campaign/shared/dm_console.txt` `S_Quest` (~1124) → `DM_InstanceQuestStart/Complete/Erase` (party-wide); `S_Exp` (~776) → `DM_PartyExp`; `S_Mode` (~795); `S_Status` (~743); `dm_checkpoint.txt` `DM_CheckpointRememberParty` (~19), `DM_CheckpointSyncParty` (~156) |
| Culvert gate | `Hercules/npc/quests/quests_prontera.txt` `Recruiter` at `prt_in,88,105` (~53) sets `MISC_QUEST \|= 8` (~126); `Culvert Guardian` at `prt_fild05,270,212` (~145) checks `MISC_QUEST & 8`; DM arc quest `20001` is a separate tracker |
| Mob info | `Hercules/conf/map/battle/monster.conf` `show_mob_info: 0` (~194) |
| EXP rate | `conf/map/battle/exp.conf` `base_exp_rate`/`job_exp_rate` = 100; no `@dmexprate` exists |
| Headless | `korangar-networking/examples/headless-tester` scenarios `shop-buy-sell`, `shop-close`, `vendor-double-transaction-classification`; runner `tools/testing/run-suite.sh --scenario <name>` |

## Task completion protocol

Append this record directly below the card before advancing:

```text
Status: DONE | IN PROGRESS | BLOCKED
Changed: <paths and behavior, or "none">
Evidence:
- Source-confirmed: <symbol and conclusion>
- Automated-verified: <exact command and result>
- Observed: <live steps and result, when required>
Not verified: <remaining boundary, or "none">
Blocker: <only for BLOCKED>
Next: <task ID>
```

## Validation ladders

Use the ladders in `qwen3-playtest-runbook.md` § "Standard validation ladders"
verbatim (Korangar: fmt → narrow test → `cargo check -p korangar` →
`cargo test -p korangar --lib` → clippy → live; Hercules: narrow check →
`make -j4` → `./dev.sh restart && ./dev.sh wait` → named scenario → log
inspection). For any card touching `korangar-interface`, also run
`cargo test -p korangar-interface`.

Live GUI checks in this runbook mean: start the stack (`Hercules/dev.sh start
&& ./dev.sh wait`), launch `korangar/target/release/korangar` from
`korangar/korangar/`, perform the steps, and attach either a screenshot path
or the exact `client_log!` lines. "It compiled" is never live evidence.

## Stage checkpoints

| After | Record | Continue with |
|---|---|---|
| PT-002 | `SR-0` | PT-010 |
| PT-013 | `SR-1` | PT-020 |
| PT-025 | `SR-2` | PT-030 |
| PT-035 | `SR-3` | PT-040 |
| PT-047 | `SR-4` | PT-050 |
| PT-052 | `SR-5` | PT-060 |
| PT-069 | `SR-6` | PT-070 |
| PT-075 | `SR-7` | PT-080 |
| PT-084 | `SR-8` | PT-090 |
| PT-092 | `SR-9` | stop: queue complete; pack upload still requires authorization |

A checkpoint is a packet for the senior developer (`git status -sb`,
`git diff --stat`, the stage's evidence records, exact passing/failing
commands, remaining live/human boundaries, proposed first card of the next
stage). It is not a request for permission.

---

## Stage 0 — baseline and instrumentation

- [ ] **PT-000 — record the execution baseline**

  Record: `git rev-parse HEAD` and `git status -sb` in both repositories; the
  `korangar` release binary mtime and the Hercules `map-server` mtime versus
  the newest source commit (both were rebuilt 2026-09-20 — confirm nothing has
  changed since); `tools/packaging/PACK_VERSION`; the running
  `client_version_to_connect`; MariaDB reachable with the `ragnarok` user.
  Write the result as the first evidence record so every later "Observed"
  line can cite these hashes.

  **Done when:** the record names both HEADs, confirms both binaries are
  current, and lists any pre-existing dirty files as not-owned-by-this-run.

- [ ] **PT-001 — instrument which window consumes a click**

  Add a diagnostic behind an environment variable (follow the existing
  `KORANGAR_PACKET_LOG` pattern; call it `KORANGAR_UI_LOG`) that prints, for
  every mouse click while the interface is hovered: the current `self.windows`
  order (index, id, `WindowClass`), the window whose layout reported the hover
  (`hovered_window` in `lay_out_windows`), and whether `InterfaceFrame::click`
  reported `handled`. This is read-only instrumentation; no behavior change.

  Then reproduce **both** P0 window reports with it running:
  1. Character select → click an empty slot → creation window. Record whether
     the creation window is last in the stack and whether the Create button's
     window receives the click.
  2. Talk to any NPC shop (e.g. `@warp alberta_in 188 21`, Weapon Dealer#alb)
     → `+1` on an item → `Buy` in the Cart window. Record which window
     consumed the `+1` and the `Buy` clicks and whether `BuyItemsPacket` was
     sent (`KORANGAR_PACKET_LOG`).

  **Done when:** both reproductions have a captured trace that names the
  window that consumed each click, and the trace either shows the defect or
  proves the click reached the intended window (in which case PT-010 must
  re-target to the element level).

- [ ] **PT-002 — classify the vendor and character-creation defects from the trace**

  Using only PT-001 output, write a one-paragraph classification for each:
  (a) draw order wrong, (b) hit-test order wrong, (c) click reaches the
  window but the element ignores it, (d) something else. State the exact
  symbol that will be changed in PT-010. Do not implement here.

  **Done when:** both classifications cite trace lines, and PT-010's target
  symbol is named.

### Checkpoint SR-0 → continue with PT-010

## Stage 1 — window stack correctness (P0)

- [ ] **PT-010 — fix the window-stack defect named by PT-002**

  Implement the fix at the symbol PT-002 named. Whatever the mechanism, the
  invariant to establish and test is: *the most recently opened or most
  recently clicked window is both drawn last and hit-tested first.* If body
  clicks currently never emit `MoveWindowToTop` (only titlebar drags do),
  decide explicitly whether a body click should raise a window; if it should,
  that is part of this card. Add a `korangar-interface` unit test with two
  overlapping windows that fails before the fix: open A, open B over it, click
  inside the overlap, assert B receives the click and B is last in the render
  order.

  **Done when:** the new test fails on the pre-fix tree and passes after;
  `cargo test -p korangar-interface` and `cargo test -p korangar --lib` pass;
  PT-001's two traces re-run and show the intended window consuming both
  clicks.

- [ ] **PT-011 — character creation opens on top and the Create click lands**

  Verification card for PT-010 (no second fix). Live: open creation from an
  empty slot, type a 4+ character name, click Create. Confirm
  `CharacterCreationWindow` is drawn over selection and the
  `InputEvent::CreateCharacter` path fires (server log line
  `Created char: account: …`).

  **Done when:** live screenshot shows creation over selection and the server
  log shows the character created without the player moving any window.

- [ ] **PT-012 — character select cannot be clicked off**

  While `WindowClass::CharacterSelection` is open, clicks that land on no
  window must not reach world/map handling, and any window opened from it
  (creation, delete confirm) must open on top. Prefer a small "modal base"
  rule in the interface (a window class that swallows non-window clicks)
  over special-casing in `lib.rs`. Unit test: with the selection window open,
  a click on empty screen produces no `PlayerMove`/`PlayerInteract`.

  **Done when:** the test passes and a live check shows clicking beside the
  slots does nothing visible.

- [ ] **PT-013 — creation window explains a disabled Create button**

  `MINIMUM_NAME_LENGTH` is 4 and the only feedback today is a hover tooltip.
  Show a persistent inline line under the name box ("Name needs at least 4
  characters") while the button is disabled, and play the rejection sound on
  a click of the disabled button. This closes the Baxter report.

  **Done when:** live: a 3-character name shows the line and a click makes the
  rejection sound; a 4-character name hides the line and Create works.

### Checkpoint SR-1 → continue with PT-020

## Stage 2 — vendor purchase and quantity controls (P0/P1)

- [ ] **PT-020 — vendor purchase works end to end in the GUI**

  Verification card for PT-010 (no second fix unless PT-002 classified the
  vendor defect as (c) or (d), in which case the fix belongs here and must
  name its symbol). Live: NPC shop → `+1`, `+10`, `+100` each add to the cart
  and the cart total updates → `Buy` sends `0x00C8` → `0x00CA` result → Zeny
  decreases → item appears in inventory → both shop windows close.

  **Done when:** `KORANGAR_PACKET_LOG` shows `0x00C8` then `0x00CA`, the
  inventory and Zeny changes are visible, and
  `./tools/testing/run-suite.sh --scenario shop-buy-sell` still passes.

- [ ] **PT-021 — the quantity dialog shows its number**

  Source-confirmed: `QuantityWindow::new(_quantity_path)` ignores the path
  and the window has no readout, so `−`/`+`/`All` change an invisible value.
  Add a `text!` bound to the chooser's current value, disable `−` at 1 and
  `+` at the maximum, and keep keyboard entry. Reuse the existing
  `state/quantity.rs` tests; add one asserting the displayed string tracks
  `increment`/`decrement`/`set_all`.

  **Done when:** live: "Drop amount…" and "Add to trade…" both show the
  number changing on every press, `All` shows the stack size, and Confirm
  drops/offers exactly that amount (packet `0x0363` for drop, `0x00E8` for
  trade).

- [ ] **PT-022 — `+10` / `+100` clamp to what is available instead of disabling**

  In `sell.rs` `disabled_cutoff` (and `buy.rs` for finite-stock shops), a
  press of `+N` with fewer than `N` available should add the remainder, not
  be disabled. Keep the button disabled only when nothing remains. Unit test
  the clamp in the click handler.

  **Done when:** live: selling a stack of 7 with `+10` adds 7; with 0 left
  the button is disabled; NPC buy `+100` still adds 100.

- [ ] **PT-023 — unusable (red) equipment can still be traded and dropped**

  The class-eligibility tinting from commit `2a8ec1c9` is correct, but the
  report says trade and drop are refused for a red shield. First locate the
  actual gate: check the item-actions menu, the trade add path
  (`try_send_trade_add`), and the drop path for any eligibility check; also
  capture the server response (`0x00EA` trade-add result, drop ack) to rule
  out a server refusal. Fix only the layer proven to refuse. Equip must remain
  refused.

  **Done when:** live: a Guard on an Acolyte (`#item BigZ Guard 1` — Guard is
  wearable, so use a class-locked item such as `Boots`, which excludes
  Acolyte) can be dropped and offered in trade, and equipping it still fails
  with the existing message.

- [ ] **PT-024 — inventory order survives a full inventory re-send**

  `Inventory::fill` replaces the list, discarding any `reorder_display`
  order, and Hercules re-sends the full inventory after trade/vending
  completion. Reproduce the "keeps rearranging" report by trading, then fix
  by re-applying the last display order (by item id + unique id) after
  `fill`. Unit test: reorder, `fill` with the same items in server order,
  assert the display order is preserved; new items append at the end.

  **Done when:** test passes and live: reorder two items, complete a trade,
  order is unchanged.

- [ ] **PT-025 — auto-loot exclusion list**

  Add a per-item exclusion to the existing area-loot/auto-pickup path
  (`state/area_loot.rs`): an item id set in `GameSettings`, editable from the
  item-actions menu ("Never auto-loot") and the game settings window. Excluded
  items still show on the ground and can be clicked.

  **Done when:** live: mark Jellopy as excluded, kill a Poring, Jellopy stays
  on the ground while other drops are picked up; the setting persists across
  relog.

### Checkpoint SR-2 → continue with PT-030

## Stage 3 — quest tracking, breadcrumbs, and late-join sync (P0)

- [ ] **PT-030 — the Tracked Objective window can always be reopened**

  Add `InputEvent::ToggleTrackedObjectiveWindow` (Menu entry, following
  `ToggleQuestLogWindow` in `menu.rs`), and make `ToggleQuestTracking` open
  the window if it is closed before refreshing. Decide and record whether the
  window stays `closable: true` (with the Menu toggle) or becomes
  non-closable like Chat/Hotbar; the plan recommends keeping it closable now
  that it can be reopened.

  **Done when:** live: close the tracker with `X`, reopen it from Menu; close
  it again, track a quest from the Quest Log, it reopens showing that quest.

- [ ] **PT-031 — a tracked quest appears in the tracker**

  Reports: "clicking track quest doesn't seem to do anything" and "quests
  don't pop up in quest tracker". With PT-030 done, reproduce with the
  window open: track `20001` from the Quest Log and confirm
  `refresh_tracked_quest_display` produces visible text. If it does not,
  trace `QuestLogState::track` → HUD state → `tracked_objective.rs` render
  and fix the broken link. Check the 2026-09-18 memory note ("quest
  track/untrack not refreshing HUD") for whether an earlier fix landed and
  regressed.

  **Done when:** live: tracking shows the quest title and objective in the
  tracker within one frame; untracking clears it.

- [ ] **PT-032 — breadcrumb guidance renders for a tracked quest**

  Verification-first: with PT-030/031 done, enable `ToggleBreadcrumbGuidance`
  for a tracked quest whose target NPC/map is known (`20001` → Prontera
  fountain / Tibbets at `prontera 146 193`). If no route or marker appears,
  this becomes a fix card: trace the breadcrumb state → `navigation` →
  render path and name the symbol.

  **Done when:** live screenshot shows a visible guidance element for the
  tracked objective, or the record proves guidance is rendered and the
  report was the closed-tracker symptom.

- [ ] **PT-033 — late-joining party members receive the active quests**

  Hercules. When DM mode is on (`$dm_mode`, `$dm_active_party`) and a
  character joins that party or logs in already in it, replay
  `DM_InstanceQuestStart` for every arc quest currently `in progress` for the
  party leader (or the checkpoint's recorded quest set — see
  `DM_CheckpointSyncParty`). Hook points: the party-join event and
  `OnPCLoginEvent` in `dm_session.txt`. Do not grant `complete` states; only
  `in progress`. Log one line per replayed quest.

  **Done when:** two-client or headless: start DM mode with party A holding
  `20001` in progress; a fresh account creates a character, joins A, and
  `@dm status` on the new character shows `A01:1` without the DM running
  `@dmquest`.

- [ ] **PT-034 — the Culvert entrance honors the campaign quest**

  Two independent gates exist today (stock `MISC_QUEST & 8` via the Recruiter;
  campaign `20001`). Decision inside this card, recorded as source-confirmed
  options: (a) `DM_InstanceQuestStart(20001)` also sets `MISC_QUEST |= 8` for
  each member; (b) the campaign's own culvert route bypasses the stock
  Guardian. Recommend (a) — smallest change, keeps the stock NPC intact.
  Implement the chosen option.

  **Done when:** live: a fresh party given `@dmquest start 20001` can walk
  through the Culvert Guardian without visiting the Recruiter.

- [ ] **PT-035 — Plankton turn-in completes or says who is missing the quest**

  Reproduce the "takes 6 Plankton but doesn't complete" report with one
  party member lacking `20001`/the hunt quest. Make the turn-in check
  per-character and, when it cannot complete, print exactly which member
  lacks which quest instead of silently taking the items. Items must not be
  consumed unless the completion succeeds.

  **Done when:** headless or two-client: with one member missing the quest,
  the items remain and the message names the member; with all members
  holding it, the quest completes for all.

### Checkpoint SR-3 → continue with PT-040

## Stage 4 — combat feedback and targeting (P1)

- [ ] **PT-040 — an armed skill tells the player how to cancel**

  The fizzle-and-stay-armed behavior at `lib.rs` ~11451-11486 is intentional,
  but there is no affordance. On the first fizzle of an armed skill, show a
  short HUD line ("No target — right-click or Esc to cancel") for ~2 s, and
  change the cursor state so an armed skill is visually distinct from the
  default cursor. Do not auto-cancel.

  **Done when:** live: press Bash with nothing hovered, click empty ground,
  the hint appears; right-click clears the reticle and the hint.

- [ ] **PT-041 — Bash has a visible skill effect**

  `SM_BASH` (id 5) is registered and fires; it lacks a distinct visual. Use
  the documented reverse-engineering method (roBrowserLegacy effect table →
  GRF path probe → correct loader) to map its hit effect in
  `world/skill_recipe.rs`. Add the failing-before test the method requires
  (wrong id/path/loader fails).

  **Done when:** live: Bash on a Poring shows the mapped effect and a normal
  attack does not.

- [ ] **PT-042 — Owl's Eye gives learn/level feedback**

  `AC_OWL` is passive (no cast is expected). Confirm the skill-tree window
  shows its level and DEX bonus, and that leveling it prints a chat line.
  If the report meant a missing cast animation, record that passives have
  none and close with the tree/chat evidence.

  **Done when:** live: raising Owl's Eye by one level shows the new level in
  the tree and a chat/HUD acknowledgment.

- [ ] **PT-043 — no `[Status-] Lost #65535` lines**

  `65535` is Hercules `SI_BLANK` (-1) on the wire: a status ending that has
  no icon. In `combat_chat.rs` (or at the `StatusChange` event boundary),
  skip logging status changes whose index is `u16::MAX`. Unit test with a
  synthesized end event.

  **Done when:** test passes and a live buff expiry that previously logged
  `#65535` logs nothing (or the real name if one exists).

- [ ] **PT-044 — Steal failure cause 10 has text**

  Look up official `useskill_fail_cause` value 10 in Hercules `clif.h` and
  the fork `skill_fail_reason` enum. If 10 is an official cause with no
  client text, add the text in the client cause table; if the site in
  `skill.c` sends cause 0 with a fork reason, add the reason. Keep
  `wire_reasons_match_the_server_enum` in step.

  **Done when:** a failed Steal prints a human-readable reason and the enum
  parity test passes.

- [ ] **PT-045 — decide: resume auto-attack after a targeted skill**

  Source-confirmed: Hercules stops the attack loop for every non-combo skill
  (`unit_skilluse_id2`: `unit->stop_attack`). The report ("Steal stops me
  from attacking") is official behavior. Write the decision options: (a)
  leave as is; (b) client re-issues `AttackTarget` on the same target after a
  targeted skill's result arrives, behind a `GameSettings` toggle. Recommend
  (b). Record `BLOCKED: awaiting DM decision`; implement in a child card if
  approved.

  **Done when:** the decision record exists with both options and a
  recommendation.

- [ ] **PT-046 — support skills target reliably**

  Three sub-checks, each with its own evidence: (1) Tab-cycle includes party
  members when a `SkillType::Support` skill is armed; (2) find why a
  party-member click sometimes does not update `last_target` (log
  `resolve_skill_activation` inputs live); (3) add an explicit self-target
  action (hotkey + party window "Me" button) so Heal/Blessing on self does
  not depend on the no-target fallback.

  **Done when:** live: Heal cast with a party member hovered heals them;
  with Tab-selected member, heals them; with the self key, heals self;
  each shows the packet target id in `KORANGAR_PACKET_LOG`.

- [ ] **PT-047 — party HUD: select member by 1–4, target self**

  Feature on top of PT-046: number keys 1–4 (when not typing) select party
  members in roster order for the next support cast; a "self" key selects
  the player. Show the selected member highlighted in the Party window.

  **Done when:** live: pressing 2 then casting Blessing buffs the second
  roster member; pressing the self key then casting buffs the player.

### Checkpoint SR-4 → continue with PT-050

## Stage 5 — movement (P1)

- [ ] **PT-050 — rubber-banding reproduction with spacebar attack**

  Fold into the QW-020/QW-023 capture format: LAN and Tailscale, measured
  ping, WASD walk, then spacebar attack mid-walk, packet log of `0x035F`
  moves and server position corrections. Record whether the correction
  follows the attack request specifically.

  **Done when:** the record has before/after positions and packet excerpts for
  both link types and names whether the attack request is the trigger.

- [ ] **PT-051 — fix the movement correction named by PT-050**

  Only if PT-050 proved a client-side cause (e.g. the client keeps walking
  after sending an attack that the server treats as a stop). Otherwise record
  the server-side cause and the config knob.

  **Done when:** re-running PT-050's steps shows no correction, or the record
  proves the remaining correction is network latency.

- [ ] **PT-052 — pathing around environment**

  Capture three concrete failing routes (map, from, to, screenshot of the
  path taken). Compare the client `PathFinder` route to the server's
  accepted route. Fix the client path only where they disagree; if the
  server path is also bad, record it as a map-data issue.

  **Done when:** all three routes are recorded and each is either fixed with
  a test on the path result or classified as server/map data.

### Checkpoint SR-5 → continue with PT-060

## Stage 6 — settings, keys, and UI polish (P3)

- [ ] **PT-060 — separate music, effects, and master volume**

  `AudioSettings` has only UI-sound fields; the engine already exposes
  `set_background_music_volume`, `set_sound_effect_volume`, and
  `set_spatial_sound_effect_volume`. Add `master_volume`, `music_volume`,
  `effect_volume` (serde defaults 1.0), three sliders plus the existing UI
  click toggle in the audio settings window, and apply them at startup and on
  change. Persist.

  **Done when:** live: music slider to 0 silences BGM while an attack sound
  still plays; effects slider to 0 does the reverse; values survive relog.

- [ ] **PT-061 — Escape closes the top window; `I` opens inventory**

  `Escape` currently toggles the Menu (`input/mod.rs` ~303). Change to: if
  a skill is armed, cancel it (existing); else if a closable window is open,
  `close_top_window`; else toggle Menu. Add unmodified `I` → inventory,
  guarded by "not typing in chat" (reuse whatever guard WASD uses). Keep
  `Alt+E`.

  **Done when:** live: Esc closes Inventory, then Character Overview, then
  opens Menu; `I` toggles inventory; typing "I" in chat does not.

- [ ] **PT-062 — Character Overview minimize/toggle button**

  `Alt+V` already toggles it; add a visible minimize control on the window
  itself and a HUD button, since the report implies the hotkey is unknown.

  **Done when:** live: the button collapses/expands the window.

- [ ] **PT-063 — lock the skill bar**

  A `GameSettings` toggle (and a padlock button on the hotbar) that disables
  drag-out/reorder of hotbar slots while locked; presses still work.

  **Done when:** live: locked bar ignores drag; unlocked bar drags as before.

- [ ] **PT-064 — inventory categories and an Equipped tab**

  After PT-024. Tabs: All / Equipped / Equipment / Consumables / Etc, using
  the item type already carried by `InventoryItem`. Manual order persists per
  tab.

  **Done when:** live: equipped items appear only in Equipped; switching tabs
  keeps order.

- [ ] **PT-065 — minimap zoom steps and UI scale to 400%**

  Add two more minimap zoom levels; raise the interface-scaling cap to 4.0
  and verify windows do not exceed the screen at 400% on a 1920×1080 target.

  **Done when:** live at 400%: Inventory, Menu, and Chat remain fully on
  screen.

- [ ] **PT-066 — rename "Save here" to "Set Respawn Point"**

  Copy change in the dialog/menu that offers Kafra/inn save.

  **Done when:** `rg "Save here"` in `korangar/src` returns nothing and the
  new label is on screen.

- [ ] **PT-067 — world map availability**

  Determine whether any world-map window exists client-side. If not, record
  it as a feature with an estimated scope (the reference client draws
  `data\texture\worldmap` assets); do not implement here.

  **Done when:** the record states existence or scope.

- [ ] **PT-068 — monster level and HP display**

  Server: `show_mob_info` in `monster.conf` (bitmask: level/HP/id) is `0`.
  Set it to show level and HP, `@reloadbattleconf`, and confirm the client
  renders the name suffix. If the client strips it, add display.

  **Done when:** live: a Poring shows its level and HP in its name or a
  tooltip.

- [ ] **PT-069 — character lights**

  Define the request with the user (a point light following the player? a
  glow on party members?) and record scope. Do not implement without the
  definition.

  **Done when:** the record has the definition or `BLOCKED: awaiting
  definition`.

### Checkpoint SR-6 → continue with PT-070

## Stage 7 — new features

- [ ] **PT-070 — player-facing obstacle overlay**

  `render_overlay_tiles` and `is_walkable` already exist behind
  `#[cfg(feature = "debug")]`. Add a non-debug overlay path that draws only
  unwalkable cells in a single translucent color (default orange), toggled by
  `InputEvent::ToggleObstacleOverlay` (Menu entry + a hotkey), default off,
  not persisted. Keep the debug palette untouched.

  **Done when:** live in a release build: the toggle shows blocked cells
  around a Prontera building and hides them again; frame rate impact is
  recorded.

- [ ] **PT-071 — regenerating charges: design record**

  Write `docs/specs/regen-charges.md` from the plan's description and the
  user's answers to the open questions (restore amount, `level/4` rounding
  with minimum 1, shared vs. separate pools, resting rule). Decide the wire
  shape: a fork status (`SC_`) carrying remaining charges + a fork skill for
  each pool, following the `SC_LANDPROTECTOR` five-touch-point pattern in
  `korangar/CLAUDE.md` §3b. `BLOCKED` until the user answers.

  **Done when:** the spec exists with every open question answered or the
  card is `BLOCKED` naming the unanswered ones.

- [ ] **PT-072 — regenerating charges: server**

  Implement per the spec: two skills (HP charge, MP charge) granted to every
  player, charge counter in a status value, 20 s per-charge regen timer,
  full refill on sit for the spec's duration, use restores the spec's
  amount. Headless scenario `regen-charges` proving counts and timers.

  **Done when:** the scenario passes and `@skillid` shows both skills.

- [ ] **PT-073 — regenerating charges: client**

  Hotbar-visible icons (red/blue potion art from the GRF via the sprite
  loader), a charge-pip readout bound to the status value, and the use packet
  wired through the existing skill-use path.

  **Done when:** live: pips decrement on use, refill over 20 s, refill fully
  after sitting; HP/SP change by the spec's amount.

- [ ] **PT-074 — regenerating charges: live party check**

  Two clients: both see each other's pips; charges persist across relog.

  **Done when:** recorded with both clients' screenshots.

- [ ] **PT-075 — `@dm exprate` (if approved in PT-082)**

  A session multiplier applied inside `DM_PartyExp` only (not
  `base_exp_rate`), settable by `@dm exprate <percent>`, shown by
  `@dm status`, reset by `@dm mode off`.

  **Done when:** `@dm exprate 200` then `@dm exp 1000` grants 2000 to each
  member (headless or two-client), and the stale `@dmexprate` mention in
  `docs/DM_INTERFACE.md` is corrected to the real syntax.

### Checkpoint SR-7 → continue with PT-080

## Stage 8 — DM decisions (P2, no code)

Each card ends `BLOCKED: awaiting DM decision` with a written recommendation.

- [ ] **PT-080 — ammo policy for bows**

  Options: (a) keep ammo; (b) `@item Arrow` grants stay as a convenience and
  the fork removes ammo consumption (`battle_config.arrow_decrement` exists);
  (c) unlimited base arrows only. Note (b) is one config key.

- [ ] **PT-081 — regeneration policy**

  Options for "×3 while walking" and "higher for casters": `conf/map/battle/
  player.conf` `natural_heal_*` keys versus a fork status. Note the earlier
  finding that overweight suppresses regen entirely.

- [ ] **PT-082 — `@dm exprate` scope**

  Multiply only `@dm exp` grants (recommended, see PT-075) or also monster EXP
  during a session.

- [ ] **PT-083 — teleport all players to town**

  `@partyrecall` covers the party. Decide whether a server-wide recall is
  wanted (`@recallall` exists in stock Hercules and is admin-only).

- [ ] **PT-084 — quest system overview document**

  After Stage 3 is done, write a player/DM-facing quest overview (tracking,
  sharing, late-join, the two Culvert gates). Blocked until PT-033/034 land.

### Checkpoint SR-8 → continue with PT-090

## Stage 9 — release

- [ ] **PT-090 — bump the pack version and rebuild both packs**

  `tools/packaging/PACK_VERSION` → today's date; `make-pack.sh --os windows
  --build --merged --skip-assets` and `--os macos …`; confirm the
  `client_version_to_connect` import line updates; restart the login server.
  Record hashes of all six zips.

  **Done when:** both merged and both update zips exist with the new version
  inside, and a client from the new pack logs in.

- [ ] **PT-091 — decide whether to enforce pack version at login**

  `check_client_version` is `false`. Record the trade-off (old packs refused
  vs. friends locked out until they update) and `BLOCK` on the user's call.

- [ ] **PT-092 — release report**

  Summarize every `DONE`/`BLOCKED` card, the live checks performed, and what
  remains for a human. Uploading packs requires authorization and is not
  performed by the executor.

  **Done when:** the report is written and every unchecked box above has a
  stated blocker.

### Checkpoint SR-9 — queue complete

## Coverage rule

Every line of the 2026-09-20 raw feedback maps to at least one card above.
If a future report arrives that maps to none, add a card; do not fold it into
an existing card's scope silently.
