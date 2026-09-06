use std::cell::UnsafeCell;

use korangar_interface::MouseMode;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::event::{ClickHandler, DropHandler, Event, EventQueue};
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItemDetails;
use ragnarok_packets::HotbarSlot;
use rust_state::{Path, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::{ItemSource, SkillSource};
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::hotbar::{HOTBAR_COLUMNS, HOTBAR_ROWS, HOTBAR_SLOTS, Hotbar, HotbarBinding};
use crate::state::localization::LocalizationPathExt;
use crate::state::skills::LearnedSkill;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{item_tooltip_text, skill_tooltip_text};

fn slot_label(slot: usize) -> &'static str {
    const LABELS: [&str; HOTBAR_SLOTS] = [
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "A1", "A2", "A3", "A4", "A5",
        "A6", "A7", "A8", "A9",
    ];
    LABELS.get(slot).copied().unwrap_or("?")
}

struct ActivateSlot {
    slot: HotbarSlot,
}

impl ClickHandler<ClientState> for ActivateSlot {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(InputEvent::CastSkill { slot: self.slot });
    }
}

struct PickupSlot<H> {
    hotbar_path: H,
    slot: HotbarSlot,
}

impl<H> ClickHandler<ClientState> for PickupSlot<H>
where
    H: Path<ClientState, Hotbar>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        match state.get(&self.hotbar_path).get_slot(self.slot) {
            Some(HotbarBinding::Skill(skill)) => queue.queue(Event::SetMouseMode {
                mouse_mode: MouseMode::Custom {
                    mode: MouseInputMode::MoveSkill {
                        skill: skill.clone(),
                        source: SkillSource::Hotbar { slot: self.slot },
                    },
                },
            }),
            Some(HotbarBinding::Item { item_id }) => {
                let Some(item) = state
                    .get(&client_state().inventory())
                    .items()
                    .iter()
                    .find(|item| item.item_id == *item_id)
                    .cloned()
                else {
                    return;
                };
                queue.queue(Event::SetMouseMode {
                    mouse_mode: MouseMode::Custom {
                        mode: MouseInputMode::MoveItem {
                            item,
                            source: ItemSource::Hotbar { slot: self.slot },
                        },
                    },
                });
            }
            None => {}
        }
    }
}

struct SlotDrop {
    slot: HotbarSlot,
}

impl DropHandler<ClientState> for SlotDrop {
    fn handle_drop(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
        match mouse_mode {
            MouseMode::Custom {
                mode: MouseInputMode::MoveSkill { source, skill },
            } => queue.queue(InputEvent::MoveSkill {
                source: *source,
                destination: SkillSource::Hotbar { slot: self.slot },
                skill: skill.clone(),
            }),
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { source, item },
            } => queue.queue(InputEvent::MoveItem {
                source: *source,
                destination: ItemSource::Hotbar { slot: self.slot },
                item: item.clone(),
            }),
            _ => {}
        }
    }
}

struct HotbarSlotBox<H, S> {
    hotbar_path: H,
    skills_path: S,
    slot: usize,
    activate: ActivateSlot,
    pickup: PickupSlot<H>,
    drop: SlotDrop,
    tooltip_text: UnsafeCell<String>,
    amount_text: UnsafeCell<String>,
}

impl<H, S> HotbarSlotBox<H, S>
where
    H: Copy,
{
    fn new(hotbar_path: H, skills_path: S, slot: usize) -> Self {
        let hotbar_slot = HotbarSlot(slot as u16);
        Self {
            hotbar_path,
            skills_path,
            slot,
            activate: ActivateSlot { slot: hotbar_slot },
            pickup: PickupSlot {
                hotbar_path,
                slot: hotbar_slot,
            },
            drop: SlotDrop { slot: hotbar_slot },
            tooltip_text: UnsafeCell::new(String::new()),
            amount_text: UnsafeCell::new(String::new()),
        }
    }
}

impl<H, S> Element<ClientState> for HotbarSlotBox<H, S>
where
    H: Path<ClientState, Hotbar> + Copy,
    S: Path<ClientState, Vec<LearnedSkill>>,
{
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        _: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| Self::LayoutInfo {
            area: resolver.with_height(40.0),
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let is_drop_target = matches!(layout.get_mouse_mode(), MouseMode::Custom {
            mode: MouseInputMode::MoveSkill { .. } | MouseInputMode::MoveItem { .. },
        });
        let is_hovered = match is_drop_target {
            true => layout_info.area.check().any_mouse_mode().run(layout),
            false => layout_info.area.check().run(layout),
        };

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(20.0),
            if is_hovered && !is_drop_target {
                Color::rgb_u8(60, 60, 60)
            } else {
                Color::rgb_u8(40, 40, 40)
            },
            Color::rgba_u8(0, 0, 0, 100),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if is_drop_target && is_hovered {
            layout.set_hovered();
            layout.register_drop_handler(&self.drop);
        }

        let binding = state.get(&self.hotbar_path).get_slot(HotbarSlot(self.slot as u16));
        match binding {
            Some(HotbarBinding::Skill(skill)) => {
                let learned = state
                    .get(&self.skills_path)
                    .iter()
                    .find(|learned| learned.skill_id == skill.skill_id);
                let color = match learned.is_some_and(|learned| learned.skill_level.0 >= skill.maximum_level.0) {
                    true => Color::WHITE,
                    false => Color::rgb_u8(160, 160, 160),
                };
                if let (Some(actions), Some(sprite)) = (&skill.actions, &skill.sprite) {
                    layout.with_clip(layout_info.area, |layout| {
                        layout.add_sprite(layout_info.area, actions, sprite, &skill.animation_state, 0, color, 1.0);
                    });
                }
                if is_hovered {
                    layout.register_click_handler(MouseButton::Left, &self.activate);
                    layout.register_click_handler(MouseButton::Right, &self.pickup);
                    let level = learned.map_or(1, |learned| learned.skill_level.0);
                    let text = skill_tooltip_text(skill.skill_id.0, &skill.skill_name, level, skill.maximum_level.0);
                    unsafe {
                        *self.tooltip_text.get() = text;
                        layout.add_tooltip(self.tooltip_text.as_ref_unchecked().as_str(), HotbarSlotTooltip.tooltip_id());
                    }
                }
            }
            Some(HotbarBinding::Item { item_id }) => {
                let inventory_path = client_state().inventory();
                let item = state.get(&inventory_path).items().iter().find(|item| item.item_id == *item_id);
                if let Some(texture) = item.and_then(|item| item.metadata.texture.clone()) {
                    layout.add_texture(layout_info.area, texture, Color::WHITE, false);
                }
                if let Some(item) = item {
                    let amount = match &item.details {
                        InventoryItemDetails::Regular { amount, .. } | InventoryItemDetails::Equippable { amount, .. } => *amount,
                    };
                    if amount > 1 {
                        unsafe {
                            *self.amount_text.get() = amount.to_string();
                        }
                        layout.add_text(
                            layout_info.area,
                            unsafe { self.amount_text.as_ref_unchecked().as_str() },
                            FontSize(12.0),
                            Color::rgb_u8(255, 200, 255),
                            Color::rgb_u8(255, 160, 60),
                            HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                            VerticalAlignment::Bottom { offset: 3.0 },
                            OverflowBehavior::Shrink,
                        );
                    }
                    if is_hovered {
                        layout.register_click_handler(MouseButton::Left, &self.activate);
                        layout.register_click_handler(MouseButton::Right, &self.pickup);
                        let text = item_tooltip_text(item.item_id.0, &item.metadata.name, None, None, None);
                        unsafe {
                            *self.tooltip_text.get() = text;
                            layout.add_tooltip(self.tooltip_text.as_ref_unchecked().as_str(), HotbarSlotTooltip.tooltip_id());
                        }
                    }
                } else if is_hovered {
                    layout.register_click_handler(MouseButton::Left, &self.activate);
                }
            }
            None => {
                if is_hovered && !is_drop_target {
                    unsafe {
                        *self.tooltip_text.get() = "Drop a skill or potion here.".to_owned();
                        layout.add_tooltip(self.tooltip_text.as_ref_unchecked().as_str(), HotbarSlotTooltip.tooltip_id());
                    }
                }
            }
        }

        layout.add_text(
            layout_info.area,
            slot_label(self.slot),
            FontSize(10.0),
            Color::rgb_u8(200, 200, 200),
            Color::rgb_u8(255, 160, 60),
            HorizontalAlignment::Left { offset: 3.0, border: 2.0 },
            VerticalAlignment::Top { offset: 2.0 },
            OverflowBehavior::Shrink,
        );
    }
}

struct HotbarSlotTooltip;

pub struct HotbarWindow<A, B> {
    hotbar_path: A,
    skills_path: B,
}

impl<A, B> HotbarWindow<A, B> {
    pub fn new(hotbar_path: A, skills_path: B) -> Self {
        Self { hotbar_path, skills_path }
    }
}

impl<A, B> CustomWindow<ClientState> for HotbarWindow<A, B>
where
    A: Path<ClientState, Hotbar> + Copy + 'static,
    B: Path<ClientState, Vec<LearnedSkill>> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Hotbar)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().hotbar_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                text! {
                    text: "1–9 · Ctrl+1–9 · Alt+1–9  (F1–F9 still work)",
                    height: 14.0,
                    font_size: FontSize(11.0),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                fragment! {
                    gaps: 4.0,
                    children: std::array::from_fn::<_, HOTBAR_ROWS, _>(|row| {
                        split! {
                            gaps: theme().window().gaps(),
                            children: std::array::from_fn::<_, HOTBAR_COLUMNS, _>(|column| {
                                HotbarSlotBox::new(self.hotbar_path, self.skills_path, row * HOTBAR_COLUMNS + column)
                            }),
                        }
                    }),
                },
            )
        }
    }
}
