mod bolts;
mod burst;
mod portal;
mod projectile;
mod unit;

use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2, Vector3};
use korangar_collision::{Frustum, Sphere};
use korangar_container::Cacheable;
use ragnarok_formats::map::EffectSource;
use ragnarok_packets::{EntityId, SkillId};
use wgpu::BlendFactor;

pub use self::bolts::FallingBolts;
pub use self::burst::{SkillBurst, SkillBurstStyle};
pub use self::portal::{PORTAL_TEXTURE_PATH, PortalVortex};
pub use self::projectile::{SkillProjectile, SoulStrikeOrbs};
pub use self::unit::{UnitCylinderSpec, UnitCylinders, UnitGroundQuad, UnitIceHorns, UnitPointLight, UnitPulse};
use crate::graphics::{Color, Texture};
use crate::renderer::EffectRenderer;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::{Camera, PointLightId, PointLightManager};

pub trait EffectBase {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool;

    fn mark_for_deletion(&mut self);

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera);

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera);
}

pub trait EffectSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool);
}

impl EffectSourceExt for EffectSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool) {
        renderer.render_marker(camera, marker_identifier, self.position, hovered);
    }
}

pub struct Effect {
    frames_per_second: usize,
    max_key: usize,
    layers: Vec<Layer>,
}

impl Effect {
    pub fn new(frames_per_second: usize, max_key: usize, layers: Vec<Layer>) -> Self {
        Self {
            frames_per_second,
            max_key,
            layers,
        }
    }
}

impl Effect {
    pub fn new_frame_timer(&self) -> FrameTimer {
        FrameTimer {
            total_timer: 0.0,
            frames_per_second: self.frames_per_second,
            max_key: self.max_key,
            current_frame: 0,
        }
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, frame_timer: &FrameTimer, position: Point3<f32>) {
        for layer in &self.layers {
            let Some(frame) = layer.interpolate_frame(frame_timer) else {
                continue;
            };

            // Truncation matches the original client; `as usize` also
            // saturates a NaN or negative index to zero.
            let texture_index = frame.texture_index as usize;

            if texture_index >= layer.textures.len() {
                continue;
            }

            renderer.render_effect(
                camera,
                position,
                layer.textures[texture_index].clone(),
                [
                    Vector2::new(frame.xy[0], frame.xy[4]),
                    Vector2::new(frame.xy[1], frame.xy[5]),
                    Vector2::new(frame.xy[3], frame.xy[7]),
                    Vector2::new(frame.xy[2], frame.xy[6]),
                ],
                [
                    Vector2::new(frame.uv[0] + frame.uv[2], frame.uv[3] + frame.uv[1]),
                    Vector2::new(frame.uv[0] + frame.uv[2], frame.uv[1]),
                    Vector2::new(frame.uv[0], frame.uv[1]),
                    Vector2::new(frame.uv[0], frame.uv[3] + frame.uv[1]),
                ],
                frame.offset,
                frame.angle,
                frame.color,
                frame.source_blend_factor,
                frame.destination_blend_factor,
            );
        }
    }
}

impl Cacheable for Effect {
    fn size(&self) -> usize {
        // We cache effects only by count.
        0
    }
}

pub struct Layer {
    textures: Vec<Arc<Texture>>,
    frames: Vec<Frame>,
}

impl Layer {
    pub fn new(textures: Vec<Arc<Texture>>, frames: Vec<Frame>) -> Self {
        Self { textures, frames }
    }
}

impl Layer {
    /// STR key frames come in two types: a basic frame (type 0) sets every
    /// field absolutely, while a morphing frame (type 1) that shares the
    /// basic frame's key holds per-key deltas that are added onto it for
    /// every key until the next basic frame. The morphing frame also drives
    /// the texture animation through its animation type and delay fields.
    fn interpolate_frame(&self, frame_timer: &FrameTimer) -> Option<Frame> {
        Self::frame_at(&self.frames, frame_timer.fractional_frame(), self.textures.len())
    }

    fn frame_at(frames: &[Frame], key_index: f32, texture_count: usize) -> Option<Frame> {
        let mut base_index = None;
        let mut morph_index = None;
        let mut last_key = 0;
        let mut last_base_key = 0;

        for (index, frame) in frames.iter().enumerate() {
            if (frame.frame_index as f32) <= key_index {
                match frame.frame_type {
                    FrameType::Basic => base_index = Some(index),
                    FrameType::Morphing => morph_index = Some(index),
                }
            }

            last_key = last_key.max(frame.frame_index);

            if let FrameType::Basic = frame.frame_type {
                last_base_key = last_base_key.max(frame.frame_index);
            }
        }

        let base_index = base_index?;

        if morph_index.is_none() && (last_key as f32) < key_index {
            return None;
        }

        let base_frame = &frames[base_index];

        match morph_index {
            Some(morph_index) if morph_index == base_index + 1 && frames[morph_index].frame_index == base_frame.frame_index => {
                let morph_frame = &frames[morph_index];
                let delta = key_index - base_frame.frame_index as f32;
                Some(Self::apply_morph(base_frame, morph_frame, delta, texture_count))
            }
            // A morphing frame exists somewhere but the current basic frame is
            // the layer's last one, so there is nothing left to animate into.
            Some(_) if base_frame.frame_index >= last_base_key => None,
            _ => Some(base_frame.clone()),
        }
    }

    fn apply_morph(base_frame: &Frame, morph_frame: &Frame, delta: f32, texture_count: usize) -> Frame {
        let color = base_frame.color + morph_frame.color * delta;
        let angle = Rad(base_frame.angle.0 + morph_frame.angle.0 * delta);
        let offset = base_frame.offset + morph_frame.offset * delta;

        let uv = (0..8)
            .map(|index| base_frame.uv[index] + morph_frame.uv[index] * delta)
            .next_chunk()
            .unwrap();

        let xy = (0..8)
            .map(|index| base_frame.xy[index] + morph_frame.xy[index] * delta)
            .next_chunk()
            .unwrap();

        let texture_count = texture_count as f32;
        let texture_index = match morph_frame.animation_type {
            // The original client resets the texture on this animation type.
            AnimationType::Type0 => 0.0,
            AnimationType::Type1 => base_frame.texture_index + morph_frame.texture_index * delta,
            // Advance by `delay` textures per key, stopping at the last one.
            AnimationType::Type2 => (base_frame.texture_index + morph_frame.delay * delta).min(texture_count - 1.0),
            // Advance by `delay` textures per key, wrapping around.
            AnimationType::Type3 => (base_frame.texture_index + morph_frame.delay * delta).rem_euclid(texture_count),
            // Like type 3, but playing in reverse.
            AnimationType::Type4 => (base_frame.texture_index - morph_frame.delay * delta).rem_euclid(texture_count),
        };

        Frame {
            frame_index: base_frame.frame_index,
            frame_type: base_frame.frame_type,
            offset,
            uv,
            xy,
            texture_index,
            animation_type: base_frame.animation_type,
            delay: base_frame.delay,
            angle,
            color,
            source_blend_factor: base_frame.source_blend_factor,
            destination_blend_factor: base_frame.destination_blend_factor,
            mt_present: base_frame.mt_present,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    frame_index: usize,
    frame_type: FrameType,
    offset: Vector2<f32>,
    uv: [f32; 8],
    xy: [f32; 8],
    texture_index: f32,
    animation_type: AnimationType,
    delay: f32,
    angle: Rad<f32>,
    color: Color,
    source_blend_factor: BlendFactor,
    destination_blend_factor: BlendFactor,
    mt_present: MultiTexturePresent,
}

impl Frame {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame_index: usize,
        frame_type: FrameType,
        offset: Vector2<f32>,
        uv: [f32; 8],
        xy: [f32; 8],
        texture_index: f32,
        animation_type: AnimationType,
        delay: f32,
        angle: Rad<f32>,
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
        mt_present: MultiTexturePresent,
    ) -> Self {
        Self {
            frame_index,
            frame_type,
            offset,
            uv,
            xy,
            texture_index,
            animation_type,
            delay,
            angle,
            color,
            source_blend_factor,
            destination_blend_factor,
            mt_present,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnimationType {
    Type0,
    Type1,
    Type2,
    Type3,
    Type4,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameType {
    Basic,
    Morphing,
}

#[derive(Debug, Clone, Copy)]
pub enum MultiTexturePresent {
    None,
}

pub struct FrameTimer {
    total_timer: f32,
    frames_per_second: usize,
    max_key: usize,
    current_frame: usize,
}

impl FrameTimer {
    /// The key index including the fractional progress towards the next key,
    /// so morph deltas advance smoothly between keys.
    fn fractional_frame(&self) -> f32 {
        self.total_timer * self.frames_per_second as f32
    }

    pub fn update(&mut self, delta_time: f32) -> bool {
        // Loading an STR and its textures is currently synchronous. On the
        // first use of a large effect (notably Meteor Storm), that work can
        // make the following application frame substantially longer than the
        // animation itself. Do not let an asset-loading stall skip the whole
        // effect before the player gets a chance to see it.
        const MAX_ANIMATION_STEP: f32 = 1.0 / 15.0;
        self.total_timer += delta_time.min(MAX_ANIMATION_STEP);
        self.current_frame = (self.total_timer / (1.0 / self.frames_per_second as f32)) as usize;

        if self.current_frame >= self.max_key {
            // TODO: better wrapping
            self.total_timer = 0.0;
            self.current_frame = 0;
            return false;
        }

        true
    }
}

#[cfg(test)]
mod frame_at_tests {
    use cgmath::{Rad, Vector2};
    use wgpu::BlendFactor;

    use super::{AnimationType, Frame, FrameTimer, FrameType, Layer};
    use crate::graphics::Color;

    fn frame(key: usize, frame_type: FrameType, animation_type: AnimationType, texture_index: f32, delay: f32) -> Frame {
        Frame::new(
            key,
            frame_type,
            Vector2::new(320.0, 290.0),
            [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            [-64.0, 64.0, 64.0, -64.0, -64.0, -64.0, 64.0, 64.0],
            texture_index,
            animation_type,
            delay,
            Rad(0.0),
            Color::rgba(1.0, 1.0, 1.0, 0.8),
            BlendFactor::SrcAlpha,
            BlendFactor::DstAlpha,
            super::MultiTexturePresent::None,
        )
    }

    fn morph_delta(key: usize, animation_type: AnimationType, texture_delta: f32, delay: f32, xy_delta: f32) -> Frame {
        Frame::new(
            key,
            FrameType::Morphing,
            Vector2::new(0.0, 0.0),
            [0.0; 8],
            [xy_delta; 8],
            texture_delta,
            animation_type,
            delay,
            Rad(0.0),
            Color::rgba(0.0, 0.0, 0.0, 0.0),
            BlendFactor::SrcAlpha,
            BlendFactor::DstAlpha,
            super::MultiTexturePresent::None,
        )
    }

    /// The shape of `lightningstrike\lightningstrike.str` layer 1: one basic
    /// frame at key 0, a zero-delta morphing frame at key 0 that cycles 21
    /// textures at 0.4 textures per key, and a final basic frame at key 53.
    #[test]
    fn morph_driven_texture_cycle_is_visible_and_advances() {
        let frames = vec![
            frame(0, FrameType::Basic, AnimationType::Type3, 0.0, 0.4),
            morph_delta(0, AnimationType::Type3, 0.0, 0.4, 0.0),
            frame(53, FrameType::Basic, AnimationType::Type3, 20.0, 0.4),
        ];

        for key in [0.0, 10.0, 25.0, 52.9] {
            let result = Layer::frame_at(&frames, key, 21).expect("layer must be visible during the animation");
            let expected_texture = (0.4 * key) % 21.0;
            assert!((result.texture_index - expected_texture).abs() < 0.001, "texture at key {key}");
            assert!(
                (result.xy[0] - result.xy[1]).abs() > 1.0,
                "quad must not be degenerate at key {key}"
            );
            assert!(result.color.alpha > 0.0, "frame must not be fully transparent at key {key}");
        }
    }

    /// The shape of `cloudh.str` layer 1: basic + morphing pairs at keys 0,
    /// 27, and 55, and a final basic frame at key 80. Morph deltas apply
    /// per key from the paired basic frame.
    #[test]
    fn morph_deltas_accumulate_from_the_paired_basic_frame() {
        let frames = vec![
            frame(0, FrameType::Basic, AnimationType::Type0, 0.0, 0.0),
            morph_delta(0, AnimationType::Type0, 0.0, 0.0, 0.5),
            frame(27, FrameType::Basic, AnimationType::Type0, 0.0, 0.0),
            morph_delta(27, AnimationType::Type0, 0.0, 0.0, -0.5),
            frame(80, FrameType::Basic, AnimationType::Type0, 0.0, 0.0),
        ];

        let result = Layer::frame_at(&frames, 10.0, 1).expect("visible in the first segment");
        assert!((result.xy[0] - (-64.0 + 0.5 * 10.0)).abs() < 0.001);

        let result = Layer::frame_at(&frames, 30.0, 1).expect("visible in the second segment");
        assert!((result.xy[0] - (-64.0 - 0.5 * 3.0)).abs() < 0.001);

        // The final basic frame has no morph to animate into; the original
        // client stops drawing the layer there.
        assert!(Layer::frame_at(&frames, 80.0, 1).is_none());
    }

    /// The shape of `storm_min.str`: dense basic-only keys display statically,
    /// and the layer is hidden before its first key and after its last.
    #[test]
    fn basic_only_layers_display_statically_within_their_key_range() {
        let frames = vec![
            frame(15, FrameType::Basic, AnimationType::Type0, 0.0, 0.0),
            frame(16, FrameType::Basic, AnimationType::Type0, 0.0, 0.0),
            frame(17, FrameType::Basic, AnimationType::Type0, 0.0, 0.0),
        ];

        assert!(Layer::frame_at(&frames, 3.0, 1).is_none(), "hidden before the first key");
        assert!(Layer::frame_at(&frames, 16.5, 1).is_some(), "visible between keys");
        assert!(Layer::frame_at(&frames, 40.0, 1).is_none(), "hidden after the last key");

        let result = Layer::frame_at(&frames, 16.5, 1).unwrap();
        assert_eq!(result.frame_index, 16, "static frames must not interpolate");
    }

    /// A single basic frame previously never rendered at all because the
    /// old key-to-frame index map skipped lone frames.
    #[test]
    fn single_frame_layers_render_on_their_key() {
        let frames = vec![frame(0, FrameType::Basic, AnimationType::Type0, 0.0, 0.0)];

        assert!(Layer::frame_at(&frames, 0.0, 1).is_some());
        assert!(Layer::frame_at(&frames, 0.9, 1).is_none(), "hidden once the key has passed");
        assert!(Layer::frame_at(&frames, 5.0, 1).is_none(), "hidden after the last key");
    }

    /// Texture animation types 2 (stop at end) and 4 (reverse) on the
    /// morphing frame.
    #[test]
    fn texture_animation_clamps_and_reverses() {
        let clamping = vec![
            frame(0, FrameType::Basic, AnimationType::Type2, 0.0, 0.0),
            morph_delta(0, AnimationType::Type2, 0.0, 1.0, 0.0),
            frame(50, FrameType::Basic, AnimationType::Type2, 0.0, 0.0),
        ];
        let result = Layer::frame_at(&clamping, 30.0, 10).unwrap();
        assert!((result.texture_index - 9.0).abs() < 0.001, "type 2 clamps at the last texture");

        let reversing = vec![
            frame(0, FrameType::Basic, AnimationType::Type4, 0.0, 0.0),
            morph_delta(0, AnimationType::Type4, 0.0, 1.0, 0.0),
            frame(50, FrameType::Basic, AnimationType::Type4, 0.0, 0.0),
        ];
        let result = Layer::frame_at(&reversing, 3.0, 10).unwrap();
        assert!(
            (result.texture_index - 7.0).abs() < 0.001,
            "type 4 plays backwards with wrap-around"
        );
    }

    #[test]
    fn asset_loading_stall_does_not_skip_an_entire_effect() {
        let mut timer = FrameTimer {
            total_timer: 0.0,
            frames_per_second: 60,
            max_key: 60,
            current_frame: 0,
        };

        assert!(timer.update(2.0), "a long loading frame must not finish a one-second effect");
        assert_eq!(timer.current_frame, 4);
    }
}

pub enum EffectCenter {
    Entity(EntityId, Point3<f32>),
    Position(Point3<f32>),
}

impl EffectCenter {
    fn to_position(&self) -> Point3<f32> {
        match self {
            EffectCenter::Entity(_, position) | EffectCenter::Position(position) => *position,
        }
    }
}

pub struct EffectWithLight {
    effect: Arc<Effect>,
    frame_timer: FrameTimer,
    center: EffectCenter,
    effect_offset: Vector3<f32>,
    point_light_id: PointLightId,
    light_offset: Vector3<f32>,
    light_color: Color,
    light_intensity: f32,
    repeating: bool,
    /// Seconds to wait before the effect starts playing (e.g. a hit burst
    /// that must coincide with the landing of a bolt volley).
    start_delay: f32,
    current_light_intensity: f32,
    gets_deleted: bool,
}

impl EffectWithLight {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        effect: Arc<Effect>,
        frame_timer: FrameTimer,
        center: EffectCenter,
        effect_offset: Vector3<f32>,
        point_light_id: PointLightId,
        light_offset: Vector3<f32>,
        light_color: Color,
        light_intensity: f32,
        repeating: bool,
        start_delay: f32,
    ) -> Self {
        Self {
            effect,
            frame_timer,
            center,
            effect_offset,
            point_light_id,
            light_offset,
            light_color,
            light_intensity,
            repeating,
            start_delay,
            current_light_intensity: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for EffectWithLight {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        const FADE_SPEED: f32 = 5.0;

        if let EffectCenter::Entity(entity_id, position) = &mut self.center
            && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == *entity_id)
        {
            let new_position = entity.get_position();
            *position = new_position;
        }

        if self.start_delay > 0.0 {
            self.start_delay -= delta_time;
            return true;
        }

        if !self.gets_deleted && !self.frame_timer.update(delta_time) && !self.repeating {
            self.gets_deleted = true;
        }

        let (target, clamping_function): (f32, fn(f32, f32) -> f32) = match self.gets_deleted {
            true => (0.0, f32::max),
            false => (self.light_intensity, f32::min),
        };

        self.current_light_intensity += (target - self.current_light_intensity) * FADE_SPEED * delta_time;
        self.current_light_intensity = clamping_function(self.current_light_intensity, target);

        !self.gets_deleted || self.current_light_intensity > 0.1
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        let frustum = Frustum::new(camera.view_projection_matrix(), true);

        let light_position = self.center.to_position() + self.light_offset;

        if frustum.intersects_sphere(&Sphere::new(light_position, self.current_light_intensity)) {
            point_light_manager.register_fading(
                self.point_light_id,
                light_position,
                self.light_color,
                self.current_light_intensity,
                self.light_intensity,
            )
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if !self.gets_deleted && self.start_delay <= 0.0 {
            self.effect.render(
                renderer,
                camera,
                &self.frame_timer,
                self.center.to_position() + self.effect_offset,
            );
        }
    }
}

/// Independent once-per-cast gates so a travel ball claim does not suppress
/// a caster STR (and vice versa). AOE target rings use their own slot so
/// multi-target damage packets share one geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UniqueEffectSlot {
    /// Caster STR / SkillBurst on the source actor (multi-target dedup).
    CasterLayer,
    /// Source→target travel projectile that must fire once per cast
    /// (Jupitel multi-hit packets, Soul Strike multi-packet, Spear Boomerang).
    TravelProjectile,
    /// Large target AOE ring (Heaven's Drive, Napalm) — one geometry per cast.
    TargetAoe,
}

/// What owns an effect, and therefore what removes it.
///
/// Both variants key on an `EntityId`, but they must not share a channel: a
/// skill unit is removed by `RemoveSkillUnit` on the *unit's* id, while a status
/// visual is removed when the afflicted entity's opt1/opt2 clears. Keying both
/// on a bare id would let one delete the other.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum EffectAnchor {
    Unit(EntityId),
    Status(EntityId),
}

#[derive(Default)]
pub struct EffectHolder {
    effects: Vec<(Box<dyn EffectBase + Send + Sync>, Option<EffectAnchor>)>,
    unique_skill_effects: Vec<(EntityId, SkillId, UniqueEffectSlot, f32)>,
}

impl EffectHolder {
    pub fn add_effect(&mut self, effect: Box<dyn EffectBase + Send + Sync>) {
        self.effects.push((effect, None));
    }

    pub fn add_unit(&mut self, effect: Box<dyn EffectBase + Send + Sync>, entity_id: EntityId) {
        self.effects.push((effect, Some(EffectAnchor::Unit(entity_id))));
    }

    /// A looping visual tied to an entity's status effect. Replaces whatever
    /// status visual that entity already had, so a status change never stacks.
    pub fn add_status_effect(&mut self, effect: Box<dyn EffectBase + Send + Sync>, entity_id: EntityId) {
        self.remove_status_effect(entity_id);
        self.effects.push((effect, Some(EffectAnchor::Status(entity_id))));
    }

    pub fn remove_status_effect(&mut self, removed_entity_id: EntityId) {
        self.effects
            .iter_mut()
            .filter(|(_, anchor)| *anchor == Some(EffectAnchor::Status(removed_entity_id)))
            .for_each(|(effect, _)| effect.mark_for_deletion());
    }

    /// Returns true once per source/skill/`slot` during `duration`.
    pub fn claim_unique_skill_effect_slot(
        &mut self,
        source_entity_id: EntityId,
        skill_id: SkillId,
        slot: UniqueEffectSlot,
        duration: f32,
    ) -> bool {
        if self.unique_skill_effects.iter().any(|(source, skill, existing_slot, _)| {
            *source == source_entity_id && *skill == skill_id && *existing_slot == slot
        }) {
            return false;
        }

        self.unique_skill_effects
            .push((source_entity_id, skill_id, slot, duration));
        true
    }

    /// Caster-layer convenience (historical API used by Magnum / Pierce / …).
    pub fn claim_unique_skill_effect(&mut self, source_entity_id: EntityId, skill_id: SkillId, duration: f32) -> bool {
        self.claim_unique_skill_effect_slot(source_entity_id, skill_id, UniqueEffectSlot::CasterLayer, duration)
    }

    pub fn remove_unit(&mut self, removed_entity_id: EntityId) {
        self.effects
            .iter_mut()
            .filter(|(_, anchor)| *anchor == Some(EffectAnchor::Unit(removed_entity_id)))
            .for_each(|(effect, _)| effect.mark_for_deletion());
    }

    pub fn clear(&mut self) {
        self.effects.clear();
        self.unique_skill_effects.clear();
    }

    pub fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) {
        self.effects.retain_mut(|(effect, _)| effect.update(entities, delta_time));
        self.unique_skill_effects
            .iter_mut()
            .for_each(|(_, _, _, remaining)| *remaining -= delta_time);
        self.unique_skill_effects.retain(|(_, _, _, remaining)| *remaining > 0.0);
    }

    pub fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        self.effects
            .iter()
            .for_each(|(effect, _)| effect.register_point_lights(point_light_manager, camera));
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        self.effects.iter().for_each(|(effect, _)| effect.render(renderer, camera));
    }
}

#[cfg(test)]
mod effect_holder_tests {
    use ragnarok_packets::{EntityId, SkillId};

    use super::{EffectHolder, UniqueEffectSlot};

    #[test]
    fn caster_effect_is_claimed_once_until_its_gate_expires() {
        let mut holder = EffectHolder::default();
        let source = EntityId(42);
        let frost_nova = SkillId(88);

        assert!(holder.claim_unique_skill_effect(source, frost_nova, 0.5));
        assert!(!holder.claim_unique_skill_effect(source, frost_nova, 0.5));

        holder.update(&[], 0.6);
        assert!(holder.claim_unique_skill_effect(source, frost_nova, 0.5));
    }

    #[test]
    fn independent_slots_do_not_block_each_other() {
        let mut holder = EffectHolder::default();
        let source = EntityId(7);
        let jupitel = SkillId(84);

        assert!(holder.claim_unique_skill_effect_slot(source, jupitel, UniqueEffectSlot::TravelProjectile, 0.25));
        // Caster and AOE rings remain available while travel is claimed.
        assert!(holder.claim_unique_skill_effect_slot(source, jupitel, UniqueEffectSlot::CasterLayer, 0.5));
        assert!(holder.claim_unique_skill_effect_slot(source, jupitel, UniqueEffectSlot::TargetAoe, 0.4));
        // Same travel slot stays blocked.
        assert!(!holder.claim_unique_skill_effect_slot(source, jupitel, UniqueEffectSlot::TravelProjectile, 0.25));
    }
}
