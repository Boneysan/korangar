//! Why the character is, or is not, regenerating (fork packet 0x0EFD).
//!
//! The server decides this (`status_recovery_ui_state`) and sends it only when
//! it changes, so the last value stays valid until the next packet. Both bytes
//! are kept raw: the server may add a value before this client knows it, and an
//! unknown value must degrade to a generic line, never to nothing.

use korangar_interface::element::StateElement;
use rust_state::RustState;

// `enum recovery_mode` in Hercules `src/map/combat_state.h`.
const MODE_SITTING: u8 = 2;
const MODE_RESPAWN: u8 = 3;
// `enum recovery_block`.
const BLOCK_OK: u8 = 0;
const BLOCK_DEAD: u8 = 1;
const BLOCK_STATUS: u8 = 2;
const BLOCK_WEIGHT: u8 = 3;
const BLOCK_COMBAT: u8 = 4;

#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct RecoveryState {
    #[hidden_element]
    mode: u8,
    #[hidden_element]
    block: u8,
    /// The HUD line. Empty when nothing is worth saying: ordinary standing
    /// recovery needs no notice.
    display_text: String,
}

impl RecoveryState {
    pub fn set(&mut self, mode: u8, block: u8) {
        self.mode = mode;
        self.block = block;
        self.display_text = describe(mode, block).to_owned();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    #[allow(dead_code)]
    pub fn display_text(&self) -> &str {
        &self.display_text
    }
}

/// A blocking reason outranks the mode: it is what the player can act on.
fn describe(mode: u8, block: u8) -> &'static str {
    match block {
        BLOCK_OK => match mode {
            MODE_SITTING => "Resting: fast recovery",
            MODE_RESPAWN => "Recovering after respawn",
            _ => "",
        },
        // Dead: the respawn window already says so.
        BLOCK_DEAD => "",
        BLOCK_STATUS => "Recovery blocked: status effect",
        BLOCK_WEIGHT => "Recovery blocked: overweight",
        BLOCK_COMBAT => "Recovery paused: in combat",
        _ => "Recovery blocked",
    }
}

#[cfg(test)]
mod tests {
    use super::RecoveryState;

    fn text(mode: u8, block: u8) -> String {
        let mut state = RecoveryState::default();
        state.set(mode, block);
        state.display_text().to_owned()
    }

    #[test]
    fn sitting_and_respawn_are_announced() {
        assert_eq!(text(2, 0), "Resting: fast recovery");
        assert_eq!(text(3, 0), "Recovering after respawn");
    }

    #[test]
    fn ordinary_standing_recovery_is_silent() {
        assert_eq!(text(1, 0), "");
        assert_eq!(text(0, 0), "");
    }

    /// The server sends mode NONE with the block for status/weight, and mode
    /// STANDING with block COMBAT; each must read as its own reason.
    #[test]
    fn each_blocking_reason_has_its_own_line() {
        assert_eq!(text(0, 2), "Recovery blocked: status effect");
        assert_eq!(text(0, 3), "Recovery blocked: overweight");
        assert_eq!(text(1, 4), "Recovery paused: in combat");
        assert_eq!(text(0, 1), "");
    }

    /// A blocking reason wins over sitting: sitting while overweight does not
    /// recover.
    #[test]
    fn a_block_outranks_the_mode() {
        assert_eq!(text(2, 3), "Recovery blocked: overweight");
    }

    /// A block the client has not heard of still shows something.
    #[test]
    fn an_unknown_block_degrades_to_a_generic_line() {
        assert_eq!(text(1, 200), "Recovery blocked");
    }

    #[test]
    fn clear_empties_the_line() {
        let mut state = RecoveryState::default();
        state.set(2, 0);
        state.clear();
        assert_eq!(state.display_text(), "");
    }
}
