use crate::error::{AppError, AppResult};
use crate::leonida::build::WorkState;
use crate::leonida::compatibility::{self, CompatibilityStatus};
use crate::leonida::detector;
use crate::leonida::process::JobStatus;
use crate::leonida::repository::{GitRepo, RepoStatus, SubmoduleState};
use crate::leonida::updater::RestorePoint;
use crate::leonida::validator::{validate_game_dir as validate_game_dir_impl, GameValidation};
use crate::process::{kill_tree, resolve_executable};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallEvent {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<crate::leonida::process::JobSnapshot>,
}

fn emit(app: &AppHandle, event: InstallEvent) {
    let _ = app.emit("install://event", event);
}

fn step(app: &AppHandle, id: &str, status: &str, detail: impl Into<String>) {
    let detail = detail.into();
    emit(
        app,
        InstallEvent {
            kind: "step".into(),
            step: Some(id.into()),
            status: Some(status.into()),
            detail: if detail.is_empty() {
                None
            } else {
                Some(detail)
            },
            line: None,
            message: None,
            snapshot: None,
        },
    );
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationState {
    pub installed: bool,
    pub repository_path: String,
    pub repository_exists: bool,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub short_commit: Option<String>,
    pub commit_subject: Option<String>,
    pub commit_date: Option<String>,
    pub dirty_files: usize,
    pub untracked_files: usize,
    pub submodules: Vec<SubmoduleState>,
    pub submodules_ready: bool,
    pub game_path: Option<String>,
    pub game_valid: bool,
    pub work_state: WorkState,
    pub restore_point: Option<RestorePoint>,
    pub git_available: bool,
}

fn short(sha: &str) -> String {
    sha.chars().take(8).collect()
}

/// Read-only view used by the Home, Installation and Diagnostics pages. It never
/// writes to the repository.
pub async fn collect(app: &AppHandle) -> InstallationState {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let repository = settings.repository_dir();
    let repo = GitRepo::new(repository.clone());

    let status = match GitRepo::detect(&repository) {
        Some(_) => repo.status().await.unwrap_or_else(|err| {
            let mut empty = RepoStatus::empty(repository.to_string_lossy().to_string());
            empty.exists = repository.exists();
            empty.error = Some(err.message);
            empty
        }),
        None => {
            let mut empty = RepoStatus::empty(repository.to_string_lossy().to_string());
            empty.exists = repository.exists();
            empty
        }
    };

    let submodules: Vec<SubmoduleState> = if status.is_repository {
        repo.submodules().await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let submodules_ready = compatibility::spec()
        .expected
        .submodules
        .iter()
        .all(|expected| {
            if !expected.required {
                return true;
            }
            submodules
                .iter()
                .any(|found| found.path == expected.path && found.initialized)
        });

    let work_state = crate::leonida::build::inspect_work(&settings.work_dir(), &settings.out_dir());

    let (game_path, game_valid) = match settings.game_path.clone() {
        Some(path) if !path.trim().is_empty() => {
            let validation = validate_game_dir_impl(&crate::paths::expand_home(&path));
            (Some(path), validation.valid)
        }
        _ => (None, false),
    };

    let (commit_subject, commit_date) = match status.head.as_deref() {
        Some(sha) => match repo.commit_info(sha).await {
            Ok(Some(info)) => (Some(info.subject), Some(info.date_human)),
            _ => (None, None),
        },
        None => (None, None),
    };

    InstallationState {
        installed: status.is_repository,
        repository_path: status.path.clone(),
        repository_exists: repository.exists(),
        branch: status.branch.clone(),
        commit: status.head.clone(),
        short_commit: status.head.as_deref().map(short),
        commit_subject,
        commit_date,
        dirty_files: status.modified_files,
        untracked_files: status.untracked_files,
        submodules,
        submodules_ready,
        game_path,
        game_valid,
        work_state,
        restore_point: crate::leonida::updater::read_restore_point(std::path::Path::new(
            &state.paths.config_dir,
        )),
        git_available: resolve_executable("git").is_some(),
    }
}

#[tauri::command]
pub async fn installation_state(app: AppHandle) -> InstallationState {
    collect(&app).await
}

#[tauri::command]
pub async fn detect_game_folders() -> Vec<detector::GameCandidate> {
    tauri::async_runtime::spawn_blocking(|| {
        detector::detect_game_folders(detector::default_game_roots())
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub fn validate_game_dir(path: String) -> GameValidation {
    validate_game_dir_impl(&crate::paths::expand_home(&path))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOptions {
    pub repository_path: String,
    pub game_path: Option<String>,
    pub out_path: String,
    pub work_path: String,
    #[serde(default)]
    pub skip_compatibility: bool,
}

fn prepare_paths(
    options: &InstallOptions,
) -> AppResult<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let repository = crate::paths::expand_home(&options.repository_path);
    let out = crate::paths::expand_home(&options.out_path);
    let work = crate::paths::expand_home(&options.work_path);
    if options.repository_path.trim().is_empty() {
        return Err(AppError::new(
            "invalid_paths",
            "Choose where Leonida should be installed",
        ));
    }
    if options.out_path.trim().is_empty() || options.work_path.trim().is_empty() {
        return Err(AppError::new(
            "invalid_paths",
            "The output and work folders are required",
        ));
    }
    if out == work {
        return Err(AppError::new(
            "invalid_paths",
            "The output folder and the work folder must be different",
        ));
    }
    if crate::paths::path_is_within(&out, &repository)
        || crate::paths::path_is_within(&work, &repository)
    {
        return Err(AppError::new(
            "invalid_paths",
            "Build output must live outside the Leonida checkout so an update never touches it",
        ));
    }
    Ok((repository, out, work))
}

#[tauri::command]
pub fn install_leonida(
    app: AppHandle,
    options: InstallOptions,
) -> AppResult<crate::leonida::process::JobSnapshot> {
    let (repository, out, work) = prepare_paths(&options)?;
    let state = app.state::<AppState>();

    if GitRepo::detect(&repository).is_some() {
        return Err(AppError::new(
            "already_installed",
            "A git checkout already exists at that path",
        )
        .with_detail("Use the Updates page to refresh it, or pick an empty folder."));
    }
    if repository.exists() {
        let occupied = std::fs::read_dir(crate::paths::fs_path(&repository))
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);
        if occupied {
            return Err(
                AppError::new("folder_not_empty", "The chosen folder is not empty").with_detail(
                    "Pick an empty folder so the launcher never overwrites your files.",
                ),
            );
        }
    }

    let url = compatibility::spec().upstream.repository.clone();
    let command = format!(
        "git clone --recurse-submodules {} {}",
        url,
        repository.display()
    );
    state.install.try_begin(
        &format!("install-{}", chrono::Utc::now().timestamp()),
        &command,
        vec![
            "preflight".into(),
            "clone".into(),
            "submodules".into(),
            "compatibility".into(),
        ],
    )?;
    let snapshot = state.install.snapshot();

    let game_path = options.game_path.clone().filter(|g| !g.trim().is_empty());
    let skip = options.skip_compatibility;
    let work_out = out.clone();
    let work_work = work.clone();
    let repo_path = repository.clone();

    tauri::async_runtime::spawn(async move {
        let emit_app = app.clone();
        let state = app.state::<AppState>();
        let job = &state.install;
        let logs = &state.logs;

        let finish = |status: JobStatus, error: Option<String>| {
            job.finish(status, error.clone());
            let snap = job.snapshot();
            emit(
                &emit_app,
                InstallEvent {
                    kind: "finished".into(),
                    step: None,
                    status: Some(snap.status.clone()),
                    detail: None,
                    line: None,
                    message: error,
                    snapshot: Some(snap),
                },
            );
        };

        if resolve_executable("git").is_none() {
            let message = "Git is not installed or is not on PATH".to_string();
            logs.error(crate::logging::Category::Git, message.clone());
            finish(JobStatus::Failed, Some(message));
            return;
        }

        step(
            &emit_app,
            "preflight",
            "done",
            "Target folder is free and git is available.",
        );
        step(&emit_app, "clone", "running", format!("Cloning {}", url));
        logs.info(crate::logging::Category::Git, format!("Cloning {}", url));

        let line_app = emit_app.clone();
        let line_job = &state.install;
        let cloned = crate::leonida::repository::clone_repo(
            &url,
            &repo_path,
            |pid| state.install.set_pid(pid),
            |line| {
                line_job.push_line(line.text.clone());
                emit(
                    &line_app,
                    InstallEvent {
                        kind: "log".into(),
                        step: Some("clone".into()),
                        status: None,
                        detail: None,
                        line: Some(line.text),
                        message: None,
                        snapshot: None,
                    },
                );
            },
        )
        .await;

        if let Err(err) = cloned {
            logs.error(
                crate::logging::Category::Git,
                format!("Clone failed: {}", err.message),
            );
            step(&emit_app, "clone", "error", err.message.clone());
            finish(JobStatus::Failed, Some(err.message));
            return;
        }

        // Persist the repository path as soon as the checkout exists so the user can
        // retry submodules or compatibility without cloning again.
        let mut settings = state.settings();
        settings.repository_path = Some(repo_path.to_string_lossy().to_string());
        settings.out_path = work_out.to_string_lossy().to_string();
        settings.work_path = work_work.to_string_lossy().to_string();
        if let Some(game) = game_path.clone() {
            settings.game_path = Some(game);
        }
        let settings = state.replace_settings(settings);
        let _ = crate::settings::save(std::path::Path::new(&state.paths.config_dir), &settings);

        step(&emit_app, "clone", "done", "Checkout complete.");

        step(
            &emit_app,
            "submodules",
            "running",
            "Initializing git submodules",
        );
        let repo = GitRepo::new(&repo_path);
        state.install.set_stage("submodules");
        let sub_app = emit_app.clone();
        let sub_job = &state.install;
        if let Err(err) = repo
            .submodule_update(|line| {
                sub_job.push_line(line.text.clone());
                emit(
                    &sub_app,
                    InstallEvent {
                        kind: "log".into(),
                        step: Some("submodules".into()),
                        status: None,
                        detail: None,
                        line: Some(line.text),
                        message: None,
                        snapshot: None,
                    },
                );
            })
            .await
        {
            logs.error(
                crate::logging::Category::Git,
                format!("Submodule update failed: {}", err.message),
            );
            step(&emit_app, "submodules", "error", err.message.clone());
            finish(JobStatus::Failed, Some(err.message));
            return;
        }
        step(&emit_app, "submodules", "done", "Submodules are ready.");

        step(
            &emit_app,
            "compatibility",
            "running",
            "Probing the installed revision",
        );
        state.install.set_stage("compatibility");
        let report = match repo.status().await {
            Ok(status) => compatibility::evaluate(&repo_path, &status).await,
            Err(err) => compatibility::CompatibilityReport::unknown(err.message),
        };
        let blocked = report.status == CompatibilityStatus::Incompatible;

        if blocked && !skip {
            let detail = report
                .checks
                .iter()
                .filter(|c| format!("{:?}", c.severity).eq_ignore_ascii_case("error"))
                .map(|c| format!("{}: {}", c.label, c.detail))
                .collect::<Vec<_>>()
                .join("\n");
            let message = "The installed revision is not compatible with this launcher".to_string();
            logs.error(crate::logging::Category::Updater, message.clone());
            step(&emit_app, "compatibility", "error", message.clone());
            finish(
                JobStatus::Failed,
                Some(if detail.is_empty() {
                    message
                } else {
                    format!("{message}\n{detail}")
                }),
            );
            return;
        }

        let note = match report.status {
            CompatibilityStatus::Compatible => "Revision is compatible.".to_string(),
            CompatibilityStatus::Degraded => {
                "Revision works with warnings. Review the compatibility report.".to_string()
            }
            CompatibilityStatus::Incompatible => {
                "Installed anyway because compatibility checks were skipped. Builds will refuse to start."
                    .to_string()
            }
            CompatibilityStatus::Unknown => "Compatibility could not be verified.".to_string(),
        };
        step(&emit_app, "compatibility", "done", note.clone());
        logs.info(
            crate::logging::Category::Updater,
            format!("Install finished: {}", note),
        );
        state.install.set_pid(None);
        finish(JobStatus::Succeeded, None);
    });

    Ok(snapshot)
}

#[tauri::command]
pub fn cancel_install(app: AppHandle) -> bool {
    let state = app.state::<AppState>();
    cancel_job(state.install.request_cancel())
}

fn cancel_job(pid: Option<u32>) -> bool {
    match pid {
        Some(pid) => {
            kill_tree(pid);
            true
        }
        None => false,
    }
}

#[tauri::command]
pub fn install_state(app: AppHandle) -> crate::leonida::process::JobSnapshot {
    app.state::<AppState>().install.snapshot()
}

#[tauri::command]
pub async fn update_submodules(app: AppHandle) -> AppResult<crate::leonida::process::JobSnapshot> {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let repository = settings.repository_dir();
    if GitRepo::detect(&repository).is_none() {
        return Err(AppError::new(
            "not_installed",
            "Leonida is not installed yet",
        ));
    }
    state.install.try_begin(
        &format!("submodules-{}", chrono::Utc::now().timestamp()),
        "git submodule sync --recursive && git submodule update --init --recursive",
        vec!["submodules".into()],
    )?;
    let snapshot = state.install.snapshot();

    tauri::async_runtime::spawn(async move {
        let emit_app = app.clone();
        let state = app.state::<AppState>();
        let job = &state.install;
        step(
            &emit_app,
            "submodules",
            "running",
            "Synchronizing submodules",
        );
        let repo = GitRepo::new(&repository);
        let sub_app = emit_app.clone();
        let result = repo
            .submodule_update(|line| {
                job.push_line(line.text.clone());
                emit(
                    &sub_app,
                    InstallEvent {
                        kind: "log".into(),
                        step: Some("submodules".into()),
                        status: None,
                        detail: None,
                        line: Some(line.text),
                        message: None,
                        snapshot: None,
                    },
                );
            })
            .await;
        job.set_pid(None);
        match result {
            Ok(()) => {
                step(
                    &emit_app,
                    "submodules",
                    "done",
                    "Submodules are up to date.",
                );
                job.finish(JobStatus::Succeeded, None);
                let snap = job.snapshot();
                emit(
                    &emit_app,
                    InstallEvent {
                        kind: "finished".into(),
                        step: None,
                        status: Some(snap.status.clone()),
                        detail: None,
                        line: None,
                        message: None,
                        snapshot: Some(snap),
                    },
                );
            }
            Err(err) => {
                step(&emit_app, "submodules", "error", err.message.clone());
                job.finish(JobStatus::Failed, Some(err.message.clone()));
                let snap = job.snapshot();
                emit(
                    &emit_app,
                    InstallEvent {
                        kind: "finished".into(),
                        step: None,
                        status: Some(snap.status.clone()),
                        detail: None,
                        line: None,
                        message: Some(err.message),
                        snapshot: Some(snap),
                    },
                );
            }
        }
    });

    Ok(snapshot)
}
