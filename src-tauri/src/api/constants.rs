use std::time::Duration;

pub const LAUNCHER_API_BASE: &str = "https://launcher.gryphline.com/api";
pub const BATCH_PROXY_URL: &str = "https://launcher.gryphline.com/api/proxy/batch_proxy";
pub const WEB_BATCH_PROXY_URL: &str = "https://launcher.gryphline.com/api/proxy/web/batch_proxy";

/// XOR-subtract key used to decrypt the per-file resource index manifests
/// (`index_main.json` / `index_initial.json`). Recovered by reversing the
/// official launcher.
pub const RESOURCE_INDEX_KEY: &[u8] = b"Assets/Beyond/DynamicAssets/Gameplay/UI/Fonts/";

/// VFS game assets live under this subdirectory of the game install. The
/// resource index `name` fields are relative to it (they start with `VFS/`).
pub const STREAMING_ASSETS_SUBDIR: &str = "Endfield_Data/StreamingAssets";

pub const GAME_APPCODE: &str = "YDUTE5gscDZ229CW";

pub const CHANNEL: &str = "6";
pub const SUB_CHANNEL: &str = "9999";

/// Timeout for small metadata requests (game version check, launcher content,
/// resource index, Proton release listings, ZIP central-directory reads).
/// These should always return quickly; if they don't, the server or network
/// is unreachable and we should fail fast instead of hanging the UI forever.
pub const API_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Large file downloads have no total time limit — a multi-GB transfer can
/// legitimately take a long time — but must not go this long without the
/// server even starting to respond before we give up on it.
pub const DOWNLOAD_STALL_TIMEOUT: Duration = Duration::from_secs(30);
