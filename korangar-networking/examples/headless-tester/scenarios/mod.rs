mod combat;
mod dialogue;
mod dm;
mod gm;
mod items;
mod movement;
mod observer;
mod session;
/// Generated from Hercules' own `skill_db.conf` — see
/// `tools/generate_skill_expectations.py`. Regenerate after any skill_db change.
mod skill_expectations;
pub mod skills;
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
/// the worst case is a visible row, never a false green. Only exact, reviewed
/// entries in [`EXPECTED_SKIPS`] avoid failing the gate.
pub const SKIPPED_PREFIX: &str = "SKIPPED: ";

/// Skips which are intentional for the checked-in server fixture.
///
/// A skip outside this list is a regression: it means a scenario silently
/// stopped asserting something. Reasons are exact-match on purpose so a changed
/// precondition has to be reviewed instead of inheriting an old exemption.
pub const EXPECTED_SKIPS: &[(&str, &str)] = &[(
    "skills-novice",
    "Novice has no castable skills — its actives are quest-gated",
)];

/// Report that a scenario could not run — a precondition the harness cannot
/// establish, as opposed to a defect in the code under test.
pub fn skipped(reason: impl std::fmt::Display) -> Result<(), String> {
    Err(format!("{SKIPPED_PREFIX}{reason}"))
}

/// Whether a scenario result is a skip rather than a genuine failure.
pub fn is_skip(result: &Result<(), String>) -> bool {
    matches!(result, Err(message) if message.starts_with(SKIPPED_PREFIX))
}

/// Whether a scenario's skip reason exactly matches the reviewed baseline.
pub fn is_expected_skip(name: &str, result: &Result<(), String>) -> bool {
    let reason = match result {
        Err(message) => message.strip_prefix(SKIPPED_PREFIX),
        Ok(()) => None,
    };
    reason.is_some_and(|reason| EXPECTED_SKIPS.contains(&(name, reason)))
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

#[cfg(test)]
mod tests {
    use super::{is_expected_skip, skipped};

    #[test]
    fn expected_skips_require_the_exact_reviewed_reason() {
        assert!(is_expected_skip(
            "skills-novice",
            &skipped("Novice has no castable skills — its actives are quest-gated")
        ));
        assert!(!is_expected_skip(
            "skills-novice",
            &skipped("Novice has no castable skills")
        ));
        assert!(!is_expected_skip(
            "skills-mage",
            &skipped("Novice has no castable skills — its actives are quest-gated")
        ));
    }
}
