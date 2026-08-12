use std::f32::consts::PI;
use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Rad, Vector2, Vector3};
use korangar_collision::{Frustum, Sphere};
use wgpu::BlendFactor;

use super::EffectBase;
use crate::graphics::{Color, Texture};
use crate::renderer::EffectRenderer;
use crate::world::{Camera, PointLightId, PointLightManager};

const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

/// Short classic source-to-target projectile used by Spear Boomerang, Fire
/// Ball, Frost Diver, and Jupitel Thunder.
pub struct SkillProjectile {
    texture: Arc<Texture>,
    /// Animated frame cycle drawn instead of `texture` when non-empty
    /// (Jupitel's thunder-ball frames). `texture` still backs the point light
    /// fallback and the audit path.
    frames: Vec<Arc<Texture>>,
    frame_delay_ms: f32,
    /// Static glow drawn under the animated frames (Jupitel's thunder_center).
    core_texture: Option<Arc<Texture>>,
    source: Point3<f32>,
    target: Point3<f32>,
    elapsed: f32,
    duration: f32,
    size: Vector2<f32>,
    /// Extra rotation (radians) added to the travel-direction angle, to account
    /// for the sprite's own resting orientation (the spear art points one way,
    /// the arrow item sprite another).
    angle_offset: f32,
    /// Whether the billboard rotates onto the travel direction. Symmetric
    /// energy balls stay screen-upright like the original client's billboards.
    align_to_travel: bool,
    color: Color,
    /// Elemental halo drawn under the sprite, and the colour the point light
    /// takes. `None` leaves the projectile looking exactly like its artwork.
    glow_color: Option<Color>,
    point_light_id: Option<PointLightId>,
    light_intensity: f32,
    /// Fraction of the flight this copy lags behind the head of the shot.
    /// `0.0` is the head itself; a trail shard sits at `progress - trail_lag`.
    trail_lag: f32,
    /// World-space nudge applied to this copy, so a multi-shard trail reads as
    /// a spray of ice rather than a straight line of identical quads.
    trail_offset: Vector3<f32>,
    gets_deleted: bool,
}

impl SkillProjectile {
    pub fn spear(texture: Arc<Texture>, source: Point3<f32>, target: Point3<f32>) -> Self {
        Self {
            texture,
            frames: Vec::new(),
            frame_delay_ms: 0.0,
            core_texture: None,
            source: source + Vector3::new(0.0, 5.0, 0.0),
            target: target + Vector3::new(0.0, 5.0, 0.0),
            elapsed: 0.0,
            duration: 0.14,
            size: Vector2::new(100.0, 100.0),
            angle_offset: std::f32::consts::PI,
            align_to_travel: true,
            color: Color::WHITE,
            glow_color: None,
            point_light_id: None,
            light_intensity: 0.0,
            trail_lag: 0.0,
            trail_offset: Vector3::new(0.0, 0.0, 0.0),
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
            frames: Vec::new(),
            frame_delay_ms: 0.0,
            core_texture: None,
            source,
            target,
            elapsed: 0.0,
            duration,
            size,
            // The arrow item icon rests pointing up-right, so bring its nose
            // onto the travel direction; -135° was dialed in against the live
            // client (the isometric camera tilts a purely horizontal shot).
            angle_offset: -135.0_f32.to_radians(),
            align_to_travel: true,
            color: Color::WHITE,
            glow_color: None,
            point_light_id: None,
            light_intensity: 0.0,
            trail_lag: 0.0,
            trail_offset: Vector3::new(0.0, 0.0, 0.0),
            gets_deleted: false,
        }
    }

    /// Phase E1 — magic travel ball (Fire Ball / Frost Diver / Jupitel).
    /// Carries a mid-flight point light matching the elemental tint.
    #[allow(clippy::too_many_arguments)]
    pub fn travel_ball(
        texture: Arc<Texture>,
        source: Point3<f32>,
        target: Point3<f32>,
        duration: f32,
        size: f32,
        color: Color,
        point_light_id: PointLightId,
        light_intensity: f32,
    ) -> Self {
        let lift = Vector3::new(0.0, 8.0, 0.0);
        Self {
            texture,
            frames: Vec::new(),
            frame_delay_ms: 0.0,
            core_texture: None,
            source: source + lift,
            target: target + lift,
            elapsed: 0.0,
            duration,
            size: Vector2::new(size, size),
            angle_offset: 0.0,
            align_to_travel: true,
            color,
            glow_color: None,
            point_light_id: Some(point_light_id),
            light_intensity,
            trail_lag: 0.0,
            trail_offset: Vector3::new(0.0, 0.0, 0.0),
            gets_deleted: false,
        }
    }

    /// Give an elemental arrow a coloured halo and a point light in flight.
    ///
    /// The stock elemental arrow sprites are only small recolours of the plain
    /// arrow — at projectile size and speed a Fire Arrow reads as "an arrow",
    /// not as fire. The halo is our own embellishment: it re-draws the arrow's
    /// own texture larger, additively, in the element's colour, so no new art
    /// is needed and the shape always matches whatever sprite was chosen.
    ///
    /// It also covers the three elemental arrows that ship *no* distinct sprite
    /// (Frozen, Counter Evil, Holy), which the sprite path alone cannot reach.
    pub fn with_elemental_glow(mut self, color: Color, point_light_id: PointLightId) -> Self {
        self.glow_color = Some(color);
        self.point_light_id = Some(point_light_id);
        // Dimmer than a spell's travel ball (Fire Ball is 48): an arrow is a far
        // smaller thing and should tint the ground it passes, not light the map.
        self.light_intensity = 26.0;
        self
    }

    /// Turn this copy into a trailing shard rather than the head of the shot.
    ///
    /// Not a fidelity feature — the original client's effect 27 is a single
    /// travelling `effect\ice` texture with no particle data in the recovered
    /// table. This is our own embellishment so the shot reads as a spray of
    /// ice, and it carries no point light (only the head does, or every shard
    /// would stack into one blown-out glare).
    pub fn with_trail(mut self, lag: f32, offset: Vector3<f32>, size_scale: f32, alpha_scale: f32) -> Self {
        self.trail_lag = lag;
        self.trail_offset = offset;
        self.size *= size_scale;
        self.color.alpha *= alpha_scale;
        self.glow_color = None;
        self.point_light_id = None;
        self.light_intensity = 0.0;
        self
    }

    /// WZ_JUPITEL travel ball, matching the original client's effect 93: the
    /// six `thunder_ball_*` frames cycle (~10 ms each in the C++ client; a
    /// touch slower here so the crackle reads at our travel durations) over a
    /// static `thunder_center` glow, both additive, screen-upright.
    pub fn jupitel_ball(
        frames: Vec<Arc<Texture>>,
        core_texture: Arc<Texture>,
        source: Point3<f32>,
        target: Point3<f32>,
        duration: f32,
        point_light_id: PointLightId,
    ) -> Self {
        let lift = Vector3::new(0.0, 8.0, 0.0);
        let lead = frames.first().cloned().unwrap_or_else(|| core_texture.clone());
        Self {
            texture: lead,
            frames,
            frame_delay_ms: 24.0,
            core_texture: Some(core_texture),
            source: source + lift,
            target: target + lift,
            elapsed: 0.0,
            duration,
            size: Vector2::new(90.0, 90.0),
            angle_offset: 0.0,
            align_to_travel: false,
            color: Color::rgb_u8(255, 250, 200),
            glow_color: None,
            point_light_id: Some(point_light_id),
            light_intensity: 48.0,
            trail_lag: 0.0,
            trail_offset: Vector3::new(0.0, 0.0, 0.0),
            gets_deleted: false,
        }
    }

    fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    fn position_at(&self, progress: f32) -> Point3<f32> {
        // A trail shard rides the same path, just further back along it. Clamped
        // at 0 so shards emerge from the caster instead of behind them.
        let progress = (progress - self.trail_lag).max(0.0);
        self.source + (self.target - self.source) * progress + self.trail_offset
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

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        let Some(point_light_id) = self.point_light_id else {
            return;
        };
        let progress = self.progress();
        // Peak mid-flight, fade in/out at the ends.
        let intensity = (progress * PI).sin().max(0.0) * self.light_intensity;
        if intensity <= 0.5 {
            return;
        }
        let light_position = self.position_at(progress);
        // An elemental arrow lights the scene in its element, not in the white of
        // its sprite tint.
        let light_color = self.glow_color.unwrap_or(self.color);
        if Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(light_position, intensity)) {
            point_light_manager.register_fading(point_light_id, light_position, light_color, intensity, self.light_intensity);
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let progress = self.progress();
        let position = self.position_at(progress);

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
        let angle = if self.align_to_travel {
            Rad(travel_angle.0 + self.angle_offset)
        } else {
            Rad(self.angle_offset)
        };

        let mut color = self.color;
        color.alpha *= 1.0 - progress * 0.25;

        let mut draw = |texture: &Arc<Texture>, half: Vector2<f32>, color: Color| {
            renderer.render_effect(
                camera,
                position,
                texture.clone(),
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
        };

        // Elemental halo under everything else: the same sprite, enlarged and
        // tinted. Additive blending means the colour only ever brightens, so the
        // arrow keeps its own shape and reads as glowing rather than repainted.
        // It pulses over the flight so a fast arrow still registers as alive.
        if let Some(glow) = self.glow_color {
            let mut glow_color = glow;
            glow_color.alpha = color.alpha * (0.45 + 0.25 * (progress * PI).sin().max(0.0));
            draw(&self.texture, self.size * (1.9 / 2.0), glow_color);
        }

        // Static glow first so the animated crackle layers over it.
        if let Some(core) = &self.core_texture {
            let mut core_color = color;
            core_color.alpha *= 0.66;
            draw(core, self.size * 0.39, core_color);
        }

        let texture = if self.frames.is_empty() {
            &self.texture
        } else {
            let elapsed_ms = self.elapsed * 1000.0;
            &self.frames[(elapsed_ms / self.frame_delay_ms.max(1.0)) as usize % self.frames.len()]
        };
        draw(texture, self.size / 2.0, color);
    }
}

/// Phase E1 — MG_SOULSTRIKE: one ghost orb per hit, staggered, caster → target.
/// Classic Soul Strike is code-drawn orbs, not a modern nested STR for travel.
#[allow(dead_code)]
pub struct SoulStrikeOrbs {
    texture: Arc<Texture>,
    source: Point3<f32>,
    target: Point3<f32>,
    delays: Vec<f32>,
    elapsed: f32,
    total_duration: f32,
    point_light_id: PointLightId,
    gets_deleted: bool,
}

/// Default single-orb travel when the impact delay is unknown.
#[allow(dead_code)]
const DEFAULT_ORB_TRAVEL: f32 = 0.32;
#[allow(dead_code)]
const SOUL_LIGHT: Color = Color::rgb_u8(190, 120, 255);
#[allow(dead_code)]
const SOUL_LIGHT_INTENSITY: f32 = 38.0;

#[allow(dead_code)]
impl SoulStrikeOrbs {
    /// `arrival_secs` is the impact due boundary (seconds). Orbs are packed so
    /// the last one lands near that boundary instead of overshooting it.
    pub fn new(
        texture: Arc<Texture>,
        source: Point3<f32>,
        target: Point3<f32>,
        orb_count: usize,
        arrival_secs: f32,
        point_light_id: PointLightId,
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
            point_light_id,
            gets_deleted: false,
        }
    }

    fn travel_duration(&self) -> f32 {
        let last_delay = self.delays.last().copied().unwrap_or(0.0);
        (self.total_duration - last_delay).max(0.08)
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

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        let travel = self.travel_duration();
        // Light follows the first still-traveling orb (lead of the volley).
        for delay in &self.delays {
            let progress = (self.elapsed - delay) / travel;
            if !(0.0..1.0).contains(&progress) {
                continue;
            }
            let lateral = Vector3::new((progress - 0.5) * 1.5, (1.0 - progress) * 2.0, (progress - 0.5) * -1.2);
            let light_position = self.source + (self.target - self.source) * progress + lateral;
            let intensity = (1.0 - progress) * SOUL_LIGHT_INTENSITY;
            if intensity > 0.5
                && Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(light_position, intensity))
            {
                point_light_manager.register_fading(self.point_light_id, light_position, SOUL_LIGHT, intensity, SOUL_LIGHT_INTENSITY);
            }
            break;
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let half = Vector2::new(48.0, 48.0);
        let travel = self.travel_duration();
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
