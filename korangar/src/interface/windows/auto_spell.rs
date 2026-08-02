use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::SkillId;
use rust_state::{ManuallyAssertExt, Path, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

/// One button per skill Auto Spell is offering.
struct AutoSpellList<A> {
    skills_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> AutoSpellList<A> {
    fn new(skills_path: A) -> Self {
        Self {
            skills_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for AutoSpellList<A>
where
    A: Path<ClientState, Vec<SkillId>>,
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

            let skills = state.get(&self.skills_path);
            self.elements.truncate(skills.len());

            for index in self.elements.len()..skills.len() {
                let skill_path = self.skills_path.index(index).manually_asserted();

                self.elements.push(ErasedElement::new(button! {
                    // Skill *names* need the skill table, which this window does
                    // not hold; the id is at least unambiguous and the icons in
                    // the skill tree carry the naming.
                    text: format!("Skill {}", state.get(&skill_path).0),
                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        let skill_id = *state.get(&skill_path);
                        queue.queue(InputEvent::SelectAutoSpell { skill_id });
                    },
                }));
            }

            self.elements
                .iter_mut()
                .enumerate()
                .for_each(|(index, element)| element.create_layout_info(state, store.child_store(index as u64), resolver));
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements
            .iter()
            .enumerate()
            .for_each(|(index, element)| element.lay_out(state, store.child_store(index as u64), &(), layout));
    }
}

/// Auto Spell's skill chooser.
///
/// `ZC_AUTOSPELLLIST` was unmodelled, so casting Auto Spell as a Sage did
/// nothing observable at all — the server was waiting for a choice the client
/// had no way to make.
pub struct AutoSpellWindow<A> {
    skills_path: A,
}

impl<A> AutoSpellWindow<A> {
    pub fn new(skills_path: A) -> Self {
        Self { skills_path }
    }
}

impl<A> CustomWindow<ClientState> for AutoSpellWindow<A>
where
    A: Path<ClientState, Vec<SkillId>> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::AutoSpell)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Auto Spell",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: "Choose a spell to cast automatically:",
                },
                AutoSpellList::new(self.skills_path),
            )
        }
    }
}
