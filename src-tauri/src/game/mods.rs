//! Support for the Endfield modding ecosystem.
//!
//! Practically every character/skin mod for Endfield runs on **3DMigoto** —
//! either the bare [Endfield fork](https://github.com/wakka810/3dmigoto-arknights-endfield)
//! or EFMI/XXMI built on top of it. 3DMigoto is a `d3d11.dll` proxy: it sits
//! between the game and D3D11, intercepts draw calls and swaps the vertex
//! buffers for the ones a mod supplies.
//!
//! Two consequences drive everything in this module:
//!
//! * **It is D3D11-only.** There is no Vulkan equivalent — the interception
//!   points simply do not exist in that API — so the game must run *without*
//!   `-vulkan`, on its D3D11 path (which Proton then translates back to Vulkan
//!   through DXVK). That costs frames, which is why modded launches are a
//!   separate action rather than a setting that silently taxes every session.
//! * **Wine has to be told to prefer the game's own `d3d11.dll`.** By default
//!   it loads its builtin (DXVK) and 3DMigoto never gets a look in;
//!   `WINEDLLOVERRIDES=d3d11=n,b` flips the order so the native proxy wins and
//!   chains to DXVK itself.
//!
//! Mods that patch the game rather than the renderer (Endfield Uncensored and
//! friends) are API-agnostic and need none of this — they work on a normal
//! Vulkan launch, so the launcher stays out of their way.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// The 3DMigoto proxy DLL, dropped next to `Endfield.exe`.
const LOADER_DLL: &str = "d3d11.dll";

/// 3DMigoto's configuration file, shipped alongside the DLL.
const LOADER_INI: &str = "d3dx.ini";

/// Where 3DMigoto looks for mods, one directory per mod.
const MODS_SUBDIR: &str = "Mods";

/// What the launcher knows about the mod setup in the game directory.
#[derive(Debug, Clone, Serialize)]
pub struct ModsStatus {
    /// A `d3d11.dll` proxy is present next to the game executable.
    pub loader_installed: bool,
    /// `d3dx.ini` is there too — without it 3DMigoto loads but does nothing,
    /// which is the usual "I installed it and no mods show up" case.
    pub loader_configured: bool,
    /// Absolute path of the `Mods` directory (whether or not it exists).
    pub mods_dir: String,
    /// Number of mods installed — every direct subdirectory counts as one.
    pub mod_count: usize,
    /// The game directory itself is missing, so nothing else here means much.
    pub game_dir_missing: bool,
}

/// The `Mods` directory 3DMigoto reads, next to the game executable.
pub fn mods_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(MODS_SUBDIR)
}

/// Inspect the game directory for an installed mod loader and its mods.
pub fn status(game_dir: &Path) -> ModsStatus {
    let mods = mods_dir(game_dir);

    // Only direct subdirectories count: 3DMigoto treats each one as a mod, and
    // loose files (a README, a stray .ini) are not mods.
    let mod_count = std::fs::read_dir(&mods)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter(|e| {
                    // 3DMigoto's own convention: a leading "DISABLED" marks a
                    // mod that is present but switched off.
                    !e.file_name()
                        .to_string_lossy()
                        .to_uppercase()
                        .starts_with("DISABLED")
                })
                .count()
        })
        .unwrap_or(0);

    ModsStatus {
        loader_installed: game_dir.join(LOADER_DLL).is_file(),
        loader_configured: game_dir.join(LOADER_INI).is_file(),
        mods_dir: mods.to_string_lossy().to_string(),
        mod_count,
        game_dir_missing: !game_dir.is_dir(),
    }
}

/// Create the `Mods` directory if it is not there yet, so "open mods folder"
/// always lands somewhere the user can drop a mod into.
pub fn ensure_mods_dir(game_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = mods_dir(game_dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// GitHub repository publishing the Endfield 3DMigoto build.
const LOADER_REPO: &str = "wakka810/3dmigoto-arknights-endfield";

/// Suffix appended to a game file we had to move aside, so an uninstall can
/// put the original back.
const BACKUP_SUFFIX: &str = ".llauncher-orig";

/// Entries of the release archive we deliberately do not install.
///
/// `nvapi64.dll` is 3DMigoto's stereo-3D helper. Under Proton it collides with
/// Wine's own nvapi implementation and buys a mod user nothing, so it stays
/// out. `loader.exe` is the alternative injection route — it works by
/// launching the game itself, which is our job, and the DLL proxy is the
/// method that actually behaves under Wine. The rest is repository noise.
fn is_skipped(rel: &str) -> bool {
    let lower = rel.to_lowercase();
    lower == "nvapi64.dll"
        || lower == "loader.exe"
        || lower == "loader.c"
        || lower == "readme.md"
        || lower.starts_with(".github/")
}

/// A file the game itself ships and the loader wants to replace. We move the
/// original aside instead of destroying it.
fn needs_backup(rel: &str) -> bool {
    rel.eq_ignore_ascii_case("d3dcompiler_47.dll")
}

#[derive(Debug, Clone, Serialize)]
pub struct LoaderInstallResult {
    /// Release tag that was installed.
    pub version: String,
    /// Number of files written into the game directory.
    pub files: usize,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// Download the latest 3DMigoto build and unpack it into the game directory.
///
/// The archive has a single top-level directory; its contents go directly next
/// to `Endfield.exe`, which is where a `d3d11.dll` proxy has to live to be
/// found ahead of the system one.
pub async fn install_loader(
    client: &reqwest::Client,
    game_dir: &Path,
) -> Result<LoaderInstallResult, AppError> {
    if !game_dir.is_dir() {
        return Err(AppError::GameNotFound(format!(
            "Game directory does not exist: {}",
            game_dir.display()
        )));
    }

    let release: GhRelease = client
        .get(format!(
            "https://api.github.com/repos/{}/releases/latest",
            LOADER_REPO
        ))
        .header("User-Agent", "LLauncher")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(".zip"))
        .ok_or_else(|| {
            AppError::Api("The 3DMigoto release has no .zip asset to install".to_string())
        })?;

    let bytes = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "LLauncher")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let game_dir = game_dir.to_path_buf();
    let version = release.tag_name.clone();
    let files = tokio::task::spawn_blocking(move || unpack_loader(&bytes, &game_dir))
        .await
        .map_err(|e| AppError::Api(format!("mod loader install task failed: {}", e)))??;

    crate::logging::info(format!(
        "mods: installed 3DMigoto {} ({} files)",
        version, files
    ));
    Ok(LoaderInstallResult { version, files })
}

/// Unpack the release archive into the game directory, stripping the single
/// top-level folder and skipping the entries we do not want.
fn unpack_loader(bytes: &[u8], game_dir: &Path) -> Result<usize, AppError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

    let mut written = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

        // `enclosed_name` rejects absolute paths and `..` traversal, so a
        // malicious archive cannot write outside the game directory.
        let Some(path) = entry.enclosed_name() else {
            continue;
        };

        // Strip the "<repo>-main/" wrapper the release archive puts around
        // everything.
        let mut parts = path.components();
        parts.next();
        let rel: PathBuf = parts.collect();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if is_skipped(&rel_str) {
            continue;
        }

        let target = game_dir.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Never clobber a mod the user already installed: the archive ships
        // sample content under Mods/, and their own files win.
        if rel_str.starts_with("Mods/") && target.exists() {
            continue;
        }

        // Keep the game's own copy of a file the loader overwrites, so
        // uninstalling can restore it.
        if needs_backup(&rel_str) && target.exists() {
            let backup = target.with_extension(format!(
                "{}{}",
                target.extension().unwrap_or_default().to_string_lossy(),
                BACKUP_SUFFIX
            ));
            if !backup.exists() {
                std::fs::rename(&target, &backup)?;
            }
        }

        let mut out = std::fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
        written += 1;
    }

    Ok(written)
}

/// Remove the loader, leaving the user's `Mods` directory untouched.
pub fn uninstall_loader(game_dir: &Path) -> Result<(), AppError> {
    for file in [LOADER_DLL, LOADER_INI, "d3dcompiler_47.dll"] {
        let path = game_dir.join(file);
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
    }

    // Put the game's original d3dcompiler back where the loader displaced it.
    let backup = game_dir.join(format!("d3dcompiler_47.dll{}", BACKUP_SUFFIX));
    if backup.is_file() {
        std::fs::rename(&backup, game_dir.join("d3dcompiler_47.dll"))?;
    }

    let shader_fixes = game_dir.join("ShaderFixes");
    if shader_fixes.is_dir() {
        std::fs::remove_dir_all(&shader_fixes)?;
    }

    crate::logging::info("mods: removed the 3DMigoto loader");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "llauncher-mods-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reports_a_bare_game_dir_as_unmodded() {
        let dir = tempdir();
        let s = status(&dir);
        assert!(!s.loader_installed);
        assert!(!s.loader_configured);
        assert_eq!(s.mod_count, 0);
        assert!(!s.game_dir_missing);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn counts_mod_directories_and_skips_disabled_ones() {
        // A mod is a directory; loose files are not, and 3DMigoto's DISABLED
        // prefix means the user switched that one off — neither should be
        // counted as an active mod.
        let dir = tempdir();
        std::fs::write(dir.join(LOADER_DLL), b"stub").unwrap();
        std::fs::write(dir.join(LOADER_INI), b"stub").unwrap();
        let mods = mods_dir(&dir);
        std::fs::create_dir_all(mods.join("SomeSkin")).unwrap();
        std::fs::create_dir_all(mods.join("AnotherSkin")).unwrap();
        std::fs::create_dir_all(mods.join("DISABLED_OldSkin")).unwrap();
        std::fs::write(mods.join("readme.txt"), b"hi").unwrap();

        let s = status(&dir);
        assert!(s.loader_installed);
        assert!(s.loader_configured);
        assert_eq!(s.mod_count, 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn flags_a_missing_game_dir() {
        let s = status(Path::new("/nonexistent/llauncher/game/dir"));
        assert!(s.game_dir_missing);
        assert!(!s.loader_installed);
    }
}
