//! macOS: the game is a Windows executable, run through the Wine build the
//! launcher installed — the official WineHQ Wine Staging package with the
//! Endfield modules and DXMT dropped in (see `download::wine`). There is no
//! Proton here (a Linux-only project), so that install is the macOS
//! equivalent of DWProton: it is what gets the game past its anti-cheat.
//!
//! Any other Wine build can be pointed at through the setting. Without the
//! patched modules the anti-cheat driver aborts on start, so that is a
//! debugging aid rather than a supported path — which is also why nothing
//! is auto-detected outside the launcher's own installs: a stray
//! `Wine Stable.app` would make the launcher say "ready" about a build that
//! cannot run the game.
//!
//! The prefix layout matches the Linux side exactly — `<base>/endfield/pfx` —
//! so `game::prefix`'s backup, restore and reset work here unchanged.

use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{parse_custom_env_vars, shell_escape, GameProcess, LaunchedGame};
use crate::config::paths;
use crate::config::settings::AppSettings;
use crate::error::AppError;

/// A usable Wine install: the loader that runs the game, and — when it sits
/// next to the loader, which it does in every build we know of — the server
/// that owns the prefix.
pub struct WineInstall {
    pub wine: PathBuf,
    pub wineserver: Option<PathBuf>,
}

/// The loader is plain `wine` in Wine 9+, which dropped the split, and
/// `wine64` in older builds someone might still point the setting at.
const LOADER_NAMES: [&str; 2] = ["wine", "wine64"];

/// Find the Wine install to use: the configured one if there is one, else the
/// launcher's most recent own install.
pub fn resolve_wine(settings: &AppSettings) -> Option<WineInstall> {
    let hint = settings.macos_wine_dir.trim();
    if !hint.is_empty() {
        // An explicit setting is never silently ignored: if it points at
        // nothing usable the caller reports that, rather than starting the
        // game under some other Wine the user did not choose.
        return from_hint(Path::new(hint));
    }
    crate::download::wine::list_installed(&paths::default_wine_dir())
        .into_iter()
        .find_map(|w| from_hint(Path::new(&w.path)))
}

/// Resolve one path — a loader binary, an install root, or an app bundle —
/// into a `WineInstall`.
fn from_hint(path: &Path) -> Option<WineInstall> {
    if is_executable_file(path) {
        return Some(WineInstall {
            wineserver: sibling_wineserver(path),
            wine: path.to_path_buf(),
        });
    }
    if !path.is_dir() {
        return None;
    }

    // The directory may be the `bin` itself, an install root above it, or a
    // WineHQ-style application bundle with the Wine tree buried inside.
    let bin_dirs = [
        path.to_path_buf(),
        path.join("bin"),
        path.join("Contents/Resources/wine/bin"),
    ];

    bin_dirs.iter().find_map(|dir| {
        LOADER_NAMES
            .iter()
            .map(|name| dir.join(name))
            .find(|loader| is_executable_file(loader))
            .map(|wine| WineInstall {
                wineserver: sibling_wineserver(&wine),
                wine,
            })
    })
}

fn sibling_wineserver(wine: &Path) -> Option<PathBuf> {
    let server = wine.parent()?.join("wineserver");
    server.is_file().then_some(server)
}

fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Resolve the Wine prefix directory. Same layout as the Linux side — the
/// prefix proper lives in a `pfx` subdirectory — so the prefix tools do not
/// need to know which platform made it.
pub fn resolve_prefix_dir(settings: &AppSettings, _game_path: &Path) -> PathBuf {
    if !settings.proton_prefix_dir.trim().is_empty() {
        return Path::new(settings.proton_prefix_dir.trim()).join("endfield");
    }
    paths::default_proton_prefix_dir().join("endfield")
}

/// Shared preamble for anything run inside the game's prefix, so the game and
/// the prefix tools (winecfg etc.) see the exact same environment.
fn build_env_script(settings: &AppSettings, prefix: &Path, with_mods: bool) -> String {
    let mut script = String::new();

    script.push_str(&format!(
        "export WINEPREFIX={}\n",
        shell_escape(&prefix.to_string_lossy())
    ));

    // Wine's default debug channels cost real frames and fill the launch log
    // with noise that buries the lines the failure dialog looks for.
    script.push_str("export WINEDEBUG=-all\n");

    // Rosetta hides AVX from translated code unless asked; the game's
    // binaries and DXMT expect it to be there. Ignored on an Intel Mac.
    if settings.macos_advertise_avx {
        script.push_str("export ROSETTA_ADVERTISE_AVX=1\n");
    }

    if settings.macos_metal_hud {
        script.push_str("export MTL_HUD_ENABLED=1\n");
    }

    // Same reason as on Linux: a modded launch needs Wine to load the game's
    // own d3d11.dll — the 3DMigoto proxy — ahead of its builtin, and `dxgi`
    // rides along for ReShade. See `crate::game::mods`.
    if with_mods {
        script.push_str("export WINEDLLOVERRIDES='d3d11=n,b;dxgi=n,b'\n");
    }

    // Custom env vars (KEY=VALUE per line), last so they can override ours.
    for (key, value) in parse_custom_env_vars(&settings.custom_env_vars) {
        script.push_str(&format!("export {}={}\n", key, shell_escape(value)));
    }

    script
}

/// The renderer switch on the command line. The game defaults to its own
/// Vulkan renderer (or D3D12) and a direct launch of `Endfield.exe` does not
/// inherit whatever the official launcher would have chosen — so without a
/// word from us it goes Vulkan → MoltenVK, which does not render. DXMT only
/// does Direct3D 11, and 3DMigoto only hooks Direct3D 11, hence the default.
fn renderer_args(settings: &AppSettings, with_mods: bool) -> &'static str {
    if settings.macos_native_vulkan && !with_mods {
        "-vulkan"
    } else {
        "-force-d3d11"
    }
}

/// Start the game. `with_mods` keeps the D3D11 path and lets the 3DMigoto
/// proxy in — see `crate::game::mods`.
pub fn launch_game(settings: &AppSettings, with_mods: bool) -> Result<LaunchedGame, AppError> {
    let game_path = Path::new(&settings.game_dir);
    let exe_path = game_path.join("Endfield.exe");

    if !exe_path.exists() {
        return Err(AppError::GameNotFound(format!(
            "Executable not found: {}",
            exe_path.display()
        )));
    }

    let wine = resolve_wine(settings).ok_or_else(|| AppError::WineNotFound(wine_hint(settings)))?;

    // Every Wine build for macOS is x86-64. Without Rosetta the loader dies
    // with "Bad CPU type in executable" before writing a line of log, which
    // is a poor way to learn that.
    if crate::download::wine::host_is_apple_silicon() && !crate::download::wine::rosetta_available() {
        return Err(AppError::Api(
            "Rosetta 2 is not installed. Download Wine again from Settings > Wine \
             (the launcher installs Rosetta on the way), or run \
             `softwareupdate --install-rosetta` in Terminal."
                .to_string(),
        ));
    }

    let prefix = resolve_prefix_dir(settings, game_path).join("pfx");
    std::fs::create_dir_all(&prefix)?;
    ensure_winemetal_in_prefix(&wine, &prefix);

    let log_path = paths::launch_log_path();
    std::fs::create_dir_all(log_path.parent().unwrap())?;

    let mut script = build_env_script(settings, &prefix, with_mods);
    script.push_str(&format!(
        "cd {}\n",
        shell_escape(&game_path.to_string_lossy())
    ));

    // Wine takes a Unix path for the executable and translates it itself, so
    // unlike the Linux side there is no Z: drive path to build.
    let mut launch_cmd = format!(
        "{} {} {}",
        shell_escape(&wine.wine.to_string_lossy()),
        shell_escape(&exe_path.to_string_lossy()),
        renderer_args(settings, with_mods)
    );

    if !settings.custom_launch_args.is_empty() {
        launch_cmd.push(' ');
        launch_cmd.push_str(&settings.custom_launch_args);
    }

    script.push_str(&format!(
        "exec {} > {} 2>&1\n",
        launch_cmd,
        shell_escape(&log_path.to_string_lossy())
    ));

    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(&script);

    // Detach into a new process group so closing the launcher (or the tray
    // hiding it) does not propagate signals to the game.
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

/// DXMT's `winemetal.dll` has to exist in the prefix's system32 as well as
/// among the builtins (its install guide says so). `wineboot` lays a
/// placeholder there when it creates the prefix, but a prefix made before
/// DXMT went into this Wine has none — so copy it in when it is missing.
/// Harmless for a Wine without DXMT: there is nothing to copy.
fn ensure_winemetal_in_prefix(wine: &WineInstall, prefix: &Path) {
    let Some(bin) = wine.wine.parent() else {
        return;
    };
    let builtin = bin.join("../lib/wine/x86_64-windows/winemetal.dll");
    if !builtin.is_file() {
        return;
    }
    let system32 = prefix.join("drive_c/windows/system32");
    if !system32.is_dir() || system32.join("winemetal.dll").exists() {
        return;
    }
    if let Err(e) = std::fs::copy(&builtin, system32.join("winemetal.dll")) {
        crate::logging::warn(format!("DXMT: cannot place winemetal.dll in the prefix: {}", e));
    }
}

/// Message for the "no Wine" failure, distinguishing a bad setting from a Mac
/// that simply has nothing installed — the fix differs.
fn wine_hint(settings: &AppSettings) -> String {
    let configured = settings.macos_wine_dir.trim();
    if configured.is_empty() {
        "No Wine installation found. Download one in Settings > Wine: the launcher \
         installs Wine Staging with the Endfield modules and DXMT."
            .to_string()
    } else {
        format!("No wine or wine64 binary under: {}", configured)
    }
}

/// Shut down the prefix's wineserver, and with it every wine process of the
/// prefix. `force` escalates the signal it sends its clients from SIGINT to
/// SIGKILL.
///
/// Same rationale as the Linux side: wineserver puts itself in a session of
/// its own, so a game that outlived its loader is reparented out of our
/// process group and survives the group kill.
pub fn shutdown_wineserver(settings: &AppSettings, force: bool) {
    let Some(wine) = resolve_wine(settings) else {
        return;
    };
    let Some(wineserver) = wine.wineserver else {
        return;
    };
    let prefix = resolve_prefix_dir(settings, Path::new(&settings.game_dir)).join("pfx");
    if !prefix.exists() {
        return;
    }
    let _ = Command::new(wineserver)
        .arg(if force { "-k9" } else { "-k" })
        .env("WINEPREFIX", &prefix)
        .status();
}

/// Run a Wine tool (winecfg, regedit, ...) inside the game's prefix with the
/// exact environment the game itself gets. Fire-and-forget: the tool opens its
/// own window and the user closes it when done.
pub fn run_prefix_tool(settings: &AppSettings, tool: &str) -> Result<(), AppError> {
    let wine = resolve_wine(settings).ok_or_else(|| AppError::WineNotFound(wine_hint(settings)))?;

    let prefix = resolve_prefix_dir(settings, Path::new(&settings.game_dir)).join("pfx");
    std::fs::create_dir_all(&prefix)?;

    let mut script = build_env_script(settings, &prefix, false);
    script.push_str(&format!(
        "exec {} {} > /dev/null 2>&1\n",
        shell_escape(&wine.wine.to_string_lossy()),
        shell_escape(tool)
    ));

    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(&script);
    cmd.process_group(0);
    cmd.spawn()
        .map_err(|e| AppError::Api(format!("Failed to run {}: {}", tool, e)))?;
    Ok(())
}

/// Ask the game to shut down: the whole process group (bash -> wine -> game)
/// gets a SIGTERM.
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
    fn env_script_always_pins_the_prefix() {
        let settings = AppSettings::default();
        let script = build_env_script(&settings, Path::new("/Users/a/prefix/endfield/pfx"), false);
        assert!(script.contains("export WINEPREFIX='/Users/a/prefix/endfield/pfx'\n"));
        // Defaults: the Rosetta AVX hint on, the HUD off.
        assert!(script.contains("export ROSETTA_ADVERTISE_AVX=1\n"));
        assert!(!script.contains("MTL_HUD_ENABLED"));
    }

    #[test]
    fn modded_launch_overrides_the_renderer_dlls() {
        // Without the override Wine loads its builtin d3d11 and the 3DMigoto
        // proxy never runs, which looks exactly like "the mods do nothing".
        let settings = AppSettings::default();
        let script = build_env_script(&settings, Path::new("/tmp/pfx"), true);
        assert!(script.contains("export WINEDLLOVERRIDES='d3d11=n,b;dxgi=n,b'\n"));
    }

    #[test]
    fn the_game_is_held_on_direct3d_11_unless_asked_otherwise() {
        // DXMT translates D3D11 only; the game's own default is Vulkan, which
        // MoltenVK cannot render for it.
        let mut settings = AppSettings::default();
        assert_eq!(renderer_args(&settings, false), "-force-d3d11");
        settings.macos_native_vulkan = true;
        assert_eq!(renderer_args(&settings, false), "-vulkan");
        // Mods hook D3D11, so they win over the Vulkan toggle.
        assert_eq!(renderer_args(&settings, true), "-force-d3d11");
    }

    #[test]
    fn an_explicit_wine_path_is_never_silently_replaced() {
        // A setting pointing at nothing usable must fail, not fall through to
        // whatever else happens to be installed on the machine.
        let mut settings = AppSettings::default();
        settings.macos_wine_dir = "/nonexistent/wine".to_string();
        assert!(resolve_wine(&settings).is_none());
    }
}
