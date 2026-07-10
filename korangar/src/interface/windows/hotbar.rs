use korangar_components::skill_box;
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::HotbarSlot;
use rust_state::{ArrayLookupExt, OptionExt, Path};

use crate::interface::resource::SkillSource;
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::localization::LocalizationPathExt;
use crate::state::skills::{LearnableSkill, LearnedSkill, LearnedSkillPath};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Key captions under each hotbar slot (matches F1–F10 input bindings).
const HOTBAR_KEY_LABELS: [&str; 10] = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10"];

pub struct HotbarWindow<A, B, const N: usize> {
    hotbar_path: A,
    skills_path: B,
}

impl<A, B, const N: usize> HotbarWindow<A, B, N> {
    pub fn new(hotbar_path: A, skills_path: B) -> Self {
        Self { hotbar_path, skills_path }
    }
}

impl<A, B, const N: usize> CustomWindow<ClientState> for HotbarWindow<A, B, N>
where
    A: Path<ClientState, [Option<LearnableSkill>; N]>,
    B: Path<ClientState, Vec<LearnedSkill>>,
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
                split! {
                    gaps: theme().window().gaps(),
                    children: std::array::from_fn::<_, N, _>(|slot| {
                        let learnable_skill_path = self.hotbar_path.array_index(slot).unwrapped();
                        let learned_skill_path = LearnedSkillPath::new(learnable_skill_path, self.skills_path);
                        let key_label = HOTBAR_KEY_LABELS.get(slot).copied().unwrap_or("?");

                        // Skill icon on top, bound key caption underneath (official RO style).
                        fragment! {
                            gaps: 2.0,
                            children: (
                                skill_box! {
                                    learnable_skill_path,
                                    learned_skill_path,
                                    source: SkillSource::Hotbar { slot: HotbarSlot(slot as u16) },
                                },
                                text! {
                                    text: key_label,
                                    height: 14.0,
                                    font_size: FontSize(11.0),
                                    horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                                    vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        }
                    }),
                },
            )
        }
    }
}
