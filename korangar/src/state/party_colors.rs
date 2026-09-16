//! Stable party minimap colors from membership order.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

const DEFAULTS: [Rgb; 6] = [
    Rgb(70, 160, 255),
    Rgb(255, 170, 40),
    Rgb(80, 220, 120),
    Rgb(220, 80, 200),
    Rgb(255, 90, 90),
    Rgb(180, 220, 255),
];

pub fn color_for_index(index: usize) -> Rgb {
    DEFAULTS[index % DEFAULTS.len()]
}

pub fn contrast_ok(color: Rgb) -> bool {
    let lum = 0.299 * f32::from(color.0) + 0.587 * f32::from(color.1) + 0.114 * f32::from(color.2);
    lum >= 40.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_is_stable_across_reconnect() {
        let first = [color_for_index(0), color_for_index(1)];
        let again = [color_for_index(0), color_for_index(1)];
        assert_eq!(first, again);
        assert_ne!(color_for_index(0), color_for_index(1));
        assert!(contrast_ok(Rgb(255, 255, 255)));
        assert!(!contrast_ok(Rgb(0, 0, 0)));
    }
}
