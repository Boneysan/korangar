use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct CharacterOverviewWindow<A, B, C, D> {
    player_name_path: A,
    player_class_name_path: B,
    base_level_path: C,
    job_level_path: D,
}

impl<A, B, C, D> CharacterOverviewWindow<A, B, C, D> {
    pub fn new(player_name_path: A, player_class_name_path: B, base_level_path: C, job_level_path: D) -> Self {
        Self {
            player_name_path,
            player_class_name_path,
            base_level_path,
            job_level_path,
        }
    }
}

impl<A, B, C, D> CustomWindow<ClientState> for CharacterOverviewWindow<A, B, C, D>
where
    A: Path<ClientState, String>,
    B: Path<ClientState, String>,
    C: Path<ClientState, usize>,
    D: Path<ClientState, usize>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterOverview)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().character_overview_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            minimum_width: 300.0,
            maximum_width: 900.0,
            elements: (
                fragment! {
                    gaps: 4.0,
                    children: (
                        split! {
                            children: (
                                text! {
                                    text: client_state().localization().name_text(),
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: self.player_name_path,
                                    color: Color::rgb_u8(255, 144, 13),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        // The window showed two *levels* and no class, so nothing on
                        // screen said what the character actually was -- which cost a
                        // testing session when `@jobchange 15` (Monk) was mistaken for
                        // 16 (Sage) and the missing skills looked like a client bug.
                        split! {
                            children: (
                                text! {
                                    text: "Job",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: self.player_class_name_path,
                                    color: Color::rgb_u8(255, 200, 13),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        split! {
                            children: (
                                text! {
                                    text: client_state().localization().base_level_text(),
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: PartialEqDisplaySelector::new(self.base_level_path),
                                    color: Color::rgb_u8(13, 231, 255),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        split! {
                            children: (
                                text! {
                                    text: client_state().localization().job_level_text(),
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: PartialEqDisplaySelector::new(self.job_level_path),
                                    color: Color::rgb_u8(13, 231, 255),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                    ),
                },
                button! {
                    text: client_state().localization().inventory_button_text(),
                    event: InputEvent::ToggleInventoryWindow,
                },
                button! {
                    text: client_state().localization().equipment_button_text(),
                    event: InputEvent::ToggleEquipmentWindow,
                },
                button! {
                    text: client_state().localization().skill_tree_button_text(),
                    event: InputEvent::ToggleSkillTreeWindow,
                },
                button! {
                    text: client_state().localization().stats_button_text(),
                    event: InputEvent::ToggleStatsWindow,
                },
                button! {
                    text: client_state().localization().friend_list_button_text(),
                    event: InputEvent::ToggleFriendListWindow,
                },
                button! {
                    text: "Party",
                    event: InputEvent::TogglePartyWindow,
                },
                button! {
                    text: "HUD",
                    event: InputEvent::ToggleHudWindow,
                },
                button! {
                    text: client_state().localization().minimap_button_text(),
                    event: InputEvent::ToggleMinimapWindow,
                },
                button! {
                    text: client_state().localization().menu_button_text(),
                    event: InputEvent::ToggleMenuWindow,
                },
            ),
        }
    }
}
