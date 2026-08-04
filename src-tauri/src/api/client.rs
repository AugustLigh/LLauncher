use reqwest::Client;

use super::constants::*;
use super::types::*;
use crate::error::AppError;

pub async fn get_latest_game_version(
    client: &Client,
    current_version: &str,
) -> Result<GameVersionResponse, AppError> {
    let payload = BatchProxyRequest {
        seq: Some("1".to_string()),
        proxy_reqs: vec![ProxyReq {
            kind: "get_latest_game".to_string(),
            get_latest_game_req: Some(GetLatestGameReq {
                version: current_version.to_string(),
                appcode: GAME_APPCODE.to_string(),
                channel: CHANNEL.to_string(),
                sub_channel: SUB_CHANNEL.to_string(),
                device_id: "llauncher".to_string(),
            }),
            get_sidebar_req: None,
            get_single_ent_req: None,
            get_main_bg_image_req: None,
            get_banner_req: None,
            get_announcement_req: None,
        }],
    };

    let resp: BatchProxyResponse = client
        .post(BATCH_PROXY_URL)
        .json(&payload)
        .timeout(API_REQUEST_TIMEOUT)
        .send()
        .await?
        .json()
        .await?;

    let rsp = resp
        .proxy_rsps
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Api("Empty response".to_string()))?;

    let game_rsp = rsp
        .get("get_latest_game_rsp")
        .ok_or_else(|| AppError::Api("Missing get_latest_game_rsp".to_string()))?;

    let version_response: GameVersionResponse = serde_json::from_value(game_rsp.clone())?;
    Ok(version_response)
}

pub async fn get_launcher_content(
    client: &Client,
    language: &str,
) -> Result<LauncherContent, AppError> {
    let content_req = |kind: &str| -> ProxyReq {
        let req = ContentReq {
            appcode: GAME_APPCODE.to_string(),
            language: language.to_string(),
            channel: CHANNEL.to_string(),
            sub_channel: SUB_CHANNEL.to_string(),
            platform: "Windows".to_string(),
            source: "launcher".to_string(),
        };
        let mut proxy = ProxyReq {
            kind: kind.to_string(),
            get_latest_game_req: None,
            get_sidebar_req: None,
            get_single_ent_req: None,
            get_main_bg_image_req: None,
            get_banner_req: None,
            get_announcement_req: None,
        };
        match kind {
            "get_sidebar" => proxy.get_sidebar_req = Some(req),
            "get_single_ent" => proxy.get_single_ent_req = Some(req),
            "get_main_bg_image" => proxy.get_main_bg_image_req = Some(req),
            "get_banner" => proxy.get_banner_req = Some(req),
            "get_announcement" => proxy.get_announcement_req = Some(req),
            _ => {}
        }
        proxy
    };

    let payload = BatchProxyRequest {
        seq: None,
        proxy_reqs: vec![
            content_req("get_sidebar"),
            content_req("get_single_ent"),
            content_req("get_main_bg_image"),
            content_req("get_banner"),
            content_req("get_announcement"),
        ],
    };

    let resp: BatchProxyResponse = client
        .post(WEB_BATCH_PROXY_URL)
        .json(&payload)
        .timeout(API_REQUEST_TIMEOUT)
        .send()
        .await?
        .json()
        .await?;

    let mut content = LauncherContent::default();

    for rsp in resp.proxy_rsps {
        let kind = rsp.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let rsp_key = format!("{}_rsp", kind);

        if let Some(data) = rsp.get(&rsp_key) {
            match kind {
                "get_main_bg_image" => {
                    if let Some(bg) = data.get("main_bg_image") {
                        if let Ok(parsed) = serde_json::from_value::<BackgroundImage>(bg.clone()) {
                            content.background = parsed;
                        }
                    }
                }
                "get_banner" => {
                    if let Some(banners) = data.get("banners") {
                        if let Ok(parsed) = serde_json::from_value::<Vec<Banner>>(banners.clone()) {
                            content.banners = parsed;
                        }
                    }
                }
                "get_announcement" => {
                    if let Some(tabs) = data.get("tabs") {
                        if let Ok(parsed) = serde_json::from_value::<Vec<NewsTab>>(tabs.clone()) {
                            content.news_tabs = parsed;
                        }
                    }
                }
                "get_sidebar" => {
                    if let Some(sidebars) = data.get("sidebars") {
                        if let Ok(parsed) = serde_json::from_value::<Vec<Sidebar>>(sidebars.clone())
                        {
                            content.sidebars = parsed;
                        }
                    }
                }
                "get_single_ent" => {
                    if let Some(ent) = data.get("single_ent") {
                        if let Ok(parsed) = serde_json::from_value::<SingleEnt>(ent.clone()) {
                            content.single_ent = Some(parsed);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(content)
}
