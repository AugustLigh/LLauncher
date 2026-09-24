use tauri::State;
use crate::config::paths;
use crate::config::settings::AppSettings;
use crate::error::AppError;
use crate::state::AppState;

/// Collect system / configuration info for bug reports.
#[tauri::command]
#[specta::specta]
pub async fn get_debug_info(state: State<'_, AppState>) -> Result<String, AppError> {
    let settings = state.settings.lock().await.clone();

    // Shells out to uname/lspci and reads the log file — keep that off the
    // async runtime thread.
    tokio::task::spawn_blocking(move || build_debug_info(settings))
        .await
        .map_err(|e| AppError::Api(format!("debug info task failed: {}", e)))
}

pub(crate) fn build_debug_info(settings: AppSettings) -> String {
    // A wider tail than the launch-failure path: in-game crashes (e.g. issue
    // #21) abort long after the startup banner, so 30 lines often miss the
    // actual backtrace.
    let log_tail = crate::game::launcher::read_log_tail(&paths::launch_log_path(), 200);
    let launcher_log = crate::logging::tail(60);

    format!(
        "{header}\n\
         \n--- launcher.log tail ---\n{launcher_log}\n\
         \n--- launch.log tail ---\n{log_tail}",
        header = debug_header(&settings),
        launcher_log = launcher_log,
        log_tail = log_tail,
    )
}

/// Run a command and capture its trimmed stdout; empty string on any failure.
fn run_capture(cmd: &str, args: &[&str]) -> String {
    let mut command = std::process::Command::new(cmd);
    command.args(args);
    crate::util::strip_appimage_libs(&mut command);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: don't flash a console window at the user.
        command.creation_flags(0x0800_0000);
    }
    command
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn debug_header(settings: &AppSettings) -> String {
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            c.lines().find(|l| l.starts_with("PRETTY_NAME=")).map(|l| {
                l.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string()
            })
        })
        .unwrap_or_else(|| "unknown".to_string());

    let kernel = run_capture("uname", &["-r"]);
    let gpu = run_capture(
        "sh",
        &[
            "-c",
            "lspci -nn 2>/dev/null | grep -Ei 'vga|3d' | sed 's/^[0-9a-f:.]* //'",
        ],
    );
    let proton = std::path::Path::new(&settings.proton_dir)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| settings.proton_dir.clone());

    format!(
        "LLauncher {version}\n\
         OS: {os} (kernel {kernel})\n\
         Session: {desktop} / {session}\n\
         GPU: {gpu}\n\
         Proton: {proton}\n\
         Game version: {game_version}\n\
         ntsync: {ntsync}\n\
         Flags: vulkan={vulkan} wayland={wayland} dxvk_async={dxvk} gamemode={gamemode} mangohud={mangohud} gamescope={gamescope} prime={prime} fsync_off={fsync} esync_off={esync} sdl_input={sdl_input}\n\
         Flatpak: {flatpak}",
        version = env!("CARGO_PKG_VERSION"),
        os = os,
        kernel = kernel,
        desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "?".into()),
        session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "?".into()),
        gpu = if gpu.is_empty() { "unknown".to_string() } else { gpu },
        proton = proton,
        game_version = if settings.installed_version.is_empty() { "not installed" } else { &settings.installed_version },
        ntsync = std::path::Path::new("/dev/ntsync").exists(),
        vulkan = settings.use_native_vulkan,
        wayland = settings.use_wayland,
        dxvk = settings.use_dxvk_async,
        gamemode = settings.use_gamemode,
        mangohud = settings.use_mangohud,
        gamescope = settings.use_gamescope,
        prime = settings.use_prime_offload,
        fsync = settings.disable_fsync,
        esync = settings.disable_esync,
        sdl_input = settings.use_sdl_input,
        flatpak = std::env::var_os("FLATPAK_ID").is_some(),
    )
}

#[cfg(target_os = "macos")]
fn debug_header(settings: &AppSettings) -> String {
    let os = run_capture("sw_vers", &["-productVersion"]);
    let build = run_capture("sw_vers", &["-buildVersion"]);
    // `machdep.cpu.brand_string` names the Apple silicon or Intel part; the
    // GPU is the same chip on Apple silicon, so one line covers both.
    let cpu = run_capture("sysctl", &["-n", "machdep.cpu.brand_string"]);
    // 1 on an arm64 Mac, so a bug report says whether the game came through
    // Rosetta at all.
    let arm = run_capture("sysctl", &["-n", "hw.optional.arm64"]) == "1";

    let wine = crate::game::launcher::resolve_wine(settings)
        .map(|w| w.wine.to_string_lossy().to_string())
        .unwrap_or_else(|| "not found".to_string());
    let dxmt = crate::download::wine::dxmt_version_for(std::path::Path::new(&wine))
        .unwrap_or_else(|| "none".to_string());
    let modules = crate::download::wine::modules_version_for(std::path::Path::new(&wine))
        .unwrap_or_else(|| "none".to_string());

    format!(
        "LLauncher {version}\n\
         OS: macOS {os} (build {build})\n\
         CPU: {cpu} (arm64: {arm}, rosetta: {rosetta})\n\
         Wine: {wine}\n\
         DXMT: {dxmt}\n\
         Endfield modules: {modules}\n\
         Game version: {game_version}\n\
         Flags: avx={avx} metal_hud={hud} vulkan={vulkan} discord_rpc={discord} on_launch={on_launch}",
        version = env!("CARGO_PKG_VERSION"),
        os = if os.is_empty() { "version unknown".to_string() } else { os },
        build = if build.is_empty() { "?".to_string() } else { build },
        cpu = if cpu.is_empty() { "unknown".to_string() } else { cpu },
        arm = arm,
        rosetta = crate::download::wine::rosetta_available(),
        wine = wine,
        dxmt = dxmt,
        modules = modules,
        game_version = if settings.installed_version.is_empty() { "not installed" } else { &settings.installed_version },
        avx = settings.macos_advertise_avx,
        hud = settings.macos_metal_hud,
        vulkan = settings.macos_native_vulkan,
        discord = settings.use_discord_rpc,
        on_launch = settings.on_launch_action,
    )
}

#[cfg(windows)]
fn debug_header(settings: &AppSettings) -> String {
    // One PowerShell round-trip for both facts: the OS caption with its build
    // number on the first line, the GPU names on the second. Spawning it twice
    // would double the ~half-second startup cost for no gain.
    let probe = run_capture(
        "powershell",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$os = Get-CimInstance Win32_OperatingSystem; \
             \"$($os.Caption) (build $($os.BuildNumber))\"; \
             (Get-CimInstance Win32_VideoController | \
              Select-Object -ExpandProperty Name) -join ', '",
        ],
    );
    let mut lines = probe.lines();
    let os = lines.next().unwrap_or("").trim().to_string();
    let gpu = lines.next().unwrap_or("").trim().to_string();

    format!(
        "LLauncher {version}\n\
         OS: {os}\n\
         GPU: {gpu}\n\
         Game version: {game_version}\n\
         Flags: run_as_admin={admin} discord_rpc={discord} on_launch={on_launch}",
        version = env!("CARGO_PKG_VERSION"),
        os = if os.is_empty() {
            "Windows (version unknown)".to_string()
        } else {
            os
        },
        gpu = if gpu.is_empty() {
            "unknown".to_string()
        } else {
            gpu
        },
        game_version = if settings.installed_version.is_empty() {
            "not installed"
        } else {
            &settings.installed_version
        },
        admin = settings.windows_run_as_admin,
        discord = settings.use_discord_rpc,
        on_launch = settings.on_launch_action,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn read_launch_log() -> Result<String, AppError> {
    tokio::task::spawn_blocking(|| {
        let log_path = paths::launch_log_path();
        if !log_path.exists() {
            return Ok(String::new());
        }
        Ok(std::fs::read_to_string(&log_path)?)
    })
    .await
    .map_err(|e| AppError::Api(format!("read log task failed: {}", e)))?
}


