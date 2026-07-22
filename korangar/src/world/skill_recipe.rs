//! Typed, phase-owned presentation recipes for server-authoritative skills.
//!
//! A recipe does not decide whether a skill executed. Network packets own
//! that decision and select a phase; this table only declares the visual,
//! projectile, and audio tracks attached to that phase.

use ragnarok_packets::SkillId;

use crate::Color;

/// Which effect family an asset belongs to. RO ships two, and they are played
/// by completely different pipelines: keyframe scripts under
/// `data\texture\effect\*.str`, and classic sprite animations under
/// `data\sprite\이팩트\*.spr` + `.act`.
///
/// The distinction is not stylistic. The classic single-target spells have no
/// `.str` file in the GRFs at all, so they can only be drawn authentically
/// through the sprite path. See `docs/plans/classic-effect-fidelity.md`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResolvedEffect {
    /// A keyframe script, relative to `data\texture\effect\`.
    Str(&'static str),
    /// A sprite animation, relative to `data\sprite\` and without an
    /// extension, plus the ACT action to play.
    Sprite { path: &'static str, action_index: usize },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EffectAsset {
    Fixed(&'static str),
    /// A classic sprite effect. `action_index` selects the ACT action, which
    /// for most effect sheets is 0.
    Sprite {
        path: &'static str,
        action_index: usize,
    },
    FireHit,
    WindHit,
    Meteor,
    Firewall,
}

impl EffectAsset {
    /// Convenience for the common case of a sprite sheet whose effect is its
    /// first ACT action.
    pub const fn sprite(path: &'static str) -> Self {
        Self::Sprite { path, action_index: 0 }
    }

    pub fn resolve(self) -> ResolvedEffect {
        match self {
            Self::Sprite { path, action_index } => ResolvedEffect::Sprite { path, action_index },
            other => ResolvedEffect::Str(other.resolve_str()),
        }
    }

    /// Resolve the `.str` path for the keyframe-script variants. Panics on
    /// [`Self::Sprite`], which has no `.str` equivalent by construction —
    /// callers must go through [`Self::resolve`].
    fn resolve_str(self) -> &'static str {
        match self {
            Self::Fixed(path) => path,
            Self::Sprite { path, .. } => unreachable!("sprite effect {path} has no .str form; use EffectAsset::resolve"),
            Self::FireHit => match rand_aes::tls::rand_range_u32(1..=3) {
                1 => "firehit1.str",
                2 => "firehit2.str",
                _ => "firehit3.str",
            },
            Self::WindHit => match rand_aes::tls::rand_range_u32(1..=3) {
                1 => "windhit1.str",
                2 => "windhit2.str",
                _ => "windhit3.str",
            },
            Self::Meteor => match rand_aes::tls::rand_range_u32(1..=4) {
                1 => "meteor1.str",
                2 => "meteor2.str",
                3 => "meteor3.str",
                _ => "meteor4.str",
            },
            Self::Firewall => match rand_aes::tls::rand_range_u32(1..=2) {
                1 => "firewall1.str",
                _ => "firewall2.str",
            },
        }
    }

    /// Every `.str` this asset can resolve to. Sprite effects contribute
    /// nothing here by design — they are covered by [`Self::sprite_path`].
    #[cfg(test)]
    pub fn variants(self) -> Vec<&'static str> {
        match self {
            Self::Fixed(path) => vec![path],
            Self::Sprite { .. } => Vec::new(),
            Self::FireHit => vec!["firehit1.str", "firehit2.str", "firehit3.str"],
            Self::WindHit => vec!["windhit1.str", "windhit2.str", "windhit3.str"],
            Self::Meteor => vec!["meteor1.str", "meteor2.str", "meteor3.str", "meteor4.str"],
            Self::Firewall => vec!["firewall1.str", "firewall2.str"],
        }
    }

    /// The sprite sheet backing this asset, if it is a sprite effect.
    pub fn sprite_path(self) -> Option<&'static str> {
        match self {
            Self::Sprite { path, .. } => Some(path),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EffectTrack {
    pub asset: EffectAsset,
    pub light_color: Color,
    pub start_delay: f32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SoundAsset {
    Fixed(&'static str),
    FireArrow,
}

impl SoundAsset {
    pub fn resolve(self) -> &'static str {
        match self {
            Self::Fixed(path) => path,
            Self::FireArrow => match rand_aes::tls::rand_range_u32(1..=3) {
                1 => "effect\\ef_firearrow1.wav",
                2 => "effect\\ef_firearrow2.wav",
                _ => "effect\\ef_firearrow3.wav",
            },
        }
    }

    #[cfg(test)]
    pub fn variants(self) -> Vec<&'static str> {
        match self {
            Self::Fixed(path) => vec![path],
            Self::FireArrow => vec![
                "effect\\ef_firearrow1.wav",
                "effect\\ef_firearrow2.wav",
                "effect\\ef_firearrow3.wav",
            ],
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SuccessfulCasterEffect {
    MagnumBreak,
    FrostNova,
    Raid,
    MeteorAssault,
    IgnitionBreak,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DamageCasterEffect {
    Pierce,
    BrandishSpear,
    SpearStab,
    SpearBoomerang,
    BowlingBash,
    SonicBlow,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DamageTargetEffect {
    BrandishSpear,
    BowlingBash,
    SonicBlow,
    /// Phase E1 — MG_NAPALMBEAT psychic shock rings.
    NapalmBeat,
    /// Phase E1 — WZ_EARTHSPIKE single ground spike.
    EarthSpike,
    /// Phase E1 — WZ_HEAVENDRIVE multi-spike ring.
    HeavensDrive,
}

/// Source→target travel ball kind for classic Mage/Wizard spells (Phase E1).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TravelBallKind {
    FireBall,
    FrostDiver,
    Jupitel,
}

impl TravelBallKind {
    /// Texture paths confirmed present via `probes_e1_procedural_effect_textures`
    /// (GRF `file_exists` / archive listing). Prefer root classic assets over
    /// unrelated modern nested packs when both exist.
    pub fn texture_path(self) -> &'static str {
        match self {
            // Root fire splash — reads as a fire ball better than bolt frames.
            Self::FireBall => "effect\\fire_blast.bmp",
            // Classic ice disc/crystal (not the cold-bolt arrow).
            Self::FrostDiver => "effect\\ice.tga",
            // Lightning burst used as a travel ball with yellow tint.
            Self::Jupitel => "effect\\번개4.bmp",
        }
    }

    pub fn duration(self) -> f32 {
        match self {
            Self::FireBall => 0.28,
            Self::FrostDiver => 0.32,
            Self::Jupitel => 0.38,
        }
    }

    pub fn size(self) -> f32 {
        match self {
            Self::FireBall => 78.0,
            Self::FrostDiver => 70.0,
            Self::Jupitel => 84.0,
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::FireBall => Color::rgb_u8(255, 140, 50),
            Self::FrostDiver => Color::rgb_u8(180, 230, 255),
            Self::Jupitel => Color::rgb_u8(255, 250, 180),
        }
    }

    /// Mid-flight point-light peak intensity (world units).
    pub fn light_intensity(self) -> f32 {
        match self {
            Self::FireBall => 42.0,
            Self::FrostDiver => 36.0,
            Self::Jupitel => 48.0,
        }
    }
}

/// Soul Strike orb texture (soft particle). Confirmed in GRF probe.
pub const SOUL_STRIKE_ORB_TEXTURE: &str = "effect\\pok1.tga";
/// Napalm Beat ring/flash. Blue ring + purple tint reads more psychic than slash.
pub const NAPALM_BEAT_TEXTURE: &str = "effect\\ring_blue.tga";
/// Earth Spike / Heaven's Drive primary stone face.
pub const EARTH_SPIKE_TEXTURE: &str = "effect\\bd_stonecurse.tga";
/// Secondary stone layer for layered spikes.
pub const EARTH_SPIKE_TEXTURE_SECONDARY: &str = "effect\\crystallization\\cry_stone_01.tga";

pub const COLDBOLT_BOLT_FRAMES: &[&str] = &["effect\\icearrow.tga"];
pub const FIREBOLT_BOLT_FRAMES: &[&str] = &[
    "effect\\불화살1.tga",
    "effect\\불화살2.tga",
    "effect\\불화살3.tga",
    "effect\\불화살4.tga",
    "effect\\불화살5.tga",
    "effect\\불화살6.tga",
];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProjectileRecipe {
    FallingBolts(&'static [&'static str]),
    Spear,
    /// Phase E1 — single source→target travel ball.
    TravelBall(TravelBallKind),
    /// Phase E1 — Soul Strike multi-orb volley (orb count = hit_count).
    SoulStrikeOrbs,
}

/// Procedural texture paths referenced by Phase E1 target/travel recipes.
/// Kept explicit so the GRF audit can assert them without walking spawn code.
#[cfg(test)]
pub const E1_PROCEDURAL_TEXTURES: &[&str] = &[
    SOUL_STRIKE_ORB_TEXTURE,
    NAPALM_BEAT_TEXTURE,
    EARTH_SPIKE_TEXTURE,
    EARTH_SPIKE_TEXTURE_SECONDARY,
    "effect\\fire_blast.bmp",
    "effect\\ice.tga",
    "effect\\번개4.bmp",
    "effect\\lens1.tga",
    "effect\\lens2.tga",
    "effect\\ring_yellow.tga",
    "effect\\purpleslash.tga",
];

#[derive(Copy, Clone, Debug)]
pub struct SkillPresentationRecipe {
    pub successful_caster_effect: Option<SuccessfulCasterEffect>,
    pub damage_caster_effect: Option<DamageCasterEffect>,
    pub damage_target_effect: Option<DamageTargetEffect>,
    pub hit_effects: &'static [EffectTrack],
    pub ground_effect: Option<EffectTrack>,
    pub projectile: Option<ProjectileRecipe>,
    pub successful_caster_sounds: &'static [SoundAsset],
    pub damage_caster_sounds: &'static [SoundAsset],
    pub damage_target_sounds: &'static [SoundAsset],
    pub hit_sounds: &'static [SoundAsset],
    pub ground_sounds: &'static [SoundAsset],
}

const EMPTY: SkillPresentationRecipe = SkillPresentationRecipe {
    successful_caster_effect: None,
    damage_caster_effect: None,
    damage_target_effect: None,
    hit_effects: &[],
    ground_effect: None,
    projectile: None,
    successful_caster_sounds: &[],
    damage_caster_sounds: &[],
    damage_target_sounds: &[],
    hit_sounds: &[],
    ground_sounds: &[],
};

const FIRE_HIT: EffectTrack = EffectTrack {
    asset: EffectAsset::FireHit,
    light_color: Color::rgb_u8(255, 90, 25),
    start_delay: 0.0,
};
const WIND_HIT: EffectTrack = EffectTrack {
    asset: EffectAsset::WindHit,
    light_color: Color::rgb_u8(255, 240, 150),
    start_delay: 0.0,
};
const EARTH_HIT: EffectTrack = EffectTrack {
    asset: EffectAsset::Fixed("earthhit.str"),
    light_color: Color::rgb_u8(215, 180, 110),
    start_delay: 0.0,
};
const HOLY_HIT: EffectTrack = EffectTrack {
    asset: EffectAsset::Fixed("holyhit.str"),
    light_color: Color::rgb_u8(255, 245, 190),
    start_delay: 0.0,
};
const SOUL_STRIKE_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("new_soulexpansion\\new_soulexpansion_hit\\new_soulexpansion_hit.str"),
    light_color: Color::rgb_u8(190, 120, 255),
    start_delay: 0.0,
}];
const FROST_DIVER_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("freeze.str"),
    light_color: Color::rgb_u8(150, 225, 255),
    start_delay: 0.0,
}];
const LIGHTNING_BOLT_HITS: &[EffectTrack] = &[
    EffectTrack {
        asset: EffectAsset::Fixed("lightning.str"),
        light_color: Color::rgb_u8(255, 240, 150),
        start_delay: 0.0,
    },
    WIND_HIT,
];
const FIRE_PILLAR_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("firepillarbomb.str"),
    light_color: Color::rgb_u8(255, 80, 20),
    start_delay: 0.0,
}];
const VERMILION_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::WindHit,
    light_color: Color::rgb_u8(245, 235, 150),
    start_delay: 0.0,
}];
const SHOCKWAVE_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("shockwavehit.str"),
    light_color: Color::rgb_u8(210, 180, 255),
    start_delay: 0.0,
}];
const SANDMAN_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("sandman.str"),
    light_color: Color::rgb_u8(230, 205, 130),
    start_delay: 0.0,
}];
const FREEZING_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("freezing.str"),
    light_color: Color::rgb_u8(150, 225, 255),
    start_delay: 0.0,
}];
const BLAST_MINE_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("blastmine.str"),
    light_color: Color::rgb_u8(255, 120, 35),
    start_delay: 0.0,
}];
const CLAYMORE_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("claymore.str"),
    light_color: Color::rgb_u8(255, 80, 25),
    start_delay: 0.0,
}];
const POISON_REACT_HITS: &[EffectTrack] = &[EffectTrack {
    asset: EffectAsset::Fixed("poisonreact.str"),
    light_color: Color::rgb_u8(160, 90, 210),
    start_delay: 0.0,
}];

macro_rules! recipe {
    ($($field:ident: $value:expr),* $(,)?) => {
        SkillPresentationRecipe { $($field: $value,)* ..EMPTY }
    };
}

/// Resolve every skill ID to a complete five-phase contract. Unknown skills
/// deliberately return an empty recipe, making "no proven presentation track"
/// explicit instead of falling through several unrelated runtime tables.
pub fn skill_presentation_recipe(skill_id: SkillId) -> SkillPresentationRecipe {
    match skill_id.0 {
        5 => recipe!(hit_sounds: &[SoundAsset::Fixed("effect\\ef_bash.wav")]),
        7 => recipe!(
            successful_caster_effect: Some(SuccessfulCasterEffect::MagnumBreak),
            successful_caster_sounds: &[SoundAsset::Fixed("effect\\ef_magnumbreak.wav")],
        ),
        11 => recipe!(
            damage_target_effect: Some(DamageTargetEffect::NapalmBeat),
            hit_sounds: &[SoundAsset::Fixed("effect\\ef_napalmbeat.wav")],
        ),
        13 => recipe!(
            projectile: Some(ProjectileRecipe::SoulStrikeOrbs),
            hit_effects: SOUL_STRIKE_HITS,
            hit_sounds: &[SoundAsset::Fixed("effect\\ef_soulstrike.wav")],
        ),
        14 => recipe!(
            projectile: Some(ProjectileRecipe::FallingBolts(COLDBOLT_BOLT_FRAMES)),
            hit_sounds: &[SoundAsset::Fixed("effect\\ef_icearrow.wav")],
        ),
        15 => recipe!(
            projectile: Some(ProjectileRecipe::TravelBall(TravelBallKind::FrostDiver)),
            hit_effects: FROST_DIVER_HITS,
            hit_sounds: &[SoundAsset::Fixed("effect\\ef_frostdiver.wav")],
        ),
        17 => recipe!(
            projectile: Some(ProjectileRecipe::TravelBall(TravelBallKind::FireBall)),
            hit_effects: &[FIRE_HIT],
            hit_sounds: &[SoundAsset::Fixed("effect\\ef_fireball.wav")],
        ),
        18 => recipe!(
            hit_effects: &[FIRE_HIT],
            ground_effect: Some(EffectTrack {
                asset: EffectAsset::Firewall,
                light_color: Color::rgb_u8(255, 45, 10),
                start_delay: 0.0,
            }),
            ground_sounds: &[SoundAsset::Fixed("effect\\ef_firewall.wav")],
        ),
        19 => recipe!(
            hit_effects: &[FIRE_HIT],
            projectile: Some(ProjectileRecipe::FallingBolts(FIREBOLT_BOLT_FRAMES)),
            hit_sounds: &[SoundAsset::FireArrow],
        ),
        20 => recipe!(hit_effects: LIGHTNING_BOLT_HITS),
        21 => recipe!(
            hit_effects: &[WIND_HIT],
            ground_effect: Some(EffectTrack {
                asset: EffectAsset::Fixed("thunderstorm.str"),
                light_color: Color::rgb_u8(255, 240, 150),
                start_delay: 0.0,
            }),
            ground_sounds: &[SoundAsset::Fixed("effect\\ef_thunderstorm.wav")],
        ),
        56 => recipe!(
            damage_caster_effect: Some(DamageCasterEffect::Pierce),
            hit_effects: &[EARTH_HIT],
        ),
        57 => recipe!(
            damage_caster_effect: Some(DamageCasterEffect::BrandishSpear),
            damage_target_effect: Some(DamageTargetEffect::BrandishSpear),
            damage_caster_sounds: &[SoundAsset::Fixed("effect\\knight_brandish_spear.wav")],
        ),
        58 => recipe!(
            damage_caster_effect: Some(DamageCasterEffect::SpearStab),
            damage_target_sounds: &[SoundAsset::Fixed("_enemy_hit_normal1.wav")],
        ),
        59 => recipe!(
            damage_caster_effect: Some(DamageCasterEffect::SpearBoomerang),
            projectile: Some(ProjectileRecipe::Spear),
            damage_caster_sounds: &[SoundAsset::Fixed("effect\\knight_spear_boomerang.wav")],
        ),
        62 => recipe!(
            damage_caster_effect: Some(DamageCasterEffect::BowlingBash),
            damage_target_effect: Some(DamageTargetEffect::BowlingBash),
            damage_target_sounds: &[
                SoundAsset::Fixed("_enemy_hit_normal1.wav"),
                SoundAsset::Fixed("effect\\ef_hit2.wav"),
            ],
        ),
        70 => recipe!(ground_effect: Some(EffectTrack {
            asset: EffectAsset::Fixed("sanctuary.str"),
            light_color: Color::rgb_u8(130, 255, 175),
            start_delay: 0.0,
        })),
        77 => recipe!(hit_effects: &[HOLY_HIT]),
        79 => recipe!(
            hit_effects: &[HOLY_HIT],
            ground_effect: Some(EffectTrack {
                asset: EffectAsset::Fixed("magnus.str"),
                light_color: Color::rgb_u8(255, 225, 170),
                start_delay: 0.0,
            }),
        ),
        80 => recipe!(
            hit_effects: FIRE_PILLAR_HITS,
            ground_effect: Some(EffectTrack {
                asset: EffectAsset::Fixed("firepillar.str"),
                light_color: Color::rgb_u8(255, 65, 15),
                start_delay: 0.0,
            }),
        ),
        81 => recipe!(hit_effects: &[FIRE_HIT]),
        83 => recipe!(
            hit_effects: &[FIRE_HIT],
            ground_effect: Some(EffectTrack {
                asset: EffectAsset::Meteor,
                light_color: Color::rgb_u8(255, 95, 25),
                start_delay: 0.0,
            }),
        ),
        85 => recipe!(
            hit_effects: VERMILION_HITS,
            ground_effect: Some(EffectTrack {
                asset: EffectAsset::Fixed("lord.str"),
                light_color: Color::rgb_u8(245, 235, 150),
                start_delay: 0.0,
            }),
        ),
        88 => recipe!(successful_caster_effect: Some(SuccessfulCasterEffect::FrostNova)),
        89 => recipe!(
            ground_effect: Some(EffectTrack {
                asset: EffectAsset::Fixed("stormgust.str"),
                light_color: Color::rgb_u8(175, 225, 255),
                start_delay: 0.0,
            }),
            ground_sounds: &[SoundAsset::Fixed("effect\\storm.wav")],
        ),
        // WZ_JUPITEL — travel ball + wind/lightning hits (Phase E1).
        84 => recipe!(
            projectile: Some(ProjectileRecipe::TravelBall(TravelBallKind::Jupitel)),
            hit_effects: LIGHTNING_BOLT_HITS,
            hit_sounds: &[SoundAsset::Fixed("effect\\ef_thunderstorm.wav")],
        ),
        90 => recipe!(
            damage_target_effect: Some(DamageTargetEffect::EarthSpike),
            hit_effects: &[EARTH_HIT],
        ),
        91 => recipe!(
            damage_target_effect: Some(DamageTargetEffect::HeavensDrive),
            hit_effects: &[EARTH_HIT],
        ),
        92 => recipe!(ground_effect: Some(EffectTrack {
            asset: EffectAsset::Fixed("quagmire.str"),
            light_color: Color::rgb_u8(135, 105, 75),
            start_delay: 0.0,
        })),
        110 => recipe!(ground_effect: Some(EffectTrack {
            asset: EffectAsset::Fixed("crashearth.str"),
            light_color: Color::rgb_u8(235, 190, 95),
            start_delay: 0.0,
        })),
        115 => recipe!(ground_effect: Some(EffectTrack {
            asset: EffectAsset::Fixed("skidtrap.str"),
            light_color: Color::rgb_u8(235, 205, 90),
            start_delay: 0.0,
        })),
        118 => recipe!(hit_effects: SHOCKWAVE_HITS),
        119 => recipe!(hit_effects: SANDMAN_HITS),
        121 => recipe!(hit_effects: FREEZING_HITS),
        122 => recipe!(hit_effects: BLAST_MINE_HITS),
        123 => recipe!(hit_effects: CLAYMORE_HITS),
        136 => recipe!(
            damage_caster_effect: Some(DamageCasterEffect::SonicBlow),
            damage_target_effect: Some(DamageTargetEffect::SonicBlow),
            damage_target_sounds: &[SoundAsset::Fixed("effect\\assasin_sonicblow.wav")],
        ),
        139 => recipe!(hit_effects: POISON_REACT_HITS),
        140 => recipe!(ground_effect: Some(EffectTrack {
            asset: EffectAsset::Fixed("venomdust.str"),
            light_color: Color::rgb_u8(140, 75, 190),
            start_delay: 0.0,
        })),
        156 => recipe!(hit_effects: &[HOLY_HIT]),
        // Acolyte / Priest support feedback (M1-008 catalog expansion).
        // Acolyte / Priest (IDs verified against Hercules skill_db).
        28 => recipe!(hit_effects: &[HOLY_HIT]), // AL_HEAL
        29 => recipe!(hit_effects: &[HOLY_HIT]), // AL_INCAGI
        30 => recipe!(hit_effects: &[HOLY_HIT]), // AL_DECAGI
        33 => recipe!(hit_effects: &[HOLY_HIT]), // AL_ANGELUS
        34 => recipe!(hit_effects: &[HOLY_HIT]), // AL_BLESSING
        66 => recipe!(hit_effects: &[HOLY_HIT]), // PR_IMPOSITIO
        67 => recipe!(hit_effects: &[HOLY_HIT]), // PR_SUFFRAGIUM
        68 => recipe!(hit_effects: &[HOLY_HIT]), // PR_ASPERSIO
        73 => recipe!(hit_effects: &[HOLY_HIT]), // PR_KYRIE
        74 => recipe!(hit_effects: &[HOLY_HIT]), // PR_MAGNIFICAT
        75 => recipe!(hit_effects: &[HOLY_HIT]), // PR_GLORIA
        // AL_HOLYLIGHT is 156 (already mapped above as HOLY_HIT)
        214 => recipe!(successful_caster_effect: Some(SuccessfulCasterEffect::Raid)),
        406 => recipe!(successful_caster_effect: Some(SuccessfulCasterEffect::MeteorAssault)),
        2006 => recipe!(successful_caster_effect: Some(SuccessfulCasterEffect::IgnitionBreak)),
        _ => EMPTY,
    }
}

/// IDs with at least one proven presentation track in this build. Kept
/// explicit so audits can compare recipe coverage with skillinfo/skill trees.
#[cfg(test)]
pub const MAPPED_SKILL_IDS: &[SkillId] = &[
    SkillId(5),
    SkillId(7),
    SkillId(11),
    SkillId(13),
    SkillId(14),
    SkillId(15),
    SkillId(17),
    SkillId(18),
    SkillId(19),
    SkillId(20),
    SkillId(21),
    SkillId(56),
    SkillId(57),
    SkillId(58),
    SkillId(59),
    SkillId(62),
    SkillId(70),
    SkillId(77),
    SkillId(79),
    SkillId(80),
    SkillId(81),
    SkillId(83),
    SkillId(84),
    SkillId(85),
    SkillId(88),
    SkillId(89),
    SkillId(90),
    SkillId(91),
    SkillId(92),
    SkillId(110),
    SkillId(115),
    SkillId(118),
    SkillId(119),
    SkillId(121),
    SkillId(122),
    SkillId(123),
    SkillId(136),
    SkillId(139),
    SkillId(140),
    SkillId(156),
    SkillId(28),
    SkillId(29),
    SkillId(30),
    SkillId(33),
    SkillId(34),
    SkillId(66),
    SkillId(67),
    SkillId(68),
    SkillId(73),
    SkillId(74),
    SkillId(75),
    SkillId(214),
    SkillId(406),
    SkillId(2006),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_mapped_skill_has_a_non_empty_recipe() {
        for skill_id in MAPPED_SKILL_IDS {
            let recipe = skill_presentation_recipe(*skill_id);
            assert!(
                recipe.successful_caster_effect.is_some()
                    || recipe.damage_caster_effect.is_some()
                    || recipe.damage_target_effect.is_some()
                    || !recipe.hit_effects.is_empty()
                    || recipe.ground_effect.is_some()
                    || recipe.projectile.is_some()
                    || !recipe.successful_caster_sounds.is_empty()
                    || !recipe.damage_caster_sounds.is_empty()
                    || !recipe.damage_target_sounds.is_empty()
                    || !recipe.hit_sounds.is_empty()
                    || !recipe.ground_sounds.is_empty(),
                "skill {} is listed but empty",
                skill_id.0
            );
        }
    }

    #[test]
    fn projectile_and_audio_ownership_is_per_skill() {
        assert_eq!(skill_presentation_recipe(SkillId(59)).projectile, Some(ProjectileRecipe::Spear));
        assert_eq!(
            skill_presentation_recipe(SkillId(19)).projectile,
            Some(ProjectileRecipe::FallingBolts(FIREBOLT_BOLT_FRAMES))
        );
        assert_eq!(skill_presentation_recipe(SkillId(62)).damage_target_sounds.len(), 2);
    }

    #[test]
    fn phase_e1_code_drawn_recipes_are_wired() {
        assert_eq!(
            skill_presentation_recipe(SkillId(11)).damage_target_effect,
            Some(DamageTargetEffect::NapalmBeat)
        );
        assert_eq!(
            skill_presentation_recipe(SkillId(13)).projectile,
            Some(ProjectileRecipe::SoulStrikeOrbs)
        );
        assert_eq!(
            skill_presentation_recipe(SkillId(15)).projectile,
            Some(ProjectileRecipe::TravelBall(TravelBallKind::FrostDiver))
        );
        assert_eq!(
            skill_presentation_recipe(SkillId(17)).projectile,
            Some(ProjectileRecipe::TravelBall(TravelBallKind::FireBall))
        );
        assert_eq!(
            skill_presentation_recipe(SkillId(84)).projectile,
            Some(ProjectileRecipe::TravelBall(TravelBallKind::Jupitel))
        );
        assert_eq!(
            skill_presentation_recipe(SkillId(90)).damage_target_effect,
            Some(DamageTargetEffect::EarthSpike)
        );
        assert_eq!(
            skill_presentation_recipe(SkillId(91)).damage_target_effect,
            Some(DamageTargetEffect::HeavensDrive)
        );
    }

    #[test]
    fn unknown_skill_has_an_explicit_empty_contract() {
        let recipe = skill_presentation_recipe(SkillId(u16::MAX));
        assert!(recipe.hit_effects.is_empty());
        assert!(recipe.projectile.is_none());
        assert!(recipe.ground_sounds.is_empty());
    }

    #[test]
    fn every_data_asset_declares_its_complete_audit_variant_set() {
        for skill_id in MAPPED_SKILL_IDS {
            let recipe = skill_presentation_recipe(*skill_id);
            for effect in recipe.hit_effects.iter().chain(recipe.ground_effect.iter()) {
                assert!(
                    !effect.asset.variants().is_empty(),
                    "skill {} effect is not auditable",
                    skill_id.0
                );
            }
            for sound in recipe
                .successful_caster_sounds
                .iter()
                .chain(recipe.damage_caster_sounds)
                .chain(recipe.damage_target_sounds)
                .chain(recipe.hit_sounds)
                .chain(recipe.ground_sounds)
            {
                assert!(!sound.variants().is_empty(), "skill {} sound is not auditable", skill_id.0);
            }
        }
    }
}
