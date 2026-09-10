//! The macOS compatibility layer, downloaded and installed by the launcher —
//! the DWProton flow's counterpart, assembled from three open-source pieces:
//!
//! * **Wine Staging for macOS** — the official WineHQ macOS packages
//!   (Gcenx/macOS_Wine_builds): a `Wine Staging.app` bundle in a `.tar.xz`,
//!   x86-64 only, with winevulkan + MoltenVK inside.
//! * **The Endfield modules** — four Wine modules (`ntdll.so`, `ntdll.dll`,
//!   `kernel32.dll`, `ntoskrnl.exe`) built by this project's own CI from the
//!   same Wine sources, with the dw-proton anti-cheat patches and the
//!   Rosetta 2 fixes from Endfield_FineWine applied (see `macos/wine/`).
//!   They replace the originals inside the bundle. Without them the game's
//!   anti-cheat driver aborts on kernel calls Wine leaves unimplemented, and
//!   under Rosetta the protector faults on plain NOPs — so a Wine build is
//!   only offered when a module set exists for that exact version.
//! * **DXMT** (3Shain/dxmt) — a Direct3D 11 → Metal translator, the
//!   open-source equivalent of the D3DMetal CrossOver ships. Its `-builtin`
//!   release is a set of Wine builtin DLLs that replace the wined3d ones
//!   inside the bundle, which is exactly how Heroic installs it.
//!
//! Neither Wine nor DXMT has anything to do with Endfield; the module set is
//! the only game-specific part, and it is a drop-in like the others. This
//! module produces a Wine install and hands back its path.
//!
//! Compiled on every platform even though only macOS ever calls it: the code
//! is plain HTTP + tar + file copies, and the Linux CI job is the only one
//! that runs `cargo check` on every push, so keeping it in the shared build
//! catches mistakes here without a Mac.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::Emitter;

use crate::api::constants::API_REQUEST_TIMEOUT;
use crate::api::types::{InstalledProton, ProtonDownloadComplete, ProtonReleaseInfo};
use crate::error::AppError;

use super::proton::{download_asset, emit_progress, parse_release_with};

const WINE_RELEASES_URL: &str = "https://api.github.com/repos/Gcenx/macOS_Wine_builds/releases";
const DXMT_LATEST_URL: &str = "https://api.github.com/repos/3Shain/dxmt/releases/latest";

/// Where the module sets are published: one GitHub release per Wine version,
/// tagged `wine-modules-<version>`, made by `.github/workflows/wine-macos.yml`.
pub const MODULES_REPO: &str = "AugustLigh/LLauncher";
const MODULES_TAG_PREFIX: &str = "wine-modules-";

/// The Wine version installed by default and flagged as recommended in the
/// picker: the one the module set was last built and smoke-tested against.
pub const RECOMMENDED_WINE_TAG: &str = "11.16";

/// Where the Wine binary lives inside a WineHQ macOS bundle.
const WINE_BIN_IN_APP: &str = "Contents/Resources/wine/bin/wine";
/// Where Wine's builtin modules live, and therefore where DXMT's and the
/// patched ones go.
const WINE_LIB_IN_APP: &str = "Contents/Resources/wine/lib/wine";

/// Markers written next to the bundle, recording what went into it.
const DXMT_MARKER: &str = "dxmt.version";
const MODULES_MARKER: &str = "endfield-modules.version";

/// The patched modules, relative to both the module archive's `lib/wine`
/// and the bundle's. `ntdll.dll` is unchanged source, but the PE half of
/// ntdll and its unix half share a syscall table, so they travel together.
const MODULE_FILES: [&str; 4] = [
    "x86_64-unix/ntdll.so",
    "x86_64-windows/ntdll.dll",
    "x86_64-windows/kernel32.dll",
    "x86_64-windows/ntoskrnl.exe",
];

/// The DXMT files that go into the bundle, relative to both the DXMT tarball
/// root and `lib/wine`. Same list Heroic copies. 32-bit ones included: the
/// game is 64-bit, but a launcher-run tool inside the prefix need not be.
const DXMT_FILES: [&str; 11] = [
    "x86_64-windows/d3d10core.dll",
    "x86_64-windows/d3d11.dll",
    "x86_64-windows/dxgi.dll",
    "x86_64-windows/nvapi64.dll",
    "x86_64-windows/nvngx.dll",
    "x86_64-windows/winemetal.dll",
    "x86_64-unix/winemetal.so",
    "i386-windows/d3d10core.dll",
    "i386-windows/d3d11.dll",
    "i386-windows/dxgi.dll",
    "i386-windows/winemetal.dll",
];

/// GitHub's API refuses requests without a User-Agent.
fn github(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    client
        .get(url)
        .header("User-Agent", concat!("LLauncher/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .timeout(API_REQUEST_TIMEOUT)
}

fn is_wine_staging_asset(name: &str) -> bool {
    name.starts_with("wine-staging-") && name.ends_with("-osx64.tar.xz")
}

/// The archive `macos/wine/build-modules.sh` produces for one Wine version.
fn modules_asset_name(wine_tag: &str) -> String {
    format!("endfield-wine-modules-{}-macos.tar.xz", wine_tag)
}

fn modules_release_tag(wine_tag: &str) -> String {
    format!("{}{}", MODULES_TAG_PREFIX, wine_tag)
}

/// Every Wine Staging release WineHQ published, newest first. Each release
/// also carries a `wine-devel` build; staging is the one Heroic pairs with
/// DXMT and the one the module sets are built against.
async fn list_wine_builds(client: &reqwest::Client) -> Result<Vec<ProtonReleaseInfo>, AppError> {
    let resp: Vec<serde_json::Value> = github(client, WINE_RELEASES_URL)
        .query(&[("per_page", "20")])
        .send()
        .await?
        .json()
        .await?;

    Ok(resp
        .iter()
        .filter(|r| !r["prerelease"].as_bool().unwrap_or(false))
        .filter_map(|r| parse_release_with(r, is_wine_staging_asset))
        .collect())
}

/// The Wine versions a module set has been published for.
async fn list_module_sets(client: &reqwest::Client) -> Result<HashSet<String>, AppError> {
    let url = format!("https://api.github.com/repos/{}/releases", MODULES_REPO);
    let resp: Vec<serde_json::Value> = github(client, &url)
        .query(&[("per_page", "100")])
        .send()
        .await?
        .json()
        .await?;

    Ok(resp
        .iter()
        .filter_map(|r| r["tag_name"].as_str()?.strip_prefix(MODULES_TAG_PREFIX))
        .map(str::to_string)
        .collect())
}

/// The Wine builds the launcher can offer: those WineHQ published *and* a
/// module set exists for, newest first. A Wine without the modules cannot
/// run the game, so it is not listed at all.
pub async fn list_releases(client: &reqwest::Client) -> Result<Vec<ProtonReleaseInfo>, AppError> {
    let (builds, sets) = tokio::try_join!(list_wine_builds(client), list_module_sets(client))?;
    let releases: Vec<ProtonReleaseInfo> =
        builds.into_iter().filter(|r| sets.contains(&r.tag_name)).collect();
    if releases.is_empty() {
        return Err(AppError::ProtonDownloadFailed(
            "No Wine build with a published Endfield module set was found".into(),
        ));
    }
    Ok(releases)
}

/// The build installed when nothing is chosen: the recommended one while a
/// module set for it is still published, else the newest one that has one.
pub async fn get_latest(client: &reqwest::Client) -> Result<ProtonReleaseInfo, AppError> {
    let releases = list_releases(client).await?;
    Ok(releases
        .iter()
        .find(|r| r.tag_name == RECOMMENDED_WINE_TAG)
        .cloned()
        .unwrap_or_else(|| releases[0].clone()))
}

/// The module set for one Wine version.
async fn get_module_set(client: &reqwest::Client, wine_tag: &str) -> Result<ProtonReleaseInfo, AppError> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/tags/{}",
        MODULES_REPO,
        modules_release_tag(wine_tag)
    );
    let resp: serde_json::Value = github(client, &url).send().await?.json().await?;
    let wanted = modules_asset_name(wine_tag);
    parse_release_with(&resp, |name| name == wanted).ok_or_else(|| {
        AppError::ProtonDownloadFailed(format!(
            "No Endfield module set is published for Wine {} (release {} has no {})",
            wine_tag,
            modules_release_tag(wine_tag),
            wanted
        ))
    })
}

async fn get_latest_dxmt(client: &reqwest::Client) -> Result<ProtonReleaseInfo, AppError> {
    let resp: serde_json::Value = github(client, DXMT_LATEST_URL).send().await?.json().await?;
    parse_release_with(&resp, |name| name.starts_with("dxmt-") && name.ends_with("-builtin.tar.gz"))
        .ok_or_else(|| AppError::ProtonDownloadFailed("No DXMT builtin asset found".into()))
}

/// Download Wine Staging into `<base_dir>/wine-staging-<tag>/`, then — on
/// Apple silicon — fetch DXMT and drop it into the bundle, then fetch the
/// module set for that Wine version and drop that in too. Returns the bundle
/// path (what `macos_wine_dir` should point at) and the Wine tag.
///
/// Every download reports on the `proton://*` channels, so the frontend's
/// existing progress UI covers them; each later stage is announced as
/// another "downloading" pass. Cancelling during any of them aborts the
/// whole install.
pub async fn download_and_install(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    cancel_flag: &Arc<AtomicBool>,
    base_dir: &Path,
    release: Option<ProtonReleaseInfo>,
) -> Result<(String, String), AppError> {
    let info = match release {
        Some(r) => r,
        None => get_latest(client).await?,
    };

    // The modules are fetched before the 190 MB Wine download, so a version
    // without a published set fails in a second rather than after it.
    let modules = get_module_set(client, &info.tag_name).await?;

    // Every Wine build for macOS is x86-64, so an Apple silicon Mac needs
    // Rosetta 2 before any of this is worth downloading. Installing it takes
    // an administrator, which a GUI app asks for through the system's own
    // password dialog — that keeps "download, then play" a two-click affair
    // instead of sending the user to Terminal.
    if host_is_apple_silicon() && !rosetta_available() {
        emit_progress(app, 0, 0, 0, "rosetta");
        tokio::task::spawn_blocking(install_rosetta)
            .await
            .map_err(|e| AppError::Api(format!("rosetta task failed: {}", e)))??;
    }

    // One directory per release, like the DWProton layout: the bundle inside
    // is always called `Wine Staging.app`, so the tag has to go on the parent.
    // A leftover from an interrupted install is wiped first, so the markers
    // never describe a bundle that is half one thing and half another.
    let install_dir = base_dir.join(format!("wine-staging-{}", info.tag_name));
    if install_dir.exists() {
        std::fs::remove_dir_all(&install_dir)?;
    }
    std::fs::create_dir_all(&install_dir)?;
    let archive_path = install_dir.join(&info.file_name);

    let (downloaded, total) = download_asset(app, client, cancel_flag, &info, &archive_path).await?;
    emit_progress(app, downloaded, total, 0, "extracting");
    extract(&archive_path, &install_dir, "-xJf")?;
    let _ = std::fs::remove_file(&archive_path);

    let app_dir = find_wine_app(&install_dir).ok_or_else(|| {
        AppError::ProtonDownloadFailed("Extracted archive holds no Wine bundle".into())
    })?;

    // DXMT is what makes this a gaming stack rather than a bare Wine. It
    // supports Intel Macs only experimentally and only for a few GPUs — a
    // wrong guess there leaves the game unable to render at all — so, like
    // Heroic, it is installed on Apple silicon alone. Intel keeps wined3d and
    // the `-vulkan` path.
    if host_is_apple_silicon() {
        let dxmt = get_latest_dxmt(client).await?;
        let dxmt_dir = base_dir.join("dxmt");
        std::fs::create_dir_all(&dxmt_dir)?;
        let dxmt_archive = dxmt_dir.join(&dxmt.file_name);

        let (downloaded, total) =
            download_asset(app, client, cancel_flag, &dxmt, &dxmt_archive).await?;
        emit_progress(app, downloaded, total, 0, "extracting");
        extract(&dxmt_archive, &dxmt_dir, "-xzf")?;
        let _ = std::fs::remove_file(&dxmt_archive);

        let payload = find_dxmt_payload(&dxmt_dir).ok_or_else(|| {
            AppError::ProtonDownloadFailed("Extracted DXMT archive holds no winemetal.so".into())
        })?;
        install_dxmt(&payload, &app_dir)?;
        std::fs::write(install_dir.join(DXMT_MARKER), &dxmt.tag_name)?;
        // The unpacked payload has served its purpose; the marker records it.
        let _ = std::fs::remove_dir_all(&payload);
    }

    // The Endfield modules go in last, over whatever the two stages above
    // left: they touch none of the same files.
    let modules_dir = install_dir.join("modules");
    std::fs::create_dir_all(&modules_dir)?;
    let modules_archive = modules_dir.join(&modules.file_name);
    let (downloaded, total) =
        download_asset(app, client, cancel_flag, &modules, &modules_archive).await?;
    emit_progress(app, downloaded, total, 0, "extracting");
    extract(&modules_archive, &modules_dir, "-xJf")?;
    let payload = find_modules_payload(&modules_dir).ok_or_else(|| {
        AppError::ProtonDownloadFailed("Extracted module archive holds no ntdll.so".into())
    })?;
    install_modules(&payload, &app_dir)?;
    std::fs::write(install_dir.join(MODULES_MARKER), &modules.tag_name)?;
    let _ = std::fs::remove_dir_all(&modules_dir);

    let wine_dir = app_dir.to_string_lossy().to_string();
    app.emit(
        "proton://complete",
        ProtonDownloadComplete {
            proton_dir: wine_dir.clone(),
            version: info.tag_name.clone(),
        },
    )
    .ok();

    Ok((wine_dir, info.tag_name))
}

fn extract(archive: &Path, dest: &Path, flags: &str) -> Result<(), AppError> {
    let output = std::process::Command::new("tar")
        .arg(flags)
        .arg(archive)
        .arg("-C")
        .arg(dest)
        .output()
        .map_err(|_| AppError::TarNotFound)?;
    if !output.status.success() {
        return Err(AppError::ExtractionFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(())
}

/// Copy DXMT's builtins over Wine's. Every file is required: a half-installed
/// DXMT (say, `d3d11.dll` without `winemetal.so`) fails in ways that look
/// like a broken game rather than a broken install.
fn install_dxmt(payload: &Path, app_dir: &Path) -> Result<(), AppError> {
    let lib = app_dir.join(WINE_LIB_IN_APP);
    for rel in DXMT_FILES {
        let from = payload.join(rel);
        let to = lib.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&from, &to).map_err(|e| {
            AppError::ProtonDownloadFailed(format!("DXMT: cannot install {}: {}", rel, e))
        })?;
    }
    Ok(())
}

/// Copy the patched modules over Wine's own, keeping the originals as
/// `<name>.orig` beside them for anyone who wants to compare. The Mach-O one
/// is re-signed afterwards: every native library in the bundle carries an
/// ad-hoc signature and the copy keeps the one from CI, but signing again
/// costs nothing and covers a build that skipped it.
fn install_modules(payload: &Path, app_dir: &Path) -> Result<(), AppError> {
    let lib = app_dir.join(WINE_LIB_IN_APP);
    for rel in MODULE_FILES {
        let from = payload.join("lib/wine").join(rel);
        let to = lib.join(rel);
        if !from.is_file() {
            return Err(AppError::ProtonDownloadFailed(format!(
                "Module archive is missing {}",
                rel
            )));
        }
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let backup = to.with_extension(format!(
            "{}.orig",
            to.extension().and_then(|e| e.to_str()).unwrap_or("")
        ));
        if to.is_file() && !backup.exists() {
            std::fs::copy(&to, &backup)?;
        }
        std::fs::copy(&from, &to).map_err(|e| {
            AppError::ProtonDownloadFailed(format!("modules: cannot install {}: {}", rel, e))
        })?;
        if rel.ends_with(".so") {
            codesign(&to);
        }
    }
    Ok(())
}

/// Ad-hoc code signature, macOS only. Failure is logged, not fatal: the
/// module already carries the signature CI gave it.
fn codesign(path: &Path) {
    if !cfg!(target_os = "macos") {
        return;
    }
    match std::process::Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(path)
        .output()
    {
        Ok(o) if o.status.success() => {}
        Ok(o) => crate::logging::warn(format!(
            "codesign {}: {}",
            path.display(),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => crate::logging::warn(format!("codesign {}: {}", path.display(), e)),
    }
}

/// The `.app` bundle inside an unpacked Wine release.
pub fn find_wine_app(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.join(WINE_BIN_IN_APP).is_file())
}

/// The directory the DXMT tarball unpacks to (`v0.80/`, named after the tag),
/// located by content rather than name.
fn find_dxmt_payload(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.join("x86_64-unix/winemetal.so").is_file())
}

/// The directory the module archive unpacks to
/// (`endfield-wine-modules-<tag>-macos/`), located by content: the archive
/// root itself if the tarball has no top-level directory.
fn find_modules_payload(dir: &Path) -> Option<PathBuf> {
    let has_modules = |p: &Path| p.join("lib/wine/x86_64-unix/ntdll.so").is_file();
    if has_modules(dir) {
        return Some(dir.to_path_buf());
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| has_modules(p))
}

/// Every Wine the launcher installed under `base_dir`, most recently
/// installed first, each with the DXMT and module-set versions that went
/// into it so the picker can show what a build actually contains. Install
/// time rather than name: `11.9` sorts after `11.16` as a string, and what
/// auto-detection wants is "the one the user just downloaded".
pub fn list_installed(base_dir: &Path) -> Vec<InstalledProton> {
    let mut installed: Vec<(std::time::SystemTime, InstalledProton)> = std::fs::read_dir(base_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter_map(|dir| {
            let app = find_wine_app(&dir)?;
            let installed_at = std::fs::metadata(&dir)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            Some((
                installed_at,
                InstalledProton {
                    name: dir.file_name()?.to_string_lossy().to_string(),
                    path: app.to_string_lossy().to_string(),
                    dxmt: marker(&dir, DXMT_MARKER),
                    wine_patch: marker(&dir, MODULES_MARKER),
                },
            ))
        })
        .collect();
    installed.sort_by(|a, b| b.0.cmp(&a.0));
    installed.into_iter().map(|(_, w)| w).collect()
}

fn marker(install_dir: &Path, name: &str) -> Option<String> {
    let v = std::fs::read_to_string(install_dir.join(name)).ok()?;
    let v = v.trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// The DXMT version inside whatever bundle `wine_dir` points at — the bundle
/// itself or anything inside it — by walking up to the directory holding the
/// marker. Only the macOS system check and debug report ask; the rest of
/// this module is exercised through the shared commands on every platform.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn dxmt_version_for(wine_dir: &Path) -> Option<String> {
    wine_dir.ancestors().find_map(|d| marker(d, DXMT_MARKER))
}

/// Same for the Endfield module set. Empty means the anti-cheat will not
/// load, whatever else the bundle contains.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn modules_version_for(wine_dir: &Path) -> Option<String> {
    wine_dir.ancestors().find_map(|d| marker(d, MODULES_MARKER))
}

/// Rosetta 2 is present when the kernel can run an x86-64 binary. Cheap, and
/// the only check that does not depend on where Apple keeps Rosetta's files
/// this release. False anywhere that is not a Mac.
pub fn rosetta_available() -> bool {
    std::process::Command::new("arch")
        .args(["-x86_64", "/usr/bin/true"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Install Rosetta 2 through `softwareupdate`, elevated via AppleScript's
/// `with administrator privileges` — the standard macOS password prompt.
/// Cancelling that prompt fails the call; the message says what to run by
/// hand instead.
fn install_rosetta() -> Result<(), AppError> {
    let script = "do shell script \"/usr/sbin/softwareupdate --install-rosetta --agree-to-license\" \
                  with administrator privileges";
    let output = std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| AppError::Api(format!("cannot run osascript: {}", e)))?;
    if output.status.success() && rosetta_available() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(AppError::Api(format!(
        "Rosetta 2 could not be installed{}. Run `softwareupdate --install-rosetta` \
         in Terminal, then download again.",
        if stderr.is_empty() { String::new() } else { format!(": {}", stderr) }
    )))
}

/// True on an Apple silicon Mac, whichever slice of the launcher is running:
/// `hw.optional.arm64` exists (and is 1) only there. Anywhere else — Intel
/// Macs, and the other platforms this module is compiled on — false.
pub fn host_is_apple_silicon() -> bool {
    std::process::Command::new("sysctl")
        .args(["-n", "hw.optional.arm64"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_only_the_staging_build() {
        // Each release carries a wine-devel tarball beside the staging one.
        assert!(is_wine_staging_asset("wine-staging-11.16-osx64.tar.xz"));
        assert!(!is_wine_staging_asset("wine-devel-11.16-osx64.tar.xz"));
        assert!(!is_wine_staging_asset("wine-staging-11.16-osx64.tar.xz.sha256"));
    }

    #[test]
    fn module_set_names_follow_the_build_script() {
        // What macos/wine/build-modules.sh and wine-macos.yml produce.
        assert_eq!(modules_release_tag("11.16"), "wine-modules-11.16");
        assert_eq!(modules_asset_name("11.16"), "endfield-wine-modules-11.16-macos.tar.xz");
    }

    #[test]
    fn installed_list_names_what_went_into_each_build() {
        let tmp = std::env::temp_dir().join(format!("llauncher-wine-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let with = tmp.join("wine-staging-11.16");
        let bin = with.join("Wine Staging.app").join(WINE_BIN_IN_APP);
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, b"").unwrap();
        std::fs::write(with.join(DXMT_MARKER), "v0.80\n").unwrap();
        std::fs::write(with.join(MODULES_MARKER), "wine-modules-11.16\n").unwrap();
        // A directory without a bundle inside must not show up.
        std::fs::create_dir_all(tmp.join("dxmt")).unwrap();

        let list = list_installed(&tmp);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "wine-staging-11.16");
        assert_eq!(list[0].dxmt.as_deref(), Some("v0.80"));
        assert_eq!(list[0].wine_patch.as_deref(), Some("wine-modules-11.16"));
        assert!(list[0].path.ends_with("Wine Staging.app"));
        // Both markers are found from the bundle path the setting stores.
        let bundle = Path::new(&list[0].path);
        assert_eq!(dxmt_version_for(bundle).as_deref(), Some("v0.80"));
        assert_eq!(modules_version_for(bundle).as_deref(), Some("wine-modules-11.16"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn modules_go_over_the_bundle_originals() {
        let tmp = std::env::temp_dir().join(format!("llauncher-modules-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        // A module archive as build-modules.sh lays it out, under its
        // top-level directory.
        let payload_root = tmp.join("modules");
        let payload = payload_root.join("endfield-wine-modules-11.16-macos");
        for rel in MODULE_FILES {
            let f = payload.join("lib/wine").join(rel);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(&f, b"patched").unwrap();
        }
        assert_eq!(find_modules_payload(&payload_root).as_deref(), Some(payload.as_path()));
        // A bundle with stock modules in place.
        let app = tmp.join("Wine Staging.app");
        let lib = app.join(WINE_LIB_IN_APP);
        for rel in MODULE_FILES {
            let f = lib.join(rel);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(&f, b"stock").unwrap();
        }

        install_modules(&payload, &app).unwrap();
        for rel in MODULE_FILES {
            assert_eq!(std::fs::read(lib.join(rel)).unwrap(), b"patched");
        }
        assert_eq!(std::fs::read(lib.join("x86_64-unix/ntdll.so.orig")).unwrap(), b"stock");
        assert_eq!(std::fs::read(lib.join("x86_64-windows/ntoskrnl.exe.orig")).unwrap(), b"stock");
        // A second install keeps the first backup: the stock file, not the
        // previous patched one.
        std::fs::write(payload.join("lib/wine/x86_64-unix/ntdll.so"), b"patched2").unwrap();
        install_modules(&payload, &app).unwrap();
        assert_eq!(std::fs::read(lib.join("x86_64-unix/ntdll.so.orig")).unwrap(), b"stock");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
