use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Arc;

use crate::config::settings::AppSettings;

pub struct AppState {
    pub settings: tokio::sync::Mutex<AppSettings>,
    pub http_client: reqwest::Client,
    pub download_active: Arc<AtomicBool>,
    pub proton_download_active: Arc<AtomicBool>,
    pub game_running: Arc<AtomicBool>,
    /// PID of the spawned game process group leader, if running; 0 = none.
    /// A plain atomic instead of `Mutex<Option<u32>>`: real PIDs are always
    /// non-zero, so 0 as a sentinel avoids a lock (and the possibility of it
    /// getting poisoned) for what is just a single-word get/set/clear.
    pub game_pid: Arc<AtomicU32>,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings: tokio::sync::Mutex::new(settings),
            http_client: reqwest::Client::builder()
                .tcp_nodelay(true)
                .pool_max_idle_per_host(10)
                // Bounds only the TCP/TLS handshake, so it's safe to apply to
                // every request including large downloads — a connect that
                // hasn't succeeded within 10s is not going to.
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            download_active: Arc::new(AtomicBool::new(false)),
            proton_download_active: Arc::new(AtomicBool::new(false)),
            game_running: Arc::new(AtomicBool::new(false)),
            game_pid: Arc::new(AtomicU32::new(0)),
        }
    }
}
