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
use crate::loaders::{ActionLoader, SpriteLoader};
use crate::world::animation::{compute_action_layouts, merge_frame};
use crate::world::{ActionEvent, Animation, AnimationData, AnimationFrame, AnimationFramePart, AnimationLayer, AnimationPair};
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
    let mut animations: Vec<Animation> = Vec::new();

    for action in animation_pair.actions.actions.iter() {
        let mut action_frames: Vec<AnimationFrame> = Vec::new();

        for motion in action.motions.iter() {
            let mut motion_frames: Vec<AnimationFrame> = Vec::new();

            // Empty motions are real frames in an ACT. Keep a placeholder so
            // motion indices stay aligned with CActRes.
            let event: Option<ActionEvent> = if let Some(event_id) = motion.event_id
                && event_id != -1
                && let Some(event) = animation_pair.actions.events.get(event_id as usize).copied()
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
                    sprite_number += animation_pair.sprites.palette_size;
                }

                let texture_size = animation_pair.sprites.textures[sprite_number].get_size();
                let mut height = texture_size.height;
                let mut width = texture_size.width;

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
                    width = (width as f32 * zoom.x).ceil() as u32;
                    height = (height as f32 * zoom.y).ceil() as u32;
                }

                let angle = match sprite_clip.angle {
                    Some(value) => value as f32 / 360.0 * 2.0 * std::f32::consts::PI,
                    None => 0.0,
                };

                // Raw clip offset only — attach is applied at compose time.
                let offset = sprite_clip.position.map(|component| component);
                let mirror = sprite_clip.mirror_on != 0;

                let size = Vector2::new(width as i32, height as i32);
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
            frame.attach_point = match motion.attach_point_count {
                Some(1) if !motion.attach_points.is_empty() => Some(motion.attach_points[0].position),
                _ => None,
            };

            action_frames.push(frame);
        }

        animations.push(Animation { frames: action_frames });
    }

    AnimationLayer {
        path_key,
        sprites: Some(animation_pair.sprites.clone()),
        actions: Some(animation_pair.actions.clone()),
        animations,
    }
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
