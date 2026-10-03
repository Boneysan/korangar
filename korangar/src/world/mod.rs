mod action;
pub(crate) mod animation;
mod attack_element;
pub mod build_planner;
mod cameras;
pub mod crafting;
mod effect;
mod emote;
mod entity;
mod fade_state;
mod ground_item;
mod impact;
pub mod item_bonus;
mod library;
mod light;
mod map;
mod model;
mod monster_card;
mod navigation;
mod object;
mod particles;
mod pathing;
pub mod population;
mod skill_layout;
mod skill_recipe;
mod skill_unit_registry;
mod sound;
mod special_effect;
mod sprite_effect;
pub mod stat_formulas;
pub mod stat_preview;
pub mod stat_view;
mod unit_recipe;
mod video;
pub mod world_region;

use std::sync::Arc;

pub use self::action::*;
pub use self::animation::*;
#[allow(unused_imports)]
pub use self::attack_element::{ElementCue, elemental_effectiveness, identify_player_hit};
#[allow(unused_imports)]
pub use self::build_planner::*;
pub use self::cameras::*;
#[allow(unused_imports)]
pub use self::crafting::*;
pub use self::effect::*;
pub use self::emote::*;
pub use self::entity::*;
pub use self::fade_state::*;
pub use self::ground_item::*;
pub(crate) use self::impact::*;
#[allow(unused_imports)]
pub use self::item_bonus::*;
pub use self::library::*;
pub use self::light::*;
pub use self::map::*;
pub use self::model::*;
pub use self::monster_card::monster_facts;
pub use self::navigation::*;
pub use self::object::*;
pub use self::particles::*;
pub use self::pathing::*;
pub use self::population::*;
pub use self::skill_layout::*;
pub use self::skill_recipe::*;
pub use self::skill_unit_registry::*;
pub use self::sound::*;
pub use self::special_effect::*;
pub use self::sprite_effect::*;
pub use self::stat_preview::*;
#[allow(unused_imports)]
pub use self::stat_view::*;
pub use self::unit_recipe::*;
pub use self::video::*;
pub use self::world_region::*;
use crate::graphics::Texture;

pub struct ResourceSetBuffer<K> {
    visible: Vec<K>,
}

impl<K> Default for ResourceSetBuffer<K> {
    fn default() -> Self {
        Self { visible: Vec::new() }
    }
}

impl<K> ResourceSetBuffer<K> {
    pub(super) fn create_set(&mut self, initializer: impl FnOnce(&mut Vec<K>)) -> ResourceSet<'_, K> {
        self.visible.clear();

        initializer(&mut self.visible);

        ResourceSet { visible: &self.visible }
    }
}

#[derive(Default)]
pub struct ResourceSet<'a, K> {
    visible: &'a [K],
}

impl<K> ResourceSet<'_, K> {
    pub(super) fn iterate_visible(&self) -> std::slice::Iter<'_, K> {
        self.visible.iter()
    }
}

#[derive(Debug, Clone)]
pub struct ResourceMetadata {
    pub texture: Option<Arc<Texture>>,
    pub name: String,
}
