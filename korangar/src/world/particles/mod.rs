use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Point3, Vector3};
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
use korangar_interface::application::Clip;
use ragnarok_packets::{EntityId, QuestColor, QuestEffectPacket};
use rand_aes::tls::rand_f32;

use crate::Map;
use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::{FontSize, ImageType, Scaling, TextureLoader};
use crate::renderer::{AlignHorizontal, GameInterfaceRenderer, SpriteRenderer};
use crate::world::Camera;

pub trait Particle {
    fn update(&mut self, delta_time: f32) -> bool;

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize);
}

fn random_velocity() -> f32 {
    rand_f32() * 40.0 - 20.0
}

pub struct DamageNumber {
    position: Point3<f32>,
    damage_amount: String,
    velocity_y: f32,
    velocity_x: f32,
    velocity_z: f32,
    timer: f32,
    is_critical: bool,
    font_scale: f32,
}

impl DamageNumber {
    pub fn new(position: Point3<f32>, damage_amount: String, is_critical: bool, font_scale: f32) -> Self {
        Self {
            position,
            damage_amount,
            velocity_y: 50.0,
            velocity_x: random_velocity(),
            velocity_z: random_velocity(),
            timer: 0.6,
            is_critical,
            font_scale,
        }
    }
}

impl Particle for DamageNumber {
    fn update(&mut self, delta_time: f32) -> bool {
        self.velocity_y -= 200.0 * delta_time;

        self.position.y += self.velocity_y * delta_time;
        self.position.x += self.velocity_x * delta_time;
        self.position.z += self.velocity_z * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        let color = match self.is_critical {
            true => Color::rgb_u8(255, 180, 0),
            false => Color::WHITE,
        };

        renderer.render_damage_text(&self.damage_amount, final_position, color, FontSize(16.0 * self.font_scale));
    }
}

pub struct Miss {
    position: Point3<f32>,
    timer: f32,
    font_scale: f32,
}

impl Miss {
    pub fn new(position: Point3<f32>, font_scale: f32) -> Self {
        Self {
            position,
            timer: 0.6,
            font_scale,
        }
    }
}

impl Particle for Miss {
    fn update(&mut self, delta_time: f32) -> bool {
        self.position.y += (self.timer - 0.1).max(0.0) * 70.0 * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };
        let alpha = (self.timer * 10.0).min(1.0);

        renderer.render_damage_text(
            "miss",
            final_position,
            Color::rgba(1.0, 0.0, 0.0, alpha),
            FontSize(20.0 * self.font_scale),
        );
    }
}

pub struct HealNumber {
    position: Point3<f32>,
    heal_amount: String,
    velocity_y: f32,
    timer: f32,
    font_scale: f32,
}

/// A single, expiring in-world visualization for the most recent party ping.
pub struct PartyPingMarker {
    position: Point3<f32>,
    label: String,
    color: Color,
    remaining: f32,
}

impl PartyPingMarker {
    pub fn new(position: Point3<f32>, kind: String, sender: String) -> Self {
        let color = match kind.as_str() {
            "assist" => Color::rgb_u8(240, 228, 66),
            "danger" => Color::rgb_u8(213, 94, 0),
            "retreat" => Color::rgb_u8(230, 159, 0),
            "ready" => Color::rgb_u8(0, 114, 178),
            "on-my-way" => Color::rgb_u8(86, 180, 233),
            _ => Color::rgb_u8(204, 121, 167),
        };
        Self {
            position: position + Vector3::new(0.0, 22.0, 0.0),
            label: format!("[{kind}] {sender}"),
            color,
            remaining: 15.0,
        }
    }

    fn update(&mut self, delta_time: f32) -> bool {
        self.remaining -= delta_time;
        self.remaining > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip = camera.view_projection_matrix() * self.position.to_homogeneous();
        if clip.w <= 0.0 {
            return;
        }
        let screen = camera.clip_to_screen_space(clip);
        if !(0.0..=1.0).contains(&screen.x) || !(0.0..=1.0).contains(&screen.y) {
            return;
        }
        let center = ScreenPosition {
            left: screen.x * window_size.width,
            top: screen.y * window_size.height,
        };
        renderer.render_rectangle(
            ScreenPosition {
                left: center.left - 5.0,
                top: center.top - 5.0,
            },
            ScreenSize::uniform(10.0),
            self.color,
        );
        renderer.render_text(
            &self.label,
            ScreenPosition {
                left: center.left,
                top: center.top - 24.0,
            },
            Color::WHITE,
            FontSize(12.0),
            AlignHorizontal::Center,
        );
    }
}

impl HealNumber {
    pub fn new(position: Point3<f32>, heal_amount: String, font_scale: f32) -> Self {
        Self {
            position,
            heal_amount,
            velocity_y: 50.0,
            timer: 1.0,
            font_scale,
        }
    }
}

impl Particle for HealNumber {
    fn update(&mut self, delta_time: f32) -> bool {
        self.velocity_y -= 50.0 * delta_time;

        self.position.y += self.velocity_y * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        renderer.render_damage_text(
            &self.heal_amount,
            final_position,
            Color::rgb_u8(30, 255, 30),
            FontSize(16.0 * self.font_scale),
        );
    }
}

pub struct QuestIcon {
    position: Point3<f32>,
    texture: Arc<Texture>,
    color: Color,
}

impl QuestIcon {
    pub fn new(texture_loader: &TextureLoader, map: &Map, quest_effect: QuestEffectPacket) -> Option<Self> {
        // TODO: Use the height of the entity as offset.
        let icon_offset = Vector3::new(0.0, 25.0, 0.0);
        let Some(entity_position) = map.get_world_position(quest_effect.position) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!("[{}] quest icon is out of map bounds", "error".red());
            return None;
        };

        let position = entity_position + icon_offset;
        let effect_id = quest_effect.effect as usize;
        let texture = match texture_loader.get_or_load(
            &format!("유저인터페이스\\minimap\\quest_{}_{}.bmp", effect_id, 1), /* 1 - 3 */
            ImageType::Color,
        ) {
            Ok(t) => t,
            Err(_) => {
                #[cfg(feature = "debug")]
                korangar_debug::logging::print_debug!(
                    "[{}] failed to load quest marker texture for effect {}",
                    "warning".yellow(),
                    effect_id
                );
                return None;
            }
        };
        let color = match quest_effect.color {
            QuestColor::Yellow => Color::rgb_u8(200, 200, 30),
            QuestColor::Orange => Color::rgb_u8(200, 100, 30),
            QuestColor::Green => Color::rgb_u8(30, 200, 30),
            QuestColor::Purple => Color::rgb_u8(200, 30, 200),
        };

        Some(Self { position, texture, color })
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize, scaling_factor: f32) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        renderer.render_sprite(
            self.texture.clone(),
            final_position - ScreenSize::uniform(15.0 * scaling_factor),
            ScreenSize::uniform(30.0 * scaling_factor),
            ScreenClip::unbound(),
            self.color,
            true,
            false,
        );
    }
}

#[derive(Default)]
pub struct ParticleHolder {
    particles: Vec<Box<dyn Particle + Send + Sync>>,
    quest_icons: HashMap<EntityId, QuestIcon>,
    party_ping: Option<PartyPingMarker>,
}

impl ParticleHolder {
    pub fn spawn_particle(&mut self, particle: Box<dyn Particle + Send + Sync>) {
        self.particles.push(particle);
    }

    pub fn set_party_ping_marker(&mut self, marker: Option<PartyPingMarker>) {
        self.party_ping = marker;
    }

    pub fn add_quest_icon(&mut self, texture_loader: &TextureLoader, map: &Map, quest_effect: QuestEffectPacket) {
        let entity_id = quest_effect.entity_id;

        if let Some(quest_icon) = QuestIcon::new(texture_loader, map, quest_effect) {
            self.quest_icons.insert(entity_id, quest_icon);
        }
    }

    pub fn remove_quest_icon(&mut self, entity_id: EntityId) {
        self.quest_icons.remove(&entity_id);
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.quest_icons.clear();
        self.party_ping = None;
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("update particles"))]
    pub fn update(&mut self, delta_time: f32) {
        self.particles.retain_mut(|particle| particle.update(delta_time));
        if self.party_ping.as_mut().is_some_and(|marker| !marker.update(delta_time)) {
            self.party_ping = None;
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render particles"))]
    pub fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize, scaling: Scaling) {
        self.particles
            .iter()
            .for_each(|particle| particle.render(renderer, camera, window_size));
        if let Some(marker) = &self.party_ping {
            marker.render(renderer, camera, window_size);
        }

        // Render quest icons for all active effects. We use the positions from the
        // QuestEffectPacket (not requiring the base entity to be present in the
        // current entities list). This matches official client behavior for guide
        // markers / instruction spots (e.g. "!" at teleport areas or quest points),
        // which can appear even for special entities (warps) or when the main sprite
        // isn't loaded. The black outline / visibility comes from the quest_*.bmp
        // assets themselves.
        self.quest_icons
            .values()
            .for_each(|quest_icon| quest_icon.render(renderer, camera, window_size, scaling.get_factor()));
    }
}

#[cfg(test)]
mod party_ping_tests {
    use cgmath::Point3;

    use super::{ParticleHolder, PartyPingMarker};

    #[test]
    fn in_world_party_ping_replaces_previous_marker_and_expires() {
        let mut holder = ParticleHolder::default();
        let position = Point3::new(1.0, 2.0, 3.0);
        holder.set_party_ping_marker(Some(PartyPingMarker::new(position, "danger".to_owned(), "Ada".to_owned())));
        holder.set_party_ping_marker(Some(PartyPingMarker::new(position, "assist".to_owned(), "Lin".to_owned())));
        assert_eq!(
            holder.party_ping.as_ref().map(|marker| marker.label.as_str()),
            Some("[assist] Lin")
        );

        holder.update(14.9);
        assert!(holder.party_ping.is_some());
        holder.update(0.2);
        assert!(holder.party_ping.is_none());
    }
}
