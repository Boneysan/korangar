use std::string::String;
use std::sync::Arc;

use arrayvec::ArrayVec;
use cgmath::{EuclideanSpace, Point3, Vector2, VectorSpace};
use korangar_audio::AudioEngine;
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
use korangar_interface::element::StateElement;
use korangar_interface::window::{StateWindow, Window};
use korangar_networking::EntityData;
use ragnarok_packets::{
    AccountId, AttackRange, CharacterInformation, ClientTick, Direction, DisappearanceReason, EntityId, EntityOption, ItemId, JobId, Sex,
    SkillId, SpriteChangeType, StatType, TilePosition, WorldPosition,
};
use rust_state::{Path, RustState, VecItem};
#[cfg(feature = "debug")]
use smallvec::smallvec_inline;
#[cfg(feature = "debug")]
use wgpu::{BufferUsages, Device, Queue};

use crate::Color;
#[cfg(feature = "debug")]
use crate::graphics::reduce_vertices;
#[cfg(feature = "debug")]
use crate::graphics::{BindlessSupport, DebugRectangleInstruction};
use crate::graphics::{EntityInstruction, ScreenPosition, ScreenSize};
use crate::loaders::GameFileLoader;
#[cfg(feature = "debug")]
use crate::loaders::{GAT_TILE_SIZE, split_mesh_by_texture};
use crate::renderer::GameInterfaceRenderer;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
use crate::state::ClientState;
use crate::state::theme::{InterfaceThemeType, WorldTheme};
use crate::world::{
    AccessoryName, AccessoryNameKey, ActionEvent, AnimationData, AnimationState, Camera, FadeDirection, FadeState, IsBabyJob, JobIdentity,
    Library, MAX_WALK_PATH_SIZE, Map, PathFinder, StatusTint, native_real_weapon_id,
};
#[cfg(feature = "debug")]
use crate::world::{MarkerIdentifier, SubMesh};
#[cfg(feature = "debug")]
use crate::{Buffer, ModelVertex};

const MALE_HAIR_LOOKUP: &[usize] = &[2, 2, 1, 7, 5, 4, 3, 6, 8, 9, 10, 12, 11];
const FEMALE_HAIR_LOOKUP: &[usize] = &[2, 2, 4, 7, 1, 5, 3, 6, 12, 10, 9, 11, 8];
const SPATIAL_SOUND_RANGE: f32 = 250.0;
const FADE_IN_DURATION_MS: u32 = 500;
const BABY_JOB_SCALE: f32 = 0.75;
const SI_TRICKDEAD: u16 = 29;
const SI_SU_STOOP: u16 = 893;
const SI_SUHIDE: u16 = 933;
/// `opt1` is an enum (one state at a time), `opt2` a bitmask. Values from
/// Hercules `src/map/status.h`. Note the packet field names are swapped
/// relative to their contents: `body_state` carries opt1, `health_state` opt2.
const OPT1_STONE: u16 = 1;
const OPT1_FREEZE: u16 = 2;
const OPT1_STUN: u16 = 3;
/// Petrif*ying* — the target still walks and attacks through this phase.
const OPT1_STONEWAIT: u16 = 6;

const OPT1_SLEEP: u16 = 4;

const OPT2_POISON: u16 = 0x0001;
const OPT2_CURSE: u16 = 0x0002;
const OPT2_SILENCE: u16 = 0x0004;
const OPT2_BLIND: u16 = 0x0010;
const OPT2_DEADLY_POISON: u16 = 0x0080;

/// Whether a status holds the sprite still.
///
/// Hercules blocks movement for **every** `opt1` state except `OPT1_STONEWAIT`
/// and `OPT1_BURNING` (`unit.c:1304`), so a stunned or sleeping entity is
/// standing still server-side — its sprite must not keep walking through an
/// idle loop. `STONEWAIT` is deliberately excluded: the *petrifying* phase
/// still moves and attacks until the wait timer flips it to `OPT1_STONE`, which
/// is the same distinction [`Common::status_tint`] draws.
pub fn status_freezes_animation(body_state: u16) -> bool {
    matches!(body_state, OPT1_STONE | OPT1_FREEZE | OPT1_STUN | OPT1_SLEEP)
}

/// The looping STR played on an entity for as long as a status is active.
///
/// Only assets confirmed present in the GRF appear here (probed 2026-07-26):
/// `stun.str`, `sleep.str`, `poison.str`, `silence.str` exist; `curse.str`,
/// `blind.str` and `stone.str` do **not**, so those statuses stay tint-only
/// rather than getting an invented stand-in.
///
/// Freeze and petrification are deliberately absent: both already read clearly
/// through [`Common::status_tint`] plus the paused animation, and the `freeze`
/// STRs are the one-shot application flash fired by the special-effect packet,
/// not a loop.
pub fn status_effect_asset(body_state: u16, health_state: u16) -> Option<&'static str> {
    match body_state {
        OPT1_STUN => return Some("stun.str"),
        OPT1_SLEEP => return Some("sleep.str"),
        _ => {}
    }

    // opt2 is a bitmask and several can be set at once; pick the most
    // incapacitating so the entity never carries two loops.
    if health_state & (OPT2_POISON | OPT2_DEADLY_POISON) != 0 {
        Some("poison.str")
    } else if health_state & OPT2_SILENCE != 0 {
        Some("silence.str")
    } else {
        None
    }
}

#[derive(Clone)]
pub enum ResourceState<T> {
    Available(T),
    Unavailable,
    Requested,
}

impl<T> ResourceState<T> {
    pub fn as_option(&self) -> Option<&T> {
        match self {
            ResourceState::Available(value) => Some(value),
            _requested_or_unavailable => None,
        }
    }
}

#[derive(Clone, RustState, StateElement)]
pub struct Movement {
    #[hidden_element]
    steps: ArrayVec<Step, MAX_WALK_PATH_SIZE>,
    starting_timestamp: u32,
    #[cfg(feature = "debug")]
    #[hidden_element]
    pub pathing: Option<Pathing>,
}

impl Movement {
    pub fn new(steps: ArrayVec<Step, MAX_WALK_PATH_SIZE>, starting_timestamp: u32) -> Self {
        Self {
            steps,
            starting_timestamp,
            #[cfg(feature = "debug")]
            pathing: None,
        }
    }
}

#[cfg(feature = "debug")]
#[derive(Clone)]
pub struct Pathing {
    pub vertex_buffer: Arc<Buffer<ModelVertex>>,
    pub index_buffer: Arc<Buffer<u32>>,
    pub submeshes: Vec<SubMesh>,
}

#[derive(Copy, Clone)]
pub struct Step {
    arrival_position: TilePosition,
    arrival_timestamp: u32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EntityType {
    Hidden,
    Monster,
    Npc,
    Player,
    Warp,
}

impl From<JobId> for EntityType {
    fn from(job_id: JobId) -> Self {
        match job_id.0 {
            45 => EntityType::Warp,
            111 | 139 => EntityType::Hidden,
            0..=44 | 4000..=5999 => EntityType::Player,
            46..=999 | 10000..=19999 => EntityType::Npc,
            1000..=3999 | 20000..=29999 => EntityType::Monster,
            _ => EntityType::Npc,
        }
    }
}

#[cfg(test)]
mod entity_type_tests {
    use ragnarok_packets::{JobId, SkillId};

    use super::{EntityType, native_impact_extra_delay_ms};

    #[test]
    fn modern_hidden_warp_npc_is_not_rendered() {
        assert_eq!(EntityType::from(JobId(139)), EntityType::Hidden);
    }

    #[test]
    fn native_projectile_delay_exceptions_are_route_specific() {
        assert_eq!(native_impact_extra_delay_ms(JobId(1016), None), 192);
        assert_eq!(native_impact_extra_delay_ms(JobId(1285), None), 912);
        assert_eq!(native_impact_extra_delay_ms(JobId(1286), None), 408);
        assert_eq!(native_impact_extra_delay_ms(JobId(1285), Some(SkillId(1))), 0);
        assert_eq!(native_impact_extra_delay_ms(JobId(1420), Some(SkillId(1))), 192);
    }
}

#[derive(Clone, RustState, StateElement)]
pub struct Common {
    pub entity_id: EntityId,
    pub job_id: JobId,
    /// Spirit spheres orbiting this entity (Monk spheres, Gunslinger coins).
    /// Stored from `ZC_SPIRITS`; drawing them is a separate asset task.
    pub spirit_spheres: u16,
    pub health_points: usize,
    pub maximum_health_points: usize,
    pub movement_speed: usize,
    pub direction: Direction,
    pub head_direction: usize,
    pub sex: Sex,
    /// Hair style id, from the spawn packet's `head` field.
    ///
    /// Lives here rather than on [`Player`] because **remote players are built
    /// as `Entity::Npc`**, which uses this struct. While it was
    /// Player-only, every observer resolved head `1` (the `_ => 1` fallback
    /// in `get_entity_part_files`) for everybody else — permanently, not
    /// just after a change — and `set_hair` silently no-opped for them.
    pub head: usize,
    pub weapon: u32,
    pub shield: u32,
    /// Lower headgear view id (`vd->head_bottom`).
    ///
    /// This and the six fields below live on `Common` for the same reason
    /// `head` does — remote players are `Entity::Npc` — and they are seeded
    /// from [`EntityData`], so an `AddEntity` rebuild reproduces them
    /// instead of wiping them. That is why they do **not** need the
    /// off-entity map `remote_ammunition` uses: ammunition is the one
    /// attribute the spawn packet does not carry, so it alone has no
    /// rebuild-safe home.
    ///
    /// Wire names, like `head` and `option` above, so the observer-parity
    /// audits can diff these field names against `EntityData`'s directly.
    ///
    /// Stored but not yet drawn — sprite composition is still body + head +
    /// weapon + shield. Rendering them needs accessory sprite paths and palette
    /// files, which this tree does not have yet.
    pub accessory: u16,
    /// Upper headgear view id (`vd->head_top`).
    pub accessory2: u16,
    /// Middle headgear view id (`vd->head_mid`).
    pub accessory3: u16,
    /// Hair colour palette (`vd->hair_color`).
    pub head_palette: u16,
    /// Clothes colour palette (`vd->cloth_color`).
    pub body_palette: u16,
    /// Garment / robe view id (`vd->robe`).
    pub robe: u16,
    /// Alternate body style (`vd->body_style`, the `LOOK_BODY2` slot).
    pub body: u16,
    #[hidden_element]
    pub entity_type: EntityType,
    /// Raw `sc->option` from `ZC_STATE_CHANGE` (M1-007). Interpret via
    /// [`EntityOption`]; `0` until the server says otherwise.
    pub option: u32,
    /// Raw `sc->opt1` / spawn `bodyState`.
    pub body_state: u16,
    /// Raw `sc->opt2` / spawn `healthState`.
    pub health_state: u16,
    /// Packet `isPKModeON`; native actor field `+0x2C0`.
    pub is_pk_mode_on: bool,
    /// True on town / safe maps (no monsters). Relaxes an armed player's
    /// standing pose from the battle-ready ReadyFight stance back to Idle.
    pub in_safe_zone: bool,
    pub active_movement: Option<Movement>,
    pub animation_data: Option<Arc<AnimationData>>,
    pub tile_position: TilePosition,
    pub world_position: Point3<f32>,
    pub scale: f32,
    #[hidden_element]
    details: ResourceState<String>,
    #[hidden_element]
    animation_state: AnimationState,
    #[hidden_element]
    trick_dead: bool,
    #[hidden_element]
    su_hide: bool,
    #[hidden_element]
    su_stoop: bool,
    #[hidden_element]
    active_cast: Option<ActorCast>,
    stopped_moving: bool,
    #[hidden_element]
    fade_state: FadeState,
}

#[derive(Copy, Clone)]
pub(crate) struct ActorCast {
    _skill_id: SkillId,
    ends_at: ClientTick,
    total_ms: u32,
}

#[cfg_attr(feature = "debug", korangar_debug::profile)]
#[allow(clippy::invisible_characters)]
pub(crate) fn get_sprite_path_for_player_job(job_id: JobId) -> &'static str {
    match job_id.0 {
        0 => "초보자",             // NOVICE
        1 => "검사",               // SWORDMAN
        2 => "마법사",             // MAGICIAN
        3 => "궁수",               // ARCHER
        4 => "성직자",             // ACOLYTE
        5 => "상인",               // MERCHANT
        6 => "도둑",               // THIEF
        7 => "기사",               // KNIGHT
        8 => "성투사",             // PRIEST
        9 => "위저드",             // WIZARD
        10 => "제철공",            // BLACKSMITH
        11 => "헌터",              // HUNTER
        12 => "어세신",            // ASSASSIN
        13 => "페코페코_기사",     // KNIGHT2
        14 => "크루세이더",        // CRUSADER
        15 => "몽크",              // MONK
        16 => "세이지",            // SAGE
        17 => "로그",              // ROGUE
        18 => "연금술사",          // ALCHEMIST
        19 => "바드",              // BARD
        20 => "무희",              // DANCER
        23 => "슈퍼노비스",        // SUPERNOVICE
        24 => "건너",              // GUNSLINGER
        25 => "닌자",              // NINJA
        4001 => "초보자",          // NOVICE_H
        4002 => "검사",            // SWORDMAN_H
        4003 => "마법사",          // MAGICIAN_H
        4004 => "궁수",            // ARCHER_H
        4005 => "성직자",          // ACOLYTE_H
        4006 => "상인",            // MERCHANT_H
        4007 => "도둑",            // THIEF_H
        4008 => "로드나이트",      // KNIGHT_H
        4009 => "하이프리",        // PRIEST_H
        4010 => "하이위저드",      // WIZARD_H
        4011 => "화이트스미스",    // BLACKSMITH_H
        4012 => "스나이퍼",        // HUNTER_H
        4013 => "어쌔신크로스",    // ASSASSIN_H
        4014 => "엔대운",          // CHICKEN_H
        4015 => "팔라딘",          // CRUSADER_H
        4016 => "챔피온",          // MONK_H
        4017 => "프로페서",        // SAGE_H
        4018 => "스토커",          // ROGUE_H
        4019 => "크리에이터",      // ALCHEMIST_H
        4020 => "클라운",          // BARD_H
        4021 => "집시",            // DANCER_H
        4023 => "초보자",          // NOVICE_B
        4024 => "검사",            // SWORDMAN_B
        4025 => "마법사",          // MAGICIAN_B
        4026 => "궁수",            // ARCHER_B
        4027 => "성직자",          // ACOLYTE_B
        4028 => "상인",            // MERCHANT_B
        4029 => "도둑",            // THIEF_B
        4030 => "기사",            // KNIGHT_B
        4031 => "성투사",          // PRIEST_B
        4032 => "위저드",          // WIZARD_B
        4033 => "제철공",          // BLACKSMITH_B
        4034 => "헌터",            // HUNTER_B
        4035 => "어세신",          // ASSASSIN_B
        4037 => "크루세이더",      // CRUSADER_B
        4038 => "몽크",            // MONK_B
        4039 => "세이지",          // SAGE_B
        4040 => "로그",            // ROGUE_B
        4041 => "연금술사",        // ALCHEMIST_B
        4042 => "바드",            // BARD_B
        4043 => "무희",            // DANCER_B
        4045 => "슈퍼노비스",      // SUPERNOVICE_B
        4054 => "룬나이트",        // RUNE_KNIGHT
        4055 => "워록",            // WARLOCK
        4056 => "레인져",          // RANGER
        4057 => "아크비숍",        // ARCH_BISHOP
        4058 => "미케닉",          // MECHANIC
        4059 => "길로틴크로스",    // GUILLOTINE_CROSS
        4066 => "가드",            // ROYAL_GUARD
        4067 => "소서러",          // SORCERER
        4068 => "민스트럴",        // MINSTREL
        4069 => "원더러",          // WANDERER
        4070 => "슈라",            // SURA
        4071 => "제네릭",          // GENETIC
        4072 => "쉐도우체이서",    // SHADOW_CHASER
        4060 => "룬나이트",        // RUNE_KNIGHT_H
        4061 => "워록",            // WARLOCK_H
        4062 => "레인져",          // RANGER_H
        4063 => "아크비숍",        // ARCH_BISHOP_H
        4064 => "미케닉",          // MECHANIC_H
        4065 => "길로틴크로스",    // GUILLOTINE_CROSS_H
        4073 => "가드",            // ROYAL_GUARD_H
        4074 => "소서러",          // SORCERER_H
        4075 => "민스트럴",        // MINSTREL_H
        4076 => "원더러",          // WANDERER_H
        4077 => "슈라",            // SURA_H
        4078 => "제네릭",          // GENETIC_H
        4079 => "쉐도우체이서",    // SHADOW_CHASER_H
        4096 => "룬나이트",        // RUNE_KNIGHT_B
        4097 => "워록",            // WARLOCK_B
        4098 => "레인져",          // RANGER_B
        4099 => "아크비숍",        // ARCHBISHOP_B
        4100 => "미케닉",          // MECHANIC_B
        4101 => "길로틴크로스",    // GUILLOTINE_CROSS_B
        4102 => "가드",            // ROYAL_GUARD_B
        4103 => "소서러",          // SORCERER_B
        4104 => "민스트럴",        // MINSTREL_B
        4105 => "원더러",          // WANDERER_B
        4106 => "슈라",            // SURA_B
        4107 => "제네릭",          // GENETIC_B
        4108 => "쉐도우체이서",    // SHADOW_CHASER_B
        4046 => "태권소년",        // TAEKWON
        4047 => "권성",            // STAR
        4049 => "소울링커",        // LINKER
        4190 => "슈퍼노비스",      // SUPERNOVICE2
        4191 => "슈퍼노비스",      // SUPERNOVICE2_B
        4211 => "KAGEROU",         // KAGEROU
        4212 => "OBORO",           // OBORO
        4215 => "REBELLION",       // REBELLION
        4222 => "닌자",            // NINJA_B
        4223 => "KAGEROU",         // KAGEROU_B
        4224 => "OBORO",           // OBORO_B
        4225 => "태권소년",        // TAEKWON_B
        4226 => "권성",            // STAR_B
        4227 => "소울링커",        // LINKER_B
        4228 => "건너",            // GUNSLINGER_B
        4229 => "REBELLION",       // REBELLION_B
        4239 => "성제",            // STAR EMPEROR
        4240 => "소울리퍼",        // SOUL REAPER
        4241 => "성제",            // STAR_EMPEROR_B
        4242 => "소울리퍼",        // SOUL_REAPER_B
        4252 => "DRAGON_KNIGHT",   // DRAGON KNIGHT
        4253 => "MEISTER",         // MEISTER
        4254 => "SHADOW_CROSS",    // SHADOW CROSS
        4255 => "ARCH_MAGE",       // ARCH MAGE
        4256 => "CARDINAL",        // CARDINAL
        4257 => "WINDHAWK",        // WINDHAWK
        4258 => "IMPERIAL_GUARD",  // IMPERIAL GUARD
        4259 => "BIOLO",           // BIOLO
        4260 => "ABYSS_CHASER",    // ABYSS CHASER
        4261 => "ELEMETAL_MASTER", // ELEMENTAL MASTER
        4262 => "INQUISITOR",      // INQUISITOR
        4263 => "TROUBADOUR",      // TROUBADOUR
        4264 => "TROUVERE",        // TROUVERE
        4302 => "SKY_EMPEROR",     // SKY EMPEROR
        4303 => "SOUL_ASCETIC",    // SOUL ASCETIC
        4304 => "SHINKIRO",        // SHINKIRO
        4305 => "SHIRANUI",        // SHIRANUI
        4306 => "NIGHT_WATCH",     // NIGHT WATCH
        4307 => "HYPER_NOVICE",    // HYPER NOVICE
        _ => "초보자",             // NOVICE,
    }
}

pub(crate) fn get_entity_part_files(
    library: &Library,
    entity_type: EntityType,
    job_id: JobId,
    sex: Sex,
    head: Option<usize>,
) -> Vec<String> {
    let sex_sprite_path = match sex == Sex::Female {
        true => "여",
        false => "남",
    };

    fn player_body_path(sex_sprite_path: &str, job_id: JobId) -> String {
        format!(
            "인간족\\몸통\\{}\\{}_{}",
            sex_sprite_path,
            get_sprite_path_for_player_job(job_id),
            sex_sprite_path
        )
    }

    fn player_head_path(sex_sprite_path: &str, head_id: usize) -> String {
        format!("인간족\\머리통\\{}\\{}_{}", sex_sprite_path, head_id, sex_sprite_path)
    }

    let head_id = match (sex, head) {
        (Sex::Male, Some(head)) if (0..MALE_HAIR_LOOKUP.len()).contains(&head) => MALE_HAIR_LOOKUP[head],
        (Sex::Male, Some(head)) => head,
        (Sex::Female, Some(head)) if (0..FEMALE_HAIR_LOOKUP.len()).contains(&head) => FEMALE_HAIR_LOOKUP[head],
        (Sex::Female, Some(head)) => head,
        _ => 1,
    };

    match entity_type {
        EntityType::Player => vec![
            player_body_path(sex_sprite_path, job_id),
            player_head_path(sex_sprite_path, head_id),
        ],
        EntityType::Npc => vec![format!("npc\\{}", library.get::<JobIdentity>(job_id).to_string())],
        EntityType::Monster => vec![format!("몬스터\\{}", library.get::<JobIdentity>(job_id).to_string())],
        // Warp and hidden NPCs are server-side trigger entities, not visible actors.
        // Trying to load their job identity, such as WARPNPC, renders the missing
        // sprite fallback as a misleading shadow marker.
        EntityType::Warp | EntityType::Hidden => Vec::new(),
    }
}

/// Upper bound of classic weapon *class* view IDs (roBrowser `WeaponType.MAX`).
/// Hercules `PACKETVER >= 4` sends raw item IDs (≥ this) as LOOK_WEAPON /
/// LOOK_SHIELD for equipped weapons; class views stay in
/// `0..WEAPON_VIEW_CLASS_MAX`.
pub const WEAPON_VIEW_CLASS_MAX: u32 = 31;

/// Weapon appearance class → classic weapon sprite name. Verified against the
/// configured official GRFs with `weapon-sprite-audit`: two-handed swords,
/// spears, and axes ship their own `양손*` sprites, classic rods/staves ship
/// no weapon sprite in these archives, and the
/// Assassin dual-wield combinations (25..=30) have dedicated pair sprites.
pub(crate) fn weapon_resource_suffix(weapon: u32) -> Option<&'static str> {
    match native_real_weapon_id(weapon) {
        1 => Some("단검"),
        2 => Some("검"),
        3 => Some("양손검"),
        4 => Some("창"),
        5 => Some("양손창"),
        6 => Some("도끼"),
        7 => Some("양손도끼"),
        8 | 9 => Some("클럽"),
        11 => Some("활"),
        12 => Some("너클"),
        13 => Some("악기"),
        14 => Some("채찍"),
        15 => Some("책"),
        16 => Some("카타르_카타르"),
        17 => Some("권총"),
        18 => Some("라이플"),
        19 => Some("기관총"),
        20 => Some("샷건"),
        22 => Some("수리검"),
        25 => Some("단검_단검"),
        26 => Some("검_검"),
        27 => Some("도끼_도끼"),
        28 => Some("단검_검"),
        29 => Some("단검_도끼"),
        30 => Some("검_도끼"),
        // 10/23 (rods) and 21 (grenade launcher) have no classic weapon
        // sprite in the archives.
        _ => None,
    }
}

/// Map a raw item ID to the classic weapon class view used by attack selection
/// and class sprite names. Mirrors roBrowser `DB.getWeaponViewID` ranges plus
/// the special sub-ranges Gravity nested inside broader ID bands.
///
/// Appearance values already in `0..WEAPON_VIEW_CLASS_MAX` should go through
/// [`weapon_view_from_appearance`] instead so expansion IDs hit
/// `native_real_weapon_id`.
pub fn weapon_view_from_item_id(item_id: u32) -> u32 {
    // Nested exceptions inside broader ranges (roBrowser / Gravity tables).
    if (1116..=1118).contains(&item_id) {
        return 3; // two-hand sword
    }
    if (1314..=1315).contains(&item_id) {
        return 7; // two-hand axe
    }
    if (1410..=1412).contains(&item_id) {
        return 5; // two-hand spear
    }
    if (1472..=1473).contains(&item_id) {
        return 10; // rod
    }
    if item_id == 1599 {
        return 8; // mace
    }
    if matches!(item_id, 13157 | 13158 | 13159 | 13172 | 13177) {
        return 19; // gatling
    }
    if matches!(item_id, 13154 | 13155 | 13156 | 13167 | 13168 | 13169 | 13173 | 13178) {
        return 20; // shotgun
    }
    if matches!(item_id, 13160 | 13161 | 13162 | 13174 | 13179) {
        return 21; // grenade
    }

    match item_id {
        1100..=1149 | 13400..=13499 => 2,  // 1H sword
        1150..=1199 | 21000..=21999 => 3,  // 2H sword
        1200..=1249 | 13000..=13099 => 1,  // dagger
        1250..=1299 => 16,                 // katar
        1300..=1349 => 6,                  // 1H axe
        1350..=1399 => 7,                  // 2H axe
        1400..=1449 => 4,                  // 1H spear
        1450..=1499 => 5,                  // 2H spear
        1500..=1549 => 8,                  // mace
        1550..=1599 => 15,                 // book
        1600..=1699 => 10,                 // rod
        1700..=1749 | 18100..=18499 => 11, // bow
        1800..=1849 => 12,                 // knuckle
        1900..=1949 => 13,                 // instrument
        1950..=1999 => 14,                 // whip
        2000..=2049 | 20000..=20999 => 23, // 2H rod
        13100..=13149 => 17,               // handgun
        13150..=13199 => 18,               // rifle (exceptions above)
        13300..=13399 => 22,               // shuriken
        _ => 0,
    }
}

/// Normalize a LOOK_WEAPON / inventory appearance value to a class view.
///
/// - Class views (`0..31`) pass through `GetRealWeaponId` expansion mapping.
/// - Item IDs (`≥ 31`) resolve via [`weapon_view_from_item_id`].
pub fn weapon_view_from_appearance(appearance: u32) -> u32 {
    if appearance < WEAPON_VIEW_CLASS_MAX {
        native_real_weapon_id(appearance)
    } else {
        weapon_view_from_item_id(appearance)
    }
}

/// Assassin dual-wield combination of two single-hand class views → views
/// `25..=30`. Returns `None` when the pair is not a dual-wieldable combo.
pub fn combine_dual_wield_view(right_view: u32, left_view: u32) -> Option<u32> {
    match (right_view, left_view) {
        (1, 1) => Some(25),          // dagger + dagger
        (2, 2) => Some(26),          // sword + sword
        (6, 6) => Some(27),          // axe + axe
        (1, 2) | (2, 1) => Some(28), // dagger + sword
        (1, 6) | (6, 1) => Some(29), // dagger + axe
        (2, 6) | (6, 2) => Some(30), // sword + axe
        _ => None,
    }
}

/// Whether a LOOK_SHIELD / left-hand appearance is an off-hand *weapon*
/// (dual-wield) rather than a shield or empty hand.
///
/// Hercules `get_weapon_view` (PACKETVER ≥ 4) puts the left-hand item nameid
/// into the shield channel for dual-wield. Classic shield looks on that
/// channel are either item IDs in the shield band (`2101..=2200`) or class
/// views `1..=4` (Guard/Buckler/Shield/Mirror). Dual-wield hands use weapon
/// item IDs (≥ `WEAPON_VIEW_CLASS_MAX`), not class views 1..=4.
pub fn appearance_is_offhand_weapon(appearance: u32) -> bool {
    if appearance == 0 {
        return false;
    }
    if appearance < WEAPON_VIEW_CLASS_MAX {
        // Class views 1..=4 are classic shields on this channel. Pre-combined
        // dual class views (25..=30) are weapons if they ever appear here.
        return matches!(appearance, 25..=30);
    }
    // Classic shield items.
    if (2101..=2200).contains(&appearance) {
        return false;
    }
    // Anything that maps to a non-fist weapon class is a weapon.
    weapon_view_from_item_id(appearance) != 0
}

/// Weapon class used for attack-action selection and class sprite fallback.
/// Combines Assassin left/right hands into views `25..=30` when both are
/// dual-wield weapons.
pub fn effective_weapon_view(weapon_appearance: u32, left_appearance: u32) -> u32 {
    let right = weapon_view_from_appearance(weapon_appearance);
    if !appearance_is_offhand_weapon(left_appearance) {
        return right;
    }
    let left = weapon_view_from_appearance(left_appearance);
    combine_dual_wield_view(right, left).unwrap_or(right)
}

/// True when the path is a `_검광` sword-trail layer (not the base weapon).
#[allow(dead_code)]
pub fn is_weapon_trail_path(path: &str) -> bool {
    let normalized = path.replace('/', "\\");
    normalized.ends_with("_검광")
}

/// The folder under `인간족\` holding a job's weapon sprites. Usually the
/// body sprite folder, but the audit showed three exceptions: the Priest
/// family's weapon files live under `프리스트`, Royal Guard's under `로얄가드`,
/// and the transcendent second classes (and Shadow Chaser) ship no weapon
/// sprites of their own. Korangar currently resolves them to base-class files.
pub(crate) fn get_weapon_sprite_folder(job_id: JobId) -> &'static str {
    match job_id.0 {
        8 | 4009 | 4031 => "프리스트",
        4008 => "기사",
        4010 => "위저드",
        4011 => "제철공",
        4012 => "헌터",
        4013 => "어세신",
        4014 => "페코페코_기사",
        4015 => "크루세이더",
        4016 => "몽크",
        4017 => "세이지",
        4018 => "로그",
        4019 => "연금술사",
        4020 => "바드",
        4021 => "무희",
        4066 | 4073 | 4102 => "로얄가드",
        4072 | 4079 | 4108 => "로그",
        _ => get_sprite_path_for_player_job(job_id),
    }
}

fn sex_path_token(sex: Sex) -> &'static str {
    match sex == Sex::Female {
        true => "여",
        false => "남",
    }
}

fn sprite_part_exists(game_file_loader: &GameFileLoader, part_file: &str) -> bool {
    // `file_exists` does not normalize case the way `get` does.
    game_file_loader.file_exists(&format!("data\\sprite\\{part_file}.spr").to_lowercase())
}

/// Class + optional per-item path candidates for one hand's weapon appearance.
fn weapon_part_candidates(folder: &str, sex: &str, appearance: u32, prefer_dual_class: Option<u32>) -> Vec<String> {
    let mut out = Vec::new();
    // Phase D: exact per-item path first (`기사_남_1530`), never a placeholder.
    if appearance >= WEAPON_VIEW_CLASS_MAX {
        out.push(format!("인간족\\{folder}\\{folder}_{sex}_{appearance}"));
    }
    // Dual-wield class pair (`단검_단검` …) when both hands are weapons.
    if let Some(dual_view) = prefer_dual_class
        && let Some(suffix) = weapon_resource_suffix(dual_view)
    {
        out.push(format!("인간족\\{folder}\\{folder}_{sex}_{suffix}"));
    }
    // Single-hand class sprite from the normalized view.
    let view = weapon_view_from_appearance(appearance);
    if prefer_dual_class != Some(view)
        && let Some(suffix) = weapon_resource_suffix(view)
    {
        out.push(format!("인간족\\{folder}\\{folder}_{sex}_{suffix}"));
    }
    out
}

fn first_existing_part(game_file_loader: &GameFileLoader, candidates: &[String]) -> Option<String> {
    candidates.iter().find(|part| sprite_part_exists(game_file_loader, part)).cloned()
}

/// Item sprite path for an ammunition item resource name, e.g. `철화살` (Iron
/// Arrow) → `아이템\철화살.spr`. Classic RO draws the flying projectile with
/// the ammo *item*'s own sprite, which is why the per-type variants read
/// differently in flight.
pub fn ammunition_projectile_sprite_path(resource_name: &str) -> String {
    format!("아이템\\{resource_name}.spr")
}

/// The generic arrow resource. `iteminfo` hands this back for most ammunition,
/// including arrows that ship a distinct sprite of their own, which is why
/// every elemental arrow otherwise flies looking identical.
pub const GENERIC_ARROW_RESOURCE: &str = "화살";

/// Distinct sprite resource for the **elemental** arrows, used only where
/// `iteminfo` falls back to [`GENERIC_ARROW_RESOURCE`].
///
/// This is a deliberate, small divergence from the original client: it drives
/// the flying projectile off the arrow's *element* rather than off whatever
/// `iteminfo` happens to name, so a Fire Arrow reads as a fire arrow in flight.
/// The client's own mapping still wins wherever it names a specific sprite —
/// see `spawn_ranged_attack_projectile`.
///
/// Ids and elements are from Hercules `db/re/item_db.conf`, taken from each
/// item's `bonus bAtkEle,Ele_*` script rather than from its name. Every sprite
/// below was confirmed present in `data.grf` with `tools/grf_list.py`; the
/// item↔sprite pairing is a translation of the Korean resource names and is the
/// part to re-check if one looks wrong in flight.
///
/// Frozen Arrow (1759, Water), Arrow of Counter Evil (1766, Holy) and Holy
/// Arrow (1772, Holy) are elemental but ship **no** distinct sprite, so they
/// are deliberately absent and keep whatever `iteminfo` gives them.
pub fn elemental_ammunition_resource(item_id: ItemId) -> Option<&'static str> {
    match item_id.0 {
        1751 => Some("은화살"),       // Silver Arrow — Holy
        1752 => Some("불화살"),       // Fire Arrow — Fire
        1754 => Some("수정화살"),     // Crystal Arrow — Water
        1755 => Some("바람의화살"),   // Arrow of Wind — Wind
        1756 => Some("돌화살"),       // Stone Arrow — Earth
        1757 => Some("무형의화살"),   // Immaterial Arrow — Ghost
        1762 => Some("녹슨화살"),     // Rusty Arrow — Poison
        1763 => Some("독화살"),       // Poison Arrow — Poison
        1767 => Some("그림자의화살"), // Arrow of Shadow — Dark
        _ => None,
    }
}

/// Attack element of an ammunition item.
///
/// Only the elements that ammunition actually carries; there is no `Neutral`
/// because a neutral arrow has nothing to tint and must not glow at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmunitionElement {
    Fire,
    Water,
    Wind,
    Earth,
    Holy,
    Dark,
    Poison,
    Ghost,
}

impl AmmunitionElement {
    /// Colour of the in-flight glow. Drawn additively and used for the point
    /// light, so these are light colours rather than surface colours: bright
    /// and unsaturated enough to stay visible against both dark maps and
    /// daylight.
    pub const fn glow_color(self) -> Color {
        match self {
            Self::Fire => Color::rgb_u8(255, 120, 40),
            Self::Water => Color::rgb_u8(90, 175, 255),
            Self::Wind => Color::rgb_u8(120, 255, 175),
            Self::Earth => Color::rgb_u8(215, 160, 75),
            Self::Holy => Color::rgb_u8(255, 240, 170),
            Self::Dark => Color::rgb_u8(150, 85, 220),
            Self::Poison => Color::rgb_u8(165, 220, 60),
            Self::Ghost => Color::rgb_u8(200, 220, 255),
        }
    }
}

/// Attack element of an ammunition item, or `None` for neutral ammo.
///
/// Taken from each item's `bonus bAtkEle,Ele_*` script in Hercules
/// `db/re/item_db.conf` — mechanical, not name matching, so Rusty Arrow
/// (Poison) and Silver Arrow (Holy) land correctly rather than by guesswork.
///
/// Deliberately wider than [`elemental_ammunition_resource`]: Frozen Arrow
/// (1759), Arrow of Counter Evil (1766) and Holy Arrow (1772) are elemental but
/// ship no distinct sprite, so a glow is the *only* way they can read as
/// elemental in flight. Keep the two lists in step when either changes.
pub fn ammunition_element(item_id: ItemId) -> Option<AmmunitionElement> {
    match item_id.0 {
        1751 | 1766 | 1772 => Some(AmmunitionElement::Holy), // Silver / Counter Evil / Holy Arrow
        1752 => Some(AmmunitionElement::Fire),               // Fire Arrow
        1754 | 1759 => Some(AmmunitionElement::Water),       // Crystal / Frozen Arrow
        1755 => Some(AmmunitionElement::Wind),               // Arrow of Wind
        1756 => Some(AmmunitionElement::Earth),              // Stone Arrow
        1757 => Some(AmmunitionElement::Ghost),              // Immaterial Arrow
        1762 | 1763 => Some(AmmunitionElement::Poison),      // Rusty / Poison Arrow
        1767 => Some(AmmunitionElement::Dark),               // Arrow of Shadow
        _ => None,
    }
}

/// Fallback projectile sprite for a weapon class view, used when the shooter's
/// real ammunition cannot be resolved (a remote entity whose inventory we never
/// see, or an item missing from `iteminfo`). `None` for melee weapons.
///
/// Ranged views: bow (11), gunslinger firearms (17-21), huuma shuriken (22).
pub fn ranged_attack_projectile_sprite(view: u32) -> Option<&'static str> {
    match native_real_weapon_id(view) {
        11 => Some("아이템\\화살.spr"),        // bow → Arrow
        17..=21 => Some("아이템\\탄약통.spr"), // firearms → Bullet
        22 => Some("아이템\\수리검.spr"),      // huuma shuriken → Shuriken
        _ => None,
    }
}

/// Canonical ammunition item for a ranged weapon class view, or `None` for
/// melee. Ids from Hercules `db/re/item_db.conf`: Arrow 1750, Bullet 13200,
/// Shuriken 13250.
///
/// The server never reports what ammunition another character has loaded, so
/// this stands in for everyone but the local player, whose equipped ammo is
/// known exactly and takes priority.
pub fn ranged_attack_default_ammunition(view: u32) -> Option<ItemId> {
    match native_real_weapon_id(view) {
        11 => Some(ItemId(1750)),
        17..=21 => Some(ItemId(13200)),
        22 => Some(ItemId(13250)),
        _ => None,
    }
}

/// Weapon class views that receive a `_검광` trail layer in Ragexe
/// `2019-06-05f` (function around `0x00976590`, switch table `0x00976EC0`).
///
/// Views **with** trail: 1–7 (dagger/sword/2hs/spear/2hspear/axe/2haxe),
/// 16–18 (katar/handgun/rifle), 25–30 (dual-wield pairs).
/// Views **without**: mace/rod/bow/knuckle/instrument/whip/book/…
///
/// Trail path builder is `0x007C4B30` with format `\%s_%s%s%s.%s` and suffix
/// table `_검광` / `_발광` (`0x00B1C7C4` / `0x00B1C7CC`). Korangar loads
/// `_검광` when present; `_발광` is not wired yet.
fn native_weapon_view_has_geom_trail(view: u32) -> bool {
    matches!(view, 1..=7 | 16..=18 | 25..=30)
}

/// Append `_검광` trail layer when native would for this class view and the
/// archives ship the SPR. Always probe for per-item bases (`…_1530_검광`)
/// because those files exist in the GRF even when the class view is trail-less.
fn push_weapon_trail_part(
    files: &mut Vec<String>,
    game_file_loader: &GameFileLoader,
    weapon_part: &str,
    class_view: u32,
    is_per_item_base: bool,
) {
    if !is_per_item_base && !native_weapon_view_has_geom_trail(class_view) {
        return;
    }
    let trail = format!("{weapon_part}_검광");
    if sprite_part_exists(game_file_loader, &trail) {
        files.push(trail);
    }
}

/// Append the equipped weapon's sprite layer(s) when the archives actually ship
/// them for this job/sex/weapon combination. Requesting a file that does not
/// exist would render the placeholder fallback sprite on top of the actor.
///
/// Phase D order:
/// 1. right-hand per-item path (item ID appearance)
/// 2. dual-wield class pair when left hand is also a weapon
/// 3. right-hand class sprite
/// 4. matching `_검광` trail for the chosen base
/// 5. off-hand weapon as a second layer when dual class was not used
fn push_weapon_part_file(files: &mut Vec<String>, common: &Common, game_file_loader: &GameFileLoader) {
    if common.entity_type != EntityType::Player {
        return;
    }

    let sex = sex_path_token(common.sex);
    let folder = get_weapon_sprite_folder(common.job_id);
    let right = common.weapon;
    let left = common.shield;
    let dual_view = if appearance_is_offhand_weapon(left) {
        combine_dual_wield_view(weapon_view_from_appearance(right), weapon_view_from_appearance(left))
    } else {
        None
    };

    let right_candidates = weapon_part_candidates(folder, sex, right, dual_view);
    let Some(right_part) = first_existing_part(game_file_loader, &right_candidates) else {
        // Bare fist / rod / missing combination — no weapon layer.
        return;
    };

    let used_dual_class = dual_view
        .and_then(weapon_resource_suffix)
        .is_some_and(|suffix| right_part.ends_with(&format!("_{suffix}")));
    let right_view = dual_view.unwrap_or_else(|| weapon_view_from_appearance(right));
    let right_is_per_item = right >= WEAPON_VIEW_CLASS_MAX && right_part.ends_with(&format!("_{right}"));

    files.push(right_part.clone());
    push_weapon_trail_part(files, game_file_loader, &right_part, right_view, right_is_per_item);

    // Second weapon layer for dual-wield when we did not already load the
    // combined pair sprite (per-item right + class/item left).
    if appearance_is_offhand_weapon(left) && !used_dual_class {
        let left_candidates = weapon_part_candidates(folder, sex, left, None);
        if let Some(left_part) = first_existing_part(game_file_loader, &left_candidates)
            && left_part != right_part
            && !files.iter().any(|p| p == &left_part)
        {
            let left_view = weapon_view_from_appearance(left);
            let left_is_per_item = left >= WEAPON_VIEW_CLASS_MAX && left_part.ends_with(&format!("_{left}"));
            files.push(left_part.clone());
            push_weapon_trail_part(files, game_file_loader, &left_part, left_view, left_is_per_item);
        }
    }
}

/// Ragexe `0x7C46C0` (2019-06-05f, SHA-256 `61663a6f…`) builds shield paths.
///
/// Job name table entries are compound (`기사\\기사`), so:
/// - class: `방패\%s_%s%s.%s` → `방패\기사\기사_남_가드.spr`
/// - item (view ≥ 5 and special job check at `0x9A2430`):
///   `방패\%s_%s_%d_방패.%s` → `방패\기사\기사_남_{view}_방패.spr`
///
/// Class suffixes (with leading `_` in the binary name table at `0x72796C`):
/// `_가드`, `_쉴드`, `_미러쉴드`, `_버클러`. Item ViewSprite 1..4 maps to those
/// names in classic DB order (Guard/Buckler/Shield/Mirror), not table storage
/// order.
///
/// Job ids `> 0xF6E` (3950) are remapped `job - 3950` for the name table index.
fn native_shield_class_suffix(shield_view: u32) -> Option<&'static str> {
    // Matches ViewSprite in item_db + GRF class files, not the binary string
    // table order (which stores 가드/쉴드/미러쉴드/버클러).
    match shield_view {
        1 => Some("가드"),
        2 => Some("버클러"),
        3 => Some("쉴드"),
        4 => Some("미러쉴드"),
        _ => None,
    }
}

/// Job folder tokens for shield sprites (same sprite-job names as body).
fn shield_sprite_folders(job_id: JobId) -> Vec<&'static str> {
    let body = get_sprite_path_for_player_job(job_id);
    let weapon_alias = get_weapon_sprite_folder(job_id);
    if body == weapon_alias {
        vec![body]
    } else {
        vec![body, weapon_alias]
    }
}

fn shield_part_candidates(job_folder: &str, sex: &str, shield_view: u32) -> Vec<String> {
    let mut out = Vec::new();
    // Native tries the item-id form first when view ≥ 5 (and the job is
    // allowed by 0x9A2430). We always probe it for view ≥ 5; missing SPR is
    // harmless.
    if shield_view >= 5 {
        out.push(format!("방패\\{job_folder}\\{job_folder}_{sex}_{shield_view}_방패"));
    }
    if let Some(class_name) = native_shield_class_suffix(shield_view) {
        out.push(format!("방패\\{job_folder}\\{job_folder}_{sex}_{class_name}"));
    } else if shield_view > 0 {
        // Non-classic view that still has a class-style file under some packs.
        out.push(format!("방패\\{job_folder}\\{job_folder}_{sex}_{shield_view}_방패"));
        out.push(format!("방패\\{job_folder}\\{job_folder}_{sex}_{shield_view}"));
    }
    out
}

/// Append the equipped shield layer when the archives ship a matching SPR.
/// Paths match Ragexe `0x7C46C0` (not under `인간족\`).
///
/// Dual-wield left-hand weapons ride the LOOK_SHIELD channel as item IDs and
/// are rendered as weapon layers by [`push_weapon_part_file`] — skip them here.
fn push_shield_part_file(files: &mut Vec<String>, common: &Common, game_file_loader: &GameFileLoader) {
    if common.entity_type != EntityType::Player || common.shield == 0 {
        return;
    }
    if appearance_is_offhand_weapon(common.shield) {
        return;
    }

    let sex = match common.sex == Sex::Female {
        true => "여",
        false => "남",
    };

    for folder in shield_sprite_folders(common.job_id) {
        for part_file in shield_part_candidates(folder, sex, common.shield) {
            if game_file_loader.file_exists(&format!("data\\sprite\\{part_file}.spr").to_lowercase()) {
                files.push(part_file);
                return;
            }
        }
    }
}

/// Headgear sprite path for one view id, if the archive has it.
///
/// Hats live beside the head under
/// `data\\sprite\\악세사리\\{sex}\\{sex}_{name}` and are authored in the head's
/// own coordinate frame: measured against the archive, a headgear ACT carries
/// the *same* body-relative attach point the head does for the same facing.
/// That makes a hat a sibling of the head, not its child: the ordinary
/// `-child + body` parenting every secondary layer already gets is what lands
/// it on the head.
fn headgear_part_file(library: &Library, game_file_loader: &GameFileLoader, sex: Sex, view_id: u16) -> Option<String> {
    if view_id == 0 {
        return None;
    }

    let name = library.try_get::<AccessoryName>(AccessoryNameKey {
        view_id,
        female: sex == Sex::Female,
    })?;
    let name = name.as_str();
    if name.is_empty() {
        return None;
    }

    let part_file = headgear_sprite_path(sex_path_token(sex), name);
    sprite_part_exists(game_file_loader, &part_file).then_some(part_file)
}

/// Join the sex token and the `accname.lub` sprite name.
///
/// The table stores the name with its own leading underscore (`_고글`), so the
/// separator must not be added twice — `남__고글` resolves to nothing, and
/// silently: a hat whose sprite is not found looks exactly like no hat.
/// Verified against the archive with `playtest-sprite-audit hat-lookup`:
/// 1706 of 1984 mapped ids resolve, the remainder being regional headgear
/// these GRFs do not ship.
fn headgear_sprite_path(sex_token: &str, name: &str) -> String {
    match name.starts_with('_') {
        true => format!("악세사리\\{sex_token}\\{sex_token}{name}"),
        false => format!("악세사리\\{sex_token}\\{sex_token}_{name}"),
    }
}

/// Append the three headgear slots in classic stacking order.
///
/// Order is bottom → middle → top, and it must stay directly after the head so
/// the compositor's layer order paints hats over the head but under the weapon.
/// Takes raw view ids so the character-select cards, which have the ids from
/// the character list but no [`Common`], resolve hats by the same rule as a
/// spawned entity.
pub(crate) fn push_headgear_part_files_for_views(
    files: &mut Vec<String>,
    library: &Library,
    game_file_loader: &GameFileLoader,
    sex: Sex,
    view_ids: [u16; 3],
) {
    for view_id in view_ids {
        if let Some(part_file) = headgear_part_file(library, game_file_loader, sex, view_id) {
            files.push(part_file);
        }
    }
}

fn push_headgear_part_files(files: &mut Vec<String>, common: &Common, library: &Library, game_file_loader: &GameFileLoader) {
    if common.entity_type != EntityType::Player {
        return;
    }

    push_headgear_part_files_for_views(files, library, game_file_loader, common.sex, [
        common.accessory,
        common.accessory3,
        common.accessory2,
    ]);
}

fn weapon_sound(weapon_view: u32) -> &'static str {
    match native_real_weapon_id(weapon_view) {
        1 => "attack_short_sword.wav",
        2 => "attack_sword.wav",
        3 => "attack_twohand_sword.wav",
        4 | 5 => "attack_spear.wav",
        6 | 7 => "attack_axe.wav",
        8 | 9 => "attack_mace.wav",
        10 | 23 => "attack_rod.wav",
        11 => "attack_bow1.wav",
        14 => "attack_whip.wav",
        15 => "attack_book.wav",
        16 => "attack_katar.wav",
        22 => "attack_sword.wav",
        25 | 28 | 29 => "attack_short_sword.wav",
        26 | 30 => "attack_sword.wav",
        27 => "attack_axe.wav",
        _ => "attack_fist.wav",
    }
}

/// Hard-coded projectile travel additions in the reference client's basic
/// and skill damage controllers. These are added after the source ACT event
/// offset and apply only to the listed actor/job IDs.
fn native_impact_extra_delay_ms(job_id: JobId, skill_id: Option<SkillId>) -> u32 {
    match (skill_id, job_id.0) {
        (Some(_), 1016 | 1420) => 192,
        (Some(_), _) => 0,
        (None, 1016 | 1420) => 192,
        (None, 1285 | 1830) => 912,
        (None, 1286 | 1287 | 1829) => 408,
        (None, _) => 0,
    }
}

impl Common {
    /// Alpha applied to an entity concealed by Hiding / Cloaking / Chase Walk.
    ///
    /// The original client draws *your own* concealed character translucent
    /// rather than fully invisible, so you can still see where you are.
    /// Entities the server does not want you to see are never sent in the
    /// first place, so anything we are asked to draw with a conceal flag is
    /// one we are allowed to see.
    const CONCEALED_ALPHA: f32 = 0.3;

    pub fn new(
        library: &Library,
        entity_data: &EntityData,
        tile_position: TilePosition,
        world_position: Point3<f32>,
        client_tick: ClientTick,
    ) -> Self {
        let entity_id = entity_data.entity_id;
        let job_id = entity_data.job_id;
        let head_direction = entity_data.head_direction;
        let direction = entity_data.position.direction;

        let movement_speed = entity_data.movement_speed as usize;
        let health_points = entity_data.health_points as usize;
        let maximum_health_points = entity_data.maximum_health_points as usize;
        let sex = entity_data.sex;
        let weapon = entity_data.weapon;
        let shield = entity_data.shield;

        let active_movement = None;
        let entity_type = job_id.into();

        let details = ResourceState::Unavailable;
        let mut animation_state = AnimationState::new(entity_type, client_tick);
        match entity_data.state {
            1 => animation_state.dead(entity_type, client_tick),
            2 => animation_state.sit(entity_type, client_tick),
            // Armed players (or PK mode) spawn in the ReadyFight stance so the
            // weapon renders; its Idle frames are the blank unarmed stand.
            _ if entity_data.is_pk_mode_on || weapon != 0 => animation_state.idle(entity_type, true, client_tick),
            _ => {}
        }
        animation_state.set_status_paused(status_freezes_animation(entity_data.body_state), client_tick);
        let scale = match library.get::<IsBabyJob>(job_id) {
            IsBabyJob(true) => BABY_JOB_SCALE,
            IsBabyJob(false) => 1.0,
        };

        Self {
            tile_position,
            world_position,
            entity_id,
            job_id,
            spirit_spheres: 0,
            direction,
            head_direction,
            sex,
            head: entity_data.head as usize,
            weapon,
            shield,
            accessory: entity_data.accessory,
            accessory2: entity_data.accessory2,
            accessory3: entity_data.accessory3,
            head_palette: entity_data.head_palette,
            body_palette: entity_data.body_palette,
            robe: entity_data.robe,
            body: entity_data.body,
            active_movement,
            entity_type,
            option: entity_data.option,
            body_state: entity_data.body_state,
            health_state: entity_data.health_state,
            is_pk_mode_on: entity_data.is_pk_mode_on,
            in_safe_zone: false,
            movement_speed,
            health_points,
            maximum_health_points,
            animation_data: None,
            details,
            animation_state,
            trick_dead: false,
            su_hide: false,
            su_stoop: false,
            active_cast: None,
            stopped_moving: false,
            fade_state: FadeState::new(FADE_IN_DURATION_MS, client_tick),
            scale,
        }
    }

    pub fn get_entity_part_files(&self, library: &Library, game_file_loader: &GameFileLoader) -> Vec<String> {
        let mut files = get_entity_part_files(library, self.entity_type, self.job_id, self.sex, Some(self.head));
        push_headgear_part_files(&mut files, self, library, game_file_loader);
        push_weapon_part_file(&mut files, self, game_file_loader);
        push_shield_part_file(&mut files, self, game_file_loader);
        files
    }

    pub fn is_dead(&self) -> bool {
        self.animation_state.is_dead()
    }

    pub fn is_death_animation_over(&self) -> bool {
        match self.animation_data.as_ref() {
            Some(animation_data) => self.is_dead() && animation_data.is_animation_over(&self.animation_state),
            None => false,
        }
    }

    pub fn is_fading(&self) -> bool {
        self.fade_state.is_fading()
    }

    pub fn update(&mut self, audio_engine: &AudioEngine<GameFileLoader>, map: &Map, camera: &dyn Camera, client_tick: ClientTick) {
        self.update_movement(map, client_tick);
        if self.active_cast.is_some_and(|cast| client_tick.0 >= cast.ends_at.0) {
            self.active_cast = None;
        }
        self.animation_state.update(client_tick);

        if self.fade_state.is_fading() && self.fade_state.is_done_fading_in(client_tick) {
            self.fade_state = FadeState::Opaque;
        }

        if let Some(animation_data) = self.animation_data.as_ref() {
            // Deliver crossed frame events before any completion transition:
            // the transition swaps the playback identity, which would discard
            // crossings on the final motions of the finished action.
            for event in animation_data.take_crossed_events(&mut self.animation_state, camera, self.direction) {
                match event {
                    ActionEvent::Sound { key } => {
                        audio_engine.play_spatial_sound_effect(key, self.world_position, SPATIAL_SOUND_RANGE);
                    }
                    ActionEvent::Attack => {
                        let view = effective_weapon_view(self.weapon, self.shield);
                        let key = audio_engine.load(weapon_sound(view));
                        audio_engine.play_spatial_sound_effect(key, self.world_position, SPATIAL_SOUND_RANGE);
                    }
                    ActionEvent::Unknown => { /* Nothing to do */ }
                }
            }

            if animation_data.is_animation_over(&self.animation_state) {
                self.animation_state
                    .apply_completion_transition(self.entity_type, self.wants_ready_fight_stance(), client_tick);
            }
        }
    }

    fn update_movement(&mut self, map: &Map, client_tick: ClientTick) {
        self.stopped_moving = false;

        if let Some(active_movement) = self.active_movement.take() {
            let last_step = active_movement.steps.last().unwrap();

            if client_tick.0 > last_step.arrival_timestamp {
                self.set_position(map, last_step.arrival_position, client_tick);
                self.stopped_moving = true;
            } else {
                let mut last_step_index = 0;
                while active_movement.steps[last_step_index + 1].arrival_timestamp < client_tick.0 {
                    last_step_index += 1;
                }

                let last_step = active_movement.steps[last_step_index];
                let next_step = active_movement.steps[last_step_index + 1];

                self.tile_position = next_step.arrival_position;

                let last_step_position = Vector2::new(last_step.arrival_position.x as isize, last_step.arrival_position.y as isize);
                let next_step_position = Vector2::new(next_step.arrival_position.x as isize, next_step.arrival_position.y as isize);

                let array = last_step_position - next_step_position;
                let array: &[isize; 2] = array.as_ref();
                self.direction = (*array).try_into().unwrap();

                let Some(last_step_position) = map.get_world_position(last_step.arrival_position) else {
                    self.active_movement = active_movement.into();
                    return;
                };
                let Some(next_step_position) = map.get_world_position(next_step.arrival_position) else {
                    self.active_movement = active_movement.into();
                    return;
                };

                let clamped_tick = u32::max(last_step.arrival_timestamp, client_tick.0);
                let total = next_step.arrival_timestamp - last_step.arrival_timestamp;
                let offset = clamped_tick - last_step.arrival_timestamp;

                let movement_elapsed = (1.0 / total as f32) * offset as f32;
                let position = last_step_position.to_vec().lerp(next_step_position.to_vec(), movement_elapsed);

                self.world_position = Point3::from_vec(position);
                self.active_movement = active_movement.into();
            }
        }
    }

    fn set_position(&mut self, map: &Map, position: TilePosition, client_tick: ClientTick) {
        let Some(world_position) = map.get_world_position(position) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!("[{}] entity position is out of map bounds", "error".red());
            return;
        };

        self.tile_position = position;
        self.world_position = world_position;
        self.active_movement = None;
        if !self.action_request_locked() {
            self.animation_state
                .idle(self.entity_type, self.wants_ready_fight_stance(), client_tick);
        }
    }

    pub fn move_from_to(
        &mut self,
        map: &Map,
        path_finder: &mut PathFinder,
        start: TilePosition,
        goal: TilePosition,
        starting_timestamp: ClientTick,
    ) {
        if self.action_request_locked() {
            return;
        }
        if let Some(path) = path_finder.find_walkable_path(map, start, goal) {
            if path.len() <= 1 {
                return;
            }

            let mut last_timestamp = starting_timestamp.0;
            let mut last_position: Option<TilePosition> = None;

            let steps: ArrayVec<Step, MAX_WALK_PATH_SIZE> = path
                .iter()
                .map(|&step| {
                    if let Some(position) = last_position {
                        const DIAGONAL_MULTIPLIER: f32 = 1.4;

                        let speed = match position.x == step.x || position.y == step.y {
                            // `true` means we are moving orthogonally
                            true => self.movement_speed as u32,
                            // `false` means we are moving diagonally
                            false => (self.movement_speed as f32 * DIAGONAL_MULTIPLIER) as u32,
                        };

                        let arrival_position = step;
                        let arrival_timestamp = last_timestamp + speed;

                        last_timestamp = arrival_timestamp;
                        last_position = Some(arrival_position);

                        Step {
                            arrival_position,
                            arrival_timestamp,
                        }
                    } else {
                        last_position = Some(start);

                        Step {
                            arrival_position: start,
                            arrival_timestamp: last_timestamp,
                        }
                    }
                })
                .collect();

            // If there is only a single step the player is already on the correct tile.
            if steps.len() > 1 {
                self.active_movement = Movement::new(steps, starting_timestamp.0).into();

                if !self.animation_state.is_walking() {
                    self.animation_state.walk(self.entity_type, self.movement_speed, starting_timestamp);
                }
            }
        }
    }

    fn action_request_locked(&self) -> bool {
        self.trick_dead || self.animation_state.is_dead()
    }

    /// A player stands in the battle-ready pose (ReadyFight, group 4) rather
    /// than the peaceful unarmed Idle (group 0) whenever a weapon is equipped
    /// (or in PK mode) — but relaxes to Idle on town / safe maps, where there
    /// are no monsters. The weapon sprite's Idle frames are blank, so an armed
    /// player renders no weapon while relaxed; that matches the classic look
    /// of a sheathed weapon in town.
    fn wants_ready_fight_stance(&self) -> bool {
        !self.in_safe_zone && (self.is_pk_mode_on || self.weapon != 0)
    }

    fn start_cast(&mut self, skill_id: SkillId, cast_ms: u32, now: ClientTick) {
        self.active_cast = (cast_ms > 0).then_some(ActorCast {
            _skill_id: skill_id,
            ends_at: ClientTick(now.0.saturating_add(cast_ms)),
            total_ms: cast_ms,
        });
    }

    fn clear_cast(&mut self) {
        self.active_cast = None;
    }

    fn cast_bar(&self, now: ClientTick) -> Option<(f32, f32)> {
        let cast = self.active_cast?;
        if cast.ends_at.0 <= now.0 {
            return None;
        }
        let total = cast.total_ms.max(1) as f32;
        Some(((cast.ends_at.0 - now.0) as f32, total))
    }

    fn update_state(&mut self, option: u32, body_state: u16, health_state: u16, is_pk_mode_on: bool, client_tick: ClientTick) {
        self.option = option;
        self.body_state = body_state;
        self.health_state = health_state;
        let pk_changed = self.is_pk_mode_on != is_pk_mode_on;
        self.is_pk_mode_on = is_pk_mode_on;
        self.animation_state
            .set_status_paused(status_freezes_animation(body_state), client_tick);

        if pk_changed && self.animation_state.is_neutral() && !self.action_request_locked() {
            self.animation_state
                .idle(self.entity_type, self.wants_ready_fight_stance(), client_tick);
        }
    }

    fn update_animation_status(&mut self, index: u16, gained: bool, client_tick: ClientTick) {
        match index {
            SI_TRICKDEAD => {
                self.trick_dead = gained;
                self.active_movement = None;
                self.clear_cast();
                if gained {
                    self.animation_state.trick_dead(self.entity_type, client_tick);
                } else if !self.animation_state.is_dead() {
                    self.animation_state
                        .idle(self.entity_type, self.wants_ready_fight_stance(), client_tick);
                }
            }
            SI_SUHIDE => {
                self.su_hide = gained;
                if self.action_request_locked() {
                    return;
                }
                if gained {
                    self.animation_state.status_pose(self.entity_type, self.job_id, 48, client_tick);
                } else if self.su_stoop {
                    self.animation_state.status_pose(self.entity_type, self.job_id, 47, client_tick);
                } else {
                    self.animation_state
                        .idle(self.entity_type, self.wants_ready_fight_stance(), client_tick);
                }
            }
            SI_SU_STOOP => {
                self.su_stoop = gained;
                if self.action_request_locked() || self.su_hide {
                    return;
                }
                if gained {
                    self.animation_state.status_pose(self.entity_type, self.job_id, 47, client_tick);
                } else {
                    self.animation_state
                        .idle(self.entity_type, self.wants_ready_fight_stance(), client_tick);
                }
            }
            _ => {}
        }
    }

    #[cfg(feature = "debug")]
    fn pathing_texture_coordinates(steps: &[Step], step: Vector2<usize>, index: usize) -> ([Vector2<f32>; 4], i32) {
        if steps.len() - 1 == index {
            return (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                ],
                0,
            );
        }

        let arrival_position = steps[index + 1].arrival_position;
        let delta = Vector2::new(
            arrival_position.x as isize - step.x as isize,
            arrival_position.y as isize - step.y as isize,
        );

        match delta {
            Vector2 { x: 1, y: 0 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: -1, y: 0 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 0, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 0, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 1, y: 1 } => (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                ],
                2,
            ),
            Vector2 { x: -1, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ],
                2,
            ),
            Vector2 { x: 1, y: -1 } => (
                [
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                ],
                2,
            ),
            Vector2 { x: -1, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                2,
            ),
            _other => panic!("incorrent pathing"),
        }
    }

    #[cfg(feature = "debug")]
    pub fn generate_pathing_mesh(&mut self, device: &Device, queue: &Queue, bindless_support: BindlessSupport, map: &Map) {
        use crate::{Color, NativeModelVertex};

        const PATHING_MESH_OFFSET: f32 = 0.95;

        let mut pathing_native_vertices = Vec::new();

        let Some(active_movement) = self.active_movement.as_mut() else {
            return;
        };

        let mesh_color = match self.entity_type {
            EntityType::Player => Color::rgb_u8(25, 250, 225),
            EntityType::Npc => Color::rgb_u8(170, 250, 25),
            EntityType::Monster => Color::rgb_u8(250, 100, 25),
            _ => Color::WHITE,
        };

        for (index, Step { arrival_position, .. }) in active_movement.steps.iter().copied().enumerate() {
            let Some(tile) = map.get_tile(arrival_position) else {
                korangar_debug::logging::print_debug!("[{}] movement is out of map bounds", "error".red());
                continue;
            };

            let offset = Vector2::new(
                arrival_position.x as f32 * GAT_TILE_SIZE,
                arrival_position.y as f32 * GAT_TILE_SIZE,
            );

            let first_position = Point3::new(offset.x, tile.southwest_corner_height + PATHING_MESH_OFFSET, offset.y);
            let second_position = Point3::new(
                offset.x + GAT_TILE_SIZE,
                tile.southeast_corner_height + PATHING_MESH_OFFSET,
                offset.y,
            );
            let third_position = Point3::new(
                offset.x,
                tile.northwest_corner_height + PATHING_MESH_OFFSET,
                offset.y + GAT_TILE_SIZE,
            );
            let fourth_position = Point3::new(
                offset.x + GAT_TILE_SIZE,
                tile.northeast_corner_height + PATHING_MESH_OFFSET,
                offset.y + GAT_TILE_SIZE,
            );

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(third_position, second_position, fourth_position);

            let (texture_coordinates, texture_index) = Self::pathing_texture_coordinates(
                &active_movement.steps,
                Vector2::new(arrival_position.x as usize, arrival_position.y as usize),
                index,
            );

            if let Some(first_normal) = first_normal {
                pathing_native_vertices.push(NativeModelVertex::new(
                    first_position,
                    first_normal,
                    texture_coordinates[0],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    second_position,
                    first_normal,
                    texture_coordinates[1],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    third_position,
                    first_normal,
                    texture_coordinates[2],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
            }

            if let Some(second_normal) = second_normal {
                pathing_native_vertices.push(NativeModelVertex::new(
                    third_position,
                    second_normal,
                    texture_coordinates[2],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    second_position,
                    second_normal,
                    texture_coordinates[1],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    fourth_position,
                    second_normal,
                    texture_coordinates[3],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
            }
        }

        let pathing_vertices = NativeModelVertex::convert_to_model_vertices(pathing_native_vertices, None);
        let (pathing_vertices, mut pathing_indices) = reduce_vertices(&pathing_vertices);

        let submeshes = match bindless_support {
            BindlessSupport::Full | BindlessSupport::Limited => {
                vec![SubMesh {
                    index_offset: 0,
                    index_count: pathing_indices.len() as u32,
                    base_vertex: 0,
                    texture_index: 0,
                    transparent: true,
                }]
            }
            BindlessSupport::None => split_mesh_by_texture(&pathing_vertices, &mut pathing_indices, None, None, None),
        };

        if let Some(pathing) = active_movement.pathing.as_mut() {
            pathing.vertex_buffer.write_exact(queue, pathing_vertices.as_slice());
            pathing.submeshes = submeshes;
        } else {
            let pathing_vertex_buffer = Arc::new(Buffer::with_data(
                device,
                queue,
                "pathing vertex buffer",
                BufferUsages::VERTEX | BufferUsages::COPY_DST,
                &pathing_vertices,
            ));

            let pathing_index_buffer = Arc::new(Buffer::with_data(
                device,
                queue,
                "pathing index buffer",
                BufferUsages::INDEX | BufferUsages::COPY_DST,
                &pathing_indices,
            ));

            active_movement.pathing = Some(Pathing {
                vertex_buffer: pathing_vertex_buffer,
                index_buffer: pathing_index_buffer,
                submeshes,
            });
        }
    }

    /// How a status effect recolours this entity's sprite.
    ///
    /// `opt1` is exclusive and takes precedence over the `opt2` bitmask,
    /// matching the server: `status.c` keeps them in separate fields and
    /// opt1 states are the incapacitating ones.
    fn status_tint(&self) -> StatusTint {
        match self.body_state {
            // Petrification turns the sprite to stone, which is a LOSS of colour,
            // not a darkening — hence near-full desaturation with a barely-tinted
            // multiply. A grey multiply alone reads as "standing in shadow".
            OPT1_STONE => return StatusTint::drained(Color::rgb(0.82, 0.82, 0.86), 0.95),
            // STONEWAIT is the *petrifying* phase, and Hercules deliberately lets
            // the target keep walking and attacking through it (`unit.c` exempts
            // it from the movement block; `status.c` only calls `stop_walking`
            // when the wait timer flips it to OPT1_STONE). So it must NOT look
            // like stone yet — just a hint of grey creeping in, then the snap.
            OPT1_STONEWAIT => return StatusTint::drained(Color::rgb(0.94, 0.94, 0.96), 0.3),
            OPT1_FREEZE => return StatusTint::tinted(Color::rgb(0.55, 0.75, 1.0)), // icy cyan
            OPT1_STUN => return StatusTint::tinted(Color::rgb(1.0, 0.9, 0.55)),    // dazed yellow
            _ => {}
        }

        if self.health_state & OPT2_DEADLY_POISON != 0 {
            StatusTint::tinted(Color::rgb(0.65, 0.35, 0.7)) // deadly poison — deeper violet
        } else if self.health_state & OPT2_POISON != 0 {
            StatusTint::tinted(Color::rgb(0.72, 0.55, 0.8)) // poison — sickly violet
        } else if self.health_state & OPT2_CURSE != 0 {
            // Curse drains colour too, but only partway and warmer than stone.
            StatusTint::drained(Color::rgb(0.85, 0.78, 0.78), 0.6)
        } else if self.health_state & OPT2_BLIND != 0 {
            StatusTint::drained(Color::rgb(0.75, 0.75, 0.8), 0.35) // blind — dim and washed out
        } else {
            StatusTint::NONE
        }
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, add_to_picker: bool, client_tick: ClientTick) {
        if let Some(animation_data) = self.animation_data.as_ref() {
            // M1-007: modulate the existing fade alpha so hide/cloak is visible.
            let mut alpha = self.fade_state.calculate_alpha(client_tick);
            if EntityOption::from_raw(self.option).is_concealed() {
                alpha *= Self::CONCEALED_ALPHA;
            }

            animation_data.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                self.world_position,
                &self.animation_state,
                self.direction,
                alpha,
                self.status_tint(),
                self.scale,
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, camera: &dyn Camera) {
        if let Some(animation_data) = self.animation_data.as_ref() {
            animation_data.render_debug(
                instructions,
                camera,
                self.world_position,
                &self.animation_state,
                self.direction,
                Color::rgb_u8(255, 0, 0),
                Color::rgb_u8(0, 255, 0),
                self.scale,
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) {
        renderer.render_marker(camera, marker_identifier, self.world_position, hovered);
    }
}

#[derive(Clone, RustState, StateWindow)]
pub struct Player {
    common: Common,
    pub spell_points: usize,
    pub activity_points: usize,
    pub maximum_spell_points: usize,
    pub maximum_activity_points: usize,
    pub base_level: usize,
    pub job_level: usize,
    pub stat_points: u32,
    pub strength: i32,
    pub bonus_strength: i32,
    pub strength_stat_points_cost: u8,
    pub agility: i32,
    pub bonus_agility: i32,
    pub agility_stat_points_cost: u8,
    pub vitality: i32,
    pub bonus_vitality: i32,
    pub vitality_stat_points_cost: u8,
    pub intelligence: i32,
    pub bonus_intelligence: i32,
    pub intelligence_stat_points_cost: u8,
    pub dexterity: i32,
    pub bonus_dexterity: i32,
    pub dexterity_stat_points_cost: u8,
    pub luck: i32,
    pub bonus_luck: i32,
    pub luck_stat_points_cost: u8,
    pub attack_speed: u32,
    pub skill_points: u32,
    /// Current inventory weight in 0.1 units (display as `/10`).
    pub weight: u32,
    /// Maximum inventory weight in 0.1 units (display as `/10`).
    pub maximum_weight: u32,
    /// Server critical-weight percent for natural-heal cutoff (typically 50).
    /// Used for inventory weight coloring thresholds.
    pub critical_weight_percent: u32,
    /// Melee attack range from `ZC_ATTACK_RANGE` (tiles).
    pub attack_range: AttackRange,
    /// Wallet zeny from `ZC_STATUS` / `StatType::Zeny`.
    pub zeny: u32,
    /// Current base experience points.
    pub base_experience: u64,
    /// Current job experience points.
    pub job_experience: u64,
    /// Base experience required for the next base level.
    pub next_base_experience: u64,
    /// Job experience required for the next job level.
    pub next_job_experience: u64,
}

impl Player {
    /// This function creates the player entity free-floating in the
    /// "void". When a new map is loaded on map change, the server sends
    /// the correct position we need to position the player to.
    pub fn new(library: &Library, account_id: AccountId, character_information: &CharacterInformation, client_tick: ClientTick) -> Self {
        let spell_points = character_information.spell_points as usize;
        let activity_points = 0;
        let maximum_spell_points = character_information.maximum_spell_points as usize;
        let maximum_activity_points = 0;
        let base_level = character_information.base_level as usize;
        let job_level = character_information.job_level as usize;
        let stat_points = character_information.stat_points as u32;

        let entity_data = EntityData::from_character(account_id, character_information, WorldPosition::origin());
        let tile_position = TilePosition::new(0, 0);
        let position = Point3::origin();

        let mut common = Common::new(library, &entity_data, tile_position, position, client_tick);
        // Player's own character should not fade in.
        common.fade_state = FadeState::Opaque;

        Self {
            common,
            spell_points,
            activity_points,
            maximum_spell_points,
            maximum_activity_points,
            base_level,
            job_level,
            stat_points,
            strength: character_information.strength as i32,
            bonus_strength: 0,
            strength_stat_points_cost: 0,
            agility: character_information.agility as i32,
            bonus_agility: 0,
            agility_stat_points_cost: 0,
            vitality: character_information.vitality as i32,
            bonus_vitality: 0,
            vitality_stat_points_cost: 0,
            intelligence: character_information.intelligence as i32,
            bonus_intelligence: 0,
            intelligence_stat_points_cost: 0,
            dexterity: character_information.dexterity as i32,
            bonus_dexterity: 0,
            dexterity_stat_points_cost: 0,
            luck: character_information.luck as i32,
            bonus_luck: 0,
            luck_stat_points_cost: 0,
            attack_speed: 0,
            skill_points: 0,
            weight: 0,
            maximum_weight: 0,
            // Official default natural-heal weight rate is 50%.
            critical_weight_percent: 50,
            attack_range: AttackRange(1),
            zeny: 0,
            base_experience: 0,
            job_experience: 0,
            next_base_experience: 0,
            next_job_experience: 0,
        }
    }

    pub fn clear_cast(&mut self) {
        self.common.clear_cast();
    }

    /// Remaining cast time progress as `(remaining, total)` for the cast bar.
    pub fn cast_bar(&self, now: ClientTick) -> Option<(f32, f32)> {
        self.common.cast_bar(now)
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn update_stat(&mut self, stat_type: StatType) {
        match stat_type {
            StatType::MaximumHealthPoints(value) => self.common.maximum_health_points = value as usize,
            StatType::MaximumSpellPoints(value) => self.maximum_spell_points = value as usize,
            StatType::HealthPoints(value) => self.common.health_points = value as usize,
            StatType::SpellPoints(value) => self.spell_points = value as usize,
            StatType::ActivityPoints(value) => self.activity_points = value as usize,
            StatType::MaximumActivityPoints(value) => self.maximum_activity_points = value as usize,
            StatType::MovementSpeed(value) => self.common.movement_speed = value as usize,
            StatType::BaseLevel(value) => self.base_level = value as usize,
            StatType::JobLevel(value) => self.job_level = value as usize,
            StatType::StatPoints(stat_points) => self.stat_points = stat_points,
            StatType::Strength(base, bonus) => {
                self.strength = base;
                self.bonus_strength = bonus;
            }
            StatType::Agility(base, bonus) => {
                self.agility = base;
                self.bonus_agility = bonus;
            }
            StatType::Vitality(base, bonus) => {
                self.vitality = base;
                self.bonus_vitality = bonus;
            }
            StatType::Intelligence(base, bonus) => {
                self.intelligence = base;
                self.bonus_intelligence = bonus;
            }
            StatType::Dexterity(base, bonus) => {
                self.dexterity = base;
                self.bonus_dexterity = bonus;
            }
            StatType::Luck(base, bonus) => {
                self.luck = base;
                self.bonus_luck = bonus;
            }
            StatType::StrengthStatPointCost(cost) => self.strength_stat_points_cost = cost,
            StatType::AgilityStatPointCost(cost) => self.agility_stat_points_cost = cost,
            StatType::VitalityStatPointCost(cost) => self.vitality_stat_points_cost = cost,
            StatType::IntelligenceStatPointCost(cost) => self.intelligence_stat_points_cost = cost,
            StatType::DexterityStatPointCost(cost) => self.dexterity_stat_points_cost = cost,
            StatType::LuckStatPointCost(cost) => self.luck_stat_points_cost = cost,
            StatType::AttackSpeed(attack_speed) => self.attack_speed = attack_speed,
            StatType::SkillPoints(skill_points) => self.skill_points = skill_points,
            StatType::Weight(value) => self.weight = value,
            StatType::MaximumWeight(value) => self.maximum_weight = value,
            StatType::Zeny(value) => self.zeny = value,
            StatType::BaseExperience(value) => self.base_experience = value,
            StatType::JobExperience(value) => self.job_experience = value,
            StatType::NextBaseExperience(value) => self.next_base_experience = value,
            StatType::NextJobExperience(value) => self.next_job_experience = value,
            _ => {}
        }
    }

    /// Soft overweight starts at the server's critical-weight percent (usually
    /// 50%).
    pub fn is_overweight(&self) -> bool {
        self.maximum_weight > 0 && self.weight * 100 >= self.maximum_weight * self.critical_weight_percent
    }

    /// Hard overweight at 90% of max weight (cannot attack / use skills in RO).
    pub fn is_hard_overweight(&self) -> bool {
        self.maximum_weight > 0 && self.weight * 10 >= self.maximum_weight * 9
    }

    pub fn render_status(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        theme: &WorldTheme,
        window_size: ScreenSize,
        client_tick: ClientTick,
    ) {
        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.player_bar_width;
        let gap = theme.status_bar.gap;
        let cast_height = if self.cast_bar(client_tick).is_some() {
            theme.status_bar.activity_point_height
        } else {
            0.0
        };
        let total_height = theme.status_bar.health_height
            + theme.status_bar.spell_point_height
            + theme.status_bar.activity_point_height
            + cast_height
            + gap * (if cast_height > 0.0 { 3.0 } else { 2.0 });

        let mut offset = 0.0;

        let background_position = final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0);

        let background_size = ScreenSize {
            width: bar_width,
            height: total_height,
        } + theme.status_bar.border_size * 2.0;

        renderer.render_rectangle(background_position, background_size, theme.status_bar.background_color);

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.health_height,
            },
            theme.status_bar.player_health_color,
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );

        offset += gap + theme.status_bar.health_height;

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.spell_point_height,
            },
            theme.status_bar.spell_point_color,
            self.maximum_spell_points as f32,
            self.spell_points as f32,
        );

        offset += gap + theme.status_bar.spell_point_height;

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.activity_point_height,
            },
            theme.status_bar.activity_point_color,
            self.maximum_activity_points as f32,
            self.activity_points as f32,
        );

        if let Some((remaining, total)) = self.cast_bar(client_tick) {
            offset += gap + theme.status_bar.activity_point_height;
            // Fill grows as cast completes (elapsed = total - remaining).
            let elapsed = (total - remaining).max(0.0);
            renderer.render_bar(
                final_position + ScreenPosition::only_top(offset),
                ScreenSize {
                    width: bar_width,
                    height: theme.status_bar.activity_point_height,
                },
                Color::rgb_u8(255, 210, 60),
                total,
                elapsed,
            );
        }
    }

    pub fn get_entity_part_files(&self, library: &Library, game_file_loader: &GameFileLoader) -> Vec<String> {
        let common = self.get_common();
        let mut files = get_entity_part_files(library, common.entity_type, common.job_id, common.sex, Some(common.head));
        push_weapon_part_file(&mut files, common, game_file_loader);
        push_shield_part_file(&mut files, common, game_file_loader);
        files
    }
}

#[derive(Clone, RustState, StateWindow)]
pub struct Npc {
    common: Common,
}

impl Npc {
    pub fn new(
        library: &Library,
        map: &Map,
        path_finder: &mut PathFinder,
        entity_data: EntityData,
        client_tick: ClientTick,
    ) -> Option<Self> {
        let Some(position) = map.get_world_position(entity_data.position.tile_position()) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!(
                "[{}] NPC with id {:?} is out of map bounds",
                "error".red(),
                entity_data.entity_id
            );
            return None;
        };

        let mut common = Common::new(
            library,
            &entity_data,
            entity_data.position.tile_position(),
            position,
            client_tick,
        );

        if let Some(destination) = entity_data.destination {
            common.move_from_to(
                map,
                path_finder,
                entity_data.position.tile_position(),
                destination.tile_position(),
                client_tick,
            );
        }

        Some(Self { common })
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        if self.common.entity_type != EntityType::Monster {
            return;
        }

        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.enemy_bar_width;

        renderer.render_rectangle(
            final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height,
            } + (theme.status_bar.border_size * 2.0),
            theme.status_bar.background_color,
        );

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height,
            },
            theme.status_bar.enemy_health_color,
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );
    }

    /// Ally / party-member bars: HP, SP and cast progress (other player
    /// entities appear as `Npc` with `EntityType::Player`).
    ///
    /// `health` and `spell` are passed in rather than read off `Common` because
    /// a party member's vitals arrive on `ZC_NOTIFY_HP_TO_GROUPM` and live in
    /// `PartyState`, keyed by account id — the entity itself is never told. The
    /// cast bar *is* local: `SkillCast` is broadcast to the whole area and
    /// `Entity::start_cast` writes it straight onto `Common`, so an observer
    /// already has it.
    ///
    /// Each bar is skipped independently when its data is unknown, so a member
    /// whose SP has not arrived yet still gets an HP bar.
    #[allow(clippy::too_many_arguments)]
    pub fn render_ally_status(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        theme: &WorldTheme,
        window_size: ScreenSize,
        health: Option<(usize, usize)>,
        spell: Option<(usize, usize)>,
        client_tick: ClientTick,
    ) {
        if self.common.entity_type != EntityType::Player {
            return;
        }

        let cast = self.common.cast_bar(client_tick);

        if health.is_none() && spell.is_none() && cast.is_none() {
            return;
        }

        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.enemy_bar_width;
        let bar_height = theme.status_bar.enemy_health_height;
        let gap = theme.status_bar.gap;

        let bar_count = health.is_some() as u32 + spell.is_some() as u32 + cast.is_some() as u32;
        let total_height = bar_height * bar_count as f32 + gap * bar_count.saturating_sub(1) as f32;

        renderer.render_rectangle(
            final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0),
            ScreenSize {
                width: bar_width,
                height: total_height,
            } + (theme.status_bar.border_size * 2.0),
            theme.status_bar.background_color,
        );

        let mut offset = 0.0;
        let bar_size = ScreenSize {
            width: bar_width,
            height: bar_height,
        };

        if let Some((current, maximum)) = health {
            renderer.render_bar(
                final_position,
                bar_size,
                Color::rgb_u8(80, 220, 120),
                maximum as f32,
                current as f32,
            );
            offset += gap + bar_height;
        }

        if let Some((current, maximum)) = spell {
            renderer.render_bar(
                final_position + ScreenPosition::only_top(offset),
                bar_size,
                theme.status_bar.spell_point_color,
                maximum as f32,
                current as f32,
            );
            offset += gap + bar_height;
        }

        if let Some((remaining, total)) = cast {
            // Fill grows as the cast completes, matching the local player's bar.
            let elapsed = (total - remaining).max(0.0);
            renderer.render_bar(
                final_position + ScreenPosition::only_top(offset),
                bar_size,
                Color::rgb_u8(255, 210, 60),
                total,
                elapsed,
            );
        }
    }
}

#[derive(Clone, StateElement)]
pub enum Entity {
    Player(Player),
    Npc(Npc),
}

impl Entity {
    fn get_common(&self) -> &Common {
        match self {
            Self::Player(player) => player.get_common(),
            Self::Npc(npc) => npc.get_common(),
        }
    }

    fn get_common_mut(&mut self) -> &mut Common {
        match self {
            Self::Player(player) => player.get_common_mut(),
            Self::Npc(npc) => npc.get_common_mut(),
        }
    }

    pub fn get_entity_id(&self) -> EntityId {
        self.get_common().entity_id
    }

    /// Right-hand weapon appearance (item id or class view) of this entity.
    pub fn get_weapon(&self) -> u32 {
        self.get_common().weapon
    }

    pub fn get_job_id(&self) -> JobId {
        self.get_common().job_id
    }

    pub fn get_entity_type(&self) -> EntityType {
        self.get_common().entity_type
    }

    pub fn is_death_animation_over(&self) -> bool {
        self.get_common().is_death_animation_over()
    }

    pub fn is_fading(&self) -> bool {
        self.get_common().is_fading()
    }

    pub fn fade_out(&mut self, reason: DisappearanceReason, client_tick: ClientTick) {
        const DIED_FADE_DURATION_MS: u32 = 2000;

        let fade_state = &mut self.get_common_mut().fade_state;

        let fade_duration = match reason {
            DisappearanceReason::OutOfSight => FADE_IN_DURATION_MS,
            DisappearanceReason::Died => DIED_FADE_DURATION_MS,
            _ => FADE_IN_DURATION_MS,
        };

        let current_alpha = fade_state.calculate_alpha(client_tick);
        *fade_state = FadeState::from_alpha(current_alpha, FadeDirection::Out, client_tick, fade_duration);
    }

    pub fn inherit_fade_state(&mut self, previous_entity: &Entity, client_tick: ClientTick) {
        let fade_state = &mut self.get_common_mut().fade_state;
        let previous_fade_state = previous_entity.get_common().fade_state;

        *fade_state = match previous_fade_state {
            FadeState::Opaque => FadeState::Opaque,
            FadeState::Fading { .. } => {
                let alpha = previous_fade_state.calculate_alpha(client_tick);
                FadeState::from_alpha(alpha, FadeDirection::In, client_tick, FADE_IN_DURATION_MS)
            }
        };
    }

    pub fn should_be_removed(&self, client_tick: ClientTick) -> bool {
        self.get_common().fade_state.is_done_fading_out(client_tick)
    }

    pub fn are_details_unavailable(&self) -> bool {
        match &self.get_common().details {
            ResourceState::Unavailable => true,
            _requested_or_available => false,
        }
    }

    pub fn set_job(&mut self, library: &Library, job_id: JobId) {
        let scale = match library.get::<IsBabyJob>(job_id) {
            IsBabyJob(true) => BABY_JOB_SCALE,
            IsBabyJob(false) => 1.0,
        };

        let common = self.get_common_mut();
        common.job_id = job_id;
        common.scale = scale;
    }

    /// Written through [`Common`] so it applies to any variant. Gating this on
    /// `Self::Player` meant it silently did nothing for remote players, which
    /// are `Entity::Npc`.
    pub fn set_hair(&mut self, hair_id: usize) {
        self.get_common_mut().head = hair_id;
    }

    /// Apply a sprite change that has no dedicated setter of its own.
    ///
    /// Returns whether the value actually changed, so a caller can skip
    /// rebuilding sprite layers for a no-op broadcast — the server re-sends
    /// these on enter-view, so redundant ones are routine.
    ///
    /// Deliberately written through [`Common`]: every one of these is invisible
    /// to observers if it lands on [`Player`] alone, which is exactly how every
    /// remote player ended up with hair style 1.
    pub fn set_look(&mut self, look_type: &SpriteChangeType, value: u32) -> bool {
        let common = self.get_common_mut();
        let short = value as u16;

        let previous = match look_type {
            SpriteChangeType::HeadBottom => std::mem::replace(&mut common.accessory, short),
            SpriteChangeType::HeadTop => std::mem::replace(&mut common.accessory2, short),
            SpriteChangeType::HeadMiddle => std::mem::replace(&mut common.accessory3, short),
            SpriteChangeType::HairCollor => std::mem::replace(&mut common.head_palette, short),
            SpriteChangeType::ClothesColor => std::mem::replace(&mut common.body_palette, short),
            SpriteChangeType::Robe => std::mem::replace(&mut common.robe, short),
            SpriteChangeType::Body2 => std::mem::replace(&mut common.body, short),
            // `Shoes` and `Body` have no view slot in the spawn packet and
            // nothing reads them; the original client ignores them too. They
            // reach this function rather than being dropped at the packet
            // boundary so the coverage stays visible in one place.
            SpriteChangeType::Shoes | SpriteChangeType::Body => return false,
            // Handled by their own events before reaching here.
            SpriteChangeType::Base
            | SpriteChangeType::Hair
            | SpriteChangeType::Weapon
            | SpriteChangeType::Shield
            | SpriteChangeType::Ammunition => return false,
        };

        previous != short
    }

    /// Turn in place. Movement overwrites `direction` on its own, so this is
    /// only ever the standing case (`ZC_CHANGE_DIRECTION`).
    pub fn set_direction(&mut self, direction: Direction, head_direction: u16) {
        let common = self.get_common_mut();
        common.direction = direction;
        common.head_direction = head_direction as usize;
    }

    pub fn set_animation_data(&mut self, animation_data: Arc<AnimationData>) {
        self.get_common_mut().animation_data = Some(animation_data)
    }

    pub fn animation_data(&self) -> Option<Arc<AnimationData>> {
        self.get_common().animation_data.clone()
    }

    pub fn get_entity_part_files(&self, library: &Library, game_file_loader: &GameFileLoader) -> Vec<String> {
        match self {
            Self::Player(player) => player.get_entity_part_files(library, game_file_loader),
            Self::Npc(npc) => npc.get_common().get_entity_part_files(library, game_file_loader),
        }
    }

    pub fn set_details_requested(&mut self) {
        self.get_common_mut().details = ResourceState::Requested;
    }

    pub fn set_details(&mut self, details: String) {
        self.get_common_mut().details = ResourceState::Available(details);
    }

    pub fn get_details(&self) -> Option<&String> {
        self.get_common().details.as_option()
    }

    pub fn get_tile_position(&self) -> TilePosition {
        self.get_common().tile_position
    }

    pub fn get_position(&self) -> Point3<f32> {
        self.get_common().world_position
    }

    /// Height of the actor's current composed sprite frame in world units.
    /// Returns `None` while its animation data is still loading.
    pub fn get_visual_height(&self, camera: &dyn Camera) -> Option<f32> {
        let common = self.get_common();
        common.animation_data.as_ref().map(|animation_data| {
            animation_data.current_frame_world_height(&common.animation_state, camera, common.direction, common.scale)
        })
    }

    pub fn set_position(&mut self, map: &Map, position: TilePosition, client_tick: ClientTick) {
        self.get_common_mut().set_position(map, position, client_tick);
    }

    /// Spirit spheres orbiting this entity. Drawing them is a separate asset
    /// task; this only records what the server reported.
    pub fn set_spirit_spheres(&mut self, amount: u16) {
        self.get_common_mut().spirit_spheres = amount;
    }

    pub fn set_dead(&mut self, client_tick: ClientTick) {
        let common = self.get_common_mut();
        if common.action_request_locked() {
            return;
        }
        common.active_movement = None;
        common.clear_cast();
        common.animation_state.dead(common.entity_type, client_tick);
    }

    pub fn set_idle(&mut self, client_tick: ClientTick) {
        let common = self.get_common_mut();
        if !common.action_request_locked() {
            let ready_fight_stance = common.wants_ready_fight_stance();
            common.animation_state.idle(common.entity_type, ready_fight_stance, client_tick);
        }
    }

    /// Force the entity back to a living idle pose, bypassing the death
    /// action-lock. `set_idle` no-ops while dead (`action_request_locked` is
    /// true when `is_dead`), so respawn/resurrection must use this instead or
    /// the entity stays in its death pose.
    pub fn revive(&mut self, client_tick: ClientTick) {
        let common = self.get_common_mut();
        common.trick_dead = false;
        common.active_movement = None;
        let ready_fight_stance = common.wants_ready_fight_stance();
        common.animation_state.idle(common.entity_type, ready_fight_stance, client_tick);
    }

    /// After an equipped-weapon change, flip the standing stance
    /// (Idle <-> ReadyFight) to match the new armed state — but only while the
    /// entity is already standing, so a walk/attack/sit/death motion is never
    /// interrupted.
    pub fn refresh_neutral_stance(&mut self, client_tick: ClientTick) {
        let common = self.get_common_mut();
        if common.animation_state.is_neutral_standing() {
            let ready_fight_stance = common.wants_ready_fight_stance();
            common.animation_state.idle(common.entity_type, ready_fight_stance, client_tick);
        }
    }

    pub fn set_sit(&mut self, client_tick: ClientTick) {
        let common = self.get_common_mut();
        if !common.action_request_locked() {
            common.animation_state.sit(common.entity_type, client_tick);
        }
    }

    pub fn is_sitting(&self) -> bool {
        self.get_common().animation_state.is_sitting()
    }

    pub fn set_pickup(&mut self, client_tick: ClientTick) {
        let common = self.get_common_mut();
        if !common.action_request_locked() {
            common.animation_state.pickup(common.entity_type, client_tick);
        }
    }

    pub fn rotate_towards(&mut self, target_position: TilePosition) {
        let common = self.get_common_mut();

        // FIX: This check is a little bit broken. This will prefer rotation diagonally
        // over rotating straight.
        if let Ok(direction) = Direction::try_from([
            (common.tile_position.x as isize - target_position.x as isize).clamp(-1, 1),
            (common.tile_position.y as isize - target_position.y as isize).clamp(-1, 1),
        ]) {
            common.direction = direction;
        }
    }

    pub fn set_attack(&mut self, _attack_duration: u32, critical: bool, client_tick: ClientTick) {
        let common = self.get_common_mut();
        // Execution is terminal for the cast overlay even when the actor pose
        // request is rejected by Trick Dead/death. Ragexe's request guard owns
        // the pose, not the lifetime of the already completed server cast.
        common.clear_cast();
        if common.action_request_locked() {
            return;
        }
        common.animation_state.attack(common.entity_type, critical, client_tick);
    }

    pub fn set_skill_attack(&mut self, skill_id: Option<SkillId>, _attack_duration: u32, _critical: bool, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        if let Some(skill_id) = skill_id {
            let common = self.get_common_mut();
            common.clear_cast();
            if common.action_request_locked() {
                return;
            }
            let (job_id, sex, weapon) = (common.job_id, common.sex, effective_weapon_view(common.weapon, common.shield));
            common
                .animation_state
                .skill_attack(entity_type, job_id, sex, weapon, skill_id, common.su_hide, client_tick);
        } else if entity_type == EntityType::Player {
            // The target Ragexe sends both ordinary and critical player
            // attacks through the same recovered job/sex/weapon selector.
            // Its separate attack-event position is not a playback-speed
            // multiplier; see docs/specs/combat-animation-pipeline.md.
            let common = self.get_common_mut();
            common.clear_cast();
            if common.action_request_locked() {
                return;
            }
            let (job_id, sex, weapon) = (common.job_id, common.sex, effective_weapon_view(common.weapon, common.shield));
            common.animation_state.weapon_attack(entity_type, job_id, sex, weapon, client_tick);
        } else {
            // Monster/NPC ACT layouts expose only their single attack group;
            // packet critical styling must not select a nonexistent player
            // Attack3 group.
            self.set_attack(0, false, client_tick);
        }
    }

    /// Play the damaged-entity flinch using Ragexe's dMotion/288 reaction
    /// clock. Real death and Trick Dead reject the request; walking does not —
    /// Ragexe accepts state 4 and leaves path ownership independent.
    pub fn set_hurt(&mut self, damage_delay: u32, client_tick: ClientTick) {
        let common = self.get_common_mut();
        if common.action_request_locked() {
            return;
        }
        common
            .animation_state
            .hurt(common.entity_type, common.job_id, damage_delay, client_tick);
    }

    /// Calculate the target-event offset after the source action has been
    /// selected. Missing animation data contributes no ACT delay; this can
    /// only occur while an asynchronously loaded actor is not yet drawable.
    pub fn impact_delay_ms(&self, skill_id: Option<SkillId>, camera_direction: usize) -> u32 {
        let common = self.get_common();
        let direction_index = (camera_direction + u16::from(common.direction) as usize) & 7;
        let action_delay = common
            .animation_data
            .as_ref()
            .map(|animation_data| {
                animation_data.attack_impact_delay_ms(
                    &common.animation_state,
                    direction_index,
                    common.animation_state.impact_event_position_override(),
                )
            })
            .unwrap_or(0);

        action_delay.saturating_add(native_impact_extra_delay_ms(common.job_id, skill_id))
    }

    pub fn set_weapon(&mut self, weapon: u32) {
        self.get_common_mut().weapon = weapon;
    }

    /// Mark whether this entity is on a town / safe map, which relaxes an armed
    /// player's standing pose from ReadyFight back to Idle.
    pub fn set_in_safe_zone(&mut self, in_safe_zone: bool) {
        self.get_common_mut().in_safe_zone = in_safe_zone;
    }

    pub fn set_shield(&mut self, shield: u32) {
        self.get_common_mut().shield = shield;
    }

    pub fn stopped_moving(&self) -> bool {
        self.get_common().stopped_moving
    }

    pub fn stop_movement(&mut self) {
        self.get_common_mut().active_movement = None;
    }

    pub fn update_health(&mut self, health_points: usize, maximum_health_points: usize) {
        let common = self.get_common_mut();
        common.health_points = health_points;
        common.maximum_health_points = maximum_health_points;
    }

    /// Apply the complete `ZC_STATE_CHANGE` record. These four fields are one
    /// atomic native input; splitting them loses opt1 playback holds and the
    /// `+0x2C0` PK-ready neutral rule.
    pub fn update_state(&mut self, option: u32, body_state: u16, health_state: u16, is_pk_mode_on: bool, client_tick: ClientTick) {
        self.get_common_mut()
            .update_state(option, body_state, health_state, is_pk_mode_on, client_tick);
    }

    pub fn update_animation_status(&mut self, index: u16, gained: bool, client_tick: ClientTick) {
        self.get_common_mut().update_animation_status(index, gained, client_tick);
    }

    pub fn start_cast(&mut self, skill_id: SkillId, cast_ms: u32, client_tick: ClientTick) {
        self.get_common_mut().start_cast(skill_id, cast_ms, client_tick);
    }

    pub fn clear_cast(&mut self) {
        self.get_common_mut().clear_cast();
    }

    /// Whether a cast bar is still running at `client_tick`. Keyed off the same
    /// state the cast bar draws from, so "there is a bar on screen" and "a
    /// cancel would do something" can never disagree.
    pub fn is_casting(&self, client_tick: ClientTick) -> bool {
        self.get_common().cast_bar(client_tick).is_some()
    }

    pub fn update(&mut self, audio_engine: &AudioEngine<GameFileLoader>, map: &Map, camera: &dyn Camera, client_tick: ClientTick) {
        self.get_common_mut().update(audio_engine, map, camera, client_tick);
    }

    pub fn move_from_to(
        &mut self,
        map: &Map,
        path_finder: &mut PathFinder,
        from: TilePosition,
        to: TilePosition,
        starting_timestamp: ClientTick,
    ) {
        self.get_common_mut().move_from_to(map, path_finder, from, to, starting_timestamp);
    }

    #[cfg(feature = "debug")]
    pub fn generate_pathing_mesh(&mut self, device: &Device, queue: &Queue, bindless_support: BindlessSupport, map: &Map) {
        self.get_common_mut().generate_pathing_mesh(device, queue, bindless_support, map);
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, add_to_picker: bool, client_tick: ClientTick) {
        self.get_common().render(instructions, camera, add_to_picker, client_tick);
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, camera: &dyn Camera) {
        self.get_common().render_debug(instructions, camera);
    }

    #[cfg(feature = "debug")]
    pub fn get_pathing(&self) -> Option<&Pathing> {
        self.get_common()
            .active_movement
            .as_ref()
            .and_then(|movement| movement.pathing.as_ref())
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) {
        self.get_common().render_marker(renderer, camera, marker_identifier, hovered);
    }

    pub fn render_status(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        theme: &WorldTheme,
        window_size: ScreenSize,
        client_tick: ClientTick,
    ) {
        match self {
            Self::Player(player) => player.render_status(renderer, camera, theme, window_size, client_tick),
            Self::Npc(npc) => npc.render_status(renderer, camera, theme, window_size),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_ally_status(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        theme: &WorldTheme,
        window_size: ScreenSize,
        health: Option<(usize, usize)>,
        spell: Option<(usize, usize)>,
        client_tick: ClientTick,
    ) {
        match self {
            Self::Player(_) => {}
            Self::Npc(npc) => npc.render_ally_status(renderer, camera, theme, window_size, health, spell, client_tick),
        }
    }
}

impl VecItem for Entity {
    type Id = EntityId;

    fn get_id(&self) -> Self::Id {
        self.get_entity_id()
    }
}

// TODO: Derive this
impl StateWindow<ClientState> for Entity {
    fn to_window<'a>(_self_path: impl Path<ClientState, Self>) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Entity",
            theme: InterfaceThemeType::InGame,
            closable: true,
            // TODO: This is gonna be a bit hacky but we want to have this save path possibly be
            // None and dispaly a message if the entity disappeared.
            elements: (),
        }
    }

    fn to_window_mut<'a>(_self_path: impl Path<ClientState, Self>) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Entity",
            theme: InterfaceThemeType::InGame,
            closable: true,
            // TODO: This is gonna be a bit hacky but we want to have this save path possibly be
            // None and dispaly a message if the entity disappeared.
            elements: (),
        }
    }
}

#[cfg(test)]
mod status_effect_asset_tests {
    use super::{
        OPT1_FREEZE, OPT1_SLEEP, OPT1_STONE, OPT1_STONEWAIT, OPT1_STUN, OPT2_BLIND, OPT2_CURSE, OPT2_DEADLY_POISON, OPT2_POISON,
        OPT2_SILENCE, status_effect_asset, status_freezes_animation,
    };

    /// Hercules blocks movement for every opt1 state bar STONEWAIT/BURNING
    /// (`unit.c:1304`), so all of these are standing still server-side.
    #[test]
    fn incapacitating_states_freeze_the_sprite() {
        assert!(status_freezes_animation(OPT1_STONE));
        assert!(status_freezes_animation(OPT1_FREEZE));
        assert!(status_freezes_animation(OPT1_STUN));
        assert!(status_freezes_animation(OPT1_SLEEP));
    }

    /// The petrifying phase still walks and attacks, so freezing it here would
    /// contradict the server — the same distinction the tint table draws.
    #[test]
    fn stonewait_keeps_animating() {
        assert!(!status_freezes_animation(OPT1_STONEWAIT));
    }

    #[test]
    fn healthy_entity_keeps_animating() {
        assert!(!status_freezes_animation(0));
    }

    #[test]
    fn healthy_entity_has_no_status_visual() {
        assert_eq!(status_effect_asset(0, 0), None);
    }

    #[test]
    fn opt1_states_map_to_their_loops() {
        assert_eq!(status_effect_asset(OPT1_STUN, 0), Some("stun.str"));
        assert_eq!(status_effect_asset(OPT1_SLEEP, 0), Some("sleep.str"));
    }

    #[test]
    fn opt2_bits_map_to_their_loops() {
        assert_eq!(status_effect_asset(0, OPT2_POISON), Some("poison.str"));
        assert_eq!(status_effect_asset(0, OPT2_DEADLY_POISON), Some("poison.str"));
        assert_eq!(status_effect_asset(0, OPT2_SILENCE), Some("silence.str"));
    }

    /// opt1 is exclusive and incapacitating, so it wins over any opt2 bit —
    /// otherwise a stunned, poisoned entity could show two loops at once.
    #[test]
    fn opt1_takes_precedence_over_opt2() {
        assert_eq!(status_effect_asset(OPT1_STUN, OPT2_POISON | OPT2_SILENCE), Some("stun.str"));
    }

    /// Several opt2 bits can be set simultaneously; exactly one loop must win.
    #[test]
    fn overlapping_opt2_bits_pick_one_loop() {
        assert_eq!(status_effect_asset(0, OPT2_POISON | OPT2_SILENCE), Some("poison.str"));
    }

    /// These have no GRF asset (probed 2026-07-26), so they stay tint-only
    /// rather than falling back to a stand-in from another status.
    #[test]
    fn statuses_without_an_asset_stay_tint_only() {
        assert_eq!(status_effect_asset(0, OPT2_CURSE), None);
        assert_eq!(status_effect_asset(0, OPT2_BLIND), None);
        assert_eq!(status_effect_asset(OPT1_STONE, 0), None);
        assert_eq!(status_effect_asset(OPT1_STONEWAIT, 0), None);
        assert_eq!(status_effect_asset(OPT1_FREEZE, 0), None);
    }
}

#[cfg(test)]
mod weapon_layer_tests {
    use ragnarok_packets::{ItemId, JobId};

    use super::{
        WEAPON_VIEW_CLASS_MAX, appearance_is_offhand_weapon, combine_dual_wield_view, effective_weapon_view, get_weapon_sprite_folder,
        is_weapon_trail_path, shield_part_candidates, weapon_part_candidates, weapon_resource_suffix, weapon_view_from_appearance,
        weapon_view_from_item_id,
    };

    #[test]
    fn two_handed_weapons_use_their_own_sprites() {
        assert_eq!(weapon_resource_suffix(2), Some("검"));
        assert_eq!(weapon_resource_suffix(3), Some("양손검"));
        assert_eq!(weapon_resource_suffix(4), Some("창"));
        assert_eq!(weapon_resource_suffix(5), Some("양손창"));
        assert_eq!(weapon_resource_suffix(7), Some("양손도끼"));
    }

    #[test]
    fn classic_rods_have_no_weapon_sprite() {
        assert_eq!(weapon_resource_suffix(10), None);
        assert_eq!(weapon_resource_suffix(23), None);
    }

    #[test]
    fn transcendent_classes_reuse_base_weapon_folders() {
        assert_eq!(get_weapon_sprite_folder(JobId(4013)), "어세신");
        assert_eq!(get_weapon_sprite_folder(JobId(4018)), "로그");
        assert_eq!(get_weapon_sprite_folder(JobId(4008)), "기사");
    }

    #[test]
    fn priest_family_weapons_live_under_priest_folder() {
        assert_eq!(get_weapon_sprite_folder(JobId(8)), "프리스트");
        assert_eq!(get_weapon_sprite_folder(JobId(4009)), "프리스트");
    }

    #[test]
    fn third_classes_keep_their_own_weapon_folders() {
        assert_eq!(get_weapon_sprite_folder(JobId(4054)), "룬나이트");
        assert_eq!(get_weapon_sprite_folder(JobId(4059)), "길로틴크로스");
    }

    #[test]
    fn item_ids_map_to_classic_weapon_views() {
        assert_eq!(weapon_view_from_item_id(1101), 2); // Sword
        assert_eq!(weapon_view_from_item_id(1201), 1); // Knife
        assert_eq!(weapon_view_from_item_id(1250), 16); // Jur / katar
        assert_eq!(weapon_view_from_item_id(1401), 4); // Javelin
        assert_eq!(weapon_view_from_item_id(1451), 5); // Pike
        assert_eq!(weapon_view_from_item_id(1530), 8); // Mjolnir → mace
        assert_eq!(weapon_view_from_item_id(1116), 3); // Katana nested 2HS
        assert_eq!(weapon_view_from_item_id(501), 0); // potion
    }

    #[test]
    fn appearance_class_views_pass_through_real_weapon_id() {
        assert_eq!(weapon_view_from_appearance(2), 2);
        assert_eq!(weapon_view_from_appearance(31), weapon_view_from_item_id(31));
        assert!(WEAPON_VIEW_CLASS_MAX == 31);
        // Item ID path
        assert_eq!(weapon_view_from_appearance(1530), 8);
    }

    #[test]
    fn assassin_left_right_combine_to_dual_views() {
        assert_eq!(combine_dual_wield_view(1, 1), Some(25));
        assert_eq!(combine_dual_wield_view(2, 2), Some(26));
        assert_eq!(combine_dual_wield_view(6, 6), Some(27));
        assert_eq!(combine_dual_wield_view(1, 2), Some(28));
        assert_eq!(combine_dual_wield_view(2, 1), Some(28));
        assert_eq!(combine_dual_wield_view(1, 6), Some(29));
        assert_eq!(combine_dual_wield_view(2, 6), Some(30));
        assert_eq!(combine_dual_wield_view(1, 4), None); // dagger + spear
        assert_eq!(effective_weapon_view(1201, 1201), 25); // two dagger items
        assert_eq!(effective_weapon_view(1101, 1201), 28); // sword + dagger items
        assert_eq!(effective_weapon_view(1101, 2101), 2); // sword + Guard shield
        assert_eq!(effective_weapon_view(16, 0), 16); // katar alone
    }

    #[test]
    fn offhand_weapon_detection_distinguishes_shields() {
        assert!(appearance_is_offhand_weapon(1201)); // dagger item
        assert!(appearance_is_offhand_weapon(25)); // pre-combined dual class
        assert!(!appearance_is_offhand_weapon(0));
        assert!(!appearance_is_offhand_weapon(2101)); // Guard item
        assert!(!appearance_is_offhand_weapon(1)); // Guard class view
        assert!(!appearance_is_offhand_weapon(3)); // Shield class view
        assert!(!appearance_is_offhand_weapon(2)); // Buckler class view
    }

    #[test]
    fn per_item_candidates_precede_class_suffix() {
        let c = weapon_part_candidates("기사", "남", 1530, None);
        assert_eq!(c[0], "인간족\\기사\\기사_남_1530");
        assert!(c.iter().any(|p| p.ends_with("_클럽")));
    }

    #[test]
    fn dual_class_candidate_inserted_before_single_class() {
        let c = weapon_part_candidates("어세신", "남", 1201, Some(25));
        assert!(c.iter().any(|p| p.ends_with("_단검_단검")));
        assert!(c.iter().any(|p| p.ends_with("_단검") && !p.ends_with("_단검_단검")));
    }

    #[test]
    fn trail_path_suffix_is_detected() {
        assert!(is_weapon_trail_path("인간족\\기사\\기사_남_검_검광"));
        assert!(!is_weapon_trail_path("인간족\\기사\\기사_남_검"));
    }

    #[test]
    fn native_geom_trail_views_match_ragexe_switch_table() {
        // Melee blades/spears/axes + katar/guns + dual pairs.
        for view in [1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 25, 26, 27, 28, 29, 30] {
            assert!(super::native_weapon_view_has_geom_trail(view), "view {view} should trail");
        }
        // Mace, rod, bow, book, etc. — no class trail in native.
        for view in [8, 9, 10, 11, 12, 13, 14, 15, 19, 20, 22, 23] {
            assert!(!super::native_weapon_view_has_geom_trail(view), "view {view} should not trail");
        }
    }

    #[test]
    fn bow_fires_a_ranged_projectile_melee_does_not() {
        // Bow (view 11) draws the arrow item sprite as its projectile.
        assert_eq!(super::ranged_attack_projectile_sprite(11), Some("아이템\\화살.spr"));
        // A bow item id also resolves through the appearance path.
        assert_eq!(
            super::ranged_attack_projectile_sprite(super::weapon_view_from_item_id(1710)),
            Some("아이템\\화살.spr")
        );
        // Melee weapons and bare hands never spawn a projectile.
        for view in [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 13, 14, 15, 16, 23] {
            assert_eq!(super::ranged_attack_projectile_sprite(view), None, "view {view} is melee");
        }
    }

    #[test]
    fn firearms_and_shuriken_fire_their_own_projectile() {
        // Gunslinger firearms (handgun/rifle/gatling/shotgun/launcher) draw a
        // Bullet, huuma shuriken draws a Shuriken. Both sprites were confirmed
        // present in the configured GRFs.
        for view in 17..=21 {
            assert_eq!(
                super::ranged_attack_projectile_sprite(view),
                Some("아이템\\탄약통.spr"),
                "view {view} is a firearm"
            );
        }
        assert_eq!(super::ranged_attack_projectile_sprite(22), Some("아이템\\수리검.spr"));
    }

    #[test]
    fn default_ammunition_matches_the_hercules_item_ids() {
        // db/re/item_db.conf: Arrow 1750, Bullet 13200, Shuriken 13250.
        assert_eq!(super::ranged_attack_default_ammunition(11), Some(ItemId(1750)));
        for view in 17..=21 {
            assert_eq!(super::ranged_attack_default_ammunition(view), Some(ItemId(13200)));
        }
        assert_eq!(super::ranged_attack_default_ammunition(22), Some(ItemId(13250)));
        // Every view with a projectile sprite also has default ammo, so the two
        // tables can never disagree about which weapons are ranged.
        for view in 0..super::WEAPON_VIEW_CLASS_MAX {
            assert_eq!(
                super::ranged_attack_projectile_sprite(view).is_some(),
                super::ranged_attack_default_ammunition(view).is_some(),
                "view {view}"
            );
        }
    }

    #[test]
    fn ammunition_sprite_path_uses_the_item_folder() {
        // Iron Arrow's iteminfo resource, as the projectile the client draws.
        assert_eq!(super::ammunition_projectile_sprite_path("철화살"), "아이템\\철화살.spr");
    }

    #[test]
    fn elemental_arrows_map_to_their_own_sprite() {
        // Ids and elements come from each item's `bonus bAtkEle,Ele_*` script in
        // db/re/item_db.conf, not from its name. All nine sprites were confirmed
        // present in data.grf with tools/grf_list.py.
        let expected = [
            (1751u32, "은화살"),    // Silver Arrow — Holy
            (1752, "불화살"),       // Fire Arrow — Fire
            (1754, "수정화살"),     // Crystal Arrow — Water
            (1755, "바람의화살"),   // Arrow of Wind — Wind
            (1756, "돌화살"),       // Stone Arrow — Earth
            (1757, "무형의화살"),   // Immaterial Arrow — Ghost
            (1762, "녹슨화살"),     // Rusty Arrow — Poison
            (1763, "독화살"),       // Poison Arrow — Poison
            (1767, "그림자의화살"), // Arrow of Shadow — Dark
        ];

        for (item_id, resource) in expected {
            assert_eq!(
                super::elemental_ammunition_resource(ItemId(item_id)),
                Some(resource),
                "item {item_id}"
            );
            // None of them may collapse back onto the generic arrow, which is the
            // whole point of the table.
            assert_ne!(resource, super::GENERIC_ARROW_RESOURCE, "item {item_id}");
        }

        // Plain Arrow keeps the generic sprite, and the three elemental arrows that
        // ship no distinct sprite stay absent rather than being guessed at.
        for item_id in [1750, 1759, 1766, 1772] {
            assert_eq!(super::elemental_ammunition_resource(ItemId(item_id)), None, "item {item_id}");
        }
    }

    #[test]
    fn ammunition_elements_match_their_item_db_script() {
        use super::AmmunitionElement::*;

        // From `bonus bAtkEle,Ele_*` in db/re/item_db.conf.
        let expected = [
            (1751u32, Holy), // Silver Arrow
            (1752, Fire),    // Fire Arrow
            (1754, Water),   // Crystal Arrow
            (1755, Wind),    // Arrow of Wind
            (1756, Earth),   // Stone Arrow
            (1757, Ghost),   // Immaterial Arrow
            (1759, Water),   // Frozen Arrow — no distinct sprite, glow only
            (1762, Poison),  // Rusty Arrow
            (1763, Poison),  // Poison Arrow
            (1766, Holy),    // Arrow of Counter Evil — no distinct sprite, glow only
            (1767, Dark),    // Arrow of Shadow
            (1772, Holy),    // Holy Arrow — no distinct sprite, glow only
        ];

        for (item_id, element) in expected {
            assert_eq!(super::ammunition_element(ItemId(item_id)), Some(element), "item {item_id}");
        }

        // Neutral ammo must not glow at all: plain/Steel/Stun/Iron Arrow, and the
        // status-effect arrows that carry no element.
        for item_id in [1750, 1753, 1758, 1760, 1761, 1764, 1765, 1768, 1769, 1770] {
            assert_eq!(super::ammunition_element(ItemId(item_id)), None, "item {item_id}");
        }
    }

    #[test]
    fn every_elemental_sprite_arrow_also_has_an_element() {
        // The glow table is the wider of the two; a sprite entry without a matching
        // element would be an arrow that changes shape but never lights up.
        for item_id in [1751, 1752, 1754, 1755, 1756, 1757, 1762, 1763, 1767] {
            assert!(
                super::ammunition_element(ItemId(item_id)).is_some(),
                "item {item_id} has a sprite but no element"
            );
        }
    }

    #[test]
    fn native_shield_paths_match_ragexe_sprintf_forms() {
        // Class form: 방패\%s_%s%s with job token "기사\기사", sex "남", name "_가드"
        assert_eq!(shield_part_candidates("기사", "남", 1), vec![
            "방패\\기사\\기사_남_가드".to_owned()
        ]);
        assert_eq!(shield_part_candidates("기사", "남", 2)[0], "방패\\기사\\기사_남_버클러");
        assert_eq!(shield_part_candidates("기사", "여", 3)[0], "방패\\기사\\기사_여_쉴드");
        assert_eq!(shield_part_candidates("기사", "남", 4)[0], "방패\\기사\\기사_남_미러쉴드");
        // Item form first when view ≥ 5: 방패\%s_%s_%d_방패
        let high = shield_part_candidates("기사", "남", 28901);
        assert_eq!(high[0], "방패\\기사\\기사_남_28901_방패");
    }
}

#[cfg(test)]
mod headgear_tests {
    use super::headgear_sprite_path;
    use crate::world::animation::{is_shield_part_path, is_weapon_part_path};

    #[test]
    fn accname_entries_keep_their_own_separator() {
        // Every entry in the shipped table looks like this: the underscore is
        // part of the stored name, so joining with another one produced
        // `남__고글` and every hat silently resolved to nothing.
        assert_eq!(headgear_sprite_path("남", "_고글"), "악세사리\\남\\남_고글");
        assert_eq!(headgear_sprite_path("여", "_10식안경"), "악세사리\\여\\여_10식안경");
    }

    #[test]
    fn a_name_without_a_separator_still_gets_one() {
        assert_eq!(headgear_sprite_path("남", "고글"), "악세사리\\남\\남_고글");
    }

    #[test]
    fn headgear_is_neither_weapon_nor_shield() {
        // Both matchers gate on their own prefix (`인간족\`, `방패\`), which is
        // what keeps a hat in the ordinary layer order — after the head and
        // before the weapon — instead of being reordered behind the body.
        let path = headgear_sprite_path("남", "_고글");
        assert!(!is_weapon_part_path(&path));
        assert!(!is_shield_part_path(&path));
    }
}
