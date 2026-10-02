use std::collections::HashMap;
use std::path::{Path, PathBuf};

use korangar_interface::element::StateElement;
use rust_state::RustState;

use crate::dm::reference_data::reference_data;
use crate::world::{BuildPlan, PlannedStats, ProjectedCombatStats, StatKind};

/// The stat cap used when the server's stat rules do not name the job (a
/// mounted or cosmetic form with no `job_db.conf` block). Hercules' own default
/// for a group that sets no `MaxStats` (`status.c`).
pub const FALLBACK_MAX_STAT: u16 = 99;
/// Highest Base Level the planner offers (Renewal `statpoint.txt` rows exist
/// beyond this, but this server's classes stop here).
pub const PLANNER_MAX_BASE_LEVEL: usize = 99;
/// Highest Job Level the planner offers.
pub const PLANNER_MAX_JOB_LEVEL: usize = 50;

const ALL_STATS: [StatKind; 6] = [
    StatKind::Strength,
    StatKind::Agility,
    StatKind::Vitality,
    StatKind::Intelligence,
    StatKind::Dexterity,
    StatKind::Luck,
];

/// The live character values a plan is measured against.
#[derive(Clone, Debug, Default)]
pub struct PlannerBaseline {
    pub job_id: u16,
    pub base_level: usize,
    pub job_level: usize,
    pub stats: PlannedStats,
    pub max_hp: usize,
    pub max_sp: usize,
    pub max_weight: u32,
    /// Live learned skill levels.
    pub skills: HashMap<u16, u16>,
    /// Live unspent skill points.
    pub skill_points: u32,
}

/// One line of the planned skill list.
#[derive(Clone, Default, RustState)]
pub struct PlannerSkillRow {
    pub skill_id: u16,
    pub text: String,
}

impl PlannerBaseline {
    fn projections(&self) -> ProjectedCombatStats {
        ProjectedCombatStats::calculate(
            self.base_level,
            self.job_level,
            &self.stats,
            None,
            self.weight_without_strength(),
            self.job_id,
        )
    }

    fn weight_without_strength(&self) -> u32 {
        self.max_weight.saturating_sub(self.stats.strength as u32 * 300)
    }
}

/// Simulated build planner (GDD F03). Holds a hypothetical plan and the text
/// the window shows for it. Nothing here ever reaches the network.
#[derive(Default, RustState, StateElement)]
pub struct BuildPlannerState {
    #[hidden_element]
    plan: Option<BuildPlan>,
    #[hidden_element]
    baseline: PlannerBaseline,
    #[hidden_element]
    max_stat: u16,
    #[hidden_element]
    skill_rows: Vec<PlannerSkillRow>,
    header_text: String,
    base_level_text: String,
    job_level_text: String,
    points_text: String,
    strength_text: String,
    agility_text: String,
    vitality_text: String,
    intelligence_text: String,
    dexterity_text: String,
    luck_text: String,
    hp_sp_text: String,
    hit_flee_text: String,
    def_text: String,
    atk_text: String,
    crit_text: String,
    cast_weight_text: String,
    status_text: String,
}

fn delta(value: i64, baseline: i64) -> String {
    format!("{value} ({:+})", value - baseline)
}

fn skill_rows(plan: &BuildPlan, baseline: &PlannerBaseline) -> Vec<PlannerSkillRow> {
    let Some(tree) = reference_data().job_skill_tree_by_id(plan.job_id) else {
        return Vec::new();
    };
    tree.skills
        .iter()
        .map(|skill| {
            let level = plan.skills.get(&skill.skill_id).copied().unwrap_or(0);
            let learned = baseline.skills.get(&skill.skill_id).copied().unwrap_or(0);
            let name = crate::world::skill_display_name(skill.skill_id).unwrap_or(skill.name.as_str());
            let mut text = format!("{name} {level}/{} ({:+})", skill.max_level, level as i32 - learned as i32);
            if level < skill.max_level {
                if let Err(reason) = plan.can_increase_skill(skill.skill_id, skill.max_level, Some(tree)) {
                    text.push_str(&format!(" · {reason}"));
                }
            }
            PlannerSkillRow {
                skill_id: skill.skill_id,
                text,
            }
        })
        .collect()
}

impl BuildPlannerState {
    /// Start (or restart) a plan at the live character's current point.
    pub fn start(&mut self, baseline: PlannerBaseline) {
        // Both facts come from the server's own tables: the job's `MaxStats`
        // (unit_parameters_db.conf, not `battle.conf`'s max_parameter) and
        // whether the class is JOBL_UPPER, which grants 52 extra points. A job
        // the tables do not name falls back to a plain 99-cap, non-upper class.
        let rules = reference_data().stat_job(baseline.job_id);
        self.max_stat = rules.map_or(FALLBACK_MAX_STAT, |job| job.max_stats);
        let mut plan = BuildPlan::new("Planner", baseline.job_id, rules.is_some_and(|job| job.upper));
        plan.target_base_level = baseline.base_level;
        plan.target_job_level = baseline.job_level;
        plan.stats = baseline.stats.clone();
        plan.skills = baseline.skills.clone();
        plan.skill_points_at_start = Some(baseline.skills.values().map(|&level| level as u32).sum::<u32>() + baseline.skill_points);
        plan.start_job_level = baseline.job_level;
        self.plan = Some(plan);
        self.baseline = baseline;
        self.status_text = "Simulation only: nothing is sent to the server.".to_owned();
        self.refresh();
    }

    /// Where a slot lives. Plans are per job: a Knight's plan is meaningless
    /// to a Wizard, and keying the file keeps one slot per class.
    pub fn slot_path(directory: &Path, job_id: u16, slot: u8) -> PathBuf {
        directory.join(format!("job-{job_id}-slot-{slot}.json"))
    }

    /// Write the current plan to `slot`. Nothing is sent to the server.
    pub fn save_slot(&mut self, directory: &Path, slot: u8) -> Result<(), String> {
        let plan = self.plan.as_mut().ok_or("no plan is open")?;
        plan.name = format!("Slot {slot}");
        let json = plan.save_to_json()?;
        std::fs::create_dir_all(directory).map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
        let path = Self::slot_path(directory, plan.job_id, slot);
        std::fs::write(&path, json).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        self.status_text = format!("Saved to slot {slot}.");
        Ok(())
    }

    /// Replace the plan with the one in `slot`, re-fitted to the character as
    /// it is *now*. Only the intent (target levels, stats, skills) is taken
    /// from the file: the point budget is rebuilt from the live character,
    /// because the saved one describes the character as it was when saved.
    pub fn load_slot(&mut self, directory: &Path, slot: u8) -> Result<(), String> {
        let current = self.plan.as_ref().ok_or("no plan is open")?;
        let path = Self::slot_path(directory, current.job_id, slot);
        let json = std::fs::read_to_string(&path).map_err(|_| format!("slot {slot} is empty"))?;
        self.load_json(&json)
            .inspect(|()| self.status_text = format!("Loaded slot {slot}."))
    }

    fn load_json(&mut self, json: &str) -> Result<(), String> {
        let loaded = BuildPlan::load_from_json(json)?;
        let mut plan = self.plan.clone().ok_or("no plan is open")?;
        if loaded.job_id != plan.job_id {
            return Err(format!(
                "plan is for job {}, this character is job {}",
                loaded.job_id, plan.job_id
            ));
        }
        plan.target_base_level = loaded.target_base_level.clamp(1, PLANNER_MAX_BASE_LEVEL);
        plan.target_job_level = loaded.target_job_level.clamp(plan.start_job_level.max(1), PLANNER_MAX_JOB_LEVEL);
        plan.stats = loaded.stats;
        plan.skills = loaded.skills;
        // Levels the character already has are spent for good.
        for (&skill_id, &level) in &self.baseline.skills {
            let entry = plan.skills.entry(skill_id).or_insert(0);
            *entry = (*entry).max(level);
        }
        if ALL_STATS
            .iter()
            .any(|&stat| plan.stats.get_stat(stat) > self.max_stat || plan.stats.get_stat(stat) == 0)
        {
            return Err(format!("plan has a stat outside 1..={}", self.max_stat));
        }
        if plan.spent_stat_points() > plan.total_stat_points() {
            return Err("plan no longer fits: it spends more stat points than that level grants".to_owned());
        }
        if plan.spent_skill_points() > plan.total_skill_points() {
            return Err("plan no longer fits: it spends more skill points than this character has".to_owned());
        }
        self.plan = Some(plan);
        self.refresh();
        Ok(())
    }

    /// Show the outcome of a save or load in the window.
    pub fn report(&mut self, result: Result<(), String>) {
        if let Err(reason) = result {
            self.status_text = reason;
        }
    }

    pub fn reset(&mut self) {
        let baseline = self.baseline.clone();
        self.start(baseline);
    }

    pub fn adjust_stat(&mut self, stat: StatKind, change: i8) {
        let Some(plan) = self.plan.as_mut() else { return };
        let result = match change >= 0 {
            true => plan.increase_stat(stat, self.max_stat),
            false => plan.decrease_stat(stat, 1),
        };
        self.status_text = match result {
            Ok(()) => "Simulation only: nothing is sent to the server.".to_owned(),
            Err(reason) => reason.to_owned(),
        };
        self.refresh();
    }

    pub fn adjust_base_level(&mut self, change: i16) {
        let Some(plan) = self.plan.as_mut() else { return };
        let target = (plan.target_base_level as i32 + change as i32).max(1) as usize;
        let previous = plan.target_base_level;
        plan.set_target_base_level(target, PLANNER_MAX_BASE_LEVEL);
        // A lower level cannot hold more points than it grants; refuse the
        // step instead of silently leaving the plan over-allocated.
        if plan.spent_stat_points() > plan.total_stat_points() {
            plan.target_base_level = previous;
            self.status_text = "Spent points exceed that level's total; lower a stat first.".to_owned();
        } else {
            self.status_text = "Simulation only: nothing is sent to the server.".to_owned();
        }
        self.refresh();
    }

    pub fn adjust_job_level(&mut self, change: i16) {
        let Some(plan) = self.plan.as_mut() else { return };
        // Job levels already earned cannot be un-earned, so the floor is the
        // live level the plan started from.
        let floor = plan.start_job_level.max(1);
        let target = (plan.target_job_level as i32 + change as i32).max(floor as i32) as usize;
        let previous = plan.target_job_level;
        plan.set_target_job_level(target, PLANNER_MAX_JOB_LEVEL);
        if plan.spent_skill_points() > plan.total_skill_points() {
            plan.target_job_level = previous;
        }
        self.refresh();
    }

    pub fn adjust_skill(&mut self, skill_id: u16, change: i8) {
        let Some(plan) = self.plan.as_mut() else { return };
        let tree = reference_data().job_skill_tree_by_id(plan.job_id);
        let maximum = tree
            .and_then(|tree| tree.skills.iter().find(|skill| skill.skill_id == skill_id))
            .map(|skill| skill.max_level)
            .unwrap_or(0);
        let learned = self.baseline.skills.get(&skill_id).copied().unwrap_or(0);
        let current = plan.skills.get(&skill_id).copied().unwrap_or(0);
        let result = match change >= 0 {
            true => plan.increase_skill(skill_id, maximum, tree),
            // Levels the character already has are spent for good; only
            // levels added in the plan can be taken back.
            false if current <= learned => Err("levels already learned cannot be taken back".to_owned()),
            false => plan.decrease_skill(skill_id, tree),
        };
        self.status_text = match result {
            Ok(()) => "Simulation only: nothing is sent to the server.".to_owned(),
            Err(reason) => reason,
        };
        self.refresh();
    }

    fn refresh(&mut self) {
        let Some(plan) = self.plan.as_ref() else { return };
        let baseline = &self.baseline;
        let projected = ProjectedCombatStats::calculate(
            plan.target_base_level,
            plan.target_job_level,
            &plan.stats,
            None,
            baseline.weight_without_strength(),
            plan.job_id,
        );
        let current = baseline.projections();

        self.header_text = format!("Job {} · estimates are labelled, not server-exact", plan.job_id);
        self.base_level_text = format!("Target Base Lv {} / {PLANNER_MAX_BASE_LEVEL}", plan.target_base_level);
        self.job_level_text = format!("Target Job Lv {} / {PLANNER_MAX_JOB_LEVEL}", plan.target_job_level);
        self.points_text = format!(
            "Stat points {} left of {} · Skill points {} left of {}",
            plan.remaining_stat_points(),
            plan.total_stat_points(),
            plan.remaining_skill_points(),
            plan.total_skill_points()
        );

        let rows = ALL_STATS.map(|stat| {
            let value = plan.stats.get_stat(stat);
            let cost = crate::world::stat_upgrade_cost(value);
            let cost = match value >= self.max_stat {
                true => "max".to_owned(),
                false => format!("next {cost}"),
            };
            format!(
                "{} {} ({:+}) · {cost}",
                stat.name(),
                value,
                value as i32 - baseline.stats.get_stat(stat) as i32
            )
        });
        let [strength, agility, vitality, intelligence, dexterity, luck] = rows;
        self.strength_text = strength;
        self.agility_text = agility;
        self.vitality_text = vitality;
        self.intelligence_text = intelligence;
        self.dexterity_text = dexterity;
        self.luck_text = luck;

        self.hp_sp_text = match (
            projected.base_max_hp,
            current.base_max_hp,
            projected.base_max_sp,
            current.base_max_sp,
        ) {
            (Some(hp), Some(current_hp), Some(sp), Some(current_sp)) => format!(
                "Base HP {} · Base SP {} (class table; gear and statuses add to these; now {} / {} with gear)",
                delta(hp as i64, current_hp as i64),
                delta(sp as i64, current_sp as i64),
                baseline.max_hp,
                baseline.max_sp
            ),
            _ => "HP and SP: no class table for this job".to_owned(),
        };
        self.hit_flee_text = format!(
            "HIT {} · FLEE {}",
            delta(projected.hit as i64, current.hit as i64),
            delta(projected.flee as i64, current.flee as i64)
        );
        self.def_text = format!(
            "Soft DEF {} · Soft MDEF {}",
            delta(projected.soft_def as i64, current.soft_def as i64),
            delta(projected.soft_mdef as i64, current.soft_mdef as i64)
        );
        self.atk_text = format!(
            "Status ATK {} · Status MATK {}",
            delta(projected.status_atk as i64, current.status_atk as i64),
            delta(projected.status_matk as i64, current.status_matk as i64)
        );
        self.crit_text = format!(
            "Crit {:.1}% · Perfect dodge {:.1}%",
            projected.crit_tenth_percent as f32 / 10.0,
            projected.perfect_dodge_tenth_percent as f32 / 10.0
        );
        self.skill_rows = skill_rows(plan, baseline);
        self.cast_weight_text = format!(
            "Variable cast -{:.1}% (estimate) · Weight {}",
            projected.variable_cast_reduction_pct,
            projected.max_weight / 10
        );
    }

    #[cfg(test)]
    fn plan(&self) -> &BuildPlan {
        self.plan.as_ref().expect("started")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> PlannerBaseline {
        PlannerBaseline {
            job_id: 7,
            base_level: 50,
            job_level: 30,
            stats: PlannedStats::new(40, 20, 30, 10, 25, 5),
            max_hp: 3_000,
            max_sp: 400,
            max_weight: 20_000,
            skills: HashMap::new(),
            skill_points: 0,
        }
    }

    #[test]
    fn starting_point_reads_the_live_values_with_zero_deltas() {
        let mut state = BuildPlannerState::default();
        state.start(baseline());

        assert!(state.strength_text.contains("40 (+0)"), "{}", state.strength_text);
        // Knight (job 7), level 50, VIT 30, INT 10. From the exported class
        // tables: HP[50] = 2208, so 2208 + 2208 * 30 / 100 = 2870; SP[50] = 160,
        // so 160 + 160 * 10 / 100 = 176. The unchanged plan has no delta.
        assert!(state.hp_sp_text.contains("Base HP 2870 (+0)"), "{}", state.hp_sp_text);
        assert!(state.hp_sp_text.contains("Base SP 176 (+0)"), "{}", state.hp_sp_text);
        assert!(state.hp_sp_text.contains("now 3000 / 400 with gear"), "{}", state.hp_sp_text);
    }

    #[test]
    fn a_point_of_vit_changes_base_hp_by_the_servers_integer_percent() {
        let mut state = BuildPlannerState::default();
        state.start(baseline());
        state.adjust_stat(StatKind::Vitality, 1);
        // VIT 31: 2208 + 2208 * 31 / 100 = 2208 + 684 = 2892, which is +22.
        assert!(state.hp_sp_text.contains("Base HP 2892 (+22)"), "{}", state.hp_sp_text);
    }

    #[test]
    fn a_job_without_a_class_table_says_so_instead_of_guessing() {
        let mut state = BuildPlannerState::default();
        state.start(PlannerBaseline { job_id: 13, ..baseline() });
        assert!(state.hp_sp_text.contains("no class table for this job"), "{}", state.hp_sp_text);
    }

    #[test]
    fn raising_a_stat_spends_its_exact_cost() {
        let mut state = BuildPlannerState::default();
        let mut live = baseline();
        live.stats = PlannedStats::new(1, 1, 1, 1, 1, 1);
        state.start(live);
        let before = state.plan().remaining_stat_points();
        state.adjust_stat(StatKind::Strength, 1);
        assert_eq!(state.plan().stats.get_stat(StatKind::Strength), 2);
        assert_eq!(
            before - state.plan().remaining_stat_points(),
            crate::world::stat_upgrade_cost(1)
        );
    }

    #[test]
    fn raising_a_stat_without_points_is_refused_and_says_so() {
        let mut state = BuildPlannerState::default();
        let mut live = baseline();
        live.base_level = 1;
        live.stats = PlannedStats::new(90, 90, 90, 90, 90, 90);
        state.start(live);
        state.adjust_stat(StatKind::Strength, 1);
        assert_eq!(state.plan().stats.get_stat(StatKind::Strength), 90);
        assert!(state.status_text.contains("insufficient"), "{}", state.status_text);
    }

    #[test]
    fn lowering_a_stat_frees_points_for_another() {
        let mut state = BuildPlannerState::default();
        state.start(baseline());
        let before = state.plan().remaining_stat_points();
        state.adjust_stat(StatKind::Luck, -1);
        assert_eq!(state.plan().stats.get_stat(StatKind::Luck), 4);
        assert!(state.plan().remaining_stat_points() > before);
        assert!(state.luck_text.contains("(-1)"), "{}", state.luck_text);
    }

    #[test]
    fn a_level_that_cannot_hold_the_spent_points_is_refused() {
        let mut state = BuildPlannerState::default();
        state.start(baseline());
        let spent = state.plan().spent_stat_points();
        // Walk the level down until the plan stops following.
        for _ in 0..80 {
            state.adjust_base_level(-1);
        }
        let plan = state.plan();
        assert!(plan.total_stat_points() >= spent);
        assert!(plan.target_base_level > 1);
        assert!(state.status_text.contains("exceed"), "{}", state.status_text);
    }

    #[test]
    fn reset_returns_to_the_live_baseline() {
        let mut state = BuildPlannerState::default();
        state.start(baseline());
        state.adjust_stat(StatKind::Luck, -1);
        state.adjust_base_level(10);
        state.reset();
        assert_eq!(state.plan().stats, baseline().stats);
        assert_eq!(state.plan().target_base_level, 50);
    }

    #[test]
    fn stats_never_pass_the_server_maximum() {
        let mut state = BuildPlannerState::default();
        let mut live = baseline();
        live.stats = PlannedStats::new(99, 1, 1, 1, 1, 1);
        live.base_level = 99;
        state.start(live);
        state.adjust_stat(StatKind::Strength, 1);
        assert_eq!(state.plan().stats.get_stat(StatKind::Strength), 99);
        assert!(state.status_text.contains("maximum"));
    }

    fn knight() -> PlannerBaseline {
        PlannerBaseline {
            job_id: 7,
            base_level: 60,
            job_level: 40,
            // NV_BASIC 9 and SM_SWORD 10 learned, three points unspent.
            skills: HashMap::from([(1, 9), (2, 10)]),
            skill_points: 3,
            ..baseline()
        }
    }

    #[test]
    fn skill_budget_is_the_live_unspent_points_not_the_job_level() {
        let mut state = BuildPlannerState::default();
        state.start(knight());
        // 49 skill points would be "job level 40 - 1 = 39" under the legacy
        // rule, which a second class's live levels already exceed.
        assert_eq!(state.plan().remaining_skill_points(), 3);
        assert_eq!(state.plan().total_skill_points(), 22);
    }

    #[test]
    fn raising_a_skill_spends_a_point_and_follows_prerequisites() {
        let mut state = BuildPlannerState::default();
        state.start(knight());

        // Two-Hand Quicken needs Two-Hand Sword Mastery Lv 1, which is not learned.
        state.adjust_skill(60, 1);
        assert_eq!(state.plan().skills.get(&60), None);
        assert!(state.status_text.contains("SM_TWOHAND"), "{}", state.status_text);

        state.adjust_skill(3, 1);
        state.adjust_skill(60, 1);
        assert_eq!(state.plan().skills.get(&3), Some(&1));
        assert_eq!(state.plan().skills.get(&60), Some(&1));
        assert_eq!(state.plan().remaining_skill_points(), 1);
    }

    #[test]
    fn a_prerequisite_cannot_be_lowered_under_a_planned_skill() {
        let mut state = BuildPlannerState::default();
        state.start(knight());
        state.adjust_skill(3, 1);
        state.adjust_skill(60, 1);
        state.adjust_skill(3, -1);
        assert_eq!(state.plan().skills.get(&3), Some(&1));
        assert!(state.status_text.contains("cannot reduce"), "{}", state.status_text);
    }

    #[test]
    fn levels_the_character_already_has_cannot_be_taken_back() {
        let mut state = BuildPlannerState::default();
        state.start(knight());
        state.adjust_skill(2, -1);
        assert_eq!(state.plan().skills.get(&2), Some(&10));
        assert!(state.status_text.contains("already learned"), "{}", state.status_text);
    }

    #[test]
    fn skill_points_run_out_and_job_levels_add_one_each() {
        let mut state = BuildPlannerState::default();
        state.start(knight());
        for _ in 0..3 {
            state.adjust_skill(55, 1);
        }
        assert_eq!(state.plan().remaining_skill_points(), 0);
        state.adjust_skill(55, 1);
        assert_eq!(state.plan().skills.get(&55), Some(&3));
        assert!(state.status_text.contains("no unallocated"), "{}", state.status_text);

        state.adjust_job_level(2);
        assert_eq!(state.plan().remaining_skill_points(), 2);
        // Earned job levels cannot be un-earned below the live level.
        state.adjust_job_level(-10);
        assert_eq!(state.plan().target_job_level, 40);
    }

    #[test]
    fn the_list_has_one_row_per_tree_skill_with_the_blocker_shown() {
        let mut state = BuildPlannerState::default();
        state.start(knight());
        let rows = &state.skill_rows;
        assert_eq!(rows.len(), reference_data().job_skill_tree_by_id(7).unwrap().skills.len());
        let quicken = rows.iter().find(|row| row.skill_id == 60).expect("row");
        assert!(
            quicken.text.contains("0/10") && quicken.text.contains("SM_TWOHAND"),
            "{}",
            quicken.text
        );
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("korangar-planner-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn a_saved_plan_loads_back_identically() {
        let directory = scratch_dir("roundtrip");
        let mut state = BuildPlannerState::default();
        state.start(knight());
        state.adjust_stat(StatKind::Luck, -1);
        state.adjust_skill(3, 1);
        state.adjust_job_level(2);
        let saved = state.plan().clone();
        state.save_slot(&directory, 2).unwrap();

        let mut fresh = BuildPlannerState::default();
        fresh.start(knight());
        fresh.load_slot(&directory, 2).unwrap();
        assert_eq!(fresh.plan().stats, saved.stats);
        assert_eq!(fresh.plan().skills, saved.skills);
        assert_eq!(fresh.plan().target_job_level, saved.target_job_level);
        assert!(fresh.status_text.contains("Loaded slot 2"), "{}", fresh.status_text);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn an_empty_slot_is_reported_and_changes_nothing() {
        let directory = scratch_dir("empty");
        let mut state = BuildPlannerState::default();
        state.start(knight());
        let before = state.plan().clone();
        let error = state.load_slot(&directory, 3).unwrap_err();
        assert_eq!(error, "slot 3 is empty");
        assert_eq!(*state.plan(), before);
    }

    #[test]
    fn a_plan_for_another_job_is_refused() {
        let directory = scratch_dir("job");
        let mut wizard = BuildPlannerState::default();
        wizard.start(PlannerBaseline { job_id: 9, ..knight() });
        wizard.save_slot(&directory, 1).unwrap();
        // Put the wizard's file where the knight's slot would be.
        std::fs::copy(
            BuildPlannerState::slot_path(&directory, 9, 1),
            BuildPlannerState::slot_path(&directory, 7, 1),
        )
        .unwrap();

        let mut state = BuildPlannerState::default();
        state.start(knight());
        let before = state.plan().clone();
        let error = state.load_slot(&directory, 1).unwrap_err();
        assert!(error.contains("job 9"), "{error}");
        assert_eq!(*state.plan(), before);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_plan_that_no_longer_fits_is_refused_and_leaves_the_plan_alone() {
        let directory = scratch_dir("stale");
        let mut state = BuildPlannerState::default();
        state.start(knight());
        state.adjust_skill(3, 1);
        state.save_slot(&directory, 1).unwrap();

        // The same character, but with no points left to have bought that.
        let mut poorer = BuildPlannerState::default();
        poorer.start(PlannerBaseline {
            skill_points: 0,
            ..knight()
        });
        let before = poorer.plan().clone();
        let error = poorer.load_slot(&directory, 1).unwrap_err();
        assert!(error.contains("no longer fits"), "{error}");
        assert_eq!(*poorer.plan(), before);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn loading_cannot_undo_levels_the_character_has_since_learned() {
        let directory = scratch_dir("learned");
        let mut state = BuildPlannerState::default();
        state.start(knight());
        state.save_slot(&directory, 1).unwrap();

        // Since saving, the character learned Spear Mastery 2.
        let mut later = BuildPlannerState::default();
        later.start(PlannerBaseline {
            skills: HashMap::from([(1, 9), (2, 10), (55, 2)]),
            ..knight()
        });
        later.load_slot(&directory, 1).unwrap();
        assert_eq!(later.plan().skills.get(&55), Some(&2));
        let _ = std::fs::remove_dir_all(&directory);
    }

    fn at_level_99(job_id: u16, stats: PlannedStats) -> PlannerBaseline {
        PlannerBaseline {
            job_id,
            base_level: 99,
            stats,
            ..baseline()
        }
    }

    #[test]
    fn the_stat_cap_comes_from_the_jobs_parameter_group() {
        // unit_parameters_db.conf: BabyFirstClasses -> 80, ThirdClasses -> 130,
        // FirstClasses -> 99.
        let mut baby = BuildPlannerState::default();
        baby.start(at_level_99(4023, PlannedStats::new(80, 1, 1, 1, 1, 1)));
        baby.adjust_stat(StatKind::Strength, 1);
        assert_eq!(baby.plan().stats.get_stat(StatKind::Strength), 80, "a baby class stops at 80");
        assert!(baby.status_text.contains("maximum"), "{}", baby.status_text);

        let mut third = BuildPlannerState::default();
        third.start(at_level_99(4054, PlannedStats::new(99, 1, 1, 1, 1, 1)));
        third.adjust_stat(StatKind::Strength, 1);
        assert_eq!(
            third.plan().stats.get_stat(StatKind::Strength),
            100,
            "a third class goes past 99"
        );

        let mut novice = BuildPlannerState::default();
        novice.start(at_level_99(0, PlannedStats::new(99, 1, 1, 1, 1, 1)));
        novice.adjust_stat(StatKind::Strength, 1);
        assert_eq!(novice.plan().stats.get_stat(StatKind::Strength), 99);
    }

    #[test]
    fn upper_classes_get_the_extra_points_including_transcendent_third_classes() {
        let total = |job_id| {
            let mut state = BuildPlannerState::default();
            state.start(at_level_99(job_id, PlannedStats::new(1, 1, 1, 1, 1, 1)));
            state.plan().total_stat_points()
        };
        // statpoint.txt: 1273 at level 99; pc_resetstate adds 52 for JOBL_UPPER.
        assert_eq!(total(7), 1273, "Knight is not an upper class");
        assert_eq!(total(4008), 1273 + 52, "Lord Knight is");
        assert_eq!(total(4054), 1273, "Rune Knight is not");
        assert_eq!(
            total(4060),
            1273 + 52,
            "Rune Knight Trans is (the old 4001..=4021 guess missed it)"
        );
        assert_eq!(total(4014), 1273, "4014 has no job_db block and is no longer guessed upper");
    }

    #[test]
    fn a_job_the_tables_do_not_name_falls_back_to_a_plain_99_cap() {
        let mut state = BuildPlannerState::default();
        // 13 is Knight on a Peco Peco: no job_db.conf block.
        assert!(reference_data().stat_job(13).is_none());
        state.start(at_level_99(13, PlannedStats::new(99, 1, 1, 1, 1, 1)));
        state.adjust_stat(StatKind::Strength, 1);
        assert_eq!(state.plan().stats.get_stat(StatKind::Strength), 99);
        assert_eq!(state.plan().total_stat_points(), 1273);
    }

    #[test]
    fn the_planners_point_table_is_the_servers() {
        let rules = &reference_data().stat_rules;
        assert_eq!(rules.points_at_level.len(), crate::world::STAT_POINTS_TABLE.len());
        assert!(
            rules
                .points_at_level
                .iter()
                .zip(crate::world::STAT_POINTS_TABLE)
                .all(|(server, client)| server == client),
            "the client table has drifted from db/re/statpoint.txt"
        );
        assert_eq!(rules.upper_class_extra_points, 52);
        assert_eq!(rules.points_at_level(99), Some(1273));
        assert_eq!(rules.points_at_level(175), Some(3278));
        assert_eq!(rules.points_at_level(0), None);
        assert_eq!(rules.points_at_level(176), None);
    }
}
