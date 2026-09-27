//! Launcher-side log file.
//!
//! The game's own stdout/stderr already lands in `launch.log`; this is for the
//! launcher itself. Until now every backend failure was only ever emitted to
//! the webview, so anything that happened before the UI was listening, or
//! after it died, was simply lost — and "it doesn't work" reports had nothing
//! to attach. Every `AppError` handed back to the frontend, plus the key
//! lifecycle events (launch, exit, download start/end), goes here.
//!
//! Plain append with a size cap and a single rotation (`.log` → `.log.1`);
//! no dependency on a logging crate for what is a handful of lines per run.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Rotate once the active file grows past this.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

static LOCK: Mutex<()> = Mutex::new(());

/// `~/.local/share/llauncher/llauncher.log` on Linux,
/// `%APPDATA%\llauncher\llauncher.log` on Windows.
pub fn log_path() -> PathBuf {
    crate::config::paths::data_base().join("llauncher.log")
}

fn write_line(level: &str, msg: &str) {
    let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let path = log_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > MAX_BYTES {
            let _ = std::fs::rename(&path, path.with_extension("log.1"));
        }
    }
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Single line per entry so the tail stays greppable; embedded newlines
    // (multi-line tar/wine stderr) are folded.
    let msg = msg.replace('\n', " | ");
    let _ = writeln!(file, "{} [{}] {}", format_ts(now), level, msg);
}

/// `YYYY-MM-DD HH:MM:SS` in UTC without pulling in a date crate.
fn format_ts(secs: u64) -> String {
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil-from-days (Howard Hinnant), valid for the Unix epoch onward.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, h, m, s)
}

pub fn info(msg: impl AsRef<str>) {
    write_line("info", msg.as_ref());
}

pub fn warn(msg: impl AsRef<str>) {
    write_line("warn", msg.as_ref());
}

pub fn error(msg: impl AsRef<str>) {
    write_line("error", msg.as_ref());
}

/// Last `max_lines` lines of the launcher log, for the debug-info report.
pub fn tail(max_lines: usize) -> String {
    let content = std::fs::read_to_string(log_path()).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_unix_timestamps_as_utc_civil_time() {
        assert_eq!(format_ts(0), "1970-01-01 00:00:00");
        // 2026-09-02 18:00:00 UTC
        assert_eq!(format_ts(1_788_372_000), "2026-09-02 18:00:00");
        // Leap day.
        assert_eq!(format_ts(1_709_164_800), "2024-02-29 00:00:00");
    }
}
