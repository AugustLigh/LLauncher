//! Windows: the game runs natively, so launching it is little more than
//! spawning the executable in its own directory.
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
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::{parse_custom_env_vars, ExitInfo, GameProcess, LaunchedGame};
use crate::config::paths;
use crate::config::settings::AppSettings;
use crate::error::AppError;

/// Keep the game out of the launcher's console signal handling, so closing
/// the launcher cannot take the game with it (the Linux side does the same
/// with `process_group(0)`).
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
/// Don't flash a console window for the helper processes we shell out to.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `CreateProcess` refuses a `requireAdministrator` binary with this code.
const ERROR_ELEVATION_REQUIRED: i32 = 740;

pub fn launch_game(settings: &AppSettings) -> Result<LaunchedGame, AppError> {
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

    if settings.windows_run_as_admin {
        let process = launch_elevated(&exe_path, game_path, &settings.custom_launch_args, settings)?;
        return Ok(LaunchedGame { process, log_path });
    }

    let mut cmd = Command::new(&exe_path);
    cmd.current_dir(game_path);
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);

    for (key, value) in parse_custom_env_vars(&settings.custom_env_vars) {
        cmd.env(key, value);
    }

    // The game is a GUI process and normally writes nothing here, but a crash
    // handler or a `-log`-style flag might — and the failure dialog reads this
    // file, so give it somewhere to land.
    if let Ok(log_file) = std::fs::File::create(&log_path) {
        if let Ok(err_file) = log_file.try_clone() {
            cmd.stdout(log_file).stderr(err_file);
        }
    }

    // Passed through verbatim, exactly as the Linux side appends them to the
    // shell command line: the user typed a command line, not a list of
    // pre-split arguments.
    let extra = settings.custom_launch_args.trim();
    if !extra.is_empty() {
        cmd.raw_arg(extra);
    }

    match cmd.spawn() {
        Ok(child) => Ok(LaunchedGame {
            process: GameProcess::Child(child),
            log_path,
        }),
        Err(e) if e.raw_os_error() == Some(ERROR_ELEVATION_REQUIRED) => {
            // The game demands administrator rights. Retry through the shell
            // so Windows shows the UAC prompt instead of failing outright.
            crate::logging::info(
                "game requires elevation — retrying the launch through ShellExecuteEx".to_string(),
            );
            let process = launch_elevated(&exe_path, game_path, extra, settings)?;
            Ok(LaunchedGame { process, log_path })
        }
        Err(e) => Err(AppError::GameNotFound(format!("Failed to launch: {}", e))),
    }
}

/// Start the game through `ShellExecuteExW` with the `runas` verb: Windows
/// raises the UAC prompt and starts the process elevated.
///
/// Neither custom environment variables nor output redirection survive this
/// path — the elevated process is created by the AppInfo service, not by us —
/// so both are dropped, with a note in the launcher log.
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
pub fn shutdown_wineserver(_settings: &AppSettings) {}

pub fn run_prefix_tool(_settings: &AppSettings, tool: &str) -> Result<(), AppError> {
    Err(AppError::Unsupported(format!("Wine tool {}", tool)))
}
