//! Diagnostics that outlive the window they were printed to.
//!
//! The client had no log file and no panic hook, so its console was the only
//! place any diagnostic existed. That is fine on a developer's machine and
//! useless everywhere else: a friend hits a problem, closes the terminal
//! (which also kills the game, since it owns the console), and there is
//! nothing left to send. Screenshotting a scrollback is not a support
//! workflow.
//!
//! So every unconditional `eprintln!`/`println!` in the client now goes
//! through [`client_log!`], which writes to **both** the console and
//! `korangar.log` beside the executable. The console half is deliberate while
//! the Windows pack is still unproven — see
//! `docs/plans/friends-distribution.md` section 9, "the console window". When
//! that is closed off with `windows_subsystem = "windows"`, this file is what
//! makes it safe to do.
//!
//! Panics land here too, via a hook that writes the payload and location
//! before the default handler prints it.

use std::fmt::Arguments;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// Written beside the executable, not the working directory: the launcher sets
/// the CWD for us, but a friend who starts the `.exe` directly should still
/// leave the log somewhere findable rather than wherever Explorer happened to
/// be.
const LOG_FILE_NAME: &str = "korangar.log";

/// One previous run is kept. A crash a friend reports an hour later is usually
/// the run *before* the one they are looking at, because the first thing
/// anybody does is try again.
const PREVIOUS_LOG_FILE_NAME: &str = "korangar.log.previous";

/// `None` when the file could not be opened — a read-only install directory,
/// a locked file, a sandbox. That is not worth refusing to start over, so the
/// client keeps running and the console keeps being the only sink.
static LOG_FILE: OnceLock<Option<Mutex<File>>> = OnceLock::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

fn log_directory() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|path| path.parent().map(Path::to_path_buf))
}

/// Where this run is writing. Also what the friend-facing instructions name.
pub fn log_path() -> PathBuf {
    if let Some(path) = LOG_PATH.get() {
        return path.clone();
    }
    match log_directory() {
        Some(directory) => directory.join(LOG_FILE_NAME),
        None => PathBuf::from(LOG_FILE_NAME),
    }
}

/// Opens the log and installs the panic hook. Safe to call more than once;
/// only the first call does anything.
///
/// Call this before anything that can fail, which in practice means first
/// thing in `main` — a panic during startup is exactly the one a friend cannot
/// otherwise report.
pub fn init() {
    LOG_FILE.get_or_init(|| {
        let preferred = log_path();
        let fallback = std::env::temp_dir().join("korangar").join(LOG_FILE_NAME);
        for path in [preferred, fallback] {
            if let Some(directory) = path.parent() {
                let _ = std::fs::create_dir_all(directory);
            }
            if path.exists() {
                let previous = path.with_file_name(PREVIOUS_LOG_FILE_NAME);
                // Windows rename does not replace the destination. Remove only
                // our older backup before rotating the most recent run.
                let _ = std::fs::remove_file(&previous);
                let _ = std::fs::rename(&path, previous);
            }
            match OpenOptions::new().create(true).write(true).truncate(true).open(&path) {
                Ok(mut file) => {
                    // No formatting libraries before the first persisted byte.
                    // A later native startup fault still leaves this breadcrumb.
                    let _ = file.write_all(b"[startup] log opened\n");
                    let _ = file.flush();
                    let _ = LOG_PATH.set(path);
                    return Some(Mutex::new(file));
                }
                Err(error) => {
                    let _ = writeln!(std::io::stderr().lock(), "[log] cannot open {}: {error}", path.display());
                }
            }
        }
        None
    });

    install_panic_hook();

    // Start the frame clock HERE, not lazily on the first frame. `OnceLock`
    // would otherwise anchor it to the first redraw request, and
    // "first frame rendered 0.1s after startup" would be measuring the wrong
    // interval -- observed reading 0.1s for a real gap of 1.0s.
    let _ = PROCESS_START.set(Instant::now());

    write_line(format_args!(
        "[korangar] {} starting -- crate {} pack {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        env!("CARGO_PKG_VERSION"),
        korangar_networking::PACK_VERSION
    ));
    write_line(format_args!(
        "[startup] platform={}/{} log={}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        log_path().display()
    ));
    write_line(format_args!(
        "[startup] executable={:?} working_directory={:?}",
        std::env::current_exe(),
        std::env::current_dir()
    ));
    write_line(format_args!(
        "[startup] cpus={}",
        std::thread::available_parallelism().map(|count| count.get()).unwrap_or(0)
    ));
    for name in [
        "WGPU_BACKEND",
        "WGPU_ADAPTER_NAME",
        "KORANGAR_ALLOW_SOFTWARE_RENDERING",
        "KORANGAR_PACKET_LOG",
    ] {
        write_line(format_args!(
            "[startup] {name}={}",
            std::env::var(name).unwrap_or_else(|_| "<default>".to_owned())
        ));
    }
}

/// The hook writes to the log and then defers to the default one, so a
/// developer watching the console still sees exactly what they saw before.
fn install_panic_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();

    INSTALLED.get_or_init(|| {
        let default_hook = std::panic::take_hook();

        std::panic::set_hook(Box::new(move |info| {
            let location = match info.location() {
                Some(location) => format!("{}:{}:{}", location.file(), location.line(), location.column()),
                None => "unknown location".to_string(),
            };

            // Deliberately NOT the shared Mutex: the panic may have happened
            // while that lock was held, and a deadlock inside a panic hook
            // would replace a readable crash with a hang.
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path()) {
                let _ = writeln!(
                    file,
                    "{} [panic] at {}: {}",
                    chrono::Local::now().format("%H:%M:%S%.3f"),
                    location,
                    payload_message(info)
                );
                let _ = writeln!(file, "{}", std::backtrace::Backtrace::force_capture());
                let _ = file.flush();
            }

            default_hook(info);
        }));
    });
}

fn payload_message(info: &std::panic::PanicHookInfo<'_>) -> String {
    let payload = info.payload();

    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

/// Writes one timestamped line to the log file and to stderr.
///
/// Reached through [`client_log!`] rather than called directly. Never panics
/// and never blocks on a poisoned lock: a diagnostic that can take the client
/// down with it is worse than a missing diagnostic.
pub fn write_line(arguments: Arguments<'_>) {
    if let Some(Some(file)) = LOG_FILE.get()
        && let Ok(mut file) = file.lock()
    {
        let _ = writeln!(file, "{} {}", chrono::Local::now().format("%H:%M:%S%.3f"), arguments);
        let _ = file.flush();
    }
    // A detached/closed Windows console must not prevent the file write or
    // turn a harmless diagnostic into a panic.
    let _ = writeln!(std::io::stderr().lock(), "{arguments}");
}

/// How often the render loop reports that it is still alive.
const FRAME_HEARTBEAT_MS: u64 = 60_000;

/// How often to report that the surface still is not ready. Shorter than the
/// heartbeat, because this is the state a player is staring at a blank window
/// in, and a log that goes quiet for a minute there looks like a hang.
const WAITING_REPORT_MS: u64 = 5_000;

/// Sentinel for [`WAITING_SINCE_MS`]: not currently waiting.
const NOT_WAITING: u64 = u64::MAX;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();
static FRAMES_RENDERED: AtomicU64 = AtomicU64::new(0);
static LAST_REPORT_MS: AtomicU64 = AtomicU64::new(0);
static FRAMES_AT_LAST_REPORT: AtomicU64 = AtomicU64::new(0);
static WAITING_SINCE_MS: AtomicU64 = AtomicU64::new(NOT_WAITING);
static LAST_WAITING_REPORT_MS: AtomicU64 = AtomicU64::new(NOT_WAITING);

fn elapsed_ms() -> u64 {
    PROCESS_START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Call once per frame that actually rendered.
///
/// The first of these is the most valuable line in the log for a white screen:
/// it separates "the client never drew anything" from "the client is drawing,
/// and what it draws is white". Nothing else distinguished those two, so
/// `Troubleshoot.bat` had to start the game once per backend and compare four
/// exit codes to guess at it -- a friend's evening, to recover something the
/// client knew all along.
///
/// After that it is a heartbeat, so a log that simply stops means the process
/// died rather than that the session ended. A pack log ending at `towninfo` was
/// read as a crash on 2026-09-05 and was a perfectly healthy run.
///
/// **Deliberately lock-free on the common path.** This runs once per frame, and
/// on this project a diagnostic in the render loop has already cost frame rate
/// once. Two relaxed loads and one increment per frame; the mutex-free
/// compare-exchange below means only the thread that actually wins the
/// heartbeat does any formatting.
pub fn note_frame_rendered() {
    let frame = FRAMES_RENDERED.fetch_add(1, Ordering::Relaxed) + 1;
    let now_ms = elapsed_ms();

    if WAITING_SINCE_MS.load(Ordering::Relaxed) != NOT_WAITING {
        let waiting_since = WAITING_SINCE_MS.swap(NOT_WAITING, Ordering::Relaxed);
        LAST_WAITING_REPORT_MS.store(NOT_WAITING, Ordering::Relaxed);
        if waiting_since != NOT_WAITING && frame > 1 {
            write_line(format_args!(
                "[frame] rendering again after waiting {:.1}s for the surface",
                now_ms.saturating_sub(waiting_since) as f32 / 1000.0
            ));
        }
    }

    if frame == 1 {
        LAST_REPORT_MS.store(now_ms, Ordering::Relaxed);
        FRAMES_AT_LAST_REPORT.store(1, Ordering::Relaxed);
        write_line(format_args!(
            "[frame] first frame rendered {:.1}s after startup",
            now_ms as f32 / 1000.0
        ));
        return;
    }

    let last_report = LAST_REPORT_MS.load(Ordering::Relaxed);
    let elapsed = now_ms.saturating_sub(last_report);
    if elapsed >= FRAME_HEARTBEAT_MS
        && LAST_REPORT_MS
            .compare_exchange(last_report, now_ms, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        let frames = frame.saturating_sub(FRAMES_AT_LAST_REPORT.swap(frame, Ordering::Relaxed));
        write_line(format_args!(
            "[frame] {frame} frames rendered, {:.1} fps over the last {:.0}s",
            frames as f32 / (elapsed as f32 / 1000.0),
            elapsed as f32 / 1000.0
        ));
    }
}

/// Call when a redraw was asked for but the surface was not ready to render.
///
/// A window that never leaves this state IS the white screen, and it used to
/// produce no output at all -- the loop just kept requesting redraws in
/// silence.
pub fn note_frame_waiting() {
    let now_ms = elapsed_ms();
    let waiting_since = match WAITING_SINCE_MS.compare_exchange(NOT_WAITING, now_ms, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => now_ms,
        Err(existing) => existing,
    };

    let last_report = LAST_WAITING_REPORT_MS.load(Ordering::Relaxed);
    let due = last_report == NOT_WAITING || now_ms.saturating_sub(last_report) >= WAITING_REPORT_MS;

    if due
        && LAST_WAITING_REPORT_MS
            .compare_exchange(last_report, now_ms, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        write_line(format_args!(
            "[frame] surface not ready to render, waiting {:.1}s -- the window is blank because nothing has been drawn yet",
            now_ms.saturating_sub(waiting_since) as f32 / 1000.0
        ));
    }
}

/// How many frames this run has rendered. Zero at shutdown means the client ran
/// and never drew anything, which is worth saying out loud in the last line.
pub fn frames_rendered() -> u64 {
    FRAMES_RENDERED.load(Ordering::Relaxed)
}

#[cfg(test)]
mod frame_accounting_tests {
    use super::*;

    /// Deliberately ONE test over the whole sequence. These counters are
    /// process-global because there is exactly one window, so a second test
    /// touching them would race this one and both would be flaky.
    #[test]
    fn a_first_frame_clears_the_wait_it_ended() {
        assert_eq!(frames_rendered(), 0, "another test is using the frame counters");

        note_frame_waiting();
        assert_ne!(
            WAITING_SINCE_MS.load(Ordering::Relaxed),
            NOT_WAITING,
            "waiting must be recorded, or the blank-window state reports nothing"
        );

        note_frame_rendered();
        assert_eq!(frames_rendered(), 1);
        // If the wait is not cleared here, the next frame reports a wait that
        // ended minutes ago -- worse than saying nothing, because it reads as a
        // stall that is still happening.
        assert_eq!(WAITING_SINCE_MS.load(Ordering::Relaxed), NOT_WAITING);
        assert_eq!(LAST_WAITING_REPORT_MS.load(Ordering::Relaxed), NOT_WAITING);

        note_frame_rendered();
        assert_eq!(frames_rendered(), 2);
    }
}

/// Like `eprintln!`, but the line also survives in `korangar.log`.
///
/// Use for anything a player might have to report. Developer tracing that is
/// already gated behind an environment variable can use it too — when somebody
/// turns that gate on, they want the output in the file they are about to
/// send.
#[macro_export]
macro_rules! client_log {
    ($($argument:tt)*) => {
        $crate::logging::write_line(format_args!($($argument)*))
    };
}
