//! Typed presentation recipes for persistent skill units (Phase E2).
//!
//! A unit exists from `AddSkillUnit` until `RemoveSkillUnit` — the server owns
//! its lifetime completely. This table only declares what a live unit looks
//! like. Mappings come from the reverse-engineered original-client tables
//! (unit ID → effect ID → assets); see
//! `docs/plans/classic-effect-fidelity.md` for sourcing and verification.

use ragnarok_packets::UnitId;

use crate::Color;
use crate::world::{UnitCylinderSpec, UnitPulse};

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
    /// Land Protector's flat pulsing floor tile (`LPEffect`), one per cell.
    GroundQuad {
        texture: &'static str,
        /// Half-width in world units.
        half_size: f32,
        color: Color,
        pulse: UnitPulse,
    },
    /// A `data\sprite\이팩트\*` animation repeating for the unit's lifetime
    /// (Venom Dust, Demonstration) — these have no `.str` equivalent.
    LoopingSprite {
        path: &'static str,
        action_index: usize,
        /// Lift off the cell; ACT frames are bottom-normalized.
        lift: f32,
    },
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
    pulse: None,
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
        pulse: None,
    },
    UnitCylinderSpec {
        bottom_radius: 6.0,
        top_radius: 5.0,
        height: 16.0,
        spin_speed: 4.4,
        sides: 20,
        alpha: 0.8,
        yaw: 0.0,
        pulse: None,
    },
    UnitCylinderSpec {
        bottom_radius: 3.6,
        top_radius: 3.0,
        height: 24.0,
        spin_speed: -3.2,
        sides: 20,
        alpha: 0.7,
        yaw: 0.0,
        pulse: None,
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
        pulse: None,
    },
    UnitCylinderSpec {
        bottom_radius: 3.1,
        top_radius: 3.1,
        height: 7.5,
        spin_speed: 0.5,
        sides: 4,
        alpha: 0.35,
        yaw: std::f32::consts::FRAC_PI_4,
        pulse: None,
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
        pulse: None,
    },
    UnitCylinderSpec {
        bottom_radius: 3.1,
        top_radius: 3.1,
        height: 14.0,
        spin_speed: -0.5,
        sides: 4,
        alpha: 0.35,
        yaw: std::f32::consts::FRAC_PI_4,
        pulse: None,
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
    pulse: None,
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
        pulse: None,
    },
    UnitCylinderSpec {
        bottom_radius: 3.5,
        top_radius: 7.5,
        height: 15.0,
        spin_speed: -1.7,
        sides: 20,
        alpha: 0.55,
        yaw: 0.0,
        pulse: None,
    },
    UnitCylinderSpec {
        bottom_radius: 2.5,
        top_radius: 5.0,
        height: 21.0,
        spin_speed: 2.6,
        sides: 20,
        alpha: 0.5,
        yaw: 0.0,
        pulse: None,
    },
];

/// The elemental field units (Volcano / Deluge / Violent Gale) share one
/// `PropertyGround` shape and differ only by ring texture and tint: a rotating
/// truncated cone flaring upward, breathing between half and full size.
///
/// **Sized per cell, not per field.** Hercules `Layout: 3` is a 7×7 square, so
/// the server sends 49 `AddSkillUnit` packets and this cone is drawn 49 times.
/// Reading the effect table's `top 3.0 / bottom 1.0` as *cells* made each cone
/// 6 cells wide and the field rendered as one solid block of fire (live,
/// 2026-07-24). Those numbers are already in the effect's own world units —
/// the same scale as the live-verified Sanctuary/Magnus specs above.
///
/// The top radius slightly exceeds the 5.0-unit cell pitch so neighbors merge
/// into a continuous field; `alpha` is correspondingly low because ~4 additive
/// surfaces stack at any given point.
const ELEMENTAL_FIELD_CYLINDERS: &[UnitCylinderSpec] = &[UnitCylinderSpec {
    bottom_radius: 2.0,
    // Full cell pitch: neighbors just touch, so the field reads as one
    // continuous blaze (user asked for bigger after the first live look,
    // 2026-07-24) while each cone still reads individually at the edges.
    top_radius: 5.0,
    height: 10.0,
    spin_speed: 1.5,
    // 12 sides still reads round at this radius and costs 40% fewer quads
    // than 20 — it is paid 49× per field.
    sides: 12,
    alpha: 0.45,
    yaw: 0.0,
    pulse: Some(UnitPulse {
        min_scale: 0.5,
        max_scale: 1.0,
        speed: 2.0,
    }),
}];

/// Land Protector: one low square glow per cell, yawed 45° like the other
/// square map units. Deliberately short — it marks the floor, and 121 of these
/// overlap at Lv5, so the alpha is the lowest in the table.
const LAND_PROTECTOR_CYLINDERS: &[UnitCylinderSpec] = &[UnitCylinderSpec {
    bottom_radius: 2.5,
    top_radius: 2.5,
    height: 3.0,
    spin_speed: 0.0,
    // Raised from 0.35 live (2026-07-24) — even 121 stacked cells read too
    // faint. The "ground effects want more brightness than theory suggests"
    // lesson yet again.
    sides: 4,
    alpha: 0.6,
    yaw: std::f32::consts::FRAC_PI_4,
    // Deeper breath too: 0.85–1.0 was barely perceptible.
    pulse: Some(UnitPulse {
        min_scale: 0.65,
        max_scale: 1.0,
        speed: 3.0,
    }),
}];

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
        // --- Batch 2 ---
        UnitId::Volcano => Some(UnitPresentation {
            body: Some(UnitBody::Cylinders {
                texture: "effect\\ring_red.tga",
                specs: ELEMENTAL_FIELD_CYLINDERS,
                color: Color::rgb_u8(255, 120, 60),
            }),
            // Dim per cell: 49 of these overlap across the field.
            light: Some((Color::rgb_u8(255, 90, 30), 22.0)),
            ..NONE
        }),
        UnitId::Deluge => Some(UnitPresentation {
            body: Some(UnitBody::Cylinders {
                texture: "effect\\ring_blue.tga",
                specs: ELEMENTAL_FIELD_CYLINDERS,
                color: Color::rgb_u8(110, 170, 255),
            }),
            light: Some((Color::rgb_u8(80, 150, 255), 22.0)),
            ..NONE
        }),
        UnitId::Violentgale => Some(UnitPresentation {
            body: Some(UnitBody::Cylinders {
                texture: "effect\\ring_yellow.tga",
                specs: ELEMENTAL_FIELD_CYLINDERS,
                color: Color::rgb_u8(220, 255, 150),
            }),
            light: Some((Color::rgb_u8(200, 255, 130), 22.0)),
            ..NONE
        }),
        UnitId::Landprotector => Some(UnitPresentation {
            // The original's `LPEffect` is a flat floor tile per cell, and
            // `UnitGroundQuad` draws exactly that — but effects composite in a
            // depth-less post-processing pass, so a ground-parallel quad lands
            // on top of the player sprite (live, 2026-07-24). Until there is a
            // depth-tested ground-decal pass, use the low square glow that
            // Sanctuary/Magnus already passed live with: it still overlays,
            // but reads as light on the floor rather than a sticker on the
            // character. `UnitGroundQuad` stays wired for that future pass.
            body: Some(UnitBody::Cylinders {
                texture: "effect\\aaa copy.bmp",
                specs: LAND_PROTECTOR_CYLINDERS,
                color: Color::rgb_u8(190, 230, 255),
            }),
            // Land Protector is 11×11 cells at Lv5 (15×15 at Lv10), so the
            // per-cell light stays modest — but 12.0 was invisible live.
            light: Some((Color::rgb_u8(150, 200, 255), 26.0)),
            ..NONE
        }),
        UnitId::Venomdust => Some(UnitPresentation {
            // Effect 171: `particle3` rising at the cell, `repeat: true`.
            body: Some(UnitBody::LoopingSprite {
                path: "이팩트\\particle3",
                action_index: 0,
                lift: 4.0,
            }),
            light: Some((Color::rgb_u8(150, 90, 200), 28.0)),
            ..NONE
        }),
        UnitId::Demonstration => Some(UnitPresentation {
            // Effect 302: the Alchemist bomb's own sprite, looped at the cell.
            body: Some(UnitBody::LoopingSprite {
                path: "이팩트\\데몬스트레이션",
                action_index: 0,
                lift: 4.0,
            }),
            light: Some((Color::rgb_u8(255, 120, 40), 38.0)),
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
    UnitId::Volcano,
    UnitId::Deluge,
    UnitId::Violentgale,
    UnitId::Landprotector,
    UnitId::Venomdust,
    UnitId::Demonstration,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loaders::GAT_TILE_SIZE;

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
    fn elemental_fields_share_the_property_ground_shape_and_differ_only_by_element() {
        let elements = [
            (UnitId::Volcano, "effect\\ring_red.tga"),
            (UnitId::Deluge, "effect\\ring_blue.tga"),
            (UnitId::Violentgale, "effect\\ring_yellow.tga"),
        ];

        for (unit_id, expected_texture) in elements {
            let Some(UnitBody::Cylinders { texture, specs, .. }) = unit_presentation(unit_id).unwrap().body else {
                panic!("{unit_id:?} must be a cylinder body");
            };
            assert_eq!(texture, expected_texture);

            assert_eq!(specs.len(), 1, "{unit_id:?} is a single cone");

            // Sized per cell, not per field: Hercules `Layout: 3` sends 49
            // `AddSkillUnit` packets for one 7×7 field, so a cone much wider
            // than a cell renders the field as a solid block — that was the
            // 2026-07-24 live failure. Guard the ceiling, not an exact value,
            // so brightness/size tuning stays free.
            assert!(
                specs[0].top_radius <= GAT_TILE_SIZE,
                "{unit_id:?} cone is drawn per cell and must not swallow its neighbors"
            );
            assert!(specs[0].bottom_radius < specs[0].top_radius, "{unit_id:?} flares upward");

            // The original breathes these fields between half and full size.
            let pulse = specs[0].pulse.as_ref().expect("elemental fields pulse");
            assert_eq!((pulse.min_scale, pulse.max_scale), (0.5, 1.0));
        }
    }

    #[test]
    fn looping_sprite_units_name_a_sprite_folder_path() {
        for unit_id in [UnitId::Venomdust, UnitId::Demonstration] {
            let Some(UnitBody::LoopingSprite { path, .. }) = unit_presentation(unit_id).unwrap().body else {
                panic!("{unit_id:?} must loop a sprite");
            };
            // These resolve under `data\sprite\`, not `data\texture\effect\`.
            assert!(path.starts_with("이팩트\\"), "{unit_id:?} path {path} is not a sprite path");
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
