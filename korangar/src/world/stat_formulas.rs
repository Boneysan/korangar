//! Player derived-stat formulas, mirrored from Hercules' Renewal build.
//!
//! Every function here restates one expression in `Hercules/src/map/status.c`
//! or `skill.c` *with the same arithmetic*, because the arithmetic is the whole
//! point: several of these are computed in `float` and truncated **once**,
//! which is not the same as summing separately floored terms (level 3, VIT 2,
//! AGI 3 gives soft DEF 3 on the server and 2 if each term is floored first).
//!
//! Source-confirmed against the checkout this client ships against; none of it
//! has been observed on a live server. Values are *base* values: equipment,
//! cards and status effects add to them afterwards.

/// Maximum value of the `vcast_stat_scale` battle setting this server runs
/// (`conf/map/battle/skill.conf`).
pub const VCAST_STAT_SCALE: f32 = 530.0;

/// `status_calc_misc`: `st->hit += level + dex + luk / 3 + 175` (players).
pub fn hit(base_level: i32, dex: i32, luk: i32) -> i32 {
    base_level + dex + luk / 3 + 175
}

/// `status_calc_misc`: `st->flee += level + agi + luk / 5 + 100` (players).
pub fn flee(base_level: i32, agi: i32, luk: i32) -> i32 {
    base_level + agi + luk / 5 + 100
}

/// `status_calc_misc`: `st->def2 += (int)(((float)level + vit) / 2 + (float)agi
/// / 5)`. The sum is truncated once, after the fractions are added.
pub fn soft_def(base_level: i32, vit: i32, agi: i32) -> i32 {
    ((base_level as f32 + vit as f32) / 2.0 + agi as f32 / 5.0) as i32
}

/// `status_calc_misc`: `st->mdef2 += (int)(int_ + (float)level / 4 +
/// (float)(dex + vit) / 5)`. The sum is truncated once, after the fractions are
/// added.
pub fn soft_mdef(base_level: i32, int: i32, dex: i32, vit: i32) -> i32 {
    (int as f32 + base_level as f32 / 4.0 + (dex + vit) as f32 / 5.0) as i32
}

/// `status_calc_misc`: `st->cri += 10 + luk * 10 / 3`, in tenths of a percent
/// (integer division).
pub fn critical_tenths(luk: i32) -> i32 {
    10 + luk * 10 / 3
}

/// `status_calc_misc`: `st->flee2 += luk + 10`, in tenths of a percent.
pub fn perfect_dodge_tenths(luk: i32) -> i32 {
    luk + 10
}

/// `status_base_atk` (Renewal, players):
/// `(int)(dstr + (float)dex / 5 + (float)luk / 3 + (float)base_level / 4)`.
///
/// For bows, instruments, whips and guns the server swaps STR and DEX before
/// this, so `ranged` selects DEX as the primary stat. The sum is truncated
/// once.
pub fn status_atk(base_level: i32, str: i32, dex: i32, luk: i32, ranged: bool) -> i32 {
    let (primary, secondary) = match ranged {
        true => (dex, str),
        false => (str, dex),
    };
    (primary as f32 + secondary as f32 / 5.0 + luk as f32 / 3.0 + base_level as f32 / 4.0) as i32
}

/// `status_base_matk` (players): `int + int / 2 + dex / 5 + luk / 3 + level /
/// 4`, all integer divisions.
pub fn status_matk(base_level: i32, int: i32, dex: i32, luk: i32) -> i32 {
    int + int / 2 + dex / 5 + luk / 3 + base_level / 4
}

/// Share of the *variable* part of a cast that DEX and INT remove, as a
/// percentage (0-100).
///
/// `skill_vfcastfix`: `time = (1 - sqrt((dex * 2 + int) / vcast_stat_scale)) *
/// time`, floored at zero afterwards. It is a square root, not a straight line,
/// so the first points of DEX and INT matter most.
pub fn variable_cast_reduction_percent(dex: i32, int: i32) -> f32 {
    let stat = (dex * 2 + int).max(0) as f32;
    (stat / VCAST_STAT_SCALE).sqrt().min(1.0) * 100.0
}

/// The class facts `status_get_base_maxhp` and `status_get_base_maxsp` branch
/// on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClassFlags {
    pub upper: bool,
    pub baby: bool,
    pub super_novice: bool,
    pub expanded_super_novice: bool,
}

impl From<&crate::dm::reference_data::ReferenceJobTables> for ClassFlags {
    fn from(job: &crate::dm::reference_data::ReferenceJobTables) -> Self {
        Self {
            upper: job.upper,
            baby: job.baby,
            super_novice: job.super_novice,
            expanded_super_novice: job.expanded_super_novice,
        }
    }
}

/// `status_get_base_maxhp`: the class table value, +2000 for a Super Novice at
/// Base Level 99 (Expanded Super Novice at 150), +25% for upper classes or
/// x70% for babies, then +1% per VIT. All integer arithmetic. Equipment and
/// status bonuses are added afterwards by the server and are not included.
/// (The server also triples HP for a ranked Taekwon over level 90; that needs
/// a live ranking and is not modelled.) This is the raw base value, which is
/// what the server saves; the value it *shows* is clamped to at least 1
/// (`status_calc_maxhp`), so a table that collapses to 1 saves 0 and displays
/// 1.
pub fn base_max_hp(table_value: u64, base_level: usize, vit: i32, class: ClassFlags) -> u64 {
    let mut value = table_value;
    if class.super_novice && base_level >= 99 {
        value += 2000;
    }
    if class.expanded_super_novice && base_level >= 150 {
        value += 2000;
    }
    if class.upper {
        value += value * 25 / 100;
    } else if class.baby {
        value = value * 70 / 100;
    }
    value + value * vit.max(0) as u64 / 100
}

/// `status_get_base_maxsp`: the class table value, +25% for upper classes or
/// x70% for babies, then +1% per INT (integer arithmetic, before equipment and
/// status bonuses).
pub fn base_max_sp(table_value: u64, int: i32, class: ClassFlags) -> u64 {
    let mut value = table_value;
    if class.upper {
        value += value * 25 / 100;
    } else if class.baby {
        value = value * 70 / 100;
    }
    value + value * int.max(0) as u64 / 100
}

/// Base ASPD from stats and the job's base ASPD for the weapon, before status
/// effects, equipment bonuses and the class cap (`status_base_amotion_pc`,
/// Renewal ASPD):
/// `(int)(sqrt(dex^2 / 5 + agi^2 / 2) / 4 + 196 + skill_bonus * agi / 200) -
/// min(class_base, 200)`.
///
/// `ranged` is true for bows, instruments, whips and guns, where DEX counts for
/// `dex^2 / 7` instead of `dex^2 / 5`. `skill_bonus` is the sum of the passive
/// ASPD skills the server adds (Advanced Book, Single Action, Plagiarism,
/// Musical Lesson). The mixed `float`/`double` steps follow the C source.
pub fn base_aspd(dex: i32, agi: i32, class_base: u16, ranged: bool, skill_bonus: i32) -> i32 {
    let dex_term = (dex * dex) as f32 / if ranged { 7.0 } else { 5.0 };
    let stats = dex_term + (agi * agi) as f32 * 0.5;
    let temp = ((stats as f64).sqrt() * 0.25f32 as f64) as f32 + 196.0;
    let raw = (temp + (skill_bonus as f32 * agi as f32 / 200.0)) as i32;
    raw - i32::from(class_base.min(200))
}

/// Everything that feeds the server's attack-motion calculation for a player
/// (`status_base_amotion_pc` and the assembly in `status_calc_bl_main`, Renewal
/// ASPD). All fields are values the server derives from skills, equipment and
/// statuses; the defaults describe a bare character with none of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AspdInputs {
    pub agi: i32,
    pub dex: i32,
    /// `B` from [`class_aspd_base`].
    pub class_base: u16,
    /// Bow, instrument, whip or gun: DEX counts as `dex^2 / 7`.
    pub ranged: bool,
    /// Passive skill bonuses (see [`passive_aspd_bonus`]).
    pub passive_bonus: i32,
    /// Status bonuses that join the passive term: potions (`+4`/`+6`/`+9`),
    /// Quicken (`7`), Adrenaline Rush (`7`), Berserk (`15`), and so on
    /// (`status_calc_aspd` with flag 1).
    pub status_flat_bonus: i32,
    /// `st->aspd_rate`, 1000 by default. Mounts lower it (a Peco rider without
    /// Cavalier Mastery has 500); it multiplies the ASPD value itself.
    pub rate: i32,
    /// Percent of the gap to ASPD 195 that is closed: equipment `bAspdRate`
    /// plus the percentage statuses (`status_calc_aspd` with flag 2). May be
    /// negative.
    pub percent: i32,
    /// Equipment `bAspd` in milliseconds (`-10` per point; negative is faster).
    pub flat_ms: i32,
    /// Milliseconds the fixed status adjustments remove
    /// (`status_calc_fix_aspd`).
    pub fixed_ms: i32,
    pub max_aspd: u16,
}

impl Default for AspdInputs {
    fn default() -> Self {
        Self {
            agi: 1,
            dex: 1,
            class_base: 0,
            ranged: false,
            passive_bonus: 0,
            status_flat_bonus: 0,
            rate: 1000,
            percent: 0,
            flat_ms: 0,
            fixed_ms: 0,
            max_aspd: 190,
        }
    }
}

/// The server's attack motion in milliseconds, in the order the C code applies
/// the steps: base ASPD, the rate multiplier, the share of the gap to 195,
/// conversion to milliseconds, the flat equipment bonus, the fixed status
/// adjustments, and the clamp to the class cap and 2000.
pub fn attack_motion(inputs: &AspdInputs) -> u32 {
    let mut aspd = base_aspd(
        inputs.dex,
        inputs.agi,
        inputs.class_base,
        inputs.ranged,
        inputs.passive_bonus + inputs.status_flat_bonus,
    );
    if inputs.rate != 1000 {
        aspd = aspd * inputs.rate / 1000;
    }
    // `amotion += (max(0xc3 - amotion, 2) * (aspd_rate2 + calc_aspd(2))) / 100`
    aspd += (195 - aspd).max(2) * inputs.percent / 100;
    let motion = 10 * (200 - aspd) + inputs.flat_ms - inputs.fixed_ms;
    let fastest = 10 * (200 - i32::from(inputs.max_aspd));
    motion.clamp(fastest, 2000) as u32
}

/// The passive ASPD skills the server adds to the base formula (all
/// whole-number divisions): Advanced Book with a book `(level - 1) / 2 + 1`,
/// Single Action `(level + 1) / 2` (any weapon), Plagiarism `level`, and
/// Musical Lesson with an instrument `level`.
pub fn passive_aspd_bonus(
    book: bool,
    instrument: bool,
    advanced_book: i32,
    single_action: i32,
    plagiarism: i32,
    musical_lesson: i32,
) -> i32 {
    let mut bonus = 0;
    if book && advanced_book > 0 {
        bonus += (advanced_book - 1) / 2 + 1;
    }
    if single_action > 0 {
        bonus += (single_action + 1) / 2;
    }
    if plagiarism > 0 {
        bonus += plagiarism;
    }
    if instrument && musical_lesson > 0 {
        bonus += musical_lesson;
    }
    bonus
}

/// The class base ASPD `B` for an equipment setup (`status_base_amotion_pc`):
/// the right-hand weapon's value, plus a quarter of the left-hand weapon's
/// value when dual wielding, plus the Shield value when a shield is worn.
pub fn class_aspd_base(right_weapon: u16, left_weapon: Option<u16>, shield: u16) -> u16 {
    right_weapon + left_weapon.map_or(0, |value| value / 4) + shield
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_and_flee_use_integer_division_of_luck() {
        // 99 + 99 + 99/3 + 175 and 99 + 99 + 99/5 + 100.
        assert_eq!(hit(99, 99, 99), 99 + 99 + 33 + 175);
        assert_eq!(flee(99, 99, 99), 99 + 99 + 19 + 100);
        assert_eq!(hit(1, 1, 2), 1 + 1 + 0 + 175);
    }

    #[test]
    fn soft_def_truncates_the_float_sum_once() {
        // (3 + 2) / 2 = 2.5 and 3 / 5 = 0.6, so 3.1 -> 3. Flooring each term
        // first gives 2 + 0 = 2, which is what the client used to show.
        assert_eq!(soft_def(3, 2, 3), 3);
        // 1.0 + 0.8 = 1.8 -> 1.
        assert_eq!(soft_def(1, 1, 4), 1);
        // A carry across the terms: 0.5 + 0.8 = 1.3 -> 1 (floors would give 0).
        assert_eq!(soft_def(0, 1, 4), 1);
        assert_eq!(soft_def(99, 99, 99), 99 + 19);
    }

    #[test]
    fn soft_mdef_truncates_the_float_sum_once() {
        // 0 + 5 / 4 + (2 + 2) / 5 = 1.25 + 0.8 = 2.05 -> 2; floors give 1 + 0 = 1.
        assert_eq!(soft_mdef(5, 0, 2, 2), 2);
        // 99 + 24.75 + 39.6 = 163.35 -> 163; floored term by term it would be 162.
        assert_eq!(soft_mdef(99, 99, 99, 99), 163);
    }

    #[test]
    fn critical_is_a_third_of_luck_not_three_tenths() {
        // 10 + 99 * 10 / 3 = 340 tenths = 34.0; the old 1.0 + 99 * 0.3 gave 30.7.
        assert_eq!(critical_tenths(99), 340);
        assert_eq!(critical_tenths(0), 10);
        assert_eq!(critical_tenths(2), 10 + 6);
        assert_eq!(perfect_dodge_tenths(99), 109);
    }

    #[test]
    fn status_atk_truncates_once_and_swaps_stats_for_ranged_weapons() {
        // Melee: 99 + 99/5 + 99/3 + 99/4 = 99 + 19.8 + 33 + 24.75 = 176.55 -> 176.
        assert_eq!(status_atk(99, 99, 99, 99, false), 176);
        // STR 99, DEX 50, LUK 30, level 90.
        // Melee: 99 + 10 + 10 + 22.5 = 141.5 -> 141.
        assert_eq!(status_atk(90, 99, 50, 30, false), 141);
        // Ranged swaps them: 50 + 99/5 + 10 + 22.5 = 102.3 -> 102.
        assert_eq!(status_atk(90, 99, 50, 30, true), 102);
    }

    #[test]
    fn status_matk_uses_integer_divisions() {
        // 99 + 49 + 19 + 33 + 24.
        assert_eq!(status_matk(99, 99, 99, 99), 99 + 49 + 19 + 33 + 24);
    }

    #[test]
    fn variable_cast_reduction_is_a_square_root_of_the_stat_scale() {
        assert_eq!(variable_cast_reduction_percent(0, 0), 0.0);
        // dex * 2 + int = 297 -> sqrt(297 / 530) = 0.7486 -> 74.9%, where a
        // straight line would say 56.0%.
        let reduction = variable_cast_reduction_percent(99, 99);
        assert!((reduction - 74.86).abs() < 0.05, "{reduction}");
        // The scale itself removes the whole variable part, and nothing goes past it.
        assert_eq!(variable_cast_reduction_percent(265, 0), 100.0);
        assert_eq!(variable_cast_reduction_percent(300, 100), 100.0);
        // Early points matter most: a fifth of the scale (106 of 530) already
        // removes 44.7% of the variable cast.
        assert!((variable_cast_reduction_percent(53, 0) - 44.7).abs() < 0.1);
    }

    fn knight() -> ClassFlags {
        ClassFlags::default()
    }

    #[test]
    fn base_max_hp_applies_class_adjustments_then_vit_in_integer_percent() {
        // Upper: 1000 + 250 = 1250, then + 1250 * 50 / 100 = 625.
        let upper = ClassFlags { upper: true, ..knight() };
        assert_eq!(base_max_hp(1000, 50, 50, upper), 1875);
        // Baby: 1000 * 70 / 100 = 700, then + 700 * 10 / 100 = 70.
        let baby = ClassFlags { baby: true, ..knight() };
        assert_eq!(base_max_hp(1000, 50, 10, baby), 770);
        // Integer division: 1001 + 1001 * 25 / 100 = 1001 + 250 = 1251.
        assert_eq!(base_max_hp(1001, 50, 0, upper), 1251);
        // No VIT, no adjustment: the table value as is.
        assert_eq!(base_max_hp(1000, 50, 0, knight()), 1000);
    }

    #[test]
    fn super_novices_get_their_flat_hp_before_the_percent_steps() {
        let super_novice = ClassFlags {
            super_novice: true,
            ..knight()
        };
        assert_eq!(base_max_hp(1000, 99, 0, super_novice), 3000, "+2000 at level 99");
        assert_eq!(base_max_hp(1000, 98, 0, super_novice), 1000, "not before level 99");
        // Expanded Super Novice is both: +2000 at 99 and another +2000 at 150.
        let expanded = ClassFlags {
            super_novice: true,
            expanded_super_novice: true,
            ..knight()
        };
        assert_eq!(base_max_hp(1000, 150, 0, expanded), 5000);
        assert_eq!(base_max_hp(1000, 149, 0, expanded), 3000);
    }

    #[test]
    fn base_max_sp_uses_the_same_class_steps_with_int() {
        let upper = ClassFlags { upper: true, ..knight() };
        // 160 + 160 * 25 / 100 = 200, then + 200 * 10 / 100 = 20.
        assert_eq!(base_max_sp(160, 10, upper), 220);
        // Super Novice has no flat SP bonus.
        let super_novice = ClassFlags {
            super_novice: true,
            ..knight()
        };
        assert_eq!(base_max_sp(160, 0, super_novice), 160);
        assert_eq!(base_max_sp(160, 10, knight()), 176);
    }

    #[test]
    fn base_aspd_follows_the_renewal_formula() {
        // Values computed independently: 196 + sqrt(dex^2/5 + agi^2/2)/4 + skill * agi
        // / 200, truncated, minus the job's base ASPD for the weapon (capped at
        // 200).
        assert_eq!(base_aspd(1, 1, 45, false, 0), 151);
        assert_eq!(base_aspd(99, 99, 45, false, 0), 171);
        // Ten points of passive ASPD skills add 10 * 99 / 200 = 4.95 before truncation.
        assert_eq!(base_aspd(99, 99, 45, false, 10), 176);
        assert_eq!(base_aspd(50, 70, 55, false, 0), 154);
        // Ranged weapons weigh DEX as dex^2 / 7.
        assert_eq!(base_aspd(99, 99, 0, true, 0), 215);
        assert!(base_aspd(99, 99, 0, true, 0) < base_aspd(99, 99, 0, false, 0));
        // The class base is capped at 200, so 250 behaves like 200.
        assert_eq!(base_aspd(1, 1, 250, false, 0), base_aspd(1, 1, 200, false, 0));
        assert_eq!(base_aspd(1, 1, 250, false, 0), -4);
    }

    #[test]
    fn class_base_adds_shield_and_a_quarter_of_the_off_hand_weapon() {
        assert_eq!(class_aspd_base(45, None, 0), 45);
        assert_eq!(class_aspd_base(45, None, 5), 50);
        // Dual daggers: 49 + 49 / 4 = 61.
        assert_eq!(class_aspd_base(49, Some(49), 0), 61);
    }

    #[test]
    fn attack_motion_is_ten_times_the_gap_and_respects_the_class_cap() {
        // A bare character with class base 0 and AGI/DEX 1: ASPD 196, motion 40,
        // which the class cap raises to 100 (190) or 70 (193).
        let bare = AspdInputs {
            class_base: 0,
            ..AspdInputs::default()
        };
        assert_eq!(attack_motion(&bare), 100);
        assert_eq!(attack_motion(&AspdInputs { max_aspd: 193, ..bare }), 70);
        // With a class base of 50: ASPD 146 and motion 540.
        assert_eq!(
            attack_motion(&AspdInputs {
                class_base: 50,
                ..AspdInputs::default()
            }),
            540
        );
        // A very slow character cannot exceed the slowest motion the server allows.
        assert_eq!(
            attack_motion(&AspdInputs {
                class_base: 200,
                rate: 100,
                ..AspdInputs::default()
            }),
            2000
        );
    }

    #[test]
    fn rate_percent_and_flat_follow_the_servers_order_with_c_truncation() {
        let base = AspdInputs {
            agi: 130,
            dex: 130,
            class_base: 40,
            ..AspdInputs::default()
        };
        // ASPD 183 with these stats; 183 * 500 / 1000 = 91 (a mounted Knight with no
        // mastery).
        assert_eq!(attack_motion(&AspdInputs { rate: 500, ..base }), 1090);
        // 10 % of the gap to 195: (195 - 183) * 10 / 100 = 1 (rounded toward zero).
        assert_eq!(attack_motion(&AspdInputs { percent: 10, ..base }), attack_motion(&base) - 10);
        // A negative percentage also rounds toward zero: (195 - 183) * -5 / 100 = 0.
        assert_eq!(attack_motion(&AspdInputs { percent: -5, ..base }), attack_motion(&base));
        // Flat bonuses are in milliseconds, after the conversion.
        assert_eq!(attack_motion(&AspdInputs { flat_ms: 50, ..base }), attack_motion(&base) + 50);
    }

    #[test]
    fn passive_aspd_skills_need_their_weapon_and_use_whole_number_division() {
        // Advanced Book: (level - 1) / 2 + 1, only with a book.
        assert_eq!(passive_aspd_bonus(true, false, 10, 0, 0, 0), 5);
        assert_eq!(passive_aspd_bonus(false, false, 10, 0, 0, 0), 0);
        assert_eq!(passive_aspd_bonus(true, false, 1, 0, 0, 0), 1);
        // Single Action: (level + 1) / 2, any weapon. Plagiarism: its level.
        assert_eq!(passive_aspd_bonus(false, false, 0, 9, 7, 0), 5 + 7);
        // Musical Lesson needs an instrument.
        assert_eq!(passive_aspd_bonus(false, true, 0, 0, 0, 10), 10);
        assert_eq!(passive_aspd_bonus(false, false, 0, 0, 0, 10), 0);
    }
}
