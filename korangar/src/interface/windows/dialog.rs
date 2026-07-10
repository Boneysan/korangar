use std::cell::UnsafeCell;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::ElementStoreMut;
use korangar_interface::element::{Element, ElementBox, ErasedElement, StateElement};
use korangar_interface::layout::{Resolvers, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::EntityId;
use rust_state::{Path, RustState, State};

use super::WindowClass;
use crate::input::InputEvent;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

const MAXIMUM_DIALOG_INPUT_LENGTH: usize = 70;

/// ZST focus id for the dialog input text box.
struct DialogInputTextBox;

/// A small wrapper struct that serves two purposes:
/// - Making the elements nicer to construct by putting the [`UnsafeCell::new`]
///   and [`Box::new`] behind a function call.
/// - Storing which elements are next / choice / input widgets so we can remove
///   them when the dialog advances (stale choice buttons get GM-kicked by Hercules).
#[derive(RustState, StateElement)]
pub struct DialogElement {
    /// Stores the UI element.
    // TODO: Unfortunately this has to be an unsafe cell as of now. Ideally this can be changed
    // later.
    #[hidden_element]
    element: UnsafeCell<ElementBox<ClientState>>,
    is_next_button: bool,
    is_input_widget: bool,
    is_choice_button: bool,
}

impl DialogElement {
    /// Creates a new dialog element.
    #[inline(always)]
    fn new<E>(element: E, is_next_button: bool) -> Self
    where
        E: Element<ClientState> + 'static,
    {
        Self {
            element: UnsafeCell::new(ErasedElement::new(element)),
            is_next_button,
            is_input_widget: false,
            is_choice_button: false,
        }
    }

    #[inline(always)]
    fn new_input<E>(element: E) -> Self
    where
        E: Element<ClientState> + 'static,
    {
        Self {
            element: UnsafeCell::new(ErasedElement::new(element)),
            is_next_button: false,
            is_input_widget: true,
            is_choice_button: false,
        }
    }

    #[inline(always)]
    fn new_choice<E>(element: E) -> Self
    where
        E: Element<ClientState> + 'static,
    {
        Self {
            element: UnsafeCell::new(ErasedElement::new(element)),
            is_next_button: false,
            is_input_widget: false,
            is_choice_button: true,
        }
    }

    fn is_transient(&self) -> bool {
        self.is_next_button || self.is_input_widget || self.is_choice_button
    }
}

fn normalize_dialog_text(text: String) -> String {
    if !text.contains('<') {
        return text;
    }

    let mut output = String::with_capacity(text.len());
    let mut remaining = text.as_str();

    while let Some(tag_start) = remaining.find('<') {
        output.push_str(&remaining[..tag_start]);

        let tag_and_after = &remaining[tag_start..];
        let Some(tag_end) = tag_and_after.find('>') else {
            output.push_str(tag_and_after);
            return output;
        };

        let tag_name = tag_and_after[1..tag_end]
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();

        if tag_name == "INFO" {
            let after_tag = &tag_and_after[tag_end + 1..];
            let after_tag_uppercase = after_tag.to_ascii_uppercase();

            remaining = match after_tag_uppercase.find("</INFO>") {
                Some(info_end) => &after_tag[info_end + "</INFO>".len()..],
                None => after_tag,
            };
        } else if matches!(
            tag_name.as_str(),
            "NAVI" | "/NAVI" | "/INFO" | "URL" | "/URL" | "ITEM" | "/ITEM"
        ) {
            remaining = &tag_and_after[tag_end + 1..];
        } else {
            output.push('<');
            remaining = &tag_and_after[1..];
        }
    }

    output.push_str(remaining);
    output
}

#[cfg(test)]
mod tests {
    use super::normalize_dialog_text;

    #[test]
    fn normalize_dialog_text_strips_navigation_markup() {
        let text = "If you need more help with the map, press <NAVI>[Hun]<INFO>izlude,122,207,</INFO></NAVI> here";

        assert_eq!(
            normalize_dialog_text(text.to_owned()),
            "If you need more help with the map, press [Hun] here"
        );
    }

    #[test]
    fn normalize_dialog_text_leaves_unknown_angle_text() {
        let text = "Use < and > when explaining ranges.";

        assert_eq!(normalize_dialog_text(text.to_owned()), text);
    }
}

/// Internal state of the dialog window.
#[derive(RustState, StateElement)]
pub struct DialogWindowState {
    /// All current dialog elements.
    elements: Vec<DialogElement>,
    /// The entity id of the NPC the player is talking to.
    npc_id: EntityId,
    /// Whether or not the elements should be cleared the next time
    /// [`start`](Self::start) is called.
    clear_next: bool,
    /// Buffer for the active NPC number/string input box.
    input_text: String,
    /// True while a conversation is open (server holds `npc_id` on the player).
    /// Closing the window without telling the server leaves the player "busy".
    active: bool,
}

impl DialogWindowState {
    /// Initialize the dialog. This is important so we have the correct entity
    /// id when sending packets to the server.
    pub fn initialize(&mut self, npc_id: EntityId) -> &mut Self {
        self.npc_id = npc_id;
        self.active = true;
        self
    }

    /// Whether a conversation is in progress (window may or may not still be open).
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn npc_id(&self) -> EntityId {
        self.npc_id
    }

    /// Drop next / choice / input controls (keep history text).
    pub fn clear_transient_controls(&mut self) {
        self.elements.retain(|element| !element.is_transient());
    }

    /// Add text to the dialog.
    pub fn add_text(&mut self, text: String) {
        use korangar_interface::prelude::*;

        // New script page: clear after Next, and always drop stale choice buttons.
        // Leaving old `select()` buttons around makes Hercules GM-kick the player
        // for "Invalid menu selection ... valid range is [1..0]".
        if self.clear_next {
            self.elements.clear();
            self.clear_next = false;
        } else {
            self.clear_transient_controls();
        }

        let text = normalize_dialog_text(text);

        self.elements.push(DialogElement::new(
            text! {
                text: text,
            },
            false,
        ));
    }

    /// Add add next button to the dialog.
    ///
    /// This also sets the internal state to clear the dialog the next time
    /// [`start`](Self::start) is called.
    pub fn add_next_button(&mut self) {
        use korangar_interface::prelude::*;

        // Only one Next at a time; drop leftover choices from a prior select.
        self.clear_transient_controls();

        let npc_id = self.npc_id;

        self.elements.push(DialogElement::new(
            button! {
                text: client_state().localization().next_button_text(),
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::NextDialog { npc_id });
                },
            },
            true,
        ));

        self.clear_next = true;
    }

    /// Add a close button to the dialog.
    ///
    /// This also removes any existing "Next"-buttons.
    ///
    /// I am unsure why that's the behavior of the official client.
    pub fn add_close_button(&mut self) {
        use korangar_interface::prelude::*;

        self.clear_transient_controls();

        let npc_id = self.npc_id;

        self.elements.push(DialogElement::new(
            button! {
                text: client_state().localization().close_button_text(),
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::CloseDialog { npc_id });
                },
            },
            false,
        ));
    }

    /// Add multiple buttons, one for each choice.
    ///
    /// This also removes any existing "Next"-buttons.
    ///
    /// I am unsure why that's the behavior of the official client.
    pub fn add_choice_buttons(&mut self, choices: Vec<String>) {
        use korangar_interface::prelude::*;

        self.clear_transient_controls();

        // Empty menu must not render clickable placeholders — Hercules treats any
        // selection while `npc_menu == 0` as a hack and GM-kicks the player.
        if choices.is_empty() {
            return;
        }

        let npc_id = self.npc_id;

        choices.into_iter().enumerate().for_each(|(index, text)| {
            self.elements.push(DialogElement::new_choice(button! {
                text: text,
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::ChooseDialogOption {
                        npc_id,
                        option: index as i8 + 1,
                    });
                },
            }))
        });
    }

    /// Show a numeric input box (server `input` / `ZC_OPEN_EDITDLG`).
    ///
    /// Keeps existing dialog text; removes next/input widgets so only one
    /// input control is active.
    pub fn add_number_input(&mut self) {
        use korangar_interface::prelude::*;

        self.clear_transient_controls();
        self.input_text.clear();
        self.clear_next = false;

        let npc_id = self.npc_id;
        let input_path = client_state().dialog_window().input_text();

        let submit = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let text = state.get(&input_path);
            let value = text.trim().parse::<i32>().unwrap_or(0);
            queue.queue(InputEvent::SubmitDialogNumber { npc_id, value });
        };

        self.elements.push(DialogElement::new_input(text_box! {
            ghost_text: client_state().localization().dialog_number_input_ghost(),
            state: input_path,
            input_handler: DefaultHandler::<_, _, MAXIMUM_DIALOG_INPUT_LENGTH>::new(input_path, submit),
            focus_id: DialogInputTextBox,
            overflow_behavior: OverflowBehavior::Shrink,
        }));

        self.elements.push(DialogElement::new_input(button! {
            text: client_state().localization().dialog_input_ok_text(),
            event: submit,
        }));
    }

    /// Show a string input box (server `input` string / `ZC_OPEN_EDITDLGSTR`).
    pub fn add_string_input(&mut self) {
        use korangar_interface::prelude::*;

        self.clear_transient_controls();
        self.input_text.clear();
        self.clear_next = false;

        let npc_id = self.npc_id;
        let input_path = client_state().dialog_window().input_text();

        let submit = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let text = state.get(&input_path).clone();
            queue.queue(InputEvent::SubmitDialogString { npc_id, text });
        };

        self.elements.push(DialogElement::new_input(text_box! {
            ghost_text: client_state().localization().dialog_string_input_ghost(),
            state: input_path,
            input_handler: DefaultHandler::<_, _, MAXIMUM_DIALOG_INPUT_LENGTH>::new(input_path, submit),
            focus_id: DialogInputTextBox,
            overflow_behavior: OverflowBehavior::Shrink,
        }));

        self.elements.push(DialogElement::new_input(button! {
            text: client_state().localization().dialog_input_ok_text(),
            event: submit,
        }));
    }

    /// Drop the input widgets after a successful submit (mes text stays).
    pub fn finish_input(&mut self) {
        self.elements.retain(|element| !element.is_input_widget);
        self.input_text.clear();
    }

    /// End the dialog.
    ///
    /// This has no side effects.
    pub fn end(&mut self) {
        self.elements.clear();
        self.clear_next = false;
        self.input_text.clear();
        self.active = false;
        self.npc_id = EntityId(0);
    }
}

impl Default for DialogWindowState {
    fn default() -> Self {
        Self {
            elements: Default::default(),
            // Arguably not very clean but avoids using an Option.
            npc_id: EntityId(0),
            clear_next: false,
            input_text: String::new(),
            active: false,
        }
    }
}

/// Wrapper struct for collecting all [`DialogElement::element`]s into a single
/// element.
struct InnerElement<A> {
    dialog_elements_path: A,
}

impl<A> Element<ClientState> for InnerElement<A>
where
    A: Path<ClientState, Vec<DialogElement>>,
{
    type LayoutInfo = ();

    fn create_layout_info(&mut self, state: &State<ClientState>, mut store: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) {
        with_single_resolver(resolvers, |resolver| {
            state
                .get(&self.dialog_elements_path)
                .iter()
                .enumerate()
                .for_each(|(index, dialog_element)| {
                    // We only create this mutable reference for the lifetime of this scope, and
                    // since nothing is captured from the element this is safe.
                    let element = unsafe { &mut *dialog_element.element.get() };

                    element.create_layout_info(state, store.child_store(index as u64), resolver)
                });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: korangar_interface::element::store::ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut korangar_interface::layout::WindowLayout<'a, ClientState>,
    ) {
        state
            .get(&self.dialog_elements_path)
            .iter()
            .enumerate()
            .for_each(|(index, dialog_element)| {
                // There are no mutable references at this point in time and the immutable
                // reference will be dropped after the interface is rendered, making this safe.
                let element = unsafe { &*dialog_element.element.get() };

                element.lay_out(state, store.child_store(index as u64), &(), layout)
            });
    }
}

/// A window representing a dialog with an NPC.
pub struct DialogWindow<A> {
    /// Path to the [`DialogWindowState`].
    window_state_path: A,
}

impl<A> DialogWindow<A> {
    /// Creates a new dialog window.
    ///
    /// This does not modify the [`DialogWindowState`].
    pub fn new(window_state_path: A) -> Self {
        Self { window_state_path }
    }
}

impl<A> CustomWindow<ClientState> for DialogWindow<A>
where
    A: Path<ClientState, DialogWindowState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Dialog)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().dialog_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // Allow canceling out of long/looping NPC scripts. Closing without
            // notifying the server would leave the player "busy" (ZC_MSG 1923).
            closable: true,
            elements: (
                InnerElement {
                    dialog_elements_path: self.window_state_path.elements(),
                },
            ),
        }
    }
}
