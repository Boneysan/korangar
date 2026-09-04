//! Choices offered by the character creation window.
//!
//! The ranges here are not guesses. They come from listing `data.grf` with
//! `tools/grf_list.py`:
//!
//!   * hair sprites live at `data\sprite\인간족\머리통\<sex>\<style>_<sex>.spr`
//!     and run 1..=42, contiguous, identically for both sexes. There is **no**
//!     style 0, which is what the client used to send.
//!   * hair palettes live at
//!     `data\palette\머리\머리<style>_<sex>_<palette>.pal`. Palettes 1..=8
//!     exist for every style and sex; palette **0 is missing** for seven
//!     combinations (male 35/37/39/40, female 36/40/41), so it is not a safe
//!     default.

use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ragnarok_packets::Sex;
use rust_state::RustState;

/// Hair styles present in the archive, as drop-down labels. A dedicated table
/// rather than a formatted number, because `DropDownItem::text` hands back a
/// borrow and the option list has to outlive the frame that draws it.
const HAIR_STYLE_LABELS: [&str; 42] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24",
    "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42",
];

/// A hair style id, 1-based to match the sprite files.
///
/// A named field rather than a tuple struct: the `StateElement` derive does not
/// handle tuple structs, and fails looking for a `_0` accessor. `RustState` has
/// to come with it -- the element derive reaches for the per-field path
/// accessor that one generates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, RustState, StateElement)]
pub struct HairStyle {
    pub id: u16,
}

impl HairStyle {
    /// Style 1 exists for both sexes and is the classic default.
    pub const DEFAULT: Self = Self { id: 1 };

    fn all() -> Vec<Self> {
        (1..=HAIR_STYLE_LABELS.len() as u16).map(|id| Self { id }).collect()
    }
}

impl DropDownItem<HairStyle> for HairStyle {
    fn text(&self) -> &str {
        // Clamped rather than indexed blind: the value round-trips through the
        // settings file and the wire, and an out-of-range id must not panic the
        // character screen.
        HAIR_STYLE_LABELS.get(self.id.saturating_sub(1) as usize).copied().unwrap_or("1")
    }

    fn value(&self) -> HairStyle {
        *self
    }
}

/// The character's sex, chosen per character.
///
/// Separate from [`Sex`] because that is the wire enum and carries `Both` and
/// `Server`, neither of which a player may pick -- Hercules denies creation for
/// anything that is not male or female (`char.c`,
/// `char_parse_char_create_new_char`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, StateElement)]
pub enum CharacterSex {
    Female,
    Male,
}

impl CharacterSex {
    fn all() -> Vec<Self> {
        vec![Self::Female, Self::Male]
    }
}

impl From<CharacterSex> for Sex {
    fn from(sex: CharacterSex) -> Self {
        match sex {
            CharacterSex::Female => Sex::Female,
            CharacterSex::Male => Sex::Male,
        }
    }
}

impl DropDownItem<CharacterSex> for CharacterSex {
    fn text(&self) -> &str {
        match self {
            CharacterSex::Female => "Female",
            CharacterSex::Male => "Male",
        }
    }

    fn value(&self) -> CharacterSex {
        *self
    }
}

/// Status points a freshly created character has to spend.
///
/// The literal `48` in `char_make_new_char_sql`'s INSERT (Hercules
/// `src/char/char.c`), alongside all six stats at 1.
pub const STARTING_STATUS_POINTS: u32 = 48;

/// The six jobs a Novice can change into.
///
/// First jobs only, on purpose: past that the build depends on gear and party,
/// and a level-1 character is nowhere near the decision.
#[derive(Copy, Clone, Debug, PartialEq, Eq, StateElement)]
pub enum StartingClass {
    Swordsman,
    Mage,
    Archer,
    Acolyte,
    Merchant,
    Thief,
}

impl StartingClass {
    fn all() -> Vec<Self> {
        vec![
            Self::Swordsman,
            Self::Mage,
            Self::Archer,
            Self::Acolyte,
            Self::Merchant,
            Self::Thief,
        ]
    }

    /// A level-1 spend of the 48 status points every character is created with
    /// (`char.c`, the literal `48` in `make_new_char_sql`'s INSERT).
    pub fn recommended_stats(&self) -> StatSpread {
        match self {
            Self::Swordsman => StatSpread::new(9, 9, 8, 1, 2, 1),
            Self::Mage => StatSpread::new(1, 5, 3, 11, 9, 1),
            Self::Archer => StatSpread::new(1, 11, 5, 1, 11, 1),
            Self::Acolyte => StatSpread::new(1, 3, 9, 9, 7, 1),
            Self::Merchant => StatSpread::new(9, 1, 9, 1, 9, 1),
            Self::Thief => StatSpread::new(7, 11, 1, 1, 9, 1),
        }
    }

    /// One line on why, so the numbers read as a suggestion rather than an
    /// oracle.
    pub fn recommendation_reason(&self) -> &'static str {
        match self {
            Self::Swordsman => "Hits hard and survives. VIT for HP, AGI to dodge.",
            Self::Mage => "INT is your damage and your SP pool. DEX makes casts land.",
            Self::Archer => "DEX is bow damage AND accuracy. AGI for attack speed.",
            Self::Acolyte => "Heal scales on INT. VIT keeps you standing while you cast.",
            Self::Merchant => "STR to carry and to hit. DEX for accuracy while you trade.",
            Self::Thief => "AGI to dodge and attack fast. DEX so your hits connect.",
        }
    }
}

/// Six target stat values for a level-1 character.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StatSpread {
    pub strength: u16,
    pub agility: u16,
    pub vitality: u16,
    pub intelligence: u16,
    pub dexterity: u16,
    pub luck: u16,
}

impl StatSpread {
    const fn new(strength: u16, agility: u16, vitality: u16, intelligence: u16, dexterity: u16, luck: u16) -> Self {
        Self {
            strength,
            agility,
            vitality,
            intelligence,
            dexterity,
            luck,
        }
    }

    /// What raising one stat from 1 to `target` costs.
    ///
    /// Hercules charges `2 + (n - 1) / 10` per step on renewal and
    /// `1 + (n + 9) / 10` otherwise (`pc_need_status_point`). Those are the
    /// same number for every step from 1 to 10, and this only has to agree with
    /// the server over the range 48 points can reach, so one formula covers
    /// both builds.
    fn cost_of(target: u16) -> u32 {
        (1..target).map(|value| 2 + u32::from(value - 1) / 10).sum()
    }

    /// Total status points this spread costs from a freshly created character.
    pub fn cost(&self) -> u32 {
        [
            self.strength,
            self.agility,
            self.vitality,
            self.intelligence,
            self.dexterity,
            self.luck,
        ]
        .into_iter()
        .map(Self::cost_of)
        .sum()
    }

    /// The stats worth raising, as one line.
    ///
    /// Stats left at 1 are omitted -- a row reading "LUK 1" is noise, since
    /// every stat starts there. Plain spaces separate them: the interface font
    /// has no glyph for decorative middots and draws nothing at all for them.
    pub fn summary(&self) -> String {
        [
            ("STR", self.strength),
            ("AGI", self.agility),
            ("VIT", self.vitality),
            ("INT", self.intelligence),
            ("DEX", self.dexterity),
            ("LUK", self.luck),
        ]
        .into_iter()
        .filter(|(_, value)| *value > 1)
        .map(|(name, value)| format!("{name} {value}"))
        .collect::<Vec<_>>()
        .join("   ")
    }
}

impl DropDownItem<StartingClass> for StartingClass {
    fn text(&self) -> &str {
        match self {
            Self::Swordsman => "Swordsman",
            Self::Mage => "Mage",
            Self::Archer => "Archer",
            Self::Acolyte => "Acolyte",
            Self::Merchant => "Merchant",
            Self::Thief => "Thief",
        }
    }

    fn value(&self) -> StartingClass {
        *self
    }
}

/// What the player has chosen so far, plus the lists to choose from.
///
/// Mirrors the shape the graphics settings use -- selected value beside the
/// option list -- so the drop-downs work the same way.
#[derive(RustState, StateElement)]
pub struct CharacterCreation {
    pub sex: CharacterSex,
    pub hair_style: HairStyle,
    /// Whether the "help me choose" panel is showing.
    pub show_recommendation: bool,
    pub starting_class: StartingClass,
    pub sexes: Vec<CharacterSex>,
    pub hair_styles: Vec<HairStyle>,
    pub starting_classes: Vec<StartingClass>,
}

impl Default for CharacterCreation {
    fn default() -> Self {
        Self {
            sex: CharacterSex::Male,
            hair_style: HairStyle::DEFAULT,
            show_recommendation: false,
            starting_class: StartingClass::Swordsman,
            sexes: CharacterSex::all(),
            hair_styles: HairStyle::all(),
            starting_classes: StartingClass::all(),
        }
    }
}

impl CharacterCreation {
    /// Back to the defaults, so the window does not open showing whatever the
    /// last character used.
    pub fn reset(&mut self) {
        self.sex = CharacterSex::Male;
        self.hair_style = HairStyle::DEFAULT;
        self.show_recommendation = false;
        self.starting_class = StartingClass::Swordsman;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recommendations are the whole point of the "help me choose" panel,
    /// and a build a level-1 character cannot afford is worse than no advice at
    /// all -- the player would follow it, run out, and be left with a
    /// half-built character and no way back.
    #[test]
    fn every_recommendation_fits_the_starting_budget() {
        for class in StartingClass::all() {
            let cost = class.recommended_stats().cost();

            assert!(
                cost <= STARTING_STATUS_POINTS,
                "{} recommends {cost} points, more than the {STARTING_STATUS_POINTS} a new character has",
                class.text()
            );
        }
    }

    /// Points left unspent are points wasted at level 1, so each build should
    /// use the whole budget rather than merely fit inside it.
    #[test]
    fn every_recommendation_spends_the_whole_budget() {
        for class in StartingClass::all() {
            assert_eq!(
                class.recommended_stats().cost(),
                STARTING_STATUS_POINTS,
                "{} leaves points unspent",
                class.text()
            );
        }
    }

    /// Mirrors `pc_need_status_point`: 2 per step from 1 to 10, 3 from 11 to
    /// 20. Raising one stat 1 -> 11 is ten steps at 2.
    #[test]
    fn cost_matches_the_server_formula() {
        assert_eq!(StatSpread::cost_of(1), 0);
        assert_eq!(StatSpread::cost_of(2), 2);
        assert_eq!(StatSpread::cost_of(10), 18);
        assert_eq!(StatSpread::cost_of(11), 20);
        // The first step that costs 3 rather than 2.
        assert_eq!(StatSpread::cost_of(12), 23);
    }

    /// Stats still at their starting value are dropped: a row reading "LUK 1"
    /// is noise on a character whose every stat starts at 1.
    #[test]
    fn summary_omits_untouched_stats() {
        let summary = StartingClass::Merchant.recommended_stats().summary();

        assert_eq!(summary, "STR 9   VIT 9   DEX 9");
        assert!(!summary.contains("LUK"));
    }
}
