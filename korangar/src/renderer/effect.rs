use std::sync::Arc;

use cgmath::{Matrix2, Point3, Rad, Vector2};
use korangar_interface::application::Position;
use wgpu::BlendFactor;

use crate::graphics::{Color, EffectInstruction, GroundDecalInstruction, ScreenPosition, ScreenSize, Texture};
use crate::world::Camera;

pub struct EffectRenderer {
    instructions: Vec<EffectInstruction>,
    ground_decals: Vec<GroundDecalInstruction>,
    window_size: ScreenSize,
}

impl EffectRenderer {
    pub fn new(window_size: ScreenSize) -> Self {
        Self {
            instructions: Vec::default(),
            ground_decals: Vec::default(),
            window_size,
        }
    }

    pub fn clear(&mut self) {
        self.instructions.clear();
        self.ground_decals.clear();
    }

    pub fn get_instructions(&self) -> &[EffectInstruction] {
        self.instructions.as_ref()
    }

    pub fn get_ground_decals(&self) -> &[GroundDecalInstruction] {
        self.ground_decals.as_ref()
    }

    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.window_size = window_size;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_effect(
        &mut self,
        camera: &dyn Camera,
        position: Point3<f32>,
        texture: Arc<Texture>,
        corner_screen_position: [Vector2<f32>; 4],
        texture_coordinates: [Vector2<f32>; 4],
        offset: Vector2<f32>,
        angle: Rad<f32>,
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
    ) {
        const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

        let clip_space_position = camera.view_projection_matrix() * position.to_homogeneous();
        let screen_space_position = camera.clip_to_screen_space(clip_space_position);

        let half_screen = Vector2::new(self.window_size.width / 2.0, self.window_size.height / 2.0);
        let rotation_matrix = Matrix2::from_angle(angle);

        let corner_screen_position =
            corner_screen_position.map(|position| (rotation_matrix * position) + offset - EFFECT_ORIGIN - half_screen);

        let clip_space_positions = corner_screen_position.map(|position| {
            let normalized_screen_position = Vector2::new(
                (position.x / half_screen.x) * 0.5 + 0.5 + screen_space_position.x,
                (position.y / half_screen.y) * 0.5 + 0.5 + screen_space_position.y,
            );
            let clip_space_position = camera.screen_to_clip_space(normalized_screen_position);
            ScreenPosition::new(clip_space_position.x, clip_space_position.y)
        });

        self.instructions.push(EffectInstruction {
            top_left: clip_space_positions[0],
            bottom_left: clip_space_positions[2],
            top_right: clip_space_positions[1],
            bottom_right: clip_space_positions[3],
            texture_top_left: texture_coordinates[2],
            texture_bottom_left: texture_coordinates[3],
            texture_top_right: texture_coordinates[1],
            texture_bottom_right: texture_coordinates[0],
            color,
            source_blend_factor,
            destination_blend_factor,
            texture,
        });
    }

    /// Draws a quad whose corners are world-space positions, unlike
    /// [`render_effect`](Self::render_effect), which offsets screen-space
    /// corners around a single projected point. Used for geometry that has
    /// real extent in the world, like the warp portal vortex. Corners and
    /// texture coordinates are ordered `[top_left, top_right, bottom_left,
    /// bottom_right]`.
    #[allow(clippy::too_many_arguments)]
    pub fn render_effect_world_quad(
        &mut self,
        camera: &dyn Camera,
        corners: [Point3<f32>; 4],
        texture: Arc<Texture>,
        texture_coordinates: [Vector2<f32>; 4],
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
    ) {
        let view_projection = camera.view_projection_matrix();
        let mut clip_space_positions = [ScreenPosition::default(); 4];

        for (index, corner) in corners.iter().enumerate() {
            let position = view_projection * corner.to_homogeneous();

            // Skip quads that reach behind the near plane instead of letting
            // the perspective division mirror them across the screen.
            if position.w <= 0.1 {
                return;
            }

            clip_space_positions[index] = ScreenPosition::new(position.x / position.w, position.y / position.w);
        }

        self.instructions.push(EffectInstruction {
            top_left: clip_space_positions[0],
            top_right: clip_space_positions[1],
            bottom_left: clip_space_positions[2],
            bottom_right: clip_space_positions[3],
            texture_top_left: texture_coordinates[0],
            texture_top_right: texture_coordinates[1],
            texture_bottom_left: texture_coordinates[2],
            texture_bottom_right: texture_coordinates[3],
            color,
            source_blend_factor,
            destination_blend_factor,
            texture,
        });
    }

    /// Queues a flat, **depth-tested** ground quad, drawn in the forward pass
    /// rather than composited on top like [`render_effect_world_quad`]. Use for
    /// ground-parallel tiles (Land Protector) where an entity standing on the
    /// tile must occlude it. Corners and texture coordinates are ordered
    /// `[top_left, top_right, bottom_left, bottom_right]`; the world corners are
    /// kept as-is so the forward pass can project and depth-test them.
    ///
    /// [`render_effect_world_quad`]: Self::render_effect_world_quad
    pub fn render_ground_decal(
        &mut self,
        corners: [Point3<f32>; 4],
        texture: Arc<Texture>,
        texture_coordinates: [Vector2<f32>; 4],
        color: Color,
    ) {
        self.ground_decals.push(GroundDecalInstruction {
            corners,
            texture_coordinates,
            color,
            texture,
        });
    }
}
