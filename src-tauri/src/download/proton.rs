use futures_util::StreamExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

use crate::api::types::{ProtonDownloadComplete, ProtonDownloadProgress, ProtonReleaseInfo};
use crate::error::AppError;

const DWPROTON_RELEASES_URL: &str =
    "https://dawn.wine/api/v1/repos/dawn-winery/dwproton/releases";

/// The DWProton release we install by default and flag as recommended.
///
/// We deliberately do **not** track upstream `/latest`. The 11.x series (built
/// on wine-11) regressed badly for Endfield: a hard abort on the unimplemented
/// `ntoskrnl.exe.PsGetProcessExitStatus` stub plus a rendering collapse to
/// single-digit FPS with missing textures (GitHub issue #20). 10.0-26 is the
/// last stable 10.x build and is what a first-run user should get.
pub const RECOMMENDED_DWPROTON_TAG: &str = "dwproton-10.0-26";

fn parse_release(release: &serde_json::Value) -> Option<ProtonReleaseInfo> {
    let tag_name = release["tag_name"].as_str()?.to_string();
    let assets = release["assets"].as_array()?;

    let asset = assets.iter().find(|a| {
        a["name"]
            .as_str()
            .map(|n| n.contains("x86_64") && n.ends_with(".tar.xz"))
            .unwrap_or(false)
    })?;

    let download_url = asset["browser_download_url"].as_str()?.to_string();
    let file_name = asset["name"].as_str().unwrap_or("dwproton.tar.xz").to_string();
    let size = asset["size"].as_u64().unwrap_or(0);

    // Parse date: "2025-01-15T12:00:00Z" -> "2025-01-15"
    let published_at = release["published_at"]
        .as_str()
        .or_else(|| release["created_at"].as_str())
        .unwrap_or("")
        .split('T')
        .next()
        .unwrap_or("")
        .to_string();

    Some(ProtonReleaseInfo {
        tag_name,
        download_url,
        file_name,
        size,
        published_at,
    })
}

pub async fn get_latest_dwproton_info(
    client: &reqwest::Client,
) -> Result<ProtonReleaseInfo, AppError> {
    let resp: serde_json::Value = client
        .get(&format!("{}/latest", DWPROTON_RELEASES_URL))
        .send()
        .await?
        .json()
        .await?;

    parse_release(&resp)
        .ok_or_else(|| AppError::ProtonDownloadFailed("No x86_64.tar.xz asset found".into()))
}

/// Resolve the release info for the recommended (pinned) DWProton build.
///
/// Falls back to the newest available release if the pinned tag ever vanishes
/// upstream, so a removed tag can never break first-run setup.
pub async fn get_recommended_dwproton_info(
    client: &reqwest::Client,
) -> Result<ProtonReleaseInfo, AppError> {
    let releases = list_dwproton_releases(client).await?;
    if let Some(found) = releases
        .iter()
        .find(|r| r.tag_name == RECOMMENDED_DWPROTON_TAG)
    {
        return Ok(found.clone());
    }
    // Pinned tag not in the window — fall back to the newest build rather than
    // failing setup outright (list is returned newest-first).
    releases
        .into_iter()
        .next()
        .ok_or_else(|| AppError::ProtonDownloadFailed("No DWProton releases found".into()))
}

pub async fn list_dwproton_releases(
    client: &reqwest::Client,
) -> Result<Vec<ProtonReleaseInfo>, AppError> {
    let resp: Vec<serde_json::Value> = client
        .get(DWPROTON_RELEASES_URL)
        .query(&[("limit", "20")])
        .send()
        .await?
        .json()
        .await?;

    let releases: Vec<ProtonReleaseInfo> = resp.iter().filter_map(parse_release).collect();
    Ok(releases)
}

pub async fn download_and_extract_dwproton(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    cancel_flag: &Arc<AtomicBool>,
    dest_dir: &str,
    release: Option<ProtonReleaseInfo>,
) -> Result<(String, String), AppError> {
    // No explicit release → install the pinned recommended build, not `/latest`
    // (see RECOMMENDED_DWPROTON_TAG and issue #20).
    let info = match release {
        Some(r) => r,
        None => get_recommended_dwproton_info(client).await?,
    };

    let dest_path = Path::new(dest_dir);
    std::fs::create_dir_all(dest_path)?;

    let archive_path = dest_path.join(&info.file_name);

    // Download
    emit_progress(app, 0, info.size, 0, "downloading");

    let response = client.get(&info.download_url).send().await?;
    let total_size = response.content_length().unwrap_or(info.size);

    let mut stream = response.bytes_stream();
    let file = tokio::fs::File::create(&archive_path)
        .await
        .map_err(AppError::Io)?;
    let mut writer = tokio::io::BufWriter::with_capacity(512 * 1024, file);
    let mut downloaded: u64 = 0;
    let mut last_emit = std::time::Instant::now();
    let start_time = std::time::Instant::now();

    use tokio::io::AsyncWriteExt;

    while let Some(chunk) = stream.next().await {
        if !cancel_flag.load(Ordering::SeqCst) {
            drop(writer);
            let _ = std::fs::remove_file(&archive_path);
            return Err(AppError::Cancelled);
        }

        let chunk = chunk.map_err(AppError::Http)?;
        writer.write_all(&chunk).await.map_err(AppError::Io)?;
        downloaded += chunk.len() as u64;

        if last_emit.elapsed().as_millis() >= 150 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.1 {
                (downloaded as f64 / elapsed) as u64
            } else {
                0
            };
            emit_progress(app, downloaded, total_size, speed, "downloading");
            last_emit = std::time::Instant::now();
        }
    }

    writer.flush().await.map_err(AppError::Io)?;
    drop(writer);

    // Extract
    emit_progress(app, downloaded, total_size, 0, "extracting");

    let mut tar_cmd = std::process::Command::new("tar");
    tar_cmd
        .arg("-xJf")
        .arg(archive_path.to_string_lossy().to_string())
        .arg("-C")
        .arg(dest_path.to_string_lossy().to_string());
    // Without this, the bundled (older) liblzma leaks into the system `xz`
    // that `tar` execs and extraction dies with a version-mismatch (issue #19).
    crate::util::strip_appimage_libs(&mut tar_cmd);
    let output = tar_cmd.output().map_err(|_| AppError::TarNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::ExtractionFailed(stderr.to_string()));
    }

    // Clean up archive
    let _ = std::fs::remove_file(&archive_path);

    // Find the extracted proton directory
    let proton_dir = find_proton_dir(dest_path)?;

    let version = info.tag_name.clone();

    app.emit(
        "proton://complete",
        ProtonDownloadComplete {
            proton_dir: proton_dir.clone(),
            version: version.clone(),
        },
    )
    .ok();

    Ok((proton_dir, version))
}

fn find_proton_dir(base: &Path) -> Result<String, AppError> {
    // Look for a directory containing a `proton` executable
    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("proton").exists() {
                return Ok(path.to_string_lossy().to_string());
            }
        }
    }

    // Maybe proton is directly in base
    if base.join("proton").exists() {
        return Ok(base.to_string_lossy().to_string());
    }

    Err(AppError::ProtonDownloadFailed(
        "Could not find proton executable in extracted files".into(),
    ))
}

fn emit_progress(
    app: &tauri::AppHandle,
    bytes_downloaded: u64,
    bytes_total: u64,
    speed_bps: u64,
    stage: &str,
) {
    app.emit(
        "proton://progress",
        ProtonDownloadProgress {
            bytes_downloaded,
            bytes_total,
            speed_bps,
            stage: stage.to_string(),
        },
    )
    .ok();
}
