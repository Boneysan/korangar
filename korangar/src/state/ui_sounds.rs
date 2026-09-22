//! Restrained UI sounds: one shot per state transition.

use std::collections::HashMap;
use std::hash::Hash;

use crate::settings::AudioSettings;

/// A sink for UI sound events.
pub trait UiSoundSink {
    /// Plays the accepted / activation sound effect.
    fn play_activation(&mut self);

    /// Plays the explicit rejection sound effect.
    fn play_rejection(&mut self);
}

/// A mock sound sink that counts sound activations and rejections for tests.
#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MockUiSoundSink {
    pub activations: usize,
    pub rejections: usize,
}

#[cfg(test)]
impl MockUiSoundSink {
    pub const fn new() -> Self {
        Self {
            activations: 0,
            rejections: 0,
        }
    }
}

#[cfg(test)]
impl UiSoundSink for MockUiSoundSink {
    fn play_activation(&mut self) {
        self.activations += 1;
    }

    fn play_rejection(&mut self) {
        self.rejections += 1;
    }
}

/// Edge-detection gate for UI sounds.
///
/// Ensures that holding a button or repeating an input frame does not replay
/// the sound: sound triggers strictly on the rising edge (`false -> true`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiSoundGate {
    last_activation: bool,
}

impl UiSoundGate {
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self { last_activation: false }
    }

    /// Evaluates edge transition: returns true only when transitioning from
    /// false to true. Holding (`down == true` on successive calls) returns
    /// false.
    pub fn on_activation(&mut self, down: bool) -> bool {
        let play = down && !self.last_activation;
        self.last_activation = down;
        play
    }

    /// Explicitly resets the gate state so the next `down == true` triggers
    /// again.
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.last_activation = false;
    }

    /// Returns whether the gate is currently considered active (held down).
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.last_activation
    }
}

/// Gate collection keyed by logical action or control identity.
///
/// Ensures different controls maintain separate edge detection state,
/// avoiding any single global gate blocking distinct actions while guaranteeing
/// that no individual control repeats while held.
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct KeyedUiSoundGate<K: Eq + Hash> {
    gates: HashMap<K, UiSoundGate>,
}

#[allow(dead_code)]
impl<K: Eq + Hash> KeyedUiSoundGate<K> {
    pub fn new() -> Self {
        Self { gates: HashMap::new() }
    }

    pub fn on_activation(&mut self, key: K, down: bool) -> bool {
        self.gates.entry(key).or_default().on_activation(down)
    }

    pub fn reset(&mut self, key: &K) {
        if let Some(gate) = self.gates.get_mut(key) {
            gate.reset();
        }
    }

    pub fn clear(&mut self) {
        self.gates.clear();
    }
}

/// Logical controller managing restrained UI sounds across distinct controls
/// and actions.
#[derive(Clone, Debug, Default)]
pub struct UiSoundController {
    click_gate: UiSoundGate,
    #[allow(dead_code)]
    rejection_gate: UiSoundGate,
    #[allow(dead_code)]
    action_gates: KeyedUiSoundGate<String>,
}

impl UiSoundController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles a UI click transition (e.g. mouse button down over interface).
    pub fn on_click(&mut self, down: bool, settings: &AudioSettings, sink: &mut dyn UiSoundSink) -> bool {
        if self.click_gate.on_activation(down) && settings.ui_sound_enabled && settings.ui_sound_volume > 0.0 {
            sink.play_activation();
            return true;
        }
        false
    }

    /// Handles an action-specific activation transition (e.g. hotbar slot, menu
    /// action).
    #[allow(dead_code)]
    pub fn on_action(&mut self, action_name: &str, down: bool, settings: &AudioSettings, sink: &mut dyn UiSoundSink) -> bool {
        if self.action_gates.on_activation(action_name.to_string(), down) && settings.ui_sound_enabled && settings.ui_sound_volume > 0.0 {
            sink.play_activation();
            return true;
        }
        false
    }

    /// Handles an explicit rejection result with edge gating.
    #[allow(dead_code)]
    pub fn on_rejection(&mut self, rejected: bool, settings: &AudioSettings, sink: &mut dyn UiSoundSink) -> bool {
        if self.rejection_gate.on_activation(rejected) && settings.ui_sound_enabled && settings.ui_sound_volume > 0.0 {
            sink.play_rejection();
            return true;
        }
        false
    }

    /// Explicit single-shot rejection (e.g. on receiving a discrete failure
    /// packet).
    pub fn trigger_rejection(&mut self, settings: &AudioSettings, sink: &mut dyn UiSoundSink) -> bool {
        if settings.ui_sound_enabled && settings.ui_sound_volume > 0.0 {
            sink.play_rejection();
            true
        } else {
            false
        }
    }

    /// Explicit single-shot activation (e.g. discrete menu/server transition
    /// event).
    pub fn trigger_activation(&mut self, settings: &AudioSettings, sink: &mut dyn UiSoundSink) -> bool {
        if settings.ui_sound_enabled && settings.ui_sound_volume > 0.0 {
            sink.play_activation();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_does_not_repeat() {
        let mut gate = UiSoundGate::default();
        assert!(gate.on_activation(true));
        assert!(!gate.on_activation(true));
        assert!(!gate.on_activation(true));
        assert!(!gate.on_activation(false));
        assert!(gate.on_activation(true));
    }

    #[test]
    fn click_hold_release_reclick_via_controller() {
        let mut controller = UiSoundController::new();
        let mut sink = MockUiSoundSink::new();
        let settings = AudioSettings::default();

        // 1. Initial click -> fires activation sound
        assert!(controller.on_click(true, &settings, &mut sink));
        assert_eq!(sink.activations, 1);
        assert_eq!(sink.rejections, 0);

        // 2. Holding down across multiple frames -> zero additional sounds
        assert!(!controller.on_click(true, &settings, &mut sink));
        assert!(!controller.on_click(true, &settings, &mut sink));
        assert_eq!(sink.activations, 1);

        // 3. Release -> no sound
        assert!(!controller.on_click(false, &settings, &mut sink));
        assert_eq!(sink.activations, 1);

        // 4. Reclick -> fires second activation sound
        assert!(controller.on_click(true, &settings, &mut sink));
        assert_eq!(sink.activations, 2);
    }

    #[test]
    fn rejected_action_handling() {
        let mut controller = UiSoundController::new();
        let mut sink = MockUiSoundSink::new();
        let settings = AudioSettings::default();

        // Edge-gated rejection: triggers once, holding does not repeat
        assert!(controller.on_rejection(true, &settings, &mut sink));
        assert_eq!(sink.rejections, 1);
        assert_eq!(sink.activations, 0);

        assert!(!controller.on_rejection(true, &settings, &mut sink));
        assert_eq!(sink.rejections, 1);

        assert!(!controller.on_rejection(false, &settings, &mut sink));
        assert!(controller.on_rejection(true, &settings, &mut sink));
        assert_eq!(sink.rejections, 2);

        // Discrete packet failure trigger
        assert!(controller.trigger_rejection(&settings, &mut sink));
        assert_eq!(sink.rejections, 3);
    }

    #[test]
    fn disabled_and_zero_volume_silences_playback() {
        let mut controller = UiSoundController::new();
        let mut sink = MockUiSoundSink::new();

        // Disabled setting
        let mut disabled_settings = AudioSettings::default();
        disabled_settings.ui_sound_enabled = false;

        assert!(!controller.on_click(true, &disabled_settings, &mut sink));
        assert!(!controller.on_rejection(true, &disabled_settings, &mut sink));
        assert!(!controller.trigger_activation(&disabled_settings, &mut sink));
        assert!(!controller.trigger_rejection(&disabled_settings, &mut sink));
        assert_eq!(sink.activations, 0);
        assert_eq!(sink.rejections, 0);

        // Zero volume setting
        let mut zero_vol_settings = AudioSettings::default();
        zero_vol_settings.ui_sound_volume = 0.0;

        assert!(!controller.on_click(true, &zero_vol_settings, &mut sink));
        assert!(!controller.on_rejection(true, &zero_vol_settings, &mut sink));
        assert!(!controller.trigger_activation(&zero_vol_settings, &mut sink));
        assert!(!controller.trigger_rejection(&zero_vol_settings, &mut sink));
        assert_eq!(sink.activations, 0);
        assert_eq!(sink.rejections, 0);
    }

    #[test]
    fn independent_controls_do_not_block_each_other() {
        let mut controller = UiSoundController::new();
        let mut sink = MockUiSoundSink::new();
        let settings = AudioSettings::default();

        // Control A activated
        assert!(controller.on_action("tab_1", true, &settings, &mut sink));
        assert_eq!(sink.activations, 1);

        // Control A held
        assert!(!controller.on_action("tab_1", true, &settings, &mut sink));

        // Control B activated while A is held -> B is not blocked!
        assert!(controller.on_action("tab_2", true, &settings, &mut sink));
        assert_eq!(sink.activations, 2);

        // Mouse click while tabs are active -> click is not blocked!
        assert!(controller.on_click(true, &settings, &mut sink));
        assert_eq!(sink.activations, 3);
    }
}
