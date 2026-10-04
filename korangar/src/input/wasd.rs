//! WASD walk-request policy.
//!
//! The client still talks to the map server with `RequestPlayerMovePacket`
//! (`0x035F`). This module only decides whether a key event is allowed to send
//! and which tile it names.
//!
//! Ported from the closed PR #10 (7e21ee57, c481185f), with a corner rule
//! and a wall slide added on `main`.
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

    // A diagonal blocked at the first cell slides along whichever axis is
    // open, so a held key follows a wall instead of stopping dead at it.
    let furthest = straight_path(input.start, input.step_x, input.step_y, &walkable)
        .or_else(|| {
            (input.step_y != 0)
                .then(|| straight_path(input.start, input.step_x, 0, &walkable))
                .flatten()
        })
        .or_else(|| {
            (input.step_x != 0)
                .then(|| straight_path(input.start, 0, input.step_y, &walkable))
                .flatten()
        });

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

fn offset(tile: TilePosition, step_x: i32, step_y: i32) -> Option<TilePosition> {
    let x = u16::try_from(tile.x as i32 + step_x).ok()?;
    let y = u16::try_from(tile.y as i32 + step_y).ok()?;
    Some(TilePosition { x, y })
}

/// Furthest tile of the straight walkable line from `start`, up to
/// [`HELD_PATH`] cells. A diagonal step also needs both orthogonal neighbours
/// open, because Hercules' `path_search` (`path.c`, `chk_dir`) will not cut a
/// wall corner: a line that does is walked by a different route server-side,
/// and the client's prediction snaps back.
fn straight_path(start: TilePosition, step_x: i32, step_y: i32, walkable: &impl Fn(TilePosition) -> bool) -> Option<TilePosition> {
    if step_x == 0 && step_y == 0 {
        return None;
    }
    let mut here = start;
    let mut furthest = None;
    for _ in 0..HELD_PATH {
        let Some(next) = offset(here, step_x, step_y) else {
            break;
        };
        let corner_open =
            step_x == 0 || step_y == 0 || (offset(here, step_x, 0).is_some_and(walkable) && offset(here, 0, step_y).is_some_and(walkable));
        if !corner_open || !walkable(next) {
            break;
        }
        furthest = Some(next);
        here = next;
    }
    furthest
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
    fn a_diagonal_does_not_cut_a_wall_corner() {
        // Wall east of (100,100): Hercules will not step (100,100)->(101,101).
        let decision = decide(tile(100, 100), 1, 1, true, 1_000, 0, None, blocked_at(tile(101, 100)));
        // It slides north along the wall instead of cutting the corner.
        assert_eq!(decision, WasdDecision::Send {
            destination: tile(100, 115),
            kind: WasdSendKind::Press,
        });
    }

    #[test]
    fn a_diagonal_stops_before_a_corner_further_along() {
        // The corner at (103,102) blocks the step (102,102)->(103,103).
        let decision = decide(tile(100, 100), 1, 1, true, 1_000, 0, None, blocked_at(tile(103, 102)));
        assert_eq!(decision, WasdDecision::Send {
            destination: tile(102, 102),
            kind: WasdSendKind::Press,
        });
    }

    #[test]
    fn a_blocked_diagonal_slides_along_the_open_axis() {
        // Row y=101 is a wall; east along y=100 stays open.
        let decision = decide(tile(100, 100), 1, 1, true, 1_000, 0, None, |t: TilePosition| t.y != 101);
        assert_eq!(decision, WasdDecision::Send {
            destination: tile(115, 100),
            kind: WasdSendKind::Press,
        });
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
}
