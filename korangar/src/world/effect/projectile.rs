use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Rad, Vector2, Vector3};
use wgpu::BlendFactor;

use super::EffectBase;
use crate::graphics::{Color, Texture};
use crate::renderer::EffectRenderer;
use crate::world::{Camera, PointLightManager};

const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

/// Short classic source-to-target projectile used by Spear Boomerang, Fire Ball,
/// Frost Diver, and Jupitel Thunder.
pub struct SkillProjectile {
    texture: Arc<Texture>,
    source: Point3<f32>,
    target: Point3<f32>,
    elapsed: f32,
    duration: f32,
    size: Vector2<f32>,
    /// Extra rotation (radians) added to the travel-direction angle, to account
    /// for the sprite's own resting orientation (the spear art points one way,
    /// the arrow item sprite another).
    angle_offset: f32,
    color: Color,
    gets_deleted: bool,
}

impl SkillProjectile {
    pub fn spear(texture: Arc<Texture>, source: Point3<f32>, target: Point3<f32>) -> Self {
        Self {
            texture,
            source: source + Vector3::new(0.0, 5.0, 0.0),
            target: target + Vector3::new(0.0, 5.0, 0.0),
            elapsed: 0.0,
            duration: 0.14,
            size: Vector2::new(100.0, 100.0),
            angle_offset: std::f32::consts::PI,
            color: Color::WHITE,
            gets_deleted: false,
        }
    }

    /// Arrow/bullet fired on a normal ranged attack (bow, gun). Flies straight
    /// from shooter to target at a fixed speed, so the flight time scales with
    /// range; the sprite keeps its native aspect and is rotated to face travel.
    pub fn arrow(texture: Arc<Texture>, source: Point3<f32>, target: Point3<f32>) -> Self {
        // ~10 units above the ground so it reads at torso/bow height.
        let lift = Vector3::new(0.0, 10.0, 0.0);
        let source = source + lift;
        let target = target + lift;
        let distance = (target - source).magnitude();
        // Arrows are fast; clamp so a point-blank shot is still visible and a
        // max-range shot doesn't crawl. Speed in world units / second.
        let duration = (distance / 220.0).clamp(0.10, 0.35);
        // Item sprites are small (a few px), so scale the longest side up to a
        // readable world size, keeping the sprite's aspect ratio.
        let texture_size = texture.get_size();
        let native = Vector2::new(texture_size.width.max(1) as f32, texture_size.height.max(1) as f32);
        const TARGET_LONGEST: f32 = 40.0;
        let size = native * (TARGET_LONGEST / native.x.max(native.y));
        Self {
            texture,
            source,
            target,
            elapsed: 0.0,
            duration,
            size,
            // The arrow item icon rests pointing up-right, so bring its nose
            // onto the travel direction; -135° was dialed in against the live
            // client (the isometric camera tilts a purely horizontal shot).
            angle_offset: -135.0_f32.to_radians(),
            color: Color::WHITE,
            gets_deleted: false,
        }
    }

    /// Phase E1 — magic travel ball (Fire Ball / Frost Diver / Jupitel).
    /// Uses an effect texture and a tint so one generic path covers the set.
    pub fn travel_ball(
        texture: Arc<Texture>,
        source: Point3<f32>,
        target: Point3<f32>,
        duration: f32,
        size: f32,
        color: Color,
    ) -> Self {
        let lift = Vector3::new(0.0, 8.0, 0.0);
        Self {
            texture,
            source: source + lift,
            target: target + lift,
            elapsed: 0.0,
            duration,
            size: Vector2::new(size, size),
            angle_offset: 0.0,
            color,
            gets_deleted: false,
        }
    }
}

impl EffectBase for SkillProjectile {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.elapsed += delta_time.min(1.0 / 15.0);
        !self.gets_deleted && self.elapsed < self.duration
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let progress = (self.elapsed / self.duration).clamp(0.0, 1.0);
        let position = self.source + (self.target - self.source) * progress;

        let view_projection = camera.view_projection_matrix();
        let screen_of = |point: Point3<f32>| camera.clip_to_screen_space(view_projection * point.to_homogeneous());
        let start_screen = screen_of(self.source);
        let end_screen = screen_of(self.target);
        let direction = end_screen - start_screen;
        let travel_angle = if direction.magnitude2() > 0.0 {
            Rad(direction.y.atan2(direction.x))
        } else {
            Rad(0.0)
        };
        let angle = Rad(travel_angle.0 + self.angle_offset);

        let half = self.size / 2.0;
        let mut color = self.color;
        color.alpha *= 1.0 - progress * 0.25;

        renderer.render_effect(
            camera,
            position,
            self.texture.clone(),
            [
                Vector2::new(-half.x, -half.y),
                Vector2::new(half.x, -half.y),
                Vector2::new(-half.x, half.y),
                Vector2::new(half.x, half.y),
            ],
            [
                Vector2::new(1.0, 1.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(0.0, 0.0),
                Vector2::new(0.0, 1.0),
            ],
            EFFECT_ORIGIN,
            angle,
            color,
            BlendFactor::SrcAlpha,
            BlendFactor::One,
        );
    }
}

/// Phase E1 — MG_SOULSTRIKE: one ghost orb per hit, staggered, caster → target.
/// Classic Soul Strike is code-drawn orbs, not a modern nested STR for travel.
pub struct SoulStrikeOrbs {
    texture: Arc<Texture>,
    source: Point3<f32>,
    target: Point3<f32>,
    delays: Vec<f32>,
    elapsed: f32,
    total_duration: f32,
    gets_deleted: bool,
}

/// Default single-orb travel when the impact delay is unknown.
const DEFAULT_ORB_TRAVEL: f32 = 0.32;

impl SoulStrikeOrbs {
    /// `arrival_secs` is the impact due boundary (seconds). Orbs are packed so
    /// the last one lands near that boundary instead of overshooting it.
    pub fn new(
        texture: Arc<Texture>,
        source: Point3<f32>,
        target: Point3<f32>,
        orb_count: usize,
        arrival_secs: f32,
    ) -> Self {
        let lift = Vector3::new(0.0, 7.0, 0.0);
        let count = orb_count.max(1);
        let arrival = if arrival_secs > 0.05 {
            arrival_secs.clamp(0.18, 0.85)
        } else {
            DEFAULT_ORB_TRAVEL + (count.saturating_sub(1) as f32) * 0.11
        };
        // First orb flight time; remaining budget becomes inter-orb stagger.
        let travel = (arrival * 0.55).clamp(0.16, 0.40);
        let stagger = if count > 1 {
            ((arrival - travel) / (count - 1) as f32).max(0.0)
        } else {
            0.0
        };
        let delays = (0..count).map(|index| index as f32 * stagger).collect::<Vec<_>>();
        let total_duration = delays.last().copied().unwrap_or(0.0) + travel;
        Self {
            texture,
            source: source + lift,
            target: target + lift,
            delays,
            elapsed: 0.0,
            total_duration,
            gets_deleted: false,
        }
    }
}

impl EffectBase for SoulStrikeOrbs {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.elapsed += delta_time.min(1.0 / 15.0);
        !self.gets_deleted && self.elapsed < self.total_duration
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let half = Vector2::new(48.0, 48.0);
        // Per-orb flight = total_duration - last delay (same for all when stagger is even).
        let last_delay = self.delays.last().copied().unwrap_or(0.0);
        let travel = (self.total_duration - last_delay).max(0.08);
        for delay in &self.delays {
            let progress = (self.elapsed - delay) / travel;
            if !(0.0..1.0).contains(&progress) {
                continue;
            }
            // Slight lateral offset so multi-hit volleys don't stack perfectly.
            let lateral = Vector3::new((progress - 0.5) * 1.5, (1.0 - progress) * 2.0, (progress - 0.5) * -1.2);
            let position = self.source + (self.target - self.source) * progress + lateral;
            let size_scale = 0.75 + (1.0 - progress) * 0.5;
            let half = half * size_scale;

            renderer.render_effect(
                camera,
                position,
                self.texture.clone(),
                [
                    Vector2::new(-half.x, -half.y),
                    Vector2::new(half.x, -half.y),
                    Vector2::new(-half.x, half.y),
                    Vector2::new(half.x, half.y),
                ],
                [
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                EFFECT_ORIGIN,
                Rad(progress * std::f32::consts::TAU),
                Color::rgba(0.75, 0.45, 1.0, 1.0 - progress * 0.35),
                BlendFactor::SrcAlpha,
                BlendFactor::One,
            );
        }
    }
}
