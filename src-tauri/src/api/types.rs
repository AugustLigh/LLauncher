use serde::{Deserialize, Serialize};

// ─── Batch proxy request/response ───

#[derive(Debug, Serialize)]
pub struct BatchProxyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<String>,
    pub proxy_reqs: Vec<ProxyReq>,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct GetLatestGameReq {
    pub version: String,
    pub appcode: String,
    pub channel: String,
    pub sub_channel: String,
    pub device_id: String,
}

#[derive(Debug, Serialize)]
pub struct ContentReq {
    pub appcode: String,
    pub language: String,
    pub channel: String,
    pub sub_channel: String,
    pub platform: String,
    pub source: String,
}

// ─── Batch proxy response ───

#[derive(Debug, Deserialize)]
pub struct BatchProxyResponse {
    pub proxy_rsps: Vec<serde_json::Value>,
}

// ─── Game version response ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersionResponse {
    pub version: String,
    #[serde(default)]
    pub request_version: String,
    pub action: i32,
    pub pkg: PackageInfo,
    #[serde(default)]
    pub patch: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub packs: Vec<PackFile>,
    pub total_size: String,
    #[serde(default)]
    pub game_files_md5: String,
    #[serde(default)]
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackFile {
    pub url: String,
    pub md5: String,
    pub package_size: String,
}

// ─── Launcher content ───

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackgroundImage {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub video_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Banner {
    pub url: String,
    #[serde(default)]
    pub jump_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsTab {
    #[serde(rename = "tabName")]
    pub tab_name: String,
    #[serde(default)]
    pub announcements: Vec<Announcement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub jump_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarLabel {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub jump_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonReleaseInfo {
    pub tag_name: String,
    pub download_url: String,
    pub file_name: String,
    pub size: u64,
    #[serde(default)]
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledProton {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProtonDownloadProgress {
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub speed_bps: u64,
    pub stage: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProtonDownloadComplete {
    pub proton_dir: String,
    pub version: String,
}

// ─── Download progress events ───

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub file_index: usize,
    pub total_files: usize,
    pub file_name: String,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub speed_bps: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadFileComplete {
    pub file_index: usize,
    pub total_files: usize,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractProgress {
    pub percent: u8,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub speed_bps: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadComplete {
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadError {
    pub message: String,
}

// ─── Launch events ───

#[derive(Debug, Clone, Serialize)]
pub struct LaunchFailed {
    pub exit_code: Option<i32>,
    pub log_tail: String,
    /// Known-failure signature id (see `game::diagnose`) the frontend maps to
    /// actionable advice; None when the log matched nothing we recognize.
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameExited {
    pub exit_code: Option<i32>,
}
