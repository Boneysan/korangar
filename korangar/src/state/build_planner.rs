use korangar_interface::element::StateElement;
use rust_state::RustState;

use crate::world::{BuildPlan, PlannedStats, ProjectedCombatStats, StatKind};

/// Hercules `conf/map/battle/player.conf` `max_parameter`.
pub const PLANNER_MAX_STAT: u16 = 99;
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
}

impl PlannerBaseline {
    fn projections(&self) -> ProjectedCombatStats {
        ProjectedCombatStats::calculate(
            self.base_level,
            self.job_level,
            &self.stats,
            None,
            self.hp_per_level_and_vit(),
            self.sp_per_level_and_int(),
            self.weight_without_strength(),
        )
    }

    /// Scale the live maximum HP back to the per-level baseline the
    /// projection formula multiplies, so the plan's *current* point reads
    /// exactly the live value and only the deltas are estimates.
    fn hp_per_level_and_vit(&self) -> usize {
        let divisor = self.base_level.max(1) * (100 + self.stats.vitality as usize);
        (self.max_hp * 100).checked_div(divisor).unwrap_or(0)
    }

    fn sp_per_level_and_int(&self) -> usize {
        let divisor = self.base_level.max(1) * (100 + self.stats.intelligence as usize);
        (self.max_sp * 100).checked_div(divisor).unwrap_or(0)
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

impl BuildPlannerState {
    /// Start (or restart) a plan at the live character's current point.
    pub fn start(&mut self, baseline: PlannerBaseline) {
        // Novice High (4001) through the transcendent second classes (4021)
        // receive the rebirth bonus points.
        let transcendent = (4001..=4021).contains(&baseline.job_id);
        let mut plan = BuildPlan::new("Planner", baseline.job_id, transcendent);
        plan.target_base_level = baseline.base_level;
        plan.target_job_level = baseline.job_level;
        plan.stats = baseline.stats.clone();
        self.plan = Some(plan);
        self.baseline = baseline;
        self.status_text = "Simulation only: nothing is sent to the server.".to_owned();
        self.refresh();
    }

    pub fn reset(&mut self) {
        let baseline = self.baseline.clone();
        self.start(baseline);
    }

    pub fn adjust_stat(&mut self, stat: StatKind, change: i8) {
        let Some(plan) = self.plan.as_mut() else { return };
        let result = match change >= 0 {
            true => plan.increase_stat(stat, PLANNER_MAX_STAT),
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
        let target = (plan.target_job_level as i32 + change as i32).max(1) as usize;
        let previous = plan.target_job_level;
        plan.set_target_job_level(target, PLANNER_MAX_JOB_LEVEL);
        if plan.spent_skill_points() > plan.total_skill_points() {
            plan.target_job_level = previous;
        }
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
            baseline.hp_per_level_and_vit(),
            baseline.sp_per_level_and_int(),
            baseline.weight_without_strength(),
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
            let cost = match value >= PLANNER_MAX_STAT {
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

        self.hp_sp_text = format!(
            "HP {} · SP {} (estimate)",
            delta(projected.max_hp_estimate as i64, current.max_hp_estimate as i64),
            delta(projected.max_sp_estimate as i64, current.max_sp_estimate as i64)
        );
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
        }
    }

    #[test]
    fn starting_point_reads_the_live_values_with_zero_deltas() {
        let mut state = BuildPlannerState::default();
        state.start(baseline());

        assert!(state.strength_text.contains("40 (+0)"), "{}", state.strength_text);
        // The baseline scaling means the unchanged plan reproduces live HP.
        let live = baseline().projections();
        assert!(live.max_hp_estimate.abs_diff(3_000) <= 50, "{}", live.max_hp_estimate);
        assert!(state.hp_sp_text.contains("(+0)"), "{}", state.hp_sp_text);
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
}
