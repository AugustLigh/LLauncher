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
//!
//! Mods that patch the game rather than the renderer (Endfield Uncensored and
//! friends) are API-agnostic and need none of this — they work on a normal
//! Vulkan launch, so the launcher stays out of their way.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::AppError;

const LOADER_SUBDIR: &str = "EFMI";

const LOADER_DLL: &str = "d3d11.dll";

/// 3DMigoto's configuration file, shipped alongside the DLL.
const LOADER_INI: &str = "d3dx.ini";

const INJECTOR_DLL: &str = "3dmloader.dll";

const INJECTOR_EXE: &str = "efmi-inject.exe";
static INJECTOR_EXE_BYTES: &[u8] = include_bytes!("../../efmi-inject/efmi-inject.exe");

pub const GAME_PROCESS: &str = "Endfield.exe";

/// Where 3DMigoto looks for mods, one directory per mod.
const MODS_SUBDIR: &str = "Mods";

/// EFMI's entry point, pulled in by `d3dx.ini`. Its presence is what tells a
/// current install apart from the bare-3DMigoto one earlier versions of the
/// launcher shipped.
const EFMI_MAIN_INI: [&str; 3] = ["Core", "EFMI", "main.ini"];

/// ReShade renames itself after the API it proxies.
const RESHADE_DLL: &str = "dxgi.dll";

/// What the launcher knows about the mod setup in the game directory.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ModsStatus {
    /// 3DMigoto's `d3d11.dll` is in the loader directory.
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
    #[specta(type = f64)]
    pub mod_count: usize,
    /// ReShade (or another `dxgi.dll` proxy) is present. Detected rather than
    /// installed: ReShade ships as an interactive setup, and its add-ons —
    /// RenoDX and friends — are configured by hand anyway. All the launcher
    /// owes it is the DLL override, which the modded launch sets regardless.
    pub reshade_installed: bool,
    /// The game directory itself is missing, so nothing else here means much.
    pub game_dir_missing: bool,
}

pub fn loader_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(LOADER_SUBDIR)
}

/// The `Mods` directory 3DMigoto reads, next to its own DLL.
pub fn mods_dir(game_dir: &Path) -> PathBuf {
    loader_dir(game_dir).join(MODS_SUBDIR)
}

pub struct Injection {
    pub exe: PathBuf,
    pub injector: PathBuf,
    pub dll: PathBuf,
}

pub fn prepare_injection(game_dir: &Path) -> Result<Injection, AppError> {
    let dir = loader_dir(game_dir);
    let injection = Injection {
        exe: dir.join(INJECTOR_EXE),
        injector: dir.join(INJECTOR_DLL),
        dll: dir.join(LOADER_DLL),
    };
    if !injection.injector.is_file() || !injection.dll.is_file() {
        return Err(AppError::Api(
            "The mod loader is missing or out of date. Install it again from Settings > Mods."
                .to_string(),
        ));
    }
    std::fs::write(&injection.exe, INJECTOR_EXE_BYTES)?;
    Ok(injection)
}

/// Inspect the game directory for an installed mod loader and its mods.
pub fn status(game_dir: &Path) -> ModsStatus {
    let mods = mods_dir(game_dir);
    let dir = loader_dir(game_dir);

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
        loader_installed: dir.join(LOADER_DLL).is_file() && dir.join(INJECTOR_DLL).is_file(),
        loader_configured: dir.join(LOADER_INI).is_file(),
        efmi: EFMI_MAIN_INI
            .iter()
            .fold(dir, |p, part| p.join(part))
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

/// Entries of the release archives we deliberately do not install.
///
/// `nvapi64.dll` is 3DMigoto's stereo-3D helper; the XXMI package does not
/// ship it today, but bare 3DMigoto builds do, and under Proton it collides
/// with Wine's own nvapi and buys a mod user nothing. The rest is
/// documentation that has no business in the game directory.
fn is_skipped(rel: &str) -> bool {
    let lower = rel.to_lowercase();
    lower == "nvapi64.dll"
        || lower == "readme.md"
        || lower == "license.gpl.txt"
        || lower.starts_with(".github/")
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LoaderInstallResult {
    /// EFMI release tag that was installed — the version a user comparing
    /// notes with a mod author cares about.
    pub version: String,
    /// Number of files written into the loader directory.
    #[specta(type = f64)]
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
/// into the loader directory.
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
        // from the previous install alongside the new set.
        let dir = loader_dir(&game_dir);
        clear_loader_files(&dir)?;
        let mut files = unpack_package(&libs.bytes, &dir)?;
        files += unpack_package(&efmi.bytes, &dir)?;
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

/// Delete everything in the loader directory but the user's mods and toggles.
fn clear_loader_files(dir: &Path) -> std::io::Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name == MODS_SUBDIR || name == "d3dx_user.ini" {
            continue;
        }
        if entry.path().is_dir() {
            std::fs::remove_dir_all(entry.path())?;
        } else {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

/// Unpack a release archive into the loader directory, skipping the entries we
/// do not want.
fn unpack_package(bytes: &[u8], dir: &Path) -> Result<usize, AppError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

    let root = common_root(&mut archive);

    let mut written = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

        // `enclosed_name` rejects absolute paths and `..` traversal, so a
        // malicious archive cannot write outside the loader directory.
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

        let target = dir.join(&rel);
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
    clear_loader_files(&loader_dir(game_dir))?;
    crate::logging::info("mods: removed the mod loader");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "llauncher-mods-test-{}-{:?}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            count
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
        let mods = mods_dir(&dir);
        std::fs::create_dir_all(&mods).unwrap();
        for f in [LOADER_DLL, INJECTOR_DLL, LOADER_INI] {
            std::fs::write(loader_dir(&dir).join(f), b"stub").unwrap();
        }
        std::fs::create_dir_all(mods.join("SomeSkin")).unwrap();
        std::fs::create_dir_all(mods.join("AnotherSkin")).unwrap();
        std::fs::create_dir_all(mods.join("DISABLED_OldSkin")).unwrap();
        std::fs::write(mods.join("readme.txt"), b"hi").unwrap();

        let s = status(&dir);
        assert!(s.loader_installed);
        assert!(s.loader_configured);
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
        let dir = tempdir();
        let zip = zip_with(&[
            ("d3d11.dll", b"3dmigoto"),
            ("3dmloader.dll", b"injector"),
            ("d3dx.ini", b"[Include]"),
            ("Core/EFMI/main.ini", b"efmi"),
            ("Mods/", b""),
        ]);

        let written = unpack_package(&zip, &loader_dir(&dir)).unwrap();

        assert_eq!(written, 4);
        let s = status(&dir);
        assert!(s.loader_installed);
        assert!(s.efmi);
        assert!(prepare_injection(&dir).unwrap().exe.is_file());
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

        unpack_package(
            &zip_with(&[("Mods/MySkin/mod.ini", b"the archive's")]),
            &loader_dir(&dir),
        )
        .unwrap();

        assert_eq!(std::fs::read(&mod_ini).unwrap(), b"the user's");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn uninstalling_leaves_the_mods_and_toggles_alone() {
        let dir = tempdir();
        let loader = loader_dir(&dir);
        std::fs::create_dir_all(loader.join("Core/EFMI")).unwrap();
        std::fs::create_dir_all(mods_dir(&dir).join("MySkin")).unwrap();
        for f in [LOADER_DLL, LOADER_INI, "d3dx_user.ini"] {
            std::fs::write(loader.join(f), b"stub").unwrap();
        }

        uninstall_loader(&dir).unwrap();

        let mut left: Vec<String> = std::fs::read_dir(&loader)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, vec![MODS_SUBDIR, "d3dx_user.ini"]);
        assert!(mods_dir(&dir).join("MySkin").is_dir());
        assert!(!status(&dir).loader_installed);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn flags_a_missing_game_dir() {
        let s = status(Path::new("/nonexistent/llauncher/game/dir"));
        assert!(s.game_dir_missing);
        assert!(!s.loader_installed);
    }
}
