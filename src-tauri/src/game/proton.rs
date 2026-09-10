//! What the host can offer the game: the compatibility layer on Linux, and
//! the optional wrappers (gamemode, MangoHud, gamescope) the launch script
//! can put in front of it. macOS has a compatibility layer too — the Wine
//! build the launcher installs, with the Endfield modules and DXMT inside —
//! but none of the wrappers. On Windows none of this applies: the game runs
//! natively, so the check only reports the platform and the UI drops every
//! Proton-shaped control.

use serde::Serialize;

use crate::config::settings::AppSettings;

#[derive(Debug, Clone, Serialize)]
pub struct SystemCheck {
    /// `"linux"`, `"macos"` or `"windows"`. The frontend keys every
    /// Proton/Wine-only section off this rather than guessing from the other
    /// flags.
    pub platform: &'static str,
    /// A compatibility layer is ready: Proton on Linux, some Wine build on
    /// macOS. Always true on Windows, where none is needed.
    pub has_proton: bool,
    pub has_ntsync: bool,
    pub has_gamemode: bool,
    pub has_mangohud: bool,
    pub has_gamescope: bool,
    /// The vkBasalt Vulkan layer is installed on the host. Unlike the
    /// wrappers above it is not a command but a layer manifest, so it is
    /// looked up by file rather than by `which`.
    pub has_vkbasalt: bool,
    pub proton_path: String,
    /// macOS on Apple silicon: Rosetta 2 is installed, so the x86-64 Wine
    /// (and the game) can run at all. True everywhere else — Intel Macs run
    /// x86-64 natively and the other platforms have no such thing.
    pub has_rosetta: bool,
    /// macOS: the DXMT version inside the active Wine, empty if none. What
    /// tells the UI whether D3D11 goes to Metal or to the OpenGL fallback.
    pub dxmt_version: String,
    /// macOS: the Endfield module set inside the active Wine, empty if none —
    /// in which case the anti-cheat will not load and the UI says so.
    pub wine_patch_version: String,
}

#[cfg(target_os = "linux")]
pub fn check_system(settings: &AppSettings) -> SystemCheck {
    use std::path::Path;

    let proton_dir = settings.proton_dir.as_str();
    let has_proton = if proton_dir.is_empty() {
        false
    } else {
        Path::new(proton_dir).join("proton").exists()
    };

    SystemCheck {
        platform: "linux",
        has_proton,
        has_ntsync: check_ntsync(),
        has_gamemode: check_command("gamemoderun"),
        has_mangohud: check_command("mangohud"),
        has_gamescope: check_command("gamescope"),
        has_vkbasalt: check_vulkan_layer("vkBasalt"),
        proton_path: if has_proton {
            Path::new(proton_dir)
                .join("proton")
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        },
        has_rosetta: true,
        dxmt_version: String::new(),
        wine_patch_version: String::new(),
    }
}

/// macOS: the only thing that can be missing is Wine itself, which the
/// launcher installs the way it installs DWProton on Linux — so the check
/// reports whether one was found and where, and what went into it.
/// `has_ntsync` means "nothing is missing" here: there is no host-side
/// synchronisation primitive to look for on Darwin.
#[cfg(target_os = "macos")]
pub fn check_system(settings: &AppSettings) -> SystemCheck {
    let wine = crate::game::launcher::resolve_wine(settings);
    let proton_path = wine
        .map(|w| w.wine.to_string_lossy().to_string())
        .unwrap_or_default();
    let wine_path = std::path::Path::new(&proton_path);
    let dxmt_version = crate::download::wine::dxmt_version_for(wine_path).unwrap_or_default();
    let wine_patch_version =
        crate::download::wine::modules_version_for(wine_path).unwrap_or_default();

    SystemCheck {
        platform: "macos",
        has_proton: !proton_path.is_empty(),
        has_ntsync: true,
        has_gamemode: false,
        has_mangohud: false,
        has_gamescope: false,
        has_vkbasalt: false,
        proton_path,
        has_rosetta: crate::download::wine::rosetta_available(),
        dxmt_version,
        wine_patch_version,
    }
}

/// Nothing to check for on Windows: the game is a native binary and needs no
/// compatibility layer. `has_proton`/`has_ntsync` report *true* — they mean
/// "nothing is missing" to the UI, so a frontend that somehow still reads them
/// shows no bogus "Proton not found" warning.
#[cfg(windows)]
pub fn check_system(_settings: &AppSettings) -> SystemCheck {
    SystemCheck {
        platform: "windows",
        has_proton: true,
        has_ntsync: true,
        has_gamemode: false,
        has_mangohud: false,
        has_gamescope: false,
        has_vkbasalt: false,
        proton_path: String::new(),
        has_rosetta: true,
        dxmt_version: String::new(),
        wine_patch_version: String::new(),
    }
}

#[cfg(target_os = "linux")]
fn check_ntsync() -> bool {
    std::path::Path::new("/dev/ntsync").exists()
}

/// Look for an implicit Vulkan layer manifest by name across the search paths
/// the loader itself uses, system and per-user.
#[cfg(target_os = "linux")]
fn check_vulkan_layer(name: &str) -> bool {
    let file = format!("{}.json", name);
    let mut roots = vec![
        std::path::PathBuf::from("/usr/share/vulkan"),
        std::path::PathBuf::from("/usr/local/share/vulkan"),
        std::path::PathBuf::from("/etc/vulkan"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(std::path::Path::new(&home).join(".local/share/vulkan"));
    }
    roots.iter().any(|root| {
        ["implicit_layer.d", "explicit_layer.d"]
            .iter()
            .any(|dir| root.join(dir).join(&file).exists())
    })
}

#[cfg(target_os = "linux")]
fn check_command(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
