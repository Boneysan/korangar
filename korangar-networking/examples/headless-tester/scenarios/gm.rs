//! Phase 2 — GM command channel round trips.

use std::time::Duration;

use korangar_networking::{MessageColor, NetworkEvent};
use ragnarok_packets::StatType;

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("gm-feedback", 2, gm_feedback),
        Scenario::new("gm-job", 2, gm_job),
        Scenario::new("gm-level", 2, gm_level),
        Scenario::new("gm-allskill", 2, gm_allskill),
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
