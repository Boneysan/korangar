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

        // Status ATK (Melee) = floor(BaseLv/4) + STR + floor(DEX/5) + floor(LUK/3)
        let status_atk = (lv / 4) + str_val + (dex_val / 5) + (luk_val / 3);
        let status_atk_formula = format!(
            "floor({lv}/4) [{}] + STR [{str_val}] + floor({dex_val}/5) [{}] + floor({luk_val}/3) [{}] = {status_atk} {}",
            lv / 4,
            dex_val / 5,
            luk_val / 3,
            Provenance::Exact.label()
        );

        // Status MATK = floor(BaseLv/4) + INT + floor(INT/2) + floor(DEX/5) +
        // floor(LUK/3)
        let status_matk = (lv / 4) + int_val + (int_val / 2) + (dex_val / 5) + (luk_val / 3);
        let status_matk_formula = format!(
            "floor({lv}/4) [{}] + INT [{int_val}] + floor({int_val}/2) [{}] + floor({dex_val}/5) [{}] + floor({luk_val}/3) [{}] = \
             {status_matk} {}",
            lv / 4,
            int_val / 2,
            dex_val / 5,
            luk_val / 3,
            Provenance::Exact.label()
        );

        // HIT = 175 + BaseLv + DEX + floor(LUK/3)
        let hit = 175 + lv + dex_val + (luk_val / 3);
        let hit_formula = format!(
            "175 + BaseLv [{lv}] + DEX [{dex_val}] + floor({luk_val}/3) [{}] = {hit} {}",
            luk_val / 3,
            Provenance::Exact.label()
        );

        // FLEE = 100 + BaseLv + AGI + floor(LUK/5)
        let flee = 100 + lv + agi_val + (luk_val / 5);
        let flee_formula = format!(
            "100 + BaseLv [{lv}] + AGI [{agi_val}] + floor({luk_val}/5) [{}] = {flee} {}",
            luk_val / 5,
            Provenance::Exact.label()
        );

        // Soft DEF = floor((BaseLv + VIT) / 2) + floor(AGI / 5)
        let soft_def = ((lv + vit_val) / 2) + (agi_val / 5);
        let soft_def_formula = format!(
            "floor(({lv} + {vit_val})/2) [{}] + floor({agi_val}/5) [{}] = {soft_def} {}",
            (lv + vit_val) / 2,
            agi_val / 5,
            Provenance::Exact.label()
        );

        // Soft MDEF = INT + floor(VIT/5) + floor(DEX/5) + floor(BaseLv/4)
        let soft_mdef = int_val + (vit_val / 5) + (dex_val / 5) + (lv / 4);
        let soft_mdef_formula = format!(
            "INT [{int_val}] + floor({vit_val}/5) [{}] + floor({dex_val}/5) [{}] + floor({lv}/4) [{}] = {soft_mdef} {}",
            vit_val / 5,
            dex_val / 5,
            lv / 4,
            Provenance::Exact.label()
        );

        // CRIT = 1.0 + (LUK * 0.3)
        let crit = 1.0 + (luk_val as f32 * 0.3);
        let crit_formula = format!("1.0 + ({luk_val} * 0.3) = {crit:.1} {}", Provenance::Exact.label());

        // Perfect Dodge = 1.0 + (LUK * 0.1)
        let perfect_dodge = 1.0 + (luk_val as f32 * 0.1);
        let perfect_dodge_formula = format!("1.0 + ({luk_val} * 0.1) = {perfect_dodge:.1} {}", Provenance::Exact.label());

        // Variable Cast Reduction = min(100%, (DEX*2 + INT) / 530 * 100%)
        let cast_stat_sum = (dex_val * 2 + int_val) as f32;
        let variable_cast_reduction_pct = ((cast_stat_sum / 530.0) * 100.0).clamp(0.0, 100.0);
        let variable_cast_formula = format!(
            "(({dex_val} * 2 + {int_val}) / 530) * 100% = {variable_cast_reduction_pct:.1}% {}",
            Provenance::Estimate.label()
        );

        // Max Weight = BaseWeight + (BaseSTR * 300)
        let max_weight = input.base_weight + (input.strength as u32 * 300);
        let max_weight_formula = format!(
            "Base [{}] + (BaseSTR [{}] * 300) = {max_weight} {}",
            input.base_weight,
            input.strength,
            Provenance::Exact.label()
        );

        // Breakpoint tracking
        let mut breakpoints = Vec::new();
        let vit_sum = (lv + vit_val) as u32;
        if vit_sum % 2 != 0 {
            breakpoints.push("Soft DEF: 1 more VIT or BaseLv will yield +1 DEF".to_owned());
        }
        let agi_rem = agi_val % 5;
        if agi_rem != 0 {
            breakpoints.push(format!("Soft DEF: {} more AGI will yield +1 DEF", 5 - agi_rem));
        }
        let dex_rem = dex_val % 5;
        if dex_rem != 0 {
            breakpoints.push(format!("Status ATK/MATK & Soft MDEF: {} more DEX will yield +1", 5 - dex_rem));
        }
        let luk_rem = luk_val % 3;
        if luk_rem != 0 {
            breakpoints.push(format!("CRIT & Status ATK/MATK: {} more LUK will yield +1", 3 - luk_rem));
        }
        let pd_rem = luk_val % 10;
        if pd_rem != 0 {
            breakpoints.push(format!("Perfect Dodge: {} more LUK will yield +1", 10 - pd_rem));
        }

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

        // STR = 88, DEX = 53, LUK = 11, BaseLv = 99
        // floor(99/4) = 24
        // floor(53/5) = 10
        // floor(11/3) = 3
        // Status ATK = 24 + 88 + 10 + 3 = 125
        assert_eq!(metrics.status_atk, 125);
        assert!(metrics.status_atk_formula.contains("(exact)"));

        // HIT = 175 + 99 + 53 + 3 = 330
        assert_eq!(metrics.hit, 330);
        assert!(metrics.hit_formula.contains("(exact)"));

        // AGI = 72, floor(11/5) = 2
        // FLEE = 100 + 99 + 72 + 2 = 273
        assert_eq!(metrics.flee, 273);
        assert!(metrics.flee_formula.contains("(exact)"));

        // VIT = 74, floor((99+74)/2) = 86, floor(72/5) = 14
        // Soft DEF = 86 + 14 = 100
        assert_eq!(metrics.soft_def, 100);
        assert!(metrics.soft_def_formula.contains("(exact)"));

        // Variable cast time labelled as estimate
        assert!(metrics.variable_cast_formula.contains("(estimate)"));
        assert!(metrics.variable_cast_reduction_pct > 0.0);

        // Max Weight: 2000 + 80 * 300 = 26000
        assert_eq!(metrics.max_weight, 26000);
        assert!(metrics.max_weight_formula.contains("(exact)"));
    }
}
