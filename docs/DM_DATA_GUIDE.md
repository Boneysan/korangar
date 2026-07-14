# DM Data Integration Guide

**Purpose**: How to use the exported data assets (bestiary, cards, items, loot groups) generated from Hercules_RO for the Seal Cascade DM campaign in Korangar client and tools.

These assets enable:
- Dynamic Bestiary Journal (E7.3 + roadmap "Dynamic Bestiary Journal")
- Encounter balancing / DPS estimation (using monster stats)
- Level-appropriate loot and rewards (E7.6, custom DM rewards)
- Card collection / compounding UI
- Custom DM content (hazards, mobs, items)

**Data Assets** (in `docs/` and usable at runtime or build-time):
- `bestiary.json`: ~1759 monsters with full stats (Lv, HP, STR- LUK, ATK, DEF, MDEF, Speed, Delays, Element, XP, Skills list with rates/delays, PhysDPS/MagicDPS estimates).
- `cards.json`: 1012+ cards with ID, Name, Location (EQP_*), Script (effect), DropsFrom (mobs + chances).
- `items.json`: 13k+ items with Type, Buy price (value proxy), Weight, Atk/Matk/Def etc., Script, DropsFrom.
- `loot_groups.json`: Item groups for boxes/packages (e.g., Old Blue Box contents).
- `BESTIARY.md`, `CARDS.md`, `ITEMS.md`: Human-readable tables.

Generated via Python parsers from `Hercules_RO/db/re/*.conf` (renewal, matching PACKETVER 20190605).

## Integration Points in Client

### 1. Loading the Data
- **Build-time**: Embed processed data (e.g., convert JSON to RON or const Rust structs via build.rs for zero-runtime cost).
- **Runtime**: Load JSON in `src/dm/` (isolated module). Use `serde` (add dep if needed) or simple parser.
  - Example skeleton (in `src/dm/data.rs` or similar):
    ```rust
    use serde::Deserialize;
    #[derive(Deserialize, Debug, Clone)]
    pub struct Monster {
        pub id: u32,
        pub name: String,
        pub lv: u16,
        pub hp: u32,
        // ... full stats
        pub phys_dps: f32,
        pub skills: Vec<SkillInfo>,
    }
    // Load once at startup or map load
    pub fn load_bestiary() -> Vec<Monster> {
        // include_str!("../../docs/bestiary.json") or from archive
        serde_json::from_str(...).unwrap()
    }
    ```
- Store in `ClientState` under `dm_state` or a `DmLibrary` (parallel to `world/library/` for official data). Keep isolated for rebaseability.

### 2. Bestiary Journal (E7.3 + Dynamic Bestiary)
- **Static campaign data**: Merge `campaign_quest_journal_entries.lua` + bestiary (mobs  from campaign arcs).
- **Dynamic unlocks**: Track kills via `UpdateEntityHealth` or death events → unlock in `dm_state.bestiary_unlocked`.
- **UI**: New window or tab in quest journal. Display: name, Lv, HP, stats, element weaknesses (infer from Element), skills (with estimated DPS), lore (from planning docs).
- **Client vs official**: Override/supplement `OngoingQuestInfoList` with campaign mobs. Use bestiary for exact HP/weaknesses post-fight or DM check.
- **Suggestion**: Filter by "unlocked via kill or Lore check". In-world: use QuestEffect or particles for "lore spot" markers.

### 3. Encounter Panel & Balancing (E7.8)
- **Spawn palette**: List mobs by campaign arc + level (from bestiary). DM selects → send `@dmencounter` or spawn via custom packet/event.
- **Scaling**: Use bestiary stats as base. DM tools adjust:
  - HP multiplier
  - ATK (phys/magic separate via DPS columns)
  - Add/remove skills
- **Preview**: In DM console, show estimated party DPS vs mob threat (using player stats + bestiary).
- **Bloodied / phases**: Track via `UpdateEntityHealthPacket` against bestiary max HP.

### 4. Rewards & Loot (E7.6 + Custom)
- **Presets**: `@dmreward` minor/standard/major → map to item tiers from `items.json` (filter by Buy price, type, mob level).
- **Dynamic**: When mob dies (from bestiary), roll from its `DropsFrom` + group tables.
- **Card integration**: From `cards.json`, add card drops to loot or separate "card find" mechanic.
- **Level appropriate**: 
  ```pseudo
  for mob in party_level_range:
      suitable_items = items.filter( |i| i.Buy in range_for_level && dropped_by_mobs_around_level )
  ```
- **Suggestion**: Build a small DM "loot generator" window that queries the JSONs and emits `@dmreward` or directly adds to inventory via custom.

### 5. Cards & Compounding
- Use `cards.json` for a "Card Compendium" window or tooltip.
- Track collected via inventory events.
- UI for compounding: extend equipment window with card slots (using Loc from cards).
- DM: `@dm` commands to grant specific cards.

### 6. Other DM Features
- **Hazards**: Use bestiary for "hazard mob" templates (e.g., stats for custom hazards).
- **Quests/Flags**: Tie campaign quests (from bestiary? or separate) to mob kills.
- **In-world**: Quest effects + particles for "inspect mob for bestiary entry".

## Data Pipeline Suggestions
- **Build-time codegen** (like packet lengths): Script in `build.rs` or `tools/` to generate `src/dm/data/generated.rs` from JSON (const arrays or lazy static).
- **Custom DM content**: Extend with `docs/` overrides or System/ folder merges (per asset-pipeline plan).
- **Sync with server**: When adding custom mobs/items in Hercules, re-run parsers and update JSONs.
- **Client data**: Supplement with GRF Lua (e.g., monster_size_effect for visuals, itemInfo for descriptions). See `RO_OFFICIAL_CLIENT_STRUCTURE.md`.
- **Performance**: Cache lookups (use HashMap by ID). Preload for active campaign arc.

## Implementation Suggestions (for Codex / Team)
- **Isolation**: All in `src/dm/` (data loaders, state: `DmBestiary`, `DmLoot`, parsers) + `interface/windows/dm/`. Never touch core paths directly.
- **State**: Add to `ClientState`:
  ```rust
  dm_state: DmState {
      bestiary_unlocked: HashSet<u32>,
      collected_cards: Vec<u32>,
      // ...
  }
  ```
- **Events**: Extend NetworkEvent or use chat parser for [DMJ] to update (e.g., "mob killed" → unlock).
- **UI patterns**: Reuse `item_box`/`skill_box` for cards/loot display. Use reactive paths.
- **Next quick wins** (after E7.1 server [DMJ]):
  1. ✅ (2026-07-14) Load bestiary.json + basic journal window (static + unlock
     on kill) — `src/dm/data.rs` (embedded via `include_str!`, lazy `OnceLock`
     parse, ID/sprite/drops indexes) + `interface/windows/dm/bestiary.rs`
     (search, ???-locked entries, Reveal all, `@monster` spawn). Unlock hook on
     monster death in `lib.rs`; session-scoped (`DmCampaignState`).
  2. Dice cards from chat parse.
  3. ✅ (2026-07-14) Rewards drawer using items.json + simple filter —
     `src/dm/loot.rs` generator (level ±5 drops, difficulty budgets, type
     buckets) + `interface/windows/dm/loot.rs` (per-row `@item` grants,
     `@dmreward <arc> <tier>` presets). Launchers on the Ctrl+O DM tab.
  4. Encounter panel with bestiary search + stats preview (bestiary window
     covers search/stats/spawn; initiative & scaling still open).
- **Loot table builder**: Standalone tool (or in DM console) that takes party level + desired difficulty → outputs suggested mob + reward list.
- **Testing**: Use the data to simulate encounters (e.g., party DPS vs mob HP/DPS).

## Gaps & Future
- Full client descriptions: Pull from `System/itemInfo_EN.lua` or GRF (merge into JSON?).
- More precise DPS: Port more battle formulas or use runtime simulation.
- Dynamic drops: Server can echo custom loot via [DMJ].
- Bestiary visuals: Tie to RSW objects or effects.
- See `DM_INTERFACE.md` §9.3, `FEATURE_ROADMAP.md` E7, `PROJECT_PLAN.md` E7, `modern-mechanics.md` for bestiary.

## How to Contribute / Next Docs
- Codex: Implement E7.2/E7.3 using this data.
- More help needed: Specs for "BestiaryJournalWindow", "DMEncounterPanel", "DynamicLootFilter". Example code for JSON loading + state update. Enhanced data (e.g., add weakness inferences from Element/Race).

This data turns the DM campaign from chat spam into structured, balanced, replayable tabletop tooling. Start small with dice + journal unlocks for quick player value.

Cross-refs: `DM_CLIENT_IMPLEMENTATION.md`, `DM_SERVER_FUNCTIONS.md`, `BESTIARY.md`, `ITEMS.md`, `CARDS.md`, `CLIENT_SYSTEMS_OVERVIEW.md` §5 (state), `FEATURE_ROADMAP.md`.
