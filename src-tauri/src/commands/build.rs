use crate::error::{AppError, AppResult};
use crate::leonida::build as build_mod;
use crate::leonida::build::{BuildRequest, StageInfo, WorkState};
use crate::leonida::compatibility::{self, CompatibilityStatus};
use crate::leonida::dependencies::python_tool;
use crate::leonida::process::JobStatus;
use crate::leonida::repository::GitRepo;
use crate::process::{kill_tree, CommandSpec};
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildEvent {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<crate::leonida::process::JobSnapshot>,
}

fn emit(app: &AppHandle, event: BuildEvent) {
    let _ = app.emit("build://event", event);
}

fn emit_state(app: &AppHandle) {
    let snapshot = app.state::<AppState>().build.snapshot();
    emit(
        app,
        BuildEvent {
            kind: "state".into(),
            line: None,
            stream: None,
            stage: None,
            status: Some(snapshot.status.clone()),
            percent: None,
            eta: None,
            message: None,
            detail: None,
            snapshot: Some(snapshot),
        },
    );
}

#[tauri::command]
pub async fn build_stages(app: AppHandle) -> Vec<StageInfo> {
    let state = app.state::<AppState>();
    let path = state.settings().repository_dir();
    let detected = match GitRepo::detect(&path) {
        Some(repo) => match repo.status().await {
            Ok(status) => compatibility::evaluate(&path, &status).await.stages,
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    build_mod::stage_catalog(&detected)
}

#[tauri::command]
pub fn build_work_state(state: State<'_, AppState>) -> WorkState {
    let settings = state.settings();
    build_mod::inspect_work(&settings.work_dir(), &settings.out_dir())
}

#[tauri::command]
pub fn build_state(app: AppHandle) -> crate::leonida::process::JobSnapshot {
    app.state::<AppState>().build.snapshot()
}

#[tauri::command]
pub async fn start_build(
    app: AppHandle,
    request: BuildRequest,
) -> AppResult<crate::leonida::process::JobSnapshot> {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let repository = settings.repository_dir();

    let repo = GitRepo::detect(&repository)
        .ok_or_else(|| AppError::new("not_installed", "Leonida is not installed yet"))?;
    let status = repo.status().await?;

    let report = compatibility::evaluate(&repository, &status).await;
    if report.status == CompatibilityStatus::Incompatible {
        return Err(AppError::new(
            "incompatible_revision",
            "This Leonida revision is not compatible with the launcher",
        )
        .with_detail(
            report
                .checks
                .iter()
                .filter(|c| format!("{:?}", c.severity).eq_ignore_ascii_case("error"))
                .map(|c| format!("{}: {}", c.label, c.detail))
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    if !report.capabilities.build {
        return Err(AppError::new(
            "build_not_supported",
            "The installed Leonida does not document a build command",
        ));
    }

    let python = python_tool().await.ok_or_else(|| {
        AppError::new(
            "python_missing",
            "Python 3.12+ is required to run make_game.py",
        )
    })?;
    let args = build_mod::build_arguments(&request, &report.stages, &report.help_flags)?;

    let command_line = format!("py -3 -I make_game.py {}", args.join(" "));
    state.build.try_begin(
        &format!("build-{}", chrono::Utc::now().timestamp()),
        &command_line,
        request.stages.clone(),
    )?;

    let snapshot = state.build.snapshot();
    emit_state(&app);

    let mut spec: CommandSpec = python.spec().args(args).cwd(repository.clone());
    spec = spec.env("PYTHONUTF8", "1").env("PYTHONIOENCODING", "utf-8");
    let stages = request.stages.clone();

    tauri::async_runtime::spawn(async move {
        let emit_app = app.clone();
        let state = app.state::<AppState>();
        let job = &state.build;
        let logs = &state.logs;
        logs.info(
            crate::logging::Category::Build,
            format!("Starting build: {command_line}"),
        );

        let line_app = emit_app.clone();
        let result = crate::process::run_streaming(
            spec,
            |pid| job.set_pid(pid),
            |line| {
                job.push_line(line.text.clone());
                let signal = build_mod::parse_line(&line.text, &stages);
                if let Some(stage) = signal.stage.clone() {
                    job.set_stage(&stage);
                    emit(
                        &line_app,
                        BuildEvent {
                            kind: "stage".into(),
                            line: None,
                            stream: None,
                            stage: Some(stage),
                            status: Some("running".into()),
                            percent: None,
                            eta: None,
                            message: None,
                            detail: None,
                            snapshot: None,
                        },
                    );
                }
                if signal.percent.is_some() || signal.eta.is_some() {
                    job.progress(signal.percent, signal.eta.clone());
                    emit(
                        &line_app,
                        BuildEvent {
                            kind: "progress".into(),
                            line: None,
                            stream: None,
                            stage: None,
                            status: None,
                            percent: signal.percent,
                            eta: signal.eta,
                            message: None,
                            detail: None,
                            snapshot: None,
                        },
                    );
                }
                emit(
                    &line_app,
                    BuildEvent {
                        kind: "log".into(),
                        line: Some(line.text),
                        stream: Some(match line.stream {
                            crate::process::StreamKind::Stdout => "stdout".into(),
                            crate::process::StreamKind::Stderr => "stderr".into(),
                        }),
                        stage: None,
                        status: None,
                        percent: None,
                        eta: None,
                        message: None,
                        detail: None,
                        snapshot: None,
                    },
                );
            },
        )
        .await;

        job.set_pid(None);
        let cancelled = job.snapshot().status == "cancelling";
        match result {
            Ok(exit) if exit.success && !cancelled => {
                logs.info(
                    crate::logging::Category::Build,
                    "Build finished successfully",
                );
                job.finish(JobStatus::Succeeded, None);
            }
            Ok(_) if cancelled => {
                logs.warn(
                    crate::logging::Category::Build,
                    "Build cancelled by the user",
                );
                job.finish(JobStatus::Cancelled, None);
            }
            Ok(exit) => {
                let message = match exit.code {
                    Some(code) => format!("make_game.py exited with code {code}"),
                    None => "make_game.py was terminated before it finished".to_string(),
                };
                logs.error(crate::logging::Category::Build, message.clone());
                job.finish(JobStatus::Failed, Some(message));
            }
            Err(err) => {
                logs.error(
                    crate::logging::Category::Build,
                    format!("Build could not run: {}", err.message),
                );
                job.finish(JobStatus::Failed, Some(err.message.clone()));
                emit(
                    &emit_app,
                    BuildEvent {
                        kind: "error".into(),
                        line: None,
                        stream: None,
                        stage: None,
                        status: None,
                        percent: None,
                        eta: None,
                        message: Some(err.message),
                        detail: err.detail,
                        snapshot: None,
                    },
                );
            }
        }

        let snapshot = job.snapshot();
        emit(
            &emit_app,
            BuildEvent {
                kind: "finished".into(),
                line: None,
                stream: None,
                stage: None,
                status: Some(snapshot.status.clone()),
                percent: None,
                eta: None,
                message: snapshot.error.clone(),
                detail: None,
                snapshot: Some(snapshot),
            },
        );
    });

    Ok(snapshot)
}

#[tauri::command]
pub fn cancel_build(app: AppHandle) -> bool {
    let state = app.state::<AppState>();
    match state.build.request_cancel() {
        Some(pid) => {
            kill_tree(pid);
            true
        }
        None => false,
    }
}
