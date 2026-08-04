use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::Emitter;

use crate::api::types::{DownloadComplete, DownloadError};
use crate::error::AppError;

/// Extract a safe file name from a pack URL's last path segment. Empty
/// segments and `.`/`..` are rejected so `download_path.join(name)` can never
/// resolve outside `download_path` — a tampered pack list could otherwise
/// smuggle a `..` segment to escape the download directory.
fn safe_pack_file_name(url: &str) -> &str {
    match url.rsplit('/').next() {
        Some(name) if !name.is_empty() && name != "." && name != ".." => name,
        _ => "unknown",
    }
}

pub async fn start_download(
    app: tauri::AppHandle,
    client: reqwest::Client,
    download_active: Arc<AtomicBool>,
    download_dir: &str,
    game_dir: &str,
    current_version: &str,
    speed_limit: u64,
    max_concurrent: u32,
) -> Result<String, AppError> {
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

    // Refuse early when the download clearly cannot fit, instead of failing
    // 40 GB in. Already-downloaded pack files do not need space again.
    let existing_parts: u64 = packs
        .iter()
        .filter_map(|p| {
            let name = safe_pack_file_name(&p.url);
            std::fs::metadata(download_path.join(name)).ok().map(|m| m.len())
        })
        .sum();
    let needed = total_size.saturating_sub(existing_parts);
    std::fs::create_dir_all(download_path)?;
    if let Some(available) = crate::config::paths::available_space(download_path) {
        if available < needed {
            download_active.store(false, Ordering::SeqCst);
            let err = AppError::DiskSpace {
                path: download_dir.to_string(),
                needed_mib: needed / (1024 * 1024),
                available_mib: available / (1024 * 1024),
            };
            app.emit("download://error", DownloadError { message: err.to_string() }).ok();
            return Err(err);
        }
    }

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
        let file_name = safe_pack_file_name(&pack.url).to_string();
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

    // Cancellation may have broken out of the spawn loop after the in-flight
    // workers finished cleanly; never proceed to extraction in that case.
    if !download_active.load(Ordering::SeqCst) {
        app.emit(
            "download://error",
            DownloadError {
                message: AppError::Cancelled.to_string(),
            },
        )
        .ok();
        return Err(AppError::Cancelled);
    }

    let parts: Vec<PathBuf> = packs
        .iter()
        .map(|p| download_path.join(safe_pack_file_name(&p.url)))
        .collect();

    let marker = crate::game::state::incomplete_marker(game_path);
    std::fs::write(&marker, b"extracting").ok();

    let extract_app = app.clone();
    let game_path_owned = game_path.to_path_buf();
    let extract_parts = parts.clone();
    let extract_result = tokio::task::spawn_blocking(move || {
        crate::download::extract::extract_split_zip(
            &extract_app,
            &extract_parts,
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

    // The pack archives are no longer needed once extracted; leaving them
    // around wastes tens of GB and forces a pointless re-verify on the next
    // repair/update pass.
    for part in &parts {
        std::fs::remove_file(part).ok();
    }

    download_active.store(false, Ordering::SeqCst);

    app.emit("download://complete", DownloadComplete { version: version.clone() }).ok();

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_segments_in_pack_urls() {
        // A pack list entry whose URL ends in `/..` must not resolve
        // `download_path.join(name)` to the parent of the download directory.
        assert_eq!(
            safe_pack_file_name("https://cdn.example.com/packs/.."),
            "unknown"
        );
        assert_eq!(safe_pack_file_name("https://cdn.example.com/packs/."), "unknown");
        assert_eq!(safe_pack_file_name("https://cdn.example.com/packs/"), "unknown");
        assert_eq!(
            safe_pack_file_name("https://cdn.example.com/packs/pack01.zip"),
            "pack01.zip"
        );
    }
}
