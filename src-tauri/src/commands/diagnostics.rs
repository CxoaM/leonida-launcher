use crate::diagnostics::{self, DiagnosticReport};
use crate::error::AppResult;
use crate::logging::{LogFileInfo, LogLine};
use crate::state::AppState;
use std::path::Path;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn run_diagnostics(app: AppHandle) -> DiagnosticReport {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let paths = state.paths.clone();
    state.logs.info(
        crate::logging::Category::Diagnostics,
        "Running full diagnostics",
    );
    let report = diagnostics::run(&settings, &paths).await;
    state.logs.info(
        crate::logging::Category::Diagnostics,
        format!(
            "Diagnostics finished with overall status '{}'",
            report.overall
        ),
    );
    report
}

#[tauri::command]
pub async fn export_diagnostics(app: AppHandle) -> AppResult<String> {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let paths = state.paths.clone();
    let report = diagnostics::run(&settings, &paths).await;
    let path = diagnostics::export(&report, Path::new(&paths.logs_dir))?;
    state.logs.info(
        crate::logging::Category::Diagnostics,
        format!("Diagnostics exported to {}", path.display()),
    );
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn logs_list(state: State<'_, AppState>) -> Vec<LogFileInfo> {
    state.logs.list_files()
}

#[tauri::command]
pub fn logs_read(state: State<'_, AppState>, name: String) -> AppResult<String> {
    state.logs.read_file(&name, 4000)
}

#[tauri::command]
pub fn logs_clear(state: State<'_, AppState>, keep_days: u32) -> bool {
    state.logs.clear(keep_days);
    true
}

#[tauri::command]
pub fn logs_recent(state: State<'_, AppState>, limit: usize) -> Vec<LogLine> {
    state.logs.snapshot(limit.clamp(1, 2000))
}
