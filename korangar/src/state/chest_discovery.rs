//! Personal chest discovery. Opened state is server-authoritative, not
//! click-local.
//!
//! Provides the three distinguishable visual states (`Unopened`, `Available`,
//! and `Opened`) driven by server-authoritative achievement packets, with
//! explicit text/icon fallbacks.

#![allow(dead_code)]

use hashbrown::HashSet;
use korangar_interface::element::StateElement;
use rust_state::RustState;

use crate::Color;
use crate::world::{ChestTable, StatusTint};

/// The three distinguishable visual states a world chest can inhabit.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ChestVisualState {
    /// In the world but not yet approached/discovered by the player.
    Unopened,
    /// Discovered and ready to be opened.
    Available,
    /// Authoritatively opened by this character.
    Opened,
}

impl ChestVisualState {
    /// Distinct status tint applied to the composed sprite.
    pub fn tint(self) -> StatusTint {
        match self {
            Self::Opened => StatusTint::drained(Color::rgb(0.65, 0.65, 0.65), 0.75),
            Self::Available => StatusTint::NONE,
            Self::Unopened => StatusTint::drained(Color::rgb(0.85, 0.85, 0.85), 0.3),
        }
    }

    /// High-contrast text label for tooltips and overhead status.
    pub fn hover_tag(self) -> &'static str {
        match self {
            Self::Opened => "[Opened]",
            Self::Available => "[Available]",
            Self::Unopened => "[Unopened]",
        }
    }

    /// Overhead text / badge display color.
    pub fn display_color(self) -> Color {
        match self {
            Self::Opened => Color::rgb(0.7, 0.7, 0.7),
            Self::Available => Color::rgb(1.0, 0.85, 0.2),
            Self::Unopened => Color::rgb(0.5, 0.5, 0.5),
        }
    }
}

/// Production state for hidden chest discovery.
#[derive(Clone, Debug, PartialEq, Eq, RustState, StateElement)]
pub struct ChestDiscoveryState {
    schema_version: u32,
    #[hidden_element]
    discovered: HashSet<u32>,
    #[hidden_element]
    opened: HashSet<u32>,
}

impl Default for ChestDiscoveryState {
    fn default() -> Self {
        Self::new(crate::world::CHEST_SCHEMA_VERSION)
    }
}

impl ChestDiscoveryState {
    pub fn new(schema_version: u32) -> Self {
        Self {
            schema_version,
            discovered: HashSet::new(),
            opened: HashSet::new(),
        }
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Derives the visual state of a chest from personal discovery and server
    /// records.
    pub fn visual_state(&self, chest_id: u32) -> ChestVisualState {
        if self.opened.contains(&chest_id) {
            ChestVisualState::Opened
        } else if self.discovered.contains(&chest_id) {
            ChestVisualState::Available
        } else {
            ChestVisualState::Unopened
        }
    }

    /// Legacy string-based visual accessor for compatibility.
    pub fn visual(&self, chest_id: u32) -> &'static str {
        match self.visual_state(chest_id) {
            ChestVisualState::Opened => "opened",
            ChestVisualState::Available => "available",
            ChestVisualState::Unopened => "unopened",
        }
    }

    pub fn is_opened(&self, chest_id: u32) -> bool {
        self.opened.contains(&chest_id)
    }

    pub fn is_discovered(&self, chest_id: u32) -> bool {
        self.discovered.contains(&chest_id)
    }

    pub fn discover(&mut self, chest_id: u32) {
        self.discovered.insert(chest_id);
    }

    /// Authoritatively marks a chest as opened based on a server packet/event.
    pub fn mark_opened_from_server(&mut self, chest_id: u32) {
        self.discovered.insert(chest_id);
        self.opened.insert(chest_id);
    }

    /// Clicking an entity records player awareness but NEVER marks it opened.
    pub fn click_does_not_open(&mut self, chest_id: u32) {
        self.discover(chest_id);
    }

    /// Ingests the authoritative completed achievement list on character
    /// connection or map login.
    pub fn handle_achievement_list(&mut self, completed_ids: &[u32]) {
        for &id in completed_ids {
            self.discovered.insert(id);
            self.opened.insert(id);
        }
    }

    /// Ingests a single achievement completion update from the map server.
    pub fn handle_achievement_update(&mut self, achievement_id: u32, is_completed: bool) {
        if is_completed {
            self.discovered.insert(achievement_id);
            self.opened.insert(achievement_id);
        }
    }

    /// Resets per-character state on character selection or disconnect.
    pub fn reset(&mut self) {
        self.discovered.clear();
        self.opened.clear();
    }

    pub fn discovered_count(&self) -> usize {
        self.discovered.len()
    }

    pub fn opened_count(&self) -> usize {
        self.opened.len()
    }

    /// Number of opened chests in a specific Act I region vs total chests in
    /// that region.
    pub fn region_progress(&self, region: u8, manifest: &ChestTable) -> (usize, usize) {
        let mut opened = 0;
        let mut total = 0;
        for chest in manifest.region_chests(region) {
            total += 1;
            if self.opened.contains(&chest.id) {
                opened += 1;
            }
        }
        (opened, total)
    }

    /// Total number of opened Act I campaign chests.
    pub fn opened_act1_count(&self, manifest: &ChestTable) -> usize {
        manifest.act1_chests().filter(|c| self.opened.contains(&c.id)).count()
    }

    /// Whether all chests in an Act I region have been opened.
    pub fn is_region_complete(&self, region: u8, manifest: &ChestTable) -> bool {
        let (opened, total) = self.region_progress(region, manifest);
        total > 0 && opened >= total
    }

    /// Authorship name for an Act I region (0..5).
    pub fn region_name(region: u8) -> &'static str {
        match region {
            0 => "Prontera",
            1 => "Geffen",
            2 => "Morroc",
            3 => "Payon",
            4 => "Alberta and Izlude",
            _ => "Unknown Region",
        }
    }

    /// Regional cosmetic reward (hat item id, hat item name) for completing an
    /// Act I region.
    pub fn region_hat(region: u8) -> (u32, &'static str) {
        match region {
            0 => (5108, "Renown Detective's Cap"),
            1 => (5027, "Mage Hat"),
            2 => (2222, "Turban"),
            3 => (5170, "Feather Beret"),
            4 => (18645, "Sailor Hat"),
            _ => (0, "Unknown Hat"),
        }
    }

    /// Formats an explanation of exploration, Field Notes, Marks, and regional
    /// cosmetics.
    pub fn format_exploration_summary(&self, manifest: &ChestTable) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push("Roadside caches are one-time exploration finds per character. Once looted, they remain permanently open.".to_string());
        lines.push(
            "Act I holds 38 chests across 5 regions. Each awards a unique Field Note and a Cartographer's Mark for the party.".to_string(),
        );
        lines.push(format!(
            "Personal discoveries: {} opened ({} Act I)",
            self.opened.len(),
            self.opened_act1_count(manifest)
        ));
        for region in 0..5 {
            let (opened, total) = self.region_progress(region, manifest);
            let (_hat_id, hat_name) = Self::region_hat(region);
            let status = if total > 0 && opened >= total {
                format!("{opened}/{total} · Complete! ({hat_name} awarded)")
            } else {
                format!("{opened}/{total} found · Reward: {hat_name}")
            };
            lines.push(format!("  {} — {}", Self::region_name(region), status));
        }
        lines
    }
}

/// Backward compatibility alias.
pub type ChestBook = ChestDiscoveryState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_discovery_transitions_to_available() {
        let mut state = ChestDiscoveryState::default();
        assert_eq!(state.visual_state(120001), ChestVisualState::Unopened);

        state.discover(120001);
        assert_eq!(state.visual_state(120001), ChestVisualState::Available);
    }

    #[test]
    fn click_does_not_open_authoritatively() {
        let mut state = ChestDiscoveryState::default();
        state.discover(120001);
        assert_eq!(state.visual_state(120001), ChestVisualState::Available);

        state.click_does_not_open(120001);
        assert_eq!(state.visual_state(120001), ChestVisualState::Available);
    }

    #[test]
    fn server_update_marks_opened() {
        let mut state = ChestDiscoveryState::default();
        state.discover(120001);

        state.handle_achievement_update(120001, true);
        assert_eq!(state.visual_state(120001), ChestVisualState::Opened);
    }

    #[test]
    fn achievement_list_populates_opened_on_login() {
        let mut state = ChestDiscoveryState::default();
        state.handle_achievement_list(&[120001, 120005, 120010]);

        assert_eq!(state.visual_state(120001), ChestVisualState::Opened);
        assert_eq!(state.visual_state(120005), ChestVisualState::Opened);
        assert_eq!(state.visual_state(120010), ChestVisualState::Opened);
        assert_eq!(state.visual_state(120002), ChestVisualState::Unopened);
    }

    #[test]
    fn character_reset_clears_per_character_state() {
        let mut state = ChestDiscoveryState::default();
        state.handle_achievement_list(&[120001, 120002]);
        assert_eq!(state.opened_count(), 2);

        state.reset();
        assert_eq!(state.opened_count(), 0);
        assert_eq!(state.visual_state(120001), ChestVisualState::Unopened);
    }

    #[test]
    fn party_member_interaction_does_not_mark_opened_for_local_player() {
        let mut alice_state = ChestDiscoveryState::default();
        let mut bob_state = ChestDiscoveryState::default();

        alice_state.discover(120001);
        bob_state.discover(120001);

        // Alice interacts and receives server achievement update:
        alice_state.handle_achievement_update(120001, true);
        assert_eq!(alice_state.visual_state(120001), ChestVisualState::Opened);

        // Bob did not open it; his chest remains Available:
        assert_eq!(bob_state.visual_state(120001), ChestVisualState::Available);
    }

    #[test]
    fn duplicate_deltas_are_idempotent() {
        let mut state = ChestDiscoveryState::default();
        state.handle_achievement_update(120001, true);
        state.handle_achievement_update(120001, true);
        assert_eq!(state.visual_state(120001), ChestVisualState::Opened);
        assert_eq!(state.opened_count(), 1);
    }

    #[test]
    fn visual_state_tint_and_tags() {
        assert_eq!(ChestVisualState::Opened.hover_tag(), "[Opened]");
        assert_eq!(ChestVisualState::Available.hover_tag(), "[Available]");
        assert_eq!(ChestVisualState::Unopened.hover_tag(), "[Unopened]");

        assert_ne!(ChestVisualState::Opened.tint(), ChestVisualState::Available.tint());
    }

    #[test]
    fn region_progress_tracks_accurately() {
        let table = ChestTable::load();
        let mut state = ChestDiscoveryState::default();

        let (opened, total) = state.region_progress(0, &table);
        assert_eq!(opened, 0);
        assert_eq!(total, 12);

        state.handle_achievement_update(120001, true);
        state.handle_achievement_update(120002, true);

        let (opened, total) = state.region_progress(0, &table);
        assert_eq!(opened, 2);
        assert_eq!(total, 12);
        assert_eq!(state.opened_act1_count(&table), 2);
        assert!(!state.is_region_complete(0, &table));
    }

    #[test]
    fn regional_cosmetic_threshold_triggers_completion() {
        let table = ChestTable::load();
        let mut state = ChestDiscoveryState::default();

        // Region 4 (Alberta & Izlude) has 2 chests: 120131 and 120144
        let (opened, total) = state.region_progress(4, &table);
        assert_eq!(total, 2);
        assert_eq!(opened, 0);
        assert!(!state.is_region_complete(4, &table));

        state.handle_achievement_update(120131, true);
        assert!(!state.is_region_complete(4, &table));

        state.handle_achievement_update(120144, true);
        assert!(state.is_region_complete(4, &table));

        let (hat_id, hat_name) = ChestDiscoveryState::region_hat(4);
        assert_eq!(hat_id, 18645);
        assert_eq!(hat_name, "Sailor Hat");
    }

    #[test]
    fn region_names_and_hats_match_hercules() {
        assert_eq!(ChestDiscoveryState::region_name(0), "Prontera");
        assert_eq!(ChestDiscoveryState::region_name(1), "Geffen");
        assert_eq!(ChestDiscoveryState::region_name(2), "Morroc");
        assert_eq!(ChestDiscoveryState::region_name(3), "Payon");
        assert_eq!(ChestDiscoveryState::region_name(4), "Alberta and Izlude");

        assert_eq!(ChestDiscoveryState::region_hat(0), (5108, "Renown Detective's Cap"));
        assert_eq!(ChestDiscoveryState::region_hat(1), (5027, "Mage Hat"));
        assert_eq!(ChestDiscoveryState::region_hat(2), (2222, "Turban"));
        assert_eq!(ChestDiscoveryState::region_hat(3), (5170, "Feather Beret"));
        assert_eq!(ChestDiscoveryState::region_hat(4), (18645, "Sailor Hat"));
    }

    #[test]
    fn exploration_summary_explains_notes_marks_and_persistence() {
        let table = ChestTable::load();
        let mut state = ChestDiscoveryState::default();
        state.handle_achievement_update(120001, true);

        let summary = state.format_exploration_summary(&table);
        assert!(summary.iter().any(|l| l.contains("one-time exploration finds")));
        assert!(summary.iter().any(|l| l.contains("permanently")));
        assert!(summary.iter().any(|l| l.contains("Field Note")));
        assert!(summary.iter().any(|l| l.contains("Cartographer's Mark")));
        assert!(summary.iter().any(|l| l.contains("Prontera")));
        assert!(summary.iter().any(|l| l.contains("Renown Detective's Cap")));
    }
}
