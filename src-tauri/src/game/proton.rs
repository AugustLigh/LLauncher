use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct SystemCheck {
    pub has_proton: bool,
    pub has_ntsync: bool,
    pub has_gamemode: bool,
    pub has_mangohud: bool,
    pub proton_path: String,
}

pub fn check_system(proton_dir: &str) -> SystemCheck {
    let has_proton = if proton_dir.is_empty() {
        false
    } else {
        Path::new(proton_dir).join("proton").exists()
    };

    SystemCheck {
        has_proton,
        has_ntsync: check_ntsync(),
        has_gamemode: check_command("gamemoderun"),
        has_mangohud: check_command("mangohud"),
        proton_path: if has_proton {
            Path::new(proton_dir)
                .join("proton")
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        },
    }
}

fn check_ntsync() -> bool {
    Path::new("/dev/ntsync").exists()
}

fn check_command(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
