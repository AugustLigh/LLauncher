use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::config::paths;
use crate::config::settings::AppSettings;
use crate::error::AppError;

/// Escape a string for safe use inside single quotes in a shell command.
fn shell_escape(s: &str) -> String {
    // Replace each ' with '\'' (end quote, escaped quote, start quote)
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub struct LaunchedGame {
    pub child: Child,
    pub log_path: PathBuf,
}

/// Resolve the Proton prefix (STEAM_COMPAT_DATA_PATH) directory.
///
/// Historically the prefix lived at `<game_dir>/_proton`, which breaks when the
/// game is installed on an NTFS partition (Proton cannot create the Unix
/// symlinks it needs). We now keep it under the launcher data directory by
/// default, while still honouring an existing in-game-dir prefix so current
/// installs keep working.
fn resolve_prefix_dir(settings: &AppSettings, game_path: &Path) -> PathBuf {
    let legacy = game_path.join("_proton");
    if legacy.join("pfx").exists() {
        return legacy;
    }
    if !settings.proton_prefix_dir.trim().is_empty() {
        return Path::new(settings.proton_prefix_dir.trim()).join("endfield");
    }
    paths::default_proton_prefix_dir().join("endfield")
}

pub fn launch_game(settings: &AppSettings) -> Result<LaunchedGame, AppError> {
    let game_path = Path::new(&settings.game_dir);
    let exe_path = game_path.join("Endfield.exe");

    if !exe_path.exists() {
        return Err(AppError::GameNotFound(format!(
            "Executable not found: {}",
            exe_path.display()
        )));
    }

    let proton_path = Path::new(&settings.proton_dir).join("proton");
    if !proton_path.exists() {
        return Err(AppError::ProtonNotFound(format!(
            "Proton not found at: {}",
            proton_path.display()
        )));
    }

    // Convert Linux path to Wine Z: path
    let wine_path = format!("Z:{}", exe_path.to_string_lossy().replace('/', "\\"));

    let compat_data = resolve_prefix_dir(settings, game_path);
    std::fs::create_dir_all(&compat_data)?;

    let log_path = paths::launch_log_path();
    std::fs::create_dir_all(log_path.parent().unwrap())?;

    // Build shell script with exported env vars
    let mut script = String::new();

    // Unset Python vars that break Proton's bundled Python
    script.push_str("unset PYTHONHOME PYTHONPATH\n");

    // Standard env vars
    script.push_str(&format!(
        "export PROTON_USE_WINEALSA=1\n\
         export UMU_USE_STEAM=1\n\
         export STEAM_COMPAT_CLIENT_INSTALL_PATH={}\n\
         export STEAM_COMPAT_DATA_PATH={}\n\
         export PROTON_USE_WINED3D=0\n",
        shell_escape(&settings.proton_dir),
        shell_escape(&compat_data.to_string_lossy()),
    ));

    if settings.use_dxvk_async {
        script.push_str("export DXVK_ASYNC=1\n");
    }

    if settings.use_wayland {
        script.push_str("export PROTON_ENABLE_WAYLAND=1\n");
    }

    if settings.disable_fsync {
        script.push_str("export PROTON_NO_FSYNC=1\n");
    }
    if settings.disable_esync {
        script.push_str("export PROTON_NO_ESYNC=1\n");
    }

    // Enable ntsync if the kernel device is present. Proton picks it over
    // fsync/esync when this is set, giving better sync primitive performance.
    if Path::new("/dev/ntsync").exists() {
        script.push_str("export PROTON_ENABLE_NTSYNC=1\n");
    }

    if settings.use_canonical_hole {
        script.push_str("export WINE_CANONICAL_HOLE='skip_volatile_check'\n");
    }

    // Hybrid graphics: render on the dedicated GPU (PRIME offload). The NV
    // vars are ignored on non-NVIDIA systems, DRI_PRIME covers AMD/Intel.
    if settings.use_prime_offload {
        script.push_str(
            "export DRI_PRIME=1\n\
             export __NV_PRIME_RENDER_OFFLOAD=1\n\
             export __VK_LAYER_NV_optimus=NVIDIA_only\n\
             export __GLX_VENDOR_LIBRARY_NAME=nvidia\n",
        );
    }

    // Custom env vars (KEY=VALUE per line)
    if !settings.custom_env_vars.is_empty() {
        for line in settings.custom_env_vars.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                script.push_str(&format!(
                    "export {}={}\n",
                    key.trim(),
                    shell_escape(value.trim())
                ));
            }
        }
    }

    // cd into game directory
    script.push_str(&format!("cd {}\n", shell_escape(&game_path.to_string_lossy())));

    // Build the launch command with optional wrappers
    let proton_escaped = shell_escape(&proton_path.to_string_lossy());
    let wine_escaped = shell_escape(&wine_path);

    let mut launch_cmd = String::new();
    if settings.use_gamemode {
        launch_cmd.push_str("gamemoderun ");
    }
    if settings.use_mangohud {
        launch_cmd.push_str("mangohud ");
    }
    launch_cmd.push_str(&format!("{} run {}", proton_escaped, wine_escaped));

    // Native Vulkan renderer
    if settings.use_native_vulkan {
        launch_cmd.push_str(" -vulkan");
    }

    // Custom launch args
    if !settings.custom_launch_args.is_empty() {
        launch_cmd.push(' ');
        launch_cmd.push_str(&settings.custom_launch_args);
    }

    // Redirect output to log file
    script.push_str(&format!(
        "exec {} > {} 2>&1\n",
        launch_cmd,
        shell_escape(&log_path.to_string_lossy())
    ));

    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(&script);

    // When packaged as an AppImage, keep the bundle's older bundled libraries
    // out of Proton/Wine and their child processes — they must use the host's
    // system libraries (same root cause as issue #19's tar/xz failure).
    crate::util::strip_appimage_libs(&mut cmd);

    // Detach into a new process group so closing the launcher (or the
    // launcher hiding/closing tray) does not propagate signals to the game.
    cmd.process_group(0);

    let child = cmd
        .spawn()
        .map_err(|e| AppError::GameNotFound(format!("Failed to launch: {}", e)))?;

    Ok(LaunchedGame { child, log_path })
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
