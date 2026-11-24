use crate::error::{AppError, AppResult};
use crate::launcher_meta::meta;
use crate::version::compare_versions;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};
use url::Url;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherRelease {
    pub configured: bool,
    pub available: bool,
    pub current_version: String,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    pub download_bytes: Option<u64>,
    pub endpoint: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherUpdateProgress {
    pub phase: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub message: String,
}

fn configured_endpoint() -> AppResult<String> {
    let meta = meta();
    if !meta.updater_configured {
        return Err(AppError::new(
            "updater_not_configured",
            "Launcher updates are not configured for this build",
        )
        .with_detail(
            "resources/launcher.json still contains YOUR_GITHUB_OWNER. Run `npm run configure -- --owner <name>` before publishing releases.",
        ));
    }
    Ok(meta.release_endpoint())
}

fn release_stub(message: impl Into<String>) -> LauncherRelease {
    let meta = meta();
    LauncherRelease {
        configured: meta.updater_configured,
        available: false,
        current_version: meta.version.clone(),
        version: None,
        notes: None,
        published_at: None,
        download_bytes: None,
        endpoint: meta.updater_endpoint.clone(),
        message: message.into(),
    }
}

pub fn info() -> LauncherRelease {
    let meta = meta();
    if !meta.updater_configured {
        return release_stub(
            "Launcher updates are disabled until the repository owner is configured for this build.",
        );
    }
    release_stub("Not checked yet.")
}

async fn remote_update(app: &AppHandle) -> AppResult<Update> {
    let endpoint = configured_endpoint()?;
    let url = Url::parse(&endpoint).map_err(|e| {
        AppError::new("updater_endpoint", "The update endpoint is malformed")
            .with_detail(e.to_string())
    })?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|e| {
            AppError::new("updater_endpoint", "The update endpoint was rejected")
                .with_detail(e.to_string())
        })?
        .build()
        .map_err(|e| {
            AppError::new("updater_config", "The updater is not configured correctly")
                .with_detail(e.to_string())
        })?;
    updater
        .check()
        .await
        .map_err(|e| {
            AppError::new("updater_check", "The launcher update check failed")
                .with_detail(e.to_string())
        })?
        .ok_or_else(|| AppError::new("updater_no_release", "No launcher release is published yet"))
}

async fn download_size(url: &Url) -> Option<u64> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(format!("LeonidaLauncher/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let response = client.head(url.as_str()).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.content_length()
}

pub async fn check(app: &AppHandle) -> LauncherRelease {
    let meta = meta();
    if !meta.updater_configured {
        return release_stub(
            "Launcher updates are disabled until the repository owner is configured for this build.",
        );
    }
    match remote_update(app).await {
        Ok(update) => {
            let available = compare_versions(&update.version, &update.current_version).is_gt();
            let download_bytes = download_size(&update.download_url).await;
            LauncherRelease {
                configured: true,
                available,
                current_version: update.current_version.clone(),
                version: Some(update.version.clone()),
                notes: update.body.clone(),
                published_at: update.date.map(|d| d.to_string()),
                download_bytes,
                endpoint: meta.updater_endpoint.clone(),
                message: if available {
                    format!("Version {} is available.", update.version)
                } else {
                    "You are running the newest published launcher version.".into()
                },
            }
        }
        Err(err) if err.code == "updater_no_release" || err.code == "updater_check" => {
            LauncherRelease {
                configured: true,
                available: false,
                current_version: meta.version.clone(),
                version: None,
                notes: None,
                published_at: None,
                download_bytes: None,
                endpoint: meta.updater_endpoint.clone(),
                message: format!("{} — {}", err.message, err.detail.unwrap_or_default())
                    .trim()
                    .to_string(),
            }
        }
        Err(err) => LauncherRelease {
            configured: true,
            available: false,
            current_version: meta.version.clone(),
            version: None,
            notes: None,
            published_at: None,
            download_bytes: None,
            endpoint: meta.updater_endpoint.clone(),
            message: err.message,
        },
    }
}

/// Download the signed installer, verify it against the public key baked into the
/// build, and hand it to the NSIS installer. Progress is emitted so the UI can show
/// real byte counts instead of an invented percentage.
pub async fn install(app: &AppHandle) -> AppResult<()> {
    configured_endpoint()?;
    let update = remote_update(app).await?;
    let total_hint = download_size(&update.download_url).await;

    let _ = app.emit(
        "launcher://update-progress",
        LauncherUpdateProgress {
            phase: "downloading".into(),
            downloaded: 0,
            total: total_hint,
            message: format!("Downloading version {}.", update.version),
        },
    );

    let emitter = app.clone();
    let updater_app = app.clone();
    let result = update
        .download_and_install(
            move |chunk, total| {
                let _ = emitter.emit(
                    "launcher://update-progress",
                    LauncherUpdateProgress {
                        phase: "downloading".into(),
                        downloaded: chunk as u64,
                        total: total.or(total_hint),
                        message: "Downloading the signed installer.".into(),
                    },
                );
            },
            move || {
                let _ = updater_app.emit(
                    "launcher://update-progress",
                    LauncherUpdateProgress {
                        phase: "installing".into(),
                        downloaded: total_hint.unwrap_or(0),
                        total: total_hint,
                        message: "Installer verified. Applying the update.".into(),
                    },
                );
            },
        )
        .await;

    match result {
        Ok(()) => {
            let _ = app.emit(
                "launcher://update-progress",
                LauncherUpdateProgress {
                    phase: "restarting".into(),
                    downloaded: total_hint.unwrap_or(0),
                    total: total_hint,
                    message: "The launcher restarts to finish the update.".into(),
                },
            );
            Ok(())
        }
        Err(err) => {
            let detail = err.to_string();
            let _ = app.emit(
                "launcher://update-progress",
                LauncherUpdateProgress {
                    phase: "error".into(),
                    downloaded: 0,
                    total: total_hint,
                    message: detail.clone(),
                },
            );
            Err(AppError::new(
                "updater_install",
                "The launcher update could not be installed",
            )
            .with_detail(detail))
        }
    }
}
