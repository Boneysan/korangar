//! Phase 9 — Seal Cascade DM command contracts.

use korangar_networking::NetworkEvent;

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("dm-roll", 9, dm_roll),
        Scenario::new("dm-roll-hidden", 9, dm_roll_hidden),
        Scenario::new("dm-roll-override", 9, dm_roll_override),
        Scenario::new("dm-command-help", 9, dm_command_help),
    ]
}

fn wait_for_text(context: &mut TestContext, label: &str, needle: &str) -> Result<String, String> {
    context.wait_for(label, |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains(needle) => Some(text.clone()),
        _ => None,
    })
}

fn dm_roll(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.flush();
    context.say("@roll 2d6+3")?;
    let text = wait_for_text(&mut context, "public dice result", "rolled 2d6+3:")?;
    let total = text
        .split_once("rolled 2d6+3:")
        .and_then(|(_, total)| total.trim().split_whitespace().next())
        .and_then(|total| total.parse::<i32>().ok())
        .ok_or_else(|| format!("could not parse roll total from {text:?}"))?;
    if !(5..=15).contains(&total) {
        return Err(format!("2d6+3 total {total} is outside 5..=15"));
    }
    Ok(())
}

fn dm_roll_hidden(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.flush();
    context.say("@roll hidden 1d20+1")?;
    wait_for_text(&mut context, "hidden dice result", "1d20+1")?;
    Ok(())
}

fn dm_roll_override(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.flush();
    context.say("@roll override 17 headless-check")?;
    let text = wait_for_text(&mut context, "transparent overridden roll", "17")?;
    if !text.contains("headless-check") {
        return Err(format!("override result omitted its audit note: {text:?}"));
    }
    Ok(())
}

fn dm_command_help(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    // Only read-only commands belong in this contract test. Several shortcut
    // commands intentionally perform a default action when invoked without
    // arguments (for example, @dmreward grants loot).
    for (command, expected) in [("@dm", "[DM]"), ("@dm help", "[DM]"), ("@dmstatus", "[DM]")] {
        context.flush();
        context.say(command)?;
        wait_for_text(&mut context, &format!("feedback for {command}"), expected)?;
    }
    Ok(())
}
