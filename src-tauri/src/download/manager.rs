use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::Emitter;

use crate::api::types::{DownloadComplete, DownloadError};
use crate::error::AppError;

pub async fn start_download(
    app: tauri::AppHandle,
    client: reqwest::Client,
    download_active: Arc<AtomicBool>,
    download_dir: &str,
    game_dir: &str,
    current_version: &str,
    speed_limit: u64,
    max_concurrent: u32,
) -> Result<(), AppError> {
    download_active.store(true, Ordering::SeqCst);

    let download_path = Path::new(download_dir);
    let game_path = Path::new(game_dir);

    let mut version_info =
        crate::api::client::get_latest_game_version(&client, current_version).await?;

    // If API returned no packs (e.g. version already matches latest but game files are missing),
    // retry with empty version to request a full download
    if version_info.pkg.packs.is_empty() && !current_version.is_empty() {
        version_info =
            crate::api::client::get_latest_game_version(&client, "").await?;
    }

    if version_info.pkg.packs.is_empty() {
        download_active.store(false, Ordering::SeqCst);
        let msg = "No download packages available from server".to_string();
        app.emit("download://error", DownloadError { message: msg.clone() }).ok();
        return Err(AppError::Api(msg));
    }

    let version = version_info.version.clone();
    let packs = &version_info.pkg.packs;
    let total_files = packs.len();

    // Calculate total size for aggregate progress
    let total_size: u64 = packs
        .iter()
        .map(|p| p.package_size.parse::<u64>().unwrap_or(0))
        .sum();

    let agg_downloaded = Arc::new(AtomicU64::new(0));
    let global_start = std::time::Instant::now();

    let per_worker_limit = if speed_limit > 0 {
        (speed_limit / max_concurrent as u64).max(1024)
    } else {
        0
    };

    // Download packs concurrently
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent as usize));
    let mut handles = Vec::new();

    for (i, pack) in packs.iter().enumerate() {
        if !download_active.load(Ordering::SeqCst) {
            break;
        }

        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AppError::Cancelled)?;

        let app2 = app.clone();
        let client2 = client.clone();
        let pack2 = pack.clone();
        let file_name = pack.url.split('/').last().unwrap_or("unknown").to_string();
        let dest = download_path.join(&file_name);
        let active2 = download_active.clone();
        let agg2 = agg_downloaded.clone();
        let start = global_start;
        let ts = total_size;

        let handle = tokio::spawn(async move {
            let _permit = permit;
            crate::download::worker::download_file(
                &app2, &client2, &pack2, &dest, i, total_files, &active2, start, &agg2, ts,
                per_worker_limit,
            )
            .await
        });

        handles.push(handle);
    }

    // Wait for all downloads
    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                download_active.store(false, Ordering::SeqCst);
                app.emit(
                    "download://error",
                    DownloadError {
                        message: e.to_string(),
                    },
                )
                .ok();
                return Err(e);
            }
            Err(e) => {
                download_active.store(false, Ordering::SeqCst);
                let msg = format!("Download task failed: {}", e);
                app.emit("download://error", DownloadError { message: msg.clone() })
                    .ok();
                return Err(AppError::Api(msg));
            }
        }
    }

    let parts: Vec<PathBuf> = packs
        .iter()
        .map(|p| download_path.join(p.url.split('/').last().unwrap_or("unknown")))
        .collect();

    let marker = crate::game::state::incomplete_marker(game_path);
    std::fs::write(&marker, b"extracting").ok();

    let extract_app = app.clone();
    let game_path_owned = game_path.to_path_buf();
    let extract_result = tokio::task::spawn_blocking(move || {
        crate::download::extract::extract_split_zip(
            &extract_app,
            &parts,
            &game_path_owned,
            total_size,
        )
    })
    .await;

    let extract_outcome = match extract_result {
        Ok(inner) => inner,
        Err(e) => Err(AppError::Api(format!("Extraction task failed: {}", e))),
    };

    if let Err(e) = extract_outcome {
        download_active.store(false, Ordering::SeqCst);
        app.emit(
            "download://error",
            DownloadError {
                message: e.to_string(),
            },
        )
        .ok();
        return Err(e);
    }

    std::fs::remove_file(&marker).ok();

    download_active.store(false, Ordering::SeqCst);

    app.emit("download://complete", DownloadComplete { version }).ok();

    Ok(())
}
