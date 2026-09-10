//! Windows-only tweaks that live on the host rather than in the game process:
//! the per-executable graphics entry Windows keeps in the registry (which GPU
//! to run on, how windowed frames are presented) and the power plan swapped in
//! for the length of a session.
//!
//! The string handling is platform-neutral and unit-tested everywhere; only
//! the Win32 calls in the `win32` module at the bottom compile on Windows.

use crate::config::settings::AppSettings;

/// The `Key=Value;` list Windows stores per executable under
/// `HKCU\Software\Microsoft\DirectX\UserGpuPreferences` — the same value the
/// Settings > System > Display > Graphics page reads and writes.
#[derive(Debug, Default, PartialEq)]
pub struct GpuPreferences {
    entries: Vec<(String, String)>,
}

impl GpuPreferences {
    pub fn parse(raw: &str) -> Self {
        let entries = raw
            .split(';')
            .filter_map(|item| {
                let (key, value) = item.split_once('=')?;
                let key = key.trim();
                if key.is_empty() {
                    return None;
                }
                Some((key.to_string(), value.trim().to_string()))
            })
            .collect();
        Self { entries }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        match self
            .entries
            .iter_mut()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
        {
            Some(entry) => entry.1 = value.to_string(),
            None => self.entries.push((key.to_string(), value.to_string())),
        }
    }

    /// Remove `key`, but only while it still holds `value`: undoing the
    /// launcher's own entry must not discard a different choice the user has
    /// since made in Windows Settings.
    pub fn unset_if(&mut self, key: &str, value: &str) {
        self.entries
            .retain(|(k, v)| !(k.eq_ignore_ascii_case(key) && v == value));
    }

    /// Back to the `Key=Value;` form, trailing semicolon included, exactly as
    /// Windows writes it.
    pub fn to_registry_string(&self) -> String {
        self.entries
            .iter()
            .map(|(k, v)| format!("{}={};", k, v))
            .collect()
    }
}

/// `GpuPreference` as the Graphics page sets it: 2 is "High performance"
/// (the dedicated GPU), 1 "Power saving", 0 "Let Windows decide".
const GPU_PREFERENCE_HIGH_PERFORMANCE: &str = "2";
/// "Optimizations for windowed games" — Windows 11 22H2 and later present
/// windowed and borderless frames through the flip model when this is 1.
const SWAP_EFFECT_UPGRADE_ON: &str = "1";

/// What the launcher last wrote into the graphics entry, and for which
/// executable. Kept next to the settings file, because the registry cannot
/// say who wrote a value: "High performance" set from Windows Settings looks
/// exactly like "High performance" set from here, and switching a toggle off
/// must never throw away the user's own choice.
#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OwnedEntries {
    /// The executable the keys below were written for. A game directory that
    /// has since moved leaves its old entry alone rather than reaching into
    /// a path this install no longer uses.
    #[serde(default)]
    pub exe: String,
    #[serde(default)]
    pub keys: Vec<String>,
}

impl OwnedEntries {
    fn owns(&self, exe: &str, key: &str) -> bool {
        self.exe.eq_ignore_ascii_case(exe) && self.keys.iter().any(|k| k == key)
    }
}

/// The launcher's toggles as they map onto the entry: one key each, one value
/// each, written only while the toggle is on.
fn owned_keys(settings: &AppSettings) -> [(&'static str, &'static str, bool); 2] {
    [
        (
            "GpuPreference",
            GPU_PREFERENCE_HIGH_PERFORMANCE,
            settings.windows_prefer_dgpu,
        ),
        (
            "SwapEffectUpgradeEnable",
            SWAP_EFFECT_UPGRADE_ON,
            settings.windows_windowed_optimizations,
        ),
    ]
}

/// Fold the launcher's toggles into the existing entry for `exe`, and report
/// what the launcher now owns.
///
/// A toggle that is on writes its key. A toggle that is off removes its key
/// only if `owned` says the launcher put it there — an entry the user set on
/// the Windows graphics page, or anything else living in the same entry
/// (Auto HDR, a "power saving" pick), is left exactly as it was.
pub fn reconcile(
    current: &str,
    owned: &OwnedEntries,
    exe: &str,
    settings: &AppSettings,
) -> (String, OwnedEntries) {
    let mut prefs = GpuPreferences::parse(current);
    let mut now_owned = Vec::new();
    for (key, value, enabled) in owned_keys(settings) {
        if enabled {
            prefs.set(key, value);
            now_owned.push(key.to_string());
        } else if owned.owns(exe, key) {
            prefs.unset_if(key, value);
        }
    }
    (
        prefs.to_registry_string(),
        OwnedEntries {
            exe: exe.to_string(),
            keys: now_owned,
        },
    )
}

/// Windows' built-in "High performance" plan (`GUID_MIN_POWER_SAVINGS`).
pub const HIGH_PERFORMANCE_PLAN: u128 = 0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c;

/// A power-plan GUID the way `powercfg /list` prints it.
pub fn format_guid(guid: u128) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (guid >> 96) as u32,
        (guid >> 80) as u16,
        (guid >> 64) as u16,
        (guid >> 48) as u16,
        guid & 0xffff_ffff_ffff
    )
}

pub fn parse_guid(text: &str) -> Option<u128> {
    let text = text.trim().trim_start_matches('{').trim_end_matches('}');
    if !text.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return None;
    }
    let parts: Vec<&str> = text.split('-').collect();
    let widths = [8usize, 4, 4, 4, 12];
    if parts.len() != widths.len() || parts.iter().zip(widths).any(|(p, w)| p.len() != w) {
        return None;
    }
    u128::from_str_radix(&parts.concat(), 16).ok()
}

#[cfg(windows)]
pub use win32::*;

#[cfg(windows)]
mod win32 {
    use std::path::Path;

    use windows_sys::core::GUID;
    use windows_sys::Win32::Foundation::{LocalFree, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Power::{PowerGetActiveScheme, PowerSetActiveScheme};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    use super::*;
    use crate::config::paths;

    const GPU_PREFERENCES_KEY: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn os_error(code: u32) -> String {
        std::io::Error::from_raw_os_error(code as i32).to_string()
    }

    /// An open registry key, closed on drop.
    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            unsafe { RegCloseKey(self.0) };
        }
    }

    /// Open (creating on a profile that has never touched the Graphics page)
    /// the per-user preferences key. Windows creates the same key itself the
    /// first time the page is opened, so this leaves nothing unusual behind.
    fn open_gpu_preferences_key() -> Result<Key, String> {
        let subkey = wide(GPU_PREFERENCES_KEY);
        let mut handle: HKEY = std::ptr::null_mut();
        let code = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_QUERY_VALUE | KEY_SET_VALUE,
                std::ptr::null(),
                &mut handle,
                std::ptr::null_mut(),
            )
        };
        if code != ERROR_SUCCESS {
            return Err(format!(
                "cannot open HKCU\\{}: {}",
                GPU_PREFERENCES_KEY,
                os_error(code)
            ));
        }
        Ok(Key(handle))
    }

    fn read_string_value(key: &Key, name: &str) -> Result<Option<String>, String> {
        let name = wide(name);
        let mut kind = 0u32;
        let mut size = 0u32;
        let code = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if code == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if code != ERROR_SUCCESS {
            return Err(format!("cannot read the graphics entry: {}", os_error(code)));
        }
        if kind != REG_SZ {
            // Never overwrite something we do not understand.
            return Err(format!("the graphics entry has type {}, not REG_SZ", kind));
        }
        let mut buf = vec![0u16; size as usize / 2 + 1];
        let mut size = (buf.len() * 2) as u32;
        let code = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                buf.as_mut_ptr() as *mut u8,
                &mut size,
            )
        };
        if code != ERROR_SUCCESS {
            return Err(format!("cannot read the graphics entry: {}", os_error(code)));
        }
        let len = (size as usize / 2).min(buf.len());
        let text = String::from_utf16_lossy(&buf[..len]);
        Ok(Some(text.trim_end_matches('\0').to_string()))
    }

    fn write_string_value(key: &Key, name: &str, value: &str) -> Result<(), String> {
        let name = wide(name);
        let data = wide(value);
        let code = unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as *const u8,
                (data.len() * 2) as u32,
            )
        };
        if code != ERROR_SUCCESS {
            return Err(format!("cannot write the graphics entry: {}", os_error(code)));
        }
        Ok(())
    }

    fn delete_value(key: &Key, name: &str) -> Result<(), String> {
        let name = wide(name);
        let code = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
        if code != ERROR_SUCCESS && code != ERROR_FILE_NOT_FOUND {
            return Err(format!("cannot remove the graphics entry: {}", os_error(code)));
        }
        Ok(())
    }

    /// The value name is the executable's full path, backslashes and all: a
    /// forward-slash path (typed by hand into the game directory field) would
    /// register a separate entry Windows never matches against the process.
    fn value_name_for(exe_path: &Path) -> String {
        exe_path.to_string_lossy().replace('/', "\\")
    }

    /// What the launcher last wrote, so a toggle switched off removes the
    /// launcher's own value and nothing else. A missing or unreadable record
    /// means "owns nothing", which errs towards leaving the registry alone.
    fn read_owned_entries() -> OwnedEntries {
        std::fs::read_to_string(paths::graphics_prefs_record_path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn write_owned_entries(owned: &OwnedEntries) {
        let path = paths::graphics_prefs_record_path();
        if owned.keys.is_empty() {
            let _ = std::fs::remove_file(&path);
            return;
        }
        match serde_json::to_string(owned) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, text) {
                    crate::logging::warn(format!(
                        "graphics preferences: cannot record what was written in {}: {}",
                        path.display(),
                        e
                    ));
                }
            }
            Err(e) => crate::logging::warn(format!("graphics preferences: {}", e)),
        }
    }

    /// Bring the game's entry on the Graphics settings page in line with the
    /// launcher's toggles. Runs before every launch, so a moved game directory
    /// simply gets a fresh entry. With both toggles off and nothing else left
    /// in the entry, the entry is deleted: an untouched install leaves no
    /// trace.
    pub fn sync_gpu_preferences(exe_path: &Path, settings: &AppSettings) -> Result<(), String> {
        let value_name = value_name_for(exe_path);
        let owned = read_owned_entries();
        if !settings.windows_prefer_dgpu
            && !settings.windows_windowed_optimizations
            && owned.keys.is_empty()
        {
            // Nothing to write and nothing of ours to take back: leave the
            // registry untouched rather than opening it on every launch.
            return Ok(());
        }

        let key = open_gpu_preferences_key()?;
        let current = read_string_value(&key, &value_name)?.unwrap_or_default();
        let (wanted, now_owned) = reconcile(&current, &owned, &value_name, settings);
        if wanted != current {
            crate::logging::info(format!(
                "graphics preferences for {}: {:?} -> {:?}",
                value_name, current, wanted
            ));
            if wanted.is_empty() {
                delete_value(&key, &value_name)?;
            } else {
                write_string_value(&key, &value_name, &wanted)?;
            }
        }
        if now_owned != owned {
            write_owned_entries(&now_owned);
        }
        Ok(())
    }

    fn guid_to_u128(guid: &GUID) -> u128 {
        ((guid.data1 as u128) << 96)
            | ((guid.data2 as u128) << 80)
            | ((guid.data3 as u128) << 64)
            | (u64::from_be_bytes(guid.data4) as u128)
    }

    fn active_power_plan() -> Result<u128, String> {
        let mut guid_ptr: *mut GUID = std::ptr::null_mut();
        let code = unsafe { PowerGetActiveScheme(std::ptr::null_mut(), &mut guid_ptr) };
        if code != ERROR_SUCCESS || guid_ptr.is_null() {
            return Err(format!(
                "cannot read the active power plan: {}",
                os_error(code)
            ));
        }
        let guid = unsafe { *guid_ptr };
        unsafe { LocalFree(guid_ptr as *mut core::ffi::c_void) };
        Ok(guid_to_u128(&guid))
    }

    fn set_active_power_plan(plan: u128) -> Result<(), String> {
        let guid = GUID::from_u128(plan);
        let code = unsafe { PowerSetActiveScheme(std::ptr::null_mut(), &guid) };
        if code != ERROR_SUCCESS {
            return Err(format!(
                "cannot activate power plan {}: {}",
                format_guid(plan),
                os_error(code)
            ));
        }
        Ok(())
    }

    /// The plan to put back once the session ends.
    pub struct PowerPlanRestore {
        previous: u128,
    }

    impl PowerPlanRestore {
        pub fn restore(self) {
            match set_active_power_plan(self.previous) {
                Ok(()) => crate::logging::info(format!(
                    "power plan: restored {}",
                    format_guid(self.previous)
                )),
                Err(e) => crate::logging::warn(format!("power plan: {}", e)),
            }
            let _ = std::fs::remove_file(paths::power_plan_restore_path());
        }
    }

    /// Switch to the High performance plan for the session. The plan being
    /// replaced is also written next to the settings, so a launcher that does
    /// not outlive the game (tray quit, crash) still puts it back on its next
    /// start — see `restore_leftover_power_plan`.
    ///
    /// `None` when nothing was switched: the plan is already active, or it
    /// does not exist on this machine (OEM images drop it, and modern-standby
    /// laptops ship with Balanced alone). Creating it would need administrator
    /// rights, so that case is logged and left alone.
    pub fn switch_to_high_performance_plan() -> Option<PowerPlanRestore> {
        let previous = match active_power_plan() {
            Ok(plan) => plan,
            Err(e) => {
                crate::logging::warn(format!("power plan: {}", e));
                return None;
            }
        };
        if previous == HIGH_PERFORMANCE_PLAN {
            return None;
        }
        if let Err(e) = set_active_power_plan(HIGH_PERFORMANCE_PLAN) {
            crate::logging::warn(format!(
                "power plan: {} — is the High performance plan present on this machine?",
                e
            ));
            return None;
        }
        let marker = paths::power_plan_restore_path();
        if let Err(e) = std::fs::write(&marker, format_guid(previous)) {
            crate::logging::warn(format!(
                "power plan: cannot record the previous plan in {}: {}",
                marker.display(),
                e
            ));
        }
        crate::logging::info(format!(
            "power plan: High performance for this session, {} comes back afterwards",
            format_guid(previous)
        ));
        Some(PowerPlanRestore { previous })
    }

    /// Startup: put back a plan a previous launcher instance recorded and
    /// never restored.
    pub fn restore_leftover_power_plan() {
        let marker = paths::power_plan_restore_path();
        let Ok(text) = std::fs::read_to_string(&marker) else {
            return;
        };
        let _ = std::fs::remove_file(&marker);
        match parse_guid(&text) {
            Some(plan) => {
                crate::logging::info(
                    "power plan: a previous session left it switched, restoring".to_string(),
                );
                PowerPlanRestore { previous: plan }.restore();
            }
            None => crate::logging::warn(format!(
                "power plan: {} holds no GUID ({:?}), not restoring",
                marker.display(),
                text.trim()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Games\Endfield\Endfield.exe";

    fn settings(dgpu: bool, windowed: bool) -> AppSettings {
        let mut s = AppSettings::default();
        s.windows_prefer_dgpu = dgpu;
        s.windows_windowed_optimizations = windowed;
        s
    }

    fn owning(keys: &[&str]) -> OwnedEntries {
        OwnedEntries {
            exe: EXE.to_string(),
            keys: keys.iter().map(|k| k.to_string()).collect(),
        }
    }

    #[test]
    fn writes_the_entries_windows_expects() {
        let (entry, owned) = reconcile("", &OwnedEntries::default(), EXE, &settings(true, true));
        assert_eq!(entry, "GpuPreference=2;SwapEffectUpgradeEnable=1;");
        assert_eq!(owned, owning(&["GpuPreference", "SwapEffectUpgradeEnable"]));
    }

    #[test]
    fn leaves_other_entries_alone() {
        // Auto HDR was switched on from Windows Settings; the launcher's
        // toggles must only ever touch their own keys, on and off alike.
        let (entry, owned) = reconcile(
            "AutoHDREnable=2097;",
            &OwnedEntries::default(),
            EXE,
            &settings(true, false),
        );
        assert_eq!(entry, "AutoHDREnable=2097;GpuPreference=2;");

        let (entry, _) = reconcile(&entry, &owned, EXE, &settings(false, false));
        assert_eq!(entry, "AutoHDREnable=2097;");
    }

    #[test]
    fn switching_off_takes_back_only_what_the_launcher_wrote() {
        // The same "High performance" value can come from the Windows
        // graphics page, and the registry cannot tell the two apart — so
        // without a record saying the launcher wrote it, it stays.
        let user_set = "GpuPreference=2;";
        let (entry, owned) = reconcile(
            user_set,
            &OwnedEntries::default(),
            EXE,
            &settings(false, false),
        );
        assert_eq!(entry, user_set);
        assert!(owned.keys.is_empty());

        // With the record, the launcher's own value goes.
        let (entry, _) = reconcile(user_set, &owning(&["GpuPreference"]), EXE, &settings(false, false));
        assert_eq!(entry, "");
    }

    #[test]
    fn a_record_for_another_executable_authorizes_nothing() {
        // The game directory moved: the entry for the new path was never
        // written by us, whatever the old one said.
        let owned = OwnedEntries {
            exe: r"D:\Old\Endfield.exe".to_string(),
            keys: vec!["GpuPreference".to_string()],
        };
        let (entry, _) = reconcile("GpuPreference=2;", &owned, EXE, &settings(false, false));
        assert_eq!(entry, "GpuPreference=2;");
    }

    #[test]
    fn switching_off_keeps_a_choice_the_user_made_elsewhere() {
        // "Power saving" was picked on the Graphics page. The launcher never
        // writes 1, so an off toggle has nothing of its own to remove even
        // when it owns the key.
        let (entry, _) = reconcile(
            "GpuPreference=1;",
            &owning(&["GpuPreference"]),
            EXE,
            &settings(false, false),
        );
        assert_eq!(entry, "GpuPreference=1;");
    }

    #[test]
    fn switching_on_overrides_whatever_was_there() {
        let (entry, _) = reconcile(
            "GpuPreference=1;SwapEffectUpgradeEnable=0;",
            &OwnedEntries::default(),
            EXE,
            &settings(true, true),
        );
        assert_eq!(entry, "GpuPreference=2;SwapEffectUpgradeEnable=1;");
    }

    #[test]
    fn an_emptied_entry_comes_back_empty() {
        // That is the cue for the registry value to be deleted outright.
        let (entry, owned) = reconcile(
            "GpuPreference=2;",
            &owning(&["GpuPreference"]),
            EXE,
            &settings(false, false),
        );
        assert_eq!(entry, "");
        assert!(owned.keys.is_empty());
    }

    #[test]
    fn parsing_tolerates_sloppy_input() {
        let prefs = GpuPreferences::parse(" GpuPreference = 2 ;;garbage; =1;");
        assert_eq!(prefs.to_registry_string(), "GpuPreference=2;");
    }

    #[test]
    fn keys_match_case_insensitively() {
        let mut prefs = GpuPreferences::parse("gpupreference=2;");
        prefs.unset_if("GpuPreference", "2");
        assert_eq!(prefs.to_registry_string(), "");
    }

    #[test]
    fn guid_round_trips_in_powercfg_format() {
        assert_eq!(
            format_guid(HIGH_PERFORMANCE_PLAN),
            "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"
        );
        assert_eq!(
            parse_guid("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"),
            Some(HIGH_PERFORMANCE_PLAN)
        );
        // Braces, upper case and a trailing newline are how a hand-edited or
        // `powercfg`-pasted marker file might look.
        assert_eq!(
            parse_guid("{381B4222-F694-41F0-9685-FF5BB260DF2E}\n"),
            Some(0x381b4222_f694_41f0_9685_ff5bb260df2e)
        );
        assert_eq!(parse_guid(""), None);
        assert_eq!(parse_guid("not-a-guid"), None);
        assert_eq!(parse_guid("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635"), None);
        assert_eq!(parse_guid("+c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"), None);
    }
}
