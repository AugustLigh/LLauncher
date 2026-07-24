use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct ShaderCacheResult {
    pub files_removed: u64,
    pub bytes_freed: u64,
}

/// Delete DXVK state caches for the game: `*.dxvk-cache` files anywhere under
/// the game directory plus the prefix's `shadercache` folder if present.
/// Vulkan pipeline caches are rebuilt transparently on the next launch.
pub fn clear_shader_cache(game_dir: &Path, compat_data: &Path) -> ShaderCacheResult {
    let mut result = ShaderCacheResult {
        files_removed: 0,
        bytes_freed: 0,
    };
    remove_dxvk_caches(game_dir, &mut result, 0);

    let shadercache = compat_data.join("shadercache");
    if shadercache.is_dir() {
        collect_dir_size(&shadercache, &mut result);
        let _ = std::fs::remove_dir_all(&shadercache);
    }
    result
}

fn remove_dxvk_caches(dir: &Path, result: &mut ShaderCacheResult, depth: u32) {
    // The cache sits next to the executable; a shallow bound keeps a scan of a
    // ~60 GB install from crawling every asset directory.
    if depth > 3 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_dxvk_caches(&path, result, depth + 1);
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".dxvk-cache") || n.ends_with(".dxvk-cache.tmp"))
        {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if std::fs::remove_file(&path).is_ok() {
                result.files_removed += 1;
                result.bytes_freed += size;
            }
        }
    }
}

fn collect_dir_size(dir: &Path, result: &mut ShaderCacheResult) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_dir_size(&path, result);
        } else if let Ok(meta) = entry.metadata() {
            result.files_removed += 1;
            result.bytes_freed += meta.len();
        }
    }
}

/// Archive the whole prefix (login session, in-game settings, registry) into
/// `dest` as a gzipped tarball. Uses the system tar: it is present on every
/// distro and, unlike the zip crate, preserves the symlinks a Wine prefix is
/// full of. Archives the *contents* so a restore is prefix-name-agnostic.
pub fn backup(compat_data: &Path, dest: &Path) -> Result<(), AppError> {
    if !compat_data.join("pfx").exists() {
        return Err(AppError::Api(
            "No Proton prefix exists yet — launch the game once first".to_string(),
        ));
    }

    let mut cmd = Command::new("tar");
    cmd.arg("-czf")
        .arg(dest)
        .arg("-C")
        .arg(compat_data)
        .arg(".");
    crate::util::strip_appimage_libs(&mut cmd);

    let output = cmd
        .output()
        .map_err(|e| AppError::Api(format!("Failed to run tar: {}", e)))?;
    if !output.status.success() {
        // Do not leave a truncated archive behind.
        let _ = std::fs::remove_file(dest);
        return Err(AppError::Api(format!(
            "tar failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Replace the prefix with the contents of a backup archive.
pub fn restore(compat_data: &Path, archive: &Path) -> Result<(), AppError> {
    if !archive.exists() {
        return Err(AppError::Api(format!(
            "Backup file not found: {}",
            archive.display()
        )));
    }

    // Sanity-check the archive really is a prefix backup before wiping
    // anything: it must contain the pfx/ directory at its root.
    let mut list_cmd = Command::new("tar");
    list_cmd.arg("-tzf").arg(archive);
    crate::util::strip_appimage_libs(&mut list_cmd);
    let listing = list_cmd
        .output()
        .map_err(|e| AppError::Api(format!("Failed to run tar: {}", e)))?;
    if !listing.status.success() {
        return Err(AppError::Api(format!(
            "Not a readable backup archive: {}",
            String::from_utf8_lossy(&listing.stderr).trim()
        )));
    }
    let has_pfx = String::from_utf8_lossy(&listing.stdout)
        .lines()
        .any(|l| l.trim_start_matches("./").starts_with("pfx"));
    if !has_pfx {
        return Err(AppError::Api(
            "Archive does not look like a prefix backup (no pfx/ inside)".to_string(),
        ));
    }

    if compat_data.exists() {
        std::fs::remove_dir_all(compat_data)?;
    }
    std::fs::create_dir_all(compat_data)?;

    let mut cmd = Command::new("tar");
    cmd.arg("-xzf").arg(archive).arg("-C").arg(compat_data);
    crate::util::strip_appimage_libs(&mut cmd);

    let output = cmd
        .output()
        .map_err(|e| AppError::Api(format!("Failed to run tar: {}", e)))?;
    if !output.status.success() {
        return Err(AppError::Api(format!(
            "tar failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Delete the prefix entirely. Proton recreates a clean one on the next
/// launch; the game will re-login and re-apply its settings.
pub fn reset(compat_data: &Path) -> Result<(), AppError> {
    if compat_data.exists() {
        std::fs::remove_dir_all(compat_data)?;
    }
    Ok(())
}
