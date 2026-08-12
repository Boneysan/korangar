use std::cell::UnsafeCell;

use korangar_interface::MouseMode;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::event::{ClickHandler, DropHandler, Event, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use rust_state::{Path, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::ItemSource;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{ResourceMetadata, item_stats, item_tooltip_text};

#[derive(Default)]
struct AmountDisplay {
    amount: u16,
    string: Option<String>,
}

impl AmountDisplay {
    fn update(&mut self, new_amount: u16) {
        if self.string.is_none() || self.amount != new_amount {
            self.string = Some(new_amount.to_string());
            self.amount = new_amount;
        }
    }
}

struct ItemBoxHandler<P> {
    item_path: P,
    source: ItemSource,
    /// Grid index for inventory reorder (row-major). Unused for
    /// equipment/storage.
    display_slot: usize,
}

impl<P> ItemBoxHandler<P> {
    fn new(item_path: P, source: ItemSource, display_slot: usize) -> Self {
        Self {
            item_path,
            source,
            display_slot,
        }
    }
}

impl<P> ClickHandler<ClientState> for ItemBoxHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        // Unwrapping here is fine since we only register the handler if the slot has a
        // item.
        let item = state.try_get(&self.item_path).unwrap().clone();

        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::Custom {
                mode: MouseInputMode::MoveItem { item, source: self.source },
            },
        });
    }
}

/// Double-click: equip from inventory, or unequip from the equipment window.
struct ItemBoxDoubleClickHandler<P> {
    item_path: P,
    source: ItemSource,
}

impl<P> ItemBoxDoubleClickHandler<P> {
    fn new(item_path: P, source: ItemSource) -> Self {
        Self { item_path, source }
    }
}

/// Right-click inventory items: open the item actions popup (use/equip/drop).
struct ItemBoxRightClickHandler<P> {
    item_path: P,
    source: ItemSource,
}

impl<P> ItemBoxRightClickHandler<P> {
    fn new(item_path: P, source: ItemSource) -> Self {
        Self { item_path, source }
    }
}

impl<P> ClickHandler<ClientState> for ItemBoxRightClickHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        if self.source != ItemSource::Inventory {
            return;
        }

        let Some(item) = state.try_get(&self.item_path).cloned() else {
            return;
        };

        queue.queue(InputEvent::OpenItemActions { item });
    }
}

impl<P> ClickHandler<ClientState> for ItemBoxDoubleClickHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        let Some(item) = state.try_get(&self.item_path).cloned() else {
            return;
        };

        match self.source {
            ItemSource::Inventory => {
                // Unidentified: one-click identify (magnifier must be in inventory).
                if !item.is_identified() {
                    queue.queue(InputEvent::IdentifyItem {
                        inventory_index: item.index,
                    });
                    return;
                }

                if let InventoryItemDetails::Regular { .. } = &item.details {
                    // Consumables / etc.: use item (magnifier opens identify list).
                    queue.queue(InputEvent::UseItem {
                        inventory_index: item.index,
                    });
                    return;
                }

                if let InventoryItemDetails::Equippable {
                    equip_position,
                    equipped_position,
                    ..
                } = &item.details
                {
                    if equipped_position.is_empty() {
                        // Equip to the item's first valid slot (drag still works for multi-slot).
                        queue.queue(InputEvent::MoveItem {
                            source: ItemSource::Inventory,
                            destination: ItemSource::Equipment { position: *equip_position },
                            item,
                        });
                    } else {
                        queue.queue(InputEvent::MoveItem {
                            source: ItemSource::Equipment {
                                position: *equipped_position,
                            },
                            destination: ItemSource::Inventory,
                            item,
                        });
                    }
                }
            }
            ItemSource::Equipment { position } => {
                queue.queue(InputEvent::MoveItem {
                    source: ItemSource::Equipment { position },
                    destination: ItemSource::Inventory,
                    item,
                });
            }
            ItemSource::Storage => {
                queue.queue(InputEvent::MoveItem {
                    source: ItemSource::Storage,
                    destination: ItemSource::Inventory,
                    item,
                });
            }
        }
    }
}

impl<P> DropHandler<ClientState> for ItemBoxHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_drop(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
        if let MouseMode::Custom {
            mode: MouseInputMode::MoveItem { source, item },
        } = mouse_mode
        {
            // Drag within inventory: reorder the local display grid (server indices
            // unchanged).
            if *source == ItemSource::Inventory && self.source == ItemSource::Inventory {
                queue.queue(InputEvent::ReorderInventory {
                    from_index: item.index,
                    to_slot: self.display_slot,
                });
                return;
            }

            queue.queue(InputEvent::MoveItem {
                source: *source,
                destination: self.source,
                item: item.clone(),
            });
        }
    }
}

pub struct ItemBox<A> {
    item_path: A,
    handler: ItemBoxHandler<A>,
    double_click_handler: ItemBoxDoubleClickHandler<A>,
    right_click_handler: ItemBoxRightClickHandler<A>,
    amount_display: AmountDisplay,
    /// Tooltip text rebuilt when the hovered item changes (M1-009 stats).
    tooltip_text: UnsafeCell<String>,
}

impl<A> ItemBox<A>
where
    A: Copy,
{
    /// This function is supposed to be called from a component macro
    /// and not intended to be called manually.
    #[inline(always)]
    pub fn component_new(item_path: A, source: ItemSource, display_slot: usize) -> Self {
        Self {
            item_path,
            handler: ItemBoxHandler::new(item_path, source, display_slot),
            double_click_handler: ItemBoxDoubleClickHandler::new(item_path, source),
            right_click_handler: ItemBoxRightClickHandler::new(item_path, source),
            amount_display: AmountDisplay::default(),
            tooltip_text: UnsafeCell::new(String::new()),
        }
    }
}

impl<A> Element<ClientState> for ItemBox<A>
where
    A: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let area = resolver.with_height(40.0);

            if let Some(item) = state.try_get(&self.item_path)
                && item.metadata.texture.as_ref().is_some()
            {
                // Regular items always show their count; Equippable is gear
                // (always 1, no number) except stackable ammo, which shows > 1.
                match &item.details {
                    InventoryItemDetails::Regular { amount, .. } => self.amount_display.update(*amount),
                    InventoryItemDetails::Equippable { amount, .. } if *amount > 1 => self.amount_display.update(*amount),
                    InventoryItemDetails::Equippable { .. } => {}
                }
            }

            Self::LayoutInfo { area }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let (is_hovered, background_color) = match layout.get_mouse_mode() {
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { .. },
            } => match layout_info.area.check().any_mouse_mode().run(layout) {
                true => {
                    // Since we are not in default mouse mode we need to mark the window as
                    // hovered.
                    layout.set_hovered();

                    (true, Color::rgb_u8(80, 180, 180))
                }
                false => (false, Color::rgb_u8(180, 180, 80)),
            },
            _ => match layout_info.area.check().run(layout) {
                true => (true, Color::rgb_u8(60, 60, 60)),
                false => (false, Color::rgb_u8(40, 40, 40)),
            },
        };

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(20.0),
            background_color,
            Color::rgba_u8(0, 0, 0, 100),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if is_hovered {
            layout.register_drop_handler(&self.handler);
        }

        if let Some(item) = state.try_get(&self.item_path) {
            // Hover tooltip: name + combat stats (M1-009), with compare vs
            // equipped gear when this stack is not currently worn.
            if is_hovered && (!item.metadata.name.is_empty() || item.item_id.0 != 0) {
                struct ItemBoxTooltip;
                let refinement = match &item.details {
                    InventoryItemDetails::Equippable { refinement_level, .. } => Some(*refinement_level),
                    _ => None,
                };
                let (equipped_stats, equipped_refine) = match &item.details {
                    InventoryItemDetails::Equippable {
                        equip_position,
                        equipped_position,
                        ..
                    } if equipped_position.is_empty() && !equip_position.is_empty() => {
                        let inventory_path = client_state().inventory();
                        let inventory = state.get(&inventory_path);
                        inventory
                            .items()
                            .iter()
                            .find_map(|other| {
                                let InventoryItemDetails::Equippable {
                                    equip_position: other_equip,
                                    equipped_position: other_worn,
                                    refinement_level,
                                    ..
                                } = &other.details
                                else {
                                    return None;
                                };
                                if other_worn.is_empty() {
                                    return None;
                                }
                                // Overlap: worn item occupies a slot this item could fill.
                                if other_worn.intersects(*equip_position) || other_equip.intersects(*equip_position) {
                                    let stats = item_stats(other.item_id.0)?;
                                    Some((stats, Some(*refinement_level)))
                                } else {
                                    None
                                }
                            })
                            .map(|(s, r)| (Some(s), r))
                            .unwrap_or((None, None))
                    }
                    _ => (None, None),
                };
                let text = item_tooltip_text(item.item_id.0, &item.metadata.name, refinement, equipped_stats, equipped_refine);
                // Same pattern as character-slot display strings: keep the
                // tooltip buffer on the element so the layout borrow is stable.
                unsafe {
                    *self.tooltip_text.get() = text;
                    layout.add_tooltip(self.tooltip_text.as_ref_unchecked().as_str(), ItemBoxTooltip.tooltip_id());
                }
            }

            if let Some(texture) = item.metadata.texture.as_ref() {
                let texture_size = layout_info.area.width.min(layout_info.area.height);
                let texture_area = Area {
                    left: layout_info.area.left + (layout_info.area.width - texture_size) / 2.0,
                    top: layout_info.area.top + (layout_info.area.height - texture_size) / 2.0,
                    width: texture_size,
                    height: texture_size,
                };

                layout.add_texture(texture_area, texture.clone(), Color::WHITE, false);

                if is_hovered {
                    // Drag to equip/unequip (or rearrange). Double-click for quick equip/unequip.
                    // Right-click opens drop/use actions for inventory items.
                    layout.register_click_handler(MouseButton::Left, &self.handler);
                    layout.register_click_handler(MouseButton::DoubleLeft, &self.double_click_handler);
                    if matches!(self.right_click_handler.source, ItemSource::Inventory) {
                        layout.register_click_handler(MouseButton::Right, &self.right_click_handler);
                    }
                }

                // Draw the stack count for Regular items and for stackable ammo
                // (Equippable with amount > 1). `amount_display.string` is only
                // set in those cases, so require it rather than unwrap.
                let show_amount = match item.details {
                    InventoryItemDetails::Regular { .. } => true,
                    InventoryItemDetails::Equippable { amount, .. } => amount > 1,
                };
                if show_amount && let Some(amount_string) = self.amount_display.string.as_ref() {
                    layout.add_text(
                        layout_info.area,
                        amount_string,
                        // TODO: Put this in the theme
                        FontSize(12.0),
                        // TODO: Put this in the theme
                        Color::rgb_u8(255, 200, 255),
                        // TODO: Put this in the theme
                        Color::rgb_u8(255, 160, 60),
                        // TODO: Put this in the theme
                        HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                        // TODO: Put this in the theme
                        VerticalAlignment::Bottom { offset: 3.0 },
                        OverflowBehavior::Shrink,
                    );
                }
            }
        }
    }
}
