//! Golden's local log. One rotating text file in the app's data folder, nothing sent anywhere.
//! Every line passes through [`sanitize`] first, so account numbers, amounts, e-mail addresses
//! and keys never reach the disk even if a caller forgets to leave them out.
mod rotate;
mod sanitize;

pub use rotate::RotatingFile;
pub use sanitize::sanitize;

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Longest message kept, so one runaway string cannot fill the file.
const MAX_MESSAGE: usize = 600;

struct Logger {
    file: Mutex<RotatingFile>,
    dir: PathBuf,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

impl Log for &'static Logger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= log::max_level()
    }

    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        self.write(r.level(), r.target(), &r.args().to_string());
    }

    fn flush(&self) {}
}

impl Logger {
    fn write(&self, level: Level, target: &str, message: &str) {
        let mut msg = sanitize(message);
        if msg.chars().count() > MAX_MESSAGE {
            msg = msg.chars().take(MAX_MESSAGE).collect::<String>() + "…";
        }
        let line = format!(
            "{} {:<5} {} {}",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            level,
            sanitize(target),
            msg.replace(['\n', '\r'], " ")
        );
        if let Ok(mut f) = self.file.lock() {
            let _ = f.append(&line);
        }
    }
}

/// Start logging into `dir` (created if missing). Safe to call twice; the second call is ignored.
/// `verbose` adds debug lines.
pub fn init(dir: &Path, verbose: bool) -> Result<(), String> {
    let file = RotatingFile::open(dir, "golden", 1_000_000, 5).map_err(|e| e.to_string())?;
    let logger = LOGGER.get_or_init(|| Logger {
        file: Mutex::new(file),
        dir: dir.to_path_buf(),
    });
    // A second init (or a test that already set a logger) is fine.
    let _ = log::set_logger(Box::leak(Box::new(logger)));
    log::set_max_level(if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    });
    log::info!(
        "log started (golden {}, {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );
    Ok(())
}

/// The folder the log files live in, once [`init`] has run.
pub fn dir() -> Option<PathBuf> {
    LOGGER.get().map(|l| l.dir.clone())
}

/// A line from the screens (JavaScript side). `level` is error, warn, info or debug.
pub fn from_ui(level: &str, source: &str, message: &str) {
    let Some(l) = LOGGER.get() else { return };
    let level = match level {
        "error" => Level::Error,
        "warn" => Level::Warn,
        "debug" => Level::Debug,
        _ => Level::Info,
    };
    if level <= log::max_level() {
        l.write(level, &format!("ui:{source}"), message);
    }
}

/// The last `max_lines` lines across the current and previous files, oldest first, for the
/// "copy log" button. Already scrubbed when written.
pub fn tail(max_lines: usize) -> String {
    let Some(l) = LOGGER.get() else {
        return String::new();
    };
    l.file.lock().map(|f| f.tail(max_lines)).unwrap_or_default()
}
