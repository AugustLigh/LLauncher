use tauri::State;
use crate::config::settings::AppSettings;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    let settings = state.settings.lock().await;
    Ok(settings.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn save_settings(
    state: State<'_, AppState>,
    mut settings: AppSettings,
) -> Result<(), AppError> {
    let mut current = state.settings.lock().await;
    if (state.transfers.busy("game") || state.transfers.busy("proton"))
        && (settings.game_dir != current.game_dir
            || settings.download_dir != current.download_dir
            || settings.proton_dir != current.proton_dir
            || settings.proton_prefix_dir != current.proton_prefix_dir)
    {
        return Err(AppError::Api(
            "Wait for the current transfer before changing folders".into(),
        ));
    }
    // The frontend edits a snapshot of the settings; fields the backend owns
    // (install bookkeeping, play statistics) may have moved on since that
    // snapshot was taken — an install, an import, a play session. Keep our
    // values so a later "Save" in the settings dialog cannot roll them back
    // and, say, turn an installed game back into "Install".
    settings.installed_version = current.installed_version.clone();
    settings.total_playtime_secs = current.total_playtime_secs;
    settings.last_played = current.last_played;
    settings.autostart_initialized = current.autostart_initialized;
    settings.save_async().await?;
    state.power.set_enabled(
        settings.inhibit_sleep_on_download,
        state.transfers.any_active(),
    );
    *current = settings;
    Ok(())
}

/// Switch the display off while a long download runs (Steam Deck on
/// battery, mostly). Returns as soon as the request is queued.
#[tauri::command]
#[specta::specta]
pub fn turn_off_screen() -> Result<(), AppError> {
    crate::power::turn_off_screen()
}

#[tauri::command]
#[specta::specta]
pub async fn get_launcher_content(
    state: State<'_, AppState>,
) -> Result<crate::api::types::LauncherContent, AppError> {
    let settings = state.settings.lock().await;
    let lang = settings.language.clone();
    drop(settings);
    let mut content = crate::api::client::get_launcher_content(&state.http_client, &lang).await?;
    // Hosts without the GStreamer plugins WebKit needs can't survive even
    // attempting the video backdrop (issue #31) — hand the UI a content
    // payload with no video so it renders the static image instead.
    if !content.background.video_url.is_empty() && !crate::media::can_play_video_background() {
        content.background.video_url = String::new();
    }
    Ok(content)
}


