//! Restrained client-triggered audio cues (GDD 16.1-16.3).
//!
//! Every cue is optional reinforcement: each one also has a visible toast,
//! telegraph, or marker, and the whole set can be switched off in the game
//! settings. The limiter keeps a burst of events (a pack of casters, a card
//! shower, a spamming ping) from becoming a wall of sound.
//!
//! The sound paths were each confirmed present in `data.grf` with
//! `tools/grf_list.py`. Whether a sound *fits* its cue is an ear judgement
//! that has not been made yet; swap the path here, nothing else changes.

use std::collections::VecDeque;

use ragnarok_packets::EntityId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCue {
    /// An enemy begins a long cast aimed at you or the ground.
    DangerousCast,
    /// An enemy cast was cancelled by the party.
    Interrupt,
    QuestComplete,
    PartyPing,
    CardDrop,
}

impl AudioCue {
    const ALL: [AudioCue; 5] = [
        AudioCue::DangerousCast,
        AudioCue::Interrupt,
        AudioCue::QuestComplete,
        AudioCue::PartyPing,
        AudioCue::CardDrop,
    ];

    pub fn path(self) -> &'static str {
        match self {
            Self::DangerousCast => "data\\wav\\effect\\warning.wav",
            Self::Interrupt => "data\\wav\\_stun.wav",
            Self::QuestComplete => "data\\wav\\effect\\complete.wav",
            Self::PartyPing => "data\\wav\\party_alarm.wav",
            Self::CardDrop => "data\\wav\\effect\\p_success.wav",
        }
    }

    /// Minimum gap between two plays of the same cue.
    fn cooldown_ms(self) -> u32 {
        match self {
            Self::DangerousCast => 4_000,
            Self::Interrupt => 1_500,
            Self::QuestComplete => 3_000,
            Self::PartyPing => 3_000,
            Self::CardDrop => 2_000,
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|cue| *cue == self).unwrap_or(0)
    }
}

/// A cast shorter than this is not worth a warning sound.
pub const DANGEROUS_CAST_MINIMUM_MS: u32 = 1_000;

/// Whether a hostile monster's cast should raise [`AudioCue::DangerousCast`]:
/// it is long enough to react to, and aimed at you, at the ground, or centred
/// on the caster. A cast aimed at someone else is not a warning to you.
///
/// Ground casts arrive with target id 0, and a self-centred burst targets its
/// own caster; both are assumptions about what the server sends (see the plan's
/// F35 row), which is why they are pinned here by name.
pub fn is_dangerous_cast(cast_ms: u32, target: EntityId, caster: EntityId, local_player: Option<EntityId>) -> bool {
    cast_ms >= DANGEROUS_CAST_MINIMUM_MS && (target.0 == 0 || target == caster || Some(target) == local_player)
}

const GLOBAL_WINDOW_MS: u32 = 5_000;
const GLOBAL_MAXIMUM_PER_WINDOW: usize = 4;

fn elapsed(now: u32, earlier: u32) -> u32 {
    now.wrapping_sub(earlier)
}

#[derive(Debug, Default)]
pub struct AudioCueLimiter {
    last_played: [Option<u32>; 5],
    recent: VecDeque<u32>,
}

impl AudioCueLimiter {
    /// Whether `cue` may play at `now` (client tick, milliseconds). Records the
    /// play when it returns true.
    pub fn allow(&mut self, cue: AudioCue, now: u32) -> bool {
        while self
            .recent
            .front()
            .is_some_and(|played| elapsed(now, *played) >= GLOBAL_WINDOW_MS && elapsed(now, *played) < (1 << 31))
        {
            self.recent.pop_front();
        }
        if self.recent.len() >= GLOBAL_MAXIMUM_PER_WINDOW {
            return false;
        }
        let slot = &mut self.last_played[cue.index()];
        if slot.is_some_and(|last| elapsed(now, last) < cue.cooldown_ms()) {
            return false;
        }
        *slot = Some(now);
        self.recent.push_back(now);
        true
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cue_is_rate_limited_per_kind() {
        let mut limiter = AudioCueLimiter::default();
        assert!(limiter.allow(AudioCue::PartyPing, 1_000));
        assert!(!limiter.allow(AudioCue::PartyPing, 2_000), "ping spam is dropped");
        assert!(limiter.allow(AudioCue::PartyPing, 4_000));
    }

    #[test]
    fn different_cues_do_not_block_each_other_until_the_global_cap() {
        let mut limiter = AudioCueLimiter::default();
        assert!(limiter.allow(AudioCue::DangerousCast, 0));
        assert!(limiter.allow(AudioCue::Interrupt, 10));
        assert!(limiter.allow(AudioCue::QuestComplete, 20));
        assert!(limiter.allow(AudioCue::PartyPing, 30));
        assert!(
            !limiter.allow(AudioCue::CardDrop, 40),
            "fifth sound inside the window is dropped"
        );
        assert!(limiter.allow(AudioCue::CardDrop, GLOBAL_WINDOW_MS + 1));
    }

    #[test]
    fn the_limiter_survives_client_tick_wraparound() {
        let mut limiter = AudioCueLimiter::default();
        let near_wrap = u32::MAX - 500;
        assert!(limiter.allow(AudioCue::CardDrop, near_wrap));
        assert!(!limiter.allow(AudioCue::CardDrop, near_wrap.wrapping_add(1_000)));
        assert!(limiter.allow(AudioCue::CardDrop, near_wrap.wrapping_add(2_500)));
    }

    #[test]
    fn every_cue_points_at_a_distinct_data_grf_wav() {
        let mut paths: Vec<&str> = AudioCue::ALL.iter().map(|cue| cue.path()).collect();
        assert!(paths.iter().all(|path| path.starts_with("data\\wav\\") && path.ends_with(".wav")));
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), AudioCue::ALL.len());
    }

    fn id(value: u32) -> EntityId {
        EntityId(value)
    }

    /// A short cast is not worth interrupting your play for; the minimum is
    /// inclusive.
    #[test]
    fn a_short_cast_is_not_a_warning() {
        let minimum = super::DANGEROUS_CAST_MINIMUM_MS;
        assert!(!super::is_dangerous_cast(minimum - 1, id(0), id(900), Some(id(1))));
        assert!(super::is_dangerous_cast(minimum, id(0), id(900), Some(id(1))));
    }

    /// The three shapes of "aimed at you": the ground, itself, or you.
    #[test]
    fn a_long_cast_warns_when_aimed_at_the_ground_the_caster_or_the_local_player() {
        let long = super::DANGEROUS_CAST_MINIMUM_MS + 500;
        assert!(super::is_dangerous_cast(long, id(0), id(900), Some(id(1))), "ground cast");
        assert!(
            super::is_dangerous_cast(long, id(900), id(900), Some(id(1))),
            "self-centred burst"
        );
        assert!(super::is_dangerous_cast(long, id(1), id(900), Some(id(1))), "aimed at you");
    }

    /// A cast aimed at a party member or another monster is not a warning to
    /// you.
    #[test]
    fn a_long_cast_aimed_at_someone_else_stays_silent() {
        let long = super::DANGEROUS_CAST_MINIMUM_MS + 500;
        assert!(!super::is_dangerous_cast(long, id(2), id(900), Some(id(1))));
        assert!(!super::is_dangerous_cast(long, id(2), id(900), None), "no local player yet");
    }

    #[test]
    fn all_cue_kinds_enforce_their_individual_cooldowns() {
        for cue in AudioCue::ALL {
            let mut limiter = AudioCueLimiter::default();
            assert!(limiter.allow(cue, 10_000), "{cue:?} should be allowed initially");
            assert!(
                !limiter.allow(cue, 10_000 + cue.cooldown_ms() - 1),
                "{cue:?} should be blocked 1ms before cooldown expires"
            );
            assert!(
                limiter.allow(cue, 10_000 + cue.cooldown_ms()),
                "{cue:?} should be allowed at exact cooldown expiry"
            );
        }
    }

    #[test]
    fn rapid_firing_drops_all_intermediate_attempts_under_burst() {
        for cue in AudioCue::ALL {
            let mut limiter = AudioCueLimiter::default();
            let base_tick = 50_000;
            let mut allowed_count = 0;
            for offset in (0..cue.cooldown_ms()).step_by(50) {
                if limiter.allow(cue, base_tick + offset) {
                    allowed_count += 1;
                }
            }
            assert_eq!(allowed_count, 1, "{cue:?} allowed more than once during rapid firing");
            assert!(
                limiter.allow(cue, base_tick + cue.cooldown_ms()),
                "{cue:?} should be allowed after cooldown"
            );
        }
    }

    #[test]
    fn limiter_clear_resets_cooldowns_and_window_history() {
        let mut limiter = AudioCueLimiter::default();
        assert!(limiter.allow(AudioCue::DangerousCast, 1_000));
        assert!(limiter.allow(AudioCue::Interrupt, 1_010));
        assert!(limiter.allow(AudioCue::QuestComplete, 1_020));
        assert!(limiter.allow(AudioCue::PartyPing, 1_030));
        assert!(!limiter.allow(AudioCue::CardDrop, 1_040), "global cap reached");

        limiter.clear();
        // Immediately after clear, both per-cue and global history are reset.
        assert!(limiter.allow(AudioCue::DangerousCast, 1_040));
        assert!(limiter.allow(AudioCue::CardDrop, 1_040));
    }

    #[test]
    fn sliding_window_eviction_restores_capacity() {
        let mut limiter = AudioCueLimiter::default();
        assert!(limiter.allow(AudioCue::DangerousCast, 100));
        assert!(limiter.allow(AudioCue::Interrupt, 200));
        assert!(limiter.allow(AudioCue::QuestComplete, 300));
        assert!(limiter.allow(AudioCue::PartyPing, 400));
        assert!(!limiter.allow(AudioCue::CardDrop, 500), "5th cue blocked in window");

        // After 100 + GLOBAL_WINDOW_MS, the first event expires from the window.
        // We can now play CardDrop at tick 100 + GLOBAL_WINDOW_MS + 1.
        let next_tick = 100 + GLOBAL_WINDOW_MS + 1;
        assert!(
            limiter.allow(AudioCue::CardDrop, next_tick),
            "window eviction should allow new cue"
        );
    }
}
