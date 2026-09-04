#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{
    LimitFramerate, Msaa, PresentModeInfo, ScreenSpaceAntiAliasing, ShadowDetail, ShadowMethod, ShadowResolution, SpriteLightingMode, Ssaa,
    TextureSamplerType,
};

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GraphicsSettings {
    /// Added after the settings file already existed in the wild, so a file
    /// written by an older build has no such key. Without `serde(default)` that
    /// one missing field fails the whole parse and [`Self::load`] falls through
    /// to [`Default::default`], silently discarding every other graphics
    /// setting the player had chosen.
    #[serde(default)]
    pub display_mode: DisplayMode,
    pub lighting_mode: LightingMode,
    #[serde(default)]
    pub sprite_lighting_mode: SpriteLightingMode,
    pub vsync: bool,
    pub limit_framerate: LimitFramerate,
    pub triple_buffering: bool,
    pub texture_filtering: TextureSamplerType,
    pub msaa: Msaa,
    pub ssaa: Ssaa,
    pub screen_space_anti_aliasing: ScreenSpaceAntiAliasing,
    pub shadow_method: ShadowMethod,
    pub shadow_resolution: ShadowResolution,
    pub shadow_detail: ShadowDetail,
    pub sdsm: bool,
    pub high_quality_interface: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Windowed,
            lighting_mode: LightingMode::Enhanced,
            sprite_lighting_mode: SpriteLightingMode::default(),
            vsync: true,
            limit_framerate: LimitFramerate::Unlimited,
            triple_buffering: true,
            texture_filtering: TextureSamplerType::Anisotropic(4),
            msaa: Msaa::X4,
            ssaa: Ssaa::Off,
            screen_space_anti_aliasing: ScreenSpaceAntiAliasing::Off,
            shadow_method: ShadowMethod::SoftPCSS,
            shadow_resolution: ShadowResolution::Normal,
            shadow_detail: ShadowDetail::Medium,
            sdsm: true,
            high_quality_interface: true,
        }
    }
}

impl GraphicsSettings {
    const FILE_NAME: &'static str = "client/graphics_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load graphics settings from {}", Self::FILE_NAME.magenta());

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading graphics settings from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving graphics settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save graphics settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for GraphicsSettings {
    fn drop(&mut self) {
        self.save();
    }
}

/// The lighting mode used when rendering the game.
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, StateElement)]
pub enum LightingMode {
    /// Mode that mimics the way the original client rendered the game.
    Classic,
    /// Mode that enabled all enhanced graphics features.
    Enhanced,
}

impl DropDownItem<LightingMode> for LightingMode {
    fn text(&self) -> &str {
        match self {
            LightingMode::Classic => "Classic",
            LightingMode::Enhanced => "Enhanced",
        }
    }

    fn value(&self) -> LightingMode {
        *self
    }
}

/// How the game window covers the screen.
#[derive(Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize, StateElement)]
pub enum DisplayMode {
    /// A regular resizable window.
    #[default]
    Windowed,
    /// A borderless window the size of the monitor. Keeps the desktop
    /// resolution, so alt-tabbing away costs nothing and no other window gets
    /// rearranged.
    BorderlessFullscreen,
    /// Takes exclusive control of the monitor. Same resolution the desktop is
    /// already at, at the highest refresh rate the monitor offers for it.
    ExclusiveFullscreen,
}

impl DropDownItem<DisplayMode> for DisplayMode {
    fn text(&self) -> &str {
        match self {
            DisplayMode::Windowed => "Windowed",
            DisplayMode::BorderlessFullscreen => "Windowed Fullscreen",
            DisplayMode::ExclusiveFullscreen => "Exclusive Fullscreen",
        }
    }

    fn value(&self) -> DisplayMode {
        *self
    }
}

#[derive(RustState, StateElement)]
pub struct GraphicsSettingsCapabilities {
    display_modes: Vec<DisplayMode>,
    lighting_modes: Vec<LightingMode>,
    sprite_lighting_modes: Vec<SpriteLightingMode>,
    texture_filtering_options: Vec<TextureSamplerType>,
    limit_framerate_options: Vec<LimitFramerate>,
    supported_msaa: Vec<Msaa>,
    ssaa_options: Vec<Ssaa>,
    screen_space_anti_aliasing_options: Vec<ScreenSpaceAntiAliasing>,
    shadow_method_options: Vec<ShadowMethod>,
    shadow_resolution_options: Vec<ShadowResolution>,
    shadow_detail_options: Vec<ShadowDetail>,
    vsync_setting_disabled: bool,
}

impl Default for GraphicsSettingsCapabilities {
    fn default() -> Self {
        Self {
            display_modes: vec![
                DisplayMode::Windowed,
                DisplayMode::BorderlessFullscreen,
                DisplayMode::ExclusiveFullscreen,
            ],
            lighting_modes: vec![LightingMode::Classic, LightingMode::Enhanced],
            sprite_lighting_modes: vec![SpriteLightingMode::Classic, SpriteLightingMode::Soft, SpriteLightingMode::Enhanced],
            texture_filtering_options: vec![
                TextureSamplerType::Nearest,
                TextureSamplerType::Linear,
                TextureSamplerType::Anisotropic(4),
                TextureSamplerType::Anisotropic(8),
                TextureSamplerType::Anisotropic(16),
            ],
            limit_framerate_options: vec![
                LimitFramerate::Unlimited,
                LimitFramerate::Limit(30),
                LimitFramerate::Limit(60),
                LimitFramerate::Limit(120),
                LimitFramerate::Limit(144),
                LimitFramerate::Limit(240),
            ],
            supported_msaa: Vec::new(),
            ssaa_options: vec![Ssaa::Off, Ssaa::X2, Ssaa::X3, Ssaa::X4],
            screen_space_anti_aliasing_options: vec![ScreenSpaceAntiAliasing::Off, ScreenSpaceAntiAliasing::Fxaa],
            shadow_method_options: vec![ShadowMethod::Hard, ShadowMethod::SoftPCF, ShadowMethod::SoftPCSS],
            shadow_resolution_options: vec![ShadowResolution::Normal, ShadowResolution::Ultra, ShadowResolution::Insane],
            shadow_detail_options: vec![ShadowDetail::Low, ShadowDetail::Medium, ShadowDetail::High, ShadowDetail::Ultra],
            vsync_setting_disabled: true,
        }
    }
}

impl GraphicsSettingsCapabilities {
    pub fn update(&mut self, supported_msaa: Vec<Msaa>, present_mode_info: PresentModeInfo) {
        self.supported_msaa = supported_msaa;
        self.vsync_setting_disabled = !present_mode_info.supports_mailbox && !present_mode_info.supports_immediate;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A settings file written before `display_mode` existed, copied from the
    /// shape `client/graphics_settings.ron` had on disk at the time.
    const SETTINGS_WITHOUT_DISPLAY_MODE: &str = r#"(
    lighting_mode: Classic,
    sprite_lighting_mode: Soft,
    vsync: false,
    limit_framerate: Limit(144),
    triple_buffering: false,
    texture_filtering: Nearest,
    msaa: Off,
    ssaa: X2,
    screen_space_anti_aliasing: Fxaa,
    shadow_method: Hard,
    shadow_resolution: Ultra,
    shadow_detail: Low,
    sdsm: false,
    high_quality_interface: false,
)"#;

    /// `load` turns any parse failure into `Default::default()`, so a field
    /// added without `serde(default)` would not fail loudly — it would silently
    /// hand the player back a pristine settings file and throw away every
    /// choice they had made. Assert on a neighbouring setting, not just on the
    /// new one, because that is the damage worth catching.
    #[test]
    fn settings_file_without_display_mode_keeps_its_other_settings() {
        let settings: GraphicsSettings = ron::from_str(SETTINGS_WITHOUT_DISPLAY_MODE).expect("older settings file should still parse");

        // `matches!` rather than `assert_eq!`: these settings enums do not derive
        // `Debug`, and this test is not a reason to change their public shape.
        assert!(matches!(settings.display_mode, DisplayMode::Windowed));
        assert!(matches!(settings.shadow_resolution, ShadowResolution::Ultra));
        assert!(matches!(settings.ssaa, Ssaa::X2));
        assert!(!settings.vsync);
        assert!(!settings.high_quality_interface);

        // `GraphicsSettings` saves itself on drop, and `save` writes to a fixed
        // relative path. `cargo test` runs with the crate root as the working
        // directory, so letting this value drop would overwrite the developer's
        // real `client/graphics_settings.ron` with the fixture above — which is
        // exactly what happened the first time this test ran.
        std::mem::forget(settings);
    }
}
