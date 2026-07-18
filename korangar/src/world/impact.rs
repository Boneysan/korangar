use ragnarok_packets::{ClientTick, EntityId, SkillId};

/// Presentation payload retained between a damage packet and its native
/// ACT-derived impact boundary.
#[derive(Clone, Debug)]
pub(crate) struct DamageImpact {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub skill_id: Option<SkillId>,
    pub packet_tick: ClientTick,
    pub damage_amount: Option<usize>,
    pub hit_count: usize,
    pub damage_delay: u32,
    pub is_critical: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingImpact {
    pub due_tick: ClientTick,
    pub damage: DamageImpact,
}

/// Client-side equivalent of Ragexe's actor timed-message list for combat
/// presentation. Gameplay has already happened on the server; this queue only
/// delays the visible/audio target phase.
#[derive(Default)]
pub(crate) struct PendingImpactQueue {
    impacts: Vec<PendingImpact>,
}

impl PendingImpactQueue {
    pub fn schedule(&mut self, now: ClientTick, delay_ms: u32, damage: DamageImpact) {
        self.impacts.push(PendingImpact {
            due_tick: ClientTick(now.0.wrapping_add(delay_ms)),
            damage,
        });
    }

    pub fn drain_due(&mut self, now: ClientTick) -> Vec<PendingImpact> {
        let mut due = Vec::new();
        let mut waiting = Vec::with_capacity(self.impacts.len());

        for impact in self.impacts.drain(..) {
            if tick_reached(now, impact.due_tick) {
                due.push(impact);
            } else {
                waiting.push(impact);
            }
        }

        self.impacts = waiting;
        due
    }

    pub fn remove_target(&mut self, entity_id: EntityId) {
        self.impacts.retain(|impact| impact.damage.destination_entity_id != entity_id);
    }

    pub fn clear(&mut self) {
        self.impacts.clear();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.impacts.len()
    }
}

/// Wrapping u32 tick comparison valid while scheduled delays remain below
/// half the tick domain (about 24.8 days), which combat motions always do.
fn tick_reached(now: ClientTick, due: ClientTick) -> bool {
    now.0.wrapping_sub(due.0) < 0x8000_0000
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, EntityId, SkillId};

    use super::{DamageImpact, PendingImpactQueue, tick_reached};

    fn damage(target: u32) -> DamageImpact {
        DamageImpact {
            source_entity_id: EntityId(1),
            destination_entity_id: EntityId(target),
            skill_id: Some(SkillId(59)),
            packet_tick: ClientTick(100),
            damage_amount: Some(42),
            hit_count: 1,
            damage_delay: 288,
            is_critical: false,
        }
    }

    #[test]
    fn due_impacts_stay_queued_until_the_boundary() {
        let mut queue = PendingImpactQueue::default();
        queue.schedule(ClientTick(1_000), 120, damage(2));

        assert!(queue.drain_due(ClientTick(1_119)).is_empty());
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.drain_due(ClientTick(1_120))[0].damage.destination_entity_id, EntityId(2));
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn due_order_matches_packet_schedule_order() {
        let mut queue = PendingImpactQueue::default();
        queue.schedule(ClientTick(1_000), 100, damage(2));
        queue.schedule(ClientTick(1_000), 50, damage(3));

        let due = queue.drain_due(ClientTick(1_100));
        assert_eq!(due[0].damage.destination_entity_id, EntityId(2));
        assert_eq!(due[1].damage.destination_entity_id, EntityId(3));
    }

    #[test]
    fn tick_comparison_survives_u32_wraparound() {
        assert!(!tick_reached(ClientTick(u32::MAX - 2), ClientTick(3)));
        assert!(tick_reached(ClientTick(3), ClientTick(3)));
        assert!(tick_reached(ClientTick(4), ClientTick(3)));

        let mut queue = PendingImpactQueue::default();
        queue.schedule(ClientTick(u32::MAX - 2), 6, damage(2));
        assert!(queue.drain_due(ClientTick(2)).is_empty());
        assert_eq!(queue.drain_due(ClientTick(3)).len(), 1);
    }

    #[test]
    fn removing_a_target_preserves_other_impacts() {
        let mut queue = PendingImpactQueue::default();
        queue.schedule(ClientTick(0), 10, damage(2));
        queue.schedule(ClientTick(0), 10, damage(3));

        queue.remove_target(EntityId(2));

        let due = queue.drain_due(ClientTick(10));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].damage.destination_entity_id, EntityId(3));
    }

    #[test]
    fn clearing_discards_every_pending_impact() {
        let mut queue = PendingImpactQueue::default();
        queue.schedule(ClientTick(0), 10, damage(2));
        queue.schedule(ClientTick(0), 20, damage(3));

        queue.clear();

        assert_eq!(queue.len(), 0);
        assert!(queue.drain_due(ClientTick(20)).is_empty());
    }
}
