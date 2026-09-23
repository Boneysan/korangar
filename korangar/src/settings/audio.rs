#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

/// A persisted loudness step. The audio engine takes a 0–1 gain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub struct VolumePreset(pub u8);

impl VolumePreset {
    pub const CHOICES: [VolumePreset; 5] = [
        VolumePreset(0),
        VolumePreset(25),
        VolumePreset(50),
        VolumePreset(75),
        VolumePreset(100),
    ];

    pub fn gain(self) -> f32 {
        f32::from(self.0) / 100.0
    }
}

impl Default for VolumePreset {
    fn default() -> Self {
        VolumePreset(100)
    }
}

impl DropDownItem<VolumePreset> for VolumePreset {
    fn text(&self) -> &str {
        match self.0 {
            0 => "Off",
            25 => "25%",
            50 => "50%",
            75 => "75%",
            100 => "100%",
            _ => "Custom",
        }
    }

    fn value(&self) -> VolumePreset {
        *self
    }
}

fn default_music() -> VolumePreset {
    VolumePreset(50)
}

fn default_master() -> VolumePreset {
    VolumePreset(100)
}

fn default_effects() -> VolumePreset {
    VolumePreset(100)
}

fn default_volume_choices() -> Vec<VolumePreset> {
    VolumePreset::CHOICES.to_vec()
}

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
#[serde(default)]
pub struct AudioSettings {
    pub mute_on_focus_loss: bool,
    #[serde(default = "default_master")]
    pub master: VolumePreset,
    #[serde(default = "default_music")]
    pub music: VolumePreset,
    #[serde(default = "default_effects")]
    pub effects: VolumePreset,
    #[serde(default = "default_volume_choices")]
    pub volume_choices: Vec<VolumePreset>,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            mute_on_focus_loss: true,
            master: default_master(),
            music: default_music(),
            effects: default_effects(),
            volume_choices: default_volume_choices(),
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
