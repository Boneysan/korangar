use std::sync::Arc;

mod native_motion;
mod native_skill;

use cgmath::{Array, Matrix4, Point3, SquareMatrix, Transform, Vector2, Vector3, Zero};
use korangar_container::Cacheable;
use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, Direction, EntityId, JobId, Sex, SkillId};
use rust_state::RustState;

use self::native_motion::{NativeMotionProgram, classic_player_state7_program, shared_state_program};
use self::native_skill::{NO_ACTION, actor_state as native_skill_actor_state};
#[cfg(feature = "debug")]
use crate::graphics::DebugRectangleInstruction;
use crate::graphics::{Color, EntityInstruction};
use crate::loaders::Sprite;
use crate::world::{ActionEvent, Actions, Camera, EntityType};

const TILE_SIZE: f32 = 10.0;
const SPRITE_SCALE: f32 = 1.4;
const PICKUP_ANIMATION_FACTOR: f32 = 25.0;
/// Ragexe converts elapsed milliseconds to ACT time by dividing by 24.0,
/// then divides by the action's delay value.
const ACT_DELAY_UNIT_MS: f32 = 24.0;
/// Internal logical state for the SI_TRICKDEAD pose. Ragexe changes the
/// displayed action directly and relies on actor `+0x16C` for the lock, so it
/// must not be confused with real state 3/death.
const TRICK_DEAD_POSE_STATE: i8 = -2;

/// Ragexe `0x009A3400`: actor jobs using player-style reaction delay scaling.
fn native_player_job(job_id: JobId) -> bool {
    matches!(job_id.0, 0..=30 | 4001..=5999)
}

#[allow(dead_code)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum AnimationActionType {
    Attack1,
    Attack2,
    Attack3,
    Die,
    Freeze1,
    Freeze2,
    Hurt,
    #[default]
    Idle,
    Pickup,
    ReadyFight,
    Sit,
    Skill,
    Special,
    Walk,
}

impl AnimationActionType {
    pub fn action_base_offset(&self, entity_type: EntityType) -> usize {
        match entity_type {
            EntityType::Hidden | EntityType::Player => match self {
                AnimationActionType::Idle => 0,
                AnimationActionType::Walk => 1,
                AnimationActionType::Sit => 2,
                AnimationActionType::Pickup => 3,
                AnimationActionType::ReadyFight => 4,
                AnimationActionType::Attack1 => 5,
                AnimationActionType::Hurt => 6,
                AnimationActionType::Freeze1 => 7,
                AnimationActionType::Die => 8,
                AnimationActionType::Freeze2 => 9,
                AnimationActionType::Attack2 => 10,
                AnimationActionType::Attack3 => 11,
                AnimationActionType::Skill => 12,
                _ => 0,
            },
            EntityType::Npc | EntityType::Monster => match self {
                AnimationActionType::Idle => 0,
                AnimationActionType::Walk => 1,
                AnimationActionType::Attack1 => 2,
                AnimationActionType::Hurt => 3,
                AnimationActionType::Die => 4,
                _ => 0,
            },
            EntityType::Warp => 0,
        }
    }
}

/// Predicate rows recovered from Ragexe `0x009A2DB0`. The row assignment for
/// jobs 0..25 comes from the jump table at `0x009A2FF8`; extended jobs
/// 4002..4242 come from the byte table at `0x009A3098`. Job 4001 takes the
/// Novice row directly before that table.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum NativePlayerAttackRow {
    Swordman,
    Magician,
    Archer,
    Bow,
    Priest,
    Wizard,
    Smith,
    Assassin,
    Monk,
    Sage,
    Novice,
    Gunslinger,
    Ninja,
    Default,
}

fn native_player_attack_row(job_id: JobId) -> NativePlayerAttackRow {
    use NativePlayerAttackRow::*;

    match job_id.0 {
        0 | 23 | 4001 | 4023 | 4045 | 4190 | 4191 => Novice,
        1 | 7 | 13 | 14 | 21 | 4002 | 4008 | 4014 | 4015 | 4022 | 4024 | 4030 | 4036 | 4037 | 4044 | 4054 | 4060 | 4066 | 4073 | 4080
        | 4081 | 4082 | 4083 | 4088 | 4089 | 4090 | 4091 | 4092 | 4093 | 4094 | 4095 | 4096 | 4102 | 4109 | 4110 => Swordman,
        2 | 5 | 4003 | 4006 | 4025 | 4028 => Magician,
        3 | 4004 | 4026 => Archer,
        6 | 11 | 17 | 18 | 19 | 20 | 4007 | 4012 | 4018 | 4020 | 4021 | 4029 | 4034 | 4040 | 4042 | 4043 | 4056 | 4062 | 4068 | 4069
        | 4072 | 4075 | 4076 | 4079 | 4084 | 4085 | 4098 | 4104 | 4105 | 4108 | 4111 => Bow,
        8 | 4009 | 4031 | 4057 | 4063 | 4099 => Priest,
        9 | 4010 | 4032 | 4049 | 4055 | 4061 | 4097 | 4227 | 4240 | 4242 => Wizard,
        10 | 4011 | 4019 | 4033 | 4041 | 4058 | 4064 | 4071 | 4078 | 4086 | 4087 | 4100 | 4107 | 4112 => Smith,
        12 | 4013 | 4035 | 4059 | 4065 | 4101 => Assassin,
        15 | 4016 | 4038 | 4070 | 4077 | 4106 => Monk,
        16 | 4017 | 4039 | 4067 | 4074 | 4103 => Sage,
        24 | 4215 | 4228 | 4229 => Gunslinger,
        25 | 4222 => Ninja,
        _ => Default,
    }
}

/// `GetRealWeaponId` from the reference client's `WeaponTable_F.lub`.
/// Expansion appearance IDs collapse to classic weapon families before
/// `0x009A2DB0` evaluates the job predicate.
pub(crate) fn native_real_weapon_id(weapon: u32) -> u32 {
    match weapon {
        31..=38 => 1,
        39..=47 => 2,
        48..=51 => 3,
        52..=57 => 4,
        58..=61 => 6,
        62..=68 | 98 => 8,
        69..=72 | 99..=102 => 10,
        73..=77 => 11,
        78..=85 => 12,
        86..=88 => 14,
        89..=95 => 15,
        96..=97 => 23,
        _ => weapon,
    }
}

/// Exact player Attack2/Attack3 decision recovered from Ragexe `0x009A2DB0`
/// after its item/view lookup and dual-wield combination stages. Korangar's
/// entity appearance value is already a weapon class for ordinary equipment;
/// expansion classes still need the Lua normalization above.
fn native_player_attack_action(job_id: JobId, sex: Sex, weapon: u32) -> AnimationActionType {
    use NativePlayerAttackRow::*;

    let weapon = native_real_weapon_id(weapon);
    let attack3 = match native_player_attack_row(job_id) {
        Swordman => matches!(weapon, 4 | 5),
        Magician => weapon == 1,
        Archer => weapon != 11,
        Bow => weapon == 11,
        Priest => weapon == 15,
        Wizard => matches!((sex, weapon), (Sex::Male, 1) | (Sex::Female, 10 | 23)),
        Smith => matches!(weapon, 2 | 6..=8),
        Assassin => matches!(weapon, 16 | 25..=30),
        Monk => matches!(weapon, 0 | 12),
        Sage => matches!(weapon, 5 | 10 | 15 | 23),
        Novice => match sex {
            Sex::Female => weapon == 1,
            Sex::Male => matches!(weapon, 2 | 3 | 6..=10 | 23),
            _ => false,
        },
        Gunslinger => matches!(weapon, 18..=21),
        Ninja => weapon == 22,
        Default => false,
    };

    match attack3 {
        true => AnimationActionType::Attack3,
        false => AnimationActionType::Attack2,
    }
}

/// Attack event position returned by Ragexe `0x00991580`, in ACT motion
/// units. The caller multiplies this value by the current action delay when
/// scheduling arrows and instrument/whip hit delivery. It is not an animation
/// speed multiplier and must not be applied to the ACT playback clock.
pub(crate) fn native_player_attack_event_position(job_id: JobId, sex: Sex, weapon: u32, attack3: bool) -> f32 {
    if !attack3 {
        return match (job_id.0, sex) {
            (5, Sex::Male) => 5.85,
            (6, _) => 5.75,
            _ => 6.0,
        };
    }

    match job_id.0 {
        12 | 4013 | 4035 | 4059 | 4065 | 4101 if matches!(weapon, 16 | 25..=30) => 3.0,
        0 | 23 | 4045 | 4190 | 4191 if sex == Sex::Male => 5.85,
        _ => 6.0,
    }
}

/// Attack event position returned by the shared actor path at Ragexe
/// `0x008A9500`. An authored `"atk"` event wins; actions without one use
/// `motion_count - 2`, clamped at zero.
fn native_shared_attack_event_position(frame_count: usize, attack_event_index: Option<usize>) -> f32 {
    attack_event_index.unwrap_or_else(|| frame_count.saturating_sub(2)) as f32
}

fn action_type_for_base_offset(entity_type: EntityType, action_base_offset: usize) -> AnimationActionType {
    match entity_type {
        EntityType::Hidden | EntityType::Player => match action_base_offset {
            0 => AnimationActionType::Idle,
            1 => AnimationActionType::Walk,
            2 => AnimationActionType::Sit,
            3 => AnimationActionType::Pickup,
            4 => AnimationActionType::ReadyFight,
            5 => AnimationActionType::Attack1,
            6 => AnimationActionType::Hurt,
            7 => AnimationActionType::Freeze1,
            8 => AnimationActionType::Die,
            9 => AnimationActionType::Freeze2,
            10 => AnimationActionType::Attack2,
            11 => AnimationActionType::Attack3,
            12 => AnimationActionType::Skill,
            _ => AnimationActionType::Special,
        },
        EntityType::Monster | EntityType::Npc => match action_base_offset {
            0 => AnimationActionType::Idle,
            1 => AnimationActionType::Walk,
            2 => AnimationActionType::Attack1,
            3 => AnimationActionType::Hurt,
            4 => AnimationActionType::Die,
            _ => AnimationActionType::Special,
        },
        EntityType::Warp => AnimationActionType::Idle,
    }
}

/// Flat ACT group chosen by the shared actor dispatcher at Ragexe
/// `0x008A92D0`. State 13 is dynamic and is handled before this function.
fn native_shared_action_base_offset(actor_state: i8, job_id: JobId) -> Option<usize> {
    match actor_state {
        0 => Some(0),
        1 => Some(1),
        2 | 43 | 44 => Some(10),
        3 => Some(8),
        4 => Some(6),
        5 | 20 | 25 => Some(3),
        6 | 7 | 8 | 10 | 14 | 15 | 29 | 31 | 46 => Some(2),
        9 | 18 | 32 | 49 => Some(11),
        11 => Some(4),
        12 | 19 | 28 | 40 | 42 | 50 | 51 => Some(5),
        16 | 17 | 21..=24 | 27 | 30 | 33..=39 | 41 | 45 | 47 => Some(12),
        26 if matches!(job_id.0, 4049 | 4227 | 4240 | 4242) => Some(11),
        26 => Some(5),
        _ => None,
    }
}

/// Flat action selected by the classic sprite-player (`CPc`) state-7 branch
/// at Ragexe `0x009779A0`. This is intentionally not the neighboring
/// `CGrannyPc` job table: Korangar renders classic SPR/ACT actors.
fn native_classic_player_state7_action_base_offset(job_id: JobId) -> usize {
    match job_id.0 {
        4218..=4221 => 10,
        14 | 19 | 20 | 24 | 4037 | 4042 | 4043 | 4046 | 4047 | 4048 | 4225 | 4226 | 4228 | 4238 | 4239 | 4241 | 4243 | 4244 => 4,
        _ => 12,
    }
}

fn native_classic_player_state8_action_base_offset(job_id: JobId) -> usize {
    match job_id.0 {
        4218 | 4220 => 5,
        _ => 12,
    }
}

fn native_classic_player_state2_action(job_id: JobId, selected_action: AnimationActionType, random_tenth: u32) -> AnimationActionType {
    match job_id.0 {
        4046 | 4047 | 4048 | 4225 | 4226 | 4238 | 4239 | 4241 | 4243 | 4244 => match random_tenth < 7 {
            true => AnimationActionType::Attack3,
            false => AnimationActionType::Attack2,
        },
        _ => selected_action,
    }
}

fn normalized_native_actor_state(requested_state: i8) -> i8 {
    match requested_state {
        // The dispatcher installs state 49's program but stores logical state
        // 7, so its completed program follows state 7's neutral exit.
        49 => 7,
        _ => requested_state,
    }
}

#[derive(Clone, Debug)]
pub struct AnimationState {
    action_type: AnimationActionType,
    /// Native action groups are flat ACT offsets. Keep the resolved value;
    /// deriving it later from a player/monster semantic name loses shared
    /// skill groups such as monster state 7 -> flat group 2.
    action_base_offset: usize,
    native_actor_state: i8,
    start_time: ClientTick,
    time: u32,
    factor: Option<f32>,
    hurt_motion: Option<HurtMotionTiming>,
    motion_program: Option<NativeMotionProgram>,
    impact_event_position_override: Option<f32>,
    looping: bool,
    /// `opt1` stone/freeze set Ragexe's playback hold byte. Position and
    /// subsequent action selection may still change, but ACT time does not.
    paused: bool,
    event_cursor: FrameEventCursor,
}

#[derive(Copy, Clone, Debug)]
struct HurtMotionTiming {
    reaction_cycles: f32,
    uses_body_act_delay: bool,
}

/// Tracks how far into one playback frame events have been delivered. Keyed
/// to the playback identity (start tick + selected flat action) so a new
/// action naturally starts a fresh cursor without every setter resetting it.
#[derive(Copy, Clone, Debug, Default)]
struct FrameEventCursor {
    initialized: bool,
    playback_start: u32,
    action_base_offset: usize,
    last_raw_motion: Option<usize>,
    last_step_serial: Option<u32>,
}

impl FrameEventCursor {
    fn new(start_time: ClientTick, action_base_offset: usize) -> Self {
        Self {
            initialized: true,
            playback_start: start_time.0,
            action_base_offset,
            last_raw_motion: None,
            last_step_serial: None,
        }
    }

    fn matches(&self, start_time: ClientTick, action_base_offset: usize) -> bool {
        self.initialized && self.playback_start == start_time.0 && self.action_base_offset == action_base_offset
    }
}

impl AnimationState {
    pub fn new(entity_type: EntityType, start_time: ClientTick) -> Self {
        let action_type = AnimationActionType::Idle;
        Self {
            action_type,
            action_base_offset: action_type.action_base_offset(entity_type),
            native_actor_state: 0,
            start_time,
            time: 0,
            factor: None,
            hurt_motion: None,
            motion_program: None,
            impact_event_position_override: None,
            looping: true,
            paused: false,
            event_cursor: FrameEventCursor::default(),
        }
    }

    fn set_native_action(
        &mut self,
        entity_type: EntityType,
        action_base_offset: usize,
        native_actor_state: i8,
        looping: bool,
        client_tick: ClientTick,
    ) {
        self.action_type = action_type_for_base_offset(entity_type, action_base_offset);
        self.action_base_offset = action_base_offset;
        self.native_actor_state = native_actor_state;
        self.start_time = client_tick;
        self.time = 0;
        self.factor = None;
        self.hurt_motion = None;
        self.motion_program = None;
        self.impact_event_position_override = None;
        self.looping = looping;
    }

    pub fn idle(&mut self, entity_type: EntityType, pk_mode: bool, client_tick: ClientTick) {
        self.return_to_neutral(entity_type, pk_mode, client_tick);
    }

    pub fn attack(&mut self, entity_type: EntityType, critical: bool, client_tick: ClientTick) {
        self.action_type = match critical {
            true => AnimationActionType::Attack3,
            false => AnimationActionType::Attack1,
        };
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.native_actor_state = 2;
        self.start_time = client_tick;
        self.time = 0;
        // Ragexe does not stretch the source action to the packet's sMotion.
        // The ACT delay remains the source animation clock; sMotion belongs to
        // packet/combat scheduling rather than frame playback.
        self.factor = None;
        self.hurt_motion = None;
        self.motion_program = None;
        self.impact_event_position_override = None;
        self.looping = false;
    }

    /// Apply the target Ragexe's recovered player job/sex/weapon selector.
    /// Raw item-ID lookup and Assassin off-hand combination remain upstream
    /// appearance-normalization responsibilities.
    pub fn weapon_attack(&mut self, entity_type: EntityType, job_id: JobId, sex: Sex, weapon: u32, client_tick: ClientTick) {
        let random_tenth = rand_aes::tls::rand_range_u32(0..=9);
        self.weapon_attack_for_state(entity_type, job_id, sex, weapon, 2, random_tenth, client_tick);
    }

    fn weapon_attack_for_state(
        &mut self,
        entity_type: EntityType,
        job_id: JobId,
        sex: Sex,
        weapon: u32,
        native_actor_state: i8,
        random_tenth: u32,
        client_tick: ClientTick,
    ) {
        let selected_action = native_player_attack_action(job_id, sex, weapon);
        self.action_type = match native_actor_state {
            2 => native_classic_player_state2_action(job_id, selected_action, random_tenth),
            _ => selected_action,
        };
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.native_actor_state = native_actor_state;
        self.start_time = client_tick;
        self.time = 0;
        self.factor = None;
        self.hurt_motion = None;
        self.motion_program = None;
        self.impact_event_position_override = Some(native_player_attack_event_position(
            job_id,
            sex,
            weapon,
            self.action_type == AnimationActionType::Attack3,
        ));
        self.looping = false;
    }

    /// Resolve a skill packet through the reference client's exhaustive
    /// skill-state lookup, player intercept, and shared flat-action mapping.
    /// Returns false when a native no-action or precedence guard preserves the
    /// current action.
    pub fn skill_attack(
        &mut self,
        entity_type: EntityType,
        job_id: JobId,
        sex: Sex,
        weapon: u32,
        skill_id: SkillId,
        state_seven_locked: bool,
        client_tick: ClientTick,
    ) -> bool {
        let actor_state = native_skill_actor_state(skill_id);
        if actor_state == NO_ACTION || self.native_actor_state == 3 {
            return false;
        }

        if actor_state == 7 && (self.native_actor_state == 6 || state_seven_locked) {
            return false;
        }

        if entity_type == EntityType::Player {
            match actor_state {
                2 => {
                    let random_tenth = rand_aes::tls::rand_range_u32(0..=9);
                    self.weapon_attack_for_state(entity_type, job_id, sex, weapon, actor_state, random_tenth, client_tick);
                    return true;
                }
                9 => {
                    self.weapon_attack_for_state(entity_type, job_id, sex, weapon, actor_state, 0, client_tick);
                    return true;
                }
                7 => {
                    let action_base_offset = native_classic_player_state7_action_base_offset(job_id);
                    self.set_native_action(entity_type, action_base_offset, actor_state, false, client_tick);
                    self.impact_event_position_override = Some(6.0);
                    self.motion_program = classic_player_state7_program(job_id);
                    return true;
                }
                // Present for completeness even though this executable's
                // current catalog does not map a skill ID to state 8.
                8 => {
                    self.set_native_action(
                        entity_type,
                        native_classic_player_state8_action_base_offset(job_id),
                        actor_state,
                        false,
                        client_tick,
                    );
                    return true;
                }
                _ => {}
            }
        }

        if actor_state == 13 {
            if entity_type == EntityType::Player {
                self.weapon_attack_for_state(entity_type, job_id, sex, weapon, actor_state, 0, client_tick);
            } else {
                self.set_native_action(entity_type, 2, actor_state, false, client_tick);
            }
            return true;
        }

        let Some(action_base_offset) = native_shared_action_base_offset(actor_state, job_id) else {
            debug_assert!(false, "unhandled native actor state {actor_state} for skill {}", skill_id.0);
            return false;
        };
        let looping = matches!(actor_state, 0 | 1);
        self.set_native_action(
            entity_type,
            action_base_offset,
            normalized_native_actor_state(actor_state),
            looping,
            client_tick,
        );
        let random_variant = match actor_state {
            37 | 38 | 41 => rand_aes::tls::rand_range_u32(0..=1) != 0,
            _ => false,
        };
        self.motion_program = shared_state_program(actor_state, action_base_offset, job_id, random_variant);
        true
    }

    /// Apply the recovered post-completion transition for the current native
    /// higher actor state. Shared states 2/4/5/7/9/12/45 request state zero;
    /// classic player state 51 first becomes state 2 and exits one update
    /// later. Every other higher state deliberately holds until another game
    /// event requests a state change.
    pub fn apply_completion_transition(&mut self, entity_type: EntityType, pk_mode: bool, client_tick: ClientTick) {
        match self.native_actor_state {
            2 | 4 | 5 | 7 | 9 | 12 | 45 => self.return_to_neutral(entity_type, pk_mode, client_tick),
            51 => self.native_actor_state = 2,
            _ => {}
        }
    }

    fn return_to_neutral(&mut self, entity_type: EntityType, pk_mode: bool, client_tick: ClientTick) {
        let previous_state = self.native_actor_state;
        let action_base_offset = match entity_type == EntityType::Player && (pk_mode || matches!(previous_state, 2 | 4 | 8)) {
            true => AnimationActionType::ReadyFight.action_base_offset(entity_type),
            false => AnimationActionType::Idle.action_base_offset(entity_type),
        };
        // CPc may display ReadyFight while its logical higher state is zero.
        self.set_native_action(entity_type, action_base_offset, 0, true, client_tick);
    }

    /// Apply Ragexe's target-reaction clock. dMotion is normalized to an ACT
    /// cycle count: short reactions accelerate the per-motion delay, while
    /// long reactions play one natural cycle and hold the last motion.
    pub fn hurt(&mut self, entity_type: EntityType, job_id: JobId, damage_delay: u32, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Hurt;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.native_actor_state = 4;
        self.start_time = client_tick;
        self.time = 0;
        self.factor = None;
        self.hurt_motion = Some(HurtMotionTiming {
            reaction_cycles: damage_delay as f32 / 288.0,
            uses_body_act_delay: native_player_job(job_id),
        });
        self.motion_program = None;
        self.impact_event_position_override = None;
        self.looping = false;
    }

    pub fn pickup(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Pickup;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.native_actor_state = 5;
        self.start_time = client_tick;
        self.time = 0;
        self.factor = Some(PICKUP_ANIMATION_FACTOR);
        self.hurt_motion = None;
        self.motion_program = None;
        self.impact_event_position_override = None;
        self.looping = false;
    }

    pub fn walk(&mut self, entity_type: EntityType, movement_speed: usize, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Walk;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.native_actor_state = 1;
        self.start_time = client_tick;
        self.time = 0;
        self.factor = Some(movement_speed as f32 * 100.0 / 150.0 / 5.0);
        self.hurt_motion = None;
        self.motion_program = None;
        self.impact_event_position_override = None;
        self.looping = true;
    }

    pub fn dead(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.set_native_action(
            entity_type,
            AnimationActionType::Die.action_base_offset(entity_type),
            3,
            false,
            client_tick,
        );
    }

    pub fn sit(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.set_native_action(
            entity_type,
            AnimationActionType::Sit.action_base_offset(entity_type),
            6,
            true,
            client_tick,
        );
    }

    /// Display Trick Dead without marking the entity as genuinely dead.
    /// Native Ragexe owns the request lock in a separate `+0x16C` field and
    /// writes the death action directly rather than entering actor state 3.
    pub fn trick_dead(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.set_native_action(
            entity_type,
            AnimationActionType::Die.action_base_offset(entity_type),
            TRICK_DEAD_POSE_STATE,
            false,
            client_tick,
        );
    }

    /// Install one of the two persistent Doram status programs selected by
    /// SI_SU_STOOP (state 47) and SI_SUHIDE (state 48).
    pub fn status_pose(&mut self, entity_type: EntityType, job_id: JobId, native_actor_state: i8, client_tick: ClientTick) {
        debug_assert!(matches!(native_actor_state, 47 | 48));
        let action_base_offset = 12;
        self.set_native_action(entity_type, action_base_offset, native_actor_state, true, client_tick);
        self.motion_program = shared_state_program(native_actor_state, action_base_offset, job_id, false);
    }

    /// Pause or resume the ACT clock for petrification/freeze (`opt1` 1/2).
    /// Resuming rebases the start tick so the selected motion continues from
    /// the exact point at which the status froze it.
    pub fn set_status_paused(&mut self, paused: bool, client_tick: ClientTick) {
        if self.paused == paused {
            return;
        }

        if paused {
            self.update(client_tick);
        } else {
            self.start_time = ClientTick(client_tick.0.wrapping_sub(self.time));
        }
        self.paused = paused;
    }

    pub(crate) fn impact_event_position_override(&self) -> Option<f32> {
        self.impact_event_position_override
    }

    pub fn is_walking(&self) -> bool {
        self.action_type == AnimationActionType::Walk
    }

    pub fn is_sitting(&self) -> bool {
        self.action_type == AnimationActionType::Sit
    }

    pub fn is_dead(&self) -> bool {
        self.native_actor_state == 3
    }

    pub fn is_neutral(&self) -> bool {
        self.native_actor_state == 0
    }

    fn current_action_base_offset(&self) -> usize {
        self.motion_program
            .as_ref()
            .map(|program| program.current_step().action_base_offset)
            .unwrap_or(self.action_base_offset)
    }

    fn current_motion_override(&self) -> Option<usize> {
        self.motion_program.as_ref().map(|program| program.current_step().motion_index)
    }


    pub fn update(&mut self, client_tick: ClientTick) {
        if self.paused {
            return;
        }
        self.time = client_tick.0.wrapping_sub(self.start_time.0);
        if let Some(program) = self.motion_program.as_mut() {
            program.update(self.time);
        }
    }
}

/// One SPR+ACT layer after decode. Index 0 is always the body (clock +
/// events). Secondary layers (head, weapon, shield, …) are sampled with
/// [`native_layer_motion_index`] at compose time.
///
/// `sprites` / `actions` are `None` only in unit tests that exercise the clock
/// and event paths without GRF assets.
#[derive(Clone)]
pub struct AnimationLayer {
    /// Part path without extension (`인간족\...`), used to skip no-op swaps.
    pub path_key: Option<String>,
    pub sprites: Option<Arc<Sprite>>,
    /// Retained for Phase C2 attach-point composition at render time.
    #[allow(dead_code)]
    pub actions: Option<Arc<Actions>>,
    /// Per flat ACT action (direction included): motions for this layer only.
    pub animations: Vec<Animation>,
}

/// Player layer slots after the initial full load (body + head always present).
/// Weapon and shield are optional and ordered body → head → weapon? → shield?.
#[allow(dead_code)]
pub const PLAYER_LAYER_BODY: usize = 0;
pub const PLAYER_LAYER_HEAD: usize = 1;
/// Prefer identifying weapon/shield by [`is_shield_part_path`] rather than a
/// fixed index — shield may sit at 2 when no weapon is equipped.
pub const PLAYER_LAYER_WEAPON: usize = 2;
#[allow(dead_code)]
pub const PLAYER_LAYER_SHIELD: usize = 3;

/// Shield SPR paths live under `방패\…`, not `인간족\…`.
pub fn is_shield_part_path(path: &str) -> bool {
    path.starts_with("방패\\") || path.starts_with("방패/")
}

/// Weapon SPR paths live under `인간족\{job}\…`, never body (`몸통`) or head
/// (`머리통`) and never the shield tree.
pub fn is_weapon_part_path(path: &str) -> bool {
    if is_shield_part_path(path) {
        return false;
    }
    let normalized = path.replace('/', "\\");
    let rest = match normalized.strip_prefix("인간족\\") {
        Some(rest) => rest,
        None => return false,
    };
    !rest.starts_with("몸통\\") && !rest.starts_with("머리통\\")
}

/// First weapon path in a player part list (body, head, weapon?, shield?).
///
/// Must not use a fixed index: with shield and no weapon the shield sits at
/// index 2, and treating it as the weapon layer drops the sword on the next
/// weapon refresh.
pub fn weapon_path_from_entity_parts(parts: &[String]) -> Option<&str> {
    parts
        .iter()
        .map(String::as_str)
        .find(|path| is_weapon_part_path(path))
}

/// First shield path in a player part list.
pub fn shield_path_from_entity_parts(parts: &[String]) -> Option<&str> {
    parts
        .iter()
        .map(String::as_str)
        .find(|path| is_shield_part_path(path))
}

/// Whether the shield should be drawn *behind* the body for this camera-
/// relative facing (0..=7).
///
/// Matches the classic client hardcode (as recovered in roBrowser
/// `EntityRender.js`): `behind = direction > 1 && direction < 6` — i.e.
/// directions W / NW / N / NE (the half of the compass that shows the
/// character's back). In those facings the shield is painted first so the
/// body covers it; otherwise it is painted after weapon so it sits in front.
pub fn shield_draws_behind_body(view_direction: usize) -> bool {
    let d = view_direction & 7;
    d > 1 && d < 6
}

/// Action-global billboard layout measured across every composed motion of
/// one flat action. Applied at compose time so proportions stay stable across
/// the action (the same fix the former load-time merge used).
#[derive(Copy, Clone, Debug)]
pub struct ActionLayout {
    pub min_top: i32,
    pub max_bottom: i32,
    pub min_left: i32,
    pub max_right: i32,
}

/// Runtime-composed actor animation. Layers stay separate; body owns the
/// clock. Call [`AnimationData::compose_frame`] (or `get_frame`) to merge for
/// draw.
#[derive(RustState, Clone, StateElement)]
pub struct AnimationData {
    #[hidden_element]
    pub layers: Vec<AnimationLayer>,
    /// Body ACT delays — the only clock used for motion indexing.
    pub delays: Vec<f32>,
    /// Per flat body action: shared AABB for billboard normalize.
    #[hidden_element]
    pub action_layouts: Vec<ActionLayout>,
    #[hidden_element]
    pub entity_type: EntityType,
}

impl Cacheable for AnimationData {
    fn size(&self) -> usize {
        // We cache animations only by count.
        0
    }
}

/// Legacy pair type kept for loaders that still assemble SPR+ACT before the
/// layer decode step.
#[derive(Clone)]
pub struct AnimationPair {
    pub sprites: Arc<Sprite>,
    pub actions: Arc<Actions>,
}

#[derive(Clone)]
pub struct Animation {
    pub frames: Vec<AnimationFrame>,
}

#[derive(Clone)]
pub struct AnimationFrame {
    pub event: Option<ActionEvent>,
    /// ACT attach point for this motion, if authored (`attach_point_count == 1`).
    /// Body supplies the parent; secondary layers use
    /// `offset += -child + body` at compose time (Phase C2).
    pub attach_point: Option<Vector2<i32>>,
    pub offset: Vector2<i32>,
    pub top_left: Vector2<i32>,
    pub size: Vector2<i32>,
    pub frame_parts: Vec<AnimationFramePart>,
    #[cfg(feature = "debug")]
    pub horizontal_matrix: Matrix4<f32>,
    #[cfg(feature = "debug")]
    pub vertical_matrix: Matrix4<f32>,
}

#[derive(Clone)]
pub struct AnimationFramePart {
    pub animation_index: usize,
    pub sprite_number: usize,
    pub offset: Vector2<i32>,
    pub size: Vector2<i32>,
    pub mirror: bool,
    pub angle: f32,
    pub color: Color,
    pub affine_matrix: Matrix4<f32>,
}

impl Default for AnimationFramePart {
    fn default() -> AnimationFramePart {
        AnimationFramePart {
            animation_index: usize::MAX,
            sprite_number: usize::MAX,
            offset: Vector2::<i32>::zero(),
            size: Vector2::<i32>::zero(),
            mirror: Default::default(),
            angle: Default::default(),
            color: Default::default(),
            affine_matrix: Matrix4::<f32>::zero(),
        }
    }
}

fn animation_frame_position(animation_state: &AnimationState, delay: f32, _frame_count: usize) -> usize {
    if let Some(motion_index) = animation_state.current_motion_override() {
        return motion_index;
    }

    let frame_duration = match animation_state.hurt_motion {
        Some(timing) if timing.reaction_cycles > 0.0 && timing.reaction_cycles <= 1.0 => {
            let effective_delay = match timing.uses_body_act_delay {
                true => delay * timing.reaction_cycles,
                false => timing.reaction_cycles,
            };
            effective_delay * ACT_DELAY_UNIT_MS
        }
        _ => animation_state
            .factor
            .map(|factor| delay * factor)
            .unwrap_or(delay * ACT_DELAY_UNIT_MS),
    };

    (animation_state.time as f32 / frame_duration.max(f32::EPSILON)) as usize
}

/// True for basic weapon-swing ACT groups (Attack1/2/3). Skill/pickup/hurt
/// one-shots are excluded — their presentation is owned elsewhere.
fn is_weapon_swing_action(animation_state: &AnimationState) -> bool {
    matches!(
        animation_state.action_type,
        AnimationActionType::Attack1 | AnimationActionType::Attack2 | AnimationActionType::Attack3
    )
}

/// Motion index where a synthetic `ActionEvent::Attack` should fire when the
/// body ACT authored no events for this action. Uses the same marker as the
/// impact scheduler (`impact_event_position_override`, else
/// `motion_count - 2`) so Assassin Cross katar (table value 3.0) aligns with
/// the native attack-event position even though its body ACT is empty.
fn synthetic_attack_marker_motion(animation_state: &AnimationState, frame_count: usize) -> usize {
    let position = animation_state
        .impact_event_position_override
        .unwrap_or_else(|| native_shared_attack_event_position(frame_count, None));
    (position.floor() as usize).min(frame_count.saturating_sub(1))
}

/// The crossed-motion walk behind [`AnimationData::take_crossed_events`],
/// operating on the already-resolved action animation and delay.
///
/// A cursor keyed to the playback identity tracks the last delivered motion.
/// One-shot actions clamp at their final motion (a held frame delivers
/// nothing new); looping actions walk through the wrap. Motion programs
/// advance at most one step per actor update, so each `step_serial` change
/// is one event occurrence — a duplicate authored motion fires again, a
/// terminal held motion does not.
///
/// When a weapon-swing action's body ACT has an empty event table (common
/// for male Assassin and Assassin Cross in classic GRFs), a single synthetic
/// [`ActionEvent::Attack`] is emitted at the native attack-event marker so
/// `weapon_sound` still plays. Authored events always win and suppress the
/// fallback.
fn collect_crossed_events(animation_state: &mut AnimationState, animation: &Animation, delay: f32) -> Vec<ActionEvent> {
    let frame_count = animation.frames.len();
    if frame_count == 0 {
        return Vec::new();
    }

    if !animation_state
        .event_cursor
        .matches(animation_state.start_time, animation_state.action_base_offset)
    {
        animation_state.event_cursor = FrameEventCursor::new(animation_state.start_time, animation_state.action_base_offset);
    }
    let mut cursor = animation_state.event_cursor;
    let mut events = Vec::new();

    match animation_state.motion_program.as_ref() {
        Some(program) => {
            let serial = program.step_serial();
            if cursor.last_step_serial != Some(serial) {
                cursor.last_step_serial = Some(serial);
                let motion_index = program.current_step().motion_index.min(frame_count - 1);
                if let Some(event) = animation.frames[motion_index].event {
                    events.push(event);
                }
            }
        }
        None => {
            let raw_motion = animation_frame_position(animation_state, delay, frame_count);
            let end = match animation_state.looping {
                true => raw_motion,
                false => raw_motion.min(frame_count - 1),
            };
            let start = match cursor.last_raw_motion {
                None => 0,
                Some(last) => last.saturating_add(1),
            };

            if start <= end {
                // Native walks every crossing; bound stall recovery to the
                // final full cycle so an extreme hitch cannot flood audio.
                let first = start.max(end.saturating_sub(frame_count - 1));
                for position in first..=end {
                    if let Some(event) = animation.frames[position % frame_count].event {
                        events.push(event);
                    }
                }

                // Empty body event tables: synthesize one Attack at the
                // native marker. Authored events above already win.
                if events.is_empty()
                    && !animation_state.looping
                    && is_weapon_swing_action(animation_state)
                    && animation.frames.iter().all(|frame| frame.event.is_none())
                {
                    let marker = synthetic_attack_marker_motion(animation_state, frame_count);
                    if (first..=end).contains(&marker) {
                        events.push(ActionEvent::Attack);
                    }
                }

                cursor.last_raw_motion = Some(end);
            }
        }
    }

    animation_state.event_cursor = cursor;
    events
}

fn is_animation_complete(animation_state: &AnimationState, delay: f32, frame_count: usize) -> bool {
    if let Some(program) = animation_state.motion_program.as_ref() {
        return program.is_complete();
    }

    if let Some(timing) = animation_state.hurt_motion
        && timing.reaction_cycles > 1.0
    {
        let natural_cycle_duration = delay * ACT_DELAY_UNIT_MS * frame_count.max(1) as f32;
        return animation_state.time as f32 >= natural_cycle_duration * timing.reaction_cycles;
    }

    animation_frame_position(animation_state, delay, frame_count) >= frame_count
}

impl AnimationData {
    /// Body layer (index 0). Empty only for invalid loads.
    pub fn body_layer(&self) -> Option<&AnimationLayer> {
        self.layers.first()
    }

    fn body_animation(&self, action_index: usize) -> Option<&Animation> {
        let body = self.body_layer()?;
        body.animations.get(action_index % body.animations.len().max(1))
    }

    /// True when this data has at least body + head (player partial-swap ready).
    pub fn has_player_base_layers(&self) -> bool {
        self.layers.len() >= 2 && self.entity_type == EntityType::Player
    }

    pub fn layer_path(&self, index: usize) -> Option<&str> {
        self.layers.get(index).and_then(|layer| layer.path_key.as_deref())
    }

    /// Stamp every part's `animation_index` to its layer vector index and
    /// clear events on non-body layers.
    fn normalize_layer_indices(layers: &mut [AnimationLayer]) {
        for (layer_index, layer) in layers.iter_mut().enumerate() {
            for animation in layer.animations.iter_mut() {
                for frame in animation.frames.iter_mut() {
                    if layer_index != 0 {
                        frame.event = None;
                    }
                    for part in frame.frame_parts.iter_mut() {
                        part.animation_index = layer_index;
                    }
                }
            }
        }
    }

    /// Replace or append a layer at `index`, then recompute action layouts.
    /// Does not touch body delays (body path unchanged).
    pub fn with_layer_at(mut self, index: usize, layer: AnimationLayer) -> Self {
        if index < self.layers.len() {
            self.layers[index] = layer;
        } else if index == self.layers.len() {
            self.layers.push(layer);
        } else {
            // Unexpected gap — append rather than invent empty middle layers.
            self.layers.push(layer);
        }
        Self::normalize_layer_indices(&mut self.layers);
        self.action_layouts = compute_action_layouts(&self.layers);
        self
    }

    /// Keep only the first `len` layers (e.g. drop weapon: `truncate_layers(2)`).
    pub fn with_layers_truncated(mut self, len: usize) -> Self {
        if len < self.layers.len() {
            self.layers.truncate(len);
            Self::normalize_layer_indices(&mut self.layers);
            self.action_layouts = compute_action_layouts(&self.layers);
        }
        self
    }

    /// Split optional gear layers (after body/head) into weapon vs shield by path.
    fn player_optional_gear(&self) -> (Option<AnimationLayer>, Option<AnimationLayer>) {
        let mut weapon = None;
        let mut shield = None;
        for layer in self.layers.iter().skip(2) {
            if layer.path_key.as_deref().is_some_and(is_shield_part_path) {
                shield = Some(layer.clone());
            } else {
                weapon = Some(layer.clone());
            }
        }
        (weapon, shield)
    }

    /// Rebuild player layers as body + head + optional weapon + optional shield.
    fn with_player_gear(mut self, weapon: Option<AnimationLayer>, shield: Option<AnimationLayer>) -> Self {
        if self.layers.is_empty() {
            return self;
        }
        let mut layers = Vec::with_capacity(4);
        layers.push(self.layers[0].clone());
        if self.layers.len() > 1 {
            layers.push(self.layers[1].clone());
        }
        if let Some(weapon) = weapon {
            layers.push(weapon);
        }
        if let Some(shield) = shield {
            layers.push(shield);
        }
        self.layers = layers;
        Self::normalize_layer_indices(&mut self.layers);
        self.action_layouts = compute_action_layouts(&self.layers);
        self
    }

    /// Set or clear the weapon layer without reloading body/head/shield.
    pub fn with_weapon_layer(self, weapon: Option<AnimationLayer>) -> Self {
        let (_, shield) = self.player_optional_gear();
        self.with_player_gear(weapon, shield)
    }

    /// Set or clear the shield layer without reloading body/head/weapon.
    pub fn with_shield_layer(self, shield: Option<AnimationLayer>) -> Self {
        let (weapon, _) = self.player_optional_gear();
        self.with_player_gear(weapon, shield)
    }

    /// Replace the head layer (index 1). Requires body + head already present.
    pub fn with_head_layer(self, head: AnimationLayer) -> Self {
        self.with_layer_at(PLAYER_LAYER_HEAD, head)
    }

    pub fn shield_layer_path(&self) -> Option<&str> {
        self.layers
            .iter()
            .find(|layer| layer.path_key.as_deref().is_some_and(is_shield_part_path))
            .and_then(|layer| layer.path_key.as_deref())
    }

    /// Path of the current weapon layer, if any (never the shield path).
    pub fn weapon_layer_path(&self) -> Option<&str> {
        self.layers
            .iter()
            .find(|layer| layer.path_key.as_deref().is_some_and(is_weapon_part_path))
            .and_then(|layer| layer.path_key.as_deref())
    }

    /// Delay from source action start to Ragexe's queued target-impact event.
    /// Player weapon attacks provide the `0x00991580` marker explicitly;
    /// shared actor states use the body action's `"atk"` motion, falling back
    /// to `motion_count - 2` exactly like `0x008A9500`.
    pub(crate) fn attack_impact_delay_ms(
        &self,
        animation_state: &AnimationState,
        direction_index: usize,
        event_position_override: Option<f32>,
    ) -> u32 {
        let action_index = animation_state.current_action_base_offset() * 8 + (direction_index & 7);
        let delay = match self.delays.len() {
            0 => 4.0,
            count => self.delays[action_index % count],
        };
        let event_position = event_position_override.unwrap_or_else(|| match self.body_animation(action_index) {
            None => 0.0,
            Some(animation) => {
                let attack_event_index = animation
                    .frames
                    .iter()
                    .position(|frame| matches!(frame.event, Some(ActionEvent::Attack)));
                native_shared_attack_event_position(animation.frames.len(), attack_event_index)
            }
        });

        (event_position * delay * ACT_DELAY_UNIT_MS).round().max(0.0) as u32
    }

    pub fn is_animation_over(&self, animation_state: &AnimationState) -> bool {
        if animation_state.looping {
            return false;
        }

        let animation_action_index = animation_state.current_action_base_offset() * 8;
        if self.delays.is_empty() {
            return true;
        }
        let delay = self.delays[animation_action_index % self.delays.len()];
        let Some(animation) = self.body_animation(animation_action_index) else {
            return true;
        };

        is_animation_complete(animation_state, delay, animation.frames.len())
    }

    /// Compose the current body motion with secondary layers into one draw
    /// frame. Body owns the clock; secondary layers use
    /// [`native_layer_motion_index`] (CActRes::get_motion fallback to 0).
    /// Shield draw order is per camera-relative facing (Phase C3).
    pub fn compose_frame(&self, animation_state: &AnimationState, camera: &dyn Camera, direction: Direction) -> AnimationFrame {
        let camera_direction = camera.camera_direction();
        let view_direction = (camera_direction + u16::from(direction) as usize) & 7;
        let animation_action_index = animation_state.current_action_base_offset() * 8 + view_direction;
        self.compose_action_motion(animation_state, animation_action_index, view_direction)
    }

    /// Alias kept for call sites; returns an owned composed frame.
    pub fn get_frame(&self, animation_state: &AnimationState, camera: &dyn Camera, direction: Direction) -> AnimationFrame {
        self.compose_frame(animation_state, camera, direction)
    }

    fn compose_action_motion(
        &self,
        animation_state: &AnimationState,
        animation_action_index: usize,
        view_direction: usize,
    ) -> AnimationFrame {
        let Some(body) = self.body_layer() else {
            return empty_animation_frame();
        };
        if body.animations.is_empty() || self.delays.is_empty() {
            return empty_animation_frame();
        }

        let body_action_index = animation_action_index % body.animations.len();
        let body_animation = &body.animations[body_action_index];
        if body_animation.frames.is_empty() {
            return empty_animation_frame();
        }

        let delay = self.delays[body_action_index % self.delays.len()];
        let raw_motion = animation_frame_position(animation_state, delay, body_animation.frames.len());

        // Player idle: suppress Doridori by always showing motion 0.
        let body_motion = if self.entity_type == EntityType::Player && animation_state.action_type == AnimationActionType::Idle {
            0
        } else if animation_state.looping {
            raw_motion % body_animation.frames.len()
        } else {
            raw_motion.min(body_animation.frames.len() - 1)
        };

        let body_attach = body_animation.frames[body_motion].attach_point;

        // Build each layer's motion frame, then order for draw. `animation_index`
        // on parts still identifies the SPR layer; only the merge/paint order
        // changes so `extra_depth_offset` puts the shield behind the body when
        // facing away (native hardcode, roBrowser EntityRender).
        let mut non_shield_frames = Vec::with_capacity(self.layers.len());
        let mut shield_frames = Vec::new();
        for (layer_index, layer) in self.layers.iter().enumerate() {
            let Some(layer_animation) = layer.animations.get(body_action_index % layer.animations.len().max(1)) else {
                continue;
            };
            let Some(layer_motion) = native_layer_motion_index(body_motion, layer_animation.frames.len()) else {
                continue;
            };
            let mut frame = layer_animation.frames[layer_motion].clone();
            // Events are body-owned (native scanner walks only the body ACT).
            if layer_index != 0 {
                frame.event = None;
                // Parent secondary layers (head, …) to the body attach of the
                // *body* motion index. When the secondary layer fell back to
                // motion 0, its own attach still comes from that motion.
                apply_child_attach(&mut frame, body_attach);
            }
            if layer.path_key.as_deref().is_some_and(is_shield_part_path) {
                shield_frames.push(frame);
            } else {
                non_shield_frames.push(frame);
            }
        }

        let mut layer_frames = Vec::with_capacity(non_shield_frames.len() + shield_frames.len());
        if shield_draws_behind_body(view_direction) {
            // shield → body → head → weapon
            layer_frames.append(&mut shield_frames);
            layer_frames.append(&mut non_shield_frames);
        } else {
            // body → head → weapon → shield
            layer_frames.append(&mut non_shield_frames);
            layer_frames.append(&mut shield_frames);
        }

        let mut composed = merge_frame(&mut layer_frames);
        let layout = self
            .action_layouts
            .get(body_action_index)
            .copied()
            .unwrap_or(ActionLayout {
                min_top: 0,
                max_bottom: 0,
                min_left: 0,
                max_right: 0,
            });
        finalize_frame_layout(&mut composed, layout);
        composed
    }

    /// Deliver each ACT frame event crossed since the previous call, exactly
    /// once per crossing. Native Ragexe fires events by walking every body
    /// motion passed since the prior actor update (`0x008AC860`), including
    /// the loop wrap — not by sampling the currently displayed frame — so a
    /// slow application frame that jumps several motions still fires the
    /// events on the skipped frames.
    pub fn take_crossed_events(&self, animation_state: &mut AnimationState, camera: &dyn Camera, direction: Direction) -> Vec<ActionEvent> {
        if self.delays.is_empty() || self.body_layer().is_none_or(|layer| layer.animations.is_empty()) {
            return Vec::new();
        }

        let camera_direction = camera.camera_direction();
        let direction = (camera_direction + u16::from(direction) as usize) & 7;
        let animation_action_index = animation_state.current_action_base_offset() * 8 + direction;

        let delay = self.delays[animation_action_index % self.delays.len()];
        let Some(animation) = self.body_animation(animation_action_index) else {
            return Vec::new();
        };

        collect_crossed_events(animation_state, animation, delay)
    }

    pub fn calculate_world_matrix(
        &self,
        camera: &dyn Camera,
        frame: &AnimationFrame,
        entity_position: Point3<f32>,
        scale: f32,
    ) -> Matrix4<f32> {
        // Offset the image to below the ground by frame.offset.y.
        // Add 0.5 to change from center of pixel to the lower border of pixel
        let origin_y = -frame.offset.y as f32 + 0.5;
        // TODO - TBD : Change the entity z coordinate to 0.0.
        // Add 1.0 in z-coordinate, because the entity is at point with z = 1.0.
        // The operation is performed beforehand to correctly rotate the billboard.
        let origin = Point3::new(0.0, origin_y, 0.0) * SPRITE_SCALE / TILE_SIZE + Vector3::unit_z();
        let size = Vector2::new(frame.size.x as f32, frame.size.y as f32) * (SPRITE_SCALE / TILE_SIZE) * scale;
        camera.billboard_matrix(entity_position, origin, size)
    }

    /// Height of the entity's current rendered frame in world units. This is
    /// used by overlays that must sit above actors whose sprites vary greatly
    /// in height (players, small mobs, and bosses).
    pub fn current_frame_world_height(
        &self,
        animation_state: &AnimationState,
        camera: &dyn Camera,
        direction: Direction,
        scale: f32,
    ) -> f32 {
        let frame = self.get_frame(animation_state, camera, direction);
        frame.size.y as f32 * (SPRITE_SCALE / TILE_SIZE) * scale
    }

    pub fn get_texture_coordinates(&self) -> (Vector2<f32>, Vector2<f32>) {
        let cell_count = Vector2::new(1, 1);
        let cell_position = Vector2::new(0, 0);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);
        (texture_size, texture_position)
    }

    /// Number of flat body actions (used by emote debug / diagnostics).
    pub fn body_action_count(&self) -> usize {
        self.body_layer().map(|layer| layer.animations.len()).unwrap_or(0)
    }

    /// Total play time of one action in milliseconds, ignoring animation
    /// state factors. Used for one-shot playback like emote bubbles.
    pub fn action_duration_ms(&self, action_index: usize) -> u32 {
        if self.delays.is_empty() {
            return 0;
        }
        let delay = self.delays[action_index % self.delays.len()];
        let Some(animation) = self.body_animation(action_index) else {
            return 0;
        };
        (animation.frames.len() as f32 * delay * ACT_DELAY_UNIT_MS) as u32
    }

    /// Renders a single action selected by raw index with no direction
    /// handling, playing once and holding the last frame. Emote bubbles use
    /// the wire emote ID as the action index into `emotion.act`. Returns
    /// false if the action has no renderable frames.
    pub fn render_action_frame(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        camera: &dyn Camera,
        entity_id: EntityId,
        entity_position: Point3<f32>,
        action_index: usize,
        time_ms: u32,
    ) -> bool {
        if self.delays.is_empty() {
            return false;
        }
        let Some(animation) = self.body_animation(action_index) else {
            return false;
        };
        if animation.frames.is_empty() {
            return false;
        }

        let delay = self.delays[action_index % self.delays.len()];
        let frame_time_ms = delay * ACT_DELAY_UNIT_MS;
        let frame_index = ((time_ms as f32 / frame_time_ms) as usize).min(animation.frames.len() - 1);

        // Emotes are single-layer; compose with a synthetic idle-like state
        // that pins the requested motion via a one-shot non-looping clock.
        let body_attach = animation.frames[frame_index].attach_point;
        let mut layer_frames = Vec::with_capacity(self.layers.len());
        for (layer_index, layer) in self.layers.iter().enumerate() {
            let Some(layer_animation) = layer.animations.get(action_index % layer.animations.len().max(1)) else {
                continue;
            };
            let Some(layer_motion) = native_layer_motion_index(frame_index, layer_animation.frames.len()) else {
                continue;
            };
            let mut frame = layer_animation.frames[layer_motion].clone();
            if layer_index != 0 {
                frame.event = None;
                apply_child_attach(&mut frame, body_attach);
            }
            layer_frames.push(frame);
        }
        let mut composed = merge_frame(&mut layer_frames);
        let layout = self.action_layouts.get(action_index).copied().unwrap_or(ActionLayout {
            min_top: 0,
            max_bottom: 0,
            min_left: 0,
            max_right: 0,
        });
        finalize_frame_layout(&mut composed, layout);

        let world_matrix = self.calculate_world_matrix(camera, &composed, entity_position, 1.0);
        self.push_frame_instructions(instructions, camera, &composed, world_matrix, entity_id, false, 1.0);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        camera: &dyn Camera,
        add_to_picker: bool,
        entity_id: EntityId,
        entity_position: Point3<f32>,
        animation_state: &AnimationState,
        direction: Direction,
        fade_alpha: f32,
        scale: f32,
    ) {
        let frame = self.get_frame(animation_state, camera, direction);
        let world_matrix = self.calculate_world_matrix(camera, &frame, entity_position, scale);
        self.push_frame_instructions(instructions, camera, &frame, world_matrix, entity_id, add_to_picker, fade_alpha);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_frame_instructions(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        camera: &dyn Camera,
        frame: &AnimationFrame,
        world_matrix: Matrix4<f32>,
        entity_id: EntityId,
        add_to_picker: bool,
        fade_alpha: f32,
    ) {
        for (index, frame_part) in frame.frame_parts.iter().enumerate() {
            let animation_index = frame_part.animation_index;
            let sprite_number = frame_part.sprite_number;
            let Some(texture) = self
                .layers
                .get(animation_index)
                .and_then(|layer| layer.sprites.as_ref())
                .and_then(|sprites| sprites.textures.get(sprite_number))
            else {
                continue;
            };

            let frame_size = Vector2::new(frame.size.x as f32, frame.size.y as f32);

            let (texture_size, texture_position) = self.get_texture_coordinates();
            let (depth_offset, curvature) = camera.calculate_depth_offset_and_curvature(&world_matrix, SPRITE_SCALE, SPRITE_SCALE);

            let position = world_matrix.transform_point(Point3::from_value(0.0));
            let distance = camera.distance_to(position);
            let color = frame_part.color * fade_alpha;

            instructions.push(EntityInstruction {
                world: world_matrix,
                frame_part_transform: frame_part.affine_matrix,
                texture_position,
                texture_size,
                frame_size,
                depth_offset,
                extra_depth_offset: 0.005 * index as f32,
                curvature,
                color,
                mirror: frame_part.mirror,
                entity_id,
                add_to_picker,
                texture: texture.clone(),
                distance,
            });
        }
    }

    #[cfg(feature = "debug")]
    #[allow(clippy::too_many_arguments)]
    pub fn render_debug(
        &self,
        instructions: &mut Vec<DebugRectangleInstruction>,
        camera: &dyn Camera,
        entity_position: Point3<f32>,
        animation_state: &AnimationState,
        direction: Direction,
        color_external: Color,
        color_internal: Color,
        scale: f32,
    ) {
        let frame = self.get_frame(animation_state, camera, direction);
        let world_matrix = self.calculate_world_matrix(camera, &frame, entity_position, scale);
        instructions.push(DebugRectangleInstruction {
            world: world_matrix,
            color: color_external,
        });
        instructions.push(DebugRectangleInstruction {
            world: world_matrix * frame.horizontal_matrix,
            color: color_external,
        });
        instructions.push(DebugRectangleInstruction {
            world: world_matrix * frame.vertical_matrix,
            color: color_external,
        });

        for frame_part in frame.frame_parts.iter() {
            instructions.push(DebugRectangleInstruction {
                world: world_matrix * frame_part.affine_matrix,
                color: color_internal,
            });
        }
    }
}

/// Reproduce CActRes::get_motion for a valid action: return the requested
/// motion when present, motion zero when the index is out of range, and no
/// motion for an empty action.
pub(crate) fn native_layer_motion_index(motion_index: usize, layer_length: usize) -> Option<usize> {
    if layer_length == 0 {
        return None;
    }

    Some(if motion_index < layer_length { motion_index } else { 0 })
}

fn empty_animation_frame() -> AnimationFrame {
    AnimationFrame {
        event: None,
        attach_point: None,
        size: Vector2::new(1, 1),
        top_left: Vector2::zero(),
        offset: Vector2::zero(),
        frame_parts: Vec::new(),
        #[cfg(feature = "debug")]
        horizontal_matrix: Matrix4::identity(),
        #[cfg(feature = "debug")]
        vertical_matrix: Matrix4::identity(),
    }
}

/// Align a secondary layer's geometry to the body attach point.
/// `delta = -child_attach + body_attach` matches the classic head↔body rule.
pub(crate) fn apply_child_attach(frame: &mut AnimationFrame, body_attach: Option<Vector2<i32>>) {
    let Some(child_attach) = frame.attach_point else {
        return;
    };
    let Some(parent_attach) = body_attach else {
        return;
    };
    let delta = -child_attach + parent_attach;
    frame.offset += delta;
    for part in frame.frame_parts.iter_mut() {
        part.offset += delta;
    }
}

/// Merge layer frames into one AABB + part list. Events: first non-None wins
/// (callers strip secondary-layer events first).
pub(crate) fn merge_frame(frames: &mut [AnimationFrame]) -> AnimationFrame {
    for frame in frames.iter_mut() {
        let half_size = (frame.size - Vector2::new(1, 1)) / 2;
        frame.top_left = frame.offset - half_size;
    }

    if frames.is_empty() {
        return empty_animation_frame();
    }

    let top_left_x = frames.iter().min_by_key(|frame| frame.top_left.x).unwrap().top_left.x;
    let top_left_y = frames.iter().min_by_key(|frame| frame.top_left.y).unwrap().top_left.y;
    let frame_x = frames.iter().max_by_key(|frame| frame.top_left.x + frame.size.x).unwrap();
    let frame_y = frames.iter().max_by_key(|frame| frame.top_left.y + frame.size.y).unwrap();
    let new_width = (frame_x.top_left.x + frame_x.size.x) - top_left_x;
    let new_height = (frame_y.top_left.y + frame_y.size.y) - top_left_y;

    let mut new_frame_parts = Vec::with_capacity(frames.iter().map(|frame| frame.frame_parts.len()).sum());
    for frame in frames.iter_mut() {
        new_frame_parts.append(&mut frame.frame_parts);
    }
    let event = frames.iter().filter_map(|frame| frame.event).next();
    // Composed frames no longer need a single attach; parenting was applied
    // per layer before the merge.
    let attach_point = None;

    AnimationFrame {
        event,
        attach_point,
        size: Vector2::new(new_width, new_height),
        top_left: Vector2::zero(),
        offset: Vector2::new(top_left_x + (new_width - 1) / 2, top_left_y + (new_height - 1) / 2),
        frame_parts: new_frame_parts,
        #[cfg(feature = "debug")]
        horizontal_matrix: Matrix4::identity(),
        #[cfg(feature = "debug")]
        vertical_matrix: Matrix4::identity(),
    }
}

/// Measure the union AABB of a composed frame into an accumulating layout.
pub(crate) fn accumulate_action_layout(layout: &mut ActionLayout, frame: &AnimationFrame) {
    let center_x = (frame.size.x - 1) / 2;
    let center_y = (frame.size.y - 1) / 2;
    layout.min_left = layout.min_left.min(frame.offset.x - center_x);
    layout.max_right = layout.max_right.max(frame.offset.x + (frame.size.x - 1 - center_x));
    layout.min_top = layout.min_top.min(frame.offset.y - center_y);
    layout.max_bottom = layout.max_bottom.max(frame.offset.y + (frame.size.y - 1 - center_y));
}

pub(crate) fn empty_action_layout() -> ActionLayout {
    ActionLayout {
        min_top: i32::MAX,
        max_bottom: 0,
        min_left: i32::MAX,
        max_right: 0,
    }
}

fn vector2_i32_to_f32(vector: Vector2<i32>) -> Vector2<f32> {
    vector.map(|value| value as f32)
}

fn calculate_new_size(min_top: i32, max_bottom: i32, min_left: i32, max_right: i32) -> Vector2<i32> {
    let size_x = 2 * i32::abs(min_left).max(i32::abs(max_right)) + 1;
    let mut padding = 1;
    if (max_bottom - min_top) % 2 == 0 {
        padding = 0;
    }
    let size_y = max_bottom - min_top + 1 + padding;
    Vector2::new(size_x, size_y)
}

fn convert_coordinates(coordinates: Vector2<f32>, size: Vector2<f32>) -> Vector2<f32> {
    assert!(size.x != 0.0 && size.y != 0.0);
    let x = (coordinates.x / size.x - 1.0 / 2.0) * 2.0;
    let y = 2.0 - (coordinates.y / size.y) * 2.0;
    Vector2::<f32>::new(x, y)
}

fn calculate_recenter_rotation_matrix(texture_frame_center: Vector2<f32>, angle: f32) -> Matrix4<f32> {
    use cgmath::Rad;
    let translation_to_center_matrix = Matrix4::from_translation(texture_frame_center.extend(0.0));
    let rotation_matrix = Matrix4::from_angle_z(Rad(angle));
    let translation_to_origin_matrix = Matrix4::from_translation((-texture_frame_center).extend(0.0));
    translation_to_center_matrix * rotation_matrix * translation_to_origin_matrix
}

fn calculate_scale_matrix(
    texture_top_left: Vector2<f32>,
    texture_bottom_left: Vector2<f32>,
    texture_bottom_right: Vector2<f32>,
) -> Matrix4<f32> {
    Matrix4::from_nonuniform_scale(
        (texture_bottom_right.x - texture_bottom_left.x) / 2.0,
        (texture_top_left.y - texture_bottom_left.y) / 2.0,
        1.0,
    )
}

fn calculate_translation_matrix(
    texture_top_left: Vector2<f32>,
    texture_bottom_left: Vector2<f32>,
    texture_bottom_right: Vector2<f32>,
) -> Matrix4<f32> {
    let texture_center = (texture_top_left + texture_bottom_right) / 2.0;
    let texture_frame_new_center = Vector2::new(0.0, (texture_top_left.y - texture_bottom_left.y) / 2.0);
    Matrix4::from_translation(texture_center.extend(1.0) - texture_frame_new_center.extend(1.0))
}

/// Apply the action-global layout and bake per-part affine matrices.
pub(crate) fn finalize_frame_layout(frame: &mut AnimationFrame, layout: ActionLayout) {
    let min_top = if layout.min_top == i32::MAX { 0 } else { layout.min_top };
    let min_left = if layout.min_left == i32::MAX { 0 } else { layout.min_left };
    let max_bottom = layout.max_bottom;
    let max_right = layout.max_right;

    frame.size = calculate_new_size(min_top, max_bottom, min_left, max_right);
    frame.offset = Vector2::new(0, -max_bottom);

    #[cfg(feature = "debug")]
    {
        let frame_size = vector2_i32_to_f32(frame.size);
        let frame_top_left = Vector2::new(-frame_size.x / 2.0, -frame_size.y + 0.5);
        let frame_origin = vector2_i32_to_f32(frame.offset);
        let bounding_box_origin = frame_origin - frame_top_left;

        let texture_top_left = convert_coordinates(Vector2::new(0.0, bounding_box_origin.y), frame_size);
        let texture_bottom_left = convert_coordinates(Vector2::new(0.0, bounding_box_origin.y), frame_size);
        let texture_bottom_right = convert_coordinates(Vector2::new(frame_size.x, bounding_box_origin.y), frame_size);
        let scale_matrix = calculate_scale_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
        let translation_matrix = calculate_translation_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
        frame.horizontal_matrix = translation_matrix * scale_matrix;

        let texture_top_left = convert_coordinates(Vector2::new(bounding_box_origin.x, 0.0), frame_size);
        let texture_bottom_left = convert_coordinates(Vector2::new(bounding_box_origin.x, frame_size.y), frame_size);
        let texture_bottom_right = convert_coordinates(Vector2::new(bounding_box_origin.x, frame_size.y), frame_size);
        let scale_matrix = calculate_scale_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
        let translation_matrix = calculate_translation_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
        frame.vertical_matrix = translation_matrix * scale_matrix;
    }

    for frame_part in frame.frame_parts.iter_mut() {
        let frame_size = vector2_i32_to_f32(frame.size);
        let frame_origin = vector2_i32_to_f32(frame.offset);
        let frame_top_left = Vector2::new(-frame_size.x / 2.0, -frame_size.y + 0.5);
        let frame_part_top_left_shift = vector2_i32_to_f32(-((frame_part.size - Vector2::new(1, 1)) / 2));
        let frame_part_offset = vector2_i32_to_f32(frame_part.offset);
        let frame_part_top_left =
            (frame_origin + frame_part_offset + frame_part_top_left_shift) - Vector2::<f32>::from_value(0.5);

        let texture_frame_center = Vector2::new(0.0, 1.0);
        let new_vector = vector2_i32_to_f32(frame_part.size);
        let top_left = frame_part_top_left - frame_top_left;
        let bottom_left = top_left + new_vector.y * Vector2::<f32>::unit_y();
        let bottom_right = top_left + new_vector;

        let texture_top_left = convert_coordinates(top_left, frame_size);
        let texture_bottom_left = convert_coordinates(bottom_left, frame_size);
        let texture_bottom_right = convert_coordinates(bottom_right, frame_size);

        let rotation_matrix = calculate_recenter_rotation_matrix(texture_frame_center, -frame_part.angle);
        let scale_matrix = calculate_scale_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
        let translation_matrix = calculate_translation_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
        frame_part.affine_matrix = translation_matrix * scale_matrix * rotation_matrix;
    }
}

/// Build per-action layouts by composing every body motion once (load-time).
pub(crate) fn compute_action_layouts(layers: &[AnimationLayer]) -> Vec<ActionLayout> {
    let Some(body) = layers.first() else {
        return Vec::new();
    };

    let mut layouts = Vec::with_capacity(body.animations.len());
    for action_index in 0..body.animations.len() {
        let body_frames = &body.animations[action_index].frames;
        let mut layout = empty_action_layout();
        for body_motion in 0..body_frames.len() {
            let body_attach = body_frames[body_motion].attach_point;
            let mut layer_frames = Vec::with_capacity(layers.len());
            for (layer_index, layer) in layers.iter().enumerate() {
                let Some(layer_animation) = layer.animations.get(action_index) else {
                    continue;
                };
                let Some(layer_motion) = native_layer_motion_index(body_motion, layer_animation.frames.len()) else {
                    continue;
                };
                let mut frame = layer_animation.frames[layer_motion].clone();
                if layer_index != 0 {
                    frame.event = None;
                    apply_child_attach(&mut frame, body_attach);
                }
                layer_frames.push(frame);
            }
            let composed = merge_frame(&mut layer_frames);
            accumulate_action_layout(&mut layout, &composed);
        }
        if layout.min_top == i32::MAX {
            layout = ActionLayout {
                min_top: 0,
                max_bottom: 0,
                min_left: 0,
                max_right: 0,
            };
        }
        layouts.push(layout);
    }
    layouts
}

#[cfg(test)]
mod weapon_action_tests {
    use korangar_loaders::FileLoader;
    use mlua::Lua;
    use ragnarok_packets::{ClientTick, JobId, Sex, SkillId};

    use super::{
        AnimationActionType, AnimationData, AnimationState, TRICK_DEAD_POSE_STATE, animation_frame_position, is_animation_complete,
        native_classic_player_state2_action, native_classic_player_state7_action_base_offset,
        native_classic_player_state8_action_base_offset, native_player_attack_action, native_player_attack_event_position,
        native_player_job, native_real_weapon_id, native_shared_attack_event_position,
    };
    use crate::EntityType;
    use crate::loaders::GameFileLoader;

    #[test]
    fn natural_act_clock_uses_24_milliseconds_per_delay_unit() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.time = 95;
        assert_eq!(animation_frame_position(&state, 4.0, 5), 0);
        state.time = 96;
        assert_eq!(animation_frame_position(&state, 4.0, 5), 1);
    }

    #[test]
    fn native_player_job_predicate_has_exact_boundaries() {
        for job_id in [0, 30, 4001, 5999] {
            assert!(native_player_job(JobId(job_id)));
        }
        for job_id in [31, 4000, 6000, 10000] {
            assert!(!native_player_job(JobId(job_id)));
        }
    }

    #[test]
    fn short_player_hurt_scales_the_body_act_delay() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.hurt(EntityType::Player, JobId(7), 144, ClientTick(0));
        state.time = 47;
        assert_eq!(animation_frame_position(&state, 4.0, 3), 0);
        state.time = 48;
        assert_eq!(animation_frame_position(&state, 4.0, 3), 1);
    }

    #[test]
    fn short_monster_hurt_uses_the_normalized_delay_directly() {
        let mut state = AnimationState::new(EntityType::Monster, ClientTick(0));
        state.hurt(EntityType::Monster, JobId(1002), 144, ClientTick(0));
        state.time = 11;
        assert_eq!(animation_frame_position(&state, 9.0, 3), 0);
        state.time = 12;
        assert_eq!(animation_frame_position(&state, 9.0, 3), 1);
    }

    #[test]
    fn long_hurt_holds_the_last_motion_until_the_cycle_threshold() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.hurt(EntityType::Player, JobId(7), 576, ClientTick(0));
        state.time = 288;
        assert_eq!(animation_frame_position(&state, 4.0, 3), 3);
        assert!(!is_animation_complete(&state, 4.0, 3));
        state.time = 575;
        assert!(!is_animation_complete(&state, 4.0, 3));
        state.time = 576;
        assert!(is_animation_complete(&state, 4.0, 3));
    }

    #[test]
    fn knight_spear_uses_thrust_action() {
        assert_eq!(
            native_player_attack_action(JobId(7), Sex::Male, 4),
            AnimationActionType::Attack3
        );
    }

    #[test]
    fn knight_sword_uses_second_action() {
        assert_eq!(
            native_player_attack_action(JobId(7), Sex::Male, 2),
            AnimationActionType::Attack2
        );
    }

    #[test]
    fn rune_knight_inherits_knight_spear_row() {
        assert_eq!(
            native_player_attack_action(JobId(4054), Sex::Female, 5),
            AnimationActionType::Attack3
        );
    }

    #[test]
    fn archer_bow_uses_second_action() {
        assert_eq!(
            native_player_attack_action(JobId(3), Sex::Female, 11),
            AnimationActionType::Attack2
        );
    }

    #[test]
    fn hunter_bow_uses_third_action() {
        assert_eq!(
            native_player_attack_action(JobId(11), Sex::Male, 11),
            AnimationActionType::Attack3
        );
    }

    #[test]
    fn assassin_cross_katar_uses_third_action() {
        assert_eq!(
            native_player_attack_action(JobId(4013), Sex::Female, 16),
            AnimationActionType::Attack3
        );
    }

    #[test]
    fn wizard_rod_action_mirrors_by_sex() {
        assert_eq!(
            native_player_attack_action(JobId(9), Sex::Female, 10),
            AnimationActionType::Attack3
        );
        assert_eq!(
            native_player_attack_action(JobId(9), Sex::Male, 10),
            AnimationActionType::Attack2
        );
    }

    #[test]
    fn bare_hands_use_second_action() {
        assert_eq!(
            native_player_attack_action(JobId(7), Sex::Male, 0),
            AnimationActionType::Attack2
        );
    }

    #[test]
    fn dancer_dagger_uses_second_action() {
        assert_eq!(
            native_player_attack_action(JobId(20), Sex::Female, 1),
            AnimationActionType::Attack2
        );
    }

    #[test]
    fn gunslinger_bare_hands_use_second_action() {
        assert_eq!(
            native_player_attack_action(JobId(24), Sex::Male, 0),
            AnimationActionType::Attack2
        );
    }

    #[test]
    fn expansion_weapon_ids_are_normalized_before_selection() {
        assert_eq!(native_real_weapon_id(31), 1);
        assert_eq!(native_real_weapon_id(73), 11);
        assert_eq!(native_real_weapon_id(98), 8);
        assert_eq!(native_real_weapon_id(103), 103);
        assert_eq!(
            native_player_attack_action(JobId(3), Sex::Female, 73),
            AnimationActionType::Attack2
        );
    }

    #[test]
    fn selector_never_uses_player_attack_one() {
        for job_id in (0..=25).chain(4001..=4242).map(JobId) {
            for sex in [Sex::Female, Sex::Male] {
                for weapon in 0..=102 {
                    assert_ne!(
                        native_player_attack_action(job_id, sex, weapon),
                        AnimationActionType::Attack1,
                        "job={} sex={sex:?} weapon={weapon}",
                        job_id.0
                    );
                }
            }
        }
    }

    #[test]
    fn native_attack_event_positions_are_not_speed_multipliers() {
        assert_eq!(native_player_attack_event_position(JobId(12), Sex::Female, 16, true), 3.0);
        assert_eq!(native_player_attack_event_position(JobId(0), Sex::Male, 0, true), 5.85);
        assert_eq!(native_player_attack_event_position(JobId(5), Sex::Male, 0, false), 5.85);
        assert_eq!(native_player_attack_event_position(JobId(6), Sex::Female, 0, false), 5.75);
        assert_eq!(native_player_attack_event_position(JobId(7), Sex::Male, 4, true), 6.0);
    }

    #[test]
    fn spear_boomerang_uses_shared_state_12_attack_one() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(10));

        assert!(state.skill_attack(EntityType::Player, JobId(7), Sex::Male, 4, SkillId(59), false, ClientTick(20),));
        assert_eq!(state.native_actor_state, 12);
        assert_eq!(state.action_base_offset, 5);
        assert_eq!(state.action_type, AnimationActionType::Attack1);
        assert_eq!(state.impact_event_position_override, None);
    }

    #[test]
    fn player_states_two_and_nine_share_the_weapon_selector() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));

        assert!(state.skill_attack(EntityType::Player, JobId(7), Sex::Male, 4, SkillId(5), false, ClientTick(10),));
        assert_eq!(state.native_actor_state, 2);
        assert_eq!(state.action_base_offset, 11);
        assert_eq!(state.impact_event_position_override, Some(6.0));

        assert!(state.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(316), false, ClientTick(20),));
        assert_eq!(state.native_actor_state, 9);
        assert_eq!(state.action_base_offset, 10);
        assert_eq!(state.impact_event_position_override, Some(6.0));
    }

    #[test]
    fn default_state_seven_applies_player_job_routes() {
        let mut standard = AnimationState::new(EntityType::Player, ClientTick(0));
        assert!(standard.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(14), false, ClientTick(10),));
        assert_eq!(standard.action_base_offset, 12);
        assert_eq!(standard.action_type, AnimationActionType::Skill);
        assert_eq!(standard.impact_event_position_override, Some(6.0));

        let mut crusader = AnimationState::new(EntityType::Player, ClientTick(0));
        assert!(crusader.skill_attack(
            EntityType::Player,
            JobId(14),
            Sex::Female,
            2,
            SkillId(14),
            false,
            ClientTick(20),
        ));
        assert_eq!(crusader.action_base_offset, 4);
        assert_eq!(crusader.action_type, AnimationActionType::ReadyFight);
        assert!(!crusader.looping);

        let mut monk = AnimationState::new(EntityType::Player, ClientTick(0));
        assert!(monk.skill_attack(EntityType::Player, JobId(15), Sex::Male, 0, SkillId(14), false, ClientTick(30),));
        assert_eq!(monk.action_base_offset, 12);
        assert_eq!(animation_frame_position(&monk, 4.0, 8), 0);
        monk.update(ClientTick(429));
        assert!(!is_animation_complete(&monk, 4.0, 8));
        monk.update(ClientTick(430));
        assert!(is_animation_complete(&monk, 4.0, 8));
    }

    #[test]
    fn classic_player_routes_replace_the_granny_model_intercept() {
        assert_eq!(native_classic_player_state7_action_base_offset(JobId(4020)), 12);
        assert_eq!(native_classic_player_state7_action_base_offset(JobId(4047)), 4);
        assert_eq!(native_classic_player_state7_action_base_offset(JobId(4219)), 10);
        assert_eq!(native_classic_player_state8_action_base_offset(JobId(4218)), 5);
        assert_eq!(native_classic_player_state8_action_base_offset(JobId(4219)), 12);
    }

    #[test]
    fn classic_state_two_random_override_is_job_scoped() {
        assert_eq!(
            native_classic_player_state2_action(JobId(4047), AnimationActionType::Attack2, 0),
            AnimationActionType::Attack3
        );
        assert_eq!(
            native_classic_player_state2_action(JobId(4047), AnimationActionType::Attack3, 6),
            AnimationActionType::Attack3
        );
        assert_eq!(
            native_classic_player_state2_action(JobId(4047), AnimationActionType::Attack3, 7),
            AnimationActionType::Attack2
        );
        assert_eq!(
            native_classic_player_state2_action(JobId(7), AnimationActionType::Attack2, 0),
            AnimationActionType::Attack2
        );
    }

    #[test]
    fn completion_exits_follow_native_state_instead_of_action_semantics() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.weapon_attack_for_state(EntityType::Player, JobId(7), Sex::Male, 2, 2, 9, ClientTick(10));
        state.apply_completion_transition(EntityType::Player, false, ClientTick(20));
        assert_eq!(state.native_actor_state, 0);
        assert_eq!(state.action_base_offset, 4, "player state 2 returns through CPc ReadyFight");
        assert!(state.looping);

        state.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(59), false, ClientTick(30));
        state.apply_completion_transition(EntityType::Player, false, ClientTick(40));
        assert_eq!(state.native_actor_state, 0);
        assert_eq!(state.action_base_offset, 0, "shared state 12 returns to Idle");

        state.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(2327), false, ClientTick(50));
        state.apply_completion_transition(EntityType::Player, false, ClientTick(60));
        assert_eq!(
            state.native_actor_state, 18,
            "state 18 holds even though its ACT group looks like an attack"
        );

        state.set_native_action(EntityType::Player, 12, 8, false, ClientTick(70));
        state.idle(EntityType::Player, false, ClientTick(80));
        assert_eq!(state.native_actor_state, 0);
        assert_eq!(
            state.action_base_offset, 4,
            "an explicit CPc state-0 request remembers prior state 8"
        );
    }

    #[test]
    fn normalized_state_49_and_two_stage_state_51_use_native_exits() {
        let mut state49 = AnimationState::new(EntityType::Monster, ClientTick(0));
        state49.skill_attack(
            EntityType::Monster,
            JobId(1002),
            Sex::Male,
            0,
            SkillId(5021),
            false,
            ClientTick(10),
        );
        assert_eq!(state49.native_actor_state, 7);
        assert_eq!(state49.current_motion_override(), Some(0));
        state49.update(ClientTick(1_010));
        assert!(is_animation_complete(&state49, 4.0, 8));
        state49.apply_completion_transition(EntityType::Monster, false, ClientTick(1_010));
        assert_eq!(state49.native_actor_state, 0);
        assert_eq!(state49.action_base_offset, 0);

        let mut state51 = AnimationState::new(EntityType::Player, ClientTick(0));
        state51.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(2580), false, ClientTick(20));
        state51.update(ClientTick(520));
        assert!(is_animation_complete(&state51, 4.0, 8));
        state51.apply_completion_transition(EntityType::Player, false, ClientTick(520));
        assert_eq!(state51.native_actor_state, 2);
        state51.apply_completion_transition(EntityType::Player, false, ClientTick(536));
        assert_eq!(state51.native_actor_state, 0);
        assert_eq!(state51.action_base_offset, 4);
    }

    #[test]
    fn state_seven_and_no_action_guards_preserve_current_action() {
        let mut sitting = AnimationState::new(EntityType::Player, ClientTick(0));
        sitting.sit(EntityType::Player, ClientTick(10));
        assert!(!sitting.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(14), false, ClientTick(20),));
        assert_eq!(sitting.native_actor_state, 6);
        assert_eq!(sitting.action_base_offset, 2);
        assert_eq!(sitting.start_time.0, 10);

        let mut attacking = AnimationState::new(EntityType::Player, ClientTick(0));
        attacking.weapon_attack(EntityType::Player, JobId(7), Sex::Male, 4, ClientTick(30));
        assert!(!attacking.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(446), false, ClientTick(40),));
        assert_eq!(attacking.native_actor_state, 2);
        assert_eq!(attacking.action_base_offset, 11);
        assert_eq!(attacking.start_time.0, 30);
        assert_eq!(attacking.impact_event_position_override, Some(6.0));
    }

    #[test]
    fn su_hide_guard_blocks_only_native_state_seven() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        assert!(!state.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(14), true, ClientTick(10),));
        assert_eq!(state.native_actor_state, 0);

        assert!(state.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(5), true, ClientTick(20),));
        assert_eq!(state.native_actor_state, 2);
    }

    #[test]
    fn pk_mode_keeps_neutral_player_in_ready_fight() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.idle(EntityType::Player, true, ClientTick(10));
        assert_eq!(state.native_actor_state, 0);
        assert_eq!(state.action_base_offset, 4);

        state.idle(EntityType::Player, false, ClientTick(20));
        assert_eq!(state.action_base_offset, 0);
    }

    #[test]
    fn stone_and_freeze_pause_then_resume_the_selected_act_clock() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.walk(EntityType::Player, 150, ClientTick(0));
        state.set_status_paused(true, ClientTick(100));
        state.update(ClientTick(500));
        assert_eq!(state.time, 100);

        state.set_status_paused(false, ClientTick(500));
        state.update(ClientTick(550));
        assert_eq!(state.time, 150);
    }

    #[test]
    fn trick_dead_pose_is_not_real_death_and_does_not_auto_exit() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.trick_dead(EntityType::Player, ClientTick(10));
        assert_eq!(state.action_base_offset, 8);
        assert!(!state.is_dead());
        state.apply_completion_transition(EntityType::Player, false, ClientTick(1_000));
        assert_eq!(state.native_actor_state, TRICK_DEAD_POSE_STATE);
    }

    #[test]
    fn doram_statuses_install_their_recovered_persistent_programs() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.status_pose(EntityType::Player, JobId(4218), 47, ClientTick(10));
        assert_eq!(state.native_actor_state, 47);
        assert_eq!(state.action_base_offset, 12);
        assert!(state.motion_program.is_some());

        state.status_pose(EntityType::Player, JobId(4218), 48, ClientTick(20));
        assert_eq!(state.native_actor_state, 48);
        assert_eq!(state.current_motion_override(), Some(4));
    }

    #[test]
    fn monster_state_seven_keeps_shared_flat_group_two() {
        let mut state = AnimationState::new(EntityType::Monster, ClientTick(0));
        assert!(state.skill_attack(
            EntityType::Monster,
            JobId(1002),
            Sex::Male,
            0,
            SkillId(14),
            false,
            ClientTick(10),
        ));
        assert_eq!(state.native_actor_state, 7);
        assert_eq!(state.action_base_offset, 2);
        assert_eq!(state.action_type, AnimationActionType::Attack1);
        assert!(!state.looping);
    }

    #[test]
    fn state_26_uses_job_specific_attack_group() {
        let mut linker = AnimationState::new(EntityType::Player, ClientTick(0));
        assert!(linker.skill_attack(
            EntityType::Player,
            JobId(4049),
            Sex::Female,
            10,
            SkillId(419),
            false,
            ClientTick(10),
        ));
        assert_eq!(linker.action_base_offset, 11);

        let mut knight = AnimationState::new(EntityType::Player, ClientTick(0));
        assert!(knight.skill_attack(EntityType::Player, JobId(7), Sex::Male, 2, SkillId(419), false, ClientTick(10),));
        assert_eq!(knight.action_base_offset, 5);
    }

    #[test]
    fn impact_offset_multiplies_event_position_by_act_delay_and_24ms() {
        let animation_data = AnimationData {
            layers: Vec::new(),
            delays: vec![4.0, 5.0],
            action_layouts: Vec::new(),
            entity_type: EntityType::Player,
        };
        let state = AnimationState::new(EntityType::Player, ClientTick(0));

        assert_eq!(animation_data.attack_impact_delay_ms(&state, 0, Some(6.0)), 576);
        assert_eq!(animation_data.attack_impact_delay_ms(&state, 0, Some(5.85)), 562);
        assert_eq!(animation_data.attack_impact_delay_ms(&state, 1, Some(6.0)), 720);
    }

    #[test]
    fn shared_attack_event_uses_authored_marker_or_motion_count_minus_two() {
        assert_eq!(native_shared_attack_event_position(9, Some(4)), 4.0);
        assert_eq!(native_shared_attack_event_position(9, None), 7.0);
        assert_eq!(native_shared_attack_event_position(1, None), 0.0);
        assert_eq!(native_shared_attack_event_position(0, None), 0.0);
    }

    #[test]
    #[ignore = "requires the configured reference-client archives"]
    fn reports_reference_weapon_id_normalization() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();
        game_file_loader.load_patched_lua_files();

        let lua = Lua::new();
        for path in [
            "data\\luafiles514\\lua files\\datainfo\\weapontable.lub",
            "data\\luafiles514\\lua files\\datainfo\\weapontable_f.lub",
        ] {
            if let Ok(data) = game_file_loader.get(path) {
                lua.load(&data)
                    .exec()
                    .unwrap_or_else(|error| panic!("{path} must execute: {error}"));
            }
        }
        let normalize = lua
            .globals()
            .get::<mlua::Function>("GetRealWeaponId")
            .expect("WeaponTable must define GetRealWeaponId");

        for weapon_id in 0..=512u32 {
            let normalized = normalize.call::<u32>(weapon_id).expect("normalization must return an integer");
            assert_eq!(native_real_weapon_id(weapon_id), normalized, "weapon {weapon_id}");
        }
    }
}

/// Golden timeline tests for the animation-fidelity plan (phase A): chain the
/// pieces a damage packet drives — source action selection, the ACT-derived
/// impact boundary, `PendingImpactQueue`, and the target Hurt clock — through
/// synthetic ACT data and assert the exact millisecond boundaries.
#[cfg(test)]
mod golden_timeline_tests {
    use cgmath::Vector2;
    #[cfg(feature = "debug")]
    use cgmath::{Matrix4, Zero};
    use ragnarok_packets::{ClientTick, EntityId, JobId, Sex, SkillId};

    use super::{
        Animation, AnimationActionType, AnimationData, AnimationFrame, AnimationState, animation_frame_position, is_animation_complete,
    };
    use crate::EntityType;
    use crate::world::ActionEvent;
    use crate::world::impact::{DamageImpact, PendingImpactQueue};

    fn frame(event: Option<ActionEvent>) -> AnimationFrame {
        AnimationFrame {
            event,
            attach_point: None,
            offset: Vector2::new(0, 0),
            top_left: Vector2::new(0, 0),
            size: Vector2::new(0, 0),
            frame_parts: Vec::new(),
            #[cfg(feature = "debug")]
            horizontal_matrix: Matrix4::zero(),
            #[cfg(feature = "debug")]
            vertical_matrix: Matrix4::zero(),
        }
    }

    /// One synthetic action reused for every action index (modulo indexing),
    /// mirroring one direction of a body ACT with `frame_count` motions.
    fn animation_data(entity_type: EntityType, delay: f32, frame_count: usize, attack_event_index: Option<usize>) -> AnimationData {
        use super::{ActionLayout, AnimationLayer};

        let frames = (0..frame_count)
            .map(|index| frame((attack_event_index == Some(index)).then_some(ActionEvent::Attack)))
            .collect();
        AnimationData {
            layers: vec![AnimationLayer {
                path_key: None,
                sprites: None,
                actions: None,
                animations: vec![Animation { frames }],
            }],
            delays: vec![delay],
            action_layouts: vec![ActionLayout {
                min_top: 0,
                max_bottom: 0,
                min_left: 0,
                max_right: 0,
            }],
            entity_type,
        }
    }

    fn damage(target: u32, damage_delay: u32) -> DamageImpact {
        DamageImpact {
            source_entity_id: EntityId(2000000),
            destination_entity_id: EntityId(target),
            skill_id: None,
            packet_tick: ClientTick(5000),
            damage_amount: Some(120),
            hit_count: 1,
            damage_delay,
            is_critical: false,
        }
    }

    /// A Knight spear basic attack (`ZC_NOTIFY_ACT` type Damage): the source
    /// plays Attack3 on its natural ACT clock, the impact boundary is the
    /// exact `0x00991580` marker (6.0) × delay × 24 ms, and a `dMotion` of
    /// exactly one reaction cycle plays the target Hurt at natural speed.
    #[test]
    fn knight_spear_attack_timeline_hits_native_boundaries() {
        let packet_arrival = ClientTick(10_000);
        let mut source_state = AnimationState::new(EntityType::Player, packet_arrival);
        source_state.weapon_attack(EntityType::Player, JobId(7), Sex::Male, 4, packet_arrival);
        assert_eq!(source_state.action_type, AnimationActionType::Attack3);
        assert_eq!(source_state.impact_event_position_override(), Some(6.0));

        // The Knight body ACT delay for Attack3 is 4.0 in the classic data.
        let source_data = animation_data(EntityType::Player, 4.0, 8, None);
        let impact_delay = source_data.attack_impact_delay_ms(&source_state, 0, source_state.impact_event_position_override());
        assert_eq!(impact_delay, 576, "6.0 × 4.0 × 24 ms");

        let mut queue = PendingImpactQueue::default();
        queue.schedule(packet_arrival, impact_delay, damage(110000001, 288));
        assert!(queue.drain_due(ClientTick(10_575)).is_empty(), "one tick before the boundary");
        let due = queue.drain_due(ClientTick(10_576));
        assert_eq!(due.len(), 1, "due exactly at the ACT-derived boundary");

        // Hurt begins at the due boundary, not at packet receipt. dMotion 288
        // is exactly one reaction cycle: natural ACT speed (96 ms per motion),
        // complete after frame_count × 96 ms.
        let due_tick = ClientTick(10_576);
        let mut target_state = AnimationState::new(EntityType::Player, due_tick);
        target_state.hurt(EntityType::Player, JobId(0), due[0].damage.damage_delay, due_tick);

        target_state.time = 95;
        assert_eq!(animation_frame_position(&target_state, 4.0, 3), 0);
        target_state.time = 96;
        assert_eq!(animation_frame_position(&target_state, 4.0, 3), 1);
        target_state.time = 287;
        assert!(!is_animation_complete(&target_state, 4.0, 3));
        target_state.time = 288;
        assert!(is_animation_complete(&target_state, 4.0, 3));
    }

    /// A monster source action uses the shared actor path: an authored `"atk"`
    /// event owns the impact boundary; without one the boundary falls back to
    /// `motion_count - 2` exactly like Ragexe `0x008A9500`.
    #[test]
    fn monster_impact_boundary_uses_atk_event_with_native_fallback() {
        let packet_arrival = ClientTick(20_000);
        let mut source_state = AnimationState::new(EntityType::Monster, packet_arrival);
        // Unmapped skill IDs resolve to native default state 7; a non-player
        // actor keeps native flat group 2 (the monster attack action).
        let animated = source_state.skill_attack(
            EntityType::Monster,
            JobId(1002),
            Sex::Male,
            0,
            SkillId(9999),
            false,
            packet_arrival,
        );
        assert!(animated);
        assert_eq!(source_state.impact_event_position_override(), None);

        let authored = animation_data(EntityType::Monster, 5.0, 6, Some(3));
        assert_eq!(
            authored.attack_impact_delay_ms(&source_state, 0, None),
            360,
            "authored atk event at motion 3: 3 × 5.0 × 24 ms"
        );

        let unauthored = animation_data(EntityType::Monster, 5.0, 6, None);
        assert_eq!(
            unauthored.attack_impact_delay_ms(&source_state, 0, None),
            480,
            "fallback motion_count - 2: 4 × 5.0 × 24 ms"
        );
    }

    /// Spear Boomerang (skill 59) is proven native state 12 → flat Attack1;
    /// its former throwing/Attack2 override was wrong. Pin the corrected
    /// resolution end-to-end through `skill_attack`.
    #[test]
    fn spear_boomerang_resolves_state_12_attack1() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        let animated = state.skill_attack(EntityType::Player, JobId(7), Sex::Male, 4, SkillId(59), false, ClientTick(0));
        assert!(animated);
        assert_eq!(state.action_type, AnimationActionType::Attack1);
        assert!(state.motion_program.is_none(), "state 12 installs no higher motion program");
    }
}

/// Phase C1: runtime layer composition — body clock + secondary motion-0 fallback.
#[cfg(test)]
mod runtime_compose_tests {
    use cgmath::{Vector2, Zero};
    #[cfg(feature = "debug")]
    use cgmath::Matrix4;
    use ragnarok_packets::ClientTick;

    use super::{
        ActionLayout, Animation, AnimationData, AnimationFrame, AnimationFramePart, AnimationLayer, AnimationState,
        is_weapon_part_path, native_layer_motion_index, shield_draws_behind_body, shield_path_from_entity_parts,
        weapon_path_from_entity_parts,
    };
    use crate::EntityType;
    use crate::graphics::Color;

    fn part(layer: usize) -> AnimationFramePart {
        AnimationFramePart {
            animation_index: layer,
            sprite_number: 0,
            offset: Vector2::new(0, 0),
            size: Vector2::new(2, 2),
            mirror: false,
            angle: 0.0,
            color: Color {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 1.0,
            },
            affine_matrix: cgmath::Matrix4::from_scale(1.0),
        }
    }

    fn layer_frame(layer: usize) -> AnimationFrame {
        AnimationFrame {
            event: None,
            attach_point: None,
            offset: Vector2::new(0, 0),
            top_left: Vector2::zero(),
            size: Vector2::new(2, 2),
            frame_parts: vec![part(layer)],
            #[cfg(feature = "debug")]
            horizontal_matrix: Matrix4::from_scale(1.0),
            #[cfg(feature = "debug")]
            vertical_matrix: Matrix4::from_scale(1.0),
        }
    }

    fn make_layer(layer_index: usize, motion_count: usize) -> AnimationLayer {
        let frames: Vec<_> = (0..motion_count).map(|_| layer_frame(layer_index)).collect();
        AnimationLayer {
            path_key: Some(format!("layer-{layer_index}")),
            sprites: None,
            actions: None,
            animations: vec![Animation { frames }],
        }
    }

    #[test]
    fn compose_uses_body_motion_count_and_motion_zero_fallback() {
        // Body 4 motions, head only 2 — body motion 3 must sample head motion 0.
        assert_eq!(native_layer_motion_index(3, 2), Some(0));
        assert_eq!(native_layer_motion_index(1, 2), Some(1));

        let data = AnimationData {
            layers: vec![make_layer(0, 4), make_layer(1, 2)],
            delays: vec![4.0],
            action_layouts: vec![ActionLayout {
                min_top: 0,
                max_bottom: 1,
                min_left: 0,
                max_right: 1,
            }],
            entity_type: EntityType::Player,
        };

        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.attack(EntityType::Player, false, ClientTick(0));
        // Advance to body motion 3 (4.0 delay × 24 ms = 96 ms per motion).
        state.time = 3 * 96;

        // Attack1 base offset is 5; only one synthetic action exists so % 1 → 0.
        let frame = data.compose_action_motion(&state, 5 * 8, 0);
        assert_eq!(frame.frame_parts.len(), 2, "body + head parts");
        assert_eq!(frame.frame_parts[0].animation_index, 0);
        assert_eq!(frame.frame_parts[1].animation_index, 1);
    }

    #[test]
    fn child_attach_aligns_to_body_attach_point() {
        use super::apply_child_attach;

        // Body attach at (10, 20); head attach at (3, 4) → delta = (7, 16).
        let mut head = layer_frame(1);
        head.attach_point = Some(Vector2::new(3, 4));
        head.offset = Vector2::new(1, 2);
        head.frame_parts[0].offset = Vector2::new(1, 2);

        apply_child_attach(&mut head, Some(Vector2::new(10, 20)));
        assert_eq!(head.offset, Vector2::new(8, 18));
        assert_eq!(head.frame_parts[0].offset, Vector2::new(8, 18));
    }

    #[test]
    fn attach_uses_body_motion_when_secondary_falls_back_to_motion_zero() {
        // Body 3 motions with moving attach; head only 1 motion (always motion 0).
        let mut body_frames = Vec::new();
        for i in 0..3 {
            let mut f = layer_frame(0);
            f.attach_point = Some(Vector2::new(0, i as i32 * 10));
            body_frames.push(f);
        }
        let mut head = layer_frame(1);
        head.attach_point = Some(Vector2::new(0, 0));
        head.offset = Vector2::new(0, 0);
        head.frame_parts[0].offset = Vector2::new(0, 0);

        let data = AnimationData {
            layers: vec![
                AnimationLayer {
                    path_key: Some("body".into()),
                    sprites: None,
                    actions: None,
                    animations: vec![Animation { frames: body_frames }],
                },
                AnimationLayer {
                    path_key: Some("head".into()),
                    sprites: None,
                    actions: None,
                    animations: vec![Animation {
                        frames: vec![head],
                    }],
                },
            ],
            delays: vec![4.0],
            action_layouts: vec![ActionLayout {
                min_top: 0,
                max_bottom: 2,
                min_left: 0,
                max_right: 2,
            }],
            entity_type: EntityType::Player,
        };

        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.attack(EntityType::Player, false, ClientTick(0));
        state.time = 2 * 96; // body motion 2

        let frame = data.compose_action_motion(&state, 5 * 8, 0);
        // Head should be shifted by body attach (0, 20) − head attach (0, 0).
        let head_part = frame.frame_parts.iter().find(|p| p.animation_index == 1).unwrap();
        assert_eq!(head_part.offset.y, 20, "head follows body attach of motion 2");
    }

    #[test]
    fn weapon_layer_swap_preserves_body_and_head_paths() {
        let body = make_layer(0, 2);
        let head = make_layer(1, 2);
        let sword = {
            let mut layer = make_layer(2, 2);
            layer.path_key = Some("sword".into());
            layer
        };
        let spear = {
            let mut layer = make_layer(2, 2);
            layer.path_key = Some("spear".into());
            layer
        };

        let base = AnimationData {
            layers: vec![body, head, sword],
            delays: vec![4.0],
            action_layouts: vec![ActionLayout {
                min_top: 0,
                max_bottom: 1,
                min_left: 0,
                max_right: 1,
            }],
            entity_type: EntityType::Player,
        };
        assert!(base.has_player_base_layers());
        assert_eq!(base.layer_path(0), Some("layer-0"));
        assert_eq!(base.layer_path(2), Some("sword"));

        let swapped = base.clone().with_weapon_layer(Some(spear));
        assert_eq!(swapped.layers.len(), 3);
        assert_eq!(swapped.layer_path(0), Some("layer-0"));
        assert_eq!(swapped.layer_path(1), Some("layer-1"));
        assert_eq!(swapped.layer_path(2), Some("spear"));
        // Parts reindexed to weapon slot.
        assert!(
            swapped.layers[2]
                .animations
                .iter()
                .flat_map(|a| a.frames.iter())
                .flat_map(|f| f.frame_parts.iter())
                .all(|p| p.animation_index == 2)
        );

        let unequipped = swapped.with_weapon_layer(None);
        assert_eq!(unequipped.layers.len(), 2);
        assert_eq!(unequipped.layer_path(2), None);
    }

    #[test]
    fn shield_layer_swap_preserves_weapon() {
        let body = make_layer(0, 2);
        let head = make_layer(1, 2);
        let mut sword = make_layer(2, 2);
        sword.path_key = Some("인간족\\기사\\기사_남_검".into());
        let mut guard = make_layer(3, 2);
        guard.path_key = Some("방패\\기사\\기사_남_가드".into());

        let base = AnimationData {
            layers: vec![body, head, sword],
            delays: vec![4.0],
            action_layouts: vec![ActionLayout {
                min_top: 0,
                max_bottom: 1,
                min_left: 0,
                max_right: 1,
            }],
            entity_type: EntityType::Player,
        };

        let with_shield = base.clone().with_shield_layer(Some(guard));
        assert_eq!(with_shield.layers.len(), 4);
        assert_eq!(with_shield.layer_path(2), Some("인간족\\기사\\기사_남_검"));
        assert_eq!(with_shield.shield_layer_path(), Some("방패\\기사\\기사_남_가드"));
        assert_eq!(with_shield.weapon_layer_path(), Some("인간족\\기사\\기사_남_검"));

        // Unequip weapon — shield remains.
        let no_weapon = with_shield.with_weapon_layer(None);
        assert_eq!(no_weapon.layers.len(), 3);
        assert_eq!(no_weapon.shield_layer_path(), Some("방패\\기사\\기사_남_가드"));
        assert_eq!(no_weapon.weapon_layer_path(), None);
        assert!(!no_weapon.layers[2].path_key.as_deref().unwrap().contains("인간족"));

        let no_shield = no_weapon.with_shield_layer(None);
        assert_eq!(no_shield.layers.len(), 2);
        assert_eq!(no_shield.shield_layer_path(), None);
    }

    #[test]
    fn shield_draw_order_follows_view_direction() {
        // body(0), head(1), weapon(2), shield(3) — paint order is what merge
        // emits; later parts get higher extra_depth_offset (in front).
        let mut body = make_layer(0, 2);
        body.path_key = Some("인간족\\몸통\\남\\기사_남".into());
        let mut head = make_layer(1, 2);
        head.path_key = Some("인간족\\머리통\\남\\1_남".into());
        let mut sword = make_layer(2, 2);
        sword.path_key = Some("인간족\\기사\\기사_남_검".into());
        let mut guard = make_layer(3, 2);
        guard.path_key = Some("방패\\기사\\기사_남_가드".into());

        let data = AnimationData {
            layers: vec![body, head, sword, guard],
            delays: vec![4.0; 8],
            action_layouts: vec![ActionLayout {
                min_top: 0,
                max_bottom: 1,
                min_left: 0,
                max_right: 1,
            }; 8],
            entity_type: EntityType::Player,
        };

        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        // Front-ish facings (0,1,6,7): shield last.
        for dir in [0usize, 1, 6, 7] {
            assert!(!shield_draws_behind_body(dir), "dir {dir} is front");
            let frame = data.compose_action_motion(&state, dir, dir);
            let order: Vec<_> = frame.frame_parts.iter().map(|p| p.animation_index).collect();
            assert_eq!(order, vec![0, 1, 2, 3], "front dir {dir}: body,head,weapon,shield");
        }
        // Back half (2..=5): shield first so body covers it.
        for dir in [2usize, 3, 4, 5] {
            assert!(shield_draws_behind_body(dir), "dir {dir} is behind");
            let frame = data.compose_action_motion(&state, dir, dir);
            let order: Vec<_> = frame.frame_parts.iter().map(|p| p.animation_index).collect();
            assert_eq!(order, vec![3, 0, 1, 2], "back dir {dir}: shield,body,head,weapon");
        }
        // silence unused mut warning if any
        let _ = &mut state;
    }

    #[test]
    fn weapon_path_from_parts_skips_shield_at_index_two() {
        // body, head, shield only — index 2 is the Guard, not a weapon.
        let shield_only = vec![
            "인간족\\몸통\\남\\기사_남".into(),
            "인간족\\머리통\\남\\1_남".into(),
            "방패\\기사\\기사_남_가드".into(),
        ];
        assert_eq!(weapon_path_from_entity_parts(&shield_only), None);
        assert_eq!(
            shield_path_from_entity_parts(&shield_only),
            Some("방패\\기사\\기사_남_가드")
        );

        // body, head, sword, shield — weapon is not "whatever is at [2]" alone;
        // both paths must resolve independently.
        let both = vec![
            "인간족\\몸통\\남\\기사_남".into(),
            "인간족\\머리통\\남\\1_남".into(),
            "인간족\\기사\\기사_남_검".into(),
            "방패\\기사\\기사_남_가드".into(),
        ];
        assert_eq!(
            weapon_path_from_entity_parts(&both),
            Some("인간족\\기사\\기사_남_검")
        );
        assert_eq!(
            shield_path_from_entity_parts(&both),
            Some("방패\\기사\\기사_남_가드")
        );

        assert!(is_weapon_part_path("인간족\\기사\\기사_남_검"));
        assert!(!is_weapon_part_path("방패\\기사\\기사_남_가드"));
        assert!(!is_weapon_part_path("인간족\\몸통\\남\\기사_남"));
        assert!(!is_weapon_part_path("인간족\\머리통\\남\\1_남"));
    }

    #[test]
    fn weapon_swap_with_shield_path_arg_does_not_clobber_sword() {
        // Simulates the old bug: refresh_entity_weapon_layer(parts.get(2)) when
        // parts = [body, head, shield] would pass the guard path as "weapon".
        let body = make_layer(0, 2);
        let head = make_layer(1, 2);
        let mut sword = make_layer(2, 2);
        sword.path_key = Some("인간족\\기사\\기사_남_검".into());
        let mut guard = make_layer(3, 2);
        guard.path_key = Some("방패\\기사\\기사_남_가드".into());

        let equipped = AnimationData {
            layers: vec![body, head, sword, guard.clone()],
            delays: vec![4.0],
            action_layouts: vec![ActionLayout {
                min_top: 0,
                max_bottom: 1,
                min_left: 0,
                max_right: 1,
            }],
            entity_type: EntityType::Player,
        };

        // with_weapon_layer(None) is what apply_weapon_layer_swap does after
        // filtering a shield path out of the weapon_path argument.
        let after_bad_refresh = equipped.with_weapon_layer(None).with_shield_layer(Some(guard));
        // Caller must re-apply the real sword; the shield alone must not replace it.
        assert_eq!(after_bad_refresh.weapon_layer_path(), None);
        assert_eq!(
            after_bad_refresh.shield_layer_path(),
            Some("방패\\기사\\기사_남_가드")
        );

        let mut sword_again = make_layer(2, 2);
        sword_again.path_key = Some("인간족\\기사\\기사_남_검".into());
        let restored = after_bad_refresh.with_weapon_layer(Some(sword_again));
        assert_eq!(restored.weapon_layer_path(), Some("인간족\\기사\\기사_남_검"));
        assert_eq!(restored.shield_layer_path(), Some("방패\\기사\\기사_남_가드"));
        assert_eq!(restored.layers.len(), 4);
    }
}

/// Phase B: crossed-motion event cursor (`0x008AC860`). Events fire once per
/// body motion crossed since the prior actor update — not once per displayed
/// frame sample — so multi-motion jumps, loop wraps, held finals, and motion-
/// program `step_serial` occurrences are all pinned here.
#[cfg(test)]
mod frame_event_cursor_tests {
    use cgmath::Vector2;
    #[cfg(feature = "debug")]
    use cgmath::{Matrix4, Zero};
    use ragnarok_packets::{ClientTick, JobId, Sex};

    use super::native_motion::shared_state_program;
    use super::{Animation, AnimationActionType, AnimationFrame, AnimationState, collect_crossed_events};
    use crate::EntityType;
    use crate::world::ActionEvent;

    /// Delay units → ms: `delay * 24`. With delay 4.0 each motion is 96 ms.
    const DELAY: f32 = 4.0;
    const MOTION_MS: u32 = 96;

    fn frame(event: Option<ActionEvent>) -> AnimationFrame {
        AnimationFrame {
            event,
            attach_point: None,
            offset: Vector2::new(0, 0),
            top_left: Vector2::new(0, 0),
            size: Vector2::new(0, 0),
            frame_parts: Vec::new(),
            #[cfg(feature = "debug")]
            horizontal_matrix: Matrix4::zero(),
            #[cfg(feature = "debug")]
            vertical_matrix: Matrix4::zero(),
        }
    }

    /// `event_at[i]` is true when motion `i` authors an `Attack` event.
    fn animation(event_at: &[bool]) -> Animation {
        Animation {
            frames: event_at
                .iter()
                .map(|&has| frame(has.then_some(ActionEvent::Attack)))
                .collect(),
        }
    }

    fn one_shot_state(start: ClientTick) -> AnimationState {
        let mut state = AnimationState::new(EntityType::Player, start);
        state.attack(EntityType::Player, false, start);
        state
    }

    fn looping_state(start: ClientTick) -> AnimationState {
        AnimationState::new(EntityType::Player, start)
    }

    fn event_count(events: &[ActionEvent]) -> usize {
        events.iter().filter(|event| matches!(event, ActionEvent::Attack)).count()
    }

    /// A slow application frame that jumps several motions must deliver every
    /// authored event on the crossed frames, not only the displayed end motion.
    #[test]
    fn multi_motion_jump_fires_every_crossed_event() {
        // Motions 0..=4; authored events on 1, 2, and 3.
        let anim = animation(&[false, true, true, true, false]);
        let mut state = one_shot_state(ClientTick(0));

        // First poll lands on motion 0 — no event.
        state.time = 0;
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 0);

        // Jump to motion 3 (288 ms): must fire events on 1, 2, and 3.
        state.time = 3 * MOTION_MS;
        let events = collect_crossed_events(&mut state, &anim, DELAY);
        assert_eq!(event_count(&events), 3, "jump 0→3 must cross events on 1, 2, 3");
    }

    /// Looping actions continue the raw motion index past one cycle so the
    /// wrap re-fires frame-0 events on the next pass.
    #[test]
    fn loop_wrap_fires_events_on_the_next_cycle() {
        // Events on motions 0 and 2 of a 4-frame loop.
        let anim = animation(&[true, false, true, false]);
        let mut state = looping_state(ClientTick(0));

        state.time = 0;
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 1);

        // Through motion 3: crosses 1, 2, 3 → one event (motion 2).
        state.time = 3 * MOTION_MS;
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 1);

        // Motion 4 is the wrap onto cycle frame 0 — must re-fire its event.
        state.time = 4 * MOTION_MS;
        assert_eq!(
            event_count(&collect_crossed_events(&mut state, &anim, DELAY)),
            1,
            "loop wrap must fire the event on motion 0 of the next cycle"
        );

        // Motion 6 lands on cycle frame 2 again.
        state.time = 6 * MOTION_MS;
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 1);
    }

    /// One-shot actions clamp at the final motion; holding it produces no
    /// further crossings (image stays, sound does not re-fire).
    #[test]
    fn held_final_frame_produces_no_new_crossings() {
        let anim = animation(&[false, false, true, false]);
        let mut state = one_shot_state(ClientTick(0));

        // Reach and deliver through the last motion (index 3).
        state.time = 3 * MOTION_MS;
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 1);

        // Still on the held final frame — raw motion keeps growing but the
        // one-shot end clamp keeps `end` at frame_count-1, so start > end.
        state.time = 10 * MOTION_MS;
        assert!(
            collect_crossed_events(&mut state, &anim, DELAY).is_empty(),
            "held final frame must not re-fire events"
        );
        state.time = 20 * MOTION_MS;
        assert!(collect_crossed_events(&mut state, &anim, DELAY).is_empty());
    }

    /// Extreme hitches only recover the final full cycle of motions so a stall
    /// cannot flood the audio mixer with dozens of backlog events.
    #[test]
    fn stall_recovery_is_bounded_to_one_cycle() {
        // Every motion authors an event; 4-frame loop.
        let anim = animation(&[true, true, true, true]);
        let mut state = looping_state(ClientTick(0));

        state.time = 0;
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 1);

        // Jump from motion 0 to motion 20: unbound would emit 20 events; the
        // cursor keeps only the last full cycle (motions 17..=20 → 4 events).
        state.time = 20 * MOTION_MS;
        assert_eq!(
            event_count(&collect_crossed_events(&mut state, &anim, DELAY)),
            4,
            "stall recovery must bound delivery to one full cycle"
        );
    }

    /// Motion programs advance at most one step per actor update; each
    /// `step_serial` change is one event occurrence — a duplicate authored
    /// motion fires again, a terminal held motion does not.
    #[test]
    fn motion_program_fires_once_per_step_serial_occurrence() {
        // State 19: sequence [0, 0, 0, 1, 1], cadence 10 actor updates.
        let anim = animation(&[true, true, false, false, false]);
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.action_base_offset = 12;
        state.native_actor_state = 19;
        state.looping = false;
        state.motion_program = shared_state_program(19, 12, JobId(7), false);

        // Serial 0 / motion 0.
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 1);
        // Same serial again — no re-fire.
        assert!(collect_crossed_events(&mut state, &anim, DELAY).is_empty());

        // Advance to the next duplicate motion-0 step (serial 1).
        for elapsed in 1..=10 {
            state.motion_program.as_mut().unwrap().update(elapsed);
        }
        assert_eq!(state.motion_program.as_ref().unwrap().step_serial(), 1);
        assert_eq!(
            event_count(&collect_crossed_events(&mut state, &anim, DELAY)),
            1,
            "duplicate authored motion must fire again on a new step_serial"
        );

        // Advance through serials 2, 3, 4 to the terminal held motion 1.
        for elapsed in 11..=50 {
            state.motion_program.as_mut().unwrap().update(elapsed);
        }
        assert_eq!(state.motion_program.as_ref().unwrap().step_serial(), 4);
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 1);

        // Terminal hold: more updates leave the serial unchanged.
        for elapsed in 51..=100 {
            state.motion_program.as_mut().unwrap().update(elapsed);
        }
        assert_eq!(state.motion_program.as_ref().unwrap().step_serial(), 4);
        assert!(
            collect_crossed_events(&mut state, &anim, DELAY).is_empty(),
            "terminal held motion must not create new event occurrences"
        );
    }

    /// A new playback identity (new start tick / action) resets the cursor so
    /// the first motion of the new action is delivered fresh.
    #[test]
    fn new_playback_identity_resets_the_cursor() {
        let anim = animation(&[true, false, true, false]);
        let mut state = one_shot_state(ClientTick(100));
        state.time = 2 * MOTION_MS;
        assert_eq!(event_count(&collect_crossed_events(&mut state, &anim, DELAY)), 2);

        // New attack starts a new cursor; motion 0 of the fresh playback fires.
        state.attack(EntityType::Player, false, ClientTick(500));
        state.time = 0;
        assert_eq!(
            event_count(&collect_crossed_events(&mut state, &anim, DELAY)),
            1,
            "new start_time must reset the frame-event cursor"
        );
    }

    /// Assassin Cross body ACTs ship with an empty event table. The native
    /// attack-event marker (katar → 3.0) still owns the swing sound via a
    /// synthetic `ActionEvent::Attack` that `weapon_sound` resolves.
    #[test]
    fn empty_body_act_synthesizes_attack_at_native_marker() {
        // 8-frame Attack3 with no authored events — male Assassin Cross shape.
        let anim = animation(&[false; 8]);
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        // Job 4013 Assassin Cross, katar view 16 → Attack3, marker 3.0.
        state.weapon_attack(EntityType::Player, JobId(4013), Sex::Male, 16, ClientTick(0));
        assert_eq!(state.action_type, AnimationActionType::Attack3);
        assert_eq!(state.impact_event_position_override(), Some(3.0));

        // Before the marker: silence.
        state.time = 2 * MOTION_MS;
        assert!(
            collect_crossed_events(&mut state, &anim, DELAY).is_empty(),
            "synthetic Attack must wait for the native marker motion"
        );

        // Crossing motion 3: exactly one Attack.
        state.time = 3 * MOTION_MS;
        let events = collect_crossed_events(&mut state, &anim, DELAY);
        assert_eq!(events, vec![ActionEvent::Attack]);

        // Held / further motions: no re-fire.
        state.time = 7 * MOTION_MS;
        assert!(collect_crossed_events(&mut state, &anim, DELAY).is_empty());
    }

    /// Authored body events always win; never stack a synthetic Attack on top
    /// of an already-populated event table (Knight spear shape: event at 5,
    /// marker at 6 — only one delivery).
    #[test]
    fn authored_events_suppress_synthetic_attack_fallback() {
        let anim = animation(&[false, false, false, false, false, true, false, false]);
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.weapon_attack(EntityType::Player, JobId(7), Sex::Male, 4, ClientTick(0));
        assert_eq!(state.impact_event_position_override(), Some(6.0));

        state.time = 7 * MOTION_MS;
        let events = collect_crossed_events(&mut state, &anim, DELAY);
        assert_eq!(
            event_count(&events),
            1,
            "authored event must suppress the empty-table synthetic Attack"
        );
    }
}
