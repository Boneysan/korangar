use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, State};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::renderer::LayoutExt;
use crate::state::character_creation::{CharacterCreation, CharacterCreationPathExt, CreationStat, STARTING_STATUS_POINTS};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// How tall the preview reserves, and how much the sprite is enlarged.
///
/// RO body sprites are small; drawn at 1:1 in a dialog the character is a
/// thumbnail, so it is scaled up to something worth looking at.
const PREVIEW_HEIGHT: f32 = 180.0;
const PREVIEW_SCALE: f32 = 2.0;

/// Draws the character being designed, inside the window.
///
/// The renderer has been able to draw a sprite in the interface all along --
/// `LayoutExt::add_sprite` and `CustomInstruction::Sprite` are both
/// implemented -- but nothing called it. The preview is submitted as one
/// composed animation so the normal world-sprite attachment rules keep the
/// body and head together for all eight facings.
struct CharacterPreview<A> {
    creation_path: A,
}

struct CharacterPreviewLayoutInfo {
    area: Area,
}

impl<A> Element<ClientState> for CharacterPreview<A>
where
    A: Path<ClientState, CharacterCreation>,
{
    type LayoutInfo = CharacterPreviewLayoutInfo;

    fn create_layout_info(
        &mut self,
        _state: &State<ClientState>,
        _store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| CharacterPreviewLayoutInfo {
            area: resolver.with_height(PREVIEW_HEIGHT),
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let creation = state.get(&self.creation_path);

        // Nothing to draw until the sprites arrive; the window simply reserves
        // the space so it does not jump when they do.
        let Some(animation_data) = creation.preview.as_ref() else {
            return;
        };

        // Motion zero matches the in-world idle compositor's Doridori
        // suppression. The renderer fits the complete frame into this area,
        // so scaling never separates or clips one layer independently.
        layout.add_animation(
            layout_info.area,
            animation_data,
            creation.preview_direction,
            Color::WHITE,
            PREVIEW_SCALE,
        );
    }
}

const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

pub struct CharacterCreationWindow<A, B> {
    character_name_path: A,
    creation_path: B,
    slot: usize,
}

impl<A, B> CharacterCreationWindow<A, B> {
    pub fn new(character_name_path: A, creation_path: B, slot: usize) -> Self {
        Self {
            character_name_path,
            creation_path,
            slot,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for CharacterCreationWindow<A, B>
where
    A: Path<ClientState, String>,
    B: Path<ClientState, CharacterCreation>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterCreation)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct CharacterName;

        let disabled = ComputedSelector::new_default(move |state: &ClientState| {
            self.character_name_path.follow_safe(state).len() < MINIMUM_NAME_LENGTH
        });

        // Static strings chosen by the selected class. The panel is advisory
        // only: stats cannot ride the creation packet (0x0A39 has no stat
        // fields, and for PACKETVER >= 20120307 the char server writes
        // 1,1,1,1,1,1 and 48 points regardless), so the player spends these
        // themselves in the stats window once in game.
        let recommended_stats = ComputedSelector::new_default(move |state: &ClientState| {
            self.creation_path.starting_class().follow_safe(state).recommended_stats().summary()
        });

        let recommendation_reason = ComputedSelector::new_default(move |state: &ClientState| {
            self.creation_path.starting_class().follow_safe(state).recommendation_reason()
        });

        // Derived rather than written out, so the sentence cannot drift from the
        // numbers above it.
        let recommendation_budget = ComputedSelector::new_default(move |state: &ClientState| {
            let cost = self.creation_path.starting_class().follow_safe(state).recommended_stats().cost();

            format!("Spends {cost} of your {STARTING_STATUS_POINTS} points.")
        });

        // Points not yet spent. Derived from the allocation rather than tracked
        // beside it, so the two cannot disagree.
        let points_remaining = ComputedSelector::new_default(move |state: &ClientState| {
            let remaining = self.creation_path.stats().follow_safe(state).remaining();

            format!("Points remaining: {remaining} of {STARTING_STATUS_POINTS}")
        });

        macro_rules! stat_row {
            ($stat:expr) => {
                split! {
                    children: (
                        text! {
                            text: $stat.label(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "-",
                            event: InputEvent::AdjustCreationStat { stat: $stat, raise: false },
                        },
                        text! {
                            text: ComputedSelector::new_default(move |state: &ClientState| {
                                self.creation_path.stats().follow_safe(state).get($stat).to_string()
                            }),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "+",
                            event: InputEvent::AdjustCreationStat { stat: $stat, raise: true },
                        }
                    )
                }
            };
        }

        let create_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&self.character_name_path).clone();
            let sex = *state.get(&self.creation_path.sex());
            let hair_style = *state.get(&self.creation_path.hair_style());

            queue.queue(InputEvent::CreateCharacter {
                slot: self.slot,
                name,
                sex,
                hair_style,
            });
        };

        window! {
            title: client_state().localization().create_character_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            closable: true,
            // The theme's default cap is sized for the short form this window
            // used to be. With sex, hair, the six stat rows and the help panel
            // it needs room, and anything past the cap is simply cut off rather
            // than scrolled.
            maximum_height: 900.0,
            // Two columns now: the character on the left, the form on the
            // right, so hair and sex can be judged against what they do.
            maximum_width: 900.0,
            elements: split! {
                children: (
                    fragment! {
                        children: (
                            CharacterPreview {
                                creation_path: self.creation_path,
                            },
                            split! {
                                children: (
                                    button! {
                                        text: "<",
                                        event: InputEvent::RotateCharacterPreview { clockwise: false },
                                    },
                                    text! {
                                        text: "Turn",
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                    button! {
                                        text: ">",
                                        event: InputEvent::RotateCharacterPreview { clockwise: true },
                                    }
                                )
                            }
                        )
                    },
                    fragment! {
                        children: (
                text_box! {
                    ghost_text: client_state().localization().character_name_text(),
                    state: self.character_name_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.character_name_path, create_action),
                    focus_id: CharacterName,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    children: (
                        text! {
                            text: "Sex",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        drop_down! {
                            selected: self.creation_path.sex(),
                            options: self.creation_path.sexes(),
                        }
                    )
                },
                // Arrows rather than a drop-down: with a live preview beside
                // it, hair is chosen by looking at the character, not by
                // picking a number out of a list of forty-two.
                //
                // Plain `<` and `>`: the interface font has no glyph for arrow
                // characters and draws nothing at all for them.
                split! {
                    children: (
                        text! {
                            text: "Hair style",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "<",
                            event: InputEvent::CycleHairStyle { forward: false },
                        },
                        text! {
                            text: ComputedSelector::new_default(move |state: &ClientState| {
                                self.creation_path.hair_style().follow_safe(state).id.to_string()
                            }),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: ">",
                            event: InputEvent::CycleHairStyle { forward: true },
                        }
                    )
                },
                text! {
                    text: points_remaining,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    children: (stat_row!(CreationStat::Strength), stat_row!(CreationStat::Intelligence))
                },
                split! {
                    children: (stat_row!(CreationStat::Agility), stat_row!(CreationStat::Dexterity))
                },
                split! {
                    children: (stat_row!(CreationStat::Vitality), stat_row!(CreationStat::Luck))
                },
                state_button! {
                    text: "Help me choose my stats",
                    state: self.creation_path.show_recommendation(),
                    event: Toggle(self.creation_path.show_recommendation()),
                },
                either! {
                    selector: self.creation_path.show_recommendation(),
                    on_true: (
                        split! {
                            children: (
                                text! {
                                    text: "First job",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                drop_down! {
                                    selected: self.creation_path.starting_class(),
                                    options: self.creation_path.starting_classes(),
                                },
                                button! {
                                    text: "Use this build",
                                    event: InputEvent::UseRecommendedStats,
                                }
                            )
                        },
                        text! {
                            text: recommended_stats,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: recommendation_reason,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: recommendation_budget,
                            overflow_behavior: OverflowBehavior::Shrink,
                        }
                    ),
                    on_false: EmptyElement,
                },
                button! {
                    text: client_state().localization().create_character_button_text(),
                    disabled,
                    disabled_tooltip: client_state().localization().create_character_button_tooltip(),
                    event: create_action,
                }
                        )
                    }
                )
            },
        }
    }
}
