# Release readiness — 2026-09-16

## Candidate state

The Korangar and Hercules worktrees are uncommitted working trees. No commit,
push, upload, or publication is authorized by this report. The candidate spans
the active QW implementation slices for typed objectives, campaign flags and
checkpoints, reconciliation, breadcrumb/portal routing, recovery, and
overflow-safe encumbrance handling.

This report was refreshed during the 2026-09-16 execution pass. Latest local
live evidence is archived under `korangar/tools/testing/runs/`; the Windows
matrix now records runner identity and UTC execution time in its report.

## Revision and rollback boundary

- Korangar base revision: `866a8bac4d0055e63e2944d91b0931e9f078d5f0`
- Hercules base revision: `0e9cc355c738704bf7d7e4c43db59c6fa5a886b7`
- Both worktrees are dirty and uncommitted. The Korangar diff contains 38
  tracked files plus the documented untracked artifacts; the Hercules diff
  contains 21 tracked files. These counts prevent a clean revision from being
  mistaken for the candidate state.
- Rollback boundary: preserve the current worktrees and their diffs as the
  rollback package. No reset, commit, push, upload, or publication is
  authorized. Any release decision must first snapshot or otherwise archive
  both worktrees, then perform the explicitly authorized action.

## Automated evidence

- `cargo test -p korangar --lib`: 527 passed, 17 ignored.
- `cargo clippy -p korangar -- -Dwarnings`: passed.
- `cargo clippy -p korangar --features debug -- -Dwarnings`: passed.
- `cargo clippy -p korangar-networking --lib -- -Dwarnings`: passed.
- `./tools/check-campaign.sh`: passed; 41 campaign include lines, 0 loader errors.
- DM parser/state focus: 14 tests passed; legacy v1 reconciliation parsing is covered.
- Both worktrees pass `git diff --check`.
- `tools/testing/check-release-readiness.sh`: passed; every unchecked runbook
  QW ID is present in the report, and hashes, required live artifacts, gate
  inventory, and both worktree diff checks are consistent.
- `Hercules/tools/check-checkpoint-migration.sh`: isolated migration and
  MariaDB restart persistence passed; it is included in `check-campaign.sh`.
- QW-054 durable checkpoint/reconciliation: the disposable integration runner
  applied the real migration and `dm-checkpoint-reconcile` passed against a
  private MariaDB in `tools/testing/runs/20260916-231914.scoped` with clean
  fixture teardown. The run covers two-client advancement, offline/reconnect,
  ahead-member refusal, leave/rejoin, and administrative rollback; isolated
  MariaDB restart persistence remains covered by
  `Hercules/tools/check-checkpoint-migration.sh`.
- QW-052 typed-objective synchronization: the real Arc 1 beat shortcut delivered
  the authoritative `Talk` objective for quest 20006 to both clients in
  `tools/testing/runs/20260916-203746.scoped`, and the real Arc 2 completion
  shortcut delivered the authoritative `DM` objective for quest 20012 to both
  clients in `tools/testing/runs/20260916-204107.scoped`; the campaign checker
  also guards both shortcut producers. The same two-client Arc 1 shortcut
  contract delivered the authoritative `Explore` quest 20001 update and
  in-progress `Interact` quest 20005 update to both clients in
  `tools/testing/runs/20260916-205201.scoped`; the live Kill objective contract
  passed in 12.5s in `tools/testing/runs/20260916-213809.scoped` using a real
  player kill and authoritative quest update. Direct GUI NPC activation on
  the dungeon-map NPC paths remains an acceptance gate. The inventory-backed Arc 1 Collect contract passed in 10.9s in
  `tools/testing/runs/20260916-210047.scoped` with required item counts and
  clean teardown.
- A fresh disposable two-client Talk objective rerun passed in
  `tools/testing/runs/20260916-223314.scoped` with clean teardown.
- The consolidated `dm-objective-type-matrix` passed in
  `tools/testing/runs/20260916-232810.scoped`, exercising Talk, DM encounter,
  Explore, Interact, Collect, and Kill sequentially on one disposable server
  build with clean teardown. Objective icon/label mappings are now owned by
  the typed model and covered by unit tests.
- QW-056 journal refresh lifecycle: the two-client party contract covered
  QuestAdded, offline completion/removal, reconnect QuestList hydration, and
  synchronized party erase in
  `tools/testing/runs/20260916-205649.scoped` with clean teardown; GUI journal
  observation remains open.
- A fresh `dm-party-quest-refresh` rerun passed in
  `tools/testing/runs/20260916-233018.scoped` with clean disposable-server
  teardown; the remaining gate is still seated late-join journal rendering.
- QW-055 carried-item ownership: the live Wynne turn-in consumed the Arc 1
  contract items and synchronized carrier-cleared checkpoint snapshots to both
  clients in `tools/testing/runs/20260916-214436.scoped`. The live
  independently-ahead member fixture now passes preview `ahead=1` and confirm
  refusal in `tools/testing/runs/20260916-220301.scoped`; the normal migrated-
  schema reconciliation fixture was also revalidated in
  `tools/testing/runs/20260916-215826.scoped` with clean teardown. Session Board
  GUI visualization remains open.
- Skill-failure decoding: 10 reason/cause text tests and 3 packet-handler
  tests passed; live `skill-fail-reason-packet` passed in
  `tools/testing/runs/20260916-200536.scoped`.
- QW-046 area-loot filtering: the client click path now passes real item
  weights, remaining capacity, occupied-slot capacity, and map walkability to
  the queue builder; 11 area-loot state tests and the static cancellation
  contract pass. The live `loot-pickup-multi-pile` contract also passed in
  `tools/testing/runs/20260916-232255.scoped`, proving two distinct floor
  entities are each granted once under two-client contention. Seated multi-pile
  click/cancel observation remains open.
- QW-072 recovery protocol: the six-section sitting/overweight/poison matrix
  now asserts server-authored sitting, standing, and blocked
  `RecoveryState` packets; it passed in
  `tools/testing/runs/20260916-221241.scoped`. Graphical HUD observation remains
  open.
- QW-075 trade boundary: a two-client disposable run accepted exact-fit item
  staging at `max_weight - 1` and refused an over-cap batch without emitting a
  partner-side offer in `tools/testing/runs/20260916-221632.scoped`.
- QW-079 authored quest EXP: the disposable authored dialogue fixture called
  `DM_PartyExp(1000, 500)`, emitted exact base/job packets, and removed quest
  2000 in `tools/testing/runs/20260916-222537.scoped`; seated toast/HUD pixels
  remain a GUI gate.
- QW-075 storage boundary: a disposable fixture deposited two Arrows at the
  hard-cap boundary, refused an over-cap withdrawal with explicit server text,
  and successfully withdrew after making room in
  `tools/testing/runs/20260916-222921.scoped`.
- QW-075 death boundary: a disposable overweight character died, respawned
  alive, and retained its exact pre-death weight in
  `tools/testing/runs/20260916-223128.scoped`.
- QW-075 is now complete: cart protocol coverage has typed `0x0126`/`0x0127` requests,
  `0x012C` refusal decoding, and a passing server-capacity boundary artifact
  at `tools/testing/runs/20260916-231247.scoped`; the combined server matrix,
  static path audit, and recovery/configuration tests are also green.
- Local live contracts: Increase AGI (`20260916-195916.scoped`), loot race
  (`20260916-195943.scoped`), sitting/respawn recovery
  (`20260916-200023.scoped`, `20260916-200013.scoped`), and DM experience
  (`20260916-200003.scoped`) passed.
- Windows installation matrix: all 8 cases pass locally; native CI remains
  unobserved because the workflow is not yet committed/pushed.
- Navigation route contract: the full Prontera → `prt_maze02` route now has
  coordinate-level assertions for intended, wrong-portal, teleport, and
  respawn recomputation; seated HUD/world observation remains open.
- QW-085 privacy contract: a live two-client run hid a player from the
  observer and restored visibility after unhide in
  `tools/testing/runs/20260916-223833.scoped`; graphical overlap selection
  remains a manual gate.
- QW-046 manual capture: the client now supports opt-in
  `KORANGAR_AREA_LOOT_TRACE=1` logging for queue decisions and cancellation
  reasons; normal behavior is unchanged.
- QW-063 manual capture: the client now supports opt-in
  `KORANGAR_NAVIGATION_TRACE=1` logging for recomputation reasons, route maps,
  active portal coordinates, destinations, and revisions.
- QW-072 manual capture: the client now supports opt-in
  `KORANGAR_RECOVERY_TRACE=1` logging of server recovery mode/block packets
  beside the exact HUD status text.
- QW-079 manual capture: the client now supports opt-in
  `KORANGAR_EXP_TRACE=1` logging of EXP award packets, toast text, and updated
  base/job HUD totals.
- QW-039 manual capture: the client now supports opt-in
  `KORANGAR_HOTBAR_TRACE=1` logging of cleared slots and prior bindings for
  right-click and drag-off-bar verification.
- QW-085 manual capture: the client now supports opt-in
  `KORANGAR_PRIVACY_TRACE=1` logging of picker hits and visible candidate
  depth/distance ordering after privacy filtering.
- QW-055/QW-056 manual capture: the client now supports opt-in
  `KORANGAR_DMJ_TRACE=1` logging of accepted/rejected authoritative DMJ
  messages and resulting checkpoint/objective/reconciliation state counts.

## Artifacts

- DMJ transport and client state: `korangar/src/dm/parser.rs`,
  `korangar/src/dm/mod.rs`.
- Session Board and route presentation: `korangar/src/interface/windows/quest_log.rs`,
  `korangar/src/state/navigation.rs`.
- Server checkpoint/reconciliation scripts:
  `Hercules/npc/custom/dm_campaign/shared/dm_checkpoint.txt` and
  `Hercules/npc/custom/dm_campaign/shared/dm_dmj.txt`.
- Checkpoint migration:
  `Hercules/sql-files/upgrades/2026-09-16--campaign-checkpoint.sql`.

## Open acceptance gates

- Remaining unchecked cards are explicitly classified as follows: GUI/live
  display gates (`QW-006`, `QW-039`, `QW-046`, `QW-052`, `QW-055`, `QW-056`,
  `QW-062`, `QW-063`, `QW-072`, `QW-079`, `QW-085`); native Windows gates
  (`QW-011`, `QW-014`, `QW-101`); missing friends-playtest input
  (`QW-020`, `QW-030`); explicit user choice (`QW-087`); second-client or
  end-to-end playtest gates (`QW-102`, `QW-103`, `QW-104`); and final release
  authorization (`QW-105`).
- The restricted project database still lacks the checkpoint tables and the
  current user lacks `CREATE`; deployment therefore requires the DBA/owner to
  apply the migration. The isolated migration, restart persistence, and
  two-client reconciliation acceptance are complete.
- Native Windows installer/repair verification remains open; the PowerShell
  matrix on macOS is not equivalent to a native Windows run.
- Graphical HUD, portal-marker, recovery-state, and player-privacy acceptance
  require a seated client session.
- QW-087 requires explicit user selection/approval of a cosmetics slice.
- QW-020/QW-030 still require the actual friends-playtest skill names and
  packet captures; the generic decoder is implemented and validated, but those
  source-specific failures must not be invented.

## Release action

No release action should be taken until the open gates are recorded as passed
and the user explicitly authorizes commit, push, upload, or publication.
