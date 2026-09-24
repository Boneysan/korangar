use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::settings::{BindableAction, GameSettings, GameSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::skills::SkillTreePathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

struct KeyBindingList<A> {
    game_settings_path: A,
    elements: Vec<ElementBox<ClientState>>,
    labels: Vec<String>,
}

struct GroundSkillModeList<A> {
    game_settings_path: A,
    elements: Vec<ElementBox<ClientState>>,
    entries: Vec<(u16, String)>,
}

impl<A> Element<ClientState> for GroundSkillModeList<A>
where
    A: Path<ClientState, GameSettings> + Copy + 'static,
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
            let settings = state.get(&self.game_settings_path);
            let mut entries = state
                .get(&client_state().skill_tree().skills())
                .iter()
                .filter(|skill| {
                    matches!(
                        skill.skill_type,
                        ragnarok_packets::SkillType::Ground | ragnarok_packets::SkillType::Trap
                    )
                })
                .map(|skill| {
                    let mode = settings.effective_ground_skill_target_mode(skill.skill_id.0);
                    let source = if settings.ground_skill_target_modes.contains_key(&skill.skill_id.0) {
                        "override"
                    } else {
                        "global"
                    };
                    (skill.skill_id.0, format!("{} — {} ({source})", skill.skill_name, mode.label()))
                })
                .collect::<Vec<_>>();
            entries.sort_by_key(|(skill_id, _)| *skill_id);
            if entries != self.entries {
                self.elements.clear();
                if entries.is_empty() {
                    self.elements
                        .push(ErasedElement::new(text! { text: "No learned ground/trap skills." }));
                } else {
                    for (skill_id, label) in &entries {
                        self.elements.push(ErasedElement::new(button! {
                            text: label.clone(),
                            tooltip: "Click to cycle this skill through Aim + click, Quickcast, Hold + release, then inherit global settings.",
                            event: InputEvent::CycleGroundSkillTargetMode(*skill_id),
                        }));
                    }
                }
                self.entries = std::mem::take(&mut entries);
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

impl<A> Element<ClientState> for KeyBindingList<A>
where
    A: Path<ClientState, GameSettings> + Copy + 'static,
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
            let settings = state.get(&self.game_settings_path);
            let actions = BindableAction::all();
            let labels = actions
                .iter()
                .map(|action| {
                    if settings.pending_key_binding == Some(*action) {
                        format!("{} — press chord (Escape cancels)", action.label())
                    } else {
                        format!("{} — {}", action.label(), settings.key_bindings.chord(*action).display())
                    }
                })
                .collect::<Vec<_>>();
            if labels != self.labels {
                self.elements.clear();
                for (action, label) in actions.iter().copied().zip(labels.iter().cloned()) {
                    self.elements.push(ErasedElement::new(button! {
                        text: label,
                        event: InputEvent::BeginKeyBindingCapture(action),
                    }));
                }
                self.labels = labels;
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

#[derive(Default)]
pub struct GameSettingsWindow<A> {
    game_settings_path: A,
}

impl<A> GameSettingsWindow<A> {
    pub fn new(game_settings_path: A) -> Self {
        Self { game_settings_path }
    }
}

impl<A> CustomWindow<ClientState> for GameSettingsWindow<A>
where
    A: Path<ClientState, GameSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::GameSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().game_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                state_button! {
                    text: client_state().localization().auto_attack_button_text(),
                    state: self.game_settings_path.auto_attack(),
                    event: Toggle(self.game_settings_path.auto_attack()),
                },
                state_button! {
                    text: client_state().localization().show_minimap_button_text(),
                    state: self.game_settings_path.show_minimap(),
                    event: InputEvent::ToggleMinimapWindow,
                },
                state_button! {
                    text: "Warn before high-level maps",
                    tooltip: "Show a non-blocking toast when the map's static-spawn mean is at least 15 levels above your character. This never prevents travel.",
                    state: self.game_settings_path.warn_dangerous_maps(),
                    event: Toggle(self.game_settings_path.warn_dangerous_maps()),
                },
                state_button! {
                    text: "WASD movement",
                    tooltip: "Walk with W A S D relative to the camera. Click-to-move still works.",
                    state: self.game_settings_path.wasd_movement(),
                    event: Toggle(self.game_settings_path.wasd_movement()),
                },
                state_button! {
                    text: "Reduce motion",
                    tooltip: "Disable nonessential camera shake. Combat cues and telegraphs remain visible.",
                    state: self.game_settings_path.reduce_motion(),
                    event: Toggle(self.game_settings_path.reduce_motion()),
                },
                state_button! {
                    text: "Reduce flashing",
                    tooltip: "Dim procedural combat bursts and their point lights. Skill timing, telegraphs, and sounds remain unchanged.",
                    state: self.game_settings_path.reduce_flashing(),
                    event: Toggle(self.game_settings_path.reduce_flashing()),
                },
                state_button! {
                    text: "Quickcast ground skills at cursor",
                    tooltip: "Global default for ground/trap skills unless a learned skill has an override. When enabled, selecting a skill casts at the current map cell; without a valid cursor target it falls back to aim-and-click.",
                    state: self.game_settings_path.quickcast_ground_skills(),
                    event: Toggle(self.game_settings_path.quickcast_ground_skills()),
                },
                state_button! {
                    text: "Hold ground skill key to aim, release to cast",
                    tooltip: "Global default for ground/trap skills unless a learned skill has an override. Hold its hotbar key while aiming, then release over a valid target; this takes precedence over the global quickcast option.",
                    state: self.game_settings_path.hold_aim_release_ground_skills(),
                    event: Toggle(self.game_settings_path.hold_aim_release_ground_skills()),
                },
                text! { text: "Per-skill ground/trap targeting — click a learned skill to cycle its override." },
                scroll_view! {
                    children: GroundSkillModeList {
                        game_settings_path: self.game_settings_path,
                        elements: Vec::new(),
                        entries: Vec::new(),
                    }
                },
                state_button! {
                    text: "Show quest markers",
                    tooltip: "Show or hide quest indicators on NPCs and objective locations.",
                    state: self.game_settings_path.show_quest_markers(),
                    event: Toggle(self.game_settings_path.show_quest_markers()),
                },
                state_button! {
                    text: "Show combat text",
                    tooltip: "Show floating damage, miss, and healing numbers. This does not change combat results or sound cues.",
                    state: self.game_settings_path.show_combat_text(),
                    event: Toggle(self.game_settings_path.show_combat_text()),
                },
                button! { text: "Cycle combat text detail (all / important / status only)", tooltip: "All shows every number; Important keeps critical hits, misses, and healing; Status only hides floating numbers while leaving textual status notices visible.", event: InputEvent::CycleCombatTextFrequency },
                button! { text: "Cycle combat text size (small / normal / large)", tooltip: "Adjusts floating combat text size independently from the overall interface scale.", event: InputEvent::CycleCombatTextSize },
                text! { text: "Keyboard shortcuts — select an action, then press its replacement chord. Escape cancels." },
                scroll_view! { children: KeyBindingList { game_settings_path: self.game_settings_path, elements: Vec::new(), labels: Vec::new() } },
                button! { text: "Reset keyboard shortcuts", event: InputEvent::ResetKeyBindings },
                button! { text: "Import shortcuts", tooltip: "Read and validate client/keybindings.ron", event: InputEvent::ImportKeyBindings },
                button! { text: "Export shortcuts", tooltip: "Write client/keybindings.ron", event: InputEvent::ExportKeyBindings },
                text! { text: "HUD layout" },
                button! { text: "Lock / unlock HUD editing", tooltip: "Locks window movement and resizing.", event: InputEvent::ToggleHudEditLock },
                button! { text: "Classic layout", event: InputEvent::SelectHudLayout("Classic") },
                button! { text: "Modern layout", event: InputEvent::SelectHudLayout("Modern") },
                button! { text: "Save current as My Layout", tooltip: "Overwrites the per-character My Layout slot.", event: InputEvent::SaveHudLayout("My Layout") },
                button! { text: "Reset HUD layout", event: InputEvent::ResetHudLayout },
                button! { text: "Cycle HUD snap grid (off / 8 / 16 / 32 px)", tooltip: "When enabled, dragged windows snap their positions to the selected screen-pixel grid.", event: InputEvent::CycleHudSnapGrid },
            ),
        }
    }
}
