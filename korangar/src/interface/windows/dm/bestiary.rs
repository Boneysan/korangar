//! Bestiary Journal (`docs/specs/bestiary-journal.md`).
//!
//! Searchable monster manual backed by the embedded Hercules bestiary
//! export. Entries unlock when the party defeats the monster (tracked in
//! [`crate::dm::DmCampaignState`]); the DM can reveal everything with the
//! toggle. The Spawn button emits an ordinary `@monster` command that the
//! server still permission-checks.

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, RustState, State, VecIndexExt};

use super::TextLines;
use crate::dm::{BestiaryMonster, DmCampaignStatePathExt, dm_data};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

const MAXIMUM_SEARCH_LENGTH: usize = 32;
const MAXIMUM_RESULTS: usize = 25;

/// One row of the search-result list.
#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct BestiaryRow {
    pub label: String,
    pub mob_id: u32,
}

/// Internal state of the bestiary journal window.
#[derive(Default, RustState, StateElement)]
pub struct BestiaryWindowState {
    search: String,
    results: Vec<BestiaryRow>,
    detail: Vec<String>,
    /// Sprite name of the selected monster (empty = none selected).
    selected_sprite: String,
    /// DM toggle: show full statistics for locked entries too.
    reveal_all: bool,
}

pub struct BestiaryWindow<A> {
    window_state_path: A,
}

impl<A> BestiaryWindow<A> {
    pub fn new(window_state_path: A) -> Self {
        Self { window_state_path }
    }
}

fn is_unlocked(state: &State<ClientState>, mob_id: u32) -> bool {
    state.get(&client_state().dm_campaign().bestiary_unlocked()).contains(&mob_id)
}

fn row_label(monster: &BestiaryMonster, unlocked: bool) -> String {
    match unlocked {
        true => format!("{} (Lv {})", monster.display_name(), monster.lv),
        false => format!("??? (Lv {})", monster.lv),
    }
}

/// Full detail lines for an unlocked (or revealed) entry.
fn detail_lines(monster: &BestiaryMonster) -> Vec<String> {
    let data = dm_data();
    let mut lines = vec![
        format!("{}  —  Lv {}", monster.display_name(), monster.lv),
        format!("HP {}   SP {}", monster.hp, monster.sp),
        format!(
            "Atk {}-{}   Def {}   Mdef {}   Range {}",
            monster.attack[0], monster.attack[1], monster.def, monster.mdef, monster.attack_range
        ),
        format!("Phys DPS {:.0}   Magic DPS {:.0}", monster.phys_dps, monster.magic_dps),
        format!("XP {}   Job XP {}", monster.exp, monster.job_exp),
    ];
    if let (Some(element), Some(race)) = (&monster.element, &monster.race) {
        lines.push(format!("{element}   {race}"));
    }
    if monster.has_mvp_drops {
        lines.push(format!("MVP — bonus XP {}", monster.mvp_exp));
    }

    let drops = data.drops_for_sprite(&monster.sprite_name);
    if !drops.is_empty() {
        lines.push("Drops:".to_owned());
        for item in drops.iter().take(6) {
            let rate = item
                .drops_from
                .iter()
                .find(|(sprite, _)| sprite == &monster.sprite_name)
                .map(|(_, rate)| *rate as f32 / 100.0)
                .unwrap_or_default();
            lines.push(format!("  {} ({rate:.2}%)", item.display_name()));
        }
    }
    let cards: Vec<String> = data
        .cards
        .iter()
        .filter(|card| card.drops_from.iter().any(|(sprite, _)| sprite == &monster.sprite_name))
        .map(|card| card.display_name())
        .collect();
    if !cards.is_empty() {
        lines.push(format!("Cards: {}", cards.join(", ")));
    }
    lines
}

/// Result list: one button per row, selecting the monster into the detail
/// pane (friend-list rebuild pattern).
struct ResultList<A> {
    window_state_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> Element<ClientState> for ResultList<A>
where
    A: Path<ClientState, BestiaryWindowState> + Copy + 'static,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;

            let window_state_path = self.window_state_path;
            let result_count = state.get(&window_state_path.results()).len();

            if result_count < self.elements.len() {
                self.elements.truncate(result_count);
            } else {
                for index in self.elements.len()..result_count {
                    let row_path = window_state_path.results().index(index).manually_asserted();
                    self.elements.push(ErasedElement::new(button! {
                        text: row_path.label(),
                        event: move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                            let mob_id = *state.get(&row_path.mob_id());
                            let Some(monster) = dm_data().monster_by_id(mob_id) else { return };

                            let unlocked = is_unlocked(state, mob_id) || *state.get(&window_state_path.reveal_all());
                            let lines = match unlocked {
                                true => detail_lines(monster),
                                false => vec![
                                    format!("??? (Lv {})", monster.lv),
                                    "Defeat one to record it in the journal.".to_owned(),
                                ],
                            };
                            state.update_value(window_state_path.detail(), lines);
                            state.update_value(window_state_path.selected_sprite(), monster.sprite_name.clone());
                        },
                    }));
                }
            }

            self.elements.iter_mut().enumerate().for_each(|(index, element)| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}

impl<A> CustomWindow<ClientState> for BestiaryWindow<A>
where
    A: Path<ClientState, BestiaryWindowState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Bestiary)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct BestiarySearchBox;

        let window_state_path = self.window_state_path;

        let run_search = move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
            let query = state.get(&window_state_path.search()).clone();
            let reveal_all = *state.get(&window_state_path.reveal_all());
            let rows: Vec<BestiaryRow> = dm_data()
                .search_monsters(&query, MAXIMUM_RESULTS)
                .into_iter()
                .map(|monster| BestiaryRow {
                    label: row_label(monster, reveal_all || is_unlocked(state, monster.id)),
                    mob_id: monster.id,
                })
                .collect();
            state.update_value(window_state_path.results(), rows);
        };

        let toggle_reveal = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            state.update_value_with(window_state_path.reveal_all(), |reveal| *reveal = !*reveal);
            run_search(state, queue);
        };

        let spawn_selected = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let sprite_name = state.get(&window_state_path.selected_sprite()).clone();
            if !sprite_name.is_empty() {
                queue.queue(InputEvent::SendMessage {
                    text: format!("@monster {sprite_name}"),
                });
            }
        };

        window! {
            title: "Bestiary Journal",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text_box! {
                    ghost_text: "Search monsters…",
                    state: window_state_path.search(),
                    input_handler: DefaultHandler::<_, _, MAXIMUM_SEARCH_LENGTH>::new(window_state_path.search(), run_search),
                    focus_id: BestiarySearchBox,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Search",
                            event: run_search,
                        },
                        button! {
                            text: "Reveal all (DM)",
                            tooltip: "Toggle showing full statistics for undiscovered entries",
                            event: toggle_reveal,
                        },
                        button! {
                            text: "Spawn (DM)",
                            tooltip: "Spawn the selected monster with [^000001@monster^000000]",
                            event: spawn_selected,
                        },
                    ),
                },
                ResultList {
                    window_state_path,
                    elements: Vec::new(),
                },
                TextLines::new(window_state_path.detail()),
            ),
        }
    }
}
