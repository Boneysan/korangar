//! DM Loot & Rewards Generator (`docs/specs/dm-loot-generator.md`).
//!
//! Suggests level-appropriate loot from the embedded item/bestiary exports;
//! nothing reaches the server until a grant button sends an `@item` /
//! `@dmreward` chat command, which Hercules validates (`DM_RequireDM`).

use std::time::{SystemTime, UNIX_EPOCH};

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, PathExt, RustState, State, VecIndexExt};

use crate::dm::{LootDifficulty, dm_data, generate_loot};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;
use crate::state::ClientState;

const MAXIMUM_NUMBER_LENGTH: usize = 4;
const DEFAULT_PARTY_LEVEL: u16 = 40;

/// One suggested reward with the chat command that grants it.
#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct LootRow {
    pub label: String,
    pub command: String,
}

/// Internal state of the loot generator window.
#[derive(RustState, StateElement)]
pub struct LootWindowState {
    /// Party level as typed (parsed on generate; defaults to 40).
    level_text: String,
    /// Campaign arc for the server-side `@dmreward` presets.
    arc_text: String,
    /// 0 = minor, 1 = standard, 2 = major.
    difficulty: u8,
    rows: Vec<LootRow>,
}

impl Default for LootWindowState {
    fn default() -> Self {
        Self {
            level_text: DEFAULT_PARTY_LEVEL.to_string(),
            arc_text: "1".to_owned(),
            difficulty: 1,
            rows: Vec::new(),
        }
    }
}

fn selected_difficulty(index: u8) -> LootDifficulty {
    match index {
        0 => LootDifficulty::Minor,
        2 => LootDifficulty::Major,
        _ => LootDifficulty::Standard,
    }
}

pub struct LootGeneratorWindow<A> {
    window_state_path: A,
}

impl<A> LootGeneratorWindow<A> {
    pub fn new(window_state_path: A) -> Self {
        Self { window_state_path }
    }
}

/// Suggestion list: label plus a per-row grant button.
struct SuggestionList<A> {
    window_state_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> Element<ClientState> for SuggestionList<A>
where
    A: Path<ClientState, LootWindowState> + Copy + 'static,
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
            let row_count = state.get(&window_state_path.rows()).len();

            if row_count < self.elements.len() {
                self.elements.truncate(row_count);
            } else {
                for index in self.elements.len()..row_count {
                    let row_path = window_state_path.rows().index(index).manually_asserted();
                    self.elements.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: (
                            text! {
                                text: row_path.label(),
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            button! {
                                text: "Grant",
                                tooltip: "Send this row's [^000001@item^000000] command",
                                event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    let command = state.get(&row_path.command()).clone();
                                    if !command.is_empty() {
                                        queue.queue(InputEvent::SendMessage { text: command });
                                    }
                                },
                            },
                        ),
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

impl<A> CustomWindow<ClientState> for LootGeneratorWindow<A>
where
    A: Path<ClientState, LootWindowState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::DmLoot)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct PartyLevelTextBox;
        struct ArcTextBox;

        let window_state_path = self.window_state_path;

        let is_difficulty = move |index: u8| {
            ComputedSelector::new_default(move |state: &ClientState| *window_state_path.difficulty().follow_safe(state) == index)
        };
        let select_difficulty = move |index: u8| {
            move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| state.update_value(window_state_path.difficulty(), index)
        };

        let generate = move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
            let party_level: u16 = state
                .get(&window_state_path.level_text())
                .trim()
                .parse()
                .unwrap_or(DEFAULT_PARTY_LEVEL);
            let difficulty = selected_difficulty(*state.get(&window_state_path.difficulty()));
            let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|entry| entry.subsec_nanos()).unwrap_or(1) as u64;

            let rows: Vec<LootRow> = generate_loot(dm_data(), party_level, difficulty, seed)
                .into_iter()
                .map(|suggestion| {
                    let source = dm_data()
                        .monster_by_sprite(&suggestion.source)
                        .map(|monster| monster.display_name())
                        .unwrap_or_else(|| suggestion.source.clone());
                    LootRow {
                        label: format!("{}x {} ({}z, {source})", suggestion.quantity, suggestion.name, suggestion.value),
                        command: format!("@item {} {}", suggestion.item_id, suggestion.quantity),
                    }
                })
                .collect();
            state.update_value(window_state_path.rows(), rows);
        };

        let dmreward = move |suffix: &'static str| {
            move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                let arc = state.get(&window_state_path.arc_text()).trim().parse::<u8>().unwrap_or(1);
                let tier = selected_difficulty(*state.get(&window_state_path.difficulty())).label();
                let text = match suffix.is_empty() {
                    true => format!("@dmreward {arc} {tier}"),
                    false => format!("@dmreward {arc} {tier} {suffix}"),
                };
                queue.queue(InputEvent::SendMessage { text });
            }
        };

        window! {
            title: "Loot Generator (DM)",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text! {
                            text: "Party level:",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "40",
                            state: window_state_path.level_text(),
                            input_handler: DefaultHandler::<_, _, MAXIMUM_NUMBER_LENGTH>::new(window_state_path.level_text(), generate),
                            focus_id: PartyLevelTextBox,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: "Arc:",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "1",
                            state: window_state_path.arc_text(),
                            input_handler: DefaultHandler::<_, _, MAXIMUM_NUMBER_LENGTH>::new(window_state_path.arc_text(), generate),
                            focus_id: ArcTextBox,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Minor",
                            disabled: is_difficulty(0),
                            event: select_difficulty(0),
                        },
                        button! {
                            text: "Standard",
                            disabled: is_difficulty(1),
                            event: select_difficulty(1),
                        },
                        button! {
                            text: "Major",
                            disabled: is_difficulty(2),
                            event: select_difficulty(2),
                        },
                        button! {
                            text: "Generate",
                            tooltip: "Suggest level-appropriate loot from the Hercules item export",
                            event: generate,
                        },
                    ),
                },
                SuggestionList {
                    window_state_path,
                    elements: Vec::new(),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Server preview",
                            tooltip: "Preview the server-side preset [^000001@dmreward <arc> <tier> preview^000000]",
                            event: dmreward("preview"),
                        },
                        button! {
                            text: "Server reward",
                            tooltip: "Run the server-side preset [^000001@dmreward <arc> <tier>^000000]",
                            event: dmreward(""),
                        },
                    ),
                },
            ),
        }
    }
}
