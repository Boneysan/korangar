use std::cell::UnsafeCell;

use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector};

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{CraftingOddsInput, Player, best_anvil, crafting_odds_text, is_baby_class};

/// The F30 body text, rebuilt from the player, the skill tree and the bag
/// each time it is drawn, so a stat point, a skill level or an anvil picked
/// up shows at once. A dedicated selector, like the stats window's, because a
/// `ComputedSelector` with a `String` output is ambiguous to `text!`.
struct CraftingOddsSelector<A> {
    player_path: A,
    text: UnsafeCell<String>,
}

impl<A> Selector<ClientState, String> for CraftingOddsSelector<A>
where
    A: Path<ClientState, Player>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let player = self.player_path.follow_safe(state);
        let job_id = player.get_common().job_id.0;
        let input = CraftingOddsInput {
            job_level: player.job_level as i32,
            int_: player.intelligence + player.bonus_intelligence,
            dex: player.dexterity + player.bonus_dexterity,
            luk: player.luck + player.bonus_luck,
            skills: client_state().skill_tree().follow_safe(state).learned_levels().collect(),
            anvil: best_anvil(
                client_state()
                    .inventory()
                    .follow_safe(state)
                    .items()
                    .iter()
                    .map(|item| item.item_id.0),
            ),
            is_baby: is_baby_class(job_id),
        };
        unsafe {
            *self.text.get() = crafting_odds_text(&input);
            Some(self.text.as_ref_unchecked())
        }
    }
}

/// F30: the success chance of the crafts this character can attempt, from
/// the formulas in `world/crafting.rs`. Read-only; nothing is sent.
pub struct CraftingOddsWindow<A> {
    player_path: A,
}

impl<A> CraftingOddsWindow<A> {
    pub fn new(player_path: A) -> Self {
        Self { player_path }
    }
}

impl<A> CustomWindow<ClientState> for CraftingOddsWindow<A>
where
    A: Path<ClientState, Player> + Copy,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CraftingOdds)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Crafting Odds",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: CraftingOddsSelector {
                        player_path: self.player_path,
                        text: UnsafeCell::default(),
                    },
                    overflow_behavior: OverflowBehavior::Shrink,
                },
            ),
        }
    }
}
