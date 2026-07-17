use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use cgmath::Deg;
#[cfg(feature = "debug")]
use korangar_container::CacheStatistics;
use korangar_container::SimpleCache;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_loaders::FileLoader;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::effect::EffectData;
use ragnarok_formats::version::GenericFormatMetadata;
use wgpu::BlendFactor;

use super::error::LoadError;
use super::{ImageType, TextureLoader};
use crate::graphics::Color;
use crate::loaders::GameFileLoader;
use crate::world::{AnimationType, Effect, Frame, FrameType, Layer, MultiTexturePresent};

const MAX_CACHE_COUNT: u32 = 256;
// We cache effects only by count.
const MAX_CACHE_SIZE: usize = usize::MAX;

pub struct EffectLoader {
    game_file_loader: Arc<GameFileLoader>,
    cache: Mutex<SimpleCache<String, Arc<Effect>>>,
}

impl EffectLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>) -> Self {
        Self {
            game_file_loader,
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

    fn load(&self, path: &str, texture_loader: &TextureLoader) -> Result<Arc<Effect>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load effect from {}", path.magenta()));

        let bytes = self
            .game_file_loader
            .get(&format!("data\\texture\\effect\\{path}"))
            .map_err(LoadError::File)?;
        let mut byte_reader: ByteReader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);

        // TODO: Add fallback
        let effect_data = EffectData::from_bytes(&mut byte_reader).map_err(LoadError::Conversion)?;

        let prefix = match path.chars().rev().position(|character| character == '\\') {
            Some(offset) => path.split_at(path.len() - offset).0,
            None => "",
        };

        let effect = Arc::new(Effect::new(
            effect_data.frames_per_second as usize,
            effect_data.max_key as usize,
            effect_data
                .layers
                .into_iter()
                .map(|layer_data| {
                    let mut previous_source_blend_factor = None;
                    let mut previous_destination_blend_factor = None;

                    Layer::new(
                        layer_data
                            .texture_names
                            .into_iter()
                            .map(|name| {
                                let path = format!("effect\\{}{}", prefix, name.name);
                                texture_loader.get_or_load(&path, ImageType::Color).unwrap()
                            })
                            .collect(),
                        layer_data
                            .frames
                            .into_iter()
                            .map(|frame| {
                                let source_blend_factor = parse_blend_factor(frame.source_blend_factor, previous_source_blend_factor, true);
                                previous_source_blend_factor = Some(source_blend_factor);

                                let destination_blend_factor =
                                    parse_blend_factor(frame.destination_blend_factor, previous_destination_blend_factor, false);
                                previous_destination_blend_factor = Some(destination_blend_factor);

                                let animation_type = parse_animation_type(frame.animation_type);
                                let frame_type = parse_frame_type(frame.frame_type);
                                let mt_present = parse_mt_present(frame.mt_present);

                                Frame::new(
                                    frame.frame_index as usize,
                                    frame_type,
                                    frame.offset,
                                    frame.uv,
                                    frame.xy,
                                    frame.texture_index,
                                    animation_type,
                                    frame.delay,
                                    Deg(frame.angle / (1024.0 / 360.0)).into(),
                                    Color::rgba(
                                        frame.color[0] / 255.0,
                                        frame.color[1] / 255.0,
                                        frame.color[2] / 255.0,
                                        frame.color[3] / 255.0,
                                    ),
                                    source_blend_factor,
                                    destination_blend_factor,
                                    mt_present,
                                )
                            })
                            .collect(),
                    )
                })
                .collect(),
        ));

        let _result = self.cache.lock().unwrap().insert(path.to_string(), effect.clone());

        #[cfg(feature = "debug")]
        if let Err(error) = _result {
            print_debug!(
                "[{}] effect could not be added to cache. Path: '{}': {:?}",
                "error".red(),
                &path,
                error
            );
        }

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(effect)
    }

    pub fn get_or_load(&self, path: &str, texture_loader: &TextureLoader) -> Result<Arc<Effect>, LoadError> {
        let Some(effect) = self.cache.lock().unwrap().get(path).cloned() else {
            return self.load(path, texture_loader);
        };

        Ok(effect)
    }
}

fn parse_blend_factor(value: i32, previous: Option<BlendFactor>, is_source: bool) -> BlendFactor {
    match value {
        0 => previous.unwrap(),
        1 => BlendFactor::Zero,
        2 => BlendFactor::One,
        3 => BlendFactor::Src,
        4 => BlendFactor::OneMinusSrc,
        5 => BlendFactor::SrcAlpha,
        6 => BlendFactor::OneMinusSrcAlpha,
        7 => BlendFactor::DstAlpha,
        8 => BlendFactor::OneMinusDstAlpha,
        9 => BlendFactor::Dst,
        10 => BlendFactor::OneMinusDst,
        11 => BlendFactor::SrcAlphaSaturated,
        // D3DBLEND_BOTHSRCALPHA
        //
        // Obsolete. Starting with DirectX 6, you can achieve the same effect
        // by setting the source and destination blend factors to D3DBLEND_SRCALPHA
        // and D3DBLEND_INVSRCALPHA in separate calls.
        12 if is_source => BlendFactor::SrcAlpha,
        12 if !is_source => BlendFactor::OneMinusSrcAlpha,
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown blend factor found in frame data: {value}", "error".red());
            BlendFactor::Zero
        }
    }
}

fn parse_animation_type(value: i32) -> AnimationType {
    match value {
        0 => AnimationType::Type0,
        1 => AnimationType::Type1,
        2 => AnimationType::Type2,
        3 => AnimationType::Type3,
        4 => AnimationType::Type4,
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown animation type found in frame data: {value}", "error".red());
            AnimationType::Type1
        }
    }
}

fn parse_frame_type(value: i32) -> FrameType {
    match value {
        0 => FrameType::Basic,
        1 => FrameType::Morphing,
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown frame type found in frame data: {value}", "error".red());
            FrameType::Basic
        }
    }
}

#[cfg(test)]
mod diagnostics {
    use korangar_loaders::FileLoader;
    use ragnarok_bytes::{ByteReader, FromBytes};
    use ragnarok_formats::effect::EffectData;
    use ragnarok_formats::version::GenericFormatMetadata;

    use crate::loaders::GameFileLoader;

    /// Diagnostic dump for the M1-008 STR renderer work; deliberately ignored
    /// because it opens the configured multi-gigabyte GRFs.
    ///
    /// Run: cargo test -p korangar str_frame_structure -- --ignored --nocapture
    #[test]
    #[ignore]
    fn reports_str_frame_structure() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        // The files mapped in `skill_hit_effects` / `ground_skill_effect`,
        // plus `cloudh.str` as a canonical basic-plus-morphing-pair reference.
        let paths = [
            "stormgust.str",    // Storm Gust ground cast (storm_min.str is its reduced variant)
            "thunderstorm.str", // Thunderstorm ground cast (classic)
            "firehit1.str",     // Fire Bolt per-hit burst (classic, random 1-3)
            "firehit2.str",
            "firehit3.str",
            "windhit1.str", // Thunderstorm / Lightning Bolt per-hit burst (random 1-3)
            "windhit2.str",
            "windhit3.str",
            "lightning.str",                                                       // Lightning Bolt strike (classic)
            "new_soulexpansion\\new_soulexpansion_hit\\new_soulexpansion_hit.str", // Soul Strike
            "freeze.str",                                                          // Frost Diver target
            "earthhit.str",                                                        // Pierce / earth spells
            "holyhit.str",                                                         // priest holy hits
            "firepillarbomb.str",                                                  // Fire Pillar target
            "shockwavehit.str",                                                    // Shockwave Trap
            "sandman.str",                                                         // Sandman Trap
            "freezing.str",                                                        // Freezing Trap
            "blastmine.str",                                                       // Blast Mine
            "claymore.str",                                                        // Claymore Trap
            "poisonreact.str",                                                     // Poison React hit
            "firewall1.str",                                                       // Firewall cast flash (random 1-2)
            "firewall2.str",
            "sanctuary.str",  // Sanctuary cast
            "magnus.str",     // Magnus Exorcismus cast
            "firepillar.str", // Fire Pillar cast
            "meteor1.str",    // Meteor Storm (random 1-4)
            "meteor2.str",
            "meteor3.str",
            "meteor4.str",
            "lord.str",             // Lord of Vermilion
            "quagmire.str",         // Quagmire
            "crashearth.str",       // Hammer Fall
            "skidtrap.str",         // Skid Trap
            "venomdust.str",        // Venom Dust
            "pierce.str",           // Knight Pierce caster
            "brandish.str",         // Brandish Spear target sweep
            "brandish2.str",        // Brandish Spear caster
            "spearstab.str",        // Spear Stab caster
            "spearboomerang.str",   // Spear Boomerang caster
            "bowling.str",          // Bowling Bash caster
            "sonicblow.str",        // Sonic Blow target
            "이그니션브레이크.str", // Rune Knight Ignition Break
            "cloudh.str",
        ];

        let filter = std::env::var("KORANGAR_EFFECT_FILTER").ok();
        for path in paths {
            if filter.as_ref().is_some_and(|filter| !path.contains(filter)) {
                continue;
            }
            let full_path = format!("data\\texture\\effect\\{path}");
            let Ok(bytes) = game_file_loader.get(&full_path) else {
                println!("=== {path}: NOT FOUND");
                continue;
            };
            let mut byte_reader: ByteReader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);
            let effect_data = match EffectData::from_bytes(&mut byte_reader) {
                Ok(data) => data,
                Err(error) => {
                    println!("=== {path}: PARSE ERROR {error:?}");
                    continue;
                }
            };

            println!(
                "=== {path}: version {:?}, fps {}, max_key {}, {} layers, {} unconsumed bytes",
                effect_data.version,
                effect_data.frames_per_second,
                effect_data.max_key,
                effect_data.layers.len(),
                bytes.len() - byte_reader.get_offset(),
            );

            for (layer_index, layer) in effect_data.layers.iter().enumerate() {
                let texture_names: Vec<&str> = layer.texture_names.iter().map(|name| name.name.as_str()).collect();
                println!(
                    "  layer {layer_index}: {} textures {:?}, {} frames",
                    layer.texture_names.len(),
                    texture_names,
                    layer.frames.len()
                );
                for frame in &layer.frames {
                    let width = (frame.xy[0] - frame.xy[1]).abs().max((frame.xy[2] - frame.xy[3]).abs());
                    let height = (frame.xy[4] - frame.xy[7]).abs().max((frame.xy[5] - frame.xy[6]).abs());
                    println!(
                        "    key {:>3} type {} anim {} tex {:>4.1} delay {:>5.1} blend {}/{} mt {} offset ({:>6.1},{:>6.1}) size \
                         ({:>6.1},{:>6.1}) alpha {:>5.1} angle {:>7.1}",
                        frame.frame_index,
                        frame.frame_type,
                        frame.animation_type,
                        frame.texture_index,
                        frame.delay,
                        frame.source_blend_factor,
                        frame.destination_blend_factor,
                        frame.mt_present,
                        frame.offset.x,
                        frame.offset.y,
                        width,
                        height,
                        frame.color[3],
                        frame.angle,
                    );
                }
            }
        }

        for path in [
            "ring_yellow.tga",
            "대폭발.tga",
            "lens1.tga",
            "lens2.tga",
            "purpleslash.tga",
            "ring2.bmp",
        ] {
            let full_path = format!("data\\texture\\effect\\{path}");
            println!(
                "=== procedural texture {path}: {}",
                if game_file_loader.get(&full_path).is_ok() {
                    "FOUND"
                } else {
                    "NOT FOUND"
                }
            );
        }
        for path in ["data\\sprite\\이팩트\\창.spr", "data\\sprite\\이팩트\\창.act"] {
            println!(
                "=== projectile asset {path}: {}",
                if game_file_loader.get(path).is_ok() { "FOUND" } else { "NOT FOUND" }
            );
        }
    }
}

fn parse_mt_present(value: i32) -> MultiTexturePresent {
    match value {
        0 => MultiTexturePresent::None,
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown multi texture present found in frame data: {value}", "error".red());
            MultiTexturePresent::None
        }
    }
}
