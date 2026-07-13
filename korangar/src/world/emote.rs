use std::sync::Arc;

use ragnarok_packets::{ClientTick, EntityId};

use crate::graphics::EntityInstruction;
use crate::world::{AnimationData, Camera, Entity};

/// Sprite file (relative to `data\sprite\`, no extension) holding all emote
/// animations. One ACT action per emote, indexed by the wire emote ID.
pub const EMOTE_SPRITE_FILE: &str = "이팩트\\emotion";

/// Sentinel used to route the async load of the shared emote animation data
/// back to [`EmoteBubbles`] instead of an entity.
pub const EMOTE_ANIMATION_ENTITY_ID: EntityId = EntityId(u32::MAX);

/// Extra display time after the animation finishes playing.
const LINGER_MS: u32 = 400;
/// Lifetime bound used while the shared animation data is still loading.
const FALLBACK_LIFETIME_MS: u32 = 3000;

/// Diagnostics for the emote pipeline, enabled by setting the
/// `KORANGAR_EMOTE_DEBUG` environment variable.
pub fn emote_debug_enabled() -> bool {
    std::env::var_os("KORANGAR_EMOTE_DEBUG").is_some()
}

struct EmoteBubble {
    entity_id: EntityId,
    action_index: usize,
    start_time: ClientTick,
}

/// Emote balloons playing above entity heads, spawned from `DisplayEmotion`
/// network events. The emotion sprite sheet is shared and loaded once.
#[derive(Default)]
pub struct EmoteBubbles {
    animation_data: Option<Arc<AnimationData>>,
    bubbles: Vec<EmoteBubble>,
}

impl EmoteBubbles {
    pub fn has_animation_data(&self) -> bool {
        self.animation_data.is_some()
    }

    pub fn set_animation_data(&mut self, animation_data: Arc<AnimationData>) {
        if emote_debug_enabled() {
            eprintln!(
                "[emote] animation data ready: {} actions, {} delays",
                animation_data.animations.len(),
                animation_data.delays.len()
            );
        }
        self.animation_data = Some(animation_data);
    }

    pub fn show(&mut self, entity_id: EntityId, emotion: u8, client_tick: ClientTick) {
        // A new emote replaces the entity's current one, like the original client.
        self.bubbles.retain(|bubble| bubble.entity_id != entity_id);
        self.bubbles.push(EmoteBubble {
            entity_id,
            action_index: emotion as usize,
            start_time: client_tick,
        });
    }

    pub fn clear(&mut self) {
        self.bubbles.clear();
    }

    pub fn update(&mut self, client_tick: ClientTick) {
        let animation_data = &self.animation_data;
        self.bubbles.retain(|bubble| {
            let age = client_tick.0.wrapping_sub(bubble.start_time.0);
            let lifetime = animation_data
                .as_ref()
                .map(|data| data.action_duration_ms(bubble.action_index) + LINGER_MS)
                .unwrap_or(FALLBACK_LIFETIME_MS);
            age < lifetime
        });
    }

    pub fn render(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        entities: &[Entity],
        camera: &dyn Camera,
        client_tick: ClientTick,
    ) {
        let Some(animation_data) = self.animation_data.as_ref() else {
            if emote_debug_enabled() && !self.bubbles.is_empty() {
                eprintln!("[emote] {} bubble(s) pending but animation data not loaded yet", self.bubbles.len());
            }
            return;
        };

        for bubble in &self.bubbles {
            let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == bubble.entity_id) else {
                continue;
            };

            let time = client_tick.0.wrapping_sub(bubble.start_time.0);
            let rendered = animation_data.render_action_frame(
                instructions,
                camera,
                bubble.entity_id,
                entity.get_position(),
                bubble.action_index,
                time,
            );

            if emote_debug_enabled() && time < 50 {
                let position = entity.get_position();
                eprintln!(
                    "[emote] render action={} rendered={rendered} entity={} position=({:.1},{:.1},{:.1})",
                    bubble.action_index, bubble.entity_id.0, position.x, position.y, position.z
                );
            }
        }
    }
}
