//! Typed presentation recipes for persistent skill units (Phase E2).
//!
//! A unit exists from `AddSkillUnit` until `RemoveSkillUnit` — the server owns
//! its lifetime completely. This table only declares what a live unit looks
//! like. Mappings come from the reverse-engineered original-client tables
//! (unit ID → effect ID → assets); see
//! `docs/plans/classic-effect-fidelity.md` for sourcing and verification.

use ragnarok_packets::UnitId;

use crate::Color;
use crate::world::UnitCylinderSpec;

/// Procedural persistent geometry families.
pub enum UnitBody {
    /// Layered rotating textured cylinders (Safety Wall, portals, Sanctuary,
    /// Magnus, Fire Pillar) — the original's dominant unit primitive.
    Cylinders {
        texture: &'static str,
        specs: &'static [UnitCylinderSpec],
        color: Color,
    },
    /// Ice Wall's per-cell cluster of ice horns.
    IceHorns { texture: &'static str },
}

/// Everything a unit spawn plays and keeps alive.
pub struct UnitPresentation {
    /// One-shot STR at spawn (Safety Wall's glass flash). Self-expiring.
    pub intro_str: Option<&'static str>,
    /// STR looped for the unit's whole lifetime (Fire Wall, Pneuma, Quagmire).
    pub looping_str: Option<&'static str>,
    /// Procedural persistent geometry.
    pub body: Option<UnitBody>,
    /// One-shot spawn sound.
    pub sound: Option<&'static str>,
    /// Steady point light while the unit lives.
    pub light: Option<(Color, f32)>,
}

const NONE: UnitPresentation = UnitPresentation {
    intro_str: None,
    looping_str: None,
    body: None,
    sound: None,
    light: None,
};

// Cylinder layouts, sized from the original's cell units (×5.0 world units per
// cell), heights eyeballed against the existing PortalVortex scale.

/// Pre-warp ("ready") portal: one flat pulsing floor ring.
const WARP_WAITING_CYLINDERS: &[UnitCylinderSpec] = &[UnitCylinderSpec {
    bottom_radius: 12.0,
    top_radius: 19.5,
    height: 0.6,
    spin_speed: 2.0,
    sides: 20,
    alpha: 0.7,
    yaw: 0.0,
}];

/// Active warp portal: the classic swirling vortex plus the floor ring.
const WARP_ACTIVE_CYLINDERS: &[UnitCylinderSpec] = &[
    UnitCylinderSpec {
        bottom_radius: 12.0,
        top_radius: 19.5,
        height: 0.6,
        spin_speed: 2.0,
        sides: 20,
        alpha: 0.6,
        yaw: 0.0,
    },
    UnitCylinderSpec {
        bottom_radius: 6.0,
        top_radius: 5.0,
        height: 16.0,
        spin_speed: 4.4,
        sides: 20,
        alpha: 0.8,
        yaw: 0.0,
    },
    UnitCylinderSpec {
        bottom_radius: 3.6,
        top_radius: 3.0,
        height: 24.0,
        spin_speed: -3.2,
        sides: 20,
        alpha: 0.7,
        yaw: 0.0,
    },
];

/// Sanctuary map unit: low square glow, yawed 45° like the original.
/// Brightened + second shimmer layer after live feedback 2026-07-23.
const SANCTUARY_CYLINDERS: &[UnitCylinderSpec] = &[
    UnitCylinderSpec {
        bottom_radius: 3.5,
        top_radius: 3.5,
        height: 6.0,
        spin_speed: 0.0,
        sides: 4,
        alpha: 0.55,
        yaw: std::f32::consts::FRAC_PI_4,
    },
    UnitCylinderSpec {
        bottom_radius: 3.1,
        top_radius: 3.1,
        height: 7.5,
        spin_speed: 0.5,
        sides: 4,
        alpha: 0.35,
        yaw: std::f32::consts::FRAC_PI_4,
    },
];

/// Magnus Exorcismus map unit: taller square pillar, red.
/// Brightened + second shimmer layer after live feedback 2026-07-23.
const MAGNUS_CYLINDERS: &[UnitCylinderSpec] = &[
    UnitCylinderSpec {
        bottom_radius: 3.5,
        top_radius: 3.5,
        height: 12.0,
        spin_speed: 0.0,
        sides: 4,
        alpha: 0.5,
        yaw: std::f32::consts::FRAC_PI_4,
    },
    UnitCylinderSpec {
        bottom_radius: 3.1,
        top_radius: 3.1,
        height: 14.0,
        spin_speed: -0.5,
        sides: 4,
        alpha: 0.35,
        yaw: std::f32::consts::FRAC_PI_4,
    },
];

/// Pneuma covers 3x3 cells; the looping STR alone read as a small blip, so a
/// wide, slow, soft dome marks the real protected footprint.
const PNEUMA_CYLINDERS: &[UnitCylinderSpec] = &[UnitCylinderSpec {
    bottom_radius: 7.5,
    top_radius: 5.5,
    height: 3.0,
    spin_speed: 0.6,
    sides: 20,
    alpha: 0.4,
    yaw: 0.0,
}];

/// Fire Pillar (armed, waiting): three nested widening red swirls.
/// Brightened after live feedback 2026-07-23 (0.35 alpha read too faint).
const FIRE_PILLAR_CYLINDERS: &[UnitCylinderSpec] = &[
    UnitCylinderSpec {
        bottom_radius: 5.0,
        top_radius: 10.0,
        height: 9.0,
        spin_speed: 2.2,
        sides: 20,
        alpha: 0.6,
        yaw: 0.0,
    },
    UnitCylinderSpec {
        bottom_radius: 3.5,
        top_radius: 7.5,
        height: 15.0,
        spin_speed: -1.7,
        sides: 20,
        alpha: 0.55,
        yaw: 0.0,
    },
    UnitCylinderSpec {
        bottom_radius: 2.5,
        top_radius: 5.0,
        height: 21.0,
        spin_speed: 2.6,
        sides: 20,
        alpha: 0.5,
        yaw: 0.0,
    },
];

/// Resolve a unit ID to its presentation. `None` keeps the explicit empty
/// contract — unmapped units draw nothing rather than guessing.
pub fn unit_presentation(unit_id: UnitId) -> Option<UnitPresentation> {
    match unit_id {
        UnitId::Safetywall => Some(UnitPresentation {
            // Loop Gravity's authored wall animation for the unit's lifetime,
            // exactly like Fire Wall does — procedural pyramids/cylinders
            // read as flat light blobs live (2026-07-23).
            looping_str: Some("safetywall.str"),
            sound: Some("effect\\ef_glasswall.wav"),
            light: Some((Color::rgb_u8(200, 90, 220), 42.0)),
            ..NONE
        }),
        UnitId::Firewall => Some(UnitPresentation {
            looping_str: Some("firewall.str"),
            sound: Some("effect\\ef_firewall.wav"),
            light: Some((Color::rgb_u8(255, 30, 0), 60.0)),
            ..NONE
        }),
        UnitId::Pneuma => Some(UnitPresentation {
            looping_str: Some("pneuma1.str"),
            body: Some(UnitBody::Cylinders {
                texture: "effect\\alpha_down.tga",
                specs: PNEUMA_CYLINDERS,
                color: Color::rgb_u8(170, 235, 255),
            }),
            light: Some((Color::rgb_u8(120, 230, 170), 60.0)),
            ..NONE
        }),
        UnitId::WarpWaiting => Some(UnitPresentation {
            body: Some(UnitBody::Cylinders {
                texture: "effect\\ring_blue.tga",
                specs: WARP_WAITING_CYLINDERS,
                color: Color::rgb_u8(150, 150, 255),
            }),
            sound: Some("effect\\ef_readyportal.wav"),
            light: Some((Color::rgb_u8(120, 140, 255), 35.0)),
            ..NONE
        }),
        UnitId::WarpActive => Some(UnitPresentation {
            body: Some(UnitBody::Cylinders {
                texture: "effect\\ring_blue.tga",
                specs: WARP_ACTIVE_CYLINDERS,
                color: Color::rgb_u8(150, 150, 255),
            }),
            sound: Some("effect\\ef_portal.wav"),
            light: Some((Color::rgb_u8(120, 140, 255), 55.0)),
            ..NONE
        }),
        UnitId::Sanctuary => Some(UnitPresentation {
            body: Some(UnitBody::Cylinders {
                texture: "effect\\magic_green.tga",
                specs: SANCTUARY_CYLINDERS,
                color: Color::rgb_u8(130, 230, 130),
            }),
            light: Some((Color::rgb_u8(130, 255, 175), 50.0)),
            ..NONE
        }),
        UnitId::Magnus => Some(UnitPresentation {
            body: Some(UnitBody::Cylinders {
                texture: "effect\\ring_red.tga",
                specs: MAGNUS_CYLINDERS,
                color: Color::rgb_u8(255, 110, 90),
            }),
            light: Some((Color::rgb_u8(255, 150, 110), 50.0)),
            ..NONE
        }),
        UnitId::FirepillarWaiting => Some(UnitPresentation {
            body: Some(UnitBody::Cylinders {
                texture: "effect\\magic_red.tga",
                specs: FIRE_PILLAR_CYLINDERS,
                color: Color::rgb_u8(255, 150, 80),
            }),
            light: Some((Color::rgb_u8(255, 80, 20), 40.0)),
            ..NONE
        }),
        UnitId::Icewall => Some(UnitPresentation {
            body: Some(UnitBody::IceHorns {
                texture: "effect\\ice.tga",
            }),
            sound: Some("effect\\wizard_icewall.wav"),
            light: Some((Color::rgb_u8(180, 230, 255), 32.0)),
            ..NONE
        }),
        UnitId::Quagmire => Some(UnitPresentation {
            looping_str: Some("quagmire.str"),
            sound: Some("effect\\wizard_quagmire.wav"),
            // Brighter after live feedback — the field read too faint.
            light: Some((Color::rgb_u8(150, 120, 70), 38.0)),
            ..NONE
        }),
        _ => None,
    }
}

/// Units with a wired presentation, for audits.
#[cfg(test)]
pub const MAPPED_UNIT_IDS: &[UnitId] = &[
    UnitId::Safetywall,
    UnitId::Firewall,
    UnitId::Pneuma,
    UnitId::WarpWaiting,
    UnitId::WarpActive,
    UnitId::Sanctuary,
    UnitId::Magnus,
    UnitId::FirepillarWaiting,
    UnitId::Icewall,
    UnitId::Quagmire,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mapped_unit_has_a_visible_presentation() {
        for unit_id in MAPPED_UNIT_IDS {
            let presentation = unit_presentation(*unit_id).expect("mapped unit must resolve");
            assert!(
                presentation.looping_str.is_some() || presentation.body.is_some(),
                "unit {unit_id:?} has no persistent visual"
            );
        }
    }

    #[test]
    fn unmapped_units_stay_explicitly_empty() {
        assert!(unit_presentation(UnitId::Graffiti).is_none());
    }

    #[test]
    fn preexisting_units_keep_their_presentation() {
        // Firewall and Pneuma predate this table; their look must not regress.
        let firewall = unit_presentation(UnitId::Firewall).unwrap();
        assert_eq!(firewall.looping_str, Some("firewall.str"));
        let pneuma = unit_presentation(UnitId::Pneuma).unwrap();
        assert_eq!(pneuma.looping_str, Some("pneuma1.str"));
    }
}
