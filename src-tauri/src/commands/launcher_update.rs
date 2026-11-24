use crate::error::AppResult;
use crate::state::AppState;
use crate::updater::{self, LauncherRelease};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn launcher_update_info() -> LauncherRelease {
    updater::info()
}

#[tauri::command]
pub async fn launcher_update_check(app: AppHandle) -> LauncherRelease {
    let state = app.state::<AppState>();
    state.logs.info(
        crate::logging::Category::Updater,
        "Checking for launcher updates",
    );
    let release = updater::check(&app).await;
    state.logs.info(
        crate::logging::Category::Updater,
        format!("Launcher update check: {}", release.message),
    );
    release
}

#[tauri::command]
pub async fn launcher_update_install(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<bool> {
    state.logs.info(
        crate::logging::Category::Updater,
        "Installing launcher update",
    );
    updater::install(&app).await?;
    Ok(true)
}
