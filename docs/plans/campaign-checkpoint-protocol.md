# Durable campaign checkpoint and reconciliation protocol

**Owners:** QW-054–056  
**Repositories:** Hercules and Korangar

## Goal

Persist party campaign progress on the server and provide an explicit,
auditable preview/confirm flow for reconnecting or late-joining members. The
current client-only `Checkpoint` model is not authoritative and must not be used
to change quest state by itself.

## Authority and persistence

Use Hercules character/global registries for the first private-server slice,
unless the state exceeds their safe shape; then add a small SQL table. Persist:

- campaign id and schema version;
- party checkpoint step;
- eligible member character ids;
- per-member reconciled step;
- item-bearing step owner where applicable;
- last actor/time and an append-only DM-readable transition log.

Define a migration/default for characters with no record. Never key durable
state only by transient party id.

## Server transition rules

- Forward-only ordinary transitions; only an explicit DM recovery command may
  roll back.
- Actor must be eligible and at the expected prior step.
- Item-bearing transitions validate and consume only the actor's items inside
  one script transaction.
- Leaving/rejoining does not erase eligibility or ownership.
- An already-ahead member is never moved backward.
- Offline members are not silently mutated. They receive a reconciliation
  proposal after login/join.

Implement reusable script functions under
`npc/custom/dm_campaign/shared/`, and have arc scripts call those functions
instead of duplicating registry manipulation.

## Transport

The first transport slice uses the existing structured `[DMJ]` event channel,
with server-colored chat as the delivery lane. Define versioned
messages for:

- checkpoint snapshot;
- reconciliation preview with exact quest flags/items affected;
- confirmation request/result;
- transition notification and refusal reason.

If `[DMJ]` cannot provide the required direction or payload integrity, add one
versioned custom packet family to both Hercules and
`korangar-networking/src/packet_versions/version_20220406.rs`. Do not encode
state in display prose.

The current implementation accepts DMJ version 1 checkpoint, flag, and typed
objective messages only from server-colored chat. Each message carries a
monotonic sequence; the client rejects malformed/future versions and stale
messages. Hercules emits checkpoint/flag snapshots after SQL-backed party
synchronization, while SQL remains authoritative. The checkpoint row is
namespaced by the stable campaign id (`seal_cascade`) plus the current party
id, carries a schema version and last actor, and writes an append-only
transition log for advance/consume/reset events. A separate durable member
table keeps eligible character ids and adopts the checkpoint when a member
leaves and later rejoins under a new transient party id. A custom packet family
is still unnecessary unless live testing shows chat delivery or integrity is
insufficient.

The reusable `DM_DMJObjective` script helper is wired to the Arc 1 Deviruchi
completion transition as the first typed DM encounter producer. It broadcasts
the completed objective to eligible online party members; additional encounter
transitions can call the same helper without changing the client protocol.

Reconciliation preview and confirm results now use a typed version-1
`reconcile` message carrying party, arc/step, mode, eligible, ahead, offline,
unavailable, and changed counts. Korangar stores the latest result as
authoritative client state; prose remains supplemental diagnostics.

Administrative rollback additionally emits an explicit version-1 `reset` DMJ
message. It clears the client campaign mirror and establishes the next sequence
epoch, so a valid reset is not mistaken for a stale backward transition.

## Client integration

1. Replace the standalone test model with `CampaignCheckpointState` in
   `ClientState`, initialized/reset with character state.
2. Parse snapshots/proposals into typed networking events.
3. Add a Session Board section showing current party step, local step, previewed
   changes, affected carried items, and Confirm/Cancel.
4. Confirmation sends only the proposal token/version; the server revalidates
   before applying and returns a fresh snapshot.
5. Refresh journal/tracker only after the authoritative result event.

## Tests

- Script/server: forward transition, stale expected step, ineligible actor,
  wrong item owner, duplicate confirm, rollback permission, restart persistence.
- Protocol: malformed version, expired/replayed proposal, disconnect between
  preview and confirm.
- Two clients: offline advancement, reconnect, late join, leave/rejoin,
  already-ahead member, item-bearing step, server restart.

The migration has also been exercised in an isolated temporary MariaDB
instance: schema creation, checkpoint/member/audit inserts, clean database
shutdown/start, and row recovery all passed. The project-local `ragnarok`
account still cannot apply the migration because it lacks `CREATE`.

The real two-client `dm-checkpoint-reconcile` scenario also covers an online
party transition, offline advancement, reconnect confirmation, leave/rejoin,
and carried-item owner propagation on the authoritative checkpoint echo. The
fresh disposable-schema run passed in
`tools/testing/runs/20260916-203309.scoped`; live independently-ahead member
creation and actual carried-item consumption remain acceptance fixtures.

## Done

- Restart preserves checkpoint and ownership.
- No client message can directly set another character's progress.
- Every reconciliation displays exact effects and requires confirmation.
- Transition log and DM recovery procedure are documented and tested.
- The client checkpoint module is used in production and strict Clippy passes.
