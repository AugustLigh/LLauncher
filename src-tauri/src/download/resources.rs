//! File-level game sync via the official resource manifest.
//!
//! Unlike the `packs` flow (a full split-ZIP of the whole game, re-downloaded
//! in its entirety on every update), the resource manifest lists every VFS
//! asset individually with its MD5 and an individual download URL. By comparing
//! the manifest against what is on disk we download only the files that are
//! missing or changed — which is what powers both the "check file integrity"
//! action and incremental updates.
//!
//! Flow:
//!   1. `get_latest_game` → latest version + `pkg.file_path` (→ `rand_str`).
//!   2. `GET /game/get_latest_resources` → resource roots (`main`, `initial`).
//!   3. download + decrypt `index_<name>.json` (base64, then byte-subtract the
//!      recovered key) → list of `{name, md5, size}`.
//!   4. hash the local file for each entry; collect mismatches/missing.
//!   5. download only those from `<resource path>/<name>`, MD5-checked, written
//!      atomically via a temp file + rename.

use futures_util::StreamExt;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::api::constants::*;
use crate::error::AppError;

const PLATFORM: &str = "Windows";

// ─── API response shapes ───

#[derive(Debug, Deserialize)]
struct ResourcesResponse {
    #[serde(default)]
    resources: Vec<ResourceEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct ResourceEntry {
    name: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct ResourceIndex {
    #[serde(default)]
    files: Vec<IndexFile>,
}

#[derive(Debug, Deserialize)]
struct IndexFile {
    name: String,
    #[serde(default)]
    md5: Option<String>,
    #[serde(default)]
    size: u64,
}

/// One resolved manifest entry: where to fetch it and what it should hash to.
#[derive(Debug, Clone)]
struct ManifestFile {
    /// Path relative to the streaming-assets root, e.g. `VFS/AB/CD.chk`.
    name: String,
    md5: String,
    size: u64,
    url: String,
}

// ─── Progress events ───

#[derive(Debug, Clone, Serialize)]
pub struct IntegrityProgress {
    /// `fetching` | `verifying` | `downloading`
    pub stage: String,
    pub files_done: usize,
    pub total_files: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub speed_bps: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntegrityComplete {
    pub checked: usize,
    pub repaired: usize,
    pub bytes_downloaded: u64,
}

// ─── Helpers ───

fn major_minor(version: &str) -> String {
    version.split('.').take(2).collect::<Vec<_>>().join(".")
}

/// `.../1.3.3_BGlQHu2HgMlqTuqG/files` → `BGlQHu2HgMlqTuqG`.
fn extract_rand_str(file_path: &str) -> Option<String> {
    let dir = file_path.trim_end_matches('/').rsplit('/').nth(1)?;
    dir.rsplit_once('_').map(|(_, r)| r.to_string())
}

/// The index files are base64 text, then a byte-wise subtract cipher keyed on
/// [`RESOURCE_INDEX_KEY`]: `plain[i] = enc[i] - key[i % key.len()] (mod 256)`.
fn decrypt_index(body: &[u8]) -> Result<Vec<u8>, AppError> {
    use base64::Engine;
    let filtered: Vec<u8> = body
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(&filtered)
        .map_err(|e| AppError::Api(format!("resource index base64 decode failed: {e}")))?;

    let key = RESOURCE_INDEX_KEY;
    let mut out = vec![0u8; raw.len()];
    for (i, b) in raw.iter().enumerate() {
        out[i] = b.wrapping_sub(key[i % key.len()]);
    }
    Ok(out)
}

/// Reject manifest entries whose `name` could escape `assets_root` once
/// joined onto it (zip-slip style path traversal): absolute paths or any
/// `..` component. The resource index is decrypted server data — a
/// compromised/MITM'd CDN response must not be able to write outside the
/// game's StreamingAssets directory.
fn is_safe_relative_path(name: &str) -> bool {
    let path = Path::new(name);
    if path.is_absolute() {
        return false;
    }
    !path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
}

fn file_md5(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 4 * 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn get_resources(
    client: &reqwest::Client,
    version: &str,
    rand_str: &str,
) -> Result<ResourcesResponse, AppError> {
    let game_version = major_minor(version);
    let url = format!("{}/game/get_latest_resources", LAUNCHER_API_BASE);
    let resp = client
        .get(&url)
        .query(&[
            ("appcode", GAME_APPCODE),
            ("game_version", game_version.as_str()),
            ("version", version),
            ("platform", PLATFORM),
            ("rand_str", rand_str),
        ])
        .timeout(API_REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json::<ResourcesResponse>()
        .await?;
    Ok(resp)
}

async fn fetch_index(
    client: &reqwest::Client,
    res_path: &str,
    index_name: &str,
) -> Result<Vec<IndexFile>, AppError> {
    let url = format!("{}/{}", res_path.trim_end_matches('/'), index_name);
    let body = client
        .get(&url)
        .timeout(API_REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let decrypted = decrypt_index(&body)?;
    let index: ResourceIndex = serde_json::from_slice(&decrypted)
        .map_err(|e| AppError::Api(format!("resource index parse failed: {e}")))?;
    Ok(index.files)
}

/// Resolve the full manifest (every VFS file with its hash + download URL).
async fn build_manifest(
    client: &reqwest::Client,
    version: &str,
    rand_str: &str,
) -> Result<Vec<ManifestFile>, AppError> {
    let res = get_resources(client, version, rand_str).await?;
    let mut out = Vec::new();
    for entry in &res.resources {
        let index_name = if entry.name.contains("initial") {
            "index_initial.json"
        } else if entry.name.contains("main") {
            "index_main.json"
        } else {
            continue;
        };
        let files = fetch_index(client, &entry.path, index_name).await?;
        let base = entry.path.trim_end_matches('/');
        for f in files {
            let Some(md5) = f.md5 else { continue };
            if md5.is_empty() {
                continue;
            }
            if !is_safe_relative_path(&f.name) {
                continue;
            }
            out.push(ManifestFile {
                url: format!("{}/{}", base, f.name),
                name: f.name,
                md5,
                size: f.size,
            });
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn emit_progress(
    app: &tauri::AppHandle,
    channel: &str,
    stage: &str,
    files_done: usize,
    total_files: usize,
    bytes_done: u64,
    bytes_total: u64,
    speed_bps: u64,
) {
    app.emit(
        &format!("{}://progress", channel),
        IntegrityProgress {
            stage: stage.to_string(),
            files_done,
            total_files,
            bytes_done,
            bytes_total,
            speed_bps,
        },
    )
    .ok();
}

// ─── Orchestration ───

/// Verify the installed VFS assets against the latest manifest and re-download
/// any that are missing or corrupt. Returns how many were checked / repaired.
pub async fn verify_and_repair(
    app: tauri::AppHandle,
    client: reqwest::Client,
    cancel_flag: Arc<AtomicBool>,
    game_dir: String,
    max_concurrent: u32,
    channel: String,
) -> Result<IntegrityComplete, AppError> {
    cancel_flag.store(true, Ordering::SeqCst);
    let max_concurrent = max_concurrent.clamp(1, 8) as usize;

    // 1–3. Resolve the manifest.
    emit_progress(&app, &channel, "fetching", 0, 0, 0, 0, 0);
    let vinfo = crate::api::client::get_latest_game_version(&client, "").await?;
    let version = vinfo.version.clone();
    let rand_str = extract_rand_str(&vinfo.pkg.file_path)
        .ok_or_else(|| AppError::Api("Could not derive rand_str from file_path".into()))?;

    let manifest = Arc::new(build_manifest(&client, &version, &rand_str).await?);
    let total_files = manifest.len();
    if total_files == 0 {
        return Err(AppError::Api("Resource manifest was empty".into()));
    }

    let assets_root = Arc::new(Path::new(&game_dir).join(STREAMING_ASSETS_SUBDIR));

    // 4. Verify: hash every local file in parallel, collect the ones to fetch.
    let checked = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let mut verify_handles = Vec::with_capacity(total_files);

    for idx in 0..total_files {
        if !cancel_flag.load(Ordering::SeqCst) {
            return Err(AppError::Cancelled);
        }
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AppError::Cancelled)?;
        let manifest = manifest.clone();
        let assets_root = assets_root.clone();
        let checked = checked.clone();
        let app = app.clone();
        let cancel_flag = cancel_flag.clone();
        let channel = channel.clone();

        verify_handles.push(tokio::task::spawn_blocking(move || -> Option<usize> {
            let _permit = permit;
            let mf = &manifest[idx];
            let local = assets_root.join(&mf.name);
            let needs_download = if !cancel_flag.load(Ordering::SeqCst) {
                false
            } else {
                match std::fs::metadata(&local) {
                    Ok(m) if m.len() == mf.size => {
                        file_md5(&local).map(|h| h != mf.md5).unwrap_or(true)
                    }
                    _ => true,
                }
            };

            let done = checked.fetch_add(1, Ordering::Relaxed) + 1;
            if done % 32 == 0 || done == manifest.len() {
                emit_progress(&app, &channel, "verifying", done, manifest.len(), 0, 0, 0);
            }
            if needs_download {
                Some(idx)
            } else {
                None
            }
        }));
    }

    let mut to_download = Vec::new();
    for handle in verify_handles {
        if let Ok(Some(idx)) = handle.await {
            to_download.push(idx);
        }
    }
    if !cancel_flag.load(Ordering::SeqCst) {
        return Err(AppError::Cancelled);
    }
    emit_progress(&app, &channel, "verifying", total_files, total_files, 0, 0, 0);

    // 5. Download the mismatched / missing files.
    let repaired = to_download.len();
    let bytes_total: u64 = to_download.iter().map(|&i| manifest[i].size).sum();

    if repaired > 0 {
        std::fs::create_dir_all(assets_root.as_path())?;
        if let Some(available) = crate::config::paths::available_space(assets_root.as_path()) {
            if available < bytes_total {
                return Err(AppError::DiskSpace {
                    path: assets_root.to_string_lossy().to_string(),
                    needed_mib: bytes_total / (1024 * 1024),
                    available_mib: available / (1024 * 1024),
                });
            }
        }

        let downloaded = Arc::new(AtomicU64::new(0));
        let done_files = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        let mut dl_handles = Vec::with_capacity(repaired);

        for &idx in &to_download {
            if !cancel_flag.load(Ordering::SeqCst) {
                return Err(AppError::Cancelled);
            }
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| AppError::Cancelled)?;
            let mf = manifest[idx].clone();
            let assets_root = assets_root.clone();
            let client = client.clone();
            let cancel_flag = cancel_flag.clone();
            let downloaded = downloaded.clone();
            let done_files = done_files.clone();
            let app = app.clone();
            let channel = channel.clone();

            dl_handles.push(tokio::spawn(async move {
                let _permit = permit;
                let res = download_one(
                    &client,
                    &mf,
                    assets_root.as_path(),
                    &cancel_flag,
                    &downloaded,
                    &app,
                    &channel,
                    bytes_total,
                    repaired,
                    &done_files,
                    start,
                )
                .await;
                done_files.fetch_add(1, Ordering::Relaxed);
                res
            }));
        }

        for handle in dl_handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(AppError::Api(format!("integrity task failed: {e}"))),
            }
        }
        let total_dl = downloaded.load(Ordering::Relaxed);
        emit_progress(&app, &channel, "downloading", repaired, repaired, total_dl, bytes_total, 0);
    }

    let report = IntegrityComplete {
        checked: total_files,
        repaired,
        bytes_downloaded: bytes_total,
    };
    app.emit(&format!("{}://complete", channel), report.clone()).ok();
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
async fn download_one(
    client: &reqwest::Client,
    mf: &ManifestFile,
    assets_root: &Path,
    cancel_flag: &Arc<AtomicBool>,
    downloaded: &Arc<AtomicU64>,
    app: &tauri::AppHandle,
    channel: &str,
    bytes_total: u64,
    total_files: usize,
    done_files: &Arc<AtomicUsize>,
    start: Instant,
) -> Result<(), AppError> {
    let local = assets_root.join(&mf.name);
    if let Some(parent) = local.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(AppError::Io)?;
    }
    let tmp = local.with_file_name(format!(
        "{}.llauncher_tmp",
        local.file_name().unwrap_or_default().to_string_lossy()
    ));

    let response = crate::util::send_with_stall_timeout(client.get(&mf.url), DOWNLOAD_STALL_TIMEOUT)
        .await?
        .error_for_status()?;
    let mut stream = response.bytes_stream();
    let file = tokio::fs::File::create(&tmp).await.map_err(AppError::Io)?;
    let mut writer = BufWriter::with_capacity(1024 * 1024, file);
    let mut hasher = Md5::new();
    let mut last_emit = Instant::now();

    while let Some(chunk) = stream.next().await {
        if !cancel_flag.load(Ordering::SeqCst) {
            drop(writer);
            tokio::fs::remove_file(&tmp).await.ok();
            return Err(AppError::Cancelled);
        }
        let chunk = chunk.map_err(AppError::Http)?;
        writer.write_all(&chunk).await.map_err(AppError::Io)?;
        hasher.update(&chunk);
        let total_dl = downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed) + chunk.len() as u64;

        if last_emit.elapsed().as_millis() >= 150 {
            let elapsed = start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.1 {
                (total_dl as f64 / elapsed) as u64
            } else {
                0
            };
            emit_progress(
                app,
                channel,
                "downloading",
                done_files.load(Ordering::Relaxed),
                total_files,
                total_dl,
                bytes_total,
                speed,
            );
            last_emit = Instant::now();
        }
    }

    writer.flush().await.map_err(AppError::Io)?;
    drop(writer);

    let actual = format!("{:x}", hasher.finalize());
    if actual != mf.md5 {
        tokio::fs::remove_file(&tmp).await.ok();
        return Err(AppError::Md5Mismatch {
            expected: mf.md5.clone(),
            actual,
        });
    }

    tokio::fs::rename(&tmp, &local).await.map_err(AppError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn rand_str_from_file_path() {
        assert_eq!(
            extract_rand_str(
                "https://beyond.hg-cdn.com/APP/1.3/update/6/6/Windows/1.3.3_BGlQHu2HgMlqTuqG/files"
            ),
            Some("BGlQHu2HgMlqTuqG".to_string())
        );
        // Trailing slash tolerated.
        assert_eq!(
            extract_rand_str(".../1.0.14_Qk2mXHuAH1JWKF37/files/"),
            Some("Qk2mXHuAH1JWKF37".to_string())
        );
    }

    #[test]
    fn major_minor_takes_two_components() {
        assert_eq!(major_minor("1.3.3"), "1.3");
        assert_eq!(major_minor("1.0.14"), "1.0");
    }

    #[test]
    fn rejects_path_traversal_in_manifest_names() {
        // A malicious/compromised resource index must not be able to write
        // outside the game's StreamingAssets root via `..` or an absolute path.
        assert!(!is_safe_relative_path("../../etc/cron.d/evil"));
        assert!(!is_safe_relative_path("VFS/../../etc/passwd"));
        assert!(!is_safe_relative_path("/etc/passwd"));
        assert!(is_safe_relative_path("VFS/AB/CD.chk"));
    }

    #[test]
    fn decrypt_round_trips_the_official_cipher() {
        // Mirror the launcher's encrypt side: add key bytes, then base64.
        let plain = br#"{"files":[{"name":"VFS/AB/CD.chk","md5":"deadbeef","size":12}]}"#;
        let key = RESOURCE_INDEX_KEY;
        let enc: Vec<u8> = plain
            .iter()
            .enumerate()
            .map(|(i, b)| b.wrapping_add(key[i % key.len()]))
            .collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&enc);

        let decoded = decrypt_index(b64.as_bytes()).unwrap();
        assert_eq!(decoded, plain);
        // And it parses as the index shape we expect.
        let idx: ResourceIndex = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(idx.files.len(), 1);
        assert_eq!(idx.files[0].name, "VFS/AB/CD.chk");
        assert_eq!(idx.files[0].size, 12);
    }
}
