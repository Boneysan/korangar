# Bestiary unlock persistence — player Adventure Guide

**Parent:** [GDD §9.5–9.6](../GDD.md), [Adventure Guide data contract](encyclopedia-data.md), and [next-slices plan](../plans/gdd-next-slices.md) S7. This supersedes the older client-local-first and DM-global proposal. The DM Bestiary may keep its separate session state; it must not become the authority for player knowledge.

## Authority and key

Hercules stores monster discovery milestones **per account**. A first encounter/kill (and later reviewed events such as a lore check) can advance a badge, encounter history, or authored non-spoiler note. This ledger never gates searchable monster mechanics, drops, rates, cards, map links, or other build-planning facts. A kill grants the configured milestone to the killer's account. Whether eligible nearby party members also receive discovery credit is a separate implementation choice; if added, grant it server-side, still keyed by account. An account cannot be downgraded by a stale client, map change, DM view toggle, or reconnect. Later non-story visited-place, rumor, and general-service unlocks use the same account scope and distinct typed keys. **Story quests, chapter completion, choices, and story-gated access never enter this account ledger**; they remain per-character, including in DM Session mode ([quest-sync contract](dm-party-quest-sync.md)).

Use Hercules account variables for initial implementation, named by stable monster ID with a bounded tier value. Before choosing one variable per monster, measure the real variable count and login-query cost for the current 1,759 monsters; if the store is too large, use a dedicated account/monster table with the same external semantics. Do not store a single growing CSV in a global variable. Keep a schema version and an explicit migration path for any previously saved client-local RON unlocks: import only after the server validates the account/monster IDs, union by highest tier, and never accept peer party messages as authoritative grants.

## Snapshot and delta protocol

On login/reconnect, Hercules sends a bounded snapshot of discovered `(monster_id, milestone)` pairs. Chunk if the chosen carrier cannot hold the full set; include account/session identity, sequence and final-chunk marker, and replace stale client state only after a complete snapshot. Kill/discovery deltas include ID, milestone, and sequence. The client applies monotonic milestone merge and may cache the snapshot locally for quick UI opening, but a fresh server snapshot wins over cached membership; offline cache is labeled stale. The Guide's mechanical reference remains usable while this sync is pending. The existing `[DMJ]` structured echo is a possible server-originated carrier only if it passes size, ordering, and visibility checks. A dedicated fork packet is acceptable when it does not.

The friends-server Guide is open by default. Discovery synchronization changes badges and authored encounter notes, not reference-field visibility. Never represent a locally hidden field in packaged JSON as a security boundary. Campaign plot spoilers and DM tools need their own server-authorized policy; they are not part of this ledger.

## Acceptance

- Kill a Poring, relog, and see its discovery milestone retained on a second character of the same account; a different account starts at its own milestone.
- Snapshot handles zero, many, interrupted, and reordered chunks without inventing unlocks; delta after snapshot merges once.
- Missing/corrupt local cache does not crash or erase server state. DM reveal-all does not persist.
- Two accounts with different discovery histories see the same verified Poring stats, drops, and source links. No party peer can forge a milestone grant.
