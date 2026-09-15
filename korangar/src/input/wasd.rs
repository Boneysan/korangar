//! WASD walk-request policy.
//!
//! The client still talks to the map server with `RequestPlayerMovePacket`
//! (`0x035F`). This module only decides whether a key event is allowed to send
//! and which tile it names.
//!
//! A tap and a hold use the same packet: the longest straight walkable path up
//! to [`HELD_PATH`]. Releasing names one cell ahead. Every extra send while a
//! walk is already in flight is a correction — the server answers with
//! `PlayerMovePacket` (`0x0087`) from its own origin.

use ragnarok_packets::TilePosition;

/// Minimum gap between `0x035F` sends. Matches the live client throttle.
pub const THROTTLE_MS: u32 = 200;
/// Longest straight path a held key may request. Stays under Hercules
/// `max_walk_path` (17 stock).
pub const HELD_PATH: i32 = 15;
/// Re-issue a held path only when this close to the previous destination.
pub const REFRESH_WITHIN: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasdSendKind {
    Press,
    Extend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WasdIntent {
    pub target: TilePosition,
    pub step_x: i32,
    pub step_y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WasdMoveInput {
    pub start: TilePosition,
    pub step_x: i32,
    pub step_y: i32,
    pub fresh: bool,
    pub client_tick: u32,
    pub last_tick: u32,
    pub current: Option<WasdIntent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasdDecision {
    Send { destination: TilePosition, kind: WasdSendKind },
    Silent,
}

pub fn chebyshev(a: TilePosition, b: TilePosition) -> u16 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

pub fn decide_keyboard_move(input: WasdMoveInput, walkable: impl Fn(TilePosition) -> bool) -> WasdDecision {
    if input.step_x == 0 && input.step_y == 0 {
        return WasdDecision::Silent;
    }

    let is_direction_change = match input.current {
        Some(intent) => intent.step_x != input.step_x || intent.step_y != input.step_y,
        None => false,
    };

    // Throttle sustained sends in the same direction, but allow a genuine direction
    // change to repath immediately so turn/reverse intent is never silently
    // dropped.
    if !is_direction_change && input.client_tick.wrapping_sub(input.last_tick) < THROTTLE_MS {
        return WasdDecision::Silent;
    }

    let repath = match input.current {
        None => true,
        Some(intent) => input.fresh || is_direction_change || chebyshev(input.start, intent.target) <= REFRESH_WITHIN,
    };
    if !repath {
        return WasdDecision::Silent;
    }

    let mut furthest = None;
    for distance in 1..=HELD_PATH {
        let tile_x = input.start.x as i32 + input.step_x * distance;
        let tile_y = input.start.y as i32 + input.step_y * distance;
        if tile_x < 0 || tile_y < 0 {
            break;
        }
        let tile = TilePosition {
            x: tile_x as u16,
            y: tile_y as u16,
        };
        if walkable(tile) {
            furthest = Some(tile);
        } else {
            break;
        }
    }

    match furthest {
        // Coalesce duplicate destinations: if destination matches the current in-flight
        // intent target, the server is already walking there; resending causes a redundant
        // 0x035F / 0x0087 snap.
        Some(destination) if destination != input.start && input.current.as_ref().map(|i| i.target) != Some(destination) => {
            let kind = if input.fresh { WasdSendKind::Press } else { WasdSendKind::Extend };
            WasdDecision::Send { destination, kind }
        }
        _ => WasdDecision::Silent,
    }
}

/// Key-release tile: one step ahead of `here` when that cell is walkable.
/// Naming the cell underfoot can ask the server to walk backwards.
pub fn stop_tile(here: TilePosition, step_x: i32, step_y: i32, walkable: impl Fn(TilePosition) -> bool) -> TilePosition {
    let ahead_x = (here.x as i32 + step_x).max(0) as u16;
    let ahead_y = (here.y as i32 + step_y).max(0) as u16;
    let ahead = TilePosition { x: ahead_x, y: ahead_y };
    if walkable(ahead) { ahead } else { here }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasdStopDecision {
    Send { destination: TilePosition },
    Silent,
}

/// Decides whether key release needs to send a stop move packet.
/// Coalesces redundant stops: if the character is already at or near the
/// in-flight target, or if the stop tile equals the in-flight destination,
/// stays Silent.
pub fn decide_keyboard_stop(here: TilePosition, intent: Option<WasdIntent>, walkable: impl Fn(TilePosition) -> bool) -> WasdStopDecision {
    let Some(intent) = intent else {
        return WasdStopDecision::Silent;
    };

    if here == intent.target {
        return WasdStopDecision::Silent;
    }

    let ahead = stop_tile(here, intent.step_x, intent.step_y, &walkable);

    // If stop destination is identical to the in-flight target, the server is
    // already walking there; sending again produces a duplicate 0x035F /
    // redundant correction.
    if ahead == intent.target {
        return WasdStopDecision::Silent;
    }

    // Do not walk beyond the intent target in the travel direction.
    if chebyshev(here, intent.target) < chebyshev(here, ahead) {
        return WasdStopDecision::Silent;
    }

    // If ahead is not walkable and falls back to here, do not ask server to walk
    // backwards.
    if ahead == here {
        return WasdStopDecision::Silent;
    }

    WasdStopDecision::Send { destination: ahead }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasdInvalidator {
    Knockback,
    ServerStop,
    Warp,
    MapLoad,
    Death,
    StunFreeze,
    CastRooting,
}

impl WasdInvalidator {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Knockback => "knockback",
            Self::ServerStop => "server-stop",
            Self::Warp => "warp",
            Self::MapLoad => "map-load",
            Self::Death => "death",
            Self::StunFreeze => "stun-freeze",
            Self::CastRooting => "cast-rooting",
        }
    }
}

/// Clears held intent upon an authoritative event so no stale pre-event path
/// is resent and the player may resume intentionally from the new position.
pub fn invalidate_held_intent(_invalidator: WasdInvalidator) -> Option<WasdIntent> {
    None
}

/// After an authoritative snap that leaves the stored dest far away, a held
/// key does not repath. That is the stale-intent mechanism.
pub fn held_intent_is_stale(start: TilePosition, intent: WasdIntent) -> bool {
    chebyshev(start, intent.target) > REFRESH_WITHIN
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(x: u16, y: u16) -> TilePosition {
        TilePosition { x, y }
    }

    fn decide(
        start: TilePosition,
        step_x: i32,
        step_y: i32,
        fresh: bool,
        client_tick: u32,
        last_tick: u32,
        current: Option<WasdIntent>,
        walkable: impl Fn(TilePosition) -> bool,
    ) -> WasdDecision {
        decide_keyboard_move(
            WasdMoveInput {
                start,
                step_x,
                step_y,
                fresh,
                client_tick,
                last_tick,
                current,
            },
            walkable,
        )
    }

    fn open(_tile: TilePosition) -> bool {
        true
    }

    fn blocked_at(wall: TilePosition) -> impl Fn(TilePosition) -> bool {
        move |tile| tile != wall
    }

    #[test]
    fn press_sends_held_path_not_one_cell() {
        let start = tile(100, 100);
        let decision = decide(start, 1, 0, true, 1_000, 0, None, open);
        assert_eq!(decision, WasdDecision::Send {
            destination: tile(115, 100),
            kind: WasdSendKind::Press,
        });
        if let WasdDecision::Send { destination, .. } = decision {
            assert_eq!(chebyshev(start, destination), HELD_PATH as u16);
            assert_ne!(chebyshev(start, destination), 1);
        }
    }

    #[test]
    fn hold_does_not_resend_until_within_one_cell() {
        let intent = WasdIntent {
            target: tile(115, 100),
            step_x: 1,
            step_y: 0,
        };
        let far = decide(tile(105, 100), 1, 0, false, 1_400, 1_000, Some(intent), open);
        assert_eq!(far, WasdDecision::Silent);

        let near = decide(tile(114, 100), 1, 0, false, 1_400, 1_000, Some(intent), open);
        assert_eq!(near, WasdDecision::Send {
            destination: tile(129, 100),
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn throttle_suppresses_even_a_fresh_press() {
        let start = tile(100, 100);
        let decision = decide(start, 1, 0, true, 1_199, 1_000, None, open);
        assert_eq!(decision, WasdDecision::Silent);
        let released = decide(start, 1, 0, true, 1_200, 1_000, None, open);
        assert!(matches!(released, WasdDecision::Send { .. }));
    }

    #[test]
    fn direction_change_repaths() {
        let intent = WasdIntent {
            target: tile(115, 100),
            step_x: 1,
            step_y: 0,
        };
        let decision = decide(tile(105, 100), 0, 1, false, 1_400, 1_000, Some(intent), open);
        assert_eq!(decision, WasdDecision::Send {
            destination: tile(105, 115),
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn obstacle_shortens_the_path() {
        let start = tile(100, 100);
        let decision = decide(start, 1, 0, true, 1_000, 0, None, blocked_at(tile(106, 100)));
        assert_eq!(decision, WasdDecision::Send {
            destination: tile(105, 100),
            kind: WasdSendKind::Press,
        });
    }

    #[test]
    fn stop_names_one_cell_ahead_when_walkable() {
        assert_eq!(stop_tile(tile(110, 100), 1, 0, open), tile(111, 100));
        assert_eq!(stop_tile(tile(110, 100), 1, 0, blocked_at(tile(111, 100))), tile(110, 100));
    }

    #[test]
    fn warp_or_knockback_leaves_held_intent_stale() {
        let intent = WasdIntent {
            target: tile(115, 100),
            step_x: 1,
            step_y: 0,
        };
        let after_warp = tile(50, 50);
        assert!(held_intent_is_stale(after_warp, intent));
        let decision = decide(after_warp, 1, 0, false, 2_000, 1_000, Some(intent), open);
        assert_eq!(decision, WasdDecision::Silent);
    }

    #[test]
    fn historical_one_cell_then_path_pair_is_gone() {
        let start = tile(100, 100);
        let first = decide(start, 1, 0, true, 1_000, 0, None, open);
        let WasdDecision::Send {
            destination: first_dest, ..
        } = first
        else {
            panic!("press must send");
        };
        let second = decide(
            start,
            1,
            0,
            false,
            1_200,
            1_000,
            Some(WasdIntent {
                target: first_dest,
                step_x: 1,
                step_y: 0,
            }),
            open,
        );
        assert_eq!(second, WasdDecision::Silent);
        assert_eq!(chebyshev(start, first_dest), 15);
    }

    #[test]
    fn tap_preserves_single_tile_movement_and_avoids_duplicate_stops() {
        let start = tile(100, 100);
        let press = decide(start, 1, 0, true, 1_000, 0, None, open);
        assert_eq!(press, WasdDecision::Send {
            destination: tile(115, 100),
            kind: WasdSendKind::Press,
        });

        let intent = WasdIntent {
            target: tile(115, 100),
            step_x: 1,
            step_y: 0,
        };
        let stop = decide_keyboard_stop(start, Some(intent), open);
        assert_eq!(stop, WasdStopDecision::Send {
            destination: tile(101, 100)
        });

        // Corridor / 1-cell path: wall at 102.
        let blocked = blocked_at(tile(102, 100));
        let press_1cell = decide(start, 1, 0, true, 1_000, 0, None, &blocked);
        assert_eq!(press_1cell, WasdDecision::Send {
            destination: tile(101, 100),
            kind: WasdSendKind::Press,
        });
        let intent_1cell = WasdIntent {
            target: tile(101, 100),
            step_x: 1,
            step_y: 0,
        };
        // On release, ahead == intent.target: stop decision is Silent to avoid
        // duplicate 0x035F send.
        let stop_1cell = decide_keyboard_stop(start, Some(intent_1cell), &blocked);
        assert_eq!(stop_1cell, WasdStopDecision::Silent);
    }

    #[test]
    fn hold_coalesces_duplicate_destinations_and_extends_smoothly() {
        let intent = WasdIntent {
            target: tile(115, 100),
            step_x: 1,
            step_y: 0,
        };
        // Far: silent
        assert_eq!(
            decide(tile(105, 100), 1, 0, false, 1_400, 1_000, Some(intent), open),
            WasdDecision::Silent
        );

        // Near: extends to 129
        assert_eq!(
            decide(tile(114, 100), 1, 0, false, 1_400, 1_000, Some(intent), open),
            WasdDecision::Send {
                destination: tile(129, 100),
                kind: WasdSendKind::Extend,
            }
        );

        // Near but blocked at wall: furthest would be 115 (same as intent.target). Must
        // coalesce and stay Silent.
        let wall = blocked_at(tile(116, 100));
        assert_eq!(
            decide(tile(114, 100), 1, 0, false, 1_400, 1_000, Some(intent), &wall),
            WasdDecision::Silent
        );
    }

    #[test]
    fn release_stops_safely_and_coalesces_when_arrived() {
        let intent = WasdIntent {
            target: tile(115, 100),
            step_x: 1,
            step_y: 0,
        };
        // Released mid-walk at 108: stop at 109.
        assert_eq!(
            decide_keyboard_stop(tile(108, 100), Some(intent), open),
            WasdStopDecision::Send {
                destination: tile(109, 100)
            }
        );

        // Released at destination 115: already arrived, stays Silent.
        assert_eq!(
            decide_keyboard_stop(tile(115, 100), Some(intent), open),
            WasdStopDecision::Silent
        );

        // Released when ahead is blocked by wall: stays Silent.
        let wall = blocked_at(tile(110, 100));
        assert_eq!(
            decide_keyboard_stop(tile(109, 100), Some(intent), &wall),
            WasdStopDecision::Silent
        );
    }

    #[test]
    fn opposite_direction_repaths_without_dropped_intent() {
        let intent = WasdIntent {
            target: tile(115, 100),
            step_x: 1,
            step_y: 0,
        };
        // Moving East, player turns West at t=1050 (only 50ms into the 200ms throttle).
        // Direction change repaths immediately.
        let turn_west = decide(tile(105, 100), -1, 0, false, 1_050, 1_000, Some(intent), open);
        assert_eq!(turn_west, WasdDecision::Send {
            destination: tile(90, 100),
            kind: WasdSendKind::Extend,
        });

        // Collision preserved: if West is blocked by wall at 104, does not send walk
        // into wall.
        let wall_west = blocked_at(tile(104, 100));
        let blocked_turn = decide(tile(105, 100), -1, 0, false, 1_050, 1_000, Some(intent), &wall_west);
        assert_eq!(blocked_turn, WasdDecision::Silent);
    }

    #[test]
    fn diagonal_changes_repath_and_enforce_collision() {
        let intent = WasdIntent {
            target: tile(100, 115),
            step_x: 0,
            step_y: 1,
        };
        // Walking North, player adds East key -> diagonal North-East (1, 1).
        // Repaths immediately to diagonal path up to 15 cells.
        let diagonal = decide(tile(100, 105), 1, 1, false, 1_050, 1_000, Some(intent), open);
        assert_eq!(diagonal, WasdDecision::Send {
            destination: tile(115, 120),
            kind: WasdSendKind::Extend,
        });

        // Collision check: obstacle at (103, 108) stops diagonal path before obstacle.
        let obstacle = blocked_at(tile(103, 108));
        let diagonal_blocked = decide(tile(100, 105), 1, 1, false, 1_050, 1_000, Some(intent), &obstacle);
        assert_eq!(diagonal_blocked, WasdDecision::Send {
            destination: tile(102, 107),
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn invalidator_knockback_clears_intent_and_allows_fresh_resume() {
        let pre_intent = WasdIntent {
            target: tile(115, 100),
            step_x: 1,
            step_y: 0,
        };
        assert_eq!(invalidate_held_intent(WasdInvalidator::Knockback), None);
        // Player knocked back to (90, 100). Held key resumes from (90, 100) towards
        // 105, not old 115.
        let resume = decide(tile(90, 100), 1, 0, false, 2_000, 1_000, None, open);
        assert_eq!(resume, WasdDecision::Send {
            destination: tile(105, 100),
            kind: WasdSendKind::Extend,
        });
        assert_ne!(resume, WasdDecision::Send {
            destination: pre_intent.target,
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn invalidator_server_stop_clears_intent_and_allows_fresh_resume() {
        assert_eq!(invalidate_held_intent(WasdInvalidator::ServerStop), None);
        // Server stopped player at (104, 100). Resumes from (104, 100) to 119.
        let resume = decide(tile(104, 100), 1, 0, false, 2_000, 1_000, None, open);
        assert_eq!(resume, WasdDecision::Send {
            destination: tile(119, 100),
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn invalidator_warp_clears_intent_and_allows_fresh_resume() {
        assert_eq!(invalidate_held_intent(WasdInvalidator::Warp), None);
        // Warped to (50, 50). Held key resumes from (50, 50) to 65, never resends old
        // (115, 100).
        let resume = decide(tile(50, 50), 1, 0, false, 2_000, 1_000, None, open);
        assert_eq!(resume, WasdDecision::Send {
            destination: tile(65, 50),
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn invalidator_map_load_clears_intent_and_allows_fresh_resume() {
        assert_eq!(invalidate_held_intent(WasdInvalidator::MapLoad), None);
        // New map loaded at (155, 180). Resumes from (155, 180) to 170.
        let resume = decide(tile(155, 180), 1, 0, false, 2_000, 1_000, None, open);
        assert_eq!(resume, WasdDecision::Send {
            destination: tile(170, 180),
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn invalidator_death_clears_intent_and_allows_fresh_resume() {
        assert_eq!(invalidate_held_intent(WasdInvalidator::Death), None);
        // Dead player cannot resend old intent. After respawning at (155, 180), fresh
        // walk sends 170.
        let resume = decide(tile(155, 180), 1, 0, false, 2_000, 1_000, None, open);
        assert_eq!(resume, WasdDecision::Send {
            destination: tile(170, 180),
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn invalidator_stun_freeze_clears_intent_and_allows_fresh_resume() {
        assert_eq!(invalidate_held_intent(WasdInvalidator::StunFreeze), None);
        // Stunned at (102, 100). Once stun expires, resumes from (102, 100) to 117.
        let resume = decide(tile(102, 100), 1, 0, false, 2_000, 1_000, None, open);
        assert_eq!(resume, WasdDecision::Send {
            destination: tile(117, 100),
            kind: WasdSendKind::Extend,
        });
    }

    #[test]
    fn invalidator_cast_rooting_clears_intent_and_allows_fresh_resume() {
        assert_eq!(invalidate_held_intent(WasdInvalidator::CastRooting), None);
        // Rooted during spell cast at (103, 100). Once cast finishes, resumes from
        // (103, 100) to 118.
        let resume = decide(tile(103, 100), 1, 0, false, 2_000, 1_000, None, open);
        assert_eq!(resume, WasdDecision::Send {
            destination: tile(118, 100),
            kind: WasdSendKind::Extend,
        });
    }
}
