use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{JobId, RefinableWeaponInformation};

use super::selection_list::SelectionList;
use crate::dm::reference_data::reference_data;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct WeaponRefineWindow {
    weapons: Vec<(RefinableWeaponInformation, String)>,
    job_level: usize,
    job_id: JobId,
}

impl WeaponRefineWindow {
    pub fn new(weapons: Vec<(RefinableWeaponInformation, String)>, job_level: usize, job_id: JobId) -> Self {
        Self {
            weapons,
            job_level,
            job_id,
        }
    }
}

impl CustomWindow<ClientState> for WeaponRefineWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::WeaponRefine)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let refinement = &reference_data().refinement;
        let job_level = self.job_level;
        let job_id = self.job_id;
        let entries = self.weapons.into_iter().map(|(weapon, name)| {
            let item = reference_data().item_by_id(weapon.item_id.0);
            let weapon_level = item.and_then(|item| item.weapon_level);
            let row = weapon_level.and_then(|level| refinement.weapon_levels.iter().find(|row| row.weapon_level == level));
            let target_level = weapon.refinement_level.saturating_add(1);
            let chance = row.and_then(|row| row.base_chance_percent_by_target_level.get(&target_level.to_string()).copied());
            let chance = chance.map(|base_percent| {
                let mut per_mille = i32::from(base_percent) * 10;
                if job_id.0 == 4064 {
                    // Hercules JOB_MECHANIC_T: the skill adds a flat 100/1000.
                    per_mille += i32::from(refinement.mechanic_transcendent_flat_bonus_percent) * 10;
                } else {
                    per_mille += i32::from(refinement.job_level_bonus_per_job_level_from_50_per_mille) * (job_level as i32 - 50);
                }
                (per_mille.clamp(0, 1000) as f32) / 10.0
            });
            let material = row.map(|row| row.material.as_str()).unwrap_or("unknown material");
            let chance_text = chance
                .map(|chance| format!("{chance:.1}%"))
                .unwrap_or_else(|| "chance unavailable".to_owned());
            let text = format!("{name}  +{} — {chance_text} — {material}", weapon.refinement_level);
            let tooltip = format!(
                "Attempt to refine to +{target_level}\nSuccess chance: {chance_text}\nConsumes 1 {material}\nFailure: weapon is \
                 destroyed\nInventory slot {}\nCards: {}, {}, {}, {}",
                weapon.inventory_index.0, weapon.cards[0].0, weapon.cards[1].0, weapon.cards[2].0, weapon.cards[3].0
            );
            let event = InputEvent::RefineWeapon {
                inventory_index: weapon.inventory_index,
            };
            (text, tooltip, event)
        });

        window! {
            title: "Select weapon to refine",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                text! { text: "Choose a weapon. Success chance includes your current job-level bonus. No Zeny cost; one ore is consumed. Failure destroys the weapon.", overflow_behavior: OverflowBehavior::Shrink },
                SelectionList::new(entries),
                button! {
                    text: "Cancel",
                    event: InputEvent::CancelWeaponRefine,
                },
            ),
        }
    }
}
