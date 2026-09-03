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
    /// Windows only: start the game elevated (UAC prompt) right away instead
    /// of waiting for `CreateProcess` to fail with ERROR_ELEVATION_REQUIRED.
    /// Some anti-cheat drivers need it; most installs do not.
    #[serde(default)]
    pub windows_run_as_admin: bool,
    /// Run the game through the vkBasalt post-processing layer (sharpening,
    /// colour correction, ReShade-format effects). Native Vulkan, so unlike
    /// the 3DMigoto path it costs nothing extra in renderer terms.
    #[serde(default)]
    pub use_vkbasalt: bool,
    /// Show the "play with mods" action: the game then starts on its D3D11
    /// path with the `d3d11.dll` proxy (3DMigoto/EFMI) loaded. Off by default —
    /// it costs frames and the game is anti-cheat protected, so it is opt-in.
    #[serde(default)]
    pub mods_enabled: bool,
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
            windows_run_as_admin: false,
            use_vkbasalt: false,
            mods_enabled: false,
            total_playtime_secs: 0,
            last_played: 0,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let path = paths::settings_path();
        if !path.exists() {
            return Self::default();
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                crate::logging::warn(format!("settings: cannot read {}: {}", path.display(), e));
                return Self::default();
            }
        };
        match serde_json::from_str(&content) {
            Ok(settings) => settings,
            Err(e) => {
                // A corrupt settings file (half-written by a crash, hand-edited
                // badly) used to silently reset everything, which turned an
                // installed game back into "Install". Keep the broken file
                // around for the user instead of overwriting it on the next
                // save, and say so in the log.
                let backup = path.with_extension("json.corrupt");
                let _ = std::fs::rename(&path, &backup);
                crate::logging::error(format!(
                    "settings: {} is not valid JSON ({}); moved to {} and using defaults",
                    path.display(),
                    e,
                    backup.display()
                ));
                Self::default()
            }
        }
    }

    /// Write atomically: serialise to a sibling temp file, then rename over the
    /// real one. A crash or power loss mid-write leaves the previous file
    /// intact rather than a truncated one that `load` would reject.
    pub fn save(&self) -> Result<(), crate::error::AppError> {
        let path = paths::settings_path();
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        let content = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, &path)?;
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
