//! Phase 3 — movement and world state.

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::{Direction, WorldPosition};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("walk", 3, walk),
        Scenario::new("warp-crossmap", 3, warp_crossmap),
        Scenario::new("navigation-warp-traversal", 3, navigation_warp_traversal),
        Scenario::new("navigation-izlude-ferry-service", 3, navigation_izlude_ferry_service),
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

    // Static graph edge: prontera (156,22), 3x2 touch radii -> prt_fild08
    // (170,375). Force a cross-map setup even if the reused test character
    // logged in on Prontera; Hercules treats a same-map @warp as a no-op.
    context.warp("geffen", 119, 59)?;
    context.warp("prontera", 156, 80)?;
    if context.map_name != "prontera" {
        return Err(format!("expected Prontera setup, landed on {}", context.map_name));
    }
    context.walk_to(156, 30)?;
    let field_position = walk_through_portal(&mut context, "prt_fild08", &[
        (156, 24),
        (157, 24),
        (155, 24),
        (158, 24),
        (154, 24),
        (159, 24),
    ])?;
    if field_position.x.abs_diff(170) > 3 || field_position.y.abs_diff(375) > 3 {
        return Err(format!(
            "Prontera portal arrived at unexpected prt_fild08 cell ({}, {})",
            field_position.x, field_position.y
        ));
    }

    // Return edge: prt_fild08 (170,378), 3x2 -> prontera (156,26).
    // Cross-map setup ensures @warp can place us near the return edge; a
    // same-map @warp is a no-op in stock Hercules.
    context.warp("prontera", 156, 30)?;
    context.warp("prt_fild08", 170, 370)?;
    let prontera_position = walk_through_portal(&mut context, "prontera", &[
        (170, 376),
        (171, 376),
        (169, 376),
        (172, 376),
        (168, 376),
        (173, 376),
    ])?;
    if prontera_position.x.abs_diff(156) > 3 || prontera_position.y.abs_diff(26) > 3 {
        return Err(format!(
            "prt_fild08 portal arrived at unexpected Prontera cell ({}, {})",
            prontera_position.x, prontera_position.y
        ));
    }
    Ok(())
}

/// Verify Izlude's real NPC service leg to the Byalan waiting area: pay the
/// sailor and validate the server-selected map transfer. The static dungeon
/// entrance is a separate, still-unverified walk-warp edge.
fn navigation_izlude_ferry_service(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    // Start with a cross-map setup because Hercules treats a same-map @warp as
    // a no-op. The renewal sailor NPC is at (197, 205).
    context.warp("geffen", 119, 59)?;
    context.warp("izlude", 197, 198)?;
    if context.map_name != "izlude" {
        return Err(format!("expected Izlude setup, landed on {}", context.map_name));
    }
    let sailor_id = context
        .entities
        .iter()
        .find(|(_, data)| {
            let pos = data.position.tile_position();
            pos.x.abs_diff(197) <= 2 && pos.y.abs_diff(205) <= 2
        })
        .map(|(id, _)| *id)
        .ok_or_else(|| "Izlude Sailor NPC not visible near (197, 205)".to_owned())?;

    // Fresh disposable integration characters start with an empty wallet.
    // Provision fare through the fixture's GM command; this account/database
    // is discarded by the integration runner after the scenario.
    context.say("@zeny 500")?;
    context.wait_for("test-only ferry fare", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::Zeny(value),
        } if *value >= 150 => Some(*value),
        _ => None,
    })?;
    let starting_zeny = context.zeny;
    context.flush();
    context.net.start_dialog(sailor_id).map_err(|_| "disconnected")?;
    context.wait_for("Izlude Sailor greeting", |event| match event {
        NetworkEvent::AddNextButton { npc_id } if *npc_id == sailor_id => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.next_dialog(sailor_id).map_err(|_| "disconnected")?;
    let choices = context.wait_for("Izlude Sailor travel choices", |event| match event {
        NetworkEvent::AddChoiceButtons { npc_id, choices } if *npc_id == sailor_id => Some(choices.clone()),
        _ => None,
    })?;
    if choices.len() < 2 || !choices[0].contains("Byalan Island") {
        return Err(format!("unexpected Izlude Sailor choices: {choices:?}"));
    }
    context.flush();
    context.net.choose_dialog_option(sailor_id, 1).map_err(|_| "disconnected")?;

    context.wait_for("Sailor fare deduction", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::Zeny(value),
        } if *value == starting_zeny - 150 => Some(*value),
        _ => None,
    })?;
    context.wait_for("ChangeMap to Byalan waiting area", |event| match event {
        NetworkEvent::ChangeMap { map_name, position } if map_name == "izlu2dun" => Some(*position),
        _ => None,
    })?;
    if context.map_name != "izlu2dun" {
        return Err(format!("ferry should arrive at izlu2dun, landed on {}", context.map_name));
    }

    // The static edge (`izlu2dun` 108,83 -> `iz_dun00` 168,168) is separately
    // modeled in the navigation graph. Its live walk traversal remains open:
    // this fixture server returned no movement acknowledgements at those cells.
    Ok(())
}

/// Walk toward each server-verified cell in a static warp rectangle until a
/// movement acknowledgement or the expected map transition is observed.
fn walk_through_portal(
    context: &mut TestContext,
    expected_map: &str,
    cells: &[(u16, u16)],
) -> Result<ragnarok_packets::TilePosition, String> {
    enum Step {
        Moved(ragnarok_packets::TilePosition),
        ChangedMap(ragnarok_packets::TilePosition),
    }

    let mut observed_moves = Vec::new();
    for &(x, y) in cells {
        if context.map_name == expected_map {
            return context.wait_for(&format!("ChangeMap to {expected_map}"), |event| match event {
                NetworkEvent::ChangeMap { map_name, position } if map_name == expected_map => Some(*position),
                _ => None,
            });
        }
        let start = context.position;
        context.flush();
        context
            .net
            .player_move(WorldPosition::new(x, y, Direction::North))
            .map_err(|_| "disconnected")?;
        let mut classify = |event: &NetworkEvent| match event {
            NetworkEvent::ChangeMap { map_name, position } if map_name == expected_map => Some(Step::ChangedMap(*position)),
            NetworkEvent::PlayerMove { destination, .. } => Some(Step::Moved(destination.tile_position())),
            _ => None,
        };
        match context.wait_for_within("movement toward graph portal", Duration::from_secs(4), &mut classify) {
            Ok(Step::ChangedMap(position)) => return Ok(position),
            Ok(Step::Moved(destination)) => {
                observed_moves.push(format!("({x}, {y}) -> ({}, {})", destination.x, destination.y));
                let distance = start.x.abs_diff(destination.x).max(start.y.abs_diff(destination.y)) as u64;
                context.pump(Duration::from_millis((distance * 200 + 500).min(4000)));
                if context.map_name == expected_map {
                    return context.wait_for(&format!("ChangeMap to {expected_map}"), |event| match event {
                        NetworkEvent::ChangeMap { map_name, position } if map_name == expected_map => Some(*position),
                        _ => None,
                    });
                }
            }
            Err(_) => continue,
        }
    }

    Err(format!(
        "no walkable entry among {} cells; remained on {} at ({}, {}); movement acks: {:?}",
        cells.len(),
        context.map_name,
        context.position.x,
        context.position.y,
        observed_moves
    ))
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
