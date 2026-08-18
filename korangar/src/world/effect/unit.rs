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
use crate::graphics::{Color, GroundDecalBlend, Texture};
use crate::loaders::GAT_TILE_SIZE;
use crate::renderer::{EffectRenderer, GROUND_DECAL_TEXTURE_COORDINATES};
use crate::world::{Camera, PointLightId, PointLightManager};

/// Breathing size animation. The original's elemental field units (Volcano,
/// Deluge, Violent Gale) scale between 0.5× and 1.0× rather than standing
/// still.
pub struct UnitPulse {
    pub min_scale: f32,
    pub max_scale: f32,
    /// Radians/second of the sine driving the scale.
    pub speed: f32,
}

impl UnitPulse {
    /// Radius multiplier at `age` seconds, offset by a per-cell `phase` in
    /// radians. A ground field is one unit *per cell*, so without the phase all
    /// 81 cells of a 9×9 breathe in lockstep and the field reads as one slab
    /// changing size rather than a surface that is alive.
    fn scale_at(&self, age: f32, phase: f32) -> f32 {
        let wave = 0.5 + 0.5 * (age * self.speed + phase).sin();
        self.min_scale + (self.max_scale - self.min_scale) * wave
    }
}

/// Per-cell animation offset, stable for a given cell so a unit re-sent when a
/// player walks into view keeps the phase it already had. Derived from the cell
/// coordinates rather than spawn order, because the server's cell order is
/// row-major from a corner and would make the field sweep rather than shimmer.
fn cell_phase(position: Point3<f32>) -> f32 {
    let x = (position.x / GAT_TILE_SIZE).floor() as i32 as u32;
    let z = (position.z / GAT_TILE_SIZE).floor() as i32 as u32;
    hash01(x.wrapping_mul(73_856_093) ^ z.wrapping_mul(19_349_663))
}

/// How long a cell takes to fade in, and the spread of the per-cell delay in
/// front of it. Cells arrive in one burst; staggering their appearance makes
/// the field bloom instead of popping into existence complete.
const CELL_FADE_IN: f32 = 0.35;
const CELL_FADE_IN_SPREAD: f32 = 0.3;
/// Fade on the way out. The server removes a field cell by cell, so without
/// this the tiles blink out.
const CELL_FADE_OUT: f32 = 0.45;

/// Shared fade/stagger state for the ground-tile bodies.
struct CellFade {
    age: f32,
    delay: f32,
    /// Seconds since removal was requested; `None` while the unit is alive.
    fading_out: Option<f32>,
}

impl CellFade {
    fn new(phase: f32) -> Self {
        Self {
            age: 0.0,
            delay: phase * CELL_FADE_IN_SPREAD,
            fading_out: None,
        }
    }

    /// Returns whether the unit is still alive. A unit being torn down stays
    /// alive until it has faded, which is why this owns the lifetime decision.
    fn update(&mut self, delta_time: f32) -> bool {
        self.age += delta_time;

        match self.fading_out.as_mut() {
            Some(elapsed) => {
                *elapsed += delta_time;
                *elapsed < CELL_FADE_OUT
            }
            None => true,
        }
    }

    fn begin_fade_out(&mut self) {
        self.fading_out.get_or_insert(0.0);
    }

    /// Alpha multiplier for this frame.
    fn opacity(&self) -> f32 {
        match self.fading_out {
            Some(elapsed) => (1.0 - elapsed / CELL_FADE_OUT).clamp(0.0, 1.0),
            None => ((self.age - self.delay) / CELL_FADE_IN).clamp(0.0, 1.0),
        }
    }
}

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
    /// Optional breathing scale; `None` holds a fixed size.
    pub pulse: Option<UnitPulse>,
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
    /// Unwrapped lifetime, driving [`UnitPulse`]. Kept separate from `spin`,
    /// which wraps at `TAU` and would step the pulse on every wrap.
    age: f32,
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
            age: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for UnitCylinders {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.spin = (self.spin + delta_time) % TAU;
        self.age += delta_time;
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
            .map(|spec| {
                // Cull against the pulse's widest extent, never the current one.
                let widest = spec.pulse.as_ref().map_or(1.0, |pulse| pulse.max_scale.max(1.0));
                spec.height.max(spec.bottom_radius * widest).max(spec.top_radius * widest)
            })
            .fold(0.0, f32::max);
        if !Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(self.position, culling_radius)) {
            return;
        }

        for spec in self.specs {
            let rotation = spec.yaw + self.spin * spec.spin_speed;
            // Phase 0: the cylinder bodies are stacked layers of one unit, not
            // one unit per cell, so they are meant to breathe together.
            let scale = spec.pulse.as_ref().map_or(1.0, |pulse| pulse.scale_at(self.age, 0.0));
            let (bottom_radius, top_radius) = (spec.bottom_radius * scale, spec.top_radius * scale);
            let mut color = self.color;
            color.alpha = spec.alpha;
            let sides = spec.sides.max(3);

            for segment in 0..sides {
                let angle_start = rotation + segment as f32 / sides as f32 * TAU;
                let angle_end = rotation + (segment + 1) as f32 / sides as f32 * TAU;

                let ring_point =
                    |angle: f32, radius: f32, height: f32| self.position + Vector3::new(angle.cos() * radius, height, angle.sin() * radius);

                let corners = [
                    ring_point(angle_start, top_radius, spec.height),
                    ring_point(angle_end, top_radius, spec.height),
                    ring_point(angle_start, bottom_radius, 0.0),
                    ring_point(angle_end, bottom_radius, 0.0),
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

/// Light-only companion for unit bodies that don't render their own point
/// light ([`UnitGroundQuad`], looping sprite units). [`UnitCylinders`] and the
/// STR-backed units register theirs directly; this fills the gap so a
/// recipe's declared `light` always reaches the world.
pub struct UnitPointLight {
    position: Point3<f32>,
    point_light_id: PointLightId,
    color: Color,
    intensity: f32,
    gets_deleted: bool,
}

impl UnitPointLight {
    pub fn new(position: Point3<f32>, point_light_id: PointLightId, color: Color, intensity: f32) -> Self {
        Self {
            position,
            point_light_id,
            color,
            intensity,
            gets_deleted: false,
        }
    }
}

impl EffectBase for UnitPointLight {
    fn update(&mut self, _entities: &[crate::world::Entity], _delta_time: f32) -> bool {
        !self.gets_deleted
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        if self.intensity <= 0.0 {
            return;
        }
        let light_position = self.position + Vector3::new(0.0, 4.0, 0.0);
        if Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(light_position, self.intensity)) {
            point_light_manager.register_fading(self.point_light_id, light_position, self.color, self.intensity, self.intensity);
        }
    }

    fn render(&self, _renderer: &mut EffectRenderer, _camera: &dyn Camera) {}
}

/// Flat pulsing floor tile, one per unit cell — the original's `LPEffect`
/// (Land Protector), a single ground-aligned quad that breathes rather than
/// any 3D body.
pub struct UnitGroundQuad {
    texture: Arc<Texture>,
    position: Point3<f32>,
    /// Half-width in world units.
    half_size: f32,
    color: Color,
    pulse: UnitPulse,
    blend: GroundDecalBlend,
    /// Per-cell animation offset in radians.
    phase: f32,
    fade: CellFade,
}

impl UnitGroundQuad {
    /// Lift off the terrain so the tile never z-fights the ground mesh.
    const GROUND_LIFT: f32 = 0.6;

    pub fn new(
        texture: Arc<Texture>,
        position: Point3<f32>,
        half_size: f32,
        color: Color,
        pulse: UnitPulse,
        blend: GroundDecalBlend,
    ) -> Self {
        let phase = cell_phase(position);

        Self {
            texture,
            position,
            half_size,
            color,
            pulse,
            blend,
            phase: phase * TAU,
            fade: CellFade::new(phase),
        }
    }
}

impl EffectBase for UnitGroundQuad {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.fade.update(delta_time)
    }

    fn mark_for_deletion(&mut self) {
        self.fade.begin_fade_out();
    }

    fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let widest = self.half_size * self.pulse.max_scale.max(1.0);
        if !Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(self.position, widest)) {
            return;
        }

        let opacity = self.fade.opacity();
        if opacity <= 0.0 {
            return;
        }
        let color = Color {
            alpha: self.color.alpha * opacity,
            ..self.color
        };

        let half = self.half_size * self.pulse.scale_at(self.fade.age, self.phase);
        let center = self.position + Vector3::new(0.0, Self::GROUND_LIFT, 0.0);
        let corner = |x: f32, z: f32| center + Vector3::new(x * half, 0.0, z * half);

        // A ground-parallel tile must be depth-tested, unlike the vertical
        // cylinder/horn bodies: the postprocessing effect path composites on top
        // of everything, which would draw the tile over a player standing on it.
        // The decal path draws it in the forward pass so terrain occludes it and
        // entities compose over it.
        renderer.render_ground_decal(
            [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
            self.texture.clone(),
            GROUND_DECAL_TEXTURE_COORDINATES,
            color,
            self.blend,
        );
    }
}

/// The authentic classic ground-tile recipe, which is **two** layers rather
/// than one (roBrowserLegacy `Renderer/Effects/Songs.js`):
///
/// 1. a `FlatColorTile` per cell — a flat tint with no artwork, and
/// 2. a `Tiles.HoveringTexture` above it, bobbing between `+0.2` and `+0.6`
///    cell (`z + 0.4 - 0.2·sin(oddEven + tick/540π)`) with a per-cell phase so
///    neighbouring cells do not rise and fall together.
///
/// Approximating this as a single textured quad would lose both the bob and the
/// separation between tint and artwork, which is why it needed its own body.
/// Unblocks `PA_GOSPEL`, `PF_FOGWALL` and `NPC_EVILLAND`, whose only missing
/// piece was this.
/// The hovering half of a layered ground unit — the artwork riding above the
/// tint, and everything that decides how it draws.
///
/// Grouped rather than passed loose: these five travel together, `new` was at
/// nine arguments once the frame list and the blend family joined them, and
/// "hover layer" is a concept the type is already named for.
pub struct HoverLayer {
    /// One entry draws a still layer; several cycle at `fps`.
    pub frames: Vec<Arc<Texture>>,
    pub fps: f32,
    /// Half-width in world units.
    pub half_size: f32,
    pub opacity: f32,
    pub blend: GroundDecalBlend,
}

pub struct UnitLayeredGroundQuad {
    tile_texture: Arc<Texture>,
    hover: HoverLayer,
    position: Point3<f32>,
    half_size: f32,
    tile_color: Option<Color>,
    phase: f32,
    fade: CellFade,
}

impl UnitLayeredGroundQuad {
    /// Bob midpoint and swing, in cells, straight from the original's
    /// `z + 0.4 - 0.2·sin(…)`.
    const BOB_CENTER_CELLS: f32 = 0.4;
    /// `tick/540π` is radians per millisecond, so a second advances the sine by
    /// `1000/540π`.
    const BOB_SPEED: f32 = 0.589;
    const BOB_SWING_CELLS: f32 = 0.2;

    pub fn new(tile_texture: Arc<Texture>, hover: HoverLayer, position: Point3<f32>, half_size: f32, tile_color: Option<Color>) -> Self {
        let phase = cell_phase(position);

        Self {
            tile_texture,
            hover,
            position,
            half_size,
            tile_color,
            phase: phase * TAU,
            fade: CellFade::new(phase),
        }
    }

    /// The frame to draw this instant.
    ///
    /// Offset by the cell's own phase so a field boils rather than blinking in
    /// unison — the same trick the bob uses, and the reason a 15-cell wall does
    /// not look like one animation played fifteen times.
    fn hover_frame(&self) -> &Arc<Texture> {
        if self.hover.frames.len() < 2 || self.hover.fps <= 0.0 {
            return &self.hover.frames[0];
        }

        let count = self.hover.frames.len() as f32;
        let advance = self.fade.age * self.hover.fps + self.phase * count / TAU;
        let index = (advance.rem_euclid(count)) as usize;
        &self.hover.frames[index.min(self.hover.frames.len() - 1)]
    }

    /// Height of the hovering layer above the terrain, in world units.
    fn hover_lift(&self) -> f32 {
        let bob = Self::BOB_CENTER_CELLS - Self::BOB_SWING_CELLS * (self.phase + self.fade.age * Self::BOB_SPEED).sin();
        bob * GAT_TILE_SIZE
    }
}

impl EffectBase for UnitLayeredGroundQuad {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.fade.update(delta_time)
    }

    fn mark_for_deletion(&mut self) {
        self.fade.begin_fade_out();
    }

    fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let widest = self.half_size.max(self.hover.half_size);
        if !Frustum::new(camera.view_projection_matrix(), true).intersects_sphere(&Sphere::new(self.position, widest)) {
            return;
        }

        let opacity = self.fade.opacity();
        if opacity <= 0.0 {
            return;
        }

        let quad = |center: Point3<f32>, half: f32| {
            let corner = |x: f32, z: f32| center + Vector3::new(x * half, 0.0, z * half);
            [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)]
        };

        // Lower layer: the flat tint, on the ground like any other decal. Absent
        // for recipes that are only their hovering artwork.
        if let Some(tile_color) = self.tile_color {
            let tile_center = self.position + Vector3::new(0.0, UnitGroundQuad::GROUND_LIFT, 0.0);
            renderer.render_ground_decal(
                quad(tile_center, self.half_size),
                self.tile_texture.clone(),
                GROUND_DECAL_TEXTURE_COORDINATES,
                Color {
                    alpha: tile_color.alpha * opacity,
                    ..tile_color
                },
                // The tint is a flat colour, never artwork, so it is always the
                // ordinary blend whatever family the hover layer belongs to.
                GroundDecalBlend::Alpha,
            );
        }

        // Upper layer: the artwork, riding above the tint.
        let hover_center = self.position + Vector3::new(0.0, self.hover_lift(), 0.0);
        renderer.render_ground_decal(
            quad(hover_center, self.hover.half_size),
            self.hover_frame().clone(),
            GROUND_DECAL_TEXTURE_COORDINATES,
            Color::rgba(1.0, 1.0, 1.0, self.hover.opacity * opacity),
            self.hover.blend,
        );
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
