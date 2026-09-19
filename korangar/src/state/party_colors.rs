//! Stable party minimap and roster colors from membership order and identity.

use ragnarok_packets::{AccountId, CharacterId};
use serde::{Deserialize, Serialize};

/// RGB color for party members on minimap and party window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Returns 6-character hex representation without leading '#' or '^'.
    #[allow(dead_code)]
    pub fn hex_string(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// Returns inline color span code (e.g. `^46A0FF`).
    pub fn to_inline_code(self) -> String {
        format!("^{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

/// 12 distinct, high-contrast, accessible default colors.
pub const DEFAULTS: [Rgb; 12] = [
    Rgb(70, 160, 255),  // Sky Blue
    Rgb(255, 170, 40),  // Vivid Orange
    Rgb(80, 220, 120),  // Emerald Green
    Rgb(220, 80, 200),  // Magenta
    Rgb(255, 90, 90),   // Coral Red
    Rgb(80, 220, 240),  // Cyan
    Rgb(250, 210, 50),  // Amber Gold
    Rgb(170, 110, 250), // Purple / Violet
    Rgb(170, 230, 60),  // Lime
    Rgb(255, 130, 180), // Rose Pink
    Rgb(40, 190, 170),  // Teal
    Rgb(180, 200, 255), // Ice Lavender
];

pub fn color_for_index(index: usize) -> Rgb {
    DEFAULTS[index % DEFAULTS.len()]
}

/// Validates contrast against dark backgrounds and verifies non-collision
/// with reserved inline control codes (`^000000` reset and `^000001`
/// highlight).
#[allow(dead_code)]
pub fn contrast_ok(color: Rgb) -> bool {
    if color.0 == 0 && color.1 == 0 && color.2 <= 1 {
        return false;
    }
    let lum = 0.299 * f32::from(color.0) + 0.587 * f32::from(color.1) + 0.114 * f32::from(color.2);
    lum >= 40.0
}

/// Adjusts dark or reserved colors to guarantee readability and valid inline
/// syntax.
#[allow(dead_code)]
pub fn ensure_contrast(color: Rgb) -> Rgb {
    if color.0 == 0 && color.1 == 0 && color.2 <= 1 {
        return DEFAULTS[0];
    }
    let lum = 0.299 * f32::from(color.0) + 0.587 * f32::from(color.1) + 0.114 * f32::from(color.2);
    if lum < 40.0 {
        let scale = if lum > 0.0 { 40.0 / lum } else { 1.0 };
        let r = (f32::from(color.0) * scale).clamp(50.0, 255.0) as u8;
        let g = (f32::from(color.1) * scale).clamp(50.0, 255.0) as u8;
        let b = (f32::from(color.2) * scale).clamp(50.0, 255.0) as u8;
        Rgb(r, g, b)
    } else {
        color
    }
}

/// Stable identifier for a party member: prefers CharacterId, falls back to
/// AccountId.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PartyMemberKey {
    Character(CharacterId),
    Account(AccountId),
}

impl PartyMemberKey {
    pub fn new(character_id: Option<CharacterId>, account_id: AccountId) -> Self {
        match character_id {
            Some(cid) => PartyMemberKey::Character(cid),
            None => PartyMemberKey::Account(account_id),
        }
    }
}

/// State tracking deterministic, stable party color assignments across roster
/// updates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartyColorState {
    assignments: Vec<(PartyMemberKey, Rgb)>,
}

impl PartyColorState {
    pub fn new() -> Self {
        Self { assignments: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.assignments.clear();
    }

    pub fn color_for_key(&self, key: &PartyMemberKey) -> Option<Rgb> {
        self.assignments.iter().find(|(k, _)| k == key).map(|(_, c)| *c)
    }

    pub fn assign_or_get(&mut self, key: PartyMemberKey) -> Rgb {
        if let Some(color) = self.color_for_key(&key) {
            return color;
        }
        let used: Vec<Rgb> = self.assignments.iter().map(|(_, c)| *c).collect();
        let color = Self::next_available_color(&used);
        self.assignments.push((key, color));
        color
    }

    pub fn sync_members(&mut self, keys: &[PartyMemberKey]) {
        let mut updated = Vec::new();
        let mut used = Vec::new();

        // 1. Retain assignments for members still present
        for &key in keys {
            if let Some(color) = self.color_for_key(&key)
                && !updated.iter().any(|(k, _)| *k == key)
            {
                updated.push((key, color));
                used.push(color);
            }
        }

        // 2. Assign distinct default for new members
        for &key in keys {
            if !updated.iter().any(|(k, _)| *k == key) {
                let color = Self::next_available_color(&used);
                used.push(color);
                updated.push((key, color));
            }
        }

        self.assignments = updated;
    }

    pub fn remove(&mut self, key: &PartyMemberKey) {
        self.assignments.retain(|(k, _)| k != key);
    }

    fn next_available_color(used: &[Rgb]) -> Rgb {
        for &default_color in &DEFAULTS {
            if !used.contains(&default_color) {
                return default_color;
            }
        }
        // Fallback for parties exceeding palette size
        color_for_index(used.len())
    }
}

/// Local-only color overrides. Never sent on the wire.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyColorOverrides {
    #[serde(default)]
    entries: Vec<PersistedOverride>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedOverride {
    character_id: Option<u32>,
    account_id: u32,
    r: u8,
    g: u8,
    b: u8,
}

impl PartyColorOverrides {
    pub fn resolve(&self, key: PartyMemberKey, fallback: Rgb) -> Rgb {
        self.get(key).unwrap_or(fallback)
    }

    pub fn get(&self, key: PartyMemberKey) -> Option<Rgb> {
        self.entries
            .iter()
            .find(|entry| entry.matches(key))
            .map(|entry| Rgb(entry.r, entry.g, entry.b))
    }

    pub fn set(&mut self, key: PartyMemberKey, color: Rgb) -> Rgb {
        let color = ensure_contrast(color);
        self.entries.retain(|entry| !entry.matches(key));
        self.entries.push(PersistedOverride::from_key(key, color));
        color
    }

    pub fn reset(&mut self, key: PartyMemberKey) {
        self.entries.retain(|entry| !entry.matches(key));
    }

    pub fn cycle(&mut self, key: PartyMemberKey, current: Rgb) -> Rgb {
        let next = DEFAULTS
            .iter()
            .position(|&color| color == current)
            .map(|index| DEFAULTS[(index + 1) % DEFAULTS.len()])
            .unwrap_or(DEFAULTS[0]);
        self.set(key, next)
    }
}

impl PersistedOverride {
    fn from_key(key: PartyMemberKey, color: Rgb) -> Self {
        match key {
            PartyMemberKey::Character(CharacterId(id)) => Self {
                character_id: Some(id),
                account_id: 0,
                r: color.0,
                g: color.1,
                b: color.2,
            },
            PartyMemberKey::Account(AccountId(id)) => Self {
                character_id: None,
                account_id: id,
                r: color.0,
                g: color.1,
                b: color.2,
            },
        }
    }

    fn matches(&self, key: PartyMemberKey) -> bool {
        match key {
            PartyMemberKey::Character(CharacterId(id)) => self.character_id == Some(id),
            PartyMemberKey::Account(AccountId(id)) => self.character_id.is_none() && self.account_id == id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_is_stable_across_reconnect() {
        let mut state1 = PartyColorState::new();
        let member_a = PartyMemberKey::Character(CharacterId(101));
        let member_b = PartyMemberKey::Character(CharacterId(102));

        state1.sync_members(&[member_a, member_b]);
        let color_a1 = state1.color_for_key(&member_a).unwrap();
        let color_b1 = state1.color_for_key(&member_b).unwrap();

        // Reconnect with same roster
        let mut state2 = PartyColorState::new();
        state2.sync_members(&[member_a, member_b]);
        let color_a2 = state2.color_for_key(&member_a).unwrap();
        let color_b2 = state2.color_for_key(&member_b).unwrap();

        assert_eq!(color_a1, color_a2);
        assert_eq!(color_b1, color_b2);
        assert_ne!(color_a1, color_b1);
    }

    #[test]
    fn two_clients_see_locally_consistent_labels() {
        let client_a = &mut PartyColorState::new();
        let client_b = &mut PartyColorState::new();

        let roster = [
            PartyMemberKey::Character(CharacterId(501)),
            PartyMemberKey::Character(CharacterId(502)),
            PartyMemberKey::Character(CharacterId(503)),
        ];

        client_a.sync_members(&roster);
        client_b.sync_members(&roster);

        for member in roster {
            let col_a = client_a.color_for_key(&member).unwrap();
            let col_b = client_b.color_for_key(&member).unwrap();
            assert_eq!(
                col_a, col_b,
                "Client A and Client B must assign identical color for member {member:?}"
            );
        }
    }

    #[test]
    fn reorder_preserves_assignments() {
        let mut state = PartyColorState::new();
        let member_a = PartyMemberKey::Character(CharacterId(1));
        let member_b = PartyMemberKey::Character(CharacterId(2));
        let member_c = PartyMemberKey::Character(CharacterId(3));

        state.sync_members(&[member_a, member_b, member_c]);
        let color_a = state.color_for_key(&member_a).unwrap();
        let color_b = state.color_for_key(&member_b).unwrap();
        let color_c = state.color_for_key(&member_c).unwrap();

        // Roster arrives reordered
        state.sync_members(&[member_c, member_a, member_b]);
        assert_eq!(state.color_for_key(&member_a).unwrap(), color_a);
        assert_eq!(state.color_for_key(&member_b).unwrap(), color_b);
        assert_eq!(state.color_for_key(&member_c).unwrap(), color_c);
    }

    #[test]
    fn leave_and_rejoin_preserves_remaining_members() {
        let mut state = PartyColorState::new();
        let member_a = PartyMemberKey::Character(CharacterId(1));
        let member_b = PartyMemberKey::Character(CharacterId(2));
        let member_c = PartyMemberKey::Character(CharacterId(3));

        state.sync_members(&[member_a, member_b, member_c]);
        let color_a = state.color_for_key(&member_a).unwrap();
        let color_b = state.color_for_key(&member_b).unwrap();
        let color_c = state.color_for_key(&member_c).unwrap();

        // Member B leaves
        state.sync_members(&[member_a, member_c]);
        assert_eq!(state.color_for_key(&member_a).unwrap(), color_a);
        assert_eq!(state.color_for_key(&member_c).unwrap(), color_c);
        assert_eq!(state.color_for_key(&member_b), None);

        // Member D joins: picks up the freed color_b
        let member_d = PartyMemberKey::Character(CharacterId(4));
        state.sync_members(&[member_a, member_c, member_d]);
        assert_eq!(state.color_for_key(&member_a).unwrap(), color_a);
        assert_eq!(state.color_for_key(&member_c).unwrap(), color_c);
        assert_eq!(state.color_for_key(&member_d).unwrap(), color_b);
    }

    #[test]
    fn fallback_to_account_id_when_character_id_unavailable() {
        let mut state = PartyColorState::new();
        let key_without_char = PartyMemberKey::new(None, AccountId(2000001));
        let key_with_char = PartyMemberKey::new(Some(CharacterId(99)), AccountId(2000002));

        let color1 = state.assign_or_get(key_without_char);
        let color2 = state.assign_or_get(key_with_char);

        assert_ne!(color1, color2);
        assert_eq!(state.color_for_key(&key_without_char).unwrap(), color1);
        assert_eq!(state.color_for_key(&key_with_char).unwrap(), color2);
    }

    #[test]
    fn collision_beyond_twelve_members_cycles_deterministically() {
        let mut state = PartyColorState::new();
        let mut keys = Vec::new();
        for id in 1..=15 {
            keys.push(PartyMemberKey::Character(CharacterId(id)));
        }

        state.sync_members(&keys);
        for key in &keys {
            assert!(state.color_for_key(key).is_some());
        }

        // Member 13 cycles to DEFAULTS[0]
        assert_eq!(state.color_for_key(&keys[0]).unwrap(), DEFAULTS[0]);
        assert_eq!(state.color_for_key(&keys[12]).unwrap(), DEFAULTS[0]);
    }

    #[test]
    fn all_default_colors_pass_contrast() {
        for &color in &DEFAULTS {
            assert!(contrast_ok(color), "Default color {color:?} must pass contrast check");
        }
    }

    #[test]
    fn ensure_contrast_corrects_dark_and_reserved_colors() {
        assert!(contrast_ok(Rgb(255, 255, 255)));
        assert!(!contrast_ok(Rgb(0, 0, 0)));
        assert!(!contrast_ok(Rgb(0, 0, 1))); // reserved code
        assert!(!contrast_ok(Rgb(10, 10, 10))); // too dark

        let fixed_black = ensure_contrast(Rgb(0, 0, 0));
        assert!(contrast_ok(fixed_black));

        let fixed_reserved = ensure_contrast(Rgb(0, 0, 1));
        assert!(contrast_ok(fixed_reserved));

        let fixed_dark = ensure_contrast(Rgb(10, 10, 10));
        assert!(contrast_ok(fixed_dark));
    }

    #[test]
    fn local_overrides_round_trip_reset_and_correct_contrast() {
        let key = PartyMemberKey::Character(CharacterId(42));
        let mut overrides = PartyColorOverrides::default();
        let fallback = DEFAULTS[0];

        let stored = overrides.set(key, Rgb(10, 10, 10));
        assert!(contrast_ok(stored));
        assert_eq!(overrides.resolve(key, fallback), stored);
        assert_ne!(stored, fallback);

        let encoded = ron::ser::to_string(&overrides).unwrap();
        let decoded: PartyColorOverrides = ron::from_str(&encoded).unwrap();
        assert_eq!(decoded.resolve(key, fallback), stored);

        overrides.reset(key);
        assert_eq!(overrides.resolve(key, fallback), fallback);
    }
}
