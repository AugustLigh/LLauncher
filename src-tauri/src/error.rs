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
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
