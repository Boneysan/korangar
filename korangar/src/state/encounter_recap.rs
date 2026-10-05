//! End-of-encounter boss combat recap (GDD §6.9, §16 / F16).
//!
//! Tracks active MVP and boss combat strictly from server-confirmed events
//! (DamageEffect, SkillCast interrupts, EntityDisappearance). Upon boss
//! defeat, produces an aggregated summary of damage dealt, damage taken,
//! encounter duration, and MVP recognition without altering gameplay or
//! forcing scripted cutscenes.

use ragnarok_packets::{ClientTick, EntityId};

/// A finalized recap of a concluded boss encounter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncounterRecap {
    pub boss_name: String,
    pub boss_entity_id: EntityId,
    pub duration_ms: u32,
    pub damage_dealt: u32,
    pub damage_taken: u32,
    pub casts_interrupted: u32,
    pub mvp_player_name: Option<String>,
    /// The reward's display name, resolved when it arrived: user-facing text
    /// never shows a raw item id.
    pub mvp_reward: Option<String>,
    pub mvp_experience: Option<u32>,
}

impl EncounterRecap {
    /// The defeat toast: one line with every tracked number, so damage taken
    /// and interrupts are not only recorded but seen (F16).
    pub fn toast_text(&self) -> String {
        let casts = match self.casts_interrupted {
            1 => "1 cast".to_owned(),
            count => format!("{count} casts"),
        };
        format!(
            "Defeated {} in {:.1}s: dealt {}, took {}, interrupted {casts}",
            self.boss_name,
            (self.duration_ms as f32) / 1000.0,
            self.damage_dealt,
            self.damage_taken,
        )
    }

    /// The full recap, posted to chat when the boss falls so it outlives the
    /// toast.
    pub fn format_summary(&self) -> String {
        let seconds = (self.duration_ms as f32) / 1000.0;
        let mut lines = vec![
            format!("=== Encounter Recap: {} ===", self.boss_name),
            format!("Duration: {:.1}s", seconds),
            format!("Damage Dealt: {}", self.damage_dealt),
            format!("Damage Taken: {}", self.damage_taken),
            format!("Enemy Casts Interrupted: {}", self.casts_interrupted),
        ];

        if let Some(mvp) = &self.mvp_player_name {
            lines.push(format!("MVP: {mvp}"));
        }
        if let Some(reward) = &self.mvp_reward {
            lines.push(format!("MVP Reward: {reward}"));
        }
        if let Some(experience) = self.mvp_experience {
            lines.push(format!("MVP Bonus EXP: {experience}"));
        }

        lines.join("\n")
    }
}

/// How long before the boss's death an MVP packet may arrive and still be
/// this kill's. The server sends both in one tick; this allows for jitter.
const MVP_WINDOW_MS: u32 = 2000;

/// Accumulator tracking boss engagement state.
#[derive(Clone, Debug, Default)]
pub struct EncounterRecapState {
    active_boss_id: Option<EntityId>,
    active_boss_name: String,
    start_tick: Option<ClientTick>,
    damage_dealt: u32,
    damage_taken: u32,
    casts_interrupted: u32,
    pending_mvp_name: Option<String>,
    pending_mvp_reward: Option<String>,
    pending_mvp_experience: Option<u32>,
    /// When the last MVP packet arrived.
    pending_mvp_tick: Option<u32>,
    latest_recap: Option<EncounterRecap>,
}

impl EncounterRecapState {
    /// Begin or continue tracking an encounter with a verified MVP / Boss
    /// entity.
    pub fn start_or_continue_encounter(&mut self, boss_id: EntityId, boss_name: &str, now: ClientTick) {
        if self.active_boss_id == Some(boss_id) {
            return;
        }

        // If switching to a new boss, reset active accumulator.
        self.active_boss_id = Some(boss_id);
        self.active_boss_name = boss_name.to_owned();
        self.start_tick = Some(now);
        self.damage_dealt = 0;
        self.damage_taken = 0;
        self.casts_interrupted = 0;
        self.pending_mvp_name = None;
        self.pending_mvp_reward = None;
        self.pending_mvp_experience = None;
        self.pending_mvp_tick = None;
    }

    /// Record damage dealt by the player to a target entity.
    pub fn record_damage_dealt(&mut self, target_id: EntityId, amount: u32) {
        if self.active_boss_id == Some(target_id) {
            self.damage_dealt = self.damage_dealt.saturating_add(amount);
        }
    }

    /// Record damage taken by the player from an attacking entity.
    pub fn record_damage_taken(&mut self, attacker_id: EntityId, amount: u32) {
        if self.active_boss_id == Some(attacker_id) {
            self.damage_taken = self.damage_taken.saturating_add(amount);
        }
    }

    /// Record an interrupted enemy cast.
    pub fn record_cast_interrupted(&mut self, caster_id: EntityId) {
        if self.active_boss_id == Some(caster_id) {
            self.casts_interrupted = self.casts_interrupted.saturating_add(1);
        }
    }

    /// Record the MVP announcement (0x010C), the MVP's reward (0x010A, its
    /// resolved name) or bonus EXP (0x010B). The server sends them just before
    /// the boss's death packet, so they attach to the encounter it closes.
    /// None of them names the monster, so they are taken only while a boss
    /// is being tracked: an MVP killed by strangers nearby is not ours.
    pub fn record_mvp_award(&mut self, player_name: Option<String>, reward: Option<String>, experience: Option<u32>, now: ClientTick) {
        if self.active_boss_id.is_none() {
            return;
        }
        self.pending_mvp_tick = Some(now.0);
        if player_name.is_some() {
            self.pending_mvp_name = player_name;
        }
        if reward.is_some() {
            self.pending_mvp_reward = reward;
        }
        if experience.is_some() {
            self.pending_mvp_experience = experience;
        }
    }

    /// Conclude the encounter when the boss entity dies.
    pub fn finish_encounter(&mut self, boss_id: EntityId, now: ClientTick) -> Option<EncounterRecap> {
        if self.active_boss_id != Some(boss_id) {
            return None;
        }

        // `mob_dead` sends the MVP packets in the same server tick as the death
        // packet. Anything older belongs to some other kill nearby.
        if self.pending_mvp_tick.is_some_and(|tick| now.0.saturating_sub(tick) > MVP_WINDOW_MS) {
            self.pending_mvp_name = None;
            self.pending_mvp_reward = None;
            self.pending_mvp_experience = None;
        }
        self.pending_mvp_tick = None;

        let start = self.start_tick.unwrap_or(now);
        let duration_ms = now.0.saturating_sub(start.0);

        let recap = EncounterRecap {
            boss_name: std::mem::take(&mut self.active_boss_name),
            boss_entity_id: boss_id,
            duration_ms,
            damage_dealt: self.damage_dealt,
            damage_taken: self.damage_taken,
            casts_interrupted: self.casts_interrupted,
            mvp_player_name: self.pending_mvp_name.take(),
            mvp_reward: self.pending_mvp_reward.take(),
            mvp_experience: self.pending_mvp_experience.take(),
        };

        self.active_boss_id = None;
        self.start_tick = None;
        self.damage_dealt = 0;
        self.damage_taken = 0;
        self.casts_interrupted = 0;

        self.latest_recap = Some(recap.clone());
        Some(recap)
    }

    /// Reset tracking upon map change or target abandonment without victory.
    pub fn clear_active(&mut self) {
        self.active_boss_id = None;
        self.active_boss_name.clear();
        self.start_tick = None;
        self.damage_dealt = 0;
        self.damage_taken = 0;
        self.casts_interrupted = 0;
        self.pending_mvp_name = None;
        self.pending_mvp_reward = None;
        self.pending_mvp_experience = None;
        self.pending_mvp_tick = None;
    }

    /// Get the most recently completed encounter recap, if any.
    #[cfg(test)]
    pub fn latest_recap(&self) -> Option<&EncounterRecap> {
        self.latest_recap.as_ref()
    }

    /// Whether a boss encounter is actively being tracked.
    #[cfg(test)]
    pub fn is_tracking_boss(&self, boss_id: EntityId) -> bool {
        self.active_boss_id == Some(boss_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(val: u32) -> EntityId {
        EntityId(val)
    }

    fn tick(ms: u32) -> ClientTick {
        ClientTick(ms)
    }

    #[test]
    fn encounter_lifecycle_tracks_combat_and_produces_recap() {
        let mut state = EncounterRecapState::default();
        let boss = id(1115); // Eddga
        let minion = id(1002);

        state.start_or_continue_encounter(boss, "Eddga", tick(1000));
        assert!(state.is_tracking_boss(boss));

        // Damage to boss is accumulated.
        state.record_damage_dealt(boss, 500);
        state.record_damage_dealt(boss, 750);
        // Damage to minion is ignored by boss recap.
        state.record_damage_dealt(minion, 200);

        // Damage received from boss is accumulated.
        state.record_damage_taken(boss, 300);
        state.record_damage_taken(minion, 50);

        // Interrupted cast.
        state.record_cast_interrupted(boss);
        state.record_cast_interrupted(minion);

        // MVP award.
        state.record_mvp_award(Some("Hero".to_string()), None, None, tick(23_900));
        state.record_mvp_award(None, Some("Red Potion".to_string()), None, tick(23_900));
        state.record_mvp_award(None, None, Some(4200), tick(23_900));

        // Boss defeat at tick 25000 (24.0s elapsed).
        let recap = state.finish_encounter(boss, tick(25000)).expect("recap produced");

        assert_eq!(recap.boss_name, "Eddga");
        assert_eq!(recap.boss_entity_id, boss);
        assert_eq!(recap.duration_ms, 24000);
        assert_eq!(recap.damage_dealt, 1250);
        assert_eq!(recap.damage_taken, 300);
        assert_eq!(recap.casts_interrupted, 1);
        assert_eq!(recap.mvp_player_name.as_deref(), Some("Hero"));
        assert_eq!(recap.mvp_reward.as_deref(), Some("Red Potion"));
        assert_eq!(recap.mvp_experience, Some(4200));

        // Active state is cleared.
        assert!(!state.is_tracking_boss(boss));
        assert_eq!(state.latest_recap(), Some(&recap));

        let summary = recap.format_summary();
        assert!(summary.contains("Encounter Recap: Eddga"));
        assert!(summary.contains("Duration: 24.0s"));
        assert!(summary.contains("Damage Dealt: 1250"));
        assert!(summary.contains("Damage Taken: 300"));
        assert!(summary.contains("Enemy Casts Interrupted: 1"));
        assert!(summary.contains("MVP: Hero"));
        assert!(summary.contains("MVP Reward: Red Potion"), "{summary}");
        assert!(summary.contains("MVP Bonus EXP: 4200"), "{summary}");

        // The toast carries every tracked number, not only damage dealt.
        assert_eq!(
            recap.toast_text(),
            "Defeated Eddga in 24.0s: dealt 1250, took 300, interrupted 1 cast"
        );
    }

    #[test]
    fn toast_counts_only_the_bosss_own_hits_and_casts() {
        let mut state = EncounterRecapState::default();
        let (boss, minion) = (id(1115), id(2000));
        state.start_or_continue_encounter(boss, "Eddga", tick(0));
        state.record_damage_taken(boss, 400);
        state.record_damage_taken(minion, 999);
        state.record_cast_interrupted(minion);
        state.record_cast_interrupted(boss);
        state.record_cast_interrupted(boss);

        let recap = state.finish_encounter(boss, tick(1500)).expect("tracked");
        assert_eq!(
            recap.toast_text(),
            "Defeated Eddga in 1.5s: dealt 0, took 400, interrupted 2 casts"
        );
    }

    #[test]
    fn map_change_or_escape_clears_active_without_corrupting_latest() {
        let mut state = EncounterRecapState::default();
        let boss = id(1150); // Moonlight Flower

        state.start_or_continue_encounter(boss, "Moonlight Flower", tick(100));
        state.record_damage_dealt(boss, 1000);

        // Map change clears active
        state.clear_active();
        assert!(!state.is_tracking_boss(boss));
        assert_eq!(state.latest_recap(), None);

        // Finishing a cleared encounter produces nothing
        assert_eq!(state.finish_encounter(boss, tick(500)), None);
    }

    #[test]
    fn another_kills_mvp_during_our_fight_is_not_ours() {
        let mut state = EncounterRecapState::default();
        let boss = id(1115);
        state.start_or_continue_encounter(boss, "Eddga", tick(0));
        // Strangers finish a different MVP nearby, 20 s into our fight.
        state.record_mvp_award(Some("Stranger".to_string()), None, None, tick(20_000));
        let recap = state.finish_encounter(boss, tick(40_000)).expect("tracked");
        assert_eq!(recap.mvp_player_name, None);
        assert!(!recap.format_summary().contains("MVP"), "{}", recap.format_summary());

        // Packets that arrive with the death are kept.
        state.start_or_continue_encounter(boss, "Eddga", tick(50_000));
        state.record_mvp_award(Some("Hero".to_string()), None, None, tick(59_990));
        let recap = state.finish_encounter(boss, tick(60_000)).expect("tracked");
        assert_eq!(recap.mvp_player_name.as_deref(), Some("Hero"));
    }

    #[test]
    fn an_mvp_nobody_here_was_fighting_is_ignored() {
        let mut state = EncounterRecapState::default();
        state.record_mvp_award(Some("Stranger".to_string()), None, None, tick(0));
        let boss = id(1115);
        state.start_or_continue_encounter(boss, "Eddga", tick(0));
        let recap = state.finish_encounter(boss, tick(1000)).expect("tracked");
        assert_eq!(recap.mvp_player_name, None);
    }
}
