//! Post-mortem analysis of the launch log for known failure signatures.
//!
//! When the game dies for a reason we can recognize, the failure dialog can
//! tell the user what to actually do instead of just dumping the log at them.

use crate::download::proton::RECOMMENDED_DWPROTON_TAG;

/// Stable identifiers for known failure signatures. The frontend maps these
/// to translated, actionable advice — keep them in sync with `launchFailed.*`
/// hint strings in `src/i18n/`.
pub const HINT_DWPROTON_OUTDATED: &str = "dwproton-outdated";
pub const HINT_X_CLIENTS_EXHAUSTED: &str = "x-clients-exhausted";

/// What the game's anti-cheat exits with since 1.6 when the Wine underneath
/// cannot load its driver (dwproton 10.x; issue #48).
const ACE_DRIVER_EXIT_CODE: i32 = 222;

/// Scan the launch log tail and the exit code for failure signatures we know
/// the fix for.
pub fn diagnose_launch_failure(
    log_tail: &str,
    proton_dir: &str,
    exit_code: Option<i32>,
) -> Option<&'static str> {
    // Xlib, refused a connection by the X server. The launcher reaps the
    // prefix after every session precisely so this cannot build up, so seeing
    // it means something else on the machine is holding the connections —
    // ending the session is the only cure once the limit is hit (issue #33).
    // Checked first: the anti-cheat dies right after it, and that is fallout.
    if log_tail
        .lines()
        .any(|l| l.trim() == "Maximum number of clients reached")
    {
        return Some(HINT_X_CLIENTS_EXHAUSTED);
    }

    // DWProton builds older than the recommended one cannot run the game's
    // anti-cheat: 11.0-11/11.0-12 abort on unimplemented ntoskrnl stubs
    // (`PsGetProcessExitStatus`, `InbvAcquireDisplayOwnership`, ... — each
    // build died on a different one, so match any), and since the game's 1.6
    // update 10.x cannot load the new driver and the game exits with 222.
    // See `RECOMMENDED_DWPROTON_TAG`.
    let ntoskrnl_abort = log_tail
        .lines()
        .any(|l| l.contains("unimplemented function ntoskrnl.exe.") && l.contains("aborting"));
    if (ntoskrnl_abort || exit_code == Some(ACE_DRIVER_EXIT_CODE)) && is_outdated_dwproton(proton_dir)
    {
        return Some(HINT_DWPROTON_OUTDATED);
    }

    None
}

/// Whether the active Proton directory is a DWProton build older than the
/// recommended one (`.../dwproton-10.0-26-x86_64`). Matched on the path's
/// file name so an unrelated parent directory can't trigger it.
fn is_outdated_dwproton(proton_dir: &str) -> bool {
    let build = std::path::Path::new(proton_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(dwproton_version);
    match (build, dwproton_version(RECOMMENDED_DWPROTON_TAG)) {
        (Some(build), Some(recommended)) => build < recommended,
        _ => false,
    }
}

/// `dwproton-11.0-14[-x86_64]` → `[11, 0, 14]`.
fn dwproton_version(name: &str) -> Option<Vec<u32>> {
    let rest = name.strip_prefix("dwproton-")?;
    let rest = rest.strip_suffix("-x86_64").unwrap_or(rest);
    rest.split(['.', '-']).map(|p| p.parse().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROTON_10: &str = "/home/user/.local/share/llauncher/proton/dwproton-10.0-26-x86_64";
    const PROTON_11_OLD: &str = "/home/user/.local/share/llauncher/proton/dwproton-11.0-11-x86_64";
    const PROTON_11: &str = "/home/user/.local/share/llauncher/proton/dwproton-11.0-14-x86_64";

    #[test]
    fn recognizes_ntoskrnl_stub_aborts_on_old_dwproton_11() {
        // Both stubs seen in the wild (dawn-winery/dwproton#30 and a user
        // report against 11.0-11) must map to the same advice: switch to the
        // recommended build.
        for func in ["PsGetProcessExitStatus", "InbvAcquireDisplayOwnership"] {
            let log = format!(
                "ntsync: up and running.\n\
                 wine: Call from 00006FFFFFBFD947 to unimplemented function ntoskrnl.exe.{}, aborting\n",
                func
            );
            assert_eq!(
                diagnose_launch_failure(&log, PROTON_11_OLD, Some(1)),
                Some(HINT_DWPROTON_OUTDATED)
            );
        }
    }

    #[test]
    fn recognizes_the_ace_driver_exit_on_dwproton_10() {
        assert_eq!(
            diagnose_launch_failure("", PROTON_10, Some(222)),
            Some(HINT_DWPROTON_OUTDATED)
        );
    }

    #[test]
    fn recognizes_an_exhausted_x_server() {
        // The game froze on the intro logo because Xlib could not connect;
        // the ntoskrnl abort on the next line is the fallout, not the cause.
        let log = "gamemodeauto: \n\
                   Maximum number of clients reached\n\
                   wine: Call from 00006FFFFFBFD187 to unimplemented function \
                   ntoskrnl.exe.PsGetProcessExitStatus, aborting\n";
        assert_eq!(
            diagnose_launch_failure(log, PROTON_10, Some(1)),
            Some(HINT_X_CLIENTS_EXHAUSTED)
        );
    }

    #[test]
    fn stays_quiet_without_a_known_signature() {
        // GStreamer plugin warnings and EGL noise are not fatal on their own —
        // no advice must be offered for a log that only contains those.
        let log = "GStreamer-WARNING: Failed to load plugin libgstvpx.so: libvpx.so.9: cannot open\n\
                   libEGL warning: egl: failed to create dri2 screen\n";
        assert_eq!(diagnose_launch_failure(log, PROTON_10, Some(1)), None);
    }

    #[test]
    fn stays_quiet_on_the_recommended_build_and_others() {
        // On the recommended build the same symptoms would be something new —
        // wrong advice is worse than none. Same for a Proton we don't know.
        let log =
            "wine: Call from 0x1 to unimplemented function ntoskrnl.exe.PsGetProcessExitStatus, aborting\n";
        assert_eq!(diagnose_launch_failure(log, PROTON_11, Some(222)), None);
        assert_eq!(diagnose_launch_failure(log, "/opt/GE-Proton10-4", Some(222)), None);
        assert_eq!(diagnose_launch_failure(log, "", Some(222)), None);
    }

    #[test]
    fn compares_dwproton_builds_on_the_directory_name_only() {
        assert!(is_outdated_dwproton("/opt/protons/dwproton-11.0-12-x86_64"));
        assert!(is_outdated_dwproton("/opt/protons/dwproton-10.0-26"));
        assert!(!is_outdated_dwproton("/opt/protons/dwproton-11.0-14-x86_64"));
        assert!(!is_outdated_dwproton("/opt/protons/dwproton-11.1-2-x86_64"));
        assert!(!is_outdated_dwproton("/opt/dwproton-10.0-26/dwproton-11.0-14"));
    }
}
