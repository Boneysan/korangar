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
}
