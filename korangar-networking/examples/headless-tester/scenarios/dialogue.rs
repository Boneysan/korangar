//! Phase 7 — NPC dialogue scenarios.

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::EntityId;

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("dialogue-linear", 7, dialogue_linear),
        Scenario::new("dialogue-choice", 7, dialogue_choice),
        Scenario::new("dialogue-number", 7, dialogue_number),
        Scenario::new("dialogue-string", 7, dialogue_string),
        Scenario::new("dialogue-warp", 7, dialogue_warp),
    ]
}

/// Helper to prepare the character, reload scripts, warp to Prontera, and
/// locate the test NPC.
fn prepare_npc(config: &Config) -> Result<(TestContext, EntityId), String> {
    let mut context = TestContext::connect(config)?;

    // Reload scripts to ensure dialogue test NPC is loaded.
    context.say("@reloadscript")?;
    context.pump(Duration::from_millis(1000));

    // Warp next to the test NPC.
    context.warp("prontera", 160, 200)?;

    let npc_id = match context
        .entities
        .iter()
        .find(|(_, data)| {
            let pos = data.position.tile_position();
            (pos.x as i32 - 160).abs() <= 3 && (pos.y as i32 - 200).abs() <= 3
        })
        .map(|(id, _)| *id)
    {
        Some(id) => id,
        None => {
            println!("Visible entities: {:?}", context.entities);
            return Err("Dialogue Test NPC not found near (160, 200)".to_owned());
        }
    };

    Ok((context, npc_id))
}

fn dialogue_linear(config: &Config) -> Result<(), String> {
    let (mut context, npc_id) = prepare_npc(config)?;

    context.flush();
    context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;

    // First greeting screen
    let text = context.wait_for("OpenDialog (greeting)", |event| match event {
        NetworkEvent::OpenDialog { text, npc_id: id } if *id == npc_id => Some(text.clone()),
        _ => None,
    })?;
    if !text.contains("Dialogue Test NPC") {
        return Err(format!("Unexpected greeting text: {}", text));
    }

    context.wait_for("AddNextButton (greeting)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Go to next screen (scenario select)
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    context.wait_for("AddChoiceButtons (select)", |event| match event {
        NetworkEvent::AddChoiceButtons { npc_id: id, .. } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Select Option 1 (Linear Dialogue)
    context.flush();
    context.net.choose_dialog_option(npc_id, 1).map_err(|_| "disconnected")?;

    // You chose Linear Dialogue screen
    let text = context.wait_for("OpenDialog (linear confirm)", |event| match event {
        NetworkEvent::OpenDialog { text, npc_id: id } if *id == npc_id => Some(text.clone()),
        _ => None,
    })?;
    if !text.contains("You chose Linear Dialogue") {
        return Err(format!("Unexpected linear confirm text: {}", text));
    }

    context.wait_for("AddNextButton (linear confirm)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Next screen (final linear text)
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    let text = context.wait_for("OpenDialog (final text)", |event| match event {
        NetworkEvent::OpenDialog { text, npc_id: id } if *id == npc_id => Some(text.clone()),
        _ => None,
    })?;
    if !text.contains("This is the next screen") {
        return Err(format!("Unexpected final linear text: {}", text));
    }

    context.wait_for("AddCloseButton (final)", |event| match event {
        NetworkEvent::AddCloseButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Close the dialogue
    context.flush();
    context.net.close_dialog(npc_id).map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(200));

    Ok(())
}

fn dialogue_choice(config: &Config) -> Result<(), String> {
    let (mut context, npc_id) = prepare_npc(config)?;

    context.flush();
    context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;

    // Skip greeting screen
    context.wait_for("AddNextButton (greeting)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    // Choice screen
    let choices = context.wait_for("AddChoiceButtons (select)", |event| match event {
        NetworkEvent::AddChoiceButtons { choices, npc_id: id } if *id == npc_id => Some(choices.clone()),
        _ => None,
    })?;

    let expected = vec![
        "Linear Dialogue".to_owned(),
        "Number Input".to_owned(),
        "String Input".to_owned(),
        "Dialogue Warp".to_owned(),
    ];
    if choices != expected {
        return Err(format!("Choices mismatch. Got: {:?}", choices));
    }

    // Choose option 1 to end dialogue cleanly
    context.flush();
    context.net.choose_dialog_option(npc_id, 1).map_err(|_| "disconnected")?;

    context.wait_for("AddNextButton (linear confirm)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    context.wait_for("AddCloseButton (final)", |event| match event {
        NetworkEvent::AddCloseButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.close_dialog(npc_id).map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(200));

    Ok(())
}

fn dialogue_number(config: &Config) -> Result<(), String> {
    let (mut context, npc_id) = prepare_npc(config)?;

    context.flush();
    context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;

    // Skip greeting screen
    context.wait_for("AddNextButton (greeting)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    // Choice screen -> Select Option 2 (Number Input)
    context.wait_for("AddChoiceButtons (select)", |event| match event {
        NetworkEvent::AddChoiceButtons { npc_id: id, .. } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.choose_dialog_option(npc_id, 2).map_err(|_| "disconnected")?;

    // Wait for "Please input a number:" screen
    context.wait_for("AddNextButton (number prompt)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    // Wait for request number input packet
    context.wait_for("NpcRequestNumberInput", |event| match event {
        NetworkEvent::NpcRequestNumberInput { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Submit number 42
    context.flush();
    context.net.submit_dialog_number(npc_id, 42).map_err(|_| "disconnected")?;

    // Wait for confirmation text "You entered number: 42"
    let text = context.wait_for("OpenDialog (number confirmation)", |event| match event {
        NetworkEvent::OpenDialog { text, npc_id: id } if *id == npc_id => Some(text.clone()),
        _ => None,
    })?;
    if !text.contains("You entered number: 42") {
        return Err(format!("Unexpected confirmation text: {}", text));
    }

    context.wait_for("AddCloseButton (number final)", |event| match event {
        NetworkEvent::AddCloseButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Close the dialogue
    context.flush();
    context.net.close_dialog(npc_id).map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(200));

    Ok(())
}

fn dialogue_string(config: &Config) -> Result<(), String> {
    let (mut context, npc_id) = prepare_npc(config)?;

    context.flush();
    context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;

    // Skip greeting screen
    context.wait_for("AddNextButton (greeting)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    // Choice screen -> Select Option 3 (String Input)
    context.wait_for("AddChoiceButtons (select)", |event| match event {
        NetworkEvent::AddChoiceButtons { npc_id: id, .. } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.choose_dialog_option(npc_id, 3).map_err(|_| "disconnected")?;

    // Wait for "Please input a string:" screen
    context.wait_for("AddNextButton (string prompt)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    // Wait for request string input packet
    context.wait_for("NpcRequestStringInput", |event| match event {
        NetworkEvent::NpcRequestStringInput { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Submit string "hello world"
    context.flush();
    context
        .net
        .submit_dialog_string(npc_id, "hello world".to_string())
        .map_err(|_| "disconnected")?;

    // Wait for confirmation text "You entered string: hello world"
    let text = context.wait_for("OpenDialog (string confirmation)", |event| match event {
        NetworkEvent::OpenDialog { text, npc_id: id } if *id == npc_id => Some(text.clone()),
        _ => None,
    })?;
    if !text.contains("You entered string: hello world") {
        return Err(format!("Unexpected confirmation text: {}", text));
    }

    context.wait_for("AddCloseButton (string final)", |event| match event {
        NetworkEvent::AddCloseButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;

    // Close the dialogue
    context.flush();
    context.net.close_dialog(npc_id).map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(200));

    Ok(())
}

fn dialogue_warp(config: &Config) -> Result<(), String> {
    let (mut context, npc_id) = prepare_npc(config)?;

    context.flush();
    context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;

    // Skip greeting screen
    context.wait_for("AddNextButton (greeting)", |event| match event {
        NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;

    // Choice screen -> Select Option 4 (Dialogue Warp)
    context.wait_for("AddChoiceButtons (select)", |event| match event {
        NetworkEvent::AddChoiceButtons { npc_id: id, .. } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.choose_dialog_option(npc_id, 4).map_err(|_| "disconnected")?;

    // Wait for "Warping you to payon..." screen
    context.wait_for("AddCloseButton (warp confirm)", |event| match event {
        NetworkEvent::AddCloseButton { npc_id: id } if *id == npc_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.close_dialog(npc_id).map_err(|_| "disconnected")?;

    // Wait for ChangeMap event to Payon
    context.wait_for("ChangeMap (to payon)", |event| match event {
        NetworkEvent::ChangeMap { map_name, .. } if map_name.contains("payon") => Some(()),
        _ => None,
    })?;

    Ok(())
}
