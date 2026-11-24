use crate::error::{AppError, AppResult};
use crate::leonida::compatibility::{self, spec, CompatibilityReport};
use crate::leonida::repository::{BackupRef, GitRepo};
use crate::leonida::updater::{
    self, ApplyOptions, RemotePolicy, RestorePoint, UpdateHistoryEntry, UpdateOutcome, UpdateStage,
    UpstreamCheck,
};
use crate::state::AppState;
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgressEvent {
    pub stage: String,
    pub label: String,
    pub detail: String,
}

fn emit_progress(app: &AppHandle, stage: UpdateStage, detail: String) {
    let _ = app.emit(
        "update://progress",
        UpdateProgressEvent {
            stage: format!("{:?}", stage).to_ascii_lowercase(),
            label: stage.label().to_string(),
            detail,
        },
    );
}

fn repository(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    let state = app.state::<AppState>();
    let path = state.settings().repository_dir();
    if GitRepo::detect(&path).is_none() {
        return Err(AppError::new(
            "not_installed",
            "Leonida is not installed at the configured path",
        )
        .with_detail("Install it first from the Installation page."));
    }
    Ok(path)
}

#[tauri::command]
pub async fn compatibility_report(app: AppHandle) -> CompatibilityReport {
    let state = app.state::<AppState>();
    let path = state.settings().repository_dir();
    match GitRepo::detect(&path) {
        Some(repo) => match repo.status().await {
            Ok(status) => compatibility::evaluate(&path, &status).await,
            Err(err) => CompatibilityReport::unknown(err.message),
        },
        None => CompatibilityReport::unknown(
            "Leonida is not installed yet. Run the installation wizard to fetch it from the official repository.",
        ),
    }
}

#[tauri::command]
pub async fn upstream_check(app: AppHandle) -> AppResult<UpstreamCheck> {
    let path = repository(&app)?;
    let emit_app = app.clone();
    let check = updater::check(&path, RemotePolicy::OfficialOnly, |line| {
        let _ = emit_app.emit(
            "update://progress",
            UpdateProgressEvent {
                stage: "fetch".into(),
                label: UpdateStage::Fetch.label().to_string(),
                detail: line.text,
            },
        );
    })
    .await?;
    app.state::<AppState>().logs.info(
        crate::logging::Category::Git,
        format!("Upstream check: {} commit(s) behind", check.behind),
    );
    Ok(check)
}

#[tauri::command]
pub async fn apply_upstream_update(
    app: AppHandle,
    allow_unverified: bool,
) -> AppResult<UpdateOutcome> {
    let state = app.state::<AppState>();
    let path = state.settings().repository_dir();
    let config_dir = std::path::PathBuf::from(&state.paths.config_dir);
    if GitRepo::detect(&path).is_none() {
        return Err(AppError::new(
            "not_installed",
            "Leonida is not installed yet",
        ));
    }

    let emit_app = app.clone();
    let options = ApplyOptions {
        require_verified: !allow_unverified,
        verify_compatibility: true,
    };
    let outcome = updater::apply(
        &path,
        &config_dir,
        RemotePolicy::OfficialOnly,
        options,
        |stage, detail| {
            emit_progress(&emit_app, stage, detail);
        },
    )
    .await?;

    state.logs.info(
        crate::logging::Category::Updater,
        format!(
            "Upstream update {}: {} -> {}",
            if outcome.success {
                "succeeded"
            } else {
                "failed"
            },
            outcome.from_commit.clone().unwrap_or_default(),
            outcome.to_commit.clone().unwrap_or_default()
        ),
    );
    Ok(outcome)
}

#[tauri::command]
pub async fn rollback_upstream_update(
    app: AppHandle,
    backup_ref: Option<String>,
) -> AppResult<UpdateOutcome> {
    let state = app.state::<AppState>();
    let path = state.settings().repository_dir();
    let config_dir = std::path::PathBuf::from(&state.paths.config_dir);
    let emit_app = app.clone();
    let outcome = updater::rollback(&path, &config_dir, backup_ref, |stage, detail| {
        emit_progress(&emit_app, stage, detail);
    })
    .await?;
    state.logs.info(
        crate::logging::Category::Updater,
        format!("Rollback applied: {}", outcome.message),
    );
    Ok(outcome)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePointView {
    pub created_at: String,
    pub branch: String,
    pub commit: String,
    pub backup_ref: String,
    pub target: String,
}

impl From<BackupRef> for RestorePointView {
    fn from(value: BackupRef) -> Self {
        Self {
            created_at: value.created_at,
            branch: spec().upstream.default_branch.clone(),
            target: value.sha.chars().take(8).collect(),
            commit: value.sha,
            backup_ref: value.name,
        }
    }
}

#[tauri::command]
pub async fn restore_points(app: AppHandle) -> AppResult<Vec<RestorePointView>> {
    let state = app.state::<AppState>();
    let path = state.settings().repository_dir();
    if GitRepo::detect(&path).is_none() {
        return Ok(Vec::new());
    }
    let backups = updater::list_backups(&path).await.unwrap_or_default();
    Ok(backups.into_iter().map(RestorePointView::from).collect())
}

#[tauri::command]
pub fn update_history(app: AppHandle) -> Vec<UpdateHistoryEntry> {
    let state = app.state::<AppState>();
    updater::load_history(&updater::history_path(Path::new(&state.paths.config_dir)))
}

#[tauri::command]
pub fn clear_update_history(app: AppHandle) -> bool {
    let state = app.state::<AppState>();
    let path = updater::history_path(Path::new(&state.paths.config_dir));
    crate::storage::write_json(&path, &Vec::<UpdateHistoryEntry>::new()).is_ok()
}

#[allow(dead_code)]
fn restore_point_view(point: RestorePoint) -> RestorePointView {
    RestorePointView {
        created_at: point.created_at,
        branch: point.branch,
        commit: point.commit,
        backup_ref: point.backup_ref,
        target: point.target,
    }
}
