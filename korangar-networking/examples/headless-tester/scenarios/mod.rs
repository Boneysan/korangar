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

/// Marker prefix for a scenario that could not run its assertions.
///
/// `Scenario::run` is `fn(&Config) -> Result<(), String>` and 114 functions share
/// that signature, so a third outcome as a return *type* would touch every one of
/// them. A skip is instead an `Err` whose message starts with this prefix, which
/// the runner classifies separately from a real failure.
///
/// **It is an `Err` and not an `Ok` on purpose.** The previous code returned
/// `Ok(())` for a skip, so a skipped scenario printed "skipped" and was still
/// tallied as PASS — which is how `skills-dancer` and `skills-gypsy` sat red
/// behind a green "114/114" for weeks. Putting a skip on the failure side means
/// the worst case is a visible amber row, never a false green. A skip does not
/// affect the exit code, so it does not break the gate either.
pub const SKIPPED_PREFIX: &str = "SKIPPED: ";

/// Report that a scenario could not run — a precondition the harness cannot
/// establish, as opposed to a defect in the code under test.
pub fn skipped(reason: impl std::fmt::Display) -> Result<(), String> {
    Err(format!("{SKIPPED_PREFIX}{reason}"))
}

/// Whether a scenario result is a skip rather than a genuine failure.
pub fn is_skip(result: &Result<(), String>) -> bool {
    matches!(result, Err(message) if message.starts_with(SKIPPED_PREFIX))
}

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
