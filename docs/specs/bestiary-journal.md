# Targeted Spec — Dynamic Bestiary Journal (E7.3 + Roadmap)

**Parents**: FEATURE_ROADMAP.md (Dynamic Bestiary Journal), PROJECT_PLAN.md E7.3, DM_INTERFACE.md §9.3, DM_DATA_GUIDE.md, BESTIARY.md + bestiary.json.

**Purpose**: A native, unlockable monster manual that displays accurate stats, weaknesses, skills, and lore for campaign-relevant mobs. Unlocks via kills or DM Lore checks. Replaces/supplements official quest journal for Seal Cascade content.

**Data Sources** (already generated):
- `docs/bestiary.json` — Primary: Id, Name, Lv, Hp, Stats (STR- LUK), Attack, Def, Mdef, MoveSpeed, AttackDelay, Element, Race (inferred), XP, Skills (with Level/Rate/Delay), PhysDPS/MagicDPS.
- Cross with `cards.json` for "associated cards".
- `items.json` for drop-related lore if needed.
- Static campaign data from `Hercules_RO/npc/custom/dm_campaign/` + planning docs (arcs, which mobs appear).

## Architecture

**Client-side** (`src/dm/` isolated):
- `DmBestiary` state in `ClientState.dm_state`:
  ```rust
  pub struct DmBestiary {
      unlocked: HashSet<u32>,           // mob IDs
      entries: HashMap<u32, BestiaryEntry>, // loaded from JSON + unlocks
  }
  ```
- Load `bestiary.json` at startup (or lazy on first open). Filter to campaign mobs (e.g., IDs from quest data or hardcoded list for arcs 0-4).
- Unlock logic:
  - On `NetworkEvent::RemoveEntity` (death) or health update to 0 → if entity matches bestiary mob, unlock.
  - Or via [DMJ] echo from server "lore_check" or DM command.
  - DM can force-unlock via console.

**UI** (`interface/windows/dm/bestiary_journal.rs` or extend existing quest journal):
- Tab or separate window: "Bestiary".
- List view (by arc/level) + detail view.
- Detail panel:
  - Header: Name, Lv, Sprite (reuse existing sprite loading).
  - Stats table: HP, STR/AGI/VIT/INT/DEX/LUK, ATK, DEF, MDEF, Speed, Element.
  - Weaknesses: Infer from Element + Race (e.g., Fire weak to Water).
  - Skills: List with estimated DPS contribution.
  - Drops: Link to items.json or show common drops.
  - Lore: Static text from campaign planning (or [DMJ]).
- Unlock states: Locked entries show "???" + silhouette. Unlock animation on first discovery.
- Search/filter by level, element, arc.

**Integration**:
- Reuse `item_box`/`skill_box` patterns for drops/skills.
- Reactive via RustState paths on `dm_state.bestiary`.
- In-world: QuestEffect or custom particles on "lore stones" or specific mobs trigger unlock prompt.

## Data Flow

1. Build-time (optional): `build.rs` or `tools/merge_bestiary.py` merges `bestiary.json` + campaign mob list → `src/dm/data/bestiary.ron` (or const).
2. Runtime load → `DmBestiary`.
3. Network events → unlock entries.
4. UI binds to unlocked set + entry data.
5. DM console can query/edit (for testing).

## Implementation Steps (MVP)

1. Add `dm_state.bestiary` to `ClientState` (hidden).
2. Loader: `src/dm/data/bestiary.rs` — load JSON (or RON), filter campaign-relevant.
3. Unlock handler in `lib.rs` (on entity death or specific [DMJ]).
4. Basic window: List of unlocked + detail view (text + stats table).
5. Wire to quest journal or hotkey.
6. Polish: Sprite rendering, inferred weaknesses, search.

## DM-Specific Features

- **Lore checks**: Server can send [DMJ] for "successful Lore check on X" → unlock.
- **DM overrides**: DM window to "reveal" entries for party or force stats.
- **Balancing tie-in**: Display estimated party DPS vs mob (from bestiary PhysDPS).
- **Card synergy**: Show "Drops X Card" from cards.json.

## Risks & Gotchas

- Data sync: Re-run parsers when Hercules mob DB changes.
- Performance: 1700+ entries — load lazily or pre-filter to ~100-200 campaign mobs.
- Client vs server: Bestiary is read-only reference; server is truth for current HP.
- Rebase: All in `src/dm/`.

## Acceptance Criteria

- Journal opens with static + dynamic unlocks.
- Accurate stats from bestiary.json displayed.
- Unlock on kill (or DM command).
- Searchable/filterable by level/element/arc.
- Works in party (unlocks per-character or shared via [DMJ]?).

See `DM_DATA_GUIDE.md` for loading patterns and `BESTIARY.md` for data schema.

**Next**: After E7.2 dice cards, this is high-value for immersion and planning.
