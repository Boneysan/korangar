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
use std::sync::{Mutex, OnceLock};

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

    write_line(format_args!(
        "[korangar] {} starting -- version {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        env!("CARGO_PKG_VERSION")
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
