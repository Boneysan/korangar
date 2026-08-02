use korangar_interface::application::Size;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, StateElement};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::MessageColor;
use rust_state::{Path, PathExt, RustState, State};

use super::WindowClass;
use crate::graphics::Color;
use crate::input::InputEvent;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{ChatThemePathExt, InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ChatMessage, ClientState, ClientStatePathExt, client_state, client_theme};

const MAXIMUM_CHAT_MESSAGE_LENGTH: usize = 80;
/// Ragnarok character names cap at 24.
const MAXIMUM_CHARACTER_NAME_LENGTH: usize = 24;

/// ZST for getting the focus id of the chat text box. This is only needed to
/// focus the chat when pressing enter.
pub struct ChatTextBox;

/// ZST for the whisper-target field's focus id.
pub struct WhisperTargetTextBox;

struct ChatLayoutInfo {
    area: Area,
    // TODO: Don't allocate this every frame.
    message_heights: Vec<f32>,
}

struct ChatElement<A> {
    chat_messages_path: A,
}

impl<A> ChatElement<A> {
    fn new(chat_messages_path: A) -> Self {
        Self { chat_messages_path }
    }
}

impl<A> Element<ClientState> for ChatElement<A>
where
    A: Path<ClientState, Vec<ChatMessage>>,
{
    type LayoutInfo = ChatLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let chat_messages = state.get(&self.chat_messages_path);
            // TODO: Theme this.
            let message_spacing = 5.0;

            let mut total_height = 0.0;
            let message_heights = chat_messages
                .iter()
                .map(|chat_message| {
                    let color = match chat_message.color {
                        MessageColor::Rgb { red, green, blue } => Color::rgb_u8(red, green, blue),
                        // TODO: Make the color right.
                        MessageColor::Broadcast => Color::monochrome_u8(255),
                        // TODO: Make the color right.
                        MessageColor::Server => Color::monochrome_u8(255),
                        // TODO: Make the color right.
                        MessageColor::Error => Color::monochrome_u8(255),
                        // TODO: Make the color right.
                        MessageColor::Information => Color::monochrome_u8(255),
                    };

                    let (size, _) = resolver.get_text_dimensions(
                        &chat_message.text,
                        color,
                        Color::rgb_u8(255, 160, 60),
                        // TODO: Theme this.
                        FontSize(14.0),
                        HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                        OverflowBehavior::LineBreak,
                    );

                    if total_height != 0.0 {
                        total_height += message_spacing;
                    }

                    total_height += size.height();

                    size.height()
                })
                .collect();

            let area = resolver.with_height(total_height);

            Self::LayoutInfo { area, message_heights }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let chat_messages = state.get(&self.chat_messages_path);
        // TODO: Theme this.
        let message_spacing = 5.0;

        let mut offset = 0.0;
        chat_messages
            .iter()
            .zip(layout_info.message_heights.iter())
            .for_each(|(chat_message, message_height)| {
                let color = match chat_message.color {
                    MessageColor::Rgb { red, green, blue } => Color::rgb_u8(red, green, blue),
                    // TODO: Make the color right.
                    MessageColor::Broadcast => Color::monochrome_u8(255),
                    // TODO: Make the color right.
                    MessageColor::Server => Color::monochrome_u8(255),
                    // TODO: Make the color right.
                    MessageColor::Error => Color::monochrome_u8(255),
                    // TODO: Make the color right.
                    MessageColor::Information => Color::monochrome_u8(255),
                };

                if offset != 0.0 {
                    offset += message_spacing;
                }

                let text_area = Area {
                    left: layout_info.area.left,
                    top: layout_info.area.top + offset,
                    width: layout_info.area.width,
                    height: *message_height,
                };

                layout.add_text(
                    text_area,
                    &chat_message.text,
                    // TODO: Theme this.
                    FontSize(14.0),
                    color,
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::LineBreak,
                );

                offset += message_height;
            });
    }
}

/// Which channel typed text goes to. Index-based rather than an enum, matching
/// `CommandsWindowState::selected_tab`.
type ChannelIndex = u8;

const CHANNEL_PUBLIC: ChannelIndex = 0;
const CHANNEL_PARTY: ChannelIndex = 1;
const CHANNEL_WHISPER: ChannelIndex = 2;

/// Internal state of the chat window.
#[derive(Default, RustState, StateElement)]
pub struct ChatWindowState {
    current_text: String,
    /// Channel typed text is routed to. `CHANNEL_PUBLIC` by default, which is
    /// the pre-existing behaviour.
    channel: ChannelIndex,
    /// Who `CHANNEL_WHISPER` talks to.
    whisper_target: String,
}

impl ChatWindowState {
    /// Aim the chat at a character. Used by the Whisper buttons in the friend
    /// list and party roster, so a whisper does not require typing `/w <name>`.
    ///
    /// Does not focus the text box: focus is claimed in the input pass
    /// (`interface_frame.focus_element(ChatTextBox)`) and is not reachable from
    /// an event handler, so the player still presses Enter to start typing.
    pub fn start_whisper(&mut self, character_name: String) {
        self.channel = CHANNEL_WHISPER;
        self.whisper_target = character_name;
    }
}

pub struct ChatWindow<A, B> {
    chat_window_state: A,
    chat_messages_path: B,
}

impl<A, B> ChatWindow<A, B> {
    pub fn new(chat_window_state: A, chat_messages_path: B) -> Self {
        Self {
            chat_window_state,
            chat_messages_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for ChatWindow<A, B>
where
    A: Path<ClientState, ChatWindowState>,
    B: Path<ClientState, Vec<ChatMessage>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Chat)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let current_text_path = self.chat_window_state.current_text();
        let channel_path = self.chat_window_state.channel();
        let whisper_target_path = self.chat_window_state.whisper_target();

        let send_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let text = state.get(&current_text_path);

            if text.is_empty() {
                return;
            }

            // Anything the player typed as a command wins over the channel --
            // otherwise `/party leave` or `@heal` would be unusable while the
            // channel is set to Party, silently becoming party chat instead.
            let outgoing = match text.starts_with('/') || text.starts_with('@') {
                true => text.clone(),
                false => {
                    let target = state.get(&whisper_target_path).trim().to_owned();
                    match *state.get(&channel_path) {
                        CHANNEL_PARTY => format!("/p {text}"),
                        // With no target this becomes `/w  <text>`, which the
                        // command handler answers with a usage hint. That is
                        // deliberate: a whisper must never fall back to public
                        // chat, which would leak it to everyone.
                        CHANNEL_WHISPER => format!("/w {target} {text}"),
                        _ => text.clone(),
                    }
                }
            };

            // Clear the text box.
            state.update_value_with(current_text_path, |current_text| current_text.clear());
            queue.queue(InputEvent::SendMessage { text: outgoing });
            queue.queue(Event::Unfocus);
        };

        // The active channel's button is disabled as the selected indicator,
        // the same convention the DM command panel uses for its tabs.
        let is_channel =
            move |index: ChannelIndex| ComputedSelector::new_default(move |state: &ClientState| *channel_path.follow_safe(state) == index);
        let select_channel = move |index: ChannelIndex| {
            move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| state.update_value(channel_path, index)
        };

        window! {
            title: client_state().localization().chat_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            background_color: client_theme().chat().window_color(),
            resizable: true,
            border: 3.0,
            gaps: 2.0,
            title_gap: 0.0,
            minimum_height: 150.0,
            maximum_height: 800.0,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Say",
                            tooltip: "Send to everyone nearby",
                            disabled: is_channel(CHANNEL_PUBLIC),
                            event: select_channel(CHANNEL_PUBLIC),
                        },
                        button! {
                            text: "Party",
                            tooltip: "Send to your party [^000001/p^000000]",
                            disabled: is_channel(CHANNEL_PARTY),
                            event: select_channel(CHANNEL_PARTY),
                        },
                        button! {
                            text: "Whisper",
                            tooltip: "Send privately to one character [^000001/w <name>^000000]",
                            disabled: is_channel(CHANNEL_WHISPER),
                            event: select_channel(CHANNEL_WHISPER),
                        },
                    ),
                },
                either! {
                    selector: is_channel(CHANNEL_WHISPER),
                    on_true: text_box! {
                        ghost_text: "Whisper to…",
                        state: whisper_target_path,
                        input_handler: DefaultHandler::<_, _, MAXIMUM_CHARACTER_NAME_LENGTH>::new(whisper_target_path, |_: &State<ClientState>, _: &mut EventQueue<ClientState>| {}),
                        background_color: client_theme().chat().text_box_background_color(),
                        focused_background_color: Color::rgba(0.0, 0.0, 0.0, 0.8),
                        focus_id: WhisperTargetTextBox,
                    },
                    on_false: fragment! {
                        children: (),
                    },
                },
                text_box! {
                    ghost_text: client_state().localization().chat_text_box_message(),
                    state: current_text_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_CHAT_MESSAGE_LENGTH>::new(current_text_path, send_action),
                    background_color: client_theme().chat().text_box_background_color(),
                    focused_background_color: Color::rgba(0.0, 0.0, 0.0, 0.8),
                    focus_id: ChatTextBox,
                },
                scroll_view! {
                    follow: true,
                    children: ChatElement::new(self.chat_messages_path),
                },
            ),
        }
    }
}
