use std::cmp::Ordering;

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element, ElementBox};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, State, VecIndexExt};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::party::{PartyMemberState, PartyMemberStatePathExt, PartyState, PartyStatePathExt};
use crate::state::theme::InterfaceThemeType;

/// One clickable row per party member. A click makes that member the support
/// target and nothing else, so a healer can switch targets mid-fight without a
/// target frame opening over the screen.
struct HealerRows<A> {
    members_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> Element<ClientState> for HealerRows<A>
where
    A: Path<ClientState, Vec<PartyMemberState>>,
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

            let count = state.get(&self.members_path).len();
            match count.cmp(&self.elements.len()) {
                Ordering::Less => self.elements.truncate(count),
                Ordering::Equal => {}
                Ordering::Greater => {
                    for index in self.elements.len()..count {
                        let member_path = self.members_path.index(index).manually_asserted();
                        self.elements.push(ErasedElement::new(fragment! {
                            gaps: 2.0,
                            children: (
                                button! {
                                    text: member_path.healer_label(),
                                    tooltip: "Make this member your support target",
                                    overflow_behavior: OverflowBehavior::LineBreak,
                                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        let member = state.get(&member_path);
                                        queue.queue(InputEvent::SelectSupportTarget {
                                            account_id: member.account_id(),
                                            character_name: member.name().to_owned(),
                                        });
                                    },
                                },
                                VitalBars { member_path },
                            ),
                        }));
                    }
                }
            }

            for (index, element) in self.elements.iter_mut().enumerate() {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            }
        });
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

const HP_BAR_HEIGHT: f32 = 10.0;
const SP_BAR_HEIGHT: f32 = 4.0;
const BAR_GAP: f32 = 2.0;

/// HP colour by how much is left, so a glance finds who needs healing.
fn health_color(fraction: f32) -> Color {
    match fraction {
        fraction if fraction > 0.5 => Color::rgb_u8(70, 200, 90),
        fraction if fraction > 0.25 => Color::rgb_u8(230, 190, 60),
        _ => Color::rgb_u8(220, 70, 60),
    }
}

/// Filled HP bar with a thin SP bar under it. A member whose vitals are not
/// known yet (or who is offline) shows an empty track rather than a guess.
struct VitalBars<P> {
    member_path: P,
}

impl<P> VitalBars<P> {
    fn draw_bar<'a>(layout: &mut WindowLayout<'a, ClientState>, area: Area, fraction: Option<f32>, color: Color) {
        layout.add_rectangle(
            area,
            CornerDiameter::uniform(3.0),
            Color::rgba_u8(20, 20, 20, 200),
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );
        if let Some(fraction) = fraction.filter(|fraction| *fraction > 0.0) {
            let filled = Area {
                width: area.width * fraction.min(1.0),
                ..area
            };
            layout.add_rectangle(
                filled,
                CornerDiameter::uniform(3.0),
                color,
                Color::TRANSPARENT,
                ShadowPadding::uniform(0.0),
            );
        }
    }
}

impl<P> Element<ClientState> for VitalBars<P>
where
    P: Path<ClientState, PartyMemberState>,
{
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        _: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| BaseLayoutInfo {
            area: resolver.with_height(HP_BAR_HEIGHT + BAR_GAP + SP_BAR_HEIGHT),
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let member = state.get(&self.member_path);
        let shown = member.online();
        let fraction = |vital: Option<(usize, usize)>| vital.filter(|_| shown).map(|(current, maximum)| current as f32 / maximum as f32);
        let health = fraction(member.health());
        let area = layout_info.area;

        Self::draw_bar(
            layout,
            Area {
                height: HP_BAR_HEIGHT,
                ..area
            },
            health,
            health_color(health.unwrap_or(0.0)),
        );
        Self::draw_bar(
            layout,
            Area {
                top: area.top + HP_BAR_HEIGHT + BAR_GAP,
                height: SP_BAR_HEIGHT,
                ..area
            },
            fraction(member.spell()),
            Color::rgb_u8(80, 140, 255),
        );
    }
}

/// Party HP at a glance for healers, separate from the party management
/// window. `>` marks the current support target.
pub struct PartyHealerWindow<A> {
    party_path: A,
}

impl<A> PartyHealerWindow<A> {
    pub fn new(party_path: A) -> Self {
        Self { party_path }
    }
}

impl<A> CustomWindow<ClientState> for PartyHealerWindow<A>
where
    A: Path<ClientState, PartyState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::PartyHealer)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Party Healing",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 260.0,
            maximum_width: 600.0,
            elements: (
                HealerRows { members_path: self.party_path.members(), elements: Vec::new() },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::health_color;
    use crate::graphics::Color;

    #[test]
    fn hp_bar_turns_yellow_then_red_as_health_drops() {
        let green = health_color(1.0);
        assert_eq!(health_color(0.51), green);
        assert_eq!(health_color(0.5), Color::rgb_u8(230, 190, 60), "half is no longer healthy");
        assert_eq!(health_color(0.25), Color::rgb_u8(220, 70, 60), "a quarter is critical");
        assert_eq!(health_color(0.0), Color::rgb_u8(220, 70, 60));
    }
}
