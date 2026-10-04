use korangar_interface::application::Size;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, StateElement};
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::MessageColor;
use rust_state::{Path, PathExt, RustState, State};

use super::WindowClass;
use crate::graphics::Color;
use crate::input::InputEvent;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::settings::GameSettingsPathExt;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{ChatThemePathExt, InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ChatMessage, ClientState, ClientStatePathExt, client_state, client_theme};
use crate::world::item_stats;

const MAXIMUM_CHAT_MESSAGE_LENGTH: usize = 80;
/// Ragnarok character names cap at 24.
const MAXIMUM_CHARACTER_NAME_LENGTH: usize = 24;

/// ZST for getting the focus id of the chat text box. This is only needed to
/// focus the chat when pressing enter.
pub struct ChatTextBox;

/// ZST for the whisper-target field's focus id.
pub struct WhisperTargetTextBox;

/// Which viewing tab filters the chat feed (GDD §10.15).
pub type ChatTabIndex = u8;

pub const CHAT_TAB_ALL: ChatTabIndex = 0;
pub const CHAT_TAB_PARTY: ChatTabIndex = 1;
pub const CHAT_TAB_WHISPER: ChatTabIndex = 2;
pub const CHAT_TAB_SYSTEM: ChatTabIndex = 3;
pub const CHAT_TAB_LOOT: ChatTabIndex = 4;

/// Validated item link data from server reference state (GDD §10.15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedItemLink {
    pub item_id: u32,
    pub name: String,
    pub refine: u8,
    pub slots: u8,
    pub cards: [u32; 4],
}

/// Parse and validate an item link against authoritative item data.
/// Untrusted text cannot forge an actionable item link with fabricated stats or
/// IDs.
pub fn parse_and_validate_item_link(item_body: &str) -> Option<ValidatedItemLink> {
    let (label_part, info_part) = if let Some((before, after)) = item_body.split_once("<INFO>") {
        let info = after.strip_suffix("</INFO>").unwrap_or(after);
        (before.trim(), info.trim())
    } else {
        ("", item_body.trim())
    };

    let mut parts = info_part.split(',');
    let item_id_str = parts.next()?.trim();
    let item_id: u32 = item_id_str.parse().ok()?;

    // Server-state verification: item must exist in the authoritative database.
    let stats = item_stats(item_id)?;

    let refine: u8 = parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    // Refine sanity check: official RO refinement is at most 20.
    if refine > 20 {
        return None;
    }

    let slots: u8 = parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let max_slots = stats.slots.unwrap_or(0);
    if slots > max_slots {
        return None;
    }

    let mut cards = [0u32; 4];
    for slot in cards.iter_mut() {
        if let Some(card_str) = parts.next() {
            let card_id: u32 = card_str.trim().parse().unwrap_or(0);
            if card_id != 0 {
                let card_stat = item_stats(card_id)?;
                if !card_stat.item_type.contains("CARD") {
                    return None;
                }
                *slot = card_id;
            }
        }
    }

    let name = if !label_part.is_empty() {
        label_part.trim_matches(|c| c == '[' || c == ']').to_string()
    } else {
        stats.name.clone()
    };

    Some(ValidatedItemLink {
        item_id,
        name,
        refine,
        slots,
        cards,
    })
}

/// Sanitize untrusted chat messages containing `<ITEM>` or `<NAVI>` tags.
/// Valid item links are verified against server reference state; forged or
/// invalid links have their actionable markup defanged, preserving safe text
/// without exploits.
/// A Guide page a chat message can link to (F23, owner decision 2026-10-04).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideLinkKind {
    Monster,
    Item,
    Map,
}

impl GuideLinkKind {
    fn token_name(self) -> &'static str {
        match self {
            Self::Monster => "monster",
            Self::Item => "item",
            Self::Map => "map",
        }
    }
}

/// A validated `<GUIDE:kind:key>` link. The label always comes from the
/// client's own reference data, never from the sender, so a link cannot be
/// forged to show one name and open another, or to name an entry that does
/// not exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuideLink {
    pub kind: GuideLinkKind,
    pub key: String,
    pub label: String,
}

impl GuideLink {
    /// The wire form posted to chat.
    pub fn token(&self) -> String {
        format!("<GUIDE:{}:{}>", self.kind.token_name(), self.key)
    }
}

fn monster_label(monster: &crate::dm::reference_data::ReferenceMonster) -> String {
    match monster.name.is_empty() {
        true => monster.sprite_name.clone(),
        false => monster.name.clone(),
    }
}

/// `monster:1002`, `item:501` or `map:prontera` -> a link, only when it names
/// a real entry.
pub fn parse_guide_link(body: &str) -> Option<GuideLink> {
    let (kind, key) = body.split_once(':')?;
    let key = key.trim();
    let data = crate::dm::reference_data::reference_data();
    match kind.trim() {
        "monster" => {
            let id: u32 = key.parse().ok()?;
            let monster = data.monster_by_id(id)?;
            Some(GuideLink {
                kind: GuideLinkKind::Monster,
                key: id.to_string(),
                label: monster_label(monster),
            })
        }
        "item" => {
            let id: u32 = key.parse().ok()?;
            let item = data.item_by_id(id).or_else(|| data.card_by_id(id))?;
            Some(GuideLink {
                kind: GuideLinkKind::Item,
                key: id.to_string(),
                label: item.name.clone(),
            })
        }
        "map" => {
            let valid = !key.is_empty() && key.len() <= 16 && key.chars().all(|c| c.is_ascii_alphanumeric() || "_-@".contains(c));
            (valid && crate::world::navigation_graph().maps.iter().any(|map| map == key)).then(|| GuideLink {
                kind: GuideLinkKind::Map,
                key: key.to_owned(),
                label: key.to_owned(),
            })
        }
        _ => None,
    }
}

/// `/guide <name>`: an exact (case-insensitive) map, monster or item name.
pub fn resolve_guide_query(query: &str) -> Option<GuideLink> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    let lower = query.to_lowercase();
    if let Some(map) = crate::world::navigation_graph().maps.iter().find(|map| map.to_lowercase() == lower) {
        return parse_guide_link(&format!("map:{map}"));
    }
    let data = crate::dm::reference_data::reference_data();
    if let Some(monster) = data.monsters.iter().find(|monster| monster_label(monster).to_lowercase() == lower) {
        return parse_guide_link(&format!("monster:{}", monster.id));
    }
    data.items
        .iter()
        .chain(data.cards.iter())
        .find(|item| item.name.to_lowercase() == lower)
        .and_then(|item| parse_guide_link(&format!("item:{}", item.id)))
}

/// Each `<GUIDE:...>` token in `text`, in order: `Some` when valid.
fn guide_tokens(text: &str) -> Vec<(std::ops::Range<usize>, Option<GuideLink>)> {
    let mut tokens = Vec::new();
    let mut from = 0;
    while let Some(start) = text[from..].find("<GUIDE:").map(|offset| from + offset) {
        let body_start = start + "<GUIDE:".len();
        let Some(end) = text[body_start..].find('>').map(|offset| body_start + offset) else {
            break;
        };
        tokens.push((start..end + 1, parse_guide_link(&text[body_start..end])));
        from = end + 1;
    }
    tokens
}

/// Shown text: a valid link becomes `[Guide: name]`; an invalid one is
/// defanged to `[broken Guide link]` rather than shown raw.
fn render_guide_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (range, link) in guide_tokens(text) {
        out.push_str(&text[last..range.start]);
        match link {
            Some(link) => out.push_str(&format!("[Guide: {}]", link.label)),
            None => out.push_str("[broken Guide link]"),
        }
        last = range.end;
    }
    out.push_str(&text[last..]);
    out
}

/// The first valid Guide link in a message, which a click on it opens.
pub fn first_guide_link(text: &str) -> Option<GuideLink> {
    guide_tokens(text).into_iter().find_map(|(_, link)| link)
}

pub fn sanitize_chat_item_links(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;

    while let Some(start) = remaining.find("<ITEM>") {
        result.push_str(&remaining[..start]);
        let after_open = &remaining[start + "<ITEM>".len()..];
        if let Some(end) = after_open.find("</ITEM>") {
            let item_body = &after_open[..end];
            if let Some(valid_link) = parse_and_validate_item_link(item_body) {
                if valid_link.refine > 0 && valid_link.slots > 0 {
                    result.push_str(&format!("+{}{} [{}]", valid_link.refine, valid_link.name, valid_link.slots));
                } else if valid_link.refine > 0 {
                    result.push_str(&format!("+{} {}", valid_link.refine, valid_link.name));
                } else if valid_link.slots > 0 {
                    result.push_str(&format!("{} [{}]", valid_link.name, valid_link.slots));
                } else {
                    result.push_str(&format!("[{}]", valid_link.name));
                }
            } else {
                let label = if let Some((before, _)) = item_body.split_once("<INFO>") {
                    before.trim()
                } else {
                    item_body.trim()
                };
                if !label.is_empty() {
                    result.push_str(label);
                }
            }
            remaining = &after_open[end + "</ITEM>".len()..];
        } else {
            result.push_str("<ITEM>");
            remaining = after_open;
        }
    }
    result.push_str(remaining);

    if result.contains("<NAVI>") {
        let mut navi_cleaned = String::with_capacity(result.len());
        let mut navi_remaining = result.as_str();
        while let Some(start) = navi_remaining.find("<NAVI>") {
            navi_cleaned.push_str(&navi_remaining[..start]);
            let after_open = &navi_remaining[start + "<NAVI>".len()..];
            if let Some(end) = after_open.find("</NAVI>") {
                let navi_body = &after_open[..end];
                let label = if let Some((before, _)) = navi_body.split_once("<INFO>") {
                    before.trim()
                } else {
                    navi_body.trim()
                };
                if !label.is_empty() {
                    navi_cleaned.push_str(label);
                }
                navi_remaining = &after_open[end + "</NAVI>".len()..];
            } else {
                navi_cleaned.push_str("<NAVI>");
                navi_remaining = after_open;
            }
        }
        navi_cleaned.push_str(navi_remaining);
        return navi_cleaned;
    }

    result
}

/// Determine whether a chat message belongs in the given viewing tab filter.
pub fn chat_message_matches_tab(chat_message: &ChatMessage, tab: ChatTabIndex) -> bool {
    match tab {
        CHAT_TAB_ALL => true,
        CHAT_TAB_PARTY => {
            chat_message.text.starts_with("[Party]")
                || matches!(chat_message.color, MessageColor::Rgb {
                    red: 120,
                    green: 205,
                    blue: 255
                })
        }
        CHAT_TAB_WHISPER => {
            chat_message.text.starts_with("[Whisper]")
                || matches!(chat_message.color, MessageColor::Rgb {
                    red: 255,
                    green: 140,
                    blue: 225
                })
        }
        CHAT_TAB_LOOT => {
            chat_message.text.starts_with("You got ") || chat_message.text.starts_with("Got ") || chat_message.text.starts_with("[Loot]")
        }
        CHAT_TAB_SYSTEM => {
            matches!(
                chat_message.color,
                MessageColor::Server | MessageColor::Broadcast | MessageColor::Error
            ) || (matches!(chat_message.color, MessageColor::Information)
                && !chat_message.text.starts_with("[Party]")
                && !chat_message.text.starts_with("[Whisper]")
                && !chat_message.text.starts_with("You got ")
                && !chat_message.text.starts_with("Got ")
                && !chat_message.text.starts_with("[Loot]"))
        }
        _ => true,
    }
}

struct ChatLayoutInfo {
    area: Area,
    // TODO: Don't allocate this every frame.
    message_heights: Vec<f32>,
    // `add_text` in `lay_out` needs `&'a str`, and a `String` computed there
    // cannot satisfy that lifetime -- so the timestamp-prefixed text is
    // computed once here, alongside the height it produced, and `lay_out`
    // borrows from this struct instead of recomputing it.
    display_texts: Vec<String>,
    /// Per shown message: the click that opens its first Guide link, if any.
    guide_clicks: Vec<Option<GuideLinkClick>>,
}

/// Opens a chat message's Guide link (F23).
struct GuideLinkClick(GuideLink);

impl ClickHandler<ClientState> for GuideLinkClick {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(InputEvent::OpenGuideLink {
            kind: self.0.kind,
            key: self.0.key.clone(),
        });
    }
}

struct GuideLinkTooltip;

/// GDD 10.15's chat timestamp: display-only, so it lives here rather than on
/// `ChatMessage::text` (see that field's doc comment for why). One function
/// shared by the height-measurement and render passes below, so the text
/// actually measured is always exactly the text actually drawn -- computing
/// it separately in each pass risks the two silently drifting apart and
/// wrapping/clipping the last line.
fn display_text(chat_message: &ChatMessage, show_timestamp: bool) -> String {
    let sanitized = render_guide_links(&sanitize_chat_item_links(&chat_message.text));
    match show_timestamp {
        true => format!("[{}] {}", chat_message.sent_at.format("%H:%M:%S"), sanitized),
        false => sanitized,
    }
}

struct ChatElement<A, B> {
    chat_messages_path: A,
    active_tab_path: B,
}

impl<A, B> ChatElement<A, B> {
    fn new(chat_messages_path: A, active_tab_path: B) -> Self {
        Self {
            chat_messages_path,
            active_tab_path,
        }
    }
}

impl<A, B> Element<ClientState> for ChatElement<A, B>
where
    A: Path<ClientState, crate::state::ChatHistory>,
    B: Path<ClientState, ChatTabIndex>,
{
    type LayoutInfo = ChatLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let active_tab = *state.get(&self.active_tab_path);
            let show_timestamps = *state.get(&client_state().game_settings().show_chat_timestamps());
            let chat_messages = state.get(&self.chat_messages_path);
            // TODO: Theme this.
            let message_spacing = 5.0;

            let mut total_height = 0.0;
            let (message_heights, display_texts): (Vec<f32>, Vec<String>) = chat_messages
                .iter()
                .filter(|chat_message| chat_message_matches_tab(chat_message, active_tab))
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

                    let display_text = display_text(chat_message, show_timestamps);
                    let (size, _) = resolver.get_text_dimensions(
                        &display_text,
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

                    (size.height(), display_text)
                })
                .unzip();

            let area = resolver.with_height(total_height);
            let guide_clicks = chat_messages
                .iter()
                .filter(|chat_message| chat_message_matches_tab(chat_message, active_tab))
                .map(|chat_message| first_guide_link(&chat_message.text).map(GuideLinkClick))
                .collect();

            Self::LayoutInfo {
                area,
                message_heights,
                display_texts,
                guide_clicks,
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let active_tab = *state.get(&self.active_tab_path);
        let chat_messages = state.get(&self.chat_messages_path);
        // TODO: Theme this.
        let message_spacing = 5.0;

        let mut offset = 0.0;
        chat_messages
            .iter()
            .filter(|chat_message| chat_message_matches_tab(chat_message, active_tab))
            .zip(layout_info.message_heights.iter())
            .zip(layout_info.display_texts.iter())
            .zip(layout_info.guide_clicks.iter())
            .for_each(|(((chat_message, message_height), display_text), guide_click)| {
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
                    display_text,
                    // TODO: Theme this.
                    FontSize(14.0),
                    color,
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::LineBreak,
                );

                if let Some(guide_click) = guide_click
                    && text_area.check().run(layout)
                {
                    layout.register_click_handler(MouseButton::Left, guide_click);
                    layout.add_tooltip("Click to open this in the Adventure Guide", GuideLinkTooltip.tooltip_id());
                }

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
    /// Last character to whisper *us*, for reply. Kept separate from
    /// `whisper_target` on purpose: an incoming whisper must not silently
    /// redirect a message you are part-way through typing to someone else.
    last_whisper_sender: String,
    /// Active viewing tab filter for the message feed (GDD §10.15).
    active_tab: ChatTabIndex,
    /// Per tab: how many messages the log held when that tab was last left or
    /// opened. Messages are only ever appended, so newer matching messages
    /// past this point are that tab's unread count.
    #[hidden_element]
    tab_seen: [usize; 5],
}

/// Messages after `seen` that a tab would show.
fn unread_on_tab(messages: &[ChatMessage], seen: usize, tab: ChatTabIndex) -> usize {
    messages
        .get(seen..)
        .unwrap_or_default()
        .iter()
        .filter(|message| chat_message_matches_tab(message, tab))
        .count()
}

/// `Party`, or `Party (3)` with unread messages; capped at `99+`.
fn tab_label(name: &str, unread: usize) -> String {
    match unread {
        0 => name.to_owned(),
        1..=99 => format!("{name} ({unread})"),
        _ => format!("{name} (99+)"),
    }
}

impl ChatWindowState {
    /// `text` addressed to the selected channel, the way the Send button
    /// routes typed chat (party `/p`, whisper `/w target`).
    pub fn routed(&self, text: &str) -> String {
        match self.channel {
            CHANNEL_PARTY => format!("/p {text}"),
            CHANNEL_WHISPER => format!("/w {} {text}", self.whisper_target.trim()),
            _ => text.to_owned(),
        }
    }

    #[allow(dead_code)]
    pub fn active_tab(&self) -> ChatTabIndex {
        self.active_tab
    }

    #[allow(dead_code)]
    pub fn set_active_tab(&mut self, active_tab: ChatTabIndex) {
        self.active_tab = active_tab;
    }

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

    /// Remember who whispered us so Reply and `/r` have a target, and aim the
    /// Whisper channel at them when doing so costs nothing.
    ///
    /// The channel is still **never** switched here -- that would redirect a
    /// message you are part-way through typing, which is the whole reason
    /// `last_whisper_sender` is a separate field. But leaving
    /// `whisper_target` *empty* protects nothing: switching to the Whisper
    /// channel by hand then landed on a blank target box, so the only
    /// discoverable way to answer anyone was the Reply button.
    ///
    /// Both guards matter. An existing target is someone you chose, and a
    /// part-typed message is one you may be about to send to them -- so this
    /// fills in only when there is no target *and* nothing is being composed.
    pub fn note_whisper_from(&mut self, character_name: String) {
        if self.whisper_target.is_empty() && self.current_text.is_empty() {
            self.whisper_target = character_name.clone();
        }
        self.last_whisper_sender = character_name;
    }

    pub fn last_whisper_sender(&self) -> &str {
        &self.last_whisper_sender
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
    B: Path<ClientState, crate::state::ChatHistory>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Chat)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let current_text_path = self.chat_window_state.current_text();
        let channel_path = self.chat_window_state.channel();
        let whisper_target_path = self.chat_window_state.whisper_target();
        let active_tab_path = self.chat_window_state.active_tab();

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

        let is_tab = move |index: ChatTabIndex| {
            ComputedSelector::new_default(move |state: &ClientState| *active_tab_path.follow_safe(state) == index)
        };
        let tab_seen_path = self.chat_window_state.tab_seen();
        let select_tab = move |index: ChatTabIndex| {
            move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                // Both the tab being left and the tab being opened are read up
                // to now.
                let now = state.get(&client_state().chat_messages()).len();
                let leaving = *state.get(&active_tab_path);
                state.update_value_with(tab_seen_path, move |seen| {
                    seen[usize::from(leaving)] = now;
                    seen[usize::from(index)] = now;
                });
                state.update_value(active_tab_path, index)
            }
        };
        let label = move |name: &'static str, index: ChatTabIndex| {
            ComputedSelector::<_, String>::new_default(move |state: &ClientState| {
                let unread = match *active_tab_path.follow_safe(state) == index {
                    true => 0,
                    false => unread_on_tab(
                        client_state().chat_messages().follow_safe(state),
                        tab_seen_path.follow_safe(state)[usize::from(index)],
                        index,
                    ),
                };
                tab_label(name, unread)
            })
        };

        let last_sender_path = self.chat_window_state.last_whisper_sender();
        let no_one_to_reply_to = ComputedSelector::new_default(move |state: &ClientState| last_sender_path.follow_safe(state).is_empty());
        let reply_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let sender = state.get(&last_sender_path).clone();
            if !sender.is_empty() {
                queue.queue(InputEvent::StartWhisper { character_name: sender });
            }
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
                        button! {
                            text: "Reply",
                            tooltip: "Answer the last character who whispered you [^000001/r <message>^000000]",
                            disabled: no_one_to_reply_to,
                            disabled_tooltip: "Nobody has whispered you yet",
                            event: reply_action,
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
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "All",
                            tooltip: "Show all chat messages",
                            disabled: is_tab(CHAT_TAB_ALL),
                            event: select_tab(CHAT_TAB_ALL),
                        },
                        button! {
                            text: label("Party", CHAT_TAB_PARTY),
                            tooltip: "Show party messages only",
                            disabled: is_tab(CHAT_TAB_PARTY),
                            event: select_tab(CHAT_TAB_PARTY),
                        },
                        button! {
                            text: label("Whisper", CHAT_TAB_WHISPER),
                            tooltip: "Show private whispers only",
                            disabled: is_tab(CHAT_TAB_WHISPER),
                            event: select_tab(CHAT_TAB_WHISPER),
                        },
                        button! {
                            text: label("System", CHAT_TAB_SYSTEM),
                            tooltip: "Show system notices, server messages, and errors",
                            disabled: is_tab(CHAT_TAB_SYSTEM),
                            event: select_tab(CHAT_TAB_SYSTEM),
                        },
                        button! {
                            text: label("Loot", CHAT_TAB_LOOT),
                            tooltip: "Show item pickup and drop logs",
                            disabled: is_tab(CHAT_TAB_LOOT),
                            event: select_tab(CHAT_TAB_LOOT),
                        },
                    ),
                },
                scroll_view! {
                    follow: true,
                    children: ChatElement::new(self.chat_messages_path, self.chat_window_state.active_tab()),
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use korangar_networking::MessageColor;

    use super::{
        CHANNEL_PUBLIC, CHANNEL_WHISPER, CHAT_TAB_ALL, CHAT_TAB_LOOT, CHAT_TAB_PARTY, CHAT_TAB_SYSTEM, CHAT_TAB_WHISPER, ChatWindowState,
        GuideLinkKind, chat_message_matches_tab, display_text, first_guide_link, parse_and_validate_item_link, parse_guide_link,
        render_guide_links, resolve_guide_query, sanitize_chat_item_links, tab_label, unread_on_tab,
    };
    use crate::state::ChatMessage;

    /// GDD 10.15's chat timestamp is display-only: `ChatMessage::text` must
    /// stay exactly what was passed in (parsers and marker-matching
    /// throughout this crate rely on that), while `display_text` -- what the
    /// window actually measures and draws -- carries the `[HH:MM:SS]` prefix.
    #[test]
    fn unread_counts_only_newer_messages_the_tab_would_show() {
        let public = ChatMessage::new("someone says hi".to_owned(), MessageColor::Rgb {
            red: 255,
            green: 255,
            blue: 255,
        });
        let messages = vec![public.clone(), public.clone(), public.clone()];
        // Seen up to the first message: two newer ones for All.
        assert_eq!(unread_on_tab(&messages, 1, CHAT_TAB_ALL), 2);
        // The Loot tab would show none of them, so it has nothing unread.
        let loot = messages[1..].iter().filter(|m| chat_message_matches_tab(m, CHAT_TAB_LOOT)).count();
        assert_eq!(unread_on_tab(&messages, 1, CHAT_TAB_LOOT), loot);
        assert_eq!(unread_on_tab(&messages, 3, CHAT_TAB_ALL), 0);
        // A stale index past the end is not a panic.
        assert_eq!(unread_on_tab(&messages, 99, CHAT_TAB_ALL), 0);
    }

    #[test]
    fn tab_labels_show_counts_and_cap() {
        assert_eq!(tab_label("Party", 0), "Party");
        assert_eq!(tab_label("Party", 3), "Party (3)");
        assert_eq!(tab_label("Party", 120), "Party (99+)");
    }

    #[test]
    fn display_text_prefixes_a_timestamp_without_touching_the_stored_text() {
        let mut message = ChatMessage::new("hello party".to_owned(), MessageColor::Information);
        // Fix the timestamp so this test does not depend on wall-clock time.
        message.sent_at = chrono::Local.with_ymd_and_hms(2026, 9, 27, 14, 5, 9).unwrap();

        assert_eq!(message.text, "hello party");
        assert_eq!(display_text(&message, true), "[14:05:09] hello party");
        assert_eq!(display_text(&message, false), "hello party");
    }

    /// A first whisper should leave the Whisper channel ready to answer.
    #[test]
    fn an_incoming_whisper_aims_an_empty_target() {
        let mut state = ChatWindowState::default();
        state.note_whisper_from("Bob".to_owned());

        assert_eq!(state.whisper_target, "Bob");
        assert_eq!(state.last_whisper_sender(), "Bob");
        // Receiving must never move the channel: that would redirect whatever
        // the player is typing right now.
        assert_eq!(state.channel, CHANNEL_PUBLIC);
    }

    /// A target the player chose outranks whoever happens to whisper next.
    #[test]
    fn an_incoming_whisper_does_not_steal_a_chosen_target() {
        let mut state = ChatWindowState::default();
        state.start_whisper("Alice".to_owned());
        state.note_whisper_from("Bob".to_owned());

        assert_eq!(state.whisper_target, "Alice");
        // Reply still goes to the person who actually whispered.
        assert_eq!(state.last_whisper_sender(), "Bob");
        assert_eq!(state.channel, CHANNEL_WHISPER);
    }

    /// A half-typed message must not acquire a recipient behind the player's
    /// back -- the case the separate `last_whisper_sender` field exists for.
    #[test]
    fn an_incoming_whisper_does_not_aim_a_part_typed_message() {
        let mut state = ChatWindowState::default();
        state.current_text = "meet me at prontera".to_owned();
        state.note_whisper_from("Bob".to_owned());

        assert!(state.whisper_target.is_empty());
        assert_eq!(state.last_whisper_sender(), "Bob");
    }

    #[test]
    fn chat_tab_filters_match_appropriate_messages() {
        let public_msg = ChatMessage::new("Alice: Hello all".to_owned(), MessageColor::Rgb {
            red: 255,
            green: 255,
            blue: 255,
        });
        let party_msg = ChatMessage::new("[Party] Bob: Ready to enter".to_owned(), MessageColor::Rgb {
            red: 120,
            green: 205,
            blue: 255,
        });
        let whisper_msg = ChatMessage::new("[Whisper] Carol: Secret info".to_owned(), MessageColor::Rgb {
            red: 255,
            green: 140,
            blue: 225,
        });
        let server_msg = ChatMessage::new("Map server connection established.".to_owned(), MessageColor::Server);
        let error_msg = ChatMessage::new("Skill failed: not enough SP.".to_owned(), MessageColor::Error);
        let loot_msg = ChatMessage::new("You got Jellopy (1).".to_owned(), MessageColor::Information);
        let alt_loot_msg = ChatMessage::new("Got Poring Card ×1".to_owned(), MessageColor::Information);

        // CHAT_TAB_ALL matches everything
        assert!(chat_message_matches_tab(&public_msg, CHAT_TAB_ALL));
        assert!(chat_message_matches_tab(&party_msg, CHAT_TAB_ALL));
        assert!(chat_message_matches_tab(&whisper_msg, CHAT_TAB_ALL));
        assert!(chat_message_matches_tab(&server_msg, CHAT_TAB_ALL));
        assert!(chat_message_matches_tab(&error_msg, CHAT_TAB_ALL));
        assert!(chat_message_matches_tab(&loot_msg, CHAT_TAB_ALL));

        // CHAT_TAB_PARTY
        assert!(!chat_message_matches_tab(&public_msg, CHAT_TAB_PARTY));
        assert!(chat_message_matches_tab(&party_msg, CHAT_TAB_PARTY));
        assert!(!chat_message_matches_tab(&whisper_msg, CHAT_TAB_PARTY));
        assert!(!chat_message_matches_tab(&loot_msg, CHAT_TAB_PARTY));

        // CHAT_TAB_WHISPER
        assert!(!chat_message_matches_tab(&public_msg, CHAT_TAB_WHISPER));
        assert!(!chat_message_matches_tab(&party_msg, CHAT_TAB_WHISPER));
        assert!(chat_message_matches_tab(&whisper_msg, CHAT_TAB_WHISPER));

        // CHAT_TAB_SYSTEM
        assert!(chat_message_matches_tab(&server_msg, CHAT_TAB_SYSTEM));
        assert!(chat_message_matches_tab(&error_msg, CHAT_TAB_SYSTEM));
        assert!(!chat_message_matches_tab(&party_msg, CHAT_TAB_SYSTEM));
        assert!(!chat_message_matches_tab(&whisper_msg, CHAT_TAB_SYSTEM));
        assert!(!chat_message_matches_tab(&loot_msg, CHAT_TAB_SYSTEM));

        // CHAT_TAB_LOOT
        assert!(chat_message_matches_tab(&loot_msg, CHAT_TAB_LOOT));
        assert!(chat_message_matches_tab(&alt_loot_msg, CHAT_TAB_LOOT));
        assert!(!chat_message_matches_tab(&public_msg, CHAT_TAB_LOOT));
        assert!(!chat_message_matches_tab(&party_msg, CHAT_TAB_LOOT));
    }

    #[test]
    fn safe_item_links_validate_against_item_database_and_defang_forgeries() {
        // Valid item link: ID 1101 (Sword in items.json)
        let valid_link_raw = "<ITEM>[Sword]<INFO>1101,0,0,0,0,0,0</INFO></ITEM>";
        let validated = parse_and_validate_item_link("[Sword]<INFO>1101,0,0,0,0,0,0</INFO>");
        assert!(validated.is_some());
        let val = validated.unwrap();
        assert_eq!(val.item_id, 1101);
        assert_eq!(sanitize_chat_item_links(valid_link_raw), "[Sword]");

        // Valid item link with refine and slots: +7 Sword [3]
        let refined_link = "<ITEM>[Sword]<INFO>1101,7,3,0,0,0,0</INFO></ITEM>";
        assert_eq!(sanitize_chat_item_links(refined_link), "+7Sword [3]");

        // Forged item link: fake item ID 999999
        let fake_link = "<ITEM>[Godly Blade]<INFO>999999,0,0,0,0,0,0</INFO></ITEM>";
        assert!(parse_and_validate_item_link("[Godly Blade]<INFO>999999,0,0,0,0,0,0</INFO>").is_none());
        assert_eq!(sanitize_chat_item_links(fake_link), "[Godly Blade]");

        // Forged refine > 20
        let fake_refine = "<ITEM>[Sword]<INFO>1101,99,0,0,0,0,0</INFO></ITEM>";
        assert!(parse_and_validate_item_link("[Sword]<INFO>1101,99,0,0,0,0,0</INFO>").is_none());
        assert_eq!(sanitize_chat_item_links(fake_refine), "[Sword]");

        // Forged slots exceeding base item max slots (Sword has 3 slots max)
        let fake_slots = "<ITEM>[Sword]<INFO>1101,0,9,0,0,0,0</INFO></ITEM>";
        assert!(parse_and_validate_item_link("[Sword]<INFO>1101,0,9,0,0,0,0</INFO>").is_none());
        assert_eq!(sanitize_chat_item_links(fake_slots), "[Sword]");

        // Navigation tags in chat are defanged to plain label
        let navi_in_chat = "Meet at <NAVI>[Kafra]<INFO>prontera,150,150</INFO></NAVI> now";
        assert_eq!(sanitize_chat_item_links(navi_in_chat), "Meet at [Kafra] now");
    }

    #[test]
    fn guide_links_only_name_real_entries() {
        let poring = parse_guide_link("monster:1002").expect("Poring is in the bestiary");
        assert_eq!((poring.kind, poring.key.as_str()), (GuideLinkKind::Monster, "1002"));
        assert!(!poring.label.is_empty());
        assert!(parse_guide_link("item:501").is_some(), "Red Potion");
        assert!(parse_guide_link("map:prontera").is_some());
        for forged in [
            "monster:999999",
            "monster:1002 Evil",
            "item:abc",
            "map:../etc",
            "map:nowhere_map",
            "spell:1",
            "monster",
            "",
        ] {
            assert!(parse_guide_link(forged).is_none(), "{forged:?} must not validate");
        }
        // The posted token parses back to the same link.
        let body = poring.token().trim_start_matches("<GUIDE:").trim_end_matches('>').to_owned();
        assert_eq!(parse_guide_link(&body), Some(poring));
    }

    #[test]
    fn guide_links_render_and_defang() {
        let poring = parse_guide_link("monster:1002").unwrap();
        let text = "see <GUIDE:monster:999999> then <GUIDE:monster:1002>!";
        assert_eq!(
            render_guide_links(text),
            format!("see [broken Guide link] then [Guide: {}]!", poring.label)
        );
        assert_eq!(first_guide_link(text), Some(poring));
        assert_eq!(render_guide_links("no links <GUIDE:unclosed"), "no links <GUIDE:unclosed");
        assert_eq!(first_guide_link("plain"), None);
    }

    #[test]
    fn guide_query_resolves_exact_names() {
        assert_eq!(resolve_guide_query("PRONTERA").map(|link| link.kind), Some(GuideLinkKind::Map));
        let poring = parse_guide_link("monster:1002").unwrap();
        assert_eq!(
            resolve_guide_query(&poring.label.to_uppercase()).map(|link| link.key),
            Some("1002".to_owned())
        );
        assert_eq!(resolve_guide_query("Red Potion").map(|link| link.key), Some("501".to_owned()));
        assert_eq!(resolve_guide_query("definitely not a thing"), None);
        assert_eq!(resolve_guide_query("  "), None);
    }
}
