use tauri::{Emitter, State};
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn get_game_version(
    state: State<'_, AppState>,
) -> Result<crate::api::types::GameVersionResponse, AppError> {
    let settings = state.settings.lock().await;
    let version = settings.installed_version.clone();
    drop(settings);
    crate::api::client::get_latest_game_version(&state.http_client, &version).await
}

#[tauri::command]
#[specta::specta]
pub async fn check_game_state(
    state: State<'_, AppState>,
) -> Result<crate::game::state::GameState, AppError> {
    let settings = state.settings.lock().await;
    let game_dir = settings.game_dir.clone();
    let mut installed_version = settings.installed_version.clone();
    drop(settings);

    // A game folder we have no version on record for — the user pointed the
    // launcher at an existing install in Settings, or the config was wiped —
    // is adopted on the spot, exactly as the "import existing game" link does,
    // instead of being offered a fresh install.
    if installed_version.is_empty()
        && crate::game::state::has_existing_install(std::path::Path::new(&game_dir))
    {
        let version_info =
            crate::api::client::get_latest_game_version(&state.http_client, "").await?;
        let mut settings = state.settings.lock().await;
        settings.installed_version = version_info.version.clone();
        settings.save_async().await?;
        installed_version = version_info.version;
    }

    crate::game::state::determine_game_state(&state.http_client, &game_dir, &installed_version)
        .await
}

/// Shared launch path used by the `launch_game` command and the tray menu.
///
/// Watches the spawned process until it exits. A quick exit (< 3.5s) is
/// reported as a launch failure with a tail of the log; in every case we reap
/// the child (no zombie), clear the running flag, record playtime and emit
/// `game://exited` so the UI can update / bring the window back.
pub async fn launch_and_watch(app: tauri::AppHandle, with_mods: bool) -> Result<(), AppError> {
    use tauri::Manager;

    let state = app.state::<AppState>();
    if state.game_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(AppError::Api("Game is already running".to_string()));
    }

    let settings_clone = state.settings.lock().await.clone();

    // The home-screen button only launches when the state is "ready", but the
    // tray menu and `--play` skip that check: the out-of-date game then
    // connects, shows its own "update required" dialog and closes, which looks
    // like a broken launch. Ask the server first and refuse when an update is
    // pending. A failed check (offline, API down) must not block playing.
    if let Ok(crate::game::state::GameState::UpdateAvailable {
        installed_version,
        latest_version,
    }) = crate::game::state::determine_game_state(
        &state.http_client,
        &settings_clone.game_dir,
        &settings_clone.installed_version,
    )
    .await
    {
        crate::logging::info(format!(
            "launch refused: update available ({} -> {})",
            installed_version, latest_version
        ));
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
        let _ = app.emit(
            "launch://update-required",
            crate::api::types::UpdateRequired {
                installed_version: installed_version.clone(),
                latest_version: latest_version.clone(),
            },
        );
        return Err(AppError::UpdateRequired {
            installed: installed_version,
            latest: latest_version,
        });
    }

    crate::logging::info(format!(
        "launching game (proton={}, wayland={}, gamescope={}, mods={})",
        settings_clone.proton_dir,
        settings_clone.use_wayland,
        settings_clone.use_gamescope,
        with_mods
    ));
    // The mod loader must only be visible to a modded launch — left in place,
    // it breaks the normal one (issues #34, #39). See mods::prepare_launch.
    if let Err(e) =
        crate::game::mods::prepare_launch(std::path::Path::new(&settings_clone.game_dir), with_mods)
    {
        if with_mods {
            return Err(e.into());
        }
        crate::logging::warn(format!("mods: could not park the loader: {}", e));
    }
    let mut launched = crate::game::launcher::launch_game(&settings_clone, with_mods)?;
    let game_running = state.game_running.clone();
    let game_pid = state.game_pid.clone();
    game_running.store(true, std::sync::atomic::Ordering::SeqCst);
    game_pid.store(launched.process.id(), std::sync::atomic::Ordering::SeqCst);
    let _ = app.emit("game://started", ());

    let discord = if settings_clone.use_discord_rpc {
        Some(crate::game::discord::start_presence())
    } else {
        None
    };

    let log_path = launched.log_path.clone();
    let proton_dir = settings_clone.proton_dir.clone();
    let app2 = app.clone();
    tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let session_start_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let status = loop {
            match launched.process.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(500)),
                Err(_) => break None,
            }
        };

        game_running.store(false, std::sync::atomic::Ordering::SeqCst);
        game_pid.store(0, std::sync::atomic::Ordering::SeqCst);
        if let Some(discord) = discord {
            discord.store(false, std::sync::atomic::Ordering::SeqCst);
        }

        // Undo what the platform changed on the host for the session
        // (Windows: the power plan) before anything else.
        if let Some(on_exit) = launched.on_exit.take() {
            on_exit();
        }

        let quick_exit =
            status.is_some() && started.elapsed() < std::time::Duration::from_millis(3500);
        crate::logging::info(format!(
            "game exited after {}s (code {:?}){}",
            started.elapsed().as_secs(),
            status.and_then(|s| s.code),
            if quick_exit {
                " — quick exit, treated as a failed launch"
            } else {
                ""
            }
        ));

        // Record playtime and last-played timestamp. Quick exits are failed
        // launches, not sessions — keep them out of the journal.
        {
            let state = app2.state::<AppState>();
            let mut settings = state.settings.blocking_lock();
            settings.total_playtime_secs += started.elapsed().as_secs();
            settings.last_played = session_start_unix;
            let _ = settings.save();
        }
        if !quick_exit {
            crate::config::sessions::append(crate::config::sessions::GameSession {
                start: session_start_unix,
                duration_secs: started.elapsed().as_secs(),
            });
        }

        // Reap whatever wine left behind now that the session is over. A
        // crashed game routinely leaves processes that our process group kill
        // cannot see, and they hold on to an X connection each until the
        // server starts refusing new ones ("Maximum number of clients
        // reached") and the next launch freezes on the intro logo — issue #33.
        // In the Flatpak a surviving wineserver breaks the next launch
        // outright. See launcher::shutdown_wineserver.
        {
            let settings = app2.state::<AppState>().settings.blocking_lock().clone();
            crate::game::launcher::shutdown_wineserver(&settings, false);
        }

        if let Some(status) = status {
            // Give the game a moment to flush its stderr/stdout.
            std::thread::sleep(std::time::Duration::from_millis(200));
            let log_tail = crate::game::launcher::read_log_tail(&log_path, 60);
            let hint = crate::game::diagnose::diagnose_launch_failure(&log_tail, &proton_dir);
            // A recognized fatal signature counts as a failed launch even past
            // the quick-exit window: prefix creation/upgrade alone (protonfixes
            // downloads and all) can hold the process open longer than that.
            if quick_exit || hint.is_some() {
                // The window may be hidden (tray launch, --play, "hide after
                // launch") — bring it back so the failure dialog is seen.
                if let Some(window) = app2.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app2.emit(
                    "launch://failed",
                    crate::api::types::LaunchFailed {
                        exit_code: status.code,
                        log_tail,
                        hint: hint.map(str::to_string),
                    },
                );
            }
            let _ = app2.emit(
                "game://exited",
                crate::api::types::GameExited {
                    exit_code: status.code,
                },
            );
        } else {
            let _ = app2.emit(
                "game://exited",
                crate::api::types::GameExited { exit_code: None },
            );
        }
    });

    Ok(())
}

/// `with_mods` is the second launch action on the home screen: the game runs
/// on D3D11 with the 3DMigoto proxy loaded instead of the native Vulkan
/// renderer. Absent (tray menu, `--play`) means a normal launch.
#[tauri::command]
#[specta::specta]
pub async fn launch_game(app: tauri::AppHandle, with_mods: Option<bool>) -> Result<(), AppError> {
    launch_and_watch(app, with_mods.unwrap_or(false)).await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_game(state: State<'_, AppState>) -> Result<(), AppError> {
    let pid = state.game_pid.load(std::sync::atomic::Ordering::SeqCst);
    if pid == 0 {
        return Ok(());
    }

    // Ask the game to terminate, then escalate to an outright kill if it is
    // still alive a few seconds later.
    crate::game::launcher::request_stop(pid);

    let game_running = state.game_running.clone();
    let game_pid = state.game_pid.clone();
    let settings = state.settings.lock().await.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        if !game_running.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let pid = game_pid.load(std::sync::atomic::Ordering::SeqCst);
        if pid != 0 {
            crate::game::launcher::force_stop(pid);
        }
        // Killing the process group is not enough for a game that already
        // crashed: its wine loader is gone and the process has been reparented
        // into wineserver's own session, out of the group's reach, so the stop
        // button appeared to do nothing and Endfield.exe had to be killed by
        // hand (issue #33). Tearing the prefix's wineserver down takes every
        // one of its clients with it.
        tokio::task::spawn_blocking(move || {
            crate::game::launcher::shutdown_wineserver(&settings, true);
        });
    });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn is_game_running(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(state.game_running.load(std::sync::atomic::Ordering::SeqCst))
}

/// Point the launcher at an already existing game installation (e.g. one made
/// by the official launcher under Bottles/Lutris). The folder must contain
/// Endfield.exe. The install is assumed to be up to date; if it is not, the
/// user can run Repair. Returns the version recorded.
#[tauri::command]
#[specta::specta]
pub async fn import_existing_game(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, AppError> {
    let game_path = std::path::Path::new(&path);
    if !crate::game::state::has_existing_install(game_path) {
        return Err(AppError::GameNotFound(format!(
            "Endfield.exe not found in {}",
            path
        )));
    }

    let version_info = crate::api::client::get_latest_game_version(&state.http_client, "").await?;
    let version = version_info.version.clone();

    let mut settings = state.settings.lock().await;
    settings.game_dir = path.clone();
    settings.download_dir = game_path.join("_download").to_string_lossy().to_string();
    settings.installed_version = version.clone();
    settings.save_async().await?;

    Ok(version)
}

/// Delete the game installation (game directory + its _download cache) and
/// reset the installed version. Refuses while the game is running, and only
/// acts when the directory actually contains the game.
#[tauri::command]
#[specta::specta]
pub async fn uninstall_game(state: State<'_, AppState>) -> Result<(), AppError> {
    if state.game_running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(AppError::Api(
            "Cannot uninstall while the game is running".to_string(),
        ));
    }

    let settings = state.settings.lock().await;
    let game_dir = settings.game_dir.clone();
    let download_dir = settings.download_dir.clone();
    drop(settings);

    let game_path = std::path::PathBuf::from(&game_dir);
    if !game_path.join("Endfield.exe").exists()
        && !crate::game::state::incomplete_marker(&game_path).exists()
    {
        return Err(AppError::GameNotFound(format!(
            "No game installation found in {}",
            game_dir
        )));
    }

    let download_path = std::path::PathBuf::from(&download_dir);
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        // A failed removal (permissions, a busy mount) must surface instead of
        // being swallowed and then reported as a successful uninstall with
        // tens of GB still on disk.
        std::fs::remove_dir_all(&game_path)?;
        // Only remove the download cache if it follows the _download naming —
        // the user may have pointed it at a shared folder.
        if download_path.file_name().is_some_and(|n| n == "_download") && download_path.exists() {
            std::fs::remove_dir_all(&download_path)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| AppError::Api(format!("Uninstall task failed: {}", e)))??;

    crate::logging::info(format!("uninstalled game from {}", game_dir));
    let mut settings = state.settings.lock().await;
    settings.installed_version = String::new();
    settings.save_async().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_installed_version(
    state: State<'_, AppState>,
    version: String,
) -> Result<(), AppError> {
    let mut settings = state.settings.lock().await;
    settings.installed_version = version;
    settings.save_async().await?;
    Ok(())
}

/// Full play-session journal (oldest first) for the stats card.
#[tauri::command]
#[specta::specta]
pub async fn get_game_sessions() -> Result<Vec<crate::config::sessions::GameSession>, AppError> {
    tokio::task::spawn_blocking(crate::config::sessions::load)
        .await
        .map_err(|e| AppError::Api(format!("sessions load task failed: {}", e)))
}


