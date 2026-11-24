use crate::leonida::dependencies::python_tool;
use crate::leonida::repository::{CommitInfo, GitRepo, RepoStatus, SubmoduleState};
use crate::process::run_capture;
use crate::version::{compare_versions, Version};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

pub const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
const SPEC_JSON: &str = include_str!("../../resources/compatibility.json");
const PROBE_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilitySpec {
    pub schema: u32,
    pub adapter_version: String,
    pub upstream: UpstreamSpec,
    pub launcher: LauncherSpec,
    pub requirements: RequirementsSpec,
    pub expected: ExpectedSpec,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamSpec {
    pub repository: String,
    pub default_branch: String,
    pub last_verified_commit: String,
    pub last_verified_at: String,
    pub game_version: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSpec {
    pub min_version: String,
    pub max_major: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementsSpec {
    pub python: String,
    pub dotnet: String,
    pub windows_sdk: String,
    pub llvm: String,
    pub free_gb_full_build: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedSpec {
    pub required_files: Vec<String>,
    pub optional_files: Vec<String>,
    pub required_directories: Vec<String>,
    pub optional_directories: Vec<String>,
    pub submodules: Vec<SubmoduleSpec>,
    pub cli_probe: Vec<String>,
    pub required_flags: Vec<String>,
    pub optional_flags: Vec<String>,
    pub known_stages: Vec<String>,
    pub required_stages: Vec<String>,
    pub resume_marker: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmoduleSpec {
    pub path: String,
    pub required: bool,
}

pub fn spec() -> &'static CompatibilitySpec {
    static SPEC: OnceLock<CompatibilitySpec> = OnceLock::new();
    SPEC.get_or_init(|| {
        serde_json::from_str(SPEC_JSON)
            .unwrap_or_else(|err| panic!("compatibility.json is invalid: {err}"))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Ok,
    Warning,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityCheck {
    pub id: String,
    pub label: String,
    pub severity: Severity,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompatibilityStatus {
    Compatible,
    Degraded,
    Incompatible,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub install: bool,
    pub update: bool,
    pub build: bool,
    pub assets: bool,
    pub runtime: bool,
    pub pipelines: bool,
    pub resume: bool,
    pub wave64_emulate: bool,
    pub wav: bool,
    pub full_check: bool,
    pub game_launch: bool,
    pub game_launch_reason: String,
}

impl Capabilities {
    fn none() -> Self {
        Self {
            install: false,
            update: false,
            build: false,
            assets: false,
            runtime: false,
            pipelines: false,
            resume: false,
            wave64_emulate: false,
            wav: false,
            full_check: false,
            game_launch: false,
            game_launch_reason: "The upstream runtime cannot boot the game yet.".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionRelation {
    /// verified | newer | older | unknown
    pub kind: String,
    pub commits_ahead_of_verified: usize,
    pub commits_behind_verified: usize,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityReport {
    pub status: CompatibilityStatus,
    pub adapter_version: String,
    pub launcher_version: String,
    pub spec_schema: u32,
    pub verified_commit: String,
    pub verified_at: String,
    pub installed_commit: Option<String>,
    pub installed_commit_subject: Option<String>,
    pub branch: Option<String>,
    pub revision: RevisionRelation,
    pub checks: Vec<CompatibilityCheck>,
    pub capabilities: Capabilities,
    pub stages: Vec<String>,
    pub help_flags: Vec<String>,
    pub missing_required_flags: Vec<String>,
    pub probed_help: Option<String>,
    pub probed_at: String,
    pub warnings: Vec<String>,
}

impl CompatibilityReport {
    pub fn unknown(reason: impl Into<String>) -> Self {
        let spec = spec();
        Self {
            status: CompatibilityStatus::Unknown,
            adapter_version: spec.adapter_version.clone(),
            launcher_version: LAUNCHER_VERSION.to_string(),
            spec_schema: spec.schema,
            verified_commit: spec.upstream.last_verified_commit.clone(),
            verified_at: spec.upstream.last_verified_at.clone(),
            installed_commit: None,
            installed_commit_subject: None,
            branch: None,
            revision: RevisionRelation {
                kind: "unknown".into(),
                commits_ahead_of_verified: 0,
                commits_behind_verified: 0,
                detail: "Leonida is not installed yet.".into(),
            },
            checks: vec![CompatibilityCheck {
                id: "repository".into(),
                label: "Leonida repository".into(),
                severity: Severity::Skipped,
                detail: reason.into(),
            }],
            capabilities: Capabilities::none(),
            stages: Vec::new(),
            help_flags: Vec::new(),
            missing_required_flags: Vec::new(),
            probed_help: None,
            probed_at: now(),
            warnings: Vec::new(),
        }
    }

    pub fn can_update(&self) -> bool {
        matches!(
            self.status,
            CompatibilityStatus::Compatible | CompatibilityStatus::Degraded
        )
    }
}

fn now() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Parse `--flags` out of real `--help` output.
pub fn extract_flags(help: &str) -> Vec<String> {
    static FLAG: OnceLock<Regex> = OnceLock::new();
    let re = FLAG.get_or_init(|| Regex::new(r"(--[a-z0-9][a-z0-9-]{1,40})").unwrap());
    let mut flags: Vec<String> = re.find_iter(help).map(|m| m.as_str().to_string()).collect();
    flags.sort();
    flags.dedup();
    flags
}

/// Parse the stage list from the line `--stages ... comma list from: check,shaders,...`.
pub fn extract_stages(help: &str) -> Vec<String> {
    static STAGES: OnceLock<Regex> = OnceLock::new();
    let re = STAGES.get_or_init(|| Regex::new(r"(?i)comma list from:\s*([a-z0-9,\-]+)").unwrap());
    if let Some(caps) = re.captures(help) {
        return caps[1]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    // Fallback: stages named in the help body still count as supported.
    let known = &spec().expected.known_stages;
    known
        .iter()
        .filter(|stage| {
            help.lines()
                .any(|line| line.trim_start().starts_with(stage.as_str()) && line.contains("  "))
        })
        .cloned()
        .collect()
}

pub async fn evaluate(repository: &Path, status: &RepoStatus) -> CompatibilityReport {
    let spec = spec();
    let mut checks: Vec<CompatibilityCheck> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Launcher / adapter compatibility ---------------------------------------
    let launcher_ok = {
        let v = Version::parse(LAUNCHER_VERSION);
        match v {
            Some(v) => {
                v.major() <= spec.launcher.max_major
                    && compare_versions(LAUNCHER_VERSION, &spec.launcher.min_version)
                        != std::cmp::Ordering::Less
            }
            None => false,
        }
    };
    checks.push(check(
        "launcher",
        "Launcher adapter",
        if launcher_ok {
            Severity::Ok
        } else {
            Severity::Warning
        },
        format!(
            "Integration adapter {} supports Launcher {} and newer (schema {}).",
            spec.adapter_version, spec.launcher.min_version, spec.schema
        ),
    ));
    if !launcher_ok {
        warnings
            .push("This Launcher build is outside the adapter's declared version range.".into());
    }

    if !status.is_repository {
        let mut report =
            CompatibilityReport::unknown("No git repository was found at the configured path.");
        report.checks = checks;
        return report;
    }

    let repo = GitRepo::new(repository);
    let head = status.head.clone();
    let installed_subject = match &head {
        Some(sha) => repo
            .commit_info(sha)
            .await
            .ok()
            .flatten()
            .map(|c: CommitInfo| c.subject),
        None => None,
    };

    // Files ------------------------------------------------------------------
    let mut missing_files = Vec::new();
    for file in &spec.expected.required_files {
        if !repository.join(file).exists() {
            missing_files.push(file.clone());
        }
    }
    let missing_optional: Vec<String> = spec
        .expected
        .optional_files
        .iter()
        .filter(|f| !repository.join(f.as_str()).exists())
        .cloned()
        .collect();
    if missing_files.is_empty() {
        checks.push(check(
            "files",
            "Project layout",
            Severity::Ok,
            "All expected entry points are present.",
        ));
    } else {
        checks.push(check(
            "files",
            "Project layout",
            Severity::Error,
            format!("Missing expected files: {}.", missing_files.join(", ")),
        ));
    }
    for dir in &spec.expected.required_directories {
        if !repository.join(dir).is_dir() {
            checks.push(check(
                &format!("dir-{dir}"),
                &format!("Folder /{dir}"),
                Severity::Error,
                "This folder is part of the upstream build surface and is missing.",
            ));
        }
    }
    if !missing_optional.is_empty() {
        warnings.push(format!(
            "Optional upstream entries are missing: {}.",
            missing_optional.join(", ")
        ));
    }

    // Submodules -------------------------------------------------------------
    let submodules: Vec<SubmoduleState> = repo.submodules().await.unwrap_or_default();
    for expected in &spec.expected.submodules {
        let state = submodules.iter().find(|s| s.path == expected.path);
        match state {
            Some(s) if s.initialized => checks.push(check(
                &format!("submodule-{}", expected.path),
                &format!("Submodule {}", expected.path),
                Severity::Ok,
                format!("Checked out at {}.", s.sha.clone().unwrap_or_else(|| "unknown".into())),
            )),
            Some(_) => checks.push(check(
                &format!("submodule-{}", expected.path),
                &format!("Submodule {}", expected.path),
                if expected.required { Severity::Error } else { Severity::Warning },
                "Registered but not checked out. The launcher can initialize it with 'Update submodules'.",
            )),
            None => checks.push(check(
                &format!("submodule-{}", expected.path),
                &format!("Submodule {}", expected.path),
                if expected.required { Severity::Error } else { Severity::Warning },
                "Not declared in .gitmodules. Upstream may have renamed or removed it.",
            )),
        }
    }

    // CLI probe --------------------------------------------------------------
    let mut help_text: Option<String> = None;
    let mut flags: Vec<String> = Vec::new();
    let mut stages: Vec<String> = Vec::new();
    let mut probe_error: Option<String> = None;

    match python_tool().await {
        None => {
            probe_error =
                Some("Python was not found, so make_game.py --help could not be read.".into());
        }
        Some(python) => {
            if !python.version.at_least(3, 12) {
                probe_error = Some(format!(
                    "Python {} is older than the required {}.",
                    python.version.short(),
                    spec.requirements.python
                ));
            } else {
                let mut args = vec![
                    "-I".to_string(),
                    "make_game.py".to_string(),
                    "--help".to_string(),
                ];
                // `py -3` and friends carry a prefix; the spec's probe list starts at -I.
                let spec_probe = &spec.expected.cli_probe;
                if spec_probe.len() >= 3 && spec_probe[0] == "-I" {
                    args = spec_probe.clone();
                }
                let spec_cmd = python.spec().args(args).cwd(repository.to_path_buf());
                match run_capture(&spec_cmd, PROBE_TIMEOUT).await {
                    Ok(out) if out.success() => {
                        help_text = Some(format!("{}{}", out.stdout, out.stderr));
                    }
                    Ok(out) => {
                        probe_error = Some(format!(
                            "`make_game.py --help` exited with {:?}: {}",
                            out.code,
                            out.stderr.lines().next().unwrap_or("no output")
                        ));
                    }
                    Err(err) => probe_error = Some(err.message),
                }
            }
        }
    }

    if let Some(help) = &help_text {
        flags = extract_flags(help);
        stages = extract_stages(help);
        if stages.is_empty() {
            stages = spec.expected.known_stages.clone();
            warnings.push(
                "The stage list could not be read from --help; the documented stages are assumed."
                    .into(),
            );
        }
        checks.push(check(
            "cli",
            "Build entry point",
            Severity::Ok,
            format!(
                "make_game.py answered with {} documented option(s) and {} stage(s).",
                flags.len(),
                stages.len()
            ),
        ));
    } else {
        checks.push(check(
            "cli",
            "Build entry point",
            Severity::Warning,
            probe_error
                .clone()
                .unwrap_or_else(|| "The build entry point could not be probed.".into()),
        ));
        warnings.push(probe_error.unwrap_or_else(|| "make_game.py could not be probed.".into()));
    }

    let missing_required_flags: Vec<String> = spec
        .expected
        .required_flags
        .iter()
        .filter(|f| !flags.is_empty() && !flags.contains(f))
        .cloned()
        .collect();
    if !flags.is_empty() && !missing_required_flags.is_empty() {
        checks.push(check(
            "flags",
            "Required build options",
            Severity::Error,
            format!(
                "The installed Leonida no longer accepts: {}. The integration adapter must be updated.",
                missing_required_flags.join(", ")
            ),
        ));
    } else if !flags.is_empty() {
        checks.push(check(
            "flags",
            "Required build options",
            Severity::Ok,
            "Every option the launcher relies on is still documented.",
        ));
    }

    let missing_stages: Vec<String> = spec
        .expected
        .required_stages
        .iter()
        .filter(|s| !stages.is_empty() && !stages.contains(s))
        .cloned()
        .collect();
    if !missing_stages.is_empty() {
        checks.push(check(
            "stages",
            "Build stages",
            Severity::Error,
            format!(
                "Required stage(s) disappeared: {}.",
                missing_stages.join(", ")
            ),
        ));
    }

    // Capabilities are derived from what the installed checkout actually
    // documents, never from a hard-coded assumption about a version.
    let has = |flag: &str| flags.iter().any(|f| f == flag);
    let has_stage = |stage: &str| stages.iter().any(|s| s == stage);
    let resume = help_text
        .as_deref()
        .map(|h| {
            h.to_ascii_lowercase()
                .contains(&spec.expected.resume_marker.to_ascii_lowercase())
        })
        .unwrap_or(false);
    let build_ready = status.is_repository
        && missing_files.is_empty()
        && !stages.is_empty()
        && missing_required_flags.is_empty();

    let capabilities = Capabilities {
        install: true,
        update: true,
        build: build_ready,
        assets: build_ready && has_stage("assets"),
        runtime: build_ready && has_stage("runtime"),
        pipelines: build_ready && has_stage("pipelines"),
        resume,
        wave64_emulate: has("--wave64-emulate"),
        wav: has("--wav"),
        full_check: has("--full-check"),
        game_launch: false,
        game_launch_reason:
            "Upstream does not boot the game yet, so the launcher does not offer a Play action."
                .into(),
    };

    if !resume && help_text.is_some() {
        warnings.push("Upstream no longer advertises resumable builds in --help; the launcher will still let it continue.".into());
    }

    // Revision relation ------------------------------------------------------
    let revision = match &head {
        None => RevisionRelation {
            kind: "unknown".into(),
            commits_ahead_of_verified: 0,
            commits_behind_verified: 0,
            detail: "The repository has no commits.".into(),
        },
        Some(sha) if sha == &spec.upstream.last_verified_commit => RevisionRelation {
            kind: "verified".into(),
            commits_ahead_of_verified: 0,
            commits_behind_verified: 0,
            detail: format!(
                "Installed revision matches the verified commit from {}.",
                spec.upstream.last_verified_at
            ),
        },
        Some(sha) => {
            let mut relation = RevisionRelation {
                kind: "unknown".into(),
                commits_ahead_of_verified: 0,
                commits_behind_verified: 0,
                detail: String::new(),
            };
            if repo
                .rev_parse(&spec.upstream.last_verified_commit)
                .await
                .ok()
                .flatten()
                .is_some()
            {
                let verified_is_ancestor = repo
                    .is_ancestor(&spec.upstream.last_verified_commit, sha)
                    .await
                    .unwrap_or(false);
                let installed_is_ancestor = repo
                    .is_ancestor(sha, &spec.upstream.last_verified_commit)
                    .await
                    .unwrap_or(false);
                if verified_is_ancestor {
                    let ahead = repo
                        .count_between(&format!("{}..{}", spec.upstream.last_verified_commit, sha))
                        .await
                        .unwrap_or(0);
                    relation.kind = "newer".into();
                    relation.commits_ahead_of_verified = ahead;
                    relation.detail = format!(
                        "Installed revision is {ahead} commit(s) ahead of the verified baseline {}. Verification is required.",
                        &spec.upstream.last_verified_commit[..8.min(spec.upstream.last_verified_commit.len())]
                    );
                } else if installed_is_ancestor {
                    let behind = repo
                        .count_between(&format!("{}..{}", sha, spec.upstream.last_verified_commit))
                        .await
                        .unwrap_or(0);
                    relation.kind = "older".into();
                    relation.commits_behind_verified = behind;
                    relation.detail = format!(
                        "Installed revision is {behind} commit(s) behind the verified baseline."
                    );
                } else {
                    relation.detail =
                        "Installed revision has diverged from the verified baseline; only the live probe counts.".into();
                }
            } else {
                relation.detail =
                    "The verified commit is not in the local history (shallow clone or rewritten history); only the live probe counts.".into();
            }
            checks.push(check(
                "revision",
                "Verified revision",
                if relation.kind == "newer" {
                    Severity::Warning
                } else {
                    Severity::Ok
                },
                relation.detail.clone(),
            ));
            if relation.kind == "newer" {
                warnings.push("New Leonida version detected. Compatibility verification is required. The live probe passed, but upstream moved past the verified baseline.".into());
            }
            relation
        }
    };

    let has_error = checks.iter().any(|c| c.severity == Severity::Error);
    let has_warning =
        checks.iter().any(|c| c.severity == Severity::Warning) || !warnings.is_empty();
    let overall = if has_error {
        CompatibilityStatus::Incompatible
    } else if has_warning {
        CompatibilityStatus::Degraded
    } else {
        CompatibilityStatus::Compatible
    };

    CompatibilityReport {
        status: overall,
        adapter_version: spec.adapter_version.clone(),
        launcher_version: LAUNCHER_VERSION.to_string(),
        spec_schema: spec.schema,
        verified_commit: spec.upstream.last_verified_commit.clone(),
        verified_at: spec.upstream.last_verified_at.clone(),
        installed_commit: head,
        installed_commit_subject: installed_subject,
        branch: status.branch.clone(),
        revision,
        checks,
        capabilities,
        stages,
        help_flags: flags,
        missing_required_flags,
        probed_help: help_text.map(|h| h.lines().take(120).collect::<Vec<_>>().join("\n")),
        probed_at: now(),
        warnings,
    }
}

fn check(
    id: &str,
    label: &str,
    severity: Severity,
    detail: impl Into<String>,
) -> CompatibilityCheck {
    CompatibilityCheck {
        id: id.to_string(),
        label: label.to_string(),
        severity,
        detail: detail.into(),
    }
}

/// The official upstream URL is the only update source. A fork or mirror must be
/// rejected unless the user explicitly changes the remote, because the safety
/// checks in this launcher are calibrated against the documented project.
pub fn remote_matches_upstream(remote: &str) -> bool {
    fn normalize(value: &str) -> String {
        value
            .trim()
            .trim_end_matches(".git")
            .trim_end_matches('/')
            .replace('\\', "/")
            .to_ascii_lowercase()
    }
    normalize(remote) == normalize(&spec().upstream.repository)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_loads_and_is_sane() {
        let spec = spec();
        assert_eq!(spec.upstream.default_branch, "main");
        assert!(spec
            .expected
            .required_files
            .contains(&"make_game.py".to_string()));
        assert!(spec
            .expected
            .required_flags
            .contains(&"--stages".to_string()));
        assert_eq!(spec.requirements.python, "3.12");
    }

    #[test]
    fn extracts_flags_from_help_text() {
        let help = "--game PATH  your package folder\n  --stages LIST  comma list from: check,shaders,assets,runtime,pipelines\n  --no-tui  plain console\n";
        let flags = extract_flags(help);
        assert!(flags.contains(&"--game".to_string()));
        assert!(flags.contains(&"--no-tui".to_string()));
        assert_eq!(flags.len(), 3);
    }

    #[test]
    fn extracts_stages_from_help_text() {
        let help = "usage: make_game.py\n  --stages VALUE  comma list from: check,shaders,assets,runtime,pipelines\n";
        let stages = extract_stages(help);
        assert_eq!(
            stages,
            vec!["check", "shaders", "assets", "runtime", "pipelines"]
        );
    }

    #[test]
    fn missing_stage_list_falls_back_to_documented_names() {
        let help = "check     always first\nshaders   carve shaders\n";
        let stages = extract_stages(help);
        assert!(stages.contains(&"check".to_string()));
    }

    #[test]
    fn remote_matching_requires_the_official_url() {
        assert!(remote_matches_upstream(
            "https://github.com/ubiquitarianism/leonida.git"
        ));
        assert!(remote_matches_upstream(
            "https://github.com/Ubiquitarianism/Leonida/"
        ));
        assert!(!remote_matches_upstream(
            "https://github.com/someone/leonida"
        ));
        assert!(!remote_matches_upstream(
            "https://github.com/evil/notleonida-fake"
        ));
        assert!(!remote_matches_upstream(
            "git@github.com:ubiquitarianism/leonida.git"
        ));
    }

    #[test]
    fn unknown_report_has_no_build_capability() {
        let report = CompatibilityReport::unknown("not installed");
        assert_eq!(report.status, CompatibilityStatus::Unknown);
        assert!(!report.capabilities.build);
        assert!(!report.capabilities.game_launch);
    }
}
