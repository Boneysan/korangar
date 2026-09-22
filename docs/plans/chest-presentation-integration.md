# Hidden chest presentation — implementation plan

**Owners:** QW-088–090  
**Decision:** discoveries remain per-character

## Goal

Render authoritative unopened/available/opened chest states and explain Field
Notes, pooled Marks, regional cosmetic progress, and non-reset behavior through
player/DM surfaces.

## Discovery contract

First inventory the enabled chest scripts under
`Hercules/npc/custom/dm_campaign/`: chest id, region, map/coordinate, discovery
registry, reward, Field Note/Mark effects, and repeat/reset rules. Generate or
author one versioned manifest and validate unique ids and coordinates.

The server owns discovery. A client click may request/interact but cannot mark a
chest opened. Define structured server events/snapshots for:

- manifest/schema version;
- per-character discovered/opened ids;
- Field Note count;
- pooled Mark total and regional cosmetic progress;
- interaction result/refusal.

Use the existing authenticated `[DMJ]` channel if suitable; otherwise add a
versioned packet. Send a snapshot on login/map entry and deltas after changes.

## Client integration

1. Replace `ChestBook` with production `ChestDiscoveryState` in `ClientState`.
2. Load the chest manifest through `Library` and reject incompatible schema.
3. Map server snapshot/deltas to state; clear/reload on character switch.
4. Bind chest world entities/NPCs to manifest ids. Render three distinguishable
   states using existing sprite/effect/tint facilities, with a text/icon fallback
   so color is not the only signal.
5. Interaction sends the normal request and waits for the server result before
   showing opened.
6. Add a journal section explaining personal discoveries, pooled Marks,
   cosmetic thresholds, and persistence. Surface `@fieldnotes`/`@marks` results
   through appropriate player and DM controls without expanding permissions.

## Tests

- Manifest duplicate/missing/invalid coordinate and schema failures.
- First discovery, already opened, interaction refusal, another character still
  unopened, party member interaction, relog and server restart.
- Snapshot/delta ordering, duplicate delta idempotence, stale schema.
- Field Notes/Marks values agree with commands/database.
- GUI/world visual pass for all three states and regional cosmetic persistence.

## Done

- Visual state comes only from authoritative discovery state.
- Per-character behavior survives relog/restart and does not leak between
  characters.
- Journal and commands report matching values.
- `chest_discovery.rs` is production-used; tests and strict Clippy pass.

