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

fn default_gamescope_mode() -> String {
    "fullscreen".to_string()
}

fn default_gamescope_upscaler() -> String {
    "auto".to_string()
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
    /// Run the game inside the gamescope micro-compositor.
    #[serde(default)]
    pub use_gamescope: bool,
    /// gamescope window mode: "fullscreen" | "borderless" | "windowed".
    #[serde(default = "default_gamescope_mode")]
    pub gamescope_mode: String,
    /// Game render resolution as "WIDTHxHEIGHT", empty = native.
    #[serde(default)]
    pub gamescope_render_res: String,
    /// gamescope output resolution as "WIDTHxHEIGHT", empty = auto.
    #[serde(default)]
    pub gamescope_output_res: String,
    /// Nested refresh rate / FPS cap for gamescope, 0 = off.
    #[serde(default)]
    pub gamescope_fps_limit: u32,
    /// Upscaler: "auto" | "fsr" | "nis" | "integer" | "stretch".
    #[serde(default = "default_gamescope_upscaler")]
    pub gamescope_upscaler: String,
    /// Enable HDR output in gamescope (--hdr-enabled).
    #[serde(default)]
    pub gamescope_hdr: bool,
    /// Extra raw arguments appended to the gamescope invocation.
    #[serde(default)]
    pub gamescope_extra_args: String,
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
            use_gamescope: false,
            gamescope_mode: default_gamescope_mode(),
            gamescope_render_res: String::new(),
            gamescope_output_res: String::new(),
            gamescope_fps_limit: 0,
            gamescope_upscaler: default_gamescope_upscaler(),
            gamescope_hdr: false,
            gamescope_extra_args: String::new(),
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

    /// Persist without blocking the async runtime thread. Use this from
    /// `#[tauri::command]` handlers; `save()` is for genuinely sync contexts
    /// (app setup, or code already inside `spawn_blocking`).
    pub async fn save_async(&self) -> Result<(), crate::error::AppError> {
        let settings = self.clone();
        tokio::task::spawn_blocking(move || settings.save())
            .await
            .map_err(|e| crate::error::AppError::Api(format!("settings save task failed: {}", e)))?
    }
}
