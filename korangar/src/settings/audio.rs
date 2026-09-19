#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_volume() -> f32 {
    1.0
}

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct AudioSettings {
    pub mute_on_focus_loss: bool,
    #[serde(default = "default_true")]
    pub ui_sound_enabled: bool,
    #[serde(default = "default_volume")]
    pub ui_sound_volume: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            mute_on_focus_loss: true,
            ui_sound_enabled: true,
            ui_sound_volume: 1.0,
        }
    }
}

impl AudioSettings {
    const FILE_NAME: &'static str = "client/audio_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load audio settings from {}", Self::FILE_NAME.magenta());
            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading audio settings from {}", Self::FILE_NAME.magenta());
        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn set_ui_sound_volume(&mut self, volume: f32) {
        self.ui_sound_volume = volume.clamp(0.0, 1.0);
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving audio settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save audio settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for AudioSettings {
    fn drop(&mut self) {
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_settings_default_values() {
        let settings = AudioSettings::default();
        assert!(settings.mute_on_focus_loss);
        assert!(settings.ui_sound_enabled);
        assert_eq!(settings.ui_sound_volume, 1.0);
    }

    #[test]
    fn audio_settings_round_trip_serialization() {
        let settings = AudioSettings {
            mute_on_focus_loss: false,
            ui_sound_enabled: false,
            ui_sound_volume: 0.65,
        };

        let serialized = ron::ser::to_string_pretty(&settings, PrettyConfig::new()).expect("serialization should succeed");
        let deserialized: AudioSettings = ron::from_str(&serialized).expect("deserialization should succeed");

        assert_eq!(deserialized.mute_on_focus_loss, false);
        assert_eq!(deserialized.ui_sound_enabled, false);
        assert!((deserialized.ui_sound_volume - 0.65).abs() < f32::EPSILON);
    }

    #[test]
    fn audio_settings_backward_compatible_fallback() {
        let legacy_ron = "(mute_on_focus_loss: false)";
        let deserialized: AudioSettings = ron::from_str(legacy_ron).expect("legacy deserialization should succeed");

        assert_eq!(deserialized.mute_on_focus_loss, false);
        assert_eq!(deserialized.ui_sound_enabled, true);
        assert_eq!(deserialized.ui_sound_volume, 1.0);
    }
}
