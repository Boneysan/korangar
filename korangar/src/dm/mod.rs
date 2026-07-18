//! Seal Cascade DM campaign systems.
//!
//! Kept isolated (with `interface/windows/dm/`) so the fork stays rebaseable
//! against upstream Korangar — see `CLAUDE.md` and `docs/DM_DATA_GUIDE.md`.

pub mod data;
pub mod loot;

pub use data::{BestiaryMonster, dm_data};
use korangar_interface::element::StateElement;
pub use loot::{LootDifficulty, generate_loot};
use rust_state::RustState;

/// Campaign-persistent DM state (session-scoped for now).
#[derive(Default, RustState, StateElement)]
pub struct DmCampaignState {
    /// Mob IDs the party has defeated — unlocks bestiary journal entries.
    pub bestiary_unlocked: Vec<u32>,
}
