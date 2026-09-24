use std::cell::{Cell, UnsafeCell};

use korangar_components::item_box;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;
use rust_state::{Path, PathExt, Selector, State};

use crate::ItemSource;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::inventory::{InventoryPathExt, InventoryTab, inventory_tab_matches};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{Player, ResourceMetadata};

#[derive(Clone, Copy)]
struct FilteredItemPath<P, T, Q> {
    items_path: P,
    tab_path: T,
    query_path: Q,
    slot: usize,
}

impl<P, T, Q> Selector<ClientState, InventoryItem<ResourceMetadata>, false> for FilteredItemPath<P, T, Q>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>> + Copy,
    T: Path<ClientState, InventoryTab>,
    Q: Path<ClientState, String>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a InventoryItem<ResourceMetadata>> {
        self.follow(state)
    }
}

impl<P, T, Q> Path<ClientState, InventoryItem<ResourceMetadata>, false> for FilteredItemPath<P, T, Q>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    T: Path<ClientState, InventoryTab>,
    Q: Path<ClientState, String>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a InventoryItem<ResourceMetadata>> {
        let tab = *self.tab_path.follow_safe(state);
        let query = self.query_path.follow_safe(state).to_lowercase();
        self.items_path
            .follow_safe(state)
            .iter()
            .filter(|item| inventory_tab_matches(item, tab) && item.metadata.name.to_lowercase().contains(&query))
            .nth(self.slot)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut InventoryItem<ResourceMetadata>> {
        let tab = *self.tab_path.follow_safe(state);
        let query = self.query_path.follow_safe(state).to_lowercase();
        self.items_path
            .follow_mut_safe(state)
            .iter_mut()
            .filter(|item| inventory_tab_matches(item, tab) && item.metadata.name.to_lowercase().contains(&query))
            .nth(self.slot)
    }
}

/// Formats current/max weight for the inventory footer.
///
/// RO stores weight in 0.1 units; the official client divides by 10 for
/// display. Soft overweight (≥ critical %, usually 50%) is yellow; hard
/// overweight (≥ 90%) is red.
struct WeightTextSelector<A> {
    player_path: A,
    last_key: Cell<Option<(u32, u32, u32)>>,
    text: UnsafeCell<String>,
}

impl<A> WeightTextSelector<A> {
    fn new(player_path: A) -> Self {
        Self {
            player_path,
            last_key: Cell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A> Selector<ClientState, String> for WeightTextSelector<A>
where
    A: Path<ClientState, Player>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let player = self.player_path.follow_safe(state);
        let weight = player.weight;
        let maximum_weight = player.maximum_weight;
        let critical = player.critical_weight_percent;

        unsafe {
            let last = self.last_key.get();
            if last != Some((weight, maximum_weight, critical)) {
                // Display units match the official client (raw / 10).
                let display = format!("{}/{}", weight / 10, maximum_weight / 10);
                *self.text.get() = if player.is_hard_overweight() {
                    // Red
                    format!("^FF5050{display}^000000")
                } else if player.is_overweight() {
                    // Yellow / amber
                    format!("^FFC832{display}^000000")
                } else {
                    display
                };
                self.last_key.set(Some((weight, maximum_weight, critical)));
            }
        }

        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

pub struct InventoryWindow<P, A> {
    items_path: P,
    player_path: A,
}

impl<P, A> InventoryWindow<P, A> {
    pub fn new(items_path: P, player_path: A) -> Self {
        Self { items_path, player_path }
    }
}

impl<P, A> CustomWindow<ClientState> for InventoryWindow<P, A>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>> + Copy,
    A: Path<ClientState, Player>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Inventory)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // TODO: Probably this should be more dynamic
        const INVENTORY_ROWS: usize = 4;
        const INVENTORY_COLUMNS: usize = 10;
        const MAXIMUM_SEARCH_LENGTH: usize = 64;
        struct InventorySearchBox;
        let tab_path = client_state().inventory().selected_tab();
        let query_path = client_state().inventory().search_query();
        let commit_search = |_: &State<ClientState>, _: &mut EventQueue<ClientState>| {};

        window! {
            title: client_state().localization().inventory_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text_box! {
                    ghost_text: "Search inventory",
                    state: query_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_SEARCH_LENGTH>::new(query_path, commit_search),
                    focus_id: InventorySearchBox,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                std::array::from_fn::<_, INVENTORY_ROWS, _>(|row| {
                    split! {
                        gaps: theme().window().gaps(),
                        children: std::array::from_fn::<_, INVENTORY_COLUMNS, _>(|column| {
                            let slot = row * INVENTORY_COLUMNS + column;
                            let path = FilteredItemPath {
                                items_path: self.items_path,
                                tab_path,
                                query_path,
                                slot,
                            };

                            item_box! {
                                item_path: path,
                                source: ItemSource::Inventory,
                                display_slot: slot,
                            }
                        }),
                    }
                }),
                button! {
                    text: "All",
                    event: InputEvent::SetInventoryTab(InventoryTab::All),
                },
                button! {
                    text: "Equipped",
                    event: InputEvent::SetInventoryTab(InventoryTab::Equipped),
                },
                button! {
                    text: "Gear",
                    event: InputEvent::SetInventoryTab(InventoryTab::Gear),
                },
                button! {
                    text: "Items",
                    event: InputEvent::SetInventoryTab(InventoryTab::Items),
                },
                button! {
                    text: "Sort",
                    event: InputEvent::SortInventory,
                },
                split! {
                    children: (
                        text! {
                            text: client_state().localization().weight_text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: WeightTextSelector::new(self.player_path),
                            horizontal_alignment: HorizontalAlignment::Right { offset: 5.0, border: 5.0 },
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
            ),
        }
    }
}
