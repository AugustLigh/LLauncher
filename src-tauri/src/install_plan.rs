use crate::{error::AppError, state::AppState};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct DiskRequirement {
    path: String,
    pub available: Option<u64>,
    pub required: u64,
}
#[derive(Serialize)]
pub struct InstallPlan {
    pub download_bytes: u64,
    pub unpacked_bytes: Option<u64>,
    pub disks: Vec<DiskRequirement>,
    pub blocked: bool,
}
fn ancestor(path: &Path) -> Option<&Path> {
    let mut probe = path;
    while !probe.exists() {
        probe = probe.parent()?;
    }
    Some(probe)
}
#[cfg(unix)]
fn volume(path: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    Some(std::fs::metadata(ancestor(path)?).ok()?.dev().to_string())
}
#[cfg(windows)]
fn volume(path: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetVolumePathNameW;
    let existing = std::fs::canonicalize(ancestor(path)?).ok()?;
    let path: Vec<u16> = existing
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut buffer = vec![0u16; 32768];
    if unsafe { GetVolumePathNameW(path.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) } == 0 {
        return None;
    }
    let end = buffer.iter().position(|&c| c == 0)?;
    Some(String::from_utf16_lossy(&buffer[..end]).to_lowercase())
}
fn requirements(parts: Vec<(PathBuf, u64)>) -> Vec<DiskRequirement> {
    let mut disks: BTreeMap<String, DiskRequirement> = BTreeMap::new();
    for (path, required) in parts {
        let path_text = path.to_string_lossy().to_string();
        let key = volume(&path).unwrap_or_else(|| path_text.clone());
        let disk = disks.entry(key).or_insert_with(|| DiskRequirement {
            path: path_text,
            available: crate::config::paths::available_space(&path),
            required: 0,
        });
        disk.required = disk.required.saturating_add(required);
    }
    disks.into_values().collect()
}
#[tauri::command]
pub async fn get_install_plan(state: tauri::State<'_, AppState>) -> Result<InstallPlan, AppError> {
    let settings = state.settings.lock().await.clone();
    let info = crate::api::client::get_latest_game_version(&state.http_client, "").await?;
    let download_bytes: u64 = info
        .pkg
        .packs
        .iter()
        .filter_map(|p| p.package_size.parse::<u64>().ok())
        .sum();
    let unpacked_bytes = match crate::download::packindex::fetch_central_directory(
        &state.http_client,
        &info.pkg.packs,
    )
    .await
    {
        Ok(entries) => entries
            .iter()
            .try_fold(0u64, |sum, e| sum.checked_add(e.unpacked_size?)),
        Err(_) => None,
    };
    let packs = info.pkg.packs;
    let disks = tokio::task::spawn_blocking(move || {
        let cached: u64 = packs
            .iter()
            .filter_map(|pack| {
                let name = pack.url.rsplit('/').next()?;
                if name.is_empty() || name == "." || name == ".." {
                    return None;
                }
                let size = pack.package_size.parse::<u64>().ok()?;
                Some(
                    std::fs::metadata(Path::new(&settings.download_dir).join(name))
                        .ok()?
                        .len()
                        .min(size),
                )
            })
            .sum();
        let mut parts = vec![(
            PathBuf::from(settings.download_dir),
            download_bytes.saturating_sub(cached),
        )];
        parts.push((
            PathBuf::from(settings.game_dir),
            unpacked_bytes.unwrap_or(0),
        ));
        requirements(parts)
    })
    .await
    .map_err(|e| AppError::Api(e.to_string()))?;
    let blocked = disks
        .iter()
        .any(|d| d.available.is_some_and(|free| free < d.required));
    Ok(InstallPlan {
        download_bytes,
        unpacked_bytes,
        disks,
        blocked,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds_peak_requirements_when_folders_share_a_volume() {
        let root = std::env::temp_dir();
        let disks = requirements(vec![
            (root.join("llauncher-game"), 80),
            (root.join("llauncher-cache"), 40),
        ]);
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].required, 120);
    }
}
