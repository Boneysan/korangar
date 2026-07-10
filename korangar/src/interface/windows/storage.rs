use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::storage::{StorageState, StorageStatePathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::ClientState;

pub struct StorageWindow<P> {
    storage_path: P,
}

impl<P> StorageWindow<P> {
    pub fn new(storage_path: P) -> Self {
        Self { storage_path }
    }
}

impl<P> CustomWindow<ClientState> for StorageWindow<P>
where
    P: Path<ClientState, StorageState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Storage)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let capacity = self.storage_path.capacity_text();
        let item_count_hint = "Drag inventory items here is not wired yet — use:\n  /store <inv_index> [amount]\n  /retrieve <storage_index> [amount]";

        window! {
            title: "Storage",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! { text: capacity },
                text! { text: item_count_hint },
                button! {
                    text: "Close storage",
                    event: InputEvent::CloseStorage,
                },
            )
        }
    }
}
