//! A fixed list of already-built widgets, for windows that assemble their
//! rows at layout time (quest cards, Guide source notes).

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_nth_resolver};
use rust_state::State;

use crate::state::ClientState;

/// A fixed list of already-built widgets, laid out top to bottom.
pub struct Rows {
    pub elements: Vec<ElementBox<ClientState>>,
}

impl Element<ClientState> for Rows {
    type LayoutInfo = ();

    fn get_element_count(&self, _: &State<ClientState>) -> usize {
        self.elements.len()
    }

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        for (index, element) in self.elements.iter_mut().enumerate() {
            with_nth_resolver(resolvers, index, |resolver| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (index, element) in self.elements.iter().enumerate() {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        }
    }
}
