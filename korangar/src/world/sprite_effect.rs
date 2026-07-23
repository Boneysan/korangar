//! Classic sprite-based skill effects.
//!
//! RO ships effects in two families. Keyframe scripts under
//! `data\texture\effect\*.str` back the ground and AoE spells, and
//! [`EffectLoader`](crate::loaders::EffectLoader) already plays those. The
//! classic single-target spells instead use sprite animations under
//! `data\sprite\이팩트\*.spr` + `.act`, and have **no `.str` equivalent in the
//! GRFs at all** — probing confirmed there is no `napalmbeat.str`,
//! `soulstrike.str`, `fireball.str`, `jupitel.str`, `earthspike.str`, or
//! `heavendrive.str`, while the ground-spell control group is all present.
//!
//! That gap is why those skills previously fell back to procedural stand-ins.
//! This module plays the real sprites instead, reusing the same
//! [`AnimationData::render_action_frame`] path the emote balloons use, so ACT
//! frame timing, multi-layer frames, and per-frame offsets all behave exactly
//! as they do for entities.
//!
//! Two placement modes:
//! - **Fixed** — play at a world point (impact bursts, ground spikes).
//! - **Travel** — lerp from caster to target over a duration (Soul Strike
//!   ghosts). Motion lifetime is independent of ACT length so a long impact
//!   delay still carries the sprite the whole way.
//!
//! See `docs/plans/classic-effect-fidelity.md`.

use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Point3, Vector3};
use ragnarok_packets::{ClientTick, EntityId};

use crate::graphics::EntityInstruction;
use crate::world::{AnimationData, Camera};

/// Extra display time after a fixed sprite effect finishes, so the final frame
/// is not cut off abruptly.
const LINGER_MS: u32 = 120;

/// Lifetime bound used while the animation data is still loading, so a spawn
/// whose sprite never arrives cannot leak.
const FALLBACK_LIFETIME_MS: u32 = 3000;

/// Sentinel entity IDs routing async animation-data loads back here. Counts
/// down from just below [`EMOTE_ANIMATION_ENTITY_ID`](super::EMOTE_ANIMATION_ENTITY_ID)
/// (`u32::MAX`) so the two sentinel spaces can never collide.
const SENTINEL_BASE: u32 = u32::MAX - 1;

/// Diagnostics for the sprite-effect pipeline, enabled by setting the
/// `KORANGAR_SPRITE_EFFECT_DEBUG` environment variable.
pub fn sprite_effect_debug_enabled() -> bool {
    std::env::var_os("KORANGAR_SPRITE_EFFECT_DEBUG").is_some()
}

/// How a sprite effect moves while its ACT plays.
#[derive(Clone, Copy)]
enum SpriteMotion {
    /// Stay at a fixed world point (hit / ground anchors).
    Fixed { position: Point3<f32> },
    /// Lerp from `from` to `to` over `duration_ms`, after an optional
    /// `start_delay_ms` (used to stagger multi-hit Soul Strike volleys).
    Travel {
        from: Point3<f32>,
        to: Point3<f32>,
        duration_ms: u32,
        start_delay_ms: u32,
    },
}

struct ActiveSpriteEffect {
    path: &'static str,
    action_index: usize,
    /// Wall-clock when this effect was queued (before any travel delay).
    start_time: ClientTick,
    motion: SpriteMotion,
    /// Opacity multiplier. The original client flies some effects as stacks
    /// of low-alpha duplicates (Fire Ball's trail); 1.0 for normal spawns.
    alpha: f32,
    /// Set for persistent skill units (Venom Dust, Demonstration), whose ACT
    /// repeats for the server-owned lifetime instead of playing once. The ID
    /// is the unit's entity, so `RemoveSkillUnit` can tear it down.
    unit: Option<EntityId>,
}

/// Sprite-based effects currently playing in the world, keyed by the sprite
/// file they come from. Unlike [`EmoteBubbles`](super::EmoteBubbles), which
/// shares a single sprite sheet, each effect may come from a different file,
/// so loads are tracked per path.
#[derive(Default)]
pub struct SpriteEffects {
    /// Animation data that has finished loading, by sprite path.
    loaded: HashMap<&'static str, Arc<AnimationData>>,
    /// Paths with a load in flight. The index is the sentinel slot.
    requested: Vec<&'static str>,
    active: Vec<ActiveSpriteEffect>,
}

impl SpriteEffects {
    /// Sentinel used to route the load of `slot` back to this holder.
    fn sentinel(slot: usize) -> EntityId {
        EntityId(SENTINEL_BASE - slot as u32)
    }

    /// Map a completed load's sentinel back to the path that requested it.
    /// Returns `None` for IDs that belong to entities or to the emote sheet.
    pub fn path_for_sentinel(&self, entity_id: EntityId) -> Option<&'static str> {
        let slot = SENTINEL_BASE.checked_sub(entity_id.0)? as usize;
        self.requested.get(slot).copied()
    }

    /// Claim a sentinel for `path` if it is neither loaded nor already in
    /// flight. The caller issues the actual `request_animation_data_load`.
    pub fn request_slot(&mut self, path: &'static str) -> Option<EntityId> {
        if self.loaded.contains_key(path) || self.requested.contains(&path) {
            return None;
        }

        self.requested.push(path);
        Some(Self::sentinel(self.requested.len() - 1))
    }

    pub fn set_animation_data(&mut self, path: &'static str, animation_data: Arc<AnimationData>) {
        if sprite_effect_debug_enabled() {
            eprintln!(
                "[sprite-effect] loaded {path}: {} actions",
                animation_data.body_action_count()
            );
        }

        self.loaded.insert(path, animation_data);
    }

    /// Queue an effect at a world position. Safe to call before the sprite has
    /// finished loading — the spawn simply draws nothing until it arrives,
    /// matching how emotes behave on first use.
    pub fn spawn(&mut self, path: &'static str, position: Point3<f32>, action_index: usize, client_tick: ClientTick) {
        if sprite_effect_debug_enabled() {
            eprintln!(
                "[sprite-effect] spawn {path} action={action_index} at ({:.1},{:.1},{:.1})",
                position.x, position.y, position.z
            );
        }

        self.active.push(ActiveSpriteEffect {
            path,
            action_index,
            start_time: client_tick,
            motion: SpriteMotion::Fixed { position },
            alpha: 1.0,
            unit: None,
        });
    }

    /// Queue a sprite that loops at `position` until [`Self::remove_unit`] is
    /// called for `entity_id`. Used by persistent skill units whose original
    /// presentation is a repeating `이팩트` sprite rather than a `.str` script.
    pub fn spawn_unit(
        &mut self,
        path: &'static str,
        position: Point3<f32>,
        action_index: usize,
        client_tick: ClientTick,
        entity_id: EntityId,
    ) {
        if sprite_effect_debug_enabled() {
            eprintln!(
                "[sprite-effect] unit {path} action={action_index} entity={} at ({:.1},{:.1},{:.1})",
                entity_id.0, position.x, position.y, position.z
            );
        }

        self.active.push(ActiveSpriteEffect {
            path,
            action_index,
            start_time: client_tick,
            motion: SpriteMotion::Fixed { position },
            alpha: 1.0,
            unit: Some(entity_id),
        });
    }

    /// Drop the looping sprites belonging to a skill unit the server removed.
    pub fn remove_unit(&mut self, entity_id: EntityId) {
        self.active.retain(|effect| effect.unit != Some(entity_id));
    }

    /// Queue a sprite that flies from `from` to `to` over `duration_ms`.
    /// `start_delay_ms` staggers multi-hit volleys so later orbs leave later
    /// and still land near the impact boundary when durations match the
    /// procedural Soul Strike packing. `alpha` dims trail ghosts (1.0 for the
    /// lead sprite).
    pub fn spawn_travel(
        &mut self,
        path: &'static str,
        from: Point3<f32>,
        to: Point3<f32>,
        action_index: usize,
        client_tick: ClientTick,
        duration_ms: u32,
        start_delay_ms: u32,
        alpha: f32,
    ) {
        if sprite_effect_debug_enabled() {
            eprintln!(
                "[sprite-effect] travel {path} action={action_index} \
                 from=({:.1},{:.1},{:.1}) to=({:.1},{:.1},{:.1}) \
                 duration_ms={duration_ms} delay_ms={start_delay_ms}",
                from.x, from.y, from.z, to.x, to.y, to.z
            );
        }

        self.active.push(ActiveSpriteEffect {
            path,
            action_index,
            start_time: client_tick,
            motion: SpriteMotion::Travel {
                from,
                to,
                duration_ms: duration_ms.max(1),
                start_delay_ms,
            },
            alpha,
            unit: None,
        });
    }

    pub fn clear(&mut self) {
        self.active.clear();
    }

    fn lifetime_ms(effect: &ActiveSpriteEffect, loaded: &HashMap<&'static str, Arc<AnimationData>>) -> u32 {
        // Unit sprites are owned by the server: they expire only via
        // `remove_unit`, never on a client timer.
        if effect.unit.is_some() {
            return u32::MAX;
        }

        match effect.motion {
            SpriteMotion::Fixed { .. } => loaded
                .get(effect.path)
                .map(|data| data.action_duration_ms(effect.action_index) + LINGER_MS)
                .unwrap_or(FALLBACK_LIFETIME_MS),
            // Travel must outlive the flight even if the ACT is short; linger
            // briefly on the target so arrival is readable.
            SpriteMotion::Travel {
                duration_ms,
                start_delay_ms,
                ..
            } => start_delay_ms.saturating_add(duration_ms).saturating_add(LINGER_MS),
        }
    }

    fn age_ms(effect: &ActiveSpriteEffect, client_tick: ClientTick) -> u32 {
        client_tick.0.wrapping_sub(effect.start_time.0)
    }

    /// World position for this tick, or `None` while still waiting on a travel
    /// start delay (not drawn yet).
    fn position_at(effect: &ActiveSpriteEffect, client_tick: ClientTick) -> Option<Point3<f32>> {
        match effect.motion {
            SpriteMotion::Fixed { position } => Some(position),
            SpriteMotion::Travel {
                from,
                to,
                duration_ms,
                start_delay_ms,
            } => {
                let age = Self::age_ms(effect, client_tick);
                if age < start_delay_ms {
                    return None;
                }
                let flight = age - start_delay_ms;
                let progress = (flight as f32 / duration_ms.max(1) as f32).clamp(0.0, 1.0);
                // Mild arc so multi-hit volleys don't stack on a single line.
                let lateral = Vector3::new((progress - 0.5) * 1.5, (1.0 - progress) * 2.0, (progress - 0.5) * -1.2);
                Some(from + (to - from) * progress + lateral)
            }
        }
    }

    /// ACT clock: starts when the sprite becomes visible (after travel delay).
    /// Unit sprites wrap it so the ACT repeats — `render_action_frame` clamps
    /// to the last frame rather than looping on its own.
    fn animation_time_ms(effect: &ActiveSpriteEffect, animation_data: &AnimationData, client_tick: ClientTick) -> u32 {
        let time = match effect.motion {
            SpriteMotion::Fixed { .. } => Self::age_ms(effect, client_tick),
            SpriteMotion::Travel { start_delay_ms, .. } => Self::age_ms(effect, client_tick).saturating_sub(start_delay_ms),
        };

        match effect.unit {
            Some(_) => match animation_data.action_duration_ms(effect.action_index) {
                0 => time,
                duration => time % duration,
            },
            None => time,
        }
    }

    pub fn update(&mut self, client_tick: ClientTick) {
        let loaded = &self.loaded;

        self.active.retain(|effect| {
            let age = Self::age_ms(effect, client_tick);
            age < Self::lifetime_ms(effect, loaded)
        });
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, client_tick: ClientTick) {
        for (index, effect) in self.active.iter().enumerate() {
            let Some(animation_data) = self.loaded.get(effect.path) else {
                continue;
            };
            let Some(position) = Self::position_at(effect, client_tick) else {
                continue;
            };

            let time = Self::animation_time_ms(effect, animation_data, client_tick);

            // Effects are not entities, but `render_action_frame` keys its
            // per-entity state off this ID. Reuse the sentinel space so an
            // effect can never be mistaken for a real entity.
            let first_new_instruction = instructions.len();
            animation_data.render_action_frame(
                instructions,
                camera,
                Self::sentinel(index),
                position,
                effect.action_index,
                time,
            );

            // Dim trail ghosts after the fact — the shared render path has no
            // per-call tint parameter.
            if effect.alpha < 1.0 {
                for instruction in &mut instructions[first_new_instruction..] {
                    instruction.color.alpha *= effect.alpha;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH_A: &str = "이팩트\\soule";
    const PATH_B: &str = "이팩트\\유령";

    #[test]
    fn sentinels_round_trip_to_their_path() {
        let mut effects = SpriteEffects::default();

        let first = effects.request_slot(PATH_A).expect("first request claims a slot");
        let second = effects.request_slot(PATH_B).expect("second request claims a slot");

        assert_ne!(first, second);
        assert_eq!(effects.path_for_sentinel(first), Some(PATH_A));
        assert_eq!(effects.path_for_sentinel(second), Some(PATH_B));
    }

    #[test]
    fn sentinels_stay_clear_of_the_emote_sheet_and_real_entities() {
        let mut effects = SpriteEffects::default();
        let sentinel = effects.request_slot(PATH_A).unwrap();

        assert_ne!(sentinel, crate::world::EMOTE_ANIMATION_ENTITY_ID);
        // Ordinary entity IDs are small and must not resolve to a sprite path.
        assert_eq!(effects.path_for_sentinel(EntityId(150_053)), None);
    }

    #[test]
    fn a_path_is_only_requested_once() {
        let mut effects = SpriteEffects::default();

        assert!(effects.request_slot(PATH_A).is_some());
        assert!(effects.request_slot(PATH_A).is_none(), "in-flight load must not re-request");
    }

    #[test]
    fn spawns_expire_even_when_the_sprite_never_loads() {
        let mut effects = SpriteEffects::default();
        effects.spawn(PATH_A, Point3::new(0.0, 0.0, 0.0), 0, ClientTick(0));

        effects.update(ClientTick(FALLBACK_LIFETIME_MS - 1));
        assert_eq!(effects.active.len(), 1, "still within the fallback lifetime");

        effects.update(ClientTick(FALLBACK_LIFETIME_MS));
        assert!(effects.active.is_empty(), "unloaded spawn must not leak");
    }

    #[test]
    fn unit_sprites_outlive_the_fallback_timer_and_die_with_their_unit() {
        let mut effects = SpriteEffects::default();
        let unit = EntityId(4_242);
        effects.spawn_unit(PATH_A, Point3::new(0.0, 0.0, 0.0), 0, ClientTick(0), unit);

        // A one-shot spawn would have been reaped here; the server owns this one.
        effects.update(ClientTick(FALLBACK_LIFETIME_MS * 10));
        assert_eq!(effects.active.len(), 1, "unit sprite must not expire on a client timer");

        effects.remove_unit(EntityId(9_999));
        assert_eq!(effects.active.len(), 1, "an unrelated unit removal must not touch it");

        effects.remove_unit(unit);
        assert!(effects.active.is_empty(), "RemoveSkillUnit must tear the sprite down");
    }

    #[test]
    fn travel_spawns_cover_the_full_flight() {
        let mut effects = SpriteEffects::default();
        effects.spawn_travel(
            PATH_A,
            Point3::new(0.0, 7.0, 0.0),
            Point3::new(10.0, 7.0, 0.0),
            0,
            ClientTick(0),
            400,
            100,
            1.0,
        );

        // Still in start delay — not drawn, but not expired.
        assert!(SpriteEffects::position_at(&effects.active[0], ClientTick(50)).is_none());
        effects.update(ClientTick(50));
        assert_eq!(effects.active.len(), 1);

        // Mid-flight.
        let mid = SpriteEffects::position_at(&effects.active[0], ClientTick(300)).expect("visible in flight");
        assert!(mid.x > 0.0 && mid.x < 10.0, "mid X should be between ends, got {}", mid.x);

        // After delay + duration + linger → gone.
        effects.update(ClientTick(100 + 400 + LINGER_MS));
        assert!(effects.active.is_empty());
    }
}
