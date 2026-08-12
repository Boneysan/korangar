//! Presentation mapping for native client effect IDs (`ZC_NOTIFY_EFFECT2`
//! / `DisplaySpecialEffectPacket` 0x01F3).
//!
//! Shape categories document *what kind of visual* the original effect list
//! describes (ball, ring, spike, orb, flash, STR). They are semantic labels for
//! recipe selection — not a port of third-party client code.

use ragnarok_packets::EffectId;

use crate::Color;
use crate::world::{EARTH_SPIKE_TEXTURE, NAPALM_BEAT_TEXTURE, NAPALM_BEAT_TEXTURE_SECONDARY, SkillBurstStyle};

/// High-level geometry family for an effect ID (from Hercules effect_list
/// descriptions: ball / hit / ring / spike / etc.).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum EffectShape {
    /// Soft floating orb (Soul Strike family).
    Orb,
    /// Expanding ring / psychic shock (Napalm).
    Ring,
    /// Rising ground spike (Earth Spike / Heaven's Drive).
    Spike,
    /// Traveling elemental ball (Fire Ball, Jupitel, Frost Diver travel).
    Ball,
    /// Instant flash / hit burst.
    Flash,
    /// STR-backed animation.
    Str,
}

/// How the client should present a mapped special-effect ID.
#[derive(Copy, Clone, Debug)]
pub enum SpecialEffectRecipe {
    /// Load and play a shipped STR at the entity.
    Str {
        path: &'static str,
        light_color: Color,
        light_intensity: f32,
    },
    /// Procedural `SkillBurst` (ring / spike / flash families).
    Burst {
        style: SkillBurstStyle,
        texture: &'static str,
        secondary: Option<&'static str>,
    },
}

impl SpecialEffectRecipe {
    #[allow(dead_code)]
    pub fn shape(self) -> EffectShape {
        match self {
            Self::Str { .. } => EffectShape::Str,
            Self::Burst { style, .. } => match style {
                SkillBurstStyle::NapalmBeat => EffectShape::Ring,
                SkillBurstStyle::EarthSpike | SkillBurstStyle::HeavensDrive => EffectShape::Spike,
                SkillBurstStyle::MagnumBreak | SkillBurstStyle::Raid | SkillBurstStyle::SonicBlow => EffectShape::Ring,
                SkillBurstStyle::MeteorAssault | SkillBurstStyle::MeleeHit | SkillBurstStyle::JupitelHit => EffectShape::Flash,
            },
        }
    }
}

/// Resolve a native effect ID to a presentation recipe. Unmapped IDs return
/// `None` (explicit empty contract — no closest-recipe guessing).
pub fn special_effect_recipe(effect_id: EffectId) -> Option<SpecialEffectRecipe> {
    match effect_id {
        // --- E1-related classic IDs (Hercules effect_list) ---
        EffectId::Soulstrike => Some(SpecialEffectRecipe::Str {
            path: "new_soulexpansion\\new_soulexpansion_hit\\new_soulexpansion_hit.str",
            light_color: Color::rgb_u8(190, 120, 255),
            light_intensity: 40.0,
        }),
        EffectId::Fireball | EffectId::Fireball2 | EffectId::Fireball3 => Some(SpecialEffectRecipe::Str {
            path: "firehit1.str",
            light_color: Color::rgb_u8(255, 120, 40),
            light_intensity: 45.0,
        }),
        EffectId::Frostdiver => Some(SpecialEffectRecipe::Str {
            // Travel cue when only the special-effect packet is present.
            path: "chill.str",
            light_color: Color::rgb_u8(160, 220, 255),
            light_intensity: 35.0,
        }),
        // Freeze1 vs Freeze2: the client ships BOTH `freeze.str` and `freezed.str`
        // (GRF-probed 2026-07-26) and the original picks between them, so mapping
        // both ids onto `freeze.str` threw half the distinction away.
        EffectId::Frostdiver2 | EffectId::Freeze => Some(SpecialEffectRecipe::Str {
            path: "freeze.str",
            light_color: Color::rgb_u8(150, 225, 255),
            light_intensity: 50.0,
        }),
        EffectId::Freezed => Some(SpecialEffectRecipe::Str {
            path: "freezed.str",
            light_color: Color::rgb_u8(150, 225, 255),
            light_intensity: 50.0,
        }),
        EffectId::Napalmbeat => Some(SpecialEffectRecipe::Burst {
            style: SkillBurstStyle::NapalmBeat,
            texture: NAPALM_BEAT_TEXTURE,
            secondary: Some(NAPALM_BEAT_TEXTURE_SECONDARY),
        }),
        EffectId::Earthspike => Some(SpecialEffectRecipe::Burst {
            style: SkillBurstStyle::EarthSpike,
            texture: EARTH_SPIKE_TEXTURE,
            secondary: None,
        }),
        EffectId::Heavensdrive => Some(SpecialEffectRecipe::Burst {
            style: SkillBurstStyle::HeavensDrive,
            texture: EARTH_SPIKE_TEXTURE,
            secondary: None,
        }),
        EffectId::Yufitel | EffectId::Yufitel2 => Some(SpecialEffectRecipe::Str {
            path: "lightning.str",
            light_color: Color::rgb_u8(255, 245, 160),
            light_intensity: 50.0,
        }),
        EffectId::Yufitelhit => Some(SpecialEffectRecipe::Str {
            path: "windhit1.str",
            light_color: Color::rgb_u8(255, 240, 150),
            light_intensity: 40.0,
        }),

        // --- Common STR-backed IDs already used elsewhere ---
        EffectId::Magnumbreak => Some(SpecialEffectRecipe::Burst {
            style: SkillBurstStyle::MagnumBreak,
            texture: "effect\\ring_yellow.tga",
            secondary: Some("effect\\대폭발.tga"),
        }),
        EffectId::Stormgust => Some(SpecialEffectRecipe::Str {
            path: "stormgust.str",
            light_color: Color::rgb_u8(175, 225, 255),
            light_intensity: 55.0,
        }),
        EffectId::Firewall => Some(SpecialEffectRecipe::Str {
            path: "firewall1.str",
            light_color: Color::rgb_u8(255, 45, 10),
            light_intensity: 50.0,
        }),
        EffectId::Sanctuary => Some(SpecialEffectRecipe::Str {
            path: "sanctuary.str",
            light_color: Color::rgb_u8(130, 255, 175),
            light_intensity: 40.0,
        }),
        EffectId::Magnus => Some(SpecialEffectRecipe::Str {
            path: "magnus.str",
            light_color: Color::rgb_u8(255, 225, 170),
            light_intensity: 45.0,
        }),
        EffectId::Quagmire => Some(SpecialEffectRecipe::Str {
            path: "quagmire.str",
            light_color: Color::rgb_u8(135, 105, 75),
            light_intensity: 30.0,
        }),
        EffectId::Meteorstorm => Some(SpecialEffectRecipe::Str {
            path: "meteor1.str",
            light_color: Color::rgb_u8(255, 95, 25),
            light_intensity: 55.0,
        }),
        EffectId::Lord => Some(SpecialEffectRecipe::Str {
            path: "lord.str",
            light_color: Color::rgb_u8(245, 235, 150),
            light_intensity: 50.0,
        }),
        EffectId::Firehit | EffectId::Firesplashhit => Some(SpecialEffectRecipe::Str {
            path: "firehit1.str",
            light_color: Color::rgb_u8(255, 90, 25),
            light_intensity: 40.0,
        }),
        EffectId::Coldhit => Some(SpecialEffectRecipe::Str {
            path: "chill.str",
            light_color: Color::rgb_u8(150, 225, 255),
            light_intensity: 35.0,
        }),
        EffectId::Windhit => Some(SpecialEffectRecipe::Str {
            path: "windhit1.str",
            light_color: Color::rgb_u8(255, 240, 150),
            light_intensity: 35.0,
        }),
        EffectId::Earthhit => Some(SpecialEffectRecipe::Str {
            path: "earthhit.str",
            light_color: Color::rgb_u8(215, 180, 110),
            light_intensity: 35.0,
        }),
        EffectId::Holyhit => Some(SpecialEffectRecipe::Str {
            path: "holyhit.str",
            light_color: Color::rgb_u8(255, 245, 190),
            light_intensity: 35.0,
        }),
        EffectId::Thunderstorm | EffectId::Lightbolt => Some(SpecialEffectRecipe::Str {
            path: "thunderstorm.str",
            light_color: Color::rgb_u8(255, 240, 150),
            light_intensity: 50.0,
        }),
        EffectId::Sonicblow | EffectId::Sonicblow2 => Some(SpecialEffectRecipe::Str {
            path: "sonicblow.str",
            light_color: Color::rgb_u8(220, 210, 255),
            light_intensity: 35.0,
        }),
        EffectId::Brandishspear | EffectId::Brandish2 => Some(SpecialEffectRecipe::Str {
            path: "brandish.str",
            light_color: Color::rgb_u8(235, 220, 180),
            light_intensity: 40.0,
        }),
        EffectId::Pierce | EffectId::Pierceself => Some(SpecialEffectRecipe::Str {
            path: "pierce.str",
            light_color: Color::rgb_u8(235, 220, 180),
            light_intensity: 35.0,
        }),

        // Cast-target reticle / begin-cast auras (E3 cast-circle slice).
        EffectId::Lockon => Some(SpecialEffectRecipe::Burst {
            style: SkillBurstStyle::Raid,
            texture: "effect\\ring_yellow.tga",
            secondary: None,
        }),
        EffectId::Beginspell
        | EffectId::Beginspell2
        | EffectId::Beginspell3
        | EffectId::Beginspell4
        | EffectId::Beginspell5
        | EffectId::Beginspell6
        | EffectId::Beginspell7 => Some(SpecialEffectRecipe::Burst {
            style: SkillBurstStyle::SonicBlow,
            texture: "effect\\ring_blue.tga",
            secondary: None,
        }),

        // Priest / support / hunter / trap hits used by campaign jobs.
        EffectId::Healsp | EffectId::Recovery => Some(SpecialEffectRecipe::Str {
            path: "holyhit.str",
            light_color: Color::rgb_u8(255, 245, 190),
            light_intensity: 40.0,
        }),
        EffectId::Blessing | EffectId::Incagility | EffectId::Angelus | EffectId::Gloria | EffectId::Magnificat => {
            Some(SpecialEffectRecipe::Str {
                path: "holyhit.str",
                light_color: Color::rgb_u8(255, 245, 190),
                light_intensity: 35.0,
            })
        }
        EffectId::Resurrection => Some(SpecialEffectRecipe::Str {
            path: "holyhit.str",
            light_color: Color::rgb_u8(255, 255, 220),
            light_intensity: 50.0,
        }),
        EffectId::Bowlingbash | EffectId::Bowlingself => Some(SpecialEffectRecipe::Burst {
            style: SkillBurstStyle::MeleeHit,
            texture: "effect\\lens1.tga",
            secondary: Some("effect\\lens2.tga"),
        }),
        EffectId::Spearbmr | EffectId::Spearbmrself => Some(SpecialEffectRecipe::Str {
            path: "spearboomerang.str",
            light_color: Color::rgb_u8(235, 220, 180),
            light_intensity: 35.0,
        }),
        EffectId::Bash => Some(SpecialEffectRecipe::Burst {
            style: SkillBurstStyle::MeleeHit,
            texture: "effect\\lens1.tga",
            secondary: None,
        }),
        EffectId::Crashearth => Some(SpecialEffectRecipe::Str {
            path: "crashearth.str",
            light_color: Color::rgb_u8(235, 190, 95),
            light_intensity: 40.0,
        }),
        EffectId::Venomdust => Some(SpecialEffectRecipe::Str {
            path: "venomdust.str",
            light_color: Color::rgb_u8(140, 75, 190),
            light_intensity: 30.0,
        }),
        EffectId::Skidtrap | EffectId::Blastminebomb | EffectId::Claymore | EffectId::Freezing | EffectId::Sandman => {
            Some(SpecialEffectRecipe::Burst {
                style: SkillBurstStyle::MeleeHit,
                texture: "effect\\ring_yellow.tga",
                secondary: None,
            })
        }
        EffectId::Pneuma => Some(SpecialEffectRecipe::Str {
            // No dedicated STR in many GRFs; holy flash stands in.
            path: "holyhit.str",
            light_color: Color::rgb_u8(200, 220, 255),
            light_intensity: 35.0,
        }),
        EffectId::Icewall => Some(SpecialEffectRecipe::Str {
            path: "freeze.str",
            light_color: Color::rgb_u8(150, 225, 255),
            light_intensity: 40.0,
        }),

        _ => None,
    }
}

/// Semantic shape for an effect ID even when no recipe is wired yet.
/// Used for diagnostics and future recipe design.
#[allow(dead_code)]
pub fn effect_shape_hint(effect_id: EffectId) -> Option<EffectShape> {
    if let Some(recipe) = special_effect_recipe(effect_id) {
        return Some(recipe.shape());
    }
    match effect_id {
        EffectId::Soulstrike | EffectId::Soulstrike2 | EffectId::Cone | EffectId::Sphere => Some(EffectShape::Orb),
        EffectId::Napalmbeat | EffectId::Magnumbreak | EffectId::Barrier => Some(EffectShape::Ring),
        EffectId::Earthspike | EffectId::Heavensdrive | EffectId::Icewall => Some(EffectShape::Spike),
        EffectId::Fireball
        | EffectId::Fireball2
        | EffectId::Fireball3
        | EffectId::Frostdiver
        | EffectId::Yufitel
        | EffectId::Yufitel2
        | EffectId::Waterball
        | EffectId::Waterball2 => Some(EffectShape::Ball),
        EffectId::Firehit
        | EffectId::Coldhit
        | EffectId::Windhit
        | EffectId::Earthhit
        | EffectId::Holyhit
        | EffectId::Yufitelhit
        | EffectId::Frostdiver2 => Some(EffectShape::Flash),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e1_native_ids_are_mapped() {
        assert!(special_effect_recipe(EffectId::Napalmbeat).is_some());
        assert!(special_effect_recipe(EffectId::Soulstrike).is_some());
        assert!(special_effect_recipe(EffectId::Fireball).is_some());
        assert!(special_effect_recipe(EffectId::Frostdiver2).is_some());
        assert!(special_effect_recipe(EffectId::Yufitel).is_some());
        assert!(special_effect_recipe(EffectId::Earthspike).is_some());
        assert!(special_effect_recipe(EffectId::Heavensdrive).is_some());
    }

    #[test]
    fn shape_hints_match_semantic_families() {
        // Mapped recipes report their wired presentation family first.
        assert_eq!(effect_shape_hint(EffectId::Soulstrike), Some(EffectShape::Str));
        assert_eq!(effect_shape_hint(EffectId::Napalmbeat), Some(EffectShape::Ring));
        assert_eq!(effect_shape_hint(EffectId::Earthspike), Some(EffectShape::Spike));
        assert_eq!(effect_shape_hint(EffectId::Fireball), Some(EffectShape::Str));
        assert_eq!(effect_shape_hint(EffectId::Yufitelhit), Some(EffectShape::Str));
        // Unmapped-but-known families fall back to the semantic table.
        assert_eq!(effect_shape_hint(EffectId::Waterball), Some(EffectShape::Ball));
        assert_eq!(effect_shape_hint(EffectId::Cone), Some(EffectShape::Orb));
    }

    #[test]
    fn unmapped_id_stays_explicit_none() {
        assert!(special_effect_recipe(EffectId::Max).is_none());
    }
}
