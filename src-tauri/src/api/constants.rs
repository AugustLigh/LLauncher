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
