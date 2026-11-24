use crate::commands::installation;
use crate::error::{AppError, AppResult};
use crate::launcher_meta;
use crate::leonida::dependencies;
use crate::leonida::repository::GitRepo;
use crate::paths::{disk_space as read_disk_space, DiskSpace, WorkspacePaths};
use crate::settings::{self, PathProbe, Settings};
use crate::state::AppState;
use crate::updater;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub installation: crate::commands::installation::InstallationState,
    pub compatibility: crate::leonida::compatibility::CompatibilityReport,
    pub dependencies: Vec<crate::leonida::dependencies::DependencyReport>,
    pub environment: crate::diagnostics::EnvironmentInfo,
    pub release: crate::updater::LauncherRelease,
    pub build: crate::leonida::process::JobSnapshot,
    pub install: crate::leonida::process::JobSnapshot,
    pub disks: Vec<DiskSpace>,
    pub metadata: launcher_meta::LauncherMeta,
}

#[tauri::command]
pub fn settings_load(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    let previous = state.settings();
    let saved = state.replace_settings(settings);
    crate::settings::save(std::path::Path::new(&state.paths.config_dir), &saved)?;

    if previous.launch_on_startup != saved.launch_on_startup {
        if let Err(err) = crate::settings::set_autostart(saved.launch_on_startup) {
            state.logs.warn(
                crate::logging::Category::Launcher,
                format!("Startup registration failed: {}", err.message),
            );
        }
    }
    state.logs.set_verbose(saved.verbose_logging);
    state.logs.info(
        crate::logging::Category::Launcher,
        format!(
            "Settings saved (language={}, jobs={}, autoCheck={})",
            saved.language,
            saved.effective_jobs(),
            saved.leonida_auto_check
        ),
    );
    let _ = app.emit("settings://changed", &saved);
    Ok(saved)
}

#[tauri::command]
pub fn settings_reset(app: AppHandle, state: State<'_, AppState>) -> AppResult<Settings> {
    let defaults = Settings::reset_keeping_workspace(&state.settings(), &state.paths);
    let saved = state.replace_settings(defaults);
    crate::settings::save(std::path::Path::new(&state.paths.config_dir), &saved)?;
    let _ = crate::settings::set_autostart(saved.launch_on_startup);
    state.logs.set_verbose(saved.verbose_logging);
    state.logs.info(
        crate::logging::Category::Launcher,
        "Settings reset to defaults",
    );
    let _ = app.emit("settings://changed", &saved);
    Ok(saved)
}

#[tauri::command]
pub fn paths_info(state: State<'_, AppState>) -> WorkspacePaths {
    state.paths.clone()
}

#[tauri::command]
pub fn environment_info(state: State<'_, AppState>) -> crate::diagnostics::EnvironmentInfo {
    crate::diagnostics::environment_info(&state.paths)
}

#[tauri::command]
pub fn app_metadata() -> launcher_meta::LauncherMeta {
    launcher_meta::meta()
}

#[tauri::command]
pub fn probe_path(path: String, create: Option<bool>) -> PathProbe {
    settings::probe_directory(&path, create.unwrap_or(false))
}

#[tauri::command]
pub fn disk_space(path: String) -> AppResult<DiskSpace> {
    let expanded = crate::paths::expand_home(&path);
    read_disk_space(&expanded)
}

/// Opens a folder in Explorer. Deliberately a dedicated command instead of a
/// shell call so the path is one argv value and can never be interpreted.
#[tauri::command]
pub fn reveal_path(path: String) -> AppResult<()> {
    if path.trim().is_empty() {
        return Err(AppError::new("invalid_path", "No folder was provided"));
    }
    let target = crate::paths::expand_home(&path);
    if !target.exists() {
        return Err(
            AppError::new("path_missing", "That folder no longer exists")
                .with_detail(target.to_string_lossy().to_string()),
        );
    }
    crate::process::reveal_in_explorer(&target)
}

#[tauri::command]
pub async fn workspace_summary(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<WorkspaceSummary> {
    let settings = state.settings();
    let paths = state.paths.clone();
    let repository = settings.repository_dir();

    let installation = installation::collect(&app).await;
    let compatibility = match GitRepo::detect(&repository) {
        Some(repo) => match repo.status().await {
            Ok(status) => crate::leonida::compatibility::evaluate(&repository, &status).await,
            Err(err) => crate::leonida::compatibility::CompatibilityReport::unknown(err.message),
        },
        None => crate::leonida::compatibility::CompatibilityReport::unknown(
            "Leonida is not installed yet. Run the installation wizard to fetch it from the official repository.",
        ),
    };
    let deps = dependencies::detect_all(
        &crate::leonida::compatibility::spec()
            .requirements
            .windows_sdk,
    )
    .await;

    let mut disks = Vec::new();
    for path in [&repository, &settings.out_dir(), &settings.work_dir()] {
        if let Ok(space) = read_disk_space(path) {
            if !disks.iter().any(|d: &DiskSpace| d.volume == space.volume) {
                disks.push(space);
            }
        }
    }

    Ok(WorkspaceSummary {
        installation,
        compatibility,
        dependencies: deps,
        environment: crate::diagnostics::environment_info(&paths),
        release: updater::info(),
        build: state.build.snapshot(),
        install: state.install.snapshot(),
        disks,
        metadata: launcher_meta::meta(),
    })
}
