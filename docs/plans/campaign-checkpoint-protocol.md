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

Prefer the existing structured `[DMJ]` event channel for the first version if it
already provides authenticated server-to-client delivery. Define versioned
messages for:

- checkpoint snapshot;
- reconciliation preview with exact quest flags/items affected;
- confirmation request/result;
- transition notification and refusal reason.

If `[DMJ]` cannot provide the required direction or payload integrity, add one
versioned custom packet family to both Hercules and
`korangar-networking/src/packet_versions/version_20220406.rs`. Do not encode
state in display prose.

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

## Done

- Restart preserves checkpoint and ownership.
- No client message can directly set another character's progress.
- Every reconciliation displays exact effects and requires confirmation.
- Transition log and DM recovery procedure are documented and tested.
- The client checkpoint module is used in production and strict Clippy passes.

