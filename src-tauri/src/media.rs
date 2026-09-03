//! Probe for the host's ability to play the animated launcher background.
//!
//! WebKitGTK plays `<video>` elements through the host's GStreamer. When the
//! required plugins are missing it does not fail the element gracefully — the
//! web process prints "GStreamer element autoaudiosink not found" followed by
//! GLib-GObject-CRITICAL spam and the whole UI comes up blank (issue #31), so
//! the frontend's own onError fallback never gets a chance to run. Probe for
//! the plugins up front and serve the static background image instead when
//! they are absent.

#[cfg(unix)]
use std::path::PathBuf;

/// Whether the video backdrop can be attempted at all. False means the
/// backend strips `video_url` from the launcher content and the UI renders
/// the static image, which needs no GStreamer.
#[cfg(unix)]
pub fn can_play_video_background() -> bool {
    // `autodetect` provides autoaudiosink — the exact element WebKit dies
    // without — and `playback` provides playbin, the pipeline it builds.
    // Both ship in every functional GStreamer install (base + good plugins).
    dirs_have_plugins(
        &plugin_dirs(),
        &["libgstautodetect.so", "libgstplayback.so"],
    )
}

/// Windows plays the backdrop through WebView2 (Chromium), which decodes
/// H.264/MP4 itself — there is no host plugin stack that can be missing.
#[cfg(windows)]
pub fn can_play_video_background() -> bool {
    true
}

/// Every directory the host's GStreamer may load plugins from: the standard
/// env overrides (which the AppImage sets to its bundled copy), then the
/// per-distro system locations.
#[cfg(unix)]
fn plugin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for var in [
        "GST_PLUGIN_SYSTEM_PATH_1_0",
        "GST_PLUGIN_PATH_1_0",
        "GST_PLUGIN_PATH",
    ] {
        if let Some(value) = std::env::var_os(var) {
            dirs.extend(std::env::split_paths(&value));
        }
    }
    // Debian/Ubuntu multiarch, Fedora/openSUSE, Arch.
    dirs.push("/usr/lib/x86_64-linux-gnu/gstreamer-1.0".into());
    dirs.push("/usr/lib/aarch64-linux-gnu/gstreamer-1.0".into());
    dirs.push("/usr/lib64/gstreamer-1.0".into());
    dirs.push("/usr/lib/gstreamer-1.0".into());
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/gstreamer-1.0/plugins"));
    }
    dirs
}

/// True when every named plugin file exists in at least one of the dirs.
#[cfg(unix)]
fn dirs_have_plugins(dirs: &[PathBuf], names: &[&str]) -> bool {
    names
        .iter()
        .all(|name| dirs.iter().any(|dir| dir.join(name).is_file()))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn requires_every_plugin_not_just_one() {
        let dir = std::env::temp_dir().join(format!("llauncher-gst-probe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("libgstautodetect.so"), b"").unwrap();

        let dirs = vec![dir.clone()];
        // Only autodetect present — playback missing must fail the probe:
        // a partial GStreamer install is exactly the broken case.
        assert!(!dirs_have_plugins(&dirs, &["libgstautodetect.so", "libgstplayback.so"]));

        std::fs::write(dir.join("libgstplayback.so"), b"").unwrap();
        assert!(dirs_have_plugins(&dirs, &["libgstautodetect.so", "libgstplayback.so"]));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plugins_may_be_split_across_directories() {
        let base = std::env::temp_dir().join(format!("llauncher-gst-split-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("libgstautodetect.so"), b"").unwrap();
        std::fs::write(b.join("libgstplayback.so"), b"").unwrap();

        assert!(dirs_have_plugins(
            &[a, b],
            &["libgstautodetect.so", "libgstplayback.so"]
        ));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn empty_or_missing_directories_fail_closed() {
        assert!(!dirs_have_plugins(
            &[PathBuf::from("/nonexistent-llauncher-test")],
            &["libgstautodetect.so"]
        ));
        assert!(!dirs_have_plugins(&[], &["libgstautodetect.so"]));
    }
}
