# DM Session party quest synchronization — GDD §8 / S10

**Decision (2026-09-23):** A player who joins the active campaign party late automatically inherits that party's campaign quests and story progress. This catch-up includes story flags, but does not grant past rewards. Characters remain separate; another character on the same account inherits nothing until it joins the party. General discoveries and non-story unlocks remain account-wide under GDD §9.6.

## Current implementation (source audit 2026-09-23)

- `OnPCPartyJoin` calls `DM_CheckpointSyncSelf` and `DM_CatchUpFromParty` immediately while the active campaign session is allowed. This is the automatic late-join hook.
- `OnPCQuestLog` catches up after the server restores the character's saved quest list. `OnPCLoginEvent` intentionally waits for that point; `OnPCLoadMapEvent` retries after party/map context is available. Returning party members therefore use the same status reconciliation path.
- `DM_CatchUpFromParty` reads `dm_campaign_party_state` and applies current values through `DM_SyncListedQuests` and the explicit story-flag list. Quest status is copied forward; party state does not rewind a character that is ahead. Erased quests use a stored negative value.
- Catch-up does not replay campaign rewards. `DM_ClaimPendingGrants` is a separate character-keyed queue for grants explicitly queued to an existing character, such as while offline.
- This is a latest-status snapshot, not the sequenced event log specified below. Quest/flag coverage is maintained by a hardcoded allowlist. A missed transient change cannot be replayed if the final state alone is insufficient. The `#dm_campaign_checkpoint_*` mirror variables remain account-scoped in Hercules, despite comments calling them character mirrors; alternate characters may share checkpoint display/protection state.

Automatic status inheritance is present in source but remains **unverified in a live two-character session**. The older 20 September acceptance checklist records the late-join scenario as open until it is played.

## State and eligibility

- **Target design, not current implementation:** Keep a durable, ordered **party campaign event log** keyed by campaign/run identity and party identity. Each accepted event has a unique sequence, type (`quest_set`, `quest_complete`, `quest_erase`, `flag_set`/`flag_clear`), quest ID or allowlisted story-flag name, value, actor character ID, and timestamp. A checkpoint *step* is not an event ID: several quest actions can occur between steps. Record the event once, then apply it to enrolled characters; never synthesize it from narrative chat or a client packet.
- Keep an **enrollment record and applied-event cursor per character ID** for that campaign run. Existing `dm_campaign_checkpoint_member` is a starting point but needs an enrollment sequence and per-character cursor (or companion table). When a character joins an active campaign party, automatically enroll it and catch it up to the party's current quest and story state. A temporary disconnect does not unenroll the character. Do not auto-enroll another character merely because its account has a character in the party. Preserve identity across party re-creation without merging two different campaign runs.
- Story quest log, story flags, chapter completion, and story-gated access live on the **character**. Do not use `#` or `##` variables for them or for the per-character checkpoint mirror. Prefer character variables without a prefix or query the character-keyed SQL cursor for display. The party checkpoint coordinates the group; it is not proof that a particular character completed a chapter.
- While a run is active, apply an event immediately to every enrolled online character in the party. A late joiner receives the current campaign quests and story flags as part of joining; do not replay or duplicate past rewards. A temporarily offline party member catches up on reconnect.

## Mutation and recovery contract

**Target design:** Route all DM campaign quest/flag writes through one event-producing helper. Log the typed event before per-character application; apply in sequence and advance that character's cursor only after the quest/flag postcondition is verified. `quest_set` leaves an existing active/completed quest intact; `quest_complete` ensures the quest exists before completing it; `quest_erase` removes it if present; flag events set the explicit value in order. Retries must be idempotent. If SQL logging fails, do not mutate an incomplete subset of the party; report the failure to the DM.

The current login, map-load, and party-rejoin hooks perform snapshot catch-up; they do not validate event cursors or replay a missing event range. **Target design:** On login, map load, party rejoin, and `@dm reconcile preview/confirm`, validate run identity and enrollment, compare the character cursor with the event log, and replay the missing range in order. Preview names eligible/offline/ahead/un-enrolled characters and the quest/flag changes; confirm is needed for any repair beyond ordinary post-disconnect replay. Never rewind a character who is ahead or overwrite a different run. A reset/replay needs an explicit DM operation and audit entry. The client quest log gets its normal server quest-list packets after server reconciliation; `[DMJ]` is presentation, not authority.

Rewards are **not** replayed by quest/flag catch-up. A later reward repair must use stable once-per-character grant keys and an audited preview so reconnect/retry cannot duplicate EXP, Zeny, or items. Personal inventory hand-ins remain attached to the handing-in character.

## Acceptance (not yet live-verified)

1. Two online enrolled characters receive the same DM quest start/complete/erase and branch flag, each in their own quest/character state; neither needs to be the NPC speaker or kill owner.
2. One enrolled character disconnects before two quest changes and a flag change. On return to the same run, only their missing events apply in order; both quest logs and flags then match, and a second sync changes nothing.
3. **Source hooks present; session check open.** A late joiner receives the party's campaign quests and story flags, without duplicate or retroactive rewards. A second character on the same account receives that catch-up only after it joins the party.
4. Party re-creation preserves an enrolled character's run identity and cursor without copying progress to a different character or merging unrelated runs. Ahead/failed/missing-event cases are reported, not silently overwritten.
5. A failed event write leaves all characters unchanged; a failed per-character apply leaves its cursor behind so retry repairs it. Quest/flag sync never duplicates a reward, and normal non-DM quests remain untouched.

Check the SQL migration in isolation, run the campaign script checker and `map-server --run-once`, then perform a live two-client/character-switch test. Existing static checks for `dm_checkpoint.txt` alone do not prove quest replay or character isolation.
