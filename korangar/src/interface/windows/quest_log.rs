use korangar_interface::application::Size;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use super::WindowClass;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::ClientState;
use crate::state::inventory::Inventory;
use crate::state::quests::{QuestEntry, QuestLogState, QuestLogStatePathExt};
use crate::state::theme::InterfaceThemeType;

const LINE_SPACING: f32 = 10.0;
const PROGRESS_HEIGHT: f32 = 10.0;

const TITLE_FONT_SIZE: f32 = 18.0;
const LINE_FONT_SIZE: f32 = 16.0;

/// One rendered row: the text, how tall it is, and how it is coloured.
#[derive(Clone)]
struct QuestRow {
    text: String,
    color: Color,
    font_size: f32,
    height: f32,
    progress: Option<f32>,
}

struct QuestLogLayoutInfo {
    area: Area,
    rows: Vec<QuestRow>,
}

/// Wrapped objective text, measured at the available width and interface scale.
struct QuestDetails {
    rows: Vec<QuestRow>,
}

impl QuestDetails {
    fn new(quest: &QuestEntry, inventory: &Inventory) -> Self {
        let mut rows = Vec::new();
        let mut push = |text: String, color: Color, progress: Option<f32>| {
            rows.push(QuestRow {
                text,
                color,
                font_size: LINE_FONT_SIZE,
                height: 0.0,
                progress,
            });
        };
        if quest.requirements().is_empty() {
            push(
                "Follow the quest giver's instructions. Objective details are not available in this journal yet.".into(),
                Color::monochrome_u8(215),
                None,
            );
        } else {
            let ready = quest.items_ready(|id| inventory.count_of(id));
            push(
                if ready {
                    "Items collected — return to the quest giver to hand them in.".into()
                } else {
                    "Collect the following items and keep them in your inventory.".into()
                },
                Color::monochrome_u8(235),
                None,
            );
            for item in quest.requirements() {
                let carried = inventory.count_of(item.item_id);
                let remaining = item.needed.saturating_sub(carried);
                let status = if remaining == 0 {
                    "Collected".into()
                } else {
                    format!("{remaining} more needed")
                };
                push(
                    format!("{}\n{} / {} carried · {}", item.item_name, carried, item.needed, status),
                    if remaining == 0 {
                        Color::rgb_u8(150, 230, 170)
                    } else {
                        Color::monochrome_u8(235)
                    },
                    Some(if item.needed == 0 {
                        1.0
                    } else {
                        (carried as f32 / item.needed as f32).min(1.0)
                    }),
                );
            }
        }
        Self { rows }
    }
}

impl Element<ClientState> for QuestDetails {
    type LayoutInfo = QuestLogLayoutInfo;

    fn create_layout_info(
        &mut self,
        _: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let mut rows = self.rows.clone();
            let mut total_height = 0.0;

            for row in rows.iter_mut() {
                let (size, _) = resolver.get_text_dimensions(
                    &row.text,
                    row.color,
                    row.color,
                    FontSize(row.font_size),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    OverflowBehavior::LineBreak,
                );

                row.height = size.height() + if row.progress.is_some() { PROGRESS_HEIGHT } else { 0.0 };

                if total_height != 0.0 {
                    total_height += LINE_SPACING;
                }
                total_height += row.height;
            }

            let area = resolver.with_height(total_height);

            QuestLogLayoutInfo { area, rows }
        })
    }

    fn lay_out<'a>(
        &'a self,
        _: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let mut offset = 0.0;

        for row in &layout_info.rows {
            if offset != 0.0 {
                offset += LINE_SPACING;
            }

            let row_area = Area {
                left: layout_info.area.left,
                top: layout_info.area.top + offset,
                width: layout_info.area.width,
                height: row.height - if row.progress.is_some() { PROGRESS_HEIGHT } else { 0.0 },
            };

            layout.add_text(
                row_area,
                &row.text,
                FontSize(row.font_size),
                row.color,
                row.color,
                HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::LineBreak,
            );

            if let Some(progress) = row.progress {
                let bar = Area {
                    left: row_area.left + 5.0,
                    top: row_area.top + row_area.height + 4.0,
                    width: (row_area.width - 10.0).max(0.0),
                    height: 4.0,
                };
                layout.add_rectangle(
                    bar,
                    CornerDiameter::uniform(2.0),
                    Color::monochrome_u8(65),
                    Color::rgba_u8(0, 0, 0, 0),
                    ShadowPadding::uniform(0.0),
                );
                if progress > 0.0 {
                    layout.add_rectangle(
                        Area {
                            width: bar.width * progress,
                            ..bar
                        },
                        CornerDiameter::uniform(2.0),
                        row.color,
                        Color::rgba_u8(0, 0, 0, 0),
                        ShadowPadding::uniform(0.0),
                    );
                }
            }
            offset += row.height;
        }
    }
}

/// Rebuilt from live inventory. Stable quest IDs preserve collapse state when
/// searching, pinning, or receiving a differently ordered server roster.
struct QuestList<A, B> {
    quest_log_path: A,
    inventory_path: B,
    elements: Vec<(u64, ElementBox<ClientState>)>,
}

impl<A, B> Element<ClientState> for QuestList<A, B>
where
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
{
    type LayoutInfo = ();

    fn create_layout_info(&mut self, state: &State<ClientState>, mut store: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) {
        use korangar_interface::prelude::*;
        with_single_resolver(resolvers, |resolver| {
            self.elements.clear();
            let log = state.get(&self.quest_log_path);
            let inventory = state.get(&self.inventory_path);
            let ready_count = log
                .quests()
                .iter()
                .filter(|quest| quest.items_ready(|id| inventory.count_of(id)))
                .count();
            let mut visible: Vec<_> = log
                .quests()
                .iter()
                .filter(|quest| quest.matches_search(&log.search) && (!log.ready_only || quest.items_ready(|id| inventory.count_of(id))))
                .collect();
            visible.sort_by_key(|quest| !log.is_pinned(quest.quest_id));
            self.elements.push((
                u64::MAX,
                ErasedElement::new(text! {
                    text: format!("{} shown · {} in journal · {} with items collected", visible.len(), log.quests().len(), ready_count),
                    font_size: FontSize(14.0),
                    overflow_behavior: OverflowBehavior::LineBreak,
                }),
            ));
            if visible.is_empty() {
                let message = if log.is_empty() {
                    "Your journal is empty.\nSpeak to quest givers to discover quests. Accepted quests appear here."
                } else {
                    "No matching quests.\nClear the search or turn off Items collected to see more quests."
                };
                self.elements.push((
                    u64::MAX - 1,
                    ErasedElement::new(text! {
                        text: message,
                        font_size: FontSize(LINE_FONT_SIZE),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    }),
                ));
            }
            for quest in visible {
                let id = quest.quest_id;
                let pinned = log.is_pinned(id);
                let completed = quest
                    .requirements()
                    .iter()
                    .filter(|item| inventory.count_of(item.item_id) >= item.needed)
                    .count();
                let status = if quest.requirements().is_empty() {
                    "Follow quest instructions".into()
                } else if quest.items_ready(|id| inventory.count_of(id)) {
                    "Items collected".into()
                } else {
                    format!("Collecting · {completed}/{} objectives", quest.requirements().len())
                };
                let title = format!("{}{}\n{status}", if pinned { "Pinned · " } else { "" }, quest.name());
                let path = self.quest_log_path;
                self.elements.push((
                    u64::from(id),
                    ErasedElement::new(collapsible! {
                        text: title,
                        font_size: FontSize(TITLE_FONT_SIZE),
                        title_height: 56.0,
                        overflow_behavior: OverflowBehavior::LineBreak,
                        initially_expanded: true,
                        children: (
                            QuestDetails::new(quest, inventory),
                            button! {
                                text: if pinned { "Unpin quest" } else { "Pin to top" },
                                tooltip: "Keep this quest at the top of your journal for this session",
                                height: 30.0,
                                font_size: FontSize(15.0),
                                event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                                    state.update_value_with(path, move |log| log.toggle_pin(id));
                                },
                            },
                        ),
                    }),
                ));
            }
            for (id, element) in &mut self.elements {
                element.create_layout_info(state, store.child_store(*id), resolver);
            }
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a (),
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (id, element) in &self.elements {
            element.lay_out(state, store.child_store(*id), &(), layout);
        }
    }
}

pub struct QuestLogWindow<A, B> {
    quest_log_path: A,
    inventory_path: B,
}

impl<A, B> QuestLogWindow<A, B> {
    pub fn new(quest_log_path: A, inventory_path: B) -> Self {
        Self {
            quest_log_path,
            inventory_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for QuestLogWindow<A, B>
where
    A: Path<ClientState, QuestLogState> + 'static,
    B: Path<ClientState, Inventory> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::QuestLog)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;
        struct QuestSearchBox;
        let path = self.quest_log_path;
        window! {
            title: "Quest Journal",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text_box! {
                    ghost_text: "Search quests or objective items…",
                    state: path.search(),
                    input_handler: DefaultHandler::<_, _, 80>::new(path.search(), |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| queue.queue(Event::Unfocus)),
                    focus_id: QuestSearchBox,
                    height: 32.0,
                    font_size: FontSize(16.0),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "All quests",
                            height: 32.0,
                            font_size: FontSize(15.0),
                            tooltip: "Clear search and show every quest in your journal",
                            event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                                state.update_value_with(path, |log| { log.search.clear(); log.ready_only = false; });
                            },
                        },
                        state_button! {
                            text: "Items collected",
                            height: 32.0,
                            font_size: FontSize(15.0),
                            tooltip: "Show quests whose required items are all in your inventory",
                            state: path.ready_only(),
                            event: Toggle(path.ready_only()),
                        },
                    ),
                },
                scroll_view! {
                    children: QuestList { quest_log_path: path, inventory_path: self.inventory_path, elements: Vec::new() },
                },
                text! {
                    text: "Ctrl+Q · Journal    Ctrl+W · Close",
                    font_size: FontSize(14.0),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
            ),
        }
    }
}
