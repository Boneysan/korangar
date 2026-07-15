# Targeted Spec — Campaign Quest Journal (E7.3)

**Parents**: PROJECT_PLAN.md E7.3, DM_INTERFACE.md §9.3 (player window 5),
`plans/modern-mechanics.md` §12, DM_DATA_GUIDE.md. **Depends on**: M1 (done).
**Related**: `navigation-quest-guiding.md` (waypoints/ribbons — out of scope
here), `dm-ui-window-template.md` (window pattern).

**Purpose**: Native quest-log window + HUD tracker for the campaign quests,
replacing the official client's lub-driven quest UI. Also the last port-back
row: it consumes the `QuestAdded`/`QuestRemoved`/`QuestList` network events
that were wired for the headless tester and are currently ignored by the GUI.

## Reality check on data sources (2026-07-15 audit)

- `NetworkEvent::QuestAdded { quest_id, active }`, `QuestRemoved { quest_id }`,
  `QuestList { quest_ids }` already exist
  (`korangar-networking/src/event.rs:266`). No packet work needed for
  active/complete state.
- Quest metadata lives in **`Hercules/db/quest_db.conf`**: 88 campaign
  entries (IDs 20001+), each with `Name` and optional kill `Targets`
  (`MobId`/`Count`). Arc scripts drive state via `setquest`/`completequest`.
- The `planning/campaign_quest_journal_entries.lua` and
  `tools/campaign_quest_merge.py` referenced by DM_INTERFACE.md / earlier
  plans **do not exist**. Do not build against them. Source of truth is
  `quest_db.conf`; long-form flavor text (journal prose per quest) is a new,
  optional sidecar file.

## Data pipeline

Follow the E7 embedded-data pattern (`DM_DATA_GUIDE.md`, `src/dm/data.rs`):

1. New Python parser (alongside the bestiary/items parsers) reads
   `quest_db.conf`, filters `Id >= 20000`, emits `docs/quests.json`:
   `[{ id, name, targets: [{mob_id, count}], arc }]` — arc inferred from the
   `//= See npc/custom/dm_campaign/act_XX/arc_YY_*.txt` comment lines.
2. Optional `docs/quest_flavor.json` (hand-written journal prose keyed by
   quest id); entries without flavor render name + targets only.
3. `src/dm/quests.rs` embeds both via `include_str!` + lazy `OnceLock` parse,
   indexes by quest id. Re-run parser + rebuild after DB changes (same
   workflow as bestiary data).

## State

`DmCampaignState` (`src/dm/mod.rs`) grows:

```rust
pub quest_log: Vec<ActiveQuest>,   // ActiveQuest { quest_id: u32, complete: bool }
pub tracked_quests: Vec<u32>,      // player-pinned, max 5 (HUD tracker)
```

`lib.rs` event loop handles the three quest events (mirror the
monster-death-hook pattern used for `bestiary_unlocked`):
- `QuestList` → replace `quest_log` (sent after map login — this is the
  resync path; tracked list is intersected against it).
- `QuestAdded` → push (respect `active`); toast via chat message for MVP.
- `QuestRemoved` → mark complete if it's a campaign id (the arc scripts
  `completequest` then erase), else drop.

## Windows

**Journal window** (`interface/windows/dm/quest_journal.rs`, player-facing —
not GM-gated): three sections — *Main Campaign*, *Side Quests* (non-20000
ids), *Completed*. Rows show name + kill targets with bestiary names from
`dm_data` indexes; selected row shows flavor text in a detail pane; a
"Track" toggle per row manages `tracked_quests` (cap 5, oldest evicted).
Standard window checklist applies (`dm-ui-window-template.md`): rebuild-per-
frame custom `Element` for the dynamic list, state struct registered in the
3 ClientState places, import the generated `*PathExt` traits, new
`WindowClass` arm in `cache.rs`, launcher on Ctrl+O DM tab **plus** a player
keybind (suggest Ctrl+J) since this window is not DM-only.

**HUD tracker** (Phase 2, after HUD edit mode or pinned to a fixed corner
until then): compact list of `tracked_quests` — name + `(x/y)` per target.

## Kill-count progress (known gap — investigate at build time)

The wired events carry ids only, no hunt counters. Options, in preference
order: (a) promote the hunting-progress packet the server actually sends
(`ZC_HUNTING_QUEST_INFO` 0x08FE family — check a live capture; it may
already be consumed by the length-fallback table), (b) `[DMJ]` echo from an
`OnNPCKillEvent` hook once E7.1 lands. MVP ships without live counters:
states + targets only.

**MVP first**: data pipeline + journal window with active/complete states
from the three events. Then tracked-quest HUD, then kill counters, then
flavor text. Waypoints/ribbons stay in `navigation-quest-guiding.md`.
