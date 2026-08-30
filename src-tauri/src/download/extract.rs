use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri::Emitter;
use zip::ZipArchive;

use crate::api::types::ExtractProgress;
use crate::error::AppError;

/// Upper bound on extraction worker threads, matching the cap already used
/// for download concurrency elsewhere (`download_max_concurrent.clamp(1, 8)`)
/// so this doesn't open an unreasonable number of file handles on unusual
/// (very high core count) hosts.
const MAX_EXTRACT_WORKERS: usize = 8;

pub fn extract_split_zip(
    app: &tauri::AppHandle,
    parts: &[PathBuf],
    extract_to: &Path,
    total_size: u64,
) -> Result<(), AppError> {
    std::fs::create_dir_all(extract_to)?;

    // Metadata pass: one reader is enough to read the central directory and
    // every entry's size up front. Not reused for the actual extraction below.
    let reader = MultiFileReader::new(parts).map_err(AppError::Io)?;
    let mut archive =
        ZipArchive::new(reader).map_err(|e| AppError::ExtractionFailed(e.to_string()))?;
    let entry_count = archive.len();

    // Sum uncompressed entry sizes for accurate progress denominator, and the
    // sizes of already-extracted targets (overwritten in place, so they do not
    // need free space again — important for repairs over an existing install).
    let mut bytes_total = 0u64;
    let mut existing_bytes = 0u64;
    for i in 0..entry_count {
        if let Ok(f) = archive.by_index_raw(i) {
            bytes_total += f.size();
            if let Some(p) = f.enclosed_name() {
                if let Ok(meta) = std::fs::metadata(extract_to.join(p)) {
                    existing_bytes += meta.len();
                }
            }
        }
    }
    if bytes_total == 0 {
        bytes_total = total_size;
    }
    drop(archive);

    let needed = bytes_total.saturating_sub(existing_bytes);
    if let Some(available) = crate::config::paths::available_space(extract_to) {
        if available < needed {
            return Err(AppError::DiskSpace {
                path: extract_to.to_string_lossy().to_string(),
                needed_mib: needed / (1024 * 1024),
                available_mib: available / (1024 * 1024),
            });
        }
    }

    app.emit(
        "download://extract-progress",
        ExtractProgress {
            percent: 0,
            bytes_processed: 0,
            bytes_total,
            speed_bps: 0,
        },
    )
    .ok();

    // Each entry has its own offset recorded in the central directory, so
    // entries can be read in any order — split them across a small pool of
    // worker threads, each with its own independent reader over the same
    // split files, instead of decompressing everything on one core.
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(MAX_EXTRACT_WORKERS)
        .min(entry_count.max(1));

    let start = Instant::now();
    let last_emit = Mutex::new(Instant::now());
    extract_parallel(parts, extract_to, entry_count, worker_count, |total_written| {
        if let Ok(mut le) = last_emit.try_lock() {
            if le.elapsed().as_millis() >= 500 {
                emit_progress(app, total_written, bytes_total, start);
                *le = Instant::now();
            }
        }
    })?;

    app.emit(
        "download://extract-progress",
        ExtractProgress {
            percent: 100,
            bytes_processed: bytes_total,
            bytes_total,
            speed_bps: 0,
        },
    )
    .ok();

    Ok(())
}

/// Extract every entry `0..entry_count` of the split archive at `parts`
/// across `worker_count` threads, each with its own independent reader (zip
/// entries carry their own offset in the central directory, so they can be
/// read in any order / from any number of concurrent readers over the same
/// underlying files). `on_progress` is called with the cumulative bytes
/// written so far after every chunk, from whichever worker thread wrote it.
/// Returns the first error encountered across all workers, if any.
fn extract_parallel(
    parts: &[PathBuf],
    extract_to: &Path,
    entry_count: usize,
    worker_count: usize,
    on_progress: impl Fn(u64) + Sync,
) -> Result<(), AppError> {
    let bytes_written = AtomicU64::new(0);
    let first_error: Mutex<Option<AppError>> = Mutex::new(None);

    std::thread::scope(|scope| {
        for worker_id in 0..worker_count {
            let bytes_written = &bytes_written;
            let first_error = &first_error;
            let on_progress = &on_progress;

            scope.spawn(move || {
                let reader = match MultiFileReader::new(parts) {
                    Ok(r) => r,
                    Err(e) => {
                        first_error.lock().unwrap().get_or_insert(AppError::Io(e));
                        return;
                    }
                };
                let mut archive = match ZipArchive::new(reader) {
                    Ok(a) => a,
                    Err(e) => {
                        first_error
                            .lock()
                            .unwrap()
                            .get_or_insert(AppError::ExtractionFailed(e.to_string()));
                        return;
                    }
                };
                let mut buf = vec![0u8; 256 * 1024];

                // Interleaved assignment (0, worker_count, 2*worker_count, ...)
                // spreads large and small entries across workers reasonably
                // evenly, since consecutive central-directory entries vary a
                // lot in size (engine files next to tiny VFS assets).
                for i in (worker_id..entry_count).step_by(worker_count) {
                    if first_error.lock().unwrap().is_some() {
                        return;
                    }

                    if let Err(e) = extract_one(&mut archive, i, extract_to, &mut buf, |n| {
                        let total_written = bytes_written.fetch_add(n, Ordering::Relaxed) + n;
                        on_progress(total_written);
                    }) {
                        first_error.lock().unwrap().get_or_insert(e);
                        return;
                    }
                }
            });
        }
    });

    if let Some(e) = first_error.lock().unwrap().take() {
        return Err(e);
    }
    Ok(())
}

/// Extract a single entry by index, calling `on_write` after each chunk is
/// flushed to disk (byte count of that chunk) so the caller can track overall
/// progress across every worker.
fn extract_one<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
    extract_to: &Path,
    buf: &mut [u8],
    mut on_write: impl FnMut(u64),
) -> Result<(), AppError> {
    let mut entry = archive
        .by_index(index)
        .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

    let outpath = match entry.enclosed_name() {
        Some(p) => extract_to.join(p),
        None => return Ok(()),
    };

    if entry.is_dir() {
        std::fs::create_dir_all(&outpath)?;
        return Ok(());
    }

    if let Some(parent) = outpath.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut outfile = std::fs::File::create(&outpath)?;
    loop {
        let n = entry.read(buf)?;
        if n == 0 {
            break;
        }
        outfile.write_all(&buf[..n])?;
        on_write(n as u64);
    }
    Ok(())
}

fn emit_progress(app: &tauri::AppHandle, bytes_written: u64, bytes_total: u64, start: Instant) {
    let elapsed = start.elapsed().as_secs_f64();
    let speed = if elapsed > 0.1 {
        (bytes_written as f64 / elapsed) as u64
    } else {
        0
    };
    let percent = if bytes_total > 0 {
        ((bytes_written as f64 / bytes_total as f64) * 100.0).min(99.0) as u8
    } else {
        0
    };
    app.emit(
        "download://extract-progress",
        ExtractProgress {
            percent,
            bytes_processed: bytes_written,
            bytes_total,
            speed_bps: speed,
        },
    )
    .ok();
}

/// Chains multiple files into a single `Read + Seek` stream.
/// Presents split ZIP parts as one contiguous archive to the ZIP reader.
struct MultiFileReader {
    parts: Vec<(PathBuf, u64)>,
    total_size: u64,
    current_pos: u64,
    current_part_idx: usize,
    current_file: Option<std::fs::File>,
}

impl MultiFileReader {
    fn new(paths: &[PathBuf]) -> std::io::Result<Self> {
        let mut parts = Vec::with_capacity(paths.len());
        let mut total_size = 0u64;
        for path in paths {
            let size = std::fs::metadata(path)?.len();
            parts.push((path.clone(), size));
            total_size += size;
        }
        let current_file = parts
            .first()
            .map(|(p, _)| std::fs::File::open(p))
            .transpose()?;
        Ok(MultiFileReader {
            parts,
            total_size,
            current_pos: 0,
            current_part_idx: 0,
            current_file,
        })
    }

    fn open_part(&mut self, idx: usize, offset: u64) -> std::io::Result<()> {
        if idx >= self.parts.len() {
            self.current_file = None;
            self.current_part_idx = idx;
            return Ok(());
        }
        let mut file = std::fs::File::open(&self.parts[idx].0)?;
        if offset > 0 {
            file.seek(std::io::SeekFrom::Start(offset))?;
        }
        self.current_file = Some(file);
        self.current_part_idx = idx;
        Ok(())
    }
}

impl Read for MultiFileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.current_file.is_none() {
                return Ok(0);
            }
            let n = self.current_file.as_mut().unwrap().read(buf)?;
            if n > 0 {
                self.current_pos += n as u64;
                return Ok(n);
            }
            // End of this part — advance to next
            let next = self.current_part_idx + 1;
            self.open_part(next, 0)?;
        }
    }
}

impl Seek for MultiFileReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            std::io::SeekFrom::Start(n) => n,
            std::io::SeekFrom::End(n) => {
                if n >= 0 {
                    self.total_size.saturating_add(n as u64)
                } else {
                    self.total_size.saturating_sub(n.unsigned_abs())
                }
            }
            std::io::SeekFrom::Current(n) => {
                if n >= 0 {
                    self.current_pos.saturating_add(n as u64)
                } else {
                    self.current_pos.saturating_sub(n.unsigned_abs())
                }
            }
        };

        // Find the target part and offset within it
        let mut remaining = new_pos;
        let mut target_part = self.parts.len(); // sentinel: past end
        let mut offset_in_part = 0u64;
        for (idx, (_, size)) in self.parts.iter().enumerate() {
            if remaining <= *size {
                target_part = idx;
                offset_in_part = remaining;
                break;
            }
            remaining -= size;
        }

        if target_part != self.current_part_idx || self.current_file.is_none() {
            self.open_part(target_part, offset_in_part)?;
        } else if let Some(file) = &mut self.current_file {
            file.seek(std::io::SeekFrom::Start(offset_in_part))?;
        }

        self.current_pos = new_pos;
        Ok(new_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a real (uncompressed, for simplicity) zip archive on disk.
    fn build_test_archive(path: &Path, entries: &[(String, Vec<u8>)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            writer.start_file(name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    /// Split a file into `n` roughly-equal part files at arbitrary byte
    /// offsets — mirroring how the CDN serves the game's split-pack archives
    /// (split by byte size, not at zip entry boundaries).
    fn split_into_parts(path: &Path, n: usize) -> Vec<PathBuf> {
        let data = std::fs::read(path).unwrap();
        let chunk_size = data.len().div_ceil(n).max(1);
        data.chunks(chunk_size)
            .enumerate()
            .map(|(i, chunk)| {
                let part_path = path.with_file_name(format!("part{i}"));
                std::fs::write(&part_path, chunk).unwrap();
                part_path
            })
            .collect()
    }

    #[test]
    fn extracts_every_entry_correctly_across_worker_threads_and_split_parts() {
        // The whole point of parallelizing extraction is running multiple
        // independent readers over the same split archive concurrently — if
        // that has any race (on the shared source files, on output paths, on
        // directory creation), some entries would come out missing or
        // corrupted. Every one of 37 entries, spread across 4 threads and a
        // 5-way-split archive, must land byte-for-byte intact.
        let tmp = std::env::temp_dir().join(format!("llauncher_extract_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        let entries: Vec<(String, Vec<u8>)> = (0..37u32)
            .map(|i| {
                (
                    format!("VFS/dir{}/file{i}.bin", i % 5),
                    vec![i as u8; 1000 + i as usize * 37],
                )
            })
            .collect();
        let archive_path = tmp.join("test.zip");
        build_test_archive(&archive_path, &entries);
        let parts = split_into_parts(&archive_path, 5);

        let extract_to = tmp.join("out");
        std::fs::create_dir_all(&extract_to).unwrap();

        let reader = MultiFileReader::new(&parts).unwrap();
        let archive = ZipArchive::new(reader).unwrap();
        let entry_count = archive.len();
        drop(archive);
        assert_eq!(entry_count, entries.len());

        extract_parallel(&parts, &extract_to, entry_count, 4, |_| {}).unwrap();

        for (name, data) in &entries {
            let written = std::fs::read(extract_to.join(name))
                .unwrap_or_else(|e| panic!("missing extracted file {name}: {e}"));
            assert_eq!(&written, data, "content mismatch for {name}");
        }

        std::fs::remove_dir_all(&tmp).ok();
    }
}
