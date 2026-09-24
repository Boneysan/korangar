use korangar_components::item_box;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;
use rust_state::{Path, PathExt, Selector, State};

use crate::ItemSource;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::inventory::{InventoryTab, inventory_tab_matches};
use crate::state::storage::{StorageState, StorageStatePathExt};
use crate::state::theme::InterfaceThemeType;
use crate::world::ResourceMetadata;

#[derive(Clone, Copy)]
struct FilteredStorageItemPath<P, T, Q> {
    items_path: P,
    tab_path: T,
    query_path: Q,
    slot: usize,
}

impl<P, T, Q> Selector<ClientState, InventoryItem<ResourceMetadata>, false> for FilteredStorageItemPath<P, T, Q>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>> + Copy,
    T: Path<ClientState, InventoryTab>,
    Q: Path<ClientState, String>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a InventoryItem<ResourceMetadata>> {
        self.follow(state)
    }
}

impl<P, T, Q> Path<ClientState, InventoryItem<ResourceMetadata>, false> for FilteredStorageItemPath<P, T, Q>
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

/// Kafra personal storage. Opened by the map server when `openstorage` runs
/// (`SetStorage` / `StorageAmount`). Drag inventory ↔ storage via `ItemBox`
/// (`MoveItem` → `0x0364` / `0x0365`).
pub struct StorageWindow<I, S> {
    items_path: I,
    storage_path: S,
}

impl<I, S> StorageWindow<I, S> {
    pub fn new(items_path: I, storage_path: S) -> Self {
        Self { items_path, storage_path }
    }
}

impl<I, S> CustomWindow<ClientState> for StorageWindow<I, S>
where
    I: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    S: Path<ClientState, StorageState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Storage)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // Storage holds many slots; show a usable grid (scroll via window if needed).
        const STORAGE_ROWS: usize = 6;
        const STORAGE_COLUMNS: usize = 10;
        const MAXIMUM_SEARCH_LENGTH: usize = 64;
        struct StorageSearchBox;

        let capacity = self.storage_path.capacity_text();
        let search_path = self.storage_path.search_query();
        let tab_path = self.storage_path.selected_tab();
        let commit_search = |_: &State<ClientState>, _: &mut EventQueue<ClientState>| {};

        window! {
            title: "Storage",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: capacity,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: "Drag items between Inventory and Storage.",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text_box! {
                    ghost_text: "Search storage",
                    state: search_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_SEARCH_LENGTH>::new(search_path, commit_search),
                    focus_id: StorageSearchBox,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                button! { text: "All", event: InputEvent::SetStorageTab(InventoryTab::All) },
                button! { text: "Gear", event: InputEvent::SetStorageTab(InventoryTab::Gear) },
                button! { text: "Items", event: InputEvent::SetStorageTab(InventoryTab::Items) },
                button! { text: "Consumables", event: InputEvent::SetStorageTab(InventoryTab::Consumables) },
                button! { text: "Etc", event: InputEvent::SetStorageTab(InventoryTab::Etc) },
                button! { text: "Cards", event: InputEvent::SetStorageTab(InventoryTab::Cards) },
                button! { text: "Ammo", event: InputEvent::SetStorageTab(InventoryTab::Ammo) },
                std::array::from_fn::<_, STORAGE_ROWS, _>(|row| {
                    split! {
                        gaps: theme().window().gaps(),
                        children: std::array::from_fn::<_, STORAGE_COLUMNS, _>(|column| {
                            let slot = row * STORAGE_COLUMNS + column;
                            let path = FilteredStorageItemPath {
                                items_path: self.items_path,
                                tab_path,
                                query_path: search_path,
                                slot,
                            };

                            item_box! {
                                item_path: path,
                                source: ItemSource::Storage,
                                display_slot: slot,
                            }
                        }),
                    }
                }),
                button! {
                    text: "Close storage",
                    event: InputEvent::CloseStorage,
                },
            ),
        }
    }
}
