//! Phase 6 — items, inventory, economy.

use std::time::Duration;

use korangar_networking::{HotkeyState, InventoryItemDetails, NetworkEvent, NoMetadata, ShopItem};
use ragnarok_packets::{
    BuyOrSellOption, BuyShopItemsResult, DisappearanceReason, EntityId, EquipPosition, HotbarSlot, HotbarTab, HotkeyData, HotkeyType,
    InventoryIndex, SellItemsResult, SkillId, SkillLevel, SoldItemInformation, StatType, StatUpType,
};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("item-command-multi-word", 6, item_command_multi_word),
        Scenario::new("use-consumable", 6, use_consumable),
        Scenario::new("equip-unequip", 6, equip_unequip),
        Scenario::new("drop-pickup", 6, drop_pickup),
        Scenario::new("identify", 6, identify),
        Scenario::new("identify-cancel", 6, identify_cancel),
        Scenario::new("equip-wrong-job", 6, equip_wrong_job),
        Scenario::new("shop-buy-sell", 6, shop_buy_sell),
        Scenario::new("shop-close", 6, shop_close),
        Scenario::new("use-drop-failures", 6, use_drop_failures),
        Scenario::new("storage", 6, storage),
        Scenario::new("storage-persistence", 6, storage_persistence),
        Scenario::new("inventory-order", 6, inventory_order),
        Scenario::new("inventory-split-persistence", 6, inventory_split_persistence),
        Scenario::new("autopickup-respects-drop-owner", 6, autopickup_respects_drop_owner),
        Scenario::new("loot-pickup-race", 6, loot_pickup_race),
        Scenario::new("loot-pickup-multi-pile", 6, loot_pickup_multi_pile),
        Scenario::new("autopickup-radius", 6, autopickup_radius),
        Scenario::new("autopickup-party-override", 6, autopickup_party_override),
        Scenario::new("stat-skill-points", 6, stat_skill_points),
        Scenario::new("reset-command-behavior", 6, reset_command_behavior),
        Scenario::new("reset-command-ordinary-reachability", 6, reset_command_ordinary_reachability),
        Scenario::new("refundskill-prerequisite-rejection", 6, refundskill_prerequisite_rejection),
        Scenario::new("autoloot-commands-are-recognized", 6, autoloot_commands_are_recognized),
        Scenario::new("hotkeys", 6, hotkeys),
        Scenario::new("repair-weapon-cancel", 6, repair_weapon_cancel),
        Scenario::new("repair-weapon-success", 6, repair_weapon_success),
        Scenario::new("repair-list-empty", 6, repair_list_empty),
        Scenario::new("repair-invalid-item", 6, repair_invalid_item),
    ]
}

/// The split packet (fork 0x0EFC) copies the source item, including its
/// database row id, into a new slot, so a mishandled save could duplicate or
/// drop a stack. Two back-to-back splits that cannot both be legal, then a
/// logout/relogin, must leave exactly the original count in the same stacks.
fn inventory_split_persistence(config: &Config) -> Result<(), String> {
    let item_id = 501u32; // Red Potion
    let holdings = |context: &TestContext| -> (u32, Vec<u16>) {
        let mut amounts: Vec<u16> = context
            .inventory
            .iter()
            .filter(|item| item.item_id.0 == item_id)
            .map(|item| item.amount())
            .collect();
        amounts.sort_unstable();
        (amounts.iter().map(|amount| u32::from(*amount)).sum(), amounts)
    };

    let before_logout = {
        let mut context = TestContext::connect(config)?;
        context.say(&format!("@delitem {item_id} 30000"))?;
        context.pump(Duration::from_millis(250));
        let source_index = context.give_item(item_id, 10)?;

        // Both requests leave before either answer: 10 -> 4 + 6 is legal, and
        // then a second 6 from the 4 must be refused, not split again.
        context.net.split_inventory_stack(source_index, 6).map_err(|_| "disconnected")?;
        context.net.split_inventory_stack(source_index, 6).map_err(|_| "disconnected")?;
        // The refusal is expected, but the counts are the verdict: a server
        // that wrongly accepts the second split must fail on what it did.
        let _ = context.wait_for_within(
            "rejection of the second split",
            Duration::from_secs(3),
            &mut |event| match event {
                NetworkEvent::ChatMessage { text, .. } if text.contains("amount smaller than the stack") => Some(()),
                _ => None,
            },
        );
        context.pump(Duration::from_millis(300));
        let state = holdings(&context);
        if state != (10, vec![4, 6]) {
            return Err(format!("back-to-back splits left {state:?}, expected 10 as [4, 6]"));
        }
        state
        // Dropping the context logs out, which saves the inventory.
    };
    std::thread::sleep(Duration::from_millis(900));

    let mut context = TestContext::connect(config)?;
    context.pump(Duration::from_millis(300));
    let after_relog = holdings(&context);
    let _ = context.say(&format!("@delitem {item_id} 30000"));
    if after_relog != before_logout {
        return Err(format!(
            "split stacks changed across relog: {before_logout:?} before, {after_relog:?} after (a duplicated or lost row)"
        ));
    }
    Ok(())
}

/// Automatic pickup must not take another player's drop while the drop is
/// still theirs. A monster drop belongs to its killer for
/// `item_first_get_time` (3000 ms, `conf/map/battle/drops.conf`); the fork's
/// sweep hands every attempt to stock `pc_takeitem`, which enforces that.
///
/// The killer (autopickup off) farms Porings until one drops something; the
/// bystander (solo, autopickup on) is then warped onto the drop. It must not
/// arrive during the owner's window, and it must arrive once the window ends --
/// the positive half is what proves the bystander's sweep was running at all.
fn autopickup_respects_drop_owner(config: &Config) -> Result<(), String> {
    const MAP: &str = "prontera";
    let (mut owner, mut bystander) = TestContext::connect_pair(config)?;
    for context in [&mut owner, &mut bystander] {
        let _ = context.net.leave_party();
        context.pump(Duration::from_millis(400));
    }
    let reply = owner.gm_expect_feedback("@autopickup 0")?;
    if !reply.contains("Automatic pickup: off") {
        return Err(format!("owner's @autopickup 0 was not accepted: {reply}"));
    }
    let reply = bystander.gm_expect_feedback("@autopickup 2")?;
    if !reply.contains("Automatic pickup: on") {
        return Err(format!("bystander's @autopickup 2 was not accepted: {reply}"));
    }
    owner.warp(MAP, 155, 180)?;
    bystander.warp(MAP, 140, 180)?;
    owner.say("@str 99")?;
    owner.say("@heal")?;
    let owner_id = owner.player_id;

    let mut drop = None;
    for _ in 0..25 {
        let target = owner.spawn_monster_near("PORING", 1002)?;
        let mut dead = false;
        for _ in 0..20 {
            let Some(position) = owner.entities.get(&target).map(|entity| entity.position.tile_position()) else {
                break;
            };
            if owner.position.x.abs_diff(position.x) > 1 || owner.position.y.abs_diff(position.y) > 1 {
                let _ = owner.walk_to(position.x.saturating_sub(1), position.y);
            }
            owner.flush();
            owner.net.player_attack(target).map_err(|_| "owner disconnected")?;
            let hit = owner.wait_for_within("Poring hit or death", Duration::from_secs(6), &mut |event| match event {
                NetworkEvent::RemoveEntity {
                    entity_id,
                    reason: DisappearanceReason::Died,
                } if *entity_id == target => Some(true),
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    ..
                } if *source_entity_id == owner_id && *destination_entity_id == target => Some(false),
                _ => None,
            })?;
            // A killing hit sends damage and death back to back; see
            // death_recovery_ten_kill_threshold for why the death is awaited here.
            if hit
                || owner
                    .wait_for_within(
                        "Poring death after a hit",
                        Duration::from_millis(1000),
                        &mut |event| match event {
                            NetworkEvent::RemoveEntity {
                                entity_id,
                                reason: DisappearanceReason::Died,
                            } if *entity_id == target => Some(()),
                            _ => None,
                        },
                    )
                    .is_ok()
            {
                dead = true;
                break;
            }
        }
        if !dead {
            continue;
        }
        if let Ok(found) = owner.wait_for_within("a Poring drop", Duration::from_millis(1500), &mut |event| match event {
            NetworkEvent::AddGroundItem {
                entity_id,
                item_id,
                position,
                ..
            } => Some((*entity_id, item_id.0, *position)),
            _ => None,
        }) {
            drop = Some((found, std::time::Instant::now()));
            break;
        }
    }
    let Some(((ground_id, item_id, position), dropped_at)) = drop else {
        return Err("25 Porings dropped nothing; cannot test drop ownership".to_owned());
    };

    bystander.warp(MAP, position.x, position.y)?;
    let window_end = dropped_at + Duration::from_millis(2600);
    let early = bystander.collect_for(window_end.saturating_duration_since(std::time::Instant::now()));
    let taken_early = early.iter().any(|event| {
        matches!(event, NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id)
            || matches!(event, NetworkEvent::RemoveGroundItem { entity_id } if *entity_id == ground_id)
    });
    let arrived_later = !taken_early
        && bystander
            .wait_for_within(
                "the drop once the owner's window ends",
                Duration::from_secs(5),
                &mut |event| match event {
                    NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id => Some(()),
                    _ => None,
                },
            )
            .is_ok();

    let _ = owner.say("@killmonster");
    let _ = owner.gm_expect_feedback("@autopickup 2");
    if taken_early {
        return Err(format!(
            "automatic pickup took item {item_id} from its owner's drop inside the {} ms ownership window",
            3000
        ));
    }
    if !arrived_later {
        return Err(format!(
            "the bystander never picked up item {item_id} even after the ownership window, so its automatic pickup was not running and \
             the first half proves nothing"
        ));
    }
    Ok(())
}

// Loot scenarios ported 2026-10-04 from the closed PR #10 branch (QW-046 and
// the autopickup pair), where they were written but never reached main.

/// QW-046 — two clients race one floor pile: one owner, no duplicate.
fn loot_pickup_race(config: &Config) -> Result<(), String> {
    const RED_POTION: u32 = 501;
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    primary.gm_expect_feedback("@autopickup 0")?;
    partner.gm_expect_feedback("@autopickup 0")?;
    let index = primary.give_item(RED_POTION, 1)?;
    primary.flush();
    primary.net.drop_item(index, 1).map_err(|_| "disconnected")?;
    let ground_id = primary.wait_for("AddGroundItem", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id, item_id: id, ..
        } if id.0 == RED_POTION => Some(*entity_id),
        _ => None,
    })?;
    partner.wait_for("partner AddGroundItem", |event| match event {
        NetworkEvent::AddGroundItem { entity_id, .. } if *entity_id == ground_id => Some(()),
        _ => None,
    })?;
    primary.flush();
    partner.flush();
    primary.net.pick_up_item(ground_id).map_err(|_| "disconnected")?;
    partner.net.pick_up_item(ground_id).map_err(|_| "disconnected")?;
    let primary_adds = primary
        .collect_for(Duration::from_millis(800))
        .into_iter()
        .filter(|event| matches!(event, NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == RED_POTION))
        .count();
    let partner_adds = partner
        .collect_for(Duration::from_millis(800))
        .into_iter()
        .filter(|event| matches!(event, NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == RED_POTION))
        .count();
    let _ = primary.gm_expect_feedback("@autopickup 2");
    let _ = partner.gm_expect_feedback("@autopickup 2");
    if primary_adds + partner_adds != 1 {
        return Err(format!(
            "race must grant the pile once; primary={primary_adds} partner={partner_adds}"
        ));
    }
    Ok(())
}

/// QW-046 — two clients race two distinct floor piles: each pile is granted
/// exactly once, and winning one pile does not suppress the other.
fn loot_pickup_multi_pile(config: &Config) -> Result<(), String> {
    const RED_POTION: u32 = 501;
    const ORANGE_POTION: u32 = 502;
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    primary.gm_expect_feedback("@autopickup 0")?;
    partner.gm_expect_feedback("@autopickup 0")?;

    let red_index = primary.give_item(RED_POTION, 1)?;
    let orange_index = primary.give_item(ORANGE_POTION, 1)?;
    primary.flush();
    primary.net.drop_item(red_index, 1).map_err(|_| "disconnected")?;
    primary.net.drop_item(orange_index, 1).map_err(|_| "disconnected")?;

    let red_entity = primary.wait_for("red potion AddGroundItem", |event| match event {
        NetworkEvent::AddGroundItem { entity_id, item_id, .. } if item_id.0 == RED_POTION => Some(*entity_id),
        _ => None,
    })?;
    let (orange_entity, orange_position) = primary.wait_for("orange potion AddGroundItem", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id,
            item_id,
            position,
            ..
        } if item_id.0 == ORANGE_POTION => Some((*entity_id, *position)),
        _ => None,
    })?;
    if red_entity == orange_entity {
        return Err("distinct floor piles reused one entity id".to_owned());
    }
    for entity in [red_entity, orange_entity] {
        partner.wait_for("partner multi-pile AddGroundItem", |event| match event {
            NetworkEvent::AddGroundItem { entity_id, .. } if *entity_id == entity => Some(()),
            _ => None,
        })?;
    }

    primary.flush();
    partner.flush();
    for entity in [red_entity, orange_entity] {
        primary.net.pick_up_item(entity).map_err(|_| "disconnected")?;
        partner.net.pick_up_item(entity).map_err(|_| "disconnected")?;
    }
    let primary_events = primary.collect_for(Duration::from_millis(900));
    let partner_events = partner.collect_for(Duration::from_millis(900));
    let count = |events: &[NetworkEvent], item_id| {
        events
            .iter()
            .filter(|event| matches!(event, NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id))
            .count()
    };
    let red_adds = count(&primary_events, RED_POTION) + count(&partner_events, RED_POTION);
    let orange_adds = count(&primary_events, ORANGE_POTION) + count(&partner_events, ORANGE_POTION);
    let _ = primary.gm_expect_feedback("@autopickup 2");
    let _ = partner.gm_expect_feedback("@autopickup 2");
    if red_adds != 1 || orange_adds != 1 {
        // A pile granted to nobody is not lost (the conservation audit checks
        // that); it stayed on the floor. Say where, so an intermittent failure
        // (1 in ~6 runs on 2026-10-04) explains itself: Hercules refuses a
        // pickup more than two cells away (`pc_takeitem`).
        let orange_left_floor = primary_events
            .iter()
            .chain(partner_events.iter())
            .any(|event| matches!(event, NetworkEvent::RemoveGroundItem { entity_id } if *entity_id == orange_entity));
        return Err(format!(
            "multi-pile pickup must grant each pile once; red={red_adds}, orange={orange_adds} (orange at {orange_position:?}, left the \
             floor: {orange_left_floor}; primary at {:?}, partner at {:?})",
            primary.position, partner.position
        ));
    }
    Ok(())
}

/// Did anything take the ground item, and did the bag gain it back?
fn ground_item_taken(context: &mut TestContext, entity_id: EntityId, item_id: u32, window: Duration) -> (bool, bool) {
    let mut removed = false;
    let mut added = false;

    for event in context.collect_for(window) {
        match event {
            NetworkEvent::RemoveGroundItem { entity_id: id } if id == entity_id => removed = true,
            NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id => added = true,
            _ => {}
        }
    }

    (removed, added)
}

/// Loot within two cells walks into the bag with no click
/// (`autopickup_radius`, Hercules `pc_autopickup_*`).
///
/// Positioned by `@warp` and not by walking: `walk_to` reports success once it
/// is within one cell of its target, and the exact cell where the behaviour
/// changes is the entire subject of this test. Distances are measured from the
/// position `AddGroundItem` reports rather than from where the item was
/// dropped -- `map_addflooritem` calls `search_freecell`, so a drop lands on a
/// free cell NEAR the dropper, not necessarily under them.
///
/// Four stages, because only the negatives make the positive mean anything:
/// with the feature off the item survives being stood next to; at three cells
/// it survives; at two -- the edge of the radius, and of the reach
/// `pc_takeitem` has always enforced -- it is taken. The last stage only runs
/// when the third fails, and it separates "the radius is wrong" from "the
/// sweep never ran at all".
fn autopickup_radius(config: &Config) -> Result<(), String> {
    const MAP: &str = "prontera";
    const X: u16 = 155;
    const Y: u16 = 180;

    let mut context = TestContext::connect(config)?;
    let item_id = 501; // Red Potion

    // Off first, or the item below is taken before it can be observed.
    context.gm_expect_feedback("@autopickup 0")?;
    context.warp(MAP, X, Y)?;

    // An ownerless floor item from the headless_loot_test fixture: the
    // server never auto-picks a player's own drop (`player_dropped`), which is
    // why PR #10's version of this test, dropping its own potion, cannot pass.
    context.flush();
    let reply = context.gm_expect_feedback("@testloot 501 1 0 0")?;
    if !reply.contains("Test loot placed") {
        return Err(format!(
            "the @testloot fixture is not loaded (is headless_loot_test.txt enabled?): {reply}"
        ));
    }

    let (ground_entity_id, item_position) = context.wait_for("AddGroundItem", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id,
            item_id: id,
            position,
            ..
        } if id.0 == item_id => Some((*entity_id, *position)),
        _ => None,
    })?;

    // Standing next to it, switched off. A server where none of this was wired
    // up would pass every later stage without this one.
    let (removed, _) = ground_item_taken(&mut context, ground_entity_id, item_id, Duration::from_millis(1200));
    if removed {
        return Err("the drop was taken off the floor while @autopickup was off".to_owned());
    }

    // The command's own reply is the assertion that it was understood:
    // gm_expect_feedback is satisfied by ANY server line, including a usage
    // error, so a silently rejected argument would otherwise look like a
    // feature that does not work.
    let reply = context.gm_expect_feedback("@autopickup 2")?;
    if !reply.contains("Automatic pickup: on") {
        return Err(format!("@autopickup 2 was not accepted; the server said: {reply}"));
    }

    // One cell outside the radius.
    context.warp(MAP, item_position.x + 3, item_position.y)?;
    let (removed, _) = ground_item_taken(&mut context, ground_entity_id, item_id, Duration::from_millis(1200));
    if removed {
        return Err("the drop was taken from three cells away, outside the two cell radius".to_owned());
    }

    // The edge of the radius.
    context.warp(MAP, item_position.x + 2, item_position.y)?;
    let (removed, added) = ground_item_taken(&mut context, ground_entity_id, item_id, Duration::from_millis(2500));

    if !removed {
        // Which failure is this? Standing on the item is the same code path at
        // distance zero, so if that works the sweep runs and the radius is
        // short; if it does not, the sweep is not running at all.
        context.warp(MAP, item_position.x, item_position.y)?;
        let (removed_on_top, _) = ground_item_taken(&mut context, ground_entity_id, item_id, Duration::from_millis(2500));

        if removed_on_top {
            return Err("the drop was taken standing on it but not from two cells away: the radius is shorter than it claims".to_owned());
        }

        return Err("the drop was never taken, even standing on it: the pickup sweep is not running".to_owned());
    }

    if !added {
        return Err("the drop left the ground two cells away but never arrived in the inventory".to_owned());
    }

    Ok(())
}

/// A party overrides a member's own "off" (`pc_autopickup_radius`).
///
/// The rule the server enforces: your own setting is yours for solo play, and a
/// party takes it over. Loot in a group is shared -- `party_default_share`
/// turns both item rules on when the party is formed -- so a member opting out
/// keeps nothing for themselves, they only leave drops lying on the floor for
/// everybody.
///
/// The personal OFF is established first against a real drop, so the second
/// half cannot pass by the setting having quietly failed to apply.
fn autopickup_party_override(config: &Config) -> Result<(), String> {
    const MAP: &str = "prontera";
    const X: u16 = 155;
    const Y: u16 = 180;

    let mut context = TestContext::connect(config)?;
    let item_id = 501; // Red Potion

    let _ = context.net.leave_party();
    context.pump(Duration::from_millis(600));

    let reply = context.gm_expect_feedback("@autopickup 0")?;
    if !reply.contains("Automatic pickup: off") {
        return Err(format!("@autopickup 0 was not accepted; the server said: {reply}"));
    }

    context.warp(MAP, X, Y)?;
    // An ownerless floor item from the headless_loot_test fixture: the
    // server never auto-picks a player's own drop (`player_dropped`), which is
    // why PR #10's version of this test, dropping its own potion, cannot pass.
    context.flush();
    let reply = context.gm_expect_feedback("@testloot 501 1 0 0")?;
    if !reply.contains("Test loot placed") {
        return Err(format!(
            "the @testloot fixture is not loaded (is headless_loot_test.txt enabled?): {reply}"
        ));
    }

    let (ground_entity_id, _) = context.wait_for("AddGroundItem", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id,
            item_id: id,
            position,
            ..
        } if id.0 == item_id => Some((*entity_id, *position)),
        _ => None,
    })?;

    // Solo, switched off: the drop stays where it fell.
    let (removed, _) = ground_item_taken(&mut context, ground_entity_id, item_id, Duration::from_millis(1200));
    if removed {
        return Err("the drop was taken while the character had automatic pickup off".to_owned());
    }

    // A party of one is still a party, and it is the party that decides.
    let name = format!("pick{}", std::process::id() % 10000);
    context.flush();
    context.say(&format!("@party {name}"))?;
    context.pump(Duration::from_millis(800));

    let (removed, added) = ground_item_taken(&mut context, ground_entity_id, item_id, Duration::from_millis(2500));

    // Put everything back before reporting: the party would follow this account
    // into the next scenario, and so would the stored setting.
    let _ = context.net.leave_party();
    context.pump(Duration::from_millis(300));
    let _ = context.gm_expect_feedback("@autopickup 2");

    if !removed {
        return Err("joining a party did not override the character's own off setting".to_owned());
    }
    if !added {
        return Err("the drop left the ground once in a party but never arrived in the inventory".to_owned());
    }

    Ok(())
}

/// Kept as the final suite scenario because it fills the disposable character's
/// inventory to test the no-free-slot rejection path.
pub fn split_stack_scenario() -> Scenario {
    Scenario::new("inventory-split", 6, inventory_split)
}

/// Persist a reordered permutation without changing the underlying item slot
/// identifiers used by equip/use/storage actions.
fn inventory_order(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.give_item(501, 1)?;
    context.give_item(503, 1)?; // Yellow Potion, distinct item/slot.
    let original = context.inventory.iter().map(|item| item.index).collect::<Vec<_>>();
    if original.len() < 2 {
        return Err(format!("inventory-order fixture has only {} occupied slots", original.len()));
    }
    let mut reordered = original.clone();
    reordered.reverse();
    context.flush();
    context.net.reorder_inventory(reordered.clone()).map_err(|_| "disconnected")?;
    let acknowledged = context.wait_for("InventoryOrder acknowledgement", |event| match event {
        NetworkEvent::InventoryOrder { indices } => Some(indices.clone()),
        _ => None,
    })?;
    if acknowledged != reordered {
        return Err(format!(
            "server acknowledged inventory order {acknowledged:?}, requested {reordered:?}"
        ));
    }
    let mut slot_ids = context.inventory.iter().map(|item| item.index).collect::<Vec<_>>();
    slot_ids.sort_unstable_by_key(|index| index.0);
    let mut expected_slot_ids = original;
    expected_slot_ids.sort_unstable_by_key(|index| index.0);
    if slot_ids != expected_slot_ids {
        return Err("reordering display slots changed authoritative inventory indices".to_owned());
    }
    Ok(())
}

const BS_REPAIRWEAPON: SkillId = SkillId(108);

/// `Iron Arrow` — a two-word display name whose **first word is itself an
/// item** (`Iron`, 998). That collision is the whole point: it is what made the
/// old parser fail quietly instead of erroring.
const IRON_ARROW: u32 = 1770;
const IRON: u32 = 998;

/// Guards the multi-word `@item` delta in Hercules `src/map/atcommand.c`.
///
/// Unquoted, the stock command scans `%99s %12d`: `@item Iron Arrow 500` takes
/// `Iron`, fails to read `Arrow` as a quantity, and **still returns ≥ 1**, so
/// it reports success while handing over **one Iron**. Nothing in this suite
/// would notice, because every other caller passes a numeric id.
///
/// The second assertion guards the regression that the *fix* introduced and
/// which the compiler was happy with: resolving the longest name first meant
/// the id lookup saw the whole argument string, and `atoi("1770 500")` is
/// `1770`, so the quantity was never peeled and `@item 1770 500` silently gave
/// **one** arrow — a regression on the most common DM usage. An id is only
/// accepted now when the string is numeric end to end.
///
/// Both halves fail *quietly* and in the same direction (one item instead of
/// many), so the amount is the assertion, not the item's presence.
fn item_command_multi_word(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    let stocked = |context: &mut TestContext, command: &str, expected_id: u32| -> Result<u16, String> {
        context.say(&format!("@delitem {IRON_ARROW} 30000"))?;
        context.say(&format!("@delitem {IRON} 30000"))?;
        context.pump(Duration::from_millis(400));
        context.flush();

        context.say(command)?;
        let (item_id, amount) = context.wait_for(&format!("inventory add from {command:?}"), |event| match event {
            NetworkEvent::IventoryItemAdded { item } => Some((item.item_id.0, match item.details {
                InventoryItemDetails::Regular { amount, .. } => amount,
                InventoryItemDetails::Equippable { amount, .. } => amount,
            })),
            _ => None,
        })?;

        if item_id != expected_id {
            return Err(format!(
                "{command:?} produced item {item_id}, not {expected_id} — the multi-word `@item` delta in Hercules `src/map/atcommand.c` \
                 (atcommand_item_search / atcommand_item_parse) has probably been lost in an upstream merge"
            ));
        }
        Ok(amount)
    };

    let amount = stocked(&mut context, "@item Iron Arrow 500", IRON_ARROW)?;
    if amount != 500 {
        return Err(format!(
            "`@item Iron Arrow 500` gave {amount} Iron Arrow, not 500 — the quantity was not peeled off the end of a multi-word name"
        ));
    }

    let amount = stocked(&mut context, &format!("@item {IRON_ARROW} 500"), IRON_ARROW)?;
    if amount != 500 {
        return Err(format!(
            "`@item {IRON_ARROW} 500` gave {amount}, not 500 — a bare id plus quantity regressed. The longest-name-first lookup is \
             accepting `\"{IRON_ARROW} 500\"` as an id again (`atoi` stops at the space); it must only accept a string that is numeric \
             end to end"
        ));
    }

    // Arrows stack, and accumulation here is invisible until the character is
    // overweight and every item scenario breaks at once, far from the cause.
    let _ = context.say(&format!("@delitem {IRON_ARROW} 30000"));
    let _ = context.say(&format!("@delitem {IRON} 30000"));
    context.pump(Duration::from_millis(400));
    Ok(())
}

fn prepare_repair(context: &mut TestContext) -> Result<(SkillLevel, ragnarok_packets::RepairableItemInformation), String> {
    context.ensure_job(10)?;
    context.say("@allskill")?;
    context.say("@heal")?;
    context.say("@delitem 1101 999")?;
    context.pump(Duration::from_millis(400));
    context.flush();
    context.say("@item2 1101 1 1 0 1 0 0 0 0")?;
    context.wait_for("broken Sword inventory add", |event| match event {
        NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == 1101 => Some(()),
        _ => None,
    })?;
    let level = context
        .skills
        .iter()
        .find(|skill| skill.skill_id == BS_REPAIRWEAPON)
        .map(|skill| skill.skill_level)
        .unwrap_or(SkillLevel(1));
    context.flush();
    context
        .net
        .cast_skill(BS_REPAIRWEAPON, level, context.player_id)
        .map_err(|_| "disconnected")?;
    let items = context.wait_for("RepairableItemList", |event| match event {
        NetworkEvent::RepairableItemList { items } => Some(items.clone()),
        _ => None,
    })?;
    items
        .into_iter()
        .find(|item| item.item_id.0 == 1101)
        .map(|item| (level, item))
        .ok_or_else(|| "RepairableItemList omitted the broken Sword".to_owned())
}

fn repair_weapon_cancel(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let _ = prepare_repair(&mut context)?;
    context.flush();
    context.net.cancel_item_repair().map_err(|_| "disconnected")?;
    let events = context.collect_for(Duration::from_secs(1));
    if events.iter().any(|event| matches!(event, NetworkEvent::ItemRepairResult { .. })) {
        return Err("Repair Weapon cancellation produced a repair result".to_owned());
    }
    Ok(())
}

fn repair_weapon_success(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let (_, item) = prepare_repair(&mut context)?;
    context.give_item(1002, 1)?; // Iron Ore for a level-one weapon.
    let expected_index = ragnarok_packets::InventoryIndex(item.inventory_index.0);
    context.flush();
    context.net.request_item_repair(item).map_err(|_| "disconnected")?;
    context.wait_for("successful ItemRepairResult", |event| match event {
        NetworkEvent::ItemRepairResult {
            inventory_index,
            success: true,
        } if *inventory_index == expected_index => Some(()),
        _ => None,
    })
}

/// Casting Repair Weapon with no broken equipment anywhere must not open a
/// repair list (the skill simply fails server-side).
fn repair_list_empty(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.ensure_job(10)?;
    context.say("@allskill")?;
    context.say("@delitem 1101 999")?;
    context.pump(Duration::from_millis(400));

    let level = context
        .skills
        .iter()
        .find(|skill| skill.skill_id == BS_REPAIRWEAPON)
        .map(|skill| skill.skill_level)
        .unwrap_or(SkillLevel(1));
    context.flush();
    context
        .net
        .cast_skill(BS_REPAIRWEAPON, level, context.player_id)
        .map_err(|_| "disconnected")?;
    let events = context.collect_for(Duration::from_millis(1500));
    if events.iter().any(|event| matches!(event, NetworkEvent::RepairableItemList { .. })) {
        return Err("Repair Weapon opened a repair list with nothing broken".to_owned());
    }
    Ok(())
}

/// Selecting a repair target that vanished between the list and the response
/// must not succeed or corrupt anything, and a fresh repair must still work.
fn repair_invalid_item(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let (_, stale_item) = prepare_repair(&mut context)?;

    // The listed weapon disappears before we answer the menu.
    context.say("@delitem 1101 999")?;
    context.pump(Duration::from_millis(400));
    context.flush();
    context.net.request_item_repair(stale_item).map_err(|_| "disconnected")?;
    let events = context.collect_for(Duration::from_millis(1500));
    if events.iter().any(|event| {
        matches!(
            event,
            NetworkEvent::ItemRepairResult { success: true, .. } | NetworkEvent::IventoryItemAdded { .. }
        )
    }) {
        return Err("repairing a vanished item reported success or mutated the inventory".to_owned());
    }

    // The session must still support a full, valid repair afterwards.
    let (_, item) = prepare_repair(&mut context)?;
    context.give_item(1002, 1)?; // Iron Ore for a level-one weapon.
    let expected_index = ragnarok_packets::InventoryIndex(item.inventory_index.0);
    context.flush();
    context.net.request_item_repair(item).map_err(|_| "disconnected")?;
    context.wait_for("successful ItemRepairResult after invalid attempt", |event| match event {
        NetworkEvent::ItemRepairResult {
            inventory_index,
            success: true,
        } if *inventory_index == expected_index => Some(()),
        _ => None,
    })?;
    context.say("@delitem 1101 999")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// Give ourselves a Red Potion and use it, verifying it heals us and gets
/// removed.
fn use_consumable(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let potion_id = 501; // Red Potion
    let player_id = context.player_id;

    // Ensure we are damaged so the heal is fully processed/observed.
    context.ensure_base_level(10)?;
    context.flush();
    context.say("@die")?;

    // If Kaizel (Soul Linker self-resurrection) is active from the preceding
    // skill sweep, the first @die is consumed to resurrect the player on the
    // spot without triggering a RemoveEntity event. We wait briefly, and if we
    // don't stay dead, we send @die again.
    let first_death = context.wait_for_within("first death attempt", Duration::from_secs(2), &mut |event| match event {
        NetworkEvent::RemoveEntity { entity_id, .. } if entity_id.0 == player_id.0 => Some(()),
        _ => None,
    });
    if first_death.is_err() {
        context.say("@die")?;
        context.wait_for("death", |event| match event {
            NetworkEvent::RemoveEntity { entity_id, .. } if entity_id.0 == player_id.0 => Some(()),
            _ => None,
        })?;
    }

    context.flush();
    context.net.respawn().map_err(|_| "disconnected")?;
    context.wait_for("respawn map load", |event| match event {
        NetworkEvent::ChangeMap { .. } => Some(()),
        _ => None,
    })?;
    context.net.map_loaded().map_err(|_| "disconnected")?;

    context.say("@delitem 501 100")?;
    context.pump(Duration::from_millis(200));
    let index = context.give_item(potion_id, 1)?;
    context.flush();
    context.net.use_item(index, context.account_id).map_err(|_| "disconnected")?;

    context.wait_for("InventoryItemRemoved", |event| match event {
        NetworkEvent::InventoryItemRemoved { index: removed_index, .. } if *removed_index == index => Some(()),
        _ => None,
    })?;

    // Heal back up for subsequent tests
    context.say("@heal")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// Equip a sword and then unequip it, verifying the equipped position updates.
fn equip_unequip(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.ensure_job(4008)?; // Lord Knight (can equip swords)
    let sword_id = 1101; // Town Sword

    let index = context.give_item(sword_id, 1)?;
    context.flush();
    context
        .net
        .request_item_equip(index, EquipPosition::RIGHT_HAND)
        .map_err(|_| "disconnected")?;

    context.wait_for("UpdateEquippedPosition (equip)", |event| match event {
        NetworkEvent::UpdateEquippedPosition {
            index: event_index,
            equipped_position,
        } if *event_index == index && equipped_position.contains(EquipPosition::RIGHT_HAND) => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.request_item_unequip(index).map_err(|_| "disconnected")?;

    context.wait_for("UpdateEquippedPosition (unequip)", |event| match event {
        NetworkEvent::UpdateEquippedPosition {
            index: event_index,
            equipped_position,
        } if *event_index == index && equipped_position.is_empty() => Some(()),
        _ => None,
    })?;

    context.say("@delitem 1101 1")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// Drop a Red Potion onto the ground, then pick it back up.
fn drop_pickup(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let item_id = 501; // Red Potion

    let index = context.give_item(item_id, 1)?;
    context.flush();
    context.net.drop_item(index, 1).map_err(|_| "disconnected")?;

    let (ground_entity_id, _ground_item_id) = context.wait_for("AddGroundItem", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id, item_id: id, ..
        } if id.0 == item_id => Some((*entity_id, *id)),
        _ => None,
    })?;

    context.flush();
    context.net.pick_up_item(ground_entity_id).map_err(|_| "disconnected")?;

    context.wait_for("RemoveGroundItem", |event| match event {
        NetworkEvent::RemoveGroundItem { entity_id } if *entity_id == ground_entity_id => Some(()),
        _ => None,
    })?;

    Ok(())
}

/// Identify an unidentified sword using a Magnifier.
fn identify(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    // Give ourselves an unidentified Town Sword (GM command @item2 param 3 is
    // identify: 0 = false)
    context.flush();
    context.say("@item2 1101 1 0 0 0 0 0 0 0")?;
    let sword_index = context.wait_for("unidentified item added", |event| match event {
        NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == 1101 && !item.is_identified() => Some(item.index),
        _ => None,
    })?;

    // Give ourselves a Magnifier
    let magnifier_index = context.give_item(611, 1)?;

    context.flush();
    context
        .net
        .use_item(magnifier_index, context.account_id)
        .map_err(|_| "disconnected")?;

    let (skill_id, skill_level) = context.wait_for("AutoRunSkill", |event| match event {
        NetworkEvent::AutoRunSkill { skill_id, skill_level, .. } => Some((*skill_id, *skill_level)),
        _ => None,
    })?;

    context.flush();
    context
        .net
        .cast_skill(skill_id, skill_level, context.player_id)
        .map_err(|_| "disconnected")?;

    let indices = context.wait_for("ItemIdentifyList", |event| match event {
        NetworkEvent::ItemIdentifyList { indices } => Some(indices.clone()),
        _ => None,
    })?;

    if !indices.contains(&sword_index) {
        return Err("ItemIdentifyList did not contain our unidentified sword".to_owned());
    }

    context.flush();
    context.net.request_item_identify(sword_index).map_err(|_| "disconnected")?;

    context.wait_for("ItemIdentified success", |event| match event {
        NetworkEvent::ItemIdentified { inventory_index, success } if *inventory_index == sword_index && *success => Some(()),
        _ => None,
    })?;

    context.say("@delitem 1101 1")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// Open the identify list and cancel without selecting an item.
///
/// Mirrors `weapon-refine-cancel` / `repair-weapon-cancel`: cancel must not
/// identify anything, and a second identify attempt in the same session must
/// still work (proves Hercules cleared pending menu state).
fn identify_cancel(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    context.flush();
    context.say("@item2 1101 1 0 0 0 0 0 0 0")?;
    let sword_index = context.wait_for("unidentified item added", |event| match event {
        NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == 1101 && !item.is_identified() => Some(item.index),
        _ => None,
    })?;

    let magnifier_index = context.give_item(611, 1)?;
    context.flush();
    context
        .net
        .use_item(magnifier_index, context.account_id)
        .map_err(|_| "disconnected")?;
    let (skill_id, skill_level) = context.wait_for("AutoRunSkill", |event| match event {
        NetworkEvent::AutoRunSkill { skill_id, skill_level, .. } => Some((*skill_id, *skill_level)),
        _ => None,
    })?;
    context.flush();
    context
        .net
        .cast_skill(skill_id, skill_level, context.player_id)
        .map_err(|_| "disconnected")?;
    context.wait_for("ItemIdentifyList", |event| match event {
        NetworkEvent::ItemIdentifyList { indices } if indices.contains(&sword_index) => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.cancel_item_identify().map_err(|_| "disconnected")?;
    let events = context.collect_for(Duration::from_secs(1));
    if events
        .iter()
        .any(|event| matches!(event, NetworkEvent::ItemIdentified { success: true, .. }))
    {
        return Err("identify cancel produced a successful ItemIdentified".to_owned());
    }

    // Second path: open again and complete identification.
    let magnifier_index = context.give_item(611, 1)?;
    context.flush();
    context
        .net
        .use_item(magnifier_index, context.account_id)
        .map_err(|_| "disconnected")?;
    let (skill_id, skill_level) = context.wait_for("AutoRunSkill after cancel", |event| match event {
        NetworkEvent::AutoRunSkill { skill_id, skill_level, .. } => Some((*skill_id, *skill_level)),
        _ => None,
    })?;
    context.flush();
    context
        .net
        .cast_skill(skill_id, skill_level, context.player_id)
        .map_err(|_| "disconnected")?;
    context.wait_for("ItemIdentifyList after cancel", |event| match event {
        NetworkEvent::ItemIdentifyList { indices } if indices.contains(&sword_index) => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.request_item_identify(sword_index).map_err(|_| "disconnected")?;
    context.wait_for("ItemIdentified after cancel path", |event| match event {
        NetworkEvent::ItemIdentified { inventory_index, success } if *inventory_index == sword_index && *success => Some(()),
        _ => None,
    })?;

    context.say("@delitem 1101 1")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// A job that cannot wear a sword must not equip it.
///
/// Negative equip path: Mage + Town Sword. Asserts no successful
/// `UpdateEquippedPosition` for the right hand, then a valid equip on Lord
/// Knight still works so the session is not stuck.
fn equip_wrong_job(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.ensure_job(2)?; // Mage
    context.say("@delitem 1101 999")?;
    context.pump(Duration::from_millis(300));

    let index = context.give_item(1101, 1)?;
    context.flush();
    context
        .net
        .request_item_equip(index, EquipPosition::RIGHT_HAND)
        .map_err(|_| "disconnected")?;

    let events = context.collect_for(Duration::from_secs(1));
    if events.iter().any(|event| {
        matches!(
            event,
            NetworkEvent::UpdateEquippedPosition {
                index: event_index,
                equipped_position,
            } if *event_index == index && equipped_position.contains(EquipPosition::RIGHT_HAND)
        )
    }) {
        return Err("Mage successfully equipped a Town Sword".to_owned());
    }

    // Prove the session can still equip legally.
    context.ensure_job(4008)?; // Lord Knight
    context.say("@delitem 1101 999")?;
    context.pump(Duration::from_millis(300));
    let index = context.give_item(1101, 1)?;
    context.flush();
    context
        .net
        .request_item_equip(index, EquipPosition::RIGHT_HAND)
        .map_err(|_| "disconnected")?;
    context.wait_for("legal equip after failed equip", |event| match event {
        NetworkEvent::UpdateEquippedPosition {
            index: event_index,
            equipped_position,
        } if *event_index == index && equipped_position.contains(EquipPosition::RIGHT_HAND) => Some(()),
        _ => None,
    })?;

    context.say("@delitem 1101 1")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// Find the Pet Groomer shop in Prontera, select Buy, purchase Pet Food, then
/// sell it back.
fn shop_buy_sell(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.say("@zeny 1000000")?;
    context.pump(Duration::from_millis(200));

    // Warp adjacent to the Pet Groomer at prontera,218,211
    context.warp("prontera", 218, 209)?;

    // Find the Groomer entity
    let entities = context.entities.clone();
    let mut groomer_id = None;
    for &id in entities.keys() {
        context.flush();
        context.net.entity_details(id).map_err(|_| "disconnected")?;
        let name = context.wait_for_within("UpdateEntityDetails", Duration::from_millis(400), &mut |event| match event {
            NetworkEvent::UpdateEntityDetails { entity_id, name } if *entity_id == id => Some(name.clone()),
            _ => None,
        });
        if let Ok(name) = name
            && name.contains("Pet Groomer")
        {
            groomer_id = Some(id);
            break;
        }
    }

    let groomer_id = groomer_id.ok_or("Pet Groomer NPC not found near prontera,218,211")?;

    // Open shop
    context.flush();
    context.net.start_dialog(groomer_id).map_err(|_| "disconnected")?;
    let shop_id = context.wait_for("AskBuyOrSell", |event| match event {
        NetworkEvent::AskBuyOrSell { shop_id } => Some(*shop_id),
        _ => None,
    })?;

    // Select Buy
    context.flush();
    context
        .net
        .select_buy_or_sell(shop_id, BuyOrSellOption::Buy)
        .map_err(|_| "disconnected")?;
    let shop_items = context.wait_for("OpenShop", |event| match event {
        NetworkEvent::OpenShop { items } => Some(items.clone()),
        _ => None,
    })?;

    // Find Pet Food (item id 537) in the list
    let pet_food = shop_items
        .iter()
        .find(|item| item.item_id.0 == 537)
        .ok_or("Pet Food item (537) not found in Groomer shop")?;

    // Purchase 1 Pet Food
    let purchase_item = ShopItem {
        metadata: 1, // Quantity
        item_id: pet_food.item_id,
        item_type: pet_food.item_type,
        price: pet_food.price,
        quantity: pet_food.quantity,
        weight: pet_food.weight,
        location: pet_food.location,
    };

    context.flush();
    context.net.purchase_items(vec![purchase_item]).map_err(|_| "disconnected")?;
    context.wait_for("BuyingCompleted success", |event| match event {
        NetworkEvent::BuyingCompleted {
            result: BuyShopItemsResult::Success,
        } => Some(()),
        _ => None,
    })?;

    // Find the purchased Pet Food in inventory
    let food_inventory_item = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == 537)
        .ok_or("purchased Pet Food not in inventory")?;
    let food_index = food_inventory_item.index;

    // Now select Sell
    context.flush();
    context.net.start_dialog(groomer_id).map_err(|_| "disconnected")?;
    let shop_id = context.wait_for("AskBuyOrSell (sell)", |event| match event {
        NetworkEvent::AskBuyOrSell { shop_id } => Some(*shop_id),
        _ => None,
    })?;

    context.flush();
    context
        .net
        .select_buy_or_sell(shop_id, BuyOrSellOption::Sell)
        .map_err(|_| "disconnected")?;
    let sell_items = context.wait_for("SellItemList", |event| match event {
        NetworkEvent::SellItemList { items } => Some(items.clone()),
        _ => None,
    })?;

    if !sell_items.iter().any(|item| item.inventory_index == food_index) {
        return Err("Groomer did not list our Pet Food for sale".to_owned());
    }

    // Sell it back
    context.flush();
    context
        .net
        .sell_items(vec![SoldItemInformation {
            inventory_index: food_index,
            amount: 1,
        }])
        .map_err(|_| "disconnected")?;
    context.wait_for("SellingCompleted success", |event| match event {
        NetworkEvent::SellingCompleted {
            result: SellItemsResult::Success,
        } => Some(()),
        _ => None,
    })?;

    Ok(())
}

/// Open a shop, close it with `close_shop`, and assert a subsequent purchase
/// does not complete successfully. Then reopen and complete a valid buy so the
/// session is not stuck.
fn shop_close(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.say("@zeny 1000000")?;
    context.pump(Duration::from_millis(200));
    context.warp("prontera", 218, 209)?;

    let groomer_id = find_pet_groomer(&mut context)?;
    let (_shop_id, pet_food) = open_groomer_buy(&mut context, groomer_id)?;

    context.flush();
    context.net.close_shop().map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(300));

    // Purchase after close must not succeed. `metadata` is the buy quantity.
    let purchase_item = ShopItem {
        metadata: 1u32,
        item_id: pet_food.item_id,
        item_type: pet_food.item_type,
        price: pet_food.price,
        quantity: pet_food.quantity,
        weight: pet_food.weight,
        location: pet_food.location,
    };
    context.flush();
    context
        .net
        .purchase_items(vec![purchase_item.clone()])
        .map_err(|_| "disconnected")?;
    let events = context.collect_for(Duration::from_secs(1));
    if events.iter().any(|event| {
        matches!(event, NetworkEvent::BuyingCompleted {
            result: BuyShopItemsResult::Success
        })
    }) {
        return Err("purchase after close_shop reported success".to_owned());
    }

    // Reopen and complete a real buy so the menu path is still usable.
    let (_, pet_food) = open_groomer_buy(&mut context, groomer_id)?;
    let purchase_item = ShopItem {
        metadata: 1u32,
        item_id: pet_food.item_id,
        item_type: pet_food.item_type,
        price: pet_food.price,
        quantity: pet_food.quantity,
        weight: pet_food.weight,
        location: pet_food.location,
    };
    context.flush();
    context.net.purchase_items(vec![purchase_item]).map_err(|_| "disconnected")?;
    context.wait_for("BuyingCompleted after reopen", |event| match event {
        NetworkEvent::BuyingCompleted {
            result: BuyShopItemsResult::Success,
        } => Some(()),
        _ => None,
    })?;
    context.say("@delitem 537 99")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// Invalid use/drop must not corrupt inventory; a valid use follows.
fn use_drop_failures(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let potion_id = 501u32;
    let index = context.give_item(potion_id, 2)?;
    let bogus = InventoryIndex(u16::MAX);

    context.flush();
    context.net.use_item(bogus, context.account_id).map_err(|_| "disconnected")?;
    let after_bad_use = context.collect_for(Duration::from_millis(800));
    if after_bad_use.iter().any(|event| {
        matches!(
            event,
            NetworkEvent::InventoryItemRemoved { index: removed, .. } if *removed == index
        )
    }) {
        return Err("use of invalid index removed a real stack".to_owned());
    }

    context.flush();
    context.net.drop_item(index, 0).map_err(|_| "disconnected")?;
    let after_zero = context.collect_for(Duration::from_millis(800));
    if after_zero.iter().any(|event| matches!(event, NetworkEvent::AddGroundItem { .. })) {
        return Err("drop of zero amount created a ground item".to_owned());
    }

    context.flush();
    context.net.drop_item(index, 99).map_err(|_| "disconnected")?;
    let after_excess = context.collect_for(Duration::from_millis(800));
    // Excess drop is either refused or clamped; never invent a phantom stack.
    let ground_count = after_excess
        .iter()
        .filter(|event| matches!(event, NetworkEvent::AddGroundItem { item_id, .. } if item_id.0 == potion_id))
        .count();
    if ground_count > 1 {
        return Err(format!("excess drop created {ground_count} ground items"));
    }

    // Valid drop still works in the same session (avoids death/heal races that
    // made a "use while damaged" follow-up flaky on a fresh disposable DB).
    let index = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == potion_id)
        .map(|item| item.index)
        .ok_or("no potion left for valid drop after failure cases")?;
    context.flush();
    context.net.drop_item(index, 1).map_err(|_| "disconnected")?;
    context.wait_for("valid AddGroundItem after failures", |event| match event {
        NetworkEvent::AddGroundItem { item_id, .. } if item_id.0 == potion_id => Some(()),
        _ => None,
    })?;
    let _ = context.say("@delitem 501 99");
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// Item deposited in storage survives close + full relog.
fn storage_persistence(config: &Config) -> Result<(), String> {
    let item_id = 501u32;
    let marker_amount = 3u16;

    {
        let mut context = TestContext::connect(config)?;
        let _ = context.say(&format!("@delitem {item_id} 30000"));
        context.pump(Duration::from_millis(300));
        let index = context.give_item(item_id, marker_amount)?;
        context.flush();
        context.say("@storage")?;
        context.wait_for("SetStorage", |event| match event {
            NetworkEvent::SetStorage { .. } => Some(()),
            _ => None,
        })?;
        context.flush();
        context
            .net
            .move_item_to_storage(index, marker_amount as u32)
            .map_err(|_| "disconnected")?;
        context.wait_for("StorageItemAdded marker", |event| match event {
            NetworkEvent::StorageItemAdded { item } if item.item_id.0 == item_id => Some(()),
            _ => None,
        })?;
        context.flush();
        context.net.close_storage().map_err(|_| "disconnected")?;
        context.wait_for("StorageClosed", |event| match event {
            NetworkEvent::StorageClosed => Some(()),
            _ => None,
        })?;
        // Drop runs logout; give the server a beat before the next login.
    }
    std::thread::sleep(Duration::from_millis(900));

    let mut context = TestContext::connect(config)?;
    context.flush();
    context.say("@storage")?;
    let items = context.wait_for("SetStorage after relog", |event| match event {
        NetworkEvent::SetStorage { items } => Some(items.clone()),
        _ => None,
    })?;
    let Some(item) = items.into_iter().find(|item| item.item_id.0 == item_id) else {
        return Err("storage lost the marker item across relog".to_owned());
    };
    let amount = match item.details {
        InventoryItemDetails::Regular { amount, .. } | InventoryItemDetails::Equippable { amount, .. } => amount,
    };
    if amount < marker_amount {
        return Err(format!("storage item amount {amount} < expected {marker_amount} after relog"));
    }
    context.flush();
    context
        .net
        .move_item_from_storage(item.index, marker_amount as u32)
        .map_err(|_| "disconnected")?;
    context.wait_for("StorageItemRemoved after persistence", |event| match event {
        NetworkEvent::StorageItemRemoved { .. } => Some(()),
        _ => None,
    })?;
    context.net.close_storage().map_err(|_| "disconnected")?;
    let _ = context.say(&format!("@delitem {item_id} 99"));
    context.pump(Duration::from_millis(200));
    Ok(())
}

fn find_pet_groomer(context: &mut TestContext) -> Result<EntityId, String> {
    let entities = context.entities.clone();
    for &id in entities.keys() {
        context.flush();
        context.net.entity_details(id).map_err(|_| "disconnected")?;
        let name = context.wait_for_within("UpdateEntityDetails", Duration::from_millis(400), &mut |event| match event {
            NetworkEvent::UpdateEntityDetails { entity_id, name } if *entity_id == id => Some(name.clone()),
            _ => None,
        });
        if let Ok(name) = name
            && name.contains("Pet Groomer")
        {
            return Ok(id);
        }
    }
    Err("Pet Groomer NPC not found near prontera,218,211".to_owned())
}

fn open_groomer_buy(context: &mut TestContext, groomer_id: EntityId) -> Result<(ragnarok_packets::ShopId, ShopItem<NoMetadata>), String> {
    context.flush();
    context.net.start_dialog(groomer_id).map_err(|_| "disconnected")?;
    let shop_id = context.wait_for("AskBuyOrSell", |event| match event {
        NetworkEvent::AskBuyOrSell { shop_id } => Some(*shop_id),
        _ => None,
    })?;
    context.flush();
    context
        .net
        .select_buy_or_sell(shop_id, BuyOrSellOption::Buy)
        .map_err(|_| "disconnected")?;
    let shop_items = context.wait_for("OpenShop", |event| match event {
        NetworkEvent::OpenShop { items } => Some(items.clone()),
        _ => None,
    })?;
    let pet_food = shop_items
        .into_iter()
        .find(|item| item.item_id.0 == 537)
        .ok_or("Pet Food item (537) not found in Groomer shop")?;
    Ok((shop_id, pet_food))
}

/// Partial inventory ↔ storage transfers preserve item counts and source slot
/// identity, including a merged retrieve into an existing inventory stack.
fn storage(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let item_id = 501; // Red Potion

    context.say(&format!("@delitem {item_id} 30000"))?;
    context.pump(Duration::from_millis(250));
    context.say("@storage")?;
    let existing_storage = context.wait_for("initial SetStorage", |event| match event {
        NetworkEvent::SetStorage { items } => Some(items.clone()),
        _ => None,
    })?;
    for existing in existing_storage.into_iter().filter(|item| item.item_id.0 == item_id) {
        context.flush();
        context
            .net
            .move_item_from_storage(existing.index, u32::from(existing.amount()))
            .map_err(|_| "disconnected")?;
        context.wait_for("clear old Red Potion storage stack", |event| match event {
            NetworkEvent::StorageItemRemoved { index, amount } if *index == existing.index && *amount == u32::from(existing.amount()) => {
                Some(())
            }
            _ => None,
        })?;
    }
    context.flush();
    context.net.close_storage().map_err(|_| "disconnected")?;
    context.wait_for("close storage after clearing marker", |event| {
        matches!(event, NetworkEvent::StorageClosed).then_some(())
    })?;
    context.say(&format!("@delitem {item_id} 30000"))?;
    context.pump(Duration::from_millis(250));
    let inventory_index = context.give_item(item_id, 5)?;

    context.flush();
    context.say("@storage")?;
    context.wait_for("SetStorage for transfer", |event| {
        matches!(event, NetworkEvent::SetStorage { .. }).then_some(())
    })?;

    context.flush();
    context.net.move_item_to_storage(inventory_index, 2).map_err(|_| "disconnected")?;

    let storage_item = context.wait_for("StorageItemAdded", |event| match event {
        NetworkEvent::StorageItemAdded { item } if item.item_id.0 == item_id && item.amount() == 2 => Some(item.clone()),
        _ => None,
    })?;
    context.wait_for("inventory count reduced after partial store", |event| match event {
        NetworkEvent::InventoryItemRemoved { index, amount: 2, .. } if *index == inventory_index => Some(()),
        _ => None,
    })?;
    let carried = context
        .inventory
        .iter()
        .find(|item| item.index == inventory_index)
        .map(|item| item.amount());
    if carried != Some(3) {
        return Err(format!("storing two potions left inventory amount {carried:?}, expected 3"));
    }

    context.flush();
    context
        .net
        .move_item_from_storage(storage_item.index, 1)
        .map_err(|_| "disconnected")?;
    context.wait_for("StorageItemRemoved", |event| match event {
        NetworkEvent::StorageItemRemoved {
            index: removed_index,
            amount: 1,
        } if *removed_index == storage_item.index => Some(()),
        _ => None,
    })?;
    context.wait_for("one potion merged back to inventory", |event| match event {
        NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id && item.amount() == 1 => Some(()),
        _ => None,
    })?;
    let carried = context
        .inventory
        .iter()
        .filter(|item| item.item_id.0 == item_id)
        .map(|item| item.amount())
        .sum::<u16>();
    if carried != 4 {
        return Err(format!("retrieving one potion left carried amount {carried}, expected 4"));
    }

    context.flush();
    context.net.close_storage().map_err(|_| "disconnected")?;
    context.wait_for("StorageClosed before re-open", |event| {
        matches!(event, NetworkEvent::StorageClosed).then_some(())
    })?;
    context.flush();
    context.say("@storage")?;
    let remaining_storage = context.wait_for("storage snapshot with remaining potion", |event| match event {
        NetworkEvent::SetStorage { items } => Some(items.clone()),
        _ => None,
    })?;
    let remaining_item = remaining_storage
        .into_iter()
        .find(|item| item.item_id.0 == item_id)
        .ok_or("partially transferred potion missing from storage snapshot")?;
    if remaining_item.amount() != 1 || remaining_item.index != storage_item.index {
        return Err(format!(
            "remaining storage stack was index {} amount {}, expected index {} amount 1",
            remaining_item.index.0,
            remaining_item.amount(),
            storage_item.index.0
        ));
    }
    context
        .net
        .move_item_from_storage(remaining_item.index, 1)
        .map_err(|_| "disconnected")?;
    context.wait_for("final storage item removal", |event| match event {
        NetworkEvent::StorageItemRemoved { index, amount: 1 } if *index == remaining_item.index => Some(()),
        _ => None,
    })?;
    context.wait_for("final potion merged into inventory", |event| match event {
        NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id && item.amount() == 1 => Some(()),
        _ => None,
    })?;
    let carried = context
        .inventory
        .iter()
        .filter(|item| item.item_id.0 == item_id)
        .map(|item| item.amount())
        .sum::<u16>();
    if carried != 5 {
        return Err(format!("after retrieving all potions, carried amount is {carried}, expected 5"));
    }

    context.flush();
    context.net.close_storage().map_err(|_| "disconnected")?;
    context.wait_for("StorageClosed", |event| match event {
        NetworkEvent::StorageClosed => Some(()),
        _ => None,
    })?;

    // Cleanup inventory item (the disposable integration database is dropped).
    context.say("@delitem 501 5")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// Exercise the server-authoritative stack split against valid, invalid, and
/// full-inventory paths; the disposable integration database is discarded.
fn inventory_split(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let item_id = 501u32; // Red Potion
    let initial_amount = 10u16;

    context.say(&format!("@delitem {item_id} 30000"))?;
    context.say("@delitem 1770 30000")?;
    context.say("@delitem 4001 30000")?;
    context.pump(Duration::from_millis(250));
    let source_index = context.give_item(item_id, initial_amount)?;
    let _arrow_index = context.give_item(1770, 8)?; // Iron Arrow (IT_AMMO)
    let _card_index = context.give_item(4001, 1)?; // Poring Card (IT_CARD)
    let arrow = context.inventory.iter().find(|item| item.item_id.0 == 1770);
    let card = context.inventory.iter().find(|item| item.item_id.0 == 4001);
    if !arrow.is_some_and(|item| item.item_type == 10 && item.amount() == 8) {
        return Err("live Iron Arrow packet did not report stackable ammo as item type 10, amount 8".to_owned());
    }
    if !card.is_some_and(|item| item.item_type == 6 && item.amount() == 1) {
        return Err("live Poring Card packet did not report card item type 6".to_owned());
    }
    context.flush();

    context.net.split_inventory_stack(source_index, 4).map_err(|_| "disconnected")?;
    context.wait_for("partial split source decrement", |event| match event {
        NetworkEvent::InventoryItemRemoved { index, amount, .. } if *index == source_index && *amount == 4 => Some(()),
        _ => None,
    })?;
    let split_index = context.wait_for("partial split destination stack", |event| match event {
        NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id && item.index != source_index && item.amount() == 4 => {
            Some(item.index)
        }
        _ => None,
    })?;
    let source_amount = context
        .inventory
        .iter()
        .find(|item| item.index == source_index)
        .map(|item| item.amount());
    let split_amount = context
        .inventory
        .iter()
        .find(|item| item.index == split_index)
        .map(|item| item.amount());
    if source_amount != Some(6) || split_amount != Some(4) {
        return Err(format!(
            "valid split left source/destination amounts {source_amount:?}/{split_amount:?}, expected 6/4"
        ));
    }

    for invalid_amount in [0, 6, 7] {
        context.flush();
        context
            .net
            .split_inventory_stack(source_index, invalid_amount)
            .map_err(|_| "disconnected")?;
        context.wait_for("server rejection of invalid split amount", |event| match event {
            NetworkEvent::ChatMessage { text, .. } if text.contains("Choose a stackable item and an amount smaller than the stack") => {
                Some(())
            }
            _ => None,
        })?;
        let events = context.collect_for(Duration::from_millis(100));
        if events.iter().any(|event| {
            matches!(event, NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id)
                || matches!(event, NetworkEvent::InventoryItemRemoved { index, .. } if *index == source_index)
        }) {
            return Err(format!("invalid split amount {invalid_amount} mutated the potion stacks"));
        }
    }

    // This is a fresh disposable test database when selected on its own.
    // @allstats gives sufficient weight capacity; a large count of non-stackable
    // Knives fills every remaining slot without relying on a visual inventory.
    context.say("@item 1201 300")?; // Knife (non-stackable)
    context.pump(Duration::from_secs(2));
    if !context
        .inventory
        .iter()
        .any(|item| item.item_id.0 == item_id && item.index == source_index)
    {
        return Err("inventory source potion disappeared while filling inventory".to_owned());
    }
    context.flush();
    context.net.split_inventory_stack(source_index, 1).map_err(|_| "disconnected")?;
    context.wait_for("server rejection of split with full inventory", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("Your inventory is full; make room before splitting a stack") => Some(()),
        _ => None,
    })?;
    let events = context.collect_for(Duration::from_millis(100));
    if events.iter().any(|event| {
        matches!(event, NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id)
            || matches!(event, NetworkEvent::InventoryItemRemoved { index, .. } if *index == source_index)
    }) {
        return Err("full-inventory split rejection mutated the source stack".to_owned());
    }
    let remaining = context
        .inventory
        .iter()
        .find(|item| item.index == source_index)
        .map(|item| item.amount());
    if remaining != Some(6) {
        return Err(format!(
            "full-inventory split changed source amount to {remaining:?}, expected 6"
        ));
    }
    Ok(())
}

/// Allocate stat points and skill points, asserting the stat updates correctly.
fn stat_skill_points(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    // Give ourselves GM stats first to clear any old state, reset job/level to
    // novice
    context.ensure_job(0)?;
    context.ensure_base_level(10)?;
    context.say("@reset")?;
    context.pump(Duration::from_millis(300));

    // Increase STR by 1
    context.flush();
    context
        .net
        .request_stat_up(StatUpType::Strength { amount: 1 })
        .map_err(|_| "disconnected")?;
    context.wait_for("UpdateStat STR", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::Strength(val, _),
        } if *val > 1 => Some(()),
        _ => None,
    })?;

    // Level up skill: Basic Skill (skill id 1) to level 1
    context.flush();
    context
        .net
        .level_up_skill(ragnarok_packets::SkillId(1))
        .map_err(|_| "disconnected")?;
    context.wait_for("SkillTree update", |event| match event {
        NetworkEvent::SkillTree { skill_information }
            if skill_information
                .iter()
                .any(|skill| skill.skill_id.0 == 1 && skill.skill_level.0 >= 1) =>
        {
            Some(())
        }
        _ => None,
    })?;

    // Re-heal/reset
    context.say("@reset")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// The player-facing reset commands are free, affect only their own pool, and
/// leave the resulting state intact across logout/relogin.
fn reset_command_behavior(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let starting_job = context.job_id.0;
    let starting_base_level = context.base_level;
    context.ensure_job(7)?; // Knight: SM_BASH (5) is a stable tree fixture.
    context.ensure_base_level(10)?;

    // Start from a known stat baseline and allocate one visible point. The
    // headless seats are GM fixtures, so @allskill seeds a deterministic
    // learned skill for testing reset behavior without relying on novice
    // quest-gated skills or manually earning job levels.
    context.gm_expect_feedback("@streset")?;
    let base_strength = context.wait_for("initial stat reset Strength", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::Strength(value, _),
        } => Some(*value),
        _ => None,
    })?;
    let starting_zeny = context.zeny;

    context.flush();
    context
        .net
        .request_stat_up(StatUpType::Strength { amount: 1 })
        .map_err(|_| "disconnected")?;
    context.wait_for("allocated Strength", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::Strength(value, _),
        } if *value > base_strength => Some(*value),
        _ => None,
    })?;

    context.gm_expect_feedback("@allskill")?;
    context.wait_for("SM_BASH present in skill tree", |event| match event {
        NetworkEvent::SkillTree { skill_information } => skill_information
            .iter()
            .find(|skill| skill.skill_id.0 == 5 && skill.skill_level.0 > 0)
            .map(|skill| skill.skill_level.0),
        _ => None,
    })?;

    context.gm_expect_feedback("@streset")?;
    context.wait_for("stat reset restoring Strength", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::Strength(value, _),
        } if *value == base_strength => Some(*value),
        _ => None,
    })?;
    if context
        .skills
        .iter()
        .find(|skill| skill.skill_id.0 == 5)
        .is_none_or(|skill| skill.skill_level.0 == 0)
    {
        return Err("@streset removed the independently learned SM_BASH skill".to_owned());
    }

    context.gm_expect_feedback("@skreset")?;
    context.wait_for("skill reset clearing SM_BASH", |event| match event {
        NetworkEvent::SkillTree { skill_information } => Some(
            skill_information
                .iter()
                .find(|skill| skill.skill_id.0 == 5)
                .map_or(0, |skill| skill.skill_level.0),
        ),
        _ => None,
    })?;
    let cleared_skill_level = context
        .skills
        .iter()
        .find(|skill| skill.skill_id.0 == 5)
        .map_or(0, |skill| skill.skill_level.0);
    if cleared_skill_level != 0 {
        return Err(format!("@skreset left SM_BASH at level {cleared_skill_level}"));
    }
    if context.zeny != starting_zeny {
        return Err(format!("free reset commands changed zeny: {starting_zeny} -> {}", context.zeny));
    }

    context.net.disconnect_from_map_server();
    drop(context);
    std::thread::sleep(Duration::from_millis(700));
    let mut context = TestContext::connect(config)?;
    if context.skills.iter().any(|skill| skill.skill_id.0 == 5 && skill.skill_level.0 != 0) {
        return Err(format!("SM_BASH reset did not persist across relog: {:?}", context.skills));
    }
    if context.zeny != starting_zeny {
        return Err(format!(
            "relogin changed zeny after free resets: {starting_zeny} -> {}",
            context.zeny
        ));
    }

    // Leave the shared character's class and level unchanged for later tests.
    context.gm_expect_feedback("@streset")?;
    context.gm_expect_feedback("@skreset")?;
    context.ensure_job(starting_job)?;
    context.ensure_base_level(starting_base_level)?;
    Ok(())
}

/// `reset-command-behavior` only ever ran as the shared GM (group 99)
/// fixture, so the GDD S2 acceptance gap ("group 0 command reachability is
/// still open") stayed open even after that scenario passed: `conf/groups.conf`
/// grants ordinary players `streset`/`skreset`/`refundskill`, but nothing had
/// ever exercised the commands as a non-GM account.
///
/// Deliberately account-agnostic (works whether `--username` is a GM or an
/// ordinary player), and deliberately does not depend on `@allskill` or any
/// other GM-only command to seed its fixture -- it reads whatever skill/stat
/// state the account already has rather than manufacturing it, so this is
/// safe to run against the shared GM fixture too (where it exercises the same
/// commands from the other side: with permission implied by `all_commands`
/// rather than the named per-command grant).
///
/// The commands are chat text with no dedicated ack packet: a permission
/// rejection and a genuine no-op both look like "no event arrived" at the
/// wire level unless a real state change proves the command actually ran.
/// The reachability question is answered by an `UpdateStat` on `@streset`;
/// `@skreset` and `@refundskill` are checked opportunistically, and are not
/// GDD acceptance evidence for an account with no learned skill to clear.
fn reset_command_ordinary_reachability(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let starting_zeny = context.zeny;

    context.gm_expect_feedback("@streset")?;
    context.wait_for(
        "Strength UpdateStat proving @streset executed (not silently rejected)",
        |event| match event {
            NetworkEvent::UpdateStat {
                stat_type: StatType::Strength(..),
            } => Some(()),
            _ => None,
        },
    )?;
    if context.zeny != starting_zeny {
        return Err(format!("@streset changed zeny: {starting_zeny} -> {}", context.zeny));
    }

    // Opportunistic: only meaningful evidence if the account already has a
    // learned skill to clear. A freshly-seeded ordinary test account proves
    // it; the shared GM fixture's skill state is unpredictable mid-suite.
    // Skill 1 (NV_BASIC) is excluded: every character has it innately, and
    // `@skreset` correctly never clears it -- it is not a "learned" skill.
    if let Some(skill) = context
        .skills
        .iter()
        .find(|skill| skill.skill_level.0 > 0 && skill.skill_id.0 != 1)
        .cloned()
    {
        context.gm_expect_feedback("@skreset")?;
        context.wait_for(&format!("SkillTree clearing skill {}", skill.skill_id.0), |event| match event {
            NetworkEvent::SkillTree { skill_information } => skill_information
                .iter()
                .find(|entry| entry.skill_id.0 == skill.skill_id.0)
                .map_or(Some(()), |entry| (entry.skill_level.0 == 0).then_some(())),
            _ => None,
        })?;
        if context.zeny != starting_zeny {
            return Err(format!("@skreset changed zeny: {starting_zeny} -> {}", context.zeny));
        }
    }

    Ok(())
}

/// GDD S2's other still-open item: `@refundskill`'s prerequisite rejection
/// ("single-skill refund prerequisite rejection") had never been exercised
/// live. `pc_skilldown` (`src/map/pc.c`) refuses to refund a skill below the
/// level a *learned* dependent still needs, answering with a red chat message
/// instead of a `SkillTree` update -- and leaves the skill's level untouched.
///
/// Opportunistic like its sibling above: only asserts anything when the
/// account happens to hold Swordsman's SM_BASH (id 5) at level >= 5 with
/// SM_MAGNUM (id 7, needs `SM_BASH: 5`) learned at level >= 1, so it is safe
/// to register in "all" against the shared GM fixture's unpredictable skill
/// state. A freshly-seeded ordinary test account (this row's whole point,
/// same as its sibling) proves it deliberately.
fn refundskill_prerequisite_rejection(config: &Config) -> Result<(), String> {
    const SM_BASH: u16 = 5;
    const SM_MAGNUM: u16 = 7;

    let mut context = TestContext::connect(config)?;
    let starting_zeny = context.zeny;

    let bash_level = context
        .skills
        .iter()
        .find(|skill| skill.skill_id.0 == SM_BASH)
        .map(|skill| skill.skill_level.0);
    let magnum_level = context
        .skills
        .iter()
        .find(|skill| skill.skill_id.0 == SM_MAGNUM)
        .map(|skill| skill.skill_level.0);
    let (Some(bash_level), Some(magnum_level)) = (bash_level, magnum_level) else {
        return Ok(());
    };
    if bash_level < 5 || magnum_level < 1 {
        return Ok(());
    }

    // The dependent still needs it: refunding SM_BASH must be refused, and
    // SM_BASH's level must not move.
    context.flush();
    context.say(&format!("@refundskill {SM_BASH}"))?;
    context.wait_for("prerequisite-refund rejection message", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("Refund the skills that require this one first") => Some(()),
        _ => None,
    })?;
    if context
        .skills
        .iter()
        .any(|skill| skill.skill_id.0 == SM_BASH && skill.skill_level.0 != bash_level)
    {
        return Err("SM_BASH's level moved despite the refund being refused".to_owned());
    }

    // The dependent itself has nothing relying on it: refunding it must
    // succeed and produce a real SkillTree update.
    context.flush();
    context.say(&format!("@refundskill {SM_MAGNUM}"))?;
    context.wait_for("SkillTree lowering the independent dependent skill", |event| match event {
        NetworkEvent::SkillTree { skill_information } => skill_information
            .iter()
            .find(|entry| entry.skill_id.0 == SM_MAGNUM)
            .is_some_and(|entry| entry.skill_level.0 == magnum_level - 1)
            .then_some(()),
        _ => None,
    })?;
    if context.zeny != starting_zeny {
        return Err(format!("@refundskill changed zeny: {starting_zeny} -> {}", context.zeny));
    }

    Ok(())
}

/// GDD 11.2's Loot tab (client-side,
/// `korangar/src/interface/windows/game_settings.rs`, `AutolootPanel`) sends
/// `@autoloot <rate>` and `@autoloottype +/-<type>` with no dedicated ack
/// packet -- it can only prove the command was recognized by matching the exact
/// text `ACMD(autoloot)`/`ACMD(autoloottype)` reply with
/// (`src/map/atcommand.c`). Account-agnostic like its S2 siblings,
/// so it is safe in the shared "all" run too.
fn autoloot_commands_are_recognized(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    context.flush();
    context.say("@autoloot 50")?;
    context.wait_for("autoloot rate confirmation", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("Autolooting items with drop rates of 50") => Some(()),
        _ => None,
    })?;

    context.flush();
    context.say("@autoloot 0")?;
    context.wait_for("autoloot off confirmation", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("Autoloot is now off") => Some(()),
        _ => None,
    })?;

    // +card / -card round trip. If a previous run of this same scenario left
    // "card" enabled, add first so the toggle below is a clean add-then-remove
    // regardless of starting state.
    context.flush();
    context.say("@autoloottype +card")?;
    let added = context.wait_for_within("autoloottype +card reply", Duration::from_secs(5), &mut |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("Autolooting item type: 'Card'") => Some(true),
        NetworkEvent::ChatMessage { text, .. } if text.contains("already autolooting this item type") => Some(false),
        _ => None,
    })?;
    if !added {
        // Already enabled from a previous run: this is the add, so re-add
        // fails on purpose -- confirm the exact "already enabled" wording,
        // then remove so the rest of this scenario runs from a clean state.
        context.flush();
        context.say("@autoloottype -card")?;
        context.wait_for("autoloottype -card cleanup", |event| match event {
            NetworkEvent::ChatMessage { text, .. } if text.contains("Removed item type: 'Card'") => Some(()),
            _ => None,
        })?;
        context.flush();
        context.say("@autoloottype +card")?;
        context.wait_for("autoloottype +card after cleanup", |event| match event {
            NetworkEvent::ChatMessage { text, .. } if text.contains("Autolooting item type: 'Card'") => Some(()),
            _ => None,
        })?;
    }

    context.flush();
    context.say("@autoloottype -card")?;
    context.wait_for("autoloottype -card reply", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("Removed item type: 'Card'") => Some(()),
        _ => None,
    })?;

    Ok(())
}

/// A hotkey written by the client survives a relogin — the hotbar is
/// **server-side** state, not a local preference.
///
/// **This scenario used to assert nothing.** It connected, sent
/// `set_hotkey_data`, pumped for 500ms and returned `Ok(())`, under a doc
/// comment claiming it "verified" the hotkey — so the only thing that could
/// ever redden it was the connection dropping, while `ACTION_COVERAGE` pointed
/// `set_hotkey_data` at it as though the action were covered.
///
/// `CZ_SHORTCUT_KEY_CHANGE2` is fire-and-forget: Hercules writes it straight
/// into `sd->status.hotkeys[]` and acks nothing (`clif_parse_Hotkey2`,
/// clif.c:11854). What makes it checkable at all is the *list* — Hercules
/// sends `ZC_SHORTCUT_KEY_LIST` (0x0B20) for every tab at login, from
/// `clif->hotkeysAll` under `sd->state.connect_new` (clif.c:11518). So the
/// observable is a round trip through the character save, which is also the
/// property a player actually cares about.
///
/// **The probe rotates instead of being a constant, per audit rule A6**
/// ("choose a probe value the fallback cannot produce"). All 148 scenarios
/// share one character: writing a fixed 501 and reading back 501 would pass
/// just as happily against a value a previous run left in that slot, or
/// against a server that ignored the write entirely. Reading the slot first
/// and writing *the other* potion means only a write that landed can satisfy
/// the assertion. The quantity is carried too, and asserted, so a half-written
/// row cannot pass on its id alone.
///
/// **Two cycles, because one proves less than it reads as.** The integration
/// runner builds a disposable database per run and no fixture seeds the
/// hotbar, so on the first cycle the slot is always unbound, the rotation never
/// rotates, and all that is tested is *unbound → bound*. A server that recorded
/// the first write and ignored every later one would pass. The second cycle
/// starts from the binding the first one left, so it is the **overwrite** that
/// is asserted — and it is the only thing that ever exercises the rotation. The
/// two cycles are then required to differ, which is what catches the slot
/// coming back unbound and the rotation quietly collapsing to a constant.
///
/// **Slot 37 is the last of the 38 and deliberately away from the F1–F9 row.**
/// The same character carries a hand-built hotbar for the graphical skill
/// passes (F1–F7 on the E1 Wizard), and a headless test is not entitled to
/// clobber it.
fn hotkeys(config: &Config) -> Result<(), String> {
    let first = hotkey_write_cycle(config, "first")?;
    let second = hotkey_write_cycle(config, "second")?;

    if first == second {
        return Err(format!(
            "both cycles wrote {first:?}, so the second one started from an unbound slot and nothing here tested overwriting a bound one \
             — the first write did not survive into the second cycle, or the rotation has stopped rotating"
        ));
    }
    Ok(())
}

/// One write-and-relogin cycle against hotbar slot 37. Returns the
/// `(item id, quantity)` it wrote, so the caller can require the two cycles to
/// have differed.
fn hotkey_write_cycle(config: &Config, which: &str) -> Result<(u32, u16), String> {
    const TAB: HotbarTab = HotbarTab(0);
    const SLOT: HotbarSlot = HotbarSlot(37);
    const RED_POTION: u32 = 501;
    const ORANGE_POTION: u32 = 502;

    let mut context = TestContext::connect(config)?;
    let before = read_hotkey(&mut context, TAB, SLOT)?;
    // The quantity rotates with the item, so a server that persisted the id and
    // dropped the count cannot ride through on the previous cycle's number.
    let (probe, quantity) = match before {
        Some((item_id, _)) if item_id == RED_POTION => (ORANGE_POTION, 3),
        _ => (RED_POTION, 7),
    };

    context
        .net
        .set_hotkey_data(TAB, SLOT, HotkeyData {
            hotkey_type: HotkeyType::Item,
            item_or_skill_id: probe,
            quantity_or_skill_level: quantity,
        })
        .map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(300));

    // Log out cleanly and come back: the write only reaches the character save
    // on quit, so asserting it in the same session would prove nothing beyond
    // the packet leaving the socket.
    drop(context);
    std::thread::sleep(Duration::from_millis(900));
    let mut context = TestContext::connect(config)?;

    match read_hotkey(&mut context, TAB, SLOT)? {
        Some((item_id, got)) if item_id == probe && got == quantity => Ok((probe, quantity)),
        Some((item_id, got)) => Err(format!(
            "{which} cycle: hotkey tab {} slot {} came back as item {item_id} x{got} after relogin, expected {probe} x{quantity} (it held \
             {before:?} before the write)",
            TAB.0, SLOT.0
        )),
        None => Err(format!(
            "{which} cycle: hotkey tab {} slot {} was unbound after relogin — the write never reached the character save",
            TAB.0, SLOT.0
        )),
    }
}

/// One slot out of the hotkey list the server sends at login, as
/// `(item or skill id, quantity or level)`. `None` means the slot is unbound.
///
/// Reads inside the matcher rather than cloning the event: `HotkeyState` is
/// deliberately not `Clone`, and the two numbers are the whole point.
fn read_hotkey(context: &mut TestContext, tab: HotbarTab, slot: HotbarSlot) -> Result<Option<(u32, u16)>, String> {
    let wanted_tab = tab.0;
    let index = slot.0 as usize;
    context.wait_for(&format!("SetHotkeyData for tab {wanted_tab}"), |event| match event {
        NetworkEvent::SetHotkeyData { tab, hotkeys } if tab.0 == wanted_tab => Some(match hotkeys.get(index) {
            Some(HotkeyState::Bound(data)) => Ok(Some((data.item_or_skill_id, data.quantity_or_skill_level))),
            Some(HotkeyState::Unbound) => Ok(None),
            // Not "unbound": the server sent a shorter list than the slot this
            // scenario addresses, which is a packet-shape change, not a state.
            None => Err(format!(
                "the hotkey list for tab {wanted_tab} has {} slots; slot {index} is outside it",
                hotkeys.len()
            )),
        }),
        _ => None,
    })?
}
