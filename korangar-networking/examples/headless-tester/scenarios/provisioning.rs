//! Provisioning scenarios: they leave persistent state behind on purpose and
//! are never part of `--scenario all`. Name one to run it.

use std::thread::sleep;
use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::{EquipPosition, InventoryIndex, SkillId};

use crate::context::{Config, TestContext};
use crate::scenarios::{PROVISIONING_PHASE, Scenario};

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("status-ground-truth", PROVISIONING_PHASE, status_ground_truth),
        Scenario::new("aspd-ground-truth", PROVISIONING_PHASE, aspd_ground_truth),
        Scenario::new("aspd-modifiers-ground-truth", PROVISIONING_PHASE, aspd_modifiers_ground_truth),
        Scenario::new(
            "aspd-modifiers-ground-truth-2",
            PROVISIONING_PHASE,
            aspd_modifiers_ground_truth_2,
        ),
        Scenario::new("aspd-mount-ground-truth", PROVISIONING_PHASE, aspd_mount_ground_truth),
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
    Head,
    Shoes,
    AccessoryLeft,
    AccessoryRight,
}

impl Hands {
    fn position(self) -> EquipPosition {
        match self {
            Hands::Right => EquipPosition::RIGHT_HAND,
            Hands::Both => EquipPosition::RIGHT_HAND | EquipPosition::LEFT_HAND,
            Hands::Left => EquipPosition::LEFT_HAND,
            Hands::Head => EquipPosition::HEAD_TOP,
            Hands::Shoes => EquipPosition::SHOES,
            Hands::AccessoryLeft => EquipPosition::LEFT_ACCESSORY,
            Hands::AccessoryRight => EquipPosition::RIGTH_ACCESSORY,
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

// --- ASPD modifiers: passive skills, equipment, statuses, mount, cap, whip
// ----

/// One fresh-login experiment. Statuses and mounts end with the session, so
/// each step logs in again; worn items are always removed at the end because
/// they persist on the character.
struct Step {
    label: &'static str,
    /// Which character: `GtAspd` (male) or `GtAspdF` (female, for Dancer).
    character: &'static str,
    job_id: u16,
    /// Added to AGI and DEX before measuring (relative, like `@agi`).
    extra_agi: u32,
    extra_dex: u32,
    equip: &'static [(u32, Hands)],
    /// Consumables to use (potions), one each.
    use_items: &'static [u32],
    /// Measure once before the commands and casts, so the effect is isolated.
    measure_before: bool,
    commands: &'static [&'static str],
    /// `@useskill <id> <level> self` after the commands.
    casts: &'static [(u16, u16)],
    /// Print a measurement after every command, to see each toggle's effect.
    measure_each: bool,
}

const fn step(label: &'static str, job_id: u16) -> Step {
    Step {
        label,
        character: "GtAspd",
        job_id,
        extra_agi: 0,
        extra_dex: 0,
        equip: &[],
        use_items: &[],
        measure_before: false,
        commands: &[],
        casts: &[],
        measure_each: false,
    }
}

const MODIFIER_STEPS: &[Step] = &[
    // Passive ASPD skills, measured before and after `@allskill` (level 10 each).
    Step {
        equip: &[(1550, Hands::Right)],
        measure_before: true,
        commands: &["@allskill"],
        ..step("passive Advanced Book (Sage, book)", 16)
    },
    Step {
        equip: &[(13118, Hands::Both)],
        measure_before: true,
        commands: &["@allskill"],
        ..step("passive Single Action (Gunslinger, revolver)", 24)
    },
    Step {
        equip: &[(1201, Hands::Right)],
        measure_before: true,
        commands: &["@allskill"],
        ..step("passive Plagiarism (Rogue, knife)", 17)
    },
    Step {
        equip: &[(1901, Hands::Right)],
        measure_before: true,
        commands: &["@allskill"],
        ..step("passive Musical Lesson (Bard, violin)", 19)
    },
    Step {
        measure_before: true,
        commands: &["@allskill"],
        ..step("passives need their weapon (Bard, bare hands)", 19)
    },
    // Equipment modifiers.
    Step {
        equip: &[(1307, Hands::Right)],
        ..step("equipment bAspdRate +5 (Windhawk)", 7)
    },
    Step {
        equip: &[(1283, Hands::Both)],
        ..step("equipment bAspdRate +3 (Katar of Speed)", 12)
    },
    Step {
        equip: &[(1173, Hands::Both)],
        ..step("equipment bAspdRate +8 (Muramasa_C)", 7)
    },
    Step {
        equip: &[(1165, Hands::Both)],
        ..step("equipment bAspd +2 flat (Masamune)", 7)
    },
    Step {
        equip: &[(1370, Hands::Both)],
        ..step("equipment bAspdRate -40 (Doom Slayer)", 7)
    },
    Step {
        equip: &[(1101, Hands::Right), (28901, Hands::Left)],
        ..step("flat bAspd +3 (shield)", 7)
    },
    Step {
        equip: &[(1101, Hands::Right), (28332, Hands::AccessoryLeft), (28332, Hands::AccessoryRight)],
        ..step("flat bAspd +2 x2 (two rings)", 7)
    },
    Step {
        equip: &[(1101, Hands::Right), (22041, Hands::Shoes), (18852, Hands::Head)],
        ..step("flat bAspd +2 +2 (shoes, hat)", 7)
    },
    // Statuses: potions and buffs.
    Step {
        equip: &[(1101, Hands::Right)],
        use_items: &[645],
        ..step("status Center Potion (+4)", 7)
    },
    Step {
        equip: &[(1101, Hands::Right)],
        use_items: &[656],
        ..step("status Awakening Potion (+6)", 7)
    },
    Step {
        equip: &[(1101, Hands::Right)],
        use_items: &[657],
        ..step("status Berserk Potion (+9)", 7)
    },
    Step {
        equip: &[(1116, Hands::Both)],
        commands: &["@allskill"],
        casts: &[(60, 10)],
        measure_before: true,
        ..step("status Two-Hand Quicken (Knight, katana)", 7)
    },
    Step {
        commands: &["@allskill"],
        casts: &[(29, 10)],
        measure_before: true,
        ..step("status Increase AGI (Priest)", 8)
    },
    Step {
        equip: &[(1301, Hands::Right)],
        commands: &["@allskill"],
        casts: &[(111, 5)],
        measure_before: true,
        ..step("status Adrenaline Rush (Blacksmith, axe)", 10)
    },
    // Mounted Knight, without and then with Cavalier Mastery.
    Step {
        measure_before: true,
        commands: &["@mount", "@allskill", "@mount", "@mount"],
        ..step("mount Peco (Knight)", 7)
    },
    // The class cap: stats at the class limit plus flat gear.
    Step {
        extra_agi: 45,
        extra_dex: 45,
        ..step("cap Kagerou bare hands, stat cap", 4211)
    },
    Step {
        extra_agi: 45,
        extra_dex: 45,
        equip: &[
            (28332, Hands::AccessoryLeft),
            (28332, Hands::AccessoryRight),
            (22041, Hands::Shoes),
            (18852, Hands::Head),
            (28901, Hands::Left),
        ],
        ..step("cap Kagerou with +11 flat gear (cap 190)", 4211)
    },
    Step {
        extra_agi: 50,
        extra_dex: 50,
        ..step("cap Sura bare hands, stat cap", 4070)
    },
    Step {
        extra_agi: 50,
        extra_dex: 50,
        equip: &[
            (28332, Hands::AccessoryLeft),
            (28332, Hands::AccessoryRight),
            (22041, Hands::Shoes),
            (18852, Hands::Head),
            (28901, Hands::Left),
        ],
        ..step("cap Sura with +11 flat gear (cap 193)", 4070)
    },
    // Whips need a female character.
    Step {
        character: "GtAspdF",
        equip: &[(1950, Hands::Right)],
        ..step("whip (female Dancer)", 20)
    },
];

/// Second round, after the first showed what needed redoing: mounts measured
/// at each toggle, the class cap reached with the Spoon (a constant +10 flat
/// bonus) on a third class, and the whip on a level-99 female Dancer. Each
/// step's bare "pre" reading shows whether a status from an earlier step (a
/// potion lasts 30 minutes and is saved with the character) is still active.
const MODIFIER_STEPS_2: &[Step] = &[
    Step {
        measure_before: true,
        measure_each: true,
        commands: &["@skreset", "@mount", "@mount", "@allskill", "@mount"],
        ..step("mount Peco toggles (Knight, bare hands)", 7)
    },
    Step {
        measure_before: true,
        extra_agi: 5,
        extra_dex: 5,
        equip: &[(16039, Hands::Right)],
        ..step("cap Sura with the Spoon (+10 flat)", 4070)
    },
    Step {
        measure_before: true,
        equip: &[(16039, Hands::Right), (28901, Hands::Left)],
        ..step("cap Sura with the Spoon and shield", 4070)
    },
    Step {
        measure_before: true,
        equip: &[(16039, Hands::Right)],
        ..step("Champion with the Spoon (cap 190, no clamp expected)", 4016)
    },
    Step {
        character: "GtAspdF",
        equip: &[(1950, Hands::Right)],
        measure_before: true,
        ..step("whip (female Dancer)", 20)
    },
];

fn aspd_modifiers_ground_truth(config: &Config) -> Result<(), String> {
    run_modifier_steps(config, MODIFIER_STEPS)
}

fn aspd_modifiers_ground_truth_2(config: &Config) -> Result<(), String> {
    run_modifier_steps(config, MODIFIER_STEPS_2)
}

fn run_modifier_steps(config: &Config, steps: &[Step]) -> Result<(), String> {
    let mut failures = Vec::new();
    for step in steps {
        match run_modifier_step(config, step) {
            Ok(()) => {}
            Err(error) => {
                println!("ASPD-MOD-FAILED {}: {error}", step.label);
                failures.push(format!("{}: {error}", step.label));
            }
        }
        sleep(Duration::from_secs(3));
    }
    match failures.is_empty() {
        true => Ok(()),
        false => Err(format!("{} steps failed, first: {}", failures.len(), failures[0])),
    }
}

fn motion_line(context: &TestContext, step: &Step, phase: &str) -> String {
    format!(
        "ASPD-MOD {{\"step\":\"{}\",\"phase\":\"{phase}\",\"job\":{},\"agi\":[{},{}],\"dex\":[{},{}],\"amotion\":{}}}",
        step.label, context.job_id.0, context.agility.0, context.agility.1, context.dexterity.0, context.dexterity.1, context.attack_speed
    )
}

fn run_modifier_step(config: &Config, step: &Step) -> Result<(), String> {
    if step.character == "GtAspdF" {
        crate::context::CREATE_AS_FEMALE.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    let connected = TestContext::connect_as(
        config,
        &config.username,
        &config.password,
        Some(step.character),
        Some(step.character),
    );
    crate::context::CREATE_AS_FEMALE.store(false, std::sync::atomic::Ordering::SeqCst);
    let mut context = connected?;

    let mut worn: Vec<(InventoryIndex, u32)> = Vec::new();
    let result = (|| {
        // Several items need an equip level a fresh character does not have.
        context.ensure_base_level(99)?;
        context.ensure_job(step.job_id)?;
        if step.extra_agi > 0 {
            context.say(&format!("@agi {}", step.extra_agi))?;
        }
        if step.extra_dex > 0 {
            context.say(&format!("@dex {}", step.extra_dex))?;
        }
        context.pump(Duration::from_secs(2));
        for item in step.equip {
            let index = equip(&mut context, *item)?;
            worn.push((index, item.0));
        }
        for item_id in step.use_items {
            let index = context.give_item(*item_id, 1)?;
            context.flush();
            let account = context.account_id;
            context.net.use_item(index, account).map_err(|_| "disconnected".to_owned())?;
            context.pump(Duration::from_secs(2));
        }
        context.pump(Duration::from_millis(1500));
        if step.measure_before {
            println!("{}", motion_line(&context, step, "pre"));
        }
        for command in step.commands {
            context.say(command)?;
            context.pump(Duration::from_millis(1500));
            if step.measure_each {
                println!("{}", motion_line(&context, step, &format!("after {command}")));
            }
        }
        for (skill_id, level) in step.casts {
            context.say("@heal")?;
            context.say(&format!("@useskill {skill_id} {level} self"))?;
            context.pump(Duration::from_secs(3));
        }
        context.pump(Duration::from_millis(1500));
        println!("{}", motion_line(&context, step, "post"));
        Ok::<(), String>(())
    })();
    // Always take the worn items off, so they do not leak into the next step.
    for (index, item_id) in worn {
        context.flush();
        let _ = context.net.request_item_unequip(index);
        context.pump(Duration::from_millis(500));
        let _ = context.say(&format!("@delitem {item_id} 1"));
        context.pump(Duration::from_millis(300));
    }
    result
}

/// A mounted Knight's ASPD penalty is `500 - 100 x Cavalier Mastery` per mille
/// taken off the ASPD value, so it needs Riding *without* full Cavalier
/// Mastery. `@allskill` grants level 5 (no penalty), so skills are learned one
/// point at a time instead: reset, give skill points, learn Riding once, mount,
/// measure, then raise Cavalier Mastery a level at a time and measure again.
fn aspd_mount_ground_truth(config: &Config) -> Result<(), String> {
    const RIDING: u16 = 63;
    const CAVALIER_MASTERY: u16 = 64;
    const PROVOKE: u16 = 6;
    const ENDURE: u16 = 8;
    let mut context = TestContext::connect_as(config, &config.username, &config.password, Some("GtAspd"), Some("GtAspd"))?;
    context.ensure_base_level(99)?;
    context.ensure_job(7)?;
    let line = |context: &TestContext, phase: &str| {
        println!(
            "ASPD-MOUNT {{\"phase\":\"{phase}\",\"agi\":[{},{}],\"dex\":[{},{}],\"amotion\":{}}}",
            context.agility.0, context.agility.1, context.dexterity.0, context.dexterity.1, context.attack_speed
        );
    };
    context.say("@skreset")?;
    context.pump(Duration::from_secs(2));
    context.say("@skpoint 30")?;
    context.pump(Duration::from_secs(1));
    // Riding needs Endure 1, which needs Provoke 5 (from the Knight skill tree).
    for (skill, times) in [(PROVOKE, 5), (ENDURE, 1), (RIDING, 1)] {
        for _ in 0..times {
            context.net.level_up_skill(SkillId(skill)).map_err(|_| "disconnected".to_owned())?;
            context.pump(Duration::from_millis(800));
        }
    }
    context.pump(Duration::from_secs(1));
    line(&context, "riding 1, on foot");
    context.say("@mount")?;
    context.pump(Duration::from_secs(2));
    line(&context, "riding 1, cavalier 0, mounted");
    for level in 1..=5u32 {
        context
            .net
            .level_up_skill(SkillId(CAVALIER_MASTERY))
            .map_err(|_| "disconnected".to_owned())?;
        context.pump(Duration::from_secs(2));
        line(&context, &format!("riding 1, cavalier {level}, mounted"));
    }
    context.say("@mount")?;
    context.pump(Duration::from_secs(2));
    line(&context, "dismounted");
    Ok(())
}
