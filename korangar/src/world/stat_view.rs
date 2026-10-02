//! Character stats interface modes (Simple, Detailed, Advanced) and combat
//! calculations.
//!
//! Complies with GDD §10.8 and §7.6:
//! - Simple: Plain-language description of what each stat improves plus current
//!   major effects.
//! - Detailed: Derived stat changes for the next point and selected skills
//!   affected.
//! - Advanced: Relevant formulas, breakpoints, and exact component values with
//!   explicit exact-vs-estimate provenance labels.
//!
//! Formulations are traced from Hercules Renewal (`Hercules/src/map/status.c`
//! and `pc.c`).

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::stat_formulas;
use super::stat_preview::StatKind;

/// Display mode for the Character Stats interface (GDD §10.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StatViewMode {
    /// Plain-language description of what the stat improves plus current major
    /// effects.
    #[default]
    Simple,
    /// Derived stat changes for the next point and selected skills affected.
    Detailed,
    /// Relevant formulas, breakpoints, and exact component values where
    /// available.
    Advanced,
}

/// Provenance of a calculated or displayed combat value (GDD §10.8, §10.10,
/// Decision D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provenance {
    /// Exactly matches authoritative Hercules Renewal server calculation.
    Exact,
    /// Client-side simulation or partial estimate (e.g. variable cast reduction
    /// without gear procs).
    Estimate,
    /// Server script command or formula not yet modeled client-side.
    Unsupported,
}

impl Provenance {
    pub fn label(&self) -> &'static str {
        match self {
            Provenance::Exact => "(exact)",
            Provenance::Estimate => "(estimate)",
            Provenance::Unsupported => "(unmodeled)",
        }
    }
}

/// Baseline stats input for view mode calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterStatsInput {
    pub base_level: usize,
    pub job_level: usize,
    pub strength: u16,
    pub bonus_strength: i16,
    pub agility: u16,
    pub bonus_agility: i16,
    pub vitality: u16,
    pub bonus_vitality: i16,
    pub intelligence: u16,
    pub bonus_intelligence: i16,
    pub dexterity: u16,
    pub bonus_dexterity: i16,
    pub luck: u16,
    pub bonus_luck: i16,
    pub max_hp: usize,
    pub max_sp: usize,
    pub base_weight: u32,
}

impl CharacterStatsInput {
    pub fn total_str(&self) -> u16 {
        (self.strength as i32 + self.bonus_strength as i32).max(1) as u16
    }

    pub fn total_agi(&self) -> u16 {
        (self.agility as i32 + self.bonus_agility as i32).max(1) as u16
    }

    pub fn total_vit(&self) -> u16 {
        (self.vitality as i32 + self.bonus_vitality as i32).max(1) as u16
    }

    pub fn total_int(&self) -> u16 {
        (self.intelligence as i32 + self.bonus_intelligence as i32).max(1) as u16
    }

    pub fn total_dex(&self) -> u16 {
        (self.dexterity as i32 + self.bonus_dexterity as i32).max(1) as u16
    }

    pub fn total_luk(&self) -> u16 {
        (self.luck as i32 + self.bonus_luck as i32).max(1) as u16
    }

    pub fn get_stat(&self, stat: StatKind) -> (u16, i16) {
        match stat {
            StatKind::Strength => (self.strength, self.bonus_strength),
            StatKind::Agility => (self.agility, self.bonus_agility),
            StatKind::Vitality => (self.vitality, self.bonus_vitality),
            StatKind::Intelligence => (self.intelligence, self.bonus_intelligence),
            StatKind::Dexterity => (self.dexterity, self.bonus_dexterity),
            StatKind::Luck => (self.luck, self.bonus_luck),
        }
    }
}

/// Plain-language description of a stat's role and major effects (Simple Mode,
/// GDD §10.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleStatDescription {
    pub stat: StatKind,
    pub name: &'static str,
    pub summary: &'static str,
    pub major_effects: &'static [&'static str],
}

pub fn simple_stat_description(stat: StatKind) -> SimpleStatDescription {
    match stat {
        StatKind::Strength => SimpleStatDescription {
            stat,
            name: "Strength (STR)",
            summary: "Increases physical melee damage and expands inventory carrying capacity.",
            major_effects: &[
                "+1 Status Physical ATK per point (melee weapons)",
                "+30 Max Weight capacity per base point",
                "Scales weapon attack bonus based on weapon level",
            ],
        },
        StatKind::Agility => SimpleStatDescription {
            stat,
            name: "Agility (AGI)",
            summary: "Increases attack speed and evasion against monster attacks.",
            major_effects: &[
                "+1 FLEE (dodge rate) per point",
                "Significantly increases Attack Speed (ASPD)",
                "Contributes to Soft Physical Defense (+1 DEF per 5 AGI)",
            ],
        },
        StatKind::Vitality => SimpleStatDescription {
            stat,
            name: "Vitality (VIT)",
            summary: "Expands health pool, provides physical defense, and grants status resistance.",
            major_effects: &[
                "+1% Max HP per point",
                "+1 Soft Physical Defense per 2 VIT (+1 DEF per 2 BaseLv + VIT)",
                "+2% HP healing item effectiveness per point",
                "+1% resistance against Stun, Poison, and Silence per point",
            ],
        },
        StatKind::Intelligence => SimpleStatDescription {
            stat,
            name: "Intelligence (INT)",
            summary: "Boosts magic attack power, expands mana pool, and accelerates spell casting.",
            major_effects: &[
                "+1.5 Status Magic ATK (MATK) per point",
                "+1% Max SP per point and accelerates natural SP recovery",
                "+1 Soft Magic Defense (MDEF) per point",
                "Reduces Variable Cast Time (1 INT = half of 1 DEX toward reduction)",
            ],
        },
        StatKind::Dexterity => SimpleStatDescription {
            stat,
            name: "Dexterity (DEX)",
            summary: "Improves attack accuracy, ranged physical damage, and dramatically cuts cast time.",
            major_effects: &[
                "+1 HIT (attack accuracy) per point",
                "Primary driver for Variable Cast Time reduction (2x stronger than INT)",
                "+1 Status Physical ATK per point for ranged weapons (bows, guns)",
                "Increases minimum weapon damage stability and Attack Speed (ASPD)",
            ],
        },
        StatKind::Luck => SimpleStatDescription {
            stat,
            name: "Luck (LUK)",
            summary: "Versatile attribute improving critical strikes, lucky dodge, and minor attack stats.",
            major_effects: &[
                "+0.3 Critical Hit Rate (CRIT) per point (approx. +1 CRIT per 3.3 points)",
                "+0.1 Perfect Dodge (lucky flee) per point (10 LUK = +1 Perfect Dodge)",
                "+0.3 Status Physical and Magic ATK per point (+1 ATK/MATK per 3 LUK)",
                "+1 HIT per 3 LUK and +1 FLEE per 5 LUK",
            ],
        },
    }
}

/// Detailed derived stat changes for next point and affected skills (Detailed
/// Mode, GDD §10.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailedStatRow {
    pub stat: StatKind,
    pub base_value: u16,
    pub bonus_value: i16,
    pub next_point_deltas: Vec<String>,
    pub affected_skills: &'static [&'static str],
}

pub fn detailed_stat_row(stat: StatKind, input: &CharacterStatsInput) -> DetailedStatRow {
    let (base_value, bonus_value) = input.get_stat(stat);
    let mut next_point_deltas = Vec::new();

    match stat {
        StatKind::Strength => {
            next_point_deltas.push("+1 Status ATK (Melee)".to_owned());
            next_point_deltas.push("+30 Max Weight".to_owned());
            DetailedStatRow {
                stat,
                base_value,
                bonus_value,
                next_point_deltas,
                affected_skills: &[
                    "Bash",
                    "Magnum Break",
                    "Bowling Bash",
                    "Pierce",
                    "Brandish Spear",
                    "Spiral Pierce",
                    "Mammonite",
                    "Cart Revolution",
                    "Sonic Blow",
                ],
            }
        }
        StatKind::Agility => {
            next_point_deltas.push("+1 FLEE".to_owned());
            let current_agi = input.total_agi();
            let remainder = current_agi % 5;
            let needed = 5 - remainder;
            if needed == 1 {
                next_point_deltas.push("+1 Soft DEF (breakpoint hit!)".to_owned());
            } else {
                next_point_deltas.push(format!("Soft DEF breakpoint in {needed} AGI"));
            }
            DetailedStatRow {
                stat,
                base_value,
                bonus_value,
                next_point_deltas,
                affected_skills: &[
                    "Two-Hand Quicken",
                    "Adrenaline Rush",
                    "Spear Quicken",
                    "One-Hand Quicken",
                    "Flee survivability against mobs",
                ],
            }
        }
        StatKind::Vitality => {
            next_point_deltas.push("+1% Max HP".to_owned());
            next_point_deltas.push("+2% Potion recovery".to_owned());
            let current_vit = input.total_vit();
            let sum = input.base_level as u32 + current_vit as u32;
            if (sum + 1) % 2 == 0 {
                next_point_deltas.push("+1 Soft DEF (breakpoint hit!)".to_owned());
            } else {
                next_point_deltas.push("Soft DEF breakpoint in 1 VIT".to_owned());
            }
            let mdef_needed = 5 - (current_vit % 5);
            if mdef_needed == 1 {
                next_point_deltas.push("+1 Soft MDEF (breakpoint hit!)".to_owned());
            } else {
                next_point_deltas.push(format!("Soft MDEF breakpoint in {mdef_needed} VIT"));
            }
            DetailedStatRow {
                stat,
                base_value,
                bonus_value,
                next_point_deltas,
                affected_skills: &[
                    "Potion Pitcher (alchemist)",
                    "Sacrifice (paladin)",
                    "Grand Cross (HP drain threshold)",
                    "Stun resistance (vital in PvP/MvP)",
                ],
            }
        }
        StatKind::Intelligence => {
            next_point_deltas.push("+1.5 Status MATK".to_owned());
            next_point_deltas.push("+1 Soft MDEF".to_owned());
            next_point_deltas.push("+1% Max SP".to_owned());
            next_point_deltas.push("-0.19% Variable Cast (estimate)".to_owned());
            DetailedStatRow {
                stat,
                base_value,
                bonus_value,
                next_point_deltas,
                affected_skills: &[
                    "Fire Bolt / Cold Bolt / Lightning Bolt",
                    "Storm Gust / Lord of Vermilion / Meteor Storm",
                    "Heal / Sanctuary / Coluceo Heal",
                    "Magnificat (SP recovery acceleration)",
                    "Soul Strike / Napalm Beat",
                ],
            }
        }
        StatKind::Dexterity => {
            next_point_deltas.push("+1 HIT".to_owned());
            next_point_deltas.push("-0.38% Variable Cast (estimate)".to_owned());
            let current_dex = input.total_dex();
            let needed = 5 - (current_dex % 5);
            if needed == 1 {
                next_point_deltas.push("+1 Status ATK/MATK & +1 Soft MDEF (breakpoint hit!)".to_owned());
            } else {
                next_point_deltas.push(format!("Status ATK/MATK & Soft MDEF breakpoint in {needed} DEX"));
            }
            DetailedStatRow {
                stat,
                base_value,
                bonus_value,
                next_point_deltas,
                affected_skills: &[
                    "Double Strafe",
                    "Arrow Shower",
                    "Blitz Beat",
                    "Focused Arrow Strike",
                    "Variable cast reduction on ALL casted spells",
                ],
            }
        }
        StatKind::Luck => {
            next_point_deltas.push("+0.3 CRIT (approx)".to_owned());
            next_point_deltas.push("+0.1 Perfect Dodge".to_owned());
            let current_luk = input.total_luk();
            let crit_needed = 3 - (current_luk % 3);
            if crit_needed == 1 {
                next_point_deltas.push("+1 CRIT & +1 Status ATK/MATK & +1 HIT (breakpoint hit!)".to_owned());
            } else {
                next_point_deltas.push(format!("CRIT & Status ATK/MATK breakpoint in {crit_needed} LUK"));
            }
            let pd_needed = 10 - (current_luk % 10);
            if pd_needed == 1 {
                next_point_deltas.push("+1 Perfect Dodge (breakpoint hit!)".to_owned());
            } else {
                next_point_deltas.push(format!("Perfect Dodge breakpoint in {pd_needed} LUK"));
            }
            DetailedStatRow {
                stat,
                base_value,
                bonus_value,
                next_point_deltas,
                affected_skills: &[
                    "Auto-Blitz Beat (falcon assault trigger)",
                    "Turn Undead (priest)",
                    "Critical strike rate for all auto-attacks",
                ],
            }
        }
    }
}

/// Advanced combat metrics with mathematical formulas and exact component
/// breakdowns (Advanced Mode, GDD §10.8).
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancedStatMetrics {
    pub base_level: usize,
    pub job_level: usize,
    pub status_atk: i32,
    pub status_atk_formula: String,
    pub status_matk: i32,
    pub status_matk_formula: String,
    pub hit: i32,
    pub hit_formula: String,
    pub flee: i32,
    pub flee_formula: String,
    pub soft_def: i32,
    pub soft_def_formula: String,
    pub soft_mdef: i32,
    pub soft_mdef_formula: String,
    pub crit: f32,
    pub crit_formula: String,
    pub perfect_dodge: f32,
    pub perfect_dodge_formula: String,
    pub variable_cast_reduction_pct: f32,
    pub variable_cast_formula: String,
    pub max_weight: u32,
    pub max_weight_formula: String,
    pub breakpoints: Vec<String>,
}

impl AdvancedStatMetrics {
    /// Calculate exact Renewal derived combat metrics from character stats.
    pub fn calculate(input: &CharacterStatsInput) -> Self {
        let lv = input.base_level as i32;
        let str_val = input.total_str() as i32;
        let agi_val = input.total_agi() as i32;
        let vit_val = input.total_vit() as i32;
        let int_val = input.total_int() as i32;
        let dex_val = input.total_dex() as i32;
        let luk_val = input.total_luk() as i32;

        // Every formula below is `stat_formulas`, which mirrors `status.c`
        // expression for expression (including the float arithmetic that is
        // truncated once, not term by term).
        let exact = Provenance::Exact.label();

        // Status ATK, melee weapons. Bows, instruments, whips and guns swap STR
        // and DEX; the view does not know the equipped weapon type.
        let status_atk = stat_formulas::status_atk(lv, str_val, dex_val, luk_val, false);
        let status_atk_formula = format!(
            "STR [{str_val}] + DEX/5 [{dex_val}/5] + LUK/3 [{luk_val}/3] + BaseLv/4 [{lv}/4] = {:.2} -> {status_atk} (truncated once; \
             melee weapons, ranged weapons swap STR and DEX) {exact}",
            str_val as f32 + dex_val as f32 / 5.0 + luk_val as f32 / 3.0 + lv as f32 / 4.0,
        );

        // Status MATK: integer divisions.
        let status_matk = stat_formulas::status_matk(lv, int_val, dex_val, luk_val);
        let status_matk_formula = format!(
            "INT [{int_val}] + floor({int_val}/2) [{}] + floor({dex_val}/5) [{}] + floor({luk_val}/3) [{}] + floor({lv}/4) [{}] = \
             {status_matk} {exact}",
            int_val / 2,
            dex_val / 5,
            luk_val / 3,
            lv / 4,
        );

        let hit = stat_formulas::hit(lv, dex_val, luk_val);
        let hit_formula = format!(
            "175 + BaseLv [{lv}] + DEX [{dex_val}] + floor({luk_val}/3) [{}] = {hit} {exact}",
            luk_val / 3,
        );

        let flee = stat_formulas::flee(lv, agi_val, luk_val);
        let flee_formula = format!(
            "100 + BaseLv [{lv}] + AGI [{agi_val}] + floor({luk_val}/5) [{}] = {flee} {exact}",
            luk_val / 5,
        );

        // Soft DEF: (BaseLv + VIT) / 2 + AGI / 5, truncated once.
        let soft_def = stat_formulas::soft_def(lv, vit_val, agi_val);
        let soft_def_formula = format!(
            "({lv} + {vit_val})/2 + {agi_val}/5 = {:.2} -> {soft_def} (truncated once) {exact}",
            (lv + vit_val) as f32 / 2.0 + agi_val as f32 / 5.0,
        );

        // Soft MDEF: INT + BaseLv / 4 + (DEX + VIT) / 5, truncated once.
        let soft_mdef = stat_formulas::soft_mdef(lv, int_val, dex_val, vit_val);
        let soft_mdef_formula = format!(
            "{int_val} + {lv}/4 + ({dex_val} + {vit_val})/5 = {:.2} -> {soft_mdef} (truncated once) {exact}",
            int_val as f32 + lv as f32 / 4.0 + (dex_val + vit_val) as f32 / 5.0,
        );

        // CRIT = (10 + LUK * 10 / 3) tenths of a percent.
        let crit_tenths = stat_formulas::critical_tenths(luk_val);
        let crit = crit_tenths as f32 / 10.0;
        let crit_formula = format!("(10 + {luk_val} * 10 / 3) / 10 = {crit:.1} {exact}");

        // Perfect Dodge = (LUK + 10) tenths of a percent.
        let perfect_dodge = stat_formulas::perfect_dodge_tenths(luk_val) as f32 / 10.0;
        let perfect_dodge_formula = format!("({luk_val} + 10) / 10 = {perfect_dodge:.1} {exact}");

        // Variable cast: a square root of (DEX * 2 + INT) / 530. It removes this
        // share of the *variable* part of a cast only; the fixed part is
        // reduced by other bonuses, not by these stats.
        let variable_cast_reduction_pct = stat_formulas::variable_cast_reduction_percent(dex_val, int_val);
        let variable_cast_formula =
            format!("sqrt(({dex_val} * 2 + {int_val}) / 530) * 100% = {variable_cast_reduction_pct:.1}% of the variable cast time {exact}");

        // Max Weight = BaseWeight + (BaseSTR * 300)
        let max_weight = input.base_weight + (input.strength as u32 * 300);
        let max_weight_formula = format!(
            "Base [{}] + (BaseSTR [{}] * 300) = {max_weight} {}",
            input.base_weight,
            input.strength,
            Provenance::Exact.label()
        );

        // Breakpoints: how many more points of one stat raise a derived value,
        // found by searching the same functions the values come from. (The
        // server adds fractions before truncating, so a remainder test on one
        // stat alone is not enough: Soft DEF depends on BaseLv + VIT + AGI.)
        let mut breakpoints = Vec::new();
        let mut breakpoint = |label: &str, stat: &str, value_after: &dyn Fn(i32) -> i32| {
            if let Some(more) = (1..=10).find(|extra| value_after(*extra) > value_after(0)) {
                breakpoints.push(match more {
                    1 => format!("{label}: the next point of {stat} raises it by 1"),
                    _ => format!("{label}: {more} more {stat} raises it by 1"),
                });
            }
        };
        breakpoint("Soft DEF", "AGI", &|extra| {
            stat_formulas::soft_def(lv, vit_val, agi_val + extra)
        });
        breakpoint("Soft DEF", "VIT", &|extra| {
            stat_formulas::soft_def(lv, vit_val + extra, agi_val)
        });
        breakpoint("Soft MDEF", "VIT", &|extra| {
            stat_formulas::soft_mdef(lv, int_val, dex_val, vit_val + extra)
        });
        breakpoint("Soft MDEF", "DEX", &|extra| {
            stat_formulas::soft_mdef(lv, int_val, dex_val + extra, vit_val)
        });
        breakpoint("Status ATK", "DEX", &|extra| {
            stat_formulas::status_atk(lv, str_val, dex_val + extra, luk_val, false)
        });
        breakpoint("Status ATK", "LUK", &|extra| {
            stat_formulas::status_atk(lv, str_val, dex_val, luk_val + extra, false)
        });
        breakpoint("Status MATK", "DEX", &|extra| {
            stat_formulas::status_matk(lv, int_val, dex_val + extra, luk_val)
        });
        breakpoint("Status MATK", "LUK", &|extra| {
            stat_formulas::status_matk(lv, int_val, dex_val, luk_val + extra)
        });
        breakpoint("HIT", "LUK", &|extra| stat_formulas::hit(lv, dex_val, luk_val + extra));
        breakpoint("FLEE", "LUK", &|extra| stat_formulas::flee(lv, agi_val, luk_val + extra));
        breakpoint("CRIT", "LUK", &|extra| stat_formulas::critical_tenths(luk_val + extra) / 10);

        Self {
            base_level: input.base_level,
            job_level: input.job_level,
            status_atk,
            status_atk_formula,
            status_matk,
            status_matk_formula,
            hit,
            hit_formula,
            flee,
            flee_formula,
            soft_def,
            soft_def_formula,
            soft_mdef,
            soft_mdef_formula,
            crit,
            crit_formula,
            perfect_dodge,
            perfect_dodge_formula,
            variable_cast_reduction_pct,
            variable_cast_formula,
            max_weight,
            max_weight_formula,
            breakpoints,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_level_99_knight() -> CharacterStatsInput {
        CharacterStatsInput {
            base_level: 99,
            job_level: 50,
            strength: 80,
            bonus_strength: 8,
            agility: 70,
            bonus_agility: 2,
            vitality: 70,
            bonus_vitality: 4,
            intelligence: 10,
            bonus_intelligence: 0,
            dexterity: 50,
            bonus_dexterity: 3,
            luck: 9,
            bonus_luck: 2,
            max_hp: 12500,
            max_sp: 620,
            base_weight: 2000,
        }
    }

    #[test]
    fn simple_descriptions_cover_all_six_stats() {
        for stat in [
            StatKind::Strength,
            StatKind::Agility,
            StatKind::Vitality,
            StatKind::Intelligence,
            StatKind::Dexterity,
            StatKind::Luck,
        ] {
            let desc = simple_stat_description(stat);
            assert_eq!(desc.stat, stat);
            assert!(!desc.name.is_empty());
            assert!(!desc.summary.is_empty());
            assert!(!desc.major_effects.is_empty());
        }
    }

    #[test]
    fn detailed_stat_rows_generate_next_point_deltas_and_skill_links() {
        let input = sample_level_99_knight();
        for stat in [
            StatKind::Strength,
            StatKind::Agility,
            StatKind::Vitality,
            StatKind::Intelligence,
            StatKind::Dexterity,
            StatKind::Luck,
        ] {
            let row = detailed_stat_row(stat, &input);
            assert_eq!(row.stat, stat);
            assert!(!row.next_point_deltas.is_empty());
            assert!(!row.affected_skills.is_empty());
        }
    }

    #[test]
    fn advanced_metrics_calculate_exact_renewal_formulas_and_provenance() {
        let input = sample_level_99_knight();
        let metrics = AdvancedStatMetrics::calculate(&input);

        // STR = 88, DEX = 53, LUK = 11, BaseLv = 99. status_base_atk adds the
        // fractions before truncating: 88 + 10.6 + 3.667 + 24.75 = 127.017 -> 127.
        // (Flooring each term first, as this view once did, gives 125.)
        assert_eq!(metrics.status_atk, 127);
        assert!(metrics.status_atk_formula.contains("(exact)"));

        // HIT = 175 + 99 + 53 + 3 = 330
        assert_eq!(metrics.hit, 330);
        assert!(metrics.hit_formula.contains("(exact)"));

        // AGI = 72, floor(11/5) = 2
        // FLEE = 100 + 99 + 72 + 2 = 273
        assert_eq!(metrics.flee, 273);
        assert!(metrics.flee_formula.contains("(exact)"));

        // VIT = 74, AGI = 72: (99 + 74) / 2 + 72 / 5 = 86.5 + 14.4 = 100.9 -> 100.
        assert_eq!(metrics.soft_def, 100);
        assert!(metrics.soft_def_formula.contains("(exact)"));

        // Variable cast: sqrt((DEX * 2 + INT) / 530), a square root, not a line.
        assert!(metrics.variable_cast_formula.contains("sqrt("));
        assert!(metrics.variable_cast_formula.contains("(exact)"));
        assert!(metrics.variable_cast_reduction_pct > 0.0);
        let linear = ((input.total_dex() as f32 * 2.0 + input.total_int() as f32) / 530.0) * 100.0;
        assert!(
            metrics.variable_cast_reduction_pct > linear,
            "the square root removes more than a straight line below the scale"
        );

        // Max Weight: 2000 + 80 * 300 = 26000
        assert_eq!(metrics.max_weight, 26000);
        assert!(metrics.max_weight_formula.contains("(exact)"));
    }

    #[test]
    fn breakpoints_follow_the_summed_fraction_not_one_stat_alone() {
        let input = CharacterStatsInput {
            base_level: 3,
            job_level: 1,
            strength: 1,
            bonus_strength: 0,
            agility: 3,
            bonus_agility: 0,
            vitality: 2,
            bonus_vitality: 0,
            intelligence: 1,
            bonus_intelligence: 0,
            dexterity: 1,
            bonus_dexterity: 0,
            luck: 1,
            bonus_luck: 0,
            max_hp: 100,
            max_sp: 10,
            base_weight: 2000,
        };
        let metrics = AdvancedStatMetrics::calculate(&input);
        // (3 + 2) / 2 + 3 / 5 = 3.1 -> 3. AGI 7 gives 3.9 (still 3) and AGI 8
        // gives 4.1, so it takes 5 more AGI. A remainder test on AGI alone
        // would have said 2.
        assert_eq!(metrics.soft_def, 3);
        assert!(
            metrics.breakpoints.contains(&"Soft DEF: 5 more AGI raises it by 1".to_owned()),
            "{:?}",
            metrics.breakpoints
        );
        // VIT 4 gives 3.5 + 0.6 = 4.1, so 2 more VIT (1 more gives 3.6).
        assert!(
            metrics.breakpoints.contains(&"Soft DEF: 2 more VIT raises it by 1".to_owned()),
            "{:?}",
            metrics.breakpoints
        );
        assert!(
            !metrics.breakpoints.iter().any(|line| line.contains("2 more AGI")),
            "{:?}",
            metrics.breakpoints
        );
    }
}
