//! Where the live skill units are, so the client can answer "what am I
//! standing in?".
//!
//! Hercules gives `SC_VOLCANO`, `SC_DELUGE` and `SC_VIOLENTGALE` the **same**
//! status icon (`SI_GROUNDMAGIC`, index 112 — see `db/re/sc_config.conf`), so
//! the status packet alone cannot say which elemental field the player is in.
//! The unit spawns can: every `AddSkillUnit` carries a [`UnitId`] and a cell,
//! and this registry keeps them until the server removes them.
//!
//! Lifetime is the server's, exactly as in
//! [`EffectHolder`](crate::world::EffectHolder) — entries are added on spawn
//! and dropped on `RemoveSkillUnit`, entity removal, or map change. Nothing
//! here expires on a client timer.

use cgmath::Point3;
use ragnarok_packets::{EntityId, UnitId};

use crate::loaders::GAT_TILE_SIZE;

/// A skill unit currently on the ground.
struct ActiveSkillUnit {
    entity_id: EntityId,
    unit_id: UnitId,
    position: Point3<f32>,
}

/// Every live skill unit, keyed by the entity id the server owns it under.
#[derive(Default)]
pub struct SkillUnitRegistry {
    units: Vec<ActiveSkillUnit>,
}

impl SkillUnitRegistry {
    /// Record a spawned unit. Called for **every** unit, including ones with
    /// no presentation recipe — the registry answers questions about game
    /// state, which is independent of whether we can draw the thing.
    pub fn insert(&mut self, entity_id: EntityId, unit_id: UnitId, position: Point3<f32>) {
        self.units.push(ActiveSkillUnit {
            entity_id,
            unit_id,
            position,
        });
    }

    pub fn remove(&mut self, entity_id: EntityId) {
        self.units.retain(|unit| unit.entity_id != entity_id);
    }

    pub fn clear(&mut self) {
        self.units.clear();
    }

    /// The elemental field covering `position`, if any.
    ///
    /// A unit owns one cell, so "covering" means within half a cell of its
    /// centre; the tolerance is generous because the caller's position is a
    /// continuous world point rather than a snapped tile. Only one elemental
    /// field can exist at a spot — Hercules kills overlapping units on
    /// placement (`skill.c`: they "fail to appear when casted on top of
    /// ANYTHING") — so the nearest match is unambiguous.
    pub fn elemental_field_at(&self, position: Point3<f32>) -> Option<UnitId> {
        const REACH: f32 = GAT_TILE_SIZE * 0.75;

        self.units
            .iter()
            .filter(|unit| is_elemental_field(unit.unit_id))
            .map(|unit| {
                let offset = unit.position - position;
                // Ignore height: terrain slopes move a unit's Y away from the
                // player standing on the very same cell.
                (unit, offset.x.hypot(offset.z))
            })
            .filter(|(_, distance)| *distance <= REACH)
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(unit, _)| unit.unit_id)
    }
}

/// The three Sage fields that share `SI_GROUNDMAGIC`.
pub fn is_elemental_field(unit_id: UnitId) -> bool {
    matches!(unit_id, UnitId::Volcano | UnitId::Deluge | UnitId::Violentgale)
}

/// Display name for an elemental field unit, for the status window.
pub fn elemental_field_name(unit_id: UnitId) -> Option<&'static str> {
    match unit_id {
        UnitId::Volcano => Some("Volcano"),
        UnitId::Deluge => Some("Deluge"),
        UnitId::Violentgale => Some("Violent Gale"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, z: f32) -> Point3<f32> {
        Point3::new(x, 0.0, z)
    }

    /// `UnitId` has no `PartialEq`, and the name is what callers actually
    /// consume, so assert through it.
    fn field_at(registry: &SkillUnitRegistry, position: Point3<f32>) -> Option<&'static str> {
        registry.elemental_field_at(position).and_then(elemental_field_name)
    }

    #[test]
    fn resolves_the_field_the_player_stands_in() {
        let mut registry = SkillUnitRegistry::default();
        registry.insert(EntityId(1), UnitId::Volcano, point(100.0, 100.0));
        registry.insert(EntityId(2), UnitId::Deluge, point(500.0, 500.0));

        assert_eq!(field_at(&registry, point(101.0, 99.0)), Some("Volcano"));
        assert_eq!(field_at(&registry, point(500.0, 501.0)), Some("Deluge"));
    }

    #[test]
    fn ignores_units_that_are_not_elemental_fields() {
        let mut registry = SkillUnitRegistry::default();
        registry.insert(EntityId(1), UnitId::Firewall, point(100.0, 100.0));

        assert_eq!(field_at(&registry, point(100.0, 100.0)), None);
    }

    #[test]
    fn a_field_one_cell_away_does_not_count() {
        let mut registry = SkillUnitRegistry::default();
        registry.insert(EntityId(1), UnitId::Volcano, point(100.0, 100.0));

        // A whole cell away is a different unit's cell.
        assert_eq!(field_at(&registry, point(100.0 + GAT_TILE_SIZE, 100.0)), None);
    }

    #[test]
    fn height_differences_do_not_break_the_match() {
        let mut registry = SkillUnitRegistry::default();
        // Sloped terrain: the unit sits well above the player's feet.
        registry.insert(EntityId(1), UnitId::Volcano, Point3::new(100.0, 40.0, 100.0));

        assert_eq!(field_at(&registry, point(100.0, 100.0)), Some("Volcano"));
    }

    #[test]
    fn land_protector_is_not_an_elemental_field() {
        // It shares neither the icon nor the semantics; the server sends it
        // its own status (SI_LANDPROTECTOR), so it must never be named here.
        let mut registry = SkillUnitRegistry::default();
        registry.insert(EntityId(1), UnitId::Landprotector, point(100.0, 100.0));

        assert_eq!(field_at(&registry, point(100.0, 100.0)), None);
    }

    #[test]
    fn removal_and_clear_follow_the_server() {
        let mut registry = SkillUnitRegistry::default();
        registry.insert(EntityId(1), UnitId::Volcano, point(100.0, 100.0));
        registry.insert(EntityId(2), UnitId::Deluge, point(500.0, 500.0));

        registry.remove(EntityId(1));
        assert_eq!(field_at(&registry, point(100.0, 100.0)), None);
        assert_eq!(field_at(&registry, point(500.0, 500.0)), Some("Deluge"));

        registry.clear();
        assert_eq!(field_at(&registry, point(500.0, 500.0)), None);
    }
}
