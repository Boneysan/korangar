use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use cgmath::Vector2;
#[cfg(feature = "debug")]
use cgmath::{Matrix4, SquareMatrix};
#[cfg(feature = "debug")]
use korangar_container::CacheStatistics;
use korangar_container::SimpleCache;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use num::Zero;

use super::error::LoadError;
use crate::loaders::{ActionLoader, Sprite, SpriteLoader};
use crate::world::animation::{compute_action_layouts, merge_frame};
use crate::world::{ActionEvent, Actions, Animation, AnimationData, AnimationFrame, AnimationFramePart, AnimationLayer, AnimationPair};
use crate::{Color, EntityType};

const MAX_CACHE_COUNT: u32 = 256;
// We cache animations only by count.
const MAX_CACHE_SIZE: usize = usize::MAX;

pub struct AnimationLoader {
    cache: Mutex<SimpleCache<Vec<String>, Arc<AnimationData>>>,
}

impl AnimationLoader {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            )),
        }
    }

    #[cfg(feature = "debug")]
    pub fn cache_statistics(&self) -> CacheStatistics {
        self.cache.lock().unwrap().statistics()
    }

    pub fn load(
        &self,
        sprite_loader: &SpriteLoader,
        action_loader: &ActionLoader,
        entity_type: EntityType,
        entity_part_files: &[String],
    ) -> Result<Arc<AnimationData>, LoadError> {
        let animation_pairs: Vec<AnimationPair> = entity_part_files
            .iter()
            .map(|file_path| AnimationPair {
                sprites: sprite_loader.get_or_load(&format!("{file_path}.spr")).unwrap(),
                actions: action_loader.get_or_load(&format!("{file_path}.act")).unwrap(),
            })
            .collect();

        // Decode each layer independently. Cross-layer composition happens at
        // render time (Phase C): body owns the clock; secondary layers use
        // CActRes::get_motion fallback; attach is applied at compose (C2).
        let mut layers: Vec<AnimationLayer> = Vec::with_capacity(animation_pairs.len());

        for (animation_index, animation_pair) in animation_pairs.iter().enumerate() {
            let path_key = entity_part_files.get(animation_index).cloned();
            layers.push(decode_animation_layer(animation_pair, animation_index, path_key));
        }

        let delays = animation_pairs.first().map(|pair| pair.actions.delays.clone()).unwrap_or_default();
        let action_layouts = compute_action_layouts(&layers);

        let animation_data = Arc::new(AnimationData {
            layers,
            delays,
            action_layouts,
            entity_type,
        });

        let _result = self
            .cache
            .lock()
            .unwrap()
            .insert(entity_part_files.to_vec(), animation_data.clone());

        #[cfg(feature = "debug")]
        if let Err(error) = _result {
            print_debug!(
                "[{}] animation could not be added to cache. Entity Files: '{}': {:?}",
                "error".red(),
                &entity_part_files.join(";"),
                error
            );
        }

        Ok(animation_data)
    }

    /// Load a single layer for partial swaps (weapon / hair) without rebuilding
    /// body. Uses the sprite/action caches already populated by the loaders.
    pub fn load_layer(
        &self,
        sprite_loader: &SpriteLoader,
        action_loader: &ActionLoader,
        path: &str,
        layer_index: usize,
    ) -> Result<AnimationLayer, LoadError> {
        let pair = AnimationPair {
            sprites: sprite_loader.get_or_load(&format!("{path}.spr"))?,
            actions: action_loader.get_or_load(&format!("{path}.act"))?,
        };
        Ok(decode_animation_layer(&pair, layer_index, Some(path.to_owned())))
    }

    pub fn get(&self, entity_part_files: &[String]) -> Option<Arc<AnimationData>> {
        let mut lock = self.cache.lock().unwrap();
        lock.get(entity_part_files).cloned()
    }
}

/// Decode one SPR+ACT pair into an [`AnimationLayer`]. `layer_index` stamps
/// frame-part indices and whether ACT events are kept (body only).
fn decode_animation_layer(animation_pair: &AnimationPair, layer_index: usize, path_key: Option<String>) -> AnimationLayer {
    let sprites = animation_pair.sprites.clone();
    decode_animation_layer_with_sizes(
        &animation_pair.actions,
        animation_pair.sprites.palette_size,
        |sprite_number| {
            let size = sprites.textures[sprite_number].get_size();
            Vector2::new(size.width as i32, size.height as i32)
        },
        layer_index,
        path_key,
        Some(animation_pair.sprites.clone()),
    )
}

/// The decode itself, with the texture dimensions supplied by the caller.
///
/// Splitting the size lookup out is what lets an asset audit run this exact
/// code with no GPU: SPR image dimensions are in the file, but a [`Sprite`]
/// only exposes them through uploaded textures. A second, "equivalent" decode
/// written for diagnostics would be free to disagree with the real one, which
/// is precisely the disagreement a geometry dump exists to rule out.
pub(crate) fn decode_animation_layer_with_sizes(
    actions: &Arc<Actions>,
    palette_size: usize,
    texture_size: impl Fn(usize) -> Vector2<i32>,
    layer_index: usize,
    path_key: Option<String>,
    sprites: Option<Arc<Sprite>>,
) -> AnimationLayer {
    let mut animations: Vec<Animation> = Vec::new();

    for action in actions.actions.iter() {
        let mut action_frames: Vec<AnimationFrame> = Vec::new();

        for motion in action.motions.iter() {
            let mut motion_frames: Vec<AnimationFrame> = Vec::new();

            // Empty motions are real frames in an ACT. Keep a placeholder so
            // motion indices stay aligned with CActRes.
            let event: Option<ActionEvent> = if let Some(event_id) = motion.event_id
                && event_id != -1
                && let Some(event) = actions.events.get(event_id as usize).copied()
            {
                Some(event)
            } else {
                None
            };

            for sprite_clip in motion.sprite_clips.iter() {
                if sprite_clip.sprite_number == -1 {
                    continue;
                }

                let mut sprite_number = sprite_clip.sprite_number as usize;
                let sprite_type = match sprite_clip.sprite_type {
                    Some(value) => value as usize,
                    None => 0,
                };

                if sprite_type == 1 {
                    sprite_number += palette_size;
                }

                let texture_size = texture_size(sprite_number);
                let mut height = texture_size.y;
                let mut width = texture_size.x;

                let color = match sprite_clip.color {
                    Some(color) => {
                        let alpha = (((color >> 24) & 0xFF) as u8) as f32 / 255.0;
                        let blue = (((color >> 16) & 0xFF) as u8) as f32 / 255.0;
                        let green = (((color >> 8) & 0xFF) as u8) as f32 / 255.0;
                        let red = (((color) & 0xFF) as u8) as f32 / 255.0;

                        Color { red, green, blue, alpha }
                    }
                    None => Color {
                        red: 0.0,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 0.0,
                    },
                };

                let zoom = match sprite_clip.zoom {
                    Some(value) => (value, value).into(),
                    None => sprite_clip.zoom2.unwrap_or_else(|| (1.0, 1.0).into()),
                };
                if zoom != (1.0, 1.0).into() {
                    width = (width as f32 * zoom.x).ceil() as i32;
                    height = (height as f32 * zoom.y).ceil() as i32;
                }

                let angle = match sprite_clip.angle {
                    Some(value) => value as f32 / 360.0 * 2.0 * std::f32::consts::PI,
                    None => 0.0,
                };

                // Raw clip offset only — attach is applied at compose time.
                let offset = sprite_clip.position.map(|component| component);
                let mirror = sprite_clip.mirror_on != 0;

                let size = Vector2::new(width, height);
                let frame_part = AnimationFramePart {
                    animation_index: layer_index,
                    sprite_number,
                    size,
                    offset,
                    mirror,
                    angle,
                    color,
                    ..Default::default()
                };

                let frame = AnimationFrame {
                    event: None,
                    attach_point: None,
                    size,
                    top_left: Vector2::zero(),
                    offset,
                    frame_parts: vec![frame_part],
                    #[cfg(feature = "debug")]
                    horizontal_matrix: Matrix4::identity(),
                    #[cfg(feature = "debug")]
                    vertical_matrix: Matrix4::identity(),
                };

                motion_frames.push(frame);
            }

            let mut frame = match motion_frames.len() {
                1 => motion_frames[0].clone(),
                _ => merge_frame(&mut motion_frames),
            };
            // Only the body layer keeps ACT events.
            frame.event = if layer_index == 0 { event } else { None };
            // Native (roBrowser EntityRender.renderElement): if `animation.pos.length`
            // use pos[0]. Requiring `attach_point_count == 1` dropped parenting on
            // motions that author two points (or a zero count with a leftover
            // first point), which is facing-dependent on real hair/body ACTs.
            frame.attach_point = motion_attach_point(&motion.attach_points);

            action_frames.push(frame);
        }

        animations.push(Animation { frames: action_frames });
    }

    AnimationLayer {
        path_key,
        sprites,
        actions: Some(actions.clone()),
        animations,
    }
}

fn motion_attach_point(attach_points: &[ragnarok_formats::action::AttachPoint]) -> Option<Vector2<i32>> {
    attach_points.first().map(|point| point.position)
}

#[cfg(test)]
mod layer_sync_tests {
    use crate::world::animation::native_layer_motion_index;

    #[test]
    fn secondary_layer_falls_back_to_first_motion_when_shorter_than_body() {
        assert_eq!(native_layer_motion_index(3, 4), Some(3));
        assert_eq!(native_layer_motion_index(4, 4), Some(0));
        assert_eq!(native_layer_motion_index(8, 4), Some(0));
    }

    #[test]
    fn attach_uses_the_first_authored_point_even_when_count_is_not_one() {
        use cgmath::Vector2;
        use ragnarok_formats::action::AttachPoint;

        let points = [
            AttachPoint {
                ignored: 0,
                position: Vector2::new(4, -10),
                attribute: 0,
            },
            AttachPoint {
                ignored: 0,
                position: Vector2::new(99, 99),
                attribute: 0,
            },
        ];
        assert_eq!(super::motion_attach_point(&points), Some(Vector2::new(4, -10)));
        assert_eq!(super::motion_attach_point(&[]), None);
    }

    #[test]
    fn secondary_layer_does_not_run_its_own_cycle() {
        assert_eq!(native_layer_motion_index(3, 3), Some(0));
        assert_eq!(native_layer_motion_index(7, 3), Some(0));
    }

    #[test]
    fn empty_layer_contributes_nothing() {
        assert_eq!(native_layer_motion_index(0, 0), None);
        assert_eq!(native_layer_motion_index(5, 0), None);
    }
}
