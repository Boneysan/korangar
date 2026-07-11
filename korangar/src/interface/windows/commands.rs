use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

/// GM / DM command panel. Sends Hercules atcommands as chat (`@…`).
///
/// Requires a GM account (group 99 Admin already has `all_commands`; group 5
/// Dungeon Master has campaign `@dm*` plus limited live-control commands).
pub struct CommandsWindow;

impl CustomWindow<ClientState> for CommandsWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Commands)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "GM / DM Commands",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
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
                            tooltip: "Open the arc beat director [^000001@dmbeat^000000]",
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
                text! {
                    text: "Tip: you can also type @commands in chat (e.g. @item 501 10). Account korangar is Admin.",
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
            ),
        }
    }
}
