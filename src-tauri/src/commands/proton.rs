use tauri::{Emitter, State};
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn check_system_requirements(
    state: State<'_, AppState>,
) -> Result<crate::game::proton::SystemCheck, AppError> {
    let settings = state.settings.lock().await.clone();
    // Shells out to `which` a few times on Linux and stats a list of install
    // locations on macOS — keep both off the async runtime.
    tokio::task::spawn_blocking(move || crate::game::proton::check_system(&settings))
        .await
        .map_err(|e| AppError::Api(format!("system check task failed: {}", e)))
}

// ---------------------------------------------------------------------------
// The compatibility layer the launcher installs itself. The commands keep
// their DWProton names — the frontend's picker, prompt and progress UI are the
// same on both Unix platforms — but on macOS they deal in Wine Staging + DXMT
// (see `download::wine`) and in `macos_wine_dir` rather than `proton_dir`.
// Runtime `cfg!` rather than `#[cfg]` so both arms are type-checked by every
// CI job, not only the macOS one.
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn get_dwproton_latest(
    state: State<'_, AppState>,
) -> Result<crate::api::types::ProtonReleaseInfo, AppError> {
    if cfg!(target_os = "macos") {
        return crate::download::wine::get_latest(&state.http_client).await;
    }
    crate::download::proton::get_latest_dwproton_info(&state.http_client).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_dwproton_releases(
    state: State<'_, AppState>,
) -> Result<Vec<crate::api::types::ProtonReleaseInfo>, AppError> {
    if cfg!(target_os = "macos") {
        return crate::download::wine::list_releases(&state.http_client).await;
    }
    crate::download::proton::list_dwproton_releases(&state.http_client).await
}

/// The DWProton tag we install by default and flag as recommended in the
/// picker; on macOS the Wine Staging version the Endfield module set was
/// last built and smoke-tested against.
#[tauri::command]
#[specta::specta]
pub fn recommended_proton_tag() -> &'static str {
    if cfg!(target_os = "macos") {
        return crate::download::wine::RECOMMENDED_WINE_TAG;
    }
    crate::download::proton::RECOMMENDED_DWPROTON_TAG
}

#[tauri::command]
#[specta::specta]
pub async fn list_installed_protons() -> Result<Vec<crate::api::types::InstalledProton>, AppError> {
    if cfg!(target_os = "macos") {
        let base = crate::config::paths::default_wine_dir();
        return tokio::task::spawn_blocking(move || crate::download::wine::list_installed(&base))
            .await
            .map_err(|e| AppError::Api(format!("wine list task failed: {}", e)));
    }

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
                    dxmt: None,
                    wine_patch: None,
                });
            }
        }
    }

    installed.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(installed)
}

#[tauri::command]
#[specta::specta]
pub async fn set_active_proton(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    let mut settings = state.settings.lock().await;
    if cfg!(target_os = "macos") {
        settings.macos_wine_dir = path;
    } else {
        settings.proton_dir = path;
    }
    settings.save_async().await?;
    Ok(())
}

async fn run_download_dwproton(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    release: Option<crate::api::types::ProtonReleaseInfo>,
) -> Result<(), AppError> {
    let client = state.http_client.clone();
    let cancel_flag = state.proton_download_active.clone();

    let result = if cfg!(target_os = "macos") {
        let base_dir = crate::config::paths::default_wine_dir();
        crate::download::wine::download_and_install(&app, &client, &cancel_flag, &base_dir, release)
            .await
    } else {
        // Always download to the base proton directory
        let base_dir = crate::config::paths::default_proton_dir()
            .to_string_lossy()
            .to_string();
        crate::download::proton::download_and_extract_dwproton(&app, &client, &cancel_flag, &base_dir, release)
            .await
    };

    cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);

    match result {
        Ok((layer_dir, _version)) => {
            let mut settings = state.settings.lock().await;
            if cfg!(target_os = "macos") {
                settings.macos_wine_dir = layer_dir;
            } else {
                settings.proton_dir = layer_dir;
            }
            settings.save_async().await?;
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
#[specta::specta]
pub async fn cancel_proton_download(state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .proton_download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}


#[tauri::command]
#[specta::specta]
pub async fn download_dwproton(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    release: Option<crate::api::types::ProtonReleaseInfo>,
) -> Result<(), AppError> {
    let settings = state.settings.lock().await;
    let id = state.transfers.begin(
        &app,
        "proton",
        "proton",
        &settings.game_dir,
        &settings.download_dir,
        &state.proton_download_active,
    )?;
    drop(settings);
    let result = run_download_dwproton(app.clone(), state.clone(), release).await;
    state
        .proton_download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    state.transfers.finish(
        &app,
        "proton",
        &id,
        result
            .as_ref()
            .map(|r| serde_json::to_value(r).unwrap_or_default()),
    );
    result
}


