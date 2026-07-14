//! Client-side loot suggestion generator (`docs/specs/dm-loot-generator.md`).
//!
//! Pure suggestion logic: nothing reaches the server until the DM clicks a
//! grant button, which sends ordinary `@item` / `@dmreward` chat commands
//! that Hercules still validates (`DM_RequireDM`).

use super::data::{DmData, DmItem};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LootDifficulty {
    Minor,
    #[default]
    Standard,
    Major,
}

impl LootDifficulty {
    pub fn label(self) -> &'static str {
        match self {
            LootDifficulty::Minor => "minor",
            LootDifficulty::Standard => "standard",
            LootDifficulty::Major => "major",
        }
    }

    /// Zeny-value budget per party level.
    fn budget_per_level(self) -> u32 {
        match self {
            LootDifficulty::Minor => 40,
            LootDifficulty::Standard => 120,
            LootDifficulty::Major => 350,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LootSuggestion {
    pub item_id: u32,
    pub name: String,
    pub quantity: u16,
    /// Total zeny value (`Buy * quantity`).
    pub value: u32,
    /// Sprite name of a mob around the party level that drops the item.
    pub source: String,
}

/// Level-appropriate suggestions: drops of mobs within ±5 levels of the
/// party, spread across item types, spending roughly the difficulty budget.
/// `seed` varies the picks between generations.
pub fn generate_loot(data: &DmData, party_level: u16, difficulty: LootDifficulty, seed: u64) -> Vec<LootSuggestion> {
    const LEVEL_WINDOW: i32 = 5;
    const MAXIMUM_SUGGESTIONS: usize = 6;

    let level = party_level as i32;
    let mobs: Vec<&super::data::BestiaryMonster> = data
        .bestiary
        .iter()
        .filter(|monster| (monster.lv as i32 - level).abs() <= LEVEL_WINDOW)
        .collect();

    // Candidate pool: (item, source sprite), deduplicated by item, priced.
    let mut seen = std::collections::HashSet::new();
    let mut candidates: Vec<(&DmItem, &str)> = Vec::new();
    for monster in &mobs {
        for item in data.drops_for_sprite(&monster.sprite_name) {
            if item.buy > 0 && seen.insert(item.id) {
                candidates.push((item, monster.sprite_name.as_str()));
            }
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }

    // Spread across coarse type buckets so a reward is not six potions.
    let bucket_of = |item: &DmItem| match item.item_type.as_str() {
        "IT_HEALING" | "IT_USABLE" | "IT_DELAYCONSUME" => 0usize,
        "IT_WEAPON" | "IT_ARMOR" => 1,
        "IT_CARD" => 2,
        _ => 3,
    };
    let mut buckets: [Vec<usize>; 4] = Default::default();
    for (index, (item, _)) in candidates.iter().enumerate() {
        buckets[bucket_of(item)].push(index);
    }

    // Cheap xorshift so repeated generation varies without a rand dependency.
    let mut rng_state = seed | 1;
    let mut next_random = move |bound: usize| {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        (rng_state % bound.max(1) as u64) as usize
    };

    let mut budget = party_level as u32 * difficulty.budget_per_level();
    let mut suggestions = Vec::new();
    let mut bucket_order: Vec<usize> = (0..4).filter(|&bucket| !buckets[bucket].is_empty()).collect();

    while suggestions.len() < MAXIMUM_SUGGESTIONS && budget > 0 && !bucket_order.is_empty() {
        let bucket = bucket_order[suggestions.len() % bucket_order.len()];
        let pool = &mut buckets[bucket];
        if pool.is_empty() {
            bucket_order.retain(|&entry| entry != bucket);
            continue;
        }

        // Prefer items the budget can still afford.
        let affordable: Vec<usize> = pool.iter().copied().filter(|&index| candidates[index].0.buy <= budget).collect();
        let Some(&choice) = affordable.get(next_random(affordable.len().max(1))).or(None) else {
            bucket_order.retain(|&entry| entry != bucket);
            continue;
        };
        pool.retain(|&index| index != choice);

        let (item, source) = candidates[choice];
        // Stack cheap consumables, single copies of everything else.
        let quantity = match bucket_of(item) {
            0 => ((budget / 3) / item.buy).clamp(1, 10) as u16,
            _ => 1,
        };
        let value = item.buy * quantity as u32;
        budget = budget.saturating_sub(value);

        suggestions.push(LootSuggestion {
            item_id: item.id,
            name: item.display_name(),
            quantity,
            value,
            source: source.to_owned(),
        });
    }

    suggestions
}

#[cfg(test)]
mod loot_tests {
    use super::super::data::dm_data;
    use super::*;

    #[test]
    fn generates_sensible_suggestions() {
        let data = dm_data();
        let suggestions = generate_loot(data, 40, LootDifficulty::Standard, 12345);
        assert!(!suggestions.is_empty(), "level 40 has plenty of candidate drops");
        assert!(suggestions.len() <= 6);
        for suggestion in &suggestions {
            assert!(suggestion.quantity >= 1);
            assert!(suggestion.value > 0);
            assert!(data.monster_by_sprite(&suggestion.source).is_some());
        }
        // Different seeds vary the picks.
        let other = generate_loot(data, 40, LootDifficulty::Standard, 99999);
        let first_ids: Vec<u32> = suggestions.iter().map(|entry| entry.item_id).collect();
        let other_ids: Vec<u32> = other.iter().map(|entry| entry.item_id).collect();
        assert!(first_ids != other_ids || first_ids.len() <= 1, "seeded variation");
    }
}
