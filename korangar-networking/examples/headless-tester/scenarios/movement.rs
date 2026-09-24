//! Phase 3 — movement and world state.

use korangar_networking::NetworkEvent;
use ragnarok_packets::{Direction, WorldPosition};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("walk", 3, walk),
        Scenario::new("warp-crossmap", 3, warp_crossmap),
        Scenario::new("navigation-warp-traversal", 3, navigation_warp_traversal),
        Scenario::new("entity-details", 3, entity_details),
        Scenario::new("sit-stand", 3, sit_stand),
        Scenario::new("tick-sync", 3, tick_sync),
    ]
}

/// Request a walk and verify the server paths us to the exact tile.
fn walk(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;

    let target = (context.position.x + 7, context.position.y);
    context.flush();
    context
        .net
        .player_move(WorldPosition::new(target.0, target.1, Direction::North))
        .map_err(|_| "disconnected")?;

    let destination = context.wait_for("PlayerMove", |event| match event {
        NetworkEvent::PlayerMove { destination, .. } => Some(destination.tile_position()),
        _ => None,
    })?;

    if (destination.x, destination.y) != target {
        return Err(format!(
            "asked for {:?}, server pathed to {:?}",
            target,
            (destination.x, destination.y)
        ));
    }
    Ok(())
}

/// Cross-map warp repopulates the entity set for the new map.
fn warp_crossmap(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;
    let prontera_entities = context.entities.len();

    context.warp("geffen", 119, 59)?;

    // `warp` waits for ChangeMap (which clears entities) and pumps; geffen
    // has NPCs around the fountain, so new AddEntity events must have arrived.
    if context.entities.is_empty() {
        return Err(format!(
            "no entities after cross-map warp (prontera had {prontera_entities}) — AddEntity stream missing"
        ));
    }
    Ok(())
}

/// Traverse both directions of the graph's Prontera ↔ prt_fild08 route using
/// ordinary movement, then verify the live server's map-change destinations.
fn navigation_warp_traversal(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    // Static graph edge: prontera (156,22), 3x2 -> prt_fild08 (170,375).
    context.warp("prontera", 156, 26)?;
    context.flush();
    context
        .net
        .player_move(WorldPosition::new(157, 22, Direction::North))
        .map_err(|_| "disconnected")?;
    let field_position = context.wait_for("navigation portal to prt_fild08", |event| match event {
        NetworkEvent::ChangeMap { map_name, position } if map_name == "prt_fild08" => Some(*position),
        _ => None,
    })?;
    if field_position.x.abs_diff(170) > 3 || field_position.y.abs_diff(375) > 3 {
        return Err(format!(
            "Prontera portal arrived at unexpected prt_fild08 cell ({}, {})",
            field_position.x, field_position.y
        ));
    }

    // Return edge: prt_fild08 (170,378), 3x2 -> prontera (156,26).
    context.warp("prt_fild08", 170, 374)?;
    context.flush();
    context
        .net
        .player_move(WorldPosition::new(171, 378, Direction::South))
        .map_err(|_| "disconnected")?;
    let prontera_position = context.wait_for("navigation portal back to prontera", |event| match event {
        NetworkEvent::ChangeMap { map_name, position } if map_name == "prontera" => Some(*position),
        _ => None,
    })?;
    if prontera_position.x.abs_diff(156) > 3 || prontera_position.y.abs_diff(26) > 3 {
        return Err(format!(
            "prt_fild08 portal arrived at unexpected Prontera cell ({}, {})",
            prontera_position.x, prontera_position.y
        ));
    }
    Ok(())
}

/// `RequestDetailsPacket` round trip resolves a monster's name.
fn entity_details(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.warp("prontera", 155, 180)?;
    let entity_id = context.spawn_monster("PORING", 1002)?;

    context.flush();
    context.net.entity_details(entity_id).map_err(|_| "disconnected")?;
    let name = context.wait_for("UpdateEntityDetails", |event| match event {
        NetworkEvent::UpdateEntityDetails { entity_id: id, name } if id.0 == entity_id.0 => Some(name.clone()),
        _ => None,
    })?;

    context.kill_all_monsters();

    if !name.eq_ignore_ascii_case("poring") {
        return Err(format!("expected name Poring, got {name:?}"));
    }
    Ok(())
}

/// Sit down / stand up round trips with our own entity id.
fn sit_stand(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    // Sitting requires the Basic Skill; grant it and wait for the tree update.
    context.flush();
    context.say("@allskill")?;
    context.wait_for("SkillTree after @allskill", |event| match event {
        NetworkEvent::SkillTree { skill_information } if !skill_information.is_empty() => Some(()),
        _ => None,
    })?;

    let player_id = context.player_id;

    context.flush();
    context.net.player_sit().map_err(|_| "disconnected")?;
    context.wait_for("PlayerSitDown", |event| match event {
        NetworkEvent::PlayerSitDown { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.player_stand().map_err(|_| "disconnected")?;
    context.wait_for("PlayerStandUp", |event| match event {
        NetworkEvent::PlayerStandUp { entity_id } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    })?;
    Ok(())
}

/// Tick request keeps client/server clocks in sync.
fn tick_sync(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.flush();
    context.net.request_client_tick().map_err(|_| "disconnected")?;
    context.wait_for("UpdateClientTick", |event| match event {
        NetworkEvent::UpdateClientTick { .. } => Some(()),
        _ => None,
    })?;
    Ok(())
}
