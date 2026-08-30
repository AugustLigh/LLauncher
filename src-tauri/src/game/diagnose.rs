//! Post-mortem analysis of the launch log for known failure signatures.
//!
//! When the game dies for a reason we can recognize, the failure dialog can
//! tell the user what to actually do instead of just dumping the log at them.

/// Stable identifiers for known failure signatures. The frontend maps these
/// to translated, actionable advice — keep them in sync with `launchFailed.*`
/// hint strings in `src/i18n/`.
pub const HINT_DWPROTON11_NTOSKRNL: &str = "dwproton11-ntoskrnl";

/// Scan the launch log tail for failure signatures we know the fix for.
///
/// DWProton 11.x (wine-11 based) cannot run Endfield: the game's anti-cheat
/// driver calls ntoskrnl stubs wine leaves unimplemented (`PsGetProcessExitStatus`,
/// `InbvAcquireDisplayOwnership`, ...) and wine aborts the process — see
/// dawn-winery/dwproton#30 and the pin rationale on
/// `RECOMMENDED_DWPROTON_TAG`. Match any ntoskrnl stub abort rather than the
/// individual function names: each 11.x build has died on a different one.
pub fn diagnose_launch_failure(log_tail: &str, proton_dir: &str) -> Option<&'static str> {
    let ntoskrnl_abort = log_tail
        .lines()
        .any(|l| l.contains("unimplemented function ntoskrnl.exe.") && l.contains("aborting"));

    if ntoskrnl_abort && is_dwproton_11(proton_dir) {
        return Some(HINT_DWPROTON11_NTOSKRNL);
    }

    None
}

/// Whether the active Proton directory is a DWProton 11.x build
/// (`.../dwproton-11.0-11-x86_64`). Matched on the path's file name so an
/// unrelated parent directory can't trigger it.
fn is_dwproton_11(proton_dir: &str) -> bool {
    std::path::Path::new(proton_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with("dwproton-11"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROTON_11: &str = "/home/user/.local/share/llauncher/proton/dwproton-11.0-11-x86_64";
    const PROTON_10: &str = "/home/user/.local/share/llauncher/proton/dwproton-10.0-26-x86_64";

    #[test]
    fn recognizes_ntoskrnl_stub_aborts_on_dwproton_11() {
        // Both stubs seen in the wild (dawn-winery/dwproton#30 and a user
        // report against 11.0-11) must map to the same advice: switch to the
        // recommended 10.x build.
        for func in ["PsGetProcessExitStatus", "InbvAcquireDisplayOwnership"] {
            let log = format!(
                "ntsync: up and running.\n\
                 wine: Call from 00006FFFFFBFD947 to unimplemented function ntoskrnl.exe.{}, aborting\n",
                func
            );
            assert_eq!(
                diagnose_launch_failure(&log, PROTON_11),
                Some(HINT_DWPROTON11_NTOSKRNL)
            );
        }
    }

    #[test]
    fn stays_quiet_without_a_known_signature() {
        // GStreamer plugin warnings and EGL noise are not fatal on their own —
        // no advice must be offered for a log that only contains those.
        let log = "GStreamer-WARNING: Failed to load plugin libgstvpx.so: libvpx.so.9: cannot open\n\
                   libEGL warning: egl: failed to create dri2 screen\n";
        assert_eq!(diagnose_launch_failure(log, PROTON_11), None);
    }

    #[test]
    fn ignores_ntoskrnl_aborts_on_other_proton_versions() {
        // On a 10.x build the abort would be something new, not the known
        // 11.x regression — wrong advice is worse than none.
        let log =
            "wine: Call from 0x1 to unimplemented function ntoskrnl.exe.PsGetProcessExitStatus, aborting\n";
        assert_eq!(diagnose_launch_failure(log, PROTON_10), None);
        assert_eq!(diagnose_launch_failure(log, ""), None);
    }

    #[test]
    fn matches_dwproton_11_on_the_directory_name_only() {
        assert!(is_dwproton_11("/opt/protons/dwproton-11.0-12-x86_64"));
        assert!(!is_dwproton_11("/opt/dwproton-11.0-12/dwproton-10.0-26"));
    }
}
