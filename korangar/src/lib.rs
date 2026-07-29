#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![feature(iter_next_chunk)]
#![feature(negative_impls)]
#![feature(proc_macro_hygiene)]
#![feature(random)]
#![feature(type_changing_struct_update)]
#![feature(unsized_const_params)]
#![feature(variant_count)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(macro_metavar_expr)]
#![feature(unsafe_cell_access)]
#![feature(impl_trait_in_assoc_type)]
#![feature(thread_local)]

// Helper macro to time and print the startup time of Korangar
macro_rules! time_phase {
    ($message:expr, { $($statements:tt)* }) => {
        #[cfg(feature = "debug")]
        let _statement_timer = korangar_debug::logging::Timer::new($message);

        $($statements)*

        #[cfg(feature = "debug")]
        _statement_timer.stop();
    }
}

mod dm;
mod graphics;
use crate::dm::DmCampaignStatePathExt;
mod input;
mod state;
#[macro_use]
mod interface;
mod loaders;
#[cfg(feature = "debug")]
mod networking;
mod renderer;
mod settings;
mod system;
mod world;

use std::collections::HashMap;
use std::io::Cursor;
use ragnarok_formats::transform::Transform;
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use cgmath::{InnerSpace, Point3, Vector3};
use image::{EncodableLayout, ImageFormat, ImageReader};
use input::{MouseInputMode, MouseModeExt};
use korangar_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
use korangar_interface::layout::MouseButton;
use korangar_interface::{Interface, MouseMode};
use korangar_networking::{
    DisconnectReason, HotkeyState, LoginServerLoginData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem, SellItem,
    SupportedPacketVersion,
};
#[cfg(feature = "debug")]
use networking::{PacketHistory, PacketHistoryCallback};
#[cfg(not(feature = "debug"))]
use ragnarok_packets::handler::NoPacketCallback;
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{
    AccountId, AttackRange, BuyShopItemsResult, CharacterServerInformation, ClientTick, Direction, DisappearanceReason, EntityId,
    ExperienceType, HotbarSlot, ItemId, PartyId, SellItemsResult, SkillId, SkillLevel, SkillType, TilePosition, UnitId, WorldPosition,
};
use renderer::InterfaceRenderer;
use rust_state::{ManuallyAssertExt, State};
#[cfg(feature = "debug")]
use rust_state::{VecIndexExt, VecLookupExt};
use settings::{
    AudioSettings, AudioSettingsPathExt, GraphicsSettingsCapabilities, GraphicsSettingsPathExt, InterfaceSettings, InterfaceSettingsPathExt,
};
use state::hotbar::HotbarPathExt;
use state::inventory::InventoryPathExt;
use state::localization::Localization;
use state::skills::SkillTreePathExt;
use state::theme::{CursorThemePathExt, IndicatorThemePathExt, InterfaceThemePathExt, WorldThemePathExt};
use state::{ChatMessage, ClientState, ClientStatePathExt, client_state, this_entity, this_player};
#[cfg(feature = "debug")]
use wgpu::Device;
use wgpu::wgt::{Dx12SwapchainKind, Dx12UseFrameLatencyWaitableObject, WgpuHasDisplayHandle};
use wgpu::{
    Adapter, AdapterInfo, BackendOptions, Backends, DeviceDescriptor, DeviceType, Dx12BackendOptions, Dx12Compiler, ExperimentalFeatures,
    ForceShaderModelToken, GlBackendOptions, GlDebugFns, GlFenceBehavior, Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags,
    MemoryBudgetThresholds, MemoryHints, NoopBackendOptions, PowerPreference, Queue, RequestAdapterOptions, Trace,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{Icon, Window, WindowAttributes, WindowId};

use crate::graphics::*;
use crate::input::{InputEvent, InputReport, InputSystem};
use crate::interface::cursor::{MouseCursor, MouseCursorState};
use crate::interface::resource::{ItemSource, SkillSource};
use crate::interface::windows::*;
use crate::loaders::*;
#[cfg(feature = "debug")]
use crate::renderer::{AlignHorizontal, DebugMarkerRenderer};
use crate::renderer::{EffectRenderer, GameInterfaceRenderer};
use crate::settings::{
    GameSettingsPathExt, GraphicsSettings, IN_GAME_THEMES_PATH, LightingMode, MENU_THEMES_PATH, ServiceSettingsPathExt, WORLD_THEMES_PATH,
};
use crate::state::skills::{LearnedSkill, SkillTreeLayoutPathExt, bring_skill_to_level};
use crate::state::theme::{InterfaceTheme, InterfaceThemeType, WorldTheme};
use crate::state::{BufferedAction, SelectedServicePath};
use crate::system::{FrameTimers, GameTimer};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::*;

const CLIENT_NAME: &str = "Korangar";

/// Matches Hercules `AUTH_TIMEOUT` in `login.c` — the login-server auth token
/// is only valid for this long before character-server entry is refused.
const LOGIN_AUTH_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct MissingSkillAsset {
    pub skill_id: SkillId,
    pub skill_name: String,
    pub file_name: String,
    pub missing_sprite: bool,
    pub missing_actions: bool,
}

pub fn audit_skill_assets() -> Result<(usize, Vec<MissingSkillAsset>), String> {
    let game_file_loader = GameFileLoader::default();
    game_file_loader.load_archives_from_settings();
    game_file_loader.load_patched_lua_files();
    let library = Library::new(&game_file_loader).map_err(|error| error.to_string())?;

    let mut total = 0;
    let mut missing = Vec::new();
    for (skill_id, skill_name, file_name) in library.skill_asset_entries() {
        total += 1;
        // Native archive keys are normalized to lowercase. Runtime loads go
        // through `FileLoader::get`, which performs this normalization; the
        // audit uses the cheaper existence check directly and must match it.
        let (sprite_file_name, actions_file_name) = skill_asset_file_names(file_name);
        let sprite_path = format!("data\\sprite\\아이템\\{sprite_file_name}.spr").to_lowercase();
        let actions_path = format!("data\\sprite\\아이템\\{actions_file_name}.act").to_lowercase();
        let missing_sprite = !game_file_loader.file_exists(&sprite_path);
        let missing_actions = !game_file_loader.file_exists(&actions_path);
        if missing_sprite || missing_actions {
            missing.push(MissingSkillAsset {
                skill_id,
                skill_name: skill_name.to_owned(),
                file_name: file_name.to_owned(),
                missing_sprite,
                missing_actions,
            });
        }
    }

    missing.sort_by_key(|asset| asset.skill_id.0);
    Ok((total, missing))
}

#[derive(Debug)]
pub struct MissingEntitySprite {
    pub job_id: ragnarok_packets::JobId,
    pub folder: &'static str,
    pub sprite_name: String,
    pub missing_sprite: bool,
    pub missing_actions: bool,
}

pub fn audit_entity_sprites() -> Result<(usize, Vec<MissingEntitySprite>), String> {
    let game_file_loader = GameFileLoader::default();
    game_file_loader.load_archives_from_settings();
    game_file_loader.load_patched_lua_files();
    let library = Library::new(&game_file_loader).map_err(|error| error.to_string())?;

    let mut total = 0;
    let mut missing = Vec::new();
    for (job_id, sprite_name) in library.job_identity_entries() {
        let folder = match EntityType::from(job_id) {
            EntityType::Monster => "몬스터",
            EntityType::Npc => "npc",
            // Players compose body/head sprites through a different scheme,
            // and warp/hidden trigger entities render no sprite at all.
            _ => continue,
        };
        total += 1;
        let sprite_path = format!("data\\sprite\\{folder}\\{sprite_name}.spr").to_lowercase();
        let actions_path = format!("data\\sprite\\{folder}\\{sprite_name}.act").to_lowercase();
        let missing_sprite = !game_file_loader.file_exists(&sprite_path);
        let missing_actions = !game_file_loader.file_exists(&actions_path);
        if missing_sprite || missing_actions {
            missing.push(MissingEntitySprite {
                job_id,
                folder,
                sprite_name,
                missing_sprite,
                missing_actions,
            });
        }
    }

    missing.sort_by_key(|entry| entry.job_id.0);
    Ok((total, missing))
}

/// Everything the weapon-layer audit learned from the configured archives:
/// the actual weapon sprite files that ship under `인간족\`, and which of a
/// set of candidate weapon-name suffixes resolve for each player job folder.
pub struct WeaponSpriteAuditReport {
    /// All `data\sprite\인간족\...` SPR paths outside the body/head/shield
    /// folders, as reported by the archive file tables.
    pub discovered: Vec<String>,
    /// `(job folder, sex, found suffixes)` from direct existence probes.
    pub probed: Vec<(&'static str, &'static str, Vec<&'static str>)>,
    /// Per-item weapon sprites (`{job}_{sex}_{itemId}.spr`) from the archive
    /// listing (Phase D).
    pub per_item: Vec<String>,
    /// `_검광` sword-trail sprites from the archive listing (Phase D).
    pub trails: Vec<String>,
}

/// Candidate weapon sprite name suffixes to probe per job folder. Includes
/// every suffix the client currently uses plus plausible alternatives, so a
/// wrong table entry shows up as "candidate exists, table name doesn't".
const WEAPON_SUFFIX_CANDIDATES: &[&str] = &[
    "단검",
    "검",
    "검_검",
    "창",
    "창_창",
    "도끼",
    "도끼_도끼",
    "클럽",
    "클럽_클럽",
    "둔기",
    "메이스",
    "로드",
    "지팡이",
    "활",
    "너클",
    "악기",
    "채찍",
    "책",
    "카타르",
    "카타르_카타르",
    "권총",
    "리볼버",
    "라이플",
    "기관총",
    "개틀링",
    "샷건",
    "유탄발사기",
    "수리검",
    "풍마수리검",
    "단검_단검",
    "단검_검",
    "단검_도끼",
    "검_도끼",
    "양손검",
    "양손창",
    "양손도끼",
    "유탄발사기",
    "그레네이드",
    "그레네이드런처",
];

pub fn audit_weapon_sprites() -> Result<WeaponSpriteAuditReport, String> {
    let game_file_loader = GameFileLoader::default();
    game_file_loader.load_archives_from_settings();

    let discovered: Vec<String> = game_file_loader
        .get_files_with_extension(&[".spr"])
        .into_iter()
        .filter(|path| {
            path.starts_with("data\\sprite\\인간족\\")
                && !path.starts_with("data\\sprite\\인간족\\몸통")
                && !path.starts_with("data\\sprite\\인간족\\머리통")
                && !path.starts_with("data\\sprite\\인간족\\방패")
        })
        .collect();

    // Phase D inventory: numbered per-item bases and `_검광` trails.
    // Note: `get_files_with_extension` under-reports classic GRF folders; the
    // runtime still probes candidates with `file_exists`. This listing is a
    // coverage lower bound for the audit tool.
    let mut per_item = Vec::new();
    let mut trails = Vec::new();
    for path in &discovered {
        let lower = path.to_lowercase();
        if lower.contains("검광") || path.contains("검광") {
            trails.push(path.clone());
        }
        // `{folder}_{sex}_{digits}.spr` — digits-only suffix is a per-item model.
        if let Some(stem) = path.rsplit('\\').next() {
            let stem = stem.trim_end_matches(".spr").trim_end_matches(".SPR");
            if let Some((_, tail)) = stem.rsplit_once('_')
                && !tail.is_empty()
                && tail.chars().all(|c| c.is_ascii_digit())
            {
                per_item.push(path.clone());
            }
        }
    }
    per_item.sort();
    trails.sort();

    let mut folders: Vec<&'static str> = (0..=25)
        .chain(4001..=4108)
        .map(|job| crate::world::get_sprite_path_for_player_job(ragnarok_packets::JobId(job)))
        .filter(|folder| !folder.is_ascii())
        // The priest weapon sprites live under a folder that differs from the
        // body sprite table's name for job 8; probe it explicitly.
        .chain(["프리스트", "성직자"])
        .collect();
    folders.sort();
    folders.dedup();

    let mut probed = Vec::new();
    for folder in folders {
        for sex in ["남", "여"] {
            let found: Vec<&'static str> = WEAPON_SUFFIX_CANDIDATES
                .iter()
                .copied()
                .filter(|suffix| {
                    let path = format!("data\\sprite\\인간족\\{folder}\\{folder}_{sex}_{suffix}.spr").to_lowercase();
                    game_file_loader.file_exists(&path)
                })
                .collect();
            probed.push((folder, sex, found));
        }
    }

    Ok(WeaponSpriteAuditReport {
        discovered,
        probed,
        per_item,
        trails,
    })
}

/// Walk every player job's body / head / weapon ACT files and inventory their
/// per-action frame structure. The native actor clock is owned by the body's
/// motion count: a shorter non-empty secondary action falls back to motion 0,
/// while motions beyond the body's length are unreachable. This audit reports
/// every such mismatch, plus body attack frame counts and authored events.
pub fn audit_animation_structure() -> Result<Vec<String>, String> {
    use korangar_loaders::FileLoader;
    use ragnarok_bytes::{ByteReader, FromBytes};
    use ragnarok_formats::action::ActionsData;
    use ragnarok_formats::version::GenericFormatMetadata;

    let game_file_loader = GameFileLoader::default();
    game_file_loader.load_archives_from_settings();

    let parse = |path: String| -> Option<ActionsData> {
        let bytes = game_file_loader.get(&path).ok()?;
        let mut byte_reader: ByteReader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);
        ActionsData::from_bytes(&mut byte_reader).ok()
    };

    let mut folders: Vec<&'static str> = (0..=25)
        .chain(4001..=4108)
        .map(|job| crate::world::get_sprite_path_for_player_job(ragnarok_packets::JobId(job)))
        .filter(|folder| !folder.is_ascii())
        .collect();
    folders.sort();
    folders.dedup();

    let weapon_suffixes = [
        "단검",
        "검",
        "양손검",
        "창",
        "양손창",
        "도끼",
        "양손도끼",
        "클럽",
        "활",
        "너클",
        "악기",
        "채찍",
        "책",
        "카타르_카타르",
        "권총",
        "라이플",
        "기관총",
        "샷건",
        "수리검",
        "단검_단검",
        "검_검",
        "도끼_도끼",
        "단검_검",
        "단검_도끼",
        "검_도끼",
    ];

    let mut lines = Vec::new();
    for sex in ["남", "여"] {
        let Some(head) = parse(format!("data\\sprite\\인간족\\머리통\\{sex}\\1_{sex}.act")) else {
            continue;
        };

        for folder in &folders {
            let Some(body) = parse(format!("data\\sprite\\인간족\\몸통\\{sex}\\{folder}_{sex}.act")) else {
                continue;
            };

            let body_groups: Vec<usize> = body.actions.iter().step_by(8).map(|action| action.motions.len()).collect();
            lines.push(format!(
                "{folder} ({sex}): {} actions, groups {body_groups:?}",
                body.actions.len()
            ));

            let mut compare = |layer_name: &str, layer: &ActionsData| {
                if layer.actions.len() != body.actions.len() {
                    lines.push(format!(
                        "  MISMATCH {layer_name}: {} actions vs body {}",
                        layer.actions.len(),
                        body.actions.len()
                    ));
                }
                let mut mismatched_groups = Vec::new();
                for (action_index, (body_action, layer_action)) in body.actions.iter().zip(layer.actions.iter()).enumerate() {
                    if body_action.motions.len() != layer_action.motions.len() {
                        let group = action_index / 8;
                        if !mismatched_groups.contains(&group) {
                            mismatched_groups.push(group);
                        }
                    }
                }
                for group in mismatched_groups {
                    let counts = |data: &ActionsData| -> Vec<usize> {
                        data.actions[group * 8..(group * 8 + 8).min(data.actions.len())]
                            .iter()
                            .map(|action| action.motions.len())
                            .collect()
                    };
                    lines.push(format!(
                        "  MISMATCH {layer_name} group {group}: body {:?} vs layer {:?}",
                        counts(&body),
                        counts(layer)
                    ));
                }
            };

            compare("head", &head);

            for suffix in weapon_suffixes {
                let weapon_folder = *folder;
                let path = format!("data\\sprite\\인간족\\{weapon_folder}\\{weapon_folder}_{sex}_{suffix}.act");
                if let Some(weapon) = parse(path) {
                    compare(&format!("weapon {suffix}"), &weapon);
                }
            }

            // Sound-event placement in the three attack groups (direction 0).
            for (group, name) in [(5usize, "Attack1"), (10, "Attack2"), (11, "Attack3")] {
                let action_index = group * 8;
                if let Some(action) = body.actions.get(action_index) {
                    let events: Vec<String> = action
                        .motions
                        .iter()
                        .enumerate()
                        .filter_map(|(motion_index, motion)| {
                            motion
                                .event_id
                                .filter(|event_id| *event_id != -1)
                                .and_then(|event_id| body.events.get(event_id as usize))
                                .map(|event| format!("frame {motion_index}: {}", event.name))
                        })
                        .collect();
                    if !events.is_empty() {
                        lines.push(format!(
                            "  {name}: {} frames, events [{}]",
                            action.motions.len(),
                            events.join(", ")
                        ));
                    }
                }
            }
        }
    }

    Ok(lines)
}

const ROLLING_CUTTER_ID: SkillId = SkillId(2036);
const DEFAULT_MAP: &str = "geffen";
const START_CAMERA_FOCUS_POINT: Point3<f32> = Point3::new(600.0, 0.0, 240.0);
const DEFAULT_BACKGROUND_MUSIC: Option<&str> = Some("bgm\\01.mp3");
const MAIN_MENU_CLICK_SOUND_EFFECT: &str = "버튼소리.wav";
const ITEM_PICKUP_RANGE: AttackRange = AttackRange(1);

/// M1-008: per-hit STR effects at the struck entity, wired like the original
/// client (roBrowser's skill/effect tables were the reference — semantics
/// only). The pending-impact queue owns the target-phase delay, so offsets in
/// this table are relative to that due boundary. Cold Bolt's classic hit is
/// sound-only, its visual is the volley itself; Thunderstorm and Storm Gust
/// play their large STRs at the targeted ground (`GroundSkillEffect`), so
/// their per-hit part is small or nothing.
fn skill_hit_effects(skill_id: SkillId) -> Vec<(ResolvedEffect, Color, f32)> {
    skill_presentation_recipe(skill_id)
        .hit_effects
        .iter()
        .map(|track| (track.asset.resolve(), track.light_color, track.start_delay))
        .collect()
}

/// Classic per-hit sounds played at the struck entity. Every name here was
/// confirmed present in the configured GRFs by
/// `probes_classic_skill_sound_candidates`; classic sounds for Lightning
/// Bolt, Meteor, Lord of Vermilion, the Hunter traps, and several others
/// were not found under any known name and stay silent for now.
/// Current ground-cast mappings played at the packet's target position when
/// `ZC_NOTIFY_GROUNDSKILL` arrives. Asset and trigger evidence varies by
/// recipe; see the combat animation pipeline specification.
fn ground_skill_effect(skill_id: SkillId) -> Option<(ResolvedEffect, Color, f32)> {
    skill_presentation_recipe(skill_id)
        .ground_effect
        .map(|track| (track.asset.resolve(), track.light_color, track.start_delay))
}
// TODO: The number of point lights that can cast shadows should be configurable
// through the graphics settings. For now I just chose an arbitrary smaller
// number that should be playable on most devices.
const NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS: usize = 6;

const INITIAL_SCREEN_SIZE: ScreenSize = ScreenSize {
    width: 1280.0,
    height: 720.0,
};

const INITIAL_SCALING_FACTOR: Scaling = Scaling::new(1.0);
/// Uniform scale applied to runtime-spawned trap props. Map objects get an
/// explicit scale from the RSW; a runtime spawn has none, and unit scale draws
/// these models far larger than a trap should read. Dialled in live — the unit
/// table carries no scale for them.
const TRAP_PROP_SCALE: f32 = 0.25;
const FALLBACK_PACKET_VERSION: SupportedPacketVersion = SupportedPacketVersion::_20220406;

static ICON_DATA: &[u8] = include_bytes!("../archive/data/icon.png");

/// CTR+C was sent, and the client is supposed to close.
pub static SHUTDOWN_SIGNAL: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

#[cfg(feature = "debug")]
const DEBUG_WINDOWS: &[WindowClass] = &[
    WindowClass::CacheStatistics,
    WindowClass::ClientStateInspector,
    WindowClass::PacketInspector,
    WindowClass::Profiler,
    WindowClass::RenderOptions,
];

// Create the `threads` module.
#[cfg(feature = "debug")]
korangar_debug::create_profiler_threads!(threads, {
    Main,
    Loader,
});

pub fn init_tls_rand() {
    use std::random::*;
    let mut seed = [0; 32];
    DefaultRandomSource.fill_bytes(&mut seed);
    rand_aes::tls::rand_seed(seed.into());
}

fn initialize_shutdown_signal() {
    ctrlc::set_handler(|| {
        println!("CTRL-C received. Shutting down");
        SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}

#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn is_vmware_virtual_platform() -> bool {
    std::fs::read_to_string("/sys/class/dmi/id/product_name").is_ok_and(|product_name| product_name.to_lowercase().contains("vmware"))
}

fn create_window_attributes() -> WindowAttributes {
    let reader = ImageReader::with_format(Cursor::new(ICON_DATA), ImageFormat::Png);
    let image_buffer = reader.decode().unwrap().to_rgba8();
    let image_data = image_buffer.as_bytes().to_vec();

    assert_eq!(image_buffer.width(), image_buffer.height(), "icon must be square");
    let icon = Icon::from_rgba(image_data, image_buffer.width(), image_buffer.height()).unwrap();

    Window::default_attributes()
        .with_inner_size(LogicalSize {
            width: INITIAL_SCREEN_SIZE.width,
            height: INITIAL_SCREEN_SIZE.height,
        })
        .with_title(CLIENT_NAME)
        .with_window_icon(Some(icon))
        .with_visible(false)
}

async fn initialize_hardware_adapter(instance: &Instance, backends: Backends, compatible_surface: Option<&wgpu::Surface<'_>>) -> Adapter {
    if let Ok(desired_adapter_name) = std::env::var("WGPU_ADAPTER_NAME") {
        let desired_adapter_name = desired_adapter_name.to_lowercase();
        let adapters = instance.enumerate_adapters(backends).await;

        if let Some(adapter) = adapters.into_iter().find(|adapter| {
            adapter.get_info().name.to_lowercase().contains(&desired_adapter_name) && supports_surface(adapter, compatible_surface)
        }) {
            let adapter_info = adapter.get_info();

            if !is_software_adapter(&adapter_info) || allow_software_rendering() {
                return adapter;
            }

            panic!(
                "WGPU_ADAPTER_NAME matched software adapter '{}'. Unset WGPU_ADAPTER_NAME or set KORANGAR_ALLOW_SOFTWARE_RENDERING=1 to \
                 allow CPU rendering.",
                adapter_info.name
            );
        }

        panic!("WGPU_ADAPTER_NAME was set, but no matching graphics adapter was found");
    }

    let adapters = instance.enumerate_adapters(backends).await;
    if let Some(adapter) = adapters
        .iter()
        .filter(|adapter| supports_surface(adapter, compatible_surface))
        .filter(|adapter| !is_software_adapter(&adapter.get_info()))
        .max_by_key(|adapter| adapter_score(&adapter.get_info()))
        .cloned()
    {
        return adapter;
    }

    if allow_software_rendering() {
        return instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface,
            })
            .await
            .expect("failed to find any graphics adapter");
    }

    let available_adapters = adapters
        .iter()
        .map(|adapter| {
            let info = adapter.get_info();
            format!("{} ({:?}, {}, {})", info.name, info.device_type, info.backend, info.driver)
        })
        .collect::<Vec<_>>()
        .join(", ");

    panic!(
        "no hardware graphics adapter was found. Available adapters: [{}]. Make sure the VM exposes the GPU through Vulkan/OpenGL, or set \
         KORANGAR_ALLOW_SOFTWARE_RENDERING=1 to allow CPU rendering.",
        available_adapters
    );
}

fn supports_surface(adapter: &Adapter, compatible_surface: Option<&wgpu::Surface<'_>>) -> bool {
    let Some(surface) = compatible_surface else {
        return true;
    };

    adapter.is_surface_supported(surface) && !surface.get_capabilities(adapter).formats.is_empty()
}

fn allow_software_rendering() -> bool {
    std::env::var("KORANGAR_ALLOW_SOFTWARE_RENDERING").is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn is_software_adapter(adapter_info: &AdapterInfo) -> bool {
    let adapter_name = adapter_info.name.to_lowercase();

    adapter_info.device_type == DeviceType::Cpu
        || adapter_name.contains("llvmpipe")
        || adapter_name.contains("lavapipe")
        || adapter_name.contains("softpipe")
        || adapter_name.contains("swiftshader")
}

fn adapter_score(adapter_info: &AdapterInfo) -> u8 {
    match adapter_info.device_type {
        DeviceType::DiscreteGpu => 5,
        DeviceType::IntegratedGpu => 4,
        DeviceType::VirtualGpu => 3,
        DeviceType::Other => 2,
        DeviceType::Cpu => 0,
    }
}

/// Strip extensions / padding from a server or loader map name (`izlude_in.gat`
/// → `izlude_in`).
fn normalize_map_base_name(map_file_name: &str) -> String {
    map_file_name
        .split(['\\', '/'])
        .next_back()
        .unwrap_or(map_file_name)
        .trim()
        .trim_end_matches('\0')
        .trim_end_matches(".gat")
        .trim_end_matches(".GAT")
        .trim_end_matches(".rsw")
        .trim_end_matches(".RSW")
        .to_lowercase()
}

#[cfg(test)]
mod normalize_map_base_name_tests {
    use super::normalize_map_base_name;

    #[test]
    fn strips_gat_and_path() {
        assert_eq!(normalize_map_base_name("data\\izlude_in.gat"), "izlude_in");
        assert_eq!(normalize_map_base_name("prt_in.GAT"), "prt_in");
        assert_eq!(normalize_map_base_name("izlude"), "izlude");
        assert_eq!(normalize_map_base_name("maps/payon.rsw"), "payon");
    }
}

#[cfg(test)]
mod resolve_pending_cast_tests {
    use ragnarok_packets::{AttackRange, EntityId, ItemId, SkillType, TilePosition};

    use super::{PendingCastResolution, is_within_skill_range, resolve_pending_cast};
    use crate::graphics::PickerTarget;

    #[test]
    fn entity_targeted_casts_on_entity() {
        let id = EntityId(42);
        assert_eq!(
            resolve_pending_cast(SkillType::Attack, PickerTarget::Entity(id)),
            PendingCastResolution::CastEntity(id)
        );
        assert_eq!(
            resolve_pending_cast(SkillType::Support, PickerTarget::Entity(id)),
            PendingCastResolution::CastEntity(id)
        );
    }

    #[test]
    fn entity_targeted_on_empty_ground_fizzles() {
        // A stray click on terrain must not walk or waste the cast — it stays armed.
        assert_eq!(
            resolve_pending_cast(SkillType::Attack, PickerTarget::Tile { x: 5, y: 9 }),
            PendingCastResolution::Fizzle
        );
        assert_eq!(
            resolve_pending_cast(SkillType::Support, PickerTarget::Nothing),
            PendingCastResolution::Fizzle
        );
    }

    #[test]
    fn ground_targeted_casts_at_tile() {
        assert_eq!(
            resolve_pending_cast(SkillType::Ground, PickerTarget::Tile { x: 7, y: 3 }),
            PendingCastResolution::CastTile(TilePosition { x: 7, y: 3 })
        );
        assert_eq!(
            resolve_pending_cast(SkillType::Trap, PickerTarget::Tile { x: 1, y: 2 }),
            PendingCastResolution::CastTile(TilePosition { x: 1, y: 2 })
        );
    }

    #[test]
    fn ground_targeted_on_entity_uses_entity_tile() {
        let id = EntityId(7);
        assert_eq!(
            resolve_pending_cast(SkillType::Ground, PickerTarget::Entity(id)),
            PendingCastResolution::CastEntityTile(id)
        );
    }

    #[test]
    fn ground_targeted_on_nothing_fizzles() {
        assert_eq!(
            resolve_pending_cast(SkillType::Ground, PickerTarget::Nothing),
            PendingCastResolution::Fizzle
        );
    }

    #[test]
    fn passive_and_self_cast_never_target() {
        assert_eq!(
            resolve_pending_cast(SkillType::Passive, PickerTarget::Entity(EntityId(1))),
            PendingCastResolution::Fizzle
        );
        assert_eq!(
            resolve_pending_cast(SkillType::SelfCast, PickerTarget::Entity(EntityId(1))),
            PendingCastResolution::Fizzle
        );
    }

    #[test]
    fn skill_range_uses_the_same_chebyshev_distance_as_server_combat() {
        let player = TilePosition { x: 10, y: 10 };

        assert!(is_within_skill_range(player, TilePosition { x: 11, y: 11 }, AttackRange(1)));
        assert!(!is_within_skill_range(player, TilePosition { x: 12, y: 11 }, AttackRange(1)));
        assert!(is_within_skill_range(player, TilePosition { x: 17, y: 3 }, AttackRange(7)));
    }

    #[test]
    fn missing_skill_item_names_the_item_when_the_table_knows_it() {
        use super::format_missing_skill_item;

        let gemstone = ItemId(717);
        // Hercules leaves the count at 0 for most callers; 0 and 1 both mean one.
        assert_eq!(
            format_missing_skill_item(Some("Blue Gemstone"), gemstone, 0, false),
            "You need a Blue Gemstone to use this skill."
        );
        assert_eq!(
            format_missing_skill_item(Some("Blue Gemstone"), gemstone, 1, false),
            "You need a Blue Gemstone to use this skill."
        );
        assert_eq!(
            format_missing_skill_item(Some("Yellow Gemstone"), ItemId(715), 2, false),
            "You need 2x Yellow Gemstone to use this skill."
        );
    }

    #[test]
    fn missing_skill_item_falls_back_to_the_raw_id() {
        use super::format_missing_skill_item;

        // An item outside the tables (custom, or newer than them) still has to say
        // something actionable, and cause 72 is equipment rather than a consumable.
        assert_eq!(
            format_missing_skill_item(None, ItemId(99999), 0, false),
            "Missing required item (#99999)."
        );
        assert_eq!(
            format_missing_skill_item(None, ItemId(99999), 3, true),
            "Missing required equipment: 3x #99999."
        );
    }

    #[test]
    fn trade_rows_name_the_item_instead_of_its_id() {
        use super::trade_item_label;

        assert_eq!(trade_item_label(Some("Red Potion"), ItemId(501), 12, 0), "Red Potion x12");
        assert_eq!(trade_item_label(Some("Sword"), ItemId(1101), 1, 7), "+7 Sword x1");
        // Only an item the tables cannot name may show an id.
        assert_eq!(trade_item_label(None, ItemId(501), 12, 0), "item #501 x12");
    }
}

/// A skill armed for targeting. Pressing a targeted skill's hotbar key while
/// the cursor is not over a valid target arms it here; the next left-click
/// picks the target (RO's press-skill → reticle → click-target flow). Cancelled
/// by right-click or Escape.
#[derive(Clone, Debug)]
struct PendingSkill {
    skill_id: SkillId,
    skill_level: SkillLevel,
    skill_type: SkillType,
    attack_range: AttackRange,
    skill_name: String,
}

/// What a left-click resolves to while a skill is armed, given what the cursor
/// is over. Kept separate from the cast itself so the decision is pure and
/// testable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingCastResolution {
    /// Entity-targeted skill clicked on an entity → cast on it.
    CastEntity(EntityId),
    /// Ground-targeted skill clicked on a tile → cast there.
    CastTile(TilePosition),
    /// Ground-targeted skill clicked on an entity → cast on that entity's tile.
    CastEntityTile(EntityId),
    /// No valid target under the cursor → stay armed (fizzle). Matches the
    /// entity-targeted convention that a stray click doesn't waste the cast;
    /// explicit right-click / Escape is the only way to cancel.
    Fizzle,
}

/// Decide what a click does for an armed skill. Pure — the caller performs the
/// actual network cast (and, for [`PendingCastResolution::CastEntityTile`], the
/// entity → tile lookup) so this can be unit-tested without a live client.
fn resolve_pending_cast(skill_type: SkillType, target: PickerTarget) -> PendingCastResolution {
    match skill_type {
        SkillType::Attack | SkillType::Support => match target {
            PickerTarget::Entity(entity_id) => PendingCastResolution::CastEntity(entity_id),
            _ => PendingCastResolution::Fizzle,
        },
        SkillType::Ground | SkillType::Trap => match target {
            PickerTarget::Tile { x, y } => PendingCastResolution::CastTile(TilePosition { x, y }),
            PickerTarget::Entity(entity_id) => PendingCastResolution::CastEntityTile(entity_id),
            _ => PendingCastResolution::Fizzle,
        },
        SkillType::Passive | SkillType::SelfCast => PendingCastResolution::Fizzle,
    }
}

fn is_within_skill_range(player: TilePosition, target: TilePosition, attack_range: AttackRange) -> bool {
    player.x.abs_diff(target.x).max(player.y.abs_diff(target.y)) <= attack_range.0
}

/// Cast `pending` at whatever `target` currently resolves to. Returns `true` if
/// a cast was sent (the target is consumed and the skill should disarm),
/// `false` if it fizzled and the skill should stay armed. Shared by the
/// hover-then-press fast path and the armed click path so both agree. Takes the
/// two fields it needs by reference rather than `&mut self` so callers can hold
/// unrelated field borrows (e.g. the per-frame point-light borrow in the render
/// path).
fn cast_or_path_entity_skill<Callback: PacketCallback + Send>(
    networking_system: &mut NetworkingSystem<Callback>,
    state: &mut State<ClientState>,
    map: Option<&Map>,
    path_finder: &mut PathFinder,
    skill_id: SkillId,
    skill_level: SkillLevel,
    attack_range: AttackRange,
    entity_id: EntityId,
) {
    let player_position = state.try_follow(this_entity()).map(Entity::get_tile_position);
    let target_position = state
        .follow(client_state().entities())
        .iter()
        .find(|entity| entity.get_entity_id() == entity_id)
        .map(Entity::get_tile_position);

    if let (Some(map), Some(player_position), Some(target_position)) = (map, player_position, target_position)
        && !is_within_skill_range(player_position, target_position, attack_range)
    {
        match path_finder
            .find_walkable_path_in_range(map, player_position, target_position, attack_range)
            .and_then(|path| path.last().copied())
        {
            Some(nearest_tile) => {
                let _ = networking_system.player_move(WorldPosition {
                    x: nearest_tile.x,
                    y: nearest_tile.y,
                    direction: Direction::North,
                });
                *state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::CastSkill {
                    skill_id,
                    skill_level,
                    entity_id,
                    attack_range,
                });
            }
            // Same reasoning as the ground path: an unreachable target means the
            // cast can never land, and Hercules would drop it without a word.
            None => state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                "That target is out of range and there is no way to get closer.".to_owned(),
                MessageColor::Error,
            )),
        }
    } else {
        let _ = networking_system.cast_skill(skill_id, skill_level, entity_id);
    }
}

/// Cell a ground-targeted resolution lands on, or `None` when the click fizzled
/// (empty space, or an entity that has since despawned) and the skill should
/// stay armed. `CastEntity` is not a ground placement and also yields `None` —
/// callers handle that arm separately.
fn resolve_pending_ground_tile(state: &State<ClientState>, resolution: PendingCastResolution) -> Option<TilePosition> {
    match resolution {
        PendingCastResolution::CastTile(tile) => Some(tile),
        // A ground skill clicked on an entity centres the AoE on that entity's cell.
        PendingCastResolution::CastEntityTile(entity_id) => state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| entity.get_tile_position()),
        PendingCastResolution::CastEntity(_) | PendingCastResolution::Fizzle => None,
    }
}

/// Cast `skill_id` on the cell `tile`, walking into range first when it is out
/// of reach. Mirrors [`cast_or_path_entity_skill`] for ground placements.
///
/// Without this the cast was simply *lost*: Hercules drops an out-of-range
/// `CZ_USE_SKILL_TOGROUND` with a bare `return 0` and **no** `clif->skill_fail`
/// (`unit.c` `unit_skilluse_pos2`, the `battle->check_range` arm), so the player
/// pressed the key, spent nothing, and saw nothing at all. Hercules re-checks
/// the range from `unit_walk_toxy_timer` when the caster is already walking, so
/// approaching and re-issuing is what the server expects.
fn cast_or_path_ground_skill<Callback: PacketCallback + Send>(
    networking_system: &mut NetworkingSystem<Callback>,
    state: &mut State<ClientState>,
    map: Option<&Map>,
    path_finder: &mut PathFinder,
    skill_id: SkillId,
    skill_level: SkillLevel,
    attack_range: AttackRange,
    tile: TilePosition,
) {
    let player_position = state.try_follow(this_entity()).map(Entity::get_tile_position);

    if let (Some(map), Some(player_position)) = (map, player_position)
        && !is_within_skill_range(player_position, tile, attack_range)
    {
        match path_finder
            .find_walkable_path_in_range(map, player_position, tile, attack_range)
            .and_then(|path| path.last().copied())
        {
            Some(nearest_tile) => {
                let _ = networking_system.player_move(WorldPosition {
                    x: nearest_tile.x,
                    y: nearest_tile.y,
                    direction: Direction::North,
                });
                *state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::CastGroundSkill {
                    skill_id,
                    skill_level,
                    tile,
                    attack_range,
                });
            }
            // Nothing walkable gets us close enough, so the cast can never land.
            // Say so rather than sending a packet the server discards in silence.
            None => state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                "That spot is out of range and there is no way to get closer.".to_owned(),
                MessageColor::Error,
            )),
        }
        return;
    }

    let _ = networking_system.cast_ground_skill(skill_id, skill_level, tile);
}

/// Tell the player which skill is now armed and waiting for a target. The
/// targeting reticle alone doesn't say *which* skill it is (and skills
/// currently draw no cast effect), so this line is how arming — and a swap — is
/// visible. Takes the chat state by reference so it can be called inside the
/// input-drain loop, which holds a borrow of `self` that a `&mut self` method
/// would conflict with.
/// Name a skill's missing item requirement (`ZC_ACK_TOUSESKILL` cause 71/72).
///
/// The networking crate reports these as a bare id because it has no item DB;
/// resolving it here is what turns "Missing required item (#717)." into "You
/// need a Blue Gemstone to use this skill.".
fn missing_skill_item_text(library: &Library, item_id: ItemId, amount: u16, equipment: bool) -> String {
    // Requirement items are always named identified — the requirement is on the
    // item *kind*, and an unidentified name would not help the player find it.
    let name = resolve_item_name(library, item_id, true);

    format_missing_skill_item(name.as_deref(), item_id, amount, equipment)
}

/// Display name for an item, or `None` when the tables cannot name it.
///
/// `ItemName` has two distinct failure modes that both have to be caught before
/// showing text to the player: no entry at all, and an entry whose name is the
/// `NOTFOUND` sentinel. Callers decide what to say instead — never print the
/// sentinel, and prefer a description over a bare id.
fn resolve_item_name(library: &Library, item_id: ItemId, is_identified: bool) -> Option<String> {
    library
        .try_get::<ItemName>(ItemNameKey { item_id, is_identified })
        .map(ItemName::to_string)
        .filter(|name| name != "NOTFOUND" && !name.is_empty())
}

/// Row label for one stack in the trade window. Falls back to the id only when
/// the item tables cannot name the item at all.
pub fn trade_item_label(name: Option<&str>, item_id: ItemId, amount: u32, refine: u8) -> String {
    let name = match name {
        Some(name) => name.to_owned(),
        None => format!("item #{}", item_id.0),
    };
    match refine {
        0 => format!("{name} x{amount}"),
        refine => format!("+{refine} {name} x{amount}"),
    }
}

/// Pure text half of [`missing_skill_item_text`], so the wording is testable
/// without a loaded `Library`.
fn format_missing_skill_item(name: Option<&str>, item_id: ItemId, amount: u16, equipment: bool) -> String {
    let what = match equipment {
        true => "equipment",
        false => "item",
    };
    // Hercules sends the requirement count in `btype`, but leaves it 0 for most
    // callers; both 0 and 1 mean "one of these".
    match (name, amount) {
        (Some(name), amount) if amount > 1 => format!("You need {amount}x {name} to use this skill."),
        (Some(name), _) => format!("You need a {name} to use this skill."),
        // Not in the item tables (a custom or newer item) — the id is all we have.
        (None, amount) if amount > 1 => format!("Missing required {what}: {amount}x #{}.", item_id.0),
        (None, _) => format!("Missing required {what} (#{}).", item_id.0),
    }
}

/// Ask the server to abort our own in-progress cast, returning whether there was
/// one to abort — so the caller knows whether to fall through to the gesture's
/// normal meaning (open the menu, rotate the camera).
///
/// **Fork behaviour, not RO's.** Official Ragnarok has no player-initiated cast
/// cancel; this pairs with the `clif_parse_CancelCast` Hercules delta. Movement
/// deliberately does *not* cancel — casting still roots the character, which is
/// authentic.
///
/// The cast bar is left alone here: Hercules broadcasts `clif->skillcastcancel`,
/// which arrives back as `SkillCastCancelled` and clears it. Clearing
/// optimistically would show a cancel that might not have happened.
///
/// Takes its two fields by reference rather than `&mut self` because both call
/// sites already hold unrelated borrows of `self` (the input-event drain and the
/// per-frame point-light borrow).
fn cancel_own_cast<Callback: PacketCallback + Send>(
    networking_system: &mut NetworkingSystem<Callback>,
    state: &State<ClientState>,
    client_tick: ClientTick,
) -> bool {
    let is_casting = state
        .try_follow(this_entity())
        .is_some_and(|player| player.is_casting(client_tick));

    if is_casting {
        let _ = networking_system.cancel_cast();
    }
    is_casting
}

fn announce_armed_skill(state: &mut State<ClientState>, skill_name: &str) {
    state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
        format!("Aiming {skill_name} — click a target (right-click or Esc to cancel)."),
        MessageColor::Information,
    ));
}

pub struct Client {
    game_file_loader: Arc<GameFileLoader>,
    #[cfg(feature = "debug")]
    action_loader: Arc<ActionLoader>,
    #[cfg(feature = "debug")]
    animation_loader: Arc<AnimationLoader>,
    async_loader: Arc<AsyncLoader>,
    effect_loader: Arc<EffectLoader>,
    font_loader: Arc<FontLoader>,
    #[cfg(feature = "debug")]
    map_loader: Arc<MapLoader>,
    sprite_loader: Arc<SpriteLoader>,
    texture_loader: Arc<TextureLoader>,
    library: Arc<Library>,

    interface_renderer: InterfaceRenderer,
    bottom_interface_renderer: GameInterfaceRenderer,
    middle_interface_renderer: GameInterfaceRenderer,
    top_interface_renderer: GameInterfaceRenderer,
    effect_renderer: EffectRenderer,
    #[cfg(feature = "debug")]
    debug_marker_renderer: DebugMarkerRenderer,
    #[cfg(feature = "debug")]
    aabb_instructions: Vec<DebugAabbInstruction>,
    #[cfg(feature = "debug")]
    circle_instructions: Vec<DebugCircleInstruction>,
    #[cfg(feature = "debug")]
    rectangle_instructions: Vec<DebugRectangleInstruction>,
    model_batches: Vec<ModelBatch>,
    model_instructions: Vec<ModelInstruction>,
    entity_instructions: Vec<EntityInstruction>,
    directional_shadow_model_batches: [Vec<ModelBatch>; PARTITION_COUNT],
    directional_shadow_model_instructions: Vec<ModelInstruction>,
    directional_shadow_entity_instructions: [Vec<EntityInstruction>; PARTITION_COUNT],
    point_shadow_model_batches: Vec<ModelBatch>,
    point_shadow_model_instructions: Vec<ModelInstruction>,
    point_shadow_entity_instructions: Vec<EntityInstruction>,
    point_light_with_shadow_instructions: Vec<PointLightWithShadowInstruction>,
    point_light_instructions: Vec<PointLightInstruction>,

    input_system: InputSystem,

    interface: Interface<'static, ClientState>,
    mouse_cursor: MouseCursor,
    show_interface: bool,
    game_timer: GameTimer,

    #[cfg(feature = "debug")]
    debug_camera: DebugCamera,
    start_camera: StartCamera,
    player_camera: PlayerCamera,
    directional_shadow_camera: DirectionalShadowCamera,
    directional_shadow_partitions: Arc<Mutex<[DirectionalShadowPartition; PARTITION_COUNT]>>,
    point_shadow_camera: PointShadowCamera,

    input_event_buffer: Vec<InputEvent>,
    /// A targeted skill awaiting a click to pick its target. See
    /// [`PendingSkill`].
    pending_skill: Option<PendingSkill>,
    /// Tile texture for the ground-skill aiming footprint, loaded once at
    /// startup. `None` if the asset is missing — the footprint is then skipped
    /// rather than failing the frame.
    skill_footprint_texture: Option<Arc<Texture>>,
    pending_impacts: PendingImpactQueue,
    network_event_buffer: NetworkEventBuffer,
    // TODO: Move or remove this.
    saved_login_data: Option<LoginServerLoginData>,
    // TODO: Move or remove this.
    saved_character_server: Option<CharacterServerInformation>,
    // TODO: Move or remove this.
    saved_login_server_address: Option<SocketAddr>,
    // TODO: Move or remove this.
    saved_password: String,
    // TODO: Move or remove this.
    saved_username: String,
    // TODO: Move or remove this.
    saved_packet_version: SupportedPacketVersion,
    /// Wall-clock deadline for using the login auth token (Hercules AUTH_TIMEOUT).
    /// Only used when the player must pick among multiple character servers.
    login_auth_expires_at: Option<Instant>,

    particle_holder: ParticleHolder,
    emote_bubbles: EmoteBubbles,
    sprite_effects: SpriteEffects,
    point_light_manager: PointLightManager,
    effect_holder: EffectHolder,
    /// Which looping status STR each entity is currently showing, so an
    /// unchanged status re-sent by the server doesn't restart the animation.
    active_status_effects: HashMap<EntityId, &'static str>,
    /// Live Hunter-trap props, keyed by the unit's entity id. Geometry already
    /// lives in the map's buffer; this only holds where each one sits.
    active_trap_props: Vec<(EntityId, Arc<Model>, Transform)>,
    /// Where the live skill units are, for questions the packets cannot
    /// answer on their own (which elemental field the player is standing in).
    skill_unit_registry: SkillUnitRegistry,
    path_finder: PathFinder,

    point_light_set_buffer: ResourceSetBuffer<LightSourceKey>,
    directional_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    point_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    deferred_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    bounding_box_object_set_buffer: ResourceSetBuffer<ObjectKey>,

    #[cfg(feature = "debug")]
    pathing_texture_set: Arc<TextureSet>,
    #[cfg(feature = "debug")]
    tile_texture_set: Arc<TextureSet>,

    main_menu_click_sound_effect: SoundEffectKey,

    #[cfg(feature = "debug")]
    networking_system: NetworkingSystem<PacketHistoryCallback>,
    #[cfg(not(feature = "debug"))]
    networking_system: NetworkingSystem<NoPacketCallback>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    active_interface_settings: InterfaceSettings,
    active_graphics_settings: GraphicsSettings,
    graphics_engine: GraphicsEngine,
    queue: Queue,
    #[cfg(feature = "debug")]
    device: Device,
    window: Option<Arc<Window>>,

    map: Option<Arc<Map>>,
    /// Whether the current map is a town / safe map (no monsters); relaxes the
    /// battle-ready stance. Set on each map load from Towninfo.
    current_map_is_town: bool,
    client_state: State<ClientState>,
}

impl Client {
    fn add_str_skill_effect(
        &mut self,
        path: &'static str,
        position: Point3<f32>,
        point_light_id: PointLightId,
        light_color: Color,
        light_intensity: f32,
    ) {
        self.add_str_skill_effect_with_offset(
            path,
            position,
            Vector3::new(0.0, 0.0, 0.0),
            point_light_id,
            light_color,
            light_intensity,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn add_str_skill_effect_with_offset(
        &mut self,
        path: &'static str,
        position: Point3<f32>,
        effect_offset: Vector3<f32>,
        point_light_id: PointLightId,
        light_color: Color,
        light_intensity: f32,
    ) {
        match self.effect_loader.get_or_load(path, &self.texture_loader) {
            Ok(effect) => {
                let frame_timer = effect.new_frame_timer();
                self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                    effect,
                    frame_timer,
                    EffectCenter::Position(position),
                    effect_offset,
                    point_light_id,
                    Vector3::new(0.0, 6.0, 0.0),
                    light_color,
                    light_intensity,
                    false,
                    0.0,
                )));
            }
            Err(error) => eprintln!("[skill-effect] failed to load {path}: {error:?}"),
        }
    }

    fn play_spatial_skill_sound(&self, path: &'static str, position: Point3<f32>) {
        const SKILL_SOUND_RANGE: f32 = 250.0;

        let key = self.audio_engine.load(path);
        self.audio_engine.play_spatial_sound_effect(key, position, SKILL_SOUND_RANGE);
    }

    fn add_procedural_skill_effect(
        &mut self,
        texture_path: &'static str,
        position: Point3<f32>,
        point_light_id: PointLightId,
        style: SkillBurstStyle,
    ) {
        match self.texture_loader.get_or_load(texture_path, ImageType::Color) {
            Ok(texture) => self
                .effect_holder
                .add_effect(Box::new(SkillBurst::new(texture, position, style, point_light_id))),
            Err(error) => eprintln!("[skill-effect] failed to load {texture_path}: {error:?}"),
        }
    }

    fn add_layered_procedural_skill_effect(
        &mut self,
        texture_path: &'static str,
        secondary_texture_path: &'static str,
        position: Point3<f32>,
        point_light_id: PointLightId,
        style: SkillBurstStyle,
    ) {
        let primary = self.texture_loader.get_or_load(texture_path, ImageType::Color);
        let secondary = self.texture_loader.get_or_load(secondary_texture_path, ImageType::Color);
        match (primary, secondary) {
            (Ok(primary), Ok(secondary)) => self.effect_holder.add_effect(Box::new(
                SkillBurst::new(primary, position, style, point_light_id).with_secondary_texture(secondary),
            )),
            (Err(error), _) => eprintln!("[skill-effect] failed to load {texture_path}: {error:?}"),
            (_, Err(error)) => eprintln!("[skill-effect] failed to load {secondary_texture_path}: {error:?}"),
        }
    }

    fn add_spear_projectile(&mut self, source_position: Point3<f32>, target_position: Point3<f32>) {
        match self.sprite_loader.get_or_load("이팩트\\창.spr") {
            Ok(sprite) => match sprite.textures.first() {
                Some(texture) => self.effect_holder.add_effect(Box::new(SkillProjectile::spear(
                    texture.clone(),
                    source_position,
                    target_position,
                ))),
                None => eprintln!("[skill-effect] spear projectile sprite contains no frames"),
            },
            Err(error) => eprintln!("[skill-effect] failed to load spear projectile: {error:?}"),
        }
    }

    /// Spawn the flying projectile for a *normal* ranged attack (bow, firearm,
    /// huuma shuriken), matching the classic client, which draws the *ammo
    /// item's* sprite travelling shooter → target. No-op for melee weapons or
    /// when either position is unknown. The local player is `entities[0]`, so
    /// the entity lookup covers it too.
    fn spawn_ranged_attack_projectile(&mut self, source_entity_id: EntityId, destination_entity_id: EntityId) {
        let Some(weapon) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == source_entity_id)
            .map(|entity| entity.get_weapon())
        else {
            return;
        };
        let view = weapon_view_from_appearance(weapon);
        let Some(fallback_sprite) = ranged_attack_projectile_sprite(view) else {
            return;
        };
        let (Some(source), Some(target)) = (
            self.entity_world_position(source_entity_id),
            self.entity_world_position(destination_entity_id),
        ) else {
            return;
        };
        if source == target {
            return;
        }

        // Prefer the ammunition the shooter actually has loaded, so Iron Arrow,
        // Fire Arrow and the rest read differently in flight the way they do in
        // the original.
        //
        // For the local player the inventory is exact. For everyone else we rely on
        // the fork's `LOOK_AMMO` broadcast — official Ragnarok reports nobody else's
        // ammunition — which is `0` for an unarmed slot, an older server, or a
        // player who came into view before the server learned their ammo. The
        // weapon class default covers all of those.
        let is_local_player = self
            .client_state
            .try_follow(this_entity())
            .is_some_and(|player| player.get_entity_id() == source_entity_id);
        let ammunition = match is_local_player {
            true => self.client_state.follow(client_state().inventory()).equipped_ammunition(),
            // Not read from the entity: the ammunition broadcast can arrive before
            // the entity exists and survives it being replaced by a respawn packet.
            false => self
                .client_state
                .follow(client_state().remote_ammunition())
                .get(&AccountId(source_entity_id.0))
                .copied()
                .filter(|item_id| item_id.0 != 0),
        }
        .or_else(|| ranged_attack_default_ammunition(view));

        // `iteminfo` is authoritative wherever it names a specific sprite. It hands
        // back the generic arrow for most ammunition though — Iron Arrow included —
        // so an elemental arrow only gets its own sprite if we fill that gap.
        let ammunition_sprite = ammunition.and_then(|item_id| {
            let from_client = self
                .library
                .try_get::<ItemResource>(ItemResourceKey {
                    item_id,
                    is_identified: true,
                })
                .map(|resource| resource.to_string())
                .filter(|resource| resource != GENERIC_ARROW_RESOURCE);

            from_client
                .or_else(|| elemental_ammunition_resource(item_id).map(str::to_owned))
                .map(|resource| ammunition_projectile_sprite_path(&resource))
        });

        // An unknown or spriteless ammo item must not swallow the projectile, so
        // fall back to the weapon class default before giving up. The fallback is
        // silent on screen — every arrow type then flies as a plain arrow — so it
        // is worth being able to see which branch ran.
        let ammunition_texture = ammunition_sprite.as_deref().and_then(|path| self.first_sprite_frame(path));
        let used_fallback = ammunition_texture.is_none();
        let texture = ammunition_texture.or_else(|| self.first_sprite_frame(fallback_sprite));

        if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
            eprintln!(
                "[ranged-attack] view={view} local_player={is_local_player} ammo_item={ammunition:?} \
                 ammo_sprite={ammunition_sprite:?} used_fallback={used_fallback}"
            );
        }

        match texture {
            Some(texture) => {
                let mut projectile = SkillProjectile::arrow(texture, source, target);

                // Elemental ammo carries a glow in its element. The stock sprites are
                // only faint recolours, so this is what actually makes a Fire Arrow
                // read as fire in flight — and it reaches the elemental arrows that
                // ship no distinct sprite at all. Neutral ammo stays untouched.
                if let Some(element) = ammunition.and_then(ammunition_element) {
                    // Distinct light per in-flight arrow, so rapid fire does not
                    // collapse into one light being dragged between shots.
                    let light_id = PointLightId::new(
                        source.x.to_bits().wrapping_add(target.z.to_bits()).wrapping_mul(31) ^ ammunition.map_or(0, |id| id.0),
                    );
                    projectile = projectile.with_elemental_glow(element.glow_color(), light_id);
                }

                self.effect_holder.add_effect(Box::new(projectile));
            }
            None => eprintln!("[ranged-attack] no projectile sprite loaded for weapon view {view}"),
        }
    }

    /// First frame of a sprite, or `None` if it fails to load or has no frames.
    fn first_sprite_frame(&self, path: &str) -> Option<Arc<Texture>> {
        self.sprite_loader.get_or_load(path).ok()?.textures.first().cloned()
    }

    fn spawn_successful_caster_skill_effect(&mut self, skill_id: SkillId, source_entity_id: EntityId, position: Point3<f32>) {
        let recipe = skill_presentation_recipe(skill_id);
        if recipe.successful_caster_effect.is_none() && recipe.successful_caster_sounds.is_empty() {
            return;
        }
        if !self.effect_holder.claim_unique_skill_effect(source_entity_id, skill_id, 0.5) {
            return;
        }

        let point_light_id = PointLightId::new(source_entity_id.0 ^ u32::from(skill_id.0));
        match recipe.successful_caster_effect {
            // SM_MAGNUM — classic double expanding cylinder.
            Some(SuccessfulCasterEffect::MagnumBreak) => {
                self.add_layered_procedural_skill_effect(
                    "effect\\ring_yellow.tga",
                    "effect\\대폭발.tga",
                    position,
                    point_light_id,
                    SkillBurstStyle::MagnumBreak,
                );
                self.player_camera.shake(0.05);
            }
            // WZ_FROSTNOVA — one caster-centered freeze, including empty casts.
            Some(SuccessfulCasterEffect::FrostNova) => {
                self.add_str_skill_effect("freeze.str", position, point_light_id, Color::rgb_u8(145, 220, 255), 55.0)
            }
            // RG_RAID — classic blue radial lens streaks.
            Some(SuccessfulCasterEffect::Raid) => {
                self.add_procedural_skill_effect("effect\\lens1.tga", position, point_light_id, SkillBurstStyle::Raid)
            }
            // ASC_METEORASSAULT — eight expanding purple slashes.
            Some(SuccessfulCasterEffect::MeteorAssault) => self.add_procedural_skill_effect(
                "effect\\purpleslash.tga",
                position,
                point_light_id,
                SkillBurstStyle::MeteorAssault,
            ),
            // RK_IGNITIONBREAK — shipped classic STR.
            Some(SuccessfulCasterEffect::IgnitionBreak) => self.add_str_skill_effect(
                "이그니션브레이크.str",
                position,
                point_light_id,
                Color::rgb_u8(255, 105, 30),
                70.0,
            ),
            None => {}
        }

        for sound in recipe.successful_caster_sounds {
            self.play_spatial_skill_sound(sound.resolve(), position);
        }
    }

    fn spawn_damage_caster_skill_effect(
        &mut self,
        skill_id: SkillId,
        source_entity_id: EntityId,
        source_position: Point3<f32>,
        target_position: Point3<f32>,
        hit_count: usize,
        impact_delay_ms: u32,
        client_tick: ClientTick,
    ) {
        let recipe = skill_presentation_recipe(skill_id);
        let has_projectile = recipe.projectile.is_some();
        if recipe.damage_caster_effect.is_none() && !has_projectile && recipe.damage_caster_sounds.is_empty() {
            return;
        }

        // Independent once-per-cast slots (see UniqueEffectSlot):
        // - CasterLayer: multi-target Brandish/Pierce share one caster STR
        // - TravelProjectile: multi-hit Jupitel packets share one ball
        // FallingBolts stay unclaimed so rapid Fire Bolts still volley.
        let spawn_caster_layer = match recipe.damage_caster_effect {
            Some(_) => self.effect_holder.claim_unique_skill_effect(source_entity_id, skill_id, 0.5),
            None => false,
        };
        let once_per_cast_travel = matches!(
            recipe.projectile,
            Some(
                ProjectileRecipe::Spear
                    | ProjectileRecipe::TravelBall(_)
                    | ProjectileRecipe::JupitelBall
                    | ProjectileRecipe::SpriteTravel { .. }
            )
        );
        // Short gate: multi-hit packets arrive in the same few frames; recasts
        // after ~cast time must not be blocked for long.
        let spawn_travel = !once_per_cast_travel
            || self.effect_holder.claim_unique_skill_effect_slot(
                source_entity_id,
                skill_id,
                UniqueEffectSlot::TravelProjectile,
                0.22,
            );

        if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
            eprintln!(
                "[skill-effect] damage-caster skill={} source={} spawn_caster={spawn_caster_layer} \
                 spawn_travel={spawn_travel} impact_delay_ms={impact_delay_ms} \
                 source_pos={source_position:?} target_pos={target_position:?}",
                skill_id.0, source_entity_id.0
            );
        }

        let base_light_id = source_entity_id.0 ^ u32::from(skill_id.0);
        let source_light_id = PointLightId::new(base_light_id);
        let neutral = Color::rgb_u8(235, 220, 180);
        if spawn_caster_layer {
            match recipe.damage_caster_effect {
                Some(DamageCasterEffect::Pierce) => self.add_str_skill_effect_with_offset(
                    "pierce.str",
                    source_position,
                    Vector3::new(0.0, 3.0, 0.0),
                    source_light_id,
                    neutral,
                    35.0,
                ),
                Some(DamageCasterEffect::BrandishSpear) => {
                    self.add_str_skill_effect("brandish2.str", source_position, source_light_id, neutral, 40.0);
                }
                Some(DamageCasterEffect::SpearStab) => {
                    self.add_str_skill_effect_with_offset(
                        "spearstab.str",
                        source_position,
                        Vector3::new(0.0, 3.0, 0.0),
                        source_light_id,
                        neutral,
                        35.0,
                    );
                }
                Some(DamageCasterEffect::SpearBoomerang) => {
                    self.add_str_skill_effect_with_offset(
                        "spearboomerang.str",
                        source_position,
                        Vector3::new(0.0, 6.0, 0.0),
                        source_light_id,
                        neutral,
                        35.0,
                    );
                }
                Some(DamageCasterEffect::BowlingBash) => {
                    self.add_str_skill_effect_with_offset(
                        "bowling.str",
                        source_position,
                        Vector3::new(0.0, 6.0, 0.0),
                        source_light_id,
                        neutral,
                        40.0,
                    );
                }
                Some(DamageCasterEffect::SonicBlow) => {
                    self.add_procedural_skill_effect(
                        "effect\\ring2.bmp",
                        source_position,
                        source_light_id,
                        SkillBurstStyle::SonicBlow,
                    );
                }
                None => {}
            }
            for sound in recipe.damage_caster_sounds {
                self.play_spatial_skill_sound(sound.resolve(), source_position);
            }
        }

        let impact_delay_secs = impact_delay_ms as f32 / 1000.0;
        let mut travel_spawned = false;
        match recipe.projectile {
            Some(ProjectileRecipe::Spear) if spawn_travel => {
                self.add_spear_projectile(source_position, target_position);
                travel_spawned = true;
            }
            Some(ProjectileRecipe::TravelBall(kind)) if spawn_travel => {
                self.add_travel_ball_projectile(kind, source_position, target_position, impact_delay_secs);
                travel_spawned = true;
            }
            Some(ProjectileRecipe::JupitelBall) if spawn_travel => {
                self.add_jupitel_ball_projectile(source_position, target_position, impact_delay_secs);
                travel_spawned = true;
            }
            Some(ProjectileRecipe::SpriteTravel {
                path,
                multi_hit,
                trail_ghosts,
            }) if spawn_travel => {
                let count = if multi_hit { hit_count.max(1) } else { 1 };
                self.add_sprite_travel_projectile(
                    path,
                    source_position,
                    target_position,
                    count,
                    trail_ghosts,
                    impact_delay_secs,
                    client_tick,
                );
                travel_spawned = true;
            }
            // Per-packet volleys: never once-per-cast gated.
            Some(ProjectileRecipe::FallingBolts(frame_paths)) => {
                let textures: Vec<_> = frame_paths
                    .iter()
                    .filter_map(|path| self.texture_loader.get_or_load(path, ImageType::Color).ok())
                    .collect();
                if !textures.is_empty() {
                    self.effect_holder
                        .add_effect(Box::new(FallingBolts::new(textures, target_position, hit_count, Color::WHITE)));
                }
            }
            Some(
                ProjectileRecipe::Spear
                | ProjectileRecipe::TravelBall(_)
                | ProjectileRecipe::JupitelBall
                | ProjectileRecipe::SpriteTravel { .. },
            )
            | None => {}
        }
        // Launch sounds ride the once-per-cast travel gate so multi-hit
        // packets don't stack them.
        if travel_spawned {
            for sound in recipe.projectile_sounds {
                self.play_spatial_skill_sound(sound.resolve(), source_position);
            }
        }
    }

    fn add_travel_ball_projectile(
        &mut self,
        kind: TravelBallKind,
        source_position: Point3<f32>,
        target_position: Point3<f32>,
        impact_delay_secs: f32,
    ) {
        // Land near the impact due-tick so hit STR and ball arrival coincide.
        let duration = if impact_delay_secs > 0.05 {
            impact_delay_secs.clamp(0.12, 0.55)
        } else {
            kind.duration()
        };
        // Stable-ish id from endpoints so concurrent balls don't always share one light.
        let light_id = PointLightId::new(
            (source_position.x.to_bits())
                .wrapping_add(target_position.z.to_bits())
                .wrapping_mul(31)
                ^ duration.to_bits(),
        );
        let texture = match self.texture_loader.get_or_load(kind.texture_path(), ImageType::Color) {
            Ok(texture) => texture,
            Err(error) => {
                eprintln!("[skill-effect] failed to load travel ball {}: {error:?}", kind.texture_path());
                return;
            }
        };

        let make_head = || {
            SkillProjectile::travel_ball(
                texture.clone(),
                source_position,
                target_position,
                duration,
                kind.size(),
                kind.color(),
                light_id,
                kind.light_intensity(),
            )
        };

        self.effect_holder.add_effect(Box::new(make_head()));

        // Sideways axis of the shot, so shards spray across the flight path
        // rather than stacking into one thick line.
        let travel = target_position - source_position;
        let across = Vector3::new(-travel.z, 0.0, travel.x);
        let across = if across.magnitude2() > f32::EPSILON {
            across.normalize()
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };

        for &(lag, sideways, lift, size_scale, alpha_scale) in kind.trail_shards() {
            let offset = across * sideways + Vector3::new(0.0, lift, 0.0);
            self.effect_holder
                .add_effect(Box::new(make_head().with_trail(lag, offset, size_scale, alpha_scale)));
        }
    }

    /// WZ_JUPITEL — the original client's effect 93: an animated
    /// `thunder_ball_*` billboard over a `thunder_center` glow.
    fn add_jupitel_ball_projectile(&mut self, source_position: Point3<f32>, target_position: Point3<f32>, impact_delay_secs: f32) {
        let duration = if impact_delay_secs > 0.05 {
            impact_delay_secs.clamp(0.12, 0.55)
        } else {
            0.38
        };
        let core = match self.texture_loader.get_or_load(JUPITEL_BALL_CORE_TEXTURE, ImageType::Color) {
            Ok(core) => core,
            Err(error) => {
                eprintln!("[skill-effect] failed to load {JUPITEL_BALL_CORE_TEXTURE}: {error:?}");
                return;
            }
        };
        let frames: Vec<_> = JUPITEL_BALL_FRAMES
            .iter()
            .filter_map(|path| self.texture_loader.get_or_load(path, ImageType::Color).ok())
            .collect();
        let light_id = PointLightId::new(
            (source_position.x.to_bits())
                .wrapping_add(target_position.z.to_bits())
                .wrapping_mul(31)
                ^ duration.to_bits(),
        );
        self.effect_holder.add_effect(Box::new(SkillProjectile::jupitel_ball(
            frames,
            core,
            source_position,
            target_position,
            duration,
            light_id,
        )));
    }

    /// Spawn everything a persistent skill unit shows (Phase E2). The unit
    /// lives until `RemoveSkillUnit`; `EffectHolder::remove_unit` tears down
    /// every piece registered under its entity id.
    fn spawn_skill_unit(&mut self, entity_id: EntityId, unit_id: UnitId, position: Point3<f32>, client_tick: ClientTick) {
        // Track every unit, including ones with no presentation recipe — the
        // registry answers questions about game state, which does not depend
        // on whether we can draw the thing.
        self.skill_unit_registry.insert(entity_id, unit_id, position);

        // Hunter traps are RSM props rather than procedural bodies, so they take
        // the prop path instead of a presentation recipe. Their geometry is
        // already in the map's buffer from load, so this is only a placement.
        if let Some(model_file) = trap_model_file(unit_id) {
            match self.map.as_ref().and_then(|map| map.prop_model(model_file)) {
                Some(model) => {
                    let model = model.clone();
                    // Map objects carry an explicit scale from the RSW; a runtime
                    // spawn has none, and unit scale renders these models far
                    // larger than a trap should read on the ground. Dialled in
                    // live rather than recovered — there is no scale for these in
                    // the unit table.
                    let mut transform = Transform::position(position);
                    transform.scale = Vector3::new(TRAP_PROP_SCALE, TRAP_PROP_SCALE, TRAP_PROP_SCALE);
                    self.active_trap_props.push((entity_id, model, transform));
                }
                None => eprintln!("[skill-unit] trap prop {model_file} was not preloaded for this map"),
            }
            return;
        }

        let Some(presentation) = unit_presentation(unit_id) else {
            if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                eprintln!("[skill-unit] unmapped unit {unit_id:?} entity={}", entity_id.0);
            }
            return;
        };

        let (light_color, light_intensity) = presentation.light.unwrap_or((Color::WHITE, 0.0));
        let point_light_id = PointLightId::new(entity_id.0 ^ 0x5EED_0000);

        // Log *mapped* spawns too. Logging only the unmapped ones left no way
        // to tell "never spawned" from "spawned but invisible" when Land
        // Protector rendered nothing (2026-07-24).
        if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
            let body = match presentation.body.as_ref() {
                Some(UnitBody::Cylinders { .. }) => "cylinders",
                Some(UnitBody::IceHorns { .. }) => "ice-horns",
                Some(UnitBody::GroundQuad { .. }) => "ground-quad",
                Some(UnitBody::LoopingSprite { .. }) => "looping-sprite",
                None => "none",
            };
            eprintln!(
                "[skill-unit] spawn {unit_id:?} entity={} body={body} str={:?} at ({:.1},{:.1},{:.1})",
                entity_id.0,
                presentation.looping_str.or(presentation.intro_str),
                position.x,
                position.y,
                position.z
            );
        }

        if let Some(path) = presentation.intro_str {
            match self.effect_loader.get_or_load(path, &self.texture_loader) {
                Ok(effect) => {
                    let frame_timer = effect.new_frame_timer();
                    self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Position(position),
                        Vector3::new(0.0, 0.0, 0.0),
                        PointLightId::new(entity_id.0 ^ 0x5EED_0001),
                        Vector3::new(0.0, 6.0, 0.0),
                        light_color,
                        light_intensity,
                        false,
                        0.0,
                    )));
                }
                Err(error) => eprintln!("[skill-unit] intro STR {path} failed: {error:?}"),
            }
        }

        if let Some(path) = presentation.looping_str {
            match self.effect_loader.get_or_load(path, &self.texture_loader) {
                Ok(effect) => {
                    let frame_timer = effect.new_frame_timer();
                    self.effect_holder.add_unit(
                        Box::new(EffectWithLight::new(
                            effect,
                            frame_timer,
                            EffectCenter::Position(position),
                            Vector3::new(0.0, 0.0, 0.0),
                            point_light_id,
                            Vector3::new(0.0, 6.0, 0.0),
                            light_color,
                            light_intensity,
                            true,
                            0.0,
                        )),
                        entity_id,
                    );
                }
                Err(error) => eprintln!("[skill-unit] looping STR {path} failed: {error:?}"),
            }
        }

        match presentation.body {
            Some(UnitBody::Cylinders { texture, specs, color }) => {
                match self.texture_loader.get_or_load(texture, ImageType::Color) {
                    Ok(loaded) => self.effect_holder.add_unit(
                        Box::new(UnitCylinders::new(
                            loaded,
                            position,
                            specs,
                            color,
                            point_light_id,
                            light_color,
                            light_intensity,
                        )),
                        entity_id,
                    ),
                    Err(error) => eprintln!("[skill-unit] failed to load {texture}: {error:?}"),
                }
            }
            Some(UnitBody::IceHorns { texture }) => match self.texture_loader.get_or_load(texture, ImageType::Color) {
                Ok(loaded) => self
                    .effect_holder
                    .add_unit(Box::new(UnitIceHorns::new(loaded, position)), entity_id),
                Err(error) => eprintln!("[skill-unit] failed to load {texture}: {error:?}"),
            },
            Some(UnitBody::GroundQuad {
                texture,
                half_size,
                color,
                pulse,
            }) => match self.texture_loader.get_or_load(texture, ImageType::Color) {
                Ok(loaded) => {
                    if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                        eprintln!("[skill-unit] ground-quad texture {texture} loaded, half_size={half_size}");
                    }
                    self.effect_holder.add_unit(
                        Box::new(UnitGroundQuad::new(loaded, position, half_size, color, pulse)),
                        entity_id,
                    );
                    // The quad renders no light of its own; honor the
                    // recipe's declared glow with a companion light.
                    self.effect_holder.add_unit(
                        Box::new(UnitPointLight::new(position, point_light_id, light_color, light_intensity)),
                        entity_id,
                    );
                }
                Err(error) => eprintln!("[skill-unit] failed to load {texture}: {error:?}"),
            },
            Some(UnitBody::LoopingSprite { path, action_index, lift }) => {
                // Same sentinel routing as the one-shot sprite effects; a sheet
                // already in the cache comes back from the request immediately.
                if let Some(sentinel) = self.sprite_effects.request_slot(path)
                    && let Some(animation_data) =
                        self.async_loader
                            .request_animation_data_load(sentinel, EntityType::Npc, vec![path.to_string()])
                {
                    self.sprite_effects.set_animation_data(path, animation_data);
                }

                self.sprite_effects.spawn_unit(
                    path,
                    position + Vector3::new(0.0, lift, 0.0),
                    action_index,
                    client_tick,
                    entity_id,
                );
                // `SpriteEffects` has no light path; same companion light.
                self.effect_holder.add_unit(
                    Box::new(UnitPointLight::new(position, point_light_id, light_color, light_intensity)),
                    entity_id,
                );
            }
            None => {}
        }

        if let Some(sound) = presentation.sound {
            self.play_spatial_skill_sound(sound, position);
        }
    }

    /// Fly classic `이팩트\*.spr` sprites from caster to target. Multi-hit skills
    /// pack staggered copies so the last lands near the impact boundary.
    /// `trail_ghosts` adds dimmed duplicates close behind the lead sprite —
    /// the original Fire Ball is five low-alpha copies of the sheet flying as
    /// a comet trail.
    #[allow(clippy::too_many_arguments)]
    fn add_sprite_travel_projectile(
        &mut self,
        path: &'static str,
        source_position: Point3<f32>,
        target_position: Point3<f32>,
        orb_count: usize,
        trail_ghosts: u8,
        impact_delay_secs: f32,
        client_tick: ClientTick,
    ) {
        const BODY_LIFT: f32 = 7.0;

        let count = orb_count.max(1);
        let arrival_secs = if impact_delay_secs > 0.05 {
            impact_delay_secs.clamp(0.18, 0.85)
        } else {
            0.32 + (count.saturating_sub(1) as f32) * 0.11
        };
        let travel_secs = (arrival_secs * 0.55).clamp(0.16, 0.40);
        let stagger_secs = if count > 1 {
            ((arrival_secs - travel_secs) / (count - 1) as f32).max(0.0)
        } else {
            0.0
        };
        let travel_ms = (travel_secs * 1000.0).round() as u32;
        let from = source_position + Vector3::new(0.0, BODY_LIFT, 0.0);
        let to = target_position + Vector3::new(0.0, BODY_LIFT, 0.0);

        // Lazy-load the sheet (first cast may draw nothing until it lands).
        if let Some(sentinel) = self.sprite_effects.request_slot(path)
            && let Some(animation_data) =
                self.async_loader
                    .request_animation_data_load(sentinel, EntityType::Npc, vec![path.to_string()])
        {
            self.sprite_effects.set_animation_data(path, animation_data);
        }

        for index in 0..count {
            let delay_ms = (index as f32 * stagger_secs * 1000.0).round() as u32;
            self.sprite_effects
                .spawn_travel(path, from, to, 0, client_tick, travel_ms, delay_ms, 1.0);

            for ghost in 1..=u32::from(trail_ghosts) {
                let ghost_alpha = 0.45 * (1.0 - ghost as f32 / (u32::from(trail_ghosts) + 1) as f32);
                self.sprite_effects.spawn_travel(
                    path,
                    from,
                    to,
                    0,
                    client_tick,
                    travel_ms,
                    delay_ms + ghost * 40,
                    ghost_alpha,
                );
            }
        }
    }

    /// Attach, swap, or drop the looping visual for an entity's status effect.
    ///
    /// Driven off the same opt1/opt2 the tints use, so the two never disagree.
    /// Keyed per entity through `add_status_effect`, which replaces any existing
    /// status visual — a poisoned entity that then gets stunned shows only the
    /// stun, never both stacked.
    fn update_status_effect_visual(&mut self, entity_id: EntityId, body_state: u16, health_state: u16) {
        let Some(path) = status_effect_asset(body_state, health_state) else {
            self.effect_holder.remove_status_effect(entity_id);
            self.active_status_effects.remove(&entity_id);
            return;
        };

        // Re-spawning an identical loop every status packet would restart the
        // animation constantly — the server re-sends these on unrelated changes.
        if self.active_status_effects.get(&entity_id) == Some(&path) {
            return;
        }

        let position = self.entity_world_position(entity_id).unwrap_or(Point3::new(0.0, 0.0, 0.0));

        match self.effect_loader.get_or_load(path, &self.texture_loader) {
            Ok(effect) => {
                let frame_timer = effect.new_frame_timer();
                self.effect_holder.add_status_effect(
                    Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Entity(entity_id, position),
                        Vector3::new(0.0, 4.0, 0.0),
                        PointLightId::new(entity_id.0.wrapping_add(0x5747_0000)),
                        Vector3::new(0.0, 6.0, 0.0),
                        Color::rgb_u8(255, 255, 255),
                        0.0,
                        true,
                        0.0,
                    )),
                    entity_id,
                );
                self.active_status_effects.insert(entity_id, path);
            }
            Err(error) => eprintln!("[status-effect] {path} failed to load: {error:?}"),
        }
    }

    fn spawn_special_effect(&mut self, entity_id: EntityId, effect_id: ragnarok_packets::EffectId) {
        let Some(recipe) = special_effect_recipe(effect_id) else {
            if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                eprintln!("[skill-effect] unmapped special effect id={effect_id:?} entity={}", entity_id.0);
            }
            return;
        };

        let position = self
            .entity_world_position(entity_id)
            .or_else(|| {
                self.client_state
                    .try_follow(this_entity())
                    .filter(|entity| entity.get_entity_id() == entity_id || entity_id.0 == 0)
                    .map(Entity::get_position)
            })
            .unwrap_or(Point3::new(0.0, 0.0, 0.0));

        let point_light_id = PointLightId::new(entity_id.0.wrapping_add(0x01F3_0000));

        match recipe {
            SpecialEffectRecipe::Str {
                path,
                light_color,
                light_intensity,
            } => {
                // Prefer entity-attached center so the flash follows the actor.
                match self.effect_loader.get_or_load(path, &self.texture_loader) {
                    Ok(effect) => {
                        let frame_timer = effect.new_frame_timer();
                        self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                            effect,
                            frame_timer,
                            EffectCenter::Entity(entity_id, position),
                            Vector3::new(0.0, 4.0, 0.0),
                            point_light_id,
                            Vector3::new(0.0, 6.0, 0.0),
                            light_color,
                            light_intensity,
                            false,
                            0.0,
                        )));
                    }
                    Err(error) => eprintln!("[skill-effect] special-effect STR {path} failed: {error:?}"),
                }
            }
            SpecialEffectRecipe::Burst {
                style,
                texture,
                secondary,
            } => match secondary {
                Some(secondary) => self.add_layered_procedural_skill_effect(texture, secondary, position, point_light_id, style),
                None => self.add_procedural_skill_effect(texture, position, point_light_id, style),
            },
        }
    }

    /// Bespoke target tracks formerly mixed into the caster helper. These are
    /// invoked only after `PendingImpactQueue` reaches the native due tick.
    fn spawn_damage_target_skill_effect(
        &mut self,
        skill_id: SkillId,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        target_position: Point3<f32>,
    ) {
        let recipe = skill_presentation_recipe(skill_id);
        let target_light_id = PointLightId::new(destination_entity_id.0 ^ u32::from(skill_id.0));
        let neutral = Color::rgb_u8(235, 220, 180);

        // Large AOE rings: one geometry per cast (multi-target damage packets
        // would otherwise stack N rings on N mobs). Per-target hit STRs still
        // fire via hit_effects below the call site.
        let spawn_target_aoe = |holder: &mut EffectHolder| {
            holder.claim_unique_skill_effect_slot(source_entity_id, skill_id, UniqueEffectSlot::TargetAoe, 0.45)
        };

        match recipe.damage_target_effect {
            Some(DamageTargetEffect::BrandishSpear) => {
                self.add_str_skill_effect("brandish.str", target_position, target_light_id, neutral, 40.0)
            }
            Some(DamageTargetEffect::BowlingBash) => {
                self.add_layered_procedural_skill_effect(
                    "effect\\lens1.tga",
                    "effect\\lens2.tga",
                    target_position,
                    target_light_id,
                    SkillBurstStyle::MeleeHit,
                );
            }
            Some(DamageTargetEffect::SonicBlow) => {
                self.add_str_skill_effect(
                    "sonicblow.str",
                    target_position,
                    target_light_id,
                    Color::rgb_u8(220, 210, 255),
                    35.0,
                );
            }
            Some(DamageTargetEffect::NapalmBeat) => {
                if spawn_target_aoe(&mut self.effect_holder) {
                    // Original EF_NAPALMBEAT (32): clustered 폭발 explosion
                    // frames, over hit effect 1's converging lens streaks.
                    let primary = self.texture_loader.get_or_load(NAPALM_BEAT_TEXTURE, ImageType::Color);
                    let secondary = self.texture_loader.get_or_load(NAPALM_BEAT_TEXTURE_SECONDARY, ImageType::Color);
                    match (primary, secondary) {
                        (Ok(primary), Ok(secondary)) => {
                            let frames: Vec<_> = NAPALM_BEAT_EXPLOSION_FRAMES
                                .iter()
                                .filter_map(|path| self.texture_loader.get_or_load(path, ImageType::Color).ok())
                                .collect();
                            self.effect_holder.add_effect(Box::new(
                                SkillBurst::new(primary, target_position, SkillBurstStyle::NapalmBeat, target_light_id)
                                    .with_secondary_texture(secondary)
                                    .with_frames(frames),
                            ));
                        }
                        (Err(error), _) => eprintln!("[skill-effect] failed to load {NAPALM_BEAT_TEXTURE}: {error:?}"),
                        (_, Err(error)) => eprintln!("[skill-effect] failed to load {NAPALM_BEAT_TEXTURE_SECONDARY}: {error:?}"),
                    }
                }
            }
            Some(DamageTargetEffect::EarthSpike) => {
                // Single-target skill; no AOE gate. Stone horns + the original
                // eruption sound (effect 79 carries its own wav).
                self.add_procedural_skill_effect(
                    EARTH_SPIKE_TEXTURE,
                    target_position,
                    target_light_id,
                    SkillBurstStyle::EarthSpike,
                );
                self.play_spatial_skill_sound("effect\\wizard_earthspike.wav", target_position);
            }
            Some(DamageTargetEffect::HeavensDrive) => {
                if spawn_target_aoe(&mut self.effect_holder) {
                    self.add_procedural_skill_effect(
                        EARTH_SPIKE_TEXTURE,
                        target_position,
                        target_light_id,
                        SkillBurstStyle::HeavensDrive,
                    );
                    // Inside the gate so multi-target packets play it once.
                    self.play_spatial_skill_sound("effect\\wizard_earthspike.wav", target_position);
                }
            }
            Some(DamageTargetEffect::JupitelHit) => {
                // Original effect 94: thunder_pang flash + plasma-blast frames.
                match self.texture_loader.get_or_load(JUPITEL_HIT_PANG_TEXTURE, ImageType::Color) {
                    Ok(pang) => {
                        let frames: Vec<_> = JUPITEL_HIT_FRAMES
                            .iter()
                            .filter_map(|path| self.texture_loader.get_or_load(path, ImageType::Color).ok())
                            .collect();
                        self.effect_holder.add_effect(Box::new(
                            SkillBurst::new(pang, target_position, SkillBurstStyle::JupitelHit, target_light_id).with_frames(frames),
                        ));
                    }
                    Err(error) => eprintln!("[skill-effect] failed to load {JUPITEL_HIT_PANG_TEXTURE}: {error:?}"),
                }
            }
            None => {}
        }

        for sound in recipe.damage_target_sounds {
            self.play_spatial_skill_sound(sound.resolve(), target_position);
        }
    }

    pub fn init(sync_cache: bool, event_loop: Option<&EventLoop<()>>) -> Option<Self> {
        // We start a frame so that functions trying to start a measurement don't panic.
        #[cfg(feature = "debug")]
        let _measurement = threads::Main::start_frame();

        initialize_shutdown_signal();

        time_phase!("create global thread pool", {
            rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .start_handler(|_| init_tls_rand())
                .build_global()
                .unwrap();
        });

        time_phase!("seed main random instance", {
            init_tls_rand();
        });

        // Check if korangar is in the correct working directory and if not, try to
        // correct it.
        // NOTE: This check might be temporary or feature gated in the future.
        time_phase!("adjust working directory", {
            if !std::fs::metadata("archive").is_ok_and(|metadata| metadata.is_dir()) {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to find archive directory, attempting to change working directory {}",
                    "warning".yellow(),
                    "korangar".magenta()
                );

                if let Err(_error) = std::env::set_current_dir("korangar") {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to change working directory: {:?}", "error".red(), _error);
                }
            }
        });

        time_phase!("load graphics settings", {
            let picker_value = Arc::new(AtomicU64::new(0));
            let directional_shadow_partitions = Arc::new(Mutex::new([DirectionalShadowPartition::default(); PARTITION_COUNT]));
            let input_system = InputSystem::new(picker_value.clone());
            let graphics_settings = GraphicsSettings::new();
        });

        time_phase!("create initial window", {
            #[allow(deprecated)]
            let window = event_loop.map(|event_loop| Arc::new(event_loop.create_window(create_window_attributes()).unwrap()));
        });

        time_phase!("create adapter", {
            let backends = Backends::all().with_env();
            let instance = Instance::new(InstanceDescriptor {
                backends,
                flags: InstanceFlags::from_build_config().with_env(),
                memory_budget_thresholds: MemoryBudgetThresholds::default(),
                backend_options: BackendOptions {
                    gl: GlBackendOptions {
                        gles_minor_version: Gles3MinorVersion::Automatic,
                        fence_behavior: GlFenceBehavior::Normal,
                        debug_fns: GlDebugFns::Auto,
                    },
                    dx12: Dx12BackendOptions {
                        shader_compiler: Dx12Compiler::StaticDxc.with_env(),
                        presentation_system: Dx12SwapchainKind::DxgiFromHwnd,
                        latency_waitable_object: Dx12UseFrameLatencyWaitableObject::Wait,
                        force_shader_model: ForceShaderModelToken::default(),
                        agility_sdk: None,
                    },
                    noop: NoopBackendOptions { enable: false },
                },
                // Required by the GL backend to create a presentable EGL
                // context; unused on Vulkan, Metal and Dx12.
                display: event_loop.map(|event_loop| Box::new(event_loop.owned_display_handle()) as Box<dyn WgpuHasDisplayHandle>),
            });

            let compatible_surface = window.as_ref().map(|window| instance.create_surface(window.clone()).unwrap());
            let adapter = pollster::block_on(async { initialize_hardware_adapter(&instance, backends, compatible_surface.as_ref()).await });

            #[cfg(feature = "debug")]
            {
                let adapter_info = adapter.get_info();
                print_debug!("using adapter {} ({})", adapter_info.name, adapter_info.backend);
                print_debug!("using device {} ({})", adapter_info.device, adapter_info.vendor);
                print_debug!("using driver {} ({})", adapter_info.driver, adapter_info.driver_info);
            }
        });

        time_phase!("create device", {
            let capabilities = Capabilities::from_adapter(&adapter);

            let (device, queue) = pollster::block_on(async {
                adapter
                    .request_device(&DeviceDescriptor {
                        label: None,
                        required_features: capabilities.get_required_features(),
                        required_limits: capabilities.get_required_limits(),
                        experimental_features: ExperimentalFeatures::disabled(),
                        memory_hints: MemoryHints::Performance,
                        trace: Trace::Off,
                    })
                    .await
                    .unwrap()
            });

            #[cfg(feature = "debug")]
            device.on_uncaptured_error(Arc::new(error_handler));

            #[cfg(feature = "debug")]
            print_debug!("received {} and {}", "queue".magenta(), "device".magenta());
        });

        time_phase!("create shader compiler", {
            let shader_compiler = ShaderCompiler::new(device.clone());
        });

        time_phase!("create game file loader", {
            let game_file_loader = Arc::new(GameFileLoader::default());

            game_file_loader.load_archives_from_settings();
            game_file_loader.load_patched_lua_files();
        });

        time_phase!("calculate game file hash", {
            let game_file_hash = game_file_loader.calculate_hash();
            #[cfg(feature = "debug")]
            print_debug!("game file hash: {}", game_file_hash);
        });

        time_phase!("create audio engine", {
            let audio_engine = Arc::new(AudioEngine::new(game_file_loader.clone()));
            audio_engine.set_background_music_volume(0.1);
        });

        time_phase!("create resource managers", {
            std::fs::create_dir_all(MENU_THEMES_PATH).unwrap();
            std::fs::create_dir_all(IN_GAME_THEMES_PATH).unwrap();
            std::fs::create_dir_all(WORLD_THEMES_PATH).unwrap();

            let model_loader = Arc::new(ModelLoader::new(game_file_loader.clone(), capabilities.bindless_support()));
            let texture_loader = Arc::new(TextureLoader::new(
                device.clone(),
                queue.clone(),
                &shader_compiler,
                &capabilities,
                game_file_loader.clone(),
            ));
            let video_loader = Arc::new(VideoLoader::new(game_file_loader.clone(), texture_loader.clone()));
            let font_loader = Arc::new(FontLoader::new(
                &["NotoSans".to_owned(), "NotoSansKR".to_owned()],
                &game_file_loader,
                &texture_loader,
            ));
            let map_loader = Arc::new(MapLoader::new(
                device.clone(),
                queue.clone(),
                game_file_loader.clone(),
                audio_engine.clone(),
                capabilities.bindless_support(),
            ));
            let sprite_loader = Arc::new(SpriteLoader::new(game_file_loader.clone(), texture_loader.clone()));
            let action_loader = Arc::new(ActionLoader::new(game_file_loader.clone(), audio_engine.clone()));
            let effect_loader = Arc::new(EffectLoader::new(game_file_loader.clone()));
            let animation_loader = Arc::new(AnimationLoader::new());

            let library = Arc::new(Library::new(&game_file_loader).unwrap_or_else(|_| {
                // The library not being created correctly means that the lua files were
                // not valid. It's possible that the archive was copied from a
                // different machine with a different architecture, so the one thing
                // we can try is generating it again.

                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to execute lua files; attempting to fix it by re-patching",
                    "error".red()
                );

                game_file_loader.remove_patched_lua_files();
                game_file_loader.load_patched_lua_files();

                Library::new(&game_file_loader).unwrap()
            }));

            if sync_cache {
                sync_cache_archive(&game_file_loader, texture_loader, game_file_hash);
                return None;
            }

            game_file_loader.load_cache_archive(game_file_hash);

            let async_loader = Arc::new(AsyncLoader::new(
                action_loader.clone(),
                animation_loader.clone(),
                map_loader.clone(),
                model_loader.clone(),
                sprite_loader.clone(),
                texture_loader.clone(),
                video_loader.clone(),
                library.clone(),
            ));

            let interface_renderer = InterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                font_loader.clone(),
                &texture_loader,
                graphics_settings.high_quality_interface,
            );
            let bottom_interface_renderer = GameInterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                INITIAL_SCALING_FACTOR,
                font_loader.clone(),
                #[cfg(feature = "debug")]
                &texture_loader,
            );
            let middle_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let top_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let effect_renderer = EffectRenderer::new(INITIAL_SCREEN_SIZE);
            #[cfg(feature = "debug")]
            let debug_marker_renderer = DebugMarkerRenderer::new();

            #[cfg(feature = "debug")]
            let aabb_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let circle_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let rectangle_instructions = Vec::default();
            let model_batches = Vec::default();
            let model_instructions = Vec::default();
            let entity_instructions = Vec::default();
            let directional_shadow_model_batches = Default::default();
            let directional_shadow_model_instructions = Vec::default();
            let directional_shadow_entity_instructions = Default::default();
            let point_shadow_model_batches = Vec::default();
            let point_shadow_model_instructions = Vec::default();
            let point_shadow_entity_instructions = Vec::default();
            let point_light_with_shadow_instructions = Vec::default();
            let point_light_instructions = Vec::default();
        });

        time_phase!("create graphics engine", {
            let graphics_engine = GraphicsEngine::initialize(GraphicsEngineDescriptor {
                capabilities,
                adapter,
                instance,
                device: device.clone(),
                queue: queue.clone(),
                shader_compiler,
                texture_loader: texture_loader.clone(),
                picker_value,
                directional_shadow_partitions: directional_shadow_partitions.clone(),
            });

            if let Some(window) = window.as_ref() {
                let backend_name = graphics_engine.get_backend_name();
                window.set_title(&format!("{CLIENT_NAME} ({})", str::to_uppercase(&backend_name)));
                window.set_cursor_visible(false);
            }
        });

        time_phase!("initialize interface", {
            let mut interface = Interface::new(font_loader.clone(), INITIAL_SCREEN_SIZE);
            let mouse_cursor = MouseCursor::new(&sprite_loader, &action_loader);
            let show_interface = true;

            // The per-cell tile the aiming footprint is drawn with. Same texture
            // the original uses for Land Protector's ground tiles, which is what
            // a "this cell is affected" marker looks like in RO.
            let skill_footprint_texture = texture_loader.get_or_load(SKILL_FOOTPRINT_TEXTURE, ImageType::Color).ok();
        });

        time_phase!("initialize timer", {
            let game_timer = GameTimer::new();
        });

        time_phase!("initialize camera", {
            #[cfg(feature = "debug")]
            let debug_camera = DebugCamera::new();
            let mut start_camera = StartCamera::new();
            let player_camera = PlayerCamera::new();
            let mut directional_shadow_camera = DirectionalShadowCamera::new();
            let point_shadow_camera = PointShadowCamera::new();
            start_camera.set_focus_point(START_CAMERA_FOCUS_POINT);
        });

        // TODO: Move all of these to the ClientState
        let saved_login_data: Option<LoginServerLoginData> = None;
        let saved_character_server: Option<CharacterServerInformation> = None;
        let saved_login_server_address = None;
        let saved_password = String::new();
        let saved_username = String::new();
        let saved_packet_version = FALLBACK_PACKET_VERSION;

        time_phase!("initialize networking", {
            #[cfg(not(feature = "debug"))]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn();

            #[cfg(feature = "debug")]
            let (packet_history, packet_history_callback) = PacketHistory::new();
            #[cfg(feature = "debug")]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn_with_callback(packet_history_callback);
        });

        time_phase!("create resources", {
            let input_event_buffer = Vec::new();

            let particle_holder = ParticleHolder::default();
            let emote_bubbles = EmoteBubbles::default();
            let sprite_effects = SpriteEffects::default();
            let point_light_manager = PointLightManager::new();
            let effect_holder = EffectHolder::default();
            let skill_unit_registry = SkillUnitRegistry::default();
            let path_finder = PathFinder::default();

            let point_light_set_buffer = ResourceSetBuffer::default();
            let directional_shadow_object_set_buffer = ResourceSetBuffer::default();
            let point_shadow_object_set_buffer = ResourceSetBuffer::default();
            let deferred_object_set_buffer = ResourceSetBuffer::default();
            #[cfg(feature = "debug")]
            let bounding_box_object_set_buffer = ResourceSetBuffer::default();

            #[cfg(feature = "debug")]
            let pathing_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), video_loader.clone(), "pathing", &[
                "pathing_goal.png",
                "pathing_straight.png",
                "pathing_diagonal.png",
            ]);
            #[cfg(feature = "debug")]
            let pathing_texture_set = Arc::new(pathing_texture_set);

            #[cfg(feature = "debug")]
            let tile_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), video_loader.clone(), "tile", &[
                "tile_0.png",
                "tile_1.png",
                "tile_2.png",
                "tile_3.png",
                "tile_4.png",
                "tile_5.png",
                "tile_6.png",
            ]);
            #[cfg(feature = "debug")]
            let tile_texture_set = Arc::new(tile_texture_set);

            let main_menu_click_sound_effect = audio_engine.load(MAIN_MENU_CLICK_SOUND_EFFECT);
        });

        time_phase!("load default map", {
            let map = map_loader
                .load(
                    DEFAULT_MAP.to_string(),
                    &model_loader,
                    texture_loader.clone(),
                    video_loader,
                    &library,
                )
                .expect("failed to load initial map");

            directional_shadow_camera.set_level_bound(map.get_level_bound());

            audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);
            map.set_ambient_sound_sources(&audio_engine);
        });

        time_phase!("create client state", {
            let client_state = State::new(ClientState::new(
                &game_file_loader,
                graphics_settings.clone(),
                #[cfg(feature = "debug")]
                packet_history,
            ));
        });

        let active_interface_settings = client_state.follow(crate::client_state().interface_settings()).clone();

        interface.open_window(LoginWindow::new(
            crate::client_state().login_window(),
            crate::client_state().login_settings(),
            crate::client_state().client_info(),
        ));

        Some(Self {
            game_file_loader,
            #[cfg(feature = "debug")]
            action_loader,
            #[cfg(feature = "debug")]
            animation_loader,
            async_loader,
            effect_loader,
            font_loader,
            #[cfg(feature = "debug")]
            map_loader,
            sprite_loader,
            texture_loader,
            library,
            interface_renderer,
            bottom_interface_renderer,
            middle_interface_renderer,
            top_interface_renderer,
            effect_renderer,
            #[cfg(feature = "debug")]
            debug_marker_renderer,
            #[cfg(feature = "debug")]
            aabb_instructions,
            #[cfg(feature = "debug")]
            circle_instructions,
            #[cfg(feature = "debug")]
            rectangle_instructions,
            model_batches,
            model_instructions,
            entity_instructions,
            directional_shadow_model_batches,
            directional_shadow_model_instructions,
            directional_shadow_entity_instructions,
            point_shadow_model_batches,
            point_shadow_model_instructions,
            point_shadow_entity_instructions,
            point_light_with_shadow_instructions,
            point_light_instructions,
            input_system,
            interface,
            mouse_cursor,
            show_interface,
            game_timer,
            #[cfg(feature = "debug")]
            debug_camera,
            start_camera,
            player_camera,
            directional_shadow_camera,
            directional_shadow_partitions,
            point_shadow_camera,
            input_event_buffer,
            pending_skill: None,
            skill_footprint_texture,
            pending_impacts: PendingImpactQueue::default(),
            network_event_buffer,
            saved_login_data,
            saved_character_server,
            saved_login_server_address,
            saved_password,
            saved_username,
            saved_packet_version,
            login_auth_expires_at: None,
            particle_holder,
            emote_bubbles,
            sprite_effects,
            point_light_manager,
            effect_holder,
            active_status_effects: HashMap::new(),
            active_trap_props: Vec::new(),
            skill_unit_registry,
            path_finder,
            point_light_set_buffer,
            directional_shadow_object_set_buffer,
            point_shadow_object_set_buffer,
            deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            bounding_box_object_set_buffer,
            #[cfg(feature = "debug")]
            pathing_texture_set,
            #[cfg(feature = "debug")]
            tile_texture_set,
            main_menu_click_sound_effect,
            networking_system,
            audio_engine,
            active_interface_settings,
            active_graphics_settings: graphics_settings,
            graphics_engine,
            queue,
            #[cfg(feature = "debug")]
            device,
            window,

            map: Some(map),
            current_map_is_town: false,
            client_state,
        })
    }

    pub fn create_event_loop() -> EventLoop<()> {
        let mut event_loop_builder = EventLoop::builder();

        #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
        if is_vmware_virtual_platform() && std::env::var_os("DISPLAY").is_some() {
            event_loop_builder.with_x11();
        }

        event_loop_builder.build().unwrap()
    }

    pub fn run(&mut self, event_loop: EventLoop<()>) {
        event_loop.set_control_flow(ControlFlow::Poll);
        let _ = event_loop.run_app(self);
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn clear_render_instructions(&mut self) {
        self.interface_renderer.clear();
        self.bottom_interface_renderer.clear();
        self.middle_interface_renderer.clear();
        self.top_interface_renderer.clear();
        self.effect_renderer.clear();
        #[cfg(feature = "debug")]
        self.debug_marker_renderer.clear();

        #[cfg(feature = "debug")]
        self.aabb_instructions.clear();
        #[cfg(feature = "debug")]
        self.circle_instructions.clear();
        #[cfg(feature = "debug")]
        self.rectangle_instructions.clear();
        self.model_batches.clear();
        self.model_instructions.clear();
        self.entity_instructions.clear();
        self.directional_shadow_model_batches.iter_mut().for_each(|batch| batch.clear());
        self.directional_shadow_model_instructions.clear();
        self.directional_shadow_entity_instructions
            .iter_mut()
            .for_each(|instructions| instructions.clear());
        self.point_shadow_model_batches.clear();
        self.point_shadow_model_instructions.clear();
        self.point_shadow_entity_instructions.clear();
        self.point_light_with_shadow_instructions.clear();
        self.point_light_instructions.clear();
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_client_state(&mut self) {
        // Unset the highlighted skill just before applying. That way, if the skill is
        // still hovered the value will be the same as before, if not it will clear the
        // highlighted.
        *self.client_state.follow_mut(client_state().skill_tree_window().highlighted_skill()) = None;

        // Tick status effect timers so tiles expire.
        self.client_state
            .follow_mut(client_state().status_effects())
            .tick(std::time::Instant::now());

        // Apply the game state after all the UI work + rendering is done.
        if let Err(_errors) = self.client_state.apply() {
            #[cfg(feature = "debug")]
            {
                print_debug!("[{}] failed to apply {} updates: ", "error".red(), _errors.len());
                _errors.into_iter().for_each(|error| print_debug!("path: {}", error.type_name));
            }
        }
    }

    /// Apply any graphics or interface setting changes that the user
    /// dispatched during the previous frame.
    ///
    /// May reconfigure the GPU surface (MSAA, SSAA, present mode, shadow
    /// resolution, etc.). Surface reconfiguration is only safe between
    /// presenting the previous frame and acquiring the next swapchain image,
    /// so this *must* be called after [`Self::update_client_state`] (so the
    /// user's dispatched changes are visible) and *before*
    /// `graphics_engine.wait_for_next_frame()`. Calling it after
    /// `wait_for_next_frame` has been observed to cause surface configuration
    /// errors under DX12.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_settings(&mut self) {
        let graphics_settings = self.client_state.follow(client_state().graphics_settings());

        if self.active_graphics_settings.vsync != graphics_settings.vsync {
            self.graphics_engine.set_vsync(graphics_settings.vsync);
            self.active_graphics_settings.vsync = graphics_settings.vsync;
        }

        if self.active_graphics_settings.limit_framerate != graphics_settings.limit_framerate {
            self.graphics_engine.set_limit_framerate(graphics_settings.limit_framerate);
            self.active_graphics_settings.limit_framerate = graphics_settings.limit_framerate;
        }

        if self.active_graphics_settings.triple_buffering != graphics_settings.triple_buffering {
            self.graphics_engine.set_triple_buffering(graphics_settings.triple_buffering);
            self.active_graphics_settings.triple_buffering = graphics_settings.triple_buffering;
        }

        if self.active_graphics_settings.texture_filtering != graphics_settings.texture_filtering {
            self.graphics_engine.set_texture_sampler_type(graphics_settings.texture_filtering);
            self.active_graphics_settings.texture_filtering = graphics_settings.texture_filtering;
        }

        if self.active_graphics_settings.msaa != graphics_settings.msaa {
            self.graphics_engine.set_msaa(graphics_settings.msaa);
            self.active_graphics_settings.msaa = graphics_settings.msaa;
        }

        if self.active_graphics_settings.ssaa != graphics_settings.ssaa {
            self.graphics_engine.set_ssaa(graphics_settings.ssaa);
            self.active_graphics_settings.ssaa = graphics_settings.ssaa;
        }

        if self.active_graphics_settings.screen_space_anti_aliasing != graphics_settings.screen_space_anti_aliasing {
            self.graphics_engine
                .set_screen_space_anti_aliasing(graphics_settings.screen_space_anti_aliasing);
            self.active_graphics_settings.screen_space_anti_aliasing = graphics_settings.screen_space_anti_aliasing;
        }

        if self.active_graphics_settings.shadow_resolution != graphics_settings.shadow_resolution {
            self.graphics_engine.set_shadow_resolution(graphics_settings.shadow_resolution);
            self.active_graphics_settings.shadow_resolution = graphics_settings.shadow_resolution;
        }

        if self.active_graphics_settings.high_quality_interface != graphics_settings.high_quality_interface {
            self.interface_renderer
                .update_high_quality_interface(graphics_settings.high_quality_interface);
            self.graphics_engine
                .set_high_quality_interface(graphics_settings.high_quality_interface);
            self.active_graphics_settings.high_quality_interface = graphics_settings.high_quality_interface;
        }

        let language = *self.client_state.follow(client_state().interface_settings().language());

        if self.active_interface_settings.language != language {
            *self.client_state.follow_mut(client_state().localization()) = Localization::load_language(&self.game_file_loader, language);
            self.active_interface_settings.language = language;
        }

        let interface_settings = self.client_state.follow_mut(client_state().interface_settings());

        if self.active_interface_settings.menu_theme != interface_settings.menu_theme {
            let menu_theme = interface_settings.menu_theme.clone();
            let theme = InterfaceTheme::load(state::theme::InterfaceThemeType::Menu, &menu_theme);
            *self.client_state.follow_mut(client_state().menu_theme()) = theme;
            self.active_interface_settings.menu_theme = menu_theme;
        }

        let interface_settings = self.client_state.follow(client_state().interface_settings());

        if self.active_interface_settings.in_game_theme != interface_settings.in_game_theme {
            let in_game_theme = interface_settings.in_game_theme.clone();
            let theme = InterfaceTheme::load(InterfaceThemeType::InGame, &in_game_theme);
            *self.client_state.follow_mut(client_state().in_game_theme()) = theme;
            self.active_interface_settings.in_game_theme = in_game_theme;
        }

        let interface_settings = self.client_state.follow(client_state().interface_settings());

        if self.active_interface_settings.world_theme != interface_settings.world_theme {
            let world_theme = interface_settings.world_theme.clone();
            let theme = WorldTheme::load(&world_theme);
            *self.client_state.follow_mut(client_state().world_theme()) = theme;
            self.active_interface_settings.world_theme = world_theme;
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_interface_scaling(&mut self, scaling: Scaling) {
        self.bottom_interface_renderer.update_scaling(scaling);
        self.middle_interface_renderer.update_scaling(scaling);
        self.top_interface_renderer.update_scaling(scaling);
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn request_entity_details(&mut self, input_report: &InputReport) {
        if let PickerTarget::Entity(entity_id) = input_report.mouse_target
            && let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == entity_id)
            && entity.are_details_unavailable()
            && self.networking_system.entity_details(entity_id).is_ok()
        {
            entity.set_details_requested();
        }
    }

    fn entity_world_position(&self, entity_id: EntityId) -> Option<Point3<f32>> {
        self.client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(Entity::get_position)
            .or_else(|| {
                self.client_state
                    .try_follow(this_entity())
                    .filter(|entity| entity.get_entity_id() == entity_id)
                    .map(Entity::get_position)
            })
            .or_else(|| {
                self.client_state
                    .follow(client_state().dead_entities())
                    .iter()
                    .find(|entity| entity.get_entity_id() == entity_id)
                    .map(Entity::get_position)
            })
    }

    /// Finish a server-authoritative skill occurrence on its source actor.
    /// Entity id zero is the protocol's local-player sentinel for some result
    /// packets. `set_skill_attack` clears the cast before applying native actor
    /// request guards, so a completed skill cannot leave a stale cast bar.
    fn execute_skill_actor_action(&mut self, source_entity_id: EntityId, skill_id: SkillId, client_tick: ClientTick) {
        let animated_remote = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == source_entity_id)
            .map(|entity| entity.set_skill_attack(Some(skill_id), 0, false, client_tick))
            .is_some();

        if !animated_remote
            && let Some(entity) = self
                .client_state
                .try_follow_mut(this_entity())
                .filter(|entity| source_entity_id.0 == 0 || entity.get_entity_id() == source_entity_id)
        {
            entity.set_skill_attack(Some(skill_id), 0, false, client_tick);
        }
    }

    fn clear_skill_actor_cast(&mut self, source_entity_id: EntityId) {
        let cleared_remote = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == source_entity_id)
            .map(Entity::clear_cast)
            .is_some();

        if !cleared_remote
            && let Some(entity) = self
                .client_state
                .try_follow_mut(this_entity())
                .filter(|entity| source_entity_id.0 == 0 || entity.get_entity_id() == source_entity_id)
        {
            entity.clear_cast();
        }
    }

    /// Apply the target phase of one server-authoritative damage event. The
    /// source action and launch/caster tracks have already started; this phase
    /// owns numbers, hit effects/sounds, and target Hurt.
    fn apply_damage_impact(&mut self, pending: PendingImpact, client_tick: ClientTick) {
        let DamageImpact {
            source_entity_id,
            destination_entity_id,
            skill_id,
            packet_tick: _,
            damage_amount,
            hit_count: _,
            damage_delay,
            is_critical,
        } = pending.damage;

        let Some(target_position) = self.entity_world_position(destination_entity_id) else {
            return;
        };

        let particle: Box<dyn Particle + Send + Sync> = match damage_amount {
            Some(amount) => Box::new(DamageNumber::new(target_position, amount.to_string(), is_critical)),
            None => Box::new(Miss::new(target_position)),
        };
        self.particle_holder.spawn_particle(particle);

        if let Some(skill_id) = skill_id {
            self.spawn_damage_target_skill_effect(skill_id, source_entity_id, destination_entity_id, target_position);

            for (resolved, light_color, start_delay) in skill_hit_effects(skill_id) {
                self.spawn_resolved_effect(
                    resolved,
                    target_position,
                    PointLightId::new(destination_entity_id.0 ^ u32::from(skill_id.0)),
                    light_color,
                    45.0,
                    start_delay,
                    client_tick,
                );
            }

            for sound in skill_presentation_recipe(skill_id).hit_sounds {
                self.play_spatial_skill_sound(sound.resolve(), target_position);
            }
        }

        if damage_amount.is_some() && damage_delay > 0 {
            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == destination_entity_id)
            {
                entity.set_hurt(damage_delay, client_tick);
            } else if let Some(entity) = self
                .client_state
                .try_follow_mut(this_entity())
                .filter(|entity| entity.get_entity_id() == destination_entity_id)
            {
                entity.set_hurt(damage_delay, client_tick);
            }
        }
    }

    fn apply_due_impacts(&mut self, client_tick: ClientTick) {
        let due = self.pending_impacts.drain_due(client_tick);
        for impact in due {
            if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                eprintln!(
                    "[packet-log] impact source={} target={} skill={:?} hits={} packet_tick={} due={} now={}",
                    impact.damage.source_entity_id.0,
                    impact.damage.destination_entity_id.0,
                    impact.damage.skill_id.map(|skill_id| skill_id.0),
                    impact.damage.hit_count,
                    impact.damage.packet_tick.0,
                    impact.due_tick.0,
                    client_tick.0,
                );
            }
            self.apply_damage_impact(impact, client_tick);
        }
    }

    /// Surface a login-time error on the login form status line only (no
    /// separate Error popup — the form line is enough and stays in place).
    fn show_login_error(&mut self, message: &str) {
        eprintln!("[login] show_login_error: {message}");
        *self.client_state.follow_mut(client_state().login_window().status_message()) = message.to_owned();

        if !self.interface.is_window_with_class_open(WindowClass::Login) {
            self.interface.open_window(LoginWindow::new(
                client_state().login_window(),
                client_state().login_settings(),
                client_state().client_info(),
            ));
        }

        // Drop any leftover Error popup from older builds / other paths.
        self.interface.close_window_with_class(WindowClass::Error);
    }

    /// Tear down every connection-scoped surface and return to the login form
    /// with a status message. Used when the character-server handoff fails so
    /// the player is never left staring at a dead server-select window.
    fn return_to_login_with_error(&mut self, message: &str) {
        self.networking_system.disconnect_from_login_server();
        self.networking_system.disconnect_from_character_server();
        self.networking_system.disconnect_from_map_server();
        self.saved_login_data = None;
        self.saved_character_server = None;
        self.login_auth_expires_at = None;
        *self.client_state.follow_mut(client_state().server_select_status()) = String::new();

        #[cfg(not(feature = "debug"))]
        self.interface.close_all_windows();

        #[cfg(feature = "debug")]
        self.interface.close_all_windows_except(DEBUG_WINDOWS);

        self.interface.open_window(LoginWindow::new(
            client_state().login_window(),
            client_state().login_settings(),
            client_state().client_info(),
        ));
        self.show_login_error(message);
    }

    /// Tick the multi-server AUTH countdown. Sole-server stacks never open
    /// this window (they jump straight to character select).
    fn tick_server_select_auth(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::SelectServer) {
            return;
        }

        let Some(expires_at) = self.login_auth_expires_at else {
            return;
        };

        let now = Instant::now();
        if now >= expires_at {
            eprintln!("[login] AUTH_TIMEOUT elapsed on server select — returning to login");
            self.return_to_login_with_error(
                "Login session expired before server select (30s) — please log in again",
            );
            return;
        }

        let remaining = expires_at.saturating_duration_since(now);
        let secs = remaining.as_secs().max(1);
        *self.client_state.follow_mut(client_state().server_select_status()) =
            format!("Select a server — session expires in {secs}s");
    }

    /// Enter the character server using the same path as a manual server click.
    fn enter_character_server(&mut self, character_server_information: CharacterServerInformation) {
        let address = std::net::SocketAddr::new(
            std::net::IpAddr::V4(character_server_information.server_ip.into()),
            character_server_information.server_port,
        );
        eprintln!(
            "[login] SelectServer '{}' → {address}",
            character_server_information.server_name
        );

        self.login_auth_expires_at = None;
        *self.client_state.follow_mut(client_state().server_select_status()) =
            format!("Connecting to {}…", character_server_information.server_name);

        // Mark transition *before* dropping the login socket so a
        // LoginServerDisconnected event cannot auto-relogin and race us.
        self.saved_character_server = Some(character_server_information.clone());
        self.networking_system.disconnect_from_login_server();

        let login_data = self
            .saved_login_data
            .as_ref()
            .expect("character server entry requires a successful login");
        self.networking_system.connect_to_character_server(
            self.saved_packet_version,
            login_data,
            character_server_information,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn handle_network_events(&mut self, client_tick: ClientTick) {
        // Keep HUD cooldown line and timed compass marks in sync with server tick.
        self.client_state.follow_mut(client_state().skill_cooldowns()).tick(client_tick);
        self.client_state.follow_mut(client_state().minimap()).tick_markers(client_tick);
        self.networking_system.get_events(&mut self.network_event_buffer);

        // Deferred: cannot call &mut self helpers while draining network_event_buffer.
        let mut open_storage_ui = false;

        // Own the drained batch before dispatching so event handlers can call
        // reusable methods that borrow other Client fields mutably.
        let network_events: Vec<_> = self.network_event_buffer.drain().collect();
        for event in network_events {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    // Remove `_m`/`_f` suffix from the username. The suffix is only for *creating*
                    // an account and thus can (and needs to) be removed after the first successful
                    // login.
                    {
                        let selected_service_path =
                            SelectedServicePath::new(client_state().login_window(), client_state().login_settings());
                        let username_path = selected_service_path.username();

                        let username = self.client_state.follow_mut(username_path);

                        if let Some(stripped) = username.strip_suffix("_m") {
                            *username = stripped.to_owned();
                        } else if let Some(stripped) = username.strip_suffix("_f") {
                            *username = stripped.to_owned();
                        }
                    }

                    self.saved_login_data = Some(login_data);
                    *self.client_state.follow_mut(client_state().character_servers()) = character_servers.clone();

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    // Local Hercules (and most private servers) only advertise one
                    // character server — skip the select screen entirely so login
                    // goes account → character list with no flash.
                    if character_servers.len() == 1 {
                        let sole = character_servers.into_iter().next().unwrap();
                        eprintln!(
                            "[login] sole character server '{}' — auto-entering",
                            sole.server_name
                        );
                        self.enter_character_server(sole);
                    } else {
                        self.login_auth_expires_at = Some(Instant::now() + LOGIN_AUTH_TIMEOUT);
                        *self.client_state.follow_mut(client_state().server_select_status()) = format!(
                            "Select a server — session expires in {}s",
                            LOGIN_AUTH_TIMEOUT.as_secs()
                        );
                        self.interface
                            .open_window(ServerSelectionWindow::new(client_state().character_servers()));
                    }
                }
                NetworkEvent::LoginServerConnectionFailed { message, .. } => {
                    // M1-015: a failed re-login must not leave a stale server-select
                    // (or character-select) window sitting on a dead/half-open
                    // connection. Tear down every connection-scoped UI surface and
                    // both login/character sockets before showing the error.
                    eprintln!("[login] LoginServerConnectionFailed: {message}");
                    self.login_auth_expires_at = None;
                    self.networking_system.disconnect_from_login_server();
                    self.networking_system.disconnect_from_character_server();
                    self.interface.close_window_with_class(WindowClass::SelectServer);
                    self.interface.close_window_with_class(WindowClass::CharacterSelection);
                    self.interface.close_window_with_class(WindowClass::CharacterCreation);
                    self.show_login_error(message);
                }
                NetworkEvent::LoginServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // Do not auto-reconnect once the player is moving on to the
                        // character server (or already past it). A silent re-login
                        // here races SelectServer and can leave the UI stuck on the
                        // server-select window with a dead/half-open session.
                        let selecting_or_beyond = self.saved_character_server.is_some()
                            || self.networking_system.is_character_server_connected()
                            || self.networking_system.is_map_server_connected()
                            || self.interface.is_window_with_class_open(WindowClass::CharacterSelection);

                        if self.saved_login_data.is_some() && !selecting_or_beyond {
                            #[cfg(feature = "debug")]
                            print_debug!("Login server dropped after auth — reconnecting");

                            if let Some(socket_address) = self.saved_login_server_address {
                                eprintln!("[login] login socket dropped after auth — reconnecting");
                                self.networking_system.connect_to_login_server(
                                    self.saved_packet_version,
                                    socket_address,
                                    &self.saved_username,
                                    &self.saved_password,
                                );
                            }
                        } else if self.saved_login_data.is_none() {
                            // TCP drop before auth (or refuse already handled). Surface
                            // something if the login form has no status yet.
                            let status = self.client_state.follow(client_state().login_window().status_message());
                            if status.is_empty() {
                                eprintln!("[login] LoginServerDisconnected before auth ({reason:?})");
                                self.show_login_error("Could not connect to login server");
                            }
                        } else {
                            eprintln!(
                                "[login] LoginServerDisconnected ({reason:?}) ignored during char/map transition"
                            );
                        }
                    }
                }
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    eprintln!(
                        "[login] CharacterServerConnected slots={normal_slot_count} — requesting character list"
                    );
                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_slot_count(normal_slot_count);

                    let _ = self.networking_system.request_character_list();
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                    eprintln!("[login] CharacterServerConnectionFailed: {message}");
                    self.return_to_login_with_error(message);
                }
                NetworkEvent::CharacterServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // A drop while still on server-select / not yet on map is a
                        // failed handoff — bounce back to login instead of silently
                        // retrying forever (that left the player stuck on select).
                        let on_map = self.networking_system.is_map_server_connected();
                        if on_map {
                            #[cfg(feature = "debug")]
                            print_debug!("Character server dropped while on map — reconnecting");
                            if let (Some(login_data), Some(server)) =
                                (self.saved_login_data.as_ref(), self.saved_character_server.clone())
                            {
                                self.networking_system.connect_to_character_server(
                                    self.saved_packet_version,
                                    login_data,
                                    server,
                                );
                            }
                        } else {
                            eprintln!("[login] CharacterServerDisconnected ({reason:?}) before map — return to login");
                            self.return_to_login_with_error(
                                "Lost connection to character server (auth may have expired — log in again)",
                            );
                        }
                    } else if !self.networking_system.is_map_server_connected() && self.saved_login_data.is_some() {
                        // Intentional char disconnect without map (e.g. log out character).
                        #[cfg(not(feature = "debug"))]
                        self.interface.close_all_windows();

                        #[cfg(feature = "debug")]
                        self.interface.close_all_windows_except(DEBUG_WINDOWS);

                        self.interface.open_window(LoginWindow::new(
                            client_state().login_window(),
                            client_state().login_settings(),
                            client_state().client_info(),
                        ));
                    }
                }
                NetworkEvent::MapServerDisconnected { reason } => {
                    self.client_state.follow_mut(client_state().party_state()).clear();

                    // Drop any armed skill so it can't leak across a logout/relogin and fire on
                    // the first click of the next session.
                    self.pending_skill = None;

                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the map server with error");
                    }

                    // When both servers go down at once — a server restart, or any
                    // network loss — the character-server disconnect is delivered
                    // first and has already run `return_to_login_with_error`, which
                    // clears the saved credentials. Unwrapping them here panicked
                    // (reproduced live 2026-07-24 by stopping Hercules while in
                    // game). If they are gone we are on the login screen by design,
                    // so there is nothing to reconnect to. Mirrors the guard the
                    // `CharacterServerDisconnected` arm already uses.
                    if let (Some(login_data), Some(server)) =
                        (self.saved_login_data.as_ref(), self.saved_character_server.clone())
                    {
                        self.networking_system
                            .connect_to_character_server(self.saved_packet_version, login_data, server);
                    }

                    self.map = None;

                    self.particle_holder.clear();
                    self.pending_impacts.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.active_trap_props.clear();
                    self.active_status_effects.clear();
                    self.audio_engine.clear_ambient_sound();

                    self.client_state.follow_mut(client_state().entities()).clear();
                    self.client_state.follow_mut(client_state().dead_entities()).clear();
                    self.client_state.follow_mut(client_state().ground_items()).clear();
                    // Cleared here rather than when an entity disappears: the server
                    // re-sends ammunition on enter-view, so a stale entry is simply
                    // overwritten, whereas evicting on removal would reopen the hole
                    // this map exists to close.
                    self.client_state.follow_mut(client_state().remote_ammunition()).clear();
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;

                    self.audio_engine.play_background_music_track(None);

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.async_loader
                        .request_map_load(DEFAULT_MAP.to_string(), Some(TilePosition::new(0, 0)));
                }
                NetworkEvent::InitialStats {
                    strength_stat_points_cost,
                    agility_stat_points_cost,
                    vitality_stat_points_cost,
                    intelligence_stat_points_cost,
                    dexterity_stat_points_cost,
                    luck_stat_points_cost,
                } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.strength_stat_points_cost = strength_stat_points_cost;
                        player.agility_stat_points_cost = agility_stat_points_cost;
                        player.vitality_stat_points_cost = vitality_stat_points_cost;
                        player.intelligence_stat_points_cost = intelligence_stat_points_cost;
                        player.dexterity_stat_points_cost = dexterity_stat_points_cost;
                        player.luck_stat_points_cost = luck_stat_points_cost;
                    }
                }
                NetworkEvent::ResurrectPlayer { entity_id } => {
                    // Revive the sprite: an in-place resurrection (Resurrection
                    // skill, town revive) leaves the entity in its death pose
                    // otherwise. `revive` bypasses the death action-lock.
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.revive(client_tick);
                    }
                    // If the resurrected player is us, close the resurrect window.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id() == entity_id)
                    {
                        self.interface.close_window_with_class(WindowClass::Respawn);
                    }
                }
                NetworkEvent::PlayerSitDown { entity_id } => {
                    let found = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .map(|entity| {
                            entity.set_sit(client_tick);
                        })
                        .is_some();
                    if !found && let Some(player) = self.client_state.try_follow_mut(this_entity()) {
                        player.set_sit(client_tick);
                    }
                }
                NetworkEvent::PlayerStandUp { entity_id } => {
                    let found = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .map(|entity| {
                            entity.set_idle(client_tick);
                        })
                        .is_some();
                    if !found && let Some(player) = self.client_state.try_follow_mut(this_entity()) {
                        player.set_idle(client_tick);
                    }
                }
                NetworkEvent::AccountId { .. } => {}
                NetworkEvent::CharacterList { characters } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_characters(characters);

                    if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
                        // TODO: this will do one unnecessary restore_focus. check
                        // if that will be problematic

                        #[cfg(not(feature = "debug"))]
                        self.interface.close_all_windows();

                        #[cfg(feature = "debug")]
                        self.interface.close_all_windows_except(DEBUG_WINDOWS);

                        self.interface.open_window(CharacterSelectionWindow::new(
                            client_state().character_slots(),
                            client_state().switch_request(),
                        ));
                    }
                }
                NetworkEvent::CharacterSelectionFailed { message, .. } => self.interface.open_window(ErrorWindow::new(message.to_owned())),
                NetworkEvent::CharacterDeleted => {
                    if let Some(character_id) = self.client_state.follow_mut(client_state().currently_deleting()).take() {
                        self.client_state
                            .follow_mut(client_state().character_slots())
                            .remove_with_id(character_id);
                    }
                }
                NetworkEvent::CharacterDeletionFailed { message, .. } => {
                    *self.client_state.follow_mut(client_state().currently_deleting()) = None;
                    self.interface.open_window(ErrorWindow::new(message.to_owned()))
                }
                NetworkEvent::CharacterSelected { login_data, .. } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);
                    self.client_state.follow_mut(client_state().party_state()).clear();
                    self.client_state.follow_mut(client_state().skill_tree()).clear();
                    self.client_state.follow_mut(client_state().hotbar()).clear();

                    let saved_login_data = self.saved_login_data.as_ref().unwrap();
                    self.networking_system.disconnect_from_character_server();
                    self.networking_system
                        .connect_to_map_server(self.saved_packet_version, saved_login_data, login_data);
                    // Ask for the client tick right away, so that the player isn't de-synced when
                    // they spawn on the map.
                    let _ = self.networking_system.request_client_tick();

                    let character_information = self
                        .client_state
                        .follow(client_state().character_slots())
                        .with_id(login_data.character_id)
                        .cloned()
                        .unwrap();

                    let mut player = Entity::Player(Player::new(
                        &self.library,
                        saved_login_data.account_id,
                        &character_information,
                        client_tick,
                    ));

                    *self.client_state.follow_mut(client_state().player_name()) = character_information.name;

                    let entity_id = player.get_entity_id();
                    let entity_type = player.get_entity_type();
                    let entity_part_files = player.get_entity_part_files(&self.library, &self.game_file_loader);

                    if let Some(animation_data) = self
                        .async_loader
                        .request_animation_data_load(entity_id, entity_type, entity_part_files)
                    {
                        player.set_animation_data(animation_data);
                    }

                    let layout = self.async_loader.request_skill_tree_layout_load(player.get_job_id(), client_tick);
                    *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = layout.tabs.len().saturating_sub(1);
                    *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .clear();

                    self.client_state.follow_mut(client_state().entities()).push(player);

                    self.interface.close_window_with_class(WindowClass::CharacterSelection);
                    self.interface.open_window(CharacterOverviewWindow::new(
                        client_state().player_name(),
                        // TODO: Check that manually asserting is fine. Technically this window should only
                        // be open while the player is selected.
                        this_player().manually_asserted().base_level(),
                        // TODO: Check that manually asserting is fine. Technically this window should only
                        // be open while the player is selected.
                        this_player().manually_asserted().job_level(),
                    ));
                    self.interface
                        .open_window(ChatWindow::new(client_state().chat_window(), client_state().chat_messages()));
                    self.interface.open_window(HotbarWindow::new(
                        client_state().hotbar().skills(),
                        client_state().skill_tree().skills(),
                    ));
                    self.interface.open_window(StatusBarWindow::new(client_state().status_effects()));
                    self.interface.open_window(HudWindow::new(
                        this_player().manually_asserted(),
                        client_state().skill_cooldowns(),
                    ));
                    self.interface.open_window(PartyWindow::new(client_state().party_state()));
                    // Minimap is filled when the map resource finishes loading; open a placeholder
                    // only if the player wants it visible (Game Settings / Map button / Alt+M).
                    let show_minimap = *self.client_state.follow(client_state().game_settings().show_minimap());
                    if show_minimap && !self.interface.is_window_with_class_open(WindowClass::Minimap) {
                        self.interface.open_window(MinimapWindow);
                    }

                    // Put the dialog system in a well-defined state.
                    self.client_state.follow_mut(client_state().dialog_window()).end();

                    self.map = None;

                    self.particle_holder.clear();
                    self.pending_impacts.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.active_trap_props.clear();
                    self.active_status_effects.clear();
                    self.audio_engine.clear_ambient_sound();
                }
                NetworkEvent::CharacterCreated { character_information } => {
                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .add_character(character_information);

                    self.interface.close_window_with_class(WindowClass::CharacterCreation);
                }
                NetworkEvent::CharacterCreationFailed { message, .. } => {
                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::CharacterSlotSwitched => {
                    *self.client_state.follow_mut(client_state().switch_request()) = None;
                }
                NetworkEvent::CharacterSlotSwitchFailed => {
                    self.interface
                        .open_window(ErrorWindow::new("Failed to switch character slots".to_owned()));
                }
                NetworkEvent::AddEntity { entity_data } => {
                    if let Some(map) = &self.map
                        && let Some(npc) = Npc::new(&self.library, map, &mut self.path_finder, entity_data, client_tick)
                    {
                        let mut npc = Entity::Npc(npc);

                        let entity_id = npc.get_entity_id();
                        let entity_type = npc.get_entity_type();
                        let entity_part_files = npc.get_entity_part_files(&self.library, &self.game_file_loader);

                        #[cfg(feature = "debug")]
                        if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                            print_debug!(
                                "[packet-log] add-entity id={:?} job_id={:?} type={:?} parts={:?}",
                                entity_id,
                                npc.get_job_id(),
                                entity_type,
                                entity_part_files
                            );
                        }

                        let entities = self.client_state.follow_mut(client_state().entities());

                        // If the entity was already visible, we use it's old alpha value.
                        if let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == entity_id) {
                            npc.inherit_fade_state(entity, client_tick);
                        };

                        // Sometimes (like after a job change) the server will tell the client
                        // that a new entity appeared, even though it was already on screen. So
                        // to prevent the entity existing twice, we remove the old one.
                        entities.retain(|entity| entity.get_entity_id() != entity_id);

                        if let Some(animation_data) =
                            self.async_loader
                                .request_animation_data_load(entity_id, entity_type, entity_part_files)
                        {
                            npc.set_animation_data(animation_data);
                        }

                        #[cfg(feature = "debug")]
                        npc.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);

                        let entity_position = npc.get_position();

                        npc.set_in_safe_zone(self.current_map_is_town);
                        entities.push(npc);

                        // Map-transfer warps carry no sprite; the original
                        // client draws its blue swirling vortex there instead.
                        if entity_type == EntityType::Warp
                            && let Ok(texture) = self.texture_loader.get_or_load(PORTAL_TEXTURE_PATH, ImageType::Color)
                        {
                            self.effect_holder
                                .add_unit(Box::new(PortalVortex::new(texture, entity_position)), entity_id);
                        }
                    }
                }
                NetworkEvent::RemoveEntity { entity_id, reason } => {
                    // If the motive is dead, you need to set the player to dead.
                    if reason == DisappearanceReason::Died {
                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            let entity_type = entity.get_entity_type();

                            if entity_type == EntityType::Monster {
                                let mut entity = entity.clone();
                                entity.set_dead(client_tick);
                                entity.stop_movement();

                                // Record the kill for the bestiary journal.
                                // Raw job IDs only — the bestiary data itself
                                // is parsed lazily when the journal opens.
                                let job_id = entity.get_job_id().0 as u32;
                                let unlocked = self.client_state.follow_mut(client_state().dm_campaign().bestiary_unlocked());
                                if !unlocked.contains(&job_id) {
                                    unlocked.push(job_id);
                                }

                                // Remove the entity from the list of alive entities.
                                self.client_state
                                    .follow_mut(client_state().entities())
                                    .retain(|entity| entity.get_entity_id() != entity_id);

                                // Add the entity to the list of dead entities.
                                self.client_state.follow_mut(client_state().dead_entities()).push(entity);
                            } else if entity_type == EntityType::Player {
                                entity.set_dead(client_tick);

                                // If the player is us, we need to open the respawn window.
                                if entity_id == self.client_state.follow(client_state().entities())[0].get_entity_id() {
                                    self.interface.close_window_with_class(WindowClass::WarpSelection);
                                    self.interface.close_window_with_class(WindowClass::WeaponRefine);
                                    self.interface.open_window(RespawnWindow);
                                }
                            }
                        }
                    } else {
                        self.pending_impacts.remove_target(entity_id);

                        // For non-death disappearances, start fading out the entity.
                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            entity.fade_out(reason, client_tick);
                        }
                    }

                    // If the entity that was removed had an attack buffered we remove the entity
                    // from the buffer.
                    let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
                    if buffered_action.is_some_and(|buffered_action| buffered_action.targets_entity(entity_id)) {
                        *buffered_action = None;
                    }

                    // Drop any effect attached to the entity, like a warp's
                    // portal vortex.
                    self.effect_holder.remove_unit(entity_id);
                    self.sprite_effects.remove_unit(entity_id);
                    self.skill_unit_registry.remove(entity_id);
                }
                NetworkEvent::AddGroundItem {
                    entity_id,
                    item_id,
                    is_identified,
                    quantity,
                    position,
                    x_offset,
                    y_offset,
                } => {
                    if let Some(map) = self.map.as_ref()
                        && let Some(mut ground_item) = GroundItem::new(
                            map,
                            item_id,
                            entity_id,
                            is_identified,
                            quantity,
                            position,
                            x_offset,
                            y_offset,
                            client_tick,
                        )
                    {
                        let ground_items = self.client_state.follow_mut(client_state().ground_items());
                        let entity_part_files = ground_item.get_entity_part_files(&self.library);

                        if let Some(animation_data) = self
                            .async_loader
                            // TODO: Technically Npc is not correct here. We could add an item
                            // variant or refactor this fuction to take an optional entity
                            // type.
                            .request_animation_data_load(entity_id, EntityType::Npc, entity_part_files)
                        {
                            ground_item.set_animation_data(animation_data);
                        }

                        ground_items.push(ground_item);
                    } else {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] failed to spawn item", "error".red());
                    }
                }
                NetworkEvent::RemoveGroundItem { entity_id } => {
                    if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.fade_out(client_tick);
                    }

                    let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
                    if buffered_action.is_some_and(|buffered_action| buffered_action.is_pick_up_item(entity_id)) {
                        *buffered_action = None;
                    }
                }
                NetworkEvent::EntityMove {
                    entity_id,
                    origin,
                    destination,
                    starting_timestamp,
                } => {
                    let entities = self.client_state.follow_mut(client_state().entities());
                    let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity
                        && let Some(map) = &self.map
                    {
                        entity.move_from_to(
                            map,
                            &mut self.path_finder,
                            origin.tile_position(),
                            destination.tile_position(),
                            starting_timestamp,
                        );
                        #[cfg(feature = "debug")]
                        entity.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::EntitySlide { entity_id, position } => {
                    if let Some(map) = &self.map {
                        let is_player = self
                            .client_state
                            .try_follow(this_entity())
                            .is_some_and(|player| player.get_entity_id() == entity_id);
                        if is_player {
                            if let Some(player) = self.client_state.try_follow_mut(this_entity()) {
                                player.set_position(map, position, client_tick);
                            }
                        } else if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            entity.set_position(map, position, client_tick);
                        }
                    }
                }
                NetworkEvent::PlayerMove {
                    origin,
                    destination,
                    starting_timestamp,
                } => {
                    if let Some(map) = &self.map
                        && let Some(player) = self.client_state.try_follow_mut(this_entity())
                    {
                        player.move_from_to(
                            map,
                            &mut self.path_finder,
                            origin.tile_position(),
                            destination.tile_position(),
                            starting_timestamp,
                        );
                        #[cfg(feature = "debug")]
                        player.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::ChangeMap { map_name, position } => {
                    self.map = None;
                    self.particle_holder.clear();
                    self.pending_impacts.clear();
                    self.emote_bubbles.clear();
                    self.sprite_effects.clear();
                    self.effect_holder.clear();
                    self.skill_unit_registry.clear();
                    self.active_trap_props.clear();
                    self.active_status_effects.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    // Only the player must stay alive between map changes.
                    self.client_state.follow_mut(client_state().entities()).truncate(1);
                    self.client_state.follow_mut(client_state().dead_entities()).clear();
                    self.client_state.follow_mut(client_state().ground_items()).clear();
                    self.client_state.follow_mut(client_state().status_effects()).clear();
                    self.client_state.follow_mut(client_state().skill_cooldowns()).clear();
                    // A respawn-to-save-point (die → Respawn) arrives as a map
                    // change, and the local player survives the truncate(1)
                    // above carrying its death animation. Revive it to idle so
                    // we don't stay dead after respawning; harmless for a normal
                    // portal warp, where the player always arrives standing.
                    if let Some(player) = self.client_state.try_follow_mut(this_entity()) {
                        player.clear_cast();
                        player.revive(client_tick);
                    }
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;

                    // Close any remaining dialogs.
                    self.interface.close_window_with_class(WindowClass::Dialog);
                    self.interface.close_window_with_class(WindowClass::WarpSelection);
                    self.interface.close_window_with_class(WindowClass::WeaponRefine);

                    // Instanced maps (`000#pronter`) must load their base
                    // map's resources; the wire name has no `.rsw`.
                    let resource_name = self.game_file_loader.resolve_map_name(&map_name);
                    self.async_loader.request_map_load(resource_name, Some(position));
                }
                NetworkEvent::UpdateClientTick { client_tick, received_at } => {
                    self.game_timer.set_client_tick(client_tick, received_at);
                }
                NetworkEvent::ChatMessage { text, color } => {
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(text, color));
                }
                NetworkEvent::MessageTable { message_id, color } => {
                    let text = self.library.message_string(message_id);
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(text, color));
                }
                NetworkEvent::SkillFailedMissingItem {
                    item_id,
                    amount,
                    equipment,
                } => {
                    let text = missing_skill_item_text(&self.library, item_id, amount, equipment);
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(text, MessageColor::Error));
                }
                NetworkEvent::UpdateEntityDetails { entity_id, name } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.set_details(name);
                    }
                }
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    skill_id,
                    packet_tick,
                    damage_amount,
                    hit_count,
                    attack_duration,
                    damage_delay,
                    is_critical,
                } => {
                    let camera_direction = self.player_camera.camera_direction();
                    if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                        eprintln!(
                            "[packet-log] damage source={} target={} skill={:?} hits={hit_count} duration={attack_duration}",
                            source_entity_id.0,
                            destination_entity_id.0,
                            skill_id.map(|skill_id| skill_id.0),
                        );
                    }
                    let target_position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == destination_entity_id)
                        .map(|entity| entity.get_tile_position());

                    // Auto attack logic.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id() == source_entity_id)
                    {
                        let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                        let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                        if let Some(BufferedAction::AttackEntity { entity_id }) = *buffered_action {
                            let _ = self.networking_system.player_attack(entity_id);

                            if !auto_attack {
                                *buffered_action = None;
                            }
                        }
                    }

                    let mut source_impact_delay_ms = 0;
                    let mut animated_source = false;
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                    {
                        if let Some(target_position) = target_position {
                            entity.rotate_towards(target_position);
                        }

                        entity.set_skill_attack(skill_id, attack_duration, is_critical, client_tick);
                        source_impact_delay_ms = entity.impact_delay_ms(skill_id, camera_direction);
                        animated_source = true;
                    }
                    // The local player lives under `this_entity`, not in the
                    // remote-entity vector. Without this fallback, our own
                    // skill casts showed effects but no classic weapon action.
                    if !animated_source
                        && let Some(entity) = self
                            .client_state
                            .try_follow_mut(this_entity())
                            .filter(|entity| entity.get_entity_id() == source_entity_id)
                    {
                        if let Some(target_position) = target_position {
                            entity.rotate_towards(target_position);
                        }
                        entity.set_skill_attack(skill_id, attack_duration, is_critical, client_tick);
                        source_impact_delay_ms = entity.impact_delay_ms(skill_id, camera_direction);
                    }

                    // Caster/begin and travel tracks launch with the source
                    // action. Target hit tracks are deferred below.
                    if let Some(skill_id) = skill_id
                        && let Some(target_position) = self.entity_world_position(destination_entity_id)
                    {
                        let source_position = self.entity_world_position(source_entity_id).or_else(|| {
                            (source_entity_id.0 == 0)
                                .then(|| self.client_state.try_follow(this_entity()).map(Entity::get_position))
                                .flatten()
                        });
                        if let Some(source_position) = source_position {
                            self.spawn_damage_caster_skill_effect(
                                skill_id,
                                source_entity_id,
                                source_position,
                                target_position,
                                hit_count,
                                source_impact_delay_ms,
                                client_tick,
                            );
                        }
                    }

                    // Normal (non-skill) ranged attack: draw the flying arrow.
                    if skill_id.is_none() {
                        self.spawn_ranged_attack_projectile(source_entity_id, destination_entity_id);
                    }

                    self.pending_impacts.schedule(client_tick, source_impact_delay_ms, DamageImpact {
                        source_entity_id,
                        destination_entity_id,
                        skill_id,
                        packet_tick,
                        damage_amount,
                        hit_count,
                        damage_delay,
                        is_critical,
                    });
                }
                NetworkEvent::EntityPickUpItem { entity_id, item_entity_id } => {
                    let item_position = self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .find(|item| item.get_entity_id() == item_entity_id)
                        .map(|item| item.get_tile_position());

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        if let Some(item_position) = item_position {
                            entity.rotate_towards(item_position);
                        }

                        if matches!(entity.get_entity_type(), EntityType::Player | EntityType::Hidden) {
                            entity.set_pickup(client_tick);
                        }
                    }
                }
                NetworkEvent::HealEffect { entity_id, heal_amount } => {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .or_else(|| self.client_state.try_follow(this_entity()))
                    {
                        self.particle_holder
                            .spawn_particle(Box::new(HealNumber::new(entity.get_position(), heal_amount.to_string())));
                    }
                }
                NetworkEvent::SkillEffectNoDamage {
                    skill_id,
                    source_entity_id,
                    destination_entity_id,
                    effect_value,
                    successful,
                } => {
                    // This packet is a terminal skill result. A successful
                    // result owns the source actor action; a failed one still
                    // tears down the cast without inventing an action.
                    if successful {
                        self.execute_skill_actor_action(source_entity_id, skill_id, client_tick);
                    } else {
                        self.clear_skill_actor_cast(source_entity_id);
                    }

                    // Preserve the heal-number behavior that used to be
                    // produced directly by the packet handler.
                    const AL_HEAL: u16 = 28;
                    const AB_CHEAL: u16 = 2043;
                    const AB_HIGHNESSHEAL: u16 = 2051;
                    const HLIF_HEAL: u16 = 8001;
                    let is_heal_skill = matches!(skill_id.0, AL_HEAL | AB_CHEAL | AB_HIGHNESSHEAL | HLIF_HEAL);
                    let is_displayable = effect_value > 0 && effect_value < i32::MAX as u32;

                    if is_heal_skill
                        && is_displayable
                        && let Some(entity) = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == destination_entity_id)
                            .or_else(|| self.client_state.try_follow(this_entity()))
                    {
                        self.particle_holder
                            .spawn_particle(Box::new(HealNumber::new(entity.get_position(), effect_value.to_string())));
                    }

                    // Caster-centered visuals use this successful-use packet;
                    // it is sent even when an area skill finds no targets.
                    if successful {
                        let source_position = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|source| source.get_entity_id() == source_entity_id)
                            .map(|source| source.get_position())
                            .or_else(|| {
                                self.client_state
                                    .try_follow(this_entity())
                                    .filter(|source| source_entity_id.0 == 0 || source.get_entity_id() == source_entity_id)
                                    .map(|source| source.get_position())
                            });

                        if let Some(source_position) = source_position {
                            self.spawn_successful_caster_skill_effect(skill_id, source_entity_id, source_position);
                        }
                    }
                }
                NetworkEvent::StatusChange {
                    entity_id,
                    index,
                    gained,
                    duration_ms,
                    remaining_ms,
                    values,
                } => {
                    let updated_actor = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .map(|entity| entity.update_animation_status(index, gained, client_tick))
                        .is_some();

                    let local_id = self.client_state.try_follow(this_entity()).map(Entity::get_entity_id);
                    if !updated_actor
                        && Some(entity_id) == local_id
                        && let Some(entity) = self.client_state.try_follow_mut(this_entity())
                    {
                        entity.update_animation_status(index, gained, client_tick);
                    }

                    // The HUD remains local-only; actor guard/status-pose state
                    // above is applied to every visible entity.
                    if Some(entity_id) == local_id {
                        // Several statuses share one icon index — all three
                        // Sage fields are `SI_GROUNDMAGIC` — so name them from
                        // the unit the player is actually standing in. Must be
                        // resolved before borrowing state mutably below.
                        let specific_name = gained
                            .then(|| {
                                let position = self.client_state.try_follow(this_entity())?.get_position();
                                let unit_id = self.skill_unit_registry.elemental_field_at(position)?;
                                elemental_field_name(unit_id).map(str::to_owned)
                            })
                            .flatten();

                        let effects = self.client_state.follow_mut(client_state().status_effects());
                        if gained {
                            effects.apply(index, duration_ms, remaining_ms, values, specific_name);
                        } else {
                            effects.remove(index);
                        }
                    }
                }
                NetworkEvent::StateChange {
                    entity_id,
                    option,
                    body_state,
                    health_state,
                    is_pk_mode_on,
                } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.update_state(option, body_state, health_state, is_pk_mode_on, client_tick);
                    } else if let Some(entity) = self
                        .client_state
                        .try_follow_mut(this_entity())
                        .filter(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.update_state(option, body_state, health_state, is_pk_mode_on, client_tick);
                    }

                    self.update_status_effect_visual(entity_id, body_state, health_state);
                }
                NetworkEvent::UpdateEntityHealth {
                    entity_id,
                    health_points,
                    maximum_health_points,
                } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.update_health(health_points, maximum_health_points);
                    }
                }
                NetworkEvent::UpdateStat { stat_type } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.update_stat(stat_type);
                    }
                }
                NetworkEvent::CriticalWeightPercent { percent } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.critical_weight_percent = percent;
                    }
                }
                NetworkEvent::UpdateAttackRange { attack_range } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.attack_range = attack_range;
                    }
                }
                NetworkEvent::OpenDialog { text, npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .initialize(npc_id)
                        .add_text(text);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddNextButton { npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        // An NPCs could start the dialog with this packet so we want to make sure it's initialized.
                        .initialize(npc_id)
                        .add_next_button();

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddCloseButton { npc_id } => {
                    // Some NPCs send the `CloseButtonPacket` after the dialog
                    // has been closed. We want to filter these out because otherwise we get a
                    // close button at the start of the next dialog.
                    if self.interface.is_window_with_class_open(WindowClass::Dialog) {
                        self.client_state
                            .follow_mut(client_state().dialog_window())
                            // Technically this call is redundant since the window is already open
                            // but we keep it for consistency.
                            .initialize(npc_id)
                            .add_close_button();
                    }
                }
                NetworkEvent::AddChoiceButtons { choices, npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        // Some NPCs start the dialog with this packet so we need to make sure it's initialized.
                        .initialize(npc_id)
                        .add_choice_buttons(choices);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::NpcRequestNumberInput { npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .initialize(npc_id)
                        .add_number_input();

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::NpcRequestStringInput { npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .initialize(npc_id)
                        .add_string_input();

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddQuestEffect { quest_effect } => {
                    if let Some(map) = &self.map {
                        self.particle_holder.add_quest_icon(&self.texture_loader, map, quest_effect)
                    }
                }
                NetworkEvent::RemoveQuestEffect { entity_id } => self.particle_holder.remove_quest_icon(entity_id),
                // The quest log has no UI yet; these events exist for the
                // headless tester and a future quest-log window.
                NetworkEvent::QuestAdded { .. } => {}
                NetworkEvent::QuestRemoved { .. } => {}
                NetworkEvent::QuestList { .. } => {}
                NetworkEvent::SetInventory { items } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .fill(&self.async_loader, items);

                    let inventory = self.client_state.follow(client_state().inventory());
                    let weapon = inventory.equipped_weapon_look();
                    let left_hand = inventory.equipped_left_hand_look();
                    if let Some(player) = self.client_state.try_follow_mut(this_entity()) {
                        player.set_weapon(weapon);
                        if let Some(left) = left_hand {
                            player.set_shield(left);
                        }
                        let entity_part_files = player.get_entity_part_files(&self.library, &self.game_file_loader);
                        if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                            eprintln!(
                                "[packet-log] local equipped weapon={weapon} left={left_hand:?} parts={entity_part_files:?}"
                            );
                        }
                        Self::refresh_entity_player_gear(
                            &self.async_loader,
                            &self.library,
                            &self.game_file_loader,
                            player,
                            &entity_part_files,
                        );
                        player.refresh_neutral_stance(client_tick);
                    }
                }
                NetworkEvent::SetStorage { items } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .set_list(&self.async_loader, items);
                    open_storage_ui = true;
                }
                NetworkEvent::StorageAmount { amount, max_amount } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .set_amount(amount, max_amount);
                    // Hercules always sends capacity after the list; also opens
                    // storage when the item list is empty (Start/End only).
                    open_storage_ui = true;
                }
                NetworkEvent::StorageItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .add_item(&self.async_loader, item);
                }
                NetworkEvent::StorageItemRemoved { index, amount } => {
                    self.client_state.follow_mut(client_state().storage()).remove_item(index, amount);
                }
                NetworkEvent::StorageClosed => {
                    self.client_state.follow_mut(client_state().storage()).close();
                    self.interface.close_window_with_class(WindowClass::Storage);
                }
                NetworkEvent::ItemIdentifyList { indices } => {
                    self.client_state.follow_mut(client_state().identify_state()).set_list(indices);
                    if !self.interface.is_window_with_class_open(WindowClass::Identify) {
                        self.interface.open_window(IdentifyWindow::new(client_state().identify_state()));
                    }
                }
                NetworkEvent::ItemIdentified { inventory_index, success } => {
                    if success {
                        self.client_state
                            .follow_mut(client_state().inventory())
                            .mark_identified(inventory_index);
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Item identified.".to_owned(), MessageColor::Information));
                    } else {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "Identify cancelled or failed.".to_owned(),
                            MessageColor::Information,
                        ));
                    }
                    self.client_state.follow_mut(client_state().identify_state()).clear();
                    self.interface.close_window_with_class(WindowClass::Identify);
                }
                NetworkEvent::TradeRequest {
                    name,
                    character_id,
                    base_level,
                } => {
                    self.client_state
                        .follow_mut(client_state().trade_state())
                        .set_pending(name.clone(), character_id, base_level);
                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!("Trade request from {name} (Lv{base_level})."),
                        MessageColor::Information,
                    ));
                    if !self.interface.is_window_with_class_open(WindowClass::TradeRequest) {
                        self.interface.open_window(TradeRequestWindow);
                    }
                }
                NetworkEvent::TradeStart {
                    result,
                    character_id,
                    base_level,
                } => match result {
                    3 => {
                        let name = self.client_state.follow(client_state().trade_state()).pending_name().to_owned();
                        let name = if name.is_empty() { "Partner".to_owned() } else { name };
                        self.client_state
                            .follow_mut(client_state().trade_state())
                            .open_with_partner(name, character_id, base_level);
                        self.interface.close_window_with_class(WindowClass::TradeRequest);
                        if !self.interface.is_window_with_class_open(WindowClass::Trade) {
                            self.interface.open_window(TradeWindow::new(client_state().trade_state()));
                        }
                    }
                    0 => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        "Trade failed: character is too far.".to_owned(),
                        MessageColor::Error,
                    )),
                    1 => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        "Trade failed: character not found.".to_owned(),
                        MessageColor::Error,
                    )),
                    4 => {
                        self.client_state.follow_mut(client_state().trade_state()).clear();
                        self.interface.close_window_with_class(WindowClass::TradeRequest);
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Trade rejected.".to_owned(), MessageColor::Information));
                    }
                    5 => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        "Trade failed: target is busy.".to_owned(),
                        MessageColor::Error,
                    )),
                    _ => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!("Trade failed (result {result})."),
                        MessageColor::Error,
                    )),
                },
                NetworkEvent::TradePartnerItem {
                    item_id,
                    amount,
                    identified,
                    refine,
                    ..
                } => {
                    let name = resolve_item_name(&self.library, item_id, identified);
                    self.client_state
                        .follow_mut(client_state().trade_state())
                        .add_partner_item(item_id, amount, identified, refine, name.as_deref());
                }
                NetworkEvent::TradeAddItemResult { inventory_index, result } => {
                    if result == 0 {
                        let ours = self
                            .client_state
                            .follow(client_state().inventory().items())
                            .iter()
                            .find(|i| i.index == inventory_index)
                            .map(|item| {
                                let amount = match &item.details {
                                    korangar_networking::InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
                                    _ => 1,
                                };
                                (item.item_id, amount, item.is_identified())
                            })
                            .map(|(item_id, amount, is_identified)| {
                                let name = resolve_item_name(&self.library, item_id, is_identified);
                                (item_id, amount, trade_item_label(name.as_deref(), item_id, amount, 0))
                            });
                        if let Some((item_id, amount, label)) = ours {
                            self.client_state
                                .follow_mut(client_state().trade_state())
                                .note_our_item(item_id, amount, label);
                        }
                    } else {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            format!("Could not add item to trade (result {result})."),
                            MessageColor::Error,
                        ));
                    }
                }
                NetworkEvent::TradeLocked { who } => {
                    self.client_state.follow_mut(client_state().trade_state()).lock_side(who);
                }
                NetworkEvent::TradeCancelled => {
                    self.client_state.follow_mut(client_state().trade_state()).clear();
                    self.interface.close_window_with_class(WindowClass::Trade);
                    self.interface.close_window_with_class(WindowClass::TradeRequest);
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new("Trade cancelled.".to_owned(), MessageColor::Information));
                }
                NetworkEvent::TradeCompleted { success } => {
                    self.client_state.follow_mut(client_state().trade_state()).clear();
                    self.interface.close_window_with_class(WindowClass::Trade);
                    let msg = if success {
                        "Trade completed.".to_owned()
                    } else {
                        "Trade failed.".to_owned()
                    };
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(msg, MessageColor::Information));
                }
                NetworkEvent::IventoryItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .add_item(&self.async_loader, item);

                    // TODO: Update the selling items. If you pick up an item
                    // that you already have the sell window
                    // should allow you to sell the new
                    // amount of items.
                }
                NetworkEvent::ItemObtained {
                    item_id,
                    quantity,
                    is_identified,
                } => {
                    let name = self.library.get::<ItemName>(ItemNameKey { item_id, is_identified }).to_string();
                    let message = format!("You got {name} ({quantity}).");
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message, MessageColor::Information));
                }
                NetworkEvent::InventoryItemRemoved { index, amount, .. } => {
                    self.client_state.follow_mut(client_state().inventory()).remove_item(index, amount);
                }
                NetworkEvent::SkillTree { skill_information } => {
                    *self.client_state.follow_mut(client_state().skill_tree().skills()) =
                        skill_information.into_iter().map(LearnedSkill::new).collect();
                }
                NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_equipped_position(index, equipped_position);
                    let inventory = self.client_state.follow(client_state().inventory());
                    let weapon = inventory.equipped_weapon_look();
                    let left_hand = inventory.equipped_left_hand_look();
                    if let Some(player) = self.client_state.try_follow_mut(this_entity()) {
                        player.set_weapon(weapon);
                        if let Some(left) = left_hand {
                            player.set_shield(left);
                        }
                        let parts = player.get_entity_part_files(&self.library, &self.game_file_loader);
                        Self::refresh_entity_player_gear(
                            &self.async_loader,
                            &self.library,
                            &self.game_file_loader,
                            player,
                            &parts,
                        );
                        player.refresh_neutral_stance(client_tick);
                    }
                }
                NetworkEvent::ChangeJob { account_id, job_id } => {
                    // This event fires for *any* entity's base-sprite change
                    // (job changes, but also monster looks and disguises whose
                    // ids are not player jobs). Only the local player's change
                    // may rebuild the skill tree UI — the old unconditional
                    // lookup crashed the client live (2026-07-23).
                    let is_local_player = self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|entity| entity.get_entity_id().0 == account_id.0);
                    if is_local_player {
                        let layout = self.async_loader.request_skill_tree_layout_load(job_id, client_tick);
                        *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) =
                            layout.tabs.len().saturating_sub(1);
                        *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                        self.client_state
                            .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                            .clear();
                    }

                    // FIX: A job change does not automatically send packets for the
                    // inventory and for unequipping items. We should probably manually
                    // request a full list of items and the hotbar.

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    {
                        entity.set_job(&self.library, job_id);

                        if let Some(animation_data) = self.async_loader.request_animation_data_load(
                            entity.get_entity_id(),
                            entity.get_entity_type(),
                            entity.get_entity_part_files(&self.library, &self.game_file_loader),
                        ) {
                            entity.set_animation_data(animation_data);
                        }
                    }
                }
                NetworkEvent::ChangeHair { account_id, hair_id } => {
                    // Same defensiveness as ChangeJob: the entity may have
                    // despawned (or never spawned) by the time this arrives.
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    {
                        entity.set_hair(hair_id as usize);
                        let parts = entity.get_entity_part_files(&self.library, &self.game_file_loader);
                        Self::refresh_entity_head_layer(&self.async_loader, entity, &parts);
                    }
                }
                NetworkEvent::ChangeWeapon { account_id, weapon_id } => {
                    // Ammunition belongs to the weapon that fired it, so a weapon
                    // change invalidates it. Dropping it is not a fidelity loss: the
                    // server force-unequips ammo when a weapon comes off, so it
                    // already considers there to be none. Without this a cached
                    // arrow id can be drawn for a gun or shuriken — reachable
                    // because the server's force-unequip guards on
                    // `equip_index[EQI_AMMO] > 0`, and inventory slot 0 is valid,
                    // and because it does not cover huuma weapons at all.
                    self.client_state
                        .follow_mut(client_state().remote_ammunition())
                        .remove(&account_id);

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    {
                        entity.set_weapon(weapon_id);
                        let parts = entity.get_entity_part_files(&self.library, &self.game_file_loader);
                        Self::refresh_entity_weapon_layer(&self.async_loader, entity, &parts);
                        entity.refresh_neutral_stance(client_tick);
                    }
                }
                NetworkEvent::ChangeShield { account_id, shield_id } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    {
                        entity.set_shield(shield_id);
                        Self::refresh_entity_shield_layer(
                            &self.async_loader,
                            &self.library,
                            &self.game_file_loader,
                            entity,
                        );
                    } else if let Some(player) = self
                        .client_state
                        .try_follow_mut(this_entity())
                        .filter(|entity| entity.get_entity_id().0 == account_id.0)
                    {
                        player.set_shield(shield_id);
                        Self::refresh_entity_shield_layer(
                            &self.async_loader,
                            &self.library,
                            &self.game_file_loader,
                            player,
                        );
                    }
                }
                NetworkEvent::ChangeAmmunition { account_id, item_id } => {
                    // Purely a projectile-appearance hint, so unlike weapon/shield
                    // there is no sprite layer to rebuild — the value is read when a
                    // shot is fired. The local player is skipped deliberately: their
                    // own inventory is authoritative and already exact.
                    // Keyed by account id rather than stored on the entity: this
                    // broadcast routinely arrives before the entity exists, and a
                    // later respawn packet replaces the entity wholesale. Both used
                    // to discard it silently, leaving observers on the generic arrow.
                    self.client_state
                        .follow_mut(client_state().remote_ammunition())
                        .insert(account_id, item_id);
                }
                NetworkEvent::ChangeLook {
                    account_id,
                    look_type,
                    value,
                } => {
                    // Applied to whichever of the two entity homes matches, because
                    // the local player lives under `this_entity()` and everyone else
                    // in `entities()` — the split that made `set_hair` a silent
                    // no-op for every observer.
                    //
                    // A miss is safe here, unlike ammunition: all of these ride the
                    // spawn packet, so an entity that has not spawned yet gets the
                    // current value from `EntityData` when it does. No off-entity
                    // map needed.
                    let changed = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                        .map(|entity| entity.set_look(&look_type, value))
                        .or_else(|| {
                            self.client_state
                                .try_follow_mut(this_entity())
                                .filter(|entity| entity.get_entity_id().0 == account_id.0)
                                .map(|player| player.set_look(&look_type, value))
                        });

                    // Nothing composes headgear, robes or palettes into the sprite
                    // yet, so there is no layer to rebuild — the value is stored and
                    // will be picked up when rendering lands. Logged under the
                    // existing packet-log switch so the coverage is observable.
                    if changed == Some(true) && std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                        eprintln!("[packet-log] look change account={} type={look_type:?} value={value}", account_id.0);
                    }
                }
                NetworkEvent::EntityDirection {
                    entity_id,
                    direction,
                    head_direction,
                } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_direction(direction, head_direction);
                    }
                }
                NetworkEvent::EntityStopMove { entity_id, position } => {
                    // Snapping to the reported tile also clears `active_movement`,
                    // which is what stops the walk animation — otherwise the entity
                    // keeps striding toward a destination it already abandoned.
                    if let Some(map) = self.map.as_ref()
                        && let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_position(map, position, client_tick);
                    }
                }
                NetworkEvent::LoggedOut => {
                    // Close character UI *before* clearing character-scoped state.
                    // Skill Tree holds ManuallyAsserted paths into layout tabs; if we
                    // clear the tree while the window is still open, the next layout
                    // panics (M1-017). MapServerDisconnected also closes windows, but
                    // LoggedOut can land a frame earlier.
                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.client_state.follow_mut(client_state().party_state()).clear();
                    self.client_state.follow_mut(client_state().skill_tree()).clear();
                    self.client_state.follow_mut(client_state().hotbar()).clear();
                    self.networking_system.disconnect_from_map_server();
                }
                NetworkEvent::FriendRequest { requestee } => {
                    self.interface.open_window(FriendRequestWindow::new(requestee));
                }
                NetworkEvent::FriendRemoved { account_id, character_id } => {
                    self.client_state
                        .follow_mut(client_state().friend_list())
                        .retain(|friend| !(friend.account_id() == account_id && friend.character_id() == character_id));
                }
                NetworkEvent::FriendAdded { friend } => {
                    self.client_state
                        .follow_mut(client_state().friend_list())
                        .push(crate::state::friends::FriendEntry::from_friend(friend, true));
                }
                NetworkEvent::FriendOnlineStatus {
                    character_id,
                    online,
                    name,
                    ..
                } => {
                    let list = self.client_state.follow_mut(client_state().friend_list());
                    if let Some(entry) = list.iter_mut().find(|f| f.character_id() == character_id) {
                        entry.set_online(online);
                    }
                    let status = if online { "online" } else { "offline" };
                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!("Friend {name} is now {status}."),
                        MessageColor::Information,
                    ));
                }
                NetworkEvent::CreatePartyResult { result } => {
                    let message = match result {
                        0 => "Party successfully created.",
                        1 => "Party creation failed: that party name already exists.",
                        2 => "Party creation failed: you are already in a party.",
                        3 => "Party creation failed: parties are disabled on this map.",
                        _ => "Party creation failed.",
                    };

                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message.to_owned(), MessageColor::Information));
                }
                NetworkEvent::PartyInvite { party_id, party_name } => {
                    self.client_state
                        .follow_mut(client_state().party_state())
                        .set_pending_invite(party_id);
                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!("Party invite from {party_name}. Use /party accept or /party reject."),
                        MessageColor::Information,
                    ));
                }
                NetworkEvent::PartyInviteResult { character_name, result } => {
                    let message = match result {
                        0 => format!("{character_name} is already in a party."),
                        1 => format!("{character_name} rejected the party invite."),
                        2 => format!("{character_name} accepted the party invite."),
                        3 => "The party is full.".to_owned(),
                        5 => format!("{character_name} is blocking party invites."),
                        7 => format!("{character_name} is not online or does not exist."),
                        _ => format!("Party invite for {character_name} failed ({result})."),
                    };

                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message, MessageColor::Information));
                }
                NetworkEvent::PartyInvitationState { .. } => {}
                NetworkEvent::PartyList { party_name, members } => {
                    self.client_state
                        .follow_mut(client_state().party_state())
                        .set_roster(party_name, members);
                }
                NetworkEvent::PartyMemberAdded { member } => {
                    self.client_state
                        .follow_mut(client_state().party_state())
                        .add_or_update_member(member);
                }
                NetworkEvent::PartyMemberPosition { account_id, position } => {
                    self.client_state
                        .follow_mut(client_state().party_state())
                        .update_position(account_id, position);
                }
                NetworkEvent::PartyMemberHealth {
                    account_id,
                    health_points,
                    maximum_health_points,
                } => {
                    self.client_state.follow_mut(client_state().party_state()).update_health(
                        account_id,
                        health_points,
                        maximum_health_points,
                    );
                }
                NetworkEvent::PartyMemberJobAndLevel {
                    account_id,
                    job_id,
                    base_level,
                } => {
                    self.client_state
                        .follow_mut(client_state().party_state())
                        .update_job_and_level(account_id, job_id, base_level);
                }
                NetworkEvent::PartyMemberRemoved {
                    account_id,
                    character_name,
                    result,
                } => {
                    self.client_state.follow_mut(client_state().party_state()).remove_member(account_id);
                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!("{character_name} left the party ({result})."),
                        MessageColor::Information,
                    ));
                }
                NetworkEvent::PartyChatMessage { text, .. } => {
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(format!("[Party] {text}"), MessageColor::Information));
                }
                NetworkEvent::MarkMinimap {
                    marker_type,
                    position,
                    id,
                    color,
                    ..
                } => {
                    self.client_state.follow_mut(client_state().minimap()).apply_mark(
                        marker_type,
                        (position.x, position.y),
                        id,
                        color,
                        client_tick,
                    );
                }
                NetworkEvent::SkillCooldown { skill_id, until } => {
                    self.client_state.follow_mut(client_state().skill_cooldowns()).set(skill_id, until);
                }
                NetworkEvent::GainedExperience {
                    amount,
                    experience_type,
                    experience_source,
                    ..
                } => {
                    let kind = match experience_type {
                        ExperienceType::BaseExperience => "Base",
                        ExperienceType::JobExperience => "Job",
                    };
                    let source = match experience_source {
                        ragnarok_packets::ExperienceSource::Regular => "",
                        ragnarok_packets::ExperienceSource::Quest => " (quest)",
                    };
                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!("Gained {amount} {kind} EXP{source}"),
                        MessageColor::Information,
                    ));
                }
                NetworkEvent::WhisperReceived { sender_name, message, .. } => {
                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!("[Whisper] {sender_name}: {message}"),
                        MessageColor::Information,
                    ));
                }
                NetworkEvent::WhisperResult { result } => {
                    if result != 0 {
                        let message = match result {
                            1 => "Whisper failed: the character is not online.",
                            2 => "Whisper failed: you are ignored by the target.",
                            3 => "Whisper failed: the target is ignoring all whispers.",
                            _ => "Whisper failed: the character is not online.",
                        };

                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new(message.to_owned(), MessageColor::Information));
                    }
                }
                NetworkEvent::VisualEffect { effect_path, entity_id } => {
                    let effect = self.effect_loader.get_or_load(effect_path, &self.texture_loader).unwrap();
                    let frame_timer = effect.new_frame_timer();

                    self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0)),
                        Vector3::new(0.0, 9.0, 0.0),
                        // FIX: The point light id needs to be unique.
                        // The point light manager uses the id to decide which point light
                        // renders with a shadow. Having duplicate ids might cause some
                        // visual artifacts, such as flickering, as the point lights switch
                        // between shadows and no shadows.
                        PointLightId::new(entity_id.0),
                        Vector3::new(0.0, 12.0, 0.0),
                        Color::WHITE,
                        50.0,
                        false,
                        0.0,
                    )));
                }
                NetworkEvent::SpecialEffect { entity_id, effect_id } => {
                    self.spawn_special_effect(entity_id, effect_id);
                }
                NetworkEvent::AddSkillUnit {
                    entity_id,
                    unit_id,
                    position,
                } => {
                    let Some(map) = &self.map else {
                        continue;
                    };
                    let Some(world_position) = map.get_world_position(position) else {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                        continue;
                    };

                    self.spawn_skill_unit(entity_id, unit_id, world_position, client_tick);
                }
                NetworkEvent::RemoveSkillUnit { entity_id } => {
                    self.effect_holder.remove_unit(entity_id);
                    self.sprite_effects.remove_unit(entity_id);
                    self.skill_unit_registry.remove(entity_id);
                    self.active_trap_props.retain(|(id, _, _)| *id != entity_id);
                }
                NetworkEvent::GroundSkillEffect {
                    skill_id,
                    source_entity_id,
                    position,
                    ..
                } => {
                    self.execute_skill_actor_action(source_entity_id, skill_id, client_tick);

                    // The original client plays ground-cast area effects
                    // (Thunderstorm, Storm Gust) from this packet at the
                    // targeted position, independent of any damage landing.
                    if let Some((resolved, light_color, start_delay)) = ground_skill_effect(skill_id)
                        && let Some(map) = &self.map
                        && let Some(world_position) = map.get_world_position(position)
                    {
                        self.spawn_resolved_effect(
                            resolved,
                            world_position,
                            PointLightId::new(u32::from(position.x) ^ (u32::from(position.y) << 16) ^ u32::from(skill_id.0)),
                            light_color,
                            60.0,
                            start_delay,
                            client_tick,
                        );
                    }

                    if let Some(world_position) = self.map.as_ref().and_then(|map| map.get_world_position(position)) {
                        for sound in skill_presentation_recipe(skill_id).ground_sounds {
                            self.play_spatial_skill_sound(sound.resolve(), world_position);
                        }
                    }
                }
                NetworkEvent::SetFriendList { friend_list } => {
                    *self.client_state.follow_mut(client_state().friend_list()) = friend_list
                        .into_iter()
                        .map(|friend| crate::state::friends::FriendEntry::from_friend(friend, false))
                        .collect();
                }
                NetworkEvent::DisplayEmotion { entity_id, emotion } => {
                    if emote_debug_enabled() {
                        eprintln!(
                            "[emote] DisplayEmotion entity={} emotion={} data_loaded={}",
                            entity_id.0,
                            emotion,
                            self.emote_bubbles.has_animation_data()
                        );
                    }
                    self.emote_bubbles.show(entity_id, emotion, client_tick);

                    // The shared emotion sprite sheet loads lazily on the first
                    // emote; until the async load completes the chat line below
                    // is the only feedback.
                    if !self.emote_bubbles.has_animation_data()
                        && let Some(animation_data) =
                            self.async_loader
                                .request_animation_data_load(EMOTE_ANIMATION_ENTITY_ID, EntityType::Npc, vec![
                                    EMOTE_SPRITE_FILE.to_string(),
                                ])
                    {
                        self.emote_bubbles.set_animation_data(animation_data);
                    }

                    // The original client shows only the bubble, no chat text.
                    // Keep the line for players (useful chat log), but silence
                    // it for monsters and NPCs, whose scripted emotes would
                    // otherwise spam the log.
                    let is_local_entity = self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|entity| entity.get_entity_id() == entity_id);
                    let local_entity_name = is_local_entity.then(|| self.client_state.follow(client_state().player_name()).to_owned());
                    let player_name = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .filter(|entity| matches!(entity.get_entity_type(), EntityType::Player))
                        .and_then(|entity| entity.get_details())
                        .map(|n| n.split('#').next().unwrap_or("Someone").to_owned())
                        .or(local_entity_name);

                    if let Some(name) = player_name {
                        let text = match emotion_name(emotion) {
                            Some(emote) => format!("{name}: {emote}"),
                            None => format!("{name} uses emotion {emotion}."),
                        };
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new(text, MessageColor::Information));
                    }
                }
                NetworkEvent::SkillCast {
                    source_entity_id,
                    skill_id,
                    cast_ms,
                } => {
                    if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                        eprintln!(
                            "[skill-cast] skill={} source={} cast_ms={cast_ms}",
                            skill_id.0, source_entity_id.0
                        );
                    }
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                    {
                        entity.start_cast(skill_id, cast_ms, client_tick);
                    } else if let Some(entity) = self
                        .client_state
                        .try_follow_mut(this_entity())
                        .filter(|entity| entity.get_entity_id() == source_entity_id)
                    {
                        entity.start_cast(skill_id, cast_ms, client_tick);
                    }
                }
                NetworkEvent::SkillCastCancelled { source_entity_id } => {
                    if let Some(source_entity_id) = source_entity_id
                        && let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                    {
                        entity.clear_cast();
                    }

                    let clear_local = source_entity_id.is_none_or(|source_entity_id| {
                        self.client_state
                            .try_follow(this_entity())
                            .is_some_and(|entity| entity.get_entity_id() == source_entity_id)
                    });
                    if clear_local && let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.clear_cast();
                    }
                }
                NetworkEvent::SetHotkeyData { tab, hotkeys } => {
                    // FIX: Since we only have one hotbar at the moment, we ignore
                    // everything but 0.
                    if tab.0 != 0 {
                        continue;
                    }

                    if let Some(job_id) = self.client_state.try_follow(this_entity()).map(Entity::get_job_id) {
                        for (index, hotkey) in hotkeys.into_iter().take(10).enumerate() {
                            match hotkey {
                                HotkeyState::Bound(hotkey) => {
                                    // TODO: Properly distinguish between skill and item.
                                    let skill_id = SkillId(hotkey.item_or_skill_id as u16);

                                    let mut skill = self.async_loader.request_learnable_skill_load(job_id, skill_id, client_tick);
                                    skill.maximum_level.0 = hotkey.quantity_or_skill_level;

                                    self.client_state
                                        .follow_mut(client_state().hotbar())
                                        .set_slot(HotbarSlot(index as u16), skill);
                                }
                                HotkeyState::Unbound => self
                                    .client_state
                                    .follow_mut(client_state().hotbar())
                                    .unset_slot(HotbarSlot(index as u16)),
                            }
                        }
                    }
                }
                NetworkEvent::OpenShop { items } => {
                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    let count = items.len();
                    if count == 0 {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "Shop list was empty (no items from server).".to_owned(),
                            MessageColor::Information,
                        ));
                    } else {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            format!("Shop: {count} items for sale."),
                            MessageColor::Information,
                        ));
                    }

                    *self.client_state.follow_mut(client_state().shop_items()) = items
                        .into_iter()
                        .map(|item| self.async_loader.request_shop_item_metadata_load(item))
                        .collect();

                    self.interface
                        .open_window(BuyWindow::new(client_state().shop_items(), client_state().buy_cart()));
                    self.interface.open_window(BuyCartWindow::new(client_state().buy_cart()));
                }
                NetworkEvent::AskBuyOrSell { shop_id } => {
                    self.interface.open_window(BuyOrSellWindow::new(shop_id));
                }
                NetworkEvent::BuyingCompleted { result } => match result {
                    BuyShopItemsResult::Success => {
                        let _ = self.networking_system.close_shop();

                        // Clear the cart.
                        self.client_state.follow_mut(client_state().buy_cart()).clear();

                        self.interface.close_window_with_class(WindowClass::Buy);
                        self.interface.close_window_with_class(WindowClass::BuyCart);
                    }
                    BuyShopItemsResult::Error => {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Failed to buy items".to_owned(), MessageColor::Error));
                    }
                },
                NetworkEvent::SellItemList { items } => {
                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    let inventory_items = self.client_state.follow(client_state().inventory().items());
                    let sell_items: Vec<_> = items
                        .into_iter()
                        .filter_map(|item| {
                            let inventory_item = inventory_items
                                .iter()
                                .find(|inventory_item| inventory_item.index == item.inventory_index)?;

                            let name = inventory_item.metadata.name.clone();
                            let texture = inventory_item.metadata.texture.clone();
                            let quantity = match &inventory_item.details {
                                korangar_networking::InventoryItemDetails::Regular { amount, .. } => *amount,
                                korangar_networking::InventoryItemDetails::Equippable { .. } => 1,
                            };

                            Some(SellItem {
                                metadata: (ResourceMetadata { name, texture }, quantity),
                                inventory_index: item.inventory_index,
                                price: item.price,
                                overcharge_price: item.overcharge_price,
                            })
                        })
                        .collect();

                    if sell_items.is_empty() {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "Nothing sellable in this shop (quest-bound items cannot be sold).".to_owned(),
                            MessageColor::Information,
                        ));
                    }

                    *self.client_state.follow_mut(client_state().sell_items()) = sell_items;

                    self.interface
                        .open_window(SellWindow::new(client_state().sell_items(), client_state().sell_cart()));
                    self.interface.open_window(SellCartWindow::new(client_state().sell_cart()));
                }
                NetworkEvent::SellingCompleted { result } => match result {
                    SellItemsResult::Success => {
                        // Clear the cart.
                        self.client_state.follow_mut(client_state().buy_cart()).clear();

                        self.interface.close_window_with_class(WindowClass::Sell);
                        self.interface.close_window_with_class(WindowClass::SellCart);
                    }
                    SellItemsResult::Error => {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Failed to sell items".to_owned(), MessageColor::Error));
                    }
                },
                NetworkEvent::AttackFailed {
                    target_entity_id,
                    target_position,
                    player_position,
                    attack_range,
                } => {
                    // Make sure that the entity is on screen.
                    let target_on_screen = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .any(|entity| entity.get_entity_id() == target_entity_id);
                    let have_player = self.client_state.try_follow_mut(this_entity()).is_some();
                    let mut walking = false;

                    if let Some(map) = &self.map
                        && have_player
                        && target_on_screen
                        && let Some(path) =
                            self.path_finder
                                .find_walkable_path_in_range(&**map, player_position, target_position, attack_range)
                    {
                        let nearest_tile = *path.last().unwrap();

                        let _ = self.networking_system.player_move(WorldPosition {
                            x: nearest_tile.x,
                            y: nearest_tile.y,
                            direction: Direction::North,
                        });

                        *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity {
                            entity_id: target_entity_id,
                        });
                        walking = true;
                    }

                    if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
                        eprintln!(
                            "[attack-range] too far: target={target_entity_id:?} target_pos={target_position:?} \
                             player_pos={player_position:?} range={} on_screen={target_on_screen} have_player={have_player} \
                             have_map={} -> {}",
                            attack_range.0,
                            self.map.is_some(),
                            match walking {
                                true => "walking into range",
                                false => "did nothing",
                            }
                        );
                    }
                }
                NetworkEvent::UpdateSkill {
                    skill_id,
                    skill_level,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                } => {
                    self.client_state.follow_mut(client_state().skill_tree()).update_skill(
                        skill_id,
                        skill_level,
                        spell_point_cost,
                        attack_range,
                        upgradable,
                    );
                }
                NetworkEvent::RemoveSkill { skill_id } => {
                    self.client_state.follow_mut(client_state().skill_tree()).remove_skill(skill_id);
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .remove(&skill_id);
                }
                NetworkEvent::SkillCooldownList { cooldowns } => {
                    self.client_state
                        .follow_mut(client_state().skill_cooldowns())
                        .replace_from_server(cooldowns, client_tick);
                }
                NetworkEvent::AutoRunSkill {
                    skill_id,
                    skill_type,
                    skill_level,
                    spell_point_cost,
                    attack_range,
                    skill_name,
                    upgradable,
                } => {
                    self.client_state
                        .follow_mut(client_state().skill_tree())
                        .upsert_skill(LearnedSkill::new(ragnarok_packets::SkillInformation {
                            skill_id,
                            skill_type,
                            skill_level,
                            spell_point_cost,
                            attack_range,
                            skill_name,
                            upgradable: u8::from(upgradable),
                        }));
                }
                NetworkEvent::MonsterInformation {
                    job_id,
                    level,
                    size,
                    health_points,
                    defense,
                    race,
                    magic_defense,
                    element,
                    elemental_effectiveness,
                } => {
                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!(
                            "Monster #{} — Lv. {}, HP {}, DEF {}, MDEF {}, size {}, race {}, element {}; effectiveness {:?}",
                            job_id.0, level, health_points, defense, magic_defense, size, race, element, elemental_effectiveness
                        ),
                        MessageColor::Information,
                    ));
                }
                NetworkEvent::WarpList { skill_id, destinations } => {
                    self.interface.close_window_with_class(WindowClass::WarpSelection);
                    if destinations.is_empty() {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "The warp skill returned no destinations.".to_owned(),
                            MessageColor::Error,
                        ));
                    } else {
                        self.interface.open_window(WarpSelectionWindow::new(skill_id, destinations));
                    }
                }
                NetworkEvent::RefinableWeaponList { weapons } => {
                    self.interface.close_window_with_class(WindowClass::WeaponRefine);
                    if weapons.is_empty() {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "No weapons are eligible for Weapon Refine.".to_owned(),
                            MessageColor::Error,
                        ));
                    } else {
                        let weapons = weapons
                            .into_iter()
                            .map(|weapon| {
                                let name = self
                                    .library
                                    .get::<ItemName>(ItemNameKey {
                                        item_id: weapon.item_id,
                                        is_identified: true,
                                    })
                                    .to_string();
                                (weapon, name)
                            })
                            .collect();
                        self.interface.open_window(WeaponRefineWindow::new(weapons));
                    }
                }
                NetworkEvent::WeaponRefineResult { result, item_id } => {
                    self.interface.close_window_with_class(WindowClass::WeaponRefine);
                    let item_name =
                        resolve_item_name(&self.library, item_id, true).unwrap_or_else(|| format!("item #{}", item_id.0));
                    let (message, color) = match result {
                        0 => (format!("Weapon refine succeeded for {item_name}."), MessageColor::Information),
                        1 => (format!("Weapon refine failed for {item_name}."), MessageColor::Error),
                        2 => ("Weapon Refine skill level is too low.".to_owned(), MessageColor::Error),
                        3 => ("Required refining material is missing.".to_owned(), MessageColor::Error),
                        _ => (
                            format!("Weapon refine returned result {result} for {item_name}."),
                            MessageColor::Error,
                        ),
                    };
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message, color));
                }
                NetworkEvent::RepairableItemList { items } => {
                    self.interface.close_window_with_class(WindowClass::RepairWeapon);
                    if items.is_empty() {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "No broken equipment is available to repair.".to_owned(),
                            MessageColor::Error,
                        ));
                    } else {
                        let items = items
                            .into_iter()
                            .map(|item| {
                                let name = self
                                    .library
                                    .get::<ItemName>(ItemNameKey {
                                        item_id: item.item_id,
                                        is_identified: true,
                                    })
                                    .to_string();
                                (item, name)
                            })
                            .collect();
                        self.interface.open_window(RepairWeaponWindow::new(items));
                    }
                }
                NetworkEvent::ItemRepairResult { inventory_index, success } => {
                    self.interface.close_window_with_class(WindowClass::RepairWeapon);
                    let item_name = self
                        .client_state
                        .follow(client_state().inventory().items())
                        .iter()
                        .find(|item| item.index == inventory_index)
                        .map(|item| item.metadata.name.clone())
                        .unwrap_or_else(|| format!("item in slot {}", inventory_index.0));
                    let (message, color) = if success {
                        (format!("Repair succeeded for {item_name}."), MessageColor::Information)
                    } else {
                        (format!("Repair failed for {item_name}."), MessageColor::Error)
                    };
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message, color));
                }
            }
        }

        // Drain after the complete packet batch so zero-delay impacts see any
        // spawn/death/movement changes delivered in the same network update.
        self.apply_due_impacts(client_tick);

        if open_storage_ui {
            self.open_storage_window_if_needed();
        }
    }

    /// Keep the Map window chrome in sync with [`MinimapState::display_side`].
    fn sync_minimap_window_size(&mut self) {
        use crate::state::minimap::{MAX_MINIMAP_SIDE, MIN_MINIMAP_SIDE};

        const CHROME_W: f32 = 24.0;
        const CHROME_H: f32 = 56.0;
        const COORDS_HEIGHT: f32 = 18.0;
        const ZOOM_ROW_HEIGHT: f32 = 28.0;

        let side = self
            .client_state
            .follow(client_state().minimap())
            .display_side()
            .clamp(MIN_MINIMAP_SIDE, MAX_MINIMAP_SIDE);
        let width = side + CHROME_W;
        let height = side + COORDS_HEIGHT + ZOOM_ROW_HEIGHT + CHROME_H;
        self.interface
            .set_window_size_for_class(WindowClass::Minimap, ScreenSize { width, height });
    }

    /// Open the storage UI (and inventory for drag source) when
    /// Kafra/`@storage` opens.
    fn open_storage_window_if_needed(&mut self) {
        use crate::state::storage::StorageStatePathExt;

        if !self.interface.is_window_with_class_open(WindowClass::Storage) {
            self.interface
                .open_window(StorageWindow::new(client_state().storage().items(), client_state().storage()));
        }
        // Inventory is the drag source for store/retrieve.
        if !self.interface.is_window_with_class_open(WindowClass::Inventory) && self.client_state.try_follow(this_entity()).is_some() {
            self.interface.open_window(InventoryWindow::new(
                client_state().inventory().items(),
                this_player().manually_asserted(),
            ));
        }
    }

    /// Load the official minimap bitmap and Towninfo facility POIs for a map.
    ///
    /// Uses `file_exists` before loading so a missing BMP is not replaced by
    /// the generic "missing" texture (which would look like a broken map).
    fn refresh_minimap(&mut self, map_file_name: &str, map_width: u16, map_height: u16) {
        let base = normalize_map_base_name(map_file_name);

        // Official path under data\texture\유저인터페이스\map\{map}.bmp
        // (~857 maps in data.grf including indoor maps like izlude_in, prt_in).
        let texture = Self::find_minimap_texture(&self.game_file_loader, &self.texture_loader, &base);
        // Player blip must be a texture (UI rectangles flush before textures and
        // would sit under the map bitmap).
        let player_marker = self
            .texture_loader
            .get_or_load("유저인터페이스\\minimap\\player_1.bmp", ImageType::Color)
            .ok();

        let town_pois = self.library.town_pois(&base);
        let pois: Vec<_> = town_pois
            .iter()
            .map(|poi| {
                let icon = self.texture_loader.get_or_load(poi.kind.icon_texture_path(), ImageType::Color).ok();
                crate::state::minimap::MinimapPoi {
                    x: poi.x,
                    y: poi.y,
                    kind: poi.kind,
                    name: poi.name.clone(),
                    texture: icon,
                }
            })
            .collect();

        eprintln!(
            "[minimap] map={base} size={map_width}x{map_height} bmp={} pois={}",
            texture.is_some(),
            pois.len()
        );

        self.client_state
            .follow_mut(client_state().minimap())
            .set_map(base, map_width, map_height, texture, player_marker, pois);

        // Only auto-open when the player wants the minimap visible.
        let show = *self.client_state.follow(client_state().game_settings().show_minimap());
        if show && !self.interface.is_window_with_class_open(WindowClass::Minimap) {
            self.interface.open_window(MinimapWindow);
        } else if !show && self.interface.is_window_with_class_open(WindowClass::Minimap) {
            self.interface.close_window_with_class(WindowClass::Minimap);
        }
    }

    /// Resolve minimap BMP only when the archive actually contains it.
    ///
    /// `TextureLoader::get_or_load` falls back to a placeholder on miss, so we
    /// must call `file_exists` first or every map "succeeds" with the wrong
    /// image.
    fn find_minimap_texture(
        game_file_loader: &GameFileLoader,
        texture_loader: &TextureLoader,
        base: &str,
    ) -> Option<std::sync::Arc<crate::graphics::Texture>> {
        let candidates = [
            format!("유저인터페이스\\map\\{base}.bmp"),
            format!("유저인터페이스\\map\\{}.bmp", base.to_lowercase()),
            // Some custom/old clients use a flat map folder.
            format!("map\\{base}.bmp"),
            format!("map\\{}.bmp", base.to_lowercase()),
        ];

        for relative in candidates {
            let full = format!("data\\texture\\{relative}");
            if !game_file_loader.file_exists(&full) {
                // GRF paths are stored lowercased; also try lowercased full path.
                let lower = full.to_lowercase();
                if !game_file_loader.file_exists(&lower) {
                    continue;
                }
            }
            if let Ok(texture) = texture_loader.get_or_load(&relative, ImageType::Color) {
                return Some(texture);
            }
        }
        None
    }

    /// Toggle sit/stand for the local player (Insert / Home / `/sit`).
    fn toggle_sit(&mut self, client_tick: ClientTick) {
        if let Some(player) = self.client_state.try_follow_mut(this_entity()) {
            if player.is_sitting() {
                match self.networking_system.player_stand() {
                    Ok(()) => player.set_idle(client_tick),
                    Err(_) => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        "Stand failed: not connected to map server.".to_owned(),
                        MessageColor::Error,
                    )),
                }
            } else {
                match self.networking_system.player_sit() {
                    Ok(()) => player.set_sit(client_tick),
                    Err(_) => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        "Sit failed: not connected to map server.".to_owned(),
                        MessageColor::Error,
                    )),
                }
            }
        } else {
            self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                "Sit failed: no player entity.".to_owned(),
                MessageColor::Error,
            ));
        }
    }

    /// Handle inventory-related input events that were queued during the UI
    /// pass (after the main start-of-frame event drain). Leaves unrelated
    /// events in the buffer.
    fn flush_inventory_input_events(&mut self) {
        let mut remaining = Vec::new();

        for event in self.input_event_buffer.drain(..) {
            match event {
                InputEvent::DropItem { inventory_index, amount } => {
                    if amount == 0 {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Nothing to drop.".to_owned(), MessageColor::Error));
                    } else if self.networking_system.drop_item(inventory_index, amount).is_err() {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Not connected to map server.".to_owned(), MessageColor::Error));
                    }
                }
                InputEvent::ReorderInventory { from_index, to_slot } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .reorder_display(from_index, to_slot);
                }
                InputEvent::MoveItem { source, destination, item } => match (source, destination) {
                    (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                        let _ = self.networking_system.request_item_equip(item.index, position);
                    }
                    (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                        let _ = self.networking_system.request_item_unequip(item.index);
                    }
                    (ItemSource::Inventory, ItemSource::Storage) => {
                        let amount = match &item.details {
                            korangar_networking::InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
                            _ => 1,
                        };
                        let _ = self.networking_system.move_item_to_storage(item.index, amount);
                    }
                    (ItemSource::Storage, ItemSource::Inventory) => {
                        let amount = match &item.details {
                            korangar_networking::InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
                            _ => 1,
                        };
                        let _ = self.networking_system.move_item_from_storage(item.index, amount);
                    }
                    _ => {}
                },
                InputEvent::OpenItemActions { item } => {
                    self.interface.close_window_with_class(WindowClass::ItemActions);
                    self.interface.open_window(ItemActionsWindow::new(item));
                }
                InputEvent::CloseItemActions => {
                    self.interface.close_window_with_class(WindowClass::ItemActions);
                }
                InputEvent::UseItem { inventory_index } => {
                    if let Some(account_id) = self.saved_login_data.as_ref().map(|d| d.account_id) {
                        let _ = self.networking_system.use_item(inventory_index, account_id);
                    }
                }
                InputEvent::IdentifyItem { inventory_index } => {
                    let _ = self.networking_system.one_click_item_identify(inventory_index);
                }
                other => remaining.push(other),
            }
        }

        self.input_event_buffer = remaining;
    }

    /// Returns whether or not the interface is focused.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn process_user_events(
        &mut self,
        input_report: &InputReport,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] delta_time: f32,
    ) -> bool {
        self.interface.process_events(&mut self.input_event_buffer);

        // Closing the minimap with the window X must clear the persisted preference,
        // otherwise the next map load would force it open again.
        if self.map.is_some()
            && self.client_state.try_follow(this_player()).is_some()
            && *self.client_state.follow(client_state().game_settings().show_minimap())
            && !self.interface.is_window_with_class_open(WindowClass::Minimap)
        {
            *self.client_state.follow_mut(client_state().game_settings().show_minimap()) = false;
        }

        let interface_has_focus = self.interface.has_focus();

        if self.interface.get_mouse_mode().is_rotating_camera() {
            // TODO: Does this really need to be a InputEvent?
            let rotation = input_report.mouse_delta.width;
            self.input_event_buffer.push(InputEvent::RotateCamera { rotation });
        }

        if !interface_has_focus {
            self.input_system.handle_keyboard_input(
                &mut self.input_event_buffer,
                #[cfg(feature = "debug")]
                self.interface.get_mouse_mode().is_default(),
                #[cfg(feature = "debug")]
                *self.client_state.follow(client_state().render_options().use_debug_camera()),
            );
        } else {
            // Sit / hotbar still work while a UI widget is focused (e.g. chat box).
            // Menu shortcuts stay gated so they don't fire while typing.
            self.input_system.handle_game_action_keys(&mut self.input_event_buffer);
        }

        let mut toggle_sit = false;
        let mut sync_minimap = false;
        // Deferred: `enter_character_server` needs &mut self while this loop
        // already mutably drains `input_event_buffer`.
        let mut select_server: Option<CharacterServerInformation> = None;
        for event in self.input_event_buffer.drain(..) {
            match event {
                InputEvent::LogIn {
                    service_id,
                    username,
                    password,
                } => {
                    let service = self
                        .client_state
                        .follow(client_state().client_info().services())
                        .iter()
                        .find(|service| service.service_id() == service_id)
                        .unwrap();
                    let address = format!("{}:{}", service.address, service.port);
                    let socket_address = address
                        .to_socket_addrs()
                        .expect("Failed to resolve IP")
                        .next()
                        .expect("ill formatted service IP");

                    let packet_version = match service.packet_version {
                        Some(packet_version) => match packet_version {
                            PacketVersion::_20220406 => SupportedPacketVersion::_20220406,
                            PacketVersion::Unsupported(packet_version) => {
                                let message =
                                    format!("Selected server has an unsupported package version: {packet_version}");
                                // Inline (don't call show_login_error): this loop
                                // already mutably borrows self via drain.
                                eprintln!("[login] show_login_error: {message}");
                                *self.client_state.follow_mut(client_state().login_window().status_message()) =
                                    message;
                                self.interface.close_window_with_class(WindowClass::Error);
                                continue;
                            }
                        },
                        None => FALLBACK_PACKET_VERSION,
                    };

                    self.saved_login_server_address = Some(socket_address);
                    self.saved_username = username.clone();
                    self.saved_password = password.clone();
                    self.saved_packet_version = packet_version;

                    eprintln!("[login] connecting to {socket_address} as {username}");
                    self.networking_system
                        .connect_to_login_server(packet_version, socket_address, username, password);
                }
                InputEvent::SelectServer {
                    character_server_information,
                } => {
                    select_server = Some(character_server_information);
                }
                InputEvent::Respawn => {
                    let _ = self.networking_system.respawn();
                    self.interface.close_window_with_class(WindowClass::Respawn);
                }
                InputEvent::LogOut => {
                    let _ = self.networking_system.log_out();
                }
                InputEvent::LogOutCharacter => {
                    self.networking_system.disconnect_from_character_server();
                }
                InputEvent::Exit => SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst),
                InputEvent::ZoomCamera { zoom_factor } => self.player_camera.soft_zoom(zoom_factor),
                InputEvent::RotateCamera { rotation } => self.player_camera.soft_rotate(rotation),
                InputEvent::ResetCameraRotation => self.player_camera.reset_rotation(),
                InputEvent::ToggleMenuWindow => {
                    // Escape backs out of the most recent transient state first: an armed skill
                    // target, then an in-progress cast, and only then the menu (secondary cancel
                    // gesture alongside right-click).
                    if self.pending_skill.is_some() {
                        self.pending_skill = None;
                    } else if cancel_own_cast(&mut self.networking_system, &self.client_state, client_tick) {
                        // Cast aborted; Escape does not also open the menu.
                    } else if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Menu) {
                            true => self.interface.close_window_with_class(WindowClass::Menu),
                            false => self.interface.open_window(MenuWindow),
                        }
                    }
                }
                InputEvent::ToggleCharacterOverviewWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::CharacterOverview) {
                            true => self.interface.close_window_with_class(WindowClass::CharacterOverview),
                            false => self.interface.open_window(CharacterOverviewWindow::new(
                                client_state().player_name(),
                                this_player().manually_asserted().base_level(),
                                this_player().manually_asserted().job_level(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleInventoryWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Inventory) {
                            true => self.interface.close_window_with_class(WindowClass::Inventory),
                            false => self.interface.open_window(InventoryWindow::new(
                                client_state().inventory().items(),
                                this_player().manually_asserted(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleEquipmentWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Equipment) {
                            true => self.interface.close_window_with_class(WindowClass::Equipment),
                            false => self.interface.open_window(EquipmentWindow::new(client_state().inventory().items())),
                        }
                    }
                }
                InputEvent::ToggleSkillTreeWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::SkillTree) {
                            true => self.interface.close_window_with_class(WindowClass::SkillTree),
                            false => self.interface.open_window(SkillTreeWindow::new(
                                client_state().skill_tree_window(),
                                client_state().skill_tree().layout(),
                                client_state().skill_tree().skills(),
                                // Optional path: Skill Tree may still layout for a frame after
                                // logout clears `this_player` (M1-017). Do not manually_assert.
                                this_player().skill_points(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleStatsWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Stats) {
                            true => self.interface.close_window_with_class(WindowClass::Stats),
                            false => self.interface.open_window(StatsWindow::new(this_player().manually_asserted())),
                        }
                    }
                }
                InputEvent::ToggleGameSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::GameSettings) {
                    true => self.interface.close_window_with_class(WindowClass::GameSettings),
                    false => self.interface.open_window(GameSettingsWindow::new(client_state().game_settings())),
                },
                InputEvent::ToggleInterfaceSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::InterfaceSettings)
                {
                    true => self.interface.close_window_with_class(WindowClass::InterfaceSettings),
                    false => self.interface.open_window(InterfaceSettingsWindow::new(
                        client_state().interface_settings(),
                        client_state().interface_settings_capabilities(),
                    )),
                },
                InputEvent::ToggleGraphicsSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::GraphicsSettings) {
                    true => self.interface.close_window_with_class(WindowClass::GraphicsSettings),
                    false => self.interface.open_window(GraphicsSettingsWindow::new(
                        client_state().graphics_settings(),
                        client_state().graphics_settings_capabilities(),
                    )),
                },
                InputEvent::ToggleAudioSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::AudioSettings) {
                    true => self.interface.close_window_with_class(WindowClass::AudioSettings),
                    false => self
                        .interface
                        .open_window(AudioSettingsWindow::new(client_state().audio_settings())),
                },
                InputEvent::ToggleFriendListWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::FriendList) {
                            true => self.interface.close_window_with_class(WindowClass::FriendList),
                            false => self.interface.open_window(FriendListWindow::new(
                                client_state().friend_list_window(),
                                client_state().friend_list(),
                            )),
                        }
                    }
                }
                InputEvent::TogglePartyWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Party) {
                            true => self.interface.close_window_with_class(WindowClass::Party),
                            false => self.interface.open_window(PartyWindow::new(client_state().party_state())),
                        }
                    }
                }
                InputEvent::ToggleHudWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Hud) {
                            true => self.interface.close_window_with_class(WindowClass::Hud),
                            false => self.interface.open_window(HudWindow::new(
                                this_player().manually_asserted(),
                                client_state().skill_cooldowns(),
                            )),
                        }
                    }
                }
                InputEvent::CloseTopWindow => self.interface.close_top_window(&self.client_state),
                InputEvent::CloseAllOrdinaryWindows => {
                    self.interface
                        .close_all_windows_except(&[WindowClass::CharacterOverview, WindowClass::Chat]);
                }
                InputEvent::ToggleShowInterface => self.show_interface = !self.show_interface,
                InputEvent::SelectCharacter { slot } => {
                    let _ = self.networking_system.select_character(slot);
                }
                InputEvent::OpenCharacterCreationWindow { slot } => {
                    // Clear the name before opening the window.
                    self.client_state.follow_mut(client_state().create_character_name()).clear();

                    self.interface
                        .open_window(CharacterCreationWindow::new(client_state().create_character_name(), slot))
                }
                InputEvent::CreateCharacter { slot, name } => {
                    let _ = self.networking_system.create_character(slot, name);
                }
                InputEvent::DeleteCharacter { character_id } => {
                    if self.client_state.follow(client_state().currently_deleting()).is_none() {
                        let _ = self.networking_system.delete_character(character_id);
                        *self.client_state.follow_mut(client_state().currently_deleting()) = Some(character_id);
                    }
                }
                InputEvent::SwitchCharacterSlot {
                    origin_slot,
                    destination_slot,
                } => {
                    let _ = self.networking_system.switch_character_slot(origin_slot, destination_slot);
                }
                InputEvent::PlayerMove { destination } => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        let _ = self.networking_system.player_move(WorldPosition {
                            x: destination.x,
                            y: destination.y,
                            direction: Direction::North,
                        });
                    }

                    // Unbuffer any buffered action.
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;
                }
                InputEvent::PlayerInteract { entity_id } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        let _ = match entity.get_entity_type() {
                            EntityType::Npc => self.networking_system.start_dialog(entity_id),
                            EntityType::Monster => {
                                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                                let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                                if auto_attack {
                                    *buffered_action = Some(BufferedAction::AttackEntity { entity_id });
                                }

                                self.networking_system.player_attack(entity_id)
                            }
                            EntityType::Warp => self.networking_system.player_move({
                                let position = entity.get_tile_position();
                                WorldPosition {
                                    x: position.x,
                                    y: position.y,
                                    direction: Direction::North,
                                }
                            }),
                            _ => Ok(()),
                        };
                    }
                }
                InputEvent::PickUpItem { entity_id } => {
                    self.mouse_cursor.set_state(MouseCursorState::PickUpItem, client_tick);

                    if let Some(map) = &self.map {
                        let player_position = self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                        let item_position = self
                            .client_state
                            .follow(client_state().ground_items())
                            .iter()
                            .find(|item| item.get_entity_id() == entity_id)
                            .map(|item| item.get_tile_position());

                        if let (Some(player_position), Some(item_position)) = (player_position, item_position) {
                            if player_position
                                .x
                                .abs_diff(item_position.x)
                                .max(player_position.y.abs_diff(item_position.y))
                                <= ITEM_PICKUP_RANGE.0
                            {
                                let _ = self.networking_system.pick_up_item(entity_id);

                                *self.client_state.follow_mut(client_state().buffered_action()) = None;
                            } else if let Some(path) =
                                self.path_finder
                                    .find_walkable_path_in_range(&**map, player_position, item_position, ITEM_PICKUP_RANGE)
                                && let Some(nearest_tile) = path.last()
                            {
                                let _ = self.networking_system.player_move(WorldPosition {
                                    x: nearest_tile.x,
                                    y: nearest_tile.y,
                                    direction: Direction::North,
                                });

                                *self.client_state.follow_mut(client_state().buffered_action()) =
                                    Some(BufferedAction::PickUpItem { entity_id });
                            } else {
                                *self.client_state.follow_mut(client_state().buffered_action()) = None;
                            }
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::WarpToMap { map_name, position } => {
                    let _ = self.networking_system.warp_to_map(map_name, position);
                }
                InputEvent::SendMessage { text } => {
                    // Handle special client commands.
                    if text.as_str() == "/nc" {
                        let auto_attack = self.client_state.follow_mut(client_state().game_settings().auto_attack());
                        *auto_attack = !*auto_attack;
                        continue;
                    }

                    // Sit/stand toggle — works from chat when Insert is awkward under WSL.
                    if matches!(text.as_str(), "/sit" | "/stand") {
                        toggle_sit = true;
                        continue;
                    }

                    if let Some(rest) = text.strip_prefix("/emotion ").or_else(|| text.strip_prefix("/e ")) {
                        if let Ok(emotion) = rest.trim().parse::<u8>() {
                            let _ = self.networking_system.request_emotion(emotion);
                        } else {
                            self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                "Usage: /emotion <id>  (or /e <id>)".to_owned(),
                                MessageColor::Information,
                            ));
                        }
                        continue;
                    }

                    if let Some(rest) = text.strip_prefix("/identify ") {
                        if let Ok(index) = rest.trim().parse::<u16>() {
                            let _ = self
                                .networking_system
                                .request_item_identify(ragnarok_packets::InventoryIndex(index));
                        } else {
                            self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                "Usage: /identify <inventory_index>".to_owned(),
                                MessageColor::Information,
                            ));
                        }
                        continue;
                    }

                    if let Some(rest) = text.strip_prefix("/store ") {
                        let mut parts = rest.trim().split_whitespace();
                        if let Some(index) = parts.next().and_then(|s| s.parse::<u16>().ok()) {
                            let amount = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                            let _ = self
                                .networking_system
                                .move_item_to_storage(ragnarok_packets::InventoryIndex(index), amount);
                        } else {
                            self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                "Usage: /store <inventory_index> [amount]".to_owned(),
                                MessageColor::Information,
                            ));
                        }
                        continue;
                    }

                    if let Some(rest) = text.strip_prefix("/retrieve ") {
                        let mut parts = rest.trim().split_whitespace();
                        if let Some(index) = parts.next().and_then(|s| s.parse::<u16>().ok()) {
                            let amount = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                            let _ = self
                                .networking_system
                                .move_item_from_storage(ragnarok_packets::InventoryIndex(index), amount);
                        } else {
                            self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                "Usage: /retrieve <storage_index> [amount]".to_owned(),
                                MessageColor::Information,
                            ));
                        }
                        continue;
                    }

                    if let Some(rest) = text.strip_prefix("/trade ") {
                        let rest = rest.trim();
                        let (command, arguments) = rest.split_once(' ').unwrap_or((rest, ""));
                        let arguments = arguments.trim();
                        match command {
                            "request" if !arguments.is_empty() => {
                                if let Ok(aid) = arguments.parse::<u32>() {
                                    let _ = self.networking_system.request_trade(ragnarok_packets::AccountId(aid));
                                } else {
                                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                        "Usage: /trade request <account_id>".to_owned(),
                                        MessageColor::Information,
                                    ));
                                }
                            }
                            "add" => {
                                let mut parts = arguments.split_whitespace();
                                if let Some(index) = parts.next().and_then(|s| s.parse::<u16>().ok()) {
                                    let amount = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                                    let _ = self
                                        .networking_system
                                        .trade_add_item(ragnarok_packets::InventoryIndex(index), amount);
                                } else {
                                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                        "Usage: /trade add <inventory_index> [amount]".to_owned(),
                                        MessageColor::Information,
                                    ));
                                }
                            }
                            "zeny" => {
                                if let Ok(amount) = arguments.parse::<u32>() {
                                    let _ = self.networking_system.trade_add_zeny(amount);
                                    self.client_state.follow_mut(client_state().trade_state()).set_our_zeny(amount);
                                } else {
                                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                        "Usage: /trade zeny <amount>".to_owned(),
                                        MessageColor::Information,
                                    ));
                                }
                            }
                            "ok" => {
                                let _ = self.networking_system.trade_ok();
                            }
                            "commit" => {
                                let _ = self.networking_system.trade_commit();
                            }
                            "cancel" => {
                                let _ = self.networking_system.trade_cancel();
                            }
                            _ => {
                                self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                    "Usage: /trade request|add|zeny|ok|commit|cancel …".to_owned(),
                                    MessageColor::Information,
                                ));
                            }
                        }
                        continue;
                    }

                    if let Some(message) = text.strip_prefix("/p ") {
                        let player_name = self.client_state.follow(client_state().player_name()).to_owned();
                        let _ = self.networking_system.send_party_chat_message(&player_name, message);
                        continue;
                    }

                    if let Some(rest) = text.strip_prefix("/w ").or_else(|| text.strip_prefix("/whisper ")) {
                        if let Some((target_name, message)) = rest.trim().split_once(' ')
                            && !target_name.is_empty()
                            && !message.trim().is_empty()
                        {
                            let _ = self.networking_system.send_whisper_message(target_name, message.trim());
                        } else {
                            self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                "Usage: /w <name> <message>".to_owned(),
                                MessageColor::Information,
                            ));
                        }
                        continue;
                    }

                    if let Some(rest) = text.strip_prefix("/party ") {
                        let rest = rest.trim();
                        let (command, arguments) = rest.split_once(' ').unwrap_or((rest, ""));
                        let arguments = arguments.trim();

                        match command {
                            "create" if !arguments.is_empty() => {
                                let _ = self.networking_system.create_party(arguments);
                            }
                            "invite" if !arguments.is_empty() => {
                                let _ = self.networking_system.invite_to_party(arguments);
                            }
                            "accept" => {
                                let party_id = if arguments.is_empty() {
                                    self.client_state.follow(client_state().party_state()).pending_invite_id()
                                } else {
                                    arguments.parse::<u32>().ok().map(PartyId)
                                };

                                match party_id {
                                    Some(party_id) => {
                                        self.client_state.follow_mut(client_state().party_state()).clear_pending_invite();
                                        let _ = self.networking_system.accept_party_invite(party_id);
                                    }
                                    None => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                        "Usage: /party accept <party id>, or accept while an invite is pending.".to_owned(),
                                        MessageColor::Information,
                                    )),
                                }
                            }
                            "reject" => {
                                let party_id = if arguments.is_empty() {
                                    self.client_state.follow(client_state().party_state()).pending_invite_id()
                                } else {
                                    arguments.parse::<u32>().ok().map(PartyId)
                                };

                                match party_id {
                                    Some(party_id) => {
                                        self.client_state.follow_mut(client_state().party_state()).clear_pending_invite();
                                        let _ = self.networking_system.reject_party_invite(party_id);
                                    }
                                    None => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                        "Usage: /party reject <party id>, or reject while an invite is pending.".to_owned(),
                                        MessageColor::Information,
                                    )),
                                }
                            }
                            "leave" => {
                                let _ = self.networking_system.leave_party();
                            }
                            "block" => match arguments {
                                "on" | "true" | "1" => {
                                    let _ = self.networking_system.set_party_invitation_block(true);
                                }
                                "off" | "false" | "0" => {
                                    let _ = self.networking_system.set_party_invitation_block(false);
                                }
                                _ => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                    "Usage: /party block <on|off>".to_owned(),
                                    MessageColor::Information,
                                )),
                            },
                            _ => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                                "Usage: /party create <name>, /party invite <name>, /party accept <id>, /party reject <id>, /party leave, \
                                 /party block <on|off>"
                                    .to_owned(),
                                MessageColor::Information,
                            )),
                        }

                        continue;
                    }

                    // Atcommands (@dm, @blvl, …) never rebroadcast as public chat, so echo
                    // the request locally; server feedback arrives via 0x017F (dispbottom).
                    if text.starts_with('@') {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new(format!("→ {text}"), MessageColor::Information));
                    }

                    let _ = self
                        .networking_system
                        .send_chat_message(self.client_state.follow(client_state().player_name()), &text);
                }
                InputEvent::UseEmotion { emotion } => {
                    let _ = self.networking_system.request_emotion(emotion);
                }
                InputEvent::ToggleSit => {
                    toggle_sit = true;
                }
                InputEvent::MinimapZoomIn => {
                    self.client_state.follow_mut(client_state().minimap()).zoom_by(24.0);
                    sync_minimap = true;
                }
                InputEvent::MinimapZoomOut => {
                    self.client_state.follow_mut(client_state().minimap()).zoom_by(-24.0);
                    sync_minimap = true;
                }
                InputEvent::ToggleMinimapWindow => {
                    // Only while actually in-game (not character select / main menu map).
                    if self.map.is_some() && self.client_state.try_follow(this_player()).is_some() {
                        let open = self.interface.is_window_with_class_open(WindowClass::Minimap);
                        *self.client_state.follow_mut(client_state().game_settings().show_minimap()) = !open;
                        match open {
                            true => self.interface.close_window_with_class(WindowClass::Minimap),
                            false => self.interface.open_window(MinimapWindow),
                        }
                    }
                }
                InputEvent::NextDialog { npc_id } => {
                    // Drop the Next button immediately so double-clicks cannot
                    // send a second packet while the server is mid-script.
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .clear_transient_controls();
                    let _ = self.networking_system.next_dialog(npc_id);
                }
                InputEvent::CloseDialog { npc_id } => {
                    let _ = self.networking_system.close_dialog(npc_id);
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);
                }
                InputEvent::ChooseDialogOption { npc_id, option } => {
                    // Remove choice buttons *before* the network round-trip.
                    // Stale buttons that fire after the server left `select()`
                    // cause Hercules to GM-kick: "Invalid menu selection ...
                    // valid range is [1..0]".
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .clear_transient_controls();
                    let _ = self.networking_system.choose_dialog_option(npc_id, option);

                    if option == -1 {
                        self.client_state.follow_mut(client_state().dialog_window()).end();
                        self.interface.close_window_with_class(WindowClass::Dialog);
                    }
                }
                InputEvent::SubmitDialogNumber { npc_id, value } => {
                    let _ = self.networking_system.submit_dialog_number(npc_id, value);
                    self.client_state.follow_mut(client_state().dialog_window()).finish_input();
                }
                InputEvent::SubmitDialogString { npc_id, text } => {
                    let _ = self.networking_system.submit_dialog_string(npc_id, text);
                    self.client_state.follow_mut(client_state().dialog_window()).finish_input();
                }
                InputEvent::MoveItem { source, destination, item } => match (source, destination) {
                    (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                        let _ = self.networking_system.request_item_equip(item.index, position);
                    }
                    (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                        let _ = self.networking_system.request_item_unequip(item.index);
                    }
                    (ItemSource::Inventory, ItemSource::Storage) => {
                        let amount = match &item.details {
                            korangar_networking::InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
                            _ => 1,
                        };
                        let _ = self.networking_system.move_item_to_storage(item.index, amount);
                    }
                    (ItemSource::Storage, ItemSource::Inventory) => {
                        let amount = match &item.details {
                            korangar_networking::InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
                            _ => 1,
                        };
                        let _ = self.networking_system.move_item_from_storage(item.index, amount);
                    }
                    _ => {}
                },
                InputEvent::UseItem { inventory_index } => {
                    if let Some(account_id) = self.saved_login_data.as_ref().map(|d| d.account_id) {
                        let _ = self.networking_system.use_item(inventory_index, account_id);
                    }
                }
                InputEvent::DropItem { inventory_index, amount } => {
                    if amount == 0 {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Nothing to drop.".to_owned(), MessageColor::Error));
                    } else if self.networking_system.drop_item(inventory_index, amount).is_err() {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Not connected to map server.".to_owned(), MessageColor::Error));
                    }
                }
                InputEvent::ReorderInventory { from_index, to_slot } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .reorder_display(from_index, to_slot);
                }
                InputEvent::OpenItemActions { item } => {
                    self.interface.close_window_with_class(WindowClass::ItemActions);
                    self.interface.open_window(ItemActionsWindow::new(item));
                }
                InputEvent::CloseItemActions => {
                    self.interface.close_window_with_class(WindowClass::ItemActions);
                }
                InputEvent::IdentifyItem { inventory_index } => {
                    let _ = self.networking_system.one_click_item_identify(inventory_index);
                }
                InputEvent::IdentifyCancel => {
                    let _ = self.networking_system.cancel_item_identify();
                    self.client_state.follow_mut(client_state().identify_state()).clear();
                    self.interface.close_window_with_class(WindowClass::Identify);
                }
                InputEvent::SelectWarpDestination { skill_id, map_name } => {
                    self.interface.close_window_with_class(WindowClass::WarpSelection);
                    if self.networking_system.select_warp_destination(skill_id, map_name).is_err() {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "Could not select warp destination: not connected to the map server.".to_owned(),
                            MessageColor::Error,
                        ));
                    }
                }
                InputEvent::CancelWarpSelection { skill_id } => {
                    self.interface.close_window_with_class(WindowClass::WarpSelection);
                    let _ = self.networking_system.cancel_warp_selection(skill_id);
                }
                InputEvent::RefineWeapon { inventory_index } => {
                    self.interface.close_window_with_class(WindowClass::WeaponRefine);
                    if self.networking_system.request_weapon_refine(inventory_index).is_err() {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "Could not request weapon refine: not connected to the map server.".to_owned(),
                            MessageColor::Error,
                        ));
                    }
                }
                InputEvent::CancelWeaponRefine => {
                    self.interface.close_window_with_class(WindowClass::WeaponRefine);
                    let _ = self.networking_system.cancel_weapon_refine();
                }
                InputEvent::RepairItem { item } => {
                    self.interface.close_window_with_class(WindowClass::RepairWeapon);
                    if self.networking_system.request_item_repair(item).is_err() {
                        self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "Could not request item repair: not connected to the map server.".to_owned(),
                            MessageColor::Error,
                        ));
                    }
                }
                InputEvent::CancelItemRepair => {
                    self.interface.close_window_with_class(WindowClass::RepairWeapon);
                    let _ = self.networking_system.cancel_item_repair();
                }
                InputEvent::TradeAccept => {
                    let _ = self.networking_system.accept_trade();
                    self.interface.close_window_with_class(WindowClass::TradeRequest);
                }
                InputEvent::TradeReject => {
                    let _ = self.networking_system.reject_trade();
                    self.client_state.follow_mut(client_state().trade_state()).clear_pending();
                    self.interface.close_window_with_class(WindowClass::TradeRequest);
                }
                InputEvent::TradeOk => {
                    let _ = self.networking_system.trade_ok();
                }
                InputEvent::TradeCommit => {
                    let _ = self.networking_system.trade_commit();
                }
                InputEvent::TradeCancel => {
                    let _ = self.networking_system.trade_cancel();
                    self.client_state.follow_mut(client_state().trade_state()).clear();
                    self.interface.close_window_with_class(WindowClass::Trade);
                }
                InputEvent::CloseStorage => {
                    let _ = self.networking_system.close_storage();
                }
                InputEvent::MoveSkill {
                    source,
                    destination,
                    skill,
                } => match (source, destination) {
                    (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .update_slot(&mut self.networking_system, slot, skill);
                    }
                    (SkillSource::Hotbar { slot }, SkillSource::SkillTree) => {
                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .clear_slot(&mut self.networking_system, slot);
                    }
                    (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                        self.client_state.follow_mut(client_state().hotbar()).swap_slot(
                            &mut self.networking_system,
                            source_slot,
                            destination_slot,
                        );
                    }
                    _ => {}
                },
                InputEvent::AssignSkillToHotbar { skill } => {
                    let slot = self.client_state.follow(client_state().hotbar()).first_empty_slot();
                    match slot {
                        Some(slot) => {
                            self.client_state
                                .follow_mut(client_state().hotbar())
                                .update_slot(&mut self.networking_system, slot, skill)
                        }
                        None => self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                            "The hotbar is full. Clear a slot or drag the skill onto a slot to replace it.".to_owned(),
                            MessageColor::Error,
                        )),
                    }
                }
                InputEvent::CastSkillAtEntity {
                    skill_id,
                    skill_level,
                    attack_range,
                    entity_id,
                } => cast_or_path_entity_skill(
                    &mut self.networking_system,
                    &mut self.client_state,
                    self.map.as_deref(),
                    &mut self.path_finder,
                    skill_id,
                    skill_level,
                    attack_range,
                    entity_id,
                ),
                InputEvent::CastSkillAtTile {
                    skill_id,
                    skill_level,
                    attack_range,
                    tile,
                } => cast_or_path_ground_skill(
                    &mut self.networking_system,
                    &mut self.client_state,
                    self.map.as_deref(),
                    &mut self.path_finder,
                    skill_id,
                    skill_level,
                    attack_range,
                    tile,
                ),
                InputEvent::CastSkill { slot } => {
                    // Resolve the slot to owned data under an immutable borrow, then act — so the
                    // cast / arm / chat-feedback below can borrow self mutably without conflict.
                    let learnable_skill = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).clone();
                    // Cast at the character's CURRENT learned level, never `learnable_skill.maximum_level`
                    // — that field is whatever level got persisted into the hotkey slot at drag-time
                    // (see `Hotbar::update_slot`), which can momentarily disagree with the live skill
                    // list right after a job change / `@allskill` (server then rejects with "Skill Level
                    // is not high enough", intermittent — a retry works only because the client's skill
                    // list catches up by then). The learned-skill list is the live source of truth.
                    let skill_targeting = learnable_skill.as_ref().and_then(|learnable_skill| {
                        self.client_state
                            .follow(client_state().skill_tree().skills())
                            .iter()
                            .find(|learned_skill| learned_skill.skill_id == learnable_skill.skill_id && learned_skill.skill_level.0 > 0)
                            .map(|learned_skill| (learned_skill.skill_type, learned_skill.attack_range, learned_skill.skill_level))
                    });

                    if let (Some(learnable_skill), Some((skill_type, attack_range, skill_level))) = (learnable_skill, skill_targeting) {
                        match skill_type {
                            SkillType::Passive => {}
                            SkillType::SelfCast => {
                                let this_entity_id = self.client_state.follow(this_entity().manually_asserted()).get_entity_id();
                                match learnable_skill.skill_id == ROLLING_CUTTER_ID {
                                    true => {
                                        let _ = self.networking_system.cast_channeling_skill(
                                            learnable_skill.skill_id,
                                            skill_level,
                                            this_entity_id,
                                        );
                                    }
                                    false => {
                                        let _ = self.networking_system.cast_skill(
                                            learnable_skill.skill_id,
                                            skill_level,
                                            this_entity_id,
                                        );
                                    }
                                }
                            }
                            SkillType::Support => {
                                // Support keeps its self-target fallback: cast on the hovered entity,
                                // else on self. Self-buffs like Heal/Blessing can't reliably be aimed
                                // by clicking your own sprite, so they must not require a target.
                                let target_id = match input_report.mouse_target {
                                    PickerTarget::Entity(entity_id) => entity_id,
                                    _ => self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                                };
                                // Routed through the pathing cast for the same reason Attack is:
                                // Hercules drops an out-of-range support cast with no failure
                                // message either (`unit.c` `unit_skilluse_id2`), so healing an ally
                                // a few cells too far away did nothing at all. A self-target is
                                // always at distance 0, so it still casts instantly.
                                cast_or_path_entity_skill(
                                    &mut self.networking_system,
                                    &mut self.client_state,
                                    self.map.as_deref(),
                                    &mut self.path_finder,
                                    learnable_skill.skill_id,
                                    skill_level,
                                    attack_range,
                                    target_id,
                                );
                            }
                            SkillType::Attack => {
                                // Entity-target: fast-cast if the cursor is already over a target,
                                // otherwise arm and wait for the next left-click to pick one.
                                let pending = PendingSkill {
                                    skill_id: learnable_skill.skill_id,
                                    skill_level,
                                    skill_type,
                                    attack_range,
                                    skill_name: learnable_skill.skill_name.clone(),
                                };
                                match resolve_pending_cast(pending.skill_type, input_report.mouse_target) {
                                    PendingCastResolution::CastEntity(entity_id) => cast_or_path_entity_skill(
                                        &mut self.networking_system,
                                        &mut self.client_state,
                                        self.map.as_deref(),
                                        &mut self.path_finder,
                                        pending.skill_id,
                                        pending.skill_level,
                                        pending.attack_range,
                                        entity_id,
                                    ),
                                    _ => {
                                        announce_armed_skill(&mut self.client_state, &pending.skill_name);
                                        self.pending_skill = Some(pending);
                                    }
                                }
                            }
                            SkillType::Ground | SkillType::Trap => {
                                // Ground-target: always arm so the player aims the placement reticle
                                // and clicks where the AoE lands, rather than dropping it instantly at
                                // wherever the cursor happens to sit when the key is pressed.
                                announce_armed_skill(&mut self.client_state, &learnable_skill.skill_name);
                                self.pending_skill = Some(PendingSkill {
                                    skill_id: learnable_skill.skill_id,
                                    skill_level,
                                    skill_type,
                                    attack_range,
                                    skill_name: learnable_skill.skill_name.clone(),
                                });
                            }
                        }
                    }
                }
                InputEvent::StopSkill { slot } => {
                    if let Some(skill) = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref()
                        && skill.skill_id == ROLLING_CUTTER_ID
                    {
                        let _ = self.networking_system.stop_channeling_skill(skill.skill_id);
                    }
                }
                InputEvent::AddFriend { character_name } => {
                    if character_name.len() > 24 {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] friend name {} is too long", "error".red(), character_name.magenta());
                    } else {
                        let _ = self.networking_system.add_friend(character_name);
                    }
                }
                InputEvent::RemoveFriend { account_id, character_id } => {
                    let _ = self.networking_system.remove_friend(account_id, character_id);
                }
                InputEvent::RejectFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.reject_friend_request(account_id, character_id);
                    self.interface.close_window_with_class(WindowClass::FriendRequest);
                }
                InputEvent::AcceptFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.accept_friend_request(account_id, character_id);
                    self.interface.close_window_with_class(WindowClass::FriendRequest);
                }
                InputEvent::BuyItems { items } => {
                    let _ = self.networking_system.purchase_items(items);
                }
                InputEvent::CloseShop => {
                    let _ = self.networking_system.close_shop();

                    // Clear the carts.
                    self.client_state.follow_mut(client_state().buy_cart()).clear();
                    self.client_state.follow_mut(client_state().sell_cart()).clear();

                    self.interface.close_window_with_class(WindowClass::Buy);
                    self.interface.close_window_with_class(WindowClass::BuyCart);
                    self.interface.close_window_with_class(WindowClass::Sell);
                    self.interface.close_window_with_class(WindowClass::SellCart);
                }
                InputEvent::BuyOrSell { shop_id, buy_or_sell } => {
                    let _ = self.networking_system.select_buy_or_sell(shop_id, buy_or_sell);
                    self.interface.close_window_with_class(WindowClass::BuyOrSell);
                }
                InputEvent::SellItems { items } => {
                    let _ = self.networking_system.sell_items(items);
                }
                InputEvent::StatUp { stat_type } => {
                    let _ = self.networking_system.request_stat_up(stat_type);
                }
                InputEvent::DistributePointsForSkill { skill_id } => {
                    if let Some(available_skill_points) = self.client_state.try_follow(this_player().skill_points()).copied() {
                        let job_id = self.client_state.follow(this_entity().manually_asserted()).get_job_id();
                        let pending_skill_points = self
                            .client_state
                            .follow(client_state().skill_tree_window().pending_skill_points())
                            .len();
                        let available_skill_points = (available_skill_points as usize).saturating_sub(pending_skill_points);

                        let skill_information = self.library.get::<SkillListInformation>(skill_id);
                        let learned_skills = self.client_state.follow(client_state().skill_tree().skills());

                        let mut new_skill_points = self
                            .client_state
                            .follow(client_state().skill_tree_window().pending_skill_points())
                            .clone();

                        let current_skill_level = learned_skills
                            .iter()
                            .find(|skill| skill.skill_id == skill_id)
                            .map(|skill| skill.skill_level.0)
                            .unwrap_or_default()
                            + new_skill_points
                                .iter()
                                .filter(|pending_skill_level| **pending_skill_level == skill_id)
                                .count() as u16;

                        // If the skill is already at max level we don't do anything
                        if current_skill_level < skill_information.maximum_level.0 {
                            let target_skill_level = SkillLevel(current_skill_level + 1);

                            bring_skill_to_level(
                                &mut new_skill_points,
                                &self.library,
                                learned_skills,
                                job_id,
                                skill_id,
                                target_skill_level,
                                available_skill_points,
                            );

                            *self
                                .client_state
                                .follow_mut(client_state().skill_tree_window().pending_skill_points()) = new_skill_points;
                        }
                    }
                }
                InputEvent::LevelUpSkills { skill_ids } => {
                    for skill_id in skill_ids {
                        if self.networking_system.level_up_skill(skill_id).is_err() {
                            break;
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ReloadLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    *self.client_state.follow_mut(client_state().localization()) =
                        Localization::load_language(&self.game_file_loader, language);
                }
                #[cfg(feature = "debug")]
                InputEvent::SaveLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    self.client_state.follow(client_state().localization()).save_language(language);
                }
                #[cfg(feature = "debug")]
                InputEvent::OpenMarkerDetails { marker_identifier } => {
                    if let Some(map) = &self.map {
                        match marker_identifier {
                            MarkerIdentifier::Object(key) => {
                                let inspecting_objects = self.client_state.follow_mut(client_state().inspecting_objects());
                                let object = map.get_object(key);
                                let object_path = state::prepare_object_inspection(inspecting_objects, object);

                                self.interface.open_state_window(object_path);
                            }
                            MarkerIdentifier::LightSource(key) => {
                                let inspecting_lights = self.client_state.follow_mut(client_state().inspecting_light_sources());
                                let light_source = map.get_light_source(key);
                                let light_source_path = state::prepare_light_source_inspection(inspecting_lights, light_source);

                                self.interface.open_state_window(light_source_path);
                            }
                            MarkerIdentifier::SoundSource(index) => {
                                let inspecting_sounds = self.client_state.follow_mut(client_state().inspecting_sound_sources());
                                let sound_source = map.get_sound_source(index);
                                let sound_source_path = state::prepare_sound_source_inspection(inspecting_sounds, sound_source);

                                self.interface.open_state_window(sound_source_path);
                            }
                            MarkerIdentifier::EffectSource(index) => {
                                let inspecting_effects = self.client_state.follow_mut(client_state().inspecting_effect_sources());
                                let effect_source = map.get_effect_source(index);
                                let effect_source_path = state::prepare_effect_source_inspection(inspecting_effects, effect_source);

                                self.interface.open_state_window(effect_source_path);
                            }
                            MarkerIdentifier::Particle(..) => {
                                // TODO:
                            }
                            MarkerIdentifier::Entity(index) => {
                                let entity_id = self
                                    .client_state
                                    .try_follow(client_state().entities().index(index as usize))
                                    .expect("entity should exist")
                                    .get_entity_id();

                                // This can technically still be `None`, violating the API but we handle this
                                // case in the state window.
                                let entity_path = client_state().entities().lookup(entity_id).manually_asserted();

                                self.interface.open_state_window(entity_path);
                            }
                            MarkerIdentifier::Shadow(..) => {
                                // TODO:
                            }
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleRenderOptionsWindow => match self.interface.is_window_with_class_open(WindowClass::RenderOptions) {
                    true => self.interface.close_window_with_class(WindowClass::RenderOptions),
                    false => self
                        .interface
                        .open_window(RenderOptionsWindow::new(client_state().render_options())),
                },
                #[cfg(feature = "debug")]
                InputEvent::OpenMapDataWindow => {
                    if let Some(map) = self.map.as_ref() {
                        let inspecting_maps = self.client_state.follow_mut(client_state().inspecting_maps());
                        let map_data_path = state::prepare_map_inspection(inspecting_maps, map.get_map_data());

                        self.interface.open_state_window(map_data_path);
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleClientStateInspectorWindow => {
                    match self.interface.is_window_with_class_open(WindowClass::ClientStateInspector) {
                        true => self.interface.close_window_with_class(WindowClass::ClientStateInspector),
                        false => self.interface.open_state_window_mut(client_state()),
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleMapsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Maps) {
                            true => self.interface.close_window_with_class(WindowClass::Maps),
                            false => self.interface.open_window(MapsWindow),
                        }
                    }
                }
                InputEvent::ToggleCommandsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Commands) {
                            true => self.interface.close_window_with_class(WindowClass::Commands),
                            false => self.interface.open_window(CommandsWindow::new(client_state().commands_window())),
                        }
                    }
                }
                InputEvent::ToggleBestiaryWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Bestiary) {
                            true => self.interface.close_window_with_class(WindowClass::Bestiary),
                            false => self.interface.open_window(BestiaryWindow::new(client_state().bestiary_window())),
                        }
                    }
                }
                InputEvent::ToggleLootWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::DmLoot) {
                            true => self.interface.close_window_with_class(WindowClass::DmLoot),
                            false => self.interface.open_window(LootGeneratorWindow::new(client_state().loot_window())),
                        }
                    }
                }
                InputEvent::ToggleDiceWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Dice) {
                            true => self.interface.close_window_with_class(WindowClass::Dice),
                            false => self.interface.open_window(DiceWindow::new(client_state().dice_window())),
                        }
                    }
                }
                InputEvent::ToggleEmoteWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Emotes) {
                            true => self.interface.close_window_with_class(WindowClass::Emotes),
                            false => self.interface.open_window(EmoteWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleThemeInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::ThemeInspector) {
                    true => self.interface.close_window_with_class(WindowClass::ThemeInspector),
                    false => self.interface.open_window(ThemeInspectorWindow::new(
                        client_state().theme_inspector_window(),
                        client_state().menu_theme(),
                        client_state().in_game_theme(),
                        client_state().world_theme(),
                    )),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleProfilerWindow => match self.interface.is_window_with_class_open(WindowClass::Profiler) {
                    true => self.interface.close_window_with_class(WindowClass::Profiler),
                    false => self.interface.open_window(ProfilerWindow::new(client_state().profiler_window())),
                },
                #[cfg(feature = "debug")]
                InputEvent::TogglePacketInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::PacketInspector) {
                    true => self.interface.close_window_with_class(WindowClass::PacketInspector),
                    false => self
                        .interface
                        .open_window(PacketInspectorWindow::new(client_state().packet_history())),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleCacheStatisticsWindow => match self.interface.is_window_with_class_open(WindowClass::CacheStatistics) {
                    true => self.interface.close_window_with_class(WindowClass::CacheStatistics),
                    false => self.interface.open_state_window(client_state().cache_statistics()),
                },
                #[cfg(feature = "debug")]
                InputEvent::CameraLookAround { offset } => self.debug_camera.look_around(offset),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveForward => self.debug_camera.move_forward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveBackward => self.debug_camera.move_backward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveLeft => self.debug_camera.move_left(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveRight => self.debug_camera.move_right(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveUp => self.debug_camera.move_up(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraAccelerate => self.debug_camera.accelerate(),
                #[cfg(feature = "debug")]
                InputEvent::CameraDecelerate => self.debug_camera.decelerate(),
                #[cfg(feature = "debug")]
                InputEvent::InspectFrame { measurement } => self.interface.open_window(FrameInspectorWindow::new(measurement)),
            }
        }

        if let Some(character_server_information) = select_server {
            self.enter_character_server(character_server_information);
        }

        if toggle_sit {
            self.toggle_sit(client_tick);
        }

        if sync_minimap {
            self.sync_minimap_window_size();
        }

        // If the player closed the dialog with the window X, notify the server.
        // Without this the map-server keeps npc_id set and every other NPC click
        // returns MSG_BUSY (message 1923 / 0x783).
        self.reconcile_dialog_window_closed();

        interface_has_focus
    }

    /// When the dialog window is gone but client state still thinks a
    /// conversation is active, send `CloseDialog` and clear local state.
    fn reconcile_dialog_window_closed(&mut self) {
        let dialog_open = self.interface.is_window_with_class_open(WindowClass::Dialog);
        let active = self.client_state.follow(client_state().dialog_window()).is_active();
        if active && !dialog_open {
            let npc_id = self.client_state.follow(client_state().dialog_window()).npc_id();
            if npc_id.0 != 0 {
                let _ = self.networking_system.close_dialog(npc_id);
            }
            self.client_state.follow_mut(client_state().dialog_window()).end();
        }
    }

    #[cfg(feature = "debug")]
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_debug_windows(&mut self, delta_time: f64) {
        let is_packet_inspector_open = self.interface.is_window_with_class_open(WindowClass::PacketInspector);
        self.client_state
            .follow_mut(client_state().packet_history())
            .update(is_packet_inspector_open);

        self.client_state.follow_mut(client_state().cache_statistics()).update(
            delta_time,
            &self.map_loader,
            &self.texture_loader,
            &self.sprite_loader,
            &self.font_loader,
            &self.audio_engine,
            &self.action_loader,
            &self.animation_loader,
            &self.effect_loader,
        );
    }

    /// Phase C4 / D: swap weapon-family layers (base, trails, dual off-hand)
    /// when body+head are already loaded. Falls back to a full part-list
    /// reload if partial swap is impossible.
    ///
    /// Paths are resolved by content (`인간족\{job}\…`), not `parts[2]`: when a
    /// shield is equipped without a weapon, index 2 is the shield and treating
    /// it as a weapon would drop the sword on the next equip refresh.
    fn refresh_entity_weapon_layer(async_loader: &AsyncLoader, entity: &mut Entity, entity_part_files: &[String]) {
        let weapon_paths = crate::world::weapon_paths_from_entity_parts(entity_part_files);
        if let Some(current) = entity.animation_data()
            && let Some(updated) = async_loader.apply_weapon_layers_swap(&current, &weapon_paths)
        {
            entity.set_animation_data(updated);
            return;
        }
        if let Some(animation_data) = async_loader.request_animation_data_load(
            entity.get_entity_id(),
            entity.get_entity_type(),
            entity_part_files.to_vec(),
        ) {
            entity.set_animation_data(animation_data);
        }
    }

    /// Local-player equip path: refresh weapon then shield from the same part
    /// list so dual-equip (sword + Guard) never loses either layer.
    fn refresh_entity_player_gear(
        async_loader: &AsyncLoader,
        library: &Library,
        game_file_loader: &GameFileLoader,
        entity: &mut Entity,
        entity_part_files: &[String],
    ) {
        Self::refresh_entity_weapon_layer(async_loader, entity, entity_part_files);
        Self::refresh_entity_shield_layer(async_loader, library, game_file_loader, entity);
    }

    /// Phase C4: swap only the head layer on hair change.
    fn refresh_entity_head_layer(async_loader: &AsyncLoader, entity: &mut Entity, entity_part_files: &[String]) {
        let Some(head_path) = entity_part_files.get(1).map(|path| path.as_str()) else {
            if let Some(animation_data) = async_loader.request_animation_data_load(
                entity.get_entity_id(),
                entity.get_entity_type(),
                entity_part_files.to_vec(),
            ) {
                entity.set_animation_data(animation_data);
            }
            return;
        };
        if let Some(current) = entity.animation_data()
            && let Some(updated) = async_loader.apply_head_layer_swap(&current, head_path)
        {
            entity.set_animation_data(updated);
            return;
        }
        if let Some(animation_data) = async_loader.request_animation_data_load(
            entity.get_entity_id(),
            entity.get_entity_type(),
            entity_part_files.to_vec(),
        ) {
            entity.set_animation_data(animation_data);
        }
    }

    /// Phase C5: swap only the shield layer (`방패\…`).
    /// After the shield swap, re-apply the weapon layer from the same part list
    /// so a prior mis-classified `parts[2]` refresh cannot leave the sword gone.
    fn refresh_entity_shield_layer(
        async_loader: &AsyncLoader,
        library: &Library,
        game_file_loader: &GameFileLoader,
        entity: &mut Entity,
    ) {
        let parts = entity.get_entity_part_files(library, game_file_loader);
        let shield_path = crate::world::shield_path_from_entity_parts(&parts);
        if let Some(current) = entity.animation_data()
            && let Some(updated) = async_loader.apply_shield_layer_swap(&current, shield_path)
        {
            entity.set_animation_data(updated);
            // Keep weapon in sync (weapon may sit after shield in path logic).
            Self::refresh_entity_weapon_layer(async_loader, entity, &parts);
            return;
        }
        if let Some(animation_data) =
            async_loader.request_animation_data_load(entity.get_entity_id(), entity.get_entity_type(), parts)
        {
            entity.set_animation_data(animation_data);
        }
    }

    /// Drain finished async loads from the loader thread and apply their
    /// results to client state. This is the only place that promotes
    /// `self.map` from `None` to `Some` (when a map load completes), so it
    /// must run before the `self.map` check in [`Self::update_and_render`].
    ///
    /// Called as late as possible in the frame to give the loader thread the
    /// Play a resolved effect at `position`, dispatching to whichever pipeline
    /// its family needs: the STR keyframe player for `data\texture\effect\*.str`,
    /// or the classic sprite player for `data\sprite\이팩트\*.spr`.
    ///
    /// Sprite effects load lazily on first use, exactly like the emote sheet —
    /// the first cast of a skill spawns nothing visible while its sprite is in
    /// flight, and every later cast draws immediately.
    #[allow(clippy::too_many_arguments)]
    fn spawn_resolved_effect(
        &mut self,
        resolved: ResolvedEffect,
        position: Point3<f32>,
        point_light_id: PointLightId,
        light_color: Color,
        light_range: f32,
        start_delay: f32,
        client_tick: ClientTick,
    ) {
        match resolved {
            ResolvedEffect::Str(effect_path) => match self.effect_loader.get_or_load(effect_path, &self.texture_loader) {
                Ok(effect) => {
                    let frame_timer = effect.new_frame_timer();
                    self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Position(position),
                        Vector3::new(0.0, 0.0, 0.0),
                        point_light_id,
                        Vector3::new(0.0, 6.0, 0.0),
                        light_color,
                        light_range,
                        false,
                        start_delay,
                    )));
                }
                Err(error) => {
                    eprintln!("[skill-effect] failed to load {effect_path}: {error:?}");
                }
            },
            ResolvedEffect::Sprite { path, action_index } => {
                // A cached sheet comes back from the request immediately; only
                // a genuine miss goes through the sentinel routing.
                if let Some(sentinel) = self.sprite_effects.request_slot(path)
                    && let Some(animation_data) =
                        self.async_loader
                            .request_animation_data_load(sentinel, EntityType::Npc, vec![path.to_string()])
                {
                    self.sprite_effects.set_animation_data(path, animation_data);
                }

                // ACT frames are bottom-normalized (same as emotes). Entity and
                // ground anchors are feet-level, so without a lift the ghost
                // draws under the target. Per-skill anchor styles (ground rise
                // vs body center) can refine this later.
                const SPRITE_EFFECT_BODY_LIFT: f32 = 8.0;
                let position = position + Vector3::new(0.0, SPRITE_EFFECT_BODY_LIFT, 0.0);

                self.sprite_effects.spawn(path, position, action_index, client_tick);
            }
        }
    }

    /// maximum window to finish in-flight work.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_loaded_resources(&mut self, client_tick: ClientTick) {
        // Collect first so later steps can mutably borrow `self` (the iterator
        // from `take_completed` otherwise keeps `async_loader` borrowed).
        let completed_loads: Vec<_> = self.async_loader.take_completed().collect();
        for completed in completed_loads {
            match completed {
                (LoaderId::AnimationData(entity_id), LoadableResource::AnimationData(animation_data)) => {
                    if entity_id == EMOTE_ANIMATION_ENTITY_ID {
                        self.emote_bubbles.set_animation_data(animation_data);
                    } else if let Some(path) = self.sprite_effects.path_for_sentinel(entity_id) {
                        // Classic sprite effects route their loads through a
                        // sentinel range just below the emote sheet, so they
                        // never collide with a real entity ID.
                        self.sprite_effects.set_animation_data(path, animation_data);
                    } else if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(entity) = self
                        .client_state
                        .try_follow_mut(this_entity())
                        .filter(|entity| entity.get_entity_id() == entity_id)
                    {
                        // The local player lives under `this_entity`, not in
                        // the entities list. Without this branch the weapon
                        // layer requested on login/equip is loaded and then
                        // discarded, so the local actor keeps its char-select
                        // body/head-only animation (no weapon sprite ever).
                        entity.set_animation_data(animation_data);
                    } else if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().dead_entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.set_animation_data(animation_data);
                    }
                }
                (LoaderId::ItemSprite(item_id), LoadableResource::ItemSprite { texture }) => {
                    self.client_state
                        .follow_mut(client_state().shop_items())
                        .iter_mut()
                        .filter(|item| item.item_id == item_id)
                        .for_each(|item| item.metadata.texture = Some(texture.clone()));

                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_item_sprite(item_id, texture);
                }
                (LoaderId::Map(map_file_name), LoadableResource::Map { map, position }) => {
                    match self.client_state.try_follow(this_player()).is_none() {
                        true => {
                            // Load of main menu map
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);

                            self.interface.open_window(CharacterSelectionWindow::new(
                                client_state().character_slots(),
                                client_state().switch_request(),
                            ));

                            self.start_camera.set_focus_point(START_CAMERA_FOCUS_POINT);
                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());
                            self.client_state.follow_mut(client_state().minimap()).clear();
                        }
                        false => {
                            // Normal map switch
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine.play_background_music_track(map.background_music_track_name());

                            // Relax the battle stance on town / safe maps.
                            let base = normalize_map_base_name(&map_file_name);
                            self.current_map_is_town = self.library.is_town_map(&base);

                            if let Some(position) = position {
                                // `manually_asserted` is safe because we are in the branch where `this_player`
                                // is not `None`.
                                let in_safe_zone = self.current_map_is_town;
                                let player = self.client_state.follow_mut(this_entity().manually_asserted());

                                player.set_in_safe_zone(in_safe_zone);
                                player.set_position(map, position, client_tick);
                                player.refresh_neutral_stance(client_tick);
                                self.player_camera.set_focus_point(player.get_position());
                            }

                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());
                            let (map_w, map_h) = (map.width(), map.height());
                            let map_name = map_file_name.clone();
                            self.refresh_minimap(&map_name, map_w, map_h);
                            let _ = self.networking_system.map_loaded();
                        }
                    }
                }
                (LoaderId::SkillSprite(skill_id), LoadableResource::SkillSprite { sprite }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar().skills())
                        .iter_mut()
                        .filter_map(|slot| slot.as_mut())
                        .filter(|skill| skill.skill_id == skill_id)
                        .for_each(|skill| {
                            skill.sprite = Some(sprite.clone());
                        });

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.sprite = Some(sprite);
                    }
                }
                (LoaderId::SkillActions(skill_id), LoadableResource::SkillActions { actions }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar().skills())
                        .iter_mut()
                        .filter_map(|slot| slot.as_mut())
                        .filter(|skill| skill.skill_id == skill_id)
                        .for_each(|skill| {
                            skill.actions = Some(actions.clone());
                        });

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.actions = Some(actions);
                    }
                }
                _ => {}
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_main_camera(&mut self, window_size: ScreenSize, delta_time: f64, #[cfg(feature = "debug")] render_options: &RenderOptions) {
        if self.client_state.try_follow(this_entity()).is_some() {
            self.player_camera.update(delta_time);
            self.player_camera.generate_view_projection(window_size);
        } else {
            self.start_camera.update(delta_time);
            self.start_camera.generate_view_projection(window_size);
        }

        #[cfg(feature = "debug")]
        self.interface_renderer.update_render_options(render_options);

        #[cfg(feature = "debug")]
        if render_options.use_debug_camera {
            self.debug_camera.generate_view_projection(window_size);
        }
    }

    /// Per-frame tick for all entities, dead entities, and ground items.
    ///
    /// Must be called after [`Self::handle_network_events`] so the entity
    /// set reflects the latest spawn/despawn/move packets. Running this
    /// with a stale entity list would tick entities that the network has
    /// already removed, or miss new ones that just appeared (and on a map
    /// transition, would tick entities from the previous map).
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_entities(
        &mut self,
        map: &Map,
        currently_playing: bool,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
            #[cfg(feature = "debug")]
            _ if render_options.use_debug_camera => &self.debug_camera,
            true => &self.player_camera,
            false => &self.start_camera,
        };

        self.client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .for_each(|entity| entity.update(&self.audio_engine, map, current_camera, client_tick));

        self.client_state
            .follow_mut(client_state().dead_entities())
            .iter_mut()
            .for_each(|entity| {
                entity.update(&self.audio_engine, map, current_camera, client_tick);

                if entity.is_death_animation_over() && !entity.is_fading() {
                    entity.fade_out(DisappearanceReason::Died, client_tick);
                }
            });

        self.client_state
            .follow_mut(client_state().ground_items())
            .iter_mut()
            .for_each(|item| item.update(client_tick));

        // Remove entities that have finished fading out.
        self.client_state
            .follow_mut(client_state().entities())
            .retain(|entity| !entity.should_be_removed(client_tick));

        self.client_state
            .follow_mut(client_state().dead_entities())
            .retain(|entity| !entity.should_be_removed(client_tick));

        self.client_state
            .follow_mut(client_state().ground_items())
            .retain(|item| !item.should_be_removed(client_tick));
    }

    /// Fire any action that the player buffered while out of range or while
    /// still moving (attack, skill, pick up item). Must be called after
    /// [`Self::update_entities`] so that the player's `stopped_moving` state
    /// reflects this frame.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn process_buffered_action(&mut self) {
        let Some(true) = self.client_state.try_follow(this_entity()).map(|player| player.stopped_moving()) else {
            return;
        };

        let Some(buffered_action) = self.client_state.follow_mut(client_state().buffered_action()).take() else {
            return;
        };

        match buffered_action {
            BufferedAction::AttackEntity { entity_id } => {
                let _ = self.networking_system.player_attack(entity_id);

                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                if auto_attack {
                    *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity { entity_id });
                }
            }
            BufferedAction::PickUpItem { entity_id } => {
                if self
                    .client_state
                    .follow(client_state().ground_items())
                    .iter()
                    .any(|item| item.get_entity_id() == entity_id)
                {
                    let _ = self.networking_system.pick_up_item(entity_id);
                }
            }
            BufferedAction::CastSkill {
                skill_id,
                skill_level,
                entity_id,
                attack_range,
            } => {
                let player_position = self.client_state.try_follow(this_entity()).map(Entity::get_tile_position);
                let target_position = self
                    .client_state
                    .follow(client_state().entities())
                    .iter()
                    .find(|entity| entity.get_entity_id() == entity_id)
                    .map(Entity::get_tile_position);

                if let (Some(map), Some(player_position), Some(target_position)) = (self.map.as_deref(), player_position, target_position) {
                    if is_within_skill_range(player_position, target_position, attack_range) {
                        let _ = self.networking_system.cast_skill(skill_id, skill_level, entity_id);
                    } else if let Some(path) =
                        self.path_finder
                            .find_walkable_path_in_range(map, player_position, target_position, attack_range)
                        && let Some(nearest_tile) = path.last()
                    {
                        let _ = self.networking_system.player_move(WorldPosition {
                            x: nearest_tile.x,
                            y: nearest_tile.y,
                            direction: Direction::North,
                        });
                        *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::CastSkill {
                            skill_id,
                            skill_level,
                            entity_id,
                            attack_range,
                        });
                    }
                }
            }
            BufferedAction::CastGroundSkill {
                skill_id,
                skill_level,
                tile,
                attack_range,
            } => cast_or_path_ground_skill(
                &mut self.networking_system,
                &mut self.client_state,
                self.map.as_deref(),
                &mut self.path_finder,
                skill_id,
                skill_level,
                attack_range,
                tile,
            ),
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_audio_engine(&self, current_camera: &dyn Camera) {
        // We set the listener roughly at ear height.
        const EAR_HEIGHT: Vector3<f32> = Vector3::new(0.0, 5.0, 0.0);
        let listener = current_camera.focus_point() + EAR_HEIGHT;

        self.audio_engine
            .set_spatial_listener(listener, current_camera.view_direction(), current_camera.look_up_vector());
        self.audio_engine.update();
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn create_point_light_set<'a>(
        point_light_manager: &'a mut PointLightManager,
        point_light_set_buffer: &mut ResourceSetBuffer<LightSourceKey>,
        map: &Map,
        effect_holder: &EffectHolder,
        current_camera: &dyn Camera,
        lighting_mode: LightingMode,
    ) -> PointLightSet<'a> {
        point_light_manager.prepare();

        effect_holder.register_point_lights(point_light_manager, current_camera);

        map.register_point_lights(point_light_manager, point_light_set_buffer, current_camera);

        match lighting_mode {
            LightingMode::Classic => point_light_manager.create_point_light_set(0),
            LightingMode::Enhanced => point_light_manager.create_point_light_set(NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS),
        }
    }

    /// Applies any buffered drag from the previous frame's input and draws the
    /// per-frame top-level overlays (FPS counter when in debug, mouse cursor).
    /// Must be called after the laid-out interface frame has been dropped so
    /// that `self.interface` is no longer borrowed.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_ui_overlays(
        &mut self,
        input_report: &InputReport,
        scaling: Scaling,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        if let Some(delta) = input_report.drag {
            // TODO: The scaling should be removed here.
            self.interface.handle_drag(delta, scaling.get_factor());

            // Edge-resize of the Map window should update the minimap zoom level.
            if matches!(
                self.interface.get_mouse_mode(),
                korangar_interface::MouseMode::ResizingWindow { .. }
            ) && self.interface.is_window_with_class_open(WindowClass::Minimap)
            {
                use crate::state::minimap::{MAX_MINIMAP_SIDE, MIN_MINIMAP_SIDE};
                const CHROME_W: f32 = 24.0;
                if let Some(size) = self.interface.window_size_for_class(WindowClass::Minimap) {
                    let side = (size.width - CHROME_W).clamp(MIN_MINIMAP_SIDE, MAX_MINIMAP_SIDE);
                    self.client_state.follow_mut(client_state().minimap()).set_display_side(side);
                }
            }
        }

        #[cfg(feature = "debug")]
        if render_options.show_frames_per_second {
            let world_theme = self.client_state.follow(client_state().world_theme());

            self.top_interface_renderer.render_text(
                &self.game_timer.last_frames_per_second().to_string(),
                world_theme.overlay.text_offset,
                world_theme.overlay.foreground_color,
                world_theme.overlay.font_size,
                AlignHorizontal::Left,
            );
        }

        if self.show_interface {
            self.mouse_cursor.render(
                &self.top_interface_renderer,
                input_report.mouse_position,
                self.interface.get_mouse_mode().grabbed(),
                *self.client_state.follow(client_state().world_theme().cursor().color()),
                self.client_state.follow(client_state().interface_settings().scaling()).get_factor(),
            );
        }
    }

    #[inline(always)]
    fn update_and_render(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
            event_loop.exit();
            return;
        }

        // Clear the previous render instructions so we can rendering the new frame.
        self.clear_render_instructions();

        // It is important that we first apply any changes that were dispatched during
        // the last frame.
        self.update_client_state();

        // We can only apply the graphic changes and reconfigure the surface once the
        // previous image was presented. Moving this function to the end of the
        // function results in surface configuration errors under DX12.
        self.update_settings();

        // TODO: Shouldn't this happen later? After the scaling has been potentially
        // changed by the UI.
        let scaling = *self.client_state.follow(client_state().interface_settings().scaling());
        self.update_interface_scaling(scaling);

        let FrameTimers {
            delta_time,
            client_tick,
            animation_timer_ms,
        } = self.game_timer.update();

        let input_report = self.input_system.update_delta(client_tick);

        self.request_entity_details(&input_report);

        self.handle_network_events(client_tick);
        self.tick_server_select_auth();

        let interface_has_focus = self.process_user_events(
            &input_report,
            client_tick,
            #[cfg(feature = "debug")]
            (delta_time as f32),
        );

        // Some debug windows, such as the packet history or cache statistics, require
        // special update logic.
        #[cfg(feature = "debug")]
        self.update_debug_windows(delta_time);

        // We run this last to give the loader thread as much time as possible to
        // complete the loading. When starting the actual render process, we
        // can't modify resources anymore until the next frame.
        self.update_loaded_resources(client_tick);

        #[cfg(feature = "debug")]
        let render_options = *self.client_state.follow(client_state().render_options());

        let screen_size = self.graphics_engine.get_window_size().into();
        let currently_playing = self.client_state.try_follow(this_player()).is_some();

        self.mouse_cursor.update(client_tick);

        // Acquire the swapchain image as late as possible so all CPU-side
        // preparation overlaps with the previous frame's GPU work. After this
        // point we may not reconfigure the surface (see `update_settings`).
        let maybe_frame = self.graphics_engine.wait_for_next_frame();

        // If we don't have a map, the rendering ends here.
        let Some(map) = self.map.clone() else {
            if let Some(frame) = maybe_frame {
                self.graphics_engine.render_next_frame(frame, Default::default());
            }

            return;
        };

        self.update_entities(
            &map,
            currently_playing,
            client_tick,
            #[cfg(feature = "debug")]
            &render_options,
        );

        self.process_buffered_action();

        self.update_main_camera(
            screen_size,
            delta_time,
            #[cfg(feature = "debug")]
            &render_options,
        );

        map.advance_videos(&self.queue, delta_time);

        if let Some(player) = self.client_state.try_follow(this_entity()) {
            self.player_camera.set_smoothed_focus_point(player.get_position());
        }

        // Update particles.
        self.particle_holder.update(delta_time as f32);
        self.emote_bubbles.update(client_tick);
        self.sprite_effects.update(client_tick);
        self.effect_holder
            .update(self.client_state.follow(client_state().entities()), delta_time as f32);

        let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
            #[cfg(feature = "debug")]
            _ if render_options.use_debug_camera => &self.debug_camera,
            true => &self.player_camera,
            false => &self.start_camera,
        };

        let (view_matrix, projection_matrix) = current_camera.view_projection_matrices();
        let camera_position = current_camera.camera_position().to_homogeneous();

        #[cfg(feature = "debug")]
        let update_shadow_camera_measurement = Profiler::start_measurement("update directional shadow camera");

        let lighting_mode = *self.client_state.follow(client_state().graphics_settings().lighting_mode());
        let sprite_lighting_mode = *self.client_state.follow(client_state().graphics_settings().sprite_lighting_mode());
        let shadow_resolution = *self.client_state.follow(client_state().graphics_settings().shadow_resolution());
        let shadow_method = *self.client_state.follow(client_state().graphics_settings().shadow_method());
        let shadow_detail = *self.client_state.follow(client_state().graphics_settings().shadow_detail());
        let sdsm_enabled = *self.client_state.follow(client_state().graphics_settings().sdsm());

        let use_sdsm = sdsm_enabled & !self.player_camera.is_rotating_or_zooming_fast();

        let (directional_light_direction, directional_light_color) = map.directional_light();

        match use_sdsm {
            true => {
                self.directional_shadow_camera.update_camera_sdsm(
                    directional_light_direction,
                    &view_matrix,
                    &projection_matrix,
                    shadow_resolution.directional_shadow_resolution(),
                    self.directional_shadow_partitions.lock().unwrap().deref(),
                );
            }
            false => {
                self.directional_shadow_camera.update_camera_pssm(
                    directional_light_direction,
                    &view_matrix,
                    &projection_matrix,
                    shadow_resolution.directional_shadow_resolution(),
                );
            }
        }

        #[cfg(feature = "debug")]
        update_shadow_camera_measurement.stop();

        self.update_audio_engine(current_camera);

        #[cfg(feature = "debug")]
        let prepare_frame_measurement = Profiler::start_measurement("prepare frame");

        #[cfg(feature = "debug")]
        let hovered_marker_identifier = match input_report.mouse_target {
            PickerTarget::Marker(marker_identifier) => Some(marker_identifier),
            _ => None,
        };

        let point_light_set = Self::create_point_light_set(
            &mut self.point_light_manager,
            &mut self.point_light_set_buffer,
            &map,
            &self.effect_holder,
            current_camera,
            lighting_mode,
        );

        #[cfg(feature = "debug")]
        prepare_frame_measurement.stop();

        let mut indicator_instruction = None;
        let mut water_instruction = None;

        let mouse_mode = self.interface.get_mouse_mode().clone();
        let is_mouse_mode_default = mouse_mode.is_default();
        let last_walking_destination = mouse_mode.walk_destination();

        let mut interface_frame = {
            #[cfg(feature = "debug")]
            profile_block!("user interface");

            let is_rotating_camera = mouse_mode.is_rotating_camera();
            let is_grabbing = mouse_mode.is_grabbing();
            let is_chat_open = self.interface.is_window_with_class_open(WindowClass::Chat);

            let mut interface_frame = self
                .interface
                .lay_out_windows(&self.client_state, scaling.get_factor(), input_report.mouse_position);

            // We can only decide what to do with the user input once we know if the mouse
            // is hovering a window, so we buffer any actions for the next frame.

            let is_interface_hovered = interface_frame.is_interface_hovered();

            let cursor_state = match input_report.mouse_target {
                _ if is_rotating_camera => MouseCursorState::RotateCamera,
                _ if is_grabbing => MouseCursorState::GrabResource,
                // A skill is armed and waiting for a target: show the attack reticle over
                // the world so it's clear the next click aims the skill, not a walk.
                _ if self.pending_skill.is_some() && !is_interface_hovered => MouseCursorState::Attack,
                PickerTarget::Entity(entity_id) if !is_interface_hovered => {
                    if self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .any(|item| item.get_entity_id() == entity_id)
                    {
                        MouseCursorState::HoverItem
                    } else {
                        self.client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == entity_id)
                            .map(|entity| match entity.get_entity_type() {
                                EntityType::Npc => MouseCursorState::Dialog,
                                EntityType::Warp => MouseCursorState::Warp,
                                EntityType::Monster => MouseCursorState::Attack,
                                _ => MouseCursorState::Default,
                            })
                            .unwrap_or(MouseCursorState::Default)
                    }
                }
                _ => MouseCursorState::Default,
            };
            self.mouse_cursor.set_state(cursor_state, client_tick);

            if let Some(mouse_button) = input_report.mouse_click {
                if is_interface_hovered {
                    // Starts item/skill drag via SetMouseMode (applied immediately inside click).
                    interface_frame.click(&self.client_state, mouse_button);
                } else {
                    interface_frame.unfocus();

                    if mouse_button == MouseButton::Left {
                        if let Some(pending) = self.pending_skill.take() {
                            // A skill is armed: this click picks its target. On a hit the skill
                            // disarms; on empty ground it fizzles and stays armed (right-click or
                            // Escape is the only cancel), and the click never falls through to a
                            // walk/interact.
                            let resolved = resolve_pending_cast(pending.skill_type, input_report.mouse_target);
                            let performed = match resolved {
                                PendingCastResolution::CastEntity(entity_id) => {
                                    self.input_event_buffer.push(InputEvent::CastSkillAtEntity {
                                        skill_id: pending.skill_id,
                                        skill_level: pending.skill_level,
                                        attack_range: pending.attack_range,
                                        entity_id,
                                    });
                                    true
                                }
                                // Ground placements go through the same walk-into-range path as
                                // entity targets; an out-of-range cell is otherwise dropped by
                                // the server without any feedback at all.
                                resolution => match resolve_pending_ground_tile(&self.client_state, resolution) {
                                    Some(tile) => {
                                        self.input_event_buffer.push(InputEvent::CastSkillAtTile {
                                            skill_id: pending.skill_id,
                                            skill_level: pending.skill_level,
                                            attack_range: pending.attack_range,
                                            tile,
                                        });
                                        true
                                    }
                                    None => false,
                                },
                            };
                            if !performed {
                                // Fizzled — put it back so it stays armed for the next click.
                                self.pending_skill = Some(pending);
                            }
                        } else {
                            match input_report.mouse_target {
                                PickerTarget::Nothing => {}
                                PickerTarget::Entity(entity_id) => {
                                    let is_ground_item = self
                                        .client_state
                                        .follow(client_state().ground_items())
                                        .iter()
                                        .any(|item| item.get_entity_id() == entity_id);

                                    if is_ground_item {
                                        self.input_event_buffer.push(InputEvent::PickUpItem { entity_id })
                                    } else {
                                        self.input_event_buffer.push(InputEvent::PlayerInteract { entity_id })
                                    }
                                }
                                PickerTarget::Tile { x, y } => {
                                    let destination = TilePosition { x, y };

                                    interface_frame.set_mouse_mode(MouseInputMode::Walk { destination });

                                    self.input_event_buffer.push(InputEvent::PlayerMove { destination });
                                }
                                #[cfg(feature = "debug")]
                                PickerTarget::Marker(marker_identifier) => {
                                    self.input_event_buffer.push(InputEvent::OpenMarkerDetails { marker_identifier })
                                }
                            }
                        }
                    } else if mouse_button == MouseButton::Right {
                        if self.pending_skill.is_some() {
                            // Right-click is the primary cancel gesture for an armed skill; it
                            // clears the target and does not start a camera rotation.
                            self.pending_skill = None;
                        } else if cancel_own_cast(&mut self.networking_system, &self.client_state, client_tick) {
                            // Then an in-progress cast, for the same reason: aborting must not
                            // also start swinging the camera around.
                        } else if currently_playing {
                            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                            interface_frame.set_mouse_mode(MouseInputMode::RotateCamera);
                        }
                    } else if mouse_button == MouseButton::DoubleRight && currently_playing {
                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                        self.input_event_buffer.push(InputEvent::ResetCameraRotation);
                    }
                }
            } else if let Some(last_destination) = last_walking_destination
                && let PickerTarget::Tile { x, y } = input_report.mouse_target
                && input_report.left_mouse_button_down
                && self.pending_skill.is_none()
            {
                let destination = TilePosition { x, y };

                if last_destination != destination {
                    interface_frame.set_mouse_mode(MouseInputMode::Walk { destination });
                    self.input_event_buffer.push(InputEvent::PlayerMove { destination });
                }
            }

            if input_report.mouse_button_released {
                // Drag inventory item onto the world (not a UI drop target) → drop to ground.
                // `mouse_mode` is from after process_events, so a press on the previous frame
                // has already become MoveItem.
                if !is_interface_hovered
                    && let MouseMode::Custom {
                        mode:
                            MouseInputMode::MoveItem {
                                source: ItemSource::Inventory,
                                item,
                            },
                    } = &mouse_mode
                {
                    let amount = inventory_item_amount(item);
                    if amount > 0 {
                        self.input_event_buffer.push(InputEvent::DropItem {
                            inventory_index: item.index,
                            amount,
                        });
                    }
                }

                // Equip/storage transfers via drop handlers; queues Default mouse mode.
                interface_frame.drop(&self.client_state);
            }

            if let Some(delta) = input_report.scroll {
                if is_interface_hovered {
                    interface_frame.scroll(&self.client_state, delta);
                } else {
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                    self.input_event_buffer.push(InputEvent::ZoomCamera { zoom_factor: delta });
                }
            }

            // Focus the chat if the interface is not focused, no other element is capturing
            // the keyboard input, enter was pressed, and the chat
            // window is open.
            if (!interface_has_focus || !interface_frame.input_characters(&self.client_state, &input_report.characters))
                && input_report.characters.contains(&'\x0d')
                && is_chat_open
            {
                interface_frame.focus_element(ChatTextBox);
            }

            interface_frame
        };

        {
            let mut render_context = MapRenderContext {
                map: &map,
                current_camera,
                point_light_set: &point_light_set,
                client_state: &self.client_state,
                library: &self.library,
                mouse_position: input_report.mouse_position,
                mouse_target: input_report.mouse_target,
                screen_size,
                scaling,
                client_tick,
                animation_timer_ms,
                active_trap_props: &self.active_trap_props,
                currently_playing,
                is_mouse_mode_default,
                is_interface_hovered: interface_frame.is_interface_hovered(),
                last_walking_destination,
                buffered_action: *self.client_state.follow(client_state().buffered_action()),
                pending_skill: self.pending_skill.as_ref(),
                skill_footprint_texture: self.skill_footprint_texture.as_ref(),
                #[cfg(feature = "debug")]
                render_options: &render_options,
                #[cfg(feature = "debug")]
                hovered_marker_identifier,
                #[cfg(feature = "debug")]
                pathing_texture_set: &self.pathing_texture_set,
                #[cfg(feature = "debug")]
                tile_texture_set: &self.tile_texture_set,
                #[cfg(feature = "debug")]
                player_camera: &self.player_camera,
                #[cfg(feature = "debug")]
                start_camera: &self.start_camera,
                model_batches: &mut self.model_batches,
                model_instructions: &mut self.model_instructions,
                entity_instructions: &mut self.entity_instructions,
                directional_shadow_camera: &mut self.directional_shadow_camera,
                directional_shadow_model_batches: &mut self.directional_shadow_model_batches,
                directional_shadow_model_instructions: &mut self.directional_shadow_model_instructions,
                directional_shadow_entity_instructions: &mut self.directional_shadow_entity_instructions,
                point_shadow_camera: &mut self.point_shadow_camera,
                point_shadow_model_instructions: &mut self.point_shadow_model_instructions,
                point_light_with_shadow_instructions: &mut self.point_light_with_shadow_instructions,
                point_light_instructions: &mut self.point_light_instructions,
                directional_shadow_object_set_buffer: &mut self.directional_shadow_object_set_buffer,
                point_shadow_object_set_buffer: &mut self.point_shadow_object_set_buffer,
                deferred_object_set_buffer: &mut self.deferred_object_set_buffer,
                indicator_instruction: &mut indicator_instruction,
                water_instruction: &mut water_instruction,
                particle_holder: &mut self.particle_holder,
                emote_bubbles: &self.emote_bubbles,
                sprite_effects: &self.sprite_effects,
                effect_holder: &mut self.effect_holder,
                effect_renderer: &mut self.effect_renderer,
                bottom_interface_renderer: &self.bottom_interface_renderer,
                middle_interface_renderer: &mut self.middle_interface_renderer,
                #[cfg(feature = "debug")]
                aabb_instructions: &mut self.aabb_instructions,
                #[cfg(feature = "debug")]
                circle_instructions: &mut self.circle_instructions,
                #[cfg(feature = "debug")]
                rectangle_instructions: &mut self.rectangle_instructions,
                #[cfg(feature = "debug")]
                bounding_box_object_set_buffer: &mut self.bounding_box_object_set_buffer,
                #[cfg(feature = "debug")]
                debug_marker_renderer: &mut self.debug_marker_renderer,
            };

            #[cfg(feature = "debug")]
            render_context.render_markers();
            render_context.render_directional_shadows();
            render_context.render_point_lights();
            render_context.render_geometry();
            #[cfg(feature = "debug")]
            render_context.render_bounding_boxes();
            render_context.render_world_overlays();
        }

        let in_game_theme_path = client_state().in_game_theme().tooltip();
        let menu_theme_path = client_state().menu_theme().tooltip();
        let tooltip_theme = match currently_playing {
            true => self.client_state.follow(in_game_theme_path),
            false => self.client_state.follow(menu_theme_path),
        };

        interface_frame.render(
            &self.client_state,
            &self.interface_renderer,
            tooltip_theme,
            input_report.mouse_position,
        );

        drop(interface_frame);

        // UI click/drop handlers queue Application events (MoveItem, DropItem,
        // OpenItemActions) and SetMouseMode *after* the start-of-frame
        // process_user_events. Flush them now so inventory drag→equip and
        // drop-to-ground take effect without waiting an extra frame.
        self.interface.process_events(&mut self.input_event_buffer);
        self.flush_inventory_input_events();

        self.render_ui_overlays(
            &input_report,
            scaling,
            #[cfg(feature = "debug")]
            &render_options,
        );

        let picker_position = ScreenPosition {
            left: input_report.mouse_position.left.clamp(0.0, screen_size.width),
            top: input_report.mouse_position.top.clamp(0.0, screen_size.height),
        };

        let uniforms = Uniforms {
            view_matrix,
            projection_matrix,
            camera_position,
            animation_timer_ms,
            ambient_light_color: map.ambient_light_color(),
            enhanced_lighting: lighting_mode == LightingMode::Enhanced,
            sprite_lighting_mode,
            shadow_method,
            shadow_detail,
            use_sdsm,
            sdsm_enabled,
        };

        let interface_instructions = self.interface_renderer.get_instructions();
        let bottom_layer_instructions = self.bottom_interface_renderer.get_instructions();
        let middle_layer_instructions = self.middle_interface_renderer.get_instructions();
        let top_layer_instructions = self.top_interface_renderer.get_instructions();

        let directional_light = DirectionalLightInstruction {
            view_projection_matrix: self.directional_shadow_camera.view_projection_matrix(),
            direction: directional_light_direction,
            color: directional_light_color,
        };

        let render_instruction = RenderInstruction {
            show_interface: self.show_interface,
            picker_position,
            uniforms,
            indicator: indicator_instruction,
            interface: interface_instructions.as_slice(),
            bottom_layer_rectangles: bottom_layer_instructions.as_slice(),
            middle_layer_rectangles: middle_layer_instructions.as_slice(),
            top_layer_rectangles: top_layer_instructions.as_slice(),
            directional_light,
            directional_light_partitions: &self.directional_shadow_camera.get_partition_instructions(),
            point_light: &self.point_light_instructions,
            point_light_with_shadows: &self.point_light_with_shadow_instructions,
            model_batches: &self.model_batches,
            models: &mut self.model_instructions,
            entities: &mut self.entity_instructions,
            directional_shadow_model_batches: &self.directional_shadow_model_batches,
            directional_shadow_models: &self.directional_shadow_model_instructions,
            directional_shadow_entities: &mut self.directional_shadow_entity_instructions,
            point_shadow_models: &self.point_shadow_model_instructions,
            point_shadow_entities: &self.point_shadow_entity_instructions,
            effects: self.effect_renderer.get_instructions(),
            ground_decals: self.effect_renderer.get_ground_decals(),
            water: water_instruction,
            map_picker_tile_vertex_buffer: Some(map.get_tile_picker_vertex_buffer()),
            map_picker_tile_index_buffer: Some(map.get_tile_picker_index_buffer()),
            font_map_texture: Some(self.font_loader.get_font_map()),
            #[cfg(feature = "debug")]
            render_options,
            #[cfg(feature = "debug")]
            aabb: &self.aabb_instructions,
            #[cfg(feature = "debug")]
            circles: &self.circle_instructions,
            #[cfg(feature = "debug")]
            rectangles: &self.rectangle_instructions,
            #[cfg(feature = "debug")]
            marker: self.debug_marker_renderer.get_instructions(),
        };

        if let Some(frame) = maybe_frame {
            self.graphics_engine.render_next_frame(frame, render_instruction);
        }
    }
}

impl ApplicationHandler for Client {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // To be as portable as possible, winit recommends to initialize the window and
        // graphics backend after the first resume event is received.
        if self.window.is_none() {
            time_phase!("create window", {
                let window = Arc::new(event_loop.create_window(create_window_attributes()).unwrap());

                let backend_name = self.graphics_engine.get_backend_name();
                window.set_title(&format!("{CLIENT_NAME} ({})", str::to_uppercase(&backend_name)));
                window.set_cursor_visible(false);

                self.window = Some(window);

                #[cfg(feature = "debug")]
                print_debug!("created {}", "window".magenta());
            });
        }

        // Android devices need to drop the surface on suspend, so we might need to
        // re-create it.
        if let Some(window) = self.window.as_ref() {
            let path = client_state().graphics_settings();
            let graphics_settings = self.client_state.follow(path);

            self.graphics_engine.on_resume(
                window.clone(),
                graphics_settings.triple_buffering,
                graphics_settings.vsync,
                graphics_settings.limit_framerate,
                graphics_settings.shadow_resolution,
                graphics_settings.texture_filtering,
                graphics_settings.msaa,
                graphics_settings.ssaa,
                graphics_settings.screen_space_anti_aliasing,
                graphics_settings.high_quality_interface,
            );

            // Update graphics settings capabilities based on the new surface.
            // We don't expect the capabilities to change on consecutive calls but we
            // can't get the present mode info when initializing the client, so
            // we do it here instead.
            self.client_state
                .follow_mut(client_state().graphics_settings_capabilities())
                .update(
                    self.graphics_engine.get_supported_msaa(),
                    self.graphics_engine.get_present_mode_info(),
                );

            window.set_visible(true);

            // Kick off the self-sustaining redraw loop. Without this, if the only
            // OS-initiated `RedrawRequested` arrived before the surface existed (and
            // was skipped by the guard in `window_event`), nothing would ever render.
            window.request_redraw();
        }

        if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
            self.audio_engine.mute(false);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(screen_size) => {
                let screen_size = screen_size.max(PhysicalSize::new(1, 1)).into();
                *self.client_state.follow_mut(client_state().window_size()) = screen_size;
                self.graphics_engine.on_resize(screen_size);
                self.interface.update_window_size(screen_size);
                self.interface_renderer.update_window_size(screen_size);
                self.bottom_interface_renderer.update_window_size(screen_size);
                self.middle_interface_renderer.update_window_size(screen_size);
                self.top_interface_renderer.update_window_size(screen_size);
                self.effect_renderer.update_window_size(screen_size);

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                if !focused {
                    self.input_system.reset();
                }

                if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
                    self.audio_engine.mute(!focused);
                }
            }
            WindowEvent::CursorLeft { .. } => self.mouse_cursor.hide(),
            WindowEvent::CursorEntered { .. } => self.mouse_cursor.show(),
            WindowEvent::CursorMoved { position, .. } => self.input_system.update_mouse_position(position),
            WindowEvent::MouseInput { button, state, .. } => self.input_system.update_mouse_buttons(button, state),
            WindowEvent::MouseWheel { delta, .. } => self.input_system.update_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => {
                match event.physical_key {
                    PhysicalKey::Code(keycode) => {
                        self.input_system.update_keyboard(keycode, event.state);
                    }
                    // Under WSLg / some layouts, Insert can arrive as Unidentified physical
                    // with a Named logical key. Synthesise the KeyCode so sit still works.
                    PhysicalKey::Unidentified(_) => {
                        use winit::keyboard::{Key, NamedKey};
                        if matches!(event.logical_key, Key::Named(NamedKey::Insert)) {
                            self.input_system.update_keyboard(KeyCode::Insert, event.state);
                        } else if matches!(event.logical_key, Key::Named(NamedKey::Home)) {
                            self.input_system.update_keyboard(KeyCode::Home, event.state);
                        }
                    }
                }

                // TODO: NHA We should also support IME in the long term (winit::event::Ime)
                if let Some(text) = event.text
                    && event.state.is_pressed()
                {
                    for char in text.chars() {
                        self.input_system.buffer_character(char);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Guard against a spurious `drawRect:` reentering before `resumed` has
                // finished setting up the surface (observed on macOS/AppKit). Keep
                // requesting redraws while we wait — the render loop is otherwise
                // self-sustained by the `request_redraw` below, so silently dropping
                // this event would leave the window permanently blank.
                if !self.graphics_engine.is_ready_to_render() {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }

                #[cfg(feature = "debug")]
                let _measurement = threads::Main::start_frame();

                self.update_and_render(event_loop);

                if let Some(window) = self.window.as_mut() {
                    window.request_redraw();
                }
            }
            _ignored => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.graphics_engine.on_suspended();

        if let Some(window) = self.window.as_ref() {
            window.set_visible(false);
        }

        if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
            self.audio_engine.mute(true);
        }
    }
}

/// Bundles all the borrows needed by the per-frame rendering pipeline so that
/// they can be passed as a single `self` argument.
struct MapRenderContext<'a, 'm: 'a> {
    map: &'m Map,
    /// Live Hunter-trap props to place this frame. Not part of the map, since
    /// they appear after it loads.
    active_trap_props: &'a [(EntityId, Arc<Model>, Transform)],
    current_camera: &'a (dyn Camera + Send + Sync),
    point_light_set: &'a PointLightSet<'a>,
    client_state: &'a State<ClientState>,
    library: &'a Library,
    mouse_position: ScreenPosition,
    mouse_target: PickerTarget,
    screen_size: ScreenSize,
    scaling: Scaling,
    client_tick: ClientTick,
    animation_timer_ms: f32,
    currently_playing: bool,
    is_mouse_mode_default: bool,
    is_interface_hovered: bool,
    last_walking_destination: Option<TilePosition>,
    buffered_action: Option<BufferedAction>,
    /// The skill currently armed for targeting, so the aiming cursor can show
    /// its ground footprint.
    pending_skill: Option<&'a PendingSkill>,
    /// Tile texture for that footprint. `None` if it failed to load — the
    /// footprint is then simply not drawn.
    skill_footprint_texture: Option<&'a Arc<Texture>>,
    #[cfg(feature = "debug")]
    render_options: &'a RenderOptions,
    #[cfg(feature = "debug")]
    hovered_marker_identifier: Option<MarkerIdentifier>,
    #[cfg(feature = "debug")]
    pathing_texture_set: &'a Arc<TextureSet>,
    #[cfg(feature = "debug")]
    tile_texture_set: &'a Arc<TextureSet>,
    #[cfg(feature = "debug")]
    player_camera: &'a PlayerCamera,
    #[cfg(feature = "debug")]
    start_camera: &'a StartCamera,

    // Mutable rendering state
    model_batches: &'a mut Vec<ModelBatch>,
    model_instructions: &'a mut Vec<ModelInstruction>,
    entity_instructions: &'a mut Vec<EntityInstruction>,
    directional_shadow_camera: &'a mut DirectionalShadowCamera,
    directional_shadow_model_batches: &'a mut [Vec<ModelBatch>; PARTITION_COUNT],
    directional_shadow_model_instructions: &'a mut Vec<ModelInstruction>,
    directional_shadow_entity_instructions: &'a mut [Vec<EntityInstruction>; PARTITION_COUNT],
    point_shadow_camera: &'a mut PointShadowCamera,
    point_shadow_model_instructions: &'a mut Vec<ModelInstruction>,
    point_light_with_shadow_instructions: &'a mut Vec<PointLightWithShadowInstruction>,
    point_light_instructions: &'a mut Vec<PointLightInstruction>,
    directional_shadow_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    point_shadow_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    deferred_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    indicator_instruction: &'a mut Option<IndicatorInstruction>,
    water_instruction: &'a mut Option<WaterInstruction<'m>>,
    particle_holder: &'a mut ParticleHolder,
    emote_bubbles: &'a EmoteBubbles,
    sprite_effects: &'a SpriteEffects,
    effect_holder: &'a mut EffectHolder,
    effect_renderer: &'a mut EffectRenderer,
    bottom_interface_renderer: &'a GameInterfaceRenderer,
    middle_interface_renderer: &'a mut GameInterfaceRenderer,
    #[cfg(feature = "debug")]
    aabb_instructions: &'a mut Vec<DebugAabbInstruction>,
    #[cfg(feature = "debug")]
    circle_instructions: &'a mut Vec<DebugCircleInstruction>,
    #[cfg(feature = "debug")]
    rectangle_instructions: &'a mut Vec<DebugRectangleInstruction>,
    #[cfg(feature = "debug")]
    bounding_box_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    debug_marker_renderer: &'a mut DebugMarkerRenderer,
}

impl<'a, 'm: 'a> MapRenderContext<'a, 'm> {
    #[inline(always)]
    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    fn render_markers(&mut self) {
        let entities = self.client_state.follow(client_state().entities());

        self.map.render_markers(
            self.debug_marker_renderer,
            self.current_camera,
            self.render_options,
            entities,
            self.point_light_set,
            self.hovered_marker_identifier,
        );

        self.map.render_markers(
            self.middle_interface_renderer,
            self.current_camera,
            self.render_options,
            entities,
            self.point_light_set,
            self.hovered_marker_identifier,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_directional_shadows(&mut self) {
        let entities = self.client_state.follow(client_state().entities());
        let dead_entities = self.client_state.follow(client_state().dead_entities());
        let ground_items = self.client_state.follow(client_state().ground_items());

        for partition_index in 0..PARTITION_COUNT {
            let partition_camera = self.directional_shadow_camera.get_partition_camera(partition_index);

            let object_set = self.map.cull_objects_with_frustum(
                &partition_camera,
                self.directional_shadow_object_set_buffer,
                #[cfg(feature = "debug")]
                self.render_options.frustum_culling,
            );

            let offset = self.directional_shadow_model_instructions.len();
            let model_batches = &mut self.directional_shadow_model_batches[partition_index];
            let entity_instructions = &mut self.directional_shadow_entity_instructions[partition_index];

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_objects))]
            self.map.render_objects(
                self.directional_shadow_model_instructions,
                &object_set,
                self.animation_timer_ms,
                &partition_camera,
            );

            // Traps cast shadows like any other prop, so they belong in this
            // pass too — omitting them here would leave them floating.
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_objects))]
            self.map.render_props(
                self.directional_shadow_model_instructions,
                self.active_trap_props,
                self.animation_timer_ms,
                &partition_camera,
            );

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map))]
            self.map.render_ground(self.directional_shadow_model_instructions);

            let count = self.directional_shadow_model_instructions.len() - offset;

            model_batches.push(ModelBatch {
                offset,
                count,
                texture_set: self.map.get_texture_set().clone(),
                vertex_buffer: self.map.get_model_vertex_buffer().clone(),
                index_buffer: self.map.get_model_index_buffer().clone(),
            });

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map_tiles))]
            self.map
                .render_overlay_tiles(self.directional_shadow_model_instructions, model_batches, self.tile_texture_set);

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_pathing))]
            self.map.render_entity_pathing(
                self.directional_shadow_model_instructions,
                model_batches,
                entities,
                self.pathing_texture_set,
            );

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_ground_items))]
            self.map
                .render_ground_items(entity_instructions, ground_items, &partition_camera, self.client_tick);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
            self.map
                .render_entities(entity_instructions, entities, &partition_camera, self.client_tick);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
            self.map
                .render_dead_entities(entity_instructions, dead_entities, &partition_camera, self.client_tick);
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_point_lights(&mut self) {
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.enable_point_lights))]
        self.point_light_set.render_point_lights(self.point_light_instructions);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.enable_point_lights))]
        self.point_light_set.render_point_lights_with_shadows(
            self.map,
            self.point_shadow_camera,
            self.point_shadow_object_set_buffer,
            self.point_shadow_model_instructions,
            self.point_light_with_shadow_instructions,
            self.animation_timer_ms,
            #[cfg(feature = "debug")]
            self.render_options,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_geometry(&mut self) {
        let entities = self.client_state.follow(client_state().entities());
        let dead_entities = self.client_state.follow(client_state().dead_entities());
        let ground_items = self.client_state.follow(client_state().ground_items());
        let player = self.client_state.try_follow(this_entity());

        let offset = self.model_instructions.len();
        let object_set = self.map.cull_objects_with_frustum(
            self.current_camera,
            self.deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            self.render_options.frustum_culling,
        );

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_objects))]
        self.map.render_objects(
            self.model_instructions,
            &object_set,
            self.animation_timer_ms,
            self.current_camera,
        );

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_objects))]
        self.map.render_props(
            self.model_instructions,
            self.active_trap_props,
            self.animation_timer_ms,
            self.current_camera,
        );

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map))]
        self.map.render_ground(self.model_instructions);

        self.model_batches.push(ModelBatch {
            offset,
            count: self.model_instructions.len() - offset,
            texture_set: self.map.get_texture_set().clone(),
            vertex_buffer: self.map.get_model_vertex_buffer().clone(),
            index_buffer: self.map.get_model_index_buffer().clone(),
        });

        #[cfg(feature = "debug")]
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map_tiles))]
        self.map
            .render_overlay_tiles(self.model_instructions, self.model_batches, self.tile_texture_set);

        #[cfg(feature = "debug")]
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_pathing))]
        self.map
            .render_entity_pathing(self.model_instructions, self.model_batches, entities, self.pathing_texture_set);

        let entity_camera: &dyn Camera = match true {
            #[cfg(feature = "debug")]
            _ if self.render_options.show_entities_paper => self.player_camera,
            _ => self.current_camera,
        };

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_ground_items))]
        self.map
            .render_ground_items(self.entity_instructions, ground_items, entity_camera, self.client_tick);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
        self.map
            .render_entities(self.entity_instructions, entities, entity_camera, self.client_tick);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
        self.map
            .render_dead_entities(self.entity_instructions, dead_entities, entity_camera, self.client_tick);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
        self.emote_bubbles.render(
            self.entity_instructions,
            entities,
            player.as_deref(),
            dead_entities,
            entity_camera,
            self.client_tick,
        );

        // Classic sprite effects draw through the entity pipeline for the same
        // reason emotes do: they are ACT animations, so frame timing and
        // per-frame offsets come from the shared animation path.
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
        self.sprite_effects
            .render(self.entity_instructions, entity_camera, self.client_tick);

        #[cfg(feature = "debug")]
        if self.render_options.show_entities_debug {
            self.map.render_entities_debug(self.rectangle_instructions, entities, entity_camera);
            self.map
                .render_entities_debug(self.rectangle_instructions, dead_entities, entity_camera);
        }

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_water))]
        self.map.render_water(self.water_instruction, self.animation_timer_ms);
    }

    #[inline(always)]
    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    fn render_bounding_boxes(&mut self) {
        if self.render_options.show_bounding_boxes {
            let culling_camera: &dyn Camera = match self.currently_playing {
                true => self.player_camera,
                false => self.start_camera,
            };

            let object_set = self.map.cull_objects_with_frustum(
                culling_camera,
                self.bounding_box_object_set_buffer,
                self.render_options.frustum_culling,
            );

            self.map
                .render_bounding(self.aabb_instructions, self.render_options.frustum_culling, &object_set);
        }
    }

    /// Draws the ground area an armed skill will cover, under the cursor.
    ///
    /// The shape is the server's own layout (see [`skill_footprint`]), including
    /// the direction-dependent walls, so what the player aims is what they get.
    /// Tinted red when the target is out of range — the ground-cast path has no
    /// client-side range check, so without this the cast just fails silently.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_skill_aiming_footprint(&mut self) {
        /// Additive over lit terrain, so it stays readable without washing the
        /// ground out. See the batch-1 lesson on ground-effect alphas.
        const IN_RANGE: Color = Color::rgba(0.35, 0.75, 1.0, 0.5);
        const OUT_OF_RANGE: Color = Color::rgba(1.0, 0.3, 0.25, 0.5);

        if !self.currently_playing || self.is_interface_hovered {
            return;
        }

        let (Some(pending), Some(texture)) = (self.pending_skill, self.skill_footprint_texture) else {
            return;
        };

        let PickerTarget::Tile { x, y } = self.mouse_target else {
            return;
        };
        let target = TilePosition { x, y };

        let Some(player_position) = self.client_state.try_follow(this_entity()).map(Entity::get_tile_position) else {
            return;
        };

        let direction = facing_direction(player_position, target);
        let Some(cells) = skill_footprint(pending.skill_id, pending.skill_level, direction) else {
            return;
        };

        // A single cell is what the ordinary walk/target cursor already shows;
        // drawing a second marker on top of it just doubles the highlight.
        if cells.len() <= 1 {
            return;
        }

        let color = match is_within_skill_range(player_position, target, pending.attack_range) {
            true => IN_RANGE,
            false => OUT_OF_RANGE,
        };

        self.map
            .render_skill_footprint(self.effect_renderer, texture, target, &cells, color);
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_world_overlays(&mut self) {
        #[cfg(feature = "debug")]
        if let Some(marker_identifier) = self.hovered_marker_identifier {
            self.map.render_marker_overlay(
                self.aabb_instructions,
                self.circle_instructions,
                self.current_camera,
                marker_identifier,
                self.point_light_set,
                self.animation_timer_ms,
            );
        }

        self.particle_holder.render(
            self.bottom_interface_renderer,
            self.current_camera,
            self.screen_size,
            self.scaling,
        );

        self.effect_holder.render(self.effect_renderer, self.current_camera);

        self.render_skill_aiming_footprint();

        if let Some(player) = self.client_state.try_follow(this_entity()) {
            #[cfg(feature = "debug")]
            profile_block!("render player status");

            player.render_status(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().world_theme()),
                self.screen_size,
                self.client_tick,
            );
        }

        // Always-on green HP bars for party members visible on this map.
        {
            let party = self.client_state.follow(client_state().party_state());
            let party_account_ids: Vec<_> = party.members().iter().filter(|m| m.online()).map(|m| m.account_id().0).collect();
            if !party_account_ids.is_empty() {
                let theme = self.client_state.follow(client_state().world_theme());
                for entity in self.client_state.follow(client_state().entities()).iter().skip(1) {
                    if party_account_ids.contains(&entity.get_entity_id().0) {
                        entity.render_ally_status(self.middle_interface_renderer, self.current_camera, theme, self.screen_size);
                    }
                }
            }
        }

        if let Some(entity_id) = self.buffered_action.and_then(|action| action.target_entity_id())
            && let Some(entity) = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.render_status(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().world_theme()),
                self.screen_size,
                self.client_tick,
            );
        }

        match self.mouse_target {
            PickerTarget::Tile { x, y } => {
                // Only show if the mouse mode is default or walking.
                if self.currently_playing
                    && !self.is_interface_hovered
                    && (self.is_mouse_mode_default || self.last_walking_destination.is_some())
                {
                    let walk_indicator_color = *self.client_state.follow(client_state().world_theme().indicator().walking());

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_indicators))]
                    self.map
                        .render_walk_indicator(self.indicator_instruction, walk_indicator_color, TilePosition { x, y });
                }
            }
            PickerTarget::Entity(entity_id) => {
                if !self.is_interface_hovered && self.is_mouse_mode_default {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        // Since the buffered attack entity will render its status anyway,
                        // we make sure not to render it here again if it's the same.
                        if !self
                            .buffered_action
                            .is_some_and(|buffered_action| buffered_action.targets_entity(entity_id))
                        {
                            entity.render_status(
                                self.middle_interface_renderer,
                                self.current_camera,
                                self.client_state.follow(client_state().world_theme()),
                                self.screen_size,
                                self.client_tick,
                            );
                        }

                        if let Some(name) = &entity.get_details() {
                            let name = name.split('#').next().unwrap();
                            self.middle_interface_renderer
                                .render_hover_text(name, self.scaling, self.mouse_position);
                        }
                    } else if let Some(item) = self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        let name = self.library.get::<ItemName>(ItemNameKey {
                            item_id: item.item_id,
                            is_identified: item.is_identified,
                        });

                        // TODO: Don't allocate every frame
                        let text = format!("{name}: {}ea", item.quantity);
                        self.middle_interface_renderer
                            .render_hover_text(&text, self.scaling, self.mouse_position);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod skill_effect_asset_tests {
    use super::*;

    #[test]
    fn fire_bolt_hit_burst_does_not_delay_twice_after_the_impact_boundary() {
        let hit_effects = skill_hit_effects(SkillId(19));
        assert!(!hit_effects.is_empty());
        assert!(hit_effects.iter().all(|(_, _, start_delay)| *start_delay == 0.0));
    }

    /// Every asset a mapped skill recipe can reference must ship in the
    /// configured GRFs. Variant declarations are enumerated exhaustively; no
    /// random sampling is involved. Opens the
    /// multi-gigabyte archives; run explicitly with
    /// `cargo test -p korangar --lib all_mapped_skill_effect_assets_exist --
    /// --ignored`.
    #[test]
    #[ignore]
    fn all_mapped_skill_effect_assets_exist() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let mut paths = std::collections::BTreeSet::new();

        for skill_id in MAPPED_SKILL_IDS {
            let recipe = skill_presentation_recipe(*skill_id);
            for track in recipe.hit_effects {
                for effect_path in track.asset.variants() {
                    paths.insert(format!("data\\texture\\effect\\{effect_path}"));
                }
                if let Some(sprite_path) = track.asset.sprite_path() {
                    paths.insert(format!("data\\sprite\\{sprite_path}.spr"));
                    paths.insert(format!("data\\sprite\\{sprite_path}.act"));
                }
            }
            if let Some(track) = recipe.ground_effect {
                for effect_path in track.asset.variants() {
                    paths.insert(format!("data\\texture\\effect\\{effect_path}"));
                }
                if let Some(sprite_path) = track.asset.sprite_path() {
                    paths.insert(format!("data\\sprite\\{sprite_path}.spr"));
                    paths.insert(format!("data\\sprite\\{sprite_path}.act"));
                }
            }
            for sound in recipe
                .successful_caster_sounds
                .iter()
                .chain(recipe.damage_caster_sounds)
                .chain(recipe.damage_target_sounds)
                .chain(recipe.hit_sounds)
                .chain(recipe.ground_sounds)
            {
                for sound_path in sound.variants() {
                    paths.insert(format!("data\\wav\\{sound_path}"));
                }
            }
            match recipe.projectile {
                Some(ProjectileRecipe::FallingBolts(frame_paths)) => {
                    for frame_path in frame_paths {
                        paths.insert(format!("data\\texture\\{frame_path}"));
                    }
                }
                Some(ProjectileRecipe::TravelBall(kind)) => {
                    paths.insert(format!("data\\texture\\{}", kind.texture_path()));
                }
                Some(ProjectileRecipe::JupitelBall) => {
                    paths.insert(format!("data\\texture\\{JUPITEL_BALL_CORE_TEXTURE}"));
                    for frame_path in JUPITEL_BALL_FRAMES.iter().chain(JUPITEL_HIT_FRAMES) {
                        paths.insert(format!("data\\texture\\{frame_path}"));
                    }
                }
                Some(ProjectileRecipe::SpriteTravel { path, .. }) => {
                    paths.insert(format!("data\\sprite\\{path}.spr"));
                    paths.insert(format!("data\\sprite\\{path}.act"));
                }
                Some(ProjectileRecipe::Spear) | None => {}
            }
        }

        // Persistent skill-unit presentations (Phase E2).
        for unit_id in MAPPED_UNIT_IDS {
            let presentation = unit_presentation(*unit_id).expect("mapped unit must resolve");
            for str_path in presentation.intro_str.iter().chain(presentation.looping_str.iter()) {
                paths.insert(format!("data\\texture\\effect\\{str_path}"));
            }
            match presentation.body {
                Some(
                    UnitBody::Cylinders { texture, .. }
                    | UnitBody::IceHorns { texture }
                    | UnitBody::GroundQuad { texture, .. },
                ) => {
                    paths.insert(format!("data\\texture\\{texture}"));
                }
                Some(UnitBody::LoopingSprite { path, .. }) => {
                    paths.insert(format!("data\\sprite\\{path}.spr"));
                    paths.insert(format!("data\\sprite\\{path}.act"));
                }
                None => {}
            }
            if let Some(sound) = presentation.sound {
                paths.insert(format!("data\\wav\\{sound}"));
            }
        }

        // Assets referenced directly by the caster-effect recipes.
        for path in [
            "data\\texture\\effect\\이그니션브레이크.str",
            "data\\texture\\effect\\freeze.str",
            // E4 status visuals: the freeze pair the special-effect path now
            // distinguishes, plus the looping status STRs.
            "data\\texture\\effect\\freezed.str",
            "data\\texture\\effect\\stun.str",
            "data\\texture\\effect\\sleep.str",
            "data\\texture\\effect\\poison.str",
            "data\\texture\\effect\\silence.str",
            "data\\texture\\effect\\sonicblow.str",
            "data\\texture\\effect\\purpleslash.tga",
            "data\\texture\\effect\\ring2.bmp",
            "data\\sprite\\이팩트\\창.spr",
            "data\\wav\\effect\\assasin_sonicblow.wav",
            "data\\wav\\effect\\wizard_earthspike.wav",
        ] {
            paths.insert(path.to_owned());
        }
        // The ground-skill aiming footprint's per-cell tile.
        paths.insert(format!("data\\texture\\{SKILL_FOOTPRINT_TEXTURE}"));
        for path in E1_PROCEDURAL_TEXTURES {
            paths.insert(format!("data\\texture\\{path}"));
        }

        let missing: Vec<String> = paths
            .into_iter()
            .filter(|path| !game_file_loader.file_exists(&path.to_lowercase()))
            .collect();
        assert!(missing.is_empty(), "missing skill effect assets: {missing:#?}");
    }

}
