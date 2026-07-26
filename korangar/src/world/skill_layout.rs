//! Ground-unit footprints — which cells a ground-targeted skill will actually
//! cover, used to draw the aiming cursor as the skill's real area.
//!
//! The authority is Hercules' `skill_init_unit_layout` (`src/map/skill.c`), not
//! guesswork: square layouts come from `skill_db.conf`'s `Layout: N` (exported
//! into `docs/skills.json`), and the fifteen skills that export `Layout: -1`
//! have their cell lists hardcoded in that C function. Both are mirrored here
//! so the cursor and the server agree on the shape.
//!
//! Offsets are `(dx, dy)` in cells relative to the targeted tile, matching the
//! `dx[]`/`dy[]` pairs in the C source.

use ragnarok_packets::{SkillId, SkillLevel, TilePosition};

use crate::world::library::skill_layout_value;

/// Per-cell tile the aiming footprint is drawn with — the same texture the
/// original uses for Land Protector's ground tiles.
pub const SKILL_FOOTPRINT_TEXTURE: &str = "effect\\aaa copy.bmp";

/// Hercules caps square layouts at `MAX_SQUARE_LAYOUT` — **7**, i.e. 15×15
/// (`skill.h:55`). A larger value in the db would index past the server's
/// table, so clamp to the same bound rather than trusting the export. Land
/// Protector is the only skill that reaches it (layout 7 at Lv9-10).
const MAX_SQUARE_LAYOUT: i64 = 7;

/// The fifteen skills that export `Layout: -1`, i.e. the ones whose shape lives
/// in C rather than in the db. Ids verified against `docs/skills.json` —
/// several differ from the values a reader might assume from the Hercules
/// constant names, so do not hand-edit these without re-checking the export.
///
/// Deliberately absent: `NPC_EVILLAND` and `MH_POISON_MIST`. Hercules groups
/// them with Sanctuary and Venom Dust in the `switch`, but that branch only
/// runs for skills whose layout is `-1`; our `skill_db` gives Evil Land a real
/// `Layout: 1` square and gives Poison Mist no layout at all, so both take the
/// generic path.
mod ids {
    pub const MG_FIREWALL: u16 = 18;
    pub const AL_PNEUMA: u16 = 25;
    pub const PR_SANCTUARY: u16 = 70;
    pub const PR_MAGNUS: u16 = 79;
    pub const WZ_ICEWALL: u16 = 87;
    pub const AS_VENOMDUST: u16 = 140;
    pub const CR_GRANDCROSS: u16 = 254;
    pub const NPC_GRANDDARKNESS: u16 = 339;
    pub const PA_GOSPEL: u16 = 369;
    pub const PF_FOGWALL: u16 = 404;
    pub const NJ_TATAMIGAESHI: u16 = 527;
    pub const NJ_KAENSIN: u16 = 535;
    pub const WL_EARTHSTRAIN: u16 = 2216;
    pub const GN_WALLOFTHORN: u16 = 2482;
    pub const RL_FIRE_RAIN: u16 = 2567;
    pub const EL_FIRE_MANTLE: u16 = 8403;
}

/// `PR_SANCTUARY` — 21 cells, a 5×5 with the corners cut.
const SANCTUARY: &[(i8, i8)] = &[
    (-1, -2),
    (0, -2),
    (1, -2),
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1),
    (-2, 0),
    (-1, 0),
    (0, 0),
    (1, 0),
    (2, 0),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
    (2, 1),
    (-1, 2),
    (0, 2),
    (1, 2),
];

/// `PR_MAGNUS` / `PA_GOSPEL` — 33 cells, a 7×7 cross-octagon.
const MAGNUS: &[(i8, i8)] = &[
    (-1, -3),
    (0, -3),
    (1, -3),
    (-1, -2),
    (0, -2),
    (1, -2),
    (-3, -1),
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1),
    (3, -1),
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (0, 0),
    (1, 0),
    (2, 0),
    (3, 0),
    (-3, 1),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
    (2, 1),
    (3, 1),
    (-1, 2),
    (0, 2),
    (1, 2),
    (-1, 3),
    (0, 3),
    (1, 3),
];

/// `AS_VENOMDUST` — a plus sign.
const VENOM_DUST: &[(i8, i8)] = &[(-1, 0), (0, -1), (0, 0), (0, 1), (1, 0)];

/// `CR_GRANDCROSS` / `NPC_GRANDDARKNESS` — 29 cells, a tapering cross.
const GRAND_CROSS: &[(i8, i8)] = &[
    (0, -4),
    (0, -3),
    (-1, -2),
    (0, -2),
    (1, -2),
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1),
    (-4, 0),
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (0, 0),
    (1, 0),
    (2, 0),
    (3, 0),
    (4, 0),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
    (2, 1),
    (-1, 2),
    (0, 2),
    (1, 2),
    (0, 3),
    (0, 4),
];

/// `PF_FOGWALL` — a 5 wide × 3 deep block.
const FOG_WALL: &[(i8, i8)] = &[
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1),
    (-2, 0),
    (-1, 0),
    (0, 0),
    (1, 0),
    (2, 0),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
    (2, 1),
];

/// `NJ_KAENSIN` — a filled 5×5 with the centre cell left out.
const KAENSIN: &[(i8, i8)] = &[
    (-2, 2),
    (-1, 2),
    (0, 2),
    (1, 2),
    (2, 2),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
    (2, 1),
    (-2, 0),
    (-1, 0),
    (1, 0),
    (2, 0),
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1),
    (-2, -2),
    (-1, -2),
    (0, -2),
    (1, -2),
    (2, -2),
];

/// `GN_WALLOFTHORN` — a hollow 5×5 ring.
const WALL_OF_THORN: &[(i8, i8)] = &[
    (-1, 2),
    (-2, 2),
    (-2, 1),
    (-2, 0),
    (-2, -1),
    (-2, -2),
    (-1, -2),
    (0, -2),
    (1, -2),
    (2, -2),
    (2, -1),
    (2, 0),
    (2, 1),
    (2, 2),
    (1, 2),
    (0, 2),
];

/// `EL_FIRE_MANTLE` — the eight cells surrounding the centre.
const FIRE_MANTLE: &[(i8, i8)] = &[(-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0)];

/// `NJ_TATAMIGAESHI` — a cross whose arms grow with level (Lv1 / Lv2-3 / Lv4+).
const TATAMI_LV1: &[(i8, i8)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];
const TATAMI_LV2: &[(i8, i8)] = &[(-2, 0), (-1, 0), (1, 0), (2, 0), (0, -2), (0, -1), (0, 1), (0, 2)];
const TATAMI_LV4: &[(i8, i8)] = &[
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (1, 0),
    (2, 0),
    (3, 0),
    (0, -3),
    (0, -2),
    (0, -1),
    (0, 1),
    (0, 2),
    (0, 3),
];

/// The eight walk directions, in Hercules' `map->calc_dir` order. Only the
/// parity and axis matter to the direction-dependent walls, which is why the C
/// code switches on `i & 1`, `i & 0x2` and `i % 4`.
const fn is_diagonal(direction: u8) -> bool {
    direction & 1 == 1
}

/// `MG_FIREWALL` — 3 cells across the facing axis, 5 on a diagonal.
fn fire_wall(direction: u8) -> &'static [(i8, i8)] {
    match (is_diagonal(direction), direction & 0x2, direction % 4) {
        (true, 0, _) => &[(1, 1), (1, 0), (0, 0), (0, -1), (-1, -1)],
        (true, ..) => &[(-1, 1), (-1, 0), (0, 0), (0, -1), (1, -1)],
        (false, _, 0) => &[(-1, 0), (0, 0), (1, 0)],
        (false, ..) => &[(0, -1), (0, 0), (0, 1)],
    }
}

/// `WZ_ICEWALL` — always 5 cells, laid along the perpendicular of the facing.
fn ice_wall(direction: u8) -> &'static [(i8, i8)] {
    match (is_diagonal(direction), direction & 0x2, direction % 4) {
        (true, 0, _) => &[(2, 2), (1, 1), (0, 0), (-1, -1), (-2, -2)],
        (true, ..) => &[(-2, 2), (-1, 1), (0, 0), (1, -1), (2, -2)],
        (false, _, 0) => &[(-2, 0), (-1, 0), (0, 0), (1, 0), (2, 0)],
        (false, ..) => &[(0, -2), (0, -1), (0, 0), (0, 1), (0, 2)],
    }
}

/// `WL_EARTHSTRAIN` — a 15-cell line. Hercules only distinguishes the two
/// cardinal axes here; diagonals reuse the horizontal line.
fn earth_strain(direction: u8) -> &'static [(i8, i8)] {
    const HORIZONTAL: &[(i8, i8)] = &[
        (-7, 0),
        (-6, 0),
        (-5, 0),
        (-4, 0),
        (-3, 0),
        (-2, 0),
        (-1, 0),
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (4, 0),
        (5, 0),
        (6, 0),
        (7, 0),
    ];
    const VERTICAL: &[(i8, i8)] = &[
        (0, -7),
        (0, -6),
        (0, -5),
        (0, -4),
        (0, -3),
        (0, -2),
        (0, -1),
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (0, 5),
        (0, 6),
        (0, 7),
    ];

    match direction {
        2 | 6 => VERTICAL,
        _ => HORIZONTAL,
    }
}

/// `RL_FIRE_RAIN` — the same axis split as Earth Strain, three cells wide.
fn fire_rain(direction: u8) -> &'static [(i8, i8)] {
    match direction {
        2 | 6 => &[(0, -1), (0, 0), (0, 1)],
        _ => &[(-1, 0), (0, 0), (1, 0)],
    }
}

/// Caster→target facing, in the same 0-7 encoding the server uses
/// (`UNIT_DIR_NORTH = 0`, then counter-clockwise: NW, W, SW, S, SE, E, NE).
///
/// Ported from Hercules' `map_calc_dir` (`src/map/map.c`) so the wall-shaped
/// skills draw the orientation the server will actually place. Aiming at your
/// own tile has no meaningful direction; Hercules falls back to the unit's
/// current facing, we settle for north since the shape is about to be rejected
/// anyway.
pub fn facing_direction(from: TilePosition, to: TilePosition) -> u8 {
    let dx = to.x as i32 - from.x as i32;
    let dy = to.y as i32 - from.y as i32;

    const NORTH: u8 = 0;
    const NORTHWEST: u8 = 1;
    const WEST: u8 = 2;
    const SOUTHWEST: u8 = 3;
    const SOUTH: u8 = 4;
    const SOUTHEAST: u8 = 5;
    const EAST: u8 = 6;
    const NORTHEAST: u8 = 7;

    if dx == 0 && dy == 0 {
        NORTH
    } else if dx >= 0 && dy >= 0 {
        if dx * 2 < dy || dx == 0 {
            NORTH
        } else if dx > dy * 2 + 1 || dy == 0 {
            EAST
        } else {
            NORTHEAST
        }
    } else if dx >= 0 && dy <= 0 {
        if dx * 2 < -dy || dx == 0 {
            SOUTH
        } else if dx > -dy * 2 + 1 || dy == 0 {
            EAST
        } else {
            SOUTHEAST
        }
    } else if dx <= 0 && dy <= 0 {
        if dx * 2 > dy || dx == 0 {
            SOUTH
        } else if dx < dy * 2 + 1 || dy == 0 {
            WEST
        } else {
            SOUTHWEST
        }
    } else if -dx * 2 < dy || dx == 0 {
        NORTH
    } else if -dx > dy * 2 + 1 || dy == 0 {
        WEST
    } else {
        NORTHWEST
    }
}

/// Every cell a ground-targeted skill will occupy, relative to the aimed tile.
///
/// `direction` is the caster→target facing (0-7) and only matters for the four
/// wall-shaped skills. Returns `None` when the skill places no ground unit, so
/// callers can fall back to a single-cell cursor rather than inventing an area.
pub fn skill_footprint(skill_id: SkillId, skill_level: SkillLevel, direction: u8) -> Option<Vec<(i8, i8)>> {
    let direction = direction % 8;

    let custom: Option<&'static [(i8, i8)]> = match skill_id.0 {
        ids::PR_SANCTUARY => Some(SANCTUARY),
        ids::PR_MAGNUS | ids::PA_GOSPEL => Some(MAGNUS),
        ids::AS_VENOMDUST => Some(VENOM_DUST),
        ids::CR_GRANDCROSS | ids::NPC_GRANDDARKNESS => Some(GRAND_CROSS),
        ids::PF_FOGWALL => Some(FOG_WALL),
        ids::NJ_KAENSIN => Some(KAENSIN),
        ids::GN_WALLOFTHORN => Some(WALL_OF_THORN),
        ids::EL_FIRE_MANTLE => Some(FIRE_MANTLE),
        ids::NJ_TATAMIGAESHI => Some(match skill_level.0 {
            0 | 1 => TATAMI_LV1,
            2 | 3 => TATAMI_LV2,
            _ => TATAMI_LV4,
        }),
        ids::MG_FIREWALL => Some(fire_wall(direction)),
        ids::WZ_ICEWALL => Some(ice_wall(direction)),
        ids::WL_EARTHSTRAIN => Some(earth_strain(direction)),
        ids::RL_FIRE_RAIN => Some(fire_rain(direction)),
        _ => None,
    };

    if let Some(cells) = custom {
        return Some(cells.to_vec());
    }

    // Not a custom shape: fall back to the exported square layout. Pneuma and
    // friends are `Layout: 1` → 3×3.
    let layout = skill_layout_value(skill_id.0, skill_level.0)?;
    if layout < 0 {
        // A custom shape we have not ported. Better to show a single cell than a
        // square we know is wrong.
        return None;
    }

    let extent = layout.min(MAX_SQUARE_LAYOUT) as i8;
    let mut cells = Vec::with_capacity(((extent as usize) * 2 + 1).pow(2));
    for dy in -extent..=extent {
        for dx in -extent..=extent {
            cells.push((dx, dy));
        }
    }
    Some(cells)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNEUMA: SkillId = SkillId(ids::AL_PNEUMA);
    const STORM_GUST: SkillId = SkillId(89);
    const LAND_PROTECTOR: SkillId = SkillId(288);
    const SANCTUARY_ID: SkillId = SkillId(ids::PR_SANCTUARY);
    const FIRE_WALL: SkillId = SkillId(ids::MG_FIREWALL);
    const TATAMI: SkillId = SkillId(ids::NJ_TATAMIGAESHI);

    /// `Layout: 1` is a 3×3 — the smallest square the server ever places.
    #[test]
    fn a_square_layout_expands_to_its_full_area() {
        let cells = skill_footprint(PNEUMA, SkillLevel(1), 0).unwrap();
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(0, 0)));
        assert!(cells.contains(&(-1, -1)));
        assert!(cells.contains(&(1, 1)));
        assert!(!cells.contains(&(2, 0)));
    }

    /// Storm Gust exports `Layout: 4` → 9×9, the classic wall of ice.
    #[test]
    fn storm_gust_is_nine_across() {
        let cells = skill_footprint(STORM_GUST, SkillLevel(5), 0).unwrap();
        assert_eq!(cells.len(), 81);
    }

    /// Land Protector's layout is per-level (`[3,3,4,4,5,5,6,6,7,7]`), so the
    /// cursor must grow with the level the player actually has.
    #[test]
    fn a_levelled_layout_grows_with_level() {
        let low = skill_footprint(LAND_PROTECTOR, SkillLevel(1), 0).unwrap();
        let high = skill_footprint(LAND_PROTECTOR, SkillLevel(5), 0).unwrap();
        // Lv1 -> layout 3 -> 7x7; Lv5 -> layout 5 -> 11x11.
        assert_eq!(low.len(), 49);
        assert_eq!(high.len(), 121);
        // Lv10 -> layout 7 -> 15x15, the server's largest square. Clamping this
        // to 11x11 would quietly under-draw the biggest field in the game.
        assert_eq!(skill_footprint(LAND_PROTECTOR, SkillLevel(10), 0).unwrap().len(), 225);
    }

    /// The whole point of the custom table: Sanctuary is not a square.
    #[test]
    fn sanctuary_is_the_cut_corner_shape_not_a_square() {
        let cells = skill_footprint(SANCTUARY_ID, SkillLevel(5), 0).unwrap();
        assert_eq!(cells.len(), 21);
        // Corners of the 5x5 are cut.
        assert!(!cells.contains(&(-2, -2)));
        assert!(!cells.contains(&(2, 2)));
        assert!(cells.contains(&(0, -2)));
    }

    /// Fire Wall turns with the caster — that is why the cursor needs a
    /// direction at all.
    #[test]
    fn fire_wall_rotates_with_the_facing() {
        let horizontal = skill_footprint(FIRE_WALL, SkillLevel(1), 0).unwrap();
        let vertical = skill_footprint(FIRE_WALL, SkillLevel(1), 2).unwrap();
        assert_eq!(horizontal, vec![(-1, 0), (0, 0), (1, 0)]);
        assert_eq!(vertical, vec![(0, -1), (0, 0), (0, 1)]);
        // Diagonals are the 5-cell staircase.
        assert_eq!(skill_footprint(FIRE_WALL, SkillLevel(1), 1).unwrap().len(), 5);
    }

    #[test]
    fn tatamigaeshi_arms_grow_by_level_band() {
        assert_eq!(skill_footprint(TATAMI, SkillLevel(1), 0).unwrap().len(), 4);
        assert_eq!(skill_footprint(TATAMI, SkillLevel(3), 0).unwrap().len(), 8);
        assert_eq!(skill_footprint(TATAMI, SkillLevel(5), 0).unwrap().len(), 12);
    }

    #[test]
    fn facing_matches_the_servers_direction_encoding() {
        let origin = TilePosition { x: 50, y: 50 };
        assert_eq!(facing_direction(origin, TilePosition { x: 50, y: 60 }), 0, "north");
        assert_eq!(facing_direction(origin, TilePosition { x: 60, y: 50 }), 6, "east");
        assert_eq!(facing_direction(origin, TilePosition { x: 40, y: 50 }), 2, "west");
        assert_eq!(facing_direction(origin, TilePosition { x: 50, y: 40 }), 4, "south");
        assert_eq!(facing_direction(origin, TilePosition { x: 60, y: 60 }), 7, "north-east");
        assert_eq!(facing_direction(origin, origin), 0, "no direction to speak of");
    }

    /// Every id in [`ids`] must actually be a `Layout: -1` skill in the export.
    /// A wrong id would silently fall through to the square path and draw a
    /// shape the server will not place — and nine of these were wrong on the
    /// first pass, so this is not a hypothetical.
    #[test]
    fn every_custom_shape_id_is_a_custom_layout_in_the_export() {
        for (name, id) in [
            ("MG_FIREWALL", ids::MG_FIREWALL),
            ("PR_SANCTUARY", ids::PR_SANCTUARY),
            ("PR_MAGNUS", ids::PR_MAGNUS),
            ("WZ_ICEWALL", ids::WZ_ICEWALL),
            ("AS_VENOMDUST", ids::AS_VENOMDUST),
            ("CR_GRANDCROSS", ids::CR_GRANDCROSS),
            ("NPC_GRANDDARKNESS", ids::NPC_GRANDDARKNESS),
            ("PA_GOSPEL", ids::PA_GOSPEL),
            ("PF_FOGWALL", ids::PF_FOGWALL),
            ("NJ_TATAMIGAESHI", ids::NJ_TATAMIGAESHI),
            ("NJ_KAENSIN", ids::NJ_KAENSIN),
            ("WL_EARTHSTRAIN", ids::WL_EARTHSTRAIN),
            ("GN_WALLOFTHORN", ids::GN_WALLOFTHORN),
            ("RL_FIRE_RAIN", ids::RL_FIRE_RAIN),
            ("EL_FIRE_MANTLE", ids::EL_FIRE_MANTLE),
        ] {
            assert_eq!(skill_layout_value(id, 1), Some(-1), "{name} (id {id}) should export Layout: -1");
        }
    }

    /// The four wall-shaped skills, checked against every one of the eight
    /// directions Hercules generates in `skill_init_unit_layout`. Expected
    /// values were extracted from that loop, not written by hand.
    #[test]
    fn directional_walls_match_hercules_for_all_eight_directions() {
        const HORIZONTAL_3: &[(i8, i8)] = &[(-1, 0), (0, 0), (1, 0)];
        const VERTICAL_3: &[(i8, i8)] = &[(0, -1), (0, 0), (0, 1)];
        const FIREWALL_A: &[(i8, i8)] = &[(1, 1), (1, 0), (0, 0), (0, -1), (-1, -1)];
        const FIREWALL_B: &[(i8, i8)] = &[(-1, 1), (-1, 0), (0, 0), (0, -1), (1, -1)];

        let expected_firewall = [
            HORIZONTAL_3,
            FIREWALL_A,
            VERTICAL_3,
            FIREWALL_B,
            HORIZONTAL_3,
            FIREWALL_A,
            VERTICAL_3,
            FIREWALL_B,
        ];
        for (direction, expected) in expected_firewall.iter().enumerate() {
            assert_eq!(
                &skill_footprint(SkillId(ids::MG_FIREWALL), SkillLevel(1), direction as u8).unwrap(),
                expected,
                "Fire Wall direction {direction}"
            );
        }

        const ICEWALL_FLAT: &[(i8, i8)] = &[(-2, 0), (-1, 0), (0, 0), (1, 0), (2, 0)];
        const ICEWALL_UPRIGHT: &[(i8, i8)] = &[(0, -2), (0, -1), (0, 0), (0, 1), (0, 2)];
        const ICEWALL_A: &[(i8, i8)] = &[(2, 2), (1, 1), (0, 0), (-1, -1), (-2, -2)];
        const ICEWALL_B: &[(i8, i8)] = &[(-2, 2), (-1, 1), (0, 0), (1, -1), (2, -2)];

        let expected_icewall = [
            ICEWALL_FLAT,
            ICEWALL_A,
            ICEWALL_UPRIGHT,
            ICEWALL_B,
            ICEWALL_FLAT,
            ICEWALL_A,
            ICEWALL_UPRIGHT,
            ICEWALL_B,
        ];
        for (direction, expected) in expected_icewall.iter().enumerate() {
            assert_eq!(
                &skill_footprint(SkillId(ids::WZ_ICEWALL), SkillLevel(1), direction as u8).unwrap(),
                expected,
                "Ice Wall direction {direction}"
            );
        }

        // Earth Strain and Fire Rain split on the axis only: 2 (west) and 6
        // (east) run north-south, every other direction runs east-west.
        for direction in 0..8u8 {
            let upright = matches!(direction, 2 | 6);

            let strain = skill_footprint(SkillId(ids::WL_EARTHSTRAIN), SkillLevel(1), direction).unwrap();
            assert_eq!(strain.len(), 15);
            assert_eq!(
                strain.iter().all(|(dx, _)| *dx == 0),
                upright,
                "Earth Strain axis at direction {direction}"
            );

            let rain = skill_footprint(SkillId(ids::RL_FIRE_RAIN), SkillLevel(1), direction).unwrap();
            assert_eq!(
                &rain,
                match upright {
                    true => VERTICAL_3,
                    false => HORIZONTAL_3,
                },
                "Fire Rain direction {direction}"
            );
        }
    }

    /// A skill that places no ground unit must not get an area cursor.
    #[test]
    fn a_non_ground_skill_has_no_footprint() {
        // Bash — a plain melee attack.
        assert!(skill_footprint(SkillId(5), SkillLevel(10), 0).is_none());
    }
}
