# Targeted Spec — DM Loot & Rewards Generator (E7.6 + Loot Tables)

**Parents**: PROJECT_PLAN.md E7.6, FEATURE_ROADMAP.md (rewards drawer + loot), DM_DATA_GUIDE.md, ITEMS.md + items.json, BESTIARY.md + bestiary.json, DM_INTERFACE.md §9.1 rewards.

**Purpose**: A DM-facing tool (window or console) that generates level-appropriate, balanced rewards/loot based on party level, difficulty, and current session state. Replaces manual `@dmreward` guessing with data-driven suggestions that feed directly into server commands or inventory.

## Core Requirements

- Input: Party level range (from bestiary or player stats), desired difficulty (minor/standard/major), optional filters (item type, specific mob, arc).
- Output: Suggested item list + quantities + total "value".
- Integration: Button to emit `@dmreward ...` chat or (future) direct reward packet.
- Preview: Show estimated party power vs reward power.

## Data Sources

- `bestiary.json`: Mob levels, HP, PhysDPS/MagicDPS (for threat scaling), DropsFrom.
- `items.json`: All items with Buy price (value), Type, Weight, DropsFrom (reverse lookup: which items drop from which level mobs).
- `loot_groups.json`: Box contents (Old Blue Box, etc.) for bulk rewards.
- `cards.json`: For card-specific rewards.

## Architecture

**State** (`src/dm/loot.rs` or in `dm_state`):
```rust
pub struct DmLootState {
    party_level: u16,
    difficulty: Difficulty,  // Minor/Standard/Major
    suggested: Vec<LootSuggestion>,
}

pub struct LootSuggestion {
    item_id: u32,
    name: String,
    quantity: u16,
    value: u32,  // from Buy price
    source: LootSource,  // DirectDrop | Box | Custom
}
```

**UI** (`interface/windows/dm/loot_generator.rs` or rewards drawer):
- Filters: Level slider/range, Difficulty dropdown, Type checkboxes (Healing, Weapon, Card, etc.), "From current arc mobs only".
- Generate button → queries in-memory indexes of the JSONs.
- Results table: Item, Qty, Est. Value, Source mob.
- "Apply Rewards" → builds chat string for `@dmreward` or queues direct add.
- History: Last 5 generated reward sets.

**Generation Logic** (pure client-side, no server call until apply):
1. Get relevant mobs = bestiary.filter(lv in party_range ± delta).
2. Collect candidate drops = items that appear in those mobs' DropsFrom.
3. Score by:
   - Value tier (Buy price buckets: low/med/high for level).
   - Balance: Total reward value ≈ (party_level * difficulty_multiplier * num_players).
   - Variety: Mix healing / gear / cards / ETC.
4. For boxes: Pull from loot_groups.
5. Optional: Bias toward "themed" (e.g., if fighting insects, more insect-related drops).

**DPS / Balance Tie-in**:
- Use bestiary PhysDPS/MagicDPS to estimate encounter difficulty.
- Scale reward value proportionally (harder fight = better loot).

## Implementation Steps

1. Add loading of `items.json` + `bestiary.json` + `loot_groups.json` into `src/dm/data/`.
2. Build in-memory indexes (HashMap<item_id, Item>, HashMap<mob_id, Vec<drop>>).
3. Simple generator function (no UI first).
4. DM window with filters + generate + apply.
5. Wire "Apply" to `networking.send_chat_message` with formatted `@dmreward` (or future direct API).
6. Persist last rewards in dm_state.

## DM Usage

- Pre-encounter: "Generate standard rewards for Lv 40-45 party".
- Post-encounter: Auto-suggest based on actual mobs killed.
- DM override: Manual add any item from the DB.
- Scaling example:
  - Minor: Low Buy consumables + 1-2 cheap cards.
  - Major: Higher tier gear + rare drops + MVP-style items.

## Risks

- Data staleness: Rebuild JSONs when Hercules DB changes.
- Balance tuning: Start conservative; let DM adjust multipliers in UI.
- Performance: 13k items is fine (preload + indexes).
- Client-only: Rewards are suggestions; server still validates via DM_RequireDM.

## Acceptance

- Generator produces sensible lists for given level range.
- "Apply" successfully sends command that server accepts.
- Integrates with existing rewards drawer.
- Uses bestiary level data + item drops for "appropriate" feel.

See `DM_DATA_GUIDE.md` for loading patterns and `ITEMS.md` / `BESTIARY.md` for data schema.

This turns manual reward commands into a powerful, data-backed DM tool.
