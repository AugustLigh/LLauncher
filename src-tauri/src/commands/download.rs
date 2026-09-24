use tauri::{Emitter, State};
use crate::error::AppError;
use crate::state::AppState;

async fn run_start_download(
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
    settings.save_async().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_download(state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Discard a paused/cancelled download: stop it and delete the partial pack
/// files so they do not linger on disk. Used by the "Cancel" control (as
/// opposed to "Pause", which keeps the partial files for a later resume). Only
/// touches a launcher-managed `_download` cache, never a user-pointed folder.
#[tauri::command]
#[specta::specta]
pub async fn clear_download_cache(state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);

    let settings = state.settings.lock().await;
    let download_dir = settings.download_dir.clone();
    drop(settings);

    let path = std::path::PathBuf::from(&download_dir);
    if path.file_name().is_some_and(|n| n == "_download") {
        tokio::task::spawn_blocking(move || {
            std::fs::remove_dir_all(&path).ok();
        })
        .await
        .ok();
    }
    Ok(())
}

/// Verify the installed game's VFS assets against the official per-file resource
/// manifest and re-download only the files that are missing or corrupt.
///
/// Unlike `repair_game` (which re-fetches the full multi-GB pack set), this
/// hashes what is already on disk and pulls just the deltas — the same
/// mechanism the official launcher uses for updates. Reuses `download_active`
/// for cancellation, so `cancel_download` stops it too.
async fn run_verify_game_integrity(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::download::resources::IntegrityComplete, AppError> {
    let settings = state.settings.lock().await;
    let game_dir = settings.game_dir.clone();
    let installed_version = settings.installed_version.clone();
    let max_concurrent = settings.download_max_concurrent;
    drop(settings);

    if installed_version.is_empty()
        || !std::path::Path::new(&game_dir)
            .join("Endfield.exe")
            .exists()
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
        "integrity".to_string(),
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

/// Update an installed game to the latest version, picking the cheaper safe
/// path. The resource manifest only covers VFS assets; the engine/executable
/// files live solely in the packs. So we first check (via the latest packs'
/// ZIP central directory) whether any non-VFS file changed:
///   - engine unchanged → per-file VFS delta (download only changed assets);
///   - engine changed, or the check is inconclusive -> full pack download.
///
/// Either way the resulting install is complete. Progress for the delta path is
/// emitted on the `update://` channel; the pack path uses `download://`.
async fn run_start_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let settings = state.settings.lock().await;
    let game_dir = settings.game_dir.clone();
    let download_dir = settings.download_dir.clone();
    let installed_version = settings.installed_version.clone();
    let speed_limit = settings.download_speed_limit;
    let max_concurrent = settings.download_max_concurrent.clamp(1, 8);
    drop(settings);

    let client = state.http_client.clone();
    let download_active = state.download_active.clone();

    // Decide engine vs assets: is every non-VFS file already current?
    crate::download::packindex::emit_checking(&app);
    let version_info = crate::api::client::get_latest_game_version(&client, "").await?;
    if state.transfers.cancelled("game") {
        return Err(AppError::Cancelled);
    }
    let cd =
        crate::download::packindex::fetch_central_directory(&client, &version_info.pkg.packs).await;
    let engine_current = match cd {
        Ok(entries) => {
            // CRC-checking the non-VFS files reads ~1.4 GB; keep it off the async runtime.
            let game_dir_for_check = game_dir.clone();
            tokio::task::spawn_blocking(move || {
                crate::download::packindex::engine_is_current(&entries, &game_dir_for_check)
            })
            .await
            .unwrap_or(false)
        }
        Err(_) => false, // inconclusive → safe full update
    };

    if state.transfers.cancelled("game") {
        return Err(AppError::Cancelled);
    }
    let result: Result<String, AppError> = if engine_current {
        crate::download::resources::verify_and_repair(
            app.clone(),
            client,
            download_active.clone(),
            game_dir,
            max_concurrent,
            "update".to_string(),
        )
        .await
        .map(|_| version_info.version.clone())
    } else {
        crate::download::manager::start_download(
            app.clone(),
            client,
            download_active.clone(),
            &download_dir,
            &game_dir,
            &installed_version,
            speed_limit,
            max_concurrent,
        )
        .await
    };

    download_active.store(false, std::sync::atomic::Ordering::SeqCst);

    match result {
        Ok(version) => {
            // The delta path emits update://complete; the pack path already
            // emitted download://complete. Emit a uniform completion so the UI
            // updates regardless of which path ran.
            if engine_current {
                app.emit(
                    "update://complete",
                    crate::api::types::DownloadComplete {
                        version: version.clone(),
                    },
                )
                .ok();
            }
            let mut settings = state.settings.lock().await;
            settings.installed_version = version;
            settings.save_async().await?;
            Ok(())
        }
        Err(e) => {
            // Surface on both channels so whichever path was active is covered;
            // cancellation is filtered on the frontend.
            let msg = crate::api::types::DownloadError {
                message: e.to_string(),
            };
            app.emit("update://error", msg.clone()).ok();
            Err(e)
        }
    }
}

async fn run_repair_game(
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
    settings.save_async().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn start_download(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let settings = state.settings.lock().await;
    let id = state.transfers.begin(
        &app,
        "game",
        "install",
        &settings.game_dir,
        &settings.download_dir,
        &state.download_active,
    )?;
    drop(settings);
    let result = run_start_download(app.clone(), state.clone()).await;
    state
        .download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    state.transfers.finish(
        &app,
        "game",
        &id,
        result
            .as_ref()
            .map(|r| serde_json::to_value(r).unwrap_or_default()),
    );
    result
}

#[tauri::command]
#[specta::specta]
pub async fn start_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let settings = state.settings.lock().await;
    let id = state.transfers.begin(
        &app,
        "game",
        "update",
        &settings.game_dir,
        &settings.download_dir,
        &state.download_active,
    )?;
    drop(settings);
    let result = run_start_update(app.clone(), state.clone()).await;
    state
        .download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    state.transfers.finish(
        &app,
        "game",
        &id,
        result
            .as_ref()
            .map(|r| serde_json::to_value(r).unwrap_or_default()),
    );
    result
}

#[tauri::command]
#[specta::specta]
pub async fn verify_game_integrity(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::download::resources::IntegrityComplete, AppError> {
    let settings = state.settings.lock().await;
    let id = state.transfers.begin(
        &app,
        "game",
        "integrity",
        &settings.game_dir,
        &settings.download_dir,
        &state.download_active,
    )?;
    drop(settings);
    let result = run_verify_game_integrity(app.clone(), state.clone()).await;
    state
        .download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    state.transfers.finish(
        &app,
        "game",
        &id,
        result
            .as_ref()
            .map(|r| serde_json::to_value(r).unwrap_or_default()),
    );
    result
}

#[tauri::command]
#[specta::specta]
pub async fn repair_game(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let settings = state.settings.lock().await;
    let id = state.transfers.begin(
        &app,
        "game",
        "repair",
        &settings.game_dir,
        &settings.download_dir,
        &state.download_active,
    )?;
    drop(settings);
    let result = run_repair_game(app.clone(), state.clone()).await;
    state
        .download_active
        .store(false, std::sync::atomic::Ordering::SeqCst);
    state.transfers.finish(
        &app,
        "game",
        &id,
        result
            .as_ref()
            .map(|r| serde_json::to_value(r).unwrap_or_default()),
    );
    result
}


