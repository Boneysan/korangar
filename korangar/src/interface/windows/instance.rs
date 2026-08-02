use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::instance::{InstanceState, InstanceStatePathExt};
use crate::state::theme::InterfaceThemeType;

/// Memorial dungeon / instance information.
///
/// Opens itself when the server announces an instance and closes when the
/// server tears it down, mirroring the original client's behaviour — the
/// window is server-driven, not something the player toggles.
pub struct InstanceWindow<P> {
    instance_path: P,
}

impl<P> InstanceWindow<P> {
    pub fn new(instance_path: P) -> Self {
        Self { instance_path }
    }
}

impl<P> CustomWindow<ClientState> for InstanceWindow<P>
where
    P: Path<ClientState, InstanceState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Instance)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Instance",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: self.instance_path.display_text(),
                },
            )
        }
    }
}
