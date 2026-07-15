# Targeted Spec — Bestiary Unlock Persistence

**Parents**: `bestiary-journal.md` (shipped 2026-07-14, session-scoped),
PROJECT_PLAN.md E7 table. **Depends on**: nothing (MVP); E7.1 (`[DMJ]`) for
the server-authoritative phase.

**Purpose**: Bestiary unlocks (`DmCampaignState.bestiary_unlocked`,
`src/dm/mod.rs:19`) currently reset every session — kill a Poring, relog,
and the journal is locked again. Persist them.

## Decision

**Phase 1 (build now): client-local RON file.** Zero server dependencies,
~an afternoon of work, uses the established pattern.
**Phase 2 (after E7.1): server-authoritative, party-wide.** The right
long-term semantics — "the *party* has seen this monster" — only work
server-side. The client file then becomes a cache of the server sync.

## Phase 1 — client-local file

Follow the `window_cache.ron` pattern exactly
(`interface/windows/cache.rs:64-85`: `ron::from_str` on load, best-effort
`ron::ser::to_string_pretty` + logged-not-fatal error on save):

- File: `client/dm_campaign.ron`, structure keyed by character name so alts
  don't share discovery:
  ```ron
  ( characters: { "Wynne": ( bestiary_unlocked: [1002, 1113] ) } )
  ```
- **Load** when the character is known (map-login path, where the
  `QuestList` resync also arrives) — populate `bestiary_unlocked`.
- **Save** on each unlock (the monster-death hook in `lib.rs` that pushes to
  the vec). The file is tiny; unconditional rewrite per unlock is fine, no
  debounce needed. Also save on the DM "reveal all" toggle? **No** — reveal-
  all is a DM view mode, not discovery; persist only real kills.
- Merge rule everywhere: **union** (never remove ids on load/save; a corrupt
  or missing file degrades to session-scoped behavior, never a crash).

## Phase 2 — server-authoritative (design sketch, do not build yet)

- Server: `OnNPCKillEvent`-style hook in `dm_campaign/shared/` appends new
  mob ids to a permanent global (`$dm_bestiary$`, CSV or per-act arrays —
  Hercules global permanent variables; watch the value-length limit, split
  by act if needed). Party-wide by definition: any member's kill unlocks
  for everyone.
- Sync: on map login when `$dm_client_mode` is on, emit
  `[DMJ]{"t":"bestiary_sync","mobs":[…]}`; on kill, echo
  `[DMJ]{"t":"bestiary_unlock","mob":1002}` to the party.
- Client: union server payloads into state + local file (file keeps working
  offline / on fresh installs mid-sync).
- Cleanup rule: `@dmreset` scope decision — campaign reset probably *should*
  wipe the bestiary; put it behind an explicit `@dmreset bestiary` subflag,
  never the default.

**MVP first**: Phase 1 only. It ships before E7.1 exists and nothing about
it is throwaway — Phase 2 reuses the same file as its cache.
