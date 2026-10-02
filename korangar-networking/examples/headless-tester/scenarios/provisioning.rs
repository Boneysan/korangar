//! Provisioning scenarios: they leave persistent state behind on purpose and
//! are never part of `--scenario all`. Name one to run it.

use std::thread::sleep;
use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::{EquipPosition, InventoryIndex};

use crate::context::{Config, TestContext};
use crate::scenarios::{PROVISIONING_PHASE, Scenario};

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("status-ground-truth", PROVISIONING_PHASE, status_ground_truth),
        Scenario::new("aspd-ground-truth", PROVISIONING_PHASE, aspd_ground_truth),
    ]
}

/// One character to create and configure so the server computes its max HP/SP.
struct Target {
    name: &'static str,
    job_id: u16,
    base_level: u32,
    /// Added with `@vit` / `@int` so the +1%-per-point step is exercised too.
    extra_vit: u32,
    extra_int: u32,
}

/// The cases the class tables were *not* yet compared against a live server
/// for: baby (x70%), Super Novice (+2000 at 99), Expanded Super Novice (+2000
/// at 99 and again at 150), levels above 99, an upper third class, and the two
/// baby jobs whose generated tables dip at a level boundary.
const TARGETS: &[Target] = &[
    Target {
        name: "GtBabyNovice",
        job_id: 4023,
        base_level: 99,
        extra_vit: 30,
        extra_int: 20,
    },
    Target {
        name: "GtSuperNovice",
        job_id: 23,
        base_level: 99,
        extra_vit: 30,
        extra_int: 20,
    },
    Target {
        name: "GtSuperBaby",
        job_id: 4045,
        base_level: 99,
        extra_vit: 30,
        extra_int: 20,
    },
    Target {
        name: "GtExpSNovice",
        job_id: 4190,
        base_level: 150,
        extra_vit: 30,
        extra_int: 20,
    },
    Target {
        name: "GtRuneKnight",
        job_id: 4054,
        base_level: 175,
        extra_vit: 30,
        extra_int: 20,
    },
    Target {
        name: "GtRuneKnightT",
        job_id: 4060,
        base_level: 175,
        extra_vit: 30,
        extra_int: 20,
    },
    Target {
        name: "GtBabyKagerou",
        job_id: 4223,
        base_level: 161,
        extra_vit: 30,
        extra_int: 20,
    },
    Target {
        name: "GtBabyRebel",
        job_id: 4229,
        base_level: 151,
        extra_vit: 30,
        extra_int: 20,
    },
];

/// Create each character if it is missing, put it at its job and level, add
/// some VIT and INT, and print what the *server* reports for its max HP/SP and
/// its total VIT/INT (base + bonus, the values its formulas use).
///
/// Output is one `GROUND-TRUTH` line per character. Nothing is asserted: the
/// point is to capture the server's own numbers, which are then compared with
/// the exported class tables. The characters stay in the account on purpose, so
/// the saved database rows can be read back as a second observation.
///
/// Run with a GM account that has free slots, for example
/// `--username headless3 --password … --partner-password … --scenario
/// status-ground-truth`.
fn status_ground_truth(config: &Config) -> Result<(), String> {
    let mut failures = Vec::new();
    for target in TARGETS {
        match observe(config, target) {
            Ok(line) => println!("{line}"),
            Err(error) => {
                println!("GROUND-TRUTH-FAILED {} {error}", target.name);
                failures.push(format!("{}: {error}", target.name));
            }
        }
        // Let the map server save the character on logout before the next login.
        sleep(Duration::from_secs(3));
    }
    match failures.is_empty() {
        true => Ok(()),
        false => Err(failures.join("; ")),
    }
}

fn observe(config: &Config, target: &Target) -> Result<String, String> {
    let mut context = TestContext::connect_as(config, &config.username, &config.password, Some(target.name), Some(target.name))?;
    context.ensure_job(target.job_id)?;
    context.ensure_base_level(target.base_level)?;
    if target.extra_vit > 0 {
        context.say(&format!("@vit {}", target.extra_vit))?;
    }
    if target.extra_int > 0 {
        context.say(&format!("@int {}", target.extra_int))?;
    }
    // Let the status updates for the last change arrive.
    context.pump(Duration::from_secs(3));
    if context.max_health_points == 0 {
        return Err("the server never reported a max HP".to_owned());
    }
    Ok(format!(
        "GROUND-TRUTH \
         {{\"name\":\"{}\",\"job\":{},\"base_level\":{},\"job_level\":{},\"vit\":[{},{}],\"int\":[{},{}],\"max_hp\":{},\"max_sp\":{}}}",
        target.name,
        context.job_id.0,
        context.base_level,
        context.job_level,
        context.vitality.0,
        context.vitality.1,
        context.intelligence.0,
        context.intelligence.1,
        context.max_health_points,
        context.max_spell_points,
    ))
}

// --- ASPD -----------------------------------------------------------------

/// One equipment setup to measure: a job and what is in each hand.
struct Setup {
    job_id: u16,
    label: &'static str,
    right: Option<(u32, Hands)>,
    left: Option<(u32, Hands)>,
}

#[derive(Clone, Copy)]
enum Hands {
    /// One-handed weapon, equipped in the right hand.
    Right,
    /// Two-handed weapon (`EQP_ARMS`): both hands.
    Both,
    /// Shield or off-hand weapon.
    Left,
}

impl Hands {
    fn position(self) -> EquipPosition {
        match self {
            Hands::Right => EquipPosition::RIGHT_HAND,
            Hands::Both => EquipPosition::RIGHT_HAND | EquipPosition::LEFT_HAND,
            Hands::Left => EquipPosition::LEFT_HAND,
        }
    }
}

const fn one(item_id: u32) -> Option<(u32, Hands)> {
    Some((item_id, Hands::Right))
}
const fn two(item_id: u32) -> Option<(u32, Hands)> {
    Some((item_id, Hands::Both))
}
const fn off(item_id: u32) -> Option<(u32, Hands)> {
    Some((item_id, Hands::Left))
}

/// Weapons are the simplest script-free item of each type from `item_db.conf`
/// that the job can equip. Covers fist, one- and two-handed melee, a shield,
/// dual wield, bows/instruments/whips/five gun types (DEX counts as dex^2/7),
/// knuckle, book and rod, plus baby, upper and third-class inheritance.
const ASPD_SETUPS: &[Setup] = &[
    Setup {
        job_id: 7,
        label: "Knight fist",
        right: None,
        left: None,
    },
    Setup {
        job_id: 7,
        label: "Knight sword",
        right: one(1101),
        left: None,
    },
    Setup {
        job_id: 7,
        label: "Knight sword+shield",
        right: one(1101),
        left: off(2103),
    },
    Setup {
        job_id: 7,
        label: "Knight axe",
        right: one(1301),
        left: None,
    },
    Setup {
        job_id: 7,
        label: "Knight mace",
        right: one(1501),
        left: None,
    },
    Setup {
        job_id: 7,
        label: "Knight spear",
        right: one(1401),
        left: None,
    },
    Setup {
        job_id: 7,
        label: "Knight two-hand sword",
        right: two(1116),
        left: None,
    },
    Setup {
        job_id: 7,
        label: "Knight two-hand axe",
        right: two(1351),
        left: None,
    },
    Setup {
        job_id: 12,
        label: "Assassin knife",
        right: one(1201),
        left: None,
    },
    Setup {
        job_id: 12,
        label: "Assassin dual knives",
        right: one(1201),
        left: off(1201),
    },
    Setup {
        job_id: 12,
        label: "Assassin katar",
        right: two(1250),
        left: None,
    },
    Setup {
        job_id: 11,
        label: "Hunter bow",
        right: two(1701),
        left: None,
    },
    Setup {
        job_id: 19,
        label: "Bard violin",
        right: one(1901),
        left: None,
    },
    Setup {
        job_id: 20,
        label: "Dancer whip",
        right: one(1950),
        left: None,
    },
    Setup {
        job_id: 8,
        label: "Priest knuckle",
        right: one(1801),
        left: None,
    },
    Setup {
        job_id: 8,
        label: "Priest book",
        right: one(1550),
        left: None,
    },
    Setup {
        job_id: 9,
        label: "Wizard rod",
        right: one(1601),
        left: None,
    },
    Setup {
        job_id: 24,
        label: "Gunslinger revolver",
        right: two(13118),
        left: None,
    },
    Setup {
        job_id: 24,
        label: "Gunslinger rifle",
        right: two(13150),
        left: None,
    },
    Setup {
        job_id: 24,
        label: "Gunslinger gatling",
        right: two(13182),
        left: None,
    },
    Setup {
        job_id: 24,
        label: "Gunslinger shotgun",
        right: two(13181),
        left: None,
    },
    Setup {
        job_id: 24,
        label: "Gunslinger grenade",
        right: two(13183),
        left: None,
    },
    Setup {
        job_id: 4008,
        label: "Lord Knight sword",
        right: one(1101),
        left: None,
    },
    Setup {
        job_id: 4054,
        label: "Rune Knight sword",
        right: one(1101),
        left: None,
    },
    Setup {
        job_id: 4024,
        label: "Baby Swordsman sword",
        right: one(1101),
        left: None,
    },
];

/// (AGI, DEX) added on top of the starting stats at each stage.
const ASPD_STAGES: &[(u32, u32)] = &[(0, 0), (39, 29), (59, 69)];

/// Measure the server's own attack motion (and our formula's inputs: total AGI
/// and DEX) for every setup at three stat levels. Prints one `ASPD-TRUTH` line
/// each; nothing is asserted. Run on a GM account with a free slot:
/// `--username headless2 --password … --partner-password … --scenario
/// aspd-ground-truth`.
fn aspd_ground_truth(config: &Config) -> Result<(), String> {
    let mut context = TestContext::connect_as(config, &config.username, &config.password, Some("GtAspd"), Some("GtAspd"))?;
    context.ensure_base_level(99)?;
    let mut failures = Vec::new();
    for (stage, (agi, dex)) in ASPD_STAGES.iter().enumerate() {
        if *agi > 0 {
            context.say(&format!("@agi {agi}"))?;
        }
        if *dex > 0 {
            context.say(&format!("@dex {dex}"))?;
        }
        context.pump(Duration::from_secs(2));
        for setup in ASPD_SETUPS {
            match measure_aspd(&mut context, setup) {
                Ok(line) => println!("ASPD-TRUTH {{\"stage\":{stage},{line}"),
                Err(error) => {
                    println!("ASPD-TRUTH-FAILED stage {stage} {}: {error}", setup.label);
                    failures.push(format!("{}: {error}", setup.label));
                }
            }
        }
    }
    match failures.is_empty() {
        true => Ok(()),
        false => Err(format!("{} setups failed, first: {}", failures.len(), failures[0])),
    }
}

fn equip(context: &mut TestContext, item: (u32, Hands)) -> Result<InventoryIndex, String> {
    let index = context.give_item(item.0, 1)?;
    context.flush();
    let position = item.1.position();
    context
        .net
        .request_item_equip(index, position)
        .map_err(|_| "disconnected".to_owned())?;
    context.wait_for_within(
        &format!("equip of item {}", item.0),
        Duration::from_secs(5),
        &mut |event| match event {
            NetworkEvent::UpdateEquippedPosition {
                index: event_index,
                equipped_position,
            } if *event_index == index && !equipped_position.is_empty() => Some(()),
            _ => None,
        },
    )?;
    Ok(index)
}

fn measure_aspd(context: &mut TestContext, setup: &Setup) -> Result<String, String> {
    context.ensure_job(setup.job_id)?;
    let mut worn: Vec<(InventoryIndex, u32)> = Vec::new();
    let result = (|| {
        for item in [setup.right, setup.left].into_iter().flatten() {
            let index = equip(context, item)?;
            worn.push((index, item.0));
        }
        context.pump(Duration::from_millis(1500));
        Ok(format!(
            "\"job\":{},\"label\":\"{}\",\"agi\":[{},{}],\"dex\":[{},{}],\"amotion\":{}}}",
            context.job_id.0,
            setup.label,
            context.agility.0,
            context.agility.1,
            context.dexterity.0,
            context.dexterity.1,
            context.attack_speed
        ))
    })();
    // Always put things back, so one failed setup does not poison the next.
    for (index, item_id) in worn {
        context.flush();
        let _ = context.net.request_item_unequip(index);
        context.pump(Duration::from_millis(500));
        let _ = context.say(&format!("@delitem {item_id} 1"));
        context.pump(Duration::from_millis(300));
    }
    result
}
