//! Which element an attack uses, and how that compares with a monster's
//! element.
//!
//! Skill elements come from `docs/skills.json` (`skill_db.conf`). Monster
//! elements come from the bundled bestiary (`"Fire 1"`). The rates are
//! Hercules `db/re/attr_fix.conf`: 100 is unchanged, above 100 is advantage,
//! below 100 is a resist, and 0 is immune.
//!
//! A basic attack, and a skill marked `Ele_Weapon`, uses Neutral. That is the
//! weapon element before an endow; the client does not yet know card or status
//! weapon elements.

use crate::world::library::skill_element_name;

const ATTR_FIX: &str = include_str!("../../../../Hercules/db/re/attr_fix.conf");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Element {
    Neutral = 0,
    Water = 1,
    Earth = 2,
    Fire = 3,
    Wind = 4,
    Poison = 5,
    Holy = 6,
    Dark = 7,
    Ghost = 8,
    Undead = 9,
}

const ELEMENT_COUNT: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementCue {
    Advantage,
    Neutral,
    Resist,
    Immune,
}

fn parse_element_token(token: &str) -> Option<Element> {
    match token.trim() {
        "Neutral" | "Ele_Neutral" => Some(Element::Neutral),
        "Water" | "Ele_Water" => Some(Element::Water),
        "Earth" | "Ele_Earth" => Some(Element::Earth),
        "Fire" | "Ele_Fire" => Some(Element::Fire),
        "Wind" | "Ele_Wind" => Some(Element::Wind),
        "Poison" | "Ele_Poison" => Some(Element::Poison),
        "Holy" | "Ele_Holy" => Some(Element::Holy),
        "Dark" | "Ele_Dark" | "Shadow" => Some(Element::Dark),
        "Ghost" | "Ele_Ghost" => Some(Element::Ghost),
        "Undead" | "Ele_Undead" => Some(Element::Undead),
        _ => None,
    }
}

fn parse_monster_element(text: &str) -> Option<(Element, u8)> {
    let mut parts = text.split_whitespace();
    let element = parse_element_token(parts.next()?)?;
    let level = parts.next()?.parse::<u8>().ok()?.clamp(1, 4);
    Some((element, level))
}

fn attr_table() -> &'static [[[u16; ELEMENT_COUNT]; 4]; ELEMENT_COUNT] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[[[u16; ELEMENT_COUNT]; 4]; ELEMENT_COUNT]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [[[100u16; ELEMENT_COUNT]; 4]; ELEMENT_COUNT];
        let mut defense: Option<Element> = None;
        let mut level: Option<usize> = None;
        for raw in ATTR_FIX.lines() {
            let line = raw.split("//").next().unwrap_or("").trim().trim_end_matches('{').trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('/') {
                continue;
            }
            if let Some(name) = line.strip_suffix(':')
                && let Some(number) = name.trim().strip_prefix("Lv")
                && let Ok(parsed) = number.parse::<usize>()
                && (1..=4).contains(&parsed)
            {
                level = Some(parsed - 1);
                continue;
            }
            if let Some(name) = line.strip_suffix(':')
                && let Some(element) = parse_element_token(name.trim())
            {
                defense = Some(element);
                level = None;
                continue;
            }
            if let Some((attack_name, rate)) = line.split_once(':')
                && let (Some(defense), Some(level)) = (defense, level)
                && let Some(attack) = parse_element_token(attack_name.trim())
                && let Ok(rate) = rate.trim().trim_end_matches(',').parse::<u16>()
            {
                table[defense as usize][level][attack as usize] = rate;
            }
        }
        table
    })
}

fn rate_for(defense: Element, defense_level: u8, attack: Element) -> u16 {
    let level = (defense_level.clamp(1, 4) - 1) as usize;
    attr_table()[defense as usize][level][attack as usize]
}

fn cue_for(rate: u16) -> ElementCue {
    match rate {
        0 => ElementCue::Immune,
        1..=99 => ElementCue::Resist,
        100 => ElementCue::Neutral,
        _ => ElementCue::Advantage,
    }
}

fn attack_element(skill_id: Option<u16>, skill_level: u16) -> Element {
    let Some(skill_id) = skill_id else {
        return Element::Neutral;
    };
    match skill_element_name(skill_id, skill_level).and_then(parse_element_token) {
        Some(element) => element,
        // Ele_Weapon, or a name this table does not list, follows the weapon.
        // Unendowed weapons are Neutral.
        None => Element::Neutral,
    }
}

/// Compare a player hit with a bestiary element such as `"Fire 1"`.
/// `skill_level` is the cast level when the skill's element changes by level.
pub fn identify_player_hit(skill_id: Option<u16>, skill_level: u16, monster_element: &str) -> Option<ElementCue> {
    let (defense, defense_level) = parse_monster_element(monster_element)?;
    let attack = attack_element(skill_id, skill_level);
    let rate = rate_for(defense, defense_level, attack);
    Some(cue_for(rate))
}

/// Query the exact elemental effectiveness rate (in %) for an attack element
/// against a monster element. Sourced from Hercules `db/re/attr_fix.conf` (100
/// = neutral, 150 = 150% damage, 0 = immune).
pub fn elemental_effectiveness(attack_element_name: &str, monster_element: &str) -> Option<u16> {
    let (defense, defense_level) = parse_monster_element(monster_element)?;
    let attack = parse_element_token(attack_element_name)?;
    Some(rate_for(defense, defense_level, attack))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_level_one_takes_extra_damage_from_water() {
        assert_eq!(rate_for(Element::Fire, 1, Element::Water), 150);
        assert_eq!(cue_for(150), ElementCue::Advantage);
    }

    #[test]
    fn water_level_two_is_immune_to_water() {
        assert_eq!(rate_for(Element::Water, 2, Element::Water), 0);
        assert_eq!(cue_for(0), ElementCue::Immune);
    }

    #[test]
    fn neutral_level_one_resists_ghost() {
        assert_eq!(rate_for(Element::Neutral, 1, Element::Ghost), 70);
        assert_eq!(cue_for(70), ElementCue::Resist);
    }

    #[test]
    fn magnum_break_is_fire_against_a_fire_monster() {
        // SM_MAGNUM is skill 7, Ele_Fire. Fire versus Fire level 1 is 25%.
        assert_eq!(skill_element_name(7, 1), Some("Ele_Fire"));
        assert_eq!(identify_player_hit(Some(7), 1, "Fire 1"), Some(ElementCue::Resist));
    }

    #[test]
    fn a_basic_attack_uses_neutral() {
        // An unendowed basic attack is Neutral. Ghost level 1 takes 70% from Neutral.
        assert_eq!(identify_player_hit(None, 0, "Ghost 1"), Some(ElementCue::Resist));
        assert_eq!(identify_player_hit(None, 0, "not an element"), None);
    }

    #[test]
    fn elemental_effectiveness_queries_exact_rates() {
        assert_eq!(elemental_effectiveness("Fire", "Earth 1"), Some(150));
        assert_eq!(elemental_effectiveness("Water", "Fire 2"), Some(175)); // In Renewal, Water vs Fire 2 is 175%
        assert_eq!(elemental_effectiveness("Holy", "Undead 1"), Some(150));
        assert_eq!(elemental_effectiveness("Ghost", "Neutral 1"), Some(70));
        assert_eq!(elemental_effectiveness("Water", "Water 2"), Some(0));
    }
}
