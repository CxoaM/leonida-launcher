use crate::error::AppResult;
use crate::leonida::process::JobRuntime;
use crate::logging::LogManager;
use crate::paths::WorkspacePaths;
use crate::settings::{self, Settings};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

/// Everything a command needs. One mutex guards the settings document so two
/// pages cannot write conflicting files; long-running work lives in `JobRuntime`
/// instances that keep their own lock only for short snapshots.
pub struct AppState {
    pub paths: WorkspacePaths,
    pub settings: Mutex<Settings>,
    pub logs: LogManager,
    pub build: JobRuntime,
    pub install: JobRuntime,
    pub dependency_install: JobRuntime,
}

impl AppState {
    pub fn new(app: &AppHandle) -> AppResult<Self> {
        let paths = WorkspacePaths::resolve(app)?;
        paths.ensure()?;
        let settings = settings::load(std::path::Path::new(&paths.config_dir), &paths);

        let logs = LogManager::new();
        logs.init(
            std::path::Path::new(&paths.logs_dir),
            settings.log_retention_days,
            app.clone(),
        )?;
        logs.set_verbose(settings.verbose_logging);
        logs.info(crate::logging::Category::Launcher, "Launcher started");

        Ok(Self {
            paths,
            settings: Mutex::new(settings),
            logs,
            build: JobRuntime::new("build"),
            install: JobRuntime::new("install"),
            dependency_install: JobRuntime::new("dependency"),
        })
    }

    pub fn settings(&self) -> Settings {
        self.settings
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| Settings::defaults_for(&self.paths))
    }

    pub fn replace_settings(&self, mut next: Settings) -> Settings {
        next.normalize(&self.paths);
        if let Ok(mut guard) = self.settings.lock() {
            *guard = next.clone();
        }
        next
    }

    pub fn log(&self) -> &LogManager {
        &self.logs
    }
}

/// Resolve the state from a command that already owns an `AppHandle`. Keeping the
/// lookup in one place makes background tasks behave exactly like commands.
pub fn state_of(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}
