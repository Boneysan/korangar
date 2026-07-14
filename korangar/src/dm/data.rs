//! Data-driven DM library: the bestiary, item, and card exports generated
//! from the Hercules DB (`docs/*.json`, regenerated via the Python parsers
//! whenever the server DB changes — see `docs/DM_DATA_GUIDE.md`).
//!
//! The JSON is embedded at compile time so the client needs no runtime data
//! path, and parsed lazily on first use (the DM windows are the only
//! consumers).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

const BESTIARY_JSON: &str = include_str!("../../../docs/bestiary.json");
const ITEMS_JSON: &str = include_str!("../../../docs/items.json");
const CARDS_JSON: &str = include_str!("../../../docs/cards.json");

// The data structs model the full export schema; not every field has a
// consumer yet.
#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BestiaryMonster {
    pub id: u32,
    pub sprite_name: String,
    pub name: String,
    // A few dozen exported entries omit individual stat fields.
    #[serde(default)]
    pub lv: u16,
    #[serde(default)]
    pub hp: u32,
    #[serde(default)]
    pub sp: u32,
    #[serde(default)]
    pub exp: u32,
    #[serde(default, rename = "JExp")]
    pub job_exp: u32,
    #[serde(default)]
    pub attack_range: u16,
    #[serde(default)]
    pub def: i32,
    #[serde(default)]
    pub mdef: i32,
    /// `[attack, attack2]` as exported from the mob DB.
    #[serde(default)]
    pub attack: [i32; 2],
    #[serde(rename = "PhysDPS")]
    pub phys_dps: f32,
    #[serde(rename = "MagicDPS")]
    pub magic_dps: f32,
    pub drops_count: u32,
    pub has_mvp_drops: bool,
    pub mvp_exp: u32,
    // Only present for a handful of exported entries.
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub race: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub view_range: Option<u16>,
    #[serde(default)]
    pub chase_range: Option<u16>,
}

impl BestiaryMonster {
    /// `SCORPION` / `Orange_Potion` style identifiers as a display name.
    pub fn display_name(&self) -> String {
        prettify_identifier(&self.name)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DmItem {
    pub id: u32,
    #[serde(default)]
    pub aegis_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "Type")]
    pub item_type: String,
    #[serde(default)]
    pub buy: u32,
    #[serde(default)]
    pub weight: u32,
    #[serde(default)]
    pub script: String,
    /// `[sprite_name, rate_per_10000]` pairs.
    #[serde(default)]
    pub drops_from: Vec<(String, u32)>,
    #[serde(default)]
    pub atk: Option<i32>,
    #[serde(default)]
    pub matk: Option<i32>,
    #[serde(default)]
    pub def: Option<i32>,
    // `EquipLv` / `Refine` are omitted: the export sometimes emits them as
    // malformed strings (`"[1"`), and nothing here needs them.
    #[serde(default)]
    pub slots: Option<u8>,
}

impl DmItem {
    pub fn display_name(&self) -> String {
        prettify_identifier(&self.name)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DmCard {
    pub id: u32,
    #[serde(default)]
    pub aegis_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub loc: Option<serde_json::Value>,
    #[serde(default)]
    pub script: String,
    #[serde(default)]
    pub drops_from: Vec<(String, u32)>,
}

impl DmCard {
    pub fn display_name(&self) -> String {
        prettify_identifier(&self.name)
    }
}

fn prettify_identifier(identifier: &str) -> String {
    identifier
        .split(['_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().chain(characters.flat_map(char::to_lowercase)).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct DmData {
    pub bestiary: Vec<BestiaryMonster>,
    pub items: Vec<DmItem>,
    pub cards: Vec<DmCard>,
    bestiary_by_id: HashMap<u32, usize>,
    bestiary_by_sprite: HashMap<String, usize>,
    /// Sprite name → indices into `items` that drop from that mob.
    drops_by_sprite: HashMap<String, Vec<usize>>,
}

impl DmData {
    pub fn monster_by_id(&self, id: u32) -> Option<&BestiaryMonster> {
        self.bestiary_by_id.get(&id).map(|&index| &self.bestiary[index])
    }

    pub fn monster_by_sprite(&self, sprite_name: &str) -> Option<&BestiaryMonster> {
        self.bestiary_by_sprite.get(sprite_name).map(|&index| &self.bestiary[index])
    }

    pub fn drops_for_sprite(&self, sprite_name: &str) -> Vec<&DmItem> {
        self.drops_by_sprite
            .get(sprite_name)
            .map(|indices| indices.iter().map(|&index| &self.items[index]).collect())
            .unwrap_or_default()
    }

    /// Case-insensitive substring search over display and sprite names,
    /// sorted by level.
    pub fn search_monsters(&self, query: &str, limit: usize) -> Vec<&BestiaryMonster> {
        let query = query.to_lowercase();
        let mut matches: Vec<&BestiaryMonster> = self
            .bestiary
            .iter()
            .filter(|monster| query.is_empty() || monster.name.to_lowercase().contains(&query))
            .collect();
        matches.sort_by_key(|monster| (monster.lv, monster.id));
        matches.truncate(limit);
        matches
    }
}

/// Embedded DM data, parsed on first access.
pub fn dm_data() -> &'static DmData {
    static DATA: OnceLock<DmData> = OnceLock::new();
    DATA.get_or_init(|| {
        let bestiary: Vec<BestiaryMonster> = serde_json::from_str(BESTIARY_JSON).expect("embedded bestiary.json is valid");
        let items: Vec<DmItem> = serde_json::from_str(ITEMS_JSON).expect("embedded items.json is valid");
        let cards: Vec<DmCard> = serde_json::from_str(CARDS_JSON).expect("embedded cards.json is valid");

        let bestiary_by_id = bestiary.iter().enumerate().map(|(index, monster)| (monster.id, index)).collect();
        let bestiary_by_sprite = bestiary
            .iter()
            .enumerate()
            .map(|(index, monster)| (monster.sprite_name.clone(), index))
            .collect();

        let mut drops_by_sprite: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, item) in items.iter().enumerate() {
            for (sprite_name, _rate) in &item.drops_from {
                drops_by_sprite.entry(sprite_name.clone()).or_default().push(index);
            }
        }

        DmData {
            bestiary,
            items,
            cards,
            bestiary_by_id,
            bestiary_by_sprite,
            drops_by_sprite,
        }
    })
}

#[cfg(test)]
mod dm_data_tests {
    use super::*;

    #[test]
    fn embedded_data_parses_and_indexes() {
        let data = dm_data();
        assert!(data.bestiary.len() > 1500, "bestiary should have ~1759 entries");
        assert!(data.items.len() > 10000, "items should have ~13k entries");
        assert!(data.cards.len() > 900, "cards should have ~1012 entries");

        let poring = data.monster_by_id(1002).expect("Poring exists");
        assert_eq!(poring.sprite_name, "PORING");
        assert_eq!(poring.display_name(), "Poring");

        let results = data.search_monsters("poring", 50);
        assert!(!results.is_empty());
        assert!(results.iter().all(|monster| monster.name.to_lowercase().contains("poring")));
    }

    #[test]
    fn drop_index_links_items_to_mobs() {
        let data = dm_data();
        let drops = data.drops_for_sprite("A_LUNATIC");
        assert!(
            drops.iter().any(|item| item.id == 502),
            "Orange Potion drops from A_LUNATIC in the export"
        );
    }
}
