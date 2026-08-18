use std::sync::Arc;

use cgmath::{Matrix2, Point3, Rad, Vector2};
use korangar_interface::application::Position;
use wgpu::BlendFactor;

use crate::graphics::{Color, EffectInstruction, GroundDecalBlend, GroundDecalInstruction, ScreenPosition, ScreenSize, Texture};
use crate::world::Camera;

pub struct EffectRenderer {
    instructions: Vec<EffectInstruction>,
    ground_decals: Vec<GroundDecalInstruction>,
    window_size: ScreenSize,
}

/// Texture coordinates for a ground decal, for corners ordered
/// `[(-x,-z), (+x,-z), (-x,+z), (+x,+z)]`.
///
/// **v is flipped**, and that is the whole point of this constant. The player
/// camera sits at `DEFAULT_ANGLE` 180 degrees yaw and `CAMERA_PITCH` -55, so
/// its offset `(0, 0, d)` rotates to `(0, +0.819d, -0.574d)`: the camera is
/// above and at **-z**, looking toward +z. Screen-up on the ground plane is
/// therefore **+z**, and a picture's top belongs on the `+z` corners. The
/// identity mapping puts it on `-z` and draws every decal upside down.
///
/// Nothing caught this until 2026-08-17 because nothing asymmetric had been
/// drawn flat: Gospel's cross is vertically symmetric, Land Protector's circle
/// is radial, Fog Wall's puff is a blob. Evil Land's figure is not, and it
/// arrived on screen inverted. **Moonlit's hovering note had been upside down
/// since 2026-08-08.**
pub const GROUND_DECAL_TEXTURE_COORDINATES: [Vector2<f32>; 4] = [
    Vector2::new(0.0, 1.0),
    Vector2::new(1.0, 1.0),
    Vector2::new(0.0, 0.0),
    Vector2::new(1.0, 0.0),
];

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
    /// `[top_left, top_right, bottom_left, bottom_right]`; the world corners
    /// are kept as-is so the forward pass can project and depth-test them.
    ///
    /// `blend` must match the artwork's family — see [`GroundDecalBlend`]. Flat
    /// tints and magenta-keyed textures want `Alpha`; greyscale-on-black effect
    /// textures want `Additive` or their background draws as opaque black.
    ///
    /// [`render_effect_world_quad`]: Self::render_effect_world_quad
    pub fn render_ground_decal(
        &mut self,
        corners: [Point3<f32>; 4],
        texture: Arc<Texture>,
        texture_coordinates: [Vector2<f32>; 4],
        color: Color,
        blend: GroundDecalBlend,
    ) {
        self.ground_decals.push(GroundDecalInstruction {
            corners,
            texture_coordinates,
            color,
            texture,
            blend,
        });
    }
}

#[cfg(test)]
mod ground_decal_orientation_tests {
    use super::GROUND_DECAL_TEXTURE_COORDINATES;

    /// The decal UVs must put a picture's top on the **+z** corners.
    ///
    /// Derived, not chosen: `PlayerCamera` yaws 180 degrees with a -55 pitch,
    /// so its `(0, 0, d)` offset lands at `(0, +0.819d, -0.574d)` — the
    /// camera sits at -z looking toward +z, which makes +z the far side of
    /// the ground and so screen-up. The identity mapping draws every ground
    /// decal upside down, which is how Evil Land arrived on 2026-08-17, and
    /// how Moonlit's hovering note had been drawing since 2026-08-08
    /// without anyone noticing.
    #[test]
    fn ground_decal_uvs_put_the_picture_top_away_from_the_camera() {
        let [top_left, top_right, bottom_left, bottom_right] = GROUND_DECAL_TEXTURE_COORDINATES;

        // Corners are ordered [(-x,-z), (+x,-z), (-x,+z), (+x,+z)]. The two -z
        // corners are nearest the camera, so they carry the bottom of the
        // picture, v = 1.
        assert_eq!(top_left.y, 1.0, "the -x/-z corner is nearest the camera");
        assert_eq!(top_right.y, 1.0, "the +x/-z corner is nearest the camera");
        assert_eq!(bottom_left.y, 0.0, "the -x/+z corner is furthest away");
        assert_eq!(bottom_right.y, 0.0, "the +x/+z corner is furthest away");

        // u is *not* flipped: looking along +z with up +y, screen-right is +x,
        // so the picture's left edge belongs on -x.
        assert_eq!(top_left.x, 0.0);
        assert_eq!(bottom_left.x, 0.0);
        assert_eq!(top_right.x, 1.0);
        assert_eq!(bottom_right.x, 1.0);
    }
}
