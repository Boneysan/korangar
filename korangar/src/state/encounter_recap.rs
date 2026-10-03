//! End-of-encounter boss combat recap (GDD §6.9, §16 / F16).
//!
//! Tracks active MVP and boss combat strictly from server-confirmed events
//! (DamageEffect, SkillCast interrupts, EntityDisappearance). Upon boss
//! defeat, produces an aggregated summary of damage dealt, damage taken,
//! encounter duration, and MVP recognition without altering gameplay or
//! forcing scripted cutscenes.

use ragnarok_packets::{ClientTick, EntityId, ItemId};

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
    pub mvp_reward_item_id: Option<ItemId>,
}

impl EncounterRecap {
    /// Format a concise, readable multi-line recap for display in the HUD or
    /// combat log.
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
        if let Some(item_id) = self.mvp_reward_item_id {
            lines.push(format!("MVP Reward Item ID: {}", item_id.0));
        }

        lines.join("\n")
    }
}

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
    pending_mvp_reward: Option<ItemId>,
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

    /// Record MVP award announcement or bonus item packet.
    pub fn record_mvp_award(&mut self, player_name: Option<String>, reward_item: Option<ItemId>) {
        if player_name.is_some() {
            self.pending_mvp_name = player_name;
        }
        if reward_item.is_some() {
            self.pending_mvp_reward = reward_item;
        }
    }

    /// Conclude the encounter when the boss entity dies.
    pub fn finish_encounter(&mut self, boss_id: EntityId, now: ClientTick) -> Option<EncounterRecap> {
        if self.active_boss_id != Some(boss_id) {
            return None;
        }

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
            mvp_reward_item_id: self.pending_mvp_reward.take(),
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
    }

    /// Get the most recently completed encounter recap, if any.
    pub fn latest_recap(&self) -> Option<&EncounterRecap> {
        self.latest_recap.as_ref()
    }

    /// Dismiss the latest recap dialog.
    pub fn dismiss_latest(&mut self) {
        self.latest_recap = None;
    }

    /// Whether a boss encounter is actively being tracked.
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
        state.record_mvp_award(Some("Hero".to_string()), Some(ItemId(501)));

        // Boss defeat at tick 25000 (24.0s elapsed).
        let recap = state.finish_encounter(boss, tick(25000)).expect("recap produced");

        assert_eq!(recap.boss_name, "Eddga");
        assert_eq!(recap.boss_entity_id, boss);
        assert_eq!(recap.duration_ms, 24000);
        assert_eq!(recap.damage_dealt, 1250);
        assert_eq!(recap.damage_taken, 300);
        assert_eq!(recap.casts_interrupted, 1);
        assert_eq!(recap.mvp_player_name.as_deref(), Some("Hero"));
        assert_eq!(recap.mvp_reward_item_id, Some(ItemId(501)));

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
}
