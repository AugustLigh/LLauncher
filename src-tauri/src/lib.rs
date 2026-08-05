mod api;
mod commands;
mod config;
mod download;
mod error;
mod game;
mod state;
mod util;

use config::settings::AppSettings;
use state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_autostart::ManagerExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Use xdg-desktop-portal for file dialogs (native KDE/GNOME picker)
    std::env::set_var("GTK_USE_PORTAL", "1");

    // Workaround for WebKitGTK 2.40+ EGL_BAD_PARAMETER on Arch/CachyOS/Fedora
    // and NVIDIA setups. Disables the DMA-BUF renderer that breaks on many
    // Linux GPU stacks. Respect the user's override if they set it explicitly.
    #[cfg(target_os = "linux")]
    {
        // Point GIO at the system TLS backend module (glib-networking). The
        // AppImage does not bundle libgio{gnutls,openssl}.so, so without this
        // WebKit falls back to GDummyTlsBackend, every HTTPS request fails
        // silently and the UI goes black ~1s after launch. We only set the
        // variable when a TLS module is actually present, and never override an
        // explicit user value.
        if std::env::var_os("GIO_MODULE_DIR").is_none() {
            const GIO_MODULE_DIRS: [&str; 4] = [
                "/usr/lib/x86_64-linux-gnu/gio/modules",
                "/usr/lib64/gio/modules",
                "/usr/lib/gio/modules",
                "/usr/lib/aarch64-linux-gnu/gio/modules",
            ];

            // Prefer the module bundled inside the AppImage (APPDIR is set by
            // the AppImage runtime), then fall back to system locations.
            let mut candidates: Vec<std::path::PathBuf> = Vec::new();
            if let Some(appdir) = std::env::var_os("APPDIR") {
                candidates.push(std::path::Path::new(&appdir).join("usr/lib/gio/modules"));
            }
            candidates.extend(GIO_MODULE_DIRS.iter().map(std::path::PathBuf::from));

            for dir in candidates {
                if dir.join("libgiognutls.so").exists()
                    || dir.join("libgioopenssl.so").exists()
                {
                    std::env::set_var("GIO_MODULE_DIR", &dir);
                    break;
                }
            }
        }
    }

    let settings = AppSettings::load();
    let app_state = AppState::new(settings);

    // `llauncher --play` (desktop-file "Play" action, scripts): start the game
    // straight away and stay in the tray instead of showing the window.
    let play_requested = std::env::args().any(|a| a == "--play");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // A second `--play` invocation launches the game in the running
            // instance; anything else brings the window up as before.
            if args.iter().any(|a| a == "--play") {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    // Immediate failures (not installed, no Proton) have no
                    // dialog of their own — fall back to showing the window.
                    if commands::launch_and_watch(app.clone()).await.is_err() {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
                return;
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .setup(move |app| {
            let launch = MenuItem::with_id(app, "launch", "Launch Game", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&launch, &show, &quit])?;

            // Enable autostart on first launch
            {
                let state = app.state::<AppState>();
                let mut settings = state.settings.blocking_lock();
                if !settings.autostart_initialized {
                    let _ = app.autolaunch().enable();
                    settings.autostart_initialized = true;
                    let _ = settings.save();
                }
            }

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "launch" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            // Errors (already running, missing proton, ...) are
                            // surfaced through the launch://failed flow or
                            // silently ignored — there is no UI here.
                            let _ = commands::launch_and_watch(app).await;
                        });
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            if play_requested {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // Crashes after spawn reopen the window via launch://failed;
                    // immediate failures (not installed, no Proton) get no
                    // dialog, so bring the window back for those.
                    if commands::launch_and_watch(app_handle.clone()).await.is_err() {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_game_version,
            commands::get_launcher_content,
            commands::check_game_state,
            commands::start_download,
            commands::cancel_download,
            commands::clear_download_cache,
            commands::verify_game_integrity,
            commands::start_update,
            commands::launch_game,
            commands::stop_game,
            commands::is_game_running,
            commands::import_existing_game,
            commands::uninstall_game,
            commands::get_debug_info,
            commands::read_launch_log,
            commands::repair_game,
            commands::update_installed_version,
            commands::check_system_requirements,
            commands::get_dwproton_latest,
            commands::list_dwproton_releases,
            commands::recommended_proton_tag,
            commands::list_installed_protons,
            commands::set_active_proton,
            commands::download_dwproton,
            commands::cancel_proton_download,
            commands::get_game_sessions,
            commands::get_prefix_info,
            commands::open_prefix_folder,
            commands::run_prefix_tool,
            commands::clear_shader_cache,
            commands::backup_prefix,
            commands::restore_prefix,
            commands::reset_prefix,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
