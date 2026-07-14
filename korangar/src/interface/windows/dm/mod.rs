//! DM campaign windows (Seal Cascade). Kept in their own module for
//! rebaseability — see `CLAUDE.md` rule 4 and `docs/DM_DATA_GUIDE.md`.

mod bestiary;
mod loot;

pub use bestiary::{BestiaryWindow, BestiaryWindowState};
pub use loot::{LootGeneratorWindow, LootWindowState};

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use rust_state::{ManuallyAssertExt, Path, State, VecIndexExt};

use crate::state::ClientState;

/// Dynamic block of plain text lines bound to a `Vec<String>` in state
/// (same rebuild-per-frame pattern as the friend list).
struct TextLines<A> {
    lines_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> TextLines<A> {
    fn new(lines_path: A) -> Self {
        Self {
            lines_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for TextLines<A>
where
    A: Path<ClientState, Vec<String>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;

            let line_count = state.get(&self.lines_path).len();

            if line_count < self.elements.len() {
                self.elements.truncate(line_count);
            } else {
                for index in self.elements.len()..line_count {
                    let line_path = self.lines_path.index(index).manually_asserted();
                    self.elements.push(ErasedElement::new(text! {
                        text: line_path,
                        overflow_behavior: crate::loaders::OverflowBehavior::Shrink,
                    }));
                }
            }

            self.elements.iter_mut().enumerate().for_each(|(index, element)| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}
