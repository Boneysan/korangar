use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::event::{Event, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, RustState, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

const MAXIMUM_ROLL_INPUT_LENGTH: usize = 40;

/// ZST for the custom-roll text box focus id.
pub struct DiceRollTextBox;

/// Internal state of the dice roller window.
#[derive(Default, RustState, StateElement)]
pub struct DiceWindowState {
    /// Free-form roll expression, e.g. `3d8+2`.
    custom_text: String,
    /// When set, rolls are sent as `@roll hidden …` (GM level 60+ only; the
    /// server rejects it for non-GMs).
    hidden: bool,
}

/// Builds a click handler that sends a fixed dice expression as `@roll`,
/// honoring the window's Hidden toggle at click time.
fn roll<H>(hidden_path: H, dice: &'static str) -> impl Fn(&State<ClientState>, &mut EventQueue<ClientState>) + 'static
where
    H: Path<ClientState, bool> + Copy + 'static,
{
    move |state, queue| {
        let text = match *state.get(&hidden_path) {
            true => format!("@roll hidden {dice}"),
            false => format!("@roll {dice}"),
        };
        queue.queue(InputEvent::SendMessage { text });
    }
}

/// Builds a click handler that sends the custom-roll field as `@roll`, honoring
/// the Hidden toggle, then clears the field. Used by both the text box (Enter)
/// and the Roll button.
fn custom_roll<C, H>(custom_path: C, hidden_path: H) -> impl Fn(&State<ClientState>, &mut EventQueue<ClientState>) + 'static
where
    C: Path<ClientState, String> + Copy + 'static,
    H: Path<ClientState, bool> + Copy + 'static,
{
    move |state, queue| {
        let input = state.get(&custom_path).trim().to_owned();
        if input.is_empty() {
            return;
        }
        let text = match *state.get(&hidden_path) {
            true => format!("@roll hidden {input}"),
            false => format!("@roll {input}"),
        };
        state.update_value_with(custom_path, |current| current.clear());
        queue.queue(InputEvent::SendMessage { text });
        queue.queue(Event::Unfocus);
    }
}

/// Dice roller. Sends the Hercules `@roll` atcommand as chat. `@roll` is a
/// level-0 command, so every player can use this window; `hidden` rolls require
/// GM level 60+ (enforced server-side).
pub struct DiceWindow<A> {
    dice_window_state: A,
}

impl<A> DiceWindow<A> {
    pub fn new(dice_window_state: A) -> Self {
        Self { dice_window_state }
    }
}

impl<A> CustomWindow<ClientState> for DiceWindow<A>
where
    A: Path<ClientState, DiceWindowState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Dice)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let custom_path = self.dice_window_state.custom_text();
        let hidden_path = self.dice_window_state.hidden();

        window! {
            title: "Dice Roller",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: "Standard",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "d4",
                            tooltip: "Roll 1d4 [^000001@roll 1d4^000000]",
                            event: roll(hidden_path, "1d4"),
                        },
                        button! {
                            text: "d6",
                            tooltip: "Roll 1d6 [^000001@roll 1d6^000000]",
                            event: roll(hidden_path, "1d6"),
                        },
                        button! {
                            text: "d8",
                            tooltip: "Roll 1d8 [^000001@roll 1d8^000000]",
                            event: roll(hidden_path, "1d8"),
                        },
                        button! {
                            text: "d10",
                            tooltip: "Roll 1d10 [^000001@roll 1d10^000000]",
                            event: roll(hidden_path, "1d10"),
                        },
                        button! {
                            text: "d12",
                            tooltip: "Roll 1d12 [^000001@roll 1d12^000000]",
                            event: roll(hidden_path, "1d12"),
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "d20",
                            tooltip: "Roll 1d20 [^000001@roll 1d20^000000]",
                            event: roll(hidden_path, "1d20"),
                        },
                        button! {
                            text: "d100",
                            tooltip: "Roll 1d100 [^000001@roll 1d100^000000]",
                            event: roll(hidden_path, "1d100"),
                        },
                    ),
                },
                text! {
                    text: "Common",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "2d6",
                            tooltip: "Roll 2d6 [^000001@roll 2d6^000000]",
                            event: roll(hidden_path, "2d6"),
                        },
                        button! {
                            text: "3d6",
                            tooltip: "Roll 3d6 [^000001@roll 3d6^000000]",
                            event: roll(hidden_path, "3d6"),
                        },
                        button! {
                            text: "4d6",
                            tooltip: "Roll 4d6 [^000001@roll 4d6^000000]",
                            event: roll(hidden_path, "4d6"),
                        },
                        button! {
                            text: "2d20",
                            tooltip: "Roll 2d20 (advantage/disadvantage) [^000001@roll 2d20^000000]",
                            event: roll(hidden_path, "2d20"),
                        },
                    ),
                },
                text! {
                    text: "Custom (NdX+mod, e.g. 3d8+2)",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text_box! {
                    ghost_text: "3d8+2",
                    state: custom_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_ROLL_INPUT_LENGTH>::new(custom_path, custom_roll(custom_path, hidden_path)),
                    focus_id: DiceRollTextBox,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Roll",
                            tooltip: "Roll the custom expression [^000001@roll <expr>^000000]",
                            event: custom_roll(custom_path, hidden_path),
                        },
                    ),
                },
                state_button! {
                    text: "Hidden (GM only)",
                    tooltip: "Send rolls as ^000001@roll hidden^000000 — result reported only to you (requires GM level 60+)",
                    state: hidden_path,
                    event: Toggle(hidden_path),
                },
                text! {
                    text: "Tip: you can also type @roll 1d20 in chat. Individual dice are shown for up to 20 dice.",
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
            ),
        }
    }
}
