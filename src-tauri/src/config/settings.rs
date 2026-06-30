use serde::{Deserialize, Serialize};

use super::paths;

fn default_true() -> bool {
    true
}

fn default_on_launch_action() -> String {
    "hide".to_string()
}

fn default_max_concurrent() -> u32 {
    4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub game_dir: String,
    pub download_dir: String,
    pub proton_dir: String,
    #[serde(default)]
    pub proton_prefix_dir: String,
    pub language: String,
    pub installed_version: String,
    #[serde(default)]
    pub use_gamemode: bool,
    #[serde(default)]
    pub use_mangohud: bool,
    #[serde(default = "default_true")]
    pub use_native_vulkan: bool,
    #[serde(default = "default_true")]
    pub use_wayland: bool,
    // Off by default: a no-op under the native Vulkan renderer and a known
    // source of sporadic in-game crashes when DXVK is in play (issue #21).
    #[serde(default)]
    pub use_dxvk_async: bool,
    #[serde(default = "default_on_launch_action")]
    pub on_launch_action: String,
    #[serde(default)]
    pub disable_fsync: bool,
    #[serde(default)]
    pub disable_esync: bool,
    #[serde(default)]
    pub download_speed_limit: u64,
    #[serde(default = "default_max_concurrent")]
    pub download_max_concurrent: u32,
    #[serde(default)]
    pub use_canonical_hole: bool,
    #[serde(default)]
    pub custom_env_vars: String,
    #[serde(default)]
    pub custom_launch_args: String,
    #[serde(default)]
    pub autostart_initialized: bool,
    #[serde(default)]
    pub use_prime_offload: bool,
    #[serde(default)]
    pub use_discord_rpc: bool,
    /// Accumulated in-game time in seconds.
    #[serde(default)]
    pub total_playtime_secs: u64,
    /// Unix timestamp (seconds) of the last game launch, 0 = never.
    #[serde(default)]
    pub last_played: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            game_dir: paths::default_game_dir().to_string_lossy().to_string(),
            download_dir: paths::default_download_dir().to_string_lossy().to_string(),
            proton_dir: paths::default_proton_dir().to_string_lossy().to_string(),
            proton_prefix_dir: paths::default_proton_prefix_dir()
                .to_string_lossy()
                .to_string(),
            language: "en-us".to_string(),
            installed_version: String::new(),
            use_gamemode: false,
            use_mangohud: false,
            use_native_vulkan: true,
            use_wayland: true,
            use_dxvk_async: false,
            on_launch_action: "hide".to_string(),
            disable_fsync: false,
            disable_esync: false,
            download_speed_limit: 0,
            download_max_concurrent: 4,
            use_canonical_hole: false,
            custom_env_vars: String::new(),
            custom_launch_args: String::new(),
            autostart_initialized: false,
            use_prime_offload: false,
            use_discord_rpc: false,
            total_playtime_secs: 0,
            last_played: 0,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let path = paths::settings_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), crate::error::AppError> {
        let path = paths::settings_path();
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
