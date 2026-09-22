# Playtest adjustment plan — 2026-09-20

> Triaged from the raw DM feedback dump given 2026-09-20. This is a planning
> document only — no implementation has started from it. The execution order,
> per-card `Done when` gates, symbol anchors, and evidence protocol live in
> [`playtest-runbook-2026-09-20.md`](playtest-runbook-2026-09-20.md); work
> from its `NEXT` pointer, not from this document. Same discipline as the
> [`playtest-adjustments-2026-09-12.md`](playtest-adjustments-2026-09-12.md) /
> [`qwen3-playtest-runbook.md`](qwen3-playtest-runbook.md) pair.

## Already investigated this session (source-confirmed, not yet fixed)

Three items on the raw list were already run down live during this session's
DM support. Re-litigating them from scratch would waste a card — start from
these findings.

- **Vendor Buy button does nothing** (also covers "+1/+10/+100 buy buttons
  don't work"). `korangar/src/interface/windows/buy.rs` (add-to-cart) and
  `buy_cart.rs` (`Buy`/`Cancel`) both wire correctly in isolation —
  `BuyCartWindow` opens strictly after `BuyWindow` in `lib.rs:7374-7376`, so
  window-stack order looks right on paper. Investigation was interrupted
  before finding the actual fault. Prime suspect given the *other* confirmed
  z-order bug below: a click hit-test/draw-order mismatch specific to how
  these two windows overlap, not the input wiring itself. Needs a live click
  trace (log which window/element actually receives the click) before
  guessing further.
- **Character Select drawn behind other windows** — user found this live:
  the Character Creation window opened *behind* Character Selection, "gets
  the sound and then nothing" for Baxter. `CharacterSelectionWindow` and
  `CharacterCreationWindow` are both ordinary entries in the same
  `self.windows` stack (`korangar-interface/src/lib.rs`), opened via the same
  `open_window` path used everywhere else — so this is very likely the same
  underlying z-order/hit-test bug as the vendor issue above, not two separate
  defects. **Fix these two together; do not treat as unrelated cards.**
- **Culvert quest doesn't complete / "takes 6 Plankton but doesn't
  complete"** — root cause for the *related* "Culvert Guardian still says I
  need to register" report was found and is NOT a quest-completion bug: the
  physical entrance NPC (`prt_fild05` "Culvert Guardian") checks the stock RO
  flag `MISC_QUEST` bit 8, granted only by the "Recruiter" NPC at
  `prt_in (88,105)`. The DM campaign's own `20001`/culvert story quest is a
  **separate, unrelated tracker** (`@dmquest`/`DM_InstanceQuestStart`). The
  Plankton-count report is most likely the same class of bug as "quests
  don't sync to new accounts" below: some party members never had the
  official flag or the DM quest set, so a party-scoped completion check
  (`questprogress` per-member) silently never satisfies. Needs: confirm
  which of the 6 party members are missing which of the two systems before
  writing a fix.

## Priority order

| Priority | Meaning | Items |
|---|---|---|
| P0 | Blocks a session outright | Vendor Buy (nothing to buy = no gearing), Character Select z-order (blocks new-player onboarding), Quest Tracker can't reopen, quests not syncing to new/late-joining party members |
| P1 | Frequent friction during play | Rubberbanding/pathing, inventory rearranging, trade +/- and drop +/- buttons, tab-target for heals/buffs, red/untradeable-item drop bug, Owl's Eye and Bash missing feedback, Steal-interrupts-attack |
| P2 | Design/balance decisions needed from the DM, not bugs | No-ammo-for-bows, unlimited base ammo, mana regen multipliers, `@dm exp rate`, teleport-all-to-town |
| P3 | Polish, UX niceties, settings | Audio split (music/effects), click-ding toggle, ESC-to-close, hotkeys (I for inventory), inventory tabs/categories, minimap zoom, UI scaling, character lights, world map, monster HP/level display |
| New | Net-new features, not bugs | Obstacle/collision overlay toggle, regenerating HP/mana charge system |

## P0 — session-blocking

- [ ] **Vendor Buy button.** Start from the investigation note above — trace
      an actual click live (which window/element receives it) rather than
      re-reading the code cold.
- [ ] **Character Select / Character Creation z-order.** Same root cause
      class as the vendor bug above — investigate together. Check whether
      `korangar-interface`'s window stack has *any* click-to-front
      (`MoveWindowToTop`) trigger on ordinary body clicks, or only on
      titlebar drag; if body clicks never reorder the stack, two
      always-open windows that overlap can end up stuck in the wrong visual
      order relative to hit-testing.
- [ ] **Quest Tracker can't be reopened once closed.** Confirmed: zero menu
      entry anywhere reopens `WindowClass::TrackedObjective` — it only opens
      once, automatically, on entering the world (`lib.rs:4871`). Re-tracking
      a quest from the Quest Log (`ToggleQuestTracking`, `lib.rs:10164`)
      updates state but never calls `open_window`. Minimum fix: add a Menu
      toggle (`ToggleTrackedObjectiveWindow`, matching the pattern of
      `ToggleQuestLogWindow` etc. in `menu.rs`). Bigger fix: make it
      non-closable, since it's meant to be always-on HUD like Chat/Hotbar/HUD.
- [ ] **Quests don't sync to new accounts / late-joining party members.**
      Two new players (from this session — see the Chris/Baxter/registration
      thread) never got the active quest. This is very likely the same
      mechanism gap behind the Culvert Plankton report above: nothing
      currently re-syncs quest state to a party member who joins (or is
      created) after the DM already started the session/quest for the
      original party. Look at `DM_CheckpointSyncParty` /
      `DM_CheckpointRememberParty` in `dm_checkpoint.txt` — party-sync
      exists for checkpoints; confirm whether an equivalent exists (or is
      missing) for plain quest state.
- [ ] **New party member's quest dialogue goes nowhere ("misaligned
      quests?").** Same investigation as above — likely one symptom of the
      same missing late-join reconciliation, not a separate dialogue bug.
- [ ] **Quests don't pop up in the Quest Tracker** (added 2026-09-20, second
      pass). Distinct repro detail from the reopen bug above — this is about
      a quest never appearing as trackable at all, not the window being
      closed. Rule out the reopen bug first (is the Tracked Objective window
      even open when this happens?); if it's open and still doesn't show a
      newly-granted quest, the gap is in whatever should auto-track (or make
      trackable) a quest the moment it starts, not in `ToggleQuestTracking`
      itself.
- [ ] **No breadcrumbs to guide players** (added 2026-09-20, second pass).
      Surprising given how much breadcrumb machinery already exists client-
      side — `ToggleBreadcrumbCollapsed`, `ToggleBreadcrumbHidden`,
      `BreadcrumbScale`, `BreadcrumbOpacity`, `ToggleBreadcrumbGuidance` are
      all wired in `lib.rs`/`input/event.rs`, and the "Show Breadcrumb"
      button lives inside the Tracked Objective window (confirmed on-screen
      earlier this session). Two live hypotheses, not yet distinguished: (a)
      purely a symptom of the two bugs directly above — no trackable quest
      and/or a closed, unreopenable tracker window means there's nothing to
      ever show a breadcrumb for; (b) breadcrumb guidance is toggled on but
      genuinely fails to render/route. Check (a) first — it's the cheaper
      explanation and both its root causes are already P0 items above.

## P1 — frequent friction

- [ ] **Rubberbanding** (WASD movement, and specifically "using spacebar to
      attack"). 2026-09-12's plan already flagged internet WASD correction as
      "needs a live reproduction" — this report adds spacebar-attack as a
      trigger, worth folding into the same reproduction pass rather than a
      new card.
- [ ] **Pathing around environment is terrible.** Needs a concrete map +
      obstacle repro before touching the pathfinder.
- [ ] **Inventory keeps rearranging** ("not sure if it's after a trade or
      what"). Get a tight repro: does a trade, a sale, a drop, or a pickup
      trigger it? Client-side sort-on-mutation bug is the leading guess but
      needs the trigger nailed down first.
- [ ] **Trade +/- buttons don't work** and **Drop amount +/- and "All" don't
      work** — **one bug, source-confirmed while writing the runbook.** Both
      flows open the same `QuantityWindow`
      (`korangar/src/interface/windows/quantity.rs`). Its `−`/`+`/`All`
      buttons do fire real events (`lib.rs` ~7905-7917) and do mutate the
      `QuantityChooser`, but `QuantityWindow::new(_quantity_path)` ignores
      the path and the window has **no element that displays the current
      number** — so every press changes an invisible value. To a player that
      is indistinguishable from "the buttons do nothing." QW-040's own record
      already flagged "Graphical dialog layout" as not verified. Fix is
      PT-021: bind a readout, disable `−` at 1 and `+` at max.
- [ ] **Steal stops auto-attack — this is official server behavior, not a
      bug** (also found while anchoring). `unit_skilluse_id2` in Hercules
      `unit.c` calls `unit->stop_attack(src)` for every non-combo skill
      ("Stop attack on non-combo skills"). Moved to a decision card (PT-045):
      an opt-in client "resume attack after skill" toggle is the only fix
      that doesn't diverge from RO rules server-side.
- [ ] **Buy/sell +10 button doesn't work unless you already have 10 items —
      should round up to what's available instead.** This is the
      `disabled_cutoff` gate in `buy.rs` (and presumably its `sell.rs`
      counterpart) — currently a hard disable rather than a clamp. Small,
      well-scoped fix once the Buy button itself works again.
- [ ] **Can't trade or drop a shield because it's "red" (class can't use
      it).** Client is correctly flagging an unusable item (the
      `UnusablePresentation`/red-tint work from `2a8ec1c956`) but appears to
      be blocking trade/drop interactions on it entirely, not just purchase.
      An unusable item should still be tradeable/droppable — only *equipping*
      it should be blocked.
- [ ] **Auto-loot: no way to exclude specific items.** Feature gap, not a
      bug — needs a per-item or per-category auto-loot filter.
- [ ] **Tab-target for heals/buffs; selecting characters doesn't always
      change target; no self-target button.** Multi-part: (1) confirm
      Tab-cycle includes party members for Support-type skills, (2) find why
      a party-member click sometimes doesn't update `last_target`, (3) add
      an explicit self-target affordance (button or hotkey) instead of
      relying on "no target = self" fallback only for some skill types.
- [ ] **Owl's Eye: no skill-use text, no cast animation.** Passive skill (see
      earlier answer in this session) — confirm client isn't expecting a cast
      animation for a passive at all, vs. genuinely missing a "skill learned/
      leveled" acknowledgment.
- [ ] **"Is Bash in the skill database? Looks like a regular attack."** Source-
      confirmed Bash *is* registered (`SM_BASH`, skill id 5, in
      `skill_db.conf`) and mechanically fires — this is almost certainly a
      **missing visual effect**, not a missing skill. Needs its
      `skill_recipe.rs`/effect mapping checked against the classic-effect
      fidelity work already done for other early skills.
- [ ] **`[Skill Fail] Steal: Refused (cause 10)`.** Look up cause 10 in
      Hercules' `useskill_fail_cause`/the fork's `skill_fail_reason` system —
      per the fork's own `cause-0` history, a raw numeric cause without a
      corresponding `skill_fail_reason` mapping means the client can't say
      anything more specific than "Refused." May need a new
      `SKILLFAILREASON_*` entry at the `unit_skilluse` call site for Steal.
- [ ] **`[Status -] Lost #65535`.** Source-confirmed: `65535` is `0xFFFF`,
      which is Hercules' `SI_BLANK = -1` (`src/map/status.h`) on the wire — a
      status *ending* that has no icon at all. `status_name` in
      `korangar/src/state/status_effects.rs` falls back to `#{index}` for any
      id it doesn't know, and `combat_chat.rs` then logs it. Fix (PT-043):
      skip status-change log lines whose index is `u16::MAX`; nothing is
      actually being lost.

## P2 — design decisions for the DM, not bugs

These need a decision from you before any card gets written, not
investigation:

- [ ] No ammo consumption for bows — yes/no?
- [ ] Unlimited base ammo (same question, different phrasing — clarify if
      these are the same request).
- [ ] Mana regen ×3 while walking — new mechanic, or restoring an existing
      RO regen tier?
- [ ] Higher MP regen for casting classes specifically — flat rate bump, or
      class-conditional?
- [ ] `@dm exp rate` command — confirmed not implemented (checked this
      session; the doc index's `@dmexprate` mention is stale). If wanted,
      needs a design call: multiply future `@dm exp` grants only, or a
      session-wide EXP-rate override on top of `base_exp_rate`/
      `job_exp_rate`?
- [ ] Teleport-all-players-to-town command — `@partyrecall` already exists
      for a party; is this meant to be server-wide (every online player,
      any party) instead?

## P3 — polish / settings / nice-to-haves

- [ ] Individual audio sliders: music vs. sound effects (currently one
      combined setting per the "adjust music settings" + "every click makes
      a ding" reports).
- [ ] Toggle/volume for UI click sounds specifically.
- [ ] `ESC` closes the active/topmost window.
- [ ] `I` opens Inventory.
- [ ] Character Overview window: toggle/minimize.
- [ ] Skill bar: lock-in-place option (prevent accidental drag/reorder).
- [ ] Inventory: re-arrange/auto-organize, or a player-chosen sort; category
      tabs; move equipped items to their own tab.
- [ ] Inventory: item-detail view is slow/clunky to use — needs a concrete
      repro of what's slow (open latency? too many clicks to see detail?).
- [ ] Minimap: additional zoom levels.
- [ ] Master volume slider (separate from the music/effects split above).
- [ ] UI scaling up to 300–400% (large-TV use case).
- [ ] Character lights (visual feature, scope TBD).
- [ ] World map — does one exist at all client-side? If not, this is a new
      feature, not a bug.
- [ ] Monster level/HP numbers on hover/target.
- [ ] "Save here" button label → "Set Respawn Point" (pure copy fix).
- [ ] "No cast buffs on enemies" — unclear as written; needs a repro before
      it can even be triaged into a bug vs. a design note. (Possible reading:
      hostile-cast telegraphing/animation is missing when a monster buffs
      itself or an ally.)

## New features (added 2026-09-20, second pass)

- [ ] **Player-facing "show obstacles" overlay button.** Highlight
      unwalkable/blocked tiles (orange or similar) so players can see the
      collision layer instead of guessing from terrain art. This is a
      smaller lift than it sounds: `Map::render_overlay_tiles` in
      `korangar/src/world/map/mod.rs:823-851` already renders exactly this
      grid, colored per tile type — it's just compiled out entirely behind
      `#[cfg(feature = "debug")]`, so it doesn't exist in a normal build at
      all. The work is: (1) decide the player-facing color scheme (debug
      build's tile coloring is a dev palette, not necessarily "orange for
      blocked"), (2) expose a bindable toggle event (hotkey or HUD button,
      not the debug-only path), (3) make sure the overlay is excluded from
      release builds by default unless explicitly toggled on, since drawing
      the full walkability grid at all times would be visual noise most
      players don't want. `is_walkable` (`mod.rs:1109`) is the existing
      per-tile source of truth to color against.

- [ ] **Diablo-4-style regenerating HP/mana charges, alongside existing
      potions.** New system, not a replacement for the current potion items
      — both should coexist per the request ("you can keep potions in
      game"). As described:
      - A skill-bar-style ability (red potion icon = HP, blue = mana) rather
        than an inventory item.
      - Charge count scales with character level — request phrases it as
        "1/4 level," i.e. roughly `level / 4` charges (needs an explicit
        floor/ceiling/rounding decision and a level-1 minimum so it's never
        zero).
      - Each spent charge regenerates on its own ~20 second timer (matches
        "resting for 20 secs will regen them all" — sitting/resting fully
        refills, otherwise each charge individually comes back over 20s).
      - Needs design decisions before implementation: exact restore amount
        per use (flat, %-max, scaling with level?), whether it's two
        separate skills (HP charge, MP charge) or one dual skill, whether
        charges are shared or independent between the HP and MP sides,
        interaction with existing SP/HP regen bonuses and any cooldown/GCD,
        and whether this needs a new client-only UI element (a charge-pip
        indicator next to the hotbar) since Hercules has no native "charges"
        primitive — this would most likely be implemented as a fork-invented
        skill with a custom status (`SC_`) tracking remaining charges and a
        server-side timer, similar in shape to the existing
        `SC_LANDPROTECTOR`/`CZ_CANCEL_CAST` fork deltas documented in
        `korangar/CLAUDE.md` §3b. Scope this as its own multi-card feature,
        not a single bug-fix-sized task.

## Documentation gap (not a bug)

- [ ] **"Quests need a big overview — the mechanics, the sharing, etc."**
      This is a request for a player- or DM-facing document explaining how
      quest tracking, sharing, and party sync are *supposed* to work, not a
      code fix. Worth writing once the P0 quest-sync items above are
      actually fixed — documenting broken behavior as intended would be
      worse than no doc.

## Open questions to resolve before scheduling P2/P3

1. Are "no ammo for bows" and "unlimited base ammo" the same request?
2. Is the mana-regen change a global rate change or specific to walking vs.
   standing (title says "while walking" specifically)?
3. Does "teleport to town for all players" mean *all online players
   server-wide*, or just the active DM party (already covered by
   `@partyrecall`)?
4. Obstacle overlay: what color/style, and does it need its own settings
   toggle (persisted) or is a hold-to-show hotkey enough?
5. Regen charges: flat restore amount or percentage of max HP/MP per charge?
   Exactly how "1/4 level" rounds (floor/ceil, minimum 1 at level 1)? Shared
   or independent HP/MP charge pools? Does resting need to be uninterrupted
   for the full 20s to grant the full refill, or does partial resting give
   partial charges back?
