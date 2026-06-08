use md5::{Digest, Md5};
use std::io::Read;
use std::path::Path;

use crate::error::AppError;

pub fn verify_md5_with_progress<F>(
    path: &Path,
    expected: &str,
    mut on_read: F,
) -> Result<bool, AppError>
where
    F: FnMut(u64) -> Result<(), AppError>,
{
    let mut hasher = Md5::new();
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 8 * 1024 * 1024];

    loop {
        let n = file.read(&mut buf).map_err(AppError::Io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        on_read(n as u64)?;
    }

    let result = format!("{:x}", hasher.finalize());
    Ok(result == expected)
}
