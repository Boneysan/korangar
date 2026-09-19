# Weekend patch report

## What this patch is

This is the current campaign/gameplay patch candidate from the weekend bug-fix
pass. It improves quest guidance, campaign progression, inventory handling,
recovery, and multiplayer synchronization across the Korangar client and
Hercules server.

The changes are pushed on these branches:

- Korangar: `agent/bump-hercules-pin` — `ebb890fd`, plus an uncommitted
  2026-09-18 live-session pass (UI sound, quest tracking, Tab-target attack;
  see below)
- Hercules: `agent/map-teleport-safety` — `008149be`

## Player-facing improvements

- Quests now show clearer typed objectives: Talk, Defeat, Collect, Explore,
  Interact, and DM encounter steps.
- Tracked objectives can show remaining counts, readable destinations,
  direction/distance guidance, minimap markers, and portal route guidance.
- Campaign parties now retain durable checkpoints through reconnects and
  leave/rejoin flows, with safe reconciliation for offline or ahead-of-state
  members.
- The quest journal refreshes after quest, inventory, trade, party, and map
  changes instead of retaining stale actionable entries.
- Sitting, standing, respawn, overweight, and recovery-blocked states now have
  server-authored recovery status shown to the client.
- Recovery rates are explicit: sitting restores 25% of maximum HP and 25% of
  maximum SP every 10 seconds; respawn restores 50% of maximum HP/SP
  immediately, then fills the remaining 50% over the next 10 seconds.
- Damage cancels the respawn fill. Combat, overweight, blocking status effects,
  and death report a recovery-blocked state instead of silently applying a
  partial tick. Ordinary standing regeneration continues to use the standard
  Hercules rules.
- Inventory, storage, trade, and cart operations use consistent hard-cap weight
  handling, including exact-fit boundary behavior.
- Area-loot selection respects ownership, walkability, weight, slots, combat,
  map changes, and disappearing piles.
- Hotbar clearing, quantity selection, combat feedback, privacy filtering, and
  EXP/quest-award presentation received targeted fixes and test coverage.
- Vendor buy rows now visibly flag equipment that does not apply to the current
  player: the item icon and name are muted/red-tinted, a blocked marker is
  shown, and the tooltip explains the job/level/sex/equipment restriction.
  Purchasing remains available when appropriate; only invalid equip actions
  are blocked.
- The server warp graph and deterministic route foundation are now generated
  from actual warp scripts rather than hand-authored route examples.
- The UI confirm-click sound no longer plays when picking up an item/skill or
  grabbing a window's move/resize handle — only a completed action (a button,
  character select, a quest journal entry) chimes.
- Tracking or untracking a quest from the journal now updates the HUD
  breadcrumb and remembers the choice for next login immediately, instead of
  waiting for an unrelated quest event to refresh it.
- Tab-targeting can now attack: press Space (configurable in Game Settings,
  cycles Space → F → R → Disabled) to attack the currently Tab-selected
  target without clicking it. Deliberately a separate confirm press rather
  than attacking the instant Tab selects, so cycling through monsters to look
  around does not engage each one.
- Sitting-recovery HP and SP numbers are now colored distinctly (HP green, SP
  blue, matching the HUD) and no longer render on top of each other.

## Friends' original concern checklist

The original playtest list was tracked as T1–T13. Here is the player-facing
status for every item so the reported concerns are visible in the patch scope:

- **T1 — Rebuild and reship the packs:** The client/server changes are pushed,
  but the distributable packs still need a final rebuild and friend install
  before this is considered shipped to players.
- **T2 — Stat allocation at character creation:** The starting-stat path is
  covered by the current automated checks; a controlled live character
  creation is still the final confirmation for the original report.
- **T3 — Party window showing the old job:** The server cache/update path is
  fixed and tested; a live job change should still be checked in the friend
  client before release sign-off.
- **T4 — Head detaching at specific facings:** The attachment compositor was
  measured across playable jobs, hairs, sexes, poses, and facings with no
  standing-pose detachments found. One in-world look is still required because
  the original report was visual.
- **T5 — `iz_ac01` black screen and invisible walls:** The stale `iz_ac02`
  server cache and unsafe stair coordinates were corrected; the teleport audit
  now reports no unsafe destinations.
- **T6 — Text too small on a 15-inch laptop:** Interface scaling already exists
  in the client settings; testing 1.3 or 1.5 is the recommended player check.
- **T7 — Friend-facing update instructions:** Update and troubleshooting
  instructions are now included in the pack documentation.
- **T8 — Names when hovering the minimap:** Party-member and local-player
  blips now provide readable hover tooltips.
- **T9 — Teleport to a party member:** The party window now provides a
  non-GM, online-party-member “Go to” action through the server command path.
- **T10 — Items and potions on the hotbar:** Inventory items can be dragged to
  hotbar slots and used from slots 1–9.
- **T11 — Multiple hotbar pages:** Three nine-slot rows are available through
  the normal, Ctrl, and Alt key tiers.
- **T12 — WASD movement:** Optional camera-relative WASD movement is available;
  click-to-move remains supported.
- **T13 — Headgear animation audit:** Headgear lookup, layering, attachment,
  sprite changes, and missing-asset handling were audited. Live rendering on
  the target client remains the last visual check.

This checklist records implementation status separately from release gates:
“still needs” items are verification or packaging work, not claims that the
underlying concern was ignored.

## Additional playtest concerns

The later adjustment list included several combat, recovery, and quality-of-life
requests beyond T1–T13. Those are included in this patch review as well:

- **HP/SP recovery:** Sitting and respawn recovery now use explicit maximum-
  resource rates, while combat, overweight, poison, blocking states, and death
  expose why recovery is paused. Normal standing regeneration remains governed
  by the standard Hercules rules. The exact recovery values are listed above.
- **Better Tab targeting:** Tab targeting cycles visible, alive, hostile
  monsters in deterministic distance order, wraps cleanly, excludes players and
  hidden entities, and highlights the selected target. A crowded live combat
  session is still the final visual check.
- **Click targeting and repeated casts:** Larger sprite hit tolerance, overlap
  resolution, target outlines, target indicators, and repeated-cast target
  retention are implemented and covered by tests.
- **Failed skills and spells:** Server rejection reasons now reach the client
  with readable messages and combat-log entries for range, target, SP/item,
  cooldown, line-of-sight, weapon, and state failures. The exact skill names
  from the original friends' report were not captured, so those specific cases
  remain a live follow-up.
- **Combat feedback:** A structured combat channel now records damage, healing,
  status changes, skill failures, EXP, and loot with independent categories and
  filters.
- **Hotbar usability:** Items and potions, multiple hotbar rows, quantity use,
  and hotbar clearing behavior are covered. Right-click/off-bar clearing and
  relog persistence still deserve a seated UI check.
- **Inventory, trade, and vendor safety:** Equipped-item selling is filtered and
  server-guarded, sale cart state is corrected, exact-fit weight boundaries are
  enforced, and vendor equipment restrictions are visible before purchase.
  Exact quantity entry and the Blue Potion overweight explanation remain useful
  friend-playtest checks.
- **Loot and dropped items:** Area-loot queueing, ownership, walkability,
  disappearance, combat cancellation, and weight limits are covered. Server
  autopickup can still reclaim a dropped item when enabled, so the player guide
  should explain that behavior.
- **Movement reliability:** WASD duplicate-request suppression and correction
  handling are implemented for the authoritative server path; internet-latency
  behavior still needs a real remote playtest.
- **EXP and party pacing:** Solo, two-player, three-player, level-range, and
  base/job EXP results were measured against server totals. Campaign-specific
  rate choices remain a configuration/design decision rather than a hidden
  client assumption.
- **Party identity and presentation:** Stable party minimap colors, player
  mouseover names/classes, privacy filtering, headgear rendering, and visual
  chest states are implemented; live rendering remains the acceptance step.

## Visual quest guide

The quest guide is advisory and keeps the player in control:

- On the current map, the tracked-objective HUD shows the action, remaining
  count, readable destination, direction, and tile distance.
- The minimap marks the revealed NPC, object, monster, or destination
  coordinate.
- For objectives on another map, the journal shows the complete readable map
  route while the HUD shows only the next portal leg.
- The next portal is marked on the minimap and in the world; after the map
  transition, guidance advances to the next leg.
- On the final map, the portal marker disappears and the destination object
  becomes the active target.
- The guide never auto-walks, enters portals, or silently completes objectives.
  Manual movement and server-authoritative completion remain required.

## Multiplayer and campaign validation

The disposable integration suite verified real two-client flows for:

- Typed objective synchronization across all six objective types.
- Campaign checkpoint advancement, reconnect, leave/rejoin, ahead-member
  refusal, carried-item ownership, and administrative rollback.
- Quest add/remove, offline completion, reconnect QuestList hydration, and
  synchronized party cleanup.
- Two-client multi-pile loot contention with exactly one winner per pile.
- Encumbrance boundaries for pickup, storage, trade, death, and cart transfer.

The code and campaign checks, packet tests, state tests, formatting checks, and
release-readiness verifier all pass. Detailed evidence is recorded in the
[release-readiness report](release-readiness-2026-09-16.md).

## Known limitations before calling it a full release

The following still need environments or people that are not available in the
automated workspace:

- Seated GUI verification of HUD, journal, portal, recovery, privacy, hotbar,
  area-loot, EXP, and Session Board visuals.
- Native Windows installation/repair verification.
- The original friends-playtest skill names and packet captures.
- Internet/second-client playtesting and a full Arc I late-join walkthrough.
- A final live restart/reconciliation pass against the project database.
- A cosmetics choice for the approved vertical slice.
- The sitting-recovery HP/SP color and offset fix, and the F/R/Disabled
  alternate attack-target key bindings, are source-fixed and test-covered but
  not yet re-confirmed on screen after the latest rebuild (the sound and
  quest-tracking fixes and the Space default binding were).
- The 2026-09-18 changes above are uncommitted on top of `ebb890fd`.

These are acceptance gates, not known automated test failures. No source,
database, or release state should be reset to bypass them.

## Recommended friend playtest focus

When testing the candidate, please focus on:

1. Opening the quest journal and tracking each objective type.
2. Following a same-map breadcrumb and a multi-map portal route.
3. Testing party reconnect/late-join behavior around an active campaign step.
4. Picking up several nearby piles while overweight or during movement/combat.
5. Checking recovery text while sitting, overweight, poisoned, in combat, and
   after respawn.
6. Confirming EXP toasts/HUD totals and player privacy during overlapping
   mouseover targets.
7. Checking a vendor list for an item your character cannot equip: the muted
   icon, red name, blocked marker, and tooltip reason should agree.

Please capture the client version/branch, map and coordinates, party members,
steps to reproduce, screenshots or video, and any `KORANGAR_*_TRACE` output
when reporting a failure.

## Backend and server changes

The patch also includes substantial Hercules-side work that players may not
see directly:

- Added durable campaign checkpoint tables and migration, namespaced by
  campaign and party with eligible-member records and an append-only event log.
- Added forward-only checkpoint advancement, carried-item ownership checks,
  item-consumption clearing, reconnect/map-load synchronization, preview/confirm
  reconciliation, ahead-member refusal, and explicit administrative rollback.
- Added versioned, server-authored DMJ messages for checkpoint snapshots, flag
  transitions, typed objectives, and reconciliation results.
- Wired campaign flag transitions, authored quest beats, encounter objectives,
  and item turn-ins into the authoritative checkpoint/objective producers.
- Added server recovery-state packets and rules for sitting, standing,
  respawn, combat interruption, poison, status blocks, and overweight states.
- Unified hard-cap encumbrance handling across pickup, shops, storage, trade,
  mail/package delivery, and cart operations, including exact-fit boundaries.
- Added campaign and navigation tooling that validates scripts, extracts the
  directed warp graph, identifies gated/one-way/dynamic routes, and preserves
  source provenance.
- Expanded server-side combat, campaign, inventory, trade, storage, cart, and
  two-client integration tests. Disposable MariaDB runs apply the real schema
  migration before starting Hercules, so persistence and reconciliation are
  tested against the actual server path.

Backend source lives primarily in the [Hercules campaign scripts](../../../Hercules/npc/custom/dm_campaign/),
[checkpoint migration](../../../Hercules/sql-files/upgrades/2026-09-16--campaign-checkpoint.sql),
and [validation tools](../../../Hercules/tools/).
