//! Linux: the game is a Windows executable, run through Proton inside a
//! prefix the launcher owns.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{parse_custom_env_vars, shell_escape, GameProcess, LaunchedGame};
use crate::config::paths;
use crate::config::settings::AppSettings;
use crate::error::AppError;

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
fn build_env_script(settings: &AppSettings, compat_data: &Path, with_mods: bool) -> String {
    let mut script = String::new();

    // Unset Python vars that break Proton's bundled Python
    script.push_str("unset PYTHONHOME PYTHONPATH\n");

    // Standard env vars
    script.push_str(&format!(
        "export PROTON_USE_WINEALSA=1\n\
         export UMU_USE_STEAM=1\n\
         export STEAM_COMPAT_CLIENT_INSTALL_PATH={}\n\
         export STEAM_COMPAT_DATA_PATH={}\n\
         export PROTON_USE_WINED3D=0\n\
         export PROTON_USE_XALIA=0\n",
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

    // Gamepads. Proton's default input path hands a known controller to the
    // game as a raw HID device, which is right under Steam — Steam Input turns
    // it into an XInput pad — and useless here, where there is no Steam: an
    // XInput-only game then sees nothing at all, while the DualShock's
    // touchpad still moves the mouse through the kernel driver (issue #32).
    // PROTON_PREFER_SDL switches winebus to its SDL backend, which presents
    // every controller SDL knows as an XInput-compatible one.
    if settings.use_sdl_input {
        script.push_str("export PROTON_PREFER_SDL=1\n");
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

    // For a modded launch, 3DMigoto supplies `d3d11.dll` next to `Endfield.exe`.
    //
    // CRITICAL: We do NOT export `d3d11=n,b` globally via WINEDLLOVERRIDES!
    // Exporting it globally forces ALL child processes in the Wine session
    // (specifically PlatformProcess.exe and QtWebEngineProcess.exe) to load
    // 3DMigoto's proxy d3d11.dll. 3DMigoto is designed solely for Endfield.exe
    // (`target = Endfield.exe` in d3dx.ini), so when Qt5WebEngineCore / Chromium
    // attempts to use D3D11 through 3DMigoto, it crashes with an access violation
    // (c0000005) or throws a "d3d11.dll error" dialog (XXMI-Launcher issue #276).
    //
    // Instead, per-application overrides are configured in Wine's registry
    // (AppDefaults\\Endfield.exe has d3d11=native,builtin, while
    // AppDefaults\\PlatformProcess.exe and QtWebEngineProcess.exe have d3d11=builtin).
    // We ensure user.reg has them both ahead-of-time in Rust and via an inline
    // guard in this script.
    if with_mods {
        script.push_str(
            "if [ ! -f \"$STEAM_COMPAT_DATA_PATH/pfx/user.reg\" ]; then\n\
  \"$STEAM_COMPAT_CLIENT_INSTALL_PATH/proton\" run wineboot -u\n\
fi\n\
if [ -f \"$STEAM_COMPAT_DATA_PATH/pfx/user.reg\" ] && ! grep -q \"PlatformProcess.exe\" \"$STEAM_COMPAT_DATA_PATH/pfx/user.reg\"; then\n\
cat << 'EOF' >> \"$STEAM_COMPAT_DATA_PATH/pfx/user.reg\"\n\
\n\
[Software\\\\Wine\\\\AppDefaults\\\\Endfield.exe\\\\DllOverrides]\n\
\"d3d11\"=\"native,builtin\"\n\
\"dxgi\"=\"native,builtin\"\n\
\n\
[Software\\\\Wine\\\\AppDefaults\\\\PlatformProcess.exe\\\\DllOverrides]\n\
\"d3d11\"=\"builtin\"\n\
\"dxgi\"=\"builtin\"\n\
\n\
[Software\\\\Wine\\\\AppDefaults\\\\QtWebEngineProcess.exe\\\\DllOverrides]\n\
\"d3d11\"=\"builtin\"\n\
\"dxgi\"=\"builtin\"\n\
EOF\n\
fi\n",
        );
    }

    // OptiScaler is a proxy (`winmm.dll` next to the game) which Wine ignores
    // in favour of its builtin unless told otherwise. It hooks the NGX loader
    // rather than D3D11, so it rides along with a normal Vulkan launch.
    let mut dll_overrides: Vec<&str> = Vec::new();
    if crate::game::optiscaler::is_installed(Path::new(&settings.game_dir)) {
        dll_overrides.push("winmm=n,b");
    }
    if !dll_overrides.is_empty() {
        script.push_str(&format!(
            "export WINEDLLOVERRIDES='{}'\n",
            dll_overrides.join(";")
        ));
    }

    // NVIDIA: the driver-side DLSS knobs Proton exposes. All of them are
    // no-ops without the driver, so they are not gated on it here.
    if settings.dlss_upgrade {
        script.push_str("export PROTON_DLSS_UPGRADE=1\n");
    }
    if settings.dlss_indicator {
        script.push_str("export PROTON_DLSS_INDICATOR=1\n");
    }
    if settings.use_vk_reflex {
        script.push_str("export DXVK_NVAPI_VKREFLEX=1\n");
    }

    // vkBasalt hooks the native Vulkan renderer, so it is orthogonal to the
    // mods path and applies to an ordinary launch too.
    if settings.use_vkbasalt {
        script.push_str("export ENABLE_VKBASALT=1\n");
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

/// The renderer switch on the command line. Native Vulkan is the default on
/// Linux, translated directly by the host Vulkan driver. 3DMigoto only hooks
/// Direct3D 11, so a modded launch forces D3D11 (which Proton maps through
/// DXVK) and ignores the Vulkan toggle. Selecting DirectX 11 in settings
/// similarly forces D3D11.
fn renderer_args(settings: &AppSettings, with_mods: bool) -> &'static str {
    if settings.use_native_vulkan && !with_mods {
        "-vulkan"
    } else {
        "-force-d3d11"
    }
}

/// Build the launch command line with all wrappers, Proton execution, renderer
/// flags, and custom arguments.
fn build_launch_cmd(
    settings: &AppSettings,
    proton_escaped: &str,
    wine_escaped: &str,
    with_mods: bool,
) -> String {
    let mut launch_cmd = String::new();
    if settings.use_gamemode {
        // Deliberately unquoted, like the custom launch arguments: the user's
        // own command line, arguments and all.
        let gamemode = settings.gamemode_command.trim();
        launch_cmd.push_str(if gamemode.is_empty() {
            "gamemoderun"
        } else {
            gamemode
        });
        launch_cmd.push(' ');
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

    let renderer = renderer_args(settings, with_mods);
    if !renderer.is_empty() {
        launch_cmd.push(' ');
        launch_cmd.push_str(renderer);
    }

    // Custom launch args
    if !settings.custom_launch_args.is_empty() {
        launch_cmd.push(' ');
        launch_cmd.push_str(&settings.custom_launch_args);
    }

    launch_cmd
}

/// Ensure the prefix's user.reg has per-application DLL overrides so 3DMigoto
/// is only loaded into Endfield.exe, while helper processes (PlatformProcess.exe,
/// QtWebEngineProcess.exe) use Wine's builtin DXVK/d3d11.
pub fn ensure_prefix_appdefaults(compat_data: &Path) -> std::io::Result<()> {
    let user_reg = compat_data.join("pfx").join("user.reg");
    if !user_reg.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&user_reg)?;
    if content.contains("[Software\\\\Wine\\\\AppDefaults\\\\PlatformProcess.exe\\\\DllOverrides]") {
        return Ok(());
    }

    let entries = "\n\
[Software\\\\Wine\\\\AppDefaults\\\\Endfield.exe\\\\DllOverrides]\n\
\"d3d11\"=\"native,builtin\"\n\
\"dxgi\"=\"native,builtin\"\n\
\n\
[Software\\\\Wine\\\\AppDefaults\\\\PlatformProcess.exe\\\\DllOverrides]\n\
\"d3d11\"=\"builtin\"\n\
\"dxgi\"=\"builtin\"\n\
\n\
[Software\\\\Wine\\\\AppDefaults\\\\QtWebEngineProcess.exe\\\\DllOverrides]\n\
\"d3d11\"=\"builtin\"\n\
\"dxgi\"=\"builtin\"\n";

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&user_reg)?;
    file.write_all(entries.as_bytes())?;
    Ok(())
}

/// Start the game. `with_mods` swaps the native Vulkan renderer for the D3D11
/// path and lets the 3DMigoto proxy in — see `crate::game::mods` for why the
/// two are inseparable.
pub fn launch_game(settings: &AppSettings, with_mods: bool) -> Result<LaunchedGame, AppError> {
    let game_path = Path::new(&settings.game_dir);
    let exe_path = game_path.join("Endfield.exe");

    if !exe_path.exists() {
        return Err(AppError::GameNotFound(format!(
            "Executable not found: {}",
            exe_path.display()
        )));
    }

    let proton_dir = crate::game::proton::resolve_proton_dir(settings).ok_or_else(|| {
        AppError::ProtonNotFound(format!(
            "Proton not found at: {}",
            Path::new(&settings.proton_dir).join("proton").display()
        ))
    })?;
    let proton_path = proton_dir.join("proton");

    // Convert Linux path to Wine Z: path
    let wine_path = format!("Z:{}", exe_path.to_string_lossy().replace('/', "\\"));

    let compat_data = resolve_prefix_dir(settings, game_path);
    std::fs::create_dir_all(&compat_data)?;

    if with_mods {
        let _ = ensure_prefix_appdefaults(&compat_data);
    }

    let log_path = paths::launch_log_path();
    std::fs::create_dir_all(log_path.parent().unwrap())?;

    let mut effective_settings = settings.clone();
    effective_settings.proton_dir = proton_dir.to_string_lossy().to_string();
    let mut script = build_env_script(&effective_settings, &compat_data, with_mods);

    // cd into game directory
    script.push_str(&format!("cd {}\n", shell_escape(&game_path.to_string_lossy())));

    // Build the launch command with optional wrappers
    let proton_escaped = shell_escape(&proton_path.to_string_lossy());
    let wine_escaped = shell_escape(&wine_path);
    let launch_cmd = build_launch_cmd(settings, &proton_escaped, &wine_escaped, with_mods);

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
        on_exit: None,
    })
}

/// Shut down the prefix's wineserver (and with it every wine process of the
/// prefix, winedevice included). `force` escalates the signal it sends its
/// clients from SIGINT to SIGKILL, for processes too wedged to act on the
/// polite one.
///
/// This is the only reliable way to reap a wine session. `killpg` reaches
/// what the launch script started, but wineserver puts itself in a session of
/// its own (`setsid`), so a game process that outlived its wine loader —
/// after a crash, typically — is reparented out of our process group and
/// survives the group kill. Two reports come from that: the stop button
/// leaving `Endfield.exe` running (issue #33), and the same leftovers piling
/// up across crashed sessions until the X server refuses new connections
/// ("Maximum number of clients reached") and the next launch freezes on the
/// intro logo.
///
/// Inside the Flatpak the stakes are higher still: a surviving wineserver
/// keeps the old sandbox instance alive, and the next launch finds it through
/// the shared /run/user socket but cannot open its fsync shared memory — that
/// lives in the dead instance's private /dev/shm — so wine exits 1 before
/// loading anything, with no output.
///
/// Call after the game session ends, never while it may still be running.
pub fn shutdown_wineserver(settings: &AppSettings, force: bool) {
    let proton_dir = match crate::game::proton::resolve_proton_dir(settings) {
        Some(d) => d,
        None => return,
    };
    let wineserver = proton_dir.join("files/bin/wineserver");
    if !wineserver.exists() {
        return;
    }
    let prefix = resolve_prefix_dir(settings, Path::new(&settings.game_dir)).join("pfx");
    if !prefix.exists() {
        return;
    }
    let _ = Command::new(wineserver)
        .arg(if force { "-k9" } else { "-k" })
        .env("WINEPREFIX", &prefix)
        .status();
}

/// Run a Wine tool (winecfg, regedit, ...) inside the game's Proton prefix
/// with the exact environment the game itself gets. Fire-and-forget: the tool
/// opens its own window and the user closes it when done.
pub fn run_prefix_tool(settings: &AppSettings, tool: &str) -> Result<(), AppError> {
    let game_path = Path::new(&settings.game_dir);

    let proton_dir = crate::game::proton::resolve_proton_dir(settings).ok_or_else(|| {
        AppError::ProtonNotFound(format!(
            "Proton not found at: {}",
            Path::new(&settings.proton_dir).join("proton").display()
        ))
    })?;
    let proton_path = proton_dir.join("proton");

    let compat_data = resolve_prefix_dir(settings, game_path);
    std::fs::create_dir_all(&compat_data)?;

    let mut effective_settings = settings.clone();
    effective_settings.proton_dir = proton_dir.to_string_lossy().to_string();
    let mut script = build_env_script(&effective_settings, &compat_data, false);
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
    fn default_renderer_is_native_vulkan() {
        let mut settings = AppSettings::default();
        settings.use_native_vulkan = true;
        assert_eq!(renderer_args(&settings, false), "-vulkan");
    }

    #[test]
    fn modded_launch_forces_direct3d_11() {
        let mut settings = AppSettings::default();
        settings.use_native_vulkan = true;
        assert_eq!(renderer_args(&settings, true), "-force-d3d11");
    }

    #[test]
    fn directx_11_setting_forces_direct3d_11() {
        let mut settings = AppSettings::default();
        settings.use_native_vulkan = false;
        assert_eq!(renderer_args(&settings, false), "-force-d3d11");
        assert_eq!(renderer_args(&settings, true), "-force-d3d11");
    }

    #[test]
    fn launch_command_includes_wrappers_and_renderer() {
        let mut settings = AppSettings::default();
        settings.use_native_vulkan = true;
        settings.use_gamemode = true;
        settings.use_mangohud = true;
        settings.custom_launch_args = "-screen-fullscreen 0".to_string();

        let cmd = build_launch_cmd(&settings, "'proton'", "'Z:\\Endfield.exe'", false);
        assert_eq!(
            cmd,
            "gamemoderun mangohud 'proton' run 'Z:\\Endfield.exe' -vulkan -screen-fullscreen 0"
        );

        let modded_cmd = build_launch_cmd(&settings, "'proton'", "'Z:\\Endfield.exe'", true);
        assert_eq!(
            modded_cmd,
            "gamemoderun mangohud 'proton' run 'Z:\\Endfield.exe' -force-d3d11 -screen-fullscreen 0"
        );

        settings.gamemode_command = "taskset -c 0-7 gamemoderun".to_string();
        let custom_cmd = build_launch_cmd(&settings, "'proton'", "'Z:\\Endfield.exe'", false);
        assert_eq!(
            custom_cmd,
            "taskset -c 0-7 gamemoderun mangohud 'proton' run 'Z:\\Endfield.exe' -vulkan -screen-fullscreen 0"
        );
    }

    #[test]
    fn prefix_appdefaults_are_written_and_idempotent() {
        let tmp_dir =
            std::env::temp_dir().join(format!("llauncher_test_pfx_{}", std::process::id()));
        let pfx_dir = tmp_dir.join("pfx");
        let _ = std::fs::create_dir_all(&pfx_dir);
        let user_reg = pfx_dir.join("user.reg");

        std::fs::write(
            &user_reg,
            "WINE REGISTRY Version 2\n;; All keys relative to \\\\User\\\\...\n",
        )
        .unwrap();

        assert!(ensure_prefix_appdefaults(&tmp_dir).is_ok());
        let content = std::fs::read_to_string(&user_reg).unwrap();
        assert!(content.contains("[Software\\\\Wine\\\\AppDefaults\\\\Endfield.exe\\\\DllOverrides]"));
        assert!(content.contains(
            "[Software\\\\Wine\\\\AppDefaults\\\\PlatformProcess.exe\\\\DllOverrides]"
        ));
        assert!(content.contains(
            "[Software\\\\Wine\\\\AppDefaults\\\\QtWebEngineProcess.exe\\\\DllOverrides]"
        ));
        assert!(content.contains("\"d3d11\"=\"builtin\""));
        assert!(content.contains("\"d3d11\"=\"native,builtin\""));

        let len_first = content.len();
        assert!(ensure_prefix_appdefaults(&tmp_dir).is_ok());
        let content_second = std::fs::read_to_string(&user_reg).unwrap();
        assert_eq!(content_second.len(), len_first);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}

