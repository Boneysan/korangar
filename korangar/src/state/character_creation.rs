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

/// What the player has chosen so far, plus the lists to choose from.
///
/// Mirrors the shape the graphics settings use -- selected value beside the
/// option list -- so the drop-downs work the same way.
#[derive(RustState, StateElement)]
pub struct CharacterCreation {
    pub sex: CharacterSex,
    pub hair_style: HairStyle,
    pub sexes: Vec<CharacterSex>,
    pub hair_styles: Vec<HairStyle>,
}

impl Default for CharacterCreation {
    fn default() -> Self {
        Self {
            sex: CharacterSex::Male,
            hair_style: HairStyle::DEFAULT,
            sexes: CharacterSex::all(),
            hair_styles: HairStyle::all(),
        }
    }
}

impl CharacterCreation {
    /// Back to the defaults, so the window does not open showing whatever the
    /// last character used.
    pub fn reset(&mut self) {
        self.sex = CharacterSex::Male;
        self.hair_style = HairStyle::DEFAULT;
    }
}
