//! Phase 2 — GM command channel round trips.

use std::time::Duration;

use korangar_networking::{InventoryItemDetails, MessageColor, NetworkEvent};
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData, HotkeyType, ItemId, SkillId, StatType};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("gm-feedback", 2, gm_feedback),
        Scenario::new("gm-job", 2, gm_job),
        Scenario::new("gm-level", 2, gm_level),
        Scenario::new("gm-allskill", 2, gm_allskill),
        Scenario::new("provision-effect-roster", 2, provision_effect_roster),
        Scenario::new("gm-item", 2, gm_item),
        Scenario::new("gm-zeny", 2, gm_zeny),
        Scenario::new("gm-warp", 2, gm_warp),
        Scenario::new("gm-monster", 2, gm_monster),
    ]
}

/// Commands must produce server-styled feedback, NOT echo as public chat —
/// an echo means the account lacks GM rights (test-setup error).
fn gm_feedback(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    context.flush();
    context.say("@commands")?;
    let mut saw_public_echo = false;
    let result = context.wait_for("server feedback for @commands", |event| match event {
        NetworkEvent::ChatMessage {
            color: MessageColor::Server | MessageColor::Information,
            ..
        } => Some(()),
        NetworkEvent::ChatMessage {
            text,
            color: MessageColor::Broadcast,
        } if text.contains("@commands") => {
            saw_public_echo = true;
            None
        }
        _ => None,
    });

    if saw_public_echo {
        return Err("@commands echoed as public chat — account is not GM (group_id 99 required)".to_owned());
    }
    result?;
    Ok(())
}

/// `@job 4010` → `ChangeJob { job_id: 4010 }` (and back to novice).
fn gm_job(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    // Force a transition even if a previous run left us as High Wizard.
    if context.job_id.0 == 4010 {
        context.ensure_job(0)?;
    }
    context.ensure_job(4010)?;
    context.ensure_job(0)?;
    Ok(())
}

/// `@blevel` (relative) reaches an exact target level.
fn gm_level(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.ensure_base_level(50)?;
    context.ensure_base_level(99)?;
    Ok(())
}

/// `@allskill` populates the skill tree beyond the novice basics.
fn gm_allskill(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.ensure_job(4010)?;

    context.flush();
    context.say("@allskill")?;
    let skill_count = context.wait_for("SkillTree after @allskill", |event| match event {
        NetworkEvent::SkillTree { skill_information } if skill_information.len() > 10 => Some(skill_information.len()),
        _ => None,
    })?;
    println!("    skill tree has {skill_count} skills");
    context.ensure_job(0)?;
    Ok(())
}

/// Populate the persistent GUI characters used for the classic skill-effect
/// acceptance pass. Each character keeps its intended class, receives the
/// complete server skill tree (including prerequisites), and gets any weapon
/// types needed to exercise the effect-covered skills.
fn provision_effect_roster(config: &Config) -> Result<(), String> {
    struct CharacterSetup {
        name: &'static str,
        job_id: u16,
        expected_skills: &'static [u16],
        weapons: &'static [u32],
        /// Catalyst items consumed by the bound skills, topped up to the
        /// given count on every provisioning run.
        consumables: &'static [(u32, u16)],
    }

    const ROSTER: &[CharacterSetup] = &[
        CharacterSetup {
            name: "EffectKnight",
            job_id: 7,
            expected_skills: &[7, 56, 57, 58, 59, 62],
            // Sword for Magnum Break/Bowling Bash and spear for the spear set.
            // Shields: Guard (view 1), Buckler (2), Shield (3) — C5 path probe.
            weapons: &[1101, 1404, 2101, 2103, 2105],
            consumables: &[],
        },
        CharacterSetup {
            name: "EffectSinX",
            job_id: 4013,
            // Sonic Blow, Meteor Assault, and Enchant Deadly Poison — EDP
            // consumes a Poison Bottle per cast (fail reason 71 without one).
            expected_skills: &[136, 406, 378],
            weapons: &[1250],
            consumables: &[(678, 20)],
        },
        CharacterSetup {
            name: "EffectStalker",
            job_id: 4018,
            expected_skills: &[214],
            weapons: &[1101],
            consumables: &[],
        },
        CharacterSetup {
            name: "EffectRune",
            job_id: 4054,
            expected_skills: &[2006],
            // Rune Knight + sword + classic shields for C5.
            weapons: &[1101, 1404, 2101, 2103, 2105],
            consumables: &[],
        },
    ];

    for setup in ROSTER {
        // Create the roster character on a free slot if it does not exist yet.
        let mut context = match TestContext::connect_as(
            config,
            &config.username,
            &config.password,
            Some(setup.name),
            None,
        ) {
            Ok(context) => context,
            Err(error) if error.contains("not found") => {
                println!("    {}: creating character ({error})", setup.name);
                let mut bootstrap =
                    TestContext::connect_as(config, &config.username, &config.password, None, None)?;
                let free_slot = (0..9u8)
                    .find(|slot| {
                        !bootstrap
                            .characters
                            .iter()
                            .any(|info| info.character_number == *slot)
                    })
                    .ok_or_else(|| format!("no free character slot for {}", setup.name))?;
                bootstrap
                    .net
                    .create_character(free_slot as usize, setup.name.to_owned())
                    .map_err(|_| format!("disconnected creating {}", setup.name))?;
                let info = bootstrap.wait_for("CharacterCreated", |event| match event {
                    NetworkEvent::CharacterCreated { character_information } => {
                        Some(Ok(character_information.clone()))
                    }
                    NetworkEvent::CharacterCreationFailed { message, .. } => {
                        Some(Err(format!("creation failed for {}: {message}", setup.name)))
                    }
                    _ => None,
                })??;
                println!(
                    "    {}: created in slot {}",
                    setup.name, info.character_number
                );
                // Drop bootstrap session cleanly, then log in as the new character.
                drop(bootstrap);
                std::thread::sleep(Duration::from_millis(700));
                TestContext::connect_as(
                    config,
                    &config.username,
                    &config.password,
                    Some(setup.name),
                    None,
                )?
            }
            Err(error) => return Err(error),
        };

        context.ensure_base_level(99)?;
        context.ensure_job(setup.job_id)?;
        if context.job_id.0 != setup.job_id {
            return Err(format!(
                "{} has job {}, expected {}",
                setup.name, context.job_id.0, setup.job_id
            ));
        }

        context.flush();
        context.say("@allskill")?;
        let skills = context.wait_for("SkillTree after roster @allskill", |event| match event {
            NetworkEvent::SkillTree { skill_information } if skill_information.len() > 10 => Some(skill_information.clone()),
            _ => None,
        })?;
        for &skill_id in setup.expected_skills {
            if !skills.iter().any(|skill| skill.skill_id == SkillId(skill_id)) {
                return Err(format!("{} is missing required skill {} after @allskill", setup.name, skill_id));
            }
        }

        // Spare zeny for shops/repairs during live GUI testing.
        let _ = context.say("@zeny 5000000");
        context.pump(Duration::from_millis(200));

        for &item_id in setup.weapons {
            if !context.inventory.iter().any(|item| item.item_id == ItemId(item_id)) {
                context.give_item(item_id, 1)?;
            }
        }

        for &(item_id, target_count) in setup.consumables {
            let held = context
                .inventory
                .iter()
                .filter(|item| item.item_id == ItemId(item_id))
                .fold(0u16, |total, item| {
                    total.saturating_add(match &item.details {
                        InventoryItemDetails::Regular { amount, .. } => *amount,
                        InventoryItemDetails::Equippable { .. } => 1,
                    })
                });
            if held < target_count {
                context.give_item(item_id, target_count - held)?;
            }
        }

        // Effect-roster map for live skill/shield checks.
        let _ = context.say("@go prt_fild07");
        context.pump(Duration::from_millis(500));

        for (slot, &skill_id) in setup.expected_skills.iter().enumerate() {
            let skill_level = skills
                .iter()
                .find(|skill| skill.skill_id == SkillId(skill_id))
                .expect("required roster skill was checked above")
                .skill_level;
            context
                .net
                .set_hotkey_data(HotbarTab(0), HotbarSlot(slot as u16), HotkeyData {
                    hotkey_type: HotkeyType::Skill,
                    item_or_skill_id: skill_id as u32,
                    quantity_or_skill_level: skill_level.0,
                })
                .map_err(|_| format!("disconnected while binding {} skill {}", setup.name, skill_id))?;
        }
        context.pump(Duration::from_millis(300));
        println!(
            "    {}: job {} lv {}; bound {} skills; stocked {} gear items",
            setup.name,
            context.job_id.0,
            context.base_level,
            setup.expected_skills.len(),
            setup.weapons.len()
        );
    }
    Ok(())
}

/// `@item 501 5` → inventory add with matching id and count.
fn gm_item(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let index = context.give_item(501, 5)?;
    let item = context
        .inventory
        .iter()
        .find(|item| item.index == index)
        .ok_or("added item not tracked in inventory")?;
    if item.item_id.0 != 501 {
        return Err(format!("expected item 501, got {}", item.item_id.0));
    }
    // Clean up so repeated runs don't accumulate potions.
    context.say("@delitem 501 5")?;
    context.pump(Duration::from_millis(300));
    Ok(())
}

/// `@zeny` adjusts the zeny stat: server-relative arithmetic must round trip.
fn gm_zeny(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    context.flush();
    context.say("@zeny 1000")?;
    let after_add = context.wait_for("UpdateStat zeny (add)", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::Zeny(value),
        } => Some(*value),
        _ => None,
    })?;

    context.flush();
    context.say("@zeny -1000")?;
    context.wait_for("UpdateStat zeny (subtract)", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::Zeny(value),
        } if *value == after_add - 1000 => Some(()),
        _ => None,
    })?;
    Ok(())
}

/// `@warp` round trips through `ChangeMap` with the right map and position.
fn gm_warp(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("geffen", 119, 59)?;
    if context.map_name != "geffen" {
        return Err(format!("tracked map is {:?}", context.map_name));
    }
    context.warp("prontera", 155, 180)?;
    Ok(())
}

/// `@monster PORING` → `AddEntity` with mob id 1002; `@killmonster` removes it.
fn gm_monster(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;

    let entity_id = context.spawn_monster("PORING", 1002)?;

    context.flush();
    context.say("@killmonster")?;
    context.wait_for("RemoveEntity for the poring", |event| match event {
        NetworkEvent::RemoveEntity { entity_id: removed, .. } if removed.0 == entity_id.0 => Some(()),
        _ => None,
    })?;
    Ok(())
}
