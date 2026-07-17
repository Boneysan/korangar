use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Rad, Vector2, Vector3};
use rand_aes::tls::rand_f32;
use wgpu::BlendFactor;

use super::EffectBase;
use crate::graphics::{Color, Texture};
use crate::renderer::EffectRenderer;
use crate::world::{Camera, PointLightManager};

/// How long one bolt takes from spawn height to the target, matching the
/// original client's 500 ms fall.
const FALL_DURATION: f32 = 0.5;
/// Delay between consecutive bolts of one volley. The whole volley arrives
/// in a single damage packet (`div` hits), so the client staggers them.
const BOLT_STAGGER: f32 = 0.15;
/// Seconds per animation frame for multi-texture bolts (fire arrows animate
/// through six textures at 30 ms per frame in the original client).
const FRAME_DELAY: f32 = 0.03;

struct Bolt {
    start_offset: Vector3<f32>,
    delay: f32,
}

/// The classic code-drawn bolt volley: Fire Bolt and Cold Bolt rain their
/// hits down from the sky onto the struck entity, one projectile per hit.
/// The original client implements these as `ef_firebolt` / `ef_coldbolt`
/// (animated arrow textures falling from ~20 units above with a slight
/// horizontal offset); they were never STR files.
pub struct FallingBolts {
    textures: Vec<Arc<Texture>>,
    target: Point3<f32>,
    bolts: Vec<Bolt>,
    elapsed: f32,
    total_duration: f32,
    /// On-screen size of one bolt quad in effect pixels.
    size: Vector2<f32>,
    color: Color,
    gets_deleted: bool,
}

impl FallingBolts {
    pub fn new(textures: Vec<Arc<Texture>>, target: Point3<f32>, bolt_count: usize, color: Color) -> Self {
        let bolts = (0..bolt_count.max(1))
            .map(|index| Bolt {
                // The original starts each bolt high above the target with a
                // small randomized sideways offset, giving the volley its
                // diagonal streaks (values in world units; a cell is five).
                start_offset: Vector3::new(
                    9.0 + (rand_f32() - 0.5) * 5.0,
                    22.0 + (rand_f32() - 0.5) * 4.0,
                    4.0 + (rand_f32() - 0.5) * 5.0,
                ),
                delay: index as f32 * BOLT_STAGGER,
            })
            .collect::<Vec<_>>();
        let total_duration = bolts.last().map(|bolt| bolt.delay).unwrap_or_default() + FALL_DURATION;

        Self {
            textures,
            target,
            bolts,
            elapsed: 0.0,
            total_duration,
            size: Vector2::new(96.0, 96.0),
            color,
            gets_deleted: false,
        }
    }
}

impl EffectBase for FallingBolts {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.elapsed += delta_time;
        !self.gets_deleted && self.elapsed < self.total_duration
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.textures.is_empty() {
            return;
        }

        let frame_index = (self.elapsed / FRAME_DELAY) as usize % self.textures.len();
        let texture = &self.textures[frame_index];

        for bolt in &self.bolts {
            let progress = (self.elapsed - bolt.delay) / FALL_DURATION;
            if !(0.0..1.0).contains(&progress) {
                continue;
            }

            let position = self.target + bolt.start_offset * (1.0 - progress);

            // Align the sprite with its on-screen direction of travel so the
            // streak reads correctly from any camera rotation.
            let view_projection = camera.view_projection_matrix();
            let screen_of = |point: Point3<f32>| camera.clip_to_screen_space(view_projection * point.to_homogeneous());
            let start_screen = screen_of(self.target + bolt.start_offset);
            let end_screen = screen_of(self.target);
            let direction = end_screen - start_screen;
            let angle = if direction.magnitude2() > 0.0 {
                Rad(direction.y.atan2(direction.x))
            } else {
                Rad(0.0)
            };

            let half_width = self.size.x / 2.0;
            let half_height = self.size.y / 2.0;

            renderer.render_effect(
                camera,
                position,
                texture.clone(),
                [
                    Vector2::new(-half_width, -half_height),
                    Vector2::new(half_width, -half_height),
                    Vector2::new(-half_width, half_height),
                    Vector2::new(half_width, half_height),
                ],
                [
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                // Zero net screen offset: `render_effect` subtracts the
                // effect origin from this value.
                Vector2::new(319.0, 291.0),
                angle,
                self.color,
                BlendFactor::SrcAlpha,
                BlendFactor::One,
            );
        }
    }
}
