//! Headless protocol tester: runs scripted scenarios against a live local
//! Hercules server, asserting on the `NetworkEvent`s the shared networking
//! stack produces. See `tools/testing/headless_test_plan.md` for the plan and
//! `tools/testing/headless_findings.md` for the bug workflow.

mod context;
mod ledger;
mod scenarios;

use std::fs::File;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::Parser;
use korangar_debug::logging::Colorize;
use serde::Serialize;

use crate::context::{CONNECTION_ERROR, Config, clear_recorded_connection_error, take_recorded_connection_error};
use crate::ledger::{Ledger, PacketCoverageSummary};
use crate::scenarios::{SKIPPED_PREFIX, Scenario, all_scenarios, is_expected_skip, is_skip};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Address of the login server.
    #[arg(short, long, default_value = "127.0.0.1:6900")]
    server: SocketAddr,

    /// Username of the (GM, group_id 99) test account.
    #[arg(short, long, default_value = "korangar")]
    username: String,

    /// Password of the test account.
    #[arg(short, long, default_value = "korangar")]
    password: String,

    /// Character name to use. Defaults to the first character.
    #[arg(short, long)]
    character: Option<String>,

    /// Second (non-GM) account for multi-client scenarios; created via the
    /// `_M` registration trick on first use if it does not exist.
    #[arg(long, default_value = "headless2")]
    partner_username: String,

    #[arg(long, default_value = "headless2pw")]
    partner_password: String,

    /// Per-wait timeout in seconds.
    #[arg(short, long, default_value_t = 15)]
    timeout: u64,

    /// Scenario name, "phaseN", or "all". Use --list to see names.
    #[arg(long, default_value = "smoke")]
    scenario: String,

    /// List available scenarios and exit.
    #[arg(long, default_value_t = false)]
    list: bool,

    /// Print the packet coverage ledger even for single scenarios.
    #[arg(long, default_value_t = false)]
    report_packets: bool,

    /// Run the selected scenarios in a shuffled order, to expose ones that only
    /// pass because of state an earlier scenario happened to leave behind.
    ///
    /// All 114 scenarios share one test character, so order dependence is the
    /// suite's structural weak point — and the existing double-run gate cannot
    /// see it, because it runs the same order twice. The seed is printed and
    /// reproduces the exact order.
    #[arg(long)]
    shuffle: Option<u64>,

    /// Write a machine-readable result artifact for CI and run comparison.
    #[arg(long)]
    results_json: Option<PathBuf>,

    /// Treat a scenario which only passed on retry as a gate failure.
    #[arg(long, default_value_t = false)]
    fail_on_flaky: bool,
}

struct ScenarioExecution<'a> {
    scenario: &'a Scenario,
    result: Result<(), String>,
    elapsed: Duration,
    retried: bool,
    expected_skip: bool,
}

#[derive(Serialize)]
struct JsonScenarioResult<'a> {
    name: &'a str,
    phase: u8,
    outcome: &'static str,
    message: Option<&'a str>,
    known_issue: Option<&'a str>,
    duration_ms: u64,
    retried: bool,
}

#[derive(Serialize)]
struct JsonRunResults<'a> {
    schema_version: u8,
    client_revision: Option<String>,
    hercules_revision: Option<String>,
    shuffle_seed: Option<u64>,
    scenarios: Vec<JsonScenarioResult<'a>>,
    packet_coverage: PacketCoverageSummary,
}

/// Fisher-Yates using a hand-rolled xorshift64 PRNG.
///
/// Deterministic on purpose: an order-dependent failure is unactionable unless
/// the exact order can be replayed with the same `--shuffle <seed>`. Rolled by
/// hand because this crate has no `rand` dependency and a test-ordering shuffle
/// does not justify adding one.
fn shuffle_deterministically<T>(items: &mut [T], seed: u64) {
    // xorshift64 is degenerate when seeded with zero, so force a set bit.
    let mut state = seed | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for index in (1..items.len()).rev() {
        let swap_with = (next() % (index as u64 + 1)) as usize;
        items.swap(index, swap_with);
    }
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    let scenarios = all_scenarios();

    if arguments.list {
        // Lists every scenario regardless of `--scenario`, since the point is
        // to discover names. `--shuffle` is still honoured so an order can be
        // previewed — and verified — without running anything.
        let mut listed: Vec<_> = scenarios.iter().collect();
        if let Some(seed) = arguments.shuffle {
            shuffle_deterministically(&mut listed, seed);
        }
        for scenario in listed {
            println!("phase {}  {}", scenario.phase, scenario.name);
        }
        return ExitCode::SUCCESS;
    }

    let mut selected: Vec<_> = scenarios
        .iter()
        .filter(|scenario| match arguments.scenario.as_str() {
            "all" => true,
            name if name.starts_with("phase") => name
                .strip_prefix("phase")
                .and_then(|number| number.parse::<u8>().ok())
                .is_some_and(|number| scenario.phase == number),
            name => scenario.name == name,
        })
        .collect();

    if selected.is_empty() {
        println!(
            "[{}] No scenario matches \"{}\" (use --list)",
            "Error".red(),
            arguments.scenario
        );
        return ExitCode::FAILURE;
    }

    if let Some(seed) = arguments.shuffle {
        shuffle_deterministically(&mut selected, seed);
        println!("[{}] seed {seed} — replay this exact order with --shuffle {seed}", "Shuffled".yellow());
    }

    let ledger = Ledger::default();
    let config = Config {
        server: arguments.server,
        username: arguments.username.clone(),
        password: arguments.password.clone(),
        character: arguments.character.clone(),
        partner_username: arguments.partner_username.clone(),
        partner_password: arguments.partner_password.clone(),
        timeout: Duration::from_secs(arguments.timeout),
        ledger: ledger.clone(),
    };

    let mut results = Vec::new();

    for scenario in &selected {
        println!("\n[{}] {} (phase {})", "Running".yellow(), scenario.name, scenario.phase);
        clear_recorded_connection_error();
        let observations_before_attempt = scenarios::skills::observation_checkpoint();
        let start = Instant::now();
        let mut result = (scenario.run)(&config);
        if let Some(connection_error) = take_recorded_connection_error() {
            result = Err(connection_error);
        }
        let mut retried = false;

        // Retry once, loudly, if the map server dropped the session mid-scenario.
        //
        // This is a MITIGATION, not a root cause. It appeared once in five full
        // runs (`skills-rogue`, seed 1337), and the evidence says it is not a
        // defect in the code under test: the ledger recorded **0** packet
        // deserialization failures, the next scenario connected fine, and the
        // scenario passed standalone straight afterwards. `connect_as` already
        // retries the *login* four times, so this is a disconnect during the
        // run, not a session-teardown race.
        //
        // It is deliberately narrow and deliberately noisy. Only this one error
        // retries, a recovered attempt is FLAKY-PASS (and fails strict CI), and
        // the ledger gate is untouched — a genuine desync still fails the run
        // through `ledger.failed_count()`. If these lines start appearing
        // regularly, that is a real bug asking to be investigated, not something
        // to raise the retry count for.
        if matches!(&result, Err(message) if message.starts_with(CONNECTION_ERROR)) {
            retried = true;
            println!(
                "[{}] {} hit a map-server connection error — retrying once",
                "Retry".yellow(),
                scenario.name
            );
            scenarios::skills::restore_observation_checkpoint(observations_before_attempt);
            clear_recorded_connection_error();
            std::thread::sleep(Duration::from_secs(3));
            result = (scenario.run)(&config);
            if let Some(connection_error) = take_recorded_connection_error() {
                result = Err(connection_error);
            }
        }

        let elapsed = start.elapsed();
        let expected_skip = is_expected_skip(scenario.name, &result);

        match (&result, scenario.known_issue) {
            // A skip is checked before everything else: it means the scenario
            // never got to assert anything, so neither PASS nor FAIL is honest.
            _ if expected_skip => {
                let reason = result.as_ref().err().map_or("", |message| message.trim_start_matches(SKIPPED_PREFIX));
                println!("[{}] {}: {} ({:.1?})", "EXPECTED-SKIP".yellow(), scenario.name, reason, elapsed);
            }
            _ if is_skip(&result) => {
                let reason = result.as_ref().err().map_or("", |message| message.trim_start_matches(SKIPPED_PREFIX));
                println!("[{}] {}: {} ({:.1?})", "UNEXPECTED-SKIP".red(), scenario.name, reason, elapsed);
            }
            (Ok(()), None) if retried => {
                println!("[{}] {} ({:.1?}) — passed only after a connection retry", "FLAKY-PASS".yellow(), scenario.name, elapsed)
            }
            (Ok(()), None) => println!("[{}] {} ({:.1?})", "PASS".green(), scenario.name, elapsed),
            (Ok(()), Some(issue)) => {
                println!(
                    "[{}] {} passed but is marked as a known issue — close it in headless_findings.md! ({issue})",
                    "UNEXPECTED-PASS".yellow(),
                    scenario.name
                );
            }
            (Err(message), None) => println!("[{}] {}: {} ({:.1?})", "FAIL".red(), scenario.name, message, elapsed),
            (Err(_), Some(issue)) => {
                println!("[{}] {}: {} ({:.1?})", "KNOWN-FAIL".yellow(), scenario.name, issue, elapsed);
            }
        }
        results.push(ScenarioExecution {
            scenario,
            result,
            elapsed,
            retried,
            expected_skip,
        });

        // Give the server a moment to fully drop the session before the next
        // scenario logs in with the same account.
        std::thread::sleep(Duration::from_millis(700));
    }

    let expected_skips = results.iter().filter(|execution| execution.expected_skip).count();
    let unexpected_skips = results
        .iter()
        .filter(|execution| is_skip(&execution.result) && !execution.expected_skip)
        .count();
    let failures = results
        .iter()
        .filter(|execution| {
            execution.result.is_err() && execution.scenario.known_issue.is_none() && !is_skip(&execution.result)
        })
        .count();
    let known_fails = results
        .iter()
        .filter(|execution| {
            execution.result.is_err() && execution.scenario.known_issue.is_some() && !is_skip(&execution.result)
        })
        .count();
    let flaky_passes = results
        .iter()
        .filter(|execution| execution.result.is_ok() && execution.retried && execution.scenario.known_issue.is_none())
        .count();
    let passed = results.len() - failures - known_fails - expected_skips - unexpected_skips - flaky_passes;

    println!(
        "\n=== Summary: {} passed, {} flaky-pass, {} failed, {} expected-skip, {} unexpected-skip, {} known-fail{} ===",
        passed,
        flaky_passes,
        failures,
        expected_skips,
        unexpected_skips,
        known_fails,
        match arguments.shuffle {
            // Repeated in the summary so a failure pasted from the tail of a
            // long run still carries the seed needed to reproduce it.
            Some(seed) => format!(" (shuffled, seed {seed})"),
            None => String::new(),
        }
    );
    for execution in &results {
        match (&execution.result, execution.scenario.known_issue) {
            _ if execution.expected_skip => println!("  {} {}", "EXPECTED-SKIP".yellow(), execution.scenario.name),
            _ if is_skip(&execution.result) => println!("  {} {}", "UNEXPECTED-SKIP".red(), execution.scenario.name),
            (Ok(()), Some(_)) => println!("  {} {}", "UNEXPECTED-PASS".yellow(), execution.scenario.name),
            (Ok(()), None) if execution.retried => println!("  {} {}", "FLAKY-PASS".yellow(), execution.scenario.name),
            (Ok(()), None) => println!("  {} {}", "PASS".green(), execution.scenario.name),
            (Err(_), Some(_)) => println!("  {} {}", "KNOWN-FAIL".yellow(), execution.scenario.name),
            (Err(_), None) => println!("  {} {}", "FAIL".red(), execution.scenario.name),
        }
    }

    if expected_skips > 0 {
        println!(
            "[{}] {} reviewed scenario(s) skipped — they are visible and excluded from the pass count",
            "Note".yellow(),
            expected_skips
        );
    }

    print_what_the_run_proved();

    if arguments.report_packets || selected.len() > 1 {
        println!("{}", ledger.report());
    }

    if let Some(path) = &arguments.results_json {
        if let Err(error) = write_json_results(path, &results, arguments.shuffle, ledger.summary()) {
            println!("[{}] could not write {}: {error}", "Error".red(), path.display());
            return ExitCode::FAILURE;
        }
        println!("[{}] {}", "Results".green(), path.display());
    }

    if ledger.failed_count() > 0 {
        println!(
            "[{}] {} packet deserialization failure(s) — document in headless_findings.md",
            "Error".red(),
            ledger.failed_count()
        );
        return ExitCode::FAILURE;
    }

    let gate_failures = failures + unexpected_skips + usize::from(arguments.fail_on_flaky && flaky_passes > 0);
    match gate_failures {
        0 => ExitCode::SUCCESS,
        count => ExitCode::from(count.min(255) as u8),
    }
}

fn json_outcome(execution: &ScenarioExecution<'_>) -> &'static str {
    match (&execution.result, execution.scenario.known_issue) {
        _ if execution.expected_skip => "expected-skip",
        _ if is_skip(&execution.result) => "unexpected-skip",
        (Ok(()), Some(_)) => "unexpected-pass",
        (Ok(()), None) if execution.retried => "flaky-pass",
        (Ok(()), None) => "pass",
        (Err(_), Some(_)) => "known-fail",
        (Err(_), None) => "fail",
    }
}

fn write_json_results(
    path: &Path,
    results: &[ScenarioExecution<'_>],
    shuffle_seed: Option<u64>,
    packet_coverage: PacketCoverageSummary,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }

    let scenarios = results
        .iter()
        .map(|execution| JsonScenarioResult {
            name: execution.scenario.name,
            phase: execution.scenario.phase,
            outcome: json_outcome(execution),
            message: execution.result.as_ref().err().map(String::as_str),
            known_issue: execution.scenario.known_issue,
            duration_ms: execution.elapsed.as_millis().min(u64::MAX as u128) as u64,
            retried: execution.retried,
        })
        .collect();
    let artifact = JsonRunResults {
        schema_version: 1,
        client_revision: std::env::var("KORANGAR_CLIENT_REVISION").ok(),
        hercules_revision: std::env::var("KORANGAR_HERCULES_REVISION").ok(),
        shuffle_seed,
        scenarios,
        packet_coverage,
    };

    serde_json::to_writer_pretty(File::create(path)?, &artifact)?;
    Ok(())
}

/// The "so what" block: what this run actually established, as opposed to the
/// fact that it was green.
///
/// Three things a pass/fail count cannot say, each of which cost real time on
/// 2026-08-09:
///
/// 1. **What the skill sweep observed.** Its pass condition is "some observable
///    response arrived" — a liveness check that catches unregistered and
///    misparsed packets, which is what it is for. It is not a correctness
///    check, and "39 job sweeps, green" reads like one. When a third of the
///    observations are the server refusing the skill, the reader should be told.
/// 2. **Allowlist entries that did not earn their place.** A stale entry
///    silently absorbs a future regression in that skill; 43 of 81 were found
///    dead in one audit.
/// 3. **Scenarios that got dramatically slower without failing.** `skills-mage`
///    ran 4x its usual time and pass/fail said nothing at all; it was caught by
///    eye, which is not a mechanism.
fn print_what_the_run_proved() {
    use scenarios::skills::{ALLOWLIST_OBSERVATIONS, EXPECTATION_VERDICTS, KNOWN_INTERMITTENT, SWEEP_OUTCOMES, Verdict};

    let outcomes = SWEEP_OUTCOMES.lock().map(|guard| guard.clone()).unwrap_or_default();
    if !outcomes.is_empty() {
        let mut tally: Vec<(&str, usize)> = {
            let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for (outcome, _) in &outcomes {
                *counts.entry(*outcome).or_default() += 1;
            }
            counts.into_iter().collect()
        };
        tally.sort_by(|a, b| b.1.cmp(&a.1));

        println!("\n=== What the skill sweep observed ({} casts) ===", outcomes.len());
        for (kind, count) in &tally {
            println!("  {count:5}  {:5.1}%  {kind}", 100.0 * *count as f64 / outcomes.len() as f64);
        }
        // Name the limitation rather than leaving it to be inferred from the
        // percentages, because the whole failure mode here is a number being
        // read as a stronger claim than it supports.
        let refused = tally
            .iter()
            .filter(|(kind, _)| kind.starts_with("fail-") || *kind == "passive" || kind.starts_with("silent"))
            .map(|(_, count)| *count)
            .sum::<usize>();
        println!(
            "  -> {:.0}% of these were a refusal, a passive skill, or accepted silence. A green sweep means\n     the wire is alive, NOT that the skills work.",
            100.0 * refused as f64 / outcomes.len() as f64
        );

        // **How weak the weakest evidence is, named rather than left to be
        // inferred.** `cast` means a bar started; `post-delay` means the server
        // charged an after-cast delay and does not even carry the skill id.
        // Neither says the skill did anything, and both used to be indexed as
        // ordinary outcomes alongside `damage`.
        let acknowledged = tally
            .iter()
            .filter(|(kind, _)| kind.starts_with("cast") || kind.starts_with("post-delay"))
            .map(|(_, count)| *count)
            .sum::<usize>();
        if acknowledged > 0 {
            println!(
                "  -> {acknowledged} were only an acknowledgement (`cast` / `post-delay`): the server took the\n     request and nothing observable followed it inside the window."
            );
        }

        // What the observation window actually bought. The sweep used to stop at
        // the first event it recognised; this is how many casts had something
        // after that first event — i.e. how many results the old model got wrong
        // or under-stated.
        //
        // Counted against *answered* casts, not all rows: passives are never
        // cast and silent ones produced nothing to look past, so including them
        // would understate the window against a denominator it never had a
        // chance at. An accuracy report that is itself inaccurate is worse than
        // none — this file has that lesson written into it twenty lines up.
        let answered = outcomes.iter().filter(|(_, observed)| *observed > 0).count();
        let deeper = outcomes.iter().filter(|(_, observed)| *observed > 1).count();
        if answered > 0 {
            println!(
                "  -> {deeper} of {answered} answered casts produced evidence past the first event the sweep\n     recognised. Those are the ones the old first-match model reported wrongly.",
            );
        }
    }

    // **What the sweep would prove if it asserted the derived expectations.**
    //
    // Report-only on purpose: this is tier 1b step 2 with the assertion left
    // off. The observation window made the comparison possible at all, and these
    // numbers are what decide whether turning it into a failure is honest yet.
    // A check that reddens working skills is worse than no check.
    if let Ok(verdicts) = EXPECTATION_VERDICTS.lock() {
        if !verdicts.is_empty() {
            let count = |wanted: Verdict| verdicts.iter().filter(|(_, verdict, _)| *verdict == wanted).count();
            let (met, refused, blocked, unmet) = (
                count(Verdict::Met),
                count(Verdict::Refused),
                count(Verdict::Blocked),
                count(Verdict::Unmet),
            );

            println!("\n=== Derived expectations, measured but NOT enforced ({} casts) ===", verdicts.len());
            println!("  {met:5}  met — the skill was seen doing what skill_db says it does");
            println!("  {refused:5}  refused — the server said no; a legitimate outcome the sweep cannot avoid");
            println!("  {blocked:5}  blocked — the skill opened a choice the sweep cannot answer");
            println!("  {unmet:5}  unmet — something came back, but not the promised observable");
            if unmet > 0 {
                println!("  Enforcing today would redden the {unmet} unmet. Listing the first 20:");
                for (name, _, detail) in verdicts.iter().filter(|(_, verdict, _)| *verdict == Verdict::Unmet).take(20) {
                    println!("    {name:22} {detail}");
                }
            }
        }
    }

    if let Ok(rows) = ALLOWLIST_OBSERVATIONS.lock() {
        // Safe to delete: answered in EVERY job that swept it, and not a known
        // cross-run intermittent (one run cannot see those going quiet).
        let dead: Vec<_> = rows
            .iter()
            .filter(|(name, _, silent, _)| *silent == 0 && !KNOWN_INTERMITTENT.contains(&name.as_str()))
            .collect();
        // Answered somewhere, silent somewhere else. **Deleting one of these
        // reddens the jobs where it is silent** — the mistake already on record.
        let partial: Vec<_> = rows.iter().filter(|(_, answered, silent, _)| *answered > 0 && *silent > 0).collect();

        if !dead.is_empty() {
            println!("\n=== Allowlist entries that did not earn their place ({}) ===", dead.len());
            println!("  Answered in EVERY job that swept them. Each is a loaded gun: it will absorb a");
            println!("  real regression in that skill without a word. Remove the entry, or record why");
            println!("  it must stay. Known cross-run intermittents are excluded.");
            for (name, answered, _, example) in dead {
                println!("    {name:22} answered {answered}/{answered} jobs, e.g. {example:?}");
            }
        }
        if !partial.is_empty() {
            println!("\n=== Allowlist entries that are load-bearing in SOME jobs ({}) ===", partial.len());
            println!("  These answered somewhere and went silent somewhere else, in this same run.");
            println!("  DO NOT delete them on the strength of the answer alone — that reddens the jobs");
            println!("  where they are silent, which is exactly how three load-bearing entries were");
            println!("  nearly culled before. `tools/audits/flaky.py` shows which job is which.");
            for (name, answered, silent, example) in partial {
                println!(
                    "    {name:22} answered {answered}, silent {silent} (of {} jobs), e.g. {example:?}",
                    answered + silent
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ScenarioExecution, json_outcome, shuffle_deterministically};
    use crate::scenarios::Scenario;

    #[test]
    fn shuffle_is_deterministic_for_a_seed() {
        let mut first: Vec<_> = (0..20).collect();
        let mut second = first.clone();
        shuffle_deterministically(&mut first, 20260810);
        shuffle_deterministically(&mut second, 20260810);

        assert_eq!(first, second);
        assert_ne!(first, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn zero_seed_is_not_degenerate() {
        let mut values: Vec<_> = (0..20).collect();
        shuffle_deterministically(&mut values, 0);

        assert_ne!(values, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn recovered_retry_is_not_reported_as_a_plain_pass() {
        let scenario = Scenario::new("retry-example", 1, |_| Ok(()));
        let execution = ScenarioExecution {
            scenario: &scenario,
            result: Ok(()),
            elapsed: Duration::from_millis(1),
            retried: true,
            expected_skip: false,
        };

        assert_eq!(json_outcome(&execution), "flaky-pass");
    }
}
