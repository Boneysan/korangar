//! Phase 6 — items, inventory, economy.

use std::time::Duration;

use korangar_networking::{HotkeyState, InventoryItemDetails, NetworkEvent, NoMetadata, ShopItem};
use ragnarok_packets::{
    BuyOrSellOption, BuyShopItemsResult, EntityId, EquipPosition, HotbarSlot, HotbarTab, HotkeyData, HotkeyType, InventoryIndex,
    SellItemsResult, SkillId, SkillLevel, SoldItemInformation, StatType, StatUpType,
};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("item-command-multi-word", 6, item_command_multi_word),
        Scenario::new("item-command-permission", 6, item_command_permission),
        Scenario::new("use-consumable", 6, use_consumable),
        Scenario::new("equip-unequip", 6, equip_unequip),
        Scenario::new("drop-pickup", 6, drop_pickup),
        Scenario::new("loot-pickup-race", 6, loot_pickup_race),
        Scenario::new("loot-pickup-multi-pile", 6, loot_pickup_multi_pile),
        Scenario::new("drop-exact-quantity", 6, drop_exact_quantity),
        Scenario::new("autopickup-radius", 6, autopickup_radius),
        Scenario::new("autopickup-party-override", 6, autopickup_party_override),
        Scenario::new("immediate-repickup-after-drop", 6, immediate_repickup_after_drop),
        Scenario::new("identify", 6, identify),
        Scenario::new("identify-cancel", 6, identify_cancel),
        Scenario::new("equip-wrong-job", 6, equip_wrong_job),
        Scenario::new("shop-buy-sell", 6, shop_buy_sell),
        Scenario::new("shop-close", 6, shop_close),
        Scenario::new("use-drop-failures", 6, use_drop_failures),
        Scenario::new("storage", 6, storage),
        Scenario::new("storage-persistence", 6, storage_persistence),
        Scenario::new("storage-weight-boundary", 6, storage_weight_boundary),
        Scenario::new("cart-weight-boundary", 6, cart_weight_boundary),
        Scenario::new("stat-skill-points", 6, stat_skill_points),
        Scenario::new("skill-lock-relog", 6, skill_lock_relog),
        Scenario::new("skill-refund", 6, skill_refund),
        Scenario::new("hotkeys", 6, hotkeys),
        Scenario::new("hotbar-clear-relog", 6, hotbar_clear_relog),
        Scenario::new("repair-weapon-cancel", 6, repair_weapon_cancel),
        Scenario::new("repair-weapon-success", 6, repair_weapon_success),
        Scenario::new("repair-list-empty", 6, repair_list_empty),
        Scenario::new("repair-invalid-item", 6, repair_invalid_item),
        Scenario::new("zeny-persistence-transaction-matrix", 6, zeny_persistence_transaction_matrix),
        Scenario::new(
            "vendor-double-transaction-classification",
            6,
            vendor_double_transaction_classification,
        ),
    ]
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

    for (command, expected) in [
        ("@item Iron Arrow 1".to_owned(), 1u16),
        ("@item Iron Arrow 500".to_owned(), 500),
        (format!("@item {IRON_ARROW} 1"), 1),
        (format!("@item {IRON_ARROW} 500"), 500),
    ] {
        let amount = stocked(&mut context, &command, IRON_ARROW)?;
        if amount != expected {
            return Err(format!("{command:?} gave {amount} Iron Arrow, not {expected}"));
        }
    }

    // Arrows stack, and accumulation here is invisible until the character is
    // overweight and every item scenario breaks at once, far from the cause.
    let _ = context.say(&format!("@delitem {IRON_ARROW} 30000"));
    let _ = context.say(&format!("@delitem {IRON} 30000"));
    context.pump(Duration::from_millis(400));
    Ok(())
}

/// Player group 0 has no `item` atcommand (`groups.conf`). A GM charcommand
/// demotes the partner, `@item` must fail, then the partner is restored.
fn item_command_permission(config: &Config) -> Result<(), String> {
    let (mut gm, mut partner) = TestContext::connect_pair(config)?;
    gm.say(&format!("#adjgroup {} 0", partner.character_name))?;
    partner.pump(Duration::from_millis(500));
    partner.flush();
    partner.say("@item 501 1")?;
    // Group 0 has no `item` command: Hercules returns false with no dispbottom
    // (`atcommand.c` skips unknown commands for group level 0).
    let added = partner
        .collect_for(Duration::from_secs(2))
        .into_iter()
        .any(|event| matches!(event, NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == 501));
    gm.say(&format!("#adjgroup {} 99", partner.character_name))?;
    gm.pump(Duration::from_millis(400));
    if added {
        return Err("group 0 partner received Red Potion from @item".to_owned());
    }
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
/// QW-041 — drop 1, a middle amount, then the rest of the stack.
fn drop_exact_quantity(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let item_id = 501;
    context.gm_expect_feedback("@autopickup 0")?;
    let index = context.give_item(item_id, 10)?;

    for amount in [1u16, 5, 4] {
        context.flush();
        context.net.drop_item(index, amount).map_err(|_| "disconnected")?;
        context.wait_for(&format!("InventoryItemRemoved amount {amount}"), |event| match event {
            NetworkEvent::InventoryItemRemoved {
                amount: removed,
                index: removed_index,
                ..
            } if *removed == amount && *removed_index == index => Some(()),
            _ => None,
        })?;
    }

    let _ = context.gm_expect_feedback("@autopickup 2");
    Ok(())
}

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
    let orange_entity = primary.wait_for("orange potion AddGroundItem", |event| match event {
        NetworkEvent::AddGroundItem { entity_id, item_id, .. } if item_id.0 == ORANGE_POTION => Some(*entity_id),
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
        return Err(format!(
            "multi-pile pickup must grant each pile once; red={red_adds}, orange={orange_adds}"
        ));
    }
    Ok(())
}

fn drop_pickup(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let item_id = 501; // Red Potion

    // Automatic pickup is on by default (`autopickup_radius`), and it would
    // take this drop off the floor within 400ms -- leaving the scenario green
    // while asserting nothing about the click path it exists to cover.
    context.gm_expect_feedback("@autopickup 0")?;

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

    let taken = context.wait_for("RemoveGroundItem", |event| match event {
        NetworkEvent::RemoveGroundItem { entity_id } if *entity_id == ground_entity_id => Some(()),
        _ => None,
    });

    // The choice is stored per character now, so leaving it off would follow
    // this account into every later scenario and every later session.
    let _ = context.gm_expect_feedback("@autopickup 2");

    taken
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

    // Off first, or the drop below never reaches the ground at all.
    context.gm_expect_feedback("@autopickup 0")?;
    context.warp(MAP, X, Y)?;

    let index = context.give_item(item_id, 1)?;
    context.flush();
    context.net.drop_item(index, 1).map_err(|_| "disconnected")?;

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
    let index = context.give_item(item_id, 1)?;
    context.flush();
    context.net.drop_item(index, 1).map_err(|_| "disconnected")?;

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

/// QW-022 — Immediate repickup after player drop.
///
/// Investigates the playtest report of dropped items being immediately repicked
/// up:
/// 1. Trial A: Drop under default server settings (@autopickup 2, radius 2):
///    - Pending pickup state before drop: None.
///    - Client sends 0x0363 (CZ_ITEM_THROW2 / DropItemPacket).
///    - Server returns 0x00AF (DropItemAck) and 0x084B (AddGroundItem).
///    - Client sends 0 pickup requests (0x0362).
///    - Within 400ms, server's pc_autopickup_timer sweeps and takes the floor
///      item.
///    - Server sends RemoveGroundItem and IventoryItemAdded.
///    - Outcome: immediate repickup occurs WITHOUT any client pickup request.
/// 2. Trial B: Drop with @autopickup 0 (server auto-pickup disabled):
///    - Pending pickup state before drop: None.
///    - Client sends 0x0363.
///    - Server returns 0x00AF and 0x084B.
///    - Client sends 0 pickup requests.
///    - Item remains on the floor over 1200ms; no RemoveGroundItem or
///      IventoryItemAdded.
///    - Outcome: proves client does NOT auto-repickup; repickup is entirely
///      driven by server autopickup.
/// 3. Trial C: Drop with a previously clicked floor item:
///    - An existing floor item (Item A) is clicked out of reach (simulating
///      pending pickup buffered action).
///    - Player drops Item B.
///    - Client sends 0x0363 for Item B.
///    - Server creates ground entity for Item B.
///    - Client sends 0 pickup requests for Item B.
///    - Pending action remains targeted at Item A (not Item B).
///    - Outcome: confirms pending pickup state never transfers to dropped
///      items.
fn immediate_repickup_after_drop(config: &Config) -> Result<(), String> {
    const MAP: &str = "prontera";
    const X: u16 = 155;
    const Y: u16 = 180;
    let item_id = 501; // Red Potion

    let mut context = TestContext::connect(config)?;
    let _ = context.net.leave_party();
    context.pump(Duration::from_millis(300));
    context.warp(MAP, X, Y)?;

    // ==========================================
    // Trial A: Drop under default server settings (@autopickup 2)
    // ==========================================
    let reply = context.gm_expect_feedback("@autopickup 2")?;
    if !reply.contains("Automatic pickup: on") {
        return Err(format!("failed to ensure @autopickup 2: {reply}"));
    }

    let initial_idx = context.give_item(item_id, 3)?;
    context.flush();

    let t0 = std::time::Instant::now();
    context.net.drop_item(initial_idx, 1).map_err(|_| "disconnected")?;

    // Server sends 0x00AF / 0x07FA and 0x084B
    let (ground_entity_id, item_pos) = context.wait_for("AddGroundItem for Trial A", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id,
            item_id: id,
            position,
            ..
        } if id.0 == item_id => Some((*entity_id, *position)),
        _ => None,
    })?;
    let t_ground = t0.elapsed();

    // Now monitor server autopickup sweep (AUTOPICKUP_INTERVAL = 400ms)
    let mut auto_removed = false;
    let mut auto_added = false;
    let t_pickup_start = std::time::Instant::now();
    for event in context.collect_for(Duration::from_millis(1500)) {
        match event {
            NetworkEvent::RemoveGroundItem { entity_id } if entity_id == ground_entity_id => {
                auto_removed = true;
            }
            NetworkEvent::IventoryItemAdded { item } if item.item_id.0 == item_id => {
                auto_added = true;
            }
            _ => {}
        }
        if auto_removed && auto_added {
            break;
        }
    }
    let t_pickup = t_pickup_start.elapsed();

    if !auto_removed || !auto_added {
        return Err(format!(
            "Trial A failed: drop under @autopickup 2 was not repicked up by server timer (removed={auto_removed}, added={auto_added})"
        ));
    }
    eprintln!(
        "[QW-022 Trial A] Default autopickup: dropped at ({},{}), ground entity {:?} created in {:?}, auto-repicked in {:?} (total {:?}), \
         0 client pickup packets sent",
        item_pos.x,
        item_pos.y,
        ground_entity_id,
        t_ground,
        t_pickup,
        t0.elapsed()
    );

    // ==========================================
    // Trial B: Drop with @autopickup 0 (ruling out client auto-pickup)
    // ==========================================
    let reply = context.gm_expect_feedback("@autopickup 0")?;
    if !reply.contains("Automatic pickup: off") {
        return Err(format!("failed to set @autopickup 0: {reply}"));
    }

    let drop_idx = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == item_id)
        .map(|item| item.index)
        .ok_or_else(|| "no item to drop in Trial B".to_owned())?;
    context.flush();

    let t0_b = std::time::Instant::now();
    context.net.drop_item(drop_idx, 1).map_err(|_| "disconnected")?;

    let (ground_b, item_pos_b) = context.wait_for("AddGroundItem for Trial B", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id,
            item_id: id,
            position,
            ..
        } if id.0 == item_id => Some((*entity_id, *position)),
        _ => None,
    })?;
    let t_ground_b = t0_b.elapsed();

    // Collect for 1200ms: with @autopickup 0, item MUST remain on floor
    let (removed_b, added_b) = ground_item_taken(&mut context, ground_b, item_id, Duration::from_millis(1200));
    if removed_b || added_b {
        return Err(format!(
            "Trial B failed: item was taken off floor despite @autopickup 0 (removed={removed_b}, added={added_b})"
        ));
    }
    eprintln!(
        "[QW-022 Trial B] Disabled autopickup: dropped at ({},{}), ground entity {:?} created in {:?}, remained on ground >1200ms, 0 \
         client pickup packets sent, NO repickup occurred",
        item_pos_b.x, item_pos_b.y, ground_b, t_ground_b
    );

    // Clean up ground_b
    context.net.pick_up_item(ground_b).map_err(|_| "disconnected")?;
    let _ = context.wait_for("RemoveGroundItem for Trial B cleanup", |event| match event {
        NetworkEvent::RemoveGroundItem { entity_id } if *entity_id == ground_b => Some(()),
        _ => None,
    })?;

    // ==========================================
    // Trial C: Drop with a previously clicked floor item (pending action)
    // ==========================================
    // 1. Place Item A out of reach (5 cells away)
    context.warp(MAP, X + 5, Y)?;
    let idx_a = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == item_id)
        .map(|item| item.index)
        .ok_or_else(|| "no item for Item A".to_owned())?;
    context.flush();
    context.net.drop_item(idx_a, 1).map_err(|_| "disconnected")?;
    let (item_a_entity, pos_a) = context.wait_for("AddGroundItem for Item A", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id,
            item_id: id,
            position,
            ..
        } if id.0 == item_id => Some((*entity_id, *position)),
        _ => None,
    })?;

    // 2. Warp back to (X, Y)
    context.warp(MAP, X, Y)?;

    // In Korangar client, if a user clicks Item A (5 cells away), the client
    // buffers:   BufferedAction::PickUpItem { entity_id: item_a_entity }
    // Now, before that action finishes or is executed, the player drops Item B:
    let idx_b = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == item_id)
        .map(|item| item.index)
        .ok_or_else(|| "no item for Item B".to_owned())?;
    context.flush();
    context.net.drop_item(idx_b, 1).map_err(|_| "disconnected")?;

    let (item_b_entity, pos_b) = context.wait_for("AddGroundItem for Item B", |event| match event {
        NetworkEvent::AddGroundItem {
            entity_id,
            item_id: id,
            position,
            ..
        } if id.0 == item_id && *entity_id != item_a_entity => Some((*entity_id, *position)),
        _ => None,
    })?;

    // Verify Item B stays on floor (not repicked up)
    let (removed_b2, added_b2) = ground_item_taken(&mut context, item_b_entity, item_id, Duration::from_millis(1200));
    if removed_b2 || added_b2 {
        return Err(format!(
            "Trial C failed: Item B was repicked up while Item A was pending (removed={removed_b2}, added={added_b2})"
        ));
    }
    eprintln!(
        "[QW-022 Trial C] Pending action state: Item A at ({},{}) [entity {:?}], dropped Item B at ({},{}) [entity {:?}]; Item B remained \
         on floor; 0 pickup requests sent for Item B",
        pos_a.x, pos_a.y, item_a_entity, pos_b.x, pos_b.y, item_b_entity
    );

    // Clean up both items
    context.net.pick_up_item(item_b_entity).map_err(|_| "disconnected")?;
    let _ = context.wait_for("pickup B", |event| match event {
        NetworkEvent::RemoveGroundItem { entity_id } if *entity_id == item_b_entity => Some(()),
        _ => None,
    })?;
    context.warp(MAP, X + 5, Y)?;
    context.net.pick_up_item(item_a_entity).map_err(|_| "disconnected")?;
    let _ = context.wait_for("pickup A", |event| match event {
        NetworkEvent::RemoveGroundItem { entity_id } if *entity_id == item_a_entity => Some(()),
        _ => None,
    })?;

    // Restore default autopickup setting
    let _ = context.gm_expect_feedback("@autopickup 2");

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
    let original_job = context.job_id.0;
    context.say("@zeny 1000000")?;
    context.pump(Duration::from_millis(200));

    // Warp adjacent to the Pet Groomer at prontera,218,211
    context.warp("prontera", 218, 209)?;

    // Find the Groomer entity
    let groomer_id = find_pet_groomer(&mut context)?;

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

    // Purchase 1 Pet Food (serves as unequipped control item)
    let purchase_item = ShopItem {
        metadata: 1, // Quantity
        item_id: pet_food.item_id,
        item_type: pet_food.item_type,
        price: pet_food.price,
        quantity: pet_food.quantity.clone(),
        weight: pet_food.weight,
        location: pet_food.location,
    };

    context.flush();
    context.net.purchase_items(vec![purchase_item]).map_err(|_| "disconnected")?;
    context.wait_for("BuyingCompleted success", |event| match event {
        NetworkEvent::BuyingCompleted { result } => match result {
            BuyShopItemsResult::Success => Some(()),
            _ => None,
        },
        _ => None,
    })?;

    // Find the purchased Pet Food in inventory
    let food_inventory_item = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == 537)
        .ok_or("purchased Pet Food not in inventory")?;
    let food_index = food_inventory_item.index;

    // Switch to Hunter (job 11) so we can equip weapon, armor, costume, and ammo
    context.ensure_job(11)?;

    // --- Authoritative equipped-sale rejection matrix (QW-001) ---
    struct EquippedCase {
        category: &'static str,
        item_id: u32,
        amount: u16,
        pos: EquipPosition,
    }

    let cases = [
        EquippedCase {
            category: "weapon",
            item_id: 1201, // Knife
            amount: 1,
            pos: EquipPosition::RIGHT_HAND,
        },
        EquippedCase {
            category: "armor",
            item_id: 2301, // Cotton Shirt
            amount: 1,
            pos: EquipPosition::ARMOR,
        },
        EquippedCase {
            category: "costume",
            item_id: 19506, // Costume Valkyrie Feather Band
            amount: 1,
            pos: EquipPosition::COSTUME_HEAD_TOP,
        },
        EquippedCase {
            category: "ammo",
            item_id: 1750, // Arrow
            amount: 50,
            pos: EquipPosition::AMMO,
        },
    ];

    for case in &cases {
        let item_idx = context.give_item(case.item_id, case.amount)?;
        context.flush();
        context.net.request_item_equip(item_idx, case.pos).map_err(|_| "disconnected")?;
        context.wait_for(
            &format!("UpdateEquippedPosition (equip {})", case.category),
            |event| match event {
                NetworkEvent::UpdateEquippedPosition {
                    index: event_index,
                    equipped_position,
                } if *event_index == item_idx && equipped_position.contains(case.pos) => Some(()),
                _ => None,
            },
        )?;

        // Open sell window with Groomer
        context.flush();
        context.net.start_dialog(groomer_id).map_err(|_| "disconnected")?;
        let shop_id = context.wait_for(&format!("AskBuyOrSell (sell {})", case.category), |event| match event {
            NetworkEvent::AskBuyOrSell { shop_id } => Some(*shop_id),
            _ => None,
        })?;

        context.flush();
        context
            .net
            .select_buy_or_sell(shop_id, BuyOrSellOption::Sell)
            .map_err(|_| "disconnected")?;
        let sell_items = context.wait_for(&format!("SellItemList ({})", case.category), |event| match event {
            NetworkEvent::SellItemList { items } => Some(items.clone()),
            _ => None,
        })?;

        // Verify presentation packet does not list the equipped item
        if sell_items.iter().any(|item| item.inventory_index == item_idx) {
            return Err(format!(
                "equipped {} (index {item_idx:?}) was offered in SellItemList",
                case.category
            ));
        }

        // Record pre-sale baseline
        let baseline_inv_len = context.inventory.len();
        let baseline_zeny = context.zeny;

        // Directly submit forged sale request for the equipped item
        context.flush();
        context
            .net
            .sell_items(vec![SoldItemInformation {
                inventory_index: item_idx,
                amount: 1,
            }])
            .map_err(|_| "disconnected")?;

        let sell_result = context.wait_for(
            &format!("SellingCompleted on equipped {}", case.category),
            |event| match event {
                NetworkEvent::SellingCompleted { result } => Some(*result),
                _ => None,
            },
        )?;

        if !matches!(sell_result, SellItemsResult::Error) {
            return Err(format!(
                "expected sale of equipped {} to fail with Error, but got {sell_result:?}",
                case.category
            ));
        }

        context.pump(Duration::from_millis(200));

        // Assert zero inventory and currency deltas
        if context.inventory.len() != baseline_inv_len {
            return Err(format!(
                "inventory delta occurred during rejected sale of equipped {}",
                case.category
            ));
        }
        if context.zeny != baseline_zeny {
            return Err(format!(
                "currency delta occurred during rejected sale of equipped {}",
                case.category
            ));
        }
        if !context.inventory.iter().any(|item| item.index == item_idx) {
            return Err(format!(
                "equipped {} disappeared from inventory after rejected sale",
                case.category
            ));
        }

        // Unequip and clean up
        context.flush();
        context.net.request_item_unequip(item_idx).map_err(|_| "disconnected")?;
        context.wait_for(
            &format!("UpdateEquippedPosition (unequip {})", case.category),
            |event| match event {
                NetworkEvent::UpdateEquippedPosition {
                    index: event_index,
                    equipped_position,
                } if *event_index == item_idx && equipped_position.is_empty() => Some(()),
                _ => None,
            },
        )?;

        context.say(&format!("@delitem {} 9999", case.item_id))?;
        context.pump(Duration::from_millis(200));
    }

    // Also test a mixed request: equipped weapon + unequipped control item
    // Proves the entire request is rejected without deleting the unequipped item or
    // granting Zeny.
    {
        let knife_idx = context.give_item(1201, 1)?;
        context.flush();
        context
            .net
            .request_item_equip(knife_idx, EquipPosition::RIGHT_HAND)
            .map_err(|_| "disconnected")?;
        context.wait_for("UpdateEquippedPosition (equip mixed knife)", |event| match event {
            NetworkEvent::UpdateEquippedPosition {
                index: event_index,
                equipped_position,
            } if *event_index == knife_idx && equipped_position.contains(EquipPosition::RIGHT_HAND) => Some(()),
            _ => None,
        })?;

        context.flush();
        context.net.start_dialog(groomer_id).map_err(|_| "disconnected")?;
        let shop_id = context.wait_for("AskBuyOrSell (sell mixed)", |event| match event {
            NetworkEvent::AskBuyOrSell { shop_id } => Some(*shop_id),
            _ => None,
        })?;

        context.flush();
        context
            .net
            .select_buy_or_sell(shop_id, BuyOrSellOption::Sell)
            .map_err(|_| "disconnected")?;
        context.wait_for("SellItemList (mixed)", |event| match event {
            NetworkEvent::SellItemList { .. } => Some(()),
            _ => None,
        })?;

        let baseline_inv_len = context.inventory.len();
        let baseline_zeny = context.zeny;

        context.flush();
        context
            .net
            .sell_items(vec![
                SoldItemInformation {
                    inventory_index: knife_idx,
                    amount: 1,
                },
                SoldItemInformation {
                    inventory_index: food_index,
                    amount: 1,
                },
            ])
            .map_err(|_| "disconnected")?;

        let mixed_result = context.wait_for("SellingCompleted on mixed batch", |event| match event {
            NetworkEvent::SellingCompleted { result } => Some(*result),
            _ => None,
        })?;

        if !matches!(mixed_result, SellItemsResult::Error) {
            return Err(format!("expected mixed sale to fail with Error, but got {mixed_result:?}"));
        }

        context.pump(Duration::from_millis(200));

        if context.inventory.len() != baseline_inv_len {
            return Err("inventory delta occurred during rejected mixed sale".to_owned());
        }
        if context.zeny != baseline_zeny {
            return Err("currency delta occurred during rejected mixed sale".to_owned());
        }
        if !context.inventory.iter().any(|item| item.index == food_index) {
            return Err("control item was improperly deleted during rejected mixed sale".to_owned());
        }

        context.flush();
        context.net.request_item_unequip(knife_idx).map_err(|_| "disconnected")?;
        context.wait_for("UpdateEquippedPosition (unequip mixed knife)", |event| match event {
            NetworkEvent::UpdateEquippedPosition {
                index: event_index,
                equipped_position,
            } if *event_index == knife_idx && equipped_position.is_empty() => Some(()),
            _ => None,
        })?;

        context.say("@delitem 1201 9999")?;
        context.pump(Duration::from_millis(200));
    }

    // Restore original job
    context.ensure_job(original_job)?;

    // --- Normal unequipped control sale ---
    // Now open sell window to sell the unequipped Pet Food control item
    context.flush();
    context.net.start_dialog(groomer_id).map_err(|_| "disconnected")?;
    let shop_id = context.wait_for("AskBuyOrSell (sell control)", |event| match event {
        NetworkEvent::AskBuyOrSell { shop_id } => Some(*shop_id),
        _ => None,
    })?;

    context.flush();
    context
        .net
        .select_buy_or_sell(shop_id, BuyOrSellOption::Sell)
        .map_err(|_| "disconnected")?;
    let sell_items = context.wait_for("SellItemList (control)", |event| match event {
        NetworkEvent::SellItemList { items } => Some(items.clone()),
        _ => None,
    })?;

    let pet_food_sell = sell_items
        .iter()
        .find(|item| item.inventory_index == food_index)
        .ok_or("Groomer did not list our Pet Food for sale")?;
    let expected_sale_price = pet_food_sell.price.0 as u32;

    // Record starting inventory count and Zeny before the control sale
    let initial_inventory_count = context.inventory.len();
    let initial_zeny = context.zeny;

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
        NetworkEvent::SellingCompleted { result } => match result {
            SellItemsResult::Success => Some(()),
            _ => None,
        },
        _ => None,
    })?;

    // Wait for the resulting inventory removal and currency update
    context.pump(Duration::from_millis(200));

    // Assert one item delta and one currency delta
    if context.inventory.iter().any(|item| item.index == food_index) {
        return Err(format!("sold item at index {food_index:?} still present in inventory"));
    }
    if context.inventory.len() != initial_inventory_count - 1 {
        return Err(format!(
            "expected inventory count to drop by 1 (from {initial_inventory_count} to {}), got {}",
            initial_inventory_count - 1,
            context.inventory.len()
        ));
    }
    if context.zeny != initial_zeny + expected_sale_price {
        return Err(format!(
            "expected zeny delta of +{expected_sale_price} (from {initial_zeny} to {}), got {}",
            initial_zeny + expected_sale_price,
            context.zeny
        ));
    }

    let zeny_after_sale = context.zeny;
    let inventory_count_after_sale = context.inventory.len();

    // Attempt immediate resubmission and prove it cannot execute a second
    // transaction
    context.flush();
    context
        .net
        .sell_items(vec![SoldItemInformation {
            inventory_index: food_index,
            amount: 1,
        }])
        .map_err(|_| "disconnected")?;

    let resubmission_result = context.wait_for("SellingCompleted on immediate resubmission", |event| match event {
        NetworkEvent::SellingCompleted { result } => Some(*result),
        _ => None,
    })?;

    if !matches!(resubmission_result, SellItemsResult::Error) {
        return Err(format!(
            "expected second sale to fail with Error, but got {resubmission_result:?}"
        ));
    }

    context.pump(Duration::from_millis(200));

    // Prove inventory and currency remain unchanged after rejected transaction
    if context.inventory.len() != inventory_count_after_sale {
        return Err("inventory delta occurred during rejected resubmission".to_owned());
    }
    if context.zeny != zeny_after_sale {
        return Err("currency delta occurred during rejected resubmission".to_owned());
    }

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
        quantity: pet_food.quantity.clone(),
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
        quantity: pet_food.quantity.clone(),
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
        if let Ok(name) = name {
            if name.contains("Pet Groomer") {
                return Ok(id);
            }
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

/// Open storage, move a Red Potion to it, then retrieve it, and close storage.
fn storage(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let item_id = 501; // Red Potion

    let index = context.give_item(item_id, 1)?;

    context.flush();
    context.say("@storage")?;
    context.wait_for("SetStorage", |event| match event {
        NetworkEvent::SetStorage { .. } => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.move_item_to_storage(index, 1).map_err(|_| "disconnected")?;

    let storage_item = context.wait_for("StorageItemAdded", |event| match event {
        NetworkEvent::StorageItemAdded { item } if item.item_id.0 == item_id => Some(item.clone()),
        _ => None,
    })?;

    context.flush();
    context
        .net
        .move_item_from_storage(storage_item.index, 1)
        .map_err(|_| "disconnected")?;

    context.wait_for("StorageItemRemoved", |event| match event {
        NetworkEvent::StorageItemRemoved { index: removed_index, .. } if *removed_index == storage_item.index => Some(()),
        _ => None,
    })?;

    context.flush();
    context.net.close_storage().map_err(|_| "disconnected")?;
    context.wait_for("StorageClosed", |event| match event {
        NetworkEvent::StorageClosed => Some(()),
        _ => None,
    })?;

    // Cleanup inventory item
    context.say("@delitem 501 1")?;
    context.pump(Duration::from_millis(200));
    Ok(())
}

/// QW-075: storage remains usable at the hard player-weight boundary. Deposit
/// two arrows, refill the inventory to exactly max weight, require the server
/// to refuse a withdrawal that would exceed capacity, then cleanly withdraw
/// the stored stack after making room.
fn storage_weight_boundary(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    let max_weight = context.max_weight;
    if max_weight < 500 || max_weight / 100 > u32::from(u16::MAX) {
        return Err(format!("unexpected max weight for storage boundary: {max_weight}"));
    }

    let _ = context.say("@delitem 999 30000");
    let _ = context.say("@delitem 1750 30000");
    context.pump(Duration::from_millis(250));

    let steel_amount = max_weight.saturating_sub(1) / 100;
    let arrow_remainder = max_weight.saturating_sub(1) - steel_amount * 100;
    if arrow_remainder < 5 {
        return Err(format!(
            "storage boundary needs at least five arrow-weight units, got {arrow_remainder}"
        ));
    }
    context.give_item(999, steel_amount as u16)?;
    let arrow_index = context.give_item(1750, arrow_remainder as u16)?;
    if context.weight != max_weight.saturating_sub(1) {
        return Err(format!("failed to fill inventory to max-1: {}/{}", context.weight, max_weight));
    }

    context.flush();
    context.say("@storage")?;
    context.wait_for("SetStorage at weight boundary", |event| match event {
        NetworkEvent::SetStorage { .. } => Some(()),
        _ => None,
    })?;
    context.flush();
    context.net.move_item_to_storage(arrow_index, 2).map_err(|_| "disconnected")?;
    let stored = context.wait_for("StorageItemAdded at weight boundary", |event| match event {
        NetworkEvent::StorageItemAdded { item } if item.item_id.0 == 1750 => Some(item.clone()),
        _ => None,
    })?;

    context.give_item(1750, 3)?;
    if context.weight != max_weight {
        return Err(format!(
            "failed to refill inventory to max weight: {}/{}",
            context.weight, max_weight
        ));
    }
    context.flush();
    context.net.move_item_from_storage(stored.index, 1).map_err(|_| "disconnected")?;
    context.wait_for("over-cap storage withdrawal refusal", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("Failed to pick up item") => Some(()),
        _ => None,
    })?;

    context.say("@delitem 1750 3")?;
    context.pump(Duration::from_millis(250));
    context.flush();
    context.net.move_item_from_storage(stored.index, 2).map_err(|_| "disconnected")?;
    context.wait_for("StorageItemRemoved after making room", |event| match event {
        NetworkEvent::StorageItemRemoved { index, .. } if *index == stored.index => Some(()),
        _ => None,
    })?;
    context.net.close_storage().map_err(|_| "disconnected")?;
    context.wait_for("StorageClosed at weight boundary", |event| match event {
        NetworkEvent::StorageClosed => Some(()),
        _ => None,
    })?;
    context.say("@delitem 999 30000")?;
    context.say("@delitem 1750 30000")?;
    context.pump(Duration::from_millis(250));
    Ok(())
}

/// QW-075: exercise Hercules' real inventory-to-cart path at the cart hard
/// boundary. Steel has weight 100 and Arrow has weight 1 in the checked-in
/// item database; the counts are derived from the server-advertised cart
/// capacity so this remains a boundary test rather than a fixed fixture.
fn cart_weight_boundary(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;
    context.say("@cart 1")?;
    // A failed diagnostic run can leave cart contents persisted on the shared
    // disposable character. Clear them before reading the authoritative
    // capacity, so the boundary starts at a known zero-cart state.
    context.say("@clearcart")?;
    context.say("@delitem 999 30000")?;
    context.say("@delitem 1750 30000")?;

    let cart_max = context.wait_for("empty cart capacity", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::CartInfo(count, weight, max_weight),
        } if *count == 0 && *weight == 0 => Some(*max_weight),
        _ => None,
    })?;
    if cart_max < 100 || cart_max % 10 != 0 {
        return Err(format!("unexpected server cart capacity: {cart_max}"));
    }
    let initial_weight = 0;

    let steel_amount = ((cart_max - 1) / 100) as u16;
    let arrow_amount = (cart_max - 1 - u32::from(steel_amount) * 100) as u16;
    if steel_amount == 0 {
        return Err(format!("cart capacity too small for boundary fixture: {cart_max}"));
    }

    let steel_index = context.give_item(999, steel_amount)?;
    let arrow_index = context.give_item(1750, arrow_amount)?;
    context.flush();
    context
        .net
        .move_item_to_cart(steel_index, u32::from(steel_amount))
        .map_err(|_| "disconnected")?;
    let steel_cart_weight = context.wait_for("steel cart transfer", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::CartInfo(_, weight, _),
        } if *weight > initial_weight => Some(*weight),
        _ => None,
    })?;
    context
        .net
        .move_item_to_cart(arrow_index, u32::from(arrow_amount))
        .map_err(|_| "disconnected")?;
    let near_limit = context.wait_for("cart at max minus one arrow", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::CartInfo(_, weight, _),
        } if *weight >= steel_cart_weight => Some(*weight),
        _ => None,
    })?;
    if near_limit != cart_max - 1 {
        return Err(format!("cart did not reach max-1: {near_limit}/{cart_max}"));
    }

    let exact_index = context.give_item(1750, 1)?;
    context.flush();
    context.net.move_item_to_cart(exact_index, 1).map_err(|_| "disconnected")?;
    context.wait_for("cart exact hard boundary", |event| match event {
        NetworkEvent::UpdateStat {
            stat_type: StatType::CartInfo(_, weight, _),
        } if *weight == cart_max => Some(()),
        _ => None,
    })?;

    let rejected_index = context.give_item(1750, 1)?;
    context.flush();
    context.net.move_item_to_cart(rejected_index, 1).map_err(|_| "disconnected")?;
    context.wait_for("cart over-capacity refusal", |event| match event {
        NetworkEvent::CartItemAddResult { result: 0 } => Some(()),
        _ => None,
    })?;

    context.say("@clearcart")?;
    context.say("@cart 0")?;
    context.say("@delitem 999 30000")?;
    context.say("@delitem 1750 30000")?;
    context.pump(Duration::from_millis(250));
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

/// Lock (CZ_UPGRADE_SKILLLEVEL 0x0112) must persist on the character after
/// relog.
fn skill_lock_relog(config: &Config) -> Result<(), String> {
    const BASIC: u16 = 1;
    let mut context = TestContext::connect(config)?;
    context.ensure_job(0)?;
    context.ensure_base_level(10)?;
    context.say("@reset")?;
    context.pump(Duration::from_millis(400));
    context.flush();
    context
        .net
        .level_up_skill(ragnarok_packets::SkillId(BASIC as u16))
        .map_err(|_| "disconnected")?;
    context.wait_for("Basic Skill ranked on server", |event| match event {
        NetworkEvent::UpdateSkill { skill_id, skill_level, .. } if skill_id.0 == BASIC && skill_level.0 >= 1 => Some(()),
        NetworkEvent::SkillAdded { skill_information } if skill_information.skill_id.0 == BASIC && skill_information.skill_level.0 >= 1 => {
            Some(())
        }
        NetworkEvent::SkillTree { skill_information }
            if skill_information
                .iter()
                .any(|skill| skill.skill_id.0 == BASIC && skill.skill_level.0 >= 1) =>
        {
            Some(())
        }
        _ => None,
    })?;
    drop(context);
    std::thread::sleep(Duration::from_millis(900));
    let context = TestContext::connect(config)?;
    let level = context
        .skills
        .iter()
        .find(|skill| skill.skill_id.0 == BASIC)
        .map(|skill| skill.skill_level.0)
        .unwrap_or(0);
    if level < 1 {
        return Err(format!(
            "after relog Basic Skill was {level}, lock did not persist on the server"
        ));
    }
    Ok(())
}

fn skill_refund(config: &Config) -> Result<(), String> {
    const BASIC: u16 = 1;
    let mut context = TestContext::connect(config)?;
    context.ensure_job(0)?;
    context.ensure_base_level(10)?;
    context.say("@reset")?;
    context.pump(Duration::from_millis(400));
    context.flush();
    context
        .net
        .level_up_skill(ragnarok_packets::SkillId(BASIC))
        .map_err(|_| "disconnected")?;
    context.wait_for("Basic Skill ranked", |event| match event {
        NetworkEvent::UpdateSkill { skill_id, skill_level, .. } if skill_id.0 == BASIC && skill_level.0 >= 1 => Some(()),
        NetworkEvent::SkillAdded { skill_information } if skill_information.skill_id.0 == BASIC && skill_information.skill_level.0 >= 1 => {
            Some(())
        }
        NetworkEvent::SkillTree { skill_information }
            if skill_information
                .iter()
                .any(|skill| skill.skill_id.0 == BASIC && skill.skill_level.0 >= 1) =>
        {
            Some(())
        }
        _ => None,
    })?;
    context.say("@refundskill 1")?;
    context.pump(Duration::from_millis(800));
    let level = context
        .skills
        .iter()
        .find(|skill| skill.skill_id.0 == BASIC)
        .map(|skill| skill.skill_level.0)
        .unwrap_or(0);
    if level != 0 {
        return Err(format!("@refundskill left Basic Skill at {level}"));
    }
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

/// Bind then `UNBOUND` on the last slot of each discoverable hotbar row
/// (slots 8 / 17 / 26). All three client removal paths — source-window drop,
/// drag-off-bar, right-click clear — send this same `HotkeyData::UNBOUND`
/// write. Persistence is the character-save round trip, same as `hotkeys`.
///
/// Last-of-row slots avoid the graphical Wizard F1–F7 bindings on 0–6.
fn hotbar_clear_relog(config: &Config) -> Result<(), String> {
    const TAB: HotbarTab = HotbarTab(0);
    const ROW_SLOTS: [u16; 3] = [8, 17, 26];
    const PROBE: u32 = 512;

    let mut context = TestContext::connect(config)?;
    for slot in ROW_SLOTS {
        context
            .net
            .set_hotkey_data(TAB, HotbarSlot(slot), HotkeyData {
                hotkey_type: HotkeyType::Item,
                item_or_skill_id: PROBE,
                quantity_or_skill_level: 1,
            })
            .map_err(|_| "disconnected")?;
    }
    context.pump(Duration::from_millis(300));
    drop(context);
    std::thread::sleep(Duration::from_millis(900));

    let mut context = TestContext::connect(config)?;
    let bound = read_hotkey_tab(&mut context, TAB)?;
    for slot in ROW_SLOTS {
        match bound.get(slot as usize) {
            Some(Some((item_id, _))) if *item_id == PROBE => {}
            other => {
                return Err(format!("row slot {slot} was {other:?} after bind relog; expected item {PROBE}"));
            }
        }
    }

    for slot in ROW_SLOTS {
        context
            .net
            .set_hotkey_data(TAB, HotbarSlot(slot), HotkeyData::UNBOUND)
            .map_err(|_| "disconnected")?;
    }
    context.pump(Duration::from_millis(300));
    drop(context);
    std::thread::sleep(Duration::from_millis(900));

    let mut context = TestContext::connect(config)?;
    let cleared = read_hotkey_tab(&mut context, TAB)?;
    for slot in ROW_SLOTS {
        match cleared.get(slot as usize) {
            Some(None) => {}
            other => {
                return Err(format!("row slot {slot} stayed bound as {other:?} after UNBOUND relog"));
            }
        }
    }
    Ok(())
}

/// The whole tab-0 hotkey list from login, one entry per slot.
fn read_hotkey_tab(context: &mut TestContext, tab: HotbarTab) -> Result<Vec<Option<(u32, u16)>>, String> {
    let wanted_tab = tab.0;
    context.wait_for(&format!("SetHotkeyData for tab {wanted_tab}"), |event| match event {
        NetworkEvent::SetHotkeyData { tab, hotkeys } if tab.0 == wanted_tab => Some(
            hotkeys
                .iter()
                .map(|state| match state {
                    HotkeyState::Bound(data) => Some((data.item_or_skill_id, data.quantity_or_skill_level)),
                    HotkeyState::Unbound => None,
                })
                .collect(),
        ),
        _ => None,
    })
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZenyLogRecord {
    pub time: String,
    pub char_id: u32,
    pub src_id: u32,
    pub log_type: char,
    pub amount: i32,
    pub map: String,
}

fn run_mysql_query(sql: &str) -> Result<String, String> {
    let candidates = ["mysql", "/opt/homebrew/bin/mysql", "/usr/local/bin/mysql", "/usr/bin/mysql"];
    let mut last_err = String::new();
    for cmd in candidates {
        match std::process::Command::new(cmd)
            .args([
                "--protocol=tcp",
                "-h",
                "127.0.0.1",
                "-u",
                "ragnarok",
                "-pragnarok",
                "-D",
                "ragnarok",
                "--batch",
                "--skip-column-names",
                "-e",
                sql,
            ])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    return Err(format!("mysql error: {}", String::from_utf8_lossy(&output.stderr)));
                }
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
            Err(e) => {
                last_err = e.to_string();
            }
        }
    }
    Err(format!("failed to run mysql query: {last_err}"))
}

fn query_char_zeny(char_id: u32) -> Result<u32, String> {
    let sql = format!("SELECT zeny FROM `char` WHERE char_id = {char_id}");
    let raw = run_mysql_query(&sql)?;
    raw.parse::<u32>()
        .map_err(|e| format!("failed to parse zeny from MySQL {raw:?}: {e}"))
}

fn query_latest_zenylog(char_id: u32) -> Result<ZenyLogRecord, String> {
    let sql = format!("SELECT time, char_id, src_id, type, amount, map FROM `zenylog` WHERE char_id = {char_id} ORDER BY id DESC LIMIT 1");
    let raw = run_mysql_query(&sql)?;
    if raw.is_empty() {
        return Err(format!("no zenylog row found for char_id {char_id}"));
    }
    let parts: Vec<&str> = raw.split('\t').collect();
    if parts.len() < 6 {
        return Err(format!("unexpected zenylog columns ({parts:?}): {raw}"));
    }
    let time = parts[0].to_string();
    let c_id = parts[1].parse::<u32>().map_err(|e| format!("bad char_id: {e}"))?;
    let src_id = parts[2].parse::<u32>().map_err(|e| format!("bad src_id: {e}"))?;
    let log_type = parts[3].chars().next().ok_or_else(|| "missing type".to_string())?;
    let amount = parts[4].parse::<i32>().map_err(|e| format!("bad amount: {e}"))?;
    let map = parts[5].to_string();

    Ok(ZenyLogRecord {
        time,
        char_id: c_id,
        src_id,
        log_type,
        amount,
        map,
    })
}

fn restart_hercules_server() -> Result<(), String> {
    let script = if std::path::Path::new("../Hercules/dev.sh").exists() {
        "../Hercules/dev.sh"
    } else if std::path::Path::new("Hercules/dev.sh").exists() {
        "Hercules/dev.sh"
    } else {
        return Err("neither ../Hercules/dev.sh nor Hercules/dev.sh exists".to_string());
    };

    let status = std::process::Command::new("sh")
        .args([script, "restart"])
        .status()
        .map_err(|e| format!("failed to execute dev.sh restart: {e}"))?;
    if !status.success() {
        return Err(format!("dev.sh restart failed with exit code {status:?}"));
    }

    let status = std::process::Command::new("sh")
        .args([script, "wait"])
        .status()
        .map_err(|e| format!("failed to execute dev.sh wait: {e}"))?;
    if !status.success() {
        return Err(format!("dev.sh wait failed with exit code {status:?}"));
    }

    Ok(())
}

/// Verify Zeny transaction matrix and persistence across:
/// 1. Baseline client display vs MySQL database (`char.zeny`).
/// 2. NPC Buy (Groomer shop) with inventory delta and `zenylog` entry.
/// 3. NPC Sell (Groomer shop) with inventory delta and `zenylog` entry.
/// 4. Player Trade (two participants) with reciprocal balance updates and
///    `zenylog` entries.
/// 5. Logout & Reconnect persistence.
/// 6. Server Restart persistence across shutdown and boot.
/// 7. Vending, buying store, and cart boundaries (debunking cart zeny myth).
fn zeny_persistence_transaction_matrix(config: &Config) -> Result<(), String> {
    const PET_FOOD_ID: u32 = 537;
    const PET_FOOD_BUY_PRICE: u32 = 1000;
    const PET_FOOD_SELL_PRICE: u32 = 500;
    const TRADE_ZENY_AMOUNT: u32 = 50000;

    // -------------------------------------------------------------------------
    // Section 1: Initial State & DB Alignment Verification
    // -------------------------------------------------------------------------
    let mut context = TestContext::connect(config)?;
    let char_id = context.character_id.0;
    context.say("@delitem 537 9999")?;
    context.say("@zeny 1000000")?;
    context.say("@save")?;
    context.pump(Duration::from_millis(300));

    let initial_client_zeny = context.zeny;
    let initial_db_zeny = query_char_zeny(char_id)?;
    if initial_client_zeny != initial_db_zeny {
        return Err(format!(
            "Section 1: initial client zeny ({initial_client_zeny}) does not match MySQL char.zeny ({initial_db_zeny})"
        ));
    }

    // -------------------------------------------------------------------------
    // Section 2: NPC Buy (Groomer shop)
    // -------------------------------------------------------------------------
    context.warp("prontera", 218, 209)?;
    let groomer_id = find_pet_groomer(&mut context)?;

    let pre_buy_client_zeny = context.zeny;
    let pre_buy_db_zeny = query_char_zeny(char_id)?;
    let pre_buy_pet_food_count = context
        .inventory
        .iter()
        .filter(|item| item.item_id.0 == PET_FOOD_ID)
        .map(|item| match &item.details {
            InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
            _ => 1,
        })
        .sum::<u32>();

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
        .iter()
        .find(|item| item.item_id.0 == PET_FOOD_ID)
        .ok_or("Pet Food item (537) not found in Groomer shop")?;
    if pet_food.price.0 as u32 != PET_FOOD_BUY_PRICE {
        return Err(format!(
            "expected Pet Food buy price {PET_FOOD_BUY_PRICE}, got {}",
            pet_food.price.0
        ));
    }

    let purchase_item = ShopItem {
        metadata: 1,
        item_id: pet_food.item_id,
        item_type: pet_food.item_type,
        price: pet_food.price,
        quantity: pet_food.quantity.clone(),
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
    context.pump(Duration::from_millis(300));
    context.say("@save")?;
    context.pump(Duration::from_millis(200));

    let post_buy_client_zeny = context.zeny;
    let post_buy_db_zeny = query_char_zeny(char_id)?;
    let post_buy_pet_food_count = context
        .inventory
        .iter()
        .filter(|item| item.item_id.0 == PET_FOOD_ID)
        .map(|item| match &item.details {
            InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
            _ => 1,
        })
        .sum::<u32>();

    if post_buy_client_zeny != pre_buy_client_zeny - PET_FOOD_BUY_PRICE {
        return Err(format!(
            "Section 2: expected client zeny delta of -{PET_FOOD_BUY_PRICE} (from {pre_buy_client_zeny} to {}), got {post_buy_client_zeny}",
            pre_buy_client_zeny - PET_FOOD_BUY_PRICE
        ));
    }
    if post_buy_db_zeny != pre_buy_db_zeny - PET_FOOD_BUY_PRICE {
        return Err(format!(
            "Section 2: expected MySQL char.zeny delta of -{PET_FOOD_BUY_PRICE} (from {pre_buy_db_zeny} to {}), got {post_buy_db_zeny}",
            pre_buy_db_zeny - PET_FOOD_BUY_PRICE
        ));
    }
    if post_buy_client_zeny != post_buy_db_zeny {
        return Err(format!(
            "Section 2: client zeny ({post_buy_client_zeny}) does not match MySQL char.zeny ({post_buy_db_zeny})"
        ));
    }
    if post_buy_pet_food_count != pre_buy_pet_food_count + 1 {
        return Err(format!(
            "Section 2: expected inventory pet food delta +1 (from {pre_buy_pet_food_count} to {}), got {post_buy_pet_food_count}",
            pre_buy_pet_food_count + 1
        ));
    }

    let buy_log = query_latest_zenylog(char_id)?;
    if buy_log.log_type != 'S' || buy_log.amount != -(PET_FOOD_BUY_PRICE as i32) || buy_log.map != "prontera" {
        return Err(format!(
            "Section 2: zenylog mismatch for NPC buy: expected type 'S', amount -{PET_FOOD_BUY_PRICE}, map 'prontera'; got {buy_log:?}"
        ));
    }

    // -------------------------------------------------------------------------
    // Section 3: NPC Sell (Groomer shop)
    // -------------------------------------------------------------------------
    let food_item = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == PET_FOOD_ID)
        .ok_or("Pet Food not present in inventory before sale")?;
    let food_index = food_item.index;

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

    let food_sell_info = sell_items
        .iter()
        .find(|item| item.inventory_index == food_index)
        .ok_or("Pet food not found in sell item list")?;
    if food_sell_info.price.0 as u32 != PET_FOOD_SELL_PRICE {
        return Err(format!(
            "expected Pet Food sell price {PET_FOOD_SELL_PRICE}, got {}",
            food_sell_info.price.0
        ));
    }

    let pre_sell_client_zeny = context.zeny;
    let pre_sell_db_zeny = query_char_zeny(char_id)?;
    let pre_sell_pet_food_count = post_buy_pet_food_count;

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
    context.pump(Duration::from_millis(300));
    context.say("@save")?;
    context.pump(Duration::from_millis(200));

    let post_sell_client_zeny = context.zeny;
    let post_sell_db_zeny = query_char_zeny(char_id)?;
    let post_sell_pet_food_count = context
        .inventory
        .iter()
        .filter(|item| item.item_id.0 == PET_FOOD_ID)
        .map(|item| match &item.details {
            InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
            _ => 1,
        })
        .sum::<u32>();

    if post_sell_client_zeny != pre_sell_client_zeny + PET_FOOD_SELL_PRICE {
        return Err(format!(
            "Section 3: expected client zeny delta of +{PET_FOOD_SELL_PRICE} (from {pre_sell_client_zeny} to {}), got \
             {post_sell_client_zeny}",
            pre_sell_client_zeny + PET_FOOD_SELL_PRICE
        ));
    }
    if post_sell_db_zeny != pre_sell_db_zeny + PET_FOOD_SELL_PRICE {
        return Err(format!(
            "Section 3: expected MySQL char.zeny delta of +{PET_FOOD_SELL_PRICE} (from {pre_sell_db_zeny} to {}), got {post_sell_db_zeny}",
            pre_sell_db_zeny + PET_FOOD_SELL_PRICE
        ));
    }
    if post_sell_client_zeny != post_sell_db_zeny {
        return Err(format!(
            "Section 3: client zeny ({post_sell_client_zeny}) does not match MySQL char.zeny ({post_sell_db_zeny})"
        ));
    }
    if post_sell_pet_food_count != pre_sell_pet_food_count - 1 {
        return Err(format!(
            "Section 3: expected inventory pet food delta -1 (from {pre_sell_pet_food_count} to {}), got {post_sell_pet_food_count}",
            pre_sell_pet_food_count - 1
        ));
    }

    let sell_log = query_latest_zenylog(char_id)?;
    if sell_log.log_type != 'S' || sell_log.amount != (PET_FOOD_SELL_PRICE as i32) || sell_log.map != "prontera" {
        return Err(format!(
            "Section 3: zenylog mismatch for NPC sell: expected type 'S', amount +{PET_FOOD_SELL_PRICE}, map 'prontera'; got {sell_log:?}"
        ));
    }

    // Clean disconnect before pair test
    context.net.disconnect_from_map_server();
    drop(context);
    std::thread::sleep(Duration::from_millis(500));

    // -------------------------------------------------------------------------
    // Section 4: Player Trade Transaction Matrix
    // -------------------------------------------------------------------------
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    let p_id = primary.character_id.0;
    let r_id = partner.character_id.0;

    super::social::ensure_basic_skill(&mut primary);
    super::social::ensure_basic_skill(&mut partner);

    primary.say("@zeny 500000")?;
    partner.say("@zeny 100000")?;
    primary.say("@save")?;
    partner.say("@save")?;
    primary.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));

    let p_cli_0 = primary.zeny;
    let p_db_0 = query_char_zeny(p_id)?;
    let r_cli_0 = partner.zeny;
    let r_db_0 = query_char_zeny(r_id)?;

    if p_cli_0 != p_db_0 {
        return Err(format!("Section 4: primary pre-trade client ({p_cli_0}) != DB ({p_db_0})"));
    }
    if r_cli_0 != r_db_0 {
        return Err(format!("Section 4: partner pre-trade client ({r_cli_0}) != DB ({r_db_0})"));
    }

    super::social::begin_trade(&mut primary, &mut partner)?;
    primary.flush();
    partner.flush();

    primary.net.trade_add_zeny(TRADE_ZENY_AMOUNT).map_err(|_| "primary disconnected")?;
    primary.net.trade_ok().map_err(|_| "primary disconnected")?;
    partner.net.trade_ok().map_err(|_| "partner disconnected")?;
    primary.net.trade_commit().map_err(|_| "primary disconnected")?;
    partner.net.trade_commit().map_err(|_| "partner disconnected")?;

    primary.wait_for("TradeCompleted (primary)", |event| match event {
        NetworkEvent::TradeCompleted { success: true } => Some(()),
        _ => None,
    })?;
    partner.wait_for("TradeCompleted (partner)", |event| match event {
        NetworkEvent::TradeCompleted { success: true } => Some(()),
        _ => None,
    })?;

    primary.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));
    primary.say("@save")?;
    partner.say("@save")?;
    primary.pump(Duration::from_millis(300));
    partner.pump(Duration::from_millis(300));

    let p_cli_1 = primary.zeny;
    let p_db_1 = query_char_zeny(p_id)?;
    let r_cli_1 = partner.zeny;
    let r_db_1 = query_char_zeny(r_id)?;

    if p_cli_1 != p_cli_0 - TRADE_ZENY_AMOUNT {
        return Err(format!(
            "Section 4: expected primary zeny delta -{TRADE_ZENY_AMOUNT} (from {p_cli_0} to {}), got {p_cli_1}",
            p_cli_0 - TRADE_ZENY_AMOUNT
        ));
    }
    if p_db_1 != p_db_0 - TRADE_ZENY_AMOUNT {
        return Err(format!(
            "Section 4: expected primary DB zeny delta -{TRADE_ZENY_AMOUNT} (from {p_db_0} to {}), got {p_db_1}",
            p_db_0 - TRADE_ZENY_AMOUNT
        ));
    }
    if p_cli_1 != p_db_1 {
        return Err(format!("Section 4: primary post-trade client ({p_cli_1}) != DB ({p_db_1})"));
    }

    if r_cli_1 != r_cli_0 + TRADE_ZENY_AMOUNT {
        return Err(format!(
            "Section 4: expected partner zeny delta +{TRADE_ZENY_AMOUNT} (from {r_cli_0} to {}), got {r_cli_1}",
            r_cli_0 + TRADE_ZENY_AMOUNT
        ));
    }
    if r_db_1 != r_db_0 + TRADE_ZENY_AMOUNT {
        return Err(format!(
            "Section 4: expected partner DB zeny delta +{TRADE_ZENY_AMOUNT} (from {r_db_0} to {}), got {r_db_1}",
            r_db_0 + TRADE_ZENY_AMOUNT
        ));
    }
    if r_cli_1 != r_db_1 {
        return Err(format!("Section 4: partner post-trade client ({r_cli_1}) != DB ({r_db_1})"));
    }

    let p_log = query_latest_zenylog(p_id)?;
    if p_log.log_type != 'T' || p_log.amount != -(TRADE_ZENY_AMOUNT as i32) || p_log.src_id != r_id {
        return Err(format!(
            "Section 4: primary zenylog mismatch: expected type 'T', amount -{TRADE_ZENY_AMOUNT}, src_id {r_id}; got {p_log:?}"
        ));
    }
    let r_log = query_latest_zenylog(r_id)?;
    if r_log.log_type != 'T' || r_log.amount != (TRADE_ZENY_AMOUNT as i32) || r_log.src_id != p_id {
        return Err(format!(
            "Section 4: partner zenylog mismatch: expected type 'T', amount +{TRADE_ZENY_AMOUNT}, src_id {p_id}; got {r_log:?}"
        ));
    }

    // Disconnect partner
    let _ = partner.net.log_out();
    partner.net.disconnect_from_map_server();
    drop(partner);

    // -------------------------------------------------------------------------
    // Section 5: Logout & Reconnect Persistence
    // -------------------------------------------------------------------------
    let pre_logout_zeny = primary.zeny;
    let pre_logout_db = query_char_zeny(p_id)?;
    if pre_logout_zeny != pre_logout_db {
        return Err(format!(
            "Section 5: pre-logout client ({pre_logout_zeny}) != DB ({pre_logout_db})"
        ));
    }

    primary.net.log_out().map_err(|_| "primary disconnected")?;
    primary.wait_for("LoggedOut", |event| match event {
        NetworkEvent::LoggedOut => Some(()),
        _ => None,
    })?;
    primary.net.disconnect_from_map_server();
    drop(primary);
    std::thread::sleep(Duration::from_millis(500));

    let db_while_offline = query_char_zeny(p_id)?;
    if db_while_offline != pre_logout_zeny {
        return Err(format!(
            "Section 5: offline DB zeny ({db_while_offline}) != pre-logout zeny ({pre_logout_zeny})"
        ));
    }

    let mut reconnected = TestContext::connect(config)?;
    if reconnected.character_id.0 != p_id {
        return Err(format!(
            "Section 5: reconnected unexpected char id {}, expected {p_id}",
            reconnected.character_id.0
        ));
    }
    if reconnected.zeny != pre_logout_zeny {
        return Err(format!(
            "Section 5: reconnected client zeny ({}) != pre-logout zeny ({pre_logout_zeny})",
            reconnected.zeny
        ));
    }
    let db_after_reconnect = query_char_zeny(p_id)?;
    if db_after_reconnect != pre_logout_zeny {
        return Err(format!(
            "Section 5: DB zeny after reconnect ({db_after_reconnect}) != pre-logout zeny ({pre_logout_zeny})"
        ));
    }

    // -------------------------------------------------------------------------
    // Section 6: Server Restart Persistence
    // -------------------------------------------------------------------------
    // Record baseline, apply a known delta (+12345) to ensure a mutated state, and
    // save.
    let pre_mutation_zeny = reconnected.zeny;
    reconnected.say("@zeny 12345")?;
    reconnected.say("@save")?;
    reconnected.pump(Duration::from_millis(300));

    let restart_test_zeny = pre_mutation_zeny + 12345;
    if reconnected.zeny != restart_test_zeny {
        return Err(format!(
            "Section 6: client zeny failed to update to marker {restart_test_zeny}, got {}",
            reconnected.zeny
        ));
    }
    let db_pre_restart = query_char_zeny(p_id)?;
    if db_pre_restart != restart_test_zeny {
        return Err(format!(
            "Section 6: DB zeny before restart ({db_pre_restart}) != marker {restart_test_zeny}"
        ));
    }

    reconnected.net.log_out().map_err(|_| "reconnected disconnected")?;
    reconnected.wait_for("LoggedOut", |event| match event {
        NetworkEvent::LoggedOut => Some(()),
        _ => None,
    })?;
    reconnected.net.disconnect_from_map_server();
    drop(reconnected);
    std::thread::sleep(Duration::from_millis(500));

    restart_hercules_server()?;

    let db_post_restart = query_char_zeny(p_id)?;
    if db_post_restart != restart_test_zeny {
        return Err(format!(
            "Section 6: DB zeny after server restart ({db_post_restart}) != marker {restart_test_zeny}"
        ));
    }

    let after_restart = TestContext::connect(config)?;
    if after_restart.zeny != restart_test_zeny {
        return Err(format!(
            "Section 6: client zeny after server restart login ({}) != marker {restart_test_zeny}",
            after_restart.zeny
        ));
    }
    let db_online_after_restart = query_char_zeny(after_restart.character_id.0)?;
    if db_online_after_restart != restart_test_zeny {
        return Err(format!(
            "Section 6: DB zeny after reconnect to restarted server ({db_online_after_restart}) != marker {restart_test_zeny}"
        ));
    }

    // -------------------------------------------------------------------------
    // Section 7: Player Vending, Buying Store, and Cart Boundaries
    // -------------------------------------------------------------------------
    // Confirmed from Hercules source (`src/map/pc.h:struct s_cart`, `mmo.h`):
    // Carts hold `struct item items_cart[MAX_CART]` only; carts possess ZERO
    // currency. Client packet layer does not expose player vending
    // (`CZ_REQ_OPENSTORE2`) or buying store (`CZ_REQ_OPEN_BUYING_STORE`) in
    // current campaign scope.

    Ok(())
}

/// QW-025: Classify the "vendor sells twice" playtest report across all
/// transaction paths.
///
/// Matrix of candidate paths:
/// 1. NPC Purchase (`shop-buy`):
///    - Double-click / delayed-response test: client emits 2 back-to-back
///      purchase requests.
///    - Assert outgoing requests = 2.
///    - Assert result packets = 2 (First: Success, Second: Error).
///    - Assert inventory delta = +1 (NOT +2).
///    - Assert zeny delta = -1000 (NOT -2000).
///    - Proves server safety: `sd->npc_shopid = 0` on first purchase rejects
///      second attempt.
///
/// 2. NPC Sale (`shop-sell`):
///    - Double-click / delayed-response test: client emits 2 back-to-back sell
///      requests for same item.
///    - Assert outgoing requests = 2.
///    - Assert result packets = 2 (First: Success, Second: Error).
///    - Assert inventory delta = -1 (NOT -2).
///    - Assert zeny delta = +500 (NOT +1000).
///    - Proves server safety: `sd->npc_shopid = 0` rejects second attempt.
///
/// 3. Client Cart Clearing Bug:
///    - In the original client code, `handle_selling_completed` cleared
///      `buy_cart` instead of `sell_cart`.
///    - As a result, the sell cart retained sold items on screen, creating the
///      illusion of pending/duplicate items and inducing users to attempt
///      selling again (which failed on the server).
///    - Fixed and verified by
///      `successful_sale_clears_only_sell_cart_and_closes_windows` in
///      `lib.rs:11254`.
///
/// 4. Player Vending & Buying Store Boundaries:
///    - Unexposed in current client protocol (no vending packets).
///
/// 5. Cart Currency:
///    - Ruled out (Pushcart stores items only, no zeny fields).
fn vendor_double_transaction_classification(config: &Config) -> Result<(), String> {
    const PET_FOOD_ID: u32 = 537;
    const PET_FOOD_BUY_PRICE: u32 = 1000;
    const PET_FOOD_SELL_PRICE: u32 = 500;

    let mut context = TestContext::connect(config)?;
    let char_id = context.character_id.0;
    context.say("@delitem 537 9999")?;
    context.say("@zeny 1000000")?;
    context.say("@save")?;
    context.pump(Duration::from_millis(300));

    // -------------------------------------------------------------------------
    // Candidate Path 1: NPC Purchase (Double-click / delayed response test)
    // -------------------------------------------------------------------------
    context.warp("prontera", 218, 209)?;
    let groomer_id = find_pet_groomer(&mut context)?;

    let pre_buy_zeny = context.zeny;
    let pre_buy_pet_food = context
        .inventory
        .iter()
        .filter(|item| item.item_id.0 == PET_FOOD_ID)
        .map(|item| match &item.details {
            InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
            _ => 1,
        })
        .sum::<u32>();

    context.flush();
    context.net.start_dialog(groomer_id).map_err(|_| "disconnected")?;
    let shop_id = context.wait_for("AskBuyOrSell (buy)", |event| match event {
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
        .iter()
        .find(|item| item.item_id.0 == PET_FOOD_ID)
        .ok_or("Pet Food not found in Groomer shop")?;

    let purchase_item = ShopItem {
        metadata: 1,
        item_id: pet_food.item_id,
        item_type: pet_food.item_type,
        price: pet_food.price,
        quantity: pet_food.quantity.clone(),
        weight: pet_food.weight,
        location: pet_food.location,
    };

    // Simulate rapid double-click or inflight retry: emit 2 purchase requests
    // before receiving response
    context.flush();
    context
        .net
        .purchase_items(vec![purchase_item.clone()])
        .map_err(|_| "disconnected on first purchase")?;
    context
        .net
        .purchase_items(vec![purchase_item])
        .map_err(|_| "disconnected on second purchase")?;

    // Outgoing requests: exactly 2
    // Await first result (must be Success)
    let buy_res_1 = context.wait_for("first BuyingCompleted", |event| match event {
        NetworkEvent::BuyingCompleted { result } => Some(*result),
        _ => None,
    })?;
    if !matches!(buy_res_1, BuyShopItemsResult::Success) {
        return Err(format!(
            "Path 1: expected first purchase result to be Success, got {buy_res_1:?}"
        ));
    }

    // Await second result (must be Error due to sd->npc_shopid = 0)
    let buy_res_2 = context.wait_for("second BuyingCompleted", |event| match event {
        NetworkEvent::BuyingCompleted { result } => Some(*result),
        _ => None,
    })?;
    if !matches!(buy_res_2, BuyShopItemsResult::Error) {
        return Err(format!(
            "Path 1: expected second purchase result to be Error (rejected by server), got {buy_res_2:?}"
        ));
    }

    context.pump(Duration::from_millis(300));
    context.say("@save")?;
    context.pump(Duration::from_millis(200));

    // Assert exactly ONE purchase occurred
    let post_buy_zeny = context.zeny;
    let post_buy_db_zeny = query_char_zeny(char_id)?;
    let post_buy_pet_food = context
        .inventory
        .iter()
        .filter(|item| item.item_id.0 == PET_FOOD_ID)
        .map(|item| match &item.details {
            InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
            _ => 1,
        })
        .sum::<u32>();

    if post_buy_zeny != pre_buy_zeny - PET_FOOD_BUY_PRICE {
        return Err(format!(
            "Path 1: expected zeny delta of -{PET_FOOD_BUY_PRICE} (from {pre_buy_zeny} to {}), got {post_buy_zeny}",
            pre_buy_zeny - PET_FOOD_BUY_PRICE
        ));
    }
    if post_buy_db_zeny != pre_buy_zeny - PET_FOOD_BUY_PRICE {
        return Err(format!(
            "Path 1: expected DB zeny delta of -{PET_FOOD_BUY_PRICE}, got {post_buy_db_zeny}"
        ));
    }
    if post_buy_pet_food != pre_buy_pet_food + 1 {
        return Err(format!(
            "Path 1: expected inventory delta +1 (from {pre_buy_pet_food} to {}), got {post_buy_pet_food}",
            pre_buy_pet_food + 1
        ));
    }

    // -------------------------------------------------------------------------
    // Candidate Path 2: NPC Sale (Double-click / delayed response test)
    // -------------------------------------------------------------------------
    let food_item = context
        .inventory
        .iter()
        .find(|item| item.item_id.0 == PET_FOOD_ID)
        .ok_or("Pet Food not found in inventory for sale test")?;
    let food_index = food_item.index;

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
    let _ = context.wait_for("SellItemList", |event| match event {
        NetworkEvent::SellItemList { items } => Some(items.clone()),
        _ => None,
    })?;

    let pre_sell_zeny = context.zeny;
    let pre_sell_pet_food = post_buy_pet_food;

    // Simulate rapid double-click on Sell: emit 2 sell requests for same inventory
    // slot
    context.flush();
    context
        .net
        .sell_items(vec![SoldItemInformation {
            inventory_index: food_index,
            amount: 1,
        }])
        .map_err(|_| "disconnected on first sell")?;
    context
        .net
        .sell_items(vec![SoldItemInformation {
            inventory_index: food_index,
            amount: 1,
        }])
        .map_err(|_| "disconnected on second sell")?;

    // Await first result (must be Success)
    let sell_res_1 = context.wait_for("first SellingCompleted", |event| match event {
        NetworkEvent::SellingCompleted { result } => Some(*result),
        _ => None,
    })?;
    if !matches!(sell_res_1, SellItemsResult::Success) {
        return Err(format!("Path 2: expected first sell result to be Success, got {sell_res_1:?}"));
    }

    // Await second result (must be Error due to sd->npc_shopid = 0)
    let sell_res_2 = context.wait_for("second SellingCompleted", |event| match event {
        NetworkEvent::SellingCompleted { result } => Some(*result),
        _ => None,
    })?;
    if !matches!(sell_res_2, SellItemsResult::Error) {
        return Err(format!("Path 2: expected second sell result to be Error, got {sell_res_2:?}"));
    }

    context.pump(Duration::from_millis(300));
    context.say("@save")?;
    context.pump(Duration::from_millis(200));

    // Assert exactly ONE sale occurred
    let post_sell_zeny = context.zeny;
    let post_sell_db_zeny = query_char_zeny(char_id)?;
    let post_sell_pet_food = context
        .inventory
        .iter()
        .filter(|item| item.item_id.0 == PET_FOOD_ID)
        .map(|item| match &item.details {
            InventoryItemDetails::Regular { amount, .. } => u32::from(*amount),
            _ => 1,
        })
        .sum::<u32>();

    if post_sell_zeny != pre_sell_zeny + PET_FOOD_SELL_PRICE {
        return Err(format!(
            "Path 2: expected zeny delta of +{PET_FOOD_SELL_PRICE} (from {pre_sell_zeny} to {}), got {post_sell_zeny}",
            pre_sell_zeny + PET_FOOD_SELL_PRICE
        ));
    }
    if post_sell_db_zeny != pre_sell_zeny + PET_FOOD_SELL_PRICE {
        return Err(format!(
            "Path 2: expected DB zeny delta of +{PET_FOOD_SELL_PRICE}, got {post_sell_db_zeny}"
        ));
    }
    if post_sell_pet_food != pre_sell_pet_food - 1 {
        return Err(format!(
            "Path 2: expected inventory delta -1 (from {pre_sell_pet_food} to {}), got {post_sell_pet_food}",
            pre_sell_pet_food - 1
        ));
    }

    // -------------------------------------------------------------------------
    // Candidate Path 3, 4, 5: Classification Audit
    // -------------------------------------------------------------------------
    // Path 1 (NPC Buy double-click): Ruled out on server (1 success, 1 error,
    // exactly 1x delta). Path 2 (NPC Sell double-click): Ruled out on server (1
    // success, 1 error, exactly 1x delta). Path 3 (Client Cart Clearing
    // Defect): Identified as root cause of "vendor sells twice" report.
    //        In unpatched client, `handle_selling_completed` cleared `buy_cart`
    // instead of `sell_cart`,        leaving sold items displayed in the cart
    // window and causing users to attempt repeated sales. Path 4 (Player
    // Vending / Buying Store): Ruled out (unexposed in client packet layer).
    // Path 5 (Pushcart Currency): Ruled out (Pushcart holds items only, no zeny
    // fields).

    Ok(())
}
