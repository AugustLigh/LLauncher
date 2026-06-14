use tauri::{Emitter, State};

use crate::config::paths;
use crate::config::settings::AppSettings;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    let settings = state.settings.lock().await;
    Ok(settings.clone())
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), AppError> {
    settings.save()?;
    let mut current = state.settings.lock().await;
    *current = settings;
    Ok(())
}

#[tauri::command]
pub async fn get_game_version(
    state: State<'_, AppState>,
) -> Result<crate::api::types::GameVersionResponse, AppError> {
    let settings = state.settings.lock().await;
    let version = settings.installed_version.clone();
    drop(settings);
    crate::api::client::get_latest_game_version(&state.http_client, &version).await
}

#[tauri::command]
pub async fn get_launcher_content(
    state: State<'_, AppState>,
) -> Result<crate::api::types::LauncherContent, AppError> {
    let settings = state.settings.lock().await;
    let lang = settings.language.clone();
    drop(settings);
    crate::api::client::get_launcher_content(&state.http_client, &lang).await
}

#[tauri::command]
pub async fn check_game_state(
    state: State<'_, AppState>,
) -> Result<crate::game::state::GameState, AppError> {
    let settings = state.settings.lock().await;
    let game_dir = settings.game_dir.clone();
    let installed_version = settings.installed_version.clone();
    drop(settings);
    crate::game::state::determine_game_state(
        &state.http_client,
        &game_dir,
        &installed_version,
    )
    .await
}

#[tauri::command]
pub async fn start_download(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let settings = state.settings.lock().await;
    let download_dir = settings.download_dir.clone();
    let game_dir = settings.game_dir.clone();
    let installed_version = settings.installed_version.clone();
    let speed_limit = settings.download_speed_limit;
    let max_concurrent = settings.download_max_concurrent.clamp(1, 8);
    drop(settings);

    let client = state.http_client.clone();
    let download_active = state.download_active.clone();

    let version = crate::download::manager::start_download(
        app,
        client,
        download_active,
        &download_dir,
        &game_dir,
        &installed_version,
        speed_limit,
        max_concurrent,
    )
    .await?;

    // Persist the installed version in the backend right away. Relying on the
    // frontend to call `update_installed_version` loses the version if the
    // window is closed or the webview dies before the event is handled, which
    // left the launcher stuck offering "Install"/"Update" on the next start.
    let mut settings = state.settings.lock().await;
    settings.installed_version = version;
    settings.save()?;
    Ok(())
}

#[tauri::command]
pub async fn cancel_download(state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Verify the installed game's VFS assets against the official per-file resource
/// manifest and re-download only the files that are missing or corrupt.
///
/// Unlike `repair_game` (which re-fetches the full multi-GB pack set), this
/// hashes what is already on disk and pulls just the deltas — the same
/// mechanism the official launcher uses for updates. Reuses `download_active`
/// for cancellation, so `cancel_download` stops it too.
#[tauri::command]
pub async fn verify_game_integrity(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::download::resources::IntegrityComplete, AppError> {
    let settings = state.settings.lock().await;
    let game_dir = settings.game_dir.clone();
    let installed_version = settings.installed_version.clone();
    let max_concurrent = settings.download_max_concurrent;
    drop(settings);

    if installed_version.is_empty()
        || !std::path::Path::new(&game_dir).join("Endfield.exe").exists()
    {
        return Err(AppError::GameNotFound(
            "Game is not installed; nothing to verify".to_string(),
        ));
    }

    let client = state.http_client.clone();
    let cancel_flag = state.download_active.clone();

    let result = crate::download::resources::verify_and_repair(
        app.clone(),
        client,
        cancel_flag.clone(),
        game_dir,
        max_concurrent,
    )
    .await;

    cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);

    if let Err(ref e) = result {
        app.emit(
            "integrity://error",
            crate::api::types::DownloadError {
                message: e.to_string(),
            },
        )
        .ok();
    }
    result
}

/// Shared launch path used by the `launch_game` command and the tray menu.
///
/// Watches the spawned process until it exits. A quick exit (< 3.5s) is
/// reported as a launch failure with a tail of the log; in every case we reap
/// the child (no zombie), clear the running flag, record playtime and emit
/// `game://exited` so the UI can update / bring the window back.
pub async fn launch_and_watch(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;

    let state = app.state::<AppState>();
    if state.game_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(AppError::Api("Game is already running".to_string()));
    }

    let settings_clone = state.settings.lock().await.clone();

    let mut launched = crate::game::launcher::launch_game(&settings_clone)?;
    let game_running = state.game_running.clone();
    let game_pid = state.game_pid.clone();
    game_running.store(true, std::sync::atomic::Ordering::SeqCst);
    *game_pid.lock().unwrap() = Some(launched.child.id());
    let _ = app.emit("game://started", ());

    let discord = if settings_clone.use_discord_rpc {
        Some(crate::game::discord::start_presence())
    } else {
        None
    };

    let log_path = launched.log_path.clone();
    let app2 = app.clone();
    tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let session_start_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let status = loop {
            match launched.child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(500)),
                Err(_) => break None,
            }
        };

        game_running.store(false, std::sync::atomic::Ordering::SeqCst);
        *game_pid.lock().unwrap() = None;
        if let Some(discord) = discord {
            discord.store(false, std::sync::atomic::Ordering::SeqCst);
        }

        // Record playtime and last-played timestamp.
        {
            let state = app2.state::<AppState>();
            let mut settings = state.settings.blocking_lock();
            settings.total_playtime_secs += started.elapsed().as_secs();
            settings.last_played = session_start_unix;
            let _ = settings.save();
        }

        if let Some(status) = status {
            if started.elapsed() < std::time::Duration::from_millis(3500) {
                // Give the game a moment to flush its stderr/stdout.
                std::thread::sleep(std::time::Duration::from_millis(200));
                let log_tail = crate::game::launcher::read_log_tail(&log_path, 60);
                let _ = app2.emit(
                    "launch://failed",
                    crate::api::types::LaunchFailed {
                        exit_code: status.code(),
                        log_tail,
                    },
                );
            }
            let _ = app2.emit(
                "game://exited",
                crate::api::types::GameExited {
                    exit_code: status.code(),
                },
            );
        } else {
            let _ = app2.emit(
                "game://exited",
                crate::api::types::GameExited { exit_code: None },
            );
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn launch_game(app: tauri::AppHandle) -> Result<(), AppError> {
    launch_and_watch(app).await
}

#[tauri::command]
pub async fn stop_game(state: State<'_, AppState>) -> Result<(), AppError> {
    let pid = *state.game_pid.lock().unwrap();
    let Some(pid) = pid else {
        return Ok(());
    };

    // Ask the whole process group (bash -> proton -> game) to terminate, then
    // escalate to SIGKILL if it is still alive a few seconds later.
    unsafe {
        libc::killpg(pid as i32, libc::SIGTERM);
    }

    let game_running = state.game_running.clone();
    let game_pid = state.game_pid.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        if game_running.load(std::sync::atomic::Ordering::SeqCst) {
            if let Some(pid) = *game_pid.lock().unwrap() {
                unsafe {
                    libc::killpg(pid as i32, libc::SIGKILL);
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn is_game_running(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(state
        .game_running
        .load(std::sync::atomic::Ordering::SeqCst))
}

/// Point the launcher at an already existing game installation (e.g. one made
/// by the official launcher under Bottles/Lutris). The folder must contain
/// Endfield.exe. The install is assumed to be up to date; if it is not, the
/// user can run Repair. Returns the version recorded.
#[tauri::command]
pub async fn import_existing_game(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, AppError> {
    let game_path = std::path::Path::new(&path);
    if !game_path.join("Endfield.exe").exists() {
        return Err(AppError::GameNotFound(format!(
            "Endfield.exe not found in {}",
            path
        )));
    }

    let version_info =
        crate::api::client::get_latest_game_version(&state.http_client, "").await?;
    let version = version_info.version.clone();

    let mut settings = state.settings.lock().await;
    settings.game_dir = path.clone();
    settings.download_dir = game_path.join("_download").to_string_lossy().to_string();
    settings.installed_version = version.clone();
    settings.save()?;

    Ok(version)
}

/// Delete the game installation (game directory + its _download cache) and
/// reset the installed version. Refuses while the game is running, and only
/// acts when the directory actually contains the game.
#[tauri::command]
pub async fn uninstall_game(state: State<'_, AppState>) -> Result<(), AppError> {
    if state.game_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(AppError::Api("Cannot uninstall while the game is running".to_string()));
    }

    let settings = state.settings.lock().await;
    let game_dir = settings.game_dir.clone();
    let download_dir = settings.download_dir.clone();
    drop(settings);

    let game_path = std::path::PathBuf::from(&game_dir);
    if !game_path.join("Endfield.exe").exists()
        && !crate::game::state::incomplete_marker(&game_path).exists()
    {
        return Err(AppError::GameNotFound(format!(
            "No game installation found in {}",
            game_dir
        )));
    }

    let download_path = std::path::PathBuf::from(&download_dir);
    tokio::task::spawn_blocking(move || {
        std::fs::remove_dir_all(&game_path).ok();
        // Only remove the download cache if it follows the _download naming —
        // the user may have pointed it at a shared folder.
        if download_path.file_name().is_some_and(|n| n == "_download") {
            std::fs::remove_dir_all(&download_path).ok();
        }
    })
    .await
    .map_err(|e| AppError::Api(format!("Uninstall task failed: {}", e)))?;

    let mut settings = state.settings.lock().await;
    settings.installed_version = String::new();
    settings.save()?;
    Ok(())
}

/// Collect system / configuration info for bug reports.
#[tauri::command]
pub async fn get_debug_info(state: State<'_, AppState>) -> Result<String, AppError> {
    let settings = state.settings.lock().await.clone();

    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let run = |cmd: &str, args: &[&str]| -> String {
        let mut command = std::process::Command::new(cmd);
        command.args(args);
        crate::util::strip_appimage_libs(&mut command);
        command
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    };

    let kernel = run("uname", &["-r"]);
    let gpu = run("sh", &["-c", "lspci -nn 2>/dev/null | grep -Ei 'vga|3d' | sed 's/^[0-9a-f:.]* //'"]);
    let proton = std::path::Path::new(&settings.proton_dir)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| settings.proton_dir.clone());

    let log_tail = crate::game::launcher::read_log_tail(&paths::launch_log_path(), 30);

    Ok(format!(
        "LLauncher {version}\n\
         OS: {os} (kernel {kernel})\n\
         Session: {desktop} / {session}\n\
         GPU: {gpu}\n\
         Proton: {proton}\n\
         Game version: {game_version}\n\
         ntsync: {ntsync}\n\
         Flags: vulkan={vulkan} wayland={wayland} dxvk_async={dxvk} gamemode={gamemode} mangohud={mangohud} prime={prime} fsync_off={fsync} esync_off={esync}\n\
         \n--- launch.log tail ---\n{log_tail}",
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
        prime = settings.use_prime_offload,
        fsync = settings.disable_fsync,
        esync = settings.disable_esync,
        log_tail = log_tail,
    ))
}

#[tauri::command]
pub async fn read_launch_log() -> Result<String, AppError> {
    let log_path = paths::launch_log_path();
    if !log_path.exists() {
        return Ok(String::new());
    }
    Ok(std::fs::read_to_string(&log_path)?)
}

#[tauri::command]
pub async fn repair_game(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Force a full repair by passing an empty installed_version: the API
    // returns the complete pack list, and the worker auto-skips already-valid
    // pack files via MD5, so untouched data is not re-downloaded.
    let settings = state.settings.lock().await;
    let download_dir = settings.download_dir.clone();
    let game_dir = settings.game_dir.clone();
    let speed_limit = settings.download_speed_limit;
    let max_concurrent = settings.download_max_concurrent.clamp(1, 8);
    drop(settings);

    let client = state.http_client.clone();
    let download_active = state.download_active.clone();

    let version = crate::download::manager::start_download(
        app,
        client,
        download_active,
        &download_dir,
        &game_dir,
        "",
        speed_limit,
        max_concurrent,
    )
    .await?;

    let mut settings = state.settings.lock().await;
    settings.installed_version = version;
    settings.save()?;
    Ok(())
}

#[tauri::command]
pub async fn update_installed_version(
    state: State<'_, AppState>,
    version: String,
) -> Result<(), AppError> {
    let mut settings = state.settings.lock().await;
    settings.installed_version = version;
    settings.save()?;
    Ok(())
}

#[tauri::command]
pub async fn check_system_requirements(
    state: State<'_, AppState>,
) -> Result<crate::game::proton::SystemCheck, AppError> {
    let settings = state.settings.lock().await;
    let proton_dir = settings.proton_dir.clone();
    drop(settings);
    Ok(crate::game::proton::check_system(&proton_dir))
}

#[tauri::command]
pub async fn get_dwproton_latest(
    state: State<'_, AppState>,
) -> Result<crate::api::types::ProtonReleaseInfo, AppError> {
    crate::download::proton::get_latest_dwproton_info(&state.http_client).await
}

#[tauri::command]
pub async fn list_dwproton_releases(
    state: State<'_, AppState>,
) -> Result<Vec<crate::api::types::ProtonReleaseInfo>, AppError> {
    crate::download::proton::list_dwproton_releases(&state.http_client).await
}

#[tauri::command]
pub async fn list_installed_protons() -> Result<Vec<crate::api::types::InstalledProton>, AppError> {
    let base = crate::config::paths::default_proton_dir();
    let mut installed = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("proton").exists() {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                installed.push(crate::api::types::InstalledProton {
                    name,
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }

    installed.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(installed)
}

#[tauri::command]
pub async fn set_active_proton(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), AppError> {
    let mut settings = state.settings.lock().await;
    settings.proton_dir = path;
    settings.save()?;
    Ok(())
}

#[tauri::command]
pub async fn download_dwproton(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    release: Option<crate::api::types::ProtonReleaseInfo>,
) -> Result<(), AppError> {
    // Always download to the base proton directory
    let base_dir = crate::config::paths::default_proton_dir()
        .to_string_lossy()
        .to_string();

    let client = state.http_client.clone();
    let cancel_flag = state.proton_download_active.clone();
    cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);

    let result =
        crate::download::proton::download_and_extract_dwproton(&app, &client, &cancel_flag, &base_dir, release)
            .await;

    cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);

    match result {
        Ok((proton_dir, _version)) => {
            let mut settings = state.settings.lock().await;
            settings.proton_dir = proton_dir;
            settings.save()?;
            Ok(())
        }
        Err(e) => {
            app.emit(
                "proton://error",
                crate::api::types::DownloadError {
                    message: e.to_string(),
                },
            )
            .ok();
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn cancel_proton_download(state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .proton_download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}
