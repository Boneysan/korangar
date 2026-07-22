use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2, Vector3};
use korangar_collision::{Frustum, Sphere};
use wgpu::BlendFactor;

use super::EffectBase;
use crate::graphics::{Color, Texture};
use crate::loaders::GAT_TILE_SIZE;
use crate::renderer::EffectRenderer;
use crate::world::{Camera, PointLightId, PointLightManager};

const SEGMENT_COUNT: usize = 24;
const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

#[derive(Clone, Copy, Debug)]
pub enum SkillBurstStyle {
    MagnumBreak,
    Raid,
    MeteorAssault,
    SonicBlow,
    MeleeHit,
    /// Phase E1 — MG_NAPALMBEAT: lens streaks converging on the target in a
    /// circle pattern (the original client's effect 1).
    NapalmBeat,
    /// Phase E1 — WZ_EARTHSPIKE: stone horns rising under the target (the
    /// original client's effect 79 — one main horn plus four small ones).
    EarthSpike,
    /// Phase E1 — WZ_HEAVENDRIVE: a 5×5 cell grid of stone horns (the
    /// original client's effect 142).
    HeavensDrive,
    /// Phase E1 — WZ_JUPITEL impact: a growing thunder_pang flash under a
    /// plasma-blast frame cycle (the original client's effect 94).
    JupitelHit,
}

impl SkillBurstStyle {
    fn duration(self) -> f32 {
        match self {
            Self::MagnumBreak => 0.3,
            Self::Raid => 0.65,
            Self::MeteorAssault => 0.5,
            Self::SonicBlow => 0.4,
            Self::MeleeHit => 0.3,
            // Long enough for the eight-frame 폭발 explosion cycle.
            Self::NapalmBeat => 0.4,
            // The original keeps Earth Spike's rocks up for 5 s; 3.5 s was
            // dialed in live 2026-07-23 — 2.0 s read as a blink.
            Self::EarthSpike => 3.5,
            Self::HeavensDrive => 1.1,
            Self::JupitelHit => 0.45,
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
            Self::JupitelHit => (Color::rgb_u8(255, 245, 170), 55.0),
        }
    }
}

/// Deterministic per-element jitter in `[0, 1)` so horn layouts differ without
/// per-cast randomness (effects can be re-rendered every frame).
fn hash01(seed: u32) -> f32 {
    let hashed = seed.wrapping_mul(0x9E37_79B9).rotate_left(13).wrapping_mul(0x85EB_CA6B);
    (hashed >> 8) as f32 / (u32::MAX >> 8) as f32
}

/// Reusable renderer for classic skill recipes that were procedural rather
/// than STR-backed: expanding cylinders, radial streaks, and slash bursts.
pub struct SkillBurst {
    texture: Arc<Texture>,
    secondary_texture: Option<Arc<Texture>>,
    /// Animated frame cycle for styles that need one (Jupitel's plasma blast).
    frames: Vec<Arc<Texture>>,
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
            frames: Vec::new(),
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

    pub fn with_frames(mut self, frames: Vec<Arc<Texture>>) -> Self {
        self.frames = frames;
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
        // Original EF_NAPALMBEAT (32): "small clustered explosions" — the
        // eight-frame 폭발 cycle at the target, drawn as a tight cluster.
        if !self.frames.is_empty() {
            let frame_index = ((progress * self.frames.len() as f32) as usize).min(self.frames.len() - 1);
            let cluster_alpha = (1.0 - progress * progress) * 0.95;
            for (cluster, (offset, size)) in [
                (Vector2::new(0.0, -12.0), 110.0),
                (Vector2::new(-26.0, 6.0), 80.0),
                (Vector2::new(24.0, -30.0), 85.0),
            ]
            .into_iter()
            .enumerate()
            {
                // Later puffs run a frame behind so the cluster ripples.
                let frame_index = frame_index.saturating_sub(cluster).min(self.frames.len() - 1);
                let half = size / 2.0;
                renderer.render_effect(
                    camera,
                    self.position,
                    self.frames[frame_index].clone(),
                    [
                        Vector2::new(-half, -half),
                        Vector2::new(half, -half),
                        Vector2::new(-half, half),
                        Vector2::new(half, half),
                    ],
                    [
                        Vector2::new(1.0, 1.0),
                        Vector2::new(1.0, 0.0),
                        Vector2::new(0.0, 0.0),
                        Vector2::new(0.0, 1.0),
                    ],
                    EFFECT_ORIGIN + offset,
                    Rad(0.0),
                    Color::rgba(1.0, 1.0, 1.0, cluster_alpha),
                    BlendFactor::SrcAlpha,
                    BlendFactor::One,
                );
            }
        }

        // Hit effect 1 underneath: eight lens1/lens2 streaks placed around the
        // target in a circle pattern, growing long and thin while converging
        // inward — the classic psychokinesis hit. No STR or sprite exists.
        let alpha = (progress * 5.0).min(1.0) * (1.0 - progress) * 0.9;
        let radius = 30.0 - progress * 18.0;
        let half_length = 25.0 + progress * 120.0;
        let half_width = 14.0 * (1.0 - progress) + 2.0;

        for index in 0..8 {
            // The original draws each streak in its own ~35° angle family;
            // deterministic jitter stands in for its per-cast randomness.
            let angle = index as f32 / 8.0 * TAU + hash01(index) * 0.6;
            let offset = Vector2::new(angle.cos(), angle.sin()) * radius;
            let texture = if index % 2 == 0 {
                &self.texture
            } else {
                self.secondary_texture.as_ref().unwrap_or(&self.texture)
            };
            let half_height = half_length;
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
                EFFECT_ORIGIN + offset + Vector2::new(0.0, -8.0),
                Rad(angle + PI / 2.0),
                Color::rgba(1.0, 1.0, 1.0, alpha),
                BlendFactor::SrcAlpha,
                BlendFactor::One,
            );
        }
    }

    /// One textured stone horn: two crossed triangular world quads tapering to
    /// a tilted peak, matching the original client's QuadHorn primitive.
    #[allow(clippy::too_many_arguments)]
    fn render_stone_horn(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        base: Point3<f32>,
        height: f32,
        half_width: f32,
        yaw: f32,
        tilt: Vector2<f32>,
        alpha: f32,
    ) {
        let peak = base + Vector3::new(tilt.x * height, height, tilt.y * height);
        let color = Color::rgba(1.0, 0.97, 0.9, alpha);
        for plane in 0..2 {
            let plane_angle = yaw + plane as f32 * (PI / 2.0);
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

    /// Earth Spike (original effect 79): one main horn under the target plus
    /// four small ones around it. Heaven's Drive (effect 142): the same horn
    /// on every cell of a 5×5 grid. Horns rise fast, hold, then sink.
    fn render_earth_spikes(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32, grid: bool) {
        let rise = (progress / 0.16).min(1.0);
        // Ease-out so the eruption snaps up and settles.
        let rise = 1.0 - (1.0 - rise) * (1.0 - rise);
        let sink = ((progress - 0.72) / 0.28).clamp(0.0, 1.0);
        let scale = rise * (1.0 - sink * sink);
        if scale <= 0.0 {
            return;
        }
        let alpha = (1.0 - sink) * 0.95;

        let mut horn = |seed: u32, dx: f32, dz: f32, height: f32, half_width: f32| {
            let yaw = hash01(seed) * TAU;
            let tilt_angle = hash01(seed.wrapping_add(101)) * TAU;
            // The original tilts each horn a few degrees off vertical.
            let tilt_amount = 0.05 + hash01(seed.wrapping_add(211)) * 0.2;
            let jitter = 0.8 + hash01(seed.wrapping_add(307)) * 0.45;
            self.render_stone_horn(
                renderer,
                camera,
                self.position + Vector3::new(dx, 0.0, dz),
                height * jitter * scale,
                half_width * (0.85 + hash01(seed.wrapping_add(401)) * 0.3),
                yaw,
                Vector2::new(tilt_angle.cos(), tilt_angle.sin()) * tilt_amount,
                alpha,
            );
        };

        match grid {
            // Heaven's Drive: one horn per cell across 5×5 cells.
            true => {
                for i in -2..=2i32 {
                    for j in -2..=2i32 {
                        let seed = ((i + 2) * 5 + (j + 2)) as u32;
                        horn(
                            seed,
                            i as f32 * GAT_TILE_SIZE,
                            j as f32 * GAT_TILE_SIZE,
                            GAT_TILE_SIZE * 1.0,
                            GAT_TILE_SIZE * 0.28,
                        );
                    }
                }
            }
            // Earth Spike: main horn plus four smaller ones around the base.
            // Sized up 2026-07-23 after live feedback — 1.35/0.55 tiles read
            // too small in game.
            false => {
                horn(0, 0.0, 0.0, GAT_TILE_SIZE * 2.1, GAT_TILE_SIZE * 0.48);
                for index in 0..4u32 {
                    let angle = index as f32 / 4.0 * TAU + hash01(index.wrapping_add(37)) * 0.9;
                    let distance = GAT_TILE_SIZE * (0.55 + hash01(index.wrapping_add(53)) * 0.3);
                    horn(
                        index + 1,
                        angle.cos() * distance,
                        angle.sin() * distance,
                        GAT_TILE_SIZE * 0.95,
                        GAT_TILE_SIZE * 0.26,
                    );
                }
            }
        }
    }

    fn render_jupitel_hit(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, progress: f32) {
        // Original effect 94: thunder_pang grows from nothing at the struck
        // entity while the plasma-blast frames crackle over it, both additive.
        let pang_grow = (progress / 0.3).min(1.0);
        let pang_size = 130.0 * pang_grow;
        let alpha = (1.0 - progress * progress) * 0.9;
        self.render_sprite(
            renderer,
            camera,
            Vector2::new(0.0, -10.0),
            Vector2::new(pang_size, pang_size),
            0.0,
            Color::rgba(1.0, 1.0, 1.0, alpha),
        );

        if !self.frames.is_empty() {
            let frame_index = (progress * self.style.duration() * 1000.0 / 45.0) as usize % self.frames.len();
            let size = 150.0;
            let half = size / 2.0;
            renderer.render_effect(
                camera,
                self.position,
                self.frames[frame_index].clone(),
                [
                    Vector2::new(-half, -half),
                    Vector2::new(half, -half),
                    Vector2::new(-half, half),
                    Vector2::new(half, half),
                ],
                [
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                EFFECT_ORIGIN + Vector2::new(0.0, -10.0),
                Rad(0.0),
                Color::rgba(1.0, 1.0, 1.0, alpha),
                BlendFactor::SrcAlpha,
                BlendFactor::One,
            );
        }
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
            SkillBurstStyle::EarthSpike => self.render_earth_spikes(renderer, camera, progress, false),
            SkillBurstStyle::HeavensDrive => self.render_earth_spikes(renderer, camera, progress, true),
            SkillBurstStyle::JupitelHit => self.render_jupitel_hit(renderer, camera, progress),
        }
    }
}
