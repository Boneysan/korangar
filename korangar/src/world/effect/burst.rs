use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2, Vector3};
use korangar_collision::{Frustum, Sphere};
use wgpu::BlendFactor;

use super::EffectBase;
use crate::graphics::{Color, Texture};
use crate::renderer::EffectRenderer;
use crate::world::{Camera, PointLightId, PointLightManager};

const SEGMENT_COUNT: usize = 24;
const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

#[derive(Clone, Copy)]
pub enum SkillBurstStyle {
    MagnumBreak,
    Raid,
    MeteorAssault,
    SonicBlow,
    MeleeHit,
    /// Phase E1 — MG_NAPALMBEAT: psychic shock rings at the struck target.
    NapalmBeat,
    /// Phase E1 — WZ_EARTHSPIKE: a single ground spike under the target.
    EarthSpike,
    /// Phase E1 — WZ_HEAVENDRIVE: a ring of ground spikes around the target.
    HeavensDrive,
}

impl SkillBurstStyle {
    fn duration(self) -> f32 {
        match self {
            Self::MagnumBreak => 0.3,
            Self::Raid => 0.65,
            Self::MeteorAssault => 0.5,
            Self::SonicBlow => 0.4,
            Self::MeleeHit => 0.3,
            Self::NapalmBeat => 0.45,
            Self::EarthSpike => 0.4,
            Self::HeavensDrive => 0.55,
        }
    }

    fn light(self) -> (Color, f32) {
        match self {
            Self::MagnumBreak => (Color::rgb_u8(255, 105, 25), 65.0),
            Self::Raid => (Color::rgb_u8(70, 80, 255), 45.0),
            Self::MeteorAssault => (Color::rgb_u8(180, 70, 255), 55.0),
            Self::SonicBlow => (Color::rgb_u8(220, 210, 255), 35.0),
            Self::MeleeHit => (Color::rgb_u8(255, 245, 220), 25.0),
            Self::NapalmBeat => (Color::rgb_u8(200, 90, 255), 55.0),
            Self::EarthSpike => (Color::rgb_u8(210, 170, 90), 40.0),
            Self::HeavensDrive => (Color::rgb_u8(200, 160, 80), 50.0),
        }
    }
}

/// Reusable renderer for classic skill recipes that were procedural rather
/// than STR-backed: expanding cylinders, radial streaks, and slash bursts.
pub struct SkillBurst {
    texture: Arc<Texture>,
    secondary_texture: Option<Arc<Texture>>,
    position: Point3<f32>,
    style: SkillBurstStyle,
    elapsed: f32,
    point_light_id: PointLightId,
    gets_deleted: bool,
}

impl SkillBurst {
    pub fn new(texture: Arc<Texture>, position: Point3<f32>, style: SkillBurstStyle, point_light_id: PointLightId) -> Self {
        Self {
            texture,
            secondary_texture: None,
            position,
            style,
            elapsed: 0.0,
            point_light_id,
            gets_deleted: false,
        }
    }

    pub fn with_secondary_texture(mut self, texture: Arc<Texture>) -> Self {
        self.secondary_texture = Some(texture);
        self
    }

    fn progress(&self) -> f32 {
        (self.elapsed / self.style.duration()).clamp(0.0, 1.0)
    }

    fn render_sprite(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        offset: Vector2<f32>,
        size: Vector2<f32>,
        angle: f32,
        color: Color,
    ) {
        let half_width = size.x / 2.0;
        let half_height = size.y / 2.0;
        renderer.render_effect(
            camera,
            self.position,
            self.texture.clone(),
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
            EFFECT_ORIGIN + offset,
            Rad(angle),
            color,
            BlendFactor::SrcAlpha,
            BlendFactor::One,
        );
    }

    fn render_magnum_break(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32) {
        let alpha = (1.0 - progress) * 0.7;
        let size_multiplier = 0.5 + progress * 1.65;

        for layer in 0..2 {
            let (bottom_radius, top_radius, height, texture) = match layer {
                0 => (4.0 * size_multiplier, 6.0 * size_multiplier, 1.0, &self.texture),
                _ => (
                    4.0 * size_multiplier,
                    1.0 * size_multiplier,
                    4.0,
                    self.secondary_texture.as_ref().unwrap_or(&self.texture),
                ),
            };

            for segment in 0..SEGMENT_COUNT {
                let start = segment as f32 / SEGMENT_COUNT as f32 * TAU + progress * (1.5 + layer as f32);
                let end = (segment + 1) as f32 / SEGMENT_COUNT as f32 * TAU + progress * (1.5 + layer as f32);
                let point = |angle: f32, radius: f32, y: f32| self.position + Vector3::new(angle.cos() * radius, y, angle.sin() * radius);
                let u0 = segment as f32 / SEGMENT_COUNT as f32;
                let u1 = (segment + 1) as f32 / SEGMENT_COUNT as f32;

                renderer.render_effect_world_quad(
                    camera,
                    [
                        point(start, top_radius, height),
                        point(end, top_radius, height),
                        point(start, bottom_radius, 0.0),
                        point(end, bottom_radius, 0.0),
                    ],
                    texture.clone(),
                    [
                        Vector2::new(u0, 0.0),
                        Vector2::new(u1, 0.0),
                        Vector2::new(u0, 1.0),
                        Vector2::new(u1, 1.0),
                    ],
                    Color::rgba(1.0, 0.75 + layer as f32 * 0.15, 0.35, alpha - layer as f32 * 0.1),
                    BlendFactor::SrcAlpha,
                    BlendFactor::One,
                );
            }
        }
    }

    fn render_melee_hit(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32) {
        let alpha = (1.0 - progress) * 0.9;
        for index in 0..8 {
            let angle = index as f32 / 8.0 * TAU + 0.3;
            let distance = 12.0 + progress * 26.0;
            let texture = if index % 2 == 0 {
                &self.texture
            } else {
                self.secondary_texture.as_ref().unwrap_or(&self.texture)
            };
            let half_width = 8.0 * (1.0 - progress).max(0.1);
            let half_height = 70.0 + progress * 75.0;
            renderer.render_effect(
                camera,
                self.position,
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
                EFFECT_ORIGIN + Vector2::new(angle.cos(), angle.sin()) * distance,
                Rad(angle + PI / 2.0),
                Color::rgba(1.0, 1.0, 1.0, alpha),
                BlendFactor::SrcAlpha,
                BlendFactor::One,
            );
        }
    }

    fn render_raid(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32) {
        // Live GUI pass 2026-07-17: the original values (thin 12 px streaks,
        // 0.2/0.3/1.0 blue at peak alpha 0.45) were invisible against lit
        // terrain — the whole burst read as a bare point light.
        let alpha = (progress * 4.0).min(1.0) * (1.0 - progress);

        // Central flash so the burst has a readable core.
        let flash_size = 130.0 + progress * 140.0;
        self.render_sprite(
            renderer,
            camera,
            Vector2::new(0.0, -10.0),
            Vector2::new(flash_size, flash_size),
            progress * 0.8,
            Color::rgba(0.55, 0.68, 1.0, (1.0 - progress) * 0.85),
        );

        for index in 0..20 {
            let angle = index as f32 / 20.0 * TAU + (index % 3) as f32 * 0.27;
            let distance = 12.0 + progress * (55.0 + (index % 5) as f32 * 7.0);
            let offset = Vector2::new(angle.cos(), angle.sin()) * distance;
            self.render_sprite(
                renderer,
                camera,
                offset,
                Vector2::new(20.0, 120.0),
                angle + PI / 2.0,
                Color::rgba(0.5, 0.62, 1.0, alpha),
            );
        }
    }

    fn render_meteor_assault(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32) {
        // Brightened alongside Raid (2026-07-17): the purple streaks washed
        // out to nothing and only the point light was visible in game.
        let alpha = (1.0 - progress) * 0.95;
        for index in 0..8 {
            let angle = index as f32 / 8.0 * TAU;
            let distance = 8.0 + progress * 70.0;
            let size = 110.0 + progress * 110.0;
            self.render_sprite(
                renderer,
                camera,
                Vector2::new(angle.cos(), angle.sin()) * distance,
                Vector2::new(size, size),
                -angle,
                Color::rgba(0.95, 0.55, 1.0, alpha),
            );
        }
    }

    fn render_sonic_blow(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32) {
        let size = 100.0 + progress * 200.0;
        self.render_sprite(
            renderer,
            camera,
            Vector2::new(0.0, -12.0),
            Vector2::new(size, size),
            progress * PI,
            Color::rgba(1.0, 1.0, 1.0, 1.0 - progress),
        );
    }

    fn render_napalm_beat(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32) {
        // Classic napalm is a psychic shock at the target: expanding violet
        // rings plus a short flash. Code-drawn — no napalm STR is used.
        let alpha = (1.0 - progress) * 0.9;
        let flash = 90.0 + progress * 160.0;
        self.render_sprite(
            renderer,
            camera,
            Vector2::new(0.0, -8.0),
            Vector2::new(flash, flash),
            progress * TAU,
            Color::rgba(0.85, 0.45, 1.0, alpha),
        );
        for layer in 0..3 {
            let size = 70.0 + progress * (140.0 + layer as f32 * 50.0);
            self.render_sprite(
                renderer,
                camera,
                Vector2::new(0.0, -6.0 - layer as f32 * 4.0),
                Vector2::new(size, size * 0.55),
                progress * (1.2 + layer as f32 * 0.4),
                Color::rgba(0.75, 0.35, 1.0, alpha * (1.0 - layer as f32 * 0.2)),
            );
        }
    }

    fn render_earth_spikes(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32, count: usize, radius: f32) {
        // Rising ground spikes: thin vertical world quads that grow then fade.
        let rise = (progress * 1.4).min(1.0);
        let alpha = (1.0 - progress) * 0.95;
        let height = 2.0 + rise * 10.0;
        let half_width = 0.55 + (1.0 - progress) * 0.35;

        for index in 0..count {
            let angle = if count == 1 {
                0.0
            } else {
                index as f32 / count as f32 * TAU + progress * 0.4
            };
            let offset = Vector3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
            let base = self.position + offset;
            let top = base + Vector3::new(0.0, height, 0.0);
            let edge = Vector3::new((-angle).sin() * half_width, 0.0, angle.cos() * half_width);
            let texture = if index % 2 == 0 {
                &self.texture
            } else {
                self.secondary_texture.as_ref().unwrap_or(&self.texture)
            };

            renderer.render_effect_world_quad(
                camera,
                [
                    top - edge,
                    top + edge,
                    base - edge,
                    base + edge,
                ],
                texture.clone(),
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                ],
                Color::rgba(0.85, 0.7, 0.35, alpha),
                BlendFactor::SrcAlpha,
                BlendFactor::OneMinusSrcAlpha,
            );
        }

        // Ground flash so the eruption reads against dark terrain.
        let flash_size = 40.0 + progress * 90.0;
        self.render_sprite(
            renderer,
            camera,
            Vector2::new(0.0, 10.0),
            Vector2::new(flash_size, flash_size * 0.45),
            0.0,
            Color::rgba(0.9, 0.75, 0.35, alpha * 0.6),
        );
    }
}

impl EffectBase for SkillBurst {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        // Match STR timers: an asset upload stall must not consume the effect.
        self.elapsed += delta_time.min(1.0 / 15.0);
        !self.gets_deleted && self.elapsed < self.style.duration()
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        let progress = self.progress();
        let (color, maximum_intensity) = self.style.light();
        let intensity = (progress * PI).sin().max(0.0) * maximum_intensity;
        let light_position = self.position + Vector3::new(0.0, 5.0, 0.0);

        if Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(light_position, intensity)) {
            point_light_manager.register_fading(self.point_light_id, light_position, color, intensity, maximum_intensity);
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let progress = self.progress();
        match self.style {
            SkillBurstStyle::MagnumBreak => self.render_magnum_break(renderer, camera, progress),
            SkillBurstStyle::Raid => self.render_raid(renderer, camera, progress),
            SkillBurstStyle::MeteorAssault => self.render_meteor_assault(renderer, camera, progress),
            SkillBurstStyle::SonicBlow => self.render_sonic_blow(renderer, camera, progress),
            SkillBurstStyle::MeleeHit => self.render_melee_hit(renderer, camera, progress),
            SkillBurstStyle::NapalmBeat => self.render_napalm_beat(renderer, camera, progress),
            SkillBurstStyle::EarthSpike => self.render_earth_spikes(renderer, camera, progress, 1, 0.0),
            SkillBurstStyle::HeavensDrive => self.render_earth_spikes(renderer, camera, progress, 6, 3.5),
        }
    }
}
