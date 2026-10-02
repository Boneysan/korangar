//! Client-side build planner simulation engine.
//!
//! Provides hypothetical Base/Job level projection, stat and skill allocation
//! staging, prerequisite validation, and derived combat stat calculation
//! without committing network packets.
//!
//! Formulations match Hercules Renewal (`Hercules/src/map/status.c`, `pc.c`,
//! and `db/re/statpoint.txt`).

#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::stat_preview::StatKind;
use crate::dm::reference_data::ReferenceJobSkillTree;

/// Cumulative status point table indexed by `BaseLevel - 1` (levels 1 through
/// 175). Verified against `Hercules/db/re/statpoint.txt`.
pub const STAT_POINTS_TABLE: &[u32] = &[
    48, 51, 54, 57, 60, 64, 68, 72, 76, 80, 85, 90, 95, 100, 105, 111, 117, 123, 129, 135, 142, 149, 156, 163, 170, 178, 186, 194, 202,
    210, 219, 228, 237, 246, 255, 265, 275, 285, 295, 305, 316, 327, 338, 349, 360, 372, 384, 396, 408, 420, 433, 446, 459, 472, 485, 499,
    513, 527, 541, 555, 570, 585, 600, 615, 630, 646, 662, 678, 694, 710, 727, 744, 761, 778, 795, 813, 831, 849, 867, 885, 904, 923, 942,
    961, 980, 1000, 1020, 1040, 1060, 1080, 1101, 1122, 1143, 1164, 1185, 1207, 1229, 1251, 1273, 1295, 1318, 1341, 1364, 1387, 1410, 1433,
    1456, 1479, 1502, 1525, 1549, 1573, 1597, 1621, 1645, 1669, 1693, 1717, 1741, 1765, 1790, 1815, 1840, 1865, 1890, 1915, 1940, 1965,
    1990, 2015, 2041, 2067, 2093, 2119, 2145, 2171, 2197, 2223, 2249, 2275, 2302, 2329, 2356, 2383, 2410, 2437, 2464, 2491, 2518, 2545,
    2573, 2601, 2629, 2657, 2685, 2713, 2741, 2770, 2799, 2828, 2857, 2886, 2915, 2944, 2974, 3004, 3034, 3064, 3094, 3124, 3154, 3185,
    3216, 3247, 3278,
];

/// Calculate total status points granted at a given Base Level in Renewal.
/// Transcendent characters receive an extra 52 points at creation (100 total at
/// Lv 1).
pub fn total_stat_points_at_level(level: usize, is_transcendent: bool) -> u32 {
    let index = (level.max(1) - 1).min(STAT_POINTS_TABLE.len() - 1);
    let base_points = STAT_POINTS_TABLE[index];
    if is_transcendent { base_points + 52 } else { base_points }
}

/// Status point cost to increase a stat from `current_value` to `current_value
/// + 1`. Traced from Hercules `pc_need_status_point` in Renewal mode:
/// - `< 100`: `2 + (val - 1) / 10`
/// - `>= 100`: `16 + 4 * ((val - 100) / 5)`
pub fn stat_upgrade_cost(current_value: u16) -> u32 {
    if current_value == 0 {
        return 2;
    }
    if current_value < 100 {
        2 + (current_value as u32 - 1) / 10
    } else {
        16 + 4 * ((current_value as u32 - 100) / 5)
    }
}

/// Total status points required to raise a stat from `start_value` to
/// `target_value`.
pub fn stat_points_spent_between(start_value: u16, target_value: u16) -> u32 {
    if target_value <= start_value {
        return 0;
    }
    (start_value..target_value).map(stat_upgrade_cost).sum()
}

/// Six core character stats in the planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedStats {
    pub strength: u16,
    pub agility: u16,
    pub vitality: u16,
    pub intelligence: u16,
    pub dexterity: u16,
    pub luck: u16,
}

impl Default for PlannedStats {
    fn default() -> Self {
        Self {
            strength: 1,
            agility: 1,
            vitality: 1,
            intelligence: 1,
            dexterity: 1,
            luck: 1,
        }
    }
}

impl PlannedStats {
    pub fn new(str: u16, agi: u16, vit: u16, int: u16, dex: u16, luk: u16) -> Self {
        Self {
            strength: str,
            agility: agi,
            vitality: vit,
            intelligence: int,
            dexterity: dex,
            luck: luk,
        }
    }

    pub fn total_points_spent_from_baseline(&self, baseline: &PlannedStats) -> u32 {
        stat_points_spent_between(baseline.strength, self.strength)
            + stat_points_spent_between(baseline.agility, self.agility)
            + stat_points_spent_between(baseline.vitality, self.vitality)
            + stat_points_spent_between(baseline.intelligence, self.intelligence)
            + stat_points_spent_between(baseline.dexterity, self.dexterity)
            + stat_points_spent_between(baseline.luck, self.luck)
    }

    pub fn total_points_spent_from_scratch(&self) -> u32 {
        self.total_points_spent_from_baseline(&PlannedStats::default())
    }

    pub fn get_stat(&self, stat: StatKind) -> u16 {
        match stat {
            StatKind::Strength => self.strength,
            StatKind::Agility => self.agility,
            StatKind::Vitality => self.vitality,
            StatKind::Intelligence => self.intelligence,
            StatKind::Dexterity => self.dexterity,
            StatKind::Luck => self.luck,
        }
    }

    pub fn set_stat(&mut self, stat: StatKind, value: u16) {
        match stat {
            StatKind::Strength => self.strength = value,
            StatKind::Agility => self.agility = value,
            StatKind::Vitality => self.vitality = value,
            StatKind::Intelligence => self.intelligence = value,
            StatKind::Dexterity => self.dexterity = value,
            StatKind::Luck => self.luck = value,
        }
    }
}

/// Projected combat parameters calculated from planned levels and attributes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectedCombatStats {
    pub base_level: usize,
    pub job_level: usize,
    pub max_hp_estimate: usize,
    pub max_sp_estimate: usize,
    pub hit: i32,
    pub flee: i32,
    pub soft_def: i32,
    pub soft_mdef: i32,
    pub crit_tenth_percent: i32,
    pub perfect_dodge_tenth_percent: i32,
    pub status_atk: i32,
    pub status_matk: i32,
    pub variable_cast_reduction_pct: f32,
    pub max_weight: u32,
}

impl ProjectedCombatStats {
    pub fn calculate(
        base_level: usize,
        job_level: usize,
        stats: &PlannedStats,
        job_bonuses: Option<&PlannedStats>,
        base_hp_baseline: usize,
        base_sp_baseline: usize,
        base_weight_limit: u32,
    ) -> Self {
        let zero_bonuses = PlannedStats::new(0, 0, 0, 0, 0, 0);
        let bonuses = job_bonuses.unwrap_or(&zero_bonuses);

        let total_str = stats.strength as i32 + bonuses.strength as i32;
        let total_agi = stats.agility as i32 + bonuses.agility as i32;
        let total_vit = stats.vitality as i32 + bonuses.vitality as i32;
        let total_int = stats.intelligence as i32 + bonuses.intelligence as i32;
        let total_dex = stats.dexterity as i32 + bonuses.dexterity as i32;
        let total_luk = stats.luck as i32 + bonuses.luck as i32;

        let level_i32 = base_level as i32;

        let hit = level_i32 + total_dex + total_luk / 3 + 175;
        let flee = level_i32 + total_agi + total_luk / 5 + 100;
        let soft_def = (level_i32 + total_vit) / 2 + total_agi / 5;
        let soft_mdef = total_int + level_i32 / 4 + (total_dex + total_vit) / 5;
        let crit_tenth_percent = 10 + total_luk * 10 / 3;
        let perfect_dodge_tenth_percent = total_luk + 10;
        let status_atk = level_i32 / 4 + total_str + total_dex / 5 + total_luk / 3;
        let status_matk = level_i32 / 4 + total_int + total_int / 2 + total_dex / 5 + total_luk / 3;

        let cast_stat_sum = total_dex * 2 + total_int;
        let variable_cast_reduction_pct = (cast_stat_sum as f32 / 530.0).clamp(0.0, 1.0) * 100.0;

        let max_weight = base_weight_limit + (stats.strength as u32 * 300);

        let max_hp_estimate = base_hp_baseline * base_level * (100 + total_vit.max(0) as usize) / 100;
        let max_sp_estimate = base_sp_baseline * base_level * (100 + total_int.max(0) as usize) / 100;

        Self {
            base_level,
            job_level,
            max_hp_estimate,
            max_sp_estimate,
            hit,
            flee,
            soft_def,
            soft_mdef,
            crit_tenth_percent,
            perfect_dodge_tenth_percent,
            status_atk,
            status_matk,
            variable_cast_reduction_pct,
            max_weight,
        }
    }
}

/// A serialized or live build planner state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildPlan {
    pub name: String,
    pub job_id: u16,
    pub is_transcendent: bool,
    pub target_base_level: usize,
    pub target_job_level: usize,
    pub stats: PlannedStats,
    pub skills: HashMap<u16, u16>,
}

impl BuildPlan {
    pub fn new(name: impl Into<String>, job_id: u16, is_transcendent: bool) -> Self {
        Self {
            name: name.into(),
            job_id,
            is_transcendent,
            target_base_level: 1,
            target_job_level: 1,
            stats: PlannedStats::default(),
            skills: HashMap::new(),
        }
    }

    /// Total status points granted at `target_base_level`.
    pub fn total_stat_points(&self) -> u32 {
        total_stat_points_at_level(self.target_base_level, self.is_transcendent)
    }

    /// Status points spent so far from initial 1/1/1/1/1/1 spread.
    pub fn spent_stat_points(&self) -> u32 {
        self.stats.total_points_spent_from_scratch()
    }

    /// Unallocated status points remaining.
    pub fn remaining_stat_points(&self) -> u32 {
        self.total_stat_points().saturating_sub(self.spent_stat_points())
    }

    /// Total skill points available at `target_job_level` (1 point per job
    /// level after Lv 1).
    pub fn total_skill_points(&self) -> u32 {
        self.target_job_level.saturating_sub(1) as u32
    }

    /// Skill points spent across all planned skills.
    pub fn spent_skill_points(&self) -> u32 {
        self.skills.values().map(|&level| level as u32).sum()
    }

    /// Unallocated skill points remaining.
    pub fn remaining_skill_points(&self) -> u32 {
        self.total_skill_points().saturating_sub(self.spent_skill_points())
    }

    /// Whether a stat can be increased by 1 point.
    pub fn can_increase_stat(&self, stat: StatKind, max_stat: u16) -> bool {
        let current = self.stats.get_stat(stat);
        if current >= max_stat {
            return false;
        }
        let cost = stat_upgrade_cost(current);
        self.remaining_stat_points() >= cost
    }

    /// Increase a stat by 1 point if points permit.
    pub fn increase_stat(&mut self, stat: StatKind, max_stat: u16) -> Result<(), &'static str> {
        let current = self.stats.get_stat(stat);
        if current >= max_stat {
            return Err("stat is already at maximum allowable value");
        }
        let cost = stat_upgrade_cost(current);
        if self.remaining_stat_points() < cost {
            return Err("insufficient unallocated status points");
        }
        self.stats.set_stat(stat, current + 1);
        Ok(())
    }

    /// Decrease a stat by 1 point down to `min_stat` (minimum 1).
    pub fn decrease_stat(&mut self, stat: StatKind, min_stat: u16) -> Result<(), &'static str> {
        let current = self.stats.get_stat(stat);
        let floor = min_stat.max(1);
        if current <= floor {
            return Err("stat is already at baseline minimum");
        }
        self.stats.set_stat(stat, current - 1);
        Ok(())
    }

    /// Check if a skill can be increased by 1 point, validating skill points
    /// and prerequisites.
    pub fn can_increase_skill(&self, skill_id: u16, max_level: u16, tree: Option<&ReferenceJobSkillTree>) -> Result<(), String> {
        if self.remaining_skill_points() == 0 {
            return Err("no unallocated skill points available".to_owned());
        }
        let current_level = self.skills.get(&skill_id).copied().unwrap_or(0);
        if current_level >= max_level {
            return Err(format!("skill #{skill_id} is already at maximum level {max_level}"));
        }

        if let Some(tree) = tree {
            if let Some(skill_entry) = tree.skills.iter().find(|s| s.skill_id == skill_id) {
                if self.target_job_level < skill_entry.minimum_job_level as usize {
                    return Err(format!(
                        "requires Job Level {} (currently targeted at {})",
                        skill_entry.minimum_job_level, self.target_job_level
                    ));
                }
                for prereq in &skill_entry.prerequisites {
                    let prereq_level = self.skills.get(&prereq.skill_id).copied().unwrap_or(0);
                    if prereq_level < prereq.level {
                        return Err(format!(
                            "requires {} Lv {} (currently Lv {})",
                            prereq.name, prereq.level, prereq_level
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Increase a skill by 1 point.
    pub fn increase_skill(&mut self, skill_id: u16, max_level: u16, tree: Option<&ReferenceJobSkillTree>) -> Result<(), String> {
        self.can_increase_skill(skill_id, max_level, tree)?;
        let entry = self.skills.entry(skill_id).or_insert(0);
        *entry += 1;
        Ok(())
    }

    /// Check if a skill can be decreased without breaking prerequisite
    /// requirements of other planned skills.
    pub fn can_decrease_skill(&self, skill_id: u16, tree: Option<&ReferenceJobSkillTree>) -> Result<(), String> {
        let current_level = self.skills.get(&skill_id).copied().unwrap_or(0);
        if current_level == 0 {
            return Err(format!("skill #{skill_id} has 0 points allocated"));
        }

        let new_level = current_level - 1;

        if let Some(tree) = tree {
            for (other_id, &other_level) in &self.skills {
                if other_level == 0 || *other_id == skill_id {
                    continue;
                }
                if let Some(other_skill) = tree.skills.iter().find(|s| s.skill_id == *other_id) {
                    for prereq in &other_skill.prerequisites {
                        if prereq.skill_id == skill_id && new_level < prereq.level {
                            return Err(format!(
                                "cannot reduce: {} Lv {} requires skill #{skill_id} Lv {}",
                                other_skill.name, other_level, prereq.level
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Decrease a skill by 1 point.
    pub fn decrease_skill(&mut self, skill_id: u16, tree: Option<&ReferenceJobSkillTree>) -> Result<(), String> {
        self.can_decrease_skill(skill_id, tree)?;
        let entry = self.skills.entry(skill_id).or_insert(0);
        if *entry > 0 {
            *entry -= 1;
            if *entry == 0 {
                self.skills.remove(&skill_id);
            }
        }
        Ok(())
    }

    /// Adjust target Base Level, clamping between 1 and `max_level`.
    pub fn set_target_base_level(&mut self, level: usize, max_level: usize) {
        self.target_base_level = level.clamp(1, max_level);
    }

    /// Adjust target Job Level, clamping between 1 and `max_level`.
    pub fn set_target_job_level(&mut self, level: usize, max_level: usize) {
        self.target_job_level = level.clamp(1, max_level);
    }

    /// Serialize the build plan to a portable JSON string.
    pub fn save_to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| format!("failed to serialize build plan: {error}"))
    }

    /// Deserialize a build plan from a JSON string.
    pub fn load_from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|error| format!("failed to parse build plan JSON: {error}"))
    }

    /// Compute projected combat parameters.
    pub fn calculate_projections(
        &self,
        job_bonuses: Option<&PlannedStats>,
        base_hp_baseline: usize,
        base_sp_baseline: usize,
        base_weight_limit: u32,
    ) -> ProjectedCombatStats {
        ProjectedCombatStats::calculate(
            self.target_base_level,
            self.target_job_level,
            &self.stats,
            job_bonuses,
            base_hp_baseline,
            base_sp_baseline,
            base_weight_limit,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dm::reference_data::reference_data;

    #[test]
    fn statpoint_table_matches_hercules_renewal_levels() {
        assert_eq!(total_stat_points_at_level(1, false), 48);
        assert_eq!(total_stat_points_at_level(1, true), 100); // 48 + 52 trans bonus
        assert_eq!(total_stat_points_at_level(2, false), 51);
        assert_eq!(total_stat_points_at_level(10, false), 80);
        assert_eq!(total_stat_points_at_level(50, false), 420);
        assert_eq!(total_stat_points_at_level(99, false), 1273);
        assert_eq!(total_stat_points_at_level(99, true), 1325); // 1273 + 52
        assert_eq!(total_stat_points_at_level(175, false), 3278);
    }

    #[test]
    fn stat_upgrade_cost_scales_at_exact_thresholds() {
        // v < 100: 2 + (v - 1)/10
        assert_eq!(stat_upgrade_cost(1), 2);
        assert_eq!(stat_upgrade_cost(10), 2);
        assert_eq!(stat_upgrade_cost(11), 3);
        assert_eq!(stat_upgrade_cost(20), 3);
        assert_eq!(stat_upgrade_cost(21), 4);
        assert_eq!(stat_upgrade_cost(90), 10);
        assert_eq!(stat_upgrade_cost(91), 11);
        assert_eq!(stat_upgrade_cost(99), 11);

        // v >= 100: 16 + 4 * ((v - 100) / 5)
        assert_eq!(stat_upgrade_cost(100), 16);
        assert_eq!(stat_upgrade_cost(104), 16);
        assert_eq!(stat_upgrade_cost(105), 20);
        assert_eq!(stat_upgrade_cost(109), 20);
        assert_eq!(stat_upgrade_cost(110), 24);
    }

    #[test]
    fn raising_stat_from_1_to_99_costs_exactly_628_points() {
        assert_eq!(stat_points_spent_between(1, 99), 628);
    }

    #[test]
    fn level_1_novice_plan_initializes_with_correct_budgets() {
        let plan = BuildPlan::new("Novice Starter", 0, false);
        assert_eq!(plan.total_stat_points(), 48);
        assert_eq!(plan.spent_stat_points(), 0);
        assert_eq!(plan.remaining_stat_points(), 48);
        assert_eq!(plan.total_skill_points(), 0);
        assert_eq!(plan.spent_skill_points(), 0);
        assert_eq!(plan.remaining_skill_points(), 0);
    }

    #[test]
    fn level_15_swordsman_stat_and_skill_allocations() {
        let mut plan = BuildPlan::new("Swordsman Early", 1, false);
        plan.set_target_base_level(15, 99);
        plan.set_target_job_level(10, 50);

        // Base Lv 15 -> 105 stat points
        assert_eq!(plan.total_stat_points(), 105);
        // Job Lv 10 -> 9 skill points
        assert_eq!(plan.total_skill_points(), 9);

        // Spend STR: 1 -> 15 (costs: 10 * 2 + 4 * 3 = 20 + 12 = 32)
        for _ in 1..15 {
            assert!(plan.increase_stat(StatKind::Strength, 99).is_ok());
        }
        assert_eq!(plan.stats.strength, 15);
        assert_eq!(plan.spent_stat_points(), 32);
        assert_eq!(plan.remaining_stat_points(), 105 - 32);

        // Spend skills: Bash 5, Sword Mastery 4
        let data = reference_data();
        let swordsman_tree = data.job_skill_tree_by_id(1);

        for _ in 0..5 {
            assert!(plan.increase_skill(5, 10, swordsman_tree).is_ok()); // Bash (ID 5)
        }
        for _ in 0..4 {
            assert!(plan.increase_skill(1, 10, swordsman_tree).is_ok()); // Sword Mastery (ID 1)
        }
        assert_eq!(plan.spent_skill_points(), 9);
        assert_eq!(plan.remaining_skill_points(), 0);

        // Cannot spend 10th skill point (out of budget)
        assert!(plan.increase_skill(5, 10, swordsman_tree).is_err());
    }

    #[test]
    fn level_50_knight_enforces_prerequisite_tree_dependencies() {
        let mut plan = BuildPlan::new("Knight Two-Hander", 7, false);
        plan.set_target_base_level(50, 99);
        plan.set_target_job_level(40, 50);

        let data = reference_data();
        let knight_tree = data.job_skill_tree_by_id(7).expect("Knight tree");

        // Two-Hand Quicken (ID 60) requires Two-Hand Sword Mastery (SM_TWOHAND, ID 3)
        // Lv 1
        let thq_attempt = plan.increase_skill(60, 10, Some(knight_tree));
        assert!(thq_attempt.is_err(), "THQ must fail without 2H Sword Mastery");
        assert!(thq_attempt.unwrap_err().contains("requires SM_TWOHAND"));

        // Two-Hand Sword Mastery (ID 3) requires 1H Sword Mastery (SM_SWORD, ID 2) Lv 1
        let thm_attempt = plan.increase_skill(3, 10, Some(knight_tree));
        assert!(thm_attempt.is_err(), "THM must fail without 1H Sword Mastery");
        assert!(thm_attempt.unwrap_err().contains("requires SM_SWORD"));

        // Learn 1H Sword Mastery Lv 1 (ID 2 has no prerequisites)
        assert!(plan.increase_skill(2, 10, Some(knight_tree)).is_ok());

        // Now Two-Hand Sword Mastery Lv 1 can be learned!
        assert!(plan.increase_skill(3, 10, Some(knight_tree)).is_ok());

        // Now Two-Hand Quicken can be learned!
        assert!(plan.increase_skill(60, 10, Some(knight_tree)).is_ok());

        // Attempting to decrease 2H Sword Mastery below Lv 1 must be rejected while THQ
        // is learned
        let decrease_thm = plan.decrease_skill(3, Some(knight_tree));
        assert!(decrease_thm.is_err(), "cannot decrease 2H mastery required by THQ");

        // Attempting to decrease 1H Sword Mastery below Lv 1 must be rejected while 2H
        // mastery is learned
        let decrease_sword = plan.decrease_skill(2, Some(knight_tree));
        assert!(decrease_sword.is_err(), "cannot decrease 1H mastery required by 2H mastery");
    }

    #[test]
    fn level_99_transcendent_projections_and_serialization() {
        let mut plan = BuildPlan::new("Lord Knight Final", 4008, true);
        plan.set_target_base_level(99, 99);
        plan.set_target_job_level(70, 70);

        // Transcendent 99 gets 1273 + 52 = 1325 stat points
        assert_eq!(plan.total_stat_points(), 1325);
        assert_eq!(plan.total_skill_points(), 69);

        // Allocate STR 80, AGI 70, VIT 70, DEX 50
        for _ in 1..80 {
            assert!(plan.increase_stat(StatKind::Strength, 99).is_ok());
        }
        for _ in 1..70 {
            assert!(plan.increase_stat(StatKind::Agility, 99).is_ok());
        }
        for _ in 1..70 {
            assert!(plan.increase_stat(StatKind::Vitality, 99).is_ok());
        }
        for _ in 1..50 {
            assert!(plan.increase_stat(StatKind::Dexterity, 99).is_ok());
        }

        assert_eq!(plan.stats.strength, 80);
        assert_eq!(plan.stats.agility, 70);
        assert_eq!(plan.stats.vitality, 70);
        assert_eq!(plan.stats.dexterity, 50);

        let job_bonuses = PlannedStats::new(8, 2, 4, 0, 3, 2);
        let projections = plan.calculate_projections(Some(&job_bonuses), 40, 5, 2000);

        // Verify combat projection values
        assert_eq!(projections.base_level, 99);
        assert_eq!(projections.job_level, 70);
        assert!(projections.hit >= 320, "HIT should exceed 320, got {}", projections.hit);
        assert!(projections.flee >= 270, "FLEE should exceed 270, got {}", projections.flee);
        assert!(projections.soft_def >= 90, "DEF should exceed 90, got {}", projections.soft_def);
        assert!(projections.max_weight >= 2000 + 80 * 300);
        assert!(projections.variable_cast_reduction_pct > 0.0);

        // Verify JSON save and load roundtrip
        let json = plan.save_to_json().expect("save plan");
        let loaded = BuildPlan::load_from_json(&json).expect("load plan");
        assert_eq!(plan, loaded);
        assert_eq!(loaded.total_stat_points(), 1325);
        assert_eq!(loaded.stats.strength, 80);
    }
}
