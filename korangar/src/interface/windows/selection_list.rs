use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use rust_state::State;

use crate::input::InputEvent;
use crate::state::ClientState;

pub struct SelectionList {
    buttons: Vec<ElementBox<ClientState>>,
}

impl SelectionList {
    pub fn new(entries: impl IntoIterator<Item = (String, String, InputEvent)>) -> Self {
        use korangar_interface::prelude::*;

        let buttons = entries
            .into_iter()
            .map(|(text, tooltip, event)| ErasedElement::new(button! { text, tooltip, event }) as ElementBox<ClientState>)
            .collect();
        Self { buttons }
    }
}

impl Element<ClientState> for SelectionList {
    type LayoutInfo = Vec<()>;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let (_, info) = resolver.with_derived(2.0, 4.0, |resolver| {
                self.buttons
                    .iter_mut()
                    .enumerate()
                    .map(|(index, button)| button.create_layout_info(state, store.child_store(index as u64), resolver))
                    .collect()
            });
            info
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        layout.with_layer(|layout| {
            for (index, button) in self.buttons.iter().enumerate() {
                button.lay_out(state, store.child_store(index as u64), &layout_info[index], layout);
            }
        });
    }
}
