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
        Scenario::new("stat-skill-points", 6, stat_skill_points),
        Scenario::new("hotkeys", 6, hotkeys),
        Scenario::new("repair-weapon-cancel", 6, repair_weapon_cancel),
        Scenario::new("repair-weapon-success", 6, repair_weapon_success),
        Scenario::new("repair-list-empty", 6, repair_list_empty),
        Scenario::new("repair-invalid-item", 6, repair_invalid_item),
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
        if let Ok(name) = name {
            if name.contains("Pet Groomer") {
                groomer_id = Some(id);
                break;
            }
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
        NetworkEvent::SellingCompleted { result } => match result {
            SellItemsResult::Success => Some(()),
            _ => None,
        },
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
/// **Slot 37 is the last of the 38 and deliberately away from the F1–F9 row.**
/// The same character carries a hand-built hotbar for the graphical skill
/// passes (F1–F7 on the E1 Wizard), and a headless test is not entitled to
/// clobber it.
fn hotkeys(config: &Config) -> Result<(), String> {
    const TAB: HotbarTab = HotbarTab(0);
    const SLOT: HotbarSlot = HotbarSlot(37);
    const RED_POTION: u32 = 501;
    const ORANGE_POTION: u32 = 502;
    const QUANTITY: u16 = 7;

    let mut context = TestContext::connect(config)?;
    let before = read_hotkey(&mut context, TAB, SLOT)?;
    let probe = match before {
        Some((item_id, _)) if item_id == RED_POTION => ORANGE_POTION,
        _ => RED_POTION,
    };

    context
        .net
        .set_hotkey_data(TAB, SLOT, HotkeyData {
            hotkey_type: HotkeyType::Item,
            item_or_skill_id: probe,
            quantity_or_skill_level: QUANTITY,
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
        Some((item_id, quantity)) if item_id == probe && quantity == QUANTITY => Ok(()),
        Some((item_id, quantity)) => Err(format!(
            "hotkey tab {} slot {} came back as item {item_id} x{quantity} after relogin, expected {probe} x{QUANTITY} (it held \
             {before:?} before the write)",
            TAB.0, SLOT.0
        )),
        None => Err(format!(
            "hotkey tab {} slot {} was unbound after relogin — the write never reached the character save",
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
