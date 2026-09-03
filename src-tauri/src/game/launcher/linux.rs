//! Linux: the game is a Windows executable, run through Proton inside a
//! prefix the launcher owns.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{parse_custom_env_vars, GameProcess, LaunchedGame};
use crate::config::paths;
use crate::config::settings::AppSettings;
use crate::error::AppError;

/// Escape a string for safe use inside single quotes in a shell command.
fn shell_escape(s: &str) -> String {
    // Replace each ' with '\'' (end quote, escaped quote, start quote)
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Resolve the Proton prefix (STEAM_COMPAT_DATA_PATH) directory.
///
/// Historically the prefix lived at `<game_dir>/_proton`, which breaks when the
/// game is installed on an NTFS partition (Proton cannot create the Unix
/// symlinks it needs). We now keep it under the launcher data directory by
/// default, while still honouring an existing in-game-dir prefix so current
/// installs keep working.
pub fn resolve_prefix_dir(settings: &AppSettings, game_path: &Path) -> PathBuf {
    let legacy = game_path.join("_proton");
    if legacy.join("pfx").exists() {
        return legacy;
    }
    if !settings.proton_prefix_dir.trim().is_empty() {
        return Path::new(settings.proton_prefix_dir.trim()).join("endfield");
    }
    paths::default_proton_prefix_dir().join("endfield")
}

/// Shared preamble for anything run inside the game's Proton prefix: exports
/// every configured env var. Used by the game launch and the prefix tools
/// (winecfg etc.) so both see the exact same environment.
fn build_env_script(settings: &AppSettings, compat_data: &Path) -> String {
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
    for (key, value) in parse_custom_env_vars(&settings.custom_env_vars) {
        script.push_str(&format!("export {}={}\n", key, shell_escape(value)));
    }

    script
}

/// Translate the gamescope settings into a command-line argument string.
/// Returns None when the resolution strings are unparsable garbage is fine —
/// unknown values are simply skipped, gamescope falls back to its defaults.
fn build_gamescope_args(settings: &AppSettings) -> String {
    let mut args: Vec<String> = Vec::new();

    match settings.gamescope_mode.as_str() {
        "borderless" => args.push("-b".into()),
        "windowed" => {}
        _ => args.push("-f".into()),
    }

    let parse_res = |s: &str| -> Option<(u32, u32)> {
        let lower = s.trim().to_lowercase();
        let (w, h) = lower.split_once('x')?;
        match (w.trim().parse::<u32>(), h.trim().parse::<u32>()) {
            (Ok(w), Ok(h)) if w > 0 && h > 0 => Some((w, h)),
            _ => None,
        }
    };

    if let Some((w, h)) = parse_res(&settings.gamescope_render_res) {
        args.push(format!("-w {} -h {}", w, h));
    }
    if let Some((w, h)) = parse_res(&settings.gamescope_output_res) {
        args.push(format!("-W {} -H {}", w, h));
    }

    if settings.gamescope_fps_limit > 0 {
        args.push(format!("-r {}", settings.gamescope_fps_limit));
    }

    match settings.gamescope_upscaler.as_str() {
        "fsr" => args.push("-F fsr".into()),
        "nis" => args.push("-F nis".into()),
        "integer" => args.push("-S integer".into()),
        "stretch" => args.push("-S stretch".into()),
        _ => {}
    }

    if settings.gamescope_hdr {
        args.push("--hdr-enabled".into());
    }

    // MangoHud's Vulkan layer misbehaves inside gamescope; the supported way
    // is gamescope's own --mangoapp overlay, so route the toggle through it.
    if settings.use_mangohud {
        args.push("--mangoapp".into());
    }

    let extra = settings.gamescope_extra_args.trim();
    if !extra.is_empty() {
        args.push(extra.to_string());
    }

    args.join(" ")
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

    let mut script = build_env_script(settings, &compat_data);

    // cd into game directory
    script.push_str(&format!("cd {}\n", shell_escape(&game_path.to_string_lossy())));

    // Build the launch command with optional wrappers
    let proton_escaped = shell_escape(&proton_path.to_string_lossy());
    let wine_escaped = shell_escape(&wine_path);

    let mut launch_cmd = String::new();
    if settings.use_gamemode {
        launch_cmd.push_str("gamemoderun ");
    }
    if settings.use_gamescope {
        // gamescope hosts the game in its own compositor; MangoHud is folded
        // into it via --mangoapp (see build_gamescope_args), not the wrapper.
        let gs_args = build_gamescope_args(settings);
        if gs_args.is_empty() {
            launch_cmd.push_str("gamescope -- ");
        } else {
            launch_cmd.push_str(&format!("gamescope {} -- ", gs_args));
        }
    } else if settings.use_mangohud {
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

    Ok(LaunchedGame {
        process: GameProcess::Child(child),
        log_path,
    })
}

/// Shut down the prefix's wineserver (and with it every wine process of the
/// prefix, winedevice included).
///
/// Needed inside the Flatpak: a wineserver that outlives the session keeps
/// the old sandbox instance alive, and the next launch finds it through the
/// shared /run/user socket but cannot open its fsync shared memory — that
/// lives in the dead instance's private /dev/shm — so wine exits 1 before
/// loading anything, with no output. Call after the game session ends, never
/// while it may still be running.
pub fn shutdown_wineserver(settings: &AppSettings) {
    let wineserver = Path::new(&settings.proton_dir).join("files/bin/wineserver");
    if !wineserver.exists() {
        return;
    }
    let prefix = resolve_prefix_dir(settings, Path::new(&settings.game_dir)).join("pfx");
    if !prefix.exists() {
        return;
    }
    let _ = Command::new(wineserver)
        .arg("-k")
        .env("WINEPREFIX", &prefix)
        .status();
}

/// Run a Wine tool (winecfg, regedit, ...) inside the game's Proton prefix
/// with the exact environment the game itself gets. Fire-and-forget: the tool
/// opens its own window and the user closes it when done.
pub fn run_prefix_tool(settings: &AppSettings, tool: &str) -> Result<(), AppError> {
    let game_path = Path::new(&settings.game_dir);

    let proton_path = Path::new(&settings.proton_dir).join("proton");
    if !proton_path.exists() {
        return Err(AppError::ProtonNotFound(format!(
            "Proton not found at: {}",
            proton_path.display()
        )));
    }

    let compat_data = resolve_prefix_dir(settings, game_path);
    std::fs::create_dir_all(&compat_data)?;

    let mut script = build_env_script(settings, &compat_data);
    script.push_str(&format!(
        "exec {} run {} > /dev/null 2>&1\n",
        shell_escape(&proton_path.to_string_lossy()),
        shell_escape(tool)
    ));

    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(&script);
    crate::util::strip_appimage_libs(&mut cmd);
    cmd.process_group(0);
    cmd.spawn()
        .map_err(|e| AppError::Api(format!("Failed to run {}: {}", tool, e)))?;
    Ok(())
}

/// Ask the game to shut down: the whole process group (bash -> proton ->
/// game) gets a SIGTERM.
pub fn request_stop(pid: u32) {
    unsafe {
        libc::killpg(pid as i32, libc::SIGTERM);
    }
}

/// Kill the process group outright, for when the polite request was ignored.
pub fn force_stop(pid: u32) {
    unsafe {
        libc::killpg(pid as i32, libc::SIGKILL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_single_quotes_in_paths() {
        // A game directory with an apostrophe in it must not break out of the
        // quoted argument in the generated shell script.
        assert_eq!(shell_escape("/home/o'brien/Games"), r"'/home/o'\''brien/Games'");
    }
}
