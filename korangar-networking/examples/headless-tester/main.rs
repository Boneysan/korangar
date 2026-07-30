//! Headless protocol tester: runs scripted scenarios against a live local
//! Hercules server, asserting on the `NetworkEvent`s the shared networking
//! stack produces. See `tools/testing/headless_test_plan.md` for the plan and
//! `tools/testing/headless_findings.md` for the bug workflow.

mod context;
mod ledger;
mod scenarios;

use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::Parser;
use korangar_debug::logging::Colorize;

use crate::context::{CONNECTION_ERROR, Config};
use crate::ledger::Ledger;
use crate::scenarios::{SKIPPED_PREFIX, all_scenarios, is_skip};

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
        username: arguments.username,
        password: arguments.password,
        character: arguments.character,
        partner_username: arguments.partner_username,
        partner_password: arguments.partner_password,
        timeout: Duration::from_secs(arguments.timeout),
        ledger: ledger.clone(),
    };

    let mut results = Vec::new();

    for scenario in &selected {
        println!("\n[{}] {} (phase {})", "Running".yellow(), scenario.name, scenario.phase);
        let start = Instant::now();
        let mut result = (scenario.run)(&config);

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
        // retries, the retry is printed, and the ledger gate is untouched — a
        // genuine desync still fails the run through `ledger.failed_count()`.
        // If these lines start appearing regularly, that is a real bug asking
        // to be investigated, not something to raise the retry count for.
        if matches!(&result, Err(message) if message.starts_with(CONNECTION_ERROR)) {
            println!(
                "[{}] {} hit a map-server connection error — retrying once",
                "Retry".yellow(),
                scenario.name
            );
            std::thread::sleep(Duration::from_secs(3));
            result = (scenario.run)(&config);
        }

        let elapsed = start.elapsed();

        match (&result, scenario.known_issue) {
            // A skip is checked before everything else: it means the scenario
            // never got to assert anything, so neither PASS nor FAIL is honest.
            _ if is_skip(&result) => {
                let reason = result.as_ref().err().map_or("", |message| message.trim_start_matches(SKIPPED_PREFIX));
                println!("[{}] {}: {} ({:.1?})", "SKIP".yellow(), scenario.name, reason, elapsed);
            }
            (Ok(()), None) => println!("[{}] {} ({:.1?})", "PASS".green(), scenario.name, elapsed),
            (Ok(()), Some(issue)) => {
                println!(
                    "[{}] {} passed but is marked as a known issue — close it in headless_findings.md! ({issue})",
                    "PASS".green(),
                    scenario.name
                );
            }
            (Err(message), None) => println!("[{}] {}: {} ({:.1?})", "FAIL".red(), scenario.name, message, elapsed),
            (Err(_), Some(issue)) => {
                println!("[{}] {}: {} ({:.1?})", "KNOWN-FAIL".yellow(), scenario.name, issue, elapsed);
            }
        }
        results.push((scenario.name, result, scenario.known_issue));

        // Give the server a moment to fully drop the session before the next
        // scenario logs in with the same account.
        std::thread::sleep(Duration::from_millis(700));
    }

    // Skips are counted on their own and excluded from "passed". Folding them
    // into the pass count is exactly what hid two permanently-red scenarios
    // behind a green 114/114 — see `SKIPPED_PREFIX`.
    let skips: Vec<_> = results.iter().filter(|(_, result, _)| is_skip(result)).collect();
    let failures: Vec<_> = results
        .iter()
        .filter(|(_, result, known_issue)| result.is_err() && known_issue.is_none() && !is_skip(result))
        .collect();
    let known_fails = results
        .iter()
        .filter(|(_, result, known_issue)| result.is_err() && known_issue.is_some() && !is_skip(result))
        .count();

    println!(
        "\n=== Summary: {} passed, {} failed, {} skipped, {} known-fail{} ===",
        results.len() - failures.len() - known_fails - skips.len(),
        failures.len(),
        skips.len(),
        known_fails,
        match arguments.shuffle {
            // Repeated in the summary so a failure pasted from the tail of a
            // long run still carries the seed needed to reproduce it.
            Some(seed) => format!(" (shuffled, seed {seed})"),
            None => String::new(),
        }
    );
    for (name, result, known_issue) in &results {
        match (result, known_issue) {
            _ if is_skip(result) => println!("  {} {}", "SKIP".yellow(), name),
            (Ok(()), _) => println!("  {} {}", "PASS".green(), name),
            (Err(_), Some(_)) => println!("  {} {}", "KNOWN-FAIL".yellow(), name),
            (Err(_), None) => println!("  {} {}", "FAIL".red(), name),
        }
    }

    if !skips.is_empty() {
        println!(
            "[{}] {} scenario(s) skipped — they asserted nothing, so treat the pass count accordingly",
            "Note".yellow(),
            skips.len()
        );
    }

    if arguments.report_packets || selected.len() > 1 {
        println!("{}", ledger.report());
    }

    if ledger.failed_count() > 0 {
        println!(
            "[{}] {} packet deserialization failure(s) — document in headless_findings.md",
            "Error".red(),
            ledger.failed_count()
        );
        return ExitCode::FAILURE;
    }

    match failures.is_empty() {
        true => ExitCode::SUCCESS,
        false => ExitCode::from(failures.len().min(255) as u8),
    }
}
