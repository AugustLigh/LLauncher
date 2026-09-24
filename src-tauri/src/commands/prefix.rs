use tauri::State;
use crate::error::AppError;
use crate::state::AppState;

/// Resolve the game's Proton prefix directory as the launch path would.
async fn prefix_dir(state: &State<'_, AppState>) -> std::path::PathBuf {
    let settings = state.settings.lock().await;
    let dir = crate::game::launcher::resolve_prefix_dir(
        &settings,
        std::path::Path::new(&settings.game_dir),
    );
    dir
}

#[derive(serde::Serialize, specta::Type)]
pub struct PrefixInfo {
    pub path: String,
    pub exists: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn get_prefix_info(state: State<'_, AppState>) -> Result<PrefixInfo, AppError> {
    let dir = prefix_dir(&state).await;
    Ok(PrefixInfo {
        exists: dir.join("pfx").exists(),
        path: dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn open_prefix_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt;
    let dir = prefix_dir(&state).await;
    if !dir.exists() {
        return Err(AppError::Api(
            "No Proton prefix exists yet — launch the game once first".to_string(),
        ));
    }
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Api(format!("Failed to open folder: {}", e)))
}

/// Run a whitelisted Wine tool inside the game prefix (winecfg / regedit).
#[tauri::command]
#[specta::specta]
pub async fn run_prefix_tool(state: State<'_, AppState>, tool: String) -> Result<(), AppError> {
    if !matches!(tool.as_str(), "winecfg" | "regedit") {
        return Err(AppError::Api(format!("Unknown prefix tool: {}", tool)));
    }
    let settings = state.settings.lock().await.clone();
    crate::game::launcher::run_prefix_tool(&settings, &tool)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_shader_cache(
    state: State<'_, AppState>,
) -> Result<crate::game::prefix::ShaderCacheResult, AppError> {
    let settings = state.settings.lock().await;
    let game_dir = std::path::PathBuf::from(&settings.game_dir);
    let compat_data = crate::game::launcher::resolve_prefix_dir(&settings, &game_dir);
    drop(settings);

    tokio::task::spawn_blocking(move || {
        crate::game::prefix::clear_shader_cache(&game_dir, &compat_data)
    })
    .await
    .map_err(|e| AppError::Api(format!("Shader cache task failed: {}", e)))
}

#[tauri::command]
#[specta::specta]
pub async fn backup_prefix(state: State<'_, AppState>, dest: String) -> Result<(), AppError> {
    let dir = prefix_dir(&state).await;
    tokio::task::spawn_blocking(move || {
        crate::game::prefix::backup(&dir, std::path::Path::new(&dest))
    })
    .await
    .map_err(|e| AppError::Api(format!("Backup task failed: {}", e)))?
}

#[tauri::command]
#[specta::specta]
pub async fn restore_prefix(state: State<'_, AppState>, archive: String) -> Result<(), AppError> {
    if state.game_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(AppError::Api(
            "Cannot restore the prefix while the game is running".to_string(),
        ));
    }
    let dir = prefix_dir(&state).await;
    tokio::task::spawn_blocking(move || {
        crate::game::prefix::restore(&dir, std::path::Path::new(&archive))
    })
    .await
    .map_err(|e| AppError::Api(format!("Restore task failed: {}", e)))?
}

#[tauri::command]
#[specta::specta]
pub async fn reset_prefix(state: State<'_, AppState>) -> Result<(), AppError> {
    if state.game_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(AppError::Api(
            "Cannot reset the prefix while the game is running".to_string(),
        ));
    }
    let dir = prefix_dir(&state).await;
    tokio::task::spawn_blocking(move || crate::game::prefix::reset(&dir))
        .await
        .map_err(|e| AppError::Api(format!("Reset task failed: {}", e)))?
}


