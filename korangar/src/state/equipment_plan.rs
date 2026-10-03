//! What to do to put a saved equipment set on (GDD F24).
//!
//! Kept free of networking and UI so the rule is testable: for each item the
//! set names, equip a copy that is not already worn, count one that is, and
//! report one that is missing. Copies are counted, so a set with two identical
//! rings needs two rings, not one.

use std::collections::HashMap;

use ragnarok_packets::{EquipPosition, InventoryIndex};

/// One equippable item in the inventory, as the plan sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanItem {
    pub index: InventoryIndex,
    pub item_id: u32,
    pub worn: bool,
    /// Where it would be equipped; used only when it is not worn.
    pub equip_position: EquipPosition,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EquipPlan {
    /// One request per copy that must be put on, never the same copy twice.
    pub to_equip: Vec<(InventoryIndex, EquipPosition)>,
    /// Set entries already satisfied by a worn copy.
    pub already_worn: usize,
    /// Set entries with no copy left to equip, one id per missing copy.
    pub missing: Vec<u32>,
}

/// `set_item_ids` is the saved set in order; an id may repeat for a pair.
pub fn plan_equipment_set(set_item_ids: &[u32], inventory: &[PlanItem]) -> EquipPlan {
    let mut plan = EquipPlan::default();
    // Copies of each id that are worn, and the spare ones (not worn) in inventory
    // order.
    let mut worn_copies: HashMap<u32, usize> = HashMap::new();
    let mut spare: HashMap<u32, Vec<PlanItem>> = HashMap::new();
    for item in inventory {
        if item.worn {
            *worn_copies.entry(item.item_id).or_default() += 1;
        } else {
            spare.entry(item.item_id).or_default().push(*item);
        }
    }

    for &item_id in set_item_ids {
        // A worn copy satisfies one entry; the next entry for the same id needs
        // another.
        if let Some(remaining) = worn_copies.get_mut(&item_id)
            && *remaining > 0
        {
            *remaining -= 1;
            plan.already_worn += 1;
            continue;
        }
        match spare.get_mut(&item_id).filter(|copies| !copies.is_empty()) {
            Some(copies) => {
                let copy = copies.remove(0);
                plan.to_equip.push((copy.index, copy.equip_position));
            }
            None => plan.missing.push(item_id),
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(index: u16, item_id: u32, worn: bool) -> PlanItem {
        PlanItem {
            index: InventoryIndex(index),
            item_id,
            worn,
            equip_position: EquipPosition::NONE,
        }
    }

    #[test]
    fn an_unworn_item_in_the_set_is_equipped() {
        let plan = plan_equipment_set(&[1101], &[item(2, 1101, false)]);
        assert_eq!(plan.to_equip.len(), 1);
        assert_eq!(plan.to_equip[0].0, InventoryIndex(2));
        assert_eq!((plan.already_worn, plan.missing.len()), (0, 0));
    }

    #[test]
    fn a_worn_item_is_counted_and_not_equipped_again() {
        let plan = plan_equipment_set(&[1101], &[item(2, 1101, true)]);
        assert!(plan.to_equip.is_empty());
        assert_eq!(plan.already_worn, 1);
    }

    #[test]
    fn an_item_the_character_does_not_have_is_reported_missing() {
        let plan = plan_equipment_set(&[1101, 2201], &[item(2, 1101, false)]);
        assert_eq!(plan.to_equip.len(), 1);
        assert_eq!(plan.missing, [2201]);
    }

    /// The reason this module exists: a pair of identical rings in the set.
    /// With two copies in the pack, both are put on, each exactly once.
    #[test]
    fn two_identical_entries_equip_two_different_copies() {
        let plan = plan_equipment_set(&[2601, 2601], &[item(2, 2601, false), item(3, 2601, false)]);
        let indices: Vec<_> = plan.to_equip.iter().map(|(index, _)| *index).collect();
        assert_eq!(indices, [InventoryIndex(2), InventoryIndex(3)]);
    }

    /// One ring already on, one in the pack: the second entry still needs
    /// equipping.
    #[test]
    fn a_worn_copy_satisfies_only_one_of_two_identical_entries() {
        let plan = plan_equipment_set(&[2601, 2601], &[item(2, 2601, true), item(3, 2601, false)]);
        assert_eq!(plan.already_worn, 1);
        assert_eq!(plan.to_equip.iter().map(|(index, _)| *index).collect::<Vec<_>>(), [
            InventoryIndex(3)
        ]);
        assert!(plan.missing.is_empty());
    }

    /// Only one ring owned but the set wants two: one is equipped, one is
    /// missing.
    #[test]
    fn a_second_copy_that_does_not_exist_is_reported_missing_not_equipped_twice() {
        let plan = plan_equipment_set(&[2601, 2601], &[item(2, 2601, false)]);
        assert_eq!(plan.to_equip.len(), 1, "the one copy is equipped once");
        assert_eq!(plan.missing, [2601]);
    }

    #[test]
    fn a_fully_worn_set_needs_nothing() {
        let plan = plan_equipment_set(&[1101, 2601, 2601], &[
            item(2, 1101, true),
            item(3, 2601, true),
            item(4, 2601, true),
        ]);
        assert!(plan.to_equip.is_empty() && plan.missing.is_empty());
        assert_eq!(plan.already_worn, 3);
    }

    #[test]
    fn an_empty_set_does_nothing() {
        assert_eq!(plan_equipment_set(&[], &[item(2, 1101, false)]), EquipPlan::default());
    }
}
