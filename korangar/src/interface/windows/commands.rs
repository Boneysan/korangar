use korangar_interface::element::StateElement;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, RustState, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

/// Which tab of the GM / DM panel is showing. Index into the header buttons.
/// 0 = DM, 1 = Beats, 2 = Character, 3 = Items, 4 = Combat, 5 = Travel.
type TabIndex = u8;

/// Internal state of the GM / DM command panel.
#[derive(Default, RustState, StateElement)]
pub struct CommandsWindowState {
    selected_tab: TabIndex,
}

/// GM / DM command panel. Sends Hercules atcommands as chat (`@…`).
///
/// Requires a GM account (group 99 Admin already has `all_commands`; group 5
/// Dungeon Master has campaign `@dm*` plus limited live-control commands).
///
/// Organized into tabs (DM · Beats · Character · Items · Combat · Travel) plus
/// a Handbook launcher because
/// a DM needs a lot of live controls; the header buttons switch `selected_tab`
/// and an `either!` chain swaps the body.
pub struct CommandsWindow<A> {
    commands_window_state: A,
}

impl<A> CommandsWindow<A> {
    pub fn new(commands_window_state: A) -> Self {
        Self { commands_window_state }
    }
}

impl<A> CustomWindow<ClientState> for CommandsWindow<A>
where
    A: Path<ClientState, CommandsWindowState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Commands)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let tab = self.commands_window_state.selected_tab();

        // The active tab's header button is disabled (greyed) as the selected
        // indicator; the others switch `selected_tab` on click.
        let is_tab = move |index: TabIndex| ComputedSelector::new_default(move |state: &ClientState| *tab.follow_safe(state) == index);
        let select_tab =
            move |index: TabIndex| move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| state.update_value(tab, index);

        window! {
            title: "GM / DM Commands",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "DM",
                            tooltip: "Campaign DM mode, status, and beat director",
                            disabled: is_tab(0),
                            event: select_tab(0),
                        },
                        button! {
                            text: "Beats",
                            tooltip: "Jump straight into an arc's beat director",
                            disabled: is_tab(1),
                            event: select_tab(1),
                        },
                        button! {
                            text: "Handbook",
                            tooltip: "Open the private Seal Cascade GM role-playing guide [^000001@dmguide^000000]",
                            event: InputEvent::SendMessage { text: "@dmguide".to_string() },
                        },
                        button! {
                            text: "Character",
                            tooltip: "Levels, zeny, stats, and skills",
                            disabled: is_tab(2),
                            event: select_tab(2),
                        },
                        button! {
                            text: "Items",
                            tooltip: "Starter gear and consumables",
                            disabled: is_tab(3),
                            event: select_tab(3),
                        },
                        button! {
                            text: "Combat",
                            tooltip: "Heal, spawn, and live combat helpers",
                            disabled: is_tab(4),
                            event: select_tab(4),
                        },
                        button! {
                            text: "Travel",
                            tooltip: "Warp to towns",
                            disabled: is_tab(5),
                            event: select_tab(5),
                        },
                    ),
                },
                either! {
                    selector: is_tab(0),
                    on_true: fragment! {
                        gaps: theme().window().gaps(),
                        children: (
                            text! {
                                text: "Seal Cascade DM mode",
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            split! {
                                gaps: theme().window().gaps(),
                                children: (
                                    button! {
                                        text: "DM mode ON",
                                        tooltip: "Suppress stock MVP/BOSS and gate campaign NPCs to your party [^000001@dm mode on^000000]",
                                        event: InputEvent::SendMessage { text: "@dm mode on".to_string() },
                                    },
                                    button! {
                                        text: "DM mode OFF",
                                        tooltip: "Restore normal MVP/BOSS and clear session [^000001@dm mode off^000000]",
                                        event: InputEvent::SendMessage { text: "@dm mode off".to_string() },
                                    },
                                ),
                            },
                            split! {
                                gaps: theme().window().gaps(),
                                children: (
                                    button! {
                                        text: "DM status",
                                        tooltip: "[^000001@dm status^000000]",
                                        event: InputEvent::SendMessage { text: "@dm status".to_string() },
                                    },
                                    button! {
                                        text: "Beat menu",
                                        tooltip: "Open the full Act → Arc → Beat director [^000001@dmbeat^000000]",
                                        event: InputEvent::SendMessage { text: "@dmbeat".to_string() },
                                    },
                                    button! {
                                        text: "DM help",
                                        tooltip: "[^000001@dm help^000000]",
                                        event: InputEvent::SendMessage { text: "@dm help".to_string() },
                                    },
                                ),
                            },
                            text! {
                                text: "Tip: Ctrl+D opens the Dice Roller. @dm* also works in chat.",
                                overflow_behavior: OverflowBehavior::LineBreak,
                            },
                        ),
                    },
                    on_false: either! {
                        selector: is_tab(1),
                        on_true: fragment! {
                            gaps: theme().window().gaps(),
                            children: (
                                text! {
                                    text: "Act I (Arcs 1-5)",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                split! {
                                    gaps: theme().window().gaps(),
                                    children: (
                                        button! {
                                            text: "Arc 1",
                                            tooltip: "Omens (Prontera) [^000001@dmbeat 1^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 1".to_string() },
                                        },
                                        button! {
                                            text: "Arc 2",
                                            tooltip: "Sleeping Forest (Payon) [^000001@dmbeat 2^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 2".to_string() },
                                        },
                                        button! {
                                            text: "Arc 3",
                                            tooltip: "Sand and Whispers (Morroc) [^000001@dmbeat 3^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 3".to_string() },
                                        },
                                        button! {
                                            text: "Arc 4",
                                            tooltip: "City Above the Beast (Geffen) [^000001@dmbeat 4^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 4".to_string() },
                                        },
                                        button! {
                                            text: "Arc 5",
                                            tooltip: "Tides and Trade (Alberta) [^000001@dmbeat 5^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 5".to_string() },
                                        },
                                    ),
                                },
                                text! {
                                    text: "Act II (Arcs 6-10)",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                split! {
                                    gaps: theme().window().gaps(),
                                    children: (
                                        button! {
                                            text: "Arc 6",
                                            tooltip: "The Floating Republic (Yuno) [^000001@dmbeat 6^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 6".to_string() },
                                        },
                                        button! {
                                            text: "Arc 7",
                                            tooltip: "Iron and Ash (Einbroch) [^000001@dmbeat 7^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 7".to_string() },
                                        },
                                        button! {
                                            text: "Arc 8",
                                            tooltip: "The Cursed Kingdom (Glast Heim) [^000001@dmbeat 8^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 8".to_string() },
                                        },
                                        button! {
                                            text: "Arc 9",
                                            tooltip: "Frozen Faith (Rachel) [^000001@dmbeat 9^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 9".to_string() },
                                        },
                                        button! {
                                            text: "Arc 10",
                                            tooltip: "The Lab Beneath (Lighthalzen) [^000001@dmbeat 10^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 10".to_string() },
                                        },
                                    ),
                                },
                                text! {
                                    text: "Act III (Arcs 11-14)",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                split! {
                                    gaps: theme().window().gaps(),
                                    children: (
                                        button! {
                                            text: "Arc 11",
                                            tooltip: "Wrath of Heaven (Hugel) [^000001@dmbeat 11^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 11".to_string() },
                                        },
                                        button! {
                                            text: "Arc 12",
                                            tooltip: "Beyond the Horizon (New World) [^000001@dmbeat 12^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 12".to_string() },
                                        },
                                        button! {
                                            text: "Arc 13",
                                            tooltip: "Island of the Damned (Nameless) [^000001@dmbeat 13^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 13".to_string() },
                                        },
                                        button! {
                                            text: "Arc 14",
                                            tooltip: "The Fire That Ends the World (Veins) [^000001@dmbeat 14^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 14".to_string() },
                                        },
                                    ),
                                },
                                text! {
                                    text: "Act IV (Arcs 15-19)",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                split! {
                                    gaps: theme().window().gaps(),
                                    children: (
                                        button! {
                                            text: "Arc 15",
                                            tooltip: "The Hero's Tomb (Thanatos) [^000001@dmbeat 15^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 15".to_string() },
                                        },
                                        button! {
                                            text: "Arc 16",
                                            tooltip: "The Royal Banquet (Prontera) [^000001@dmbeat 16^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 16".to_string() },
                                        },
                                        button! {
                                            text: "Arc 17",
                                            tooltip: "The Sage's Legacy (Varmundt) [^000001@dmbeat 17^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 17".to_string() },
                                        },
                                        button! {
                                            text: "Arc 18",
                                            tooltip: "The Witch of Death (Niflheim) [^000001@dmbeat 18^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 18".to_string() },
                                        },
                                        button! {
                                            text: "Arc 19",
                                            tooltip: "Nightmare of Midgard (Finale) [^000001@dmbeat 19^000000]",
                                            event: InputEvent::SendMessage { text: "@dmbeat 19".to_string() },
                                        },
                                    ),
                                },
                                text! {
                                    text: "Each opens that arc's beat submenu. Beat menu (DM tab) opens the full director.",
                                    overflow_behavior: OverflowBehavior::LineBreak,
                                },
                            ),
                        },
                        on_false: either! {
                            selector: is_tab(2),
                            on_true: fragment! {
                                gaps: theme().window().gaps(),
                                children: (
                                    text! {
                                        text: "Base level",
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                    split! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            button! {
                                                text: "+1",
                                                tooltip: "[^000001@blvl 1^000000]",
                                                event: InputEvent::SendMessage { text: "@blvl 1".to_string() },
                                            },
                                            button! {
                                                text: "+5",
                                                tooltip: "[^000001@blvl 5^000000]",
                                                event: InputEvent::SendMessage { text: "@blvl 5".to_string() },
                                            },
                                            button! {
                                                text: "+10",
                                                tooltip: "[^000001@blvl 10^000000]",
                                                event: InputEvent::SendMessage { text: "@blvl 10".to_string() },
                                            },
                                            button! {
                                                text: "+50",
                                                tooltip: "[^000001@blvl 50^000000]",
                                                event: InputEvent::SendMessage { text: "@blvl 50".to_string() },
                                            },
                                            button! {
                                                text: "MAX",
                                                tooltip: "[^000001@blvl 9999^000000]",
                                                event: InputEvent::SendMessage { text: "@blvl 9999".to_string() },
                                            },
                                        ),
                                    },
                                    text! {
                                        text: "Job level",
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                    split! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            button! {
                                                text: "+1",
                                                tooltip: "[^000001@jlvl 1^000000]",
                                                event: InputEvent::SendMessage { text: "@jlvl 1".to_string() },
                                            },
                                            button! {
                                                text: "+5",
                                                tooltip: "[^000001@jlvl 5^000000]",
                                                event: InputEvent::SendMessage { text: "@jlvl 5".to_string() },
                                            },
                                            button! {
                                                text: "+10",
                                                tooltip: "[^000001@jlvl 10^000000]",
                                                event: InputEvent::SendMessage { text: "@jlvl 10".to_string() },
                                            },
                                            button! {
                                                text: "MAX",
                                                tooltip: "[^000001@jlvl 9999^000000]",
                                                event: InputEvent::SendMessage { text: "@jlvl 9999".to_string() },
                                            },
                                        ),
                                    },
                                    text! {
                                        text: "Zeny",
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                    split! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            button! {
                                                text: "+10k",
                                                tooltip: "[^000001@zeny 10000^000000]",
                                                event: InputEvent::SendMessage { text: "@zeny 10000".to_string() },
                                            },
                                            button! {
                                                text: "+100k",
                                                tooltip: "[^000001@zeny 100000^000000]",
                                                event: InputEvent::SendMessage { text: "@zeny 100000".to_string() },
                                            },
                                            button! {
                                                text: "+1M",
                                                tooltip: "[^000001@zeny 1000000^000000]",
                                                event: InputEvent::SendMessage { text: "@zeny 1000000".to_string() },
                                            },
                                            button! {
                                                text: "Set 0",
                                                tooltip: "[^000001@zeny -999999999^000000] (approximate wipe)",
                                                event: InputEvent::SendMessage { text: "@zeny -999999999".to_string() },
                                            },
                                        ),
                                    },
                                    text! {
                                        text: "Stats & skills",
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                    split! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            button! {
                                                text: "All stats max",
                                                tooltip: "[^000001@allstats^000000]",
                                                event: InputEvent::SendMessage { text: "@allstats".to_string() },
                                            },
                                            button! {
                                                text: "All skills",
                                                tooltip: "[^000001@allskill^000000]",
                                                event: InputEvent::SendMessage { text: "@allskill".to_string() },
                                            },
                                            button! {
                                                text: "Reset stats",
                                                tooltip: "[^000001@reset^000000]",
                                                event: InputEvent::SendMessage { text: "@reset".to_string() },
                                            },
                                            button! {
                                                text: "Reset skills",
                                                tooltip: "[^000001@resetskill^000000]",
                                                event: InputEvent::SendMessage { text: "@resetskill".to_string() },
                                            },
                                        ),
                                    },
                                ),
                            },
                            on_false: either! {
                                selector: is_tab(3),
                                on_true: fragment! {
                                    gaps: theme().window().gaps(),
                                    children: (
                                        text! {
                                            text: "Starter gear / consumables",
                                            overflow_behavior: OverflowBehavior::Shrink,
                                        },
                                        split! {
                                            gaps: theme().window().gaps(),
                                            children: (
                                                button! {
                                                    text: "Knife",
                                                    tooltip: "[^000001@item 1201 1^000000] Knife",
                                                    event: InputEvent::SendMessage { text: "@item 1201 1".to_string() },
                                                },
                                                button! {
                                                    text: "Cotton Shirt",
                                                    tooltip: "[^000001@item 2301 1^000000]",
                                                    event: InputEvent::SendMessage { text: "@item 2301 1".to_string() },
                                                },
                                                button! {
                                                    text: "Red Potion x50",
                                                    tooltip: "[^000001@item 501 50^000000]",
                                                    event: InputEvent::SendMessage { text: "@item 501 50".to_string() },
                                                },
                                                button! {
                                                    text: "Fly Wing x20",
                                                    tooltip: "[^000001@item 601 20^000000]",
                                                    event: InputEvent::SendMessage { text: "@item 601 20".to_string() },
                                                },
                                            ),
                                        },
                                        split! {
                                            gaps: theme().window().gaps(),
                                            children: (
                                                button! {
                                                    text: "Butterfly Wing x10",
                                                    tooltip: "[^000001@item 602 10^000000]",
                                                    event: InputEvent::SendMessage { text: "@item 602 10".to_string() },
                                                },
                                                button! {
                                                    text: "Magnifier x5",
                                                    tooltip: "[^000001@item 611 5^000000] identify gear",
                                                    event: InputEvent::SendMessage { text: "@item 611 5".to_string() },
                                                },
                                                button! {
                                                    text: "Yggdrasil Seed x5",
                                                    tooltip: "[^000001@item 608 5^000000]",
                                                    event: InputEvent::SendMessage { text: "@item 608 5".to_string() },
                                                },
                                            ),
                                        },
                                    ),
                                },
                                on_false: either! {
                                    selector: is_tab(4),
                                    on_true: fragment! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            text! {
                                                text: "Combat helpers",
                                                overflow_behavior: OverflowBehavior::Shrink,
                                            },
                                            split! {
                                                gaps: theme().window().gaps(),
                                                children: (
                                                    button! {
                                                        text: "Heal",
                                                        tooltip: "[^000001@heal^000000]",
                                                        event: InputEvent::SendMessage { text: "@heal".to_string() },
                                                    },
                                                    button! {
                                                        text: "Alive",
                                                        tooltip: "[^000001@alive^000000]",
                                                        event: InputEvent::SendMessage { text: "@alive".to_string() },
                                                    },
                                                    button! {
                                                        text: "Autoloot on",
                                                        tooltip: "[^000001@autoloot 100^000000]",
                                                        event: InputEvent::SendMessage { text: "@autoloot 100".to_string() },
                                                    },
                                                    button! {
                                                        text: "Speed 1",
                                                        tooltip: "[^000001@speed 1^000000] (fast walk)",
                                                        event: InputEvent::SendMessage { text: "@speed 1".to_string() },
                                                    },
                                                ),
                                            },
                                            split! {
                                                gaps: theme().window().gaps(),
                                                children: (
                                                    button! {
                                                        text: "Spawn Poring",
                                                        tooltip: "[^000001@spawn 1002 1^000000]",
                                                        event: InputEvent::SendMessage { text: "@spawn 1002 1".to_string() },
                                                    },
                                                    button! {
                                                        text: "Spawn 5 Porings",
                                                        tooltip: "[^000001@spawn 1002 5^000000]",
                                                        event: InputEvent::SendMessage { text: "@spawn 1002 5".to_string() },
                                                    },
                                                    button! {
                                                        text: "Kill monsters",
                                                        tooltip: "[^000001@killmonster2^000000] (no loot)",
                                                        event: InputEvent::SendMessage { text: "@killmonster2".to_string() },
                                                    },
                                                ),
                                            },
                                        ),
                                    },
                                    on_false: fragment! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            text! {
                                                text: "Travel",
                                                overflow_behavior: OverflowBehavior::Shrink,
                                            },
                                            split! {
                                                gaps: theme().window().gaps(),
                                                children: (
                                                    button! {
                                                        text: "Prontera",
                                                        tooltip: "[^000001@go 0^000000]",
                                                        event: InputEvent::SendMessage { text: "@go 0".to_string() },
                                                    },
                                                    button! {
                                                        text: "Izlude",
                                                        tooltip: "[^000001@go 9^000000]",
                                                        event: InputEvent::SendMessage { text: "@go 9".to_string() },
                                                    },
                                                    button! {
                                                        text: "Geffen",
                                                        tooltip: "[^000001@go 2^000000]",
                                                        event: InputEvent::SendMessage { text: "@go 2".to_string() },
                                                    },
                                                    button! {
                                                        text: "Payon",
                                                        tooltip: "[^000001@go 4^000000]",
                                                        event: InputEvent::SendMessage { text: "@go 4".to_string() },
                                                    },
                                                ),
                                            },
                                        ),
                                    },
                                },
                            },
                        },
                    },
                },
            ),
        }
    }
}
