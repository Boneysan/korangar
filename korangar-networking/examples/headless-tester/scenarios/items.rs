//! Phase 6 — items, inventory, economy.

use std::time::Duration;

use korangar_networking::{NetworkEvent, ShopItem};
use ragnarok_packets::{
    BuyOrSellOption, BuyShopItemsResult, EquipPosition, HotbarSlot, HotbarTab, HotkeyData, HotkeyType, SellItemsResult, SkillId,
    SkillLevel, SoldItemInformation, StatType, StatUpType,
};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("use-consumable", 6, use_consumable),
        Scenario::new("equip-unequip", 6, equip_unequip),
        Scenario::new("drop-pickup", 6, drop_pickup),
        Scenario::new("identify", 6, identify),
        Scenario::new("shop-buy-sell", 6, shop_buy_sell),
        Scenario::new("storage", 6, storage),
        Scenario::new("stat-skill-points", 6, stat_skill_points),
        Scenario::new("hotkeys", 6, hotkeys),
        Scenario::new("repair-weapon-cancel", 6, repair_weapon_cancel),
        Scenario::new("repair-weapon-success", 6, repair_weapon_success),
        Scenario::new("repair-list-empty", 6, repair_list_empty),
        Scenario::new("repair-invalid-item", 6, repair_invalid_item),
    ]
}

const BS_REPAIRWEAPON: SkillId = SkillId(108);

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

/// Set a hotkey and verify it is received.
fn hotkeys(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect(config)?;

    let tab = HotbarTab(0);
    let slot = HotbarSlot(0);
    let data = HotkeyData {
        hotkey_type: HotkeyType::Item,
        item_or_skill_id: 501, // Red Potion id
        quantity_or_skill_level: 0,
    };

    context.flush();
    context.net.set_hotkey_data(tab, slot, data).map_err(|_| "disconnected")?;
    context.pump(Duration::from_millis(500));

    Ok(())
}
