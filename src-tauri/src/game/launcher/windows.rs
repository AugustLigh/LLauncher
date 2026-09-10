//! Windows: the game runs natively, so launching it is little more than
//! spawning the executable in its own directory.
//!
//! What the launcher adds on top is host-side: the game's entry on the
//! Graphics settings page (dedicated GPU, windowed-game optimizations), the
//! power plan for the length of the session, the process priority class, and
//! the `-vulkan` switch that puts the game on the renderer every Linux
//! session already uses. All of it lives in `game::windows_tweaks` and is
//! applied here, right before the process starts.
//!
//! The one wrinkle is elevation. The game's anti-cheat may ship an executable
//! manifested as `requireAdministrator`; `CreateProcess` (and with it
//! `Command::spawn`) cannot start such a binary and fails with
//! `ERROR_ELEVATION_REQUIRED` instead of showing the UAC prompt. The elevated
//! path therefore goes through `ShellExecuteExW`, which raises the prompt and
//! hands back a process handle we can watch just like a `Child`.

use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, SetPriorityClass, WaitForSingleObject, ABOVE_NORMAL_PRIORITY_CLASS,
};
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::{parse_custom_env_vars, ExitInfo, GameProcess, LaunchedGame, SessionCleanup};
use crate::config::paths;
use crate::config::settings::AppSettings;
use crate::error::AppError;
use crate::game::windows_tweaks;

/// Keep the game out of the launcher's console signal handling, so closing
/// the launcher cannot take the game with it (the Linux side does the same
/// with `process_group(0)`).
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
/// Don't flash a console window for the helper processes we shell out to.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `CreateProcess` refuses a `requireAdministrator` binary with this code.
const ERROR_ELEVATION_REQUIRED: i32 = 740;

/// A modded launch stays on D3D11 whatever the renderer toggle says: the
/// loader's `d3d11.dll` hooks that API and nothing else, and it wins the DLL
/// search order from the game directory without any further help.
pub fn launch_game(settings: &AppSettings, with_mods: bool) -> Result<LaunchedGame, AppError> {
    let game_path = Path::new(&settings.game_dir);
    let exe_path = game_path.join("Endfield.exe");

    if !exe_path.exists() {
        return Err(AppError::GameNotFound(format!(
            "Executable not found: {}",
            exe_path.display()
        )));
    }

    let log_path = paths::launch_log_path();
    if let Some(dir) = log_path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    // Host-side tweaks go first: Windows reads the graphics entry when the
    // process is created, and the power plan should be in place before the
    // game starts loading. A registry failure is logged and the launch goes
    // ahead — the toggles are conveniences, not requirements.
    if let Err(e) = windows_tweaks::sync_gpu_preferences(&exe_path, settings) {
        crate::logging::warn(format!("graphics preferences: {}", e));
    }
    let power_plan = if settings.windows_high_perf_power {
        windows_tweaks::switch_to_high_performance_plan()
    } else {
        None
    };

    let args = build_launch_args(settings, with_mods);
    match spawn(settings, &exe_path, game_path, &args, &log_path) {
        Ok(process) => Ok(LaunchedGame {
            process,
            log_path,
            on_exit: power_plan
                .map(|plan| Box::new(move || plan.restore()) as SessionCleanup),
        }),
        Err(e) => {
            // Nothing to wait for, so nothing to restore later: do it now.
            if let Some(plan) = power_plan {
                plan.restore();
            }
            Err(e)
        }
    }
}

/// The game's command line as one string: `-vulkan` when the renderer toggle
/// asks for it, then whatever the user typed. Passed through verbatim on both
/// launch paths, exactly as the Linux side appends it to the shell command —
/// the user typed a command line, not a list of pre-split arguments.
fn build_launch_args(settings: &AppSettings, with_mods: bool) -> String {
    let mut args = String::new();
    if settings.windows_use_vulkan && !with_mods {
        args.push_str("-vulkan");
    }
    let extra = settings.custom_launch_args.trim();
    if !extra.is_empty() {
        if !args.is_empty() {
            args.push(' ');
        }
        args.push_str(extra);
    }
    args
}

/// Above normal rather than high: `HIGH_PRIORITY_CLASS` outranks the audio
/// and input stacks the game depends on and is known to make things worse,
/// above-normal only puts the game ahead of ordinary background apps.
fn priority_flags(settings: &AppSettings) -> u32 {
    if settings.windows_high_priority {
        ABOVE_NORMAL_PRIORITY_CLASS
    } else {
        0
    }
}

fn spawn(
    settings: &AppSettings,
    exe_path: &Path,
    game_path: &Path,
    args: &str,
    log_path: &Path,
) -> Result<GameProcess, AppError> {
    if settings.windows_run_as_admin {
        return launch_elevated(exe_path, game_path, args, settings);
    }

    let mut cmd = Command::new(exe_path);
    cmd.current_dir(game_path);
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | priority_flags(settings));

    for (key, value) in parse_custom_env_vars(&settings.custom_env_vars) {
        cmd.env(key, value);
    }

    // The game is a GUI process and normally writes nothing here, but a crash
    // handler or a `-log`-style flag might — and the failure dialog reads this
    // file, so give it somewhere to land.
    if let Ok(log_file) = std::fs::File::create(log_path) {
        if let Ok(err_file) = log_file.try_clone() {
            cmd.stdout(log_file).stderr(err_file);
        }
    }

    if !args.is_empty() {
        cmd.raw_arg(args);
    }

    match cmd.spawn() {
        Ok(child) => Ok(GameProcess::Child(child)),
        Err(e) if e.raw_os_error() == Some(ERROR_ELEVATION_REQUIRED) => {
            // The game demands administrator rights. Retry through the shell
            // so Windows shows the UAC prompt instead of failing outright.
            crate::logging::info(
                "game requires elevation — retrying the launch through ShellExecuteEx".to_string(),
            );
            launch_elevated(exe_path, game_path, args, settings)
        }
        Err(e) => Err(AppError::GameNotFound(format!("Failed to launch: {}", e))),
    }
}

/// Start the game through `ShellExecuteExW` with the `runas` verb: Windows
/// raises the UAC prompt and starts the process elevated.
///
/// Neither custom environment variables nor output redirection survive this
/// path — the elevated process is created by the AppInfo service, not by us —
/// so both are dropped, with a note in the launcher log. The priority class
/// is attempted afterwards on the handle we get back; an unelevated launcher
/// is normally denied that on an elevated process, which is logged too.
fn launch_elevated(
    exe_path: &Path,
    working_dir: &Path,
    args: &str,
    settings: &AppSettings,
) -> Result<GameProcess, AppError> {
    if !settings.custom_env_vars.trim().is_empty() {
        crate::logging::warn(
            "custom environment variables are ignored for an elevated launch".to_string(),
        );
    }

    let verb = wide("runas");
    let file = wide(&exe_path.to_string_lossy());
    let directory = wide(&working_dir.to_string_lossy());
    let parameters = wide(args.trim());

    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpDirectory = directory.as_ptr();
    if !args.trim().is_empty() {
        info.lpParameters = parameters.as_ptr();
    }
    info.nShow = SW_SHOWNORMAL as i32;

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok == 0 || info.hProcess.is_null() {
        // Includes the user clicking "No" on the UAC prompt (ERROR_CANCELLED).
        return Err(AppError::GameNotFound(format!(
            "Failed to launch elevated: {}",
            std::io::Error::last_os_error()
        )));
    }

    let priority = priority_flags(settings);
    if priority != 0 && unsafe { SetPriorityClass(info.hProcess, priority) } == 0 {
        crate::logging::warn(format!(
            "process priority: not applied to the elevated game process ({})",
            std::io::Error::last_os_error()
        ));
    }

    Ok(GameProcess::Handle(ProcessHandle::new(info.hProcess)))
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// An owned Win32 process handle, closed on drop.
pub struct ProcessHandle {
    handle: HANDLE,
}

// The handle is only ever touched through `&mut self` and the Win32 calls used
// here are handle-safe across threads; the launch watcher moves it into a
// blocking task.
unsafe impl Send for ProcessHandle {}

impl ProcessHandle {
    fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

    pub fn id(&self) -> u32 {
        // The rest of the launcher tracks the game by PID (that is what the
        // stop button works from), so resolve it from the handle.
        unsafe { windows_sys::Win32::System::Threading::GetProcessId(self.handle) }
    }

    pub fn try_wait(&mut self) -> std::io::Result<Option<ExitInfo>> {
        // Zero timeout: poll, exactly like `Child::try_wait`.
        let waited = unsafe { WaitForSingleObject(self.handle, 0) };
        if waited != WAIT_OBJECT_0 {
            return Ok(None);
        }
        let mut code: u32 = 0;
        let ok = unsafe { GetExitCodeProcess(self.handle, &mut code) };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Some(ExitInfo {
            code: Some(code as i32),
        }))
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { CloseHandle(self.handle) };
        }
    }
}

/// Ask the game and everything it started to close.
///
/// A game running elevated cannot be signalled from an unelevated launcher —
/// Windows denies the access — so the stop button is a no-op for those
/// sessions; closing the game from inside it still works.
pub fn request_stop(pid: u32) {
    taskkill(pid, false);
}

/// Kill the game's process tree outright, for when the polite request was
/// ignored.
pub fn force_stop(pid: u32) {
    taskkill(pid, true);
}

fn taskkill(pid: u32, force: bool) {
    let mut cmd = Command::new("taskkill");
    cmd.arg("/PID").arg(pid.to_string()).arg("/T");
    if force {
        cmd.arg("/F");
    }
    cmd.creation_flags(CREATE_NO_WINDOW);
    let _ = cmd.status();
}

// ---------------------------------------------------------------------------
// Proton-only surface. The UI hides these controls on Windows; the stubs keep
// the command list identical across platforms so a stale frontend gets a clear
// error instead of "command not found".
// ---------------------------------------------------------------------------

/// There is no Wine prefix on Windows. Returned only so the prefix commands
/// have something to report; nothing is ever created here.
pub fn resolve_prefix_dir(_settings: &AppSettings, _game_path: &Path) -> PathBuf {
    paths::data_base().join("prefix")
}

/// No wineserver to reap.
pub fn shutdown_wineserver(_settings: &AppSettings, _force: bool) {}

pub fn run_prefix_tool(_settings: &AppSettings, tool: &str) -> Result<(), AppError> {
    Err(AppError::Unsupported(format!("Wine tool {}", tool)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(vulkan: bool, extra: &str) -> AppSettings {
        let mut s = AppSettings::default();
        s.windows_use_vulkan = vulkan;
        s.custom_launch_args = extra.to_string();
        s
    }

    #[test]
    fn default_command_line_matches_the_official_launcher() {
        // No renderer flag unless asked for: the official launcher starts the
        // game bare, on D3D11, and so do we.
        assert_eq!(build_launch_args(&settings(false, "  "), false), "");
    }

    #[test]
    fn vulkan_flag_precedes_the_users_arguments() {
        assert_eq!(
            build_launch_args(&settings(true, " -screen-fullscreen 0 "), false),
            "-vulkan -screen-fullscreen 0"
        );
        assert_eq!(build_launch_args(&settings(true, ""), false), "-vulkan");
    }

    #[test]
    fn modded_launch_stays_on_d3d11() {
        assert_eq!(build_launch_args(&settings(true, "-log"), true), "-log");
    }

    #[test]
    fn priority_class_is_above_normal_or_nothing() {
        let mut s = AppSettings::default();
        assert_eq!(priority_flags(&s), 0);
        s.windows_high_priority = true;
        assert_eq!(priority_flags(&s), ABOVE_NORMAL_PRIORITY_CLASS);
    }
}
