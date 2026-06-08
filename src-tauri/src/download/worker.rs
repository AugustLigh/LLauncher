use futures_util::StreamExt;
use md5::{Digest, Md5};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::api::types::{DownloadProgress, PackFile};
use crate::error::AppError;

pub async fn download_file(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    pack: &PackFile,
    dest: &Path,
    file_index: usize,
    total_files: usize,
    cancel_flag: &Arc<AtomicBool>,
    global_start: std::time::Instant,
    agg_downloaded: &Arc<AtomicU64>,
    agg_total: u64,
    speed_limit: u64,
) -> Result<(), AppError> {
    let file_name = pack
        .url
        .split('/')
        .last()
        .unwrap_or("unknown")
        .to_string();

    if dest.exists() {
        let dest_owned = dest.to_path_buf();
        let expected_md5 = pack.md5.clone();
        let cancel2 = cancel_flag.clone();
        let agg2 = agg_downloaded.clone();
        let app2 = app.clone();
        let fn2 = file_name.clone();

        let verify_result = tokio::task::spawn_blocking(move || {
            let mut last_emit = std::time::Instant::now();
            let mut verified_local: u64 = 0;
            let is_valid = crate::download::verify::verify_md5_with_progress(
                &dest_owned,
                &expected_md5,
                |n| {
                    if !cancel2.load(Ordering::SeqCst) {
                        return Err(AppError::Cancelled);
                    }
                    verified_local += n;
                    agg2.fetch_add(n, Ordering::Relaxed);
                    if last_emit.elapsed().as_millis() >= 150 {
                        let total_dl = agg2.load(Ordering::Relaxed);
                        let elapsed = global_start.elapsed().as_secs_f64();
                        let speed = if elapsed > 0.1 {
                            (total_dl as f64 / elapsed) as u64
                        } else {
                            0
                        };
                        app2.emit(
                            "download://verify-progress",
                            DownloadProgress {
                                file_index,
                                total_files,
                                file_name: fn2.clone(),
                                bytes_downloaded: total_dl,
                                bytes_total: agg_total,
                                speed_bps: speed,
                            },
                        )
                        .ok();
                        last_emit = std::time::Instant::now();
                    }
                    Ok(())
                },
            )?;
            Ok::<(bool, u64), AppError>((is_valid, verified_local))
        })
        .await
        .map_err(|e| AppError::Api(format!("verify task panicked: {}", e)))??;

        let (is_valid, verified_local) = verify_result;

        if is_valid {
            let total_dl = agg_downloaded.load(Ordering::Relaxed);
            let elapsed = global_start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.1 {
                (total_dl as f64 / elapsed) as u64
            } else {
                0
            };
            app.emit(
                "download://verify-progress",
                DownloadProgress {
                    file_index,
                    total_files,
                    file_name: file_name.clone(),
                    bytes_downloaded: total_dl,
                    bytes_total: agg_total,
                    speed_bps: speed,
                },
            )
            .ok();
            app.emit(
                "download://file-complete",
                crate::api::types::DownloadFileComplete {
                    file_index,
                    total_files,
                    file_name,
                },
            )
            .ok();
            return Ok(());
        }

        agg_downloaded.fetch_sub(verified_local, Ordering::Relaxed);
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let response = client.get(&pack.url).send().await?.error_for_status()?;

    let mut stream = response.bytes_stream();
    let file = tokio::fs::File::create(dest).await.map_err(AppError::Io)?;
    let mut writer = BufWriter::with_capacity(4 * 1024 * 1024, file);
    let mut hasher = Md5::new();
    let mut last_emit = std::time::Instant::now();

    let mut rate_bytes: u64 = 0;
    let mut rate_start = Instant::now();

    while let Some(chunk) = stream.next().await {
        if !cancel_flag.load(Ordering::SeqCst) {
            return Err(AppError::Cancelled);
        }

        let chunk = chunk.map_err(AppError::Http)?;
        writer.write_all(&chunk).await.map_err(AppError::Io)?;
        hasher.update(&chunk);

        let chunk_len = chunk.len() as u64;
        agg_downloaded.fetch_add(chunk_len, Ordering::Relaxed);

        if speed_limit > 0 {
            rate_bytes += chunk_len;
            let expected = Duration::from_secs_f64(rate_bytes as f64 / speed_limit as f64);
            let elapsed = rate_start.elapsed();
            if expected > elapsed {
                tokio::time::sleep(expected - elapsed).await;
            }
            if rate_start.elapsed().as_secs() >= 2 {
                rate_bytes = 0;
                rate_start = Instant::now();
            }
        }

        if last_emit.elapsed().as_millis() >= 150 {
            let total_dl = agg_downloaded.load(Ordering::Relaxed);
            let elapsed = global_start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.1 {
                (total_dl as f64 / elapsed) as u64
            } else {
                0
            };

            app.emit(
                "download://progress",
                DownloadProgress {
                    file_index,
                    total_files,
                    file_name: file_name.clone(),
                    bytes_downloaded: total_dl,
                    bytes_total: agg_total,
                    speed_bps: speed,
                },
            )
            .ok();
            last_emit = std::time::Instant::now();
        }
    }

    writer.flush().await.map_err(AppError::Io)?;

    // Verify MD5
    let actual_md5 = format!("{:x}", hasher.finalize());
    if actual_md5 != pack.md5 {
        return Err(AppError::Md5Mismatch {
            expected: pack.md5.clone(),
            actual: actual_md5,
        });
    }

    app.emit(
        "download://file-complete",
        crate::api::types::DownloadFileComplete {
            file_index,
            total_files,
            file_name,
        },
    )
    .ok();

    Ok(())
}
