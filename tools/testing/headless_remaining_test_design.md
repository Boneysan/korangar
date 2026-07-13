# Remaining Headless Integration Test Design

Implementation-ready specification for closing the remaining Korangar/Hercules
integration backlog. This document complements `headless_test_plan.md`; it defines
fixtures, scenario names, exact assertions, cleanup rules, and acceptance gates.

## 1. Scope and completion boundary

Headless tests own protocol serialization, packet framing, `NetworkEvent` mapping,
server state transitions, persistence, and multi-client synchronization. They do not
claim to verify pixels, animation quality, mouse hit targets, or window layout.

The backlog is complete when:

1. every scenario below is registered in `all_scenarios()`;
2. `--scenario all` passes twice consecutively from a clean Hercules start;
3. cleanup passes even after an injected mid-scenario failure;
4. the packet ledger contains no new deserialization failures;
5. `cargo test --workspace --all-features` and `skill-asset-audit` pass.

## 2. Fixtures and isolation

### Accounts

| Fixture | Default | Ownership | Purpose |
|---|---|---|---|
| GM | `korangar` | persistent, never deleted | GM bootstrap, DM commands, primary social client |
| Partner | `headless2` | persistent, non-GM | friends, party, whisper, trade |
| Lifecycle | `headless-lifecycle` | disposable account | create/delete/slot-switch tests |

The lifecycle account must never reuse the GM or graphical-verification account. Its
characters use the run suffix `hl_<pid>_<counter>` and are deleted during teardown.
If account self-registration is disabled, the wrapper creates these fixtures through a
documented SQL fixture script before starting Hercules.

### Baseline state

Each scenario records a `FixtureSnapshot` before mutation:

- account and character IDs;
- character slots and names;
- map and tile position;
- job, base level, job level, HP/SP, zeny;
- inventory `(index, item_id, amount, refine, broken)`;
- party ID, friend IDs, hotkeys;
- campaign flags touched by the scenario.

Teardown restores only fields owned by that scenario. A failed restore is a test failure
named `cleanup::<scenario>` rather than a warning.

### Timing

- ordinary packet wait: configured `--timeout` (default 15 s);
- negative assertion: 1 s quiet window;
- online/offline propagation: 5 s;
- periodic hazard: configured interval plus 2 s;
- every wait reports the last 20 events from both clients.

## 3. Harness additions

### `DualContext`

Own two independent `TestContext` values and pump both event queues fairly:

```rust
struct DualContext {
    primary: TestContext,
    partner: TestContext,
}
```

Required methods:

- `login_both(config)` and `logout_both()`;
- `wait_primary`, `wait_partner`, and `wait_both`;
- `pump_until(deadline, predicate)` without starving either socket;
- `place_together(map, x, y)` using GM warp plus partner recall;
- `ensure_not_friends`, `ensure_no_party`, `cancel_trade_if_active`;
- `Drop` performs best-effort socket logout; explicit `cleanup()` remains mandatory.

### Lifecycle fixture

`LifecycleContext` connects only through login/character servers until a scenario needs
map verification. Required helpers:

- `unique_character_name()`;
- `fresh_character_list()` after reconnect;
- `first_empty_slot()` and `occupied_slots()`;
- `delete_character_and_confirm()` including any required birth-date/email token;
- `cleanup_created_characters()`.

### Event transcript

Every scenario writes a compact transcript:

```text
elapsed_ms direction header event_summary
```

On failure, include both transcripts, fixture snapshot differences, and packet-ledger
deserialization failures.

## 4. Phase 1 — destructive session lifecycle

### `session-character-create`

Precondition: lifecycle account has an empty slot.

1. Fetch character list and select the first empty slot.
2. Create `hl_<run>_a`.
3. Assert `CharacterCreated` name and slot.
4. Reconnect and assert the character persists in the same slot.
5. Attempt the same name again; assert `CharacterCreationFailed` without list mutation.
6. Delete the created character and reconnect; assert absence.

Cleanup: delete the generated name if any step fails.

### `session-character-delete`

1. Create a disposable character and enter the map once.
2. Log out to character select.
3. Request deletion and provide confirmation.
4. Assert deletion acknowledgement.
5. Reconnect twice and assert the character remains absent.

Negative assertions: no other character ID or slot changes.

### `session-slot-switch`

1. Ensure two disposable characters exist in distinct slots A and B.
2. Record character IDs, not just names.
3. Switch A→B using the protocol API.
4. Assert `CharacterSlotSwitched`.
5. Reconnect and assert IDs occupy the opposite slots.
6. Switch them back and verify persistence.

Cleanup: restore original slots before deleting disposable characters.

### `session-logout-relogin`

1. Complete map login and send a unique chat marker.
2. Call protocol logout; assert `LoggedOut`/disconnect sequence without timeout.
3. Reconnect with the same account immediately.
4. Assert map login and a second marker succeed; no `AlreadyOnline` terminal failure.

### `session-respawn`

1. Snapshot save map and HP.
2. Issue `@die`; tolerate Kaizel double-death behavior using the existing helper.
3. Assert local-player death/removal evidence.
4. Call respawn and assert `ChangeMap` or resurrection at the save point plus HP > 0.
5. Send a movement request to prove the session is actionable.

## 5. Phase 8 — multi-client social suite

All scenarios begin with `ensure_not_friends`, `ensure_no_party`, no active trade, both
characters alive, and both characters on the same map when proximity matters.

### `social-whisper-roundtrip`

1. A sends a unique message to B.
2. B asserts `WhisperReceived` sender and exact body.
3. A asserts successful `WhisperResult` if supplied by Hercules.
4. B replies; mirror assertions.
5. A whispers a generated offline name; assert failure result and no delivery to B.

### `social-friend-lifecycle`

1. A requests B; B asserts requester account/character IDs.
2. B accepts; both assert friend-added state.
3. Reconnect B; A asserts offline then online status transitions.
4. Reconnect both and assert `SetFriendList` contains the relationship.
5. A removes B; both assert removal where Hercules emits it.
6. Reconnect and assert neither list contains the other.

### `social-friend-reject`

1. A requests B; B rejects.
2. Assert A receives rejection/failure and neither friend list changes.

### `social-party-lifecycle`

1. A creates a uniquely named party; assert success and party ID.
2. A invites B; B asserts inviter/party identity.
3. B accepts; both wait until both member IDs appear.
4. B walks at least five tiles; A asserts B's map and position update.
5. Damage and heal B; A asserts health decreases then increases.
6. A sends party chat; B asserts exact message and sender.
7. B leaves; both assert member removal.
8. A leaves/disbands; reconnect and assert no party state.

### `social-party-invite-block`

1. B enables invitation blocking.
2. A invites B; assert rejection on A and no invitation event on B during the quiet window.
3. B disables blocking in cleanup.

### `social-trade-commit`

Fixture: A owns two Red Potions and enough zeny; B has inventory capacity.

1. A requests trade; B accepts; both assert partner IDs.
2. A adds one potion; B adds a small zeny amount.
3. Assert partner-item/zeny events and add-result acknowledgements.
4. Both lock, then commit.
5. Assert `TradeCompleted` on both and exact inventory/zeny deltas.
6. Reconnect and assert deltas persisted.

### `social-trade-cancel`

Repeat through item offer, then cancel before lock. Assert cancellation on both and
snapshot-identical inventory/zeny after reconnect.

### `social-emotion`

Place clients in view, have A request a deterministic emotion, assert B receives the
correct entity ID and emotion ID exactly once.

## 6. Phase 9 — DM campaign suite

DM tests run after social tests so party recall can reuse `DualContext`. Each test calls
`@dmreset` before setup and `@dmcleanup` plus `@dmreset` after assertions.

### `dm-command-contract`

For every bound `@dm*` command:

1. invoke with no arguments and one deliberately invalid argument;
2. assert server-colored usage/error feedback contains the command name;
3. assert no packet deserialization failure and no disconnect.

This is table-driven: `(command, invalid_args, expected_fragment)`.

### `dm-roll-bounds`

Run deterministic forms `1d1`, `2d6+3`, `4d8-2` and malformed inputs.

- `1d1` must equal 1;
- other totals must fall within mathematical bounds;
- malformed input must return usage/error feedback;
- public rolls reach the partner when applicable; hidden rolls do not.

### `dm-warp-recall`

1. Form a two-client party.
2. Warp A to a known audited coordinate.
3. Invoke recall and assert B receives the same map and a nearby walkable tile.
4. Both clients send movement requests afterward.

### `dm-beat-table`

Table-drive every configured beat `(arc, beat, map, x, y)` extracted from campaign
scripts. For each enabled beat:

- invoke `@dmbeat`;
- assert the expected map and a position within the scripted arrival region;
- assert the coordinate appears in the zero-failure map asset audit;
- assert required NPC/entity/marker events where the beat defines them;
- cleanup before the next row.

### `dm-flags-status`

1. Set a unique flag/value and query `@dmstatus`.
2. Assert output contains the exact flag/value.
3. Toggle/reset it and assert status changes.
4. Reconnect if the flag is documented persistent; otherwise assert reset on cleanup.

### `dm-quest-markers`

Trigger a quest-bearing beat. Assert quest add/update and minimap/quest-effect events,
then complete/reset it and assert removal. Event IDs must match script constants.

### `dm-reward-delta`

Snapshot inventory, zeny, and experience; grant one deterministic reward-table entry;
assert exact deltas and reconnect persistence. Cleanup removes granted items/zeny only
from the fixture account.

### `dm-hazard-periodic`

1. Arm a hazard with known damage and interval.
2. Place A inside and B outside.
3. Assert A receives at least two damage ticks within tolerance and B receives none.
4. Move A outside; assert no further tick during two intervals.
5. Disarm and assert cleanup feedback.

### `dm-instance-lifecycle`

1. Create instance; capture generated map/instance identity.
2. Enter with the party and assert both map changes.
3. Invoke cleanup; assert exit map and that re-entry is rejected.
4. Query status to prove no instance remains.

### `dm-experience`

Snapshot base/job experience, grant a fixed amount, assert `GainedExperience` type and
exact delta, then restore fixture state.

## 7. Skill-menu and item-menu regression scenarios

### `skills-teleport-select`

1. Change to Acolyte and grant skills.
2. Cast Teleport level 2.
3. Assert `WarpList` contains non-empty, valid map names.
4. Send `select_warp_destination` for one offered value.
5. Assert map/position transition and successful movement afterward.

### `skills-teleport-cancel`

Cast Teleport, receive the list, close the client-side selection without sending a
destination packet, then assert movement and a second skill cast work. Hercules
interprets an empty destination as random Teleport, not cancellation; this regression
is enforced by the dedicated headless scenario.

### `skills-weapon-refine-missing-material`

1. Become Whitesmith; grant a level-one Sword but no Phracon.
2. Cast Upgrade Weapon; assert `RefinableWeaponList` contains the Sword's normalized
   inventory index.
3. Select it and assert result code 3.
4. Assert inventory and refine level are unchanged.

### `skills-weapon-refine-success`

Grant enough Phracon, select the Sword, and repeat within a bounded loop until success or
the documented maximum attempts. On result 0, assert refine level increases by one and
material count decreases. Ordinary RNG failure is recorded as an attempt, not a failed
scenario; failure to observe either valid result is a failure.

### `skills-weapon-refine-cancel`

Receive a non-empty refinable list, send `cancel_weapon_refine`, then assert no refine
result or inventory/material delta during the quiet window. Immediately cast another
skill and reopen Upgrade Weapon to prove Hercules cleared its pending menu state.

### `items-repair-list-empty`

Cast Repair Weapon with no broken equipment. Assert skill-failure feedback and no repair
list during the quiet window.

### `items-repair-success`

Fixture setup marks one disposable equipped weapon broken through a dedicated SQL fixture
or deterministic server helper before login.

1. Cast Repair Weapon and assert the repair list contains exactly that item with the
   normalized index.
2. Send the repair selection response.
3. Assert repair acknowledgement success and inventory/equipment refresh.
4. Reconnect and assert the broken flag is cleared.

Cleanup deletes the disposable item even after failure.

### `items-repair-failure`

Select an item that becomes invalid between list receipt and response (fixture removes or
repairs it server-side). Assert failure acknowledgement, no unrelated inventory mutation,
and that a second valid Repair Weapon interaction can start.

## 8. Non-graphical asset gate

`skill-assets` is a wrapper scenario that executes `skill-asset-audit` from the nested
client directory and records its summary. Current acceptance value:

```text
catalogued skills checked: 1007
skills with missing icon assets: 0
```

This proves resource presence and alias resolution, not appearance. A small manual matrix
still checks clipping and visual correctness for Novice, Acolyte, Whitesmith, Hunter, and
one third/fourth job.

## 9. Protocol API and event traceability

Coverage is enforced by a checked-in manifest rather than inferred from scenario prose:

```rust
struct CoverageEntry {
    symbol: &'static str,
    scenarios: &'static [&'static str],
    exclusion: Option<&'static str>,
}
```

Maintain one entry for every public map/character/login action on
`NetworkingSystem` and every `NetworkEvent` variant. A unit test parses/compares the
maintained symbol lists (or a generated source list) and fails when a new symbol has
neither a scenario nor an explicit exclusion. Exclusions must be concrete, for example
"connection-state getter; asserted by harness invariant" or "visual-only no-op packet";
"untested" is not an acceptable exclusion.

### Connection and failure-path scenarios

#### `session-connection-state`

At each login→character→map handoff, assert `is_*_connected()` and the corresponding
connected/disconnected event. Explicitly call each disconnect method once and verify the
state getter changes without a panic or leaked task.

#### `session-character-select-failures`

Exercise invalid/empty slot selection and an already-online/stale-session reconnect.
Assert typed `CharacterSelectionFailed`/login failure reasons, then prove a valid retry
succeeds without restarting the tester.

#### `session-character-mutation-failures`

Assert invalid character name, duplicate name, full-slot creation, invalid deletion
confirmation, and invalid slot-switch produce their typed failure events and leave the
character list unchanged.

### Party and trade rejection scenarios

#### `social-party-reject`

A invites B; B explicitly calls `reject_party_invite`. Assert A receives a rejection,
B never joins, and reconnect shows neither fixture in a party.

#### `social-trade-reject`

A requests trade; B explicitly calls `reject_trade`. Assert both sessions remain usable,
no trade window/start event follows, and inventory/zeny snapshots are unchanged.

#### `social-trade-invalid-offers`

During an accepted trade, attempt zero amount, excessive amount, nonexistent index, and
excessive zeny. Assert typed add-result failures, then cancel and verify no deltas.

### Inventory and economy negative scenarios

#### `items-equip-failures`

Attempt wrong-job equipment, invalid equip position, and an unavailable inventory index.
Assert failure feedback and no `UpdateEquippedPosition`; follow with a valid equip/unequip
to prove the menu state is not stuck.

#### `items-use-drop-failures`

Attempt use with an invalid index, drop zero, and drop more than the stack. Assert no
inventory corruption or ground item. Then perform valid use/drop/pickup in the same
session.

#### `items-identify-cancel`

Open the identify list, send `cancel_item_identify`, assert no `ItemIdentified`, then
reopen and complete identification. Separately cover `one_click_item_identify` and the
ordinary request path.

#### `items-storage-persistence`

Move a deterministic item into storage, close storage, relog, reopen, and assert it
persists. Move it back, close, relog, and assert the baseline is restored. Include full
storage and invalid-index rejection where the fixture database can provision them.

#### `items-shop-close-and-failures`

Open a stable NPC shop, call `close_shop`, and assert subsequent purchase is rejected.
Reopen and test insufficient zeny, excessive quantity, invalid item ID, selling an
unsellable item, and valid buy/sell. Assert typed completion results and exact zeny/item
deltas.

#### `items-stat-skill-hotkey-boundaries`

- stat-up with available points succeeds; without points fails without stat mutation;
- skill-up with a valid point succeeds; maxed/unknown skill fails without mutation;
- set, replace, clear, and relog a hotkey, asserting `SetHotkeyData` after each persisted
  transition.

### Deterministic modeled-event scenarios

The following events require named producers so they are not counted merely because they
appeared incidentally:

| Scenario | Required events/assertions |
|---|---|
| `world-initial-state` | `InitialStats`, `UpdateAttackRange`, `SetInventory`, `SkillTree`, initial `SetHotkeyData` |
| `world-entity-slide` | cast/command causing knockback or high jump; exact `EntitySlide` ID and destination |
| `world-message-table` | deterministic server message ID and color via script fixture |
| `world-hair-change` | GM hairstyle change; `ChangeHair` account and hair ID |
| `skills-monster-information` | Sense/Estimation; all monster stat and effectiveness fields sane |
| `skills-cooldown-snapshot` | create cooldown, map change/relog, assert `SkillCooldownList` remaining time decreases |
| `skills-auto-run` | item/skill producing `AutoRunSkill`; assert metadata, castability, replacement/removal behavior |
| `skills-update-remove` | grant/level/remove one skill; `UpdateSkill` then `RemoveSkill`, tree persistence on relog |
| `skills-visual-effect` | deterministic refine/buff effect; correct entity and known effect path |
| `world-minimap-mark` | add/update/remove marker; exact ID, position, type, and color |
| `world-ground-item-lifecycle` | add, pickup, remove/expire; exact entity and inventory deltas |
| `quest-effect-lifecycle` | add and remove quest effect with exact entity/effect fields |

### No-op and fallback packet policy

Registered no-op packets and known-length fallbacks are not silently considered covered.
The manifest assigns each one to one of three buckets:

1. **intentionally ignored** — documented reason and graphical impact;
2. **ledger-only** — occurrence and framing asserted, modeling not presently required;
3. **must model** — emitted by supported gameplay and needed for state/UI correctness.

`--scenario all` fails if a new fallback header appears without a bucket. It also fails
on every deserialization error, unknown variable length, or stream desynchronization.

### Coverage acceptance report

At suite end print and serialize:

- public actions mapped / total;
- event variants observed / total and deterministically asserted / total;
- no-op/fallback headers by policy bucket;
- scenarios skipped and the fixture capability that caused each skip;
- visual-only checks remaining manual.

The suite is not "fully covered" when a required scenario is skipped. Skips are allowed
only for explicitly optional platform capabilities and make the overall result partial,
not green.

## 10. Suite order and cleanup audit

Recommended `--scenario all` order:

1. smoke and non-destructive session checks;
2. destructive lifecycle on its disposable account;
3. GM/movement/combat/skills/items/dialogue;
4. skill-menu regressions;
5. multi-client social;
6. DM campaign;
7. asset gate;
8. final fixture audit.

The final fixture audit reconnects all three accounts and compares owned state against
their baselines. It also asserts no active party/trade/instance, no generated lifecycle
characters, no test campaign flags, and no packet-ledger deserialization failures.

## 11. CI and reporting

The integration wrapper must use a shell trap so Hercules stops on success, failure, or
interrupt. Produce both human output and `target/headless-results.json` containing:

- scenario, phase, duration, outcome;
- failure stage and message;
- headers/events observed;
- cleanup outcome;
- fixture diff;
- findings-log issue ID when known.

CI should initially run smoke + asset audit on every change and the destructive/social/DM
suite on a scheduled or explicitly provisioned runner with MariaDB and isolated accounts.
