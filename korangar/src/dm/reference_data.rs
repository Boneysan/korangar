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
const REFINE_JSON: &str = include_str!("../../../docs/refine.v1.json");
const SERVER_RULES_JSON: &str = include_str!("../../../docs/server-rules.v1.json");
const NPCS_JSON: &str = include_str!("../../../docs/npcs.v1.json");
const CRAFTING_JSON: &str = include_str!("../../../docs/crafting.v1.json");
const ITEM_GRANTS_JSON: &str = include_str!("../../../docs/item-script-grants.v1.json");
const MAP_FLAGS_JSON: &str = include_str!("../../../docs/map-flags.v1.json");
const COVERAGE_JSON: &str = include_str!("../../../docs/encyclopedia-coverage.v1.json");
const ITEM_EXCHANGES_JSON: &str = include_str!("../../../docs/item-exchanges.v1.json");
const SKILL_FORMULA_REVIEWS_JSON: &str = include_str!("../../../docs/skill-formula-reviews.v1.json");
const NPC_SERVICE_REVIEWS_JSON: &str = include_str!("../../../docs/npc-service-reviews.v1.json");
const RUMORS_JSON: &str = include_str!("../../../docs/rumors.v1.json");
const SEARCH_ALIASES_JSON: &str = include_str!("../../../docs/search-aliases.v1.json");

#[derive(Deserialize)]
struct VersionedFile<T> {
    schema_version: u32,
    source_revision: String,
    source_worktree_dirty: bool,
    mode: String,
    entries: Vec<T>,
    #[serde(default)]
    runtime_clues: Vec<ReferenceRuntimeMapFlagClue>,
    #[serde(default)]
    runtime_reviews: Vec<ReferenceRuntimeMapFlagReview>,
}

#[derive(Deserialize)]
struct VersionedItemGrantFile {
    schema_version: u32,
    source_revision: String,
    source_worktree_dirty: bool,
    mode: String,
    entries: Vec<ReferenceItemGrant>,
    consumptions: Vec<ReferenceItemConsumption>,
}

#[derive(Deserialize)]
struct VersionedItemExchangeFile {
    schema_version: u32,
    source_revision: String,
    source_worktree_dirty: bool,
    mode: String,
    entries: Vec<ReferenceItemExchange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Verified,
    Conditional,
    ConfiguredEstimate,
    SourceClue,
    NotReviewed,
    Unknown,
}

impl EvidenceState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Conditional => "conditional",
            Self::ConfiguredEstimate => "configured estimate",
            Self::SourceClue => "source clue",
            Self::NotReviewed => "not reviewed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReferenceCoverageReport {
    pub schema_version: u32,
    pub source_revision: String,
    pub mode: String,
    pub evidence_states: HashMap<EvidenceState, String>,
    pub categories: HashMap<String, serde_json::Value>,
    pub counting_policy: String,
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
    pub scripted_spawn_references: Vec<ReferenceScriptedSpawn>,
    #[serde(default)]
    pub source: Option<ReferenceSource>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceScriptedSpawn {
    pub spawn_kind: String,
    #[serde(default)]
    pub map: Option<String>,
    #[serde(default)]
    pub map_template: Option<String>,
    #[serde(default)]
    pub map_template_approximate: bool,
    #[serde(default)]
    pub map_expression: String,
    #[serde(default)]
    pub coordinates: Vec<Option<i32>>,
    #[serde(default)]
    pub amount: Option<u32>,
    pub source: String,
    pub availability: String,
}

/// Aggregated spawn records and level range for a map derived from static spawn
/// directives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapSpawnDetails {
    pub records: u64,
    pub mean_level: u16,
    pub min_level: u16,
    pub max_level: u16,
    pub species: usize,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceSpawnRegion {
    pub map: String,
    pub kind: String,
    pub spawn_records: u32,
    #[serde(default)]
    pub listed_monsters: u32,
    #[serde(default)]
    pub placements: Vec<ReferenceSpawnPlacement>,
    #[serde(default)]
    pub source: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceSpawnPlacement {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub random_map_cell: bool,
    pub x_spread: i32,
    pub y_spread: i32,
    pub amount: u32,
    pub source: String,
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
    #[serde(default)]
    pub skill_state: String,
    #[serde(default)]
    pub skill_target: String,
    #[serde(default)]
    pub cast_condition: String,
    #[serde(default)]
    pub condition_data: serde_json::Value,
    #[serde(default)]
    pub value0: serde_json::Value,
    #[serde(default)]
    pub cast_time_ms: i64,
    #[serde(default)]
    pub cancelable: bool,
    #[serde(default)]
    pub trigger_summary: String,
    #[serde(default)]
    pub target_summary: String,
    #[serde(default)]
    pub trigger_translation_status: String,
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
    pub sell: Option<u32>,
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
    pub job: HashMap<String, bool>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub loc: Option<serde_json::Value>,
    #[serde(default, rename = "equiplv")]
    pub equip_level: Option<serde_json::Value>,
    #[serde(default)]
    pub refine: Option<bool>,
    #[serde(default, rename = "weaponlv")]
    pub weapon_level: Option<u8>,
    #[serde(default)]
    pub effect_status: String,
    #[serde(default)]
    pub effect_summary: Option<String>,
    #[serde(default)]
    pub combos: Vec<ReferenceItemCombo>,
    #[serde(default)]
    pub group_contents: Vec<ReferenceItemGroupEntry>,
    #[serde(default)]
    pub contained_in_groups: Vec<ReferenceItemGroupContainer>,
    #[serde(default)]
    pub shops: Vec<ReferenceItemShop>,
    #[serde(default)]
    pub source: Option<ReferenceSource>,
    #[serde(default)]
    pub drops_from: Vec<ReferenceItemDrop>,
}

/// A production recipe (produce_db.txt) or arrow conversion
/// (create_arrow_db.txt)
#[derive(Debug, Deserialize)]
pub struct ReferenceCraftingRecipe {
    #[serde(skip_deserializing)]
    pub entry_kind: String,
    pub output_id: u32,
    pub output_name: String,
    pub output_amount: u32,
    #[serde(default)]
    pub item_level: Option<u8>,
    #[serde(default)]
    pub skill_id: Option<u32>,
    #[serde(default)]
    pub skill_name: Option<String>,
    #[serde(default)]
    pub skill_level: Option<u8>,
    pub materials: Vec<ReferenceCraftingMaterial>,
    #[serde(default)]
    pub combos: Vec<String>,
    pub source: ReferenceSource,
}

/// An item combo entry from item_combo_db.conf
#[derive(Debug, Deserialize)]
pub struct ReferenceItemComboRecipe {
    #[serde(skip_deserializing)]
    pub entry_kind: String,
    pub name: String,
    pub members: Vec<String>,
    pub script: String,
    #[serde(default)]
    pub source: ReferenceSource,
}

/// All crafting entry types
#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum ReferenceCraftingEntry {
    #[serde(rename = "production")]
    Production(ReferenceCraftingRecipe),
    #[serde(rename = "arrow_conversion")]
    ArrowConversion(ReferenceCraftingRecipe),
    #[serde(rename = "combo")]
    Combo(ReferenceItemComboRecipe),
}

#[derive(Debug, Deserialize)]
pub struct ReferenceCraftingMaterial {
    pub item_id: u32,
    pub item_name: String,
    pub amount: i32,
    pub required: bool,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemGrant {
    pub item_id: u32,
    pub item_name: String,
    pub amount: u32,
    pub grant_kind: String,
    #[serde(default)]
    pub npc_clue: Option<ReferenceGrantNpcClue>,
    pub condition_status: String,
    pub source: ReferenceScriptLocation,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemConsumption {
    pub item_id: u32,
    pub item_name: String,
    pub amount: u32,
    #[serde(default)]
    pub npc_clue: Option<ReferenceGrantNpcClue>,
    pub source: ReferenceScriptLocation,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemExchange {
    pub id: String,
    pub title: String,
    pub evidence_state: EvidenceState,
    pub reviewed_by: String,
    pub reviewed_on: String,
    pub npc: ReferenceExchangeNpc,
    #[serde(default)]
    pub inputs: Vec<ReferenceExchangeItem>,
    #[serde(default)]
    pub input_selection: Option<String>,
    pub outcomes: Vec<ReferenceExchangeOutcome>,
    pub conditions: Vec<String>,
    pub source: ReferenceExchangeSource,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceExchangeNpc {
    pub npc_id: u32,
    pub name: String,
    pub internal_name: String,
    pub service_role: String,
    pub map: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceExchangeItem {
    pub item_id: u32,
    pub amount: u32,
    pub item_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceExchangeOutcome {
    pub label: String,
    pub item_ids: Vec<u32>,
    pub amount: u32,
    pub selection: String,
    #[serde(default)]
    pub condition: Option<String>,
    pub items: Vec<ReferenceExchangeOutcomeItem>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceExchangeOutcomeItem {
    pub item_id: u32,
    pub item_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceExchangeSource {
    pub path: String,
    pub lines: Vec<u32>,
    pub reviewed_lines: Vec<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceGrantNpcClue {
    pub name: String,
    pub internal_name: String,
    pub map: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceScriptLocation {
    pub path: String,
    pub line: u32,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceMapFlag {
    pub map: String,
    pub flag: String,
    #[serde(default)]
    pub value: String,
    pub description: String,
    pub effective_static: bool,
    #[serde(default)]
    pub overridden_directive_count: Option<u32>,
    pub source: ReferenceScriptLocation,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceRuntimeMapFlagClue {
    pub operation: String,
    pub map: Option<String>,
    pub map_expression: String,
    pub flag: String,
    #[serde(default)]
    pub value: String,
    pub evidence_state: String,
    pub source: ReferenceScriptLocation,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceRuntimeMapFlagReview {
    pub id: String,
    pub map: String,
    pub title: String,
    pub evidence_state: EvidenceState,
    pub summary: String,
    pub flags: Vec<String>,
    pub conditions: Vec<String>,
    pub reviewed_by: String,
    pub reviewed_on: String,
    pub review_method: String,
    pub sources: Vec<ReferenceRuntimeMapFlagSource>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceRuntimeMapFlagSource {
    pub path: String,
    pub lines: Vec<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemCombo {
    pub members: Vec<ReferenceItemComboMember>,
    pub effect_status: String,
    #[serde(default)]
    pub effect_summary: Option<String>,
    pub source: ReferenceSource,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemComboMember {
    pub id: u32,
    pub aegis_name: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemGroupEntry {
    pub item: ReferenceItemGroupItem,
    pub selection_weight: u32,
    pub total_weight: u32,
    pub selection_chance_percent: f32,
    pub source: ReferenceSource,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemGroupContainer {
    pub container: ReferenceItemGroupItem,
    pub selection_weight: u32,
    pub total_weight: u32,
    pub selection_chance_percent: f32,
    pub source: ReferenceSource,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemGroupItem {
    pub id: u32,
    pub aegis_name: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceItemShop {
    pub npc_name: String,
    pub map: String,
    pub x: i32,
    pub y: i32,
    pub currency: String,
    pub shop_type: String,
    #[serde(default)]
    pub price: Option<u32>,
    #[serde(default)]
    pub uses_item_db_price: bool,
    #[serde(default)]
    pub quantity: Option<u32>,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceSkillPrerequisite {
    #[serde(default, rename = "SkillId")]
    pub skill_id: Option<u16>,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Level")]
    pub level: u16,
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
    #[serde(default, rename = "AttackType")]
    pub attack_type: Option<String>,
    #[serde(default, rename = "StatusChange")]
    pub status_change: Option<String>,
    #[serde(default, rename = "Prerequisites")]
    pub prerequisites: Vec<ReferenceSkillPrerequisite>,
    #[serde(default, rename = "Source")]
    pub source: Option<ReferenceSource>,
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
pub struct ReferenceRefinement {
    pub schema_version: u32,
    pub source_revision: String,
    pub source_worktree_dirty: bool,
    pub mode: String,
    pub max_useful_refine_level: u8,
    pub job_level_bonus_per_job_level_from_50_per_mille: i16,
    pub mechanic_transcendent_flat_bonus_percent: i16,
    pub on_failure: String,
    pub weapon_levels: Vec<ReferenceRefinementWeaponLevel>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceRefinementWeaponLevel {
    pub weapon_level: u8,
    pub material: String,
    pub base_chance_percent_by_target_level: HashMap<String, u16>,
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

impl ReferenceItem {
    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.matches_lowercase_query(&query)
    }

    fn matches_lowercase_query(&self, query: &str) -> bool {
        query.is_empty()
            || self.id.to_string() == query
            || self.name.to_lowercase().contains(&query)
            || self.aegis_name.to_lowercase().contains(&query)
            || self.effect_status.to_lowercase().contains(&query)
            || self
                .effect_summary
                .as_deref()
                .is_some_and(|effect| effect.to_lowercase().contains(&query))
            || self.drops_from.iter().any(|drop| drop.sprite_name.to_lowercase().contains(&query))
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ReferenceSource {
    pub path: String,
    pub record: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceSkillFormulaTarget {
    pub skill_id: u16,
    pub skill_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceSourceCitation {
    pub path: String,
    pub lines: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceSkillFormulaReview {
    pub id: String,
    pub title: String,
    pub skill_ids: Vec<ReferenceSkillFormulaTarget>,
    pub evidence_state: EvidenceState,
    pub reviewed_by: String,
    pub reviewed_on: String,
    pub review_method: String,
    pub formula: String,
    pub worked_example: String,
    pub conditions: Vec<String>,
    pub sources: Vec<ReferenceSourceCitation>,
}

#[derive(Deserialize)]
struct VersionedSkillFormulaReviewsFile {
    schema_version: u32,
    source_revision: String,
    source_worktree_dirty: bool,
    mode: String,
    entries: Vec<ReferenceSkillFormulaReview>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceRumor {
    pub id: u32,
    pub title: String,
    pub category: String,
    pub text: String,
    #[serde(default)]
    pub map_name: Option<String>,
    #[serde(default)]
    pub coordinates: Option<[u16; 2]>,
    pub source_location: String,
    #[serde(default)]
    pub related_monster_id: Option<u32>,
    #[serde(default)]
    pub related_item_id: Option<u32>,
    pub is_story_spoiler: bool,
    pub evidence_state: EvidenceState,
}

#[derive(Deserialize)]
struct VersionedRumorsFile {
    schema_version: u32,
    source_revision: String,
    source_worktree_dirty: bool,
    mode: String,
    entries: Vec<ReferenceRumor>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceNpcServiceLocation {
    pub internal_name: String,
    pub map: String,
    pub x: u16,
    pub y: u16,
    pub npc_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceKafraDestination {
    pub name: String,
    pub fee: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceKafraRouteEntry {
    pub origin_map: String,
    pub destinations: Vec<ReferenceKafraDestination>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceNpcServiceReview {
    pub id: String,
    pub title: String,
    pub service_kind: String,
    pub evidence_state: EvidenceState,
    pub reviewed_by: String,
    pub reviewed_on: String,
    pub review_method: String,
    pub conditions: Vec<String>,
    pub sources: Vec<ReferenceSourceCitation>,
    #[serde(default)]
    pub npc: Option<ReferenceNpcServiceLocation>,
    #[serde(default)]
    pub route_table: Option<Vec<ReferenceKafraRouteEntry>>,
    #[serde(default)]
    pub route_table_note: Option<String>,
    #[serde(default)]
    pub locations: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub location_count: Option<usize>,
}

#[derive(Deserialize)]
struct VersionedNpcServiceReviewsFile {
    schema_version: u32,
    source_revision: String,
    source_worktree_dirty: bool,
    mode: String,
    entries: Vec<ReferenceNpcServiceReview>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceNpc {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub internal_name: String,
    pub map: String,
    pub x: i32,
    pub y: i32,
    pub declared_type: String,
    #[serde(default)]
    pub sprite: String,
    #[serde(default)]
    pub map_known: bool,
    pub source: ReferenceNpcSource,
    #[serde(default)]
    pub offers: Vec<ReferenceNpcOffer>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceNpcOffer {
    pub item_id: u32,
    pub item_name: String,
    pub currency: String,
    pub shop_type: String,
    #[serde(default)]
    pub price: Option<u32>,
    pub uses_item_db_price: bool,
    pub source: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceNpcSource {
    pub path: String,
    pub line: u32,
}

impl ReferenceNpc {
    /// A campaign story NPC: kept out of the open player Guide.
    pub fn is_story(&self) -> bool {
        is_campaign_source_path(&self.source.path)
    }
}

impl ReferenceQuest {
    /// A campaign story quest: kept out of the open player Guide.
    pub fn is_story(&self) -> bool {
        CAMPAIGN_QUEST_IDS.contains(&self.id)
    }
}

#[derive(Debug, Deserialize)]
pub struct ReferenceServerRule {
    pub id: String,
    pub category: String,
    pub title: String,
    pub summary: String,
    pub details: Vec<String>,
    pub sources: Vec<ReferenceSource>,
}

/// An authored player term that leads to an existing Guide target.
#[derive(Debug, Deserialize)]
pub struct ReferenceAlias {
    pub alias: String,
    /// `status` (target is an `SC_` constant) or `job` (target is an exact job
    /// name).
    pub kind: String,
    pub target: String,
    pub basis: String,
}

#[derive(Debug, Deserialize)]
struct VersionedAliasesFile {
    schema_version: u32,
    entries: Vec<ReferenceAlias>,
}

/// Seal Cascade campaign quest ids (GDD 9.5 quest row; `CAMPAIGN.md`).
pub const CAMPAIGN_QUEST_IDS: std::ops::RangeInclusive<u32> = 20000..=20234;

/// Campaign scripts live under this tree. Everything the Seal Cascade story
/// declares there - hub NPCs, hidden set-piece NPCs, quest givers - is plot.
pub const CAMPAIGN_SCRIPT_PREFIX: &str = "npc/custom/dm_campaign/";

/// Whether a source path belongs to the campaign story rather than the
/// server's general content.
pub fn is_campaign_source_path(path: &str) -> bool {
    path.starts_with(CAMPAIGN_SCRIPT_PREFIX)
}

/// Whether a typed query reaches `alias`: exactly, or - for queries of four
/// letters or more - as a prefix, so "petrif" finds "petrification" while a
/// two-letter abbreviation never matches by accident.
pub fn alias_matches(alias: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    let alias = alias.to_lowercase();
    alias == query || (query.chars().count() >= 4 && alias.starts_with(&query))
}

#[derive(Debug, Deserialize)]
pub struct ReferenceStatus {
    pub id: u32,
    pub name: String,
    /// A server status with no client icon (the classic ailments). Its name
    /// is derived from the server constant, and its id sits above every icon
    /// id.
    #[serde(default)]
    pub iconless: bool,
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
    /// Skills whose explicit Hercules `StatusChange` field names this status.
    #[serde(default)]
    pub status_change_skills: Vec<ReferenceStatusSkill>,
    /// Literal C call sites; this is a source index, not an exhaustive source
    /// list.
    #[serde(default)]
    pub code_call_sites: Vec<ReferenceStatusCallSite>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceStatusCallSite {
    pub path: String,
    pub line: u32,
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
    #[serde(default)]
    pub npc_references: Vec<ReferenceQuestNpc>,
    #[serde(default)]
    pub item_reward_candidates: Vec<ReferenceQuestRewardCandidate>,
    #[serde(default)]
    pub flow_review: Option<ReferenceQuestFlowReview>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceQuestFlowReview {
    pub id: String,
    pub title: String,
    pub evidence_state: EvidenceState,
    pub reviewed_by: String,
    pub reviewed_on: String,
    pub review_method: String,
    pub conditions: Vec<String>,
    pub sources: Vec<ReferenceQuestFlowSource>,
    #[serde(default)]
    pub verified_item_rewards: Vec<ReferenceQuestVerifiedItemReward>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceQuestVerifiedItemReward {
    pub item_id: u32,
    pub item_name: String,
    pub amount: u32,
    pub explanation: String,
    pub source_path: String,
    pub source_line: u32,
    pub evidence_state: EvidenceState,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceQuestFlowSource {
    pub path: String,
    pub lines: Vec<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceQuestRewardCandidate {
    pub item_id: u32,
    pub item_name: String,
    pub amount: u32,
    pub source_path: String,
    pub source_line: u32,
    pub nearby_completequest_line: u32,
    pub distance_lines: i32,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceQuestNpc {
    pub name: String,
    pub map_name: String,
    pub x: u16,
    pub y: u16,
    pub source_path: String,
    pub source_line: u32,
    #[serde(default)]
    pub uses: Vec<String>,
    #[serde(default)]
    pub reviewed_role: Option<String>,
    #[serde(default)]
    pub reviewed_source_lines: Vec<u32>,
    #[serde(default)]
    pub review_evidence: Option<String>,
    #[serde(default)]
    pub verified_reward: Option<ReferenceQuestVerifiedReward>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceQuestVerifiedReward {
    pub base_exp: u32,
    pub item_id: u32,
    pub item_amount: u32,
    pub status: String,
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
    pub coverage_report: ReferenceCoverageReport,
    pub monsters: Vec<ReferenceMonster>,
    pub items: Vec<ReferenceItem>,
    pub cards: Vec<ReferenceItem>,
    pub skills: Vec<ReferenceSkill>,
    pub skill_formula_reviews: Vec<ReferenceSkillFormulaReview>,
    pub job_skill_trees: Vec<ReferenceJobSkillTree>,
    pub job_bonuses: Vec<ReferenceJobBonuses>,
    pub refinement: ReferenceRefinement,
    pub server_rules: Vec<ReferenceServerRule>,
    pub statuses: Vec<ReferenceStatus>,
    pub quests: Vec<ReferenceQuest>,
    pub npcs: Vec<ReferenceNpc>,
    pub crafting_entries: Vec<ReferenceCraftingEntry>,
    pub item_grants: Vec<ReferenceItemGrant>,
    pub item_consumptions: Vec<ReferenceItemConsumption>,
    pub item_exchanges: Vec<ReferenceItemExchange>,
    pub npc_services: Vec<ReferenceNpcServiceReview>,
    pub rumors: Vec<ReferenceRumor>,
    pub aliases: Vec<ReferenceAlias>,
    pub map_flags: Vec<ReferenceMapFlag>,
    pub runtime_map_flag_clues: Vec<ReferenceRuntimeMapFlagClue>,
    pub runtime_map_flag_reviews: Vec<ReferenceRuntimeMapFlagReview>,
    monsters_by_id: HashMap<u32, usize>,
    items_by_id: HashMap<u32, usize>,
    cards_by_id: HashMap<u32, usize>,
    skills_by_id: HashMap<u32, usize>,
    job_skill_trees_by_id: HashMap<u16, usize>,
    job_bonuses_by_id: HashMap<u16, usize>,
    quests_by_id: HashMap<u32, usize>,
    npc_services_by_id: HashMap<String, usize>,
    rumors_by_id: HashMap<u32, usize>,
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
        let refinement: ReferenceRefinement =
            serde_json::from_str(REFINE_JSON).map_err(|error| format!("embedded refine.v1.json is invalid: {error}"))?;
        let server_rules: VersionedFile<ReferenceServerRule> =
            serde_json::from_str(SERVER_RULES_JSON).map_err(|error| format!("embedded server-rules.v1.json is invalid: {error}"))?;
        let npcs: VersionedFile<ReferenceNpc> =
            serde_json::from_str(NPCS_JSON).map_err(|error| format!("embedded npcs.v1.json is invalid: {error}"))?;
        let crafting: VersionedFile<ReferenceCraftingEntry> =
            serde_json::from_str(CRAFTING_JSON).map_err(|error| format!("embedded crafting.v1.json is invalid: {error}"))?;
        let item_grants: VersionedItemGrantFile =
            serde_json::from_str(ITEM_GRANTS_JSON).map_err(|error| format!("embedded item-script-grants.v1.json is invalid: {error}"))?;
        let map_flags: VersionedFile<ReferenceMapFlag> =
            serde_json::from_str(MAP_FLAGS_JSON).map_err(|error| format!("embedded map-flags.v1.json is invalid: {error}"))?;
        let coverage_report: ReferenceCoverageReport =
            serde_json::from_str(COVERAGE_JSON).map_err(|error| format!("embedded encyclopedia-coverage.v1.json is invalid: {error}"))?;
        let item_exchanges: VersionedItemExchangeFile =
            serde_json::from_str(ITEM_EXCHANGES_JSON).map_err(|error| format!("embedded item-exchanges.v1.json is invalid: {error}"))?;
        let formula_reviews: VersionedSkillFormulaReviewsFile = serde_json::from_str(SKILL_FORMULA_REVIEWS_JSON)
            .map_err(|error| format!("embedded skill-formula-reviews.v1.json is invalid: {error}"))?;
        let npc_services: VersionedNpcServiceReviewsFile = serde_json::from_str(NPC_SERVICE_REVIEWS_JSON)
            .map_err(|error| format!("embedded npc-service-reviews.v1.json is invalid: {error}"))?;
        let aliases: VersionedAliasesFile =
            serde_json::from_str(SEARCH_ALIASES_JSON).map_err(|error| format!("embedded search-aliases.v1.json is invalid: {error}"))?;
        let rumors: VersionedRumorsFile =
            serde_json::from_str(RUMORS_JSON).map_err(|error| format!("embedded rumors.v1.json is invalid: {error}"))?;

        if bestiary.schema_version != 1
            || items.schema_version != 1
            || cards.schema_version != 1
            || job_skills.schema_version != 1
            || job_bonuses.schema_version != 1
            || status_reference.schema_version != 1
            || quests.schema_version != 1
            || refinement.schema_version != 1
            || server_rules.schema_version != 1
            || npcs.schema_version != 1
            || crafting.schema_version != 1
            || item_grants.schema_version != 1
            || map_flags.schema_version != 1
            || coverage_report.schema_version != 1
            || item_exchanges.schema_version != 1
            || formula_reviews.schema_version != 1
            || npc_services.schema_version != 1
            || rumors.schema_version != 1
        {
            return Err("unsupported embedded reference-data schema version".to_owned());
        }
        if bestiary.source_revision != items.source_revision
            || items.source_revision != cards.source_revision
            || cards.source_revision != job_skills.source_revision
            || job_skills.source_revision != job_bonuses.source_revision
            || job_bonuses.source_revision != status_reference.source_revision
            || status_reference.source_revision != quests.source_revision
            || refinement.source_revision != quests.source_revision
            || server_rules.source_revision != quests.source_revision
            || npcs.source_revision != quests.source_revision
            || crafting.source_revision != quests.source_revision
            || item_grants.source_revision != quests.source_revision
            || map_flags.source_revision != quests.source_revision
            || coverage_report.source_revision != quests.source_revision
            || item_exchanges.source_revision != quests.source_revision
            || formula_reviews.source_revision != quests.source_revision
            || npc_services.source_revision != quests.source_revision
            || rumors.source_revision != quests.source_revision
        {
            return Err("embedded reference files come from different Hercules revisions".to_owned());
        }
        if bestiary.source_worktree_dirty != items.source_worktree_dirty
            || items.source_worktree_dirty != cards.source_worktree_dirty
            || cards.source_worktree_dirty != job_skills.source_worktree_dirty
            || job_skills.source_worktree_dirty != job_bonuses.source_worktree_dirty
            || job_bonuses.source_worktree_dirty != status_reference.source_worktree_dirty
            || status_reference.source_worktree_dirty != quests.source_worktree_dirty
            || refinement.source_worktree_dirty != quests.source_worktree_dirty
            || server_rules.source_worktree_dirty != quests.source_worktree_dirty
            || npcs.source_worktree_dirty != quests.source_worktree_dirty
            || crafting.source_worktree_dirty != quests.source_worktree_dirty
            || item_grants.source_worktree_dirty != quests.source_worktree_dirty
            || map_flags.source_worktree_dirty != quests.source_worktree_dirty
            || item_exchanges.source_worktree_dirty != quests.source_worktree_dirty
            || formula_reviews.source_worktree_dirty != quests.source_worktree_dirty
            || npc_services.source_worktree_dirty != quests.source_worktree_dirty
            || rumors.source_worktree_dirty != quests.source_worktree_dirty
        {
            return Err("embedded reference files disagree about source worktree status".to_owned());
        }
        if bestiary.mode != items.mode
            || items.mode != cards.mode
            || cards.mode != job_skills.mode
            || job_skills.mode != job_bonuses.mode
            || job_bonuses.mode != status_reference.mode
            || status_reference.mode != quests.mode
            || refinement.mode != quests.mode
            || server_rules.mode != quests.mode
            || npcs.mode != quests.mode
            || crafting.mode != quests.mode
            || item_grants.mode != quests.mode
            || map_flags.mode != quests.mode
            || coverage_report.mode != quests.mode
            || item_exchanges.mode != quests.mode
            || formula_reviews.mode != quests.mode
            || npc_services.mode != quests.mode
            || rumors.mode != quests.mode
        {
            return Err("embedded reference files use different renewal modes".to_owned());
        }
        if coverage_report.evidence_states.len() != 6 {
            return Err("encyclopedia coverage report has an incomplete evidence-state vocabulary".to_owned());
        }

        let monsters_by_id = unique_id_index(&bestiary.entries, "monster")?;
        let items_by_id = unique_id_index(&items.entries, "item")?;
        let cards_by_id = unique_id_index(&cards.entries, "card")?;
        let skills_by_id = unique_id_index(&skills, "skill")?;
        let job_skill_trees_by_id = unique_job_id_index(&job_skills.entries)?;
        let job_bonuses_by_id = unique_job_bonus_id_index(&job_bonuses.entries)?;
        let quests_by_id = unique_quest_id_index(&quests.entries)?;
        if server_rules.entries.is_empty()
            || server_rules.entries.iter().any(|rule| {
                rule.id.trim().is_empty()
                    || rule.title.trim().is_empty()
                    || rule.summary.trim().is_empty()
                    || rule.details.is_empty()
                    || rule.sources.is_empty()
            })
        {
            return Err("embedded server-rule references are empty or invalid".to_owned());
        }
        if refinement.max_useful_refine_level == 0
            || refinement.weapon_levels.len() != 4
            || refinement.weapon_levels.iter().any(|row| {
                row.weapon_level == 0
                    || row.weapon_level > 4
                    || row.base_chance_percent_by_target_level.len() != refinement.max_useful_refine_level as usize
                    || (1..=refinement.max_useful_refine_level)
                        .any(|level| !row.base_chance_percent_by_target_level.contains_key(&level.to_string()))
            })
        {
            return Err("embedded refine reference rows are empty or invalid".to_owned());
        }
        let mut statuses = status_reference.entries;
        statuses.sort_by_key(|status| status.id);
        if statuses.is_empty() || statuses.iter().any(|status| status.name.trim().is_empty()) {
            return Err("embedded status reference rows are empty or invalid".to_owned());
        }
        let icon_rows = statuses.iter().filter(|status| !status.iconless).count();
        if icon_rows != 700 {
            return Err(format!("expected 700 status icon rows, found {icon_rows}"));
        }
        if statuses
            .iter()
            .any(|status| status.iconless && (status.id < 100_000 || status.statuses.len() != 1))
        {
            return Err("an iconless status row must have an id of 100000 or more and exactly one server status".to_owned());
        }
        if aliases.schema_version != 1 {
            return Err("unsupported search-aliases.v1.json schema".to_owned());
        }
        for alias in &aliases.entries {
            let resolves = match alias.kind.as_str() {
                "status" => statuses
                    .iter()
                    .any(|status| status.statuses.iter().any(|mechanic| mechanic.constant == alias.target)),
                // Job names live in the client's job table, not in reference
                // data; the guide tests resolve these.
                "job" => !alias.target.trim().is_empty(),
                other => return Err(format!("search alias {:?} has unknown kind {other:?}", alias.alias)),
            };
            if alias.alias.trim().is_empty() || alias.basis.trim().is_empty() || !resolves {
                return Err(format!(
                    "search alias {:?} -> {:?} ({}) is empty, has no basis, or names a target that does not exist",
                    alias.alias, alias.target, alias.kind
                ));
            }
        }
        let monster_ids: HashSet<u32> = monsters_by_id.keys().copied().collect();
        let item_ids: HashSet<u32> = items_by_id.keys().copied().collect();
        for rumor in &rumors.entries {
            if rumor.related_monster_id.is_some_and(|id| !monster_ids.contains(&id))
                || rumor.related_item_id.is_some_and(|id| !item_ids.contains(&id))
            {
                return Err(format!(
                    "rumor {} links a monster or item that is not in the Guide data (monster {:?}, item {:?})",
                    rumor.id, rumor.related_monster_id, rumor.related_item_id
                ));
            }
        }

        for quest in &quests.entries {
            if quest.npc_references.iter().any(|npc| {
                npc.name.trim().is_empty()
                    || npc.map_name.trim().is_empty()
                    || npc.source_path.trim().is_empty()
                    || npc.source_line == 0
                    || npc
                        .reviewed_role
                        .as_deref()
                        .is_some_and(|role| !matches!(role, "offer" | "turn_in"))
                    || npc.reviewed_role.is_some()
                        != (npc.review_evidence.as_deref().is_some_and(|note| !note.trim().is_empty())
                            && !npc.reviewed_source_lines.is_empty()
                            && npc.reviewed_source_lines.iter().all(|line| *line > 0))
                    || npc.verified_reward.as_ref().is_some_and(|reward| {
                        reward.status != "verified_static_helper_reward"
                            || reward.base_exp == 0
                            || reward.item_amount == 0
                            || !item_ids.contains(&reward.item_id)
                            || npc.reviewed_role.as_deref() != Some("turn_in")
                    })
            }) {
                return Err(format!("quest {} has an invalid NPC script reference", quest.id));
            }
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
                for skill in &mechanic.status_change_skills {
                    let Some(&skill_index) = skills_by_id.get(&(skill.id as u32)) else {
                        return Err(format!(
                            "status {} links missing StatusChange skill {}",
                            mechanic.constant, skill.id
                        ));
                    };
                    if skills[skill_index].name != skill.name {
                        return Err(format!("status {} StatusChange skill name/ID mismatch", mechanic.constant));
                    }
                }
                for source in &mechanic.code_call_sites {
                    if source.path.is_empty() || source.line == 0 {
                        return Err(format!("status {} has an invalid C call-site reference", mechanic.constant));
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
        for quest in &quests.entries {
            for candidate in &quest.item_reward_candidates {
                if !item_ids.contains(&candidate.item_id) || candidate.source_path.trim().is_empty() || candidate.source_line == 0 {
                    return Err(format!("quest {} has an invalid item reward candidate", quest.id));
                }
            }
        }
        for npc in &npcs.entries {
            if npc.name.trim().is_empty() || npc.map.trim().is_empty() || npc.source.path.trim().is_empty() || npc.source.line == 0 {
                return Err(format!("NPC declaration {} has invalid provenance", npc.id));
            }
            for offer in &npc.offers {
                if !item_ids.contains(&offer.item_id) || offer.source.trim().is_empty() {
                    return Err(format!("NPC declaration {} links missing item {}", npc.id, offer.item_id));
                }
            }
        }
        for exchange in &item_exchanges.entries {
            if exchange.id.trim().is_empty()
                || exchange.title.trim().is_empty()
                || exchange.source.path.trim().is_empty()
                || exchange.source.lines.is_empty()
                || exchange.source.lines.iter().any(|line| *line == 0)
                || exchange.source.reviewed_lines != exchange.source.lines
            {
                return Err(format!("reviewed item exchange {} has invalid provenance", exchange.id));
            }
            if !npcs.entries.iter().any(|npc| {
                npc.id == exchange.npc.npc_id
                    && npc.map.eq_ignore_ascii_case(&exchange.npc.map)
                    && npc.x == exchange.npc.x
                    && npc.y == exchange.npc.y
                    && (npc.internal_name == exchange.npc.internal_name || npc.name == exchange.npc.internal_name)
            }) {
                return Err(format!("reviewed item exchange {} links a missing NPC", exchange.id));
            }
            for item_id in exchange
                .inputs
                .iter()
                .map(|item| item.item_id)
                .chain(exchange.outcomes.iter().flat_map(|outcome| outcome.item_ids.iter().copied()))
            {
                if !item_ids.contains(&item_id) {
                    return Err(format!("reviewed item exchange {} links missing item {item_id}", exchange.id));
                }
            }
            if exchange.evidence_state == EvidenceState::Conditional && exchange.conditions.is_empty() {
                return Err(format!("conditional item exchange {} has no listed conditions", exchange.id));
            }
        }

        for review in &formula_reviews.entries {
            for target in &review.skill_ids {
                let Some(&skill_index) = skills_by_id.get(&(target.skill_id as u32)) else {
                    return Err(format!(
                        "skill formula review {} links missing skill {}",
                        review.id, target.skill_id
                    ));
                };
                if skills[skill_index].name != target.skill_name {
                    return Err(format!("skill formula review {} skill name/ID mismatch", review.id));
                }
            }
        }

        let npc_services_by_id = npc_services
            .entries
            .iter()
            .enumerate()
            .map(|(index, service)| (service.id.clone(), index))
            .collect::<HashMap<_, _>>();
        let rumors_by_id = rumors
            .entries
            .iter()
            .enumerate()
            .map(|(index, rumor)| (rumor.id, index))
            .collect::<HashMap<_, _>>();

        Ok(Self {
            source_revision: bestiary.source_revision,
            mode: bestiary.mode,
            coverage_report,
            monsters: bestiary.entries,
            items: items.entries,
            cards: cards.entries,
            skills,
            skill_formula_reviews: formula_reviews.entries,
            job_skill_trees: job_skills.entries,
            job_bonuses: job_bonuses.entries,
            refinement,
            server_rules: server_rules.entries,
            statuses,
            quests: quests.entries,
            npcs: npcs.entries,
            crafting_entries: crafting.entries,
            item_grants: item_grants.entries,
            item_consumptions: item_grants.consumptions,
            item_exchanges: item_exchanges.entries,
            npc_services: npc_services.entries,
            rumors: rumors.entries,
            aliases: aliases.entries,
            map_flags: map_flags.entries,
            runtime_map_flag_clues: map_flags.runtime_clues,
            runtime_map_flag_reviews: map_flags.runtime_reviews,
            monsters_by_id,
            items_by_id,
            cards_by_id,
            skills_by_id,
            job_skill_trees_by_id,
            job_bonuses_by_id,
            quests_by_id,
            npc_services_by_id,
            rumors_by_id,
        })
    }

    #[allow(dead_code)]
    pub fn npc_service_by_id(&self, id: &str) -> Option<&ReferenceNpcServiceReview> {
        self.npc_services_by_id.get(id).and_then(|&index| self.npc_services.get(index))
    }

    #[allow(dead_code)]
    pub fn rumor_by_id(&self, id: u32) -> Option<&ReferenceRumor> {
        self.rumors_by_id.get(&id).and_then(|&index| self.rumors.get(index))
    }

    #[allow(dead_code)]
    pub fn npc_service_by_index(&self, index: usize) -> Option<&ReferenceNpcServiceReview> {
        self.npc_services.get(index)
    }

    #[allow(dead_code)]
    pub fn npc_service_index_by_id(&self, id: &str) -> Option<usize> {
        self.npc_services_by_id.get(id).copied()
    }

    #[allow(dead_code)]
    pub fn services_for_map(&self, map_name: &str) -> Vec<&ReferenceNpcServiceReview> {
        self.npc_services
            .iter()
            .filter(|service| {
                service.npc.as_ref().is_some_and(|n| n.map.eq_ignore_ascii_case(map_name))
                    || service
                        .route_table
                        .as_ref()
                        .is_some_and(|routes| routes.iter().any(|r| r.origin_map.eq_ignore_ascii_case(map_name)))
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn rumors_for_map(&self, map_name: &str) -> Vec<&ReferenceRumor> {
        self.rumors
            .iter()
            .filter(|rumor| rumor.map_name.as_deref().is_some_and(|m| m.eq_ignore_ascii_case(map_name)))
            .collect()
    }

    #[allow(dead_code)]
    pub fn rumors_for_monster(&self, monster_id: u32) -> Vec<&ReferenceRumor> {
        self.rumors
            .iter()
            .filter(|rumor| rumor.related_monster_id == Some(monster_id))
            .collect()
    }

    #[allow(dead_code)]
    pub fn rumors_for_item(&self, item_id: u32) -> Vec<&ReferenceRumor> {
        self.rumors.iter().filter(|rumor| rumor.related_item_id == Some(item_id)).collect()
    }

    pub fn search_services(&self, query: &str, limit: usize) -> Vec<&ReferenceNpcServiceReview> {
        let query = query.to_lowercase();
        self.npc_services
            .iter()
            .filter(|service| {
                query.is_empty()
                    || service.id.to_lowercase().contains(&query)
                    || service.title.to_lowercase().contains(&query)
                    || service.service_kind.to_lowercase().contains(&query)
                    || service.conditions.iter().any(|c| c.to_lowercase().contains(&query))
                    || service.npc.as_ref().is_some_and(|n| {
                        n.map.to_lowercase().contains(&query)
                            || n.internal_name.to_lowercase().contains(&query)
                            || n.npc_id.to_string() == query
                    })
                    || service.route_table.as_ref().is_some_and(|routes| {
                        routes.iter().any(|r| {
                            r.origin_map.to_lowercase().contains(&query)
                                || r.destinations.iter().any(|d| d.name.to_lowercase().contains(&query))
                        })
                    })
            })
            .take(limit)
            .collect()
    }

    pub fn search_rumors(&self, query: &str, limit: usize) -> Vec<&ReferenceRumor> {
        let query = query.to_lowercase();
        self.rumors
            .iter()
            .filter(|rumor| {
                query.is_empty()
                    || rumor.id.to_string() == query
                    || rumor.title.to_lowercase().contains(&query)
                    || rumor.category.to_lowercase().contains(&query)
                    || rumor.text.to_lowercase().contains(&query)
                    || rumor.source_location.to_lowercase().contains(&query)
                    || rumor.map_name.as_ref().is_some_and(|m| m.to_lowercase().contains(&query))
                    || rumor.related_monster_id.is_some_and(|id| {
                        id.to_string() == query
                            || self
                                .monster_by_id(id)
                                .is_some_and(|m| m.name.to_lowercase().contains(&query) || m.sprite_name.to_lowercase().contains(&query))
                    })
                    || rumor.related_item_id.is_some_and(|id| {
                        id.to_string() == query
                            || self
                                .item_by_id(id)
                                .is_some_and(|i| i.name.to_lowercase().contains(&query) || i.aegis_name.to_lowercase().contains(&query))
                    })
            })
            .take(limit)
            .collect()
    }

    pub fn monster_by_id(&self, id: u32) -> Option<&ReferenceMonster> {
        self.monsters_by_id.get(&id).map(|&index| &self.monsters[index])
    }

    /// Summarize exported static spawn directives for a map, including total
    /// records, record-weighted mean level, minimum and maximum monster levels,
    /// and distinct species count.
    pub fn map_spawn_details(&self, map_name: &str) -> Option<MapSpawnDetails> {
        type Aggregate = (u64, u64, u16, u16, usize);
        static DETAILS: OnceLock<HashMap<String, MapSpawnDetails>> = OnceLock::new();
        let summaries = DETAILS.get_or_init(|| {
            let mut aggregates: HashMap<String, Aggregate> = HashMap::new();
            for monster in &self.monsters {
                for region in &monster.spawn_regions {
                    if region.spawn_records == 0 {
                        continue;
                    }
                    let map = region.map.strip_suffix(".gat").unwrap_or(&region.map).to_ascii_lowercase();
                    let records = u64::from(region.spawn_records);
                    let aggregate = aggregates.entry(map).or_insert((0, 0, u16::MAX, 0, 0));
                    aggregate.0 += records;
                    aggregate.1 += u64::from(monster.level) * records;
                    aggregate.2 = aggregate.2.min(monster.level);
                    aggregate.3 = aggregate.3.max(monster.level);
                    aggregate.4 += 1;
                }
            }
            aggregates
                .into_iter()
                .map(|(map, (records, weighted_levels, min_level, max_level, species))| {
                    (map, MapSpawnDetails {
                        records,
                        mean_level: ((weighted_levels + records / 2) / records) as u16,
                        min_level,
                        max_level,
                        species,
                    })
                })
                .collect()
        });
        let map_name = map_name.strip_suffix(".gat").unwrap_or(map_name).to_ascii_lowercase();
        summaries.get(&map_name).copied()
    }

    /// Summarize only exported, static spawn directives for a map. The level is
    /// weighted by directive count and is a guide, not a recommended-level
    /// rule.
    pub fn map_spawn_summary(&self, map_name: &str) -> Option<(u64, u16, usize)> {
        self.map_spawn_details(map_name)
            .map(|details| (details.records, details.mean_level, details.species))
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

    pub fn skill_formula_review_for_skill(&self, skill_id: u16) -> Option<&ReferenceSkillFormulaReview> {
        self.skill_formula_reviews
            .iter()
            .find(|review| review.skill_ids.iter().any(|target| target.skill_id == skill_id))
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
            // Campaign story quests are not part of the open player Guide.
            .filter(|quest| !quest.is_story())
            .filter(|quest| {
                query.is_empty()
                    || quest.name.to_lowercase().contains(&query)
                    || quest.id.to_string() == query
                    || quest.targets.iter().any(|target| {
                        target.monster_name.to_lowercase().contains(&query)
                            || target.map_name.as_ref().is_some_and(|map| map.to_lowercase().contains(&query))
                    })
                    || quest
                        .npc_references
                        .iter()
                        .any(|npc| npc.name.to_lowercase().contains(&query) || npc.map_name.to_lowercase().contains(&query))
            })
            .collect();
        matches.sort_by_key(|quest| (quest.name.to_lowercase(), quest.id));
        matches.truncate(limit);
        matches
    }

    pub fn search_npcs(&self, query: &str, limit: usize) -> Vec<&ReferenceNpc> {
        let query = query.to_lowercase();
        let mut matches: Vec<_> = self
            .npcs
            .iter()
            // Campaign story NPCs (hubs, hidden set pieces) are not part of the
            // open player Guide.
            .filter(|npc| !npc.is_story())
            .filter(|npc| {
                query.is_empty()
                    || npc.id.to_string() == query
                    || npc.display_name.to_lowercase().contains(&query)
                    || npc.name.to_lowercase().contains(&query)
                    || npc.internal_name.to_lowercase().contains(&query)
                    || npc.map.to_lowercase().contains(&query)
                    || npc.declared_type.to_lowercase().contains(&query)
                    || (query == "shop" && matches!(npc.declared_type.as_str(), "trader" | "cashshop"))
            })
            .collect();
        matches.sort_by_key(|npc| (npc.display_name.to_lowercase(), npc.map.to_lowercase(), npc.id));
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

    /// Targets of every alias of `kind` that `query` reaches.
    pub fn alias_targets(&self, kind: &str, query: &str) -> Vec<&str> {
        self.aliases
            .iter()
            .filter(|alias| alias.kind == kind && alias_matches(&alias.alias, query))
            .map(|alias| alias.target.as_str())
            .collect()
    }

    pub fn search_statuses(&self, query: &str, limit: usize) -> Vec<&ReferenceStatus> {
        let query = query.to_lowercase();
        let alias_targets = self.alias_targets("status", &query);
        let mut matches: Vec<_> = self
            .statuses
            .iter()
            .filter(|status| {
                query.is_empty()
                    || status.name.to_lowercase().contains(&query)
                    || status.id.to_string() == query
                    || status.statuses.iter().any(|mechanic| {
                        alias_targets.contains(&mechanic.constant.as_str())
                            || mechanic.constant.to_lowercase().contains(&query)
                            || mechanic.associated_skill.as_ref().is_some_and(|skill| {
                                skill.name.to_lowercase().contains(&query) || skill.description.to_lowercase().contains(&query)
                            })
                            || mechanic.status_change_skills.iter().any(|skill| {
                                skill.name.to_lowercase().contains(&query) || skill.description.to_lowercase().contains(&query)
                            })
                    })
            })
            .collect();
        // Statuses the client shows an icon for rank first; iconless server
        // statuses follow, so adding them never displaces an established result.
        matches.sort_by_key(|status| (status.iconless, status.name.to_lowercase(), status.id));
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
        let mut matches: Vec<_> = self.items.iter().filter(|item| self.item_matches_query(item, &query)).collect();
        matches.sort_by_key(|item| (item.name.to_lowercase(), item.id));
        matches.truncate(limit);
        matches
    }

    pub fn search_cards(&self, query: &str, limit: usize) -> Vec<&ReferenceItem> {
        let query = query.to_lowercase();
        let mut matches: Vec<_> = self.cards.iter().filter(|card| self.item_matches_query(card, &query)).collect();
        matches.sort_by_key(|card| (card.name.to_lowercase(), card.id));
        matches.truncate(limit);
        matches
    }

    fn item_matches_query(&self, item: &ReferenceItem, query: &str) -> bool {
        item.matches_lowercase_query(query)
            || item.drops_from.iter().any(|drop| {
                self.monster_by_id(drop.monster_id).is_some_and(|monster| {
                    monster.name.to_lowercase().contains(query)
                        || monster.jname.to_lowercase().contains(query)
                        || monster.sprite_name.to_lowercase().contains(query)
                })
            })
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
        let prt_fild08 = data.map_spawn_summary("prt_fild08").expect("static map spawns");
        assert!(prt_fild08.0 > 0);
        assert!(prt_fild08.1 > 0);
        assert!(prt_fild08.2 > 0);
        assert_eq!(data.map_spawn_summary("prt_fild08.gat"), Some(prt_fild08));
        assert_eq!(data.map_spawn_summary("no_such_map"), None);
        assert!(poring.drops.iter().any(|drop| drop.item_id == 4001 && drop.kind == "normal"));
        assert_eq!(data.card_by_id(4001).map(|card| card.aegis_name.as_str()), Some("Poring_Card"));
        assert!(data.item_by_id(984).is_some_and(|item| item.name == "Oridecon"));
        assert!(
            data.search_monsters("hydra", 10)
                .iter()
                .any(|monster| monster.sprite_name == "HYDRA")
        );
        assert!(data.search_items("oridecon", 100).iter().any(|item| item.id == 984));
        assert!(data.search_items("984", 10).iter().any(|item| item.id == 984));
        assert!(
            data.search_items("poring", data.items.len())
                .iter()
                .any(|item| item.drops_from.iter().any(|drop| drop.monster_id == 1002))
        );
        assert!(data.search_cards("4001", 10).iter().any(|card| card.id == 4001));
        assert!(data.search_cards("poring", 10).iter().any(|card| card.id == 4001));
        assert!(
            data.search_cards("scripted_not_translated", data.cards.len())
                .iter()
                .any(|card| card.effect_status == "scripted_not_translated")
        );
        assert!(data.quest_by_id(3401).is_some_and(|quest| quest.name == "Animal Monster Hunt"));
        assert!(data.search_quests("animal monster hunt", 10).iter().any(|quest| quest.id == 3401));
        assert!(
            data.search_quests("poring", 100)
                .iter()
                .any(|quest| quest.targets.iter().any(|target| target.monster_name == "Poring"))
        );
        assert!(data.search_quests("angelo", 10).iter().any(|quest| quest.id == 9030));

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

        assert_eq!(data.skill_formula_reviews.len(), 12);
        let heal_review = data.skill_formula_review_for_skill(28).expect("Heal formula review");
        assert_eq!(heal_review.id, "al_heal_renewal_formula");
        assert!(heal_review.formula.contains("BaseLevel + INT"));
        assert_eq!(heal_review.evidence_state, EvidenceState::Conditional);
        assert!(data.skill_formula_review_for_skill(9999).is_none());

        let firewall = data.skill_by_id(18).expect("Fire Wall skill");
        assert_eq!(firewall.name, "MG_FIREWALL");
        assert!(firewall.source.as_ref().is_some_and(|s| s.path == "db/re/skill_db.conf"));
        assert!(
            firewall
                .prerequisites
                .iter()
                .any(|p| p.name == "MG_FIREBALL" && p.level == 5 && p.skill_id == Some(17))
        );

        assert_eq!(data.npc_services.len(), 14);
        let kafra = data.npc_service_by_id("kafra_employee_core_services").expect("Kafra service");
        assert_eq!(kafra.service_kind, "kafra_suite");
        assert!(kafra.locations.as_ref().is_some_and(|l| !l.is_empty()));
        assert!(data.search_services("repair", 10).iter().any(|s| s.service_kind.contains("repair")));

        assert_eq!(data.rumors.len(), 8);
        let byalan_rumor = data.rumor_by_id(1).expect("Byalan rumor");
        assert_eq!(byalan_rumor.title, "Unusual Sea Creatures of Byalan");
        assert_eq!(byalan_rumor.is_story_spoiler, false);
        // Authored flavour: no server script delivers it, so it is not "verified".
        assert_eq!(byalan_rumor.evidence_state, EvidenceState::NotReviewed);
        assert!(data.search_rumors("ant jaws", 5).iter().any(|r| r.id == 2));
    }
}
