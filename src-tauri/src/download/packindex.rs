//! Read the split-pack ZIP central directory to decide whether an update can
//! be served by the cheap per-file VFS delta, or needs the full pack download.
//!
//! The resource manifest only covers `Endfield_Data/StreamingAssets/VFS/*`
//! (~98% of the bytes). The remaining ~1.4 GB — the engine, `Endfield.exe`,
//! Managed DLLs, anti-cheat, CefView — only exist in the packs. If any of those
//! non-VFS files changed (CRC32 mismatch vs the latest packs), the VFS delta
//! alone would leave the game's executable code stale, so we must fall back to
//! the full pack download. When they all match, the engine is current and the
//! VFS delta is a complete, much cheaper update.

use std::path::Path;
use tauri::Emitter;

use crate::api::types::PackFile;
use crate::error::AppError;

const VFS_PREFIX: &str = "Endfield_Data/StreamingAssets/VFS";

#[derive(Debug, Clone)]
pub struct CdEntry {
    pub name: String,
    pub crc32: u32,
}

fn read_u32(b: &[u8], off: usize) -> Option<u32> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}
fn read_u64(b: &[u8], off: usize) -> Option<u64> {
    b.get(off..off + 8)
        .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
}
fn read_u16(b: &[u8], off: usize) -> Option<u16> {
    b.get(off..off + 2)
        .map(|s| u16::from_le_bytes(s.try_into().unwrap()))
}

async fn range(client: &reqwest::Client, url: &str, start: u64, end_inclusive: u64) -> Result<Vec<u8>, AppError> {
    let resp = client
        .get(url)
        .header(reqwest::header::RANGE, format!("bytes={}-{}", start, end_inclusive))
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.bytes().await?.to_vec())
}

/// Fetch and parse the full central directory of the split-pack ZIP.
///
/// `total_size` is the sum of every pack's size; the last pack URL/size locate
/// the trailing records. Returns every entry, or an error if the directory
/// cannot be parsed in full (caller then conservatively falls back to packs).
pub async fn fetch_central_directory(
    client: &reqwest::Client,
    packs: &[PackFile],
) -> Result<Vec<CdEntry>, AppError> {
    let last = packs
        .last()
        .ok_or_else(|| AppError::Api("no packs".into()))?;
    let last_size: u64 = last.package_size.parse().unwrap_or(0);
    let total_size: u64 = packs
        .iter()
        .map(|p| p.package_size.parse::<u64>().unwrap_or(0))
        .sum();
    if last_size == 0 || total_size == 0 {
        return Err(AppError::Api("pack sizes unavailable".into()));
    }

    // 1. Read the tail of the last part to find the Zip64 EOCD record.
    let tail_len = last_size.min(64 * 1024);
    let tail = range(client, &last.url, last_size - tail_len, last_size - 1).await?;
    let z64 = tail
        .windows(4)
        .rposition(|w| w == [0x50, 0x4b, 0x06, 0x06])
        .ok_or_else(|| AppError::Api("Zip64 EOCD not found".into()))?;
    let total_entries = read_u64(&tail, z64 + 32).ok_or_else(|| AppError::Api("bad EOCD".into()))?;
    let cd_offset = read_u64(&tail, z64 + 48).ok_or_else(|| AppError::Api("bad EOCD".into()))?;

    // 2. The CD lives at the global offset `cd_offset`; it is small and sits in
    //    the last part. Fetch from there to the end of the part.
    if cd_offset < total_size - last_size {
        return Err(AppError::Api("central directory spans parts".into()));
    }
    let cd_local = cd_offset - (total_size - last_size);
    let cd_bytes = range(client, &last.url, cd_local, last_size - 1).await?;

    // 3. Walk the central-directory file headers (PK\x01\x02).
    let mut entries = Vec::with_capacity(total_entries as usize);
    let mut i = 0usize;
    while i + 46 <= cd_bytes.len() && read_u32(&cd_bytes, i) == Some(0x0201_4b50) {
        let crc32 = read_u32(&cd_bytes, i + 16).unwrap_or(0);
        let nlen = read_u16(&cd_bytes, i + 28).unwrap_or(0) as usize;
        let elen = read_u16(&cd_bytes, i + 30).unwrap_or(0) as usize;
        let clen = read_u16(&cd_bytes, i + 32).unwrap_or(0) as usize;
        let name_bytes = cd_bytes.get(i + 46..i + 46 + nlen).unwrap_or(&[]);
        let name = String::from_utf8_lossy(name_bytes).into_owned();

        entries.push(CdEntry { name, crc32 });
        i += 46 + nlen + elen + clen;
    }

    if (entries.len() as u64) != total_entries {
        return Err(AppError::Api(format!(
            "central directory incomplete: parsed {} of {}",
            entries.len(),
            total_entries
        )));
    }
    Ok(entries)
}

fn file_crc32(path: &Path) -> std::io::Result<u32> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = crc32fast::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize())
}

/// True when every non-VFS file (engine/code/anti-cheat/...) on disk matches the
/// latest packs — i.e. the executable game is already current and only the VFS
/// assets may need updating. A single missing or changed non-VFS file means the
/// engine changed and the caller must do a full pack update instead.
pub fn engine_is_current(entries: &[CdEntry], game_dir: &str) -> bool {
    let root = Path::new(game_dir);
    for e in entries {
        if e.name.ends_with('/') || e.name.starts_with(VFS_PREFIX) {
            continue; // directories and VFS assets are handled by the delta
        }
        let local = root.join(&e.name);
        match file_crc32(&local) {
            Ok(crc) if crc == e.crc32 => {}
            _ => return false,
        }
    }
    true
}

pub fn emit_checking(app: &tauri::AppHandle) {
    app.emit(
        "update://progress",
        crate::download::resources::IntegrityProgress {
            stage: "checking".to_string(),
            files_done: 0,
            total_files: 0,
            bytes_done: 0,
            bytes_total: 0,
            speed_bps: 0,
        },
    )
    .ok();
}
