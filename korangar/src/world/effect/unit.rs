//! Persistent skill-unit bodies (Phase E2).
//!
//! The original client draws most classic ground units as layered rotating
//! textured cylinders (Safety Wall, warp portals, Sanctuary, Magnus, Fire
//! Pillar) or clustered textured horns (Ice Wall), per the reverse-engineered
//! effect table — see `docs/plans/classic-effect-fidelity.md` for sourcing.
//! These effects live until [`EffectHolder::remove_unit`] marks them, exactly
//! following the server's `RemoveSkillUnit`.

use std::f32::consts::TAU;
use std::sync::Arc;

use cgmath::{Point3, Vector2, Vector3};
use korangar_collision::{Frustum, Sphere};
use wgpu::BlendFactor;

use super::EffectBase;
use super::burst::hash01;
use crate::graphics::{Color, Texture};
use crate::loaders::GAT_TILE_SIZE;
use crate::renderer::EffectRenderer;
use crate::world::{Camera, PointLightId, PointLightManager};

/// One cylinder layer of a unit body. Sizes are in world units (one map cell
/// is [`GAT_TILE_SIZE`] = 5.0). `sides` of 4 gives the square footprint the
/// original uses for Sanctuary / Magnus map units; ~20 reads as round.
pub struct UnitCylinderSpec {
    pub bottom_radius: f32,
    pub top_radius: f32,
    pub height: f32,
    /// Radians/second; sign flips direction, 0.0 is static.
    pub spin_speed: f32,
    pub sides: usize,
    pub alpha: f32,
    /// Static rotation offset — the original yaws its square units 45°.
    pub yaw: f32,
}

/// Persistent layered-cylinder unit body (additive, like the original).
pub struct UnitCylinders {
    texture: Arc<Texture>,
    position: Point3<f32>,
    specs: &'static [UnitCylinderSpec],
    color: Color,
    point_light_id: PointLightId,
    light_color: Color,
    light_intensity: f32,
    spin: f32,
    gets_deleted: bool,
}

impl UnitCylinders {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        texture: Arc<Texture>,
        position: Point3<f32>,
        specs: &'static [UnitCylinderSpec],
        color: Color,
        point_light_id: PointLightId,
        light_color: Color,
        light_intensity: f32,
    ) -> Self {
        Self {
            texture,
            position,
            specs,
            color,
            point_light_id,
            light_color,
            light_intensity,
            spin: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for UnitCylinders {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.spin = (self.spin + delta_time) % TAU;
        !self.gets_deleted
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        if self.light_intensity <= 0.0 {
            return;
        }
        let light_position = self.position + Vector3::new(0.0, 4.0, 0.0);
        if Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(light_position, self.light_intensity)) {
            point_light_manager.register_fading(
                self.point_light_id,
                light_position,
                self.light_color,
                self.light_intensity,
                self.light_intensity,
            );
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let culling_radius = self
            .specs
            .iter()
            .map(|spec| spec.height.max(spec.bottom_radius).max(spec.top_radius))
            .fold(0.0, f32::max);
        if !Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(self.position, culling_radius)) {
            return;
        }

        for spec in self.specs {
            let rotation = spec.yaw + self.spin * spec.spin_speed;
            let mut color = self.color;
            color.alpha = spec.alpha;
            let sides = spec.sides.max(3);

            for segment in 0..sides {
                let angle_start = rotation + segment as f32 / sides as f32 * TAU;
                let angle_end = rotation + (segment + 1) as f32 / sides as f32 * TAU;

                let ring_point =
                    |angle: f32, radius: f32, height: f32| self.position + Vector3::new(angle.cos() * radius, height, angle.sin() * radius);

                let corners = [
                    ring_point(angle_start, spec.top_radius, spec.height),
                    ring_point(angle_end, spec.top_radius, spec.height),
                    ring_point(angle_start, spec.bottom_radius, 0.0),
                    ring_point(angle_end, spec.bottom_radius, 0.0),
                ];

                let u_start = segment as f32 / sides as f32;
                let u_end = (segment + 1) as f32 / sides as f32;
                let texture_coordinates = [
                    Vector2::new(u_start, 0.0),
                    Vector2::new(u_end, 0.0),
                    Vector2::new(u_start, 1.0),
                    Vector2::new(u_end, 1.0),
                ];

                renderer.render_effect_world_quad(
                    camera,
                    corners,
                    self.texture.clone(),
                    texture_coordinates,
                    color,
                    BlendFactor::SrcAlpha,
                    BlendFactor::One,
                );
            }
        }
    }
}

/// Ice Wall — three ice horns per cell (the original's effect 74: `ice.tga`
/// QuadHorns 2.3–3.3 cells tall). Grows in briefly, then stands until removal.
pub struct UnitIceHorns {
    texture: Arc<Texture>,
    position: Point3<f32>,
    age: f32,
    gets_deleted: bool,
}

impl UnitIceHorns {
    pub fn new(texture: Arc<Texture>, position: Point3<f32>) -> Self {
        Self {
            texture,
            position,
            age: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for UnitIceHorns {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.age += delta_time;
        !self.gets_deleted
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let tallest = GAT_TILE_SIZE * 3.3;
        if !Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(self.position, tallest)) {
            return;
        }

        // Quick grow-in with ease-out; the wall then stands solid.
        let rise = (self.age / 0.25).min(1.0);
        let rise = 1.0 - (1.0 - rise) * (1.0 - rise);

        // Deterministic per-position variation so adjacent wall cells differ.
        let cell_seed = (self.position.x.to_bits() >> 8).wrapping_add((self.position.z.to_bits() >> 8).wrapping_mul(31));

        for index in 0..3u32 {
            let seed = cell_seed.wrapping_add(index);
            let angle = hash01(seed) * TAU;
            let distance = GAT_TILE_SIZE * 0.28 * hash01(seed.wrapping_add(17));
            let base = self.position + Vector3::new(angle.cos() * distance, 0.0, angle.sin() * distance);
            let height = GAT_TILE_SIZE * (2.3 + hash01(seed.wrapping_add(41)) * 1.0) * rise;
            let half_width = GAT_TILE_SIZE * (0.30 + hash01(seed.wrapping_add(59)) * 0.10);
            let yaw = hash01(seed.wrapping_add(73)) * TAU;
            let peak = base + Vector3::new(0.0, height, 0.0);
            let color = Color::rgba(0.85, 0.95, 1.0, 0.9);

            for plane in 0..2 {
                let plane_angle = yaw + plane as f32 * (TAU / 4.0);
                let edge = Vector3::new(plane_angle.cos() * half_width, 0.0, plane_angle.sin() * half_width);
                renderer.render_effect_world_quad(
                    camera,
                    [peak, peak, base - edge, base + edge],
                    self.texture.clone(),
                    [
                        Vector2::new(0.5, 0.0),
                        Vector2::new(0.5, 0.0),
                        Vector2::new(0.0, 1.0),
                        Vector2::new(1.0, 1.0),
                    ],
                    color,
                    BlendFactor::SrcAlpha,
                    BlendFactor::OneMinusSrcAlpha,
                );
            }
        }
    }
}
