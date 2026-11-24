use crate::error::{AppError, AppResult};
use crate::paths::{disk_space, expand_home, WorkspacePaths};
use crate::storage;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SETTINGS_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AdvancedSettings {
    /// Allow installing a revision that differs from the verified baseline after
    /// an explicit confirmation. Never used to skip structural checks.
    pub allow_unverified_upstream: bool,
    /// How many launcher-backup refs to keep for rollback.
    pub keep_restore_points: u32,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            allow_unverified_upstream: false,
            keep_restore_points: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    pub theme: String,
    pub language: String,
    pub launch_on_startup: bool,
    pub check_launcher_updates_on_start: bool,
    pub leonida_auto_check: bool,
    pub leonida_notifications: bool,
    pub leonida_check_interval_minutes: u32,
    pub repository_path: Option<String>,
    pub game_path: Option<String>,
    pub out_path: String,
    pub work_path: String,
    pub cache_path: String,
    pub jobs: u32,
    pub wave64_emulate: String,
    pub log_retention_days: u32,
    pub verbose_logging: bool,
    pub advanced: AdvancedSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA,
            theme: "dark".into(),
            language: "system".into(),
            launch_on_startup: false,
            check_launcher_updates_on_start: true,
            leonida_auto_check: true,
            leonida_notifications: true,
            leonida_check_interval_minutes: 360,
            repository_path: None,
            game_path: None,
            out_path: String::new(),
            work_path: String::new(),
            cache_path: String::new(),
            jobs: 0,
            wave64_emulate: "auto".into(),
            log_retention_days: 14,
            verbose_logging: false,
            advanced: AdvancedSettings::default(),
        }
    }
}

impl Settings {
    pub fn defaults_for(paths: &WorkspacePaths) -> Self {
        Self {
            repository_path: Some(paths.default_repository.clone()),
            out_path: paths.default_out.clone(),
            work_path: paths.default_work.clone(),
            cache_path: paths.default_cache.clone(),
            ..Self::default()
        }
    }

    /// Reset means "forget my preferences", not "forget where my workspace is".
    /// Reverting the checkout to the default location would make an existing
    /// installation look missing, so the paths survive a reset untouched.
    pub fn reset_keeping_workspace(current: &Settings, paths: &WorkspacePaths) -> Self {
        Self {
            repository_path: current.repository_path.clone(),
            game_path: current.game_path.clone(),
            out_path: current.out_path.clone(),
            work_path: current.work_path.clone(),
            cache_path: current.cache_path.clone(),
            ..Self::defaults_for(paths)
        }
    }

    /// Repair anything a hand-edited settings file could have broken.
    pub fn normalize(&mut self, paths: &WorkspacePaths) {
        self.schema_version = SETTINGS_SCHEMA;
        self.theme = "dark".into();
        if !matches!(self.language.as_str(), "en" | "ru" | "system") {
            self.language = "system".into();
        }
        if !matches!(self.wave64_emulate.as_str(), "auto" | "on" | "off") {
            self.wave64_emulate = "auto".into();
        }
        self.jobs = self.jobs.min(256);
        self.log_retention_days = self.log_retention_days.clamp(1, 365);
        self.leonida_check_interval_minutes = self.leonida_check_interval_minutes.clamp(15, 10_080);
        self.advanced.keep_restore_points = self.advanced.keep_restore_points.clamp(1, 25);
        if self.out_path.trim().is_empty() {
            self.out_path = paths.default_out.clone();
        }
        if self.work_path.trim().is_empty() {
            self.work_path = paths.default_work.clone();
        }
        if self.cache_path.trim().is_empty() {
            self.cache_path = paths.default_cache.clone();
        }
        if self
            .repository_path
            .as_deref()
            .map(|p| p.trim().is_empty())
            .unwrap_or(true)
        {
            self.repository_path = Some(paths.default_repository.clone());
        }
        if let Some(game) = self.game_path.clone() {
            if game.trim().is_empty() {
                self.game_path = None;
            }
        }
    }

    pub fn repository_dir(&self) -> PathBuf {
        expand_home(self.repository_path.as_deref().unwrap_or_default())
    }

    pub fn out_dir(&self) -> PathBuf {
        expand_home(&self.out_path)
    }

    pub fn work_dir(&self) -> PathBuf {
        expand_home(&self.work_path)
    }

    pub fn effective_jobs(&self) -> u32 {
        if self.jobs > 0 {
            return self.jobs;
        }
        (std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            / 2)
        .max(1) as u32
    }
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

pub fn load(config_dir: &Path, paths: &WorkspacePaths) -> Settings {
    match storage::read_json::<Settings>(&settings_path(config_dir)) {
        Ok(Some(mut settings)) => {
            settings.normalize(paths);
            settings
        }
        Ok(None) => Settings::defaults_for(paths),
        Err(_) => Settings::defaults_for(paths),
    }
}

pub fn save(config_dir: &Path, settings: &Settings) -> AppResult<()> {
    storage::write_json(&settings_path(config_dir), settings)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathProbe {
    pub path: String,
    pub exists: bool,
    pub is_directory: bool,
    pub writable: Option<bool>,
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
}

/// Check a folder without creating it unless asked, so the wizard can show the
/// truth about a path the user typed.
pub fn probe_directory(input: &str, create: bool) -> PathProbe {
    let path = expand_home(input);
    let mut probe = PathProbe {
        path: path.to_string_lossy().to_string(),
        exists: path.exists(),
        is_directory: path.is_dir(),
        writable: None,
        free_bytes: None,
        total_bytes: None,
        error: None,
    };
    if !probe.exists && create {
        match std::fs::create_dir_all(crate::paths::fs_path(&path)) {
            Ok(()) => {
                probe.exists = true;
                probe.is_directory = true;
            }
            Err(err) => probe.error = Some(err.to_string()),
        }
    }
    if probe.is_directory {
        probe.writable = Some(probe_write_access(&path));
    }
    if let Ok(space) = disk_space(&path) {
        probe.free_bytes = Some(space.free_bytes);
        probe.total_bytes = Some(space.total_bytes);
    }
    probe
}

pub fn probe_write_access(dir: &Path) -> bool {
    let marker = dir.join(format!(".leonida-write-test-{}", std::process::id()));
    match std::fs::write(crate::paths::fs_path(&marker), b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(crate::paths::fs_path(&marker));
            true
        }
        Err(_) => false,
    }
}

#[cfg(windows)]
fn autostart_key() -> AppResult<winreg::RegKey> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};
    use winreg::RegKey;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            KEY_ALL_ACCESS,
        )
        .map_err(|e| {
            AppError::new("autostart", "Cannot open the Windows startup registry key")
                .with_detail(e.to_string())
        })
}

#[cfg(windows)]
pub fn set_autostart(enabled: bool) -> AppResult<()> {
    const VALUE: &str = "Leonida Launcher";
    let key = autostart_key()?;
    if enabled {
        let exe = std::env::current_exe()
            .map_err(|e| AppError::io("Locating the launcher executable", e))?;
        let value = format!("\"{}\"", exe.to_string_lossy());
        key.set_value(VALUE, &value).map_err(|e| {
            AppError::new("autostart", "Cannot register the launcher for startup")
                .with_detail(e.to_string())
        })?;
    } else {
        let _ = key.delete_value(VALUE);
    }
    Ok(())
}

#[cfg(windows)]
pub fn autostart_enabled() -> bool {
    const VALUE: &str = "Leonida Launcher";
    autostart_key()
        .and_then(|key| {
            key.get_value::<String, _>(VALUE)
                .map_err(|e| AppError::new("autostart", e.to_string()))
        })
        .is_ok()
}

#[cfg(not(windows))]
pub fn set_autostart(_enabled: bool) -> AppResult<()> {
    Err(AppError::new(
        "unsupported_platform",
        "Startup registration is Windows-only",
    ))
}

#[cfg(not(windows))]
pub fn autostart_enabled() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> WorkspacePaths {
        WorkspacePaths {
            config_dir: r"C:\cfg".into(),
            data_dir: r"C:\data".into(),
            cache_dir: r"C:\data\cache".into(),
            logs_dir: r"C:\cfg\logs".into(),
            documents_dir: r"C:\Users\me\Documents".into(),
            default_repository: r"C:\data\workspace\repository".into(),
            default_out: r"C:\Users\me\Documents\Leonida".into(),
            default_work: r"C:\data\workspace\work".into(),
            default_cache: r"C:\data\cache".into(),
        }
    }

    #[test]
    fn defaults_point_outside_the_repository() {
        let settings = Settings::defaults_for(&paths());
        assert_eq!(
            settings.repository_path.as_deref(),
            Some(r"C:\data\workspace\repository")
        );
        assert!(!settings.out_dir().starts_with(settings.repository_dir()));
        assert!(!settings.work_dir().starts_with(settings.repository_dir()));
    }

    #[test]
    fn normalization_repairs_hostile_values() {
        let mut settings = Settings {
            language: "klingon".into(),
            wave64_emulate: "maybe".into(),
            jobs: 9999,
            log_retention_days: 0,
            out_path: "  ".into(),
            work_path: String::new(),
            repository_path: Some("".into()),
            ..Settings::default()
        };
        settings.normalize(&paths());
        assert_eq!(settings.language, "system");
        assert_eq!(settings.wave64_emulate, "auto");
        assert_eq!(settings.jobs, 256);
        assert_eq!(settings.log_retention_days, 1);
        assert_eq!(settings.out_path, paths().default_out);
        assert_eq!(settings.repository_path, Some(paths().default_repository));
    }

    #[test]
    fn effective_jobs_never_zero() {
        let settings = Settings::default();
        assert!(settings.effective_jobs() >= 1);
        let explicit = Settings {
            jobs: 12,
            ..Settings::default()
        };
        assert_eq!(explicit.effective_jobs(), 12);
    }

    #[test]
    fn probe_reports_missing_without_creating() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("not-here");
        let probe = probe_directory(&target.to_string_lossy(), false);
        assert!(!probe.exists);
        assert!(!target.exists());
        let created = probe_directory(&target.to_string_lossy(), true);
        assert!(created.exists);
        assert_eq!(created.writable, Some(true));
    }

    #[test]
    fn reset_keeps_the_workspace_paths() {
        let paths = paths();
        let current = Settings {
            language: "ru".into(),
            launch_on_startup: true,
            repository_path: Some(r"D:\Leonida checkout".into()),
            game_path: Some(r"E:\Games\Leonida".into()),
            out_path: r"D:\Leonida out".into(),
            work_path: r"D:\Leonida work".into(),
            jobs: 12,
            ..Settings::defaults_for(&paths)
        };
        let reset = Settings::reset_keeping_workspace(&current, &paths);
        assert_eq!(reset.language, "system");
        assert!(!reset.launch_on_startup);
        assert_eq!(reset.jobs, 0);
        assert_eq!(
            reset.repository_path.as_deref(),
            Some(r"D:\Leonida checkout")
        );
        assert_eq!(reset.game_path.as_deref(), Some(r"E:\Games\Leonida"));
        assert_eq!(reset.out_path, r"D:\Leonida out");
        assert_eq!(reset.work_path, r"D:\Leonida work");
        assert_eq!(reset.cache_path, paths.default_cache);
    }

    #[test]
    fn settings_round_trip_through_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let mut settings = Settings::defaults_for(&paths());
        settings.jobs = 7;
        settings.game_path = Some(r"D:\GTA VI".into());
        save(tmp.path(), &settings).unwrap();
        let loaded = load(tmp.path(), &paths());
        assert_eq!(loaded.jobs, 7);
        assert_eq!(loaded.game_path.as_deref(), Some(r"D:\GTA VI"));
    }
}
