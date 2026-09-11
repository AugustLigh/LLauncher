//! OptiScaler: DLSS for people without an NVIDIA card.
//!
//! Endfield's only real upscaler is DLSS — the game ships NVIDIA Streamline
//! with DLSS Super Resolution, Frame Generation and Ray Reconstruction, and
//! the "FSR" entry in its graphics menu is the engine's own TAAU under a
//! borrowed name. So on an AMD or Intel GPU the menu offers nothing worth
//! having. OptiScaler fixes that from the outside: it sits between the game
//! and `nvngx.dll`, takes the inputs the game prepares for DLSS (colour, depth,
//! motion vectors) and feeds them to FSR 3.1 or XeSS instead, spoofing an
//! NVIDIA GPU so the game unlocks the DLSS options in the first place. The
//! bundled fakenvapi and Nukem's dlssg-to-fsr3 do the same for Reflex and
//! frame generation.
//!
//! It loads as a proxy DLL next to the game executable, like 3DMigoto does
//! (see `crate::game::mods`), but it hooks the NGX loader rather than D3D11,
//! so it works on the game's native Vulkan renderer and costs no frames of
//! its own. The release is a single 7z archive with everything inside; the
//! launcher unpacks it, renames `OptiScaler.dll` to the proxy name and applies
//! the settings its wiki lists for this game and for Wine. Under Proton the
//! launch script then adds the `WINEDLLOVERRIDES` entry that makes Wine load
//! the proxy ahead of its builtin.
//!
//! Anti-cheat: OptiScaler's own README warns against online games. The
//! project's compatibility entry for Endfield reports that ACE triggers on
//! Proton-GE and Proton-CachyOS but not on DWProton, which is what this
//! launcher installs. The feature stays opt-in and the UI says so.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::AppError;

const REPO: &str = "optiscaler/OptiScaler";

/// The name OptiScaler.dll is installed under. One of the loader names the
/// project supports; `winmm.dll` is loaded by every Unity game, is not on
/// Windows' KnownDLLs list (so the game directory wins there), and leaves
/// `dxgi.dll` free for ReShade and `d3d11.dll` for 3DMigoto. The wiki lists
/// it among the names verified on this game.
pub const PROXY_DLL: &str = "winmm.dll";

/// The one file in the archive that gets renamed.
const OPTISCALER_DLL: &str = "OptiScaler.dll";

/// OptiScaler's configuration, read from next to the proxy.
const INI: &str = "OptiScaler.ini";

/// Where the launcher records what it installed, so an uninstall removes
/// exactly that and nothing the user added. First line is the release tag.
const MANIFEST: &str = "OptiScaler.llauncher-manifest";

/// Written by OptiScaler and its bundled helpers at run time.
const LOG_FILES: [&str; 3] = ["OptiScaler.log", "fakenvapi.log", "dlssg_to_fsr3.log"];

/// What a hand-made or manifest-less install may have left behind: the files
/// of the current release plus the directories OptiScaler creates itself.
const KNOWN_FILES: [&str; 17] = [
    "OptiScaler.ini",
    "fakenvapi.ini",
    "fakenvapi.dll",
    "amd_fidelityfx_dx12.dll",
    "amd_fidelityfx_framegeneration_dx12.dll",
    "amd_fidelityfx_upscaler_dx12.dll",
    "amd_fidelityfx_vk.dll",
    "dlssg_to_fsr3_amd_is_better.dll",
    "libxell.dll",
    "libxess.dll",
    "libxess_dx11.dll",
    "libxess_fg.dll",
    "nvngx.dll",
    "setup_linux.sh",
    "setup_windows.bat",
    "remove_optiscaler.sh",
    "!! README_EXTRACT ALL FILES TO GAME FOLDER !!.txt",
];
const KNOWN_DIRS: [&str; 4] = ["D3D12_Optiscaler", "Licenses", "DlssOverrides", "OptiScaler"];

/// What the launcher knows about OptiScaler in the game directory.
#[derive(Debug, Clone, Serialize)]
pub struct OptiScalerStatus {
    /// The proxy and its ini are both next to the game executable.
    pub installed: bool,
    /// Release tag of the launcher's own install, empty for a hand-made one.
    pub version: String,
    /// The game directory itself is missing, so nothing else here means much.
    pub game_dir_missing: bool,
}

/// Is OptiScaler installed under the launcher's proxy name? A hand-made
/// install under another name is left alone — the override would not reach
/// it anyway.
pub fn is_installed(game_dir: &Path) -> bool {
    game_dir.join(PROXY_DLL).is_file() && game_dir.join(INI).is_file()
}

pub fn status(game_dir: &Path) -> OptiScalerStatus {
    let version = std::fs::read_to_string(game_dir.join(MANIFEST))
        .ok()
        .and_then(|m| m.lines().next().map(|l| l.trim().to_string()))
        .unwrap_or_default();
    OptiScalerStatus {
        installed: is_installed(game_dir),
        version,
        game_dir_missing: !game_dir.is_dir(),
    }
}

/// How the install is tuned for the machine it lands on.
#[derive(Debug, Clone, Copy, Default)]
pub struct InstallOptions {
    /// The game runs under Wine. OptiScaler's Endfield entry says its
    /// default kernel32 hooks may miss the upscaler inputs there and to use
    /// the ntdll ones instead; on Windows the same switch costs stutter.
    pub wine: bool,
    /// No NVIDIA driver on the host and the game runs its Vulkan renderer,
    /// so the GPU has to be spoofed at the Vulkan level for the game to show
    /// the DLSS options at all. On the D3D11 path OptiScaler decides on its
    /// own from the DXGI adapter; Vulkan spoofing is off unless asked.
    pub spoof_vulkan: bool,
}

/// The tuning for the machine the launcher runs on.
pub fn host_options() -> InstallOptions {
    #[cfg(target_os = "linux")]
    {
        InstallOptions {
            wine: true,
            spoof_vulkan: !crate::game::proton::has_nvidia_driver(),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        InstallOptions::default()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallResult {
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

/// Download the latest OptiScaler release and unpack it into the game
/// directory, replacing whatever the launcher installed before.
pub async fn install(
    client: &reqwest::Client,
    game_dir: &Path,
    options: InstallOptions,
) -> Result<InstallResult, AppError> {
    if !game_dir.is_dir() {
        return Err(AppError::GameNotFound(format!(
            "Game directory does not exist: {}",
            game_dir.display()
        )));
    }

    let release: GhRelease = client
        .get(format!(
            "https://api.github.com/repos/{}/releases/latest",
            REPO
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
        .find(|a| a.name.to_lowercase().ends_with(".7z"))
        .ok_or_else(|| AppError::Api(format!("The latest {} release has no .7z asset", REPO)))?;

    let bytes = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "LLauncher")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec();

    let game_dir = game_dir.to_path_buf();
    let version = release.tag_name.clone();
    let tag = version.clone();
    let files = tokio::task::spawn_blocking(move || -> Result<usize, AppError> {
        // A previous install is cleared first so a file the new release no
        // longer ships cannot linger and load alongside the new set.
        remove_installed(&game_dir)?;
        let mut install = Install::new(&game_dir, &tag);
        unpack(&bytes, &mut install)?;
        install.finish(options)
    })
    .await
    .map_err(|e| AppError::Api(format!("OptiScaler install task failed: {}", e)))??;

    crate::logging::info(format!(
        "optiscaler: installed {} ({} files, wine={}, spoof_vulkan={})",
        version, files, options.wine, options.spoof_vulkan
    ));
    Ok(InstallResult { version, files })
}

/// Unpack the release archive, entry by entry, into an install in progress.
fn unpack(bytes: &[u8], install: &mut Install) -> Result<(), AppError> {
    let mut archive = sevenz_rust2::ArchiveReader::new(
        std::io::Cursor::new(bytes),
        sevenz_rust2::Password::empty(),
    )
    .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

    let mut failure: Option<AppError> = None;
    archive
        .for_each_entries(|entry, reader| {
            if entry.is_directory() {
                return Ok(true);
            }
            match install.write_entry(entry.name(), reader) {
                Ok(()) => Ok(true),
                Err(e) => {
                    failure = Some(e);
                    Ok(false)
                }
            }
        })
        .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

    match failure {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// An install being written: where it goes and what has landed so far.
struct Install {
    game_dir: PathBuf,
    tag: String,
    written: Vec<String>,
}

impl Install {
    fn new(game_dir: &Path, tag: &str) -> Self {
        Self {
            game_dir: game_dir.to_path_buf(),
            tag: tag.to_string(),
            written: Vec::new(),
        }
    }

    /// Write one archive entry where the game will find it. The setup
    /// scripts and the shouting README are the archive's own installer,
    /// whose job this is; they stay out. `OptiScaler.dll` lands under the
    /// proxy name. An existing ini is kept — OptiScaler writes the user's
    /// in-game choices into it — and only tuned afterwards.
    fn write_entry(&mut self, name: &str, data: &mut dyn Read) -> Result<(), AppError> {
        let name = name.replace('\\', "/");
        let name = name.trim_start_matches("./");
        if name.is_empty() {
            return Ok(());
        }
        // No absolute paths, no `..`: nothing may land outside the game
        // directory whatever the archive says.
        let path = Path::new(name);
        if path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Ok(());
        }
        if is_skipped(name) {
            return Ok(());
        }

        let rel = if name.eq_ignore_ascii_case(OPTISCALER_DLL) {
            PROXY_DLL.to_string()
        } else {
            name.to_string()
        };
        let target = self.game_dir.join(&rel);

        if rel.eq_ignore_ascii_case(INI) && target.is_file() {
            self.written.push(rel);
            return Ok(());
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&target)?;
        std::io::copy(data, &mut out)?;
        self.written.push(rel);
        Ok(())
    }

    /// Tune the ini for this machine and record what was installed.
    fn finish(self, options: InstallOptions) -> Result<usize, AppError> {
        if !self.game_dir.join(PROXY_DLL).is_file() {
            return Err(AppError::ExtractionFailed(format!(
                "The OptiScaler archive has no {}",
                OPTISCALER_DLL
            )));
        }
        let ini_path = self.game_dir.join(INI);
        if !ini_path.is_file() {
            return Err(AppError::ExtractionFailed(format!(
                "The OptiScaler archive has no {}",
                INI
            )));
        }
        let ini = std::fs::read_to_string(&ini_path)?;
        std::fs::write(&ini_path, tune_ini(&ini, options))?;

        let mut manifest = String::new();
        manifest.push_str(&self.tag);
        manifest.push('\n');
        for rel in &self.written {
            manifest.push_str(rel);
            manifest.push('\n');
        }
        std::fs::write(self.game_dir.join(MANIFEST), manifest)?;
        Ok(self.written.len())
    }
}

/// Archive entries that do not belong in the game directory.
fn is_skipped(rel: &str) -> bool {
    let lower = rel.to_lowercase();
    lower == "setup_linux.sh"
        || lower == "setup_windows.bat"
        || lower.starts_with("!!")
        || lower.ends_with(".txt") && !lower.contains('/')
}

/// The settings the launcher applies on top of OptiScaler's defaults.
///
/// * `UseNtdllHooks=false` under Wine — the Endfield compatibility entry's
///   own note, "mainly aimed at Linux".
/// * Vulkan-level GPU spoofing without an NVIDIA driver — the game only
///   shows DLSS to an NVIDIA card, and on Linux it runs its Vulkan renderer,
///   where OptiScaler's automatic DXGI spoofing never comes into play.
/// * FSR 3.1 as the Vulkan output — OptiScaler's own default there is the
///   older FSR 2.2. XeSS is the better pick on an Intel Arc; the in-game
///   overlay (Insert) switches between them at run time.
fn tune_ini(ini: &str, options: InstallOptions) -> String {
    let mut text = ini.to_string();
    if options.wine {
        text = set_ini_key(&text, "Hooks", "UseNtdllHooks", "false");
    }
    if options.spoof_vulkan {
        text = set_ini_key(&text, "Spoofing", "Vulkan", "true");
        text = set_ini_key(&text, "Spoofing", "VulkanExtensionSpoofing", "true");
    }
    text = set_ini_key(&text, "Upscalers", "VulkanUpscaler", "fsr31");
    text
}

/// Set `key=value` inside `[section]`, replacing the key if the section has
/// it and appending it to the section otherwise. Line endings are kept as
/// found (OptiScaler ships CRLF); comments and order are untouched.
fn set_ini_key(ini: &str, section: &str, key: &str, value: &str) -> String {
    let crlf = ini.contains("\r\n");
    let eol = if crlf { "\r\n" } else { "\n" };
    let header = format!("[{}]", section);
    let mut lines: Vec<String> = ini
        .split('\n')
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();
    // `split` leaves an empty trailing element when the text ends with a
    // newline; keep track so the file ends the way it started.
    let trailing_newline = lines.last().map(|l| l.is_empty()).unwrap_or(false);
    if trailing_newline {
        lines.pop();
    }

    let mut in_section = false;
    let mut section_end: Option<usize> = None;
    let mut replaced = false;
    for i in 0..lines.len() {
        let line = lines[i].trim();
        if line.starts_with('[') {
            if in_section {
                section_end = Some(i);
                break;
            }
            in_section = line.eq_ignore_ascii_case(&header);
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((k, _)) = line.split_once('=') else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case(key) {
            lines[i] = format!("{}={}", key, value);
            replaced = true;
            break;
        }
    }

    if !replaced {
        let entry = format!("{}={}", key, value);
        match section_end {
            // Slot it in before the blank lines that pad the next header.
            Some(end) => {
                let mut at = end;
                while at > 0 && lines[at - 1].trim().is_empty() {
                    at -= 1;
                }
                lines.insert(at, entry);
            }
            None if in_section => lines.push(entry),
            None => {
                if !lines.is_empty() {
                    lines.push(String::new());
                }
                lines.push(header);
                lines.push(entry);
            }
        }
    }

    let mut out = lines.join(eol);
    if trailing_newline {
        out.push_str(eol);
    }
    out
}

/// Remove what the launcher installed, following the manifest when there is
/// one and the known file list otherwise, plus the logs OptiScaler writes.
pub fn uninstall(game_dir: &Path) -> Result<(), AppError> {
    remove_installed(game_dir)?;
    crate::logging::info("optiscaler: removed");
    Ok(())
}

fn remove_installed(game_dir: &Path) -> Result<(), AppError> {
    let manifest_path = game_dir.join(MANIFEST);
    let mut files: Vec<String> = match std::fs::read_to_string(&manifest_path) {
        Ok(m) => m.lines().skip(1).map(|l| l.trim().to_string()).collect(),
        Err(_) => KNOWN_FILES.iter().map(|f| f.to_string()).collect(),
    };
    files.push(PROXY_DLL.to_string());
    files.extend(LOG_FILES.iter().map(|f| f.to_string()));

    let mut dirs: Vec<PathBuf> = Vec::new();
    for rel in files.iter().filter(|f| !f.is_empty()) {
        let path = game_dir.join(rel);
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
        if let Some(parent) = path.parent() {
            if parent != game_dir {
                dirs.push(parent.to_path_buf());
            }
        }
    }
    // OptiScaler's own directories go wholesale: nothing of the user's lives
    // there. Directories from the manifest go only once empty.
    for dir in KNOWN_DIRS {
        let path = game_dir.join(dir);
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        }
    }
    dirs.sort();
    dirs.dedup();
    for dir in dirs.iter().rev() {
        if dir.is_dir() {
            let _ = std::fs::remove_dir(dir);
        }
    }
    if manifest_path.is_file() {
        std::fs::remove_file(&manifest_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "llauncher-optiscaler-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const SAMPLE_INI: &str = "[Upscalers]\r\n; Vulkan\r\nVulkanUpscaler=auto\r\n\r\n\r\n[Spoofing]\r\nDxgi=auto\r\nVulkan=auto\r\n\r\n[Hooks]\r\nUseNtdllHooks=auto\r\n";

    #[test]
    fn replaces_a_key_in_its_own_section_only() {
        let out = set_ini_key(SAMPLE_INI, "Spoofing", "Vulkan", "true");
        assert!(out.contains("[Spoofing]\r\nDxgi=auto\r\nVulkan=true\r\n"));
        // The Upscalers key with a similar prefix is untouched.
        assert!(out.contains("VulkanUpscaler=auto"));
        assert!(out.ends_with("\r\n"));
    }

    #[test]
    fn appends_a_missing_key_before_the_next_section() {
        let out = set_ini_key(SAMPLE_INI, "Spoofing", "VulkanExtensionSpoofing", "true");
        assert!(out.contains(
            "Vulkan=auto\r\nVulkanExtensionSpoofing=true\r\n\r\n[Hooks]"
        ));
    }

    #[test]
    fn appends_to_the_last_section_and_creates_a_missing_one() {
        let out = set_ini_key(SAMPLE_INI, "Hooks", "EarlyHooking", "false");
        assert!(out.ends_with("UseNtdllHooks=auto\r\nEarlyHooking=false\r\n"));
        let out = set_ini_key("[A]\nx=1\n", "B", "y", "2");
        assert_eq!(out, "[A]\nx=1\n\n[B]\ny=2\n");
    }

    #[test]
    fn tunes_for_wine_without_nvidia() {
        let out = tune_ini(
            SAMPLE_INI,
            InstallOptions {
                wine: true,
                spoof_vulkan: true,
            },
        );
        assert!(out.contains("UseNtdllHooks=false"));
        assert!(out.contains("Vulkan=true"));
        assert!(out.contains("VulkanExtensionSpoofing=true"));
        assert!(out.contains("VulkanUpscaler=fsr31"));
    }

    #[test]
    fn leaves_windows_hooks_and_spoofing_alone() {
        let out = tune_ini(SAMPLE_INI, InstallOptions::default());
        assert!(out.contains("UseNtdllHooks=auto"));
        assert!(out.contains("Vulkan=auto"));
        assert!(out.contains("VulkanUpscaler=fsr31"));
    }

    fn entries(install: &mut Install, entries: &[(&str, &[u8])]) {
        for (name, data) in entries {
            let mut cursor: &[u8] = data;
            install.write_entry(name, &mut cursor).unwrap();
        }
    }

    #[test]
    fn installs_under_the_proxy_name_and_skips_the_installer() {
        let dir = tempdir();
        let mut install = Install::new(&dir, "v0.9.4");
        entries(
            &mut install,
            &[
                ("OptiScaler.dll", b"proxy"),
                ("OptiScaler.ini", SAMPLE_INI.as_bytes()),
                ("setup_linux.sh", b"#!/bin/sh"),
                ("setup_windows.bat", b"@echo off"),
                ("!! README_EXTRACT ALL FILES TO GAME FOLDER !!.txt", b"read me"),
                ("D3D12_Optiscaler\\D3D12Core.dll", b"core"),
                ("..\\escape.dll", b"nope"),
            ],
        );
        let files = install
            .finish(InstallOptions {
                wine: true,
                spoof_vulkan: false,
            })
            .unwrap();

        assert_eq!(files, 3);
        assert!(dir.join(PROXY_DLL).is_file());
        assert!(!dir.join(OPTISCALER_DLL).exists());
        assert!(!dir.join("setup_linux.sh").exists());
        assert!(!dir.join("setup_windows.bat").exists());
        assert!(dir.join("D3D12_Optiscaler/D3D12Core.dll").is_file());
        assert!(!dir.parent().unwrap().join("escape.dll").exists());
        assert!(std::fs::read_to_string(dir.join(INI))
            .unwrap()
            .contains("UseNtdllHooks=false"));
        let s = status(&dir);
        assert!(s.installed);
        assert_eq!(s.version, "v0.9.4");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn keeps_the_users_ini_but_still_tunes_it() {
        let dir = tempdir();
        std::fs::write(dir.join(INI), "[Menu]\nScale=1.5\n[Hooks]\nUseNtdllHooks=auto\n").unwrap();
        let mut install = Install::new(&dir, "v0.9.4");
        entries(
            &mut install,
            &[
                ("OptiScaler.dll", b"proxy"),
                ("OptiScaler.ini", SAMPLE_INI.as_bytes()),
            ],
        );
        install
            .finish(InstallOptions {
                wine: true,
                spoof_vulkan: false,
            })
            .unwrap();
        let ini = std::fs::read_to_string(dir.join(INI)).unwrap();
        assert!(ini.contains("Scale=1.5"));
        assert!(ini.contains("UseNtdllHooks=false"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn uninstall_follows_the_manifest_and_spares_the_rest() {
        let dir = tempdir();
        let mut install = Install::new(&dir, "v0.9.4");
        entries(
            &mut install,
            &[
                ("OptiScaler.dll", b"proxy"),
                ("OptiScaler.ini", SAMPLE_INI.as_bytes()),
                ("Licenses/XeSS_LICENSE.txt", b"license"),
                ("libxess.dll", b"xess"),
            ],
        );
        install.finish(InstallOptions::default()).unwrap();
        std::fs::write(dir.join("OptiScaler.log"), b"log").unwrap();
        std::fs::write(dir.join("d3d11.dll"), b"3dmigoto").unwrap();
        std::fs::create_dir_all(dir.join("Mods/Skin")).unwrap();

        uninstall(&dir).unwrap();

        assert!(!dir.join(PROXY_DLL).exists());
        assert!(!dir.join(INI).exists());
        assert!(!dir.join("libxess.dll").exists());
        assert!(!dir.join("Licenses").exists());
        assert!(!dir.join("OptiScaler.log").exists());
        assert!(!dir.join(MANIFEST).exists());
        assert!(dir.join("d3d11.dll").is_file());
        assert!(dir.join("Mods/Skin").is_dir());
        assert!(!status(&dir).installed);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn uninstall_without_a_manifest_uses_the_known_list() {
        let dir = tempdir();
        std::fs::write(dir.join(PROXY_DLL), b"proxy").unwrap();
        std::fs::write(dir.join(INI), b"[Hooks]\n").unwrap();
        std::fs::write(dir.join("fakenvapi.dll"), b"nvapi").unwrap();
        std::fs::create_dir_all(dir.join("D3D12_Optiscaler")).unwrap();

        uninstall(&dir).unwrap();

        assert!(!dir.join(PROXY_DLL).exists());
        assert!(!dir.join("fakenvapi.dll").exists());
        assert!(!dir.join("D3D12_Optiscaler").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn flags_a_missing_game_dir() {
        let s = status(Path::new("/nonexistent/llauncher/game/dir"));
        assert!(s.game_dir_missing);
        assert!(!s.installed);
    }
}
