use cgmath::{Deg, Matrix3, Vector3};
use korangar_interface::element::StateElement;
use ragnarok_formats::map::LightSettings;
use rust_state::RustState;

use crate::graphics::Color;

#[derive(RustState, StateElement)]
pub struct Lighting {
    ambient_color: Color,
    diffuse_color: Color,
    light_latitude: f32,
    light_longitude: f32,
}

impl Lighting {
    pub fn new(settings: LightSettings) -> Self {
        Self {
            ambient_color: settings.ambient_color.unwrap().into(),
            diffuse_color: settings.diffuse_color.unwrap().into(),
            light_latitude: settings.light_latitude.unwrap() as f32,
            light_longitude: settings.light_longitude.unwrap() as f32,
        }
    }

    pub fn ambient_light_color(&self) -> Color {
        self.ambient_color
    }

    /// Whether the map is dark enough to want the light that follows the
    /// player. See [`is_dark_lighting`].
    pub fn is_dark(&self) -> bool {
        is_dark_lighting(self.ambient_color, self.diffuse_color)
    }

    pub fn directional_light(&self) -> (Vector3<f32>, Color) {
        let rotation_around_x = Matrix3::from_angle_x(Deg(-self.light_latitude));
        let rotation_around_y = Matrix3::from_angle_y(Deg(self.light_longitude));
        let light_direction = rotation_around_y * (rotation_around_x * Vector3::new(0.0, 1.0, 0.0));

        (light_direction, self.diffuse_color)
    }
}

/// Brightness at or below which a map counts as dark: the luminance of its
/// ambient light plus its sun (diffuse) light, as stored in the map's RSW.
///
/// Measured 2026-10-05 over all 882 server maps with an RSW in the archives.
/// The values run continuously from 0.3 to 2.0, so no cutoff is clean; 1.0
/// takes the classic dungeons and caves (Payon cave 0.60, Orc dungeon 0.56,
/// sewers 0.57, Glast Heim church 0.69, Geffen dungeon 0.91-0.99) and leaves
/// every town and most fields out (Prontera 1.51, prt_fild05 1.30). Some
/// brighter dungeons (Moscovia, Niflheim, Ozkas) stay unlit by choice.
const DARK_MAP_BRIGHTNESS: f32 = 1.0;

fn luminance(color: Color) -> f32 {
    0.2126 * color.red + 0.7152 * color.green + 0.0722 * color.blue
}

pub fn is_dark_lighting(ambient: Color, diffuse: Color) -> bool {
    luminance(ambient) + luminance(diffuse) <= DARK_MAP_BRIGHTNESS + f32::EPSILON
}

#[cfg(test)]
mod tests {
    use super::is_dark_lighting;
    use crate::graphics::Color;

    fn rgb(red: f32, green: f32, blue: f32) -> Color {
        Color::rgb(red, green, blue)
    }

    #[test]
    fn dungeons_are_dark_and_towns_and_fields_are_not() {
        // RSW lighting of real maps: (ambient, diffuse).
        assert!(is_dark_lighting(rgb(0.3, 0.3, 0.3), rgb(0.3, 0.3, 0.3)), "pay_dun01");
        assert!(is_dark_lighting(rgb(0.2, 0.15, 0.2), rgb(0.4, 0.4, 0.5)), "prt_sewb1");
        assert!(is_dark_lighting(rgb(0.3, 0.5, 0.7), rgb(0.6, 0.5, 0.5)), "gef_dun01, 0.99");
        assert!(!is_dark_lighting(rgb(0.55, 0.5, 0.5), rgb(1.0, 1.0, 1.0)), "prontera");
        assert!(!is_dark_lighting(rgb(0.3, 0.3, 0.3), rgb(1.0, 1.0, 1.0)), "prt_fild05");
        assert!(
            !is_dark_lighting(rgb(0.23, 0.25, 0.3), rgb(0.85, 0.75, 0.75)),
            "gef_fild05, 1.02"
        );
    }
}
