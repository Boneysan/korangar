mod combat;
mod dialogue;
mod dm;
mod gm;
mod items;
mod movement;
mod observer;
mod session;
mod skills;
mod social;

use crate::context::Config;

pub struct Scenario {
    pub name: &'static str,
    pub phase: u8,
    pub run: fn(&Config) -> Result<(), String>,
    /// Open findings-log entry this scenario is expected to fail on. Such a
    /// failure is reported as KNOWN-FAIL and does not affect the exit code;
    /// an unexpected PASS is flagged so the entry gets closed.
    pub known_issue: Option<&'static str>,
}

impl Scenario {
    pub const fn new(name: &'static str, phase: u8, run: fn(&Config) -> Result<(), String>) -> Self {
        Self {
            name,
            phase,
            run,
            known_issue: None,
        }
    }
}

pub fn all_scenarios() -> Vec<Scenario> {
    let mut scenarios = Vec::new();
    scenarios.extend(session::scenarios());
    scenarios.extend(gm::scenarios());
    scenarios.extend(movement::scenarios());
    scenarios.extend(combat::scenarios());
    scenarios.extend(skills::scenarios());
    scenarios.extend(items::scenarios());
    scenarios.extend(dialogue::scenarios());
    scenarios.extend(social::scenarios());
    scenarios.extend(dm::scenarios());
    scenarios.extend(observer::scenarios());
    scenarios
}
