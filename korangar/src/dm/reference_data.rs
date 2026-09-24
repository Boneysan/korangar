//! Versioned, player-reference data generated from the active Hercules DB.
//!
//! Kept separate from [`super::data`] so migrating the encyclopedia cannot
//! silently change the legacy DM Bestiary or loot generator.

// These fields are staged for the player Guide, which is a later client
// slice; keep the validated typed model available without warning meanwhile.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde::Deserialize;

const BESTIARY_JSON: &str = include_str!("../../../docs/bestiary.v1.json");
const ITEMS_JSON: &str = include_str!("../../../docs/items.v1.json");
const CARDS_JSON: &str = include_str!("../../../docs/cards.v1.json");
const SKILLS_JSON: &str = include_str!("../../../docs/skills.json");
const JOB_SKILLS_JSON: &str = include_str!("../../../docs/job-skills.v1.json");
const JOB_BONUSES_JSON: &str = include_str!("../../../docs/job-bonuses.v1.json");
const STATUS_REFERENCE_JSON: &str = include_str!("../../../docs/status-effects.v1.json");
const QUESTS_JSON: &str = include_str!("../../../docs/quests.v1.json");

#[derive(Deserialize)]
struct VersionedFile<T> {
    schema_version: u32,
    source_revision: String,
    source_worktree_dirty: bool,
    mode: String,
    entries: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceMonster {
    pub id: u32,
    pub sprite_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub jname: String,
    #[serde(default)]
    pub level: u16,
    #[serde(default)]
    pub hp: u32,
    #[serde(default)]
    pub race: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub element: Option<ReferenceElement>,
    #[serde(default)]
    pub skills: Vec<ReferenceMobSkill>,
    #[serde(default)]
    pub skills_source: Option<ReferenceSource>,
    #[serde(default)]
    pub drops: Vec<ReferenceMonsterDrop>,
    #[serde(default)]
    pub spawn_regions: Vec<ReferenceSpawnRegion>,
    #[serde(default)]
    pub source: Option<ReferenceSource>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceSpawnRegion {
    pub map: String,
    pub kind: String,
    pub spawn_records: u32,
    #[serde(default)]
    pub source: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceElement {
    pub r#type: String,
    pub level: u8,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceMobSkill {
    pub skill_name: String,
    pub level: u8,
    pub rate: i64,
    pub delay_ms: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceMonsterDrop {
    pub item_id: u32,
    pub aegis_name: String,
    #[serde(default)]
    pub name: String,
    pub rate_per_10000: u32,
    pub kind: String,
    #[serde(default)]
    pub source_record: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItem {
    pub id: u32,
    pub aegis_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub buy: u32,
    #[serde(default)]
    pub weight: u32,
    #[serde(default)]
    pub atk: Option<i32>,
    #[serde(default)]
    pub matk: Option<i32>,
    #[serde(default)]
    pub defense: Option<i32>,
    #[serde(default)]
    pub slots: Option<u8>,
    #[serde(default)]
    pub effect_status: String,
    #[serde(default)]
    pub source: Option<ReferenceSource>,
    #[serde(default)]
    pub drops_from: Vec<ReferenceItemDrop>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceSkill {
    #[serde(rename = "Id")]
    pub id: u16,
    #[serde(default, rename = "Name")]
    pub name: String,
    #[serde(default, rename = "Description")]
    pub description: String,
    #[serde(default, rename = "MaxLevel")]
    pub maximum_level: u16,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceJobSkillPrerequisite {
    pub skill_id: u16,
    pub name: String,
    pub level: u16,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceJobSkill {
    pub skill_id: u16,
    pub name: String,
    pub max_level: u16,
    #[serde(default)]
    pub minimum_job_level: u16,
    #[serde(default)]
    pub prerequisites: Vec<ReferenceJobSkillPrerequisite>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceJobSkillTree {
    pub job_id: u16,
    pub tree_name: String,
    pub skills: Vec<ReferenceJobSkill>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceJobBonuses {
    pub job_id: u16,
    pub max_job_level: u16,
    pub total_bonuses: HashMap<String, u16>,
    pub bonus_levels: Vec<ReferenceJobBonusLevel>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceJobBonusLevel {
    pub job_level: u16,
    pub stat: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemDrop {
    pub monster_id: u32,
    pub sprite_name: String,
    pub rate_per_10000: u32,
    pub kind: String,
    #[serde(default)]
    pub source_record: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceSource {
    pub path: String,
    pub record: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceStatus {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub statuses: Vec<ReferenceStatusMechanic>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceStatusMechanic {
    pub constant: String,
    pub id: u32,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub calculation_flags: Vec<String>,
    #[serde(default)]
    pub associated_skill: Option<ReferenceStatusSkill>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceStatusSkill {
    pub id: u16,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceQuest {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub targets: Vec<ReferenceQuestTarget>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceQuestTarget {
    pub mob_id: Option<u32>,
    pub monster_name: String,
    pub monster_data_known: Option<bool>,
    pub count: u32,
    pub level_range: Option<[u16; 2]>,
    pub map_name: Option<String>,
}

pub struct ReferenceData {
    pub source_revision: String,
    pub mode: String,
    pub monsters: Vec<ReferenceMonster>,
    pub items: Vec<ReferenceItem>,
    pub cards: Vec<ReferenceItem>,
    pub skills: Vec<ReferenceSkill>,
    pub job_skill_trees: Vec<ReferenceJobSkillTree>,
    pub job_bonuses: Vec<ReferenceJobBonuses>,
    pub statuses: Vec<ReferenceStatus>,
    pub quests: Vec<ReferenceQuest>,
    monsters_by_id: HashMap<u32, usize>,
    items_by_id: HashMap<u32, usize>,
    cards_by_id: HashMap<u32, usize>,
    skills_by_id: HashMap<u32, usize>,
    job_skill_trees_by_id: HashMap<u16, usize>,
    job_bonuses_by_id: HashMap<u16, usize>,
    quests_by_id: HashMap<u32, usize>,
}

impl ReferenceData {
    fn from_embedded() -> Result<Self, String> {
        let bestiary: VersionedFile<ReferenceMonster> =
            serde_json::from_str(BESTIARY_JSON).map_err(|error| format!("embedded bestiary.v1.json is invalid: {error}"))?;
        let items: VersionedFile<ReferenceItem> =
            serde_json::from_str(ITEMS_JSON).map_err(|error| format!("embedded items.v1.json is invalid: {error}"))?;
        let cards: VersionedFile<ReferenceItem> =
            serde_json::from_str(CARDS_JSON).map_err(|error| format!("embedded cards.v1.json is invalid: {error}"))?;
        let skills: Vec<ReferenceSkill> =
            serde_json::from_str(SKILLS_JSON).map_err(|error| format!("embedded skills.json is invalid: {error}"))?;
        let job_skills: VersionedFile<ReferenceJobSkillTree> =
            serde_json::from_str(JOB_SKILLS_JSON).map_err(|error| format!("embedded job-skills.v1.json is invalid: {error}"))?;
        let job_bonuses: VersionedFile<ReferenceJobBonuses> =
            serde_json::from_str(JOB_BONUSES_JSON).map_err(|error| format!("embedded job-bonuses.v1.json is invalid: {error}"))?;
        let status_reference: VersionedFile<ReferenceStatus> =
            serde_json::from_str(STATUS_REFERENCE_JSON).map_err(|error| format!("embedded status-effects.v1.json is invalid: {error}"))?;
        let quests: VersionedFile<ReferenceQuest> =
            serde_json::from_str(QUESTS_JSON).map_err(|error| format!("embedded quests.v1.json is invalid: {error}"))?;

        if bestiary.schema_version != 1
            || items.schema_version != 1
            || cards.schema_version != 1
            || job_skills.schema_version != 1
            || job_bonuses.schema_version != 1
            || status_reference.schema_version != 1
            || quests.schema_version != 1
        {
            return Err("unsupported embedded reference-data schema version".to_owned());
        }
        if bestiary.source_revision != items.source_revision
            || items.source_revision != cards.source_revision
            || cards.source_revision != job_skills.source_revision
            || job_skills.source_revision != job_bonuses.source_revision
            || job_bonuses.source_revision != status_reference.source_revision
            || status_reference.source_revision != quests.source_revision
        {
            return Err("embedded reference files come from different Hercules revisions".to_owned());
        }
        if bestiary.source_worktree_dirty != items.source_worktree_dirty
            || items.source_worktree_dirty != cards.source_worktree_dirty
            || cards.source_worktree_dirty != job_skills.source_worktree_dirty
            || job_skills.source_worktree_dirty != job_bonuses.source_worktree_dirty
            || job_bonuses.source_worktree_dirty != status_reference.source_worktree_dirty
            || status_reference.source_worktree_dirty != quests.source_worktree_dirty
        {
            return Err("embedded reference files disagree about source worktree status".to_owned());
        }
        if bestiary.mode != items.mode
            || items.mode != cards.mode
            || cards.mode != job_skills.mode
            || job_skills.mode != job_bonuses.mode
            || job_bonuses.mode != status_reference.mode
            || status_reference.mode != quests.mode
        {
            return Err("embedded reference files use different renewal modes".to_owned());
        }

        let monsters_by_id = unique_id_index(&bestiary.entries, "monster")?;
        let items_by_id = unique_id_index(&items.entries, "item")?;
        let cards_by_id = unique_id_index(&cards.entries, "card")?;
        let skills_by_id = unique_id_index(&skills, "skill")?;
        let job_skill_trees_by_id = unique_job_id_index(&job_skills.entries)?;
        let job_bonuses_by_id = unique_job_bonus_id_index(&job_bonuses.entries)?;
        let quests_by_id = unique_quest_id_index(&quests.entries)?;
        let mut statuses = status_reference.entries;
        statuses.sort_by_key(|status| status.id);
        if statuses.is_empty() || statuses.iter().any(|status| status.name.trim().is_empty()) {
            return Err("embedded status reference rows are empty or invalid".to_owned());
        }
        if statuses.len() != 700 {
            return Err(format!("expected 700 status icon rows, found {}", statuses.len()));
        }
        let monster_ids: HashSet<u32> = monsters_by_id.keys().copied().collect();
        let item_ids: HashSet<u32> = items_by_id.keys().copied().collect();

        for quest in &quests.entries {
            for target in &quest.targets {
                if target.count == 0 {
                    return Err(format!("quest {} has an invalid hunt target", quest.id));
                }
                if target.monster_data_known == Some(true) && !target.mob_id.is_some_and(|id| monster_ids.contains(&id)) {
                    return Err(format!("quest {} marks a missing monster as known", quest.id));
                }
            }
        }

        for tree in &job_skills.entries {
            for skill in &tree.skills {
                let Some(&skill_index) = skills_by_id.get(&(skill.skill_id as u32)) else {
                    return Err(format!("job {} links missing skill {}", tree.job_id, skill.skill_id));
                };
                if skills[skill_index].name != skill.name {
                    return Err(format!("job {} skill {} name/ID mismatch", tree.job_id, skill.skill_id));
                }
                for prerequisite in &skill.prerequisites {
                    let Some(&prerequisite_index) = skills_by_id.get(&(prerequisite.skill_id as u32)) else {
                        return Err(format!(
                            "job {} skill {} links missing prerequisite {}",
                            tree.job_id, skill.skill_id, prerequisite.skill_id
                        ));
                    };
                    if skills[prerequisite_index].name != prerequisite.name {
                        return Err(format!(
                            "job {} prerequisite {} name/ID mismatch",
                            tree.job_id, prerequisite.skill_id
                        ));
                    }
                }
            }
        }

        for status in &statuses {
            for mechanic in &status.statuses {
                if let Some(skill) = &mechanic.associated_skill {
                    let Some(&skill_index) = skills_by_id.get(&(skill.id as u32)) else {
                        return Err(format!("status {} links missing skill {}", mechanic.constant, skill.id));
                    };
                    if skills[skill_index].name != skill.name {
                        return Err(format!("status {} skill name/ID mismatch", mechanic.constant));
                    }
                }
            }
        }

        for card in &cards.entries {
            if !item_ids.contains(&card.id) || card.item_type != "IT_CARD" {
                return Err(format!(
                    "card {} is missing from the item category or has the wrong type",
                    card.id
                ));
            }
        }

        let mut monster_links = HashSet::new();
        for monster in &bestiary.entries {
            for drop in &monster.drops {
                if !item_ids.contains(&drop.item_id) {
                    return Err(format!("monster {} links missing item {}", monster.id, drop.item_id));
                }
                monster_links.insert((monster.id, drop.item_id, drop.rate_per_10000, drop.kind.as_str()));
            }
        }
        let mut item_links = HashSet::new();
        for item in &items.entries {
            for drop in &item.drops_from {
                if !monster_ids.contains(&drop.monster_id) {
                    return Err(format!("item {} links missing monster {}", item.id, drop.monster_id));
                }
                item_links.insert((drop.monster_id, item.id, drop.rate_per_10000, drop.kind.as_str()));
            }
        }
        if monster_links != item_links {
            return Err("monster and item drop indexes do not reconcile".to_owned());
        }

        Ok(Self {
            source_revision: bestiary.source_revision,
            mode: bestiary.mode,
            monsters: bestiary.entries,
            items: items.entries,
            cards: cards.entries,
            skills,
            job_skill_trees: job_skills.entries,
            job_bonuses: job_bonuses.entries,
            statuses,
            quests: quests.entries,
            monsters_by_id,
            items_by_id,
            cards_by_id,
            skills_by_id,
            job_skill_trees_by_id,
            job_bonuses_by_id,
            quests_by_id,
        })
    }

    pub fn monster_by_id(&self, id: u32) -> Option<&ReferenceMonster> {
        self.monsters_by_id.get(&id).map(|&index| &self.monsters[index])
    }

    pub fn item_by_id(&self, id: u32) -> Option<&ReferenceItem> {
        self.items_by_id.get(&id).map(|&index| &self.items[index])
    }

    pub fn card_by_id(&self, id: u32) -> Option<&ReferenceItem> {
        self.cards_by_id.get(&id).map(|&index| &self.cards[index])
    }

    pub fn skill_by_id(&self, id: u32) -> Option<&ReferenceSkill> {
        self.skills_by_id.get(&id).map(|&index| &self.skills[index])
    }

    pub fn job_skill_tree_by_id(&self, id: u16) -> Option<&ReferenceJobSkillTree> {
        self.job_skill_trees_by_id.get(&id).map(|&index| &self.job_skill_trees[index])
    }

    pub fn job_bonuses_by_id(&self, id: u16) -> Option<&ReferenceJobBonuses> {
        self.job_bonuses_by_id.get(&id).map(|&index| &self.job_bonuses[index])
    }

    pub fn quest_by_id(&self, id: u32) -> Option<&ReferenceQuest> {
        self.quests_by_id.get(&id).map(|&index| &self.quests[index])
    }

    pub fn search_quests(&self, query: &str, limit: usize) -> Vec<&ReferenceQuest> {
        let query = query.to_lowercase();
        let mut matches: Vec<_> = self
            .quests
            .iter()
            .filter(|quest| query.is_empty() || quest.name.to_lowercase().contains(&query) || quest.id.to_string() == query)
            .collect();
        matches.sort_by_key(|quest| (quest.name.to_lowercase(), quest.id));
        matches.truncate(limit);
        matches
    }

    pub fn search_skills(&self, query: &str, limit: usize) -> Vec<&ReferenceSkill> {
        let query = query.to_lowercase();
        let mut matches: Vec<_> = self
            .skills
            .iter()
            .filter(|skill| {
                query.is_empty()
                    || skill.name.to_lowercase().contains(&query)
                    || skill.description.to_lowercase().contains(&query)
                    || skill.id.to_string() == query
            })
            .collect();
        matches.sort_by_key(|skill| (skill.description.to_lowercase(), skill.id));
        matches.truncate(limit);
        matches
    }

    pub fn search_statuses(&self, query: &str, limit: usize) -> Vec<&ReferenceStatus> {
        let query = query.to_lowercase();
        let mut matches: Vec<_> = self
            .statuses
            .iter()
            .filter(|status| {
                query.is_empty()
                    || status.name.to_lowercase().contains(&query)
                    || status.id.to_string() == query
                    || status.statuses.iter().any(|mechanic| {
                        mechanic.constant.to_lowercase().contains(&query)
                            || mechanic.associated_skill.as_ref().is_some_and(|skill| {
                                skill.name.to_lowercase().contains(&query) || skill.description.to_lowercase().contains(&query)
                            })
                    })
            })
            .collect();
        matches.sort_by_key(|status| (status.name.to_lowercase(), status.id));
        matches.truncate(limit);
        matches
    }

    pub fn search_monsters(&self, query: &str, limit: usize) -> Vec<&ReferenceMonster> {
        let query = query.to_lowercase();
        let mut matches: Vec<_> = self
            .monsters
            .iter()
            .filter(|monster| {
                query.is_empty()
                    || monster.name.to_lowercase().contains(&query)
                    || monster.jname.to_lowercase().contains(&query)
                    || monster.sprite_name.to_lowercase().contains(&query)
            })
            .collect();
        matches.sort_by_key(|monster| (monster.level, monster.id));
        matches.truncate(limit);
        matches
    }

    pub fn search_items(&self, query: &str, limit: usize) -> Vec<&ReferenceItem> {
        let query = query.to_lowercase();
        let mut matches: Vec<_> = self
            .items
            .iter()
            .filter(|item| query.is_empty() || item.name.to_lowercase().contains(&query) || item.aegis_name.to_lowercase().contains(&query))
            .collect();
        matches.sort_by_key(|item| (item.name.to_lowercase(), item.id));
        matches.truncate(limit);
        matches
    }
}

fn unique_quest_id_index(entries: &[ReferenceQuest]) -> Result<HashMap<u32, usize>, String> {
    let mut result = HashMap::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if entry.name.trim().is_empty() || result.insert(entry.id, index).is_some() {
            return Err(format!("invalid or duplicate quest ID {}", entry.id));
        }
    }
    Ok(result)
}

fn unique_id_index<T>(entries: &[T], category: &str) -> Result<HashMap<u32, usize>, String>
where
    T: HasReferenceId,
{
    let mut result = HashMap::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let id = entry.reference_id();
        if result.insert(id, index).is_some() {
            return Err(format!("duplicate {category} ID {id} in embedded reference data"));
        }
    }
    Ok(result)
}

fn unique_job_id_index(entries: &[ReferenceJobSkillTree]) -> Result<HashMap<u16, usize>, String> {
    let mut result = HashMap::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if result.insert(entry.job_id, index).is_some() {
            return Err(format!("duplicate job skill-tree ID {}", entry.job_id));
        }
    }
    Ok(result)
}

fn unique_job_bonus_id_index(entries: &[ReferenceJobBonuses]) -> Result<HashMap<u16, usize>, String> {
    let mut result = HashMap::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if result.insert(entry.job_id, index).is_some() {
            return Err(format!("duplicate job stat-bonus ID {}", entry.job_id));
        }
        let mut calculated_totals = HashMap::<String, u16>::new();
        for bonus in &entry.bonus_levels {
            if bonus.job_level == 0
                || bonus.job_level > entry.max_job_level
                || !["STR", "AGI", "VIT", "INT", "DEX", "LUK"].contains(&bonus.stat.as_str())
            {
                return Err(format!("invalid job stat bonus for job {}", entry.job_id));
            }
            *calculated_totals.entry(bonus.stat.clone()).or_default() += 1;
        }
        if calculated_totals != entry.total_bonuses {
            return Err(format!("job stat totals do not reconcile for job {}", entry.job_id));
        }
    }
    Ok(result)
}

trait HasReferenceId {
    fn reference_id(&self) -> u32;
}

impl HasReferenceId for ReferenceMonster {
    fn reference_id(&self) -> u32 {
        self.id
    }
}

impl HasReferenceId for ReferenceItem {
    fn reference_id(&self) -> u32 {
        self.id
    }
}

impl HasReferenceId for ReferenceSkill {
    fn reference_id(&self) -> u32 {
        self.id as u32
    }
}

/// Versioned reference data, loaded only when an encyclopedia consumer asks
/// for it; the existing DM windows continue using [`super::data::dm_data`].
pub fn reference_data() -> &'static ReferenceData {
    static DATA: OnceLock<ReferenceData> = OnceLock::new();
    DATA.get_or_init(|| ReferenceData::from_embedded().expect("embedded reference data failed validation"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versioned_reference_data_loads_and_reconciles_all_links() {
        let data = reference_data();
        assert_eq!(data.mode, "renewal");
        assert_eq!(data.monsters.len(), 1759);
        assert_eq!(data.items.len(), 13183);
        assert_eq!(data.cards.len(), 1012);
        assert_eq!(data.job_skill_trees.len(), 128);
        assert_eq!(data.job_bonuses.len(), 147);
        assert_eq!(data.quests.len(), 3172);
        assert!(data.source_revision.len() >= 40);

        let poring = data.monster_by_id(1002).expect("Poring exists");
        assert_eq!(poring.sprite_name, "PORING");
        assert_eq!(poring.element.as_ref().map(|element| element.r#type.as_str()), Some("Water"));
        assert!(poring.spawn_regions.iter().any(|region| region.map == "prt_fild08"));
        assert!(poring.spawn_regions.iter().all(|region| !region.source.is_empty()));
        assert!(poring.drops.iter().any(|drop| drop.item_id == 4001 && drop.kind == "normal"));
        assert_eq!(data.card_by_id(4001).map(|card| card.aegis_name.as_str()), Some("Poring_Card"));
        assert!(data.item_by_id(984).is_some_and(|item| item.name == "Oridecon"));
        assert!(
            data.search_monsters("hydra", 10)
                .iter()
                .any(|monster| monster.sprite_name == "HYDRA")
        );
        assert!(data.search_items("oridecon", 100).iter().any(|item| item.id == 984));
        assert!(data.quest_by_id(3401).is_some_and(|quest| quest.name == "Animal Monster Hunt"));
        assert!(data.search_quests("animal monster hunt", 10).iter().any(|quest| quest.id == 3401));

        let knight = data.job_skill_tree_by_id(7).expect("Knight skill tree");
        let bash = knight
            .skills
            .iter()
            .find(|skill| skill.name == "SM_BASH")
            .expect("inherited Bash skill");
        assert_eq!(bash.max_level, 10);
        assert!(
            knight
                .skills
                .iter()
                .any(|skill| skill.prerequisites.iter().any(|entry| entry.skill_id == bash.skill_id))
        );
        let knight_bonuses = data.job_bonuses_by_id(7).expect("Knight job bonuses");
        assert_eq!(knight_bonuses.max_job_level, 50);
        assert_eq!(knight_bonuses.total_bonuses.get("STR"), Some(&8));
        assert!(
            knight_bonuses
                .bonus_levels
                .iter()
                .any(|bonus| bonus.job_level == 4 && bonus.stat == "STR")
        );
    }
}
