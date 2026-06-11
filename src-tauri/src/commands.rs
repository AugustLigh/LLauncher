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

#[tauri::command]
pub async fn launch_game(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    if state.game_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(AppError::Api("Game is already running".to_string()));
    }

    let settings = state.settings.lock().await;
    let settings_clone = settings.clone();
    drop(settings);

    let mut launched = crate::game::launcher::launch_game(&settings_clone)?;
    let game_running = state.game_running.clone();
    game_running.store(true, std::sync::atomic::Ordering::SeqCst);

    // Watch the spawned process until it exits. A quick exit (< 3.5s) is
    // reported as a launch failure with a tail of the log; in every case we
    // reap the child (no zombie), clear the running flag and emit
    // `game://exited` so the UI can update / bring the window back.
    let log_path = launched.log_path.clone();
    tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let status = loop {
            match launched.child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(500)),
                Err(_) => break None,
            }
        };

        game_running.store(false, std::sync::atomic::Ordering::SeqCst);

        if let Some(status) = status {
            if started.elapsed() < std::time::Duration::from_millis(3500) {
                // Give the game a moment to flush its stderr/stdout.
                std::thread::sleep(std::time::Duration::from_millis(200));
                let log_tail = crate::game::launcher::read_log_tail(&log_path, 60);
                let _ = app.emit(
                    "launch://failed",
                    crate::api::types::LaunchFailed {
                        exit_code: status.code(),
                        log_tail,
                    },
                );
            }
            let _ = app.emit(
                "game://exited",
                crate::api::types::GameExited {
                    exit_code: status.code(),
                },
            );
        } else {
            let _ = app.emit(
                "game://exited",
                crate::api::types::GameExited { exit_code: None },
            );
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
