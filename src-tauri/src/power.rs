//! Power management around long transfers: keep the machine from suspending
//! while a download runs, and let the user switch the display off meanwhile.
//!
//! Both are best-effort. A desktop with none of the supported interfaces just
//! gets no inhibitor and a "not supported" error from the screen-off button;
//! the transfer itself is never affected.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::error::AppError;

/// Process-wide inhibitor state. `enabled` mirrors the user setting; the
/// handle is held exactly while at least one transfer is active.
pub struct Power {
    enabled: AtomicBool,
    handle: Mutex<Option<platform::Inhibitor>>,
}

impl Power {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            handle: Mutex::new(None),
        }
    }

    pub fn set_enabled(&self, enabled: bool, transfers_active: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        self.sync(transfers_active);
    }

    /// Bring the inhibitor in line with the current transfer activity:
    /// acquire it when a transfer runs and the setting is on, release it
    /// otherwise. Idempotent, cheap when nothing changes.
    pub fn sync(&self, transfers_active: bool) {
        let want = transfers_active && self.enabled.load(Ordering::SeqCst);
        let mut slot = match self.handle.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        match (want, slot.is_some()) {
            (true, false) => match platform::acquire() {
                Ok(inhibitor) => {
                    crate::logging::info(format!("power: sleep inhibited via {}", inhibitor.via()));
                    *slot = Some(inhibitor);
                }
                Err(e) => crate::logging::warn(format!("power: cannot inhibit sleep: {e}")),
            },
            (false, true) => {
                if let Some(inhibitor) = slot.take() {
                    inhibitor.release();
                    crate::logging::info("power: sleep inhibitor released");
                }
            }
            _ => {}
        }
    }
}

/// Switch the display off, the way the "Turn off screen" shortcut does. Runs
/// after a short delay so the click that triggered it (its button release,
/// an Enter key going up) cannot wake the screen straight back up.
pub fn turn_off_screen() -> Result<(), AppError> {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(800));
        if let Err(e) = platform::screen_off() {
            crate::logging::warn(format!("power: screen off failed: {e}"));
        }
    });
    Ok(())
}

/// Whether the current desktop exposes a way to switch the screen off, so
/// the UI can hide the button where it would only ever fail.
pub fn screen_off_supported() -> bool {
    platform::screen_off_supported()
}

#[cfg(target_os = "linux")]
mod platform {
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::{OwnedFd, OwnedObjectPath, Value};

    /// Whatever kept the inhibition alive. Every variant owns the D-Bus
    /// connection it was made on: an inhibit is bound to the caller's bus
    /// name, so dropping the connection is itself a release.
    pub enum Inhibitor {
        /// xdg-desktop-portal: works inside Flatpak and on any desktop with a
        /// portal backend. Released by closing the request object.
        Portal {
            conn: Connection,
            request: OwnedObjectPath,
        },
        /// org.freedesktop.ScreenSaver, implemented by KDE, GNOME, XFCE,
        /// Cinnamon and friends. Released with the cookie.
        ScreenSaver { conn: Connection, cookie: u32 },
        /// systemd-logind block inhibitor: held for as long as the fd lives.
        Logind { _conn: Connection, _fd: OwnedFd },
    }

    impl Inhibitor {
        pub fn via(&self) -> &'static str {
            match self {
                Self::Portal { .. } => "xdg-desktop-portal",
                Self::ScreenSaver { .. } => "org.freedesktop.ScreenSaver",
                Self::Logind { .. } => "logind",
            }
        }

        pub fn release(self) {
            match self {
                Self::Portal { conn, request } => {
                    if let Ok(proxy) = Proxy::new(
                        &conn,
                        "org.freedesktop.portal.Desktop",
                        request.as_str(),
                        "org.freedesktop.portal.Request",
                    ) {
                        let _ = proxy.call_method("Close", &());
                    }
                }
                Self::ScreenSaver { conn, cookie } => {
                    if let Ok(proxy) = screensaver_proxy(&conn) {
                        let _ = proxy.call_method("UnInhibit", &(cookie,));
                    }
                }
                Self::Logind { .. } => {}
            }
        }
    }

    const APP_ID: &str = "io.github.augustligh.LLauncher";
    const REASON: &str = "Downloading Arknights: Endfield";

    fn in_flatpak() -> bool {
        std::env::var_os("FLATPAK_ID").is_some()
    }

    fn screensaver_proxy(conn: &Connection) -> zbus::Result<Proxy<'static>> {
        Proxy::new(
            conn,
            "org.freedesktop.ScreenSaver",
            "/org/freedesktop/ScreenSaver",
            "org.freedesktop.ScreenSaver",
        )
    }

    fn via_portal() -> Result<Inhibitor, String> {
        let conn = Connection::session().map_err(|e| e.to_string())?;
        let proxy = Proxy::new(
            &conn,
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Inhibit",
        )
        .map_err(|e| e.to_string())?;
        // Flags: 4 = suspend, 8 = idle. Logout and user switching stay allowed.
        let mut options = std::collections::HashMap::new();
        options.insert("reason", Value::from(REASON));
        let request: OwnedObjectPath = proxy
            .call("Inhibit", &("", 4u32 | 8u32, options))
            .map_err(|e| e.to_string())?;
        Ok(Inhibitor::Portal { conn, request })
    }

    // On Plasma 6 this name is owned by KWin, which turns the inhibit into an
    // idle inhibitor: PowerDevil's idle timers (dim, blank, suspend) never
    // fire, but PowerDevil's own PolicyAgent lists nothing — do not look for
    // the launcher there when checking that it works.
    fn via_screensaver() -> Result<Inhibitor, String> {
        let conn = Connection::session().map_err(|e| e.to_string())?;
        let proxy = screensaver_proxy(&conn).map_err(|e| e.to_string())?;
        let cookie: u32 = proxy
            .call("Inhibit", &(APP_ID, REASON))
            .map_err(|e| e.to_string())?;
        Ok(Inhibitor::ScreenSaver { conn, cookie })
    }

    fn via_logind() -> Result<Inhibitor, String> {
        let conn = Connection::system().map_err(|e| e.to_string())?;
        let proxy = Proxy::new(
            &conn,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )
        .map_err(|e| e.to_string())?;
        let fd: OwnedFd = proxy
            .call("Inhibit", &("sleep:idle", "LLauncher", REASON, "block"))
            .map_err(|e| e.to_string())?;
        Ok(Inhibitor::Logind {
            _conn: conn,
            _fd: fd,
        })
    }

    pub fn acquire() -> Result<Inhibitor, String> {
        // Inside Flatpak only the portal is reachable without extra
        // permissions; outside it the desktop's own interface is the most
        // direct route and the portal a fine fallback.
        let order: &[fn() -> Result<Inhibitor, String>] = if in_flatpak() {
            &[via_portal, via_screensaver, via_logind]
        } else {
            &[via_screensaver, via_portal, via_logind]
        };
        let mut errors = Vec::new();
        for attempt in order {
            match attempt() {
                Ok(inhibitor) => return Ok(inhibitor),
                Err(e) => errors.push(e),
            }
        }
        Err(errors.join("; "))
    }

    fn has_name(conn: &Connection, name: &str) -> bool {
        let Ok(dbus) = zbus::blocking::fdo::DBusProxy::new(conn) else {
            return false;
        };
        let Ok(name) = zbus::names::BusName::try_from(name) else {
            return false;
        };
        dbus.name_has_owner(name).unwrap_or(false)
    }

    fn desktop_is(name: &str) -> bool {
        std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .split(':')
            .any(|part| part.eq_ignore_ascii_case(name))
    }

    pub fn screen_off_supported() -> bool {
        desktop_is("KDE") || desktop_is("GNOME") || std::env::var_os("DISPLAY").is_some()
    }

    pub fn screen_off() -> Result<(), String> {
        let conn = Connection::session().map_err(|e| e.to_string())?;
        let mut errors = Vec::new();

        // KDE (Steam Deck desktop mode included): PowerDevil's own shortcut,
        // works on both Wayland and X11.
        if has_name(&conn, "org.kde.kglobalaccel") {
            let result = Proxy::new(
                &conn,
                "org.kde.kglobalaccel",
                "/component/org_kde_powerdevil",
                "org.kde.kglobalaccel.Component",
            )
            .and_then(|proxy| proxy.call::<_, _, bool>("invokeShortcut", &("Turn Off Screen",)));
            match result {
                Ok(true) => return Ok(()),
                Ok(false) => errors.push("kglobalaccel: shortcut not found".to_string()),
                Err(e) => errors.push(format!("kglobalaccel: {e}")),
            }
        }

        // GNOME: activating the screen saver blanks the display (and locks
        // it when the lock screen is enabled, which is the desktop's rule).
        if has_name(&conn, "org.gnome.ScreenSaver") {
            let result = Proxy::new(
                &conn,
                "org.gnome.ScreenSaver",
                "/org/gnome/ScreenSaver",
                "org.gnome.ScreenSaver",
            )
            .and_then(|proxy| proxy.call_method("SetActive", &(true,)));
            match result {
                Ok(_) => return Ok(()),
                Err(e) => errors.push(format!("gnome-screensaver: {e}")),
            }
        }

        // Anything on X11: plain DPMS.
        if std::env::var_os("DISPLAY").is_some() {
            match std::process::Command::new("xset")
                .args(["dpms", "force", "off"])
                .status()
            {
                Ok(status) if status.success() => return Ok(()),
                Ok(status) => errors.push(format!("xset exited with {status}")),
                Err(e) => errors.push(format!("xset: {e}")),
            }
        }

        if errors.is_empty() {
            Err("no supported screen control found on this desktop".to_string())
        } else {
            Err(errors.join("; "))
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::sync::mpsc::{sync_channel, SyncSender};

    /// `SetThreadExecutionState` is per-thread and lasts as long as that
    /// thread does, so the inhibitor is a parked thread; sending on the
    /// channel (or dropping it) lets the thread clear the state and exit.
    pub struct Inhibitor {
        _release: SyncSender<()>,
    }

    impl Inhibitor {
        pub fn via(&self) -> &'static str {
            "SetThreadExecutionState"
        }
        pub fn release(self) {}
    }

    pub fn acquire() -> Result<Inhibitor, String> {
        use windows_sys::Win32::System::Power::{
            SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
        };
        let (tx, rx) = sync_channel::<()>(0);
        let (ready_tx, ready_rx) = sync_channel::<bool>(0);
        std::thread::spawn(move || {
            // SAFETY: plain Win32 call with constant flags, no pointers.
            let ok = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) } != 0;
            let _ = ready_tx.send(ok);
            if !ok {
                return;
            }
            // Blocks until the sender is dropped or a release is sent.
            let _ = rx.recv();
            // SAFETY: as above.
            unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
        });
        match ready_rx.recv() {
            Ok(true) => Ok(Inhibitor { _release: tx }),
            _ => Err("SetThreadExecutionState failed".to_string()),
        }
    }

    pub fn screen_off_supported() -> bool {
        true
    }

    pub fn screen_off() -> Result<(), String> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SendMessageW, HWND_BROADCAST, SC_MONITORPOWER, WM_SYSCOMMAND,
        };
        // lParam 2 = power off; the display wakes on the next input event.
        // SAFETY: broadcast of a documented system command, no pointers.
        unsafe { SendMessageW(HWND_BROADCAST, WM_SYSCOMMAND, SC_MONITORPOWER as usize, 2) };
        Ok(())
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    pub struct Inhibitor;
    impl Inhibitor {
        pub fn via(&self) -> &'static str {
            "none"
        }
        pub fn release(self) {}
    }
    pub fn acquire() -> Result<Inhibitor, String> {
        Err("not supported on this platform".to_string())
    }
    pub fn screen_off_supported() -> bool {
        false
    }
    pub fn screen_off() -> Result<(), String> {
        Err("not supported on this platform".to_string())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    /// Needs a live session bus, so it is opt-in:
    /// `cargo test power -- --ignored --nocapture`. Holds the inhibitor for a
    /// few seconds so it can be seen from outside (`systemd-inhibit --list`,
    /// KDE's PolicyAgent `ListInhibitions`).
    #[test]
    #[ignore]
    fn acquires_and_releases_an_inhibitor() {
        let inhibitor = super::platform::acquire().expect("no inhibitor available");
        eprintln!("inhibited via {}", inhibitor.via());
        std::thread::sleep(std::time::Duration::from_secs(4));
        inhibitor.release();
    }
}
