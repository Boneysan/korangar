//! Derived-stat next-point preview calculations and formatting for the Stats
//! window.
//!
//! Formulations match Hercules Renewal (`Hercules/src/map/status.c` and
//! `Hercules/src/map/skill.c`):
//! - HIT: +1 per DEX, +1 per 3 LUK (`st->hit += level + dex + luk / 3 + 175`)
//! - FLEE: +1 per AGI, +1 per 5 LUK (`st->flee += level + agi + luk / 5 + 100`)
//! - Soft DEF: `(int)((level + vit) / 2 + agi / 5)` in floats, truncated once
//!   (see `stat_formulas`)
//! - Soft MDEF: `(int)(int + level / 4 + (dex + vit) / 5)` in floats, truncated
//!   once (see `stat_formulas`)
//! - CRIT: +0.33 per LUK / 1 per 3 LUK (`st->cri += 10 + luk * 10 / 3` in 0.1%
//!   units)
//! - Perfect Dodge: +0.1 per LUK / 1 per 10 LUK (`st->flee2 += luk + 10` in
//!   0.1% units)
//! - Max HP / SP: the class table value, adjusted for upper/baby classes, then
//!   +1% per VIT / INT in integer steps (`status_get_base_maxhp`/`maxsp`);
//!   exact when the job is known, otherwise estimated from the live total
//! - Variable Cast: `(1 - sqrt((dex * 2 + int) / 530))` of the variable part
//!   (`skill_vfcastfix`); see `stat_formulas`, which holds the exact arithmetic
//! - Status ATK: +1 per STR (melee) / DEX (ranged), +0.2 opposite, +0.33 per
//!   LUK
//! - Max Weight: +30 per STR (+300 raw units)

use std::cmp::max;

use serde::{Deserialize, Serialize};

use super::stat_formulas;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatKind {
    Strength,
    Agility,
    Vitality,
    Intelligence,
    Dexterity,
    Luck,
}

impl StatKind {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Strength => "Strength",
            Self::Agility => "Agility",
            Self::Vitality => "Vitality",
            Self::Intelligence => "Intelligence",
            Self::Dexterity => "Dexterity",
            Self::Luck => "Luck",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatPreviewInput {
    pub base_level: usize,
    pub strength: i32,
    pub bonus_strength: i32,
    pub agility: i32,
    pub bonus_agility: i32,
    pub vitality: i32,
    pub bonus_vitality: i32,
    pub intelligence: i32,
    pub bonus_intelligence: i32,
    pub dexterity: i32,
    pub bonus_dexterity: i32,
    pub luck: i32,
    pub bonus_luck: i32,
    pub max_hp: usize,
    pub max_sp: usize,
    /// The character's job, when known: lets the HP/SP lines use the server's
    /// class tables instead of estimating from the live total.
    pub job_id: Option<u16>,
}

impl StatPreviewInput {
    #[allow(dead_code)]
    pub fn total_str(&self) -> i32 {
        self.strength + self.bonus_strength
    }

    pub fn total_agi(&self) -> i32 {
        self.agility + self.bonus_agility
    }

    pub fn total_vit(&self) -> i32 {
        self.vitality + self.bonus_vitality
    }

    pub fn total_int(&self) -> i32 {
        self.intelligence + self.bonus_intelligence
    }

    pub fn total_dex(&self) -> i32 {
        self.dexterity + self.bonus_dexterity
    }

    pub fn total_luk(&self) -> i32 {
        self.luck + self.bonus_luck
    }
}

/// Compute the next-point stat preview tooltip for the given stat.
/// The cast-time line: how much of the variable cast time DEX and INT remove
/// now, and after the next point (a square root of `(DEX * 2 + INT) / 530`).
fn variable_cast_line(dex: i32, int: i32, next_dex: i32, next_int: i32) -> String {
    format!(
        "· Variable Cast Time: {:.1}% -> {:.1}% of the variable part removed",
        stat_formulas::variable_cast_reduction_percent(dex, int),
        stat_formulas::variable_cast_reduction_percent(next_dex, next_int)
    )
}

pub fn stat_preview_tooltip(stat: StatKind, input: &StatPreviewInput, cost: u8, available_points: u32) -> String {
    let name = stat.name();
    let mut lines = Vec::with_capacity(7);

    // Header / affordability status
    if cost == 0 {
        lines.push(format!("^ff5555{name} (Max level reached)^000000"));
    } else if available_points < cost as u32 {
        lines.push(format!("^ffaa00{name} +1 (Need {cost} points, have {available_points})^000000"));
    } else {
        lines.push(format!(
            "^000001{name} +1^000000 (Cost: {cost} points · Have: {available_points})"
        ));
    }

    lines.push("^000001Next point (estimate):^000000".to_owned());

    match stat {
        StatKind::Strength => {
            lines.push("· Status ATK: +1 (melee) / +0.2 (ranged)".to_owned());
            lines.push("· Max Weight: +30".to_owned());
            lines.push("^888888[estimate] Increases melee physical attack and inventory weight limit.^000000".to_owned());
        }
        StatKind::Agility => {
            let agi = input.total_agi();
            let (level, vit) = (input.base_level as i32, input.total_vit());
            let def_delta = stat_formulas::soft_def(level, vit, agi + 1) - stat_formulas::soft_def(level, vit, agi);

            lines.push("· FLEE: +1".to_owned());
            lines.push("· Attack Speed: increases (reduces attack delay)".to_owned());
            lines.push(format!("· Soft DEF: +{def_delta}"));
            lines.push("^888888[estimate] Increases evasion rate, attack speed, and soft defense.^000000".to_owned());
        }
        StatKind::Vitality => {
            let vit = input.total_vit();
            let level = input.base_level;
            // Exact when the job's class table is known: HP is the table value
            // plus 1% per VIT in integer steps, so one point is worth the
            // difference of two evaluations. Gear bonuses add on top.
            let exact_hp_gain = input.job_id.and_then(|job| {
                let tables = &crate::dm::reference_data::reference_data().job_tables;
                Some(tables.base_max_hp(job, level, vit + 1)? as i64 - tables.base_max_hp(job, level, vit)? as i64)
            });
            let hp_gain = if input.max_hp > 0 {
                let est_base_hp = (input.max_hp as f32 / (1.0 + vit as f32 / 100.0)).round() as usize;
                max(1, (est_base_hp as f32 * 0.01).round() as usize)
            } else {
                1
            };

            let level = input.base_level as i32;
            let (agi, int, dex) = (input.total_agi(), input.total_int(), input.total_dex());
            let def_delta = stat_formulas::soft_def(level, vit + 1, agi) - stat_formulas::soft_def(level, vit, agi);
            let mdef_delta = stat_formulas::soft_mdef(level, int, dex, vit + 1) - stat_formulas::soft_mdef(level, int, dex, vit);

            lines.push(match exact_hp_gain {
                Some(gain) => format!("· Max HP: +{gain} (class table; gear bonuses add on top)"),
                None => format!("· Max HP: +1% Base HP (~+{hp_gain} HP)"),
            });
            lines.push(format!("· Soft DEF: +{def_delta}"));
            lines.push(format!("· Soft MDEF: +{mdef_delta}"));
            lines.push("· Natural HP Recovery: +1 HP per 5 VIT".to_owned());
            lines.push("^888888[estimate] Increases max HP, physical & magic defense, and healing received.^000000".to_owned());
        }
        StatKind::Intelligence => {
            let int_val = input.total_int();
            let sp_gain = if input.max_sp > 0 {
                let est_base_sp = (input.max_sp as f32 / (1.0 + int_val as f32 / 100.0)).round() as usize;
                max(1, (est_base_sp as f32 * 0.01).round() as usize)
            } else {
                1
            };
            let exact_sp_gain = input.job_id.and_then(|job| {
                let tables = &crate::dm::reference_data::reference_data().job_tables;
                let level = input.base_level;
                Some(tables.base_max_sp(job, level, int_val + 1)? as i64 - tables.base_max_sp(job, level, int_val)? as i64)
            });

            let matk_delta = ((int_val + 1) + (int_val + 1) / 2) - (int_val + int_val / 2);

            lines.push(match exact_sp_gain {
                Some(gain) => format!("· Max SP: +{gain} (class table; gear bonuses add on top)"),
                None => format!("· Max SP: +1% Base SP (~+{sp_gain} SP)"),
            });
            lines.push(format!("· Status MATK: +{matk_delta}"));
            let (level, dex, vit) = (input.base_level as i32, input.total_dex(), input.total_vit());
            let mdef_delta = stat_formulas::soft_mdef(level, int_val + 1, dex, vit) - stat_formulas::soft_mdef(level, int_val, dex, vit);
            lines.push(format!("· Soft MDEF: +{mdef_delta}"));
            lines.push(variable_cast_line(dex, int_val, dex, int_val + 1));
            lines.push("· Natural SP Recovery: +1 SP per 6 INT".to_owned());
            lines.push("^888888[estimate] Increases magic attack, max SP, magic defense, and cast speed.^000000".to_owned());
        }
        StatKind::Dexterity => {
            let dex = input.total_dex();
            let vit = input.total_vit();
            let (level, int) = (input.base_level as i32, input.total_int());
            let mdef_delta = stat_formulas::soft_mdef(level, int, dex + 1, vit) - stat_formulas::soft_mdef(level, int, dex, vit);

            lines.push("· HIT: +1".to_owned());
            lines.push(variable_cast_line(dex, int, dex + 1, int));
            lines.push("· Status ATK: +1 (ranged) / +0.2 (melee)".to_owned());
            lines.push("· Attack Speed: slightly increases".to_owned());
            lines.push(format!("· Soft MDEF: +{mdef_delta}"));
            lines.push("^888888[estimate] Increases hit rate, cast speed, ranged attack, and weapon stability.^000000".to_owned());
        }
        StatKind::Luck => {
            let luk = input.total_luk();
            let crit_delta = ((10 + (luk + 1) * 10 / 3) / 10) - ((10 + luk * 10 / 3) / 10);
            let hit_delta = ((luk + 1) / 3) - (luk / 3);
            let flee_delta = ((luk + 1) / 5) - (luk / 5);
            let flee2_delta = (((luk + 1) + 10) / 10) - ((luk + 10) / 10);

            if crit_delta > 0 {
                lines.push("· Critical (CRIT): +1 (breakpoint: 3 LUK) (+0.33% rate)".to_owned());
            } else {
                lines.push("· Critical (CRIT): +0 (+1 at next multiple of 3 LUK) (+0.33% rate)".to_owned());
            }
            if hit_delta > 0 {
                lines.push("· HIT: +1 (breakpoint reached: 3 LUK)".to_owned());
            } else {
                lines.push("· HIT: +0 (+1 at next multiple of 3 LUK)".to_owned());
            }
            if flee_delta > 0 {
                lines.push("· FLEE: +1 (breakpoint reached: 5 LUK)".to_owned());
            } else {
                lines.push("· FLEE: +0 (+1 at next multiple of 5 LUK)".to_owned());
            }
            if flee2_delta > 0 {
                lines.push("· Perfect Dodge: +1 (breakpoint reached: 10 LUK)".to_owned());
            } else {
                lines.push("· Perfect Dodge: +0 (+1 at next multiple of 10 LUK)".to_owned());
            }
            lines.push("· Status ATK / MATK: +0.33".to_owned());
            lines.push("^888888[estimate] Increases critical rate, accuracy, evasion, and perfect dodge.^000000".to_owned());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_preview_shows_status_atk_and_weight() {
        let input = StatPreviewInput {
            strength: 20,
            bonus_strength: 5,
            ..Default::default()
        };
        let text = stat_preview_tooltip(StatKind::Strength, &input, 3, 10);
        assert!(text.contains("Status ATK: +1 (melee) / +0.2 (ranged)"));
        assert!(text.contains("Max Weight: +30"));
        assert!(text.contains("[estimate]"));
        assert!(text.contains("Cost: 3 points · Have: 10"));
    }

    #[test]
    fn agi_preview_shows_flee_and_def_breakpoint() {
        // AGI 4 -> 5 reaches the +1 Soft DEF breakpoint (5 AGI).
        let input_break = StatPreviewInput {
            agility: 4,
            bonus_agility: 0,
            ..Default::default()
        };
        let text_break = stat_preview_tooltip(StatKind::Agility, &input_break, 2, 5);
        assert!(text_break.contains("FLEE: +1"));
        assert!(text_break.contains("Soft DEF: +1"), "{text_break}");

        // AGI 5 -> 6 does not reach a breakpoint.
        let input_no_break = StatPreviewInput {
            agility: 5,
            bonus_agility: 0,
            ..Default::default()
        };
        let text_no_break = stat_preview_tooltip(StatKind::Agility, &input_no_break, 2, 5);
        assert!(text_no_break.contains("Soft DEF: +0"), "{text_no_break}");
    }

    #[test]
    fn vit_preview_shows_max_hp_and_def_breakpoints() {
        let input = StatPreviewInput {
            base_level: 50,
            vitality: 29,
            bonus_vitality: 1, // total vit 30
            max_hp: 4000,
            ..Default::default()
        };
        let text = stat_preview_tooltip(StatKind::Vitality, &input, 4, 10);
        assert!(text.contains("Max HP: +1% Base HP (~+31 HP)"));
        assert!(text.contains("[estimate]"));
    }

    #[test]
    fn int_preview_shows_matk_and_cast_time() {
        let input = StatPreviewInput {
            intelligence: 40,
            bonus_intelligence: 5,
            max_sp: 800,
            ..Default::default()
        };
        let text = stat_preview_tooltip(StatKind::Intelligence, &input, 5, 20);
        assert!(text.contains("Max SP: +1% Base SP"));
        assert!(text.contains("Soft MDEF: +1"));
        // sqrt(45 / 530) = 29.14% -> sqrt(46 / 530) = 29.46%.
        assert!(
            text.contains("Variable Cast Time: 29.1% -> 29.5% of the variable part removed"),
            "{text}"
        );
    }

    #[test]
    fn dex_preview_shows_hit_and_double_cast_scale() {
        let input = StatPreviewInput {
            dexterity: 30,
            bonus_dexterity: 2,
            ..Default::default()
        };
        let text = stat_preview_tooltip(StatKind::Dexterity, &input, 4, 10);
        assert!(text.contains("HIT: +1"));
        // DEX 32 -> 33 adds 2 to DEX * 2 + INT: sqrt(64 / 530) = 34.75% -> sqrt(66 /
        // 530) = 35.29%.
        assert!(
            text.contains("Variable Cast Time: 34.7% -> 35.3% of the variable part removed"),
            "{text}"
        );
        assert!(text.contains("Status ATK: +1 (ranged) / +0.2 (melee)"));
    }

    #[test]
    fn luk_preview_shows_breakpoints() {
        // LUK 2 -> 3 hits 3 LUK breakpoint for CRIT and HIT.
        let input_at_2 = StatPreviewInput {
            luck: 2,
            bonus_luck: 0,
            ..Default::default()
        };
        let text_at_2 = stat_preview_tooltip(StatKind::Luck, &input_at_2, 2, 10);
        assert!(text_at_2.contains("Critical (CRIT): +1 (breakpoint: 3 LUK)"));
        assert!(text_at_2.contains("HIT: +1 (breakpoint reached: 3 LUK)"));

        // LUK 3 -> 4 does not hit 3 LUK breakpoint.
        let input_at_3 = StatPreviewInput {
            luck: 3,
            bonus_luck: 0,
            ..Default::default()
        };
        let text_at_3 = stat_preview_tooltip(StatKind::Luck, &input_at_3, 2, 10);
        assert!(text_at_3.contains("Critical (CRIT): +0 (+1 at next multiple of 3 LUK)"));
        assert!(text_at_3.contains("HIT: +0 (+1 at next multiple of 3 LUK)"));
    }

    #[test]
    fn unaffordable_and_maxed_states() {
        let input = StatPreviewInput::default();
        let unaffordable = stat_preview_tooltip(StatKind::Strength, &input, 5, 2);
        assert!(unaffordable.contains("Need 5 points, have 2"));

        let maxed = stat_preview_tooltip(StatKind::Strength, &input, 0, 99);
        assert!(maxed.contains("Max level reached"));
    }

    #[test]
    fn soft_def_preview_adds_the_fractions_before_truncating() {
        // Level 3, VIT 1, AGI 4: (3 + 1) / 2 + 4 / 5 = 2.8 -> 2. One more VIT
        // gives 5 / 2 + 0.8 = 3.3 -> 3, so the next point is worth +1 Soft DEF.
        // Flooring each term separately (the old model) says (4 -> 5)/2 = 2 -> 2: +0.
        let input = StatPreviewInput {
            base_level: 3,
            vitality: 1,
            agility: 4,
            ..Default::default()
        };
        let text = stat_preview_tooltip(StatKind::Vitality, &input, 2, 10);
        assert!(text.contains("Soft DEF: +1"), "{text}");
    }

    #[test]
    fn hp_and_sp_gains_are_exact_when_the_job_is_known() {
        // Knight, level 50, VIT 30 -> 31. Class table HP[50] = 2208:
        // 2208 + 2208 * 31 / 100 = 2892 against 2208 + 2208 * 30 / 100 = 2870.
        let knight = StatPreviewInput {
            base_level: 50,
            vitality: 30,
            job_id: Some(7),
            max_hp: 3000,
            ..Default::default()
        };
        let text = stat_preview_tooltip(StatKind::Vitality, &knight, 4, 10);
        assert!(text.contains("Max HP: +22 (class table; gear bonuses add on top)"), "{text}");

        // Lord Knight is an upper class: 2208 + 25% = 2760 first, then VIT.
        // 2760 + 2760 * 31 / 100 = 3615 against 2760 + 2760 * 30 / 100 = 3588.
        let lord_knight = StatPreviewInput {
            job_id: Some(4008),
            ..knight
        };
        let text = stat_preview_tooltip(StatKind::Vitality, &lord_knight, 4, 10);
        assert!(text.contains("Max HP: +27 (class table; gear bonuses add on top)"), "{text}");

        // Wizard, level 50, INT 45 -> 46. SP[50] = 460:
        // 460 + 460 * 46 / 100 = 671 against 460 + 460 * 45 / 100 = 667.
        let wizard = StatPreviewInput {
            base_level: 50,
            intelligence: 45,
            job_id: Some(9),
            max_sp: 700,
            ..Default::default()
        };
        let text = stat_preview_tooltip(StatKind::Intelligence, &wizard, 5, 20);
        assert!(text.contains("Max SP: +4 (class table; gear bonuses add on top)"), "{text}");
    }

    #[test]
    fn an_unknown_job_keeps_the_labelled_estimate() {
        let input = StatPreviewInput {
            base_level: 50,
            vitality: 29,
            bonus_vitality: 1,
            max_hp: 3_000,
            job_id: Some(13),
            ..Default::default()
        };
        let text = stat_preview_tooltip(StatKind::Vitality, &input, 4, 10);
        assert!(text.contains("Max HP: +1% Base HP (~+"), "{text}");
    }
}
