//! Provisioning scenarios: they leave persistent state behind on purpose and
//! are never part of `--scenario all`. Name one to run it.

use std::thread::sleep;
use std::time::Duration;

use crate::context::{Config, TestContext};
use crate::scenarios::{PROVISIONING_PHASE, Scenario};

pub fn scenarios() -> Vec<Scenario> {
    vec![Scenario::new("status-ground-truth", PROVISIONING_PHASE, status_ground_truth)]
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
