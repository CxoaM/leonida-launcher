pub mod commands;
pub mod diagnostics;
pub mod error;
pub mod launcher_meta;
pub mod leonida;
pub mod logging;
pub mod paths;
pub mod process;
pub mod settings;
pub mod state;
pub mod storage;
pub mod updater;
pub mod version;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = AppState::new(&handle)?;
            let stored = state.settings();

            // Keep the registry entry in sync with the stored preference (a user may
            // have moved the executable since the last run).
            if stored.launch_on_startup {
                let _ = settings::set_autostart(true);
            }

            app.manage(state);

            let recycle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let state = recycle.state::<AppState>();
                let repository = state.settings().repository_dir();
                let config_dir = std::path::PathBuf::from(&state.paths.config_dir);
                if leonida::updater::read_restore_point(&config_dir).is_none() {
                    return;
                }
                if leonida::repository::GitRepo::detect(&repository).is_none() {
                    leonida::updater::clear_restore_point(&config_dir);
                    return;
                }
                state.logs.warn(
                    logging::Category::Updater,
                    "An interrupted update was detected; restoring the recorded restore point",
                );
                match leonida::updater::recover_interrupted(&repository, &config_dir).await {
                    Ok(outcome) => state.logs.info(
                        logging::Category::Updater,
                        format!("Interrupted update recovered: {}", outcome.message),
                    ),
                    Err(err) => state.logs.error(
                        logging::Category::Updater,
                        format!("Interrupted update could not be recovered: {}", err.message),
                    ),
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::settings_load,
            commands::system::settings_save,
            commands::system::settings_reset,
            commands::system::paths_info,
            commands::system::environment_info,
            commands::system::app_metadata,
            commands::system::probe_path,
            commands::system::disk_space,
            commands::system::reveal_path,
            commands::system::workspace_summary,
            commands::installation::installation_state,
            commands::installation::detect_game_folders,
            commands::installation::validate_game_dir,
            commands::installation::install_leonida,
            commands::installation::cancel_install,
            commands::installation::update_submodules,
            commands::installation::install_state,
            commands::updates::compatibility_report,
            commands::updates::upstream_check,
            commands::updates::apply_upstream_update,
            commands::updates::rollback_upstream_update,
            commands::updates::restore_points,
            commands::updates::update_history,
            commands::updates::clear_update_history,
            commands::dependencies::check_dependencies,
            commands::dependencies::install_dependency,
            commands::dependencies::dependency_install_state,
            commands::build::build_stages,
            commands::build::build_work_state,
            commands::build::start_build,
            commands::build::cancel_build,
            commands::build::build_state,
            commands::diagnostics::run_diagnostics,
            commands::diagnostics::export_diagnostics,
            commands::diagnostics::logs_list,
            commands::diagnostics::logs_read,
            commands::diagnostics::logs_clear,
            commands::diagnostics::logs_recent,
            commands::launcher_update::launcher_update_info,
            commands::launcher_update::launcher_update_check,
            commands::launcher_update::launcher_update_install,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Leonida Launcher");
}
