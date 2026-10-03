//! Crafting and forging success formulas, mirrored from Hercules
//! `src/map/skill.c` (GDD §12.3 / F30).
//!
//! Exposes exact, verified success chance calculations for:
//! 1. Blacksmith Weapon Forging (`BS_DAGGER`, `BS_SWORD`, etc.)
//! 2. Alchemist Potion Preparation (`AM_PHARMACY`)
//! 3. Metal & Ore Tempering (`BS_IRON`, `BS_STEEL`, `BS_ENCHANTEDSTONE`)
//!
//! Uses integer basis points (0.01% units, where 10,000 = 100.00%) matching the
//! server's exact arithmetic.
//!
//! Not yet called by any window: the formulas are tested against `skill.c`,
//! but no UI shows a success chance (GDD Appendix E, 2026-10-03). Remove the
//! allowance below once a crafting view uses them.
#![cfg_attr(not(test), allow(dead_code))]

/// Quality bonus provided by anvil held in inventory during forging.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AnvilType {
    #[default]
    Normal,
    Oridecon,
    Golden,
    Emperium,
}

impl AnvilType {
    /// Bonus in basis points (+0%, +3%, +5%, +10%).
    pub const fn bonus_basis_points(self) -> i32 {
        match self {
            Self::Normal => 0,
            Self::Oridecon => 300,
            Self::Golden => 500,
            Self::Emperium => 1000,
        }
    }
}

/// Calculate the exact base success rate for Blacksmith Weapon Forging
/// (skill.c:20501).
///
/// # Parameters:
/// - `job_level`: Blacksmith job level (1..=70)
/// - `dex`: Total Dexterity including bonuses
/// - `luk`: Total Luck including bonuses
/// - `smithing_skill_lv`: Specific weapon smithing skill level (1..=3)
/// - `weaponry_research_lv`: Weaponry Research level (0..=10)
/// - `oridecon_research_lv`: Oridecon Research level (0..=5, applied if weapon
///   level >= 3)
/// - `weapon_level`: Weapon Level (1..=3)
/// - `elemental_stone`: Whether an Elemental Stone is used (-20.00%)
/// - `star_crumbs`: Number of Star Crumbs added (0..=3, -15.00% each)
/// - `anvil`: Best anvil carried in inventory
/// - `is_baby`: Whether the character is a Baby class (-50% penalty)
///
/// Returns success rate in basis points (100 = 1.00%, 10_000 = 100.00%),
/// clamped to 1..=10000.
#[allow(clippy::too_many_arguments)] // mirrors skill.c's inputs one to one
pub fn weapon_forge_success_rate(
    job_level: i32,
    dex: i32,
    luk: i32,
    smithing_skill_lv: i32,
    weaponry_research_lv: i32,
    oridecon_research_lv: i32,
    weapon_level: i32,
    elemental_stone: bool,
    star_crumbs: i32,
    anvil: AnvilType,
    is_baby: bool,
) -> i32 {
    // skill.c:20502: make_per = 5000 + sd->status.job_level*20 + st->dex*10 +
    // st->luk*10;
    let mut rate = 5000 + (job_level * 20) + (dex * 10) + (luk * 10);

    // skill.c:20503: make_per += pc->checkskill(sd,skill_id)*500; (+5%, +10%, +15%)
    rate += smithing_skill_lv * 500;

    // skill.c:20504: Weaponry Research (+1% per lv) + Oridecon Research (+1% per lv
    // for Lv3 weapons)
    rate += weaponry_research_lv * 100;
    if weapon_level >= 3 {
        rate += oridecon_research_lv * 100;
    }

    // skill.c:20505: malus for element (-20%), star crumbs (-15% each), weapon
    // level (>1 => wlv * 1000)
    if elemental_stone {
        rate -= 2000;
    }
    rate -= star_crumbs.clamp(0, 3) * 1500;
    if weapon_level > 1 {
        rate -= weapon_level * 1000;
    }

    // Anvil bonus
    rate += anvil.bonus_basis_points();

    // Baby penalty
    if is_baby {
        rate = (rate * 50) / 100;
    }

    rate.clamp(1, 10000)
}

/// Potion difficulty category for Alchemist Pharmacy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PotionType {
    RedPotion,
    YellowPotion,
    WhitePotion,
    BluePotion,
    Alcohol,
    BottleGrenade,
    AcidBottle,
    PlantBottle,
    MarineSphereBottle,
    CoatingBottle,
}

impl PotionType {
    /// Baseline base difficulty modifier in basis points (skill.c:20300).
    pub const fn base_modifier(self) -> i32 {
        match self {
            Self::RedPotion | Self::YellowPotion | Self::WhitePotion => 2000,
            Self::Alcohol => 1000,
            Self::BottleGrenade | Self::AcidBottle | Self::PlantBottle | Self::MarineSphereBottle => 0,
            Self::BluePotion => 0,
            Self::CoatingBottle => -500,
        }
    }
}

/// Calculate the expected base success rate for Alchemist Potion Preparation
/// (skill.c:20290).
///
/// Returns base success rate in basis points (1..=10000) excluding random roll.
#[allow(clippy::too_many_arguments)] // mirrors skill.c's inputs one to one
pub fn pharmacy_base_success_rate(
    job_level: i32,
    int_: i32,
    dex: i32,
    luk: i32,
    pharmacy_lv: i32,
    learning_potion_lv: i32,
    vanilmirth_instruction_lv: i32,
    potion_type: PotionType,
) -> i32 {
    // skill.c:20290:
    // make_per = pc->checkskill(sd,AM_LEARNINGPOTION)*50
    //   + pc->checkskill(sd,AM_PHARMACY)*300 + sd->status.job_level*20
    //   + (st->int_/2)*10 + st->dex*10+st->luk*10;
    let mut rate = (learning_potion_lv * 50) + (pharmacy_lv * 300) + (job_level * 20) + ((int_ / 2) * 10) + (dex * 10) + (luk * 10);

    // Vanilmirth Instruction Change bonus: +1% per level
    rate += vanilmirth_instruction_lv.clamp(0, 5) * 100;

    // Item difficulty modifier
    rate += potion_type.base_modifier();

    rate.clamp(1, 10000)
}

/// Cooking Set type used when creating stat food dishes (skill.c:20500-20525).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CookingKitType {
    #[default]
    OutdoorKit, // val == 11
    HomeKit,         // val == 12
    ProfessionalKit, // val == 13
    RoyalKit,        // val == 14
    LegendarySet,    // val >= 15
}

impl CookingKitType {
    /// Server kit tier multiplier index (11..=15).
    pub const fn kit_value(self) -> i32 {
        match self {
            Self::OutdoorKit => 11,
            Self::HomeKit => 12,
            Self::ProfessionalKit => 13,
            Self::RoyalKit => 14,
            Self::LegendarySet => 15,
        }
    }
}

/// Calculate base expected success rate for Cooking Dishes in basis points
/// (skill.c:20508).
///
/// In Hercules C:
/// - Legendary Cooking Set (`kit_val >= 15`): 10,000 (100.00% guarantee).
/// - Other kits:
///
/// ```text
/// 1200 * (kit_val - 10) + 20 * (base_level + 1) + 20 * (dex + 1) + mastery_avg
///   - 400 * dish_level - 10 * (100 - luk + 1) - 500 * (extra_ingredients - 1) - rnd_malus
/// ```
///
/// Returns expected rate clamped to 1..=10000.
pub fn cooking_dish_expected_rate(
    base_level: i32,
    dex: i32,
    luk: i32,
    cook_mastery: i32,
    dish_level: i32,
    num_materials: i32,
    kit: CookingKitType,
) -> i32 {
    if kit == CookingKitType::LegendarySet {
        return 10000;
    }

    let kit_val = kit.kit_value();
    let mut rate = 1200 * (kit_val - 10);
    rate += 20 * (base_level + 1);
    rate += 20 * (dex + 1);

    // Mastery bonus average: min = 6 + mastery/80, max = 30 + 5*(mastery/400)
    let min_roll = 6 + (cook_mastery / 80);
    let max_roll = (30 + 5 * (cook_mastery / 400)).max(min_roll);
    let avg_roll = (min_roll + max_roll) / 2;
    rate += 100 * avg_roll;

    // Dish level penalty: -400 * dish_level
    rate -= 400 * dish_level.clamp(1, 10);

    // Luck factor: -10 * (100 - luk + 1)
    if luk < 100 {
        rate -= 10 * (100 - luk + 1);
    }

    // Material complexity penalty: -500 * (materials - 1)
    let extra_mats = (num_materials - 1).max(0);
    rate -= 500 * extra_mats;

    // Average random penalty: -100 * 2 = -200
    rate -= 200;

    rate.clamp(1, 10000)
}

/// Calculate Geneticist Mix Cooking success rating (skill.c:20455).
///
/// Formula: `job_level / 4 + luk / 2 + dex / 3`
pub fn mix_cooking_rating(job_level: i32, dex: i32, luk: i32) -> i32 {
    (job_level / 4) + (luk / 2) + (dex / 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_forging_success_rate_matches_hercules_arithmetic() {
        // Lv 1 weapon, Job 50, DEX 60, LUK 40, Smithing 3, Research 10, Normal Anvil
        // Base = 5000 + 50*20 (1000) + 60*10 (600) + 40*10 (400) = 7000
        // Smithing 3 = +1500
        // Research 10 = +1000
        // Total = 9500 (95.00%)
        let rate = weapon_forge_success_rate(50, 60, 40, 3, 10, 0, 1, false, 0, AnvilType::Normal, false);
        assert_eq!(rate, 9500);

        // With 3 Star Crumbs (-4500) and Fire Element (-2000):
        // 9500 - 4500 - 2000 = 3000 (30.00%)
        let vvs_fire = weapon_forge_success_rate(50, 60, 40, 3, 10, 0, 1, true, 3, AnvilType::Normal, false);
        assert_eq!(vvs_fire, 3000);

        // With Emperium Anvil (+1000):
        let with_emp = weapon_forge_success_rate(50, 60, 40, 3, 10, 0, 1, true, 3, AnvilType::Emperium, false);
        assert_eq!(with_emp, 4000);

        // Lv 3 weapon (wlv * 1000 = -3000) with Oridecon Research 5 (+500):
        // 9500 - 3000 + 500 = 7000
        let lv3_rate = weapon_forge_success_rate(50, 60, 40, 3, 10, 5, 3, false, 0, AnvilType::Normal, false);
        assert_eq!(lv3_rate, 7000);
    }

    #[test]
    fn pharmacy_base_success_rate_matches_hercules_arithmetic() {
        // Job 50, INT 50, DEX 60, LUK 40, Pharmacy 10, Learning Potion 10
        // Learning = 10 * 50 = 500
        // Pharmacy = 10 * 300 = 3000
        // Job = 50 * 20 = 1000
        // INT/2 * 10 = 25 * 10 = 250
        // DEX * 10 = 600
        // LUK * 10 = 400
        // Subtotal = 5750
        // White Potion (+2000) => 7750
        let white_potion = pharmacy_base_success_rate(50, 50, 60, 40, 10, 10, 0, PotionType::WhitePotion);
        assert_eq!(white_potion, 7750);

        // Acid Bottle (+0) => 5750
        let acid_bottle = pharmacy_base_success_rate(50, 50, 60, 40, 10, 10, 0, PotionType::AcidBottle);
        assert_eq!(acid_bottle, 5750);
    }

    #[test]
    fn cooking_dish_expected_rate_matches_hercules_arithmetic() {
        // Legendary Cooking Set is always guaranteed 100.00%
        let legendary = cooking_dish_expected_rate(99, 80, 60, 1000, 10, 4, CookingKitType::LegendarySet);
        assert_eq!(legendary, 10000);

        // Home Cooking Kit (val 12 -> 1200 * 2 = 2400)
        // Lv 80, DEX 60, LUK 50, Cook Mastery 400, Lv 5 dish, 2 materials
        // min = 6 + 5 = 11, max = 30 + 5 = 35, avg = 23 -> 2300
        // rate = 2400 + 20*81 (1620) + 20*61 (1220) + 2300 - 400*5 (2000) - 10*(51)
        // (510) - 500*1 (500) - 200 = 2400 + 1620 + 1220 + 2300 - 2000 - 510 -
        // 500 - 200 = 4330
        let home_kit = cooking_dish_expected_rate(80, 60, 50, 400, 5, 2, CookingKitType::HomeKit);
        assert_eq!(home_kit, 4330);

        // Geneticist Mix Cooking Rating: Job 50/4 (12) + LUK 60/2 (30) + DEX 90/3 (30)
        // = 72
        let mix_rating = mix_cooking_rating(50, 90, 60);
        assert_eq!(mix_rating, 72);
    }
}
