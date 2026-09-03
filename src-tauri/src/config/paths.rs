use std::path::{Path, PathBuf};

/// Free disk space (in bytes) available to the current user on the filesystem
/// holding `path`. Walks up to the nearest existing ancestor so it also works
/// for directories that have not been created yet. `None` if it cannot be
/// determined.
pub fn available_space(path: &Path) -> Option<u64> {
    let mut probe = path;
    while !probe.exists() {
        probe = probe.parent()?;
    }
    available_space_of_existing(probe)
}

#[cfg(unix)]
fn available_space_of_existing(dir: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;

    let c_path = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } == 0 {
        Some(stat.f_bavail as u64 * stat.f_frsize as u64)
    } else {
        None
    }
}

#[cfg(windows)]
fn available_space_of_existing(dir: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    // GetDiskFreeSpaceExW wants a directory (a trailing separator is fine) and
    // a NUL-terminated wide string.
    let wide: Vec<u16> = dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // "Free bytes available to the calling user" — the quota-aware figure,
    // matching what statvfs' f_bavail reports on Unix.
    let mut available: u64 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return None;
    }
    Some(available)
}

/// Config directory: `~/.config/llauncher/` on Linux,
/// `%APPDATA%\llauncher\` on Windows.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("llauncher")
}

/// Settings file path
pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// Game launch log path, next to the settings file.
pub fn launch_log_path() -> PathBuf {
    config_dir().join("launch.log")
}

/// Play-session journal path, next to the settings file.
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
pub fn data_base() -> PathBuf {
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
