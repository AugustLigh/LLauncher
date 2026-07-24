use std::path::{Path, PathBuf};

/// Free disk space (in bytes) available to the current user on the filesystem
/// holding `path`. Walks up to the nearest existing ancestor so it also works
/// for directories that have not been created yet. `None` if it cannot be
/// determined.
pub fn available_space(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;

    let mut probe = path;
    while !probe.exists() {
        probe = probe.parent()?;
    }

    let c_path = std::ffi::CString::new(probe.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } == 0 {
        Some(stat.f_bavail as u64 * stat.f_frsize as u64)
    } else {
        None
    }
}

/// Config directory: ~/.config/llauncher/
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("llauncher")
}

/// Settings file path
pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// Game launch log path: ~/.config/llauncher/launch.log
pub fn launch_log_path() -> PathBuf {
    config_dir().join("launch.log")
}

/// Play-session journal path: ~/.config/llauncher/sessions.json
pub fn sessions_path() -> PathBuf {
    config_dir().join("sessions.json")
}

/// Default game install directory
pub fn default_game_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Games")
        .join("ArknightsEndfield")
}

/// Default download (temp) directory
pub fn default_download_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Games")
        .join("ArknightsEndfield")
        .join("_download")
}

/// Base directory for launcher data on the user's home partition.
fn data_base() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("share")
        })
        .join("llauncher")
}

/// Default proton directory
pub fn default_proton_dir() -> PathBuf {
    data_base().join("proton")
}

/// Default Proton prefix (compatdata) base directory.
///
/// Kept under the launcher's data directory rather than inside the game
/// folder, because the game may live on an NTFS partition that cannot hold the
/// Unix symlinks Proton needs to create its prefix.
pub fn default_proton_prefix_dir() -> PathBuf {
    data_base().join("prefix")
}
