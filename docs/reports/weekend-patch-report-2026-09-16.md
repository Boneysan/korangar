# Weekend patch report

## What this patch is

This is the current campaign/gameplay patch candidate from the weekend bug-fix
pass. It improves quest guidance, campaign progression, inventory handling,
recovery, and multiplayer synchronization across the Korangar client and
Hercules server.

The changes are pushed on these branches:

- Korangar: `agent/bump-hercules-pin` — `60e8e7a7`
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
