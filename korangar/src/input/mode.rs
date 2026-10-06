use std::sync::Arc;

use korangar_interface::MouseMode;
use korangar_networking::InventoryItem;
use ragnarok_packets::TilePosition;

use crate::graphics::Texture;
use crate::interface::resource::{ItemSource, SkillSource};
use crate::loaders::Sprite;
use crate::state::ClientState;
use crate::state::skills::LearnableSkill;
use crate::world::{Actions, ResourceMetadata, SpriteAnimationState};

#[derive(Debug, Clone)]
pub enum MouseInputMode {
    RotateCamera,
    Walk {
        destination: TilePosition,
    },
    MoveItem {
        source: ItemSource,
        item: InventoryItem<ResourceMetadata>,
    },
    MoveSkill {
        source: SkillSource,
        skill: LearnableSkill,
    },
}

impl From<MouseInputMode> for MouseMode<ClientState> {
    fn from(mode: MouseInputMode) -> Self {
        MouseMode::Custom { mode }
    }
}

pub enum Grabbed {
    Texture(Arc<Texture>),
    Action(Arc<Sprite>, Arc<Actions>, SpriteAnimationState),
}

pub trait MouseModeExt {
    fn is_rotating_camera(&self) -> bool;

    fn walk_destination(&self) -> Option<TilePosition>;

    fn is_grabbing(&self) -> bool;

    /// A window title bar or resize handle is being dragged. The cursor runs
    /// ahead of the window edge while it does, so it must not count as a
    /// world click.
    fn is_dragging_window(&self) -> bool;

    fn grabbed(&self) -> Option<Grabbed>;
}

impl MouseModeExt for MouseMode<ClientState> {
    fn is_rotating_camera(&self) -> bool {
        matches!(self, MouseMode::Custom {
            mode: MouseInputMode::RotateCamera
        })
    }

    fn walk_destination(&self) -> Option<TilePosition> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::Walk { destination },
            } => Some(*destination),
            _ => None,
        }
    }

    fn is_grabbing(&self) -> bool {
        matches!(
            self,
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { .. },
            } | MouseMode::Custom {
                mode: MouseInputMode::MoveSkill { .. },
            }
        )
    }

    fn is_dragging_window(&self) -> bool {
        matches!(self, MouseMode::MovingWindow { .. } | MouseMode::ResizingWindow { .. })
    }

    fn grabbed(&self) -> Option<Grabbed> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { item, .. },
            } => item.metadata.texture.as_ref().map(|texture| Grabbed::Texture(texture.clone())),
            MouseMode::Custom {
                mode: MouseInputMode::MoveSkill { skill, .. },
            } => Some(Grabbed::Action(
                skill.sprite.clone()?,
                skill.actions.clone()?,
                skill.animation_state.clone(),
            )),
            _ => None,
        }
    }
}
