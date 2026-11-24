use crate::error::{AppError, AppResult};
use crate::leonida::compatibility::{self, CompatibilityReport, CompatibilityStatus};
use crate::leonida::dependencies::{self, DependencyReport};
use crate::leonida::repository::GitRepo;
use crate::leonida::validator::{validate_game_dir, GameValidation};
use crate::paths::{disk_space, DiskSpace, WorkspacePaths};
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SUPPORTED_WINDOWS_BUILD: u32 = 19041;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInfo {
    pub launcher_version: String,
    pub os_name: String,
    pub os_edition: String,
    pub os_version: String,
    pub os_build: u32,
    pub arch: String,
    pub cpu_name: String,
    pub cpu_cores: u32,
    pub memory_total_mb: u64,
    pub memory_available_mb: u64,
    pub long_paths: Option<bool>,
    pub executable: String,
    pub config_dir: String,
    pub data_dir: String,
    pub logs_dir: String,
}

#[cfg(windows)]
fn registry_string(path: &str, name: &str) -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(path)
        .ok()?
        .get_value::<String, _>(name)
        .ok()
}

#[cfg(windows)]
fn memory_info() -> (u64, u64) {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..unsafe { std::mem::zeroed() }
    };
    // SAFETY: the struct is fully initialised and sized as the API requires.
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        return (0, 0);
    }
    (
        status.ullTotalPhys / 1024 / 1024,
        status.ullAvailPhys / 1024 / 1024,
    )
}

#[cfg(not(windows))]
fn memory_info() -> (u64, u64) {
    (0, 0)
}

pub fn environment_info(paths: &WorkspacePaths) -> EnvironmentInfo {
    let mut os_name = "Windows".to_string();
    let mut os_edition = String::new();
    let mut os_version = String::new();
    let mut os_build = 0u32;

    #[cfg(windows)]
    {
        let key = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
        if let Some(product) = registry_string(key, "ProductName") {
            os_edition = product.clone();
            os_name = product;
        }
        if let Some(display) = registry_string(key, "DisplayVersion") {
            os_version = display;
        }
        if let Some(build) = registry_string(key, "CurrentBuildNumber")
            .or_else(|| registry_string(key, "CurrentBuild"))
            .and_then(|b| b.parse::<u32>().ok())
        {
            os_build = build;
        }
        if os_build >= 22000 && os_name.contains("Windows 10") {
            os_name = os_name.replacen("Windows 10", "Windows 11", 1);
        }
    }

    let cpu_name = {
        #[cfg(windows)]
        {
            registry_string(
                r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                "ProcessorNameString",
            )
            .unwrap_or_else(|| "Unknown CPU".into())
        }
        #[cfg(not(windows))]
        {
            "Unknown CPU".to_string()
        }
    };
    let (total_mb, available_mb) = memory_info();

    EnvironmentInfo {
        launcher_version: compatibility::LAUNCHER_VERSION.to_string(),
        os_name,
        os_edition,
        os_version,
        os_build,
        arch: std::env::consts::ARCH.to_string(),
        cpu_name,
        cpu_cores: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1) as u32,
        memory_total_mb: total_mb,
        memory_available_mb: available_mb,
        long_paths: dependencies::long_paths_enabled(),
        executable: std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        config_dir: paths.config_dir.clone(),
        data_dir: paths.data_dir.clone(),
        logs_dir: paths.logs_dir.clone(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticItem {
    pub id: String,
    pub group: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkCheck {
    pub reachable: bool,
    pub detail: String,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub run_at: String,
    pub overall: String,
    pub environment: EnvironmentInfo,
    pub items: Vec<DiagnosticItem>,
    pub dependencies: Vec<DependencyReport>,
    pub compatibility: CompatibilityReport,
    pub game: Option<GameValidation>,
    pub disks: Vec<DiskSpace>,
    pub network: NetworkCheck,
    pub repository_path: String,
    pub out_path: String,
    pub work_path: String,
    pub game_path: Option<String>,
}

async fn network_check() -> NetworkCheck {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent(format!(
            "LeonidaLauncher/{}",
            compatibility::LAUNCHER_VERSION
        ))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return NetworkCheck {
                reachable: false,
                detail: err.to_string(),
                latency_ms: None,
            }
        }
    };
    let started = std::time::Instant::now();
    match client.head("https://github.com").send().await {
        Ok(response) => NetworkCheck {
            reachable: response.status().is_success() || response.status().is_redirection(),
            detail: format!("GitHub answered with HTTP {}.", response.status().as_u16()),
            latency_ms: Some(started.elapsed().as_millis() as u64),
        },
        Err(err) => NetworkCheck {
            reachable: false,
            detail: format!("GitHub is not reachable: {err}"),
            latency_ms: None,
        },
    }
}

pub async fn run(settings: &Settings, paths: &WorkspacePaths) -> DiagnosticReport {
    let environment = environment_info(paths);
    let repository = settings.repository_dir();
    let mut items: Vec<DiagnosticItem> = Vec::new();

    let mut push = |id: &str, group: &str, label: &str, status: &str, detail: String| {
        items.push(DiagnosticItem {
            id: id.to_string(),
            group: group.to_string(),
            label: label.to_string(),
            status: status.to_string(),
            detail,
        })
    };

    // Operating system ------------------------------------------------------
    let os_ok = environment.arch == "x86_64" && environment.os_build >= SUPPORTED_WINDOWS_BUILD;
    push(
        "os",
        "System",
        "Windows 10/11 x64",
        if os_ok { "ok" } else { "error" },
        format!(
            "{} {} (build {}) on {}{}",
            environment.os_name,
            environment.os_version,
            environment.os_build,
            environment.arch,
            if os_ok {
                ""
            } else {
                ". The launcher targets Windows 10 2004+ x64."
            }
        ),
    );
    push(
        "cpu",
        "System",
        "Processor",
        "info",
        format!(
            "{} · {} logical core(s)",
            environment.cpu_name, environment.cpu_cores
        ),
    );
    push(
        "memory",
        "System",
        "Memory",
        if environment.memory_total_mb >= 8192 {
            "ok"
        } else {
            "warning"
        },
        format!(
            "{} MB total, {} MB available. Leonida's .NET tooling is happiest with 8 GB or more.",
            environment.memory_total_mb, environment.memory_available_mb
        ),
    );

    // Dependencies ---------------------------------------------------------
    let compatibility_spec = compatibility::spec();
    let deps = dependencies::detect_all(&compatibility_spec.requirements.windows_sdk).await;
    let blocking: Vec<&DependencyReport> = deps.iter().filter(|d| d.blocks()).collect();
    push(
        "dependencies",
        "Toolchain",
        "Required build tools",
        if blocking.is_empty() { "ok" } else { "error" },
        if blocking.is_empty() {
            "Every required tool for the selected upstream stages is present.".to_string()
        } else {
            format!(
                "Missing or unusable: {}.",
                blocking
                    .iter()
                    .map(|d| d.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    );

    // Repository -----------------------------------------------------------
    let repo = GitRepo::new(&repository);
    let repo_status =
        repo.status()
            .await
            .unwrap_or_else(|err| crate::leonida::repository::RepoStatus {
                error: Some(err.message),
                ..crate::leonida::repository::RepoStatus::empty(
                    repository.to_string_lossy().to_string(),
                )
            });
    let compatibility_report = if repo_status.is_repository {
        compatibility::evaluate(&repository, &repo_status).await
    } else {
        CompatibilityReport::unknown("Leonida is not installed yet.")
    };

    if repo_status.is_repository {
        push(
            "repository",
            "Leonida",
            "Source checkout",
            if repo_status.modified_files == 0 {
                "ok"
            } else {
                "warning"
            },
            format!(
                "{} on {}{}{}",
                repo_status
                    .head
                    .as_deref()
                    .map(|h| &h[..8.min(h.len())])
                    .unwrap_or("unknown commit"),
                repo_status.branch.as_deref().unwrap_or("detached"),
                if repo_status.untracked_files > 0 {
                    format!(" · {} untracked file(s)", repo_status.untracked_files)
                } else {
                    String::new()
                },
                if repo_status.modified_files > 0 {
                    format!(
                        " · {} modified file(s) block updates",
                        repo_status.modified_files
                    )
                } else {
                    String::new()
                }
            ),
        );
        let submodules = repo.submodules().await.unwrap_or_default();
        let missing = submodules.iter().filter(|s| !s.initialized).count();
        push(
            "submodules",
            "Leonida",
            "Git submodules",
            if missing == 0 { "ok" } else { "warning" },
            if submodules.is_empty() {
                "No submodules are declared.".to_string()
            } else {
                format!(
                    "{} of {} checked out{}",
                    submodules.len() - missing,
                    submodules.len(),
                    if missing > 0 {
                        "; run 'Update submodules' in the installer"
                    } else {
                        ""
                    }
                )
            },
        );
    } else {
        push(
            "repository",
            "Leonida",
            "Source checkout",
            "warning",
            format!("No git checkout at {}.", repository.display()),
        );
    }

    // Compatibility --------------------------------------------------------
    let compat_status = match compatibility_report.status {
        CompatibilityStatus::Compatible => "ok",
        CompatibilityStatus::Degraded => "warning",
        CompatibilityStatus::Incompatible => "error",
        CompatibilityStatus::Unknown => "info",
    };
    push(
        "compatibility",
        "Leonida",
        "Upstream compatibility",
        compat_status,
        format!(
            "Adapter {} · verified baseline {} · {}",
            compatibility_report.adapter_version,
            &compatibility_report.verified_commit
                [..8.min(compatibility_report.verified_commit.len())],
            compatibility_report.revision.detail
        ),
    );

    // Game files -----------------------------------------------------------
    let game = settings
        .game_path
        .as_deref()
        .filter(|g| !g.trim().is_empty())
        .map(|g| validate_game_dir(&crate::paths::expand_home(g)));
    if let Some(game) = &game {
        push(
            "game",
            "Game files",
            "Your own package",
            if game.valid { "ok" } else { "error" },
            if game.valid {
                format!(
                    "{} archive(s) next to MicrosoftGame.Config.",
                    game.archive_count
                )
            } else {
                game.issues.join(" ")
            },
        );
    } else {
        push(
            "game",
            "Game files",
            "Your own package",
            "info",
            "No package folder is configured yet. The launcher never downloads game files."
                .to_string(),
        );
    }

    // Disks ----------------------------------------------------------------
    let mut disks: Vec<DiskSpace> = Vec::new();
    for (label, path) in [
        ("Repository", repository.clone()),
        ("Build output", settings.out_dir()),
        ("Work folder", settings.work_dir()),
    ] {
        if let Ok(space) = disk_space(&path) {
            let free_gb = space.free_bytes / 1024 / 1024 / 1024;
            if label == "Work folder" {
                push(
                    "disk-work",
                    "Storage",
                    "Work folder space",
                    if free_gb >= 30 { "ok" } else { "warning" },
                    format!(
                        "{free_gb} GB free on {}. A full build needs about {} GB; a loader-only build needs about 30 GB.",
                        space.volume,
                        compatibility_spec.requirements.free_gb_full_build
                    ),
                );
            }
            disks.push(space);
        }
    }

    // Network --------------------------------------------------------------
    let network = network_check().await;
    push(
        "network",
        "Network",
        "GitHub reachability",
        if network.reachable { "ok" } else { "warning" },
        network.detail.clone(),
    );

    let overall = if items.iter().any(|i| i.status == "error") {
        "error"
    } else if items.iter().any(|i| i.status == "warning") {
        "warning"
    } else {
        "ok"
    }
    .to_string();

    DiagnosticReport {
        run_at: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        overall,
        environment,
        items,
        dependencies: deps,
        compatibility: compatibility_report,
        game,
        disks,
        network,
        repository_path: repository.to_string_lossy().to_string(),
        out_path: settings.out_dir().to_string_lossy().to_string(),
        work_path: settings.work_dir().to_string_lossy().to_string(),
        game_path: settings.game_path.clone(),
    }
}

pub fn render_text(report: &DiagnosticReport) -> String {
    let mut out = String::new();
    out.push_str("Leonida Launcher diagnostics\n");
    out.push_str("===========================\n");
    out.push_str(&format!("Run at: {}\n", report.run_at));
    out.push_str(&format!(
        "Launcher: {} on {} {} (build {})\n",
        report.environment.launcher_version,
        report.environment.os_name,
        report.environment.os_version,
        report.environment.os_build
    ));
    out.push_str(&format!("Repository: {}\n", report.repository_path));
    out.push_str(&format!("Output: {}\n", report.out_path));
    out.push_str(&format!("Work: {}\n\n", report.work_path));
    for item in &report.items {
        out.push_str(&format!(
            "[{:7}] {:<22} {}\n          {}\n",
            item.status.to_uppercase(),
            item.label,
            "",
            item.detail
        ));
    }
    out.push_str("\nDependencies\n------------\n");
    for dep in &report.dependencies {
        out.push_str(&format!(
            "{:<28} {:<10} {}\n",
            dep.name,
            format!("{:?}", dep.status).to_lowercase(),
            dep.version.clone().unwrap_or_else(|| dep.detail.clone())
        ));
    }
    out.push_str("\nCompatibility\n-------------\n");
    out.push_str(&format!(
        "Status: {:?}\nVerified: {}\nInstalled: {}\n",
        report.compatibility.status,
        report.compatibility.verified_commit,
        report
            .compatibility
            .installed_commit
            .clone()
            .unwrap_or_else(|| "none".into())
    ));
    for check in &report.compatibility.checks {
        out.push_str(&format!(
            "  [{:?}] {} — {}\n",
            check.severity, check.label, check.detail
        ));
    }
    out
}

pub fn export(report: &DiagnosticReport, logs_dir: &Path) -> AppResult<PathBuf> {
    std::fs::create_dir_all(crate::paths::fs_path(logs_dir))
        .map_err(|e| AppError::io("Creating the diagnostics folder", e))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let json_path = logs_dir.join(format!("diagnostics-{stamp}.json"));
    let text_path = logs_dir.join(format!("diagnostics-{stamp}.txt"));
    crate::storage::write_json(&json_path, report)?;
    std::fs::write(crate::paths::fs_path(&text_path), render_text(report))
        .map_err(|e| AppError::io("Writing the diagnostics report", e))?;
    Ok(text_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_reports_a_windows_build_number() {
        let paths = WorkspacePaths {
            config_dir: "C:\\cfg".into(),
            data_dir: "C:\\data".into(),
            cache_dir: "C:\\data\\cache".into(),
            logs_dir: "C:\\cfg\\logs".into(),
            documents_dir: "C:\\doc".into(),
            default_repository: "C:\\data\\repo".into(),
            default_out: "C:\\doc\\Leonida".into(),
            default_work: "C:\\data\\work".into(),
            default_cache: "C:\\data\\cache".into(),
        };
        let env = environment_info(&paths);
        assert!(!env.launcher_version.is_empty());
        if cfg!(windows) {
            assert!(env.os_build > 0, "expected a real Windows build number");
            assert_eq!(env.arch, "x86_64");
            assert!(env.cpu_cores >= 1);
        }
    }

    #[test]
    fn text_export_contains_every_group() {
        let report = DiagnosticReport {
            run_at: "now".into(),
            overall: "ok".into(),
            environment: EnvironmentInfo {
                launcher_version: "1.0.0".into(),
                os_name: "Windows 11".into(),
                os_edition: "Windows 11 Pro".into(),
                os_version: "24H2".into(),
                os_build: 26100,
                arch: "x86_64".into(),
                cpu_name: "CPU".into(),
                cpu_cores: 8,
                memory_total_mb: 16384,
                memory_available_mb: 8192,
                long_paths: Some(true),
                executable: "x.exe".into(),
                config_dir: "c".into(),
                data_dir: "d".into(),
                logs_dir: "l".into(),
            },
            items: vec![DiagnosticItem {
                id: "os".into(),
                group: "System".into(),
                label: "Windows".into(),
                status: "ok".into(),
                detail: "fine".into(),
            }],
            dependencies: vec![],
            compatibility: CompatibilityReport::unknown("not installed"),
            game: None,
            disks: vec![],
            network: NetworkCheck {
                reachable: true,
                detail: "ok".into(),
                latency_ms: Some(10),
            },
            repository_path: "r".into(),
            out_path: "o".into(),
            work_path: "w".into(),
            game_path: None,
        };
        let text = render_text(&report);
        assert!(text.contains("Compatibility"));
        assert!(text.contains("Windows"));
        assert!(text.contains("Run at"));
    }
}
