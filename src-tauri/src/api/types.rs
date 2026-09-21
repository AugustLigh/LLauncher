use serde::{Deserialize, Serialize};

// ─── Batch proxy request/response ───

#[derive(Debug, Serialize, specta::Type)]
pub struct BatchProxyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<String>,
    pub proxy_reqs: Vec<ProxyReq>,
}

#[derive(Debug, Serialize, specta::Type)]
pub struct ProxyReq {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_latest_game_req: Option<GetLatestGameReq>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_sidebar_req: Option<ContentReq>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_single_ent_req: Option<ContentReq>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_main_bg_image_req: Option<ContentReq>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_banner_req: Option<ContentReq>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_announcement_req: Option<ContentReq>,
}

#[derive(Debug, Serialize, specta::Type)]
pub struct GetLatestGameReq {
    pub version: String,
    pub appcode: String,
    pub channel: String,
    pub sub_channel: String,
    pub device_id: String,
}

#[derive(Debug, Serialize, specta::Type)]
pub struct ContentReq {
    pub appcode: String,
    pub language: String,
    pub channel: String,
    pub sub_channel: String,
    pub platform: String,
    pub source: String,
}

// ─── Batch proxy response ───

#[derive(Debug, Deserialize, specta::Type)]
pub struct BatchProxyResponse {
    #[specta(type = Vec<std::collections::HashMap<String, String>>)]
    pub proxy_rsps: Vec<serde_json::Value>,
}

// ─── Game version response ───

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameVersionResponse {
    pub version: String,
    #[serde(default)]
    pub request_version: String,
    pub action: i32,
    pub pkg: PackageInfo,
    #[serde(default)]
    #[specta(type = Option<std::collections::HashMap<String, String>>)]
    pub patch: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PackageInfo {
    pub packs: Vec<PackFile>,
    pub total_size: String,
    #[serde(default)]
    pub game_files_md5: String,
    #[serde(default)]
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PackFile {
    pub url: String,
    pub md5: String,
    pub package_size: String,
}

// ─── Launcher content ───

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct LauncherContent {
    #[serde(default)]
    pub background: BackgroundImage,
    #[serde(default)]
    pub banners: Vec<Banner>,
    #[serde(default)]
    pub news_tabs: Vec<NewsTab>,
    #[serde(default)]
    pub sidebars: Vec<Sidebar>,
    #[serde(default)]
    pub single_ent: Option<SingleEnt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct BackgroundImage {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub video_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Banner {
    pub url: String,
    #[serde(default)]
    pub jump_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct NewsTab {
    #[serde(rename = "tabName")]
    pub tab_name: String,
    #[serde(default)]
    pub announcements: Vec<Announcement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Announcement {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub jump_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Sidebar {
    #[serde(default)]
    pub media: String,
    #[serde(default)]
    pub jump_url: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub sidebar_labels: Vec<SidebarLabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SidebarLabel {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub jump_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SingleEnt {
    #[serde(default)]
    pub version_url: String,
    #[serde(default)]
    pub button_url: String,
    #[serde(default)]
    pub button_hover_url: String,
    #[serde(default)]
    pub jump_url: String,
}

// ─── Proton download types ───

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ProtonReleaseInfo {
    pub tag_name: String,
    pub download_url: String,
    pub file_name: String,
    #[specta(type = f64)]
    pub size: u64,
    #[serde(default)]
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct InstalledProton {
    pub name: String,
    pub path: String,
    /// macOS only: the DXMT version installed into this Wine, shown as a
    /// badge in the picker. Absent on Linux, where the layer is Proton.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dxmt: Option<String>,
    /// macOS only: the Endfield module set (`wine-modules-<version>`) inside
    /// this Wine — what gets the game past its anti-cheat. Absent on Linux.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wine_patch: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProtonDownloadProgress {
    #[specta(type = f64)]
    pub bytes_downloaded: u64,
    #[specta(type = f64)]
    pub bytes_total: u64,
    #[specta(type = f64)]
    pub speed_bps: u64,
    pub stage: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProtonDownloadComplete {
    pub proton_dir: String,
    pub version: String,
}

// ─── Download progress events ───

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DownloadProgress {
    #[specta(type = f64)]
    pub file_index: usize,
    #[specta(type = f64)]
    pub total_files: usize,
    pub file_name: String,
    #[specta(type = f64)]
    pub bytes_downloaded: u64,
    #[specta(type = f64)]
    pub bytes_total: u64,
    #[specta(type = f64)]
    pub speed_bps: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DownloadFileComplete {
    #[specta(type = f64)]
    pub file_index: usize,
    #[specta(type = f64)]
    pub total_files: usize,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ExtractProgress {
    pub percent: u8,
    #[specta(type = f64)]
    pub bytes_processed: u64,
    #[specta(type = f64)]
    pub bytes_total: u64,
    #[specta(type = f64)]
    pub speed_bps: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DownloadComplete {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DownloadError {
    pub message: String,
}

// ─── Launch events ───

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LaunchFailed {
    pub exit_code: Option<i32>,
    pub log_tail: String,
    /// Known-failure signature id (see `game::diagnose`) the frontend maps to
    /// actionable advice; None when the log matched nothing we recognize.
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct GameExited {
    pub exit_code: Option<i32>,
}

/// A launch was refused because the server has a newer game version. Emitted
/// so a tray / `--play` launch, which has no UI of its own, can still tell
/// the user what to do.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct UpdateRequired {
    pub installed_version: String,
    pub latest_version: String,
}

