//! Small cross-cutting helpers.

use std::process::Command;

/// Strip AppImage-injected dynamic-linker paths from a child process's
/// environment.
///
/// The AppImage runtime prepends its bundled `usr/lib` to `LD_LIBRARY_PATH`
/// (and may export `LD_PRELOAD`) so the webview finds its own GTK/WebKit. Those
/// bundled libraries are usually older than a modern host's. When we then spawn
/// a *system* binary — `tar` → `xz` for DWProton extraction, or the game's
/// `bash` → Proton — it loads the wrong, older library and aborts, e.g.
/// `xz: /tmp/.mount_*/usr/lib/liblzma.so.5: version 'XZ_5.6.0' not found
/// (required by xz)` (GitHub issue #19).
///
/// We keep any genuinely user-provided entries and drop only those pointing
/// inside the AppImage mount, so system processes resolve their libraries via
/// the normal loader cache. Outside an AppImage this is a no-op.
pub fn strip_appimage_libs(cmd: &mut Command) {
    let appdir = std::env::var("APPDIR").ok();
    let appdir = appdir.as_deref().filter(|d| !d.is_empty());

    apply(cmd, "LD_LIBRARY_PATH", appdir, &[':']);
    // LD_PRELOAD entries may be separated by colons or spaces.
    apply(cmd, "LD_PRELOAD", appdir, &[':', ' ']);
}

fn apply(cmd: &mut Command, var: &str, appdir: Option<&str>, seps: &[char]) {
    let Ok(current) = std::env::var(var) else {
        return;
    };
    match sanitized(&current, appdir, seps) {
        Sanitized::Unchanged => {}
        Sanitized::Remove => {
            cmd.env_remove(var);
        }
        Sanitized::Set(value) => {
            cmd.env(var, value);
        }
    }
}

#[derive(Debug, PartialEq)]
enum Sanitized {
    /// No AppImage paths present — leave the variable as-is.
    Unchanged,
    /// Every entry pointed inside the AppImage — drop the variable entirely.
    Remove,
    /// Some entries survived — set the variable to the filtered list.
    Set(String),
}

fn sanitized(current: &str, appdir: Option<&str>, seps: &[char]) -> Sanitized {
    let entries: Vec<&str> = current
        .split(|c| seps.contains(&c))
        .filter(|p| !p.is_empty())
        .collect();
    let kept: Vec<&str> = entries
        .iter()
        .copied()
        .filter(|p| !is_appimage_path(p, appdir))
        .collect();

    if kept.len() == entries.len() {
        Sanitized::Unchanged
    } else if kept.is_empty() {
        Sanitized::Remove
    } else {
        Sanitized::Set(kept.join(":"))
    }
}

fn is_appimage_path(path: &str, appdir: Option<&str>) -> bool {
    if let Some(dir) = appdir {
        if path == dir || path.starts_with(&format!("{}/", dir)) {
            return true;
        }
    }
    // The AppImage runtime mounts the bundle at /tmp/.mount_<rand>; no real
    // system library directory ever contains that path segment.
    path.contains("/.mount_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_value_without_appimage_paths() {
        assert_eq!(
            sanitized("/usr/lib:/usr/local/lib", Some("/tmp/.mount_abc"), &[':']),
            Sanitized::Unchanged
        );
    }

    #[test]
    fn drops_var_when_only_appimage_paths() {
        // Mirrors the issue #19 layout: AppRun set LD_LIBRARY_PATH purely to its
        // own usr/lib, so the whole variable must go.
        let appdir = Some("/tmp/.mount_LLaunCEJaAhH");
        assert_eq!(
            sanitized(
                "/tmp/.mount_LLaunCEJaAhH/usr/lib:/tmp/.mount_LLaunCEJaAhH/usr/lib/x86_64-linux-gnu",
                appdir,
                &[':'],
            ),
            Sanitized::Remove
        );
    }

    #[test]
    fn keeps_user_paths_and_strips_appimage_ones() {
        let appdir = Some("/tmp/.mount_LLaunCEJaAhH");
        assert_eq!(
            sanitized(
                "/tmp/.mount_LLaunCEJaAhH/usr/lib:/opt/cuda/lib64",
                appdir,
                &[':'],
            ),
            Sanitized::Set("/opt/cuda/lib64".to_string())
        );
    }

    #[test]
    fn detects_mount_path_without_appdir() {
        // Defensive: even if APPDIR is somehow unset, the /.mount_ signature is
        // enough to recognise a leaked AppImage path.
        assert!(is_appimage_path("/tmp/.mount_xyz/usr/lib", None));
        assert!(!is_appimage_path("/usr/lib", None));
    }
}
