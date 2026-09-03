//! What the host can offer the game: the compatibility layer on Linux, and
//! the optional wrappers (gamemode, MangoHud, gamescope) the launch script
//! can put in front of it. On Windows none of this applies — the game runs
//! natively — so the check only reports the platform and the UI drops every
//! Proton-shaped control.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SystemCheck {
    /// `"linux"` or `"windows"`. The frontend keys every Proton/Wine-only
    /// section off this rather than guessing from the other flags.
    pub platform: &'static str,
    pub has_proton: bool,
    pub has_ntsync: bool,
    pub has_gamemode: bool,
    pub has_mangohud: bool,
    pub has_gamescope: bool,
    pub proton_path: String,
}

#[cfg(unix)]
pub fn check_system(proton_dir: &str) -> SystemCheck {
    use std::path::Path;

    let has_proton = if proton_dir.is_empty() {
        false
    } else {
        Path::new(proton_dir).join("proton").exists()
    };

    SystemCheck {
        platform: "linux",
        has_proton,
        has_ntsync: check_ntsync(),
        has_gamemode: check_command("gamemoderun"),
        has_mangohud: check_command("mangohud"),
        has_gamescope: check_command("gamescope"),
        proton_path: if has_proton {
            Path::new(proton_dir)
                .join("proton")
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        },
    }
}

/// Nothing to check for on Windows: the game is a native binary and needs no
/// compatibility layer. `has_proton`/`has_ntsync` report *true* — they mean
/// "nothing is missing" to the UI, so a frontend that somehow still reads them
/// shows no bogus "Proton not found" warning.
#[cfg(windows)]
pub fn check_system(_proton_dir: &str) -> SystemCheck {
    SystemCheck {
        platform: "windows",
        has_proton: true,
        has_ntsync: true,
        has_gamemode: false,
        has_mangohud: false,
        has_gamescope: false,
        proton_path: String::new(),
    }
}

#[cfg(unix)]
fn check_ntsync() -> bool {
    std::path::Path::new("/dev/ntsync").exists()
}

#[cfg(unix)]
fn check_command(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
