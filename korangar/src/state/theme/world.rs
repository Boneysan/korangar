#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_interface::element::StateElement;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{Color, ScreenPosition, ScreenSize};
use crate::loaders::FontSize;

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct OverlayTheme {
    pub foreground_color: Color,
    pub text_offset: ScreenPosition,
    pub font_size: FontSize,
}

impl Default for OverlayTheme {
    fn default() -> Self {
        Self {
            foreground_color: Color::monochrome_u8(220),
            text_offset: ScreenPosition { left: 20.0, top: 10.0 },
            font_size: FontSize(24.0),
        }
    }
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
#[serde(default)]
pub struct StatusBarTheme {
    pub background_color: Color,
    pub player_health_color: Color,
    /// Health for other player/party entities shown over the world.
    pub ally_health_color: Color,
    pub enemy_health_color: Color,
    pub spell_point_color: Color,
    pub activity_point_color: Color,
    /// Cast progress for other player/party entities shown over the world.
    pub cast_bar_color: Color,
    pub player_bar_width: f32,
    pub enemy_bar_width: f32,
    pub health_height: f32,
    pub enemy_health_height: f32,
    pub spell_point_height: f32,
    pub activity_point_height: f32,
    pub border_size: ScreenSize,
    pub gap: f32,
}

impl Default for StatusBarTheme {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(40),
            player_health_color: Color::rgb_u8(67, 163, 83),
            ally_health_color: Color::rgb_u8(80, 220, 120),
            enemy_health_color: Color::rgb_u8(206, 49, 116),
            spell_point_color: Color::rgb_u8(0, 129, 163),
            activity_point_color: Color::rgb_u8(218, 145, 81),
            cast_bar_color: Color::rgb_u8(255, 210, 60),
            player_bar_width: 85.0,
            enemy_bar_width: 60.0,
            health_height: 8.0,
            enemy_health_height: 6.0,
            spell_point_height: 4.0,
            activity_point_height: 4.0,
            border_size: ScreenSize { width: 2.0, height: 1.0 },
            gap: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct IndicatorTheme {
    pub walking: Color,
}

impl Default for IndicatorTheme {
    fn default() -> Self {
        Self {
            walking: Color::rgba_u8(0, 255, 170, 170),
        }
    }
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct CursorTheme {
    pub color: Color,
}

impl Default for CursorTheme {
    fn default() -> Self {
        Self {
            color: Color::monochrome_u8(255),
        }
    }
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
#[serde(default)]
pub struct WorldTheme {
    pub overlay: OverlayTheme,
    pub status_bar: StatusBarTheme,
    pub indicator: IndicatorTheme,
    pub cursor: CursorTheme,
    /// Local ground-target preview color while the target is in range.
    pub skill_aim_in_range: Color,
    /// Local ground-target preview color while the target is out of range.
    pub skill_aim_out_of_range: Color,
    /// Server cast telegraph color for hostile/incoming ground effects.
    pub enemy_telegraph: Color,
}

impl Default for WorldTheme {
    fn default() -> Self {
        Self {
            overlay: OverlayTheme::default(),
            status_bar: StatusBarTheme::default(),
            indicator: IndicatorTheme::default(),
            cursor: CursorTheme::default(),
            skill_aim_in_range: Color::rgba(0.35, 0.75, 1.0, 0.5),
            skill_aim_out_of_range: Color::rgba(1.0, 0.3, 0.25, 0.5),
            enemy_telegraph: Color::rgba(1.0, 0.28, 0.12, 0.48),
        }
    }
}

impl WorldTheme {
    pub fn load(name: &str) -> Self {
        use crate::settings::{DEUTERANOPIA_THEME_NAME, WORLD_THEMES_PATH};

        #[cfg(feature = "debug")]
        let timer = Timer::new("Load theme");

        if name == DEUTERANOPIA_THEME_NAME {
            let mut theme = Self::default();
            theme.status_bar.player_health_color = Color::rgb_u8(0, 114, 178);
            theme.status_bar.ally_health_color = Color::rgb_u8(0, 114, 178);
            theme.status_bar.enemy_health_color = Color::rgb_u8(213, 94, 0);
            theme.status_bar.spell_point_color = Color::rgb_u8(86, 180, 233);
            theme.status_bar.activity_point_color = Color::rgb_u8(230, 159, 0);
            theme.status_bar.cast_bar_color = Color::rgb_u8(230, 159, 0);
            theme.indicator.walking = Color::rgba_u8(0, 114, 178, 240);
            theme.cursor.color = Color::rgb_u8(230, 159, 0);
            theme.skill_aim_in_range = Color::rgba_u8(0, 114, 178, 190);
            theme.skill_aim_out_of_range = Color::rgba_u8(230, 159, 0, 210);
            theme.enemy_telegraph = Color::rgba_u8(213, 94, 0, 215);
            #[cfg(feature = "debug")]
            timer.stop();
            return theme;
        }

        let path = format!("{WORLD_THEMES_PATH}/{name}.ron");

        #[cfg(feature = "debug")]
        print_debug!("loading theme from file {}", path.magenta());

        let theme = std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .unwrap_or_else(|| {
                #[cfg(feature = "debug")]
                print_debug!("[{}] failed to load theme {}", "error".red(), name.magenta());
                WorldTheme::default()
            });

        #[cfg(feature = "debug")]
        timer.stop();

        theme
    }

    #[cfg(feature = "debug")]
    pub fn save(&self, name: &str) {
        use crate::settings::WORLD_THEMES_PATH;

        let timer = Timer::new("Save theme");

        let path = format!("{WORLD_THEMES_PATH}/{name}.ron");

        print_debug!("saving theme to file {}", path.magenta());

        let data = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new()).unwrap();

        if let Err(error) = std::fs::write(path, data) {
            print_debug!(
                "[{}] failed to save theme to {}: {:?}",
                "error".red(),
                name.magenta(),
                error.red()
            );
        }

        timer.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::{StatusBarTheme, WorldTheme};
    use crate::graphics::Color;
    use crate::settings::DEUTERANOPIA_THEME_NAME;

    #[test]
    fn deuteranopia_world_theme_uses_blue_orange_and_labeled_bars() {
        let theme = WorldTheme::load(DEUTERANOPIA_THEME_NAME);
        assert_eq!(theme.status_bar.player_health_color, Color::rgb_u8(0, 114, 178));
        assert_eq!(theme.status_bar.enemy_health_color, Color::rgb_u8(213, 94, 0));
        assert_eq!(theme.status_bar.ally_health_color, Color::rgb_u8(0, 114, 178));
        assert_eq!(theme.status_bar.activity_point_color, Color::rgb_u8(230, 159, 0));
        assert_eq!(theme.status_bar.cast_bar_color, Color::rgb_u8(230, 159, 0));
        assert_eq!(theme.cursor.color, Color::rgb_u8(230, 159, 0));
        assert_eq!(theme.skill_aim_in_range, Color::rgba_u8(0, 114, 178, 190));
        assert_eq!(theme.enemy_telegraph, Color::rgba_u8(213, 94, 0, 215));
    }

    #[test]
    fn older_world_themes_default_new_telegraph_colors() {
        let old_theme: WorldTheme = ron::from_str("()").expect("old empty world theme remains loadable");
        assert_eq!(old_theme.skill_aim_in_range, Color::rgba(0.35, 0.75, 1.0, 0.5));
        assert_eq!(old_theme.skill_aim_out_of_range, Color::rgba(1.0, 0.3, 0.25, 0.5));
        assert_eq!(old_theme.enemy_telegraph, Color::rgba(1.0, 0.28, 0.12, 0.48));
        assert_eq!(old_theme.status_bar.ally_health_color, Color::rgb_u8(80, 220, 120));
        assert_eq!(old_theme.status_bar.cast_bar_color, Color::rgb_u8(255, 210, 60));

        let old_status_bar: StatusBarTheme = ron::from_str("()").expect("old status bar themes remain loadable");
        assert_eq!(old_status_bar.ally_health_color, Color::rgb_u8(80, 220, 120));
        assert_eq!(old_status_bar.cast_bar_color, Color::rgb_u8(255, 210, 60));
    }
}
