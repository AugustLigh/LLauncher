//! Starting the game the way GRYPHLINK does.
//!
//! Since 1.6 the game refuses to run unless the official launcher started
//! it: at startup it looks up its parent process and checks the image's name
//! and Authenticode signature (`Add launch check:Launcher` / `Not found
//! matched name` in `sdklogs/OneChannelLC.log`). When the check fails it
//! tries to start GRYPHLINK itself and gives up. Under Proton the parent is
//! `steam.exe`, natively it is this launcher — so issue #48.
//!
//! GRYPHLINK's starter is `Games.exe`. `gryphlink-start.exe` (source in
//! `gryphlink-start/`) starts it suspended and creates the game with it as
//! the parent, so the check sees a signed `Games.exe` while no line of
//! GRYPHLINK ever runs. `Games.exe` is all it needs: the launcher fetches
//! that one entry out of GRYPHLINK's update package with range requests
//! (~14 MB of the ~118 MB zip), checks its CRC and keeps it per version.
//! Nothing is installed into the prefix, and no GRYPHLINK window shows up.

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::api::constants::{API_REQUEST_TIMEOUT, LAUNCHER_API_BASE};
use crate::download::packindex::{range, read_u16, read_u32};
use crate::error::AppError;

/// GRYPHLINK's own app code (the game's is `GAME_APPCODE`).
const LAUNCHER_APPCODE: &str = "TiaytKBUIEdoEwRT";
const LAUNCHER_CHANNEL: &str = "6";
const LAUNCHER_SUB_CHANNEL: &str = "6";

const STARTER_EXE: &str = "Games.exe";

const SHIM_EXE: &str = "gryphlink-start.exe";
static SHIM_EXE_BYTES: &[u8] = include_bytes!("../../gryphlink-start/gryphlink-start.exe");

/// What the platform launchers put in front of `Endfield.exe`:
/// `<shim> <starter> <Endfield.exe> [arguments...]`.
pub struct Starter {
    pub shim: PathBuf,
    pub starter: PathBuf,
}

fn root_dir() -> PathBuf {
    crate::config::paths::data_base().join("gryphlink")
}

/// Make sure a `Games.exe` is on disk and the shim next to it.
///
/// Asks for GRYPHLINK's current version and fetches its `Games.exe` when that
/// version is not here yet. Offline, or with the API down, whatever version
/// is already on disk is good enough — the game checks the signature, not
/// how recent the starter is.
pub async fn ensure_starter(client: &reqwest::Client) -> Result<Starter, AppError> {
    let root = root_dir();
    let starter = match fetch_latest(client, &root).await {
        Ok(path) => path,
        Err(e) => match newest_cached(&root) {
            Some(path) => {
                crate::logging::warn(format!(
                    "gryphlink: could not check for a newer Games.exe ({}), using {}",
                    e,
                    path.display()
                ));
                path
            }
            None => return Err(e),
        },
    };
    let shim = root.join(SHIM_EXE);
    crate::util::write_atomic(&shim, SHIM_EXE_BYTES)?;
    Ok(Starter { shim, starter })
}

async fn fetch_latest(client: &reqwest::Client, root: &Path) -> Result<PathBuf, AppError> {
    let info: serde_json::Value = client
        .get(format!("{}/launcher/get_latest", LAUNCHER_API_BASE))
        .query(&[
            ("appcode", LAUNCHER_APPCODE),
            ("channel", LAUNCHER_CHANNEL),
            ("sub_channel", LAUNCHER_SUB_CHANNEL),
            ("target_app", "Official"),
        ])
        .timeout(API_REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let version = info["version"]
        .as_str()
        .filter(|v| is_plain_version(v))
        .ok_or_else(|| AppError::Api("GRYPHLINK: no version in the response".into()))?;
    let url = info["zip_package_url"]
        .as_str()
        .ok_or_else(|| AppError::Api("GRYPHLINK: no package URL in the response".into()))?;
    let size: u64 = info["package_size"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| AppError::Api("GRYPHLINK: no package size in the response".into()))?;

    let path = root.join(version).join(STARTER_EXE);
    if path.is_file() {
        return Ok(path);
    }

    crate::logging::info(format!("gryphlink: fetching Games.exe from GRYPHLINK {}", version));
    let bytes = fetch_zip_entry(client, url, size, STARTER_EXE).await?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    crate::util::write_atomic(&path, &bytes)?;

    // Older versions are dead weight once the new one is in place.
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir() && entry.file_name() != version {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(path)
}

/// A version string fit to be a directory name: digits and dots.
fn is_plain_version(v: &str) -> bool {
    !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.') && !v.contains("..")
}

/// The `Games.exe` of the highest version already on disk.
fn newest_cached(root: &Path) -> Option<PathBuf> {
    let version_key = |name: &str| -> Vec<u64> {
        name.split('.').map(|p| p.parse().unwrap_or(0)).collect()
    };
    std::fs::read_dir(root)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let exe = e.path().join(STARTER_EXE);
            (is_plain_version(&name) && exe.is_file()).then(|| (version_key(&name), exe))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, exe)| exe)
}

/// Pull one entry out of a remote (non-Zip64) archive: the end-of-central-
/// directory record, the central directory, then just that entry's bytes.
async fn fetch_zip_entry(
    client: &reqwest::Client,
    url: &str,
    size: u64,
    name: &str,
) -> Result<Vec<u8>, AppError> {
    let bad = |what: &str| AppError::Api(format!("GRYPHLINK package: {}", what));

    let tail_len = size.min(64 * 1024);
    let tail = range(client, url, size - tail_len, size - 1).await?;
    let eocd = tail
        .windows(4)
        .rposition(|w| w == [0x50, 0x4b, 0x05, 0x06])
        .ok_or_else(|| bad("end of central directory not found"))?;
    let cd_size = read_u32(&tail, eocd + 12).ok_or_else(|| bad("truncated EOCD"))? as u64;
    let cd_offset = read_u32(&tail, eocd + 16).ok_or_else(|| bad("truncated EOCD"))? as u64;
    if cd_size == 0 || cd_offset + cd_size > size {
        return Err(bad("central directory out of range"));
    }
    let cd = range(client, url, cd_offset, cd_offset + cd_size - 1).await?;

    let mut i = 0usize;
    let entry = loop {
        if i + 46 > cd.len() || read_u32(&cd, i) != Some(0x0201_4b50) {
            return Err(bad(&format!("{} not found", name)));
        }
        let nlen = read_u16(&cd, i + 28).unwrap_or(0) as usize;
        let elen = read_u16(&cd, i + 30).unwrap_or(0) as usize;
        let clen = read_u16(&cd, i + 32).unwrap_or(0) as usize;
        if cd.get(i + 46..i + 46 + nlen) == Some(name.as_bytes()) {
            break i;
        }
        i += 46 + nlen + elen + clen;
    };
    let method = read_u16(&cd, entry + 10).unwrap_or(0);
    let crc = read_u32(&cd, entry + 16).unwrap_or(0);
    let packed = read_u32(&cd, entry + 20).unwrap_or(0) as u64;
    let unpacked = read_u32(&cd, entry + 24).unwrap_or(0) as u64;
    let local = read_u32(&cd, entry + 42).unwrap_or(0) as u64;
    if packed == 0 || packed == u32::MAX as u64 || method != 8 && method != 0 {
        return Err(bad("unsupported entry"));
    }

    let header = range(client, url, local, local + 29).await?;
    if read_u32(&header, 0) != Some(0x0403_4b50) {
        return Err(bad("bad local header"));
    }
    let data_start = local
        + 30
        + read_u16(&header, 26).unwrap_or(0) as u64
        + read_u16(&header, 28).unwrap_or(0) as u64;
    let data = range(client, url, data_start, data_start + packed - 1).await?;

    let bytes = if method == 8 {
        let mut out = Vec::with_capacity(unpacked as usize);
        flate2::read::DeflateDecoder::new(&data[..])
            .read_to_end(&mut out)
            .map_err(|e| bad(&format!("inflate failed: {}", e)))?;
        out
    } else {
        data
    };
    if bytes.len() as u64 != unpacked || crc32fast::hash(&bytes) != crc {
        return Err(bad("checksum mismatch"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_versions_become_directories() {
        assert!(is_plain_version("1.6.0"));
        assert!(!is_plain_version(""));
        assert!(!is_plain_version("../1.6.0"));
        assert!(!is_plain_version("1.6.0/x"));
    }

    #[test]
    fn picks_the_newest_cached_starter() {
        let root = std::env::temp_dir().join(format!("llauncher-gryphlink-{}", std::process::id()));
        for v in ["1.9.0", "1.10.0", "junk"] {
            std::fs::create_dir_all(root.join(v)).unwrap();
            std::fs::write(root.join(v).join(STARTER_EXE), b"").unwrap();
        }
        std::fs::create_dir_all(root.join("1.11.0")).unwrap();
        assert_eq!(
            newest_cached(&root),
            Some(root.join("1.10.0").join(STARTER_EXE))
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
