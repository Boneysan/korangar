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
    if input.client_tick.wrapping_sub(input.last_tick) < THROTTLE_MS {
        return WasdDecision::Silent;
    }
    if input.step_x == 0 && input.step_y == 0 {
        return WasdDecision::Silent;
    }

    let repath = match input.current {
        None => true,
        Some(intent) => {
            input.fresh
                || intent.step_x != input.step_x
                || intent.step_y != input.step_y
                || chebyshev(input.start, intent.target) <= REFRESH_WITHIN
        }
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
        Some(destination) if destination != input.start => {
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
}
