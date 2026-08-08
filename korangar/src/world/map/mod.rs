mod lighting;

#[cfg(feature = "debug")]
use std::collections::HashSet;
use std::mem::size_of;
use std::sync::{Arc, Mutex, RwLock};

use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector2, Vector3};
use korangar_audio::AudioEngine;
use korangar_collision::{AABB, Frustum, KDTree, Sphere};
use korangar_container::{Cacheable, SimpleKey, SimpleSlab, create_simple_key};
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
#[cfg(feature = "debug")]
use option_ext::OptionExt;
#[cfg(feature = "debug")]
use ragnarok_formats::map::EffectSource;
#[cfg(feature = "debug")]
use ragnarok_formats::map::MapData;
use hashbrown::HashMap;
use ragnarok_formats::map::{LightSource, SoundSource, Tile, TileFlags};
use ragnarok_formats::transform::Transform;
use ragnarok_packets::{ClientTick, EntityId, TilePosition};
use rust_state::RustState;
use wgpu::Queue;

pub use self::lighting::Lighting;
use super::{
    Camera, Entity, GroundItem, Model, Object, PointLightId, PointLightManager, ResourceSet, ResourceSetBuffer, SubMesh, Video,
};
#[cfg(feature = "debug")]
use super::{LightSourceExt, PointLightSet};
#[cfg(feature = "debug")]
use crate::graphics::{
    DebugAabbInstruction, DebugCircleInstruction, DebugRectangleInstruction, ModelBatch, RenderOptions, ScreenPosition, ScreenSize,
};
use crate::graphics::{EntityInstruction, IndicatorInstruction, ModelInstruction, Texture, TextureSet, WaterInstruction, WaterVertex};
use crate::loaders::GAT_TILE_SIZE;
use crate::renderer::EffectRenderer;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
use crate::world::pathing::Traversable;
use crate::{Buffer, Color, GameFileLoader, ModelVertex, TileVertex};

create_simple_key!(ObjectKey, "Key to an object inside the map");
create_simple_key!(LightSourceKey, "Key to an light source inside the map");

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MarkerIdentifier {
    Object(u32),
    LightSource(u32),
    SoundSource(u32),
    EffectSource(u32),
    Particle(u16, u16),
    Entity(u32),
    Shadow(u32),
}

#[cfg(feature = "debug")]
impl MarkerIdentifier {
    pub const SIZE: f32 = 1.5;
}

pub struct WaterPlane {
    water_opacity: f32,
    wave_height: f32,
    wave_speed: Deg<f32>,
    wave_pitch: Deg<f32>,
    texture_cycling_interval: u32,
    texture_repeat: f32,
    water_textures: Vec<Arc<Texture>>,
    vertex_buffer: Arc<Buffer<WaterVertex>>,
    index_buffer: Arc<Buffer<u32>>,
}

impl WaterPlane {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        water_opacity: f32,
        wave_height: f32,
        wave_speed: Deg<f32>,
        wave_pitch: Deg<f32>,
        texture_cycling_interval: u32,
        texture_repeat: f32,
        water_textures: Vec<Arc<Texture>>,
        vertex_buffer: Arc<Buffer<WaterVertex>>,
        index_buffer: Arc<Buffer<u32>>,
    ) -> Self {
        Self {
            water_opacity,
            wave_height,
            wave_speed,
            wave_pitch,
            texture_cycling_interval,
            texture_repeat,
            water_textures,
            vertex_buffer,
            index_buffer,
        }
    }
}

#[derive(RustState)]
pub struct Map {
    width: u16,
    height: u16,
    level_bound: AABB,
    lighting: Lighting,
    water_plane: Option<WaterPlane>,
    tiles: Vec<Tile>,
    /// Cells the *server* has re-typed at runtime (`ZC_UPDATE_MAPINFO`), keyed
    /// by tile index. Ice Wall is the common case: it makes its cells
    /// impassable while it stands and reverts them when it expires.
    ///
    /// Kept beside the tiles rather than written into them because `Map` lives
    /// behind an `Arc` in a `SimpleCache` and is **reused on a later visit** —
    /// baking a wall into the tile data would leave it there forever. For the
    /// same reason [`Self::clear_dynamic_cells`] must run when a map is
    /// (re-)entered.
    dynamic_cells: RwLock<HashMap<usize, TileFlags>>,
    sub_meshes: Vec<SubMesh>,
    vertex_buffer: Arc<Buffer<ModelVertex>>,
    index_buffer: Arc<Buffer<u32>>,
    texture_set: Arc<TextureSet>,
    objects: SimpleSlab<ObjectKey, Object>,
    /// Models loaded into this map's shared geometry buffer but not placed by
    /// the map itself — spawned at runtime instead (Hunter traps). Keyed by the
    /// same `data\model\`-relative path used to load them.
    prop_models: HashMap<&'static str, Arc<Model>>,
    light_sources: SimpleSlab<LightSourceKey, LightSource>,
    sound_sources: Vec<SoundSource>,
    #[cfg(feature = "debug")]
    effect_sources: Vec<EffectSource>,
    tile_picker_vertex_buffer: Buffer<TileVertex>,
    tile_picker_index_buffer: Buffer<u32>,
    #[cfg(feature = "debug")]
    tile_vertex_buffer: Arc<Buffer<ModelVertex>>,
    #[cfg(feature = "debug")]
    tile_index_buffer: Arc<Buffer<u32>>,
    #[cfg(feature = "debug")]
    tile_submeshes: Vec<SubMesh>,
    object_kdtree: KDTree<ObjectKey, AABB>,
    light_source_kdtree: KDTree<LightSourceKey, Sphere>,
    background_music_track_name: Option<String>,
    videos: Mutex<Vec<Video>>,
    #[cfg(feature = "debug")]
    map_data: MapData,
}

impl Map {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        width: u16,
        height: u16,
        level_bound: AABB,
        lighting: Lighting,
        water_plane: Option<WaterPlane>,
        tiles: Vec<Tile>,
        sub_meshes: Vec<SubMesh>,
        vertex_buffer: Arc<Buffer<ModelVertex>>,
        index_buffer: Arc<Buffer<u32>>,
        texture_set: Arc<TextureSet>,
        objects: SimpleSlab<ObjectKey, Object>,
        prop_models: HashMap<&'static str, Arc<Model>>,
        light_sources: SimpleSlab<LightSourceKey, LightSource>,
        sound_sources: Vec<SoundSource>,
        #[cfg(feature = "debug")] effect_sources: Vec<EffectSource>,
        tile_picker_vertex_buffer: Buffer<TileVertex>,
        tile_picker_index_buffer: Buffer<u32>,
        #[cfg(feature = "debug")] tile_vertex_buffer: Arc<Buffer<ModelVertex>>,
        #[cfg(feature = "debug")] tile_index_buffer: Arc<Buffer<u32>>,
        #[cfg(feature = "debug")] tile_submeshes: Vec<SubMesh>,
        object_kdtree: KDTree<ObjectKey, AABB>,
        light_source_kdtree: KDTree<LightSourceKey, Sphere>,
        background_music_track_name: Option<String>,
        videos: Mutex<Vec<Video>>,
        #[cfg(feature = "debug")] map_data: MapData,
    ) -> Self {
        Self {
            width,
            height,
            level_bound,
            lighting,
            water_plane,
            tiles,
            dynamic_cells: RwLock::new(HashMap::new()),
            sub_meshes,
            vertex_buffer,
            index_buffer,
            texture_set,
            objects,
            prop_models,
            light_sources,
            sound_sources,
            #[cfg(feature = "debug")]
            effect_sources,
            tile_picker_vertex_buffer,
            tile_picker_index_buffer,
            #[cfg(feature = "debug")]
            tile_vertex_buffer,
            #[cfg(feature = "debug")]
            tile_index_buffer,
            #[cfg(feature = "debug")]
            tile_submeshes,
            object_kdtree,
            light_source_kdtree,
            background_music_track_name,
            videos,
            #[cfg(feature = "debug")]
            map_data,
        }
    }
}

impl Cacheable for Map {
    fn size(&self) -> usize {
        #[cfg(not(feature = "debug"))]
        let debug_fields = 0;
        #[cfg(feature = "debug")]
        let debug_fields = self.effect_sources.len() * size_of::<EffectSource>() + self.tile_submeshes.len() * size_of::<SubMesh>();

        size_of::<Self>()
            + self.tiles.len() * size_of::<Tile>()
            + self.sub_meshes.len() * size_of::<SubMesh>()
            + self.objects.count() as usize * size_of::<Object>()
            + self.light_sources.count() as usize * size_of::<LightSource>()
            + self.sound_sources.len() * size_of::<SoundSource>()
            + self.object_kdtree.dynamic_size()
            + self.light_source_kdtree.dynamic_size()
            + debug_fields
    }
}

impl Map {
    fn average_tile_height(tile: &Tile) -> f32 {
        (tile.southwest_corner_height + tile.southeast_corner_height + tile.northwest_corner_height + tile.northeast_corner_height) / 4.0
    }

    pub fn get_world_position(&self, position: TilePosition) -> Option<Point3<f32>> {
        let height = Self::average_tile_height(self.get_tile(position)?);

        Some(Point3::new(
            position.x as f32 * GAT_TILE_SIZE + (GAT_TILE_SIZE / 2.0),
            height,
            position.y as f32 * GAT_TILE_SIZE + (GAT_TILE_SIZE / 2.0),
        ))
    }

    pub fn get_tile(&self, position: TilePosition) -> Option<&Tile> {
        self.tiles.get(position.x as usize + position.y as usize * self.width as usize)
    }

    fn tile_index(&self, position: TilePosition) -> Option<usize> {
        let index = position.x as usize + position.y as usize * self.width as usize;
        (index < self.tiles.len()).then_some(index)
    }

    /// Flags for a cell, preferring any runtime override from the server.
    fn effective_flags(&self, position: TilePosition) -> Option<TileFlags> {
        let index = self.tile_index(position)?;

        if let Some(flags) = self.dynamic_cells.read().unwrap().get(&index) {
            return Some(*flags);
        }
        self.tiles.get(index).map(|tile| tile.flags)
    }

    /// Apply a server cell re-type. `cell_type` is a gat type, decoded exactly
    /// as the map loader decodes the `.gat` file so the two cannot drift.
    ///
    /// Without this the client believes an Ice Wall's cells are still walkable
    /// and happily paths straight through it, then disagrees with the server.
    pub fn set_cell_type(&self, position: TilePosition, cell_type: u16) {
        let Some(index) = self.tile_index(position) else {
            return;
        };

        let Some(flags) = tile_flags_for_cell_type(cell_type) else {
            return;
        };

        let original = self.tiles.get(index).map(|tile| tile.flags);
        let mut dynamic_cells = self.dynamic_cells.write().unwrap();

        // Reverting to the map's own value drops the override entirely, so the
        // table stays empty in the common case.
        match original == Some(flags) {
            true => dynamic_cells.remove(&index),
            false => dynamic_cells.insert(index, flags),
        };
    }

    /// Drop every runtime cell override. **Must** run when a map is entered:
    /// the `Map` is cached behind an `Arc`, so overrides would otherwise
    /// survive into a later visit.
    pub fn clear_dynamic_cells(&self) {
        self.dynamic_cells.write().unwrap().clear();
    }

    pub fn background_music_track_name(&self) -> Option<&str> {
        self.background_music_track_name.as_deref()
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn get_texture_set(&self) -> &Arc<TextureSet> {
        &self.texture_set
    }

    pub fn get_model_vertex_buffer(&self) -> &Arc<Buffer<ModelVertex>> {
        &self.vertex_buffer
    }

    pub fn get_model_index_buffer(&self) -> &Arc<Buffer<u32>> {
        &self.index_buffer
    }

    pub fn get_tile_picker_vertex_buffer(&self) -> &Buffer<TileVertex> {
        &self.tile_picker_vertex_buffer
    }

    pub fn get_level_bound(&self) -> AABB {
        self.level_bound
    }

    pub fn get_tile_picker_index_buffer(&self) -> &Buffer<u32> {
        &self.tile_picker_index_buffer
    }

    pub fn set_ambient_sound_sources(&self, audio_engine: &AudioEngine<GameFileLoader>) {
        // We increase the range of the ambient sound,
        // so that it can ease better into the world.
        const AMBIENT_SOUND_MULTIPLIER: f32 = 1.5;

        // This is the only correct place to clear the ambient sound.
        audio_engine.clear_ambient_sound();

        let log_ambient = std::env::var_os("KORANGAR_PACKET_LOG").is_some();
        if log_ambient {
            eprintln!("[ambient] {} sound sources on this map", self.sound_sources.len());
        }

        for sound in self.sound_sources.iter() {
            let sound_effect_key = audio_engine.load(&sound.sound_file);

            if log_ambient {
                eprintln!(
                    "[ambient] {} at ({:.1},{:.1},{:.1}) range={:.1} (eff {:.1}) volume={:.2} cycle={:?}",
                    sound.sound_file,
                    sound.position.x,
                    sound.position.y,
                    sound.position.z,
                    sound.range,
                    sound.range * AMBIENT_SOUND_MULTIPLIER,
                    sound.volume,
                    sound.cycle,
                );
            }

            audio_engine.add_ambient_sound(
                sound_effect_key,
                sound.position,
                sound.range * AMBIENT_SOUND_MULTIPLIER,
                sound.volume,
                sound.cycle,
            );
        }

        audio_engine.prepare_ambient_sound_world();
    }

    // We want to make sure that the object set also captures the lifetime of the
    // map, so we never have a stale object set.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn cull_objects_with_frustum<'a>(
        &'a self,
        camera: &dyn Camera,
        object_set: &'a mut ResourceSetBuffer<ObjectKey>,
        #[cfg(feature = "debug")] enabled: bool,
    ) -> ResourceSet<'a, ObjectKey> {
        #[cfg(feature = "debug")]
        if !enabled {
            return object_set.create_set(|visible_objects| {
                self.objects.iter().for_each(|(object_key, _)| visible_objects.push(object_key));
            });
        }

        let frustum = Frustum::new(camera.view_projection_matrix(), true);

        object_set.create_set(|visible_objects| {
            self.object_kdtree.query(&frustum, visible_objects);
        })
    }

    // We want to make sure that the object set also captures the lifetime of the
    // map, so we never have a stale object set.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn cull_objects_in_sphere<'a>(
        &'a self,
        sphere: Sphere,
        object_set: &'a mut ResourceSetBuffer<ObjectKey>,
        #[cfg(feature = "debug")] enabled: bool,
    ) -> ResourceSet<'a, ObjectKey> {
        #[cfg(feature = "debug")]
        if !enabled {
            return object_set.create_set(|visible_objects| {
                self.objects.iter().for_each(|(object_key, _)| visible_objects.push(object_key));
            });
        }

        object_set.create_set(|visible_objects| {
            self.object_kdtree.query(&sphere, visible_objects);
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_objects(
        &self,
        instructions: &mut Vec<ModelInstruction>,
        object_set: &ResourceSet<ObjectKey>,
        animation_timer_ms: f32,
        camera: &dyn Camera,
    ) {
        for object_key in object_set.iterate_visible().copied() {
            if let Some(object) = self.objects.get(object_key) {
                object.render_geometry(instructions, animation_timer_ms, camera);
            }
        }
    }

    /// A model preloaded into this map's geometry buffer but placed at runtime.
    /// `None` if the map finished loading before the model was requested, or if
    /// the model failed to load — callers must treat a missing prop as "draw
    /// nothing" rather than as an error.
    pub fn prop_model(&self, model_file: &str) -> Option<&Arc<Model>> {
        self.prop_models.get(model_file)
    }

    /// Draw runtime-placed props (Hunter traps). Deliberately *not* culled
    /// through the object kd-tree: that tree is built at load from the map's own
    /// objects and a trap appears afterwards. There are only ever a handful of
    /// live traps, so per-frame frustum work would cost more than it saves.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_props(
        &self,
        instructions: &mut Vec<ModelInstruction>,
        props: &[(EntityId, Arc<Model>, Transform)],
        animation_timer_ms: f32,
        camera: &dyn Camera,
    ) {
        for (_entity_id, model, transform) in props {
            model.render_geometry(instructions, transform, animation_timer_ms, camera);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_ground(&self, instructions: &mut Vec<ModelInstruction>) {
        self.sub_meshes.iter().for_each(|mesh| {
            instructions.push(ModelInstruction {
                model_matrix: Matrix4::identity(),
                index_offset: mesh.index_offset,
                index_count: mesh.index_count,
                base_vertex: mesh.base_vertex,
                texture_index: mesh.texture_index,
                distance: f32::MAX,
                transparent: mesh.transparent,
            });
        });
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_water<'a>(&'a self, water_instruction: &mut Option<WaterInstruction<'a>>, animation_timer_ms: f32) {
        if let Some(water_plane) = self.water_plane.as_ref() {
            let frame = animation_timer_ms / (1000.0 / 60.0);

            let waveform_phase_shift = frame * water_plane.wave_speed.0;
            let waveform_amplitude = water_plane.wave_height;
            let waveform_frequency = water_plane.wave_pitch;
            let water_opacity = water_plane.water_opacity;

            let water_texture_index = (frame as u32 / water_plane.texture_cycling_interval) % water_plane.water_textures.len() as u32;

            *water_instruction = Some(WaterInstruction {
                water_texture: &water_plane.water_textures[water_texture_index as usize],
                water_vertex_buffer: &water_plane.vertex_buffer,
                water_index_buffer: &water_plane.index_buffer,
                texture_repeat: water_plane.texture_repeat,
                waveform_phase_shift,
                waveform_amplitude,
                waveform_frequency,
                water_opacity,
            });
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_entities(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        entities: &[Entity],
        camera: &dyn Camera,
        client_tick: ClientTick,
    ) {
        entities
            .iter()
            .enumerate()
            .for_each(|(index, entity)| entity.render(instructions, camera, index != 0, client_tick));
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_dead_entities(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        entities: &[Entity],
        camera: &dyn Camera,
        client_tick: ClientTick,
    ) {
        entities
            .iter()
            .for_each(|entity| entity.render(instructions, camera, false, client_tick));
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_ground_items(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        items: &[GroundItem],
        camera: &dyn Camera,
        client_tick: ClientTick,
    ) {
        items.iter().for_each(|item| item.render(instructions, camera, client_tick));
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_entities_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, entities: &[Entity], camera: &dyn Camera) {
        entities.iter().for_each(|entity| {
            entity.render_debug(instructions, camera);
        });
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_bounding(
        &self,
        instructions: &mut Vec<DebugAabbInstruction>,
        frustum_culling: bool,
        object_set: &ResourceSet<ObjectKey>,
    ) {
        let intersection_set: HashSet<ObjectKey> = object_set.iterate_visible().copied().collect();

        self.objects.iter().for_each(|(object_key, object)| {
            let intersects = intersection_set.contains(&object_key);

            let color = match !frustum_culling || intersects {
                true => Color::rgb_u8(255, 255, 0),
                false => Color::rgb_u8(255, 0, 255),
            };

            let bounding_box = object.calculate_object_aabb();
            let offset = bounding_box.size().y / 2.0;
            let position = bounding_box.center() - Vector3::new(0.0, offset, 0.0);
            let transform = Transform::position(position);
            let world_matrix = Model::calculate_bounding_box_matrix(&bounding_box, &transform);

            instructions.push(DebugAabbInstruction {
                world: world_matrix,
                color,
            });
        });
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_walk_indicator(&self, instruction: &mut Option<IndicatorInstruction>, color: Color, position: TilePosition) {
        const OFFSET: f32 = 1.0;

        // Since the picker buffer is always one frame behind the current scene, a map
        // transition can cause the picked tile to be out of bounds. To avoid a
        // panic we ensure the coordinates are in bounds.
        if position.x >= self.width || position.y >= self.height {
            return;
        }

        let Some(tile) = self.get_tile(position) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!("[{}] walk indicator out of map bounds", "error".red());
            return;
        };

        if tile.flags.contains(TileFlags::WALKABLE) {
            let base_x = position.x as f32 * GAT_TILE_SIZE;
            let base_y = position.y as f32 * GAT_TILE_SIZE;

            let upper_left = Point3::new(base_x, tile.southwest_corner_height + OFFSET, base_y);
            let upper_right = Point3::new(base_x + GAT_TILE_SIZE, tile.southeast_corner_height + OFFSET, base_y);
            let lower_left = Point3::new(base_x, tile.northwest_corner_height + OFFSET, base_y + GAT_TILE_SIZE);
            let lower_right = Point3::new(
                base_x + GAT_TILE_SIZE,
                tile.northeast_corner_height + OFFSET,
                base_y + GAT_TILE_SIZE,
            );

            *instruction = Some(IndicatorInstruction {
                upper_left,
                upper_right,
                lower_left,
                lower_right,
                color,
            });
        }
    }

    /// Draws the area a ground-targeted skill will cover, one tile-conforming
    /// decal per cell, so aiming shows the real footprint instead of a single
    /// cursor tile. `cells` are `(dx, dy)` offsets from `center` — see
    /// [`skill_footprint`], which mirrors Hercules' own layout table.
    ///
    /// Uses the depth-tested decal path rather than [`IndicatorInstruction`]
    /// (which is a single `Option`, so it cannot express a multi-cell shape) and
    /// rides the terrain via each tile's own corner heights, exactly like
    /// [`Self::render_walk_indicator`].
    ///
    /// [`skill_footprint`]: crate::world::skill_footprint
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_skill_footprint(
        &self,
        renderer: &mut EffectRenderer,
        texture: &Arc<Texture>,
        center: TilePosition,
        cells: &[(i8, i8)],
        color: Color,
    ) {
        /// Lifted slightly further than the walk indicator so the two do not
        /// z-fight when both land on the same tile.
        const OFFSET: f32 = 1.5;

        for &(dx, dy) in cells {
            // Offsets are signed and the map edge is at 0, so a footprint
            // hanging off the edge must be clipped rather than wrapped.
            let Some(x) = center.x.checked_add_signed(dx as i16) else {
                continue;
            };
            let Some(y) = center.y.checked_add_signed(dy as i16) else {
                continue;
            };

            if x >= self.width || y >= self.height {
                continue;
            }

            let Some(tile) = self.get_tile(TilePosition { x, y }) else {
                continue;
            };

            // Skipping unwalkable cells keeps the shape honest: the server will
            // not place a unit on a wall either.
            if !tile.flags.contains(TileFlags::WALKABLE) {
                continue;
            }

            let base_x = x as f32 * GAT_TILE_SIZE;
            let base_y = y as f32 * GAT_TILE_SIZE;

            renderer.render_ground_decal(
                [
                    Point3::new(base_x, tile.southwest_corner_height + OFFSET, base_y),
                    Point3::new(base_x + GAT_TILE_SIZE, tile.southeast_corner_height + OFFSET, base_y),
                    Point3::new(base_x, tile.northwest_corner_height + OFFSET, base_y + GAT_TILE_SIZE),
                    Point3::new(
                        base_x + GAT_TILE_SIZE,
                        tile.northeast_corner_height + OFFSET,
                        base_y + GAT_TILE_SIZE,
                    ),
                ],
                texture.clone(),
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                ],
                color,
            );
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn ambient_light_color(&self) -> Color {
        self.lighting.ambient_light_color()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn directional_light(&self) -> (Vector3<f32>, Color) {
        self.lighting.directional_light()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn register_point_lights(
        &self,
        point_light_manager: &mut PointLightManager,
        light_source_set_buffer: &mut ResourceSetBuffer<LightSourceKey>,
        camera: &dyn Camera,
    ) {
        let frustum = Frustum::new(camera.view_projection_matrix(), true);

        let set = light_source_set_buffer.create_set(|buffer| {
            self.light_source_kdtree.query(&frustum, buffer);
        });

        for light_source_key in set.iterate_visible().copied() {
            let light_source = self.light_sources.get(light_source_key).unwrap();

            point_light_manager.register(
                PointLightId::new(light_source_key.key()),
                light_source.position,
                light_source.color.into(),
                light_source.range,
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn get_map_data(&self) -> &MapData {
        &self.map_data
    }

    #[cfg(feature = "debug")]
    pub fn get_object(&self, key: u32) -> &Object {
        self.objects.get(ObjectKey::new(key)).expect("object key should be valid")
    }

    #[cfg(feature = "debug")]
    pub fn get_light_source(&self, key: u32) -> &LightSource {
        self.light_sources
            .get(LightSourceKey::new(key))
            .expect("light source key should be valid")
    }

    #[cfg(feature = "debug")]
    pub fn get_sound_source(&self, index: u32) -> &SoundSource {
        &self.sound_sources[index as usize]
    }

    #[cfg(feature = "debug")]
    pub fn get_effect_source(&self, index: u32) -> &EffectSource {
        &self.effect_sources[index as usize]
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_overlay_tiles(
        &self,
        model_instructions: &mut Vec<ModelInstruction>,
        model_batches: &mut Vec<ModelBatch>,
        tile_texture_set: &Arc<TextureSet>,
    ) {
        let offset = model_instructions.len();
        let count = self.tile_submeshes.len();

        self.tile_submeshes.iter().for_each(|mesh| {
            model_instructions.push(ModelInstruction {
                model_matrix: Matrix4::identity(),
                index_offset: mesh.index_offset,
                index_count: mesh.index_count,
                base_vertex: mesh.base_vertex,
                texture_index: mesh.texture_index,
                distance: f32::MAX,
                transparent: mesh.transparent,
            });
        });

        model_batches.push(ModelBatch {
            offset,
            count,
            texture_set: tile_texture_set.clone(),
            vertex_buffer: self.tile_vertex_buffer.clone(),
            index_buffer: self.tile_index_buffer.clone(),
        });
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_entity_pathing(
        &self,
        model_instructions: &mut Vec<ModelInstruction>,
        model_batches: &mut Vec<ModelBatch>,
        entities: &[Entity],
        path_texture_set: &Arc<TextureSet>,
    ) {
        entities.iter().for_each(|entity| {
            if let Some(pathing) = entity.get_pathing() {
                let offset = model_instructions.len();

                pathing.submeshes.iter().for_each(|mesh| {
                    model_instructions.push(ModelInstruction {
                        model_matrix: Matrix4::identity(),
                        index_offset: mesh.index_offset,
                        index_count: mesh.index_count,
                        base_vertex: mesh.base_vertex,
                        texture_index: mesh.texture_index,
                        distance: f32::MAX,
                        transparent: mesh.transparent,
                    });
                });

                model_batches.push(ModelBatch {
                    offset,
                    count: pathing.submeshes.len(),
                    texture_set: path_texture_set.clone(),
                    vertex_buffer: pathing.vertex_buffer.clone(),
                    index_buffer: pathing.index_buffer.clone(),
                });
            }
        });
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_markers(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        render_options: &RenderOptions,
        entities: &[Entity],
        point_light_set: &PointLightSet,
        hovered_marker_identifier: Option<MarkerIdentifier>,
    ) {
        use super::SoundSourceExt;
        use crate::EffectSourceExt;

        if render_options.show_object_markers {
            self.objects.iter().for_each(|(object_key, object)| {
                let marker_identifier = MarkerIdentifier::Object(object_key.key());

                object.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_light_markers {
            self.light_sources.iter().for_each(|(key, light_source)| {
                let marker_identifier = MarkerIdentifier::LightSource(key.key());

                light_source.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_sound_markers {
            self.sound_sources.iter().enumerate().for_each(|(index, sound_source)| {
                let marker_identifier = MarkerIdentifier::SoundSource(index as u32);

                sound_source.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_effect_markers {
            self.effect_sources.iter().enumerate().for_each(|(index, effect_source)| {
                let marker_identifier = MarkerIdentifier::EffectSource(index as u32);

                effect_source.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_entity_markers {
            entities.iter().enumerate().for_each(|(index, entity)| {
                let marker_identifier = MarkerIdentifier::Entity(index as u32);

                entity.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_shadow_markers {
            point_light_set
                .with_shadow_iterator()
                .enumerate()
                .for_each(|(index, light_source)| {
                    let marker_identifier = MarkerIdentifier::Shadow(index as u32);

                    renderer.render_marker(
                        camera,
                        marker_identifier,
                        light_source.position,
                        hovered_marker_identifier.contains(&marker_identifier),
                    );
                });
        }
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_marker_overlay(
        &self,
        aabb_instructions: &mut Vec<DebugAabbInstruction>,
        circle_instructions: &mut Vec<DebugCircleInstruction>,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        point_light_set: &PointLightSet,
        animation_timer_ms: f32,
    ) {
        let animation_seconds = animation_timer_ms / 1000.0;
        let offset = (f32::sin(animation_seconds * 5.0) + 0.5).clamp(0.0, 1.0);
        let overlay_color = Color::rgb(1.0, offset, 1.0 - offset);

        match marker_identifier {
            MarkerIdentifier::Object(key) => self
                .objects
                .get(ObjectKey::new(key))
                .unwrap()
                .render_bounding_box(aabb_instructions, overlay_color),

            MarkerIdentifier::LightSource(key) => {
                let light_source = self.light_sources.get(LightSourceKey::new(key)).unwrap();

                if let Some((screen_position, screen_size)) =
                    Self::calculate_circle_screen_position_size(camera, light_source.position, light_source.range)
                {
                    circle_instructions.push(DebugCircleInstruction {
                        position: light_source.position,
                        color: overlay_color,
                        screen_position,
                        screen_size,
                    });
                };
            }
            MarkerIdentifier::SoundSource(index) => {
                let sound_source = &self.sound_sources[index as usize];

                if let Some((screen_position, screen_size)) =
                    Self::calculate_circle_screen_position_size(camera, sound_source.position, sound_source.range)
                {
                    circle_instructions.push(DebugCircleInstruction {
                        position: sound_source.position,
                        color: overlay_color,
                        screen_position,
                        screen_size,
                    });
                };
            }
            MarkerIdentifier::EffectSource(_index) => {}
            MarkerIdentifier::Particle(_index, _particle_index) => {}
            MarkerIdentifier::Entity(_index) => {}
            MarkerIdentifier::Shadow(index) => {
                let point_light = point_light_set.with_shadow_iterator().nth(index as usize).unwrap();

                if let Some((screen_position, screen_size)) =
                    Self::calculate_circle_screen_position_size(camera, point_light.position, point_light.range)
                {
                    circle_instructions.push(DebugCircleInstruction {
                        position: point_light.position,
                        color: overlay_color,
                        screen_position,
                        screen_size,
                    });
                };
            }
        }
    }

    #[cfg(feature = "debug")]
    fn calculate_circle_screen_position_size(
        camera: &dyn Camera,
        position: Point3<f32>,
        extent: f32,
    ) -> Option<(ScreenPosition, ScreenSize)> {
        let corner_offset = (extent.powf(2.0) * 2.0).sqrt();
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, corner_offset);

        if top_left_position.w < 0.1 && bottom_right_position.w < 0.1 && camera.distance_to(position) > extent {
            return None;
        }

        let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);
        Some((screen_position, screen_size))
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn advance_videos(&self, queue: &Queue, delta_time: f64) {
        let mut videos = self.videos.lock().unwrap();

        for video in videos.iter_mut() {
            if video.should_show_next_frame(delta_time) {
                video.update_texture(queue);
            }

            video.check_for_next_frame();
        }
    }
}

/// Gat cell type to [`TileFlags`], mirroring `FromBytes for TileFlags` in
/// `ragnarok-formats`.
///
/// The server sends the same numbering over `ZC_UPDATE_MAPINFO` that the `.gat`
/// file uses, so the two mappings must agree or a re-typed cell would get
/// different flags from the identical byte in the map file. `None` for an
/// unknown type: ignoring it is safer than guessing, since guessing "blocked"
/// could strand the player behind a wall that is not there.
fn tile_flags_for_cell_type(cell_type: u16) -> Option<TileFlags> {
    let flags = match cell_type {
        0 => TileFlags::WALKABLE,
        1 => TileFlags::empty(),
        2 => TileFlags::WATER,
        3 => TileFlags::WATER | TileFlags::WALKABLE,
        4 => TileFlags::WATER | TileFlags::SNIPABLE,
        5 => TileFlags::CLIFF | TileFlags::SNIPABLE,
        6 => TileFlags::CLIFF,
        _ => return None,
    };
    Some(flags)
}

impl Traversable for Map {
    fn is_walkable(&self, position: TilePosition) -> bool {
        self.effective_flags(position)
            .map(|flags| flags.contains(TileFlags::WALKABLE))
            .unwrap_or(false)
    }

    fn is_snipeable(&self, position: TilePosition) -> bool {
        self.effective_flags(position)
            .map(|flags| flags.contains(TileFlags::SNIPABLE))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod dynamic_cell_tests {
    use ragnarok_formats::map::TileFlags;

    use super::tile_flags_for_cell_type;

    /// `Map` itself cannot be built in a unit test -- it owns GPU buffers -- so
    /// this pins the part that can drift: the cell-type mapping must match the
    /// `.gat` decoding in `ragnarok-formats`, since the server re-types cells
    /// using the same numbering the map file uses.
    #[test]
    fn cell_types_match_the_gat_mapping() {
        assert_eq!(tile_flags_for_cell_type(0), Some(TileFlags::WALKABLE));
        assert_eq!(tile_flags_for_cell_type(1), Some(TileFlags::empty()));
        assert_eq!(tile_flags_for_cell_type(2), Some(TileFlags::WATER));
        assert_eq!(tile_flags_for_cell_type(3), Some(TileFlags::WATER | TileFlags::WALKABLE));
        assert_eq!(tile_flags_for_cell_type(4), Some(TileFlags::WATER | TileFlags::SNIPABLE));
        assert_eq!(tile_flags_for_cell_type(5), Some(TileFlags::CLIFF | TileFlags::SNIPABLE));
        assert_eq!(tile_flags_for_cell_type(6), Some(TileFlags::CLIFF));
    }

    /// Ice Wall sends type 5, which must not be walkable -- the entire point of
    /// handling this packet.
    #[test]
    fn ice_wall_cells_are_not_walkable() {
        let flags = tile_flags_for_cell_type(5).expect("type 5 is a known cell type");
        assert!(!flags.contains(TileFlags::WALKABLE));
    }

    #[test]
    fn unknown_cell_types_are_ignored() {
        assert_eq!(tile_flags_for_cell_type(7), None);
        assert_eq!(tile_flags_for_cell_type(u16::MAX), None);
    }
}
