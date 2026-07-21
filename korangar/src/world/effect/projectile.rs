use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Rad, Vector2, Vector3};
use wgpu::BlendFactor;

use super::EffectBase;
use crate::graphics::{Color, Texture};
use crate::renderer::EffectRenderer;
use crate::world::{Camera, PointLightManager};

const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

/// Short classic source-to-target projectile used by Spear Boomerang.
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
        let source_screen = camera.clip_to_screen_space(view_projection * self.source.to_homogeneous());
        let target_screen = camera.clip_to_screen_space(view_projection * self.target.to_homogeneous());
        let direction = target_screen - source_screen;
        let angle = if direction.magnitude2() > 0.0 {
            Rad(direction.y.atan2(direction.x) + self.angle_offset)
        } else {
            Rad(0.0)
        };
        let half = self.size / 2.0;

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
            Color::rgba(1.0, 1.0, 1.0, 1.0 - progress * 0.25),
            BlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha,
        );
    }
}
