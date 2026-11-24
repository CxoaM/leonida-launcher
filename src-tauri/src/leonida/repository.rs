use crate::error::{command_failed, AppError, AppResult};
use crate::process::{run_capture, run_streaming, CommandSpec, StreamLine};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const GIT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub path: String,
    pub exists: bool,
    pub is_repository: bool,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub detached: bool,
    pub modified_files: usize,
    pub untracked_files: usize,
    pub remote_url: Option<String>,
    pub error: Option<String>,
}

impl RepoStatus {
    pub fn empty(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            exists: false,
            is_repository: false,
            branch: None,
            head: None,
            detached: false,
            modified_files: 0,
            untracked_files: 0,
            remote_url: None,
            error: None,
        }
    }
    pub fn is_clean(&self) -> bool {
        self.modified_files == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub subject: String,
    pub author: String,
    pub date: String,
    pub date_human: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmoduleState {
    pub path: String,
    pub sha: Option<String>,
    pub initialized: bool,
    pub modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRef {
    pub name: String,
    pub sha: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub path: PathBuf,
}

impl GitRepo {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// A folder counts as a repository when `.git` exists (a directory for normal
    /// checkouts, a file for worktrees and submodules).
    pub fn detect(path: &Path) -> Option<Self> {
        if path.join(".git").exists() {
            Some(Self::new(path))
        } else {
            None
        }
    }

    fn command(&self, args: &[String]) -> CommandSpec {
        let mut spec = CommandSpec::new("git")
            .arg("-C")
            .arg(self.path.to_string_lossy().to_string())
            .args([
                "--no-pager".to_string(),
                "-c".to_string(),
                "core.quotepath=false".to_string(),
                "-c".to_string(),
                "advice.detachedHead=false".to_string(),
            ])
            .args(args.iter().cloned())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "Never");
        spec.cwd = Some(self.path.clone());
        spec
    }

    pub async fn run(&self, args: &[&str]) -> AppResult<crate::process::Captured> {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        run_capture(&self.command(&owned), GIT_TIMEOUT).await
    }

    async fn checked(&self, args: &[&str], context: &str) -> AppResult<String> {
        let out = self.run(args).await?;
        if !out.success() {
            return Err(command_failed(
                format!("git {context}"),
                out.code,
                &out.stderr,
                &out.stdout,
            ));
        }
        Ok(out.stdout)
    }

    pub async fn status(&self) -> AppResult<RepoStatus> {
        let path = self.path.to_string_lossy().to_string();
        if !self.path.is_dir() {
            return Ok(RepoStatus::empty(path));
        }
        if GitRepo::detect(&self.path).is_none() {
            return Ok(RepoStatus::empty(path));
        }
        let out = self.run(&["status", "--porcelain=v1", "--branch"]).await?;
        if !out.success() {
            return Ok(RepoStatus {
                exists: true,
                error: Some(out.stderr.trim().to_string()),
                ..RepoStatus::empty(path)
            });
        }
        let mut parsed = parse_porcelain_status(&path, &out.stdout);
        parsed.head = self.head().await.ok().flatten();
        parsed.remote_url = self.remote_url("origin").await.ok().flatten();
        if parsed.head.is_none() && parsed.branch.is_some() {
            // A fresh `git init` has a branch name but no commit yet.
            parsed.is_repository = true;
        }
        Ok(parsed)
    }

    pub async fn head(&self) -> AppResult<Option<String>> {
        let out = self.run(&["rev-parse", "HEAD"]).await?;
        if out.success() {
            Ok(Some(out.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    pub async fn rev_parse(&self, reference: &str) -> AppResult<Option<String>> {
        let out = self
            .run(&["rev-parse", "--verify", "--quiet", reference])
            .await?;
        if out.success() {
            let sha = out.stdout.trim().to_string();
            Ok((!sha.is_empty()).then_some(sha))
        } else {
            Ok(None)
        }
    }

    pub async fn commit_info(&self, reference: &str) -> AppResult<Option<CommitInfo>> {
        let out = self
            .run(&[
                "log",
                "-1",
                "--pretty=format:%H%x1f%h%x1f%an%x1f%aI%x1f%s",
                reference,
            ])
            .await?;
        if !out.success() {
            return Ok(None);
        }
        Ok(parse_commit_line(out.stdout.trim()))
    }

    pub async fn log(&self, limit: usize, range: Option<&str>) -> AppResult<Vec<CommitInfo>> {
        let mut args = vec![
            "log".to_string(),
            format!("-{limit}"),
            "--pretty=format:%H%x1f%h%x1f%an%x1f%aI%x1f%s".to_string(),
        ];
        if let Some(range) = range {
            args.push(range.to_string());
        }
        let out = self
            .run(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .await?;
        if !out.success() {
            return Ok(Vec::new());
        }
        Ok(out.stdout.lines().filter_map(parse_commit_line).collect())
    }

    pub async fn remote_url(&self, remote: &str) -> AppResult<Option<String>> {
        let out = self.run(&["remote", "get-url", remote]).await?;
        if out.success() {
            let url = out.stdout.trim().to_string();
            Ok((!url.is_empty()).then_some(url))
        } else {
            Ok(None)
        }
    }

    pub async fn count_between(&self, range: &str) -> AppResult<usize> {
        let out = self.run(&["rev-list", "--count", range]).await?;
        if out.success() {
            Ok(out.stdout.trim().parse::<usize>().unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    pub async fn is_ancestor(&self, ancestor: &str, descendant: &str) -> AppResult<bool> {
        let out = self
            .run(&["merge-base", "--is-ancestor", ancestor, descendant])
            .await?;
        Ok(out.code == Some(0))
    }

    pub async fn fetch(
        &self,
        remote: &str,
        branch: &str,
        mut on_line: impl FnMut(StreamLine),
    ) -> AppResult<()> {
        let owned: Vec<String> = vec![
            "fetch".into(),
            "--prune".into(),
            "--no-tags".into(),
            remote.into(),
            format!("+refs/heads/{branch}:refs/remotes/{remote}/{branch}"),
        ];
        let spec = self.command(&owned);
        let mut relevant: Vec<StreamLine> = Vec::new();
        let exit = run_streaming(
            spec,
            |_| {},
            |line| {
                if line.text.to_ascii_lowercase().contains("error")
                    || line.text.to_ascii_lowercase().contains("fatal")
                {
                    relevant.push(line.clone());
                }
                on_line(line);
            },
        )
        .await?;
        if !exit.success {
            return Err(AppError::new(
                "git_fetch_failed",
                "Could not reach the upstream repository",
            )
            .with_detail(
                relevant
                    .iter()
                    .map(|l| l.text.clone())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ));
        }
        Ok(())
    }

    pub async fn create_backup_ref(&self, sha: &str, label: &str) -> AppResult<String> {
        let name = format!("launcher-backup/{label}");
        self.checked(
            &["update-ref", &format!("refs/heads/{name}"), sha],
            "create backup ref",
        )
        .await?;
        Ok(name)
    }

    pub async fn list_backups(&self) -> AppResult<Vec<BackupRef>> {
        // `for-each-ref` only understands `%%`, not the `%xNN` escapes that
        // `git log --pretty` uses, so the separator has to be a literal tab.
        // A tab can never appear in a ref name or in an ISO-8601 date.
        let format = "--format=%(refname:short)\t%(objectname)\t%(creatordate:iso-strict)";
        let out = self
            .run(&[
                "for-each-ref",
                "--sort=-creatordate",
                format,
                "refs/heads/launcher-backup",
            ])
            .await?;
        if !out.success() {
            return Ok(Vec::new());
        }
        Ok(out
            .stdout
            .lines()
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let name = parts.next()?.trim().to_string();
                let sha = parts.next()?.trim().to_string();
                let created_at = parts.next().unwrap_or("").trim().to_string();
                (!name.is_empty() && sha.len() >= 7).then_some(BackupRef {
                    name,
                    sha,
                    created_at,
                })
            })
            .collect())
    }

    pub async fn fast_forward(&self, reference: &str) -> AppResult<()> {
        self.checked(&["merge", "--ff-only", reference], "fast-forward")
            .await?;
        Ok(())
    }

    /// `reset --keep` never discards tracked work: it refuses to run when the
    /// files it would touch are modified. Only ever used to undo our own
    /// fast-forward or a user-initiated rollback to a saved ref.
    pub async fn reset_keep(&self, reference: &str) -> AppResult<()> {
        self.checked(&["reset", "--keep", reference], "reset --keep")
            .await?;
        Ok(())
    }

    pub async fn checkout_branch(&self, branch: &str) -> AppResult<()> {
        self.checked(&["checkout", branch], "checkout").await?;
        Ok(())
    }

    pub async fn submodules(&self) -> AppResult<Vec<SubmoduleState>> {
        let out = self.run(&["submodule", "status", "--recursive"]).await?;
        if !out.success() {
            return Ok(Vec::new());
        }
        Ok(out
            .stdout
            .lines()
            .filter_map(parse_submodule_line)
            .collect())
    }

    pub async fn submodule_update(&self, mut on_line: impl FnMut(StreamLine)) -> AppResult<()> {
        let sync = self.run(&["submodule", "sync", "--recursive"]).await?;
        if !sync.success() {
            return Err(command_failed(
                "git submodule sync",
                sync.code,
                &sync.stderr,
                &sync.stdout,
            ));
        }
        let owned: Vec<String> = vec![
            "submodule".into(),
            "update".into(),
            "--init".into(),
            "--recursive".into(),
            "--progress".into(),
        ];
        let spec = self.command(&owned);
        let mut errors: Vec<String> = Vec::new();
        let exit = run_streaming(
            spec,
            |_| {},
            |line| {
                if line.stream == crate::process::StreamKind::Stderr
                    && (line.text.to_ascii_lowercase().contains("fatal")
                        || line.text.to_ascii_lowercase().contains("error:"))
                {
                    errors.push(line.text.clone());
                }
                on_line(line);
            },
        )
        .await?;
        if !exit.success {
            return Err(
                AppError::new("submodule_failed", "Git submodules could not be updated")
                    .with_detail(errors.join("\n")),
            );
        }
        Ok(())
    }

    pub async fn changed_files(
        &self,
        from: &str,
        to: &str,
        limit: usize,
    ) -> AppResult<Vec<String>> {
        let out = self
            .run(&["diff", "--name-only", &format!("{from}..{to}")])
            .await?;
        if !out.success() {
            return Ok(Vec::new());
        }
        Ok(out
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(limit)
            .map(|l| l.to_string())
            .collect())
    }

    pub async fn ls_remote(&self, remote: &str, branch: &str) -> AppResult<Option<String>> {
        let out = self
            .run(&["ls-remote", remote, &format!("refs/heads/{branch}")])
            .await?;
        if !out.success() {
            return Err(AppError::new(
                "git_ls_remote_failed",
                "Could not read the upstream repository",
            )
            .with_detail(out.stderr.trim()));
        }
        Ok(out
            .stdout
            .split_whitespace()
            .next()
            .map(|s| s.to_string())
            .filter(|s| s.len() >= 7))
    }
}

fn parse_porcelain_status(path: &str, raw: &str) -> RepoStatus {
    let mut status = RepoStatus {
        path: path.to_string(),
        exists: true,
        is_repository: true,
        ..RepoStatus::empty(path)
    };
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            if let Some(branch) = rest.strip_prefix("No commits yet on ") {
                status.branch = Some(branch.trim().to_string());
                continue;
            }
            let head = rest.split_whitespace().next().unwrap_or("");
            if head == "HEAD" || head.starts_with("HEAD") {
                status.detached = true;
                status.branch = None;
            } else {
                let name = head.split("...").next().unwrap_or(head).trim();
                status.branch = (!name.is_empty()).then(|| name.to_string());
            }
            continue;
        }
        if line.starts_with("??") {
            status.untracked_files += 1;
        } else if !line.trim().is_empty() {
            status.modified_files += 1;
        }
    }
    status
}

fn parse_commit_line(line: &str) -> Option<CommitInfo> {
    let mut parts = line.split('\u{1f}');
    let sha = parts.next()?.trim().to_string();
    if sha.len() < 7 {
        return None;
    }
    let short_sha = parts
        .next()
        .unwrap_or(&sha[..7.min(sha.len())])
        .trim()
        .to_string();
    let author = parts.next().unwrap_or("").trim().to_string();
    let date = parts.next().unwrap_or("").trim().to_string();
    let subject = parts.next().unwrap_or("").trim().to_string();
    let date_human = DateTime::parse_from_rfc3339(&date)
        .map(|d: DateTime<FixedOffset>| d.format("%d %b %Y, %H:%M").to_string())
        .unwrap_or_else(|_| date.clone());
    Some(CommitInfo {
        sha,
        short_sha,
        subject,
        author,
        date,
        date_human,
    })
}

fn parse_submodule_line(line: &str) -> Option<SubmoduleState> {
    let trimmed = line.trim_end();
    if trimmed.is_empty() {
        return None;
    }
    let marker = line.chars().next().unwrap_or(' ');
    let mut parts = trimmed.split_whitespace();
    let sha = parts
        .next()
        .map(|s| s.trim_start_matches(['-', '+', 'U', ' ']).to_string());
    let path = parts.next()?.to_string();
    Some(SubmoduleState {
        path,
        sha: sha.filter(|s| s.len() >= 7),
        initialized: marker != '-',
        modified: marker == '+',
    })
}

/// Clone the official upstream with its submodules. Progress is streamed so the
/// installer can show real percentages from git itself.
pub async fn clone_repo(
    url: &str,
    target: &Path,
    on_spawn: impl FnOnce(Option<u32>),
    on_line: impl FnMut(StreamLine),
) -> AppResult<()> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(crate::paths::fs_path(parent))
        .map_err(|e| AppError::io(format!("Creating {}", parent.display()), e))?;
    let spec = CommandSpec::new("git")
        .args([
            "--no-pager",
            "clone",
            "--progress",
            "--recurse-submodules",
            "--shallow-submodules",
            url,
        ])
        .arg(target.to_string_lossy().to_string())
        .cwd(parent)
        .env("GIT_TERMINAL_PROMPT", "0");
    let mut errors: Vec<String> = Vec::new();
    let mut sink = on_line;
    let exit = run_streaming(spec, on_spawn, |line| {
        let lower = line.text.to_ascii_lowercase();
        if lower.contains("fatal:")
            || lower.contains("error:")
            || lower.contains("remote: not found")
        {
            errors.push(line.text.clone());
        }
        sink(line);
    })
    .await?;
    if !exit.success {
        return Err(AppError::new("clone_failed", "Leonida could not be cloned")
            .with_detail(errors.join("\n")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_available() -> bool {
        crate::process::resolve_executable("git").is_some()
    }

    fn init_repo(dir: &Path) -> GitRepo {
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .current_dir(dir)
                .args(args)
                .env("GIT_TERMINAL_PROMPT", "0")
                .status()
                .expect("git runs");
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["init", "-q", "-b", "main"]);
        run(&[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "first commit",
        ]);
        GitRepo::new(dir)
    }

    #[tokio::test]
    async fn detects_repository_and_parses_state() {
        if !git_available() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        assert!(GitRepo::detect(tmp.path()).is_none());
        let repo = init_repo(tmp.path());
        assert!(GitRepo::detect(tmp.path()).is_some());
        let status = repo.status().await.unwrap();
        assert!(status.is_repository);
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert!(status.is_clean());
        assert!(status.head.is_some());
    }

    #[tokio::test]
    async fn counts_modified_and_untracked_separately() {
        if !git_available() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo(tmp.path());
        std::fs::write(tmp.path().join("tracked.txt"), "one").unwrap();
        let _ = repo.run(&["add", "tracked.txt"]).await.unwrap();
        let _ = repo
            .run(&[
                "-c",
                "user.email=t@e.st",
                "-c",
                "user.name=T",
                "commit",
                "-q",
                "-m",
                "add",
            ])
            .await
            .unwrap();
        std::fs::write(tmp.path().join("tracked.txt"), "two").unwrap();
        std::fs::write(tmp.path().join("untracked.txt"), "x").unwrap();
        let status = repo.status().await.unwrap();
        assert_eq!(status.modified_files, 1);
        assert_eq!(status.untracked_files, 1);
        assert!(!status.is_clean());
    }

    #[tokio::test]
    async fn parses_commit_list_and_backups() {
        if !git_available() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo(tmp.path());
        let commits = repo.log(5, None).await.unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].subject, "first commit");
        let head = repo.head().await.unwrap().unwrap();
        let backup = repo.create_backup_ref(&head, "test").await.unwrap();
        assert_eq!(backup, "launcher-backup/test");
        let backups = repo.list_backups().await.unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].sha, head);
    }

    #[tokio::test]
    async fn fast_forward_and_rollback_round_trip() {
        if !git_available() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo(tmp.path());
        let first = repo.head().await.unwrap().unwrap();
        let _ = repo
            .run(&[
                "-c",
                "user.email=t@e.st",
                "-c",
                "user.name=T",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "second",
            ])
            .await
            .unwrap();
        let second = repo.head().await.unwrap().unwrap();
        repo.create_backup_ref(&first, "before").await.unwrap();
        repo.reset_keep(&first).await.unwrap();
        assert_eq!(repo.head().await.unwrap().unwrap(), first);
        repo.fast_forward(&second).await.unwrap();
        assert_eq!(repo.head().await.unwrap().unwrap(), second);
    }

    #[test]
    fn parses_porcelain_branch_header() {
        let status = parse_porcelain_status(
            "D:\\repo",
            "## main...origin/main [behind 3]\n M file.txt\n?? new.txt\n",
        );
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.modified_files, 1);
        assert_eq!(status.untracked_files, 1);
        assert!(!status.detached);
    }

    #[test]
    fn parses_detached_head() {
        let status = parse_porcelain_status("D:\\repo", "## HEAD (no branch)\n");
        assert!(status.detached);
        assert!(status.branch.is_none());
    }

    #[test]
    fn parses_submodule_markers() {
        let uninitialized =
            parse_submodule_line("-1234567890abcdef third_party/rpf9-explorer").unwrap();
        assert!(!uninitialized.initialized);
        let ready =
            parse_submodule_line(" 1234567890abcdef third_party/rpf9-explorer (heads/main)")
                .unwrap();
        assert!(ready.initialized);
        assert_eq!(ready.path, "third_party/rpf9-explorer");
    }
}
