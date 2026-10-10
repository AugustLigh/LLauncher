use tauri::State;

use crate::api::constants::DOWNLOAD_STALL_TIMEOUT;
use crate::config::paths;
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

/// The backdrop video from the disk cache, as raw bytes. WebKitGTK streams
/// remote media poorly and won't play it from the asset protocol. A cache
/// miss downloads first; a hit refreshes in the background for next launch.
#[tauri::command]
pub async fn get_background_video(
    state: State<'_, AppState>,
    url: String,
) -> Result<tauri::ipc::Response, AppError> {
    use std::sync::atomic::Ordering;
    // One download at a time; a remount waits for the first one's result.
    static REFRESH: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    if !url.starts_with("https://") {
        return Err(AppError::Api("Background video must be https".to_string()));
    }
    let dir = paths::cache_dir()
        .ok_or_else(|| AppError::Api("No cache directory for the background video".to_string()))?;
    // Before the cache: a refresh may have saved it again after the forget.
    if crate::media::is_unplayable(&dir, &url) {
        crate::media::forget_cached_video(&dir, &url);
        return Err(AppError::Api("Background video can't be played here".to_string()));
    }
    let read = || {
        let (dir, url) = (dir.clone(), url.clone());
        async move {
            tokio::task::spawn_blocking(move || crate::media::read_cached_video(&dir, &url))
                .await
                .map_err(|e| AppError::Api(e.to_string()))
        }
    };
    let respond = |cached: Option<(Vec<u8>, String)>| {
        cached
            .map(|(bytes, _)| tauri::ipc::Response::new(bytes))
            .ok_or_else(|| AppError::Api("Background video is not cached".to_string()))
    };
    // Only a refresh yields to game and Proton downloads.
    let flags = [state.download_active.clone(), state.proton_download_active.clone()];
    let busy = move || flags.iter().any(|f| f.load(Ordering::SeqCst));
    let client = state.http_client.clone();

    let cached = read().await?;
    let Some((_, etag)) = &cached else {
        let _guard = REFRESH.lock().await;
        if let Some(cached) = read().await? {
            return respond(Some(cached));
        }
        cache_background_video(&client, &dir, &url, None, || false).await?;
        return respond(read().await?);
    };
    if !busy() {
        if let Ok(guard) = REFRESH.try_lock() {
            let etag = Some(etag.clone());
            tokio::spawn(async move {
                let _guard = guard;
                if let Err(e) = cache_background_video(&client, &dir, &url, etag, busy).await {
                    crate::logging::warn(format!("background video refresh: {e}"));
                }
            });
        }
    }
    respond(cached)
}

/// Drop a cached background video the webview couldn't play.
#[tauri::command]
pub async fn forget_background_video(url: String) {
    if let Some(dir) = paths::cache_dir() {
        crate::media::forget_cached_video(&dir, &url);
    }
}

/// Download `url` into the cache unless the server still has `etag`.
async fn cache_background_video(
    client: &reqwest::Client,
    dir: &std::path::Path,
    url: &str,
    etag: Option<String>,
    busy: impl Fn() -> bool,
) -> Result<(), AppError> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    // Every launch holds the whole file in memory.
    const MAX_SIZE: u64 = 100 * 1024 * 1024;

    let mut req = client.get(url);
    if let Some(etag) = &etag {
        req = req.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    let resp = crate::util::send_with_stall_timeout(req, DOWNLOAD_STALL_TIMEOUT)
        .await?
        .error_for_status()?;
    let new_etag = resp
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    // The server may ignore If-None-Match and still send the same ETag.
    if resp.status() == reqwest::StatusCode::NOT_MODIFIED || (etag.is_some() && new_etag == etag) {
        return Ok(());
    }
    // Needs an ETag to revalidate and a length to verify; any other type is
    // an error page (a captive portal's, say).
    let is_video = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|t| t.split(';').next().unwrap_or_default().trim().to_ascii_lowercase())
        .is_some_and(|t| {
            matches!(t.as_str(), "video/mp4" | "application/mp4") || t.ends_with("/octet-stream")
        });
    let size = resp.content_length().filter(|len| (1..=MAX_SIZE).contains(len));
    let (Some(new_etag), Some(size), true) = (new_etag, size, is_video) else {
        return Err(AppError::Api(
            "Background video response can't be cached (no ETag, not a video, or bad length)"
                .to_string(),
        ));
    };

    tokio::fs::create_dir_all(dir).await?;
    let part = crate::media::part_path(dir);
    let download = async {
        let mut file =
            tokio::io::BufWriter::with_capacity(1024 * 1024, tokio::fs::File::create(&part).await?);
        let mut stream = resp.bytes_stream();
        let mut written = 0u64;
        while let Some(chunk) = tokio::time::timeout(DOWNLOAD_STALL_TIMEOUT, stream.next())
            .await
            .map_err(|_| AppError::Api("Background video download stalled".to_string()))?
        {
            if busy() {
                return Err(AppError::Api("Background video refresh yielded to a download".to_string()));
            }
            let chunk = chunk?;
            written += chunk.len() as u64;
            if written > size {
                break;
            }
            file.write_all(&chunk).await?;
        }
        if written != size {
            return Err(AppError::Api(format!(
                "Background video is {written} bytes, expected {size}"
            )));
        }
        file.flush().await?;
        let (dir, url) = (dir.to_path_buf(), url.to_string());
        tokio::task::spawn_blocking(move || {
            crate::media::commit_cached_video(&dir, &new_etag, size, &url)
        })
        .await
        .map_err(|e| AppError::Api(e.to_string()))??;
        Ok::<_, AppError>(())
    };
    let result = download.await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&part).await;
    }
    result
}
