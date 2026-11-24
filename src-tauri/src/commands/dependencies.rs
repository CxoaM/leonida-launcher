use crate::error::{AppError, AppResult};
use crate::leonida::compatibility::spec;
use crate::leonida::dependencies as deps;
use crate::leonida::process::JobStatus;
use crate::process::{kill_tree, resolve_executable, CommandSpec};
use crate::state::AppState;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn check_dependencies() -> Vec<deps::DependencyReport> {
    deps::detect_all(&spec().requirements.windows_sdk).await
}

#[tauri::command]
pub fn dependency_install_state(
    state: State<'_, AppState>,
) -> crate::leonida::process::JobSnapshot {
    state.dependency_install.snapshot()
}

#[tauri::command]
pub fn install_dependency(
    app: AppHandle,
    package_id: String,
) -> AppResult<crate::leonida::process::JobSnapshot> {
    let args = deps::winget_install_args(&package_id)?;
    let state = app.state::<AppState>();
    if resolve_executable("winget").is_none() {
        return Err(
            AppError::new("winget_missing", "winget is not available on this system")
                .with_detail("Install the package manually from the link shown in Diagnostics."),
        );
    }

    let command = format!("winget {}", args.join(" "));
    state.dependency_install.try_begin(
        &format!("winget-{}", chrono::Utc::now().timestamp()),
        &command,
        vec![package_id.clone()],
    )?;
    let snapshot = state.dependency_install.snapshot();

    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let job = &state.dependency_install;
        let logs = &state.logs;
        logs.info(
            crate::logging::Category::Dependencies,
            format!("Running {command}"),
        );

        let spec = CommandSpec::new("winget").args(args);
        let result = crate::process::run_streaming(
            spec,
            |pid| job.set_pid(pid),
            |line| {
                job.push_line(line.text);
            },
        )
        .await;

        job.set_pid(None);
        match result {
            Ok(exit) if exit.success => {
                job.finish(JobStatus::Succeeded, None);
                logs.info(
                    crate::logging::Category::Dependencies,
                    format!("{} installed", package_id),
                );
            }
            Ok(exit) => {
                let message = match exit.code {
                    Some(code) => format!("winget exited with code {code}"),
                    None => "winget was terminated before it finished".to_string(),
                };
                logs.error(crate::logging::Category::Dependencies, message.clone());
                job.finish(JobStatus::Failed, Some(message));
            }
            Err(err) => {
                logs.error(
                    crate::logging::Category::Dependencies,
                    format!("winget failed: {}", err.message),
                );
                job.finish(JobStatus::Failed, Some(err.message));
            }
        }
    });

    Ok(snapshot)
}

#[tauri::command]
pub fn cancel_dependency_install(app: AppHandle) -> bool {
    let state = app.state::<AppState>();
    match state.dependency_install.request_cancel() {
        Some(pid) => {
            kill_tree(pid);
            true
        }
        None => false,
    }
}
