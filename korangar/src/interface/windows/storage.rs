use korangar_components::item_box;
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;
use rust_state::{Path, VecIndexExt};

use crate::ItemSource;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::storage::{StorageState, StorageStatePathExt};
use crate::state::theme::InterfaceThemeType;
use crate::world::ResourceMetadata;

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

        let capacity = self.storage_path.capacity_text();

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
                std::array::from_fn::<_, STORAGE_ROWS, _>(|row| {
                    split! {
                        gaps: theme().window().gaps(),
                        children: std::array::from_fn::<_, STORAGE_COLUMNS, _>(|column| {
                            let slot = row * STORAGE_COLUMNS + column;
                            let path = self.items_path.index(slot);

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
