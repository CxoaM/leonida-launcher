use crate::error::{AppError, AppResult};
use crate::leonida::compatibility::{self, spec, CompatibilityReport, CompatibilityStatus};
use crate::leonida::repository::{BackupRef, CommitInfo, GitRepo, RepoStatus};
use crate::process::StreamLine;
use crate::storage;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamCheck {
    pub repository: String,
    pub branch: String,
    pub checked_at: String,
    pub installed_commit: Option<String>,
    pub installed_subject: Option<String>,
    pub latest_commit: Option<String>,
    pub latest_subject: Option<String>,
    pub behind: usize,
    pub ahead: usize,
    pub update_available: bool,
    pub commits: Vec<CommitInfo>,
    pub remote_url: Option<String>,
    pub remote_is_official: bool,
    pub modified_files: usize,
    pub untracked_files: usize,
    pub channel_note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateStage {
    Preflight,
    Fetch,
    Backup,
    Merge,
    Submodules,
    Verify,
    Rollback,
    Done,
}

impl UpdateStage {
    pub fn label(self) -> &'static str {
        match self {
            UpdateStage::Preflight => "Checking the local repository",
            UpdateStage::Fetch => "Fetching upstream commits",
            UpdateStage::Backup => "Creating a restore point",
            UpdateStage::Merge => "Applying a fast-forward update",
            UpdateStage::Submodules => "Updating git submodules",
            UpdateStage::Verify => "Verifying the result",
            UpdateStage::Rollback => "Restoring the previous revision",
            UpdateStage::Done => "Done",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOutcome {
    pub success: bool,
    pub from_commit: Option<String>,
    pub to_commit: Option<String>,
    pub rolled_back: bool,
    pub message: String,
    pub backup_ref: Option<String>,
    pub compatibility: CompatibilityReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateHistoryEntry {
    pub id: String,
    pub at: String,
    pub action: String,
    pub from_commit: Option<String>,
    pub to_commit: Option<String>,
    pub result: String,
    pub message: String,
    pub backup_ref: Option<String>,
    pub adapter_version: String,
}

/// Written before a merge and removed afterwards, so an interrupted update is
/// detectable on the next start instead of silently leaving a half-applied tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePoint {
    pub created_at: String,
    pub branch: String,
    pub commit: String,
    pub backup_ref: String,
    pub target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePolicy {
    OfficialOnly,
    /// Only used by tests against a throwaway local upstream.
    Any,
}

#[derive(Debug, Clone, Copy)]
pub struct ApplyOptions {
    pub require_verified: bool,
    pub verify_compatibility: bool,
}

impl Default for ApplyOptions {
    fn default() -> Self {
        Self {
            require_verified: true,
            verify_compatibility: true,
        }
    }
}

pub fn history_path(config_dir: &Path) -> PathBuf {
    config_dir.join("update-history.json")
}

fn restore_point_path(config_dir: &Path) -> PathBuf {
    config_dir.join("restore-point.json")
}

pub fn load_history(path: &Path) -> Vec<UpdateHistoryEntry> {
    storage::read_json::<Vec<UpdateHistoryEntry>>(path)
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub fn record_history(path: &Path, entry: UpdateHistoryEntry) {
    let mut history = load_history(path);
    history.insert(0, entry);
    history.truncate(50);
    let _ = storage::write_json(path, &history);
}

pub fn read_restore_point(config_dir: &Path) -> Option<RestorePoint> {
    storage::read_json::<RestorePoint>(&restore_point_path(config_dir))
        .ok()
        .flatten()
}

pub fn clear_restore_point(config_dir: &Path) {
    let path = restore_point_path(config_dir);
    if path.exists() {
        let _ = std::fs::remove_file(crate::paths::fs_path(&path));
    }
}

fn now() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn official_remote_check(remote_url: Option<&str>, policy: RemotePolicy) -> AppResult<bool> {
    let official = remote_url
        .map(compatibility::remote_matches_upstream)
        .unwrap_or(false);
    if policy == RemotePolicy::OfficialOnly && !official {
        return Err(AppError::new(
            "remote_mismatch",
            "This checkout does not point at the official Leonida repository",
        )
        .with_detail(format!(
            "origin = {}\nexpected = {}\n\nUpdates are only applied from the official upstream. Change the remote with `git remote set-url origin {}` if this is your own copy.",
            remote_url.unwrap_or("<none>"),
            spec().upstream.repository,
            spec().upstream.repository
        )));
    }
    Ok(official)
}

async fn preflight(
    repo_path: &Path,
    policy: RemotePolicy,
    require_clean: bool,
) -> AppResult<(GitRepo, RepoStatus, String, bool)> {
    let repo = GitRepo::new(repo_path);
    let status = repo.status().await?;
    if !status.is_repository {
        return Err(AppError::new(
            "not_installed",
            "Leonida is not installed at the configured path",
        ));
    }
    let branch = spec().upstream.default_branch.clone();
    if status.detached {
        return Err(AppError::new(
            "detached_head",
            "The repository is in detached HEAD state",
        )
        .with_detail("Switch back to the tracked branch before updating. The launcher will not force a branch change."));
    }
    if let Some(current) = status.branch.as_deref() {
        if current != branch {
            return Err(AppError::new(
                "branch_mismatch",
                format!("The checkout is on '{current}', not '{branch}'"),
            )
            .with_detail("Updates are only applied to the tracked upstream branch. Switch branches yourself if you want to follow a different one."));
        }
    }
    if require_clean && status.modified_files > 0 {
        return Err(AppError::new(
            "local_changes",
            format!(
                "{} tracked file(s) have local changes",
                status.modified_files
            ),
        )
        .with_detail("Commit or stash your changes first. The launcher never runs git reset --hard or discards your work."));
    }
    let remote = repo.remote_url("origin").await?;
    let official = official_remote_check(remote.as_deref(), policy)?;
    Ok((repo, status, branch, official))
}

pub async fn check(
    repo_path: &Path,
    policy: RemotePolicy,
    on_line: impl FnMut(StreamLine),
) -> AppResult<UpstreamCheck> {
    let spec = spec();
    let branch = spec.upstream.default_branch.clone();
    let (repo, status, _, official) = preflight(repo_path, policy, false).await?;
    let remote_url = repo.remote_url("origin").await?;

    repo.fetch("origin", &branch, on_line).await?;

    let tracking = format!("refs/remotes/origin/{branch}");
    let latest = repo.rev_parse(&tracking).await?;
    let installed = status.head.clone();
    let behind = match (&installed, &latest) {
        (Some(head), Some(_)) => repo
            .count_between(&format!("{head}..{tracking}"))
            .await
            .unwrap_or(0),
        _ => 0,
    };
    let ahead = match (&installed, &latest) {
        (Some(head), Some(_)) => repo
            .count_between(&format!("{tracking}..{head}"))
            .await
            .unwrap_or(0),
        _ => 0,
    };
    let commits = if behind > 0 {
        repo.log(25, Some(&format!("HEAD..{tracking}")))
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let latest_subject = match &latest {
        Some(sha) => repo
            .commit_info(sha)
            .await
            .ok()
            .flatten()
            .map(|c| c.subject),
        None => None,
    };
    let installed_subject = match &installed {
        Some(sha) => repo
            .commit_info(sha)
            .await
            .ok()
            .flatten()
            .map(|c| c.subject),
        None => None,
    };

    Ok(UpstreamCheck {
        repository: spec.upstream.repository.clone(),
        branch,
        checked_at: now(),
        installed_commit: installed,
        installed_subject,
        latest_commit: latest,
        latest_subject,
        behind,
        ahead,
        update_available: behind > 0,
        commits,
        remote_url,
        remote_is_official: official,
        modified_files: status.modified_files,
        untracked_files: status.untracked_files,
        channel_note: "Upstream publishes one branch (main) and no version tags, so the launcher offers no stable/latest split.".into(),
    })
}

async fn evaluate(repo_path: &Path) -> CompatibilityReport {
    let repo = GitRepo::new(repo_path);
    match repo.status().await {
        Ok(status) => compatibility::evaluate(repo_path, &status).await,
        Err(err) => CompatibilityReport::unknown(err.message),
    }
}

pub async fn apply(
    repo_path: &Path,
    config_dir: &Path,
    policy: RemotePolicy,
    options: ApplyOptions,
    mut progress: impl FnMut(UpdateStage, String),
) -> AppResult<UpdateOutcome> {
    let spec = spec();
    progress(
        UpdateStage::Preflight,
        "Checking working tree and remote".into(),
    );
    let (repo, status, branch, _) = preflight(repo_path, policy, true).await?;
    let from_commit = status.head.clone();
    let Some(head) = from_commit.clone() else {
        return Err(AppError::new("no_commits", "The repository has no commits"));
    };

    progress(UpdateStage::Fetch, format!("git fetch origin {branch}"));
    repo.fetch("origin", &branch, |_| {}).await?;
    let tracking = format!("refs/remotes/origin/{branch}");
    let Some(target) = repo.rev_parse(&tracking).await? else {
        return Err(AppError::new(
            "fetch_failed",
            format!("origin/{branch} could not be resolved after fetching"),
        ));
    };

    if target == head {
        let compatibility = evaluate(repo_path).await;
        return Ok(UpdateOutcome {
            success: true,
            from_commit: from_commit.clone(),
            to_commit: from_commit,
            rolled_back: false,
            message: "Already up to date.".into(),
            backup_ref: None,
            compatibility,
        });
    }

    let fast_forward_possible = repo.is_ancestor(&head, &target).await?;
    if !fast_forward_possible {
        return Err(AppError::new(
            "diverged_history",
            "The installed revision has diverged from upstream",
        )
        .with_detail("A fast-forward is not possible. The launcher will not merge, rebase or reset your history automatically. Review the divergence in git."));
    }

    progress(
        UpdateStage::Backup,
        "Creating a launcher-backup branch".into(),
    );
    let label = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_name = repo.create_backup_ref(&head, &label).await?;
    storage::write_json(
        &restore_point_path(config_dir),
        &RestorePoint {
            created_at: now(),
            branch: branch.clone(),
            commit: head.clone(),
            backup_ref: backup_name.clone(),
            target: target.clone(),
        },
    )?;

    progress(
        UpdateStage::Merge,
        format!("Fast-forwarding to {}", &target[..8.min(target.len())]),
    );
    if let Err(err) = repo.fast_forward(&tracking).await {
        progress(
            UpdateStage::Rollback,
            "Fast-forward failed; the tree was not changed".into(),
        );
        clear_restore_point(config_dir);
        return Err(err);
    }

    progress(
        UpdateStage::Submodules,
        "git submodule sync/update --init --recursive".into(),
    );
    if let Err(err) = repo.submodule_update(|_| {}).await {
        progress(
            UpdateStage::Rollback,
            "Submodule update failed; restoring the previous revision".into(),
        );
        let _ = repo.reset_keep(&backup_name).await;
        let _ = repo.submodule_update(|_| {}).await;
        clear_restore_point(config_dir);
        record_history(
            &history_path(config_dir),
            UpdateHistoryEntry {
                id: format!("{}-{}", label, "rollback"),
                at: now(),
                action: "update".into(),
                from_commit: from_commit.clone(),
                to_commit: Some(target.clone()),
                result: "rolled_back".into(),
                message: err.message.clone(),
                backup_ref: Some(backup_name.clone()),
                adapter_version: spec.adapter_version.clone(),
            },
        );
        return Err(
            AppError::new("submodule_failed", err.message).with_detail(format!(
                "{}\n\nThe previous revision was restored.",
                err.detail.unwrap_or_default()
            )),
        );
    }

    progress(
        UpdateStage::Verify,
        "Re-probing the installed revision".into(),
    );
    let new_head = repo.head().await?;
    if new_head.as_deref() != Some(target.as_str()) {
        let _ = repo.reset_keep(&backup_name).await;
        let _ = repo.submodule_update(|_| {}).await;
        clear_restore_point(config_dir);
        return Err(AppError::new(
            "verify_failed",
            "The repository did not end up at the fetched commit; the previous revision was restored",
        ));
    }

    let compatibility = if options.verify_compatibility {
        compatibility::evaluate(repo_path, &repo.status().await?).await
    } else {
        CompatibilityReport::unknown("Compatibility verification is disabled for this run.")
    };

    let rolled_back = false;
    let mut incompatible_error: Option<AppError> = None;

    if options.verify_compatibility {
        match compatibility.status {
            CompatibilityStatus::Incompatible => {
                incompatible_error = Some(
                    AppError::new(
                        "incompatible_upstream",
                        "New Leonida version detected. Compatibility verification failed.",
                    )
                    .with_detail(
                        compatibility
                            .checks
                            .iter()
                            .filter(|c| c.severity == compatibility::Severity::Error)
                            .map(|c| format!("{}: {}", c.label, c.detail))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ),
                );
            }
            _ if options.require_verified && compatibility.revision.kind == "newer" => {
                incompatible_error = Some(AppError::new(
                    "verification_required",
                    "New Leonida version detected. Compatibility verification is required.",
                )
                .with_detail(format!(
                    "{}\n\nThe live probe finished, but this revision is not the verified baseline. Confirm the update explicitly to install it.",
                    compatibility.revision.detail
                )));
            }
            _ => {}
        }
    }

    if let Some(err) = incompatible_error {
        progress(
            UpdateStage::Rollback,
            "Restoring the previous revision".into(),
        );
        repo.reset_keep(&backup_name).await?;
        repo.submodule_update(|_| {}).await?;
        clear_restore_point(config_dir);
        record_history(
            &history_path(config_dir),
            UpdateHistoryEntry {
                id: format!("{label}-rejected"),
                at: now(),
                action: "update".into(),
                from_commit: from_commit.clone(),
                to_commit: Some(target.clone()),
                result: "rolled_back".into(),
                message: err.message.clone(),
                backup_ref: Some(backup_name.clone()),
                adapter_version: spec.adapter_version.clone(),
            },
        );
        return Err(AppError {
            detail: Some(format!(
                "{}\n\nThe previous revision was restored from {backup_name}.",
                err.detail.clone().unwrap_or_default()
            )),
            ..err
        });
    }

    clear_restore_point(config_dir);
    record_history(
        &history_path(config_dir),
        UpdateHistoryEntry {
            id: label,
            at: now(),
            action: "update".into(),
            from_commit: from_commit.clone(),
            to_commit: new_head.clone(),
            result: "success".into(),
            message: format!("Updated to {}", new_head.clone().unwrap_or_default()),
            backup_ref: Some(backup_name.clone()),
            adapter_version: spec.adapter_version.clone(),
        },
    );
    progress(UpdateStage::Done, "Update applied".into());

    Ok(UpdateOutcome {
        success: true,
        from_commit,
        to_commit: new_head,
        rolled_back,
        message: "Leonida was updated and verified.".into(),
        backup_ref: Some(backup_name),
        compatibility,
    })
}

pub async fn list_backups(repo_path: &Path) -> AppResult<Vec<BackupRef>> {
    GitRepo::new(repo_path).list_backups().await
}

pub async fn rollback(
    repo_path: &Path,
    config_dir: &Path,
    backup_ref: Option<String>,
    mut progress: impl FnMut(UpdateStage, String),
) -> AppResult<UpdateOutcome> {
    let repo = GitRepo::new(repo_path);
    progress(UpdateStage::Preflight, "Checking the working tree".into());
    let status = repo.status().await?;
    if !status.is_repository {
        return Err(AppError::new(
            "not_installed",
            "Leonida is not installed at the configured path",
        ));
    }
    if status.modified_files > 0 {
        return Err(
            AppError::new("local_changes", "Rolling back needs a clean working tree").with_detail(
                "Tracked files have local changes. The launcher will not discard them.",
            ),
        );
    }

    let backups = repo.list_backups().await?;
    let chosen = match backup_ref {
        Some(name) => backups
            .iter()
            .find(|b| b.name == name || b.name.ends_with(&name))
            .cloned()
            .ok_or_else(|| AppError::new("no_backup", "That restore point no longer exists"))?,
        None => backups
            .first()
            .cloned()
            .ok_or_else(|| AppError::new("no_backup", "No restore point has been created yet"))?,
    };

    let from_commit = status.head.clone();
    progress(UpdateStage::Rollback, format!("Restoring {}", chosen.name));
    repo.reset_keep(&chosen.sha).await?;
    progress(UpdateStage::Submodules, "Re-syncing submodules".into());
    repo.submodule_update(|_| {}).await?;
    clear_restore_point(config_dir);
    let compatibility = evaluate(repo_path).await;
    record_history(
        &history_path(config_dir),
        UpdateHistoryEntry {
            id: format!("rollback-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")),
            at: now(),
            action: "rollback".into(),
            from_commit,
            to_commit: Some(chosen.sha.clone()),
            result: "success".into(),
            message: format!("Restored {}", chosen.name),
            backup_ref: Some(chosen.name.clone()),
            adapter_version: spec().adapter_version.clone(),
        },
    );
    progress(UpdateStage::Done, "Restore point applied".into());
    Ok(UpdateOutcome {
        success: true,
        from_commit: Some(chosen.sha.clone()),
        to_commit: Some(chosen.sha),
        rolled_back: true,
        message: "The previous revision was restored.".into(),
        backup_ref: Some(chosen.name),
        compatibility,
    })
}

/// Recover from an update that was interrupted mid-way: the tree may already be
/// fast-forwarded, so offer the restore point instead of guessing.
pub async fn recover_interrupted(repo_path: &Path, config_dir: &Path) -> AppResult<UpdateOutcome> {
    let point = read_restore_point(config_dir)
        .ok_or_else(|| AppError::new("no_restore_point", "No interrupted update was recorded"))?;
    rollback(repo_path, config_dir, Some(point.backup_ref), |_, _| {}).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git(args: &[&str], cwd: &Path) -> String {
        let out = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("git available");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn commit(repo: &Path, message: &str) {
        git(
            &[
                "-c",
                "user.email=t@e.st",
                "-c",
                "user.name=T",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                message,
            ],
            repo,
        );
    }

    fn setup() -> Option<(tempfile::TempDir, PathBuf, PathBuf)> {
        crate::process::resolve_executable("git")?;
        let tmp = tempfile::tempdir().unwrap();
        let origin = tmp.path().join("origin");
        std::fs::create_dir_all(&origin).unwrap();
        git(&["init", "-q", "-b", "main"], &origin);
        std::fs::write(origin.join("make_game.py"), "# fixture").unwrap();
        git(&["add", "."], &origin);
        commit(&origin, "first");
        let clone = tmp.path().join("clone");
        let status = Command::new("git")
            .args([
                "clone",
                "-q",
                origin.to_str().unwrap(),
                clone.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(status.success());
        Some((tmp, origin, clone))
    }

    #[test]
    fn history_is_capped_and_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let path = history_path(tmp.path());
        for i in 0..60 {
            record_history(
                &path,
                UpdateHistoryEntry {
                    id: i.to_string(),
                    at: "now".into(),
                    action: "update".into(),
                    from_commit: None,
                    to_commit: None,
                    result: "success".into(),
                    message: String::new(),
                    backup_ref: None,
                    adapter_version: "1.0.0".into(),
                },
            );
        }
        let loaded = load_history(&path);
        assert_eq!(loaded.len(), 50);
        assert_eq!(loaded[0].id, "59");
    }

    #[tokio::test]
    async fn check_and_apply_a_fast_forward_update() {
        let Some((tmp, origin, clone)) = setup() else {
            return;
        };
        let config = tmp.path().join("cfg");
        let initial = GitRepo::new(&clone).head().await.unwrap().unwrap();

        commit(&origin, "second");
        let latest = GitRepo::new(&origin).head().await.unwrap().unwrap();
        assert_ne!(initial, latest);

        let check = check(&clone, RemotePolicy::Any, |_| {}).await.unwrap();
        assert!(check.update_available);
        assert_eq!(check.behind, 1);
        assert_eq!(check.latest_commit.as_deref(), Some(latest.as_str()));
        assert_eq!(check.commits.len(), 1);
        assert_eq!(check.commits[0].subject, "second");

        let outcome = apply(
            &clone,
            &config,
            RemotePolicy::Any,
            ApplyOptions {
                require_verified: false,
                verify_compatibility: false,
            },
            |_, _| {},
        )
        .await
        .unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.to_commit.as_deref(), Some(latest.as_str()));
        let backups = list_backups(&clone).await.unwrap();
        assert_eq!(backups.len(), 1);
        assert!(read_restore_point(&config).is_none());
        assert_eq!(load_history(&history_path(&config)).len(), 1);
    }

    #[tokio::test]
    async fn rollback_restores_the_previous_commit() {
        let Some((tmp, origin, clone)) = setup() else {
            return;
        };
        let config = tmp.path().join("cfg");
        let initial = GitRepo::new(&clone).head().await.unwrap().unwrap();
        commit(&origin, "second");
        apply(
            &clone,
            &config,
            RemotePolicy::Any,
            ApplyOptions {
                require_verified: false,
                verify_compatibility: false,
            },
            |_, _| {},
        )
        .await
        .unwrap();
        let restored = rollback(&clone, &config, None, |_, _| {}).await.unwrap();
        assert!(restored.rolled_back);
        assert_eq!(restored.to_commit.as_deref(), Some(initial.as_str()));
        assert_eq!(GitRepo::new(&clone).head().await.unwrap().unwrap(), initial);
    }

    #[tokio::test]
    async fn refuses_to_update_a_dirty_working_tree() {
        let Some((tmp, origin, clone)) = setup() else {
            return;
        };
        commit(&origin, "second");
        std::fs::write(clone.join("make_game.py"), "# local edit").unwrap();
        let err = apply(
            &clone,
            &tmp.path().join("cfg"),
            RemotePolicy::Any,
            ApplyOptions {
                require_verified: false,
                verify_compatibility: false,
            },
            |_, _| {},
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, "local_changes");
        assert!(err
            .detail
            .unwrap_or_default()
            .contains("never runs git reset --hard"));
    }

    #[tokio::test]
    async fn rejects_a_non_official_remote_when_required() {
        let Some((_tmp, _origin, clone)) = setup() else {
            return;
        };
        let err = check(&clone, RemotePolicy::OfficialOnly, |_| {})
            .await
            .unwrap_err();
        assert_eq!(err.code, "remote_mismatch");
    }

    #[tokio::test]
    async fn diverged_history_is_not_forced() {
        let Some((tmp, origin, clone)) = setup() else {
            return;
        };
        let clone_repo = GitRepo::new(&clone);
        commit(&origin, "upstream second");
        git(
            &[
                "-c",
                "user.email=t@e.st",
                "-c",
                "user.name=T",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "local second",
            ],
            &clone,
        );
        let err = apply(
            &clone,
            &tmp.path().join("cfg"),
            RemotePolicy::Any,
            ApplyOptions {
                require_verified: false,
                verify_compatibility: false,
            },
            |_, _| {},
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, "diverged_history");
        // Nothing was rewritten.
        assert_eq!(
            clone_repo.head().await.unwrap(),
            GitRepo::new(&clone).head().await.unwrap()
        );
    }
}
