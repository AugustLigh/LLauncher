use tauri::State;
use crate::error::AppError;
use crate::state::AppState;

/// What the launcher can see of the mod setup: is the loader in place, how
/// many mods are installed, where to drop new ones.
#[tauri::command]
#[specta::specta]
pub async fn get_mods_status(
    state: State<'_, AppState>,
) -> Result<crate::game::mods::ModsStatus, AppError> {
    let game_dir = state.settings.lock().await.game_dir.clone();
    Ok(crate::game::mods::status(std::path::Path::new(&game_dir)))
}

/// Download and install EFMI (and the 3DMigoto build it runs on) into the
/// game directory.
#[tauri::command]
#[specta::specta]
pub async fn install_mod_loader(
    state: State<'_, AppState>,
) -> Result<crate::game::mods::LoaderInstallResult, AppError> {
    let (game_dir, client) = {
        let settings = state.settings.lock().await;
        (settings.game_dir.clone(), state.http_client.clone())
    };
    crate::game::mods::install_loader(&client, std::path::Path::new(&game_dir)).await
}

/// Remove the loader again. Installed mods are left alone.
#[tauri::command]
#[specta::specta]
pub async fn uninstall_mod_loader(state: State<'_, AppState>) -> Result<(), AppError> {
    let game_dir = state.settings.lock().await.game_dir.clone();
    let dir = std::path::PathBuf::from(game_dir);
    tokio::task::spawn_blocking(move || crate::game::mods::uninstall_loader(&dir))
        .await
        .map_err(|e| AppError::Api(format!("mod loader uninstall task failed: {}", e)))?
}

/// Open the `Mods` directory in the file manager, creating it on the way if
/// this is the user's first mod.
#[tauri::command]
#[specta::specta]
pub async fn open_mods_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt;
    let game_dir = state.settings.lock().await.game_dir.clone();
    let dir = crate::game::mods::ensure_mods_dir(std::path::Path::new(&game_dir))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Api(format!("Failed to open folder: {}", e)))
}

/// OptiScaler in the game directory: installed or not, which release.
#[tauri::command]
#[specta::specta]
pub async fn get_optiscaler_status(
    state: State<'_, AppState>,
) -> Result<crate::game::optiscaler::OptiScalerStatus, AppError> {
    let game_dir = state.settings.lock().await.game_dir.clone();
    Ok(crate::game::optiscaler::status(std::path::Path::new(&game_dir)))
}

/// Download the latest OptiScaler release into the game directory, tuned
/// for this machine (Wine hooks, GPU spoofing without an NVIDIA driver).
#[tauri::command]
#[specta::specta]
pub async fn install_optiscaler(
    state: State<'_, AppState>,
) -> Result<crate::game::optiscaler::InstallResult, AppError> {
    let (game_dir, client) = {
        let settings = state.settings.lock().await;
        (settings.game_dir.clone(), state.http_client.clone())
    };
    crate::game::optiscaler::install(
        &client,
        std::path::Path::new(&game_dir),
        crate::game::optiscaler::host_options(),
    )
    .await
}

/// Remove OptiScaler again, leaving everything else in the game directory.
#[tauri::command]
#[specta::specta]
pub async fn uninstall_optiscaler(state: State<'_, AppState>) -> Result<(), AppError> {
    let game_dir = state.settings.lock().await.game_dir.clone();
    let dir = std::path::PathBuf::from(game_dir);
    tokio::task::spawn_blocking(move || crate::game::optiscaler::uninstall(&dir))
        .await
        .map_err(|e| AppError::Api(format!("OptiScaler uninstall task failed: {}", e)))?
}


