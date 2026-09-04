use std::cell::{Ref, RefCell};
use std::sync::Arc;

use cgmath::EuclideanSpace;
#[cfg(feature = "debug")]
use korangar_interface::application::Clip;
use korangar_interface::application::{RenderLayer, ShadowPadding as _};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{ClipId, Icon, WindowLayout};

use crate::graphics::{
    Color, CornerDiameter, InterfaceRectangleInstruction, ScreenClip, ScreenPosition, ScreenSize, ShadowPadding, Texture,
};
use crate::loaders::{FontLoader, FontSize, GlyphInstruction, ImageType, OverflowBehavior, Sprite, TextureLoader};
use crate::renderer::SpriteRenderer;
use crate::state::ClientState;
use crate::world::{Actions, AnimationData, AnimationFrame, AnimationFramePart, SpriteAnimationState};

/// Renders the interface provided by [`korangar_interface`].
pub struct InterfaceRenderer {
    instructions: RefCell<Vec<InterfaceRectangleInstruction>>,
    glyphs: RefCell<Vec<GlyphInstruction>>,
    font_loader: Arc<FontLoader>,
    filled_box_texture: Arc<Texture>,
    unfilled_box_texture: Arc<Texture>,
    arrow_left_texture: Arc<Texture>,
    arrow_right_texture: Arc<Texture>,
    expanded_arrow_texture: Arc<Texture>,
    collapsed_arrow_texture: Arc<Texture>,
    eye_open_texture: Arc<Texture>,
    eye_closed_texture: Arc<Texture>,
    trash_can_texture: Arc<Texture>,
    window_size: ScreenSize,
    interface_size: ScreenSize,
    high_quality_interface: bool,
    #[cfg(feature = "debug")]
    show_rectangle_instructions: bool,
    #[cfg(feature = "debug")]
    show_glyph_instructions: bool,
    #[cfg(feature = "debug")]
    show_sprite_instructions: bool,
    #[cfg(feature = "debug")]
    show_sdf_instructions: bool,
}

impl InterfaceRenderer {
    /// Create a new interface renderer.
    ///
    /// This include loading the textures icons for rendering components.
    pub fn new(
        window_size: ScreenSize,
        font_loader: Arc<FontLoader>,
        texture_loader: &TextureLoader,
        high_quality_interface: bool,
    ) -> Self {
        let instructions = RefCell::new(Vec::default());
        let glyphs = RefCell::new(Vec::default());

        let filled_box_texture = texture_loader.get_or_load("filled_box.png", ImageType::Sdf).unwrap();
        let unfilled_box_texture = texture_loader.get_or_load("unfilled_box.png", ImageType::Sdf).unwrap();
        let arrow_left_texture = texture_loader.get_or_load("arrow_left.png", ImageType::Sdf).unwrap();
        let arrow_right_texture = texture_loader.get_or_load("arrow_right.png", ImageType::Sdf).unwrap();
        let expanded_arrow_texture = texture_loader.get_or_load("expanded_arrow.png", ImageType::Sdf).unwrap();
        let collapsed_arrow_texture = texture_loader.get_or_load("collapsed_arrow.png", ImageType::Sdf).unwrap();
        let eye_open_texture = texture_loader.get_or_load("eye_open.png", ImageType::Sdf).unwrap();
        let eye_closed_texture = texture_loader.get_or_load("eye_closed.png", ImageType::Sdf).unwrap();
        let trash_can_texture = texture_loader.get_or_load("trash_can.png", ImageType::Sdf).unwrap();

        let interface_size = if high_quality_interface { window_size * 2.0 } else { window_size };

        #[cfg(feature = "debug")]
        let show_rectangle_instructions = false;
        #[cfg(feature = "debug")]
        let show_glyph_instructions = false;
        #[cfg(feature = "debug")]
        let show_sprite_instructions = false;
        #[cfg(feature = "debug")]
        let show_sdf_instructions = false;

        Self {
            instructions,
            glyphs,
            font_loader,
            filled_box_texture,
            unfilled_box_texture,
            arrow_left_texture,
            arrow_right_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            eye_open_texture,
            eye_closed_texture,
            trash_can_texture,
            window_size,
            interface_size,
            high_quality_interface,
            #[cfg(feature = "debug")]
            show_rectangle_instructions,
            #[cfg(feature = "debug")]
            show_glyph_instructions,
            #[cfg(feature = "debug")]
            show_sprite_instructions,
            #[cfg(feature = "debug")]
            show_sdf_instructions,
        }
    }

    #[cfg(feature = "debug")]
    pub fn update_render_options(&mut self, render_options: &crate::RenderOptions) {
        self.show_rectangle_instructions = render_options.show_rectangle_instructions;
        self.show_glyph_instructions = render_options.show_glyph_instructions;
        self.show_sprite_instructions = render_options.show_sprite_instructions;
        self.show_sdf_instructions = render_options.show_sdf_instructions;
    }

    /// Don't show rectangle instructions for the duration of the closure. This
    /// is only used to render other debug areas.
    #[cfg(feature = "debug")]
    pub fn with_rectangles_direct(&mut self, f: impl Fn(&Self)) {
        let previous_state = self.show_rectangle_instructions;
        self.show_rectangle_instructions = false;

        f(self);

        self.show_rectangle_instructions = previous_state;
    }

    /// Clear render instructions.
    pub fn clear(&self) {
        self.instructions.borrow_mut().clear();
    }

    /// Get render instructions.
    pub fn get_instructions(&self) -> Ref<'_, Vec<InterfaceRectangleInstruction>> {
        self.instructions.borrow()
    }

    /// Inform the renderer of a change in the high quality interface setting.
    pub fn update_high_quality_interface(&mut self, high_quality_interface: bool) {
        self.high_quality_interface = high_quality_interface;
        self.interface_size = if self.high_quality_interface {
            self.window_size * 2.0
        } else {
            self.window_size
        };
    }

    /// Inform the renderer of new window size.
    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.window_size = window_size;
        self.interface_size = if self.high_quality_interface {
            self.window_size * 2.0
        } else {
            self.window_size
        };
    }

    /// Get the bounds of a given text, respecting the loaded font.
    pub fn get_text_dimensions(
        &self,
        text: &str,
        color: Color,
        highlight_color: Color,
        mut font_size: FontSize,
        mut available_width: f32,
        overflow_behavior: OverflowBehavior,
    ) -> (ScreenSize, FontSize) {
        if self.high_quality_interface {
            // We need to adjust the font size, or else we would create glyphs for a font
            // size, that we don't use.
            font_size = font_size * 2.0;
            available_width *= 2.0;
        }

        let (mut size, mut font_size) =
            self.font_loader
                .get_text_dimensions(text, color, highlight_color, font_size, 1.0, available_width, overflow_behavior);

        if self.high_quality_interface {
            size = size / 2.0;
            font_size = FontSize(font_size.0 / 2.0);
        }

        (size, font_size)
    }

    /// Add instruction for rendering a rectangle.
    #[allow(clippy::too_many_arguments)]
    pub fn render_rectangle(
        &self,
        position: ScreenPosition,
        size: ScreenSize,
        mut screen_clip: ScreenClip,
        mut corner_diameter: CornerDiameter,
        color: Color,
        shadow_color: Color,
        mut shadow_padding: ShadowPadding,
    ) {
        // If the rectangle is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            #[cfg(feature = "debug")]
            if self.show_rectangle_instructions {
                let screen_position = position / self.window_size;
                let screen_size = size / self.window_size;

                self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(255, 0, 0, 90),
                    corner_diameter: CornerDiameter::default(),
                    shadow_color: Color::rgba_u8(255, 150, 0, 150),
                    shadow_padding: ShadowPadding::uniform(5.0),
                });
            }

            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
            corner_diameter = corner_diameter * 2.0;
            shadow_padding = shadow_padding.scaled(2.0);
        }

        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        #[cfg(feature = "debug")]
        if self.show_rectangle_instructions {
            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                screen_position,
                screen_size,
                screen_clip,
                color: Color::rgba_u8(170, 0, 170, 90),
                corner_diameter: CornerDiameter::default(),
                shadow_color: Color::rgba_u8(0, 255, 255, 150),
                shadow_padding: ShadowPadding::uniform(5.0),
            });

            return;
        }

        let corner_diameter = corner_diameter * 0.5;

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
            shadow_color,
            shadow_padding,
        });
    }

    /// Add instructions for rendering glyphs.
    #[allow(clippy::too_many_arguments)]
    pub fn render_text(
        &self,
        text: &str,
        mut text_position: ScreenPosition,
        mut available_width: f32,
        mut screen_clip: ScreenClip,
        color: Color,
        highlight_color: Color,
        mut font_size: FontSize,
    ) -> f32 {
        // TODO: Can't we scale after laying out the text? Would cut down on
        // multiplications.
        if self.high_quality_interface {
            text_position = text_position * 2.0;
            available_width *= 2.0;
            screen_clip = screen_clip * 2.0;
            font_size = font_size * 2.0;
        }

        let mut glyphs = self.glyphs.borrow_mut();

        let mut size = self.font_loader.layout_text(
            text,
            color,
            highlight_color,
            font_size,
            1.0,
            Some(available_width),
            Some(&mut glyphs),
        );

        let mut instructions = self.instructions.borrow_mut();

        glyphs.drain(..).for_each(
            |GlyphInstruction {
                 position,
                 texture_coordinate,
                 color,
             }| {
                // If the character is not even within the bounds of the clip, discard it early
                // saving GPU resources.
                //
                // TODO: For some reason the min.y is actually max.y and vice versa. Not sure
                // how this rendering code works but that's why the check is
                // using max.y and min.y inverted.
                if text_position.left + position.min.x > screen_clip.right
                    || text_position.top + position.max.y > screen_clip.bottom
                    || text_position.left + position.max.x < screen_clip.left
                    || text_position.top + position.min.y < screen_clip.top
                {
                    #[cfg(feature = "debug")]
                    if self.show_glyph_instructions {
                        let screen_position = ScreenPosition {
                            left: text_position.left + position.min.x,
                            top: text_position.top + position.min.y,
                        } / self.interface_size;

                        let screen_size = ScreenSize {
                            width: position.width(),
                            height: position.height(),
                        } / self.interface_size;

                        instructions.push(InterfaceRectangleInstruction::Solid {
                            screen_position,
                            screen_size,
                            screen_clip: ScreenClip::unbound(),
                            color: Color::rgba_u8(255, 0, 0, 150),
                            corner_diameter: CornerDiameter::default(),
                            shadow_color: Color::rgba_u8(255, 150, 0, 150),
                            shadow_padding: ShadowPadding::uniform(0.0),
                        });
                    }

                    return;
                }

                let screen_position = ScreenPosition {
                    left: text_position.left + position.min.x,
                    top: text_position.top + position.min.y,
                } / self.interface_size;

                let screen_size = ScreenSize {
                    width: position.width(),
                    height: position.height(),
                } / self.interface_size;

                #[cfg(feature = "debug")]
                if self.show_glyph_instructions {
                    instructions.push(InterfaceRectangleInstruction::Solid {
                        screen_position,
                        screen_size,
                        screen_clip: ScreenClip::unbound(),
                        color: Color::rgba_u8(180, 255, 0, 150),
                        corner_diameter: CornerDiameter::default(),
                        shadow_color: Color::rgba_u8(0, 255, 0, 150),
                        shadow_padding: ShadowPadding::uniform(0.0),
                    });

                    return;
                }

                let texture_position = texture_coordinate.min.to_vec();
                let texture_size = texture_coordinate.max - texture_coordinate.min;

                instructions.push(InterfaceRectangleInstruction::Text {
                    screen_position,
                    screen_size,
                    screen_clip,
                    texture_position,
                    texture_size,
                    color,
                });
            },
        );

        if self.high_quality_interface {
            size.y /= 2.0;
        }

        size.y
    }

    /// Render a checkbox icon using an SDF.
    pub fn render_checkbox(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, checked: bool) {
        let texture = match checked {
            true => self.filled_box_texture.clone(),
            false => self.unfilled_box_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render a left arrow icon using an SDF.
    pub fn render_arrow_left(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color) {
        self.render_sdf(self.arrow_left_texture.clone(), position, size, clip, color);
    }

    /// Render a right arrow icon using an SDF.
    pub fn render_arrow_right(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color) {
        self.render_sdf(self.arrow_right_texture.clone(), position, size, clip, color);
    }

    /// Render an expand arrow icon using an SDF.
    pub fn render_expand_arrow(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, expanded: bool) {
        let texture = match expanded {
            true => self.expanded_arrow_texture.clone(),
            false => self.collapsed_arrow_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render an eye icon using an SDF.
    pub fn render_eye(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, open: bool) {
        let texture = match open {
            true => self.eye_open_texture.clone(),
            false => self.eye_closed_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render a trash can icon using an SDF.
    pub fn render_trash_can(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color) {
        self.render_sdf(self.trash_can_texture.clone(), position, size, clip, color);
    }
}

impl SpriteRenderer for InterfaceRenderer {
    fn render_sprite(
        &self,
        texture: Arc<Texture>,
        position: ScreenPosition,
        size: ScreenSize,
        mut screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
        mirror: bool,
    ) {
        // If the sprite is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            #[cfg(feature = "debug")]
            if self.show_sprite_instructions {
                let screen_position = position / self.window_size;
                let screen_size = size / self.window_size;

                self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(255, 0, 0, 90),
                    corner_diameter: CornerDiameter::default(),
                    shadow_color: Color::rgba_u8(255, 150, 0, 150),
                    shadow_padding: ShadowPadding::uniform(5.0),
                });
            }

            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
        }

        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        #[cfg(feature = "debug")]
        if self.show_sprite_instructions {
            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                screen_position,
                screen_size,
                screen_clip: ScreenClip::unbound(),
                color: Color::rgba_u8(150, 0, 255, 90),
                corner_diameter: CornerDiameter::default(),
                shadow_color: Color::rgba_u8(255, 0, 255, 150),
                shadow_padding: ShadowPadding::uniform(5.0),
            });

            return;
        }

        let corner_diameter = CornerDiameter::default();

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sprite {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
            texture,
            smooth,
            mirror,
        });
    }

    fn render_sdf(&self, texture: Arc<Texture>, position: ScreenPosition, size: ScreenSize, mut screen_clip: ScreenClip, color: Color) {
        // If the SDF is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            #[cfg(feature = "debug")]
            if self.show_sdf_instructions {
                let screen_position = position / self.window_size;
                let screen_size = size / self.window_size;

                self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(255, 0, 0, 90),
                    corner_diameter: CornerDiameter::default(),
                    shadow_color: Color::rgba_u8(255, 150, 0, 150),
                    shadow_padding: ShadowPadding::uniform(5.0),
                });
            }

            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
        }

        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        #[cfg(feature = "debug")]
        if self.show_sdf_instructions {
            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                screen_position,
                screen_size,
                screen_clip: ScreenClip::unbound(),
                color: Color::rgba_u8(150, 255, 0, 90),
                corner_diameter: CornerDiameter::default(),
                shadow_color: Color::rgba_u8(255, 255, 0, 150),
                shadow_padding: ShadowPadding::uniform(5.0),
            });

            return;
        }

        let corner_diameter = CornerDiameter::default();

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sdf {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
            texture,
        });
    }
}

/// An instruction to render a texture.
///
/// These are not used outside this module but are exposed through
/// [`CustomInstruction`]. Thus we explicitly allow a private interface.
#[allow(private_interfaces)]
struct TextureInstruction {
    texture: Arc<Texture>,
    clip_id: ClipId,
    area: Area,
    color: Color,
    smooth: bool,
}

/// An instruction to render a sprite.
///
/// These are not used outside this module but are exposed through
/// [`CustomInstruction`]. Thus we explicitly allow a private interface.
#[allow(private_interfaces)]
struct SpriteInstruction<'a> {
    /// Which of the eight ACT facings to draw. The world renderer derives this
    /// from the camera; an interface sprite has no camera, so the caller says.
    direction: usize,
    actions: &'a Actions,
    sprite: &'a Sprite,
    animation_state: &'a SpriteAnimationState,
    clip_id: ClipId,
    area: Area,
    color: Color,
    scaling: f32,
}

/// A complete layered ACT frame rendered as one interface object.
///
/// Unlike [`SpriteInstruction`], this goes through [`AnimationData`]'s normal
/// body/head composition before emitting rectangles. That keeps attachment,
/// layer ordering, mirroring, and scaling identical for every facing.
#[allow(private_interfaces)]
struct AnimationInstruction<'a> {
    animation_data: &'a AnimationData,
    direction: usize,
    clip_id: ClipId,
    area: Area,
    color: Color,
    maximum_scaling: f32,
}

fn animation_scaling(area: Area, frame: &AnimationFrame, maximum_scaling: f32) -> f32 {
    if frame.size.x <= 0 || frame.size.y <= 0 {
        return 0.0;
    }

    maximum_scaling
        .min(area.width / frame.size.x as f32)
        .min(area.height / frame.size.y as f32)
}

/// Convert the same pixel geometry used by `finalize_frame_layout` back into
/// an axis-aligned interface rectangle. Standing player frames do not rotate
/// their clips, so position, size, and the ACT mirror bit completely describe
/// the preview draw.
fn animation_part_area(area: Area, frame: &AnimationFrame, part: &AnimationFramePart, scaling: f32) -> Area {
    let frame_width = frame.size.x as f32 * scaling;
    let frame_height = frame.size.y as f32 * scaling;
    let frame_screen_left = area.left + (area.width - frame_width) / 2.0;
    let frame_screen_top = area.top + (area.height - frame_height) / 2.0;

    let frame_left = -(frame.size.x as f32) / 2.0;
    let frame_top = -(frame.size.y as f32) + 0.5;
    let part_left = (frame.offset.x + part.offset.x - (part.size.x - 1) / 2) as f32 - 0.5;
    let part_top = (frame.offset.y + part.offset.y - (part.size.y - 1) / 2) as f32 - 0.5;

    Area {
        left: frame_screen_left + (part_left - frame_left) * scaling,
        top: frame_screen_top + (part_top - frame_top) * scaling,
        width: part.size.x as f32 * scaling,
        height: part.size.y as f32 * scaling,
    }
}

/// A custom layout instruction.
///
/// Only pub to make the compiler happy, its not used outside of this module.
#[allow(private_interfaces)]
pub enum CustomInstruction<'a> {
    /// An instruction to render a texture.
    Texture(TextureInstruction),
    /// An instruction to render a sprite.
    Sprite(SpriteInstruction<'a>),
    /// An instruction to render a layered sprite composition.
    Animation(AnimationInstruction<'a>),
}

impl RenderLayer<ClientState> for InterfaceRenderer {
    type CustomIcon = ();
    type CustomInstruction<'a> = CustomInstruction<'a>;

    fn render_rectangle(
        &self,
        position: ScreenPosition,
        size: ScreenSize,
        clip: ScreenClip,
        corner_diameter: CornerDiameter,
        color: Color,
        shadow_color: Color,
        shadow_padding: ShadowPadding,
    ) {
        self.render_rectangle(position, size, clip, corner_diameter, color, shadow_color, shadow_padding);
    }

    fn render_text(
        &self,
        text: &str,
        position: ScreenPosition,
        available_width: f32,
        clip: ScreenClip,
        color: Color,
        highlight_color: Color,
        font_size: FontSize,
    ) {
        self.render_text(text, position, available_width, clip, color, highlight_color, font_size);
    }

    fn render_icon(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, icon: Icon<ClientState>, color: Color) {
        match icon {
            Icon::ArrowLeft => self.render_arrow_left(position, size, clip, color),
            Icon::ArrowRight => self.render_arrow_right(position, size, clip, color),
            Icon::ExpandArrow { expanded } => self.render_expand_arrow(position, size, clip, color, expanded),
            Icon::Checkbox { checked } => self.render_checkbox(position, size, clip, color, checked),
            Icon::Eye { open } => self.render_eye(position, size, clip, color, open),
            Icon::TrashCan => self.render_trash_can(position, size, clip, color),
            Icon::Custom { .. } => {}
        }
    }

    fn render_custom(&self, instruction: Self::CustomInstruction<'_>, clips: &[ScreenClip]) {
        match instruction {
            CustomInstruction::Sprite(SpriteInstruction {
                direction,
                actions,
                sprite,
                animation_state,
                clip_id,
                area,
                color,
                scaling,
            }) => {
                let position = ScreenPosition {
                    left: area.left + area.width / 2.0,
                    top: area.top + area.height / 2.0,
                };
                let screen_clip = clips[clip_id.as_index()];

                actions.render_sprite(self, sprite, animation_state, position, direction, screen_clip, color, scaling);
            }
            CustomInstruction::Animation(AnimationInstruction {
                animation_data,
                direction,
                clip_id,
                area,
                color,
                maximum_scaling,
            }) => {
                let frame = animation_data.compose_idle_frame(direction);
                let scaling = animation_scaling(area, &frame, maximum_scaling);
                let screen_clip = clips[clip_id.as_index()];

                for part in frame.frame_parts.iter() {
                    let Some(texture) = animation_data
                        .layers
                        .get(part.animation_index)
                        .and_then(|layer| layer.sprites.as_ref())
                        .and_then(|sprite| sprite.textures.get(part.sprite_number))
                    else {
                        continue;
                    };

                    let part_area = animation_part_area(area, &frame, part, scaling);
                    self.render_sprite(
                        texture.clone(),
                        ScreenPosition {
                            left: part_area.left,
                            top: part_area.top,
                        },
                        ScreenSize {
                            width: part_area.width,
                            height: part_area.height,
                        },
                        screen_clip,
                        part.color * color,
                        false,
                        part.mirror,
                    );
                }
            }
            CustomInstruction::Texture(TextureInstruction {
                texture,
                clip_id,
                area,
                color,
                smooth,
            }) => {
                let position = ScreenPosition {
                    left: area.left,
                    top: area.top,
                };
                let size = ScreenSize {
                    width: area.width,
                    height: area.height,
                };
                let screen_clip = clips[clip_id.as_index()];

                self.render_sprite(texture, position, size, screen_clip, color, smooth, false);
            }
        }
    }
}

/// Extension trait to make adding custom instructions to the [`Layout`]
/// seamless by mirroring its API.
pub trait LayoutExt<'a> {
    /// Add an instruction to render a texture.
    fn add_texture(&mut self, area: Area, texture: Arc<Texture>, color: Color, smooth: bool);

    /// Add an instruction to render a sprite.
    #[allow(clippy::too_many_arguments)]
    fn add_sprite(
        &mut self,
        area: Area,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        direction: usize,
        color: Color,
        scale: f32,
    );

    /// Add a complete layered animation in a fixed idle pose.
    fn add_animation(&mut self, area: Area, animation_data: &'a AnimationData, direction: usize, color: Color, maximum_scale: f32);
}

impl<'a> LayoutExt<'a> for WindowLayout<'a, ClientState> {
    fn add_texture(&mut self, area: Area, texture: Arc<Texture>, color: Color, smooth: bool) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);

        self.add_custom_instruction(CustomInstruction::Texture(TextureInstruction {
            texture,
            clip_id,
            area,
            color,
            smooth,
        }));
    }

    fn add_sprite(
        &mut self,
        area: Area,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        direction: usize,
        color: Color,
        scale: f32,
    ) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);
        let scaling = scale * self.get_interface_scaling();

        self.add_custom_instruction(CustomInstruction::Sprite(SpriteInstruction {
            direction,
            actions,
            sprite,
            animation_state,
            clip_id,
            area,
            color,
            scaling,
        }));
    }

    fn add_animation(&mut self, area: Area, animation_data: &'a AnimationData, direction: usize, color: Color, maximum_scale: f32) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);
        let maximum_scaling = maximum_scale * self.get_interface_scaling();

        self.add_custom_instruction(CustomInstruction::Animation(AnimationInstruction {
            animation_data,
            direction,
            clip_id,
            area,
            color,
            maximum_scaling,
        }));
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Matrix4, Vector2, Zero};
    use korangar_interface::layout::area::Area;

    use super::{animation_part_area, animation_scaling};
    use crate::graphics::Color;
    use crate::world::{AnimationFrame, AnimationFramePart};

    fn part(offset: Vector2<i32>) -> AnimationFramePart {
        AnimationFramePart {
            animation_index: 0,
            sprite_number: 0,
            offset,
            size: Vector2::new(21, 31),
            mirror: false,
            angle: 0.0,
            color: Color::WHITE,
            affine_matrix: Matrix4::from_scale(1.0),
        }
    }

    fn frame() -> AnimationFrame {
        AnimationFrame {
            event: None,
            attach_point: None,
            offset: Vector2::new(0, -20),
            top_left: Vector2::zero(),
            size: Vector2::new(81, 101),
            frame_parts: Vec::new(),
            #[cfg(feature = "debug")]
            horizontal_matrix: Matrix4::from_scale(1.0),
            #[cfg(feature = "debug")]
            vertical_matrix: Matrix4::from_scale(1.0),
        }
    }

    #[test]
    fn composed_parts_scale_their_offsets_together() {
        let area = Area {
            left: 10.0,
            top: 20.0,
            width: 300.0,
            height: 300.0,
        };
        let frame = frame();
        let body = animation_part_area(area, &frame, &part(Vector2::new(0, 0)), 2.0);
        let head = animation_part_area(area, &frame, &part(Vector2::new(4, -18)), 2.0);

        assert_eq!(head.left - body.left, 8.0);
        assert_eq!(head.top - body.top, -36.0);
    }

    #[test]
    fn composed_animation_shrinks_to_fit_the_preview_area() {
        let area = Area {
            left: 0.0,
            top: 0.0,
            width: 120.0,
            height: 150.0,
        };

        assert_eq!(animation_scaling(area, &frame(), 2.0), 120.0 / 81.0);
    }
}
