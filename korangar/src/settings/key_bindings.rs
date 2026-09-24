use serde::{Deserialize, Serialize};

/// Stable action identifiers for user-remappable keyboard shortcuts.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum BindableAction {
    OpenInventory,
    OpenCharacterOverview,
    OpenSkillTree,
    OpenStats,
    OpenParty,
    OpenEquipment,
    OpenFriendList,
    OpenHud,
    OpenAudioSettings,
    OpenGameSettings,
    OpenInterfaceSettings,
    OpenGraphicsSettings,
    OpenQuestLog,
    CloseTopWindow,
    CloseAllWindows,
    CycleMonsterTarget,
    CyclePartyTarget,
    SendDangerPing,
    ToggleInterface,
    ToggleMinimap,
    ToggleMaps,
    ToggleCommands,
    ToggleDice,
    ToggleSit,
    TargetSelf,
    ToggleEmotes,
    ToggleFullscreen,
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    HotbarSlot(u8),
}

impl BindableAction {
    pub fn all() -> Vec<Self> {
        let mut actions = vec![
            Self::OpenInventory,
            Self::OpenCharacterOverview,
            Self::OpenSkillTree,
            Self::OpenStats,
            Self::OpenParty,
            Self::OpenEquipment,
            Self::OpenFriendList,
            Self::OpenHud,
            Self::OpenAudioSettings,
            Self::OpenGameSettings,
            Self::OpenInterfaceSettings,
            Self::OpenGraphicsSettings,
            Self::OpenQuestLog,
            Self::CloseTopWindow,
            Self::CloseAllWindows,
            Self::CycleMonsterTarget,
            Self::CyclePartyTarget,
            Self::SendDangerPing,
            Self::ToggleInterface,
            Self::ToggleMinimap,
            Self::ToggleMaps,
            Self::ToggleCommands,
            Self::ToggleDice,
            Self::ToggleSit,
            Self::TargetSelf,
            Self::ToggleEmotes,
            Self::ToggleFullscreen,
            Self::MoveForward,
            Self::MoveBackward,
            Self::MoveLeft,
            Self::MoveRight,
        ];
        actions.extend((0..27).map(Self::HotbarSlot));
        actions
    }

    pub fn label(self) -> String {
        if let Self::HotbarSlot(slot) = self {
            return format!("Hotbar slot {}", slot + 1);
        }
        match self {
            Self::OpenInventory => "Open inventory",
            Self::OpenCharacterOverview => "Open character overview",
            Self::OpenSkillTree => "Open skill tree",
            Self::OpenStats => "Open stats",
            Self::OpenParty => "Open party",
            Self::OpenEquipment => "Open equipment",
            Self::OpenFriendList => "Open friend list",
            Self::OpenHud => "Open HUD",
            Self::OpenAudioSettings => "Open audio settings",
            Self::OpenGameSettings => "Open game settings",
            Self::OpenInterfaceSettings => "Open interface settings",
            Self::OpenGraphicsSettings => "Open graphics settings",
            Self::OpenQuestLog => "Open quest log",
            Self::CloseTopWindow => "Close top window",
            Self::CloseAllWindows => "Close all ordinary windows",
            Self::CycleMonsterTarget => "Cycle monster target",
            Self::CyclePartyTarget => "Cycle party target",
            Self::SendDangerPing => "Send party danger ping",
            Self::ToggleInterface => "Toggle interface",
            Self::ToggleMinimap => "Toggle minimap",
            Self::ToggleMaps => "Open world map",
            Self::ToggleCommands => "Open commands",
            Self::ToggleDice => "Open dice roller",
            Self::ToggleSit => "Sit / stand",
            Self::TargetSelf => "Target yourself",
            Self::ToggleEmotes => "Open emotes",
            Self::ToggleFullscreen => "Toggle fullscreen",
            Self::MoveForward => "Move forward",
            Self::MoveBackward => "Move backward",
            Self::MoveLeft => "Move left",
            Self::MoveRight => "Move right",
            Self::HotbarSlot(_) => unreachable!(),
        }
        .to_owned()
    }

    pub fn default_chord(self) -> KeyChord {
        if let Self::HotbarSlot(slot) = self {
            let slot = slot % 9;
            return KeyChord::new(
                format!("Digit{}", slot + 1),
                self.hotbar_row() == 1,
                self.hotbar_row() == 2,
                false,
            );
        }
        let (key, control, alt, shift) = match self {
            Self::OpenInventory => ("KeyI", false, false, false),
            Self::OpenCharacterOverview => ("KeyV", false, true, false),
            Self::OpenSkillTree => ("KeyS", false, true, false),
            Self::OpenStats => ("KeyA", false, true, false),
            Self::OpenParty => ("KeyZ", false, true, false),
            Self::OpenEquipment => ("KeyQ", false, true, false),
            Self::OpenFriendList => ("KeyH", false, true, false),
            Self::OpenHud => ("KeyH", false, true, true),
            Self::OpenAudioSettings => ("KeyO", false, true, false),
            Self::OpenGameSettings => ("KeyS", true, false, false),
            Self::OpenInterfaceSettings => ("KeyI", true, false, false),
            Self::OpenGraphicsSettings => ("KeyG", true, false, false),
            Self::OpenQuestLog => ("KeyQ", true, false, false),
            Self::CloseTopWindow => ("KeyW", true, false, false),
            Self::CloseAllWindows => ("F11", false, false, false),
            Self::CycleMonsterTarget => ("Tab", false, false, false),
            Self::CyclePartyTarget => ("Tab", true, false, true),
            Self::SendDangerPing => ("KeyG", true, true, false),
            Self::ToggleInterface => ("KeyH", true, false, false),
            Self::ToggleMinimap => ("Tab", true, false, false),
            Self::ToggleMaps => ("KeyM", true, false, false),
            Self::ToggleCommands => ("KeyO", true, false, false),
            Self::ToggleDice => ("KeyD", true, false, false),
            Self::ToggleSit => ("Insert", false, false, false),
            Self::TargetSelf => ("KeyT", false, false, false),
            Self::ToggleEmotes => ("KeyL", false, true, false),
            Self::ToggleFullscreen => ("Enter", false, true, false),
            Self::MoveForward => ("KeyW", false, false, false),
            Self::MoveBackward => ("KeyS", false, false, false),
            Self::MoveLeft => ("KeyA", false, false, false),
            Self::MoveRight => ("KeyD", false, false, false),
            Self::HotbarSlot(_) => unreachable!(),
        };
        KeyChord {
            key: key.to_owned(),
            control,
            alt,
            shift,
        }
    }

    pub const fn hotbar_row(self) -> u8 {
        match self {
            Self::HotbarSlot(slot) => slot / 9,
            _ => 0,
        }
    }
}

/// Winit's stable physical `KeyCode` debug name and exact modifier state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeyChord {
    pub key: String,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyChord {
    pub fn new(key: impl Into<String>, control: bool, alt: bool, shift: bool) -> Self {
        Self {
            key: key.into(),
            control,
            alt,
            shift,
        }
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::with_capacity(4);
        if self.control {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        let key_label = self
            .key
            .strip_prefix("Key")
            .or_else(|| self.key.strip_prefix("Digit"))
            .unwrap_or(self.key.as_str());
        parts.push(key_label);
        parts.join("+")
    }

    fn is_valid_key(&self) -> bool {
        let key = self.key.as_str();
        (key.len() == 4 && key.starts_with("Key") && key.as_bytes()[3].is_ascii_uppercase())
            || (key.len() == 6 && key.starts_with("Digit") && key.as_bytes()[5].is_ascii_digit())
            || key
                .strip_prefix('F')
                .is_some_and(|number| number.parse::<u8>().is_ok_and(|number| (1..=35).contains(&number)))
            || (key.len() == 7 && key.starts_with("Numpad") && key.as_bytes()[6].is_ascii_digit())
            || matches!(
                key,
                "Space"
                    | "Enter"
                    | "Tab"
                    | "Backspace"
                    | "Delete"
                    | "Insert"
                    | "Home"
                    | "End"
                    | "PageUp"
                    | "PageDown"
                    | "ArrowUp"
                    | "ArrowDown"
                    | "ArrowLeft"
                    | "ArrowRight"
                    | "Minus"
                    | "Equal"
                    | "BracketLeft"
                    | "BracketRight"
                    | "Backslash"
                    | "Semicolon"
                    | "Quote"
                    | "Backquote"
                    | "Comma"
                    | "Period"
                    | "Slash"
                    | "NumpadAdd"
                    | "NumpadSubtract"
                    | "NumpadMultiply"
                    | "NumpadDivide"
                    | "NumpadDecimal"
                    | "NumpadEnter"
            )
    }

    pub fn is_reserved(&self) -> bool {
        matches!(
            self.key.as_str(),
            "Escape" | "AltLeft" | "AltRight" | "ControlLeft" | "ControlRight" | "ShiftLeft" | "ShiftRight"
        ) || (self.alt && matches!(self.key.as_str(), "F4"))
            || (self.control && self.alt && self.key == "Delete")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    #[serde(default = "current_version")]
    pub version: u16,
    #[serde(default)]
    entries: Vec<(BindableAction, KeyChord)>,
}

const fn current_version() -> u16 {
    1
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            version: current_version(),
            entries: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingError {
    InvalidKey,
    Reserved,
    Conflict(BindableAction),
}

impl KeyBindings {
    pub fn chord(&self, action: BindableAction) -> KeyChord {
        self.entries
            .iter()
            .find(|(entry_action, _)| *entry_action == action)
            .map(|(_, chord)| chord.clone())
            .unwrap_or_else(|| action.default_chord())
    }

    pub fn assign(&mut self, action: BindableAction, chord: KeyChord) -> Result<(), BindingError> {
        if chord.is_reserved() {
            return Err(BindingError::Reserved);
        }
        if !chord.is_valid_key() {
            return Err(BindingError::InvalidKey);
        }
        if matches!(action, BindableAction::HotbarSlot(slot) if slot >= 27) {
            return Err(BindingError::InvalidKey);
        }
        for other_action in BindableAction::all() {
            if other_action != action && self.chord(other_action) == chord {
                return Err(BindingError::Conflict(other_action));
            }
        }
        if let Some((_, existing)) = self.entries.iter_mut().find(|(entry_action, _)| *entry_action == action) {
            *existing = chord;
        } else {
            self.entries.push((action, chord));
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.version = current_version();
        self.entries.clear();
    }

    pub fn export_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())
    }

    pub fn import_ron(data: &str) -> Result<Self, String> {
        let mut bindings: Self = ron::from_str(data).map_err(|error| format!("invalid keybinding file: {error}"))?;
        if bindings.version != current_version() {
            return Err(format!("unsupported keybinding version {}", bindings.version));
        }
        let entries = std::mem::take(&mut bindings.entries);
        let mut seen_actions = std::collections::HashSet::new();
        for (action, chord) in entries {
            if !seen_actions.insert(action) {
                return Err(format!("duplicate keybinding for {}", action.label()));
            }
            bindings
                .assign(action, chord)
                .map_err(|error| format!("invalid keybinding: {error:?}"))?;
        }
        Ok(bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::{BindableAction, BindingError, KeyBindings, KeyChord};

    #[test]
    fn shipped_bindings_preserve_distinct_chords_and_can_be_reset() {
        let mut bindings = KeyBindings::default();
        assert_eq!(bindings.chord(BindableAction::OpenInventory).display(), "I");
        assert_eq!(bindings.chord(BindableAction::OpenFriendList).display(), "Alt+H");
        assert_eq!(bindings.chord(BindableAction::OpenHud).display(), "Alt+Shift+H");
        let actions = BindableAction::all();
        for (index, action) in actions.iter().enumerate() {
            for other_action in &actions[index + 1..] {
                assert_ne!(
                    bindings.chord(*action),
                    bindings.chord(*other_action),
                    "default binding conflict: {:?}",
                    action
                );
            }
        }
        bindings
            .assign(BindableAction::OpenInventory, KeyChord::new("KeyJ", false, false, false))
            .unwrap();
        assert_eq!(bindings.chord(BindableAction::OpenInventory).key, "KeyJ");
        bindings.reset();
        assert_eq!(bindings.chord(BindableAction::OpenInventory).key, "KeyI");
        assert_eq!(
            bindings.assign(BindableAction::OpenInventory, KeyChord::new("not-a-key", false, false, false)),
            Err(BindingError::InvalidKey)
        );
    }

    #[test]
    fn reserved_and_conflicting_chords_are_rejected_without_mutating_bindings() {
        let mut bindings = KeyBindings::default();
        assert_eq!(
            bindings.assign(BindableAction::OpenInventory, KeyChord::new("Escape", false, false, false)),
            Err(BindingError::Reserved)
        );
        assert_eq!(
            bindings.assign(BindableAction::OpenInventory, KeyChord::new("KeyH", false, true, false)),
            Err(BindingError::Conflict(BindableAction::OpenFriendList)),
        );
        assert_eq!(
            bindings.assign(BindableAction::OpenInventory, KeyChord::new("not-a-key", false, false, false)),
            Err(BindingError::InvalidKey),
        );
        assert_eq!(bindings.chord(BindableAction::OpenInventory).key, "KeyI");
    }

    #[test]
    fn versioned_keybindings_round_trip_and_reject_invalid_conflicts() {
        let mut bindings = KeyBindings::default();
        bindings
            .assign(BindableAction::OpenInventory, KeyChord::new("KeyJ", false, false, false))
            .unwrap();
        let serialized = bindings.export_ron().unwrap();
        let loaded = KeyBindings::import_ron(&serialized).unwrap();
        assert_eq!(
            loaded.chord(BindableAction::OpenInventory),
            KeyChord::new("KeyJ", false, false, false)
        );
        let conflict = "(version:1,entries:[(OpenInventory,(key:\"KeyI\",control:false,alt:false,shift:false)),(OpenStats,(key:\"KeyI\",\
                        control:false,alt:false,shift:false))])";
        assert!(KeyBindings::import_ron(conflict).is_err());
    }

    #[test]
    fn hotbar_slot_bindings_round_trip_and_reject_out_of_range_slots() {
        let mut bindings = KeyBindings::default();
        let remapped = KeyChord::new("KeyJ", false, false, false);
        bindings.assign(BindableAction::HotbarSlot(26), remapped.clone()).unwrap();
        let mut loaded = KeyBindings::import_ron(&bindings.export_ron().unwrap()).unwrap();
        assert_eq!(loaded.chord(BindableAction::HotbarSlot(26)), remapped);
        assert_eq!(
            loaded.assign(BindableAction::HotbarSlot(27), KeyChord::new("KeyK", false, false, false)),
            Err(BindingError::InvalidKey),
        );
    }
}
