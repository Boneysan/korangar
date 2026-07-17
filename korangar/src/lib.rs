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

use std::io::Cursor;
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use cgmath::{Point3, Vector3};
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
    AttackRange, BuyShopItemsResult, CharacterServerInformation, ClientTick, Direction, DisappearanceReason, EntityId, ExperienceType,
    HotbarSlot, PartyId, SellItemsResult, SkillId, SkillLevel, SkillType, TilePosition, UnitId, WorldPosition,
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

const ROLLING_CUTTER_ID: SkillId = SkillId(2036);
const DEFAULT_MAP: &str = "geffen";
const START_CAMERA_FOCUS_POINT: Point3<f32> = Point3::new(600.0, 0.0, 240.0);
const DEFAULT_BACKGROUND_MUSIC: Option<&str> = Some("bgm\\01.mp3");
const MAIN_MENU_CLICK_SOUND_EFFECT: &str = "버튼소리.wav";
const ITEM_PICKUP_RANGE: AttackRange = AttackRange(1);

/// The animated fire-arrow frames of the classic `ef_firebolt` volley
/// (`불화살` = fire arrow).
const FIREBOLT_BOLT_FRAMES: &[&str] = &[
    "effect\\불화살1.tga",
    "effect\\불화살2.tga",
    "effect\\불화살3.tga",
    "effect\\불화살4.tga",
    "effect\\불화살5.tga",
    "effect\\불화살6.tga",
];
/// The classic `ef_coldbolt` volley projectile.
const COLDBOLT_BOLT_FRAMES: &[&str] = &["effect\\icearrow.tga"];
/// How long a bolt falls before it lands; hit bursts are delayed by this
/// much so they coincide with the impact (keep in sync with
/// `FallingBolts`).
const BOLT_LANDING_DELAY: f32 = 0.5;

fn firehit_effect_path() -> &'static str {
    match rand_aes::tls::rand_range_u32(1..=3) {
        1 => "firehit1.str",
        2 => "firehit2.str",
        _ => "firehit3.str",
    }
}

fn windhit_effect_path() -> &'static str {
    match rand_aes::tls::rand_range_u32(1..=3) {
        1 => "windhit1.str",
        2 => "windhit2.str",
        _ => "windhit3.str",
    }
}

fn meteor_effect_path() -> &'static str {
    match rand_aes::tls::rand_range_u32(1..=4) {
        1 => "meteor1.str",
        2 => "meteor2.str",
        3 => "meteor3.str",
        _ => "meteor4.str",
    }
}

fn firewall_effect_path() -> &'static str {
    match rand_aes::tls::rand_range_u32(1..=2) {
        1 => "firewall1.str",
        _ => "firewall2.str",
    }
}

/// M1-008: per-hit STR effects at the struck entity, wired like the original
/// client (roBrowser's skill/effect tables were the reference — semantics
/// only). Fire Bolt's burst waits for its bolt volley to land; Cold Bolt's
/// classic hit is sound-only, its visual is the volley itself; Thunderstorm
/// and Storm Gust play their large STRs at the targeted ground
/// (`GroundSkillEffect`), so their per-hit part is small or nothing.
fn skill_hit_effects(skill_id: SkillId) -> Vec<(&'static str, Color, f32)> {
    match skill_id.0 {
        // MG_SOULSTRIKE — code-drawn orbs in the original; the hit flash of
        // its direct upgrade (Soul Expansion) stands in for now.
        13 => vec![(
            "new_soulexpansion\\new_soulexpansion_hit\\new_soulexpansion_hit.str",
            Color::rgb_u8(190, 120, 255),
            0.0,
        )],
        // MG_FROSTDIVER — classic freeze/shatter on the target. The traveling
        // ice trail is code-drawn and remains a separate coverage item.
        15 => vec![("freeze.str", Color::rgb_u8(150, 225, 255), 0.0)],
        // MG_FIREBALL / MG_FIREWALL — classic fire-element hit burst.
        17 | 18 => vec![(firehit_effect_path(), Color::rgb_u8(255, 90, 25), 0.0)],
        // MG_FIREBOLT — classic firehit burst, timed to the volley landing.
        19 => vec![(firehit_effect_path(), Color::rgb_u8(255, 90, 25), BOLT_LANDING_DELAY)],
        // MG_LIGHTNINGBOLT — the classic strike plus a windhit burst.
        20 => vec![
            ("lightning.str", Color::rgb_u8(255, 240, 150), 0.0),
            (windhit_effect_path(), Color::rgb_u8(255, 240, 150), 0.0),
        ],
        // MG_THUNDERSTORM — per-target windhit; the storm itself plays at
        // the targeted ground.
        21 => vec![(windhit_effect_path(), Color::rgb_u8(255, 240, 150), 0.0)],
        // KN_PIERCE — classic earth/pierce hit.
        56 => vec![("earthhit.str", Color::rgb_u8(215, 180, 110), 0.0)],
        // PR_TURNUNDEAD / PR_MAGNUS — holy-element hit.
        77 | 79 => vec![("holyhit.str", Color::rgb_u8(255, 245, 190), 0.0)],
        // WZ_FIREPILLAR / WZ_SIGHTRASHER.
        80 => vec![("firepillarbomb.str", Color::rgb_u8(255, 80, 20), 0.0)],
        81 => vec![(firehit_effect_path(), Color::rgb_u8(255, 90, 25), 0.0)],
        // WZ_METEOR — only the fire impact belongs on the struck target. The
        // falling meteor starts earlier from GroundSkillEffect.
        83 => vec![(firehit_effect_path(), Color::rgb_u8(255, 90, 25), 0.0)],
        // WZ_VERMILION — per-target wind hit; the large field effect is
        // emitted by GroundSkillEffect.
        85 => vec![(windhit_effect_path(), Color::rgb_u8(245, 235, 150), 0.0)],
        // WZ_FROSTNOVA is handled once on the caster by the successful-use
        // event; its per-target hit is sound-only in the classic client.
        // WZ_EARTHSPIKE / WZ_HEAVENDRIVE. Their rising-rock geometry is still
        // code-drawn, but the shipped hit STR is independent and usable now.
        90 | 91 => vec![("earthhit.str", Color::rgb_u8(215, 180, 110), 0.0)],
        // Hunter trap bursts.
        118 => vec![("shockwavehit.str", Color::rgb_u8(210, 180, 255), 0.0)],
        119 => vec![("sandman.str", Color::rgb_u8(230, 205, 130), 0.0)],
        121 => vec![("freezing.str", Color::rgb_u8(150, 225, 255), 0.0)],
        122 => vec![("blastmine.str", Color::rgb_u8(255, 120, 35), 0.0)],
        123 => vec![("claymore.str", Color::rgb_u8(255, 80, 25), 0.0)],
        // AS_POISONREACT — small poison-react hit.
        139 => vec![("poisonreact.str", Color::rgb_u8(160, 90, 210), 0.0)],
        // AL_HOLYLIGHT — the classic effect table uses the holy-hit STR.
        156 => vec![("holyhit.str", Color::rgb_u8(255, 245, 190), 0.0)],
        _ => vec![],
    }
}

/// The classic falling-bolt volley for a skill's damage event, if any.
fn wizard_bolt_volley(skill_id: SkillId) -> Option<&'static [&'static str]> {
    match skill_id.0 {
        14 => Some(COLDBOLT_BOLT_FRAMES),
        19 => Some(FIREBOLT_BOLT_FRAMES),
        _ => None,
    }
}

/// Ground-cast area effects played at the targeted position when
/// `ZC_NOTIFY_GROUNDSKILL` arrives, exactly like the original client.
fn ground_skill_effect(skill_id: SkillId) -> Option<(&'static str, Color)> {
    match skill_id.0 {
        // MG_FIREWALL — cast flash; persistent cells remain AddSkillUnit.
        18 => Some((firewall_effect_path(), Color::rgb_u8(255, 45, 10))),
        // MG_THUNDERSTORM
        21 => Some(("thunderstorm.str", Color::rgb_u8(255, 240, 150))),
        // PR_SANCTUARY / PR_MAGNUS — initial cast effects. Their persistent
        // ground cylinders are a separate skill-unit renderer task.
        70 => Some(("sanctuary.str", Color::rgb_u8(130, 255, 175))),
        79 => Some(("magnus.str", Color::rgb_u8(255, 225, 170))),
        // WZ_FIREPILLAR / WZ_VERMILION.
        80 => Some(("firepillar.str", Color::rgb_u8(255, 65, 15))),
        // WZ_METEOR — the full falling meteor begins at cast completion;
        // per-target damage later adds only the fire impact.
        83 => Some((meteor_effect_path(), Color::rgb_u8(255, 95, 25))),
        85 => Some(("lord.str", Color::rgb_u8(245, 235, 150))),
        // WZ_STORMGUST
        89 => Some(("stormgust.str", Color::rgb_u8(175, 225, 255))),
        // WZ_QUAGMIRE
        92 => Some(("quagmire.str", Color::rgb_u8(135, 105, 75))),
        // BS_HAMMERFALL / HT_SKIDTRAP / AS_VENOMDUST.
        110 => Some(("crashearth.str", Color::rgb_u8(235, 190, 95))),
        115 => Some(("skidtrap.str", Color::rgb_u8(235, 205, 90))),
        140 => Some(("venomdust.str", Color::rgb_u8(140, 75, 190))),
        _ => None,
    }
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
    use ragnarok_packets::{EntityId, SkillType, TilePosition};

    use super::{PendingCastResolution, resolve_pending_cast};
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
}

/// A skill armed for targeting. Pressing a targeted skill's hotbar key while the
/// cursor is not over a valid target arms it here; the next left-click picks the
/// target (RO's press-skill → reticle → click-target flow). Cancelled by
/// right-click or Escape.
#[derive(Clone, Debug)]
struct PendingSkill {
    skill_id: SkillId,
    skill_level: SkillLevel,
    skill_type: SkillType,
    skill_name: String,
}

/// What a left-click resolves to while a skill is armed, given what the cursor is
/// over. Kept separate from the cast itself so the decision is pure and testable.
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

/// Cast `pending` at whatever `target` currently resolves to. Returns `true` if a
/// cast was sent (the target is consumed and the skill should disarm), `false` if
/// it fizzled and the skill should stay armed. Shared by the hover-then-press fast
/// path and the armed click path so both agree. Takes the two fields it needs by
/// reference rather than `&mut self` so callers can hold unrelated field borrows
/// (e.g. the per-frame point-light borrow in the render path).
fn perform_pending_cast<Callback: PacketCallback + Send>(
    networking_system: &mut NetworkingSystem<Callback>,
    state: &State<ClientState>,
    pending: &PendingSkill,
    target: PickerTarget,
) -> bool {
    match resolve_pending_cast(pending.skill_type, target) {
        PendingCastResolution::CastEntity(entity_id) => {
            let _ = networking_system.cast_skill(pending.skill_id, pending.skill_level, entity_id);
            true
        }
        PendingCastResolution::CastTile(tile) => {
            let _ = networking_system.cast_ground_skill(pending.skill_id, pending.skill_level, tile);
            true
        }
        PendingCastResolution::CastEntityTile(entity_id) => {
            // A ground skill clicked on an entity centres the AoE on that entity's cell.
            match state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == entity_id)
                .map(|entity| entity.get_tile_position())
            {
                Some(tile) => {
                    let _ = networking_system.cast_ground_skill(pending.skill_id, pending.skill_level, tile);
                    true
                }
                None => false,
            }
        }
        PendingCastResolution::Fizzle => false,
    }
}

/// Tell the player which skill is now armed and waiting for a target. The targeting
/// reticle alone doesn't say *which* skill it is (and skills currently draw no cast
/// effect), so this line is how arming — and a swap — is visible. Takes the chat
/// state by reference so it can be called inside the input-drain loop, which holds
/// a borrow of `self` that a `&mut self` method would conflict with.
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
    #[cfg(feature = "debug")]
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
    /// A targeted skill awaiting a click to pick its target. See [`PendingSkill`].
    pending_skill: Option<PendingSkill>,
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

    particle_holder: ParticleHolder,
    emote_bubbles: EmoteBubbles,
    point_light_manager: PointLightManager,
    effect_holder: EffectHolder,
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
    client_state: State<ClientState>,
}

impl Client {
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
            let point_light_manager = PointLightManager::new();
            let effect_holder = EffectHolder::default();
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
            #[cfg(feature = "debug")]
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
            network_event_buffer,
            saved_login_data,
            saved_character_server,
            saved_login_server_address,
            saved_password,
            saved_username,
            saved_packet_version,
            particle_holder,
            emote_bubbles,
            point_light_manager,
            effect_holder,
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

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn handle_network_events(&mut self, client_tick: ClientTick) {
        // Keep HUD cooldown line and timed compass marks in sync with server tick.
        self.client_state.follow_mut(client_state().skill_cooldowns()).tick(client_tick);
        self.client_state.follow_mut(client_state().minimap()).tick_markers(client_tick);
        self.networking_system.get_events(&mut self.network_event_buffer);

        // Deferred: cannot call &mut self helpers while draining network_event_buffer.
        let mut open_storage_ui = false;

        for event in self.network_event_buffer.drain() {
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

                    *self.client_state.follow_mut(client_state().character_servers()) = character_servers;

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.interface
                        .open_window(ServerSelectionWindow::new(client_state().character_servers()));
                }
                NetworkEvent::LoginServerConnectionFailed { message, .. } => {
                    self.networking_system.disconnect_from_login_server();

                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::LoginServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        let socket_address = self.saved_login_server_address.unwrap();
                        self.networking_system.connect_to_login_server(
                            self.saved_packet_version,
                            socket_address,
                            &self.saved_username,
                            &self.saved_password,
                        );
                    }
                }
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_slot_count(normal_slot_count);

                    let _ = self.networking_system.request_character_list();
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                    self.networking_system.disconnect_from_character_server();

                    // The login auth token is single-use and expires 30 seconds after
                    // login (Hercules AUTH_TIMEOUT), so a rejected character server
                    // connection can never succeed by retrying from the server
                    // selection screen. Drop the dead session and return to the login
                    // window so the user can simply log in again.
                    self.networking_system.disconnect_from_login_server();
                    self.saved_login_data = None;
                    self.saved_character_server = None;

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.interface.open_window(LoginWindow::new(
                        client_state().login_window(),
                        client_state().login_settings(),
                        client_state().client_info(),
                    ));

                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::CharacterServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        // The saved session is cleared when we intentionally return to
                        // the login screen (e.g. after the character server rejected
                        // us) — don't try to revive a dead session then.
                        if let (Some(login_data), Some(server)) = (self.saved_login_data.as_ref(), self.saved_character_server.clone()) {
                            self.networking_system
                                .connect_to_character_server(self.saved_packet_version, login_data, server);
                        }
                    } else if !self.networking_system.is_map_server_connected() && self.saved_login_data.is_some() {
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

                    let login_data = self.saved_login_data.as_ref().unwrap();
                    let server = self.saved_character_server.clone().unwrap();
                    self.networking_system
                        .connect_to_character_server(self.saved_packet_version, login_data, server);

                    self.map = None;

                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    self.client_state.follow_mut(client_state().entities()).clear();
                    self.client_state.follow_mut(client_state().dead_entities()).clear();
                    self.client_state.follow_mut(client_state().ground_items()).clear();
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
                    let entity_part_files = player.get_entity_part_files(&self.library);

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
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
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
                        let entity_part_files = npc.get_entity_part_files(&self.library);

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
                    if buffered_action.is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id)) {
                        *buffered_action = None;
                    }

                    // Drop any effect attached to the entity, like a warp's
                    // portal vortex.
                    self.effect_holder.remove_unit(entity_id);
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
                    self.emote_bubbles.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    // Only the player must stay alive between map changes.
                    self.client_state.follow_mut(client_state().entities()).truncate(1);
                    self.client_state.follow_mut(client_state().dead_entities()).clear();
                    self.client_state.follow_mut(client_state().ground_items()).clear();
                    self.client_state.follow_mut(client_state().status_effects()).clear();
                    self.client_state.follow_mut(client_state().skill_cooldowns()).clear();
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.clear_cast();
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
                    damage_amount,
                    hit_count,
                    attack_duration,
                    is_critical,
                } => {
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

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                    // TODO: Maybe also or_else this_entity?
                    {
                        if let Some(target_position) = target_position {
                            entity.rotate_towards(target_position);
                        }

                        entity.set_attack(attack_duration, is_critical, client_tick);
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == destination_entity_id)
                        .or_else(|| self.client_state.try_follow(this_entity()))
                    {
                        let particle: Box<dyn Particle + Send + Sync> = match damage_amount {
                            Some(amount) => Box::new(DamageNumber::new(entity.get_position(), amount.to_string(), is_critical)),
                            None => Box::new(Miss::new(entity.get_position())),
                        };

                        self.particle_holder.spawn_particle(particle);

                        if let Some(skill_id) = skill_id {
                            let target_position = entity.get_position();

                            if let Some(frame_paths) = wizard_bolt_volley(skill_id) {
                                let textures: Vec<_> = frame_paths
                                    .iter()
                                    .filter_map(|path| self.texture_loader.get_or_load(path, ImageType::Color).ok())
                                    .collect();
                                self.effect_holder.add_effect(Box::new(FallingBolts::new(
                                    textures,
                                    target_position,
                                    hit_count,
                                    Color::WHITE,
                                )));
                            }

                            for (effect_path, light_color, start_delay) in skill_hit_effects(skill_id) {
                                match self.effect_loader.get_or_load(effect_path, &self.texture_loader) {
                                    Ok(effect) => {
                                        let frame_timer = effect.new_frame_timer();
                                        self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                                            effect,
                                            frame_timer,
                                            EffectCenter::Position(target_position),
                                            Vector3::new(0.0, 0.0, 0.0),
                                            PointLightId::new(destination_entity_id.0 ^ u32::from(skill_id.0)),
                                            Vector3::new(0.0, 6.0, 0.0),
                                            light_color,
                                            45.0,
                                            false,
                                            start_delay,
                                        )));
                                    }
                                    Err(error) => {
                                        eprintln!("[skill-effect] failed to load {effect_path}: {error:?}");
                                    }
                                }
                            }
                        }
                    }
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

                    // WZ_FROSTNOVA: the original client plays freeze.str once
                    // on the caster at successful skill use. This packet is
                    // sent even when no enemy is in range.
                    if successful && skill_id.0 == 88 {
                        let source_position = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|source| source.get_entity_id() == source_entity_id)
                            .map(|source| source.get_position())
                            .or_else(|| self.client_state.try_follow(this_entity()).map(|source| source.get_position()));

                        if let Some(source_position) = source_position
                            && self.effect_holder.claim_unique_skill_effect(source_entity_id, skill_id, 0.5)
                        {
                            match self.effect_loader.get_or_load("freeze.str", &self.texture_loader) {
                                Ok(effect) => {
                                    let frame_timer = effect.new_frame_timer();
                                    self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                                        effect,
                                        frame_timer,
                                        EffectCenter::Position(source_position),
                                        Vector3::new(0.0, 0.0, 0.0),
                                        PointLightId::new(source_entity_id.0 ^ u32::from(skill_id.0)),
                                        Vector3::new(0.0, 6.0, 0.0),
                                        Color::rgb_u8(145, 220, 255),
                                        55.0,
                                        false,
                                        0.0,
                                    )));
                                }
                                Err(error) => eprintln!("[skill-effect] failed to load freeze.str: {error:?}"),
                            }
                        }
                    }
                }
                NetworkEvent::StatusChange {
                    entity_id,
                    index,
                    gained,
                    duration_ms,
                    remaining_ms,
                } => {
                    // Only track status effects on the local player for this slice (see
                    // buff-bar-slice.md).
                    let local_id = self
                        .client_state
                        .follow(client_state().entities())
                        .first()
                        .map(|e| e.get_entity_id());
                    if Some(entity_id) == local_id {
                        let effects = self.client_state.follow_mut(client_state().status_effects());
                        if gained {
                            effects.apply(index, duration_ms, remaining_ms);
                        } else {
                            effects.remove(index);
                        }
                    }
                }
                NetworkEvent::StateChange {
                    entity_id,
                    option,
                    body_state: _,
                    health_state: _,
                } => {
                    // M1-007. Applies to every visible entity, not just the local player:
                    // the server only sends state changes for entities we are allowed to
                    // see, so a conceal flag here means "draw it translucent", never
                    // "reveal something hidden from us". `body_state` / `health_state`
                    // (stun, poison, …) are parsed but not yet surfaced.
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.update_option(option);
                    }
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
                    self.client_state
                        .follow_mut(client_state().trade_state())
                        .add_partner_item(item_id, amount, identified, refine);
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
                                (item.item_id, amount, format!("item {} x{amount}", item.item_id.0))
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
                }
                NetworkEvent::ChangeJob { account_id, job_id } => {
                    let layout = self.async_loader.request_skill_tree_layout_load(job_id, client_tick);
                    *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = layout.tabs.len().saturating_sub(1);
                    *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .clear();

                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                        .unwrap();

                    // FIX: A job change does not automatically send packets for the
                    // inventory and for unequipping items. We should probably manually
                    // request a full list of items and the hotbar.

                    entity.set_job(&self.library, job_id);

                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }
                }
                NetworkEvent::ChangeHair { account_id, hair_id } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                        .unwrap();

                    entity.set_hair(hair_id as usize);

                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }
                }
                NetworkEvent::LoggedOut => {
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
                NetworkEvent::AddSkillUnit {
                    entity_id,
                    unit_id,
                    position,
                } => {
                    let Some(map) = &self.map else {
                        continue;
                    };

                    match unit_id {
                        UnitId::Firewall => {
                            let Some(position) = map.get_world_position(position) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                                continue;
                            };

                            let effect = self.effect_loader.get_or_load("firewall.str", &self.texture_loader).unwrap();
                            let frame_timer = effect.new_frame_timer();

                            self.effect_holder.add_unit(
                                Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    PointLightId::new(unit_id as u32),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    Color::rgb_u8(255, 30, 0),
                                    60.0,
                                    true,
                                    0.0,
                                )),
                                entity_id,
                            );
                        }
                        UnitId::Pneuma => {
                            let Some(position) = map.get_world_position(position) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                                continue;
                            };

                            let effect = self.effect_loader.get_or_load("pneuma1.str", &self.texture_loader).unwrap();
                            let frame_timer = effect.new_frame_timer();

                            self.effect_holder.add_unit(
                                Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    PointLightId::new(unit_id as u32),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    Color::rgb_u8(83, 220, 108),
                                    40.0,
                                    false,
                                    0.0,
                                )),
                                entity_id,
                            );
                        }
                        _ => {}
                    }
                }
                NetworkEvent::RemoveSkillUnit { entity_id } => {
                    self.effect_holder.remove_unit(entity_id);
                }
                NetworkEvent::GroundSkillEffect { skill_id, position, .. } => {
                    // The original client plays ground-cast area effects
                    // (Thunderstorm, Storm Gust) from this packet at the
                    // targeted position, independent of any damage landing.
                    if let Some((effect_path, light_color)) = ground_skill_effect(skill_id)
                        && let Some(map) = &self.map
                        && let Some(world_position) = map.get_world_position(position)
                    {
                        match self.effect_loader.get_or_load(effect_path, &self.texture_loader) {
                            Ok(effect) => {
                                let frame_timer = effect.new_frame_timer();
                                self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(world_position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    PointLightId::new(
                                        u32::from(position.x) ^ (u32::from(position.y) << 16) ^ u32::from(skill_id.0),
                                    ),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    light_color,
                                    60.0,
                                    false,
                                    0.0,
                                )));
                            }
                            Err(error) => {
                                eprintln!("[skill-effect] failed to load {effect_path}: {error:?}");
                            }
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
                        && let Some(animation_data) = self.async_loader.request_animation_data_load(
                            EMOTE_ANIMATION_ENTITY_ID,
                            EntityType::Npc,
                            vec![EMOTE_SPRITE_FILE.to_string()],
                        )
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
                    let local_entity_name = is_local_entity
                        .then(|| self.client_state.follow(client_state().player_name()).to_owned());
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
                    source_entity_id, cast_ms, ..
                } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player())
                        && player.get_common().entity_id == source_entity_id
                    {
                        player.start_cast(cast_ms, client_tick);
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
                    if let Some(map) = &self.map
                        && self.client_state.try_follow_mut(this_entity()).is_some()
                        // Make sure that the entity is on screen.
                        && self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .any(|entity| entity.get_entity_id() == target_entity_id)
                        && let Some(path) =
                            self.path_finder
                                .find_walkable_path_in_range(&**map, player_position, target_position, attack_range)
                    {
                        let nearest_tile = path.last().unwrap();

                        let _ = self.networking_system.player_move(WorldPosition {
                            x: nearest_tile.x,
                            y: nearest_tile.y,
                            direction: Direction::North,
                        });

                        *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity {
                            entity_id: target_entity_id,
                        });
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
                    let item_name = self
                        .library
                        .get::<ItemName>(ItemNameKey {
                            item_id,
                            is_identified: true,
                        })
                        .to_string();
                    let item_name = match item_name.as_str() {
                        "NOTFOUND" => format!("item #{}", item_id.0),
                        _ => item_name,
                    };
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
                                self.interface.open_window(ErrorWindow::new(format!(
                                    "Selected server has an unsupported package version: {packet_version}"
                                )));
                                continue;
                            }
                        },
                        None => FALLBACK_PACKET_VERSION,
                    };

                    self.saved_login_server_address = Some(socket_address);
                    self.saved_username = username.clone();
                    self.saved_password = password.clone();
                    self.saved_packet_version = packet_version;

                    self.networking_system
                        .connect_to_login_server(packet_version, socket_address, username, password);
                }
                InputEvent::SelectServer {
                    character_server_information,
                } => {
                    self.saved_character_server = Some(character_server_information.clone());

                    self.networking_system.disconnect_from_login_server();

                    // Korangar should never attempt to connect to the character
                    // server before it logged in to the login server, so it's fine to
                    // unwrap here.
                    let login_data = self.saved_login_data.as_ref().unwrap();
                    self.networking_system
                        .connect_to_character_server(self.saved_packet_version, login_data, character_server_information);
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
                    // Escape backs out of the most recent transient state first: if a skill is
                    // armed, cancel the target instead of opening the menu (secondary cancel
                    // gesture alongside right-click).
                    if self.pending_skill.is_some() {
                        self.pending_skill = None;
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
                                this_player().manually_asserted().skill_points(),
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
                InputEvent::CastSkill { slot } => {
                    // Resolve the slot to owned data under an immutable borrow, then act — so the
                    // cast / arm / chat-feedback below can borrow self mutably without conflict.
                    let learnable_skill = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).clone();
                    let skill_type = learnable_skill.as_ref().and_then(|learnable_skill| {
                        self.client_state
                            .follow(client_state().skill_tree().skills())
                            .iter()
                            .find(|learned_skill| {
                                learned_skill.skill_id == learnable_skill.skill_id
                                    && learned_skill.skill_level.0 >= learnable_skill.maximum_level.0
                            })
                            .map(|learned_skill| learned_skill.skill_type)
                    });

                    if let (Some(learnable_skill), Some(skill_type)) = (learnable_skill, skill_type) {
                        match skill_type {
                            SkillType::Passive => {}
                            SkillType::SelfCast => {
                                let this_entity_id = self.client_state.follow(this_entity().manually_asserted()).get_entity_id();
                                match learnable_skill.skill_id == ROLLING_CUTTER_ID {
                                    true => {
                                        let _ = self.networking_system.cast_channeling_skill(
                                            learnable_skill.skill_id,
                                            learnable_skill.maximum_level,
                                            this_entity_id,
                                        );
                                    }
                                    false => {
                                        let _ =
                                            self.networking_system
                                                .cast_skill(learnable_skill.skill_id, learnable_skill.maximum_level, this_entity_id);
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
                                let _ = self
                                    .networking_system
                                    .cast_skill(learnable_skill.skill_id, learnable_skill.maximum_level, target_id);
                            }
                            SkillType::Attack => {
                                // Entity-target: fast-cast if the cursor is already over a target,
                                // otherwise arm and wait for the next left-click to pick one.
                                let pending = PendingSkill {
                                    skill_id: learnable_skill.skill_id,
                                    skill_level: learnable_skill.maximum_level,
                                    skill_type,
                                    skill_name: learnable_skill.skill_name.clone(),
                                };
                                if !perform_pending_cast(
                                    &mut self.networking_system,
                                    &self.client_state,
                                    &pending,
                                    input_report.mouse_target,
                                ) {
                                    announce_armed_skill(&mut self.client_state, &pending.skill_name);
                                    self.pending_skill = Some(pending);
                                }
                            }
                            SkillType::Ground | SkillType::Trap => {
                                // Ground-target: always arm so the player aims the placement reticle
                                // and clicks where the AoE lands, rather than dropping it instantly at
                                // wherever the cursor happens to sit when the key is pressed.
                                announce_armed_skill(&mut self.client_state, &learnable_skill.skill_name);
                                self.pending_skill = Some(PendingSkill {
                                    skill_id: learnable_skill.skill_id,
                                    skill_level: learnable_skill.maximum_level,
                                    skill_type,
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

    /// Drain finished async loads from the loader thread and apply their
    /// results to client state. This is the only place that promotes
    /// `self.map` from `None` to `Some` (when a map load completes), so it
    /// must run before the `self.map` check in [`Self::update_and_render`].
    ///
    /// Called as late as possible in the frame to give the loader thread the
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
                    } else if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
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

                            if let Some(position) = position {
                                // `manually_asserted` is safe because we are in the branch where `this_player`
                                // is not `None`.
                                let player = self.client_state.follow_mut(this_entity().manually_asserted());

                                player.set_position(map, position, client_tick);
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
    /// still moving (attack, pick up item). Must be called after
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
                            if !perform_pending_cast(
                                &mut self.networking_system,
                                &self.client_state,
                                &pending,
                                input_report.mouse_target,
                            ) {
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
                currently_playing,
                is_mouse_mode_default,
                is_interface_hovered: interface_frame.is_interface_hovered(),
                last_walking_destination,
                buffered_action: *self.client_state.follow(client_state().buffered_action()),
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

        if let Some(BufferedAction::AttackEntity { entity_id }) = self.buffered_action
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
                            .is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id))
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
