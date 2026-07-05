//! Persistent app settings.
//!
//! Storage strategy:
//! - Portable builds (exe directory is writable): `settings.json` beside the
//!   executable, so the whole app travels as one folder. No registry or
//!   profile writes.
//! - Installed builds on Windows (exe in Program Files, not writable):
//!   `HKCU\Software\FindBT` registry values. No settings.json anywhere.
//! - Installed builds on macOS (no registry exists): the platform-standard
//!   `~/Library/Application Support/FindBT/settings.json`.
//!
//! Hardening: a settings.json is only honored when it passes ALL of these
//! checks, otherwise built-in defaults are used and the app carries on:
//! - file no larger than `MAX_SETTINGS_BYTES`
//! - valid JSON matching the exact schema (unknown fields rejected)
//! - `app` marker equals `"FindBT"` and `settings_version` is supported
//! - every value is a closed enum; free-form strings are never trusted
//!
//! Registry values get the same treatment: version checked, theme parsed as
//! a closed enum, anything unexpected falls back to defaults.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const SETTINGS_FILE_NAME: &str = "settings.json";
const APP_MARKER: &str = "FindBT";
const SETTINGS_VERSION: u32 = 1;
/// A valid settings file is a few hundred bytes; anything bigger is not ours.
const MAX_SETTINGS_BYTES: u64 = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSetting {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeSetting {
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn as_str(self) -> &'static str {
        match self {
            ThemeSetting::System => "system",
            ThemeSetting::Light => "light",
            ThemeSetting::Dark => "dark",
        }
    }

    fn from_str_strict(value: &str) -> Option<ThemeSetting> {
        match value {
            "system" => Some(ThemeSetting::System),
            "light" => Some(ThemeSetting::Light),
            "dark" => Some(ThemeSetting::Dark),
            _ => None,
        }
    }
}

/// The in-memory settings the app works with. Always valid.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AppSettings {
    pub theme: ThemeSetting,
}

/// On-disk representation. Kept separate from `AppSettings` so the file
/// schema is explicit and strictly validated.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsFile {
    app: String,
    settings_version: u32,
    theme: ThemeSetting,
}

impl AppSettings {
    /// Load settings, falling back to defaults on any problem whatsoever.
    /// Missing, oversized, malformed, or foreign settings must never break
    /// the app.
    pub fn load() -> AppSettings {
        if let Some(path) = portable_settings_path() {
            return try_load_file(&path).unwrap_or_default();
        }
        installed_load().unwrap_or_default()
    }

    /// Persist settings. Best-effort: failures are swallowed; settings
    /// simply won't stick, which is preferable to interrupting a capture.
    pub fn save(self) {
        if let Some(path) = portable_settings_path() {
            save_to_file(self, &path);
            return;
        }
        installed_save(self);
    }
}

/// Portable mode: the exe directory is writable, so settings live beside the
/// exe. Installed layouts (Program Files, /Applications) are not writable by
/// a normal user, which routes them to the installed persistence instead.
fn portable_settings_path() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))?;
    if dir_writable(&exe_dir) {
        Some(exe_dir.join(SETTINGS_FILE_NAME))
    } else {
        None
    }
}

fn dir_writable(dir: &Path) -> bool {
    let probe = dir.join(".findbt-write-probe");
    let _ = fs::remove_file(&probe);
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn try_load_file(path: &Path) -> Option<AppSettings> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_SETTINGS_BYTES {
        return None;
    }
    let raw = fs::read_to_string(path).ok()?;
    let file: SettingsFile = serde_json::from_str(&raw).ok()?;
    if file.app != APP_MARKER || file.settings_version != SETTINGS_VERSION {
        return None;
    }
    Some(AppSettings { theme: file.theme })
}

fn save_to_file(settings: AppSettings, path: &Path) {
    let file = SettingsFile {
        app: APP_MARKER.to_string(),
        settings_version: SETTINGS_VERSION,
        theme: settings.theme,
    };
    let Ok(json) = serde_json::to_string_pretty(&file) else {
        return;
    };
    if let Some(dir) = path.parent() {
        if fs::create_dir_all(dir).is_err() {
            return;
        }
    }
    // Write to a temp file then rename, so a crash mid-write can never
    // leave a truncated settings.json behind.
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, &json).is_ok() && fs::rename(&tmp, path).is_ok() {
        return;
    }
    let _ = fs::remove_file(&tmp);
}

// ---------------------------------------------------------------------------
// Installed persistence: Windows registry.
// ---------------------------------------------------------------------------

#[cfg(windows)]
const REGISTRY_SUBKEY: &str = "Software\\FindBT";

#[cfg(windows)]
fn installed_load() -> Option<AppSettings> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(REGISTRY_SUBKEY)
        .ok()?;
    let version: u32 = key.get_value("settings_version").ok()?;
    if version != SETTINGS_VERSION {
        return None;
    }
    let theme: String = key.get_value("theme").ok()?;
    Some(AppSettings {
        theme: ThemeSetting::from_str_strict(&theme)?,
    })
}

#[cfg(windows)]
fn installed_save(settings: AppSettings) {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let Ok((key, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(REGISTRY_SUBKEY) else {
        return;
    };
    let _ = key.set_value("settings_version", &SETTINGS_VERSION);
    let _ = key.set_value("theme", &settings.theme.as_str());
}

// ---------------------------------------------------------------------------
// Installed persistence: macOS and other unix (no registry exists there, so
// the platform-standard per-user config location is used, with the same
// hardened file validation as the portable path).
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
fn installed_load() -> Option<AppSettings> {
    try_load_file(&user_config_dir()?.join(SETTINGS_FILE_NAME))
}

#[cfg(not(windows))]
fn installed_save(settings: AppSettings) {
    if let Some(dir) = user_config_dir() {
        save_to_file(settings, &dir.join(SETTINGS_FILE_NAME));
    }
}

#[cfg(target_os = "macos")]
fn user_config_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|value| {
        PathBuf::from(value)
            .join("Library")
            .join("Application Support")
            .join(APP_MARKER)
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn user_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|base| base.join(APP_MARKER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_foreign_and_malformed_files() {
        let dir = std::env::temp_dir().join("findbt-settings-test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("settings.json");

        // Wrong app marker.
        fs::write(
            &path,
            r#"{"app":"OtherApp","settings_version":1,"theme":"dark"}"#,
        )
        .unwrap();
        assert!(try_load_file(&path).is_none());

        // Unknown field.
        fs::write(
            &path,
            r#"{"app":"FindBT","settings_version":1,"theme":"dark","extra":true}"#,
        )
        .unwrap();
        assert!(try_load_file(&path).is_none());

        // Unsupported version.
        fs::write(
            &path,
            r#"{"app":"FindBT","settings_version":99,"theme":"dark"}"#,
        )
        .unwrap();
        assert!(try_load_file(&path).is_none());

        // Not JSON at all.
        fs::write(&path, "definitely not json").unwrap();
        assert!(try_load_file(&path).is_none());

        // Valid file round-trips.
        fs::write(
            &path,
            r#"{"app":"FindBT","settings_version":1,"theme":"dark"}"#,
        )
        .unwrap();
        assert_eq!(
            try_load_file(&path),
            Some(AppSettings {
                theme: ThemeSetting::Dark
            })
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn theme_strings_are_a_closed_set() {
        assert_eq!(
            ThemeSetting::from_str_strict("system"),
            Some(ThemeSetting::System)
        );
        assert_eq!(
            ThemeSetting::from_str_strict("light"),
            Some(ThemeSetting::Light)
        );
        assert_eq!(
            ThemeSetting::from_str_strict("dark"),
            Some(ThemeSetting::Dark)
        );
        assert_eq!(ThemeSetting::from_str_strict("DARK"), None);
        assert_eq!(ThemeSetting::from_str_strict(""), None);
        assert_eq!(ThemeSetting::from_str_strict("../evil"), None);
    }
}
