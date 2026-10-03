//! Plain-language equipment, card, and script bonus interpretation and
//! contextual comparison.
//!
//! Complies with GDD §10.10, §7.6, and §5.13:
//! - Plain-language translation of Aegis / Hercules item scripts (`bonus bStr`,
//!   `bonus2 bAddRace`, etc.).
//! - Strict rejection and flagging of unsupported or unmodeled script commands
//!   rather than inventing them.
//! - Contextual "versus selected monster" comparison evaluating race, size,
//!   element, and weapon element.
//! - Exact-vs-estimate provenance tagging on all combat estimates and elemental
//!   effectiveness lookups.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::attack_element::elemental_effectiveness;
use super::stat_preview::StatKind;
use super::stat_view::Provenance;

/// Translated equipment or card bonus parsed from Hercules / Aegis script
/// syntax (GDD §10.10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemScriptBonus {
    /// Stat bonus (+/-) to a core attribute (e.g. `bStr, 3`).
    StatBonus { stat: StatKind, amount: i32 },
    /// Bonus to all six core stats (e.g. `bAllStats, 1`).
    AllStats(i32),
    /// Flat maximum HP increase (e.g. `bMaxHP, 700`).
    MaxHpFlat(i32),
    /// Percentage maximum HP increase (e.g. `bMaxHPrate, 10`).
    MaxHpPercent(i32),
    /// Flat maximum SP increase (e.g. `bMaxSP, 50`).
    MaxSpFlat(i32),
    /// Percentage maximum SP increase (e.g. `bMaxSPrate, 5`).
    MaxSpPercent(i32),
    /// Flat physical attack increase (e.g. `bBaseAtk, 20` or `bAtk, 10`).
    AtkFlat(i32),
    /// Percentage physical attack increase (e.g. `bAtkRate, 5`).
    AtkPercent(i32),
    /// Flat magic attack increase (e.g. `bMatk, 15`).
    MatkFlat(i32),
    /// Percentage magic attack increase (e.g. `bMatkRate, 5`).
    MatkPercent(i32),
    /// Flat hard physical defense increase (e.g. `bDef, 5`).
    DefFlat(i32),
    /// Percentage hard physical defense increase (e.g. `bDefRate, 5`).
    DefPercent(i32),
    /// Flat hard magic defense increase (e.g. `bMdef, 3`).
    MdefFlat(i32),
    /// Percentage hard magic defense increase (e.g. `bMdefRate, 5`).
    MdefPercent(i32),
    /// Attack accuracy increase (e.g. `bHit, 10`).
    Hit(i32),
    /// Evasion increase (e.g. `bFlee, 20`).
    Flee(i32),
    /// Lucky dodge increase (e.g. `bFlee2, 1`).
    PerfectDodge(i32),
    /// Critical hit rate increase (e.g. `bCritical, 4`).
    Critical(i32),
    /// Flat attack speed increase (e.g. `bAspd, 1`).
    AspdFlat(i32),
    /// Percentage attack speed increase (e.g. `bAspdRate, 5`).
    AspdPercent(i32),
    /// Movement speed percentage increase (e.g. `bSpeedRate, 10`).
    SpeedPercent(i32),
    /// Variable cast time percentage change (e.g. `bCastrate, -10`).
    CastTimePercent(i32),
    /// After-cast delay percentage change (e.g. `bDelayrate, -5`).
    AfterCastDelayPercent(i32),
    /// Double attack chance percentage (e.g. `bDoubleRate, 10`).
    DoubleAttackChance(i32),
    /// Critical hit damage reduction percentage (e.g. `bCriticalDef, 10`).
    CriticalDefPercent(i32),
    /// Physical or magical damage bonus vs monster race (e.g. `bAddRace,
    /// RC_DemiPlayer, 20`).
    RaceDamageBonus { race: String, percent: i32, is_magic: bool },
    /// Damage reduction from monster race (e.g. `bAddRaceTolerance,
    /// RC_DemiPlayer, 30`).
    RaceToleranceBonus { race: String, percent: i32 },
    /// Physical or magical damage bonus vs monster size (e.g. `bAddSize,
    /// Size_Medium, 15`).
    SizeDamageBonus { size: String, percent: i32, is_magic: bool },
    /// Damage reduction from monster size (e.g. `bSubSize, Size_Medium, 10`).
    SizeToleranceBonus { size: String, percent: i32 },
    /// Physical or magical damage bonus vs monster element (e.g. `bAddEle,
    /// Ele_Fire, 20`).
    ElementDamageBonus { element: String, percent: i32, is_magic: bool },
    /// Elemental damage resistance (e.g. `bSubEle, Ele_Ghost, -50` or `bSubEle,
    /// Ele_Fire, 20`).
    ElementResistanceBonus { element: String, percent: i32 },
    /// Endowed weapon attack element (e.g. `bAtkEle, Ele_Holy`).
    WeaponElement(String),
    /// Endowed armor defense element (e.g. `bDefEle, Ele_Fire`).
    ArmorElement(String),
    /// Weapon cannot be destroyed in combat (e.g. `bUnbreakableWeapon, 0`).
    IndestructibleWeapon,
    /// Armor cannot be destroyed in combat (e.g. `bUnbreakableArmor, 0`).
    IndestructibleArmor,
    /// Spell casting cannot be interrupted (e.g. `bNoCastCancel, 0`).
    UninterruptibleCast,
    /// Skills do not consume Gemstones (e.g. `bNoGemStone, 0`).
    NoGemStone,
    /// Script command or effect that is currently unmodeled client-side
    /// (rejected per GDD §10.10).
    Unsupported { raw: String, reason: String },
}

impl ItemScriptBonus {
    /// True if this bonus represents an unmodeled or rejected script command.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, ItemScriptBonus::Unsupported { .. })
    }

    /// Render human-readable plain-language text description (GDD §10.10).
    pub fn plain_text(&self) -> String {
        match self {
            ItemScriptBonus::StatBonus { stat, amount } => {
                let name = match stat {
                    StatKind::Strength => "STR",
                    StatKind::Agility => "AGI",
                    StatKind::Vitality => "VIT",
                    StatKind::Intelligence => "INT",
                    StatKind::Dexterity => "DEX",
                    StatKind::Luck => "LUK",
                };
                if *amount >= 0 {
                    format!("{name} +{amount}")
                } else {
                    format!("{name} {amount}")
                }
            }
            ItemScriptBonus::AllStats(amt) => {
                if *amt >= 0 {
                    format!("All Stats +{amt}")
                } else {
                    format!("All Stats {amt}")
                }
            }
            ItemScriptBonus::MaxHpFlat(amt) => format!("Max HP {amt:+}"),
            ItemScriptBonus::MaxHpPercent(amt) => format!("Max HP {amt:+}%"),
            ItemScriptBonus::MaxSpFlat(amt) => format!("Max SP {amt:+}"),
            ItemScriptBonus::MaxSpPercent(amt) => format!("Max SP {amt:+}%"),
            ItemScriptBonus::AtkFlat(amt) => format!("ATK {amt:+}"),
            ItemScriptBonus::AtkPercent(amt) => format!("ATK {amt:+}%"),
            ItemScriptBonus::MatkFlat(amt) => format!("MATK {amt:+}"),
            ItemScriptBonus::MatkPercent(amt) => format!("MATK {amt:+}%"),
            ItemScriptBonus::DefFlat(amt) => format!("DEF {amt:+}"),
            ItemScriptBonus::DefPercent(amt) => format!("DEF {amt:+}%"),
            ItemScriptBonus::MdefFlat(amt) => format!("MDEF {amt:+}"),
            ItemScriptBonus::MdefPercent(amt) => format!("MDEF {amt:+}%"),
            ItemScriptBonus::Hit(amt) => format!("HIT {amt:+}"),
            ItemScriptBonus::Flee(amt) => format!("FLEE {amt:+}"),
            ItemScriptBonus::PerfectDodge(amt) => format!("Perfect Dodge {amt:+}"),
            ItemScriptBonus::Critical(amt) => format!("CRIT {amt:+}"),
            ItemScriptBonus::AspdFlat(amt) => format!("ASPD {amt:+}"),
            ItemScriptBonus::AspdPercent(amt) => format!("ASPD {amt:+}%"),
            ItemScriptBonus::SpeedPercent(amt) => format!("Movement Speed {amt:+}%"),
            ItemScriptBonus::CastTimePercent(amt) => format!("Variable Cast Time {amt:+}%"),
            ItemScriptBonus::AfterCastDelayPercent(amt) => format!("After-Cast Delay {amt:+}%"),
            ItemScriptBonus::DoubleAttackChance(amt) => format!("Double Attack Chance +{amt}%"),
            ItemScriptBonus::CriticalDefPercent(amt) => format!("Critical Defense +{amt}%"),
            ItemScriptBonus::RaceDamageBonus { race, percent, is_magic } => {
                let kind = if *is_magic { "magic" } else { "physical" };
                format!("{percent:+}% {kind} damage vs {race}")
            }
            ItemScriptBonus::RaceToleranceBonus { race, percent } => {
                format!("{percent:+}% damage reduction from {race}")
            }
            ItemScriptBonus::SizeDamageBonus { size, percent, is_magic } => {
                let kind = if *is_magic { "magic" } else { "physical" };
                format!("{percent:+}% {kind} damage vs {size}")
            }
            ItemScriptBonus::SizeToleranceBonus { size, percent } => {
                format!("{percent:+}% damage reduction from {size}")
            }
            ItemScriptBonus::ElementDamageBonus {
                element,
                percent,
                is_magic,
            } => {
                let kind = if *is_magic { "magic" } else { "physical" };
                format!("{percent:+}% {kind} damage vs {element}")
            }
            ItemScriptBonus::ElementResistanceBonus { element, percent } => {
                format!("{percent:+}% resistance to {element}")
            }
            ItemScriptBonus::WeaponElement(ele) => format!("Weapon Element: {ele}"),
            ItemScriptBonus::ArmorElement(ele) => format!("Armor Element: {ele}"),
            ItemScriptBonus::IndestructibleWeapon => "Indestructible (Weapon cannot be broken)".to_owned(),
            ItemScriptBonus::IndestructibleArmor => "Indestructible (Armor cannot be broken)".to_owned(),
            ItemScriptBonus::UninterruptibleCast => "Spell casting cannot be interrupted".to_owned(),
            ItemScriptBonus::NoGemStone => "Skills do not consume Gemstones".to_owned(),
            ItemScriptBonus::Unsupported { raw, .. } => {
                format!("[Unsupported script effect: {raw} {}]", Provenance::Unsupported.label())
            }
        }
    }
}

/// Translate raw Aegis race tokens into plain-language display names.
pub fn translate_race_token(token: &str) -> String {
    match token.trim() {
        "RC_DemiHuman" | "RC_DemiPlayer" | "Demi-Human" | "DemiHuman" => "Demi-Human".to_owned(),
        "RC_Brute" | "Brute" => "Brute".to_owned(),
        "RC_Undead" | "Undead" => "Undead".to_owned(),
        "RC_Fish" | "Fish" => "Fish".to_owned(),
        "RC_Plant" | "Plant" => "Plant".to_owned(),
        "RC_Insect" | "Insect" => "Insect".to_owned(),
        "RC_Demon" | "Demon" => "Demon".to_owned(),
        "RC_Angel" | "Angel" => "Angel".to_owned(),
        "RC_Dragon" | "Dragon" => "Dragon".to_owned(),
        "RC_Formless" | "Formless" => "Formless".to_owned(),
        "RC_Boss" | "Boss" => "Boss".to_owned(),
        "RC_NonBoss" | "NonBoss" => "Non-Boss".to_owned(),
        other => other.strip_prefix("RC_").unwrap_or(other).to_owned(),
    }
}

/// Translate raw Aegis size tokens into plain-language display names.
pub fn translate_size_token(token: &str) -> String {
    match token.trim() {
        "Size_Small" | "Small" => "Small".to_owned(),
        "Size_Medium" | "Medium" => "Medium".to_owned(),
        "Size_Large" | "Large" => "Large".to_owned(),
        other => other.strip_prefix("Size_").unwrap_or(other).to_owned(),
    }
}

/// Translate raw Aegis element tokens into plain-language display names.
pub fn translate_element_token(token: &str) -> String {
    match token.trim() {
        "Ele_Neutral" | "Neutral" => "Neutral".to_owned(),
        "Ele_Water" | "Water" => "Water".to_owned(),
        "Ele_Earth" | "Earth" => "Earth".to_owned(),
        "Ele_Fire" | "Fire" => "Fire".to_owned(),
        "Ele_Wind" | "Wind" => "Wind".to_owned(),
        "Ele_Poison" | "Poison" => "Poison".to_owned(),
        "Ele_Holy" | "Holy" => "Holy".to_owned(),
        "Ele_Dark" | "Dark" | "Shadow" => "Shadow".to_owned(),
        "Ele_Ghost" | "Ghost" => "Ghost".to_owned(),
        "Ele_Undead" | "Undead" => "Undead".to_owned(),
        other => other.strip_prefix("Ele_").unwrap_or(other).to_owned(),
    }
}

/// Parse a single Hercules / Aegis script statement into an
/// [`ItemScriptBonus`].
pub fn parse_script_statement(raw_stmt: &str) -> Option<ItemScriptBonus> {
    let stmt = raw_stmt.trim().trim_matches([';', '<', '>', '"', '\'', ' ']).trim();

    if stmt.is_empty() {
        return None;
    }

    // Split on first whitespace or parenthesis
    let (cmd, args_str) = if let Some(idx) = stmt.find('(') {
        let cmd = stmt[..idx].trim();
        let rest = stmt[idx + 1..].trim_end_matches(')').trim();
        (cmd, rest)
    } else if let Some(idx) = stmt.find(char::is_whitespace) {
        let cmd = stmt[..idx].trim();
        let rest = stmt[idx + 1..].trim();
        (cmd, rest)
    } else {
        (stmt, "")
    };

    let args: Vec<&str> = args_str.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();

    match cmd {
        "bonus" => {
            if args.is_empty() {
                return Some(ItemScriptBonus::Unsupported {
                    raw: stmt.to_owned(),
                    reason: "missing bonus arguments".to_owned(),
                });
            }
            let bonus_name = args[0];
            let val = args.get(1).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);

            let bonus = match bonus_name {
                "bStr" => ItemScriptBonus::StatBonus {
                    stat: StatKind::Strength,
                    amount: val,
                },
                "bAgi" => ItemScriptBonus::StatBonus {
                    stat: StatKind::Agility,
                    amount: val,
                },
                "bVit" => ItemScriptBonus::StatBonus {
                    stat: StatKind::Vitality,
                    amount: val,
                },
                "bInt" => ItemScriptBonus::StatBonus {
                    stat: StatKind::Intelligence,
                    amount: val,
                },
                "bDex" => ItemScriptBonus::StatBonus {
                    stat: StatKind::Dexterity,
                    amount: val,
                },
                "bLuk" => ItemScriptBonus::StatBonus {
                    stat: StatKind::Luck,
                    amount: val,
                },
                "bAllStats" => ItemScriptBonus::AllStats(val),
                "bMaxHP" => ItemScriptBonus::MaxHpFlat(val),
                "bMaxHPrate" => ItemScriptBonus::MaxHpPercent(val),
                "bMaxSP" => ItemScriptBonus::MaxSpFlat(val),
                "bMaxSPrate" => ItemScriptBonus::MaxSpPercent(val),
                "bBaseAtk" | "bAtk" => ItemScriptBonus::AtkFlat(val),
                "bAtkRate" => ItemScriptBonus::AtkPercent(val),
                "bMatk" => ItemScriptBonus::MatkFlat(val),
                "bMatkRate" => ItemScriptBonus::MatkPercent(val),
                "bDef" => ItemScriptBonus::DefFlat(val),
                "bDefRate" => ItemScriptBonus::DefPercent(val),
                "bMdef" => ItemScriptBonus::MdefFlat(val),
                "bMdefRate" => ItemScriptBonus::MdefPercent(val),
                "bHit" => ItemScriptBonus::Hit(val),
                "bFlee" => ItemScriptBonus::Flee(val),
                "bFlee2" => ItemScriptBonus::PerfectDodge(val),
                "bCritical" => ItemScriptBonus::Critical(val),
                "bAspd" => ItemScriptBonus::AspdFlat(val),
                "bAspdRate" => ItemScriptBonus::AspdPercent(val),
                "bSpeedRate" => ItemScriptBonus::SpeedPercent(val),
                "bCastrate" | "bVariableCastrate" => ItemScriptBonus::CastTimePercent(val),
                "bDelayrate" => ItemScriptBonus::AfterCastDelayPercent(val),
                "bDoubleRate" => ItemScriptBonus::DoubleAttackChance(val),
                "bCriticalDef" => ItemScriptBonus::CriticalDefPercent(val),
                "bUnbreakableWeapon" => ItemScriptBonus::IndestructibleWeapon,
                "bUnbreakableArmor" => ItemScriptBonus::IndestructibleArmor,
                "bNoCastCancel" => ItemScriptBonus::UninterruptibleCast,
                "bNoGemStone" => ItemScriptBonus::NoGemStone,
                "bAtkEle" => {
                    let ele = args
                        .get(1)
                        .map(|e| translate_element_token(e))
                        .unwrap_or_else(|| "Neutral".to_owned());
                    ItemScriptBonus::WeaponElement(ele)
                }
                "bDefEle" => {
                    let ele = args
                        .get(1)
                        .map(|e| translate_element_token(e))
                        .unwrap_or_else(|| "Neutral".to_owned());
                    ItemScriptBonus::ArmorElement(ele)
                }
                _ => ItemScriptBonus::Unsupported {
                    raw: stmt.to_owned(),
                    reason: format!("unmodeled bonus type: {bonus_name}"),
                },
            };
            Some(bonus)
        }
        "bonus2" => {
            if args.len() < 3 {
                return Some(ItemScriptBonus::Unsupported {
                    raw: stmt.to_owned(),
                    reason: "insufficient bonus2 arguments".to_owned(),
                });
            }
            let bonus_name = args[0];
            let target_token = args[1];
            let val = args[2].parse::<i32>().unwrap_or(0);

            let bonus = match bonus_name {
                "bAddRace" => ItemScriptBonus::RaceDamageBonus {
                    race: translate_race_token(target_token),
                    percent: val,
                    is_magic: false,
                },
                "bMagicAddRace" => ItemScriptBonus::RaceDamageBonus {
                    race: translate_race_token(target_token),
                    percent: val,
                    is_magic: true,
                },
                "bAddRaceTolerance" | "bSubRace" | "bRaceTolerance" => ItemScriptBonus::RaceToleranceBonus {
                    race: translate_race_token(target_token),
                    percent: val,
                },
                "bAddSize" => ItemScriptBonus::SizeDamageBonus {
                    size: translate_size_token(target_token),
                    percent: val,
                    is_magic: false,
                },
                "bMagicAddSize" => ItemScriptBonus::SizeDamageBonus {
                    size: translate_size_token(target_token),
                    percent: val,
                    is_magic: true,
                },
                "bSubSize" => ItemScriptBonus::SizeToleranceBonus {
                    size: translate_size_token(target_token),
                    percent: val,
                },
                "bAddEle" => ItemScriptBonus::ElementDamageBonus {
                    element: translate_element_token(target_token),
                    percent: val,
                    is_magic: false,
                },
                "bMagicAddEle" => ItemScriptBonus::ElementDamageBonus {
                    element: translate_element_token(target_token),
                    percent: val,
                    is_magic: true,
                },
                "bSubEle" => ItemScriptBonus::ElementResistanceBonus {
                    element: translate_element_token(target_token),
                    percent: val,
                },
                _ => ItemScriptBonus::Unsupported {
                    raw: stmt.to_owned(),
                    reason: format!("unmodeled bonus2 type: {bonus_name}"),
                },
            };
            Some(bonus)
        }
        _ => Some(ItemScriptBonus::Unsupported {
            raw: stmt.to_owned(),
            reason: format!("unmodeled script command: {cmd}"),
        }),
    }
}

/// Parse an entire item script string into a list of [`ItemScriptBonus`].
pub fn parse_item_script(script: &str) -> Vec<ItemScriptBonus> {
    script.split(';').filter_map(parse_script_statement).collect()
}

/// Target monster combat facts for contextual equipment comparison (GDD §10.10,
/// §5.13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterTargetFacts {
    pub name: String,
    pub race: String,
    pub size: String,
    pub element: String,
    pub level: u16,
    pub def: Option<i32>,
    pub mdef: Option<i32>,
}

/// Contextual evaluation of equipment bonuses against a target monster.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterBonusEvaluation {
    /// Net physical damage modifier multiplier percentage (base 100 + race% +
    /// size% + ele%).
    pub physical_multiplier_pct: i32,
    /// Net magical damage modifier multiplier percentage (base 100 + race% +
    /// size% + ele%).
    pub magic_multiplier_pct: i32,
    /// Weapon element vs monster element effectiveness percentage from
    /// `attr_fix.conf` (e.g. 150 for Fire vs Earth 1).
    pub elemental_effectiveness_pct: u16,
    /// Active bonuses that specifically matched this target monster.
    pub active_matching_bonuses: Vec<String>,
}

impl MonsterBonusEvaluation {
    /// Evaluate a set of item bonuses against a target monster.
    pub fn evaluate(bonuses: &[ItemScriptBonus], target: &MonsterTargetFacts) -> Self {
        let mut physical_bonus_sum = 0;
        let mut magic_bonus_sum = 0;
        let mut active_matching_bonuses = Vec::new();
        let mut weapon_element = "Neutral".to_owned();

        let target_race = translate_race_token(&target.race);
        let target_size = translate_size_token(&target.size);
        let target_element_name = target.element.split_whitespace().next().unwrap_or("Neutral");
        let target_element_clean = translate_element_token(target_element_name);

        for bonus in bonuses {
            match bonus {
                ItemScriptBonus::RaceDamageBonus { race, percent, is_magic } => {
                    if race.eq_ignore_ascii_case(&target_race) {
                        if *is_magic {
                            magic_bonus_sum += percent;
                            active_matching_bonuses.push(format!("+{percent}% magic vs {race}"));
                        } else {
                            physical_bonus_sum += percent;
                            active_matching_bonuses.push(format!("+{percent}% physical vs {race}"));
                        }
                    }
                }
                ItemScriptBonus::SizeDamageBonus { size, percent, is_magic } => {
                    if size.eq_ignore_ascii_case(&target_size) {
                        if *is_magic {
                            magic_bonus_sum += percent;
                            active_matching_bonuses.push(format!("+{percent}% magic vs {size}"));
                        } else {
                            physical_bonus_sum += percent;
                            active_matching_bonuses.push(format!("+{percent}% physical vs {size}"));
                        }
                    }
                }
                ItemScriptBonus::ElementDamageBonus {
                    element,
                    percent,
                    is_magic,
                } => {
                    if element.eq_ignore_ascii_case(&target_element_clean) {
                        if *is_magic {
                            magic_bonus_sum += percent;
                            active_matching_bonuses.push(format!("+{percent}% magic vs {element}"));
                        } else {
                            physical_bonus_sum += percent;
                            active_matching_bonuses.push(format!("+{percent}% physical vs {element}"));
                        }
                    }
                }
                ItemScriptBonus::WeaponElement(ele) => {
                    weapon_element = ele.clone();
                }
                _ => {}
            }
        }

        let elemental_effectiveness_pct = elemental_effectiveness(&weapon_element, &target.element).unwrap_or(100);

        Self {
            physical_multiplier_pct: 100 + physical_bonus_sum,
            magic_multiplier_pct: 100 + magic_bonus_sum,
            elemental_effectiveness_pct,
            active_matching_bonuses,
        }
    }
}

/// Comparison result of a hovered item vs an equipped item against a target
/// monster (GDD §10.10).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemMonsterComparisonDelta {
    pub target_name: String,
    pub target_race: String,
    pub target_size: String,
    pub target_element: String,
    pub hovered_physical_pct: i32,
    pub equipped_physical_pct: i32,
    pub delta_physical_pct: i32,
    pub hovered_magic_pct: i32,
    pub equipped_magic_pct: i32,
    pub delta_magic_pct: i32,
    pub elemental_rate_pct: u16,
    pub matched_bonuses: Vec<String>,
}

/// Compare a hovered item's script bonuses against an equipped item's script
/// bonuses in the context of a target monster.
pub fn compare_items_against_monster(
    hovered_bonuses: &[ItemScriptBonus],
    equipped_bonuses: &[ItemScriptBonus],
    target: &MonsterTargetFacts,
) -> ItemMonsterComparisonDelta {
    let hovered_eval = MonsterBonusEvaluation::evaluate(hovered_bonuses, target);
    let equipped_eval = MonsterBonusEvaluation::evaluate(equipped_bonuses, target);

    let delta_physical_pct = hovered_eval.physical_multiplier_pct - equipped_eval.physical_multiplier_pct;

    ItemMonsterComparisonDelta {
        target_name: target.name.clone(),
        target_race: target.race.clone(),
        target_size: target.size.clone(),
        target_element: target.element.clone(),
        hovered_physical_pct: hovered_eval.physical_multiplier_pct,
        equipped_physical_pct: equipped_eval.physical_multiplier_pct,
        delta_physical_pct,
        hovered_magic_pct: hovered_eval.magic_multiplier_pct,
        equipped_magic_pct: equipped_eval.magic_multiplier_pct,
        delta_magic_pct: hovered_eval.magic_multiplier_pct - equipped_eval.magic_multiplier_pct,
        elemental_rate_pct: hovered_eval.elemental_effectiveness_pct,
        matched_bonuses: hovered_eval.active_matching_bonuses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_classic_card_scripts() {
        // Hydra Card
        let hydra = parse_item_script("bonus2 bAddRace,RC_DemiPlayer,20;");
        assert_eq!(hydra.len(), 1);
        assert_eq!(hydra[0].plain_text(), "+20% physical damage vs Demi-Human");

        // Thara Frog Card
        let thara = parse_item_script("bonus2 bAddRaceTolerance,RC_DemiPlayer,30;");
        assert_eq!(thara.len(), 1);
        assert_eq!(thara[0].plain_text(), "+30% damage reduction from Demi-Human");

        // Mantis Card
        let mantis = parse_item_script("bonus bStr,3;");
        assert_eq!(mantis.len(), 1);
        assert_eq!(mantis[0].plain_text(), "STR +3");

        // Pupa Card
        let pupa = parse_item_script("bonus bMaxHP,700;");
        assert_eq!(pupa.len(), 1);
        assert_eq!(pupa[0].plain_text(), "Max HP +700");

        // Whisper Card
        let whisper = parse_item_script("bonus bFlee,20; bonus2 bSubEle,Ele_Ghost,-50;");
        assert_eq!(whisper.len(), 2);
        assert_eq!(whisper[0].plain_text(), "FLEE +20");
        assert_eq!(whisper[1].plain_text(), "-50% resistance to Ghost");

        // Skeleton Worker Card
        let skel_worker = parse_item_script("bonus2 bAddSize,Size_Medium,15; bonus bBaseAtk,5;");
        assert_eq!(skel_worker.len(), 2);
        assert_eq!(skel_worker[0].plain_text(), "+15% physical damage vs Medium");
        assert_eq!(skel_worker[1].plain_text(), "ATK +5");

        // Balmung
        let balmung = parse_item_script("bonus bUnbreakableWeapon,0; bonus bAtkEle,Ele_Holy;");
        assert_eq!(balmung.len(), 2);
        assert_eq!(balmung[0].plain_text(), "Indestructible (Weapon cannot be broken)");
        assert_eq!(balmung[1].plain_text(), "Weapon Element: Holy");
    }

    #[test]
    fn rejects_and_flags_unsupported_script_effects() {
        let unsupported = parse_item_script("autospell \"MG_COLDBOLT\",3,10;");
        assert_eq!(unsupported.len(), 1);
        assert!(unsupported[0].is_unsupported());
        let text = unsupported[0].plain_text();
        assert!(text.contains("[Unsupported script effect:"));
        assert!(text.contains("(unmodeled)"));
    }

    #[test]
    fn evaluates_monster_comparison_against_orc_archer() {
        let orc_archer = MonsterTargetFacts {
            name: "Orc Archer".to_owned(),
            race: "Demi-Human".to_owned(),
            size: "Medium".to_owned(),
            element: "Earth 1".to_owned(),
            level: 44,
            def: Some(25),
            mdef: Some(10),
        };

        // Hovered weapon has Hydra (+20% Demi-Human) and Skeleton Worker (+15% Medium)
        let hovered = parse_item_script("bonus2 bAddRace,RC_DemiPlayer,20; bonus2 bAddSize,Size_Medium,15; bonus bAtkEle,Ele_Fire;");
        // Equipped weapon has Andre Card (+20 flat ATK)
        let equipped = parse_item_script("bonus bBaseAtk,20;");

        let comparison = compare_items_against_monster(&hovered, &equipped, &orc_archer);

        assert_eq!(comparison.target_name, "Orc Archer");
        assert_eq!(comparison.hovered_physical_pct, 135); // 100 + 20 + 15
        assert_eq!(comparison.equipped_physical_pct, 100);
        assert_eq!(comparison.delta_physical_pct, 35); // +35% damage advantage vs target
        assert_eq!(comparison.elemental_rate_pct, 150); // Fire vs Earth 1 = 150% from attr_fix.conf
        assert_eq!(comparison.matched_bonuses.len(), 2);
        // Physical cards leave the magic modifier at its base on both sides.
        assert_eq!((comparison.hovered_magic_pct, comparison.equipped_magic_pct), (100, 100));

        // A caster's bMagicAddRace is reported separately from physical bonuses.
        let rod = parse_item_script("bonus2 bMagicAddRace,RC_DemiPlayer,10;");
        let caster = compare_items_against_monster(&rod, &equipped, &orc_archer);
        assert_eq!(caster.hovered_magic_pct, 110);
        assert_eq!(caster.delta_magic_pct, 10);
        assert_eq!(caster.hovered_physical_pct, 100);
    }
}
