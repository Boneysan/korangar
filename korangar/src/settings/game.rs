use std::collections::{HashMap, HashSet};

#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use super::key_bindings::{BindableAction, KeyBindings};

const MAX_CLIENT_HUNTING_GOALS: usize = 5;

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum GroundSkillTargetMode {
    AimAndClick,
    QuickcastAtCursor,
    HoldToAimRelease,
}

impl GroundSkillTargetMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::AimAndClick => "Aim + click",
            Self::QuickcastAtCursor => "Quickcast",
            Self::HoldToAimRelease => "Hold + release",
        }
    }
}

/// Item types `@autoloottype` recognizes (`src/map/atcommand.c`,
/// `ACMD(autoloottype)`). `command_name` must match its `strncmp(message,
/// "...", N)` prefixes exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, RustState, StateElement)]
pub enum AutolootItemType {
    Healing,
    Usable,
    Etc,
    Weapon,
    Armor,
    Card,
    PetEgg,
    PetArmor,
    Ammo,
}

impl AutolootItemType {
    pub const ALL: [Self; 9] = [
        Self::Healing,
        Self::Usable,
        Self::Etc,
        Self::Weapon,
        Self::Armor,
        Self::Card,
        Self::PetEgg,
        Self::PetArmor,
        Self::Ammo,
    ];

    pub fn command_name(self) -> &'static str {
        match self {
            Self::Healing => "healing",
            Self::Usable => "usable",
            Self::Etc => "etc",
            Self::Weapon => "weapon",
            Self::Armor => "armor",
            Self::Card => "card",
            Self::PetEgg => "petegg",
            Self::PetArmor => "petarmor",
            Self::Ammo => "ammo",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Healing => "Healing items",
            Self::Usable => "Usable items",
            Self::Etc => "Etc items",
            Self::Weapon => "Weapons",
            Self::Armor => "Armor",
            Self::Card => "Cards",
            Self::PetEgg => "Pet eggs",
            Self::PetArmor => "Pet armor",
            Self::Ammo => "Ammo",
        }
    }
}

/// GDD §11.3: Visual ground-loot filter mode.
/// Note: Cards and wishlisted items are ALWAYS visible regardless of filter
/// settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum GroundLootFilter {
    /// Show all dropped items on the ground.
    #[default]
    All,
    /// Show equipment, weapons, armor, cards, and wishlisted items.
    EquipmentAndCards,
    /// Show only cards and wishlisted items.
    CardsOnly,
}

impl GroundLootFilter {
    pub const ALL: [Self; 3] = [Self::All, Self::EquipmentAndCards, Self::CardsOnly];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All items visible",
            Self::EquipmentAndCards => "Equipment & Cards only",
            Self::CardsOnly => "Cards only",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::All => Self::EquipmentAndCards,
            Self::EquipmentAndCards => Self::CardsOnly,
            Self::CardsOnly => Self::All,
        }
    }
}

/// Who produced a skill visual, for the effect-density policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectSource {
    /// The local player's own skills.
    Local,
    /// A party member.
    Party,
    /// A monster. Hostile visuals are warnings and are never thinned.
    Hostile,
    /// Any other player.
    Bystander,
}

/// How many cosmetic skill visuals from *other players* to draw (GDD 15, 16).
/// Telegraph footprints, cast bars, markers, and combat text are not skill
/// visuals in this sense and ignore this setting entirely.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum EffectDensity {
    /// Draw everything.
    #[default]
    Full,
    /// Hide bystander players' skill visuals; keep yours, party, and hostile.
    Reduced,
    /// Keep only your own and hostile skill visuals.
    Minimal,
}

impl EffectDensity {
    pub fn next(self) -> Self {
        match self {
            Self::Full => Self::Reduced,
            Self::Reduced => Self::Minimal,
            Self::Minimal => Self::Full,
        }
    }

    pub fn shows_skill_visual(self, source: EffectSource) -> bool {
        match (self, source) {
            (_, EffectSource::Local | EffectSource::Hostile) => true,
            (Self::Full, _) => true,
            (Self::Reduced, EffectSource::Party) => true,
            (Self::Reduced, EffectSource::Bystander) => false,
            (Self::Minimal, EffectSource::Party | EffectSource::Bystander) => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum CombatTextFrequency {
    /// Show every damage, miss, and healing number.
    #[default]
    All,
    /// Keep critical hits and misses, while hiding routine damage numbers.
    Important,
    /// Hide floating numbers while leaving textual status notifications intact.
    StatusOnly,
}

impl CombatTextFrequency {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Important,
            Self::Important => Self::StatusOnly,
            Self::StatusOnly => Self::All,
        }
    }

    pub fn shows_damage(self, is_critical: bool) -> bool {
        match self {
            Self::All => true,
            Self::Important => is_critical,
            Self::StatusOnly => false,
        }
    }

    pub fn shows_miss(self) -> bool {
        matches!(self, Self::All | Self::Important)
    }

    pub fn shows_healing(self) -> bool {
        matches!(self, Self::All | Self::Important)
    }
}

/// Keep server-reported per-hit damage intact while avoiding one floating
/// label per division in a multi-hit packet.
pub fn format_damage_number(amount: usize, hit_count: usize) -> String {
    let hit_count = hit_count.max(1);
    if hit_count == 1 {
        amount.to_string()
    } else {
        format!("{amount} x {hit_count}")
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum CombatTextSize {
    Small,
    #[default]
    Normal,
    Large,
}

impl CombatTextSize {
    pub fn next(self) -> Self {
        match self {
            Self::Small => Self::Normal,
            Self::Normal => Self::Large,
            Self::Large => Self::Small,
        }
    }

    pub fn scale(self) -> f32 {
        match self {
            Self::Small => 0.8,
            Self::Normal => 1.0,
            Self::Large => 1.3,
        }
    }
}

/// Named equipment set stored client-side as item IDs (GDD §10.7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedEquipmentSet {
    pub name: String,
    pub item_ids: Vec<u32>,
}

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GameSettings {
    pub auto_attack: bool,
    /// Whether the in-game minimap should be shown (Alt+M / Map button).
    /// Persisted so closing it stays closed across map changes and restarts.
    #[serde(default = "default_true")]
    pub show_minimap: bool,
    /// Show a non-blocking warning when entering a map whose static-spawn mean
    /// level is at least 15 levels above this character.
    #[serde(default = "default_true")]
    pub warn_dangerous_maps: bool,
    /// Camera-relative WASD movement. Click-to-move stays available either way.
    #[serde(default = "default_true")]
    pub wasd_movement: bool,
    /// Suppress camera shake and other nonessential camera motion.
    #[serde(default)]
    pub reduce_motion: bool,
    /// Reduce the brightness of procedural combat bursts and their point
    /// lights.
    #[serde(default)]
    pub reduce_flashing: bool,
    /// Restrained audio cues for dangerous casts, interrupts, quest
    /// completion, party pings, and card drops. Every cue also has a visual.
    #[serde(default = "default_true")]
    pub audio_cues: bool,
    /// Thin other players' cosmetic skill visuals in crowded fights.
    #[serde(default)]
    pub effect_density: EffectDensity,
    /// What routes minimise first: fewest maps, zeny, walking, or locked steps.
    #[serde(default)]
    pub route_preference: crate::world::RoutePreference,
    /// Cast a ground-targeted skill at the current cursor cell when selected,
    /// falling back to the armed aim-and-click flow when the cursor has no map
    /// target.
    #[serde(default)]
    pub quickcast_ground_skills: bool,
    /// Arm a ground/trap skill while its hotbar key is held and cast at the
    /// cursor target when that key is released.
    #[serde(default)]
    pub hold_aim_release_ground_skills: bool,
    /// Per-skill target-mode overrides keyed by the server skill ID. Missing
    /// IDs inherit the global quickcast/hold options above.
    #[serde(default)]
    #[hidden_element]
    pub ground_skill_target_modes: HashMap<u16, GroundSkillTargetMode>,
    /// Show server-provided quest markers above NPCs and objective locations.
    #[serde(default = "default_true")]
    pub show_quest_markers: bool,
    /// Draw each NPC's name over its head. Off restores the hover-only label.
    #[serde(default = "default_true")]
    pub show_npc_names: bool,
    /// Draw a line where a walkable cell meets a blocked one. Off by default.
    /// This is the client's own walk mesh: a cell the server rejects and the
    /// client still calls walkable gets no line.
    #[serde(default)]
    pub show_walk_obstacles: bool,
    /// Minimap layer toggle (GDD 10.12): Towninfo facility markers (shops,
    /// Kafra, guides, inns).
    #[serde(default = "default_true")]
    pub show_minimap_facilities: bool,
    /// Minimap layer toggle (GDD 10.12): party member blips.
    #[serde(default = "default_true")]
    pub show_minimap_party: bool,
    /// Minimap layer toggle (GDD 10.12): server compass/quest marks
    /// (`ZC_COMPASS`). Independent of `show_quest_markers`, which controls
    /// the separate overhead world markers, not the minimap.
    #[serde(default = "default_true")]
    pub show_minimap_quest_markers: bool,
    /// Minimap layer toggle (GDD 9.9, 10.12): verified portal destination
    /// labels.
    #[serde(default = "default_true")]
    pub show_minimap_portals: bool,
    /// Chat timestamp prefix (GDD 10.15). On by default, as it always was.
    #[serde(default = "default_true")]
    pub show_chat_timestamps: bool,
    /// Stats window explanation mode (F04): Simple, Detailed or Advanced.
    #[serde(default)]
    #[hidden_element]
    pub stat_view_mode: crate::world::StatViewMode,
    /// Minimap layer toggle (GDD 9.4, 10.12): broad monster population regions.
    #[serde(default = "default_true")]
    pub show_minimap_population_regions: bool,
    /// GDD 11.2's Loot tab: last rate sent to `@autoloot` (0-100, the
    /// percent-and-below drop-rate threshold Hercules autoloots regardless of
    /// type). The client cannot read the server's actual current value back
    /// -- this is what the UI last sent, not a synced state.
    #[serde(default)]
    pub autoloot_rate: u8,
    /// GDD 11.2's Loot tab: item types last told to `@autoloottype +`/`-`.
    /// Same caveat as `autoloot_rate`: a preference this UI has sent, not a
    /// value read back from the server.
    #[serde(default)]
    #[hidden_element]
    pub autoloot_types: HashSet<AutolootItemType>,
    /// GDD 11.3: Visual ground-loot filter mode.
    #[serde(default)]
    pub ground_loot_filter: GroundLootFilter,
    /// GDD 11.3: Starred/wishlisted items that are never hidden by visual
    /// filters.
    #[serde(default)]
    #[hidden_element]
    pub wishlist_items: HashSet<u32>,
    /// Show floating damage, miss, and healing numbers.
    #[serde(default = "default_true")]
    pub show_combat_text: bool,
    /// Select how much floating damage feedback to show while enabled.
    #[serde(default)]
    pub combat_text_frequency: CombatTextFrequency,
    /// Size multiplier for floating combat text.
    #[serde(default)]
    pub combat_text_size: CombatTextSize,
    /// User overrides for keyboard shortcuts; missing entries use shipped
    /// defaults.
    #[serde(default)]
    #[hidden_element]
    pub key_bindings: KeyBindings,
    /// In-memory remap capture request; deliberately not persisted.
    #[serde(skip)]
    #[hidden_element]
    pub pending_key_binding: Option<BindableAction>,
    /// Last window size in **logical** pixels, restored on the next launch.
    ///
    /// Logical rather than physical so moving between monitors of different
    /// scale factors restores the same apparent size rather than the same pixel
    /// count. `None` means "never resized", and the window opens at
    /// `INITIAL_SCREEN_SIZE`.
    #[serde(default)]
    #[hidden_element]
    pub window_size: Option<(u32, u32)>,
    /// Whether the window was maximized when it last closed. Kept separate from
    /// `window_size`, which keeps the size to restore when un-maximized.
    #[serde(default)]
    #[hidden_element]
    pub window_maximized: bool,
    /// When set, the hotbar ignores drags. Number keys still cast.
    #[serde(default)]
    pub hotbar_locked: bool,
    /// Character overview shows only the name line.
    #[serde(default)]
    pub overview_minimized: bool,
    /// Item IDs protected from dropping or NPC sale, keyed by character ID.
    #[serde(default)]
    #[hidden_element]
    pub protected_items_by_character: Vec<(u32, Vec<u32>)>,
    /// Player-selected tracked quest IDs, keyed by character ID.
    #[serde(default)]
    #[hidden_element]
    pub tracked_quests_by_character: Vec<(u32, Vec<u32>)>,
    /// Player-authored, client-only hunt targets keyed by character ID.
    #[serde(default)]
    #[hidden_element]
    pub hunting_goals_by_character: Vec<(u32, Vec<u32>)>,
    /// Named equipment sets keyed by character ID (GDD §10.7).
    #[serde(default)]
    #[hidden_element]
    pub equipment_sets_by_character: Vec<(u32, Vec<NamedEquipmentSet>)>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            auto_attack: true,
            show_minimap: true,
            warn_dangerous_maps: true,
            wasd_movement: true,
            reduce_motion: false,
            reduce_flashing: false,
            audio_cues: true,
            effect_density: EffectDensity::default(),
            route_preference: crate::world::RoutePreference::default(),
            quickcast_ground_skills: false,
            hold_aim_release_ground_skills: false,
            ground_skill_target_modes: HashMap::new(),
            show_quest_markers: true,
            show_npc_names: true,
            show_walk_obstacles: false,
            show_minimap_facilities: true,
            show_minimap_party: true,
            show_minimap_quest_markers: true,
            show_minimap_portals: true,
            show_chat_timestamps: true,
            stat_view_mode: crate::world::StatViewMode::Simple,
            show_minimap_population_regions: true,
            autoloot_rate: 0,
            autoloot_types: HashSet::new(),
            ground_loot_filter: GroundLootFilter::default(),
            wishlist_items: HashSet::new(),
            show_combat_text: true,
            combat_text_frequency: CombatTextFrequency::default(),
            combat_text_size: CombatTextSize::default(),
            key_bindings: KeyBindings::default(),
            pending_key_binding: None,
            window_size: None,
            window_maximized: false,
            hotbar_locked: false,
            overview_minimized: false,
            protected_items_by_character: Vec::new(),
            tracked_quests_by_character: Vec::new(),
            hunting_goals_by_character: Vec::new(),
            equipment_sets_by_character: Vec::new(),
        }
    }
}

impl GameSettings {
    const FILE_NAME: &'static str = "client/game_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load game settings from {}", Self::FILE_NAME.magenta());
            Default::default()
        })
    }

    /// The saved window geometry, without leaving a `GameSettings` to drop.
    ///
    /// The first window is created before the client state exists, so this is
    /// read straight off disk. It deliberately does **not** hand back a
    /// `GameSettings`: dropping one writes the file (see the `Drop` impl), so a
    /// throwaway instance here would rewrite settings during startup, before
    /// anything has been loaded that could have changed them.
    pub fn saved_window_geometry() -> (Option<(u32, u32)>, bool) {
        let settings = std::mem::ManuallyDrop::new(Self::load().unwrap_or_default());
        (settings.window_size, settings.window_maximized)
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading game settings from {}", Self::FILE_NAME.magenta());
        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving game settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save game settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }

    pub fn is_item_protected(&self, character_id: u32, item_id: u32) -> bool {
        self.protected_items_by_character
            .iter()
            .find(|(id, _)| *id == character_id)
            .is_some_and(|(_, items)| items.contains(&item_id))
    }

    pub fn toggle_item_protection(&mut self, character_id: u32, item_id: u32) {
        let position = self.protected_items_by_character.iter().position(|(id, _)| *id == character_id);
        let items = match position {
            Some(position) => &mut self.protected_items_by_character[position].1,
            None => {
                self.protected_items_by_character.push((character_id, Vec::new()));
                &mut self
                    .protected_items_by_character
                    .last_mut()
                    .expect("inserted character protection list")
                    .1
            }
        };
        if let Some(position) = items.iter().position(|id| *id == item_id) {
            items.remove(position);
        } else {
            items.push(item_id);
            items.sort_unstable();
        }
        self.protected_items_by_character.retain(|(_, items)| !items.is_empty());
    }

    pub fn tracked_quests(&self, character_id: u32) -> Option<&[u32]> {
        self.tracked_quests_by_character
            .iter()
            .find(|(id, _)| *id == character_id)
            .map(|(_, quest_ids)| quest_ids.as_slice())
    }

    pub fn set_tracked_quests(&mut self, character_id: u32, quest_ids: &[u32]) {
        let quest_ids = {
            let mut quest_ids = quest_ids.to_vec();
            quest_ids.sort_unstable();
            quest_ids.dedup();
            quest_ids
        };
        if let Some((_, existing)) = self.tracked_quests_by_character.iter_mut().find(|(id, _)| *id == character_id) {
            *existing = quest_ids;
        } else {
            self.tracked_quests_by_character.push((character_id, quest_ids));
        }
    }

    pub fn hunting_goals(&self, character_id: u32) -> Option<&[u32]> {
        self.hunting_goals_by_character
            .iter()
            .find(|(id, _)| *id == character_id)
            .map(|(_, monster_ids)| monster_ids.as_slice())
    }

    pub fn set_hunting_goals(&mut self, character_id: u32, monster_ids: &[u32]) {
        let mut monster_ids = monster_ids.to_vec();
        monster_ids.sort_unstable();
        monster_ids.dedup();
        monster_ids.truncate(MAX_CLIENT_HUNTING_GOALS);
        if let Some((_, existing)) = self.hunting_goals_by_character.iter_mut().find(|(id, _)| *id == character_id) {
            *existing = monster_ids;
        } else {
            self.hunting_goals_by_character.push((character_id, monster_ids));
        }
    }

    pub fn equipment_sets(&self, character_id: u32) -> &[NamedEquipmentSet] {
        self.equipment_sets_by_character
            .iter()
            .find(|(id, _)| *id == character_id)
            .map(|(_, sets)| sets.as_slice())
            .unwrap_or(&[])
    }

    pub fn save_equipment_set(&mut self, character_id: u32, name: String, item_ids: Vec<u32>) {
        if let Some((_, sets)) = self.equipment_sets_by_character.iter_mut().find(|(id, _)| *id == character_id) {
            if let Some(existing) = sets.iter_mut().find(|s| s.name.eq_ignore_ascii_case(&name)) {
                existing.item_ids = item_ids;
            } else {
                sets.push(NamedEquipmentSet { name, item_ids });
            }
        } else {
            self.equipment_sets_by_character
                .push((character_id, vec![NamedEquipmentSet { name, item_ids }]));
        }
    }

    pub fn delete_equipment_set(&mut self, character_id: u32, name: &str) -> bool {
        if let Some((_, sets)) = self.equipment_sets_by_character.iter_mut().find(|(id, _)| *id == character_id) {
            let before = sets.len();
            sets.retain(|s| !s.name.eq_ignore_ascii_case(name));
            sets.len() < before
        } else {
            false
        }
    }

    pub fn is_wishlisted(&self, item_id: u32) -> bool {
        self.wishlist_items.contains(&item_id)
    }

    pub fn toggle_wishlist(&mut self, item_id: u32) -> bool {
        if self.wishlist_items.contains(&item_id) {
            self.wishlist_items.remove(&item_id);
            false
        } else {
            self.wishlist_items.insert(item_id);
            true
        }
    }
}

/// GDD §11.3: Visual ground-loot filter check.
///
/// Guaranteed contracts:
/// 1. Cards-always-visible rule: Card drops are NEVER hidden by any filter.
/// 2. Starred/wishlisted items are NEVER hidden by any filter.
/// 3. Equipment & Cards filter keeps gear, armor, weapons, ammo, cards, and
///    wishlist.
/// 4. Cards Only filter keeps only cards and wishlisted items.
pub fn should_render_ground_item(filter: GroundLootFilter, wishlist: &HashSet<u32>, item_id: u32) -> bool {
    // Contract 1: Cards are ALWAYS visible.
    let stats = crate::world::item_stats(item_id);
    let is_card = stats
        .as_ref()
        .is_some_and(|s| s.item_type.eq_ignore_ascii_case("Card") || s.item_type.eq_ignore_ascii_case("IT_CARD"))
        || (4000..=5000).contains(&item_id);
    if is_card {
        return true;
    }

    // Contract 2: Wishlisted items are ALWAYS visible.
    if wishlist.contains(&item_id) {
        return true;
    }

    // Contract 3: Apply filter.
    match filter {
        GroundLootFilter::All => true,
        GroundLootFilter::EquipmentAndCards => stats.as_ref().is_some_and(|s| {
            matches!(
                s.item_type.as_str(),
                "Weapon" | "Armor" | "IT_WEAPON" | "IT_ARMOR" | "IT_AMMO" | "Ammo" | "IT_CARD" | "Card"
            )
        }),
        GroundLootFilter::CardsOnly => false,
    }
}

impl Drop for GameSettings {
    fn drop(&mut self) {
        // Tests run from the client's own directory: saving here would
        // overwrite the player's real file with whatever the test built.
        #[cfg(not(test))]
        self.save();
    }
}

impl GameSettings {
    pub fn effective_ground_skill_target_mode(&self, skill_id: u16) -> GroundSkillTargetMode {
        self.ground_skill_target_modes.get(&skill_id).copied().unwrap_or({
            if self.hold_aim_release_ground_skills {
                GroundSkillTargetMode::HoldToAimRelease
            } else if self.quickcast_ground_skills {
                GroundSkillTargetMode::QuickcastAtCursor
            } else {
                GroundSkillTargetMode::AimAndClick
            }
        })
    }

    /// Cycle explicit overrides and finally return to inheriting global
    /// settings.
    pub fn cycle_ground_skill_target_mode(&mut self, skill_id: u16) -> GroundSkillTargetMode {
        let next = match self.ground_skill_target_modes.get(&skill_id).copied() {
            None => Some(GroundSkillTargetMode::AimAndClick),
            Some(GroundSkillTargetMode::AimAndClick) => Some(GroundSkillTargetMode::QuickcastAtCursor),
            Some(GroundSkillTargetMode::QuickcastAtCursor) => Some(GroundSkillTargetMode::HoldToAimRelease),
            Some(GroundSkillTargetMode::HoldToAimRelease) => None,
        };
        if let Some(mode) = next {
            self.ground_skill_target_modes.insert(skill_id, mode);
            mode
        } else {
            self.ground_skill_target_modes.remove(&skill_id);
            self.effective_ground_skill_target_mode(skill_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::mem::ManuallyDrop;

    use super::super::key_bindings::BindableAction;
    use super::{
        AutolootItemType, CombatTextFrequency, CombatTextSize, GameSettings, GroundLootFilter, GroundSkillTargetMode, format_damage_number,
        should_render_ground_item,
    };

    #[test]
    fn autoloot_item_type_command_names_match_hercules_exactly() {
        // Hercules' ACMD(autoloottype) recognizes exactly these strings
        // (src/map/atcommand.c). A typo here would silently send a command
        // the server rejects with "Item type not found." -- wrong at the
        // wire, not a compile error.
        let expected = [
            (AutolootItemType::Healing, "healing"),
            (AutolootItemType::Usable, "usable"),
            (AutolootItemType::Etc, "etc"),
            (AutolootItemType::Weapon, "weapon"),
            (AutolootItemType::Armor, "armor"),
            (AutolootItemType::Card, "card"),
            (AutolootItemType::PetEgg, "petegg"),
            (AutolootItemType::PetArmor, "petarmor"),
            (AutolootItemType::Ammo, "ammo"),
        ];
        assert_eq!(AutolootItemType::ALL.len(), expected.len());
        for (item_type, command_name) in expected {
            assert_eq!(item_type.command_name(), command_name);
        }
        // Every ALL entry is covered above, and none twice.
        let mut seen: Vec<&'static str> = AutolootItemType::ALL.iter().map(|item_type| item_type.command_name()).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.len(),
            AutolootItemType::ALL.len(),
            "AutolootItemType::ALL has a duplicate command_name"
        );
    }

    #[test]
    fn accessibility_settings_have_safe_defaults_and_migrate_older_files() {
        let default_settings = ManuallyDrop::new(GameSettings::default());
        assert!(!default_settings.reduce_motion);
        assert!(!default_settings.reduce_flashing);
        assert!(default_settings.audio_cues);
        assert!(!default_settings.quickcast_ground_skills);
        assert!(!default_settings.hold_aim_release_ground_skills);
        assert!(default_settings.show_quest_markers);
        assert!(default_settings.show_npc_names);
        assert!(default_settings.show_minimap_facilities);
        assert!(default_settings.show_minimap_party);
        assert!(default_settings.show_minimap_quest_markers);
        assert!(default_settings.show_minimap_portals);
        assert!(default_settings.show_minimap_population_regions);
        // Off by default: nothing was ever sent to the server on a fresh
        // install, so the UI must not claim otherwise.
        assert_eq!(default_settings.autoloot_rate, 0);
        assert!(default_settings.autoloot_types.is_empty());
        assert!(default_settings.warn_dangerous_maps);
        assert!(default_settings.show_combat_text);
        assert_eq!(default_settings.combat_text_frequency, CombatTextFrequency::All);
        assert_eq!(default_settings.combat_text_size, CombatTextSize::Normal);

        let old_settings: ManuallyDrop<GameSettings> = ManuallyDrop::new(ron::from_str("(auto_attack:true)").unwrap());
        assert!(!old_settings.reduce_motion);
        assert!(!old_settings.reduce_flashing);
        assert!(old_settings.audio_cues, "older settings files keep cues on");
        assert_eq!(old_settings.effect_density, super::EffectDensity::Full);
        assert_eq!(old_settings.route_preference, crate::world::RoutePreference::FewestMaps);
        assert!(!old_settings.quickcast_ground_skills);
        assert!(!old_settings.hold_aim_release_ground_skills);
        assert!(old_settings.show_quest_markers);
        assert!(old_settings.show_npc_names, "older settings files keep NPC names on");
        assert!(old_settings.show_minimap_facilities);
        assert!(old_settings.show_minimap_party);
        assert!(old_settings.show_minimap_quest_markers);
        assert!(old_settings.show_minimap_portals);
        assert!(old_settings.show_minimap_population_regions);
        assert_eq!(old_settings.autoloot_rate, 0);
        assert!(old_settings.autoloot_types.is_empty());
        assert!(old_settings.warn_dangerous_maps);
        assert!(old_settings.show_combat_text);
        assert_eq!(old_settings.combat_text_frequency, CombatTextFrequency::All);
        assert_eq!(old_settings.combat_text_size, CombatTextSize::Normal);
        assert_eq!(old_settings.key_bindings.chord(BindableAction::OpenInventory).key, "KeyI");
        assert_eq!(old_settings.pending_key_binding, None);

        let migrated_user_choices: ManuallyDrop<GameSettings> = ManuallyDrop::new(
            ron::from_str(
                "(auto_attack:true, show_combat_text:false, warn_dangerous_maps:false, show_minimap_facilities:false, \
                 combat_text_frequency:Important, combat_text_size:Large)",
            )
            .unwrap(),
        );
        assert!(!migrated_user_choices.show_combat_text);
        assert!(!migrated_user_choices.warn_dangerous_maps);
        // Explicitly turned off in the file above; must not be silently
        // re-enabled by the same defaulting that protects untouched fields.
        assert!(!migrated_user_choices.show_minimap_facilities);
        // Untouched fields still default on, same as a brand-new install.
        assert!(migrated_user_choices.show_minimap_party);
        assert!(migrated_user_choices.show_minimap_quest_markers);
        assert!(migrated_user_choices.show_minimap_portals);
        assert!(migrated_user_choices.show_minimap_population_regions);
        assert_eq!(migrated_user_choices.combat_text_frequency, CombatTextFrequency::Important);
        assert_eq!(migrated_user_choices.combat_text_size, CombatTextSize::Large);
        let status_only: ManuallyDrop<GameSettings> =
            ManuallyDrop::new(ron::from_str("(auto_attack:true, combat_text_frequency:StatusOnly)").unwrap());
        assert_eq!(status_only.combat_text_frequency, CombatTextFrequency::StatusOnly);
        let quickcast: ManuallyDrop<GameSettings> =
            ManuallyDrop::new(ron::from_str("(auto_attack:true, quickcast_ground_skills:true)").unwrap());
        assert!(quickcast.quickcast_ground_skills);
        let hold_aim: ManuallyDrop<GameSettings> =
            ManuallyDrop::new(ron::from_str("(auto_attack:true, hold_aim_release_ground_skills:true)").unwrap());
        assert!(hold_aim.hold_aim_release_ground_skills);
        assert!(old_settings.ground_skill_target_modes.is_empty());
        let per_skill_mode: ManuallyDrop<GameSettings> =
            ManuallyDrop::new(ron::from_str("(auto_attack:true, ground_skill_target_modes:{1001:QuickcastAtCursor})").unwrap());
        assert_eq!(
            per_skill_mode.effective_ground_skill_target_mode(1001),
            GroundSkillTargetMode::QuickcastAtCursor
        );
    }

    #[test]
    fn per_skill_ground_target_modes_override_globals_and_cycle_back_to_inheritance() {
        let mut settings = ManuallyDrop::new(GameSettings::default());
        assert_eq!(
            settings.effective_ground_skill_target_mode(1001),
            GroundSkillTargetMode::AimAndClick
        );

        settings.quickcast_ground_skills = true;
        assert_eq!(
            settings.effective_ground_skill_target_mode(1001),
            GroundSkillTargetMode::QuickcastAtCursor
        );
        settings.hold_aim_release_ground_skills = true;
        assert_eq!(
            settings.effective_ground_skill_target_mode(1001),
            GroundSkillTargetMode::HoldToAimRelease
        );

        assert_eq!(
            settings.cycle_ground_skill_target_mode(1001),
            GroundSkillTargetMode::AimAndClick
        );
        assert_eq!(
            settings.effective_ground_skill_target_mode(1001),
            GroundSkillTargetMode::AimAndClick
        );
        assert_eq!(
            settings.cycle_ground_skill_target_mode(1001),
            GroundSkillTargetMode::QuickcastAtCursor
        );
        assert_eq!(
            settings.cycle_ground_skill_target_mode(1001),
            GroundSkillTargetMode::HoldToAimRelease
        );
        assert_eq!(
            settings.cycle_ground_skill_target_mode(1001),
            GroundSkillTargetMode::HoldToAimRelease
        );
        assert!(!settings.ground_skill_target_modes.contains_key(&1001));
    }

    #[test]
    fn combat_text_accessibility_modes_are_predictable_and_cyclable() {
        assert!(CombatTextFrequency::All.shows_damage(false));
        assert!(CombatTextFrequency::All.shows_miss());
        assert!(CombatTextFrequency::All.shows_healing());
        assert!(CombatTextFrequency::Important.shows_damage(true));
        assert!(!CombatTextFrequency::Important.shows_damage(false));
        assert!(CombatTextFrequency::Important.shows_miss());
        assert!(CombatTextFrequency::Important.shows_healing());
        assert!(!CombatTextFrequency::StatusOnly.shows_damage(true));
        assert!(!CombatTextFrequency::StatusOnly.shows_miss());
        assert!(!CombatTextFrequency::StatusOnly.shows_healing());
        assert_eq!(CombatTextFrequency::All.next(), CombatTextFrequency::Important);
        assert_eq!(CombatTextFrequency::Important.next(), CombatTextFrequency::StatusOnly);
        assert_eq!(CombatTextFrequency::StatusOnly.next(), CombatTextFrequency::All);

        assert_eq!(CombatTextSize::Small.next(), CombatTextSize::Normal);
        assert_eq!(CombatTextSize::Normal.next(), CombatTextSize::Large);
        assert_eq!(CombatTextSize::Large.next(), CombatTextSize::Small);
        assert!(CombatTextSize::Small.scale() < CombatTextSize::Normal.scale());
        assert!(CombatTextSize::Normal.scale() < CombatTextSize::Large.scale());
    }

    #[test]
    fn client_hunting_goals_are_deduplicated_and_scoped_per_character() {
        let mut settings = ManuallyDrop::new(GameSettings::default());
        settings.set_hunting_goals(10, &[1002, 1001, 1002]);
        settings.set_hunting_goals(11, &[1003]);
        assert_eq!(settings.hunting_goals(10), Some(&[1001, 1002][..]));
        assert_eq!(settings.hunting_goals(11), Some(&[1003][..]));
        assert_eq!(settings.hunting_goals(12), None);
        let encoded = ron::ser::to_string(&*settings).unwrap();
        let loaded: ManuallyDrop<GameSettings> = ManuallyDrop::new(ron::from_str(&encoded).unwrap());
        assert_eq!(loaded.hunting_goals(10), Some(&[1001, 1002][..]));
        assert_eq!(loaded.hunting_goals(11), Some(&[1003][..]));

        let old_settings: ManuallyDrop<GameSettings> = ManuallyDrop::new(ron::from_str("(auto_attack:true)").unwrap());
        assert!(old_settings.hunting_goals_by_character.is_empty());
    }

    #[test]
    fn multi_hit_damage_text_is_compact_without_aggregating_server_damage() {
        assert_eq!(format_damage_number(123, 1), "123");
        assert_eq!(format_damage_number(123, 0), "123");
        assert_eq!(format_damage_number(123, 5), "123 x 5");
    }

    #[test]
    fn equipment_sets_persist_per_character_and_round_trip() {
        let mut settings = ManuallyDrop::new(GameSettings::default());
        settings.save_equipment_set(100, "Farming".to_owned(), vec![1101, 2101]);
        settings.save_equipment_set(100, "Boss".to_owned(), vec![1161]);
        settings.save_equipment_set(200, "Undead".to_owned(), vec![1201]);

        let sets_100 = settings.equipment_sets(100);
        assert_eq!(sets_100.len(), 2);
        assert_eq!(sets_100[0].name, "Farming");
        assert_eq!(sets_100[0].item_ids, vec![1101, 2101]);

        let encoded = ron::ser::to_string(&*settings).unwrap();
        let mut loaded: ManuallyDrop<GameSettings> = ManuallyDrop::new(ron::from_str(&encoded).unwrap());
        assert_eq!(loaded.equipment_sets(100).len(), 2);
        assert_eq!(loaded.equipment_sets(200).len(), 1);

        assert!(loaded.delete_equipment_set(100, "Farming"));
        assert_eq!(loaded.equipment_sets(100).len(), 1);
        assert_eq!(loaded.equipment_sets(100)[0].name, "Boss");
    }

    #[test]
    fn ground_loot_filter_and_wishlist_preserve_cards_and_wishlisted_items() {
        let poring_card_id = 4001; // Poring Card
        let sword_id = 1101; // Sword (Weapon)
        let fluff_id = 914; // Fluff (Etc)

        let mut wishlist = HashSet::new();

        // Under All filter: everything visible
        assert!(should_render_ground_item(GroundLootFilter::All, &wishlist, poring_card_id));
        assert!(should_render_ground_item(GroundLootFilter::All, &wishlist, sword_id));
        assert!(should_render_ground_item(GroundLootFilter::All, &wishlist, fluff_id));

        // Under EquipmentAndCards filter: card and sword visible, fluff hidden
        assert!(should_render_ground_item(
            GroundLootFilter::EquipmentAndCards,
            &wishlist,
            poring_card_id
        ));
        assert!(should_render_ground_item(
            GroundLootFilter::EquipmentAndCards,
            &wishlist,
            sword_id
        ));
        assert!(!should_render_ground_item(
            GroundLootFilter::EquipmentAndCards,
            &wishlist,
            fluff_id
        ));

        // Wishlisting fluff makes it visible even under EquipmentAndCards filter
        wishlist.insert(fluff_id);
        assert!(should_render_ground_item(
            GroundLootFilter::EquipmentAndCards,
            &wishlist,
            fluff_id
        ));

        // Under CardsOnly filter: card and wishlisted fluff visible, non-wishlisted
        // sword hidden
        assert!(should_render_ground_item(
            GroundLootFilter::CardsOnly,
            &wishlist,
            poring_card_id
        ));
        assert!(should_render_ground_item(GroundLootFilter::CardsOnly, &wishlist, fluff_id));
        assert!(!should_render_ground_item(GroundLootFilter::CardsOnly, &wishlist, sword_id));

        // Cards-always-visible rule: card is visible even with empty wishlist on
        // CardsOnly
        wishlist.clear();
        assert!(should_render_ground_item(
            GroundLootFilter::CardsOnly,
            &wishlist,
            poring_card_id
        ));
    }

    #[test]
    fn ground_loot_settings_round_trip() {
        let mut settings = ManuallyDrop::new(GameSettings::default());
        settings.ground_loot_filter = GroundLootFilter::EquipmentAndCards;
        // `/wishlist <id>` toggles, so the first call adds.
        assert!(settings.toggle_wishlist(914));
        assert!(settings.toggle_wishlist(501));

        assert!(settings.is_wishlisted(914));
        assert!(settings.is_wishlisted(501));
        assert!(!settings.is_wishlisted(1001));

        let encoded = ron::ser::to_string(&*settings).unwrap();
        let loaded: ManuallyDrop<GameSettings> = ManuallyDrop::new(ron::from_str(&encoded).unwrap());
        assert_eq!(loaded.ground_loot_filter, GroundLootFilter::EquipmentAndCards);
        assert!(loaded.is_wishlisted(914));
        assert!(loaded.is_wishlisted(501));

        let mut loaded = loaded;
        assert!(!loaded.toggle_wishlist(914), "the second toggle removes");
        assert!(!loaded.is_wishlisted(914));
        assert!(loaded.is_wishlisted(501));
    }

    #[test]
    fn effect_density_never_thins_your_own_or_hostile_visuals() {
        use super::{EffectDensity, EffectSource};
        for density in [EffectDensity::Full, EffectDensity::Reduced, EffectDensity::Minimal] {
            assert!(
                density.shows_skill_visual(EffectSource::Local),
                "{density:?} hides your own skill"
            );
            assert!(
                density.shows_skill_visual(EffectSource::Hostile),
                "{density:?} hides a hostile warning"
            );
        }
        assert!(EffectDensity::Full.shows_skill_visual(EffectSource::Bystander));
        assert!(EffectDensity::Reduced.shows_skill_visual(EffectSource::Party));
        assert!(!EffectDensity::Reduced.shows_skill_visual(EffectSource::Bystander));
        assert!(!EffectDensity::Minimal.shows_skill_visual(EffectSource::Party));
        assert!(!EffectDensity::Minimal.shows_skill_visual(EffectSource::Bystander));
        assert_eq!(EffectDensity::Full.next(), EffectDensity::Reduced);
        assert_eq!(EffectDensity::Reduced.next(), EffectDensity::Minimal);
        assert_eq!(EffectDensity::Minimal.next(), EffectDensity::Full);
    }

    #[test]
    fn effect_density_minimal_preserves_telegraphs_and_threats() {
        use ragnarok_packets::{SkillId, SkillLevel};

        use super::{EffectDensity, EffectSource};
        use crate::world::{level_invariant_skill_footprint, skill_footprint};

        let density = EffectDensity::Minimal;
        // Hostile warnings and local actions are always permitted.
        assert!(density.shows_skill_visual(EffectSource::Hostile));
        assert!(density.shows_skill_visual(EffectSource::Local));
        // Non-hostile visuals are suppressed.
        assert!(!density.shows_skill_visual(EffectSource::Party));
        assert!(!density.shows_skill_visual(EffectSource::Bystander));

        // Lethal ground hazards and telegraph footprints (e.g., Storm Gust 89, Pneuma
        // 34) are computed independently of effect density and remain intact.
        let storm_gust = skill_footprint(SkillId(89), SkillLevel(5), 0);
        assert!(storm_gust.is_some(), "Storm Gust telegraph footprint must exist");
        let invariant = level_invariant_skill_footprint(SkillId(89), 0);
        assert!(invariant.is_some(), "Storm Gust level-invariant footprint must exist");
        let pneuma = skill_footprint(SkillId(25), SkillLevel(1), 0);
        assert!(pneuma.is_some(), "Pneuma telegraph footprint must exist");
    }

    #[test]
    fn effect_density_never_thins_cast_telegraph_footprints() {
        use ragnarok_packets::SkillId;

        use super::{EffectDensity, EffectSource};
        use crate::world::level_invariant_skill_footprint;

        // Across all densities, hostile casts and local warnings are never suppressed.
        for density in [EffectDensity::Full, EffectDensity::Reduced, EffectDensity::Minimal] {
            assert!(density.shows_skill_visual(EffectSource::Hostile));
            assert!(density.shows_skill_visual(EffectSource::Local));
        }

        // Critical lethal and area hazards retain their exact multi-cell footprint.
        let lethal_skills = [
            SkillId(89),  // Storm Gust
            SkillId(70),  // Sanctuary
            SkillId(18),  // Fire Wall
            SkillId(87),  // Ice Wall
            SkillId(25),  // Pneuma
            SkillId(254), // Grand Cross
            SkillId(404), // Fog Wall
        ];

        for skill_id in lethal_skills {
            let footprint = level_invariant_skill_footprint(skill_id, 0);
            assert!(footprint.is_some(), "Skill {skill_id:?} must retain invariant footprint");
            let cells = footprint.unwrap();
            assert!(!cells.is_empty(), "Skill {skill_id:?} footprint must not be empty");
        }
    }
}
