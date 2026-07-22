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
//! See `docs/plans/classic-effect-fidelity.md`.

use std::collections::HashMap;
use std::sync::Arc;

use cgmath::Point3;
use ragnarok_packets::{ClientTick, EntityId};

use crate::graphics::EntityInstruction;
use crate::world::{AnimationData, Camera};

/// Extra display time after a sprite effect finishes, so the final frame is
/// not cut off abruptly.
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

struct ActiveSpriteEffect {
    path: &'static str,
    position: Point3<f32>,
    action_index: usize,
    start_time: ClientTick,
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
            position,
            action_index,
            start_time: client_tick,
        });
    }

    pub fn clear(&mut self) {
        self.active.clear();
    }

    pub fn update(&mut self, client_tick: ClientTick) {
        let loaded = &self.loaded;

        self.active.retain(|effect| {
            let age = client_tick.0.wrapping_sub(effect.start_time.0);
            let lifetime = loaded
                .get(effect.path)
                .map(|data| data.action_duration_ms(effect.action_index) + LINGER_MS)
                .unwrap_or(FALLBACK_LIFETIME_MS);

            age < lifetime
        });
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, client_tick: ClientTick) {
        for (index, effect) in self.active.iter().enumerate() {
            let Some(animation_data) = self.loaded.get(effect.path) else {
                continue;
            };

            let time = client_tick.0.wrapping_sub(effect.start_time.0);

            // Effects are not entities, but `render_action_frame` keys its
            // per-entity state off this ID. Reuse the sentinel space so an
            // effect can never be mistaken for a real entity.
            animation_data.render_action_frame(
                instructions,
                camera,
                Self::sentinel(index),
                effect.position,
                effect.action_index,
                time,
            );
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
}
