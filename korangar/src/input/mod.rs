mod event;
mod key;
mod mode;

use std::mem::variant_count;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ragnarok_packets::{ClientTick, HotbarSlot};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode;

pub use self::event::InputEvent;
pub use self::key::Key;
pub use self::mode::{Grabbed, MouseInputMode, MouseModeExt};
use crate::graphics::{PickerTarget, ScreenPosition, ScreenSize};
use crate::settings::{BindableAction, KeyBindings};

const MOUSE_SCOLL_MULTIPLIER: f32 = 30.0;
const KEY_COUNT: usize = variant_count::<KeyCode>();
const DOUBLE_CLICK_TIME_MS: u32 = 250;

#[derive(Debug, Clone, Copy)]
struct PreviousMouseButton {
    button: MouseButton,
    tick: ClientTick,
}

// TODO: Rename
pub struct InputReport {
    pub mouse_click: Option<korangar_interface::layout::MouseButton>,
    pub mouse_position: ScreenPosition,
    pub mouse_delta: ScreenSize,
    pub mouse_button_released: bool,
    pub left_mouse_button_down: bool,
    pub scroll: Option<f32>,
    pub drag: Option<ScreenSize>,
    pub characters: Vec<char>,
    pub mouse_target: PickerTarget,
}

pub struct InputSystem {
    previous_mouse_position: ScreenPosition,
    new_mouse_position: ScreenPosition,
    mouse_delta: ScreenSize,
    previous_scroll_position: f32,
    new_scroll_position: f32,
    scroll_delta: f32,
    left_mouse_button: Key,
    right_mouse_button: Key,
    keys: [Key; KEY_COUNT],
    known_key_codes: Vec<KeyCode>,
    input_buffer: Vec<char>,
    picker_value: Arc<AtomicU64>,
    previous_mouse_button: Option<PreviousMouseButton>,
}

impl InputSystem {
    pub fn new(picker_value: Arc<AtomicU64>) -> Self {
        let previous_mouse_position = ScreenPosition::default();
        let new_mouse_position = ScreenPosition::default();
        let mouse_delta = ScreenSize::default();

        let previous_scroll_position = 0.0;
        let new_scroll_position = 0.0;
        let scroll_delta = 0.0;

        let left_mouse_button = Key::default();
        let right_mouse_button = Key::default();
        let keys = [Key::default(); KEY_COUNT];
        let known_key_codes = Vec::new();

        let input_buffer = Vec::new();
        let previous_mouse_button = None;

        Self {
            previous_mouse_position,
            new_mouse_position,
            mouse_delta,
            previous_scroll_position,
            new_scroll_position,
            scroll_delta,
            left_mouse_button,
            right_mouse_button,
            keys,
            known_key_codes,
            input_buffer,
            picker_value,
            previous_mouse_button,
        }
    }

    pub fn reset(&mut self) {
        self.left_mouse_button.reset();
        self.right_mouse_button.reset();
        self.keys.iter_mut().for_each(|key| key.reset());
    }

    pub fn update_mouse_position(&mut self, position: PhysicalPosition<f64>) {
        self.new_mouse_position = ScreenPosition {
            left: position.x as f32,
            top: position.y as f32,
        };
    }

    pub fn update_mouse_buttons(&mut self, button: MouseButton, state: ElementState) {
        let pressed = matches!(state, ElementState::Pressed);

        match button {
            MouseButton::Left => self.left_mouse_button.set_down(pressed),
            MouseButton::Right => self.right_mouse_button.set_down(pressed),
            _ignored => {}
        }
    }

    pub fn update_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_x, y) => self.new_scroll_position += y * MOUSE_SCOLL_MULTIPLIER,
            MouseScrollDelta::PixelDelta(position) => self.new_scroll_position += position.y as f32,
        }
    }

    pub fn buffer_character(&mut self, character: char) {
        self.input_buffer.push(character);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("update input system"))]
    pub fn update_delta(&mut self, client_tick: ClientTick) -> InputReport {
        self.mouse_delta = self.new_mouse_position - self.previous_mouse_position;
        self.previous_mouse_position = self.new_mouse_position;

        self.scroll_delta = self.new_scroll_position - self.previous_scroll_position;
        self.previous_scroll_position = self.new_scroll_position;

        self.left_mouse_button.update();
        self.right_mouse_button.update();
        self.keys.iter_mut().for_each(|key| key.update());

        let mouse_button_released = self.left_mouse_button.released() || self.right_mouse_button.released();

        let last_pixel_value = self.picker_value.load(Ordering::Acquire);
        let mouse_target = PickerTarget::from(last_pixel_value);

        let mut mouse_click = None;

        if self.left_mouse_button.pressed() {
            if let Some(previous_mouse_button) = self.previous_mouse_button
                && previous_mouse_button.button == MouseButton::Left
                && client_tick.0.wrapping_sub(previous_mouse_button.tick.0) <= DOUBLE_CLICK_TIME_MS
            {
                self.previous_mouse_button = None;

                mouse_click = Some(korangar_interface::layout::MouseButton::DoubleLeft);
            } else {
                self.previous_mouse_button = Some(PreviousMouseButton {
                    button: MouseButton::Left,
                    tick: client_tick,
                });

                mouse_click = Some(korangar_interface::layout::MouseButton::Left);
            }
        } else if self.right_mouse_button.pressed() {
            if let Some(previous_mouse_button) = self.previous_mouse_button
                && previous_mouse_button.button == MouseButton::Right
                && client_tick.0.wrapping_sub(previous_mouse_button.tick.0) <= DOUBLE_CLICK_TIME_MS
            {
                self.previous_mouse_button = None;

                mouse_click = Some(korangar_interface::layout::MouseButton::DoubleRight);
            } else {
                self.previous_mouse_button = Some(PreviousMouseButton {
                    button: MouseButton::Right,
                    tick: client_tick,
                });

                mouse_click = Some(korangar_interface::layout::MouseButton::Right);
            }
        }

        InputReport {
            mouse_click,
            mouse_position: self.new_mouse_position,
            mouse_delta: self.mouse_delta,
            mouse_button_released,
            left_mouse_button_down: self.left_mouse_button.down(),
            scroll: (self.scroll_delta != 0.0).then_some(self.scroll_delta),
            drag: self.left_mouse_button.down().then_some(self.mouse_delta),
            characters: self.input_buffer.drain(..).collect(),
            mouse_target,
        }
    }

    fn get_key(&self, key_code: KeyCode) -> &Key {
        &self.keys[key_code as usize]
    }

    /// Mark a physical (or synthesised logical) key as down/up. Safe no-op for
    /// keys that fall outside the KeyCode discriminant range used as array
    /// index.
    pub fn update_keyboard(&mut self, key_code: KeyCode, state: ElementState) {
        let index = key_code as usize;
        if index < self.keys.len() {
            if !self.known_key_codes.contains(&key_code) {
                self.known_key_codes.push(key_code);
            }
            let pressed = matches!(state, ElementState::Pressed);
            self.keys[index].set_down(pressed);
        }
    }

    fn binding_pressed(&self, bindings: &KeyBindings, action: BindableAction, allow_shift: bool) -> bool {
        self.binding_key_down(bindings, action, allow_shift, true)
    }

    fn binding_released(&self, bindings: &KeyBindings, action: BindableAction) -> bool {
        let chord = bindings.chord(action);
        let Some(key_code) = self
            .known_key_codes
            .iter()
            .copied()
            .find(|key_code| format!("{key_code:?}") == chord.key)
        else {
            return false;
        };
        let control_down = self.get_key(KeyCode::ControlLeft).down() || self.get_key(KeyCode::ControlRight).down();
        let alt_down = self.get_key(KeyCode::AltLeft).down() || self.get_key(KeyCode::AltRight).down();
        let shift_down = self.get_key(KeyCode::ShiftLeft).down() || self.get_key(KeyCode::ShiftRight).down();
        self.get_key(key_code).released() && control_down == chord.control && alt_down == chord.alt && shift_down == chord.shift
    }

    fn binding_down(&self, bindings: &KeyBindings, action: BindableAction) -> bool {
        self.binding_key_down(bindings, action, false, false)
    }

    fn binding_key_down(&self, bindings: &KeyBindings, action: BindableAction, allow_shift: bool, pressed: bool) -> bool {
        let chord = bindings.chord(action);
        let Some(key_code) = self
            .known_key_codes
            .iter()
            .copied()
            .find(|key_code| format!("{key_code:?}") == chord.key)
        else {
            return false;
        };
        let control_down = self.get_key(KeyCode::ControlLeft).down() || self.get_key(KeyCode::ControlRight).down();
        let alt_down = self.get_key(KeyCode::AltLeft).down() || self.get_key(KeyCode::AltRight).down();
        let shift_down = self.get_key(KeyCode::ShiftLeft).down() || self.get_key(KeyCode::ShiftRight).down();
        let shift_matches = if allow_shift && !chord.shift {
            true
        } else {
            shift_down == chord.shift
        };
        let key_active = if pressed {
            self.get_key(key_code).pressed()
        } else {
            self.get_key(key_code).down()
        };
        key_active && control_down == chord.control && alt_down == chord.alt && shift_matches
    }

    /// Game-action keys that should work even while a text box or window has
    /// focus. Official RO: Insert = sit/stand; F1–F9 = hotbar.
    fn capture_keybinding(&self, events: &mut Vec<InputEvent>, pending: Option<BindableAction>) -> bool {
        let Some(action) = pending else {
            return false;
        };
        if self.get_key(KeyCode::Escape).pressed() {
            events.push(InputEvent::CancelKeyBindingCapture);
            return true;
        }
        let modifier_keys = [
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::AltLeft,
            KeyCode::AltRight,
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
        ];
        if let Some(key_code) = self
            .known_key_codes
            .iter()
            .copied()
            .find(|key_code| !modifier_keys.contains(key_code) && self.get_key(*key_code).pressed())
        {
            let control = self.get_key(KeyCode::ControlLeft).down() || self.get_key(KeyCode::ControlRight).down();
            let alt = self.get_key(KeyCode::AltLeft).down() || self.get_key(KeyCode::AltRight).down();
            let shift = self.get_key(KeyCode::ShiftLeft).down() || self.get_key(KeyCode::ShiftRight).down();
            events.push(InputEvent::CapturedKeyBinding {
                action,
                chord: crate::settings::KeyChord::new(format!("{key_code:?}"), control, alt, shift),
            });
        }
        true
    }

    pub fn handle_game_action_keys(
        &mut self,
        events: &mut Vec<InputEvent>,
        bindings: &KeyBindings,
        pending_capture: Option<BindableAction>,
    ) {
        if self.capture_keybinding(events, pending_capture) {
            return;
        }
        self.push_game_action_keys(events, bindings);
    }

    fn push_game_action_keys(&mut self, events: &mut Vec<InputEvent>, bindings: &KeyBindings) {
        // Official RO: Insert toggles sit / stand.
        // Also accept Home as a WSL/laptop-friendly fallback (Insert is often awkward).
        if self.binding_pressed(bindings, BindableAction::ToggleSit, false) || self.get_key(KeyCode::Home).pressed() {
            events.push(InputEvent::ToggleSit);
        }

        // GM/DM Commands panel (Ctrl+O) lives here rather than in
        // handle_keyboard_input so it works even while the chat box is focused —
        // a DM typically types an `@dm` command, then reaches for the panel.
        // Ctrl+O produces no printable character, so it does not leak into chat.
        let control_down = self.get_key(KeyCode::ControlLeft).down() || self.get_key(KeyCode::ControlRight).down();
        if self.binding_pressed(bindings, BindableAction::ToggleCommands, false) {
            events.push(InputEvent::ToggleCommandsWindow);
        }

        // Dice roller (Ctrl+D). Also in the always-works path so players can roll
        // while the chat box is focused (same rationale as Ctrl+O).
        if self.binding_pressed(bindings, BindableAction::ToggleDice, false) {
            events.push(InputEvent::ToggleDiceWindow);
        }

        // Quest log (Ctrl+Q). Same always-works path: checking what a contract
        // still wants is the sort of thing you do mid-conversation.
        if self.binding_pressed(bindings, BindableAction::OpenQuestLog, false) {
            events.push(InputEvent::ToggleQuestLogWindow);
        }

        if self.binding_pressed(bindings, BindableAction::SendDangerPing, false) {
            events.push(InputEvent::SendPartyPing { kind: "danger".to_owned() });
        }

        // Escape still closes a window while chat or another control is focused.
        if self.get_key(KeyCode::Escape).pressed() {
            events.push(InputEvent::Escape);
        }

        // F10 belongs to chat-window height in the original client, not the hotbar.
        const HOTBAR_KEYS: [KeyCode; 9] = [
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
            KeyCode::F6,
            KeyCode::F7,
            KeyCode::F8,
            KeyCode::F9,
        ];
        let alt_down = self.get_key(KeyCode::AltLeft).down() || self.get_key(KeyCode::AltRight).down();
        let row_offset = if alt_down {
            18
        } else if control_down {
            9
        } else {
            0
        };
        for (index, key) in HOTBAR_KEYS.into_iter().enumerate() {
            let slot = HotbarSlot((index + row_offset) as u16);
            if self.get_key(key).pressed() {
                events.push(InputEvent::CastSkill { slot });
            }
            if self.get_key(key).released() {
                events.push(InputEvent::StopSkill { slot });
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn handle_keyboard_input(
        &mut self,
        events: &mut Vec<InputEvent>,
        bindings: &KeyBindings,
        pending_capture: Option<BindableAction>,
        #[cfg(feature = "debug")] process_mouse: bool,
        #[cfg(feature = "debug")] use_debug_camera: bool,
    ) {
        if self.capture_keybinding(events, pending_capture) {
            self.input_buffer.clear();
            return;
        }
        let alt_down = self.get_key(KeyCode::AltLeft).down() || self.get_key(KeyCode::AltRight).down();
        let control_down = self.get_key(KeyCode::ControlLeft).down() || self.get_key(KeyCode::ControlRight).down();
        let shift_down = self.get_key(KeyCode::ShiftLeft).down() || self.get_key(KeyCode::ShiftRight).down();

        if self.get_key(KeyCode::Escape).pressed() {
            events.push(InputEvent::Escape);
        }

        // Official client: I opens the inventory. Only on this path, so a
        // focused chat box still types the letter.
        if self.binding_pressed(bindings, BindableAction::OpenInventory, false) {
            events.push(InputEvent::ToggleInventoryWindow);
        }

        // T targets yourself for the armed skill. Shift+1–4 targets other
        // party members in roster order. The number row stays the hotbar
        // when Shift is up.
        if self.binding_pressed(bindings, BindableAction::TargetSelf, false) {
            events.push(InputEvent::TargetSelf);
        }

        if alt_down && self.get_key(KeyCode::KeyE).pressed() {
            events.push(InputEvent::ToggleInventoryWindow);
        }

        if self.binding_pressed(bindings, BindableAction::OpenCharacterOverview, false) {
            events.push(InputEvent::ToggleCharacterOverviewWindow);
        }

        if self.binding_pressed(bindings, BindableAction::OpenSkillTree, false) {
            events.push(InputEvent::ToggleSkillTreeWindow);
        }

        if self.binding_pressed(bindings, BindableAction::OpenStats, false) {
            events.push(InputEvent::ToggleStatsWindow);
        }

        if self.binding_pressed(bindings, BindableAction::OpenParty, false) {
            events.push(InputEvent::TogglePartyWindow);
        }

        if self.binding_pressed(bindings, BindableAction::OpenEquipment, false) {
            events.push(InputEvent::ToggleEquipmentWindow);
        }

        // Original-client-style emotion palette shortcut.
        if self.binding_pressed(bindings, BindableAction::ToggleEmotes, false) {
            events.push(InputEvent::ToggleEmoteWindow);
        }

        // Compatibility alias retained for the earlier fork binding.
        if alt_down && self.get_key(KeyCode::KeyP).pressed() {
            events.push(InputEvent::TogglePartyWindow);
        }

        // Alt+H is the original friend-list binding. Keep the custom HUD on
        // Alt+Shift+H so it does not replace official behavior.
        if self.binding_pressed(bindings, BindableAction::OpenHud, false) {
            events.push(InputEvent::ToggleHudWindow);
        } else if self.binding_pressed(bindings, BindableAction::OpenFriendList, false) {
            events.push(InputEvent::ToggleFriendListWindow);
        }

        // Ctrl+Tab toggles the minimap until its opacity modes land; the shifted
        // chord keeps party-target cycling reachable. Plain Tab cycles monsters.
        if self.binding_pressed(bindings, BindableAction::CyclePartyTarget, false) {
            events.push(InputEvent::CyclePartyTarget);
        } else if self.binding_pressed(bindings, BindableAction::ToggleMinimap, false) {
            events.push(InputEvent::ToggleMinimapWindow);
        } else if self.binding_pressed(bindings, BindableAction::CycleMonsterTarget, true) {
            events.push(InputEvent::CycleMonsterTarget { reverse: shift_down });
        }

        if self.binding_pressed(bindings, BindableAction::OpenAudioSettings, false)
            || (control_down && self.get_key(KeyCode::KeyA).pressed())
        {
            events.push(InputEvent::ToggleAudioSettingsWindow);
        }

        // Alt+Enter is the reflex on Windows. F11 — the other reflex — is not
        // available: the binding right below already owns it.
        if self.binding_pressed(bindings, BindableAction::ToggleFullscreen, false) {
            events.push(InputEvent::ToggleFullscreen);
        }

        if self.binding_pressed(bindings, BindableAction::CloseAllWindows, false) {
            events.push(InputEvent::CloseAllOrdinaryWindows);
        }

        if self.binding_pressed(bindings, BindableAction::OpenGameSettings, false) {
            events.push(InputEvent::ToggleGameSettingsWindow);
        }

        if self.binding_pressed(bindings, BindableAction::OpenInterfaceSettings, false) {
            events.push(InputEvent::ToggleInterfaceSettingsWindow);
        }

        if self.binding_pressed(bindings, BindableAction::OpenGraphicsSettings, false) {
            events.push(InputEvent::ToggleGraphicsSettingsWindow);
        }

        if self.binding_pressed(bindings, BindableAction::OpenAudioSettings, false) {
            events.push(InputEvent::ToggleAudioSettingsWindow);
        }

        if self.binding_pressed(bindings, BindableAction::ToggleInterface, false) {
            events.push(InputEvent::ToggleShowInterface);
        }

        // Ctrl+W, not Ctrl+Q. Ctrl+Q gained a second meaning when the quest log
        // arrived, and both fired in the same frame: the log opened and this
        // immediately closed it again, so the quest window could never be seen.
        // Found in the first day of real play, 2026-09-05.
        if self.binding_pressed(bindings, BindableAction::CloseTopWindow, false) {
            events.push(InputEvent::CloseTopWindow);
        }

        // Number-row hotbar. Only here, not in push_game_action_keys: while
        // chat is focused these keys must type digits rather than fire skills.
        for slot_index in 0..27 {
            let slot = HotbarSlot(slot_index as u16);
            let action = BindableAction::HotbarSlot(slot_index as u8);
            if self.binding_pressed(bindings, action, false) {
                events.push(InputEvent::CastSkill { slot });
            }
            if self.binding_released(bindings, action) {
                events.push(InputEvent::StopSkill { slot });
            }
        }
        const PARTY_KEYS: [KeyCode; 4] = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4];
        if shift_down && !alt_down && !control_down {
            for (index, key) in PARTY_KEYS.into_iter().enumerate() {
                if self.get_key(key).pressed() {
                    events.push(InputEvent::TargetPartyMember { index });
                }
            }
        }

        #[cfg(feature = "debug")]
        let wasd_free = !use_debug_camera;
        #[cfg(not(feature = "debug"))]
        let wasd_free = true;
        if wasd_free {
            let forward = self.binding_down(bindings, BindableAction::MoveForward);
            let back = self.binding_down(bindings, BindableAction::MoveBackward);
            let left = self.binding_down(bindings, BindableAction::MoveLeft);
            let right = self.binding_down(bindings, BindableAction::MoveRight);
            if forward || back || left || right {
                events.push(InputEvent::KeyboardMove {
                    forward,
                    back,
                    left,
                    right,
                });
            }
        }

        // Sit + hotbar always work (also when a UI element has focus — see
        // `handle_game_action_keys`).
        self.push_game_action_keys(events, bindings);

        if self.binding_pressed(bindings, BindableAction::ToggleMaps, false) {
            events.push(InputEvent::ToggleMapsWindow);
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyC).pressed() {
            events.push(InputEvent::ToggleClientStateInspectorWindow);
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyR).pressed() {
            events.push(InputEvent::ToggleRenderOptionsWindow);
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyP).pressed() {
            events.push(InputEvent::ToggleProfilerWindow);
        }

        // Ctrl+O (GM/DM Commands) is handled in push_game_action_keys so it also
        // works while the chat box is focused; keeping it here too would double-toggle.

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyN).pressed() {
            events.push(InputEvent::TogglePacketInspectorWindow);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::ShiftLeft).pressed() && use_debug_camera {
            events.push(InputEvent::CameraAccelerate);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::ShiftLeft).released() && use_debug_camera {
            events.push(InputEvent::CameraDecelerate);
        }

        // TODO: This should be moved.
        #[cfg(feature = "debug")]
        if self.right_mouse_button.down() && !self.right_mouse_button.pressed() && process_mouse && use_debug_camera {
            let offset = -cgmath::Vector2::new(self.mouse_delta.width, self.mouse_delta.height);
            events.push(InputEvent::CameraLookAround { offset });
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::KeyW).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveForward);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::KeyS).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveBackward);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::KeyA).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveLeft);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::KeyD).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveRight);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::Space).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveUp);
        }

        self.input_buffer.clear();
    }
}

#[cfg(test)]
mod keybinding_tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;

    use ragnarok_packets::ClientTick;
    use winit::event::ElementState;
    use winit::keyboard::KeyCode;

    use super::{InputEvent, InputSystem};
    use crate::settings::{BindableAction, KeyBindings, KeyChord};

    #[test]
    fn remapped_shortcut_dispatches_from_the_persisted_table() {
        let mut input = InputSystem::new(Arc::new(AtomicU64::new(0)));
        let mut bindings = KeyBindings::default();
        bindings
            .assign(BindableAction::OpenInventory, KeyChord::new("KeyJ", true, false, false))
            .expect("non-conflicting remap");
        input.update_keyboard(KeyCode::ControlLeft, ElementState::Pressed);
        input.update_keyboard(KeyCode::KeyJ, ElementState::Pressed);
        input.update_delta(ClientTick(1));

        let mut events = Vec::new();
        #[cfg(feature = "debug")]
        input.handle_keyboard_input(&mut events, &bindings, None, false, false);
        #[cfg(not(feature = "debug"))]
        input.handle_keyboard_input(&mut events, &bindings, None);

        assert!(events.iter().any(|event| matches!(event, InputEvent::ToggleInventoryWindow)));
    }

    #[test]
    fn remapped_hotbar_slot_dispatches_and_shift_party_target_stays_available() {
        let mut input = InputSystem::new(Arc::new(AtomicU64::new(0)));
        let mut bindings = KeyBindings::default();
        bindings
            .assign(BindableAction::HotbarSlot(0), KeyChord::new("KeyJ", false, false, false))
            .expect("hotbar slot can be remapped");
        input.update_keyboard(KeyCode::KeyJ, ElementState::Pressed);
        input.update_delta(ClientTick(6));
        let mut events = Vec::new();
        #[cfg(feature = "debug")]
        input.handle_keyboard_input(&mut events, &bindings, None, false, false);
        #[cfg(not(feature = "debug"))]
        input.handle_keyboard_input(&mut events, &bindings, None);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, InputEvent::CastSkill { slot } if slot.0 == 0))
        );

        let mut party_input = InputSystem::new(Arc::new(AtomicU64::new(0)));
        party_input.update_keyboard(KeyCode::Digit1, ElementState::Pressed);
        party_input.update_keyboard(KeyCode::ShiftLeft, ElementState::Pressed);
        party_input.update_delta(ClientTick(7));
        let mut events = Vec::new();
        #[cfg(feature = "debug")]
        party_input.handle_keyboard_input(&mut events, &KeyBindings::default(), None, false, false);
        #[cfg(not(feature = "debug"))]
        party_input.handle_keyboard_input(&mut events, &KeyBindings::default(), None);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, InputEvent::TargetPartyMember { index: 0 }))
        );
        assert!(!events.iter().any(|event| matches!(event, InputEvent::CastSkill { .. })));
    }

    #[test]
    fn keybinding_capture_emits_the_physical_key_and_modifier_state() {
        let mut input = InputSystem::new(Arc::new(AtomicU64::new(0)));
        input.update_keyboard(KeyCode::AltLeft, ElementState::Pressed);
        input.update_keyboard(KeyCode::ShiftLeft, ElementState::Pressed);
        input.update_keyboard(KeyCode::KeyK, ElementState::Pressed);
        input.update_delta(ClientTick(2));

        let mut events = Vec::new();
        #[cfg(feature = "debug")]
        input.handle_keyboard_input(
            &mut events,
            &KeyBindings::default(),
            Some(BindableAction::OpenHud),
            false,
            false,
        );
        #[cfg(not(feature = "debug"))]
        input.handle_keyboard_input(&mut events, &KeyBindings::default(), Some(BindableAction::OpenHud));

        assert!(events.iter().any(|event| matches!(
            event,
            InputEvent::CapturedKeyBinding {
                action: BindableAction::OpenHud,
                chord: KeyChord { key, control: false, alt: true, shift: true },
            } if key == "KeyK"
        )));
    }

    #[test]
    fn shipped_world_map_binding_works_in_release_input_path() {
        let mut input = InputSystem::new(Arc::new(AtomicU64::new(0)));
        input.update_keyboard(KeyCode::ControlLeft, ElementState::Pressed);
        input.update_keyboard(KeyCode::KeyM, ElementState::Pressed);
        input.update_delta(ClientTick(3));
        let mut events = Vec::new();
        #[cfg(feature = "debug")]
        input.handle_keyboard_input(&mut events, &KeyBindings::default(), None, false, false);
        #[cfg(not(feature = "debug"))]
        input.handle_keyboard_input(&mut events, &KeyBindings::default(), None);
        assert!(events.iter().any(|event| matches!(event, InputEvent::ToggleMapsWindow)));
    }

    #[test]
    fn danger_ping_shortcut_dispatches_the_shared_party_ping_event() {
        let mut input = InputSystem::new(Arc::new(AtomicU64::new(0)));
        input.update_keyboard(KeyCode::ControlLeft, ElementState::Pressed);
        input.update_keyboard(KeyCode::AltLeft, ElementState::Pressed);
        input.update_keyboard(KeyCode::KeyG, ElementState::Pressed);
        input.update_delta(ClientTick(5));
        let mut events = Vec::new();
        #[cfg(feature = "debug")]
        input.handle_keyboard_input(&mut events, &KeyBindings::default(), None, false, false);
        #[cfg(not(feature = "debug"))]
        input.handle_keyboard_input(&mut events, &KeyBindings::default(), None);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, InputEvent::SendPartyPing { kind } if kind == "danger"))
        );
    }

    #[test]
    fn remapped_movement_uses_exact_chord_and_preserves_default_directions() {
        let mut input = InputSystem::new(Arc::new(AtomicU64::new(0)));
        let mut bindings = KeyBindings::default();
        bindings
            .assign(BindableAction::MoveForward, KeyChord::new("ArrowUp", false, false, false))
            .expect("arrow key is an available movement binding");
        input.update_keyboard(KeyCode::ArrowUp, ElementState::Pressed);
        input.update_keyboard(KeyCode::KeyA, ElementState::Pressed);
        input.update_delta(ClientTick(4));

        let mut events = Vec::new();
        #[cfg(feature = "debug")]
        input.handle_keyboard_input(&mut events, &bindings, None, false, false);
        #[cfg(not(feature = "debug"))]
        input.handle_keyboard_input(&mut events, &bindings, None);
        assert!(events.iter().any(|event| matches!(event, InputEvent::KeyboardMove {
            forward: true,
            left: true,
            back: false,
            right: false
        })));

        let mut modified = InputSystem::new(Arc::new(AtomicU64::new(0)));
        modified.update_keyboard(KeyCode::ArrowUp, ElementState::Pressed);
        modified.update_keyboard(KeyCode::ControlLeft, ElementState::Pressed);
        modified.update_delta(ClientTick(5));
        let mut events = Vec::new();
        #[cfg(feature = "debug")]
        modified.handle_keyboard_input(&mut events, &bindings, None, false, false);
        #[cfg(not(feature = "debug"))]
        modified.handle_keyboard_input(&mut events, &bindings, None);
        assert!(!events.iter().any(|event| matches!(event, InputEvent::KeyboardMove { .. })));
    }
}
