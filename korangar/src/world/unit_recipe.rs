//! Typed presentation recipes for persistent skill units (Phase E2).
//!
//! A unit exists from `AddSkillUnit` until `RemoveSkillUnit` — the server owns
//! its lifetime completely. This table only declares what a live unit looks
//! like. Mappings come from the reverse-engineered original-client tables
//! (unit ID → effect ID → assets); see
//! `docs/plans/classic-effect-fidelity.md` for sourcing and verification.

use ragnarok_packets::UnitId;

use crate::Color;
use crate::loaders::GAT_TILE_SIZE;
use crate::world::{UnitCylinderSpec, UnitPulse};

/// Hunter traps are the one unit family the procedural bodies below cannot
/// express: their originals are RSM **models**, not textures or sprites.
///
/// Paths are relative to `data\model\`. Mapping recovered from the same
/// reverse-engineered unit table as the rest of this file — roBrowser names them
/// `ef_trap_NN`, which is its romanisation of these exact files. All ten are
/// GRF-verified present (2026-07-26) and the mapping is 1:1 with no reuse.
pub fn trap_model_file(unit_id: UnitId) -> Option<&'static str> {
    let file = match unit_id {
        UnitId::Anklesnare => "외부소품\\트랩01.rsm",
        UnitId::Skidtrap => "외부소품\\트랩02.rsm",
        UnitId::Landmine => "외부소품\\트랩03.rsm",
        UnitId::Freezingtrap => "외부소품\\트랩03_2.rsm",
        UnitId::Blastmine => "외부소품\\트랩03_3.rsm",
        UnitId::Sandman => "외부소품\\트랩03_4.rsm",
        UnitId::Flasher => "외부소품\\트랩03_5.rsm",
        UnitId::Shockwave => "외부소품\\트랩03_6.rsm",
        UnitId::Claymoretrap => "외부소품\\트랩04.rsm",
        UnitId::Talkiebox => "외부소품\\트랩05.rsm",
        _ => return None,
    };
    Some(file)
}

/// Every trap model, so a map load can put their geometry in the shared buffer
/// up front — a trap spawning mid-fight then costs only a draw instruction.
pub const TRAP_MODEL_FILES: &[&str] = &[
    "외부소품\\트랩01.rsm",
    "외부소품\\트랩02.rsm",
    "외부소품\\트랩03.rsm",
    "외부소품\\트랩03_2.rsm",
    "외부소품\\트랩03_3.rsm",
    "외부소품\\트랩03_4.rsm",
    "외부소품\\트랩03_5.rsm",
    "외부소품\\트랩03_6.rsm",
    "외부소품\\트랩04.rsm",
    "외부소품\\트랩05.rsm",
];

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
        /// `None` is the table's `FlatColorTile`: no artwork, just the colour.
        /// A texture here is artwork the original actually draws, like Land
        /// Protector's magic circle.
        texture: Option<&'static str>,
        /// Half-width in world units.
        half_size: f32,
        color: Color,
        pulse: UnitPulse,
    },
    /// The classic two-layer ground tile: a flat tint with a textured layer
    /// hovering and bobbing above it. This is the shape *every* entry in the
    /// original's song/ground-tile table takes, and it is what
    /// [`UnitBody::GroundQuad`] cannot express — a single quad loses the bob and
    /// merges tint with artwork.
    LayeredGroundQuad {
        /// Lower layer: a `FlatColorTile`, drawn with no artwork. `None` draws
        /// no tint at all, leaving only the hovering layer over bare ground —
        /// not a shape the original's table uses, but the tint is what makes a
        /// large field read as a slab, and some effects are better as their
        /// artwork alone.
        tile_color: Option<Color>,
        /// Half-width of the tint layer in world units.
        half_size: f32,
        /// Upper layer artwork, under `data\texture\`.
        hover_texture: &'static str,
        /// Half-width of the hovering layer in world units.
        hover_half_size: f32,
        hover_opacity: f32,
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
            // The original's `LPEffect` is a flat floor tile per cell. Now that
            // the depth-tested ground-decal pass exists, `UnitGroundQuad` draws
            // exactly that: the tile is occluded by terrain and a player standing
            // on it composes over it, instead of the depth-less post-processing
            // path that painted it on top of the character (the interim low
            // square glow it replaced). Half-size 2.0 = the table's 0.8-cell tile.
            body: Some(UnitBody::GroundQuad {
                texture: Some("effect\\aaa copy.bmp"),
                half_size: 2.0,
                // Translucent so the magic pattern reads over the floor rather
                // than masking it; 121 of these overlap at Lv5.
                color: Color::rgba(0.75, 0.90, 1.0, 0.65),
                pulse: UnitPulse {
                    min_scale: 0.85,
                    max_scale: 1.0,
                    speed: 2.5,
                },
            }),
            // Soft blue glow: one companion point light per cell (keyed on the
            // cell's server entity id, so all 121 register). The colour is a
            // *saturated* blue on purpose — a pale blue accumulates across 121
            // overlapping lights and clips toward white, so the field reads as a
            // soft blue only if each light is distinctly blue to begin with.
            // Land Protector is 11×11 cells at Lv5 (15×15 at Lv10); 12.0 was
            // invisible live, so intensity stays up. Dialed in live
            // (2026-07-25, user): 26 too dim → 40 a touch hot → 30 settled.
            light: Some((Color::rgb_u8(70, 135, 255), 30.0)),
            ..NONE
        }),
        // CG_MOONLIT — Sheltering Bliss. Effect **394**, whose Hercules constant
        // is the entirely misleading `EF_SPHEREWIND2`; the reverse-engineered
        // table names it "Moonlit water mill/sheltering bliss". Another instance
        // of "never take an effect from its constant name".
        //
        // The original is a `FlatColorTile` per cell — no texture at all, just a
        // flat translucent salmon quad, `uSize 0.5` over ±1.0 vertices, i.e. one
        // full cell. At 5.0 world units per cell that is half_size 2.5 (Land
        // Protector's 2.0 is its narrower 0.8-cell tile). Colour is verbatim from
        // the table: 0xff8abb at alpha 0.6.
        UnitId::Moonlit => Some(UnitPresentation {
            body: Some(UnitBody::LayeredGroundQuad {
                // Confirmed live 2026-08-06: borrowing Land Protector's tile as a
                // carrier read as Land Protector's *pattern*, so this draws with
                // no artwork at all, which is what `FlatColorTile` means.
                // **No tint layer, which is a deliberate departure from the
                // table.** Effect 394's salmon `FlatColorTile` is faithful and
                // was verified correct on screen at α 0.6 — and it still read as
                // a flat slab, because that is what a 9×9 of full-coverage tiles
                // is. Judged live 2026-08-08: the hovering notes carry the skill
                // better on their own, with the colour moved into the light.
                // The tile colour is recorded in the test below so a fidelity
                // pass can restore it deliberately.
                tile_color: None,
                half_size: 2.9,
                // **A fork embellishment.** Effect 394 is a bare `FlatColorTile`
                // with no second layer, so the hovering note is ours — added
                // because live 2026-08-08 the bare field read as inert, with no
                // animation anywhere on the area.
                //
                // The texture is not invented: `melody_a.bmp` is what the
                // original's own Humming entry hovers, and Whistle and Drum
                // Battlefield use its sibling `melody_b`. Moonlit is a Clown
                // song in that same table, and its sound is
                // "Moonlight Serenade", so a drifting note is the family's own
                // vocabulary rather than a guess. One cell wide, so the notes
                // read as individual marks over the field instead of a wash.
                hover_texture: "effect\\melody_a.bmp",
                hover_half_size: GAT_TILE_SIZE / 2.0,
                hover_opacity: 0.7,
            }),
            sound: Some("effect\\달빛세레나데.wav"),
            // The glow is korangar's addition (the original `FlatColorTile`
            // emits nothing), so it is tuned to read as the tile's own colour —
            // hence the exact `0xff8abb` above rather than an approximation.
            //
            // **The second number is a RADIUS, not a brightness.** `light` is
            // handed to `register_fading` as the range; there is no separate
            // intensity, so a light cannot be dimmed, only made smaller. That is
            // what broke this: Layout 4 is 9×9, and at radius 18.0 every cell's
            // light reached 3.6 cells in each direction, so ~40 of the 81 piled
            // up over the middle of the field.
            //
            // Stacking is survivable for a strongly hued light and fatal for a
            // pale one. Land Protector stacks 121 lights at radius 30 and still
            // reads blue, because its red channel (0.27) stays low however much
            // it accumulates. Salmon is (1.0, 0.54, 0.73) — already near white,
            // so every channel saturates together. Measured live 2026-08-08: a
            // white bloom washing the terrain *under* the tile and spilling past
            // the field edge, which is what made the tile read as flat red.
            //
            // With the tint gone the light carries the skill's colour. Two
            // constraints fight each other here, and both were measured live
            // 2026-08-08 rather than reasoned about:
            //
            // **Overlap decides the hue, not the colour value.** A field is one
            // light per cell, so a cell's pool covers `π·r_ground²/25` of its
            // neighbours. At radius 18 that was a ~40-deep pile and at radius 9
            // still ~4 deep — enough to drive every channel to 1.0, which is why
            // *blue* also came back white. Radius 6 puts it near 2.5, low enough
            // for a colour to survive. A saturated hue has further to climb
            // before it clips, so the salmon here is deepened toward rose rather
            // than the tile's near-white (1.0, 0.54, 0.73).
            //
            // **The radius must still clear the light's own 4.0 lift** off the
            // ground (`UnitPointLight`), or it illuminates nothing: 4.0 was tried
            // live and was invisible, the sphere's underside only grazing the
            // floor. That is the floor on this number, and it is close — if a
            // future pass needs the glow both wider and coloured, the real fix is
            // one light per *group* (as `claim_unit_sound` does for audio), not a
            // further tweak here.
            light: Some((Color::rgb_u8(255, 80, 135), 6.0)),
            ..NONE
        }),
        // CG_HERMODE — Wand of Hermode. `EF_BOTTOM_HERMODE` (517) is an
        // **explicitly empty** entry in the reverse-engineered table, commented
        // "(Nothing)"; the only presentation is the `517_music` variant, a sound.
        // So drawing nothing here is the authentic behaviour, not a gap — the
        // area is meant to be audible, not visible.
        UnitId::Hermode => Some(UnitPresentation {
            sound: Some("effect\\헤르모드의 지팡이.wav"),
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
        // The three renewal survivors of the original's ground-tile table. The
        // other 16 rows are songs and dances, which renewal `skill_db.conf`
        // gives no `Unit:` block, so no `AddSkillUnit` ever arrives for them —
        // a server-configuration fact, not a missing asset (pre-renewal does
        // declare them).
        //
        // Tile colours are verbatim from the recovered table. **The hover
        // opacity is not**: `HoveringTexture(path, size, opacity)` records a
        // size per entry but the recovered table only preserved the two sizes
        // that deviate, so the default size and every opacity here are
        // estimates and want the same live calibration Moonlit got.
        UnitId::Gospel => Some(UnitPresentation {
            body: Some(UnitBody::LayeredGroundQuad {
                tile_color: Some(Color::rgba(1.0, 1.0, 1.0, 0.05)),
                half_size: GAT_TILE_SIZE / 2.0,
                hover_texture: "effect\\cross_old.bmp",
                hover_half_size: GAT_TILE_SIZE,
                hover_opacity: 1.0,
            }),
            ..NONE
        }),
        UnitId::Fogwall => Some(UnitPresentation {
            body: Some(UnitBody::LayeredGroundQuad {
                tile_color: Some(Color::rgba(0.667, 0.667, 0.667, 0.6)),
                half_size: GAT_TILE_SIZE / 2.0,
                hover_texture: "effect\\lens_w.bmp",
                hover_half_size: GAT_TILE_SIZE,
                hover_opacity: 1.0,
            }),
            ..NONE
        }),
        UnitId::Evilland => Some(UnitPresentation {
            body: Some(UnitBody::LayeredGroundQuad {
                tile_color: Some(Color::rgba(0.627, 0.627, 0.627, 0.2)),
                half_size: GAT_TILE_SIZE / 2.0,
                // One of the two entries the table records an explicit size for.
                hover_texture: "effect\\curse.bmp",
                hover_half_size: GAT_TILE_SIZE / 2.0,
                hover_opacity: 1.0,
            }),
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
    UnitId::Moonlit,
    UnitId::Hermode,
    UnitId::Gospel,
    UnitId::Fogwall,
    UnitId::Evilland,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loaders::GAT_TILE_SIZE;

    #[test]
    fn every_mapped_unit_presents_something() {
        for unit_id in MAPPED_UNIT_IDS {
            let presentation = unit_presentation(*unit_id).expect("mapped unit must resolve");
            assert!(
                presentation.looping_str.is_some() || presentation.body.is_some() || presentation.sound.is_some(),
                "unit {unit_id:?} presents nothing at all"
            );
        }
    }

    #[test]
    fn hermode_is_deliberately_audible_only() {
        // `EF_BOTTOM_HERMODE` (517) is an explicitly empty entry in the
        // reverse-engineered table, commented "(Nothing)"; its only presentation
        // is the `517_music` sound variant. Drawing nothing is therefore correct,
        // so this is pinned to stop a future pass "fixing" the missing visual.
        let presentation = unit_presentation(UnitId::Hermode).expect("Hermode is mapped");
        assert!(presentation.body.is_none(), "Hermode draws nothing by design");
        assert!(presentation.looping_str.is_none(), "Hermode draws nothing by design");
        assert_eq!(presentation.sound, Some("effect\\헤르모드의 지팡이.wav"));
    }

    #[test]
    fn moonlit_is_the_tables_flat_salmon_cell_tile() {
        // Effect 394 — Hercules calls that id `EF_SPHEREWIND2`, which has nothing
        // to do with the skill; the table names it "sheltering bliss". Colour is
        // verbatim (0xff8abb @ 0.6) and the tile covers one full cell, so
        // half_size is GAT_TILE_SIZE / 2 rather than Land Protector's narrower 2.0.
        let Some(UnitBody::LayeredGroundQuad {
            tile_color,
            half_size,
            hover_texture,
            hover_half_size,
            ..
        }) = unit_presentation(UnitId::Moonlit).unwrap().body
        else {
            panic!("Moonlit must be a layered ground quad");
        };
        // **The tint is deliberately absent**, judged live 2026-08-08: the
        // table's salmon is faithful and was confirmed correct at α 0.6, but a
        // 9×9 of full-coverage tiles reads as a slab whatever its colour, so the
        // notes carry the skill alone and the colour moved into the light. The
        // table's value is recorded here so restoring it stays a decision:
        // `Color::rgba(1.0, 0.541, 0.733, 0.6)`, effect 394's verbatim 0xff8abb.
        assert_eq!(tile_color, None, "the salmon tint was dropped on purpose");

        // The deliberate fork deviations, pinned so they stay deliberate — a
        // later fidelity pass may drop them, but should do so knowingly.
        // Live 2026-08-08: at exactly one cell the soft-edged carrier showed a
        // seam at every cell border, so the tiles overlap.
        assert!(
            half_size > GAT_TILE_SIZE / 2.0,
            "tiles must overlap so the soft carrier's feather does not show as a grid"
        );
        assert!(
            half_size < GAT_TILE_SIZE * 0.7,
            "overlap is a seam fix, not a bigger field"
        );
        // Effect 394 has no second layer; the hovering note is ours, borrowed
        // from the sibling songs in the original's own table.
        assert_eq!(hover_texture, "effect\\melody_a.bmp");
        assert_eq!(hover_half_size, GAT_TILE_SIZE / 2.0);
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
