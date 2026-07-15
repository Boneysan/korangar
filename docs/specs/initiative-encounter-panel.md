# Targeted Spec — Initiative Tracker & Encounter Panel (E7.5 + E7.8)

**Parents**: PROJECT_PLAN.md E7.5/E7.8, DM_INTERFACE.md §9.3 (DM windows 4–5,
player window 2), `plans/modern-mechanics.md` §11 (data models).
**Depends on**: E7.2 (dice cards, shares the roll-result rendering), E7.1
(`[DMJ]`) for structured sync — text-parse fallback for MVP.
**Consumed by**: `atb-structured-rounds.md` (E7.14) builds directly on the
initiative order payload defined here.

**Purpose**: Two combat-facing surfaces — a turn-order tracker everyone sees,
and a DM-only encounter panel for spawning, scaling, and phase-tracking boss
fights. Together they replace the DM juggling `@monster` ids and mental HP
math mid-session.

## Reality check (2026-07-15 audit)

- **None of the server commands exist.** `@dminitiative`, `@dmencounter`,
  `@dmscale`, `@dmbloodied` appear only in design docs; the bound command
  list (`bindatcmd` across `npc/custom/dm_campaign/shared/`) has 17 commands
  and none of these. `dm_checks.txt` / `dm_combat.txt` are unwritten. This
  spec covers **both sides**; server first — the client windows are command
  generators + echo consumers and have nothing to consume until then.
- The "balance-checker starting percentages" DM_INTERFACE.md cites
  (`planning/mvp-party-balance-checker.md`) **does not exist**. Placeholder
  defaults are defined below; tune at the table and update this spec.

## Server side

**`dm_checks.txt` — `@dminitiative`** (extends the existing `@roll` style in
the dice group): for each online member of `$dm_active_party` roll
`d20 + AGI/10` (readjust divisor at the table; matches the `@dm check`
stat-modifier convention), sort descending, announce one line per combatant
(`"Initiative: Wynne 23 (d20 17 + 6)"` — parseable, see client MVP) and,
when `$dm_client_mode` is on, emit
`[DMJ]{"t":"initiative","order":[{"name":"Wynne","total":23,"roll":17,"mob":false}]}`.
Registered encounter mobs (below) are included with a DM-visible tag.
`@dminitiative clear` ends the round state.

**`dm_combat.txt`**:

- `@dmencounter register <name>` — capture the GIDs of subsequently spawned
  mobs (or `register last` for the most recent `monster()` batch) into a
  registry keyed by encounter name; `list`/`clear` subcommands. Registry is
  wiped by `@dmreset`/`@dmcleanup` (same unconditional-cleanup rule as
  E7.14 — a stale registry must never survive a crashed session).
- `@dmscale <percent>` — for each registered GID, `setunitdata` scaled
  values computed **from the mob's DB base stats, not current** (idempotent:
  `@dmscale 80` twice is still 80%): `UDT_MAXHP`, `UDT_HP` (same ratio as
  current HP so mid-fight scaling doesn't heal), `UDT_ATKMIN/UDT_ATKMAX`,
  `UDT_MATKMIN/UDT_MATKMAX`. Echo `[DMJ]{"t":"scale","pct":80}`.
- `@dmbloodied on|off` — repeating NPC timer polling registered GIDs'
  `getunitdata UDT_HP` vs `UDT_MAXHP`; on crossing 50%, one-shot announce
  + `[DMJ]{"t":"bloodied","gid":…,"name":…}` per mob.

**Starting-percentage defaults** (placeholder until play-tested; the
campaign balances around a 4-player party): 2 players → 60% HP / 75% ATK;
3 → 80/90; 4 → 100/100; 5 → 120/110. The panel seeds its slider from this
table by live party size.

## Client side

**State** (`src/dm/mod.rs` additions, per modern-mechanics §11):

```rust
pub initiative: Vec<InitiativeEntry>, // { name, total, roll, is_mob }
pub initiative_active: Option<usize>, // whose turn (DM-advanced)
pub encounter: EncounterState,        // { mobs: Vec<EncounterMob { gid, name, bloodied }>, scale_pct: u8 }
```

**Initiative tracker** (E7.5, `interface/windows/dm/initiative.rs` + HUD):
ordered list; current-turn highlight; DM view gets up/down reorder buttons
(client-local — the server doesn't care about order once rolled) and a
"Next turn" button advancing `initiative_active` (broadcast via
`[DMJ]` relay or plain chat announce so players' bars advance too). Player
view is a compact read-only HUD bar (roadmap P3/P4: edge placement, never
occluding the combat ring; P10: bounded — scroll past 8 entries).

**Encounter panel** (E7.8, `interface/windows/dm/encounter.rs`, GM-gated
like Bestiary's DM controls): spawn palette reusing `dm_data` bestiary
indexes + the Bestiary window's `@monster` emit (arc-filtered list, one
click = spawn + `@dmencounter register last`); scale slider emitting
`@dmscale <pct>` on release, seeded from the party-size table; per-mob
bloodied indicators; "Complete (manual)" button emitting the encounter-
completion fallback (`@dmbeat` advance or `@dmencounter clear` — decide
with the DM at first use).

Standard window checklist (`dm-ui-window-template.md`): rebuild-per-frame
custom `Element` for both lists, state registered in the 3 ClientState
places, `*PathExt` imports, `WindowClass` arms in `cache.rs`, launchers on
the Ctrl+O DM tab.

## Phasing

- **MVP**: server scripts + both windows, sync via **parsing the plain-text
  announce lines** (dice-cards-style regex; the line formats above are
  chosen to be regex-stable). Works before E7.1 exists.
- **Phase A proper**: switch to `[DMJ]` payloads when E7.1 lands; keep text
  parse as fallback.
- **Phase B / E7.14**: ATB structured rounds reuse `initiative` +
  `initiative_active` unchanged; the encounter panel gains per-combatant
  window overrides. High-frequency HP sync stays native
  (`UpdateEntityHealthPointsPacket`) — bloodied is threshold-detection,
  not streaming.
