//! One unusable-item presentation for inventory, vendor, trade, storage, floor.

use super::equipment_eligibility::{EligibilityTable, EquipDenial, Wearer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnusablePresentation {
    pub mute_icon: bool,
    pub blocked_marker: bool,
    pub disable_equip: bool,
    pub denial: Option<EquipDenial>,
}

impl UnusablePresentation {
    pub fn for_surface(table: &EligibilityTable, item_id: u32, wearer: Wearer, can_equip_action: bool) -> Self {
        let denial = table.denial(item_id, wearer);
        let unusable = denial.is_some();
        Self {
            mute_icon: unusable,
            blocked_marker: unusable,
            disable_equip: unusable && can_equip_action,
            denial,
        }
    }

    pub fn for_item(item_id: u32, wearer: Wearer, can_equip_action: bool) -> Self {
        Self::for_surface(EligibilityTable::get(), item_id, wearer, can_equip_action)
    }

    pub fn reason_text(&self) -> Option<&'static str> {
        self.denial.map(|d| d.tooltip())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::library::equipment_eligibility::{Sex, bundled_table};

    fn mage() -> Wearer {
        Wearer {
            job_id: 2,
            base_level: 10,
            sex: Sex::Female,
            slot: "EQP_WEAPON",
        }
    }

    #[test]
    fn same_reason_on_every_surface() {
        let table = bundled_table();
        let inventory = UnusablePresentation::for_surface(&table, 1101, mage(), true);
        let vendor = UnusablePresentation::for_surface(&table, 1101, mage(), false);
        let trade = UnusablePresentation::for_surface(&table, 1101, mage(), false);
        let storage = UnusablePresentation::for_surface(&table, 1101, mage(), false);
        let floor = UnusablePresentation::for_surface(&table, 1101, mage(), false);
        let reason = inventory.reason_text();
        assert_eq!(reason, Some("Cannot equip: job"));
        for surface in [vendor, trade, storage, floor] {
            assert_eq!(surface.reason_text(), reason);
            assert!(surface.mute_icon);
            assert!(surface.blocked_marker);
            assert!(!surface.disable_equip);
        }
        assert!(inventory.disable_equip);
    }
}
