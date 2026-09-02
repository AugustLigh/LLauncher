use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("MD5 mismatch: expected {expected}, got {actual}")]
    Md5Mismatch { expected: String, actual: String },

    #[error("Game not found: {0}")]
    GameNotFound(String),

    #[error("Proton not found: {0}")]
    ProtonNotFound(String),

    #[error("tar not found")]
    TarNotFound,

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Proton download failed: {0}")]
    ProtonDownloadFailed(String),

    #[error("Download cancelled")]
    Cancelled,

    #[error("Game update required (installed {installed}, latest {latest})")]
    UpdateRequired { installed: String, latest: String },

    #[error("Not enough free disk space in {path}: {needed_mib} MiB required, {available_mib} MiB available")]
    DiskSpace {
        path: String,
        needed_mib: u64,
        available_mib: u64,
    },
}

impl Serialize for AppError {
    // Tauri serialises a command's `Err` to hand it to the webview — the one
    // choke point every backend failure passes through, so record it in the
    // launcher log here. User-initiated cancellation is not a failure.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if !matches!(self, AppError::Cancelled) {
            crate::logging::warn(format!("command failed: {}", self));
        }
        serializer.serialize_str(&self.to_string())
    }
}
