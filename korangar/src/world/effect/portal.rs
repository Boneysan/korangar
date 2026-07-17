use std::f32::consts::TAU;
use std::sync::Arc;

use cgmath::{Point3, Vector2, Vector3};
use korangar_collision::{Frustum, Sphere};
use wgpu::BlendFactor;

use super::EffectBase;
use crate::graphics::{Color, Texture};
use crate::renderer::EffectRenderer;
use crate::world::{Camera, PointLightManager};

/// The texture the original client wraps around its warp vortex cylinders.
pub const PORTAL_TEXTURE_PATH: &str = "effect\\ring_blue.tga";

const SEGMENT_COUNT: usize = 20;

/// A rotating cylinder of the vortex. Sizes are in world units (one map cell
/// is five units); a negative spin speed rotates against the other cylinder,
/// which is what makes the composite read as a swirl.
struct VortexCylinder {
    bottom_radius: f32,
    top_radius: f32,
    height: f32,
    spin_speed: f32,
    alpha: f32,
}

const CYLINDERS: [VortexCylinder; 2] = [
    VortexCylinder {
        bottom_radius: 6.0,
        top_radius: 5.0,
        height: 16.0,
        spin_speed: 4.4,
        alpha: 0.6,
    },
    VortexCylinder {
        bottom_radius: 3.6,
        top_radius: 3.0,
        height: 24.0,
        spin_speed: -3.2,
        alpha: 0.5,
    },
];

/// The blue swirling vortex the original client draws on map-transfer warp
/// points. Warp entities carry no sprite, so this is their entire visual.
pub struct PortalVortex {
    texture: Arc<Texture>,
    position: Point3<f32>,
    spin: f32,
    gets_deleted: bool,
}

impl PortalVortex {
    pub fn new(texture: Arc<Texture>, position: Point3<f32>) -> Self {
        Self {
            texture,
            position,
            spin: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for PortalVortex {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.spin = (self.spin + delta_time) % TAU;
        !self.gets_deleted
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let frustum = Frustum::new(camera.view_projection_matrix(), true);
        let culling_radius = CYLINDERS
            .iter()
            .map(|cylinder| cylinder.height.max(cylinder.bottom_radius))
            .fold(0.0, f32::max);

        if !frustum.intersects_sphere(&Sphere::new(self.position, culling_radius)) {
            return;
        }

        for cylinder in &CYLINDERS {
            let rotation = self.spin * cylinder.spin_speed;
            let color = Color::rgba(1.0, 1.0, 1.0, cylinder.alpha);

            for segment in 0..SEGMENT_COUNT {
                let angle_start = rotation + segment as f32 / SEGMENT_COUNT as f32 * TAU;
                let angle_end = rotation + (segment + 1) as f32 / SEGMENT_COUNT as f32 * TAU;

                let ring_point = |angle: f32, radius: f32, height: f32| {
                    self.position + Vector3::new(angle.cos() * radius, height, angle.sin() * radius)
                };

                let corners = [
                    ring_point(angle_start, cylinder.top_radius, cylinder.height),
                    ring_point(angle_end, cylinder.top_radius, cylinder.height),
                    ring_point(angle_start, cylinder.bottom_radius, 0.0),
                    ring_point(angle_end, cylinder.bottom_radius, 0.0),
                ];

                let u_start = segment as f32 / SEGMENT_COUNT as f32;
                let u_end = (segment + 1) as f32 / SEGMENT_COUNT as f32;
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
