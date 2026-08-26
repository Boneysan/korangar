use korangar_interface::application::Size;
use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use super::WindowClass;
use crate::graphics::Color;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::ClientState;
use crate::state::inventory::Inventory;
use crate::state::quests::QuestLogState;
use crate::state::theme::InterfaceThemeType;

/// Vertical gap between two contracts.
const QUEST_SPACING: f32 = 8.0;
/// Vertical gap between a contract's title and its item lines.
const LINE_SPACING: f32 = 2.0;

const TITLE_FONT_SIZE: f32 = 14.0;
const LINE_FONT_SIZE: f32 = 13.0;

/// One rendered row: the text, how tall it is, and how it is coloured.
struct QuestRow {
    text: String,
    color: Color,
    font_size: f32,
    height: f32,
    /// Item lines are indented under their contract title.
    indent: f32,
}

struct QuestLogLayoutInfo {
    area: Area,
    rows: Vec<QuestRow>,
}

/// Renders the quest log as a flat list of rows.
///
/// The have-counts are read from the inventory every frame rather than cached
/// in the quest state: the player's stock changes on every pickup, drop, trade
/// and vend, and a cached copy would be a second source of truth to keep in
/// step with all of them.
struct QuestLogElement<A, B> {
    quest_log_path: A,
    inventory_path: B,
}

impl<A, B> QuestLogElement<A, B> {
    fn new(quest_log_path: A, inventory_path: B) -> Self {
        Self {
            quest_log_path,
            inventory_path,
        }
    }
}

impl<A, B> QuestLogElement<A, B>
where
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
{
    fn build_rows(&self, state: &State<ClientState>) -> Vec<QuestRow> {
        let quest_log = state.get(&self.quest_log_path);
        let inventory = state.get(&self.inventory_path);

        if quest_log.is_empty() {
            return vec![QuestRow {
                text: "No active quests.".to_owned(),
                color: Color::monochrome_u8(150),
                font_size: TITLE_FONT_SIZE,
                height: 0.0,
                indent: 0.0,
            }];
        }

        let mut rows = Vec::new();

        for quest in quest_log.quests() {
            rows.push(QuestRow {
                text: quest.name().to_owned(),
                color: Color::rgb_u8(120, 190, 255),
                font_size: TITLE_FONT_SIZE,
                height: 0.0,
                indent: 0.0,
            });

            if quest.requirements().is_empty() {
                rows.push(QuestRow {
                    text: "Speak to the quest giver.".to_owned(),
                    color: Color::monochrome_u8(150),
                    font_size: LINE_FONT_SIZE,
                    height: 0.0,
                    indent: 12.0,
                });
                continue;
            }

            for requirement in quest.requirements() {
                let carried = inventory.count_of(requirement.item_id);
                let done = carried >= requirement.needed;

                rows.push(QuestRow {
                    text: format!("{}  {} / {}", requirement.item_name, carried, requirement.needed),
                    color: match done {
                        true => Color::rgb_u8(120, 220, 120),
                        false => Color::monochrome_u8(210),
                    },
                    font_size: LINE_FONT_SIZE,
                    height: 0.0,
                    indent: 12.0,
                });
            }
        }

        rows
    }
}

impl<A, B> Element<ClientState> for QuestLogElement<A, B>
where
    A: Path<ClientState, QuestLogState>,
    B: Path<ClientState, Inventory>,
{
    type LayoutInfo = QuestLogLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let mut rows = self.build_rows(state);
            let mut total_height = 0.0;
            let mut previous_was_title = false;

            for row in rows.iter_mut() {
                let is_title = row.indent == 0.0;

                let (size, _) = resolver.get_text_dimensions(
                    &row.text,
                    row.color,
                    row.color,
                    FontSize(row.font_size),
                    HorizontalAlignment::Left {
                        offset: 5.0 + row.indent,
                        border: 3.0,
                    },
                    OverflowBehavior::LineBreak,
                );

                row.height = size.height();

                if total_height != 0.0 {
                    total_height += match is_title && !previous_was_title {
                        true => QUEST_SPACING,
                        false => LINE_SPACING,
                    };
                }
                total_height += row.height;
                previous_was_title = is_title;
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
        let mut previous_was_title = false;

        for row in &layout_info.rows {
            let is_title = row.indent == 0.0;

            if offset != 0.0 {
                offset += match is_title && !previous_was_title {
                    true => QUEST_SPACING,
                    false => LINE_SPACING,
                };
            }

            let row_area = Area {
                left: layout_info.area.left,
                top: layout_info.area.top + offset,
                width: layout_info.area.width,
                height: row.height,
            };

            layout.add_text(
                row_area,
                &row.text,
                FontSize(row.font_size),
                row.color,
                row.color,
                HorizontalAlignment::Left {
                    offset: 5.0 + row.indent,
                    border: 3.0,
                },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::LineBreak,
            );

            offset += row.height;
            previous_was_title = is_title;
        }
    }
}

/// The quest log.
///
/// Campaign hunting contracts show what they want handed in and how much of it
/// the player is carrying. Before this existed the three quest packets were
/// registered and then dropped on the floor, so every quest in the campaign was
/// invisible outside NPC dialogue.
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

        window! {
            title: "Quest Log",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                scroll_view! {
                    children: QuestLogElement::new(self.quest_log_path, self.inventory_path),
                },
            ),
        }
    }
}
