use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::Emitter;
use zip::ZipArchive;

use crate::api::types::ExtractProgress;
use crate::error::AppError;

pub fn extract_split_zip(
    app: &tauri::AppHandle,
    parts: &[PathBuf],
    extract_to: &Path,
    total_size: u64,
) -> Result<(), AppError> {
    std::fs::create_dir_all(extract_to)?;

    let reader = MultiFileReader::new(parts).map_err(AppError::Io)?;
    let mut archive =
        ZipArchive::new(reader).map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

    // Sum uncompressed entry sizes for accurate progress denominator.
    let mut bytes_total = 0u64;
    for i in 0..archive.len() {
        if let Ok(f) = archive.by_index_raw(i) {
            bytes_total += f.size();
        }
    }
    if bytes_total == 0 {
        bytes_total = total_size;
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

    let start = Instant::now();
    let mut bytes_written = 0u64;
    let mut last_emit = Instant::now();
    let mut buf = vec![0u8; 256 * 1024];

    let emit = |bytes_written: u64, bytes_total: u64| {
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
    };

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

        let outpath = match entry.enclosed_name() {
            Some(p) => extract_to.join(p),
            None => continue,
        };

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            loop {
                let n = entry.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                outfile.write_all(&buf[..n])?;
                bytes_written += n as u64;

                if last_emit.elapsed().as_millis() >= 500 {
                    emit(bytes_written, bytes_total);
                    last_emit = Instant::now();
                }
            }
        }
    }

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
