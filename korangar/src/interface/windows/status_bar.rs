use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::status_effects::StatusEffectsPathExt;
use crate::state::theme::InterfaceThemeType;

/// MVP status effect bar window. Binds to the cached display_text in
/// StatusEffects.
pub struct StatusBarWindow<P> {
    status_effects_path: P,
}

impl<P> StatusBarWindow<P> {
    pub fn new(status_effects_path: P) -> Self {
        Self { status_effects_path }
    }
}

impl<P> CustomWindow<ClientState> for StatusBarWindow<P>
where
    P: Path<ClientState, crate::state::status_effects::StatusEffects>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::StatusBar)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // The path to the display string field.
        // Because RustState generates field accessors, we can do .display_text() on the
        // effects path.
        let text_path = self.status_effects_path.display_text();

        window! {
            title: "Status",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                text! {
                    text: text_path,
                },
            )
        }
    }
}
