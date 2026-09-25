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
        Scenario::new("quest-reviewed-brasilis-npc-routes", 7, quest_reviewed_brasilis_npc_routes),
    ]
}

/// Verify both source-reviewed Guide routes against the live Brasilis NPC:
/// Angelo offers 9030 in dialogue, then handles the 9031 turn-in and starts
/// the repeat cooldown quest. Quest setup for the turn-in is deliberately
/// synthetic (`@quest add`); objective completion is not claimed here.
fn quest_reviewed_brasilis_npc_routes(config: &Config) -> Result<(), String> {
    const OFFER_QUEST: u32 = 9030;
    const TURN_IN_QUEST: u32 = 9031;
    const COOLDOWN_QUEST: u32 = 9032;
    const NPC_X: u16 = 297;
    const NPC_Y: u16 = 307;

    let mut context = TestContext::connect(config)?;
    context.ensure_base_level(40)?;
    let mut npc_id_for_cleanup = None;
    let result = (|| {
        for quest_id in [OFFER_QUEST, TURN_IN_QUEST, COOLDOWN_QUEST] {
            context.say(&format!("@quest del {quest_id}"))?;
        }
        context.warp("brasilis", NPC_X.saturating_sub(3), NPC_Y)?;
        context.pump(Duration::from_millis(300));
        let npc_id = context
            .entities
            .iter()
            .find(|(_, entity)| {
                let position = entity.position.tile_position();
                position.x == NPC_X && position.y == NPC_Y
            })
            .map(|(id, _)| *id)
            .ok_or("Angelo#br was not visible at reviewed cell brasilis (297,307)")?;
        npc_id_for_cleanup = Some(npc_id);

        context.flush();
        context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;
        let (offer_text, offer_events) = collect_dialog_page(&mut context, npc_id, "Angelo quest offer dialogue")?;
        if !offer_text.contains("Puppies have been disappearing") {
            return Err(format!("reviewed offer NPC produced unexpected dialogue page: {offer_text:?}"));
        }
        if !offer_events
            .iter()
            .any(|event| matches!(event, NetworkEvent::AddNextButton { npc_id: id } if *id == npc_id))
        {
            return Err("Angelo offer page did not end with a next button".to_owned());
        }
        context.flush();
        context.net.next_dialog(npc_id).map_err(|_| "disconnected")?;
        let (task_text, task_events) = collect_dialog_page(&mut context, npc_id, "Angelo quest task dialogue")?;
        if !task_text.contains("find") || !task_text.contains("puppies") {
            return Err(format!("reviewed offer NPC produced unexpected task text: {task_text:?}"));
        }
        if !task_events
            .iter()
            .any(|event| matches!(event, NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == OFFER_QUEST))
        {
            return Err("Angelo's offer dialogue did not add quest 9030".to_owned());
        }
        if !task_events
            .iter()
            .any(|event| matches!(event, NetworkEvent::AddCloseButton { npc_id: id } if *id == npc_id))
        {
            return Err("Angelo quest offer page did not end with a close button".to_owned());
        }
        context.flush();
        context.net.close_dialog(npc_id).map_err(|_| "disconnected")?;

        context.say(&format!("@quest del {OFFER_QUEST}"))?;
        context.say(&format!("@quest add {TURN_IN_QUEST}"))?;
        context.wait_for("synthetic quest 9031 setup", |event| match event {
            NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == TURN_IN_QUEST => Some(()),
            _ => None,
        })?;
        context.flush();
        context.net.start_dialog(npc_id).map_err(|_| "disconnected")?;
        let (turn_in_text, turn_in_events) = collect_dialog_page(&mut context, npc_id, "Angelo quest turn-in dialogue")?;
        if !turn_in_text.contains("found all of 3 puppies") {
            return Err(format!("reviewed turn-in NPC produced unexpected dialogue: {turn_in_text:?}"));
        }
        if !turn_in_events
            .iter()
            .any(|event| matches!(event, NetworkEvent::QuestRemoved { quest_id } if *quest_id == TURN_IN_QUEST))
        {
            return Err("Angelo's turn-in dialogue did not remove quest 9031".to_owned());
        }
        if !turn_in_events
            .iter()
            .any(|event| matches!(event, NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == COOLDOWN_QUEST))
        {
            return Err("Angelo's turn-in dialogue did not start cooldown quest 9032".to_owned());
        }

        Ok(())
    })();
    if let Some(npc_id) = npc_id_for_cleanup {
        let _ = context.net.close_dialog(npc_id);
    }
    for quest_id in [OFFER_QUEST, TURN_IN_QUEST, COOLDOWN_QUEST] {
        let _ = context.say(&format!("@quest del {quest_id}"));
    }
    context.pump(Duration::from_millis(100));
    result
}

/// NPC scripts send the speaker label and each `mes` as separate dialog text
/// events. Collect a complete page before asserting its player-facing content.
fn collect_dialog_page(context: &mut TestContext, npc_id: EntityId, description: &str) -> Result<(String, Vec<NetworkEvent>), String> {
    let first = context.wait_for(description, |event| match event {
        NetworkEvent::OpenDialog { npc_id: id, text } if *id == npc_id => Some(text.clone()),
        _ => None,
    })?;
    let events = context.collect_for(Duration::from_millis(200));
    let mut text = first;
    for event in &events {
        if let NetworkEvent::OpenDialog { npc_id: id, text: chunk } = event
            && *id == npc_id
        {
            text.push('\n');
            text.push_str(chunk);
        }
    }
    Ok((text, events))
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
