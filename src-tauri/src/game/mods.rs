//! Support for the Endfield modding ecosystem.
//!
//! Character and skin mods for Endfield run on **EFMI** (Endfield Model
//! Importer), which is not a loader of its own but the Endfield-specific half
//! of one: 3DMigoto supplies the `d3d11.dll` proxy that sits between the game
//! and D3D11 and intercepts draw calls, and EFMI supplies the `d3dx.ini` and
//! `Core/EFMI` scripts that decide which vertex buffers to swap. The same
//! relationship GIMI has to Genshin.
//!
//! Both halves ship as plain GitHub release archives, one repository each, so
//! the launcher installs them itself. Upstream expects them to be laid down by
//! XXMI Launcher — a Windows GUI whose entire job is downloading these two
//! zips into the game directory — which is work this launcher already does,
//! and which would be an awkward thing to run inside the Proton prefix.
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
//! The proxy is also why `d3dx.ini`'s `[Loader]` section is irrelevant here:
//! it configures the alternative route, where a separate executable injects
//! the DLL into a game it starts itself. A proxy DLL next to `Endfield.exe`
//! needs none of that, and starting the game is this launcher's job.
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

/// EFMI's entry point, pulled in by `d3dx.ini`. Its presence is what tells a
/// current install apart from the bare-3DMigoto one earlier versions of the
/// launcher shipped.
const EFMI_MAIN_INI: [&str; 3] = ["Core", "EFMI", "main.ini"];

/// Everything the loader itself owns in the game directory, cleared before an
/// install writes the new one. `Mods` is pointedly not here: it is the user's.
const LOADER_DIRS: [&str; 2] = ["Core", "ShaderFixes"];

/// ReShade renames itself after the API it proxies. `d3d11.dll` belongs to
/// 3DMigoto here, so a ReShade install alongside it lands on `dxgi.dll`.
const RESHADE_DLL: &str = "dxgi.dll";

/// What the launcher knows about the mod setup in the game directory.
#[derive(Debug, Clone, Serialize)]
pub struct ModsStatus {
    /// A `d3d11.dll` proxy is present next to the game executable.
    pub loader_installed: bool,
    /// `d3dx.ini` is there too — without it 3DMigoto loads but does nothing,
    /// which is the usual "I installed it and no mods show up" case.
    pub loader_configured: bool,
    /// The installed loader is EFMI rather than the bare 3DMigoto build the
    /// launcher used to install. The old one still loads, but it has been
    /// unmaintained since January and mods built with the current toolkit
    /// expect EFMI, so the UI offers an upgrade rather than staying quiet.
    pub efmi: bool,
    /// Absolute path of the `Mods` directory (whether or not it exists).
    pub mods_dir: String,
    /// Number of mods installed — every direct subdirectory counts as one.
    pub mod_count: usize,
    /// ReShade (or another `dxgi.dll` proxy) is present. Detected rather than
    /// installed: ReShade ships as an interactive setup, and its add-ons —
    /// RenoDX and friends — are configured by hand anyway. All the launcher
    /// owes it is the DLL override, which the modded launch sets regardless.
    pub reshade_installed: bool,
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
        efmi: EFMI_MAIN_INI
            .iter()
            .fold(game_dir.to_path_buf(), |p, part| p.join(part))
            .is_file(),
        mods_dir: mods.to_string_lossy().to_string(),
        mod_count,
        reshade_installed: game_dir.join(RESHADE_DLL).is_file(),
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

/// The 3DMigoto binaries EFMI runs on — XXMI Launcher's own build, which is
/// the one EFMI is developed and tested against.
const LIBS_REPO: &str = "SpectrumQT/XXMI-Libs-Package";

/// The Endfield half: `d3dx.ini` and the `Core/EFMI` scripts.
const EFMI_REPO: &str = "SpectrumQT/EFMI-Package";

/// Suffix appended to a game file we had to move aside, so an uninstall can
/// put the original back.
const BACKUP_SUFFIX: &str = ".llauncher-orig";

/// Entries of the release archives we deliberately do not install.
///
/// `3dmloader.dll` drives the injection route, where a separate executable
/// starts the game and pushes the DLL into it. The proxy is the method that
/// behaves under Wine, and starting the game is our job either way.
/// `nvapi64.dll` is 3DMigoto's stereo-3D helper; the XXMI package does not
/// ship it today, but bare 3DMigoto builds do, and under Proton it collides
/// with Wine's own nvapi and buys a mod user nothing. The rest is
/// documentation that has no business in the game directory.
fn is_skipped(rel: &str) -> bool {
    let lower = rel.to_lowercase();
    lower == "3dmloader.dll"
        || lower == "nvapi64.dll"
        || lower == "readme.md"
        || lower == "license.gpl.txt"
        || lower.starts_with(".github/")
}

/// A file the game itself ships and the loader wants to replace. We move the
/// original aside instead of destroying it.
fn needs_backup(rel: &str) -> bool {
    rel.eq_ignore_ascii_case("d3dcompiler_47.dll")
}

#[derive(Debug, Clone, Serialize)]
pub struct LoaderInstallResult {
    /// EFMI release tag that was installed — the version a user comparing
    /// notes with a mod author cares about.
    pub version: String,
    /// Number of files written into the game directory.
    pub files: usize,
}

/// One downloaded release archive, waiting to be unpacked.
struct Package {
    tag: String,
    bytes: Vec<u8>,
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

/// Download the latest EFMI and the 3DMigoto build it runs on, and unpack both
/// into the game directory.
///
/// The two archives are disjoint — binaries from one, scripts from the other —
/// and both land directly next to `Endfield.exe`, which is where a `d3d11.dll`
/// proxy has to live to be found ahead of the system one.
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

    let libs = fetch_package(client, LIBS_REPO).await?;
    let efmi = fetch_package(client, EFMI_REPO).await?;

    let game_dir = game_dir.to_path_buf();
    let version = efmi.tag.clone();
    let libs_version = libs.tag.clone();
    let files = tokio::task::spawn_blocking(move || -> Result<usize, AppError> {
        // Stale scripts are worse than missing ones: EFMI moves ini files
        // between releases, and 3DMigoto happily loads whatever is left over
        // from the previous install alongside the new set. Clearing what the
        // loader owns first also upgrades an install from the old bare
        // 3DMigoto build, whose ShaderFixes would otherwise keep replacing
        // shaders EFMI never asked it to.
        clear_loader_files(&game_dir)?;
        let mut files = unpack_package(&libs.bytes, &game_dir)?;
        files += unpack_package(&efmi.bytes, &game_dir)?;
        Ok(files)
    })
    .await
    .map_err(|e| AppError::Api(format!("mod loader install task failed: {}", e)))??;

    crate::logging::info(format!(
        "mods: installed EFMI {} on 3DMigoto {} ({} files)",
        version, libs_version, files
    ));
    Ok(LoaderInstallResult { version, files })
}

/// Fetch a repository's latest release and download its `.zip` asset.
async fn fetch_package(client: &reqwest::Client, repo: &str) -> Result<Package, AppError> {
    let release: GhRelease = client
        .get(format!(
            "https://api.github.com/repos/{}/releases/latest",
            repo
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
            AppError::Api(format!("The latest {} release has no .zip asset", repo))
        })?;

    let bytes = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "LLauncher")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok(Package {
        tag: release.tag_name,
        bytes: bytes.to_vec(),
    })
}

/// Delete what a previous loader install put in the game directory, leaving
/// the user's `Mods` and the displaced-file backup alone.
fn clear_loader_files(game_dir: &Path) -> Result<(), AppError> {
    for dir in LOADER_DIRS {
        let path = game_dir.join(dir);
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        }
    }
    let ini = game_dir.join(LOADER_INI);
    if ini.is_file() {
        std::fs::remove_file(&ini)?;
    }
    Ok(())
}

/// Unpack a release archive into the game directory, skipping the entries we
/// do not want.
fn unpack_package(bytes: &[u8], game_dir: &Path) -> Result<usize, AppError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

    let root = common_root(&mut archive);

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

        let rel: PathBuf = match &root {
            Some(root) => match path.strip_prefix(root) {
                Ok(rel) => rel.to_path_buf(),
                Err(_) => continue,
            },
            None => path,
        };
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

        // Never clobber a mod the user already installed: the archives ship an
        // empty Mods/, and their own files win regardless.
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

/// The single top-level directory an archive wraps everything in, if it has
/// one. A GitHub source zip does (`<repo>-main/`) and its contents belong one
/// level up; the XXMI and EFMI release packages do not, and stripping a level
/// off those would scatter their files. Deciding per archive rather than by
/// repository keeps a repackaged release from silently installing into a
/// subdirectory.
fn common_root<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Option<PathBuf> {
    let mut root: Option<PathBuf> = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).ok()?;
        let is_dir = entry.is_dir();
        let path = entry.enclosed_name()?;
        let mut parts = path.components();
        let first = PathBuf::from(parts.next()?.as_os_str());
        // A file sitting at the archive root proves there is no wrapper.
        if parts.next().is_none() && !is_dir {
            return None;
        }
        match &root {
            None => root = Some(first),
            Some(seen) if *seen != first => return None,
            Some(_) => {}
        }
    }
    root
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

    for dir in LOADER_DIRS {
        let path = game_dir.join(dir);
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        }
    }

    crate::logging::info("mods: removed the mod loader");
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
        assert!(!s.efmi);
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
        // d3d11.dll and d3dx.ini without Core/EFMI: the bare 3DMigoto build
        // earlier versions installed, which the UI offers to upgrade.
        assert!(!s.efmi);
        assert_eq!(s.mod_count, 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Build an in-memory zip. A name ending in `/` becomes a directory
    /// entry, which is how the archives we install mark their empty folders.
    fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for (name, data) in entries {
                if let Some(dir) = name.strip_suffix('/') {
                    w.add_directory(dir, opts).unwrap();
                } else {
                    w.start_file(*name, opts).unwrap();
                    std::io::Write::write_all(&mut w, data).unwrap();
                }
            }
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn unpacks_a_flat_package_where_the_game_can_see_it() {
        // The EFMI and XXMI archives have no wrapper directory: their files
        // belong next to Endfield.exe exactly as they are named. 3dmloader is
        // the injection route we do not use and must not be written.
        let dir = tempdir();
        let zip = zip_with(&[
            ("d3d11.dll", b"proxy"),
            ("3dmloader.dll", b"injector"),
            ("d3dx.ini", b"[Include]"),
            ("Core/EFMI/main.ini", b"efmi"),
            ("Mods/", b""),
        ]);

        let written = unpack_package(&zip, &dir).unwrap();

        assert_eq!(written, 3);
        assert!(dir.join("d3d11.dll").is_file());
        assert!(dir.join("Core/EFMI/main.ini").is_file());
        assert!(!dir.join("3dmloader.dll").exists());
        assert!(status(&dir).efmi);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn strips_the_wrapper_of_a_source_archive() {
        // A GitHub source zip wraps everything in "<repo>-main/", and its
        // contents belong one level up.
        let dir = tempdir();
        let zip = zip_with(&[
            ("3dmigoto-main/", b""),
            ("3dmigoto-main/d3d11.dll", b"proxy"),
            ("3dmigoto-main/ShaderFixes/help.ini", b"fix"),
        ]);

        unpack_package(&zip, &dir).unwrap();

        assert!(dir.join("d3d11.dll").is_file());
        assert!(dir.join("ShaderFixes/help.ini").is_file());
        assert!(!dir.join("3dmigoto-main").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn never_overwrites_an_installed_mod() {
        let dir = tempdir();
        let mod_ini = mods_dir(&dir).join("MySkin/mod.ini");
        std::fs::create_dir_all(mod_ini.parent().unwrap()).unwrap();
        std::fs::write(&mod_ini, b"the user's").unwrap();

        unpack_package(&zip_with(&[("Mods/MySkin/mod.ini", b"the archive's")]), &dir).unwrap();

        assert_eq!(std::fs::read(&mod_ini).unwrap(), b"the user's");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn clearing_the_loader_leaves_the_mods_alone() {
        // An install clears what the previous loader owned so its stale ini
        // files cannot load alongside the new ones — but Mods is the user's,
        // and so is the backup of the file the loader displaced.
        let dir = tempdir();
        std::fs::create_dir_all(dir.join("Core/EFMI")).unwrap();
        std::fs::create_dir_all(dir.join("ShaderFixes")).unwrap();
        std::fs::create_dir_all(mods_dir(&dir).join("MySkin")).unwrap();
        std::fs::write(dir.join("Core/EFMI/gone.ini"), b"stale").unwrap();
        std::fs::write(dir.join("ShaderFixes/old.txt"), b"stale").unwrap();
        std::fs::write(dir.join(LOADER_INI), b"old").unwrap();
        let backup = dir.join(format!("d3dcompiler_47.dll{}", BACKUP_SUFFIX));
        std::fs::write(&backup, b"the game's own").unwrap();

        clear_loader_files(&dir).unwrap();

        assert!(!dir.join("Core").exists());
        assert!(!dir.join("ShaderFixes").exists());
        assert!(!dir.join(LOADER_INI).exists());
        assert!(mods_dir(&dir).join("MySkin").is_dir());
        assert!(backup.is_file());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn flags_a_missing_game_dir() {
        let s = status(Path::new("/nonexistent/llauncher/game/dir"));
        assert!(s.game_dir_missing);
        assert!(!s.loader_installed);
    }
}
