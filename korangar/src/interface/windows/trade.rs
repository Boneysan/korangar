use korangar_interface::MouseMode;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, StateElement};
use korangar_interface::event::{DropHandler, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;
use rust_state::{Path, RustState, State};

use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::ItemSource;
use crate::interface::windows::WindowClass;
use crate::interface::windows::item_actions::inventory_item_amount;
use crate::state::theme::{GlobalThemePathExt, InterfaceThemePathExt, InterfaceThemeType};
use crate::state::trade::{TradeState, TradeStatePathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};
use crate::world::ResourceMetadata;

/// Active trade window (after accept / when we initiated and partner accepted).
/// Zeny amounts top out well below `u32::MAX`; ten digits is plenty and stops
/// a paste from overflowing the parse.
const MAXIMUM_ZENY_DIGITS: usize = 10;

/// ZST for the zeny field's focus id.
pub struct TradeZenyTextBox;

/// Internal state of the trade window.
#[derive(Default, RustState, StateElement)]
pub struct TradeWindowState {
    zeny_input: String,
}

pub struct TradeWindow<A, B> {
    window_state_path: A,
    trade_path: B,
}

impl<A, B> TradeWindow<A, B> {
    pub fn new(window_state_path: A, trade_path: B) -> Self {
        Self {
            window_state_path,
            trade_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for TradeWindow<A, B>
where
    A: Path<ClientState, TradeWindowState> + Copy + 'static,
    B: Path<ClientState, TradeState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Trade)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let text_path = self.trade_path.display_text();
        let zeny_path = self.window_state_path.zeny_input();

        // Non-numeric input is ignored rather than reported: the field is free
        // text and a stray character should not cost the player a trade.
        let add_zeny = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let Ok(amount) = state.get(&zeny_path).trim().parse::<u32>() else {
                return;
            };
            if amount > 0 {
                state.update_value_with(zeny_path, |input| input.clear());
                queue.queue(InputEvent::TradeAddZeny { amount });
                queue.queue(Event::Unfocus);
            }
        };

        window! {
            title: "Trade",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // **Deliberately not closable, and this one bricks trading for the
            // rest of the session if it is.** `trade.c:185` sets
            // `sd->state.trading = 1` on *both* players when a trade starts, and
            // only `CZ_CANCEL_EXCHANGE_ITEM` clears it. The framework's close
            // button sends nothing, so closing this window left the flag set --
            // whereupon `clif_parse_TradeRequest` (`clif.c`) returns on
            // `sd->state.trading` **before generating any response at all**. Every
            // later trade attempt, in either direction, silently does nothing:
            // no packet back, so nothing for the client to report. Cancel is the
            // way out, and it tells the server. `TradeCancelled` and
            // `TradeCompleted` still close this window, so it cannot strand.
            closable: false,
            elements: (
                text! { text: text_path },
                TradeDropArea::new(text! {
                    text: "Drag an inventory item here, or right-click it, to add it.",
                }),
                text_box! {
                    ghost_text: "Zeny to offer",
                    state: zeny_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_ZENY_DIGITS>::new(zeny_path, add_zeny),
                    focus_id: TradeZenyTextBox,
                },
                button! {
                    text: "Add zeny",
                    tooltip: "Put the amount above into the trade",
                    event: add_zeny,
                },
                button! {
                    text: "+100 zeny",
                    event: InputEvent::TradeAdjustZeny { delta: 100 },
                },
                button! {
                    text: "+1,000 zeny",
                    event: InputEvent::TradeAdjustZeny { delta: 1_000 },
                },
                button! {
                    text: "+10,000 zeny",
                    event: InputEvent::TradeAdjustZeny { delta: 10_000 },
                },
                button! {
                    text: "−100 zeny",
                    event: InputEvent::TradeAdjustZeny { delta: -100 },
                },
                button! {
                    text: "−1,000 zeny",
                    event: InputEvent::TradeAdjustZeny { delta: -1_000 },
                },
                button! {
                    text: "−10,000 zeny",
                    event: InputEvent::TradeAdjustZeny { delta: -10_000 },
                },
                button! {
                    text: "Lock offer",
                    event: InputEvent::TradeOk,
                },
                button! {
                    text: "Confirm trade",
                    event: InputEvent::TradeCommit,
                },
                button! {
                    text: "Cancel",
                    event: InputEvent::TradeCancel,
                },
            )
        }
    }
}

/// The trade add a drag produces: only an inventory item, and its whole stack,
/// matching the right-click menu's "trade all". Equipment, storage and hotbar
/// drags are not offers -- worn gear must come off through the inventory first,
/// and a hotbar slot is a shortcut, not an item.
fn trade_add_for_drag(source: ItemSource, item: &InventoryItem<ResourceMetadata>) -> Option<InputEvent> {
    match source {
        ItemSource::Inventory => Some(InputEvent::TradeAddItem {
            inventory_index: item.index,
            amount: u32::from(inventory_item_amount(item)),
        }),
        ItemSource::Equipment { .. } | ItemSource::Storage | ItemSource::Hotbar { .. } => None,
    }
}

/// Drop target around part of the trade window (GDD F28 item-grid drag). It
/// only highlights and accepts while an inventory item is being dragged, so a
/// drag that cannot become an offer never looks droppable.
struct TradeDropArea<Children> {
    children: Children,
}

impl<Children> TradeDropArea<Children> {
    fn new(children: Children) -> Self {
        Self { children }
    }
}

impl<Children> DropHandler<ClientState> for TradeDropArea<Children> {
    fn handle_drop(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
        if let MouseMode::Custom {
            mode: MouseInputMode::MoveItem { source, item },
        } = mouse_mode
            && let Some(event) = trade_add_for_drag(*source, item)
        {
            queue.queue(event);
        }
    }
}

impl<Children> Element<ClientState> for TradeDropArea<Children>
where
    Children: Element<ClientState>,
{
    type LayoutInfo = (Area, Children::LayoutInfo);

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            resolver.with_derived_unchanged(|resolver| self.children.create_layout_info(state, store, resolver))
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        use korangar_interface::prelude::*;

        if let MouseMode::Custom {
            mode: MouseInputMode::MoveItem { source, item },
        } = layout.get_mouse_mode()
            && trade_add_for_drag(*source, item).is_some()
        {
            let is_hovered = layout_info.0.check().any_mouse_mode().run(layout);
            let color = match is_hovered {
                true => *state.get(&client_theme().global().hovered_drop_area_color()),
                false => *state.get(&client_theme().global().drop_area_color()),
            };

            layout.add_rectangle(
                layout_info.0,
                *state.get(&client_theme().window().corner_diameter()),
                color.multiply_alpha(*state.get(&client_theme().global().fill_alpha())),
                color,
                *state.get(&client_theme().global().drop_area_outline()),
            );

            if is_hovered {
                // Not in default mouse mode, so the window has to be marked hovered.
                layout.set_hovered();

                layout.register_drop_handler(self);
            }
        }

        self.children.lay_out(state, store, &layout_info.1, layout);
    }
}

/// Incoming trade request accept/reject.
///
/// Names the requester and their level, both of which ride on
/// `ZC_REQ_EXCHANGE_ITEM` and were already being stored in `TradeState` -- this
/// window simply used to ignore them and say "A player wants to trade with
/// you." No protocol work was needed, unlike the party invite, whose sender
/// name genuinely is not on the wire.
pub struct TradeRequestWindow;

impl CustomWindow<ClientState> for TradeRequestWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::TradeRequest)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Trade request",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // **Deliberately not closable: a trade request must be answered.**
            // The framework's close button only closes the window -- there is no
            // close hook to hang a packet on -- so dismissing it sent no reply
            // while Hercules had already set `trade_partner` on *both* sides
            // (`trade.c` `trade_request`). The pair stayed locked with nothing on
            // screen to say so, and the client's own `pending` state was never
            // cleared either. Reject is the close button, and it tells the server.
            //
            // This cannot strand the popup: `TradeCancelled` (`ZC_CANCEL_EXCHANGE_ITEM`)
            // closes it, which is what arrives if the requester cancels or logs out.
            closable: false,
            elements: (
                text! { text: client_state().trade_state().request_text() },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Whisper",
                            tooltip: "Ask what they want before deciding",
                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                let character_name = state.get(&client_state().trade_state()).pending_name().to_owned();
                                if !character_name.is_empty() {
                                    queue.queue(InputEvent::StartWhisper { character_name });
                                }
                            },
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Reject",
                            event: InputEvent::TradeReject,
                        },
                        button! {
                            text: "Accept",
                            event: InputEvent::TradeAccept,
                        },
                    ),
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use korangar_networking::{InventoryItem, InventoryItemDetails};
    use ragnarok_packets::{EquipPosition, InventoryIndex, ItemId, RegularItemFlags};

    use super::trade_add_for_drag;
    use crate::input::InputEvent;
    use crate::interface::resource::ItemSource;
    use crate::world::ResourceMetadata;

    fn stack(index: u16, amount: u16) -> InventoryItem<ResourceMetadata> {
        InventoryItem {
            metadata: ResourceMetadata {
                texture: None,
                name: "Red Potion".to_owned(),
            },
            index: InventoryIndex(index),
            item_id: ItemId(501),
            item_type: 0,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Regular {
                amount,
                equipped_position: EquipPosition::NONE,
                flags: RegularItemFlags::empty(),
            },
        }
    }

    #[test]
    fn inventory_drag_offers_the_dragged_index_and_its_whole_stack() {
        let event = trade_add_for_drag(ItemSource::Inventory, &stack(7, 25));

        assert!(matches!(
            event,
            Some(InputEvent::TradeAddItem {
                inventory_index: InventoryIndex(7),
                amount: 25,
            })
        ));
    }

    #[test]
    fn drags_from_anywhere_but_the_inventory_are_not_offers() {
        let item = stack(7, 25);

        assert!(trade_add_for_drag(ItemSource::Storage, &item).is_none());
        assert!(
            trade_add_for_drag(
                ItemSource::Equipment {
                    position: EquipPosition::NONE
                },
                &item
            )
            .is_none()
        );
    }
}
