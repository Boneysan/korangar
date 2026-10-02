//! Derived-stat next-point preview calculations and formatting for the Stats
//! window.
//!
//! Formulations match Hercules Renewal (`Hercules/src/map/status.c` and
//! `Hercules/src/map/skill.c`):
//! - HIT: +1 per DEX, +1 per 3 LUK (`st->hit += level + dex + luk / 3 + 175`)
//! - FLEE: +1 per AGI, +1 per 5 LUK (`st->flee += level + agi + luk / 5 + 100`)
//! - Soft DEF: +0.5 per VIT, +0.2 per AGI (`st->def2 += (level + vit) / 2 + agi
//!   / 5`)
//! - Soft MDEF: +1.0 per INT, +0.2 per DEX, +0.2 per VIT (`st->mdef2 += int +
//!   level / 4 + (dex + vit) / 5`)
//! - CRIT: +0.33 per LUK / 1 per 3 LUK (`st->cri += 10 + luk * 10 / 3` in 0.1%
//!   units)
//! - Perfect Dodge: +0.1 per LUK / 1 per 10 LUK (`st->flee2 += luk + 10` in
//!   0.1% units)
//! - Max HP: +1% base Max HP per VIT (`val += val * vit / 100`)
//! - Max SP: +1% base Max SP per INT (`val += val * int / 100`)
//! - Variable Cast: `dex * 2 + int` towards stat scale 530
//! - Status ATK: +1 per STR (melee) / DEX (ranged), +0.2 opposite, +0.33 per
//!   LUK
//! - Max Weight: +30 per STR (+300 raw units)

use std::cmp::max;

use serde::{Deserialize, Serialize};

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
            let def_delta = ((agi + 1) / 5) - (agi / 5);

            lines.push("· FLEE: +1".to_owned());
            lines.push("· Attack Speed: increases (reduces attack delay)".to_owned());
            if def_delta > 0 {
                lines.push("· Soft DEF: +1 (breakpoint reached: 5 AGI)".to_owned());
            } else {
                lines.push("· Soft DEF: +0 (+1 at next multiple of 5 AGI)".to_owned());
            }
            lines.push("^888888[estimate] Increases evasion rate, attack speed, and soft defense.^000000".to_owned());
        }
        StatKind::Vitality => {
            let vit = input.total_vit();
            let hp_gain = if input.max_hp > 0 {
                let est_base_hp = (input.max_hp as f32 / (1.0 + vit as f32 / 100.0)).round() as usize;
                max(1, (est_base_hp as f32 * 0.01).round() as usize)
            } else {
                1
            };

            let level = input.base_level as i32;
            let def_delta = ((level + vit + 1) / 2) - ((level + vit) / 2);
            let dex = input.total_dex();
            let mdef_delta = ((dex + vit + 1) / 5) - ((dex + vit) / 5);

            lines.push(format!("· Max HP: +1% Base HP (~+{hp_gain} HP)"));
            if def_delta > 0 {
                lines.push("· Soft DEF: +1 (breakpoint reached: 2 VIT)".to_owned());
            } else {
                lines.push("· Soft DEF: +0 (+1 at next multiple of 2 VIT)".to_owned());
            }
            if mdef_delta > 0 {
                lines.push("· Soft MDEF: +1 (breakpoint reached: 5 DEX+VIT)".to_owned());
            } else {
                lines.push("· Soft MDEF: +0 (+1 at next multiple of 5 DEX+VIT)".to_owned());
            }
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

            let matk_delta = ((int_val + 1) + (int_val + 1) / 2) - (int_val + int_val / 2);

            lines.push(format!("· Max SP: +1% Base SP (~+{sp_gain} SP)"));
            lines.push(format!("· Status MATK: +{matk_delta}"));
            lines.push("· Soft MDEF: +1".to_owned());
            lines.push("· Variable Cast Time: +1 towards stat scale (530)".to_owned());
            lines.push("· Natural SP Recovery: +1 SP per 6 INT".to_owned());
            lines.push("^888888[estimate] Increases magic attack, max SP, magic defense, and cast speed.^000000".to_owned());
        }
        StatKind::Dexterity => {
            let dex = input.total_dex();
            let vit = input.total_vit();
            let mdef_delta = ((dex + 1 + vit) / 5) - ((dex + vit) / 5);

            lines.push("· HIT: +1".to_owned());
            lines.push("· Variable Cast Time: +2 towards stat scale (530)".to_owned());
            lines.push("· Status ATK: +1 (ranged) / +0.2 (melee)".to_owned());
            lines.push("· Attack Speed: slightly increases".to_owned());
            if mdef_delta > 0 {
                lines.push("· Soft MDEF: +1 (breakpoint reached: 5 DEX+VIT)".to_owned());
            } else {
                lines.push("· Soft MDEF: +0 (+1 at next multiple of 5 DEX+VIT)".to_owned());
            }
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
        assert!(text_break.contains("Soft DEF: +1 (breakpoint reached: 5 AGI)"));

        // AGI 5 -> 6 does not reach a breakpoint.
        let input_no_break = StatPreviewInput {
            agility: 5,
            bonus_agility: 0,
            ..Default::default()
        };
        let text_no_break = stat_preview_tooltip(StatKind::Agility, &input_no_break, 2, 5);
        assert!(text_no_break.contains("Soft DEF: +0 (+1 at next multiple of 5 AGI)"));
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
        assert!(text.contains("Variable Cast Time: +1 towards stat scale (530)"));
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
        assert!(text.contains("Variable Cast Time: +2 towards stat scale (530)"));
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
}
