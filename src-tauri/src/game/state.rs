use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::error::AppError;

pub fn incomplete_marker(game_dir: &Path) -> PathBuf {
    game_dir.join(".llauncher_incomplete")
}

/// Whether `game_dir` holds a playable install we did not necessarily put
/// there: the game binary is present and no download of ours was interrupted
/// mid-way. Used to adopt a folder the user pointed the launcher at (or one
/// installed by another launcher) without a separate "import" step.
pub fn has_existing_install(game_dir: &Path) -> bool {
    game_dir.join("Endfield.exe").exists() && !incomplete_marker(game_dir).exists()
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status")]
pub enum GameState {
    #[serde(rename = "not_installed")]
    NotInstalled { latest_version: String },
    #[serde(rename = "update_available")]
    UpdateAvailable {
        installed_version: String,
        latest_version: String,
    },
    #[serde(rename = "ready")]
    Ready { version: String },
}

pub async fn determine_game_state(
    client: &reqwest::Client,
    game_dir: &str,
    installed_version: &str,
) -> Result<GameState, AppError> {
    let game_path = Path::new(game_dir);
    let exe_path = game_path.join("Endfield.exe");

    // Get latest version from API
    let version_info =
        crate::api::client::get_latest_game_version(client, installed_version).await?;
    let latest_version = version_info.version.clone();

    if incomplete_marker(game_path).exists() {
        return Ok(GameState::NotInstalled { latest_version });
    }

    // Check if game is installed
    if installed_version.is_empty() || !exe_path.exists() {
        return Ok(GameState::NotInstalled { latest_version });
    }

    // An update is only available when the server actually returns packs to
    // apply. When the install is current the pack list is empty, so we treat it
    // as ready even if the reported version string differs from ours — this
    // avoids the launcher getting stuck endlessly offering an "Update" that
    // re-downloads and re-extracts nothing new.
    if installed_version != latest_version && !version_info.pkg.packs.is_empty() {
        return Ok(GameState::UpdateAvailable {
            installed_version: installed_version.to_string(),
            latest_version,
        });
    }

    Ok(GameState::Ready {
        version: installed_version.to_string(),
    })
}
