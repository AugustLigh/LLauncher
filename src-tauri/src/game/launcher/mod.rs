//! Starting the game and watching the process it produced.
//!
//! The two platforms have almost nothing in common here: on Linux the game is
//! a Windows executable run through Proton, wrapped in a shell script that
//! exports a page of environment variables and optional gamescope/gamemode
//! wrappers; on Windows it is simply the executable. Everything that *is*
//! shared — the launched-process handle, the log tail — lives in this file,
//! and each platform module supplies the rest behind the same names.

#[cfg(unix)]
mod linux;
#[cfg(unix)]
pub use linux::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

use std::path::{Path, PathBuf};

pub struct LaunchedGame {
    pub process: GameProcess,
    pub log_path: PathBuf,
}

/// How a finished game process exited. Mirrors the part of
/// `std::process::ExitStatus` the launcher actually uses, so the elevated
/// Windows path (a bare process handle, no `Child`) can report the same thing.
#[derive(Debug, Clone, Copy)]
pub struct ExitInfo {
    pub code: Option<i32>,
}

/// A running game, however it was started.
pub enum GameProcess {
    /// Spawned through `std::process::Command` — the normal case on both
    /// platforms.
    Child(std::process::Child),
    /// Windows only: an elevated launch goes through `ShellExecuteExW`, which
    /// returns a process handle instead of a `Child`.
    #[cfg(windows)]
    Handle(windows::ProcessHandle),
}

impl GameProcess {
    pub fn id(&self) -> u32 {
        match self {
            GameProcess::Child(child) => child.id(),
            #[cfg(windows)]
            GameProcess::Handle(handle) => handle.id(),
        }
    }

    /// Non-blocking check for a finished process, matching
    /// `std::process::Child::try_wait`: `Ok(None)` while it is still running.
    pub fn try_wait(&mut self) -> std::io::Result<Option<ExitInfo>> {
        match self {
            GameProcess::Child(child) => Ok(child.try_wait()?.map(|status| ExitInfo {
                code: status.code(),
            })),
            #[cfg(windows)]
            GameProcess::Handle(handle) => handle.try_wait(),
        }
    }
}

/// A POSIX environment variable name: letters, digits, underscore, not
/// starting with a digit. Custom env vars are user-typed `KEY=VALUE` lines;
/// on Linux they end up in `export NAME=value`, which requires `NAME` to be a
/// bare identifier — quoting it the way the value is quoted would either do
/// nothing useful or change the syntax, so invalid names are rejected instead.
/// Windows is laxer about names but gains nothing from accepting garbage, so
/// both platforms filter through the same rule.
pub fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse the `KEY=VALUE` lines of the custom-env-vars setting, skipping blank
/// lines, `#` comments and malformed names.
pub fn parse_custom_env_vars(raw: &str) -> Vec<(&str, &str)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if !is_valid_env_var_name(key) {
                return None;
            }
            Some((key, value.trim()))
        })
        .collect()
}

/// Read the tail of the launch log (last `max_lines` lines).
/// Returns empty string if log does not exist or cannot be read.
pub fn read_log_tail(log_path: &Path, max_lines: usize) -> String {
    let content = match std::fs::read_to_string(log_path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_env_var_names() {
        // `export NAME=value` requires a bare identifier for NAME — a
        // malformed custom env var line must be dropped, not turned into
        // broken (or injectable) shell syntax.
        assert!(is_valid_env_var_name("DXVK_HUD"));
        assert!(is_valid_env_var_name("_FOO"));
        assert!(is_valid_env_var_name("FOO_1"));
        assert!(!is_valid_env_var_name(""));
        assert!(!is_valid_env_var_name("1FOO"));
        assert!(!is_valid_env_var_name("FOO BAR"));
        assert!(!is_valid_env_var_name("FOO;rm -rf ~"));
    }

    #[test]
    fn parses_custom_env_var_lines() {
        let raw = "# a comment\n\
                   DXVK_HUD=fps\n\
                   \n\
                   MANGOHUD_CONFIG = cpu_temp \n\
                   1BAD=x\n\
                   no_equals_sign\n";
        assert_eq!(
            parse_custom_env_vars(raw),
            vec![("DXVK_HUD", "fps"), ("MANGOHUD_CONFIG", "cpu_temp")]
        );
    }
}
