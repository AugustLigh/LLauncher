use std::path::PathBuf;

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
