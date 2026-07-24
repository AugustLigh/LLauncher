use serde::{Deserialize, Serialize};

use super::paths;

/// One completed play session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    /// Unix timestamp (seconds) when the session started.
    pub start: u64,
    pub duration_secs: u64,
}

/// Cap the on-disk journal so it never grows unbounded.
const MAX_SESSIONS: usize = 1000;

pub fn load() -> Vec<GameSession> {
    let path = paths::sessions_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Append a finished session to the journal. Sub-minute sessions are still
/// recorded (failed launches are filtered out by the caller's quick-exit path
/// living elsewhere; a real 30-second session is a session).
pub fn append(session: GameSession) {
    let mut sessions = load();
    sessions.push(session);
    if sessions.len() > MAX_SESSIONS {
        let excess = sessions.len() - MAX_SESSIONS;
        sessions.drain(..excess);
    }
    let path = paths::sessions_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(content) = serde_json::to_string(&sessions) {
        let _ = std::fs::write(&path, content);
    }
}
