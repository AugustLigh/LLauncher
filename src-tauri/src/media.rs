//! Probe for the host's ability to play the animated launcher background.
//!
//! WebKitGTK plays `<video>` elements through the host's GStreamer. When the
//! required plugins are missing it does not fail the element gracefully — the
//! web process prints "GStreamer element autoaudiosink not found" followed by
//! GLib-GObject-CRITICAL spam and the whole UI comes up blank (issue #31), so
//! the frontend's own onError fallback never gets a chance to run. Probe for
//! the plugins up front and serve the static background image instead when
//! they are absent.

use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

/// Whether the video backdrop can be attempted at all. False means the
/// backend strips `video_url` from the launcher content and the UI renders
/// the static image, which needs no GStreamer.
#[cfg(target_os = "linux")]
pub fn can_play_video_background() -> bool {
    // The probe only sees plugin files, not whether GStreamer can load them —
    // a stale registry or a mismatched plugin still takes the UI down. A
    // setting would sit behind the very UI that fails to come up, so the
    // escape hatch is an environment variable.
    if std::env::var_os("LLAUNCHER_NO_VIDEO").is_some_and(|v| !v.is_empty() && v != "0") {
        return false;
    }
    // `autodetect` provides autoaudiosink — the exact element WebKit dies
    // without — and `playback` provides playbin, the pipeline it builds.
    // Both ship in every functional GStreamer install (base + good plugins).
    dirs_have_plugins(
        &plugin_dirs(),
        &["libgstautodetect.so", "libgstplayback.so"],
    )
}

/// Windows plays the backdrop through WebView2 (Chromium) and macOS through
/// WKWebView, both of which decode H.264/MP4 themselves — there is no host
/// plugin stack that can be missing.
#[cfg(not(target_os = "linux"))]
pub fn can_play_video_background() -> bool {
    true
}

/// Every directory the host's GStreamer may load plugins from: the standard
/// env overrides (which the AppImage sets to its bundled copy), then the
/// per-distro system locations.
#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
fn dirs_have_plugins(dirs: &[PathBuf], names: &[&str]) -> bool {
    names
        .iter()
        .all(|name| dirs.iter().any(|dir| dir.join(name).is_file()))
}

// The backdrop video cache: `background.mp4`, overwritten by each new patch's
// video, and `background.meta` with its size, ETag and source URL.

/// The cached entry's (size, ETag), when it was downloaded from `url`.
fn cached_meta(dir: &Path, url: &str) -> Option<(u64, String)> {
    let meta = std::fs::read_to_string(dir.join("background.meta")).ok()?;
    let mut lines = meta.lines();
    let (size, etag, from) = (lines.next()?, lines.next()?, lines.next()?);
    if from != url {
        return None;
    }
    Some((size.parse().ok()?, etag.to_string()))
}

/// The video cached from `url` and its ETag, if whole.
pub fn read_cached_video(dir: &Path, url: &str) -> Option<(Vec<u8>, String)> {
    let (size, etag) = cached_meta(dir, url)?;
    if std::fs::metadata(dir.join("background.mp4")).ok()?.len() != size {
        return None;
    }
    let bytes = std::fs::read(dir.join("background.mp4")).ok()?;
    (bytes.len() as u64 == size).then_some((bytes, etag))
}

pub fn part_path(dir: &Path) -> std::path::PathBuf {
    dir.join("background.mp4.part")
}

/// Move a finished download of `size` bytes into the cache.
pub fn commit_cached_video(dir: &Path, etag: &str, size: u64, url: &str) -> std::io::Result<()> {
    // Meta first: the new video must never pass under the old ETag.
    match std::fs::remove_file(dir.join("background.meta")) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    std::fs::rename(part_path(dir), dir.join("background.mp4"))?;
    crate::util::write_atomic(
        &dir.join("background.meta"),
        format!("{size}\n{etag}\n{url}").as_bytes(),
    )
}

/// Drop the video cached from `url` that the webview couldn't play, and mark
/// the URL so it isn't downloaded again on every launch.
// ponytail: the mark lasts until the URL changes; store the ETag if the
// server ever fixes a file in place.
pub fn forget_cached_video(dir: &Path, url: &str) {
    if cached_meta(dir, url).is_some() {
        let _ = std::fs::remove_file(dir.join("background.meta"));
        let _ = std::fs::remove_file(dir.join("background.mp4"));
    }
    let _ = crate::util::write_atomic(&dir.join("background.unplayable"), url.as_bytes());
}

pub fn is_unplayable(dir: &Path, url: &str) -> bool {
    std::fs::read_to_string(dir.join("background.unplayable")).is_ok_and(|u| u == url)
}

#[cfg(test)]
mod video_cache_tests {
    use super::*;

    fn store(dir: &Path, etag: &str, url: &str, bytes: &[u8]) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(part_path(dir), bytes).unwrap();
        commit_cached_video(dir, etag, bytes.len() as u64, url).unwrap();
    }

    #[test]
    fn roundtrip_overwrite_and_corruption() {
        let dir = std::env::temp_dir().join(format!("llauncher-video-{}", std::process::id()));
        let (a, b) = ("https://cdn/a.mp4", "https://cdn/b.mp4");
        let _ = std::fs::remove_dir_all(&dir); // left over by a failed run
        assert!(read_cached_video(&dir, a).is_none());
        store(&dir, "\"e1\"", a, b"one");
        assert_eq!(read_cached_video(&dir, a).unwrap(), (b"one".to_vec(), "\"e1\"".into()));
        assert!(read_cached_video(&dir, b).is_none());
        store(&dir, "\"e2\"", b, b"two");
        assert_eq!(read_cached_video(&dir, b).unwrap(), (b"two".to_vec(), "\"e2\"".into()));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        std::fs::write(dir.join("background.mp4"), b"tw").unwrap(); // truncated
        assert!(read_cached_video(&dir, b).is_none());
        store(&dir, "\"e3\"", b, b"three");
        forget_cached_video(&dir, b);
        assert!(read_cached_video(&dir, b).is_none());
        assert!(is_unplayable(&dir, b) && !is_unplayable(&dir, a));
        store(&dir, "\"e4\"", b, b"four");
        forget_cached_video(&dir, a);
        assert!(read_cached_video(&dir, b).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(all(test, target_os = "linux"))]
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
