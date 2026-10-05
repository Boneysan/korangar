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
//! Forging and pharmacy feed the Crafting Odds window
//! (`interface/windows/crafting_odds.rs`, F30).

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
/// The random term `skill.c` adds to Potion Preparation for one product, as
/// `(lowest, highest)` basis points. Hercules rolls it per attempt (`case
/// ITEMID_RED_POTION: make_per += (1+rnd()%100)*10 + 2000`), so a single
/// "success rate" for a potion does not exist: only a range does. Products
/// not named in that switch (Blue Potion, Condensed Red, Anodyne, Aloevera,
/// Embryo, the resist potions) get nothing.
pub const fn pharmacy_product_roll(item_id: u32) -> (i32, i32) {
    match item_id {
        // Red, Yellow, White Potion.
        501 | 503 | 504 => (2010, 3000),
        // Alcohol.
        970 => (1010, 2000),
        // Bottle Grenade, Acid Bottle, Plant Bottle, Marine Sphere Bottle.
        7135..=7138 => (10, 1000),
        // Condensed Yellow Potion.
        546 => (-500, -10),
        // Condensed White Potion, Glistening Coat.
        547 | 7139 => (-1000, -10),
        _ => (0, 0),
    }
}

/// Potion Preparation success chance for one product (`skill.c`, the
/// `AM_PHARMACY` case of `skill_produce_mix`), as `(lowest, highest)` basis
/// points over the server's random roll, each clamped to 1..=10000.
///
/// `vanilmirth_instruction_lv` is the alchemist's Vanilmirth's Instruction
/// Change level (+1% each), 0 without a living homunculus. Baby classes are
/// halved after everything else, as on the server.
#[allow(clippy::too_many_arguments)] // mirrors skill.c's inputs one to one
pub fn pharmacy_success_range(
    job_level: i32,
    int_: i32,
    dex: i32,
    luk: i32,
    pharmacy_lv: i32,
    learning_potion_lv: i32,
    vanilmirth_instruction_lv: i32,
    item_id: u32,
    is_baby: bool,
) -> (i32, i32) {
    // skill.c: make_per = AM_LEARNINGPOTION*50 + AM_PHARMACY*300 + job_level*20
    //   + (int/2)*10 + dex*10 + luk*10; plus HVAN_INSTRUCT*100 with a homunculus.
    let base = (learning_potion_lv * 50)
        + (pharmacy_lv * 300)
        + (job_level * 20)
        + ((int_ / 2) * 10)
        + (dex * 10)
        + (luk * 10)
        + vanilmirth_instruction_lv.max(0) * 100;
    let (low, high) = pharmacy_product_roll(item_id);
    let finish = |rate: i32| {
        // potion_produce_rate is 100 here (conf/map/battle/items.conf), so no
        // scaling; then the baby penalty and the floor of 1.
        let rate = if is_baby { (rate * 50) / 100 } else { rate };
        rate.clamp(1, 10000)
    };
    (finish(base + low), finish(base + high))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[expect(dead_code, reason = "F30: cooking odds are not shown; the window covers forging and pharmacy only")]
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
#[expect(dead_code, reason = "F30: cooking odds are not shown; the window covers forging and pharmacy only")]
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
#[expect(dead_code, reason = "F30: cooking odds are not shown; the window covers forging and pharmacy only")]
pub fn mix_cooking_rating(job_level: i32, dex: i32, luk: i32) -> i32 {
    (job_level / 4) + (luk / 2) + (dex / 3)
}

/// Item ids of the four anvils (`ITEMID_ANVIL` .. `ITEMID_EMPERIUM_ANVIL`).
const ANVILS: [(u32, AnvilType); 4] = [
    (989, AnvilType::Emperium),
    (988, AnvilType::Golden),
    (987, AnvilType::Oridecon),
    (986, AnvilType::Normal),
];

/// The anvil forging would use: the server checks Emperium, Golden, Oridecon,
/// then plain, and takes the first one carried. `None` without any.
pub fn best_anvil(carried: impl IntoIterator<Item = u32>) -> Option<AnvilType> {
    let carried: Vec<u32> = carried.into_iter().collect();
    ANVILS.iter().find(|(id, _)| carried.contains(id)).map(|(_, anvil)| *anvil)
}

const BS_WEAPONRESEARCH: u16 = 107;
const BS_ORIDEOCON: u16 = 97;
const AM_LEARNINGPOTION: u16 = 227;
const AM_PHARMACY: u16 = 228;

/// The seven smithing skills. Skill level N unlocks weapon level N (every
/// `db/produce_db.txt` row for these skills requires exactly that).
const SMITHING: [(u16, &str); 7] = [
    (98, "Dagger"),
    (99, "Sword"),
    (100, "Two-Handed Sword"),
    (101, "Axe"),
    (102, "Mace"),
    (103, "Knuckle"),
    (104, "Spear"),
];

/// Potion Preparation products grouped by the roll they share
/// ([`pharmacy_product_roll`]); the item id stands for its group.
const PHARMACY_GROUPS: [(u32, &str); 6] = [
    (501, "Red, Yellow, White Potion"),
    (970, "Alcohol"),
    (7135, "Bottle Grenade, Acid, Plant, Marine Sphere Bottle"),
    (505, "Blue Potion, Condensed Red, Anodyne, Aloevera, Embryo, resist potions"),
    (546, "Condensed Yellow Potion"),
    (547, "Condensed White Potion, Glistening Coat"),
];

/// What the F30 odds window needs, as the server last reported it. Stats are
/// totals (base plus bonus), which is what `skill.c` reads.
#[derive(Clone, Debug, Default)]
pub struct CraftingOddsInput {
    pub job_level: i32,
    pub int_: i32,
    pub dex: i32,
    pub luk: i32,
    /// `(skill id, level)` for every learned skill.
    pub skills: Vec<(u16, u16)>,
    pub anvil: Option<AnvilType>,
    pub is_baby: bool,
}

impl CraftingOddsInput {
    fn level(&self, skill_id: u16) -> i32 {
        self.skills
            .iter()
            .find(|(id, _)| *id == skill_id)
            .map_or(0, |(_, level)| i32::from(*level))
    }
}

/// Whether `job_id` is a baby class, whose crafting the server halves
/// (`sd->job & JOBL_BABY`), from the job tables the server itself uses.
pub fn is_baby_class(job_id: u16) -> bool {
    crate::dm::reference_data::reference_data()
        .job_tables
        .job(job_id)
        .is_some_and(|job| job.baby)
}

fn percent(basis_points: i32) -> String {
    format!("{}.{:02}%", basis_points / 100, basis_points % 100)
}

/// The F30 window body: forging and potion odds for the character's own
/// stats, skills and carried anvil. Only crafts the character can attempt
/// are listed.
pub fn crafting_odds_text(input: &CraftingOddsInput) -> String {
    let mut lines = Vec::new();
    let anvil_name = |anvil: AnvilType| match anvil {
        AnvilType::Normal => "Anvil",
        AnvilType::Oridecon => "Oridecon Anvil",
        AnvilType::Golden => "Golden Anvil",
        AnvilType::Emperium => "Emperium Anvil",
    };

    let smithing: Vec<_> = SMITHING
        .iter()
        .map(|(id, name)| (*name, input.level(*id)))
        .filter(|(_, level)| *level > 0)
        .collect();
    if !smithing.is_empty() {
        let anvil = input.anvil.unwrap_or_default();
        let anvil_text = match input.anvil {
            Some(anvil) => format!("{} in your bag, +{}", anvil_name(anvil), percent(anvil.bonus_basis_points())),
            None => "no anvil in your bag, +0%".to_owned(),
        };
        lines.push(format!(
            "Weapon Forging (DEX {}, LUK {}, job level {}; {anvil_text})",
            input.dex, input.luk, input.job_level
        ));
        for (name, level) in smithing {
            let rates: Vec<String> = (1..=level.min(3))
                .map(|weapon_level| {
                    let rate = weapon_forge_success_rate(
                        input.job_level,
                        input.dex,
                        input.luk,
                        level,
                        input.level(BS_WEAPONRESEARCH),
                        input.level(BS_ORIDEOCON),
                        weapon_level,
                        false,
                        0,
                        anvil,
                        input.is_baby,
                    );
                    format!("Lv{weapon_level} weapon {}", percent(rate))
                })
                .collect();
            lines.push(format!("  {name} (skill Lv {level}): {}", rates.join(", ")));
        }
        lines.push("  Each Star Crumb costs 15%, an Elemental Stone 20%.".to_owned());
    }

    let pharmacy = input.level(AM_PHARMACY);
    if pharmacy > 0 {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(format!(
            "Potion Preparation (Pharmacy Lv {pharmacy}, Learning Potion Lv {})",
            input.level(AM_LEARNINGPOTION)
        ));
        for (item_id, name) in PHARMACY_GROUPS {
            let (low, high) = pharmacy_success_range(
                input.job_level,
                input.int_,
                input.dex,
                input.luk,
                pharmacy,
                input.level(AM_LEARNINGPOTION),
                0,
                item_id,
                input.is_baby,
            );
            let range = match low == high {
                true => percent(low),
                false => format!("{} to {}", percent(low), percent(high)),
            };
            lines.push(format!("  {name}: {range}"));
        }
        lines.push(
            "  The server rolls within each range per attempt. A Vanilmirth's Instruction Change adds 1% per level (not counted here)."
                .to_owned(),
        );
    }

    if lines.is_empty() {
        return "None of your skills forge weapons or prepare potions, so there are no odds to show. Blacksmiths and Alchemists see \
                theirs here."
            .to_owned();
    }
    if input.is_baby {
        lines.push(String::new());
        lines.push("Baby classes craft at half these odds; the figures above already include it.".to_owned());
    }
    lines.join("\n")
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
    fn pharmacy_range_follows_the_servers_roll() {
        // Job 50, INT 50, DEX 60, LUK 40, Pharmacy 10, Learning Potion 10:
        // 500 + 3000 + 1000 + 250 + 600 + 400 = 5750.
        // White Potion: + (1+rnd()%100)*10 + 2000, so +2010 ..= +3000.
        assert_eq!(pharmacy_success_range(50, 50, 60, 40, 10, 10, 0, 504, false), (7760, 8750));
        // Acid Bottle: + 10 ..= 1000.
        assert_eq!(pharmacy_success_range(50, 50, 60, 40, 10, 10, 0, 7136, false), (5760, 6750));
        // Blue Potion: no roll at all.
        assert_eq!(pharmacy_success_range(50, 50, 60, 40, 10, 10, 0, 505, false), (5750, 5750));
        // Condensed Yellow loses 10 ..= 500; Glistening Coat 10 ..= 1000.
        assert_eq!(pharmacy_success_range(50, 50, 60, 40, 10, 10, 0, 546, false), (5250, 5740));
        assert_eq!(pharmacy_success_range(50, 50, 60, 40, 10, 10, 0, 7139, false), (4750, 5740));
        // A Vanilmirth's Instruction Change Lv 5 adds 500 before the roll.
        assert_eq!(pharmacy_success_range(50, 50, 60, 40, 10, 10, 5, 505, false), (6250, 6250));
        // Baby: halved after the roll, as skill.c applies it last.
        assert_eq!(pharmacy_success_range(50, 50, 60, 40, 10, 10, 0, 504, true), (3880, 4375));
    }

    #[test]
    fn pharmacy_range_is_clamped_like_the_server_roll() {
        // A strong alchemist making Red Potions can exceed 10000, which the
        // server's rnd()%10000 < make_per treats as certain.
        assert_eq!(pharmacy_success_range(70, 120, 150, 120, 10, 10, 5, 501, false).1, 10000);
        // A novice-level attempt at Glistening Coat floors at 1, never negative.
        assert_eq!(pharmacy_success_range(1, 1, 1, 1, 1, 0, 0, 7139, false).0, 1);
    }

    #[test]
    fn best_anvil_takes_the_servers_order_not_the_bag_order() {
        assert_eq!(best_anvil([986, 988, 987]), Some(AnvilType::Golden));
        assert_eq!(best_anvil([986, 989]), Some(AnvilType::Emperium));
        assert_eq!(best_anvil([986]), Some(AnvilType::Normal));
        assert_eq!(best_anvil([501, 998]), None);
    }

    #[test]
    fn baby_crafters_are_recognised_and_adults_are_not() {
        // db/constants.conf: Job_Baby_Blacksmith 4033, Job_Baby_Alchemist 4041.
        assert!(is_baby_class(4033));
        assert!(is_baby_class(4041));
        // Blacksmith 10, Alchemist 18, Whitesmith 4011, Creator 4019.
        for adult in [10, 18, 4011, 4019] {
            assert!(!is_baby_class(adult), "job {adult}");
        }
    }

    fn smith(skills: &[(u16, u16)]) -> CraftingOddsInput {
        CraftingOddsInput {
            job_level: 50,
            int_: 1,
            dex: 60,
            luk: 40,
            skills: skills.to_vec(),
            anvil: Some(AnvilType::Normal),
            is_baby: false,
        }
    }

    #[test]
    fn odds_window_lists_only_the_weapon_levels_the_skill_unlocks() {
        // Sword 2, Weaponry Research 10: 5000 + 1000 + 600 + 400 + 1000 + 1000.
        let text = crafting_odds_text(&smith(&[(99, 2), (107, 10)]));
        assert!(
            text.contains("Sword (skill Lv 2): Lv1 weapon 90.00%, Lv2 weapon 70.00%"),
            "{text}"
        );
        assert!(!text.contains("Lv3 weapon"), "{text}");
        assert!(!text.contains("Dagger"), "an unlearned smithing skill is not listed: {text}");
        assert!(!text.contains("Potion Preparation"), "{text}");
    }

    #[test]
    fn odds_window_applies_oridecon_research_only_to_level_three() {
        // Mace 3, Oridecon Research 5: Lv3 gets +500 and pays -3000.
        // 5000 + 1000 + 600 + 400 + 1500 = 8500; Lv2 8500-2000; Lv3 8500-3000+500.
        let text = crafting_odds_text(&smith(&[(102, 3), (97, 5)]));
        assert!(
            text.contains("Lv1 weapon 85.00%, Lv2 weapon 65.00%, Lv3 weapon 60.00%"),
            "{text}"
        );
    }

    #[test]
    fn odds_window_names_the_anvil_and_its_bonus() {
        let mut input = smith(&[(98, 1)]);
        input.anvil = Some(AnvilType::Emperium);
        let text = crafting_odds_text(&input);
        assert!(text.contains("Emperium Anvil in your bag, +10.00%"), "{text}");
        // 5000 + 1000 + 600 + 400 + 500 + 1000.
        assert!(text.contains("Lv1 weapon 85.00%"), "{text}");
        input.anvil = None;
        assert!(crafting_odds_text(&input).contains("no anvil in your bag"));
    }

    #[test]
    fn odds_window_shows_potion_ranges_and_the_baby_note() {
        let mut input = smith(&[(228, 10), (227, 10)]);
        input.int_ = 50;
        let text = crafting_odds_text(&input);
        assert!(text.contains("Red, Yellow, White Potion: 77.60% to 87.50%"), "{text}");
        assert!(
            text.contains("Blue Potion, Condensed Red, Anodyne, Aloevera, Embryo, resist potions: 57.50%\n"),
            "{text}"
        );
        assert!(!text.contains("Weapon Forging"), "{text}");
        assert!(!text.contains("Baby"), "{text}");
        input.is_baby = true;
        let baby = crafting_odds_text(&input);
        assert!(baby.contains("Red, Yellow, White Potion: 38.80% to 43.75%"), "{baby}");
        assert!(baby.contains("Baby classes craft at half"), "{baby}");
    }

    #[test]
    fn odds_window_explains_itself_to_a_non_crafter() {
        let text = crafting_odds_text(&smith(&[(5, 10), (28, 10)]));
        assert!(text.starts_with("None of your skills"), "{text}");
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
