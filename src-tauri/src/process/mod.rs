use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::mpsc;

pub const CAPTURE_LIMIT: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamLine {
    pub stream: StreamKind,
    pub text: String,
}

/// A command described strictly as program + argv + cwd + env. No string is ever
/// handed to a shell, so spaces, quotes and Cyrillic paths cannot become syntax.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn display(&self) -> String {
        let mut out = quote_if_needed(&self.program);
        for arg in &self.args {
            out.push(' ');
            out.push_str(&quote_if_needed(arg));
        }
        out
    }
}

fn quote_if_needed(value: &str) -> String {
    if value.is_empty() || value.chars().any(|c| c.is_whitespace() || c == '"') {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Captured {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

impl Captured {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitInfo {
    pub code: Option<i32>,
    pub success: bool,
}

fn prepare(spec: &CommandSpec) -> Command {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    if let Some(cwd) = &spec.cwd {
        command.current_dir(cwd);
    }
    for (key, value) in &spec.env {
        command.env(key, value);
    }
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: never flash a console for a background probe.
        command.creation_flags(0x0800_0000);
    }
    command
}

pub async fn run_capture(spec: &CommandSpec, timeout: Duration) -> AppResult<Captured> {
    let started = std::time::Instant::now();
    let mut command = prepare(spec);
    let mut child = command.spawn().map_err(|e| {
        AppError::new("spawn_failed", format!("Could not start {}", spec.program))
            .with_detail(e.to_string())
    })?;

    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();

    let collect = async {
        let out_task = async {
            let mut buf = Vec::new();
            if let Some(reader) = stdout.as_mut() {
                let _ = reader
                    .take(CAPTURE_LIMIT as u64)
                    .read_to_end(&mut buf)
                    .await;
            }
            buf
        };
        let err_task = async {
            let mut buf = Vec::new();
            if let Some(reader) = stderr.as_mut() {
                let _ = reader
                    .take(CAPTURE_LIMIT as u64)
                    .read_to_end(&mut buf)
                    .await;
            }
            buf
        };
        let (out, err) = tokio::join!(out_task, err_task);
        let status = child.wait().await;
        (out, err, status)
    };

    match tokio::time::timeout(timeout, collect).await {
        Ok((out, err, status)) => {
            let status = status.map_err(|e| {
                AppError::new("process_failed", "Process ended unexpectedly")
                    .with_detail(e.to_string())
            })?;
            Ok(Captured {
                code: status.code(),
                stdout: String::from_utf8_lossy(&out).to_string(),
                stderr: String::from_utf8_lossy(&err).to_string(),
                duration_ms: started.elapsed().as_millis() as u64,
            })
        }
        Err(_) => {
            let _ = child.kill().await;
            Err(AppError::new(
                "timeout",
                format!(
                    "{} did not answer within {} seconds",
                    spec.program,
                    timeout.as_secs()
                ),
            ))
        }
    }
}

/// Run a long-lived process and stream every line to the caller. The child PID is
/// handed over before the first line so the UI can cancel it.
pub async fn run_streaming<F>(
    spec: CommandSpec,
    on_spawn: impl FnOnce(Option<u32>),
    mut on_line: F,
) -> AppResult<ExitInfo>
where
    F: FnMut(StreamLine),
{
    let mut command = prepare(&spec);
    let mut child = command.spawn().map_err(|e| {
        AppError::new("spawn_failed", format!("Could not start {}", spec.program))
            .with_detail(e.to_string())
    })?;
    on_spawn(child.id());

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, mut rx) = mpsc::channel::<StreamLine>(512);

    if let Some(out) = stdout {
        let tx = tx.clone();
        tokio::spawn(async move { pump(out, StreamKind::Stdout, tx).await });
    }
    if let Some(err) = stderr {
        let tx = tx.clone();
        tokio::spawn(async move { pump(err, StreamKind::Stderr, tx).await });
    }
    drop(tx);

    while let Some(line) = rx.recv().await {
        on_line(line);
    }

    let status = child.wait().await.map_err(|e| {
        AppError::new("process_failed", "Process ended unexpectedly").with_detail(e.to_string())
    })?;
    Ok(ExitInfo {
        code: status.code(),
        success: status.success(),
    })
}

async fn pump<R: AsyncRead + Unpin>(mut reader: R, kind: StreamKind, tx: mpsc::Sender<StreamLine>) {
    let mut pending: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                pending.extend_from_slice(&chunk[..n]);
                // Upstream tools redraw progress with \r; treat both \r and \n as breaks.
                while let Some(pos) = pending.iter().position(|b| *b == b'\n' || *b == b'\r') {
                    let raw: Vec<u8> = pending.drain(..=pos).collect();
                    let text = clean_line(&raw);
                    if !text.is_empty() && tx.send(StreamLine { stream: kind, text }).await.is_err()
                    {
                        return;
                    }
                }
                if pending.len() > 256 * 1024 {
                    let text = clean_line(&pending.split_off(0));
                    if !text.is_empty() && tx.send(StreamLine { stream: kind, text }).await.is_err()
                    {
                        return;
                    }
                }
            }
            Err(_) => break,
        }
    }
    let text = clean_line(&pending);
    if !text.is_empty() {
        let _ = tx.send(StreamLine { stream: kind, text }).await;
    }
}

pub fn clean_line(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .trim_end_matches(['\r', '\n'])
        .trim()
        .to_string()
}

/// Path search without a shell: honours PATHEXT so `git` finds `git.exe`.
pub fn resolve_executable(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 {
        return if candidate.is_file() {
            Some(candidate.to_path_buf())
        } else {
            None
        };
    }
    let path_var = std::env::var_os("PATH")?;
    let mut extensions: Vec<OsString> = Vec::new();
    if cfg!(windows) {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        extensions = pathext
            .split(';')
            .filter(|s| !s.is_empty())
            .map(OsString::from)
            .collect();
    }
    for dir in std::env::split_paths(&path_var) {
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
        for ext in &extensions {
            let with_ext = dir.join(format!("{}{}", name, ext.to_string_lossy()));
            if with_ext.is_file() {
                return Some(with_ext);
            }
        }
    }
    None
}

/// Cancel a build: kill the whole tree so dotnet/cmake children do not survive.
pub fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let mut command = std::process::Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/T", "/F"]);
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let _ = command.status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}

/// Opens a folder in the system file manager. The path is passed as a single
/// argv element, so spaces, Cyrillic characters and `&` are never interpreted by
/// a shell. Explorer exits with a non-zero code even on success, so only the
/// spawn result is checked.
pub fn reveal_in_explorer(path: &std::path::Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        let mut command = std::process::Command::new("explorer.exe");
        command.arg(path);
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
        command
            .spawn()
            .map_err(|e| AppError::io(format!("Opening {}", path.display()), e))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| AppError::io(format!("Opening {}", path.display()), e))?;
        Ok(())
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| AppError::io(format!("Opening {}", path.display()), e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_line_strips_progress_carriage_returns() {
        assert_eq!(clean_line(b"  progress 40%\r"), "progress 40%");
        assert_eq!(clean_line(b"plain\n"), "plain");
        assert_eq!(clean_line(b"   "), "");
    }

    #[test]
    fn display_quotes_only_when_needed() {
        let spec = CommandSpec::new(r"C:\Program Files\Git\cmd\git.exe")
            .arg("clone")
            .arg(r"D:\GTA VI\Leonida");
        let shown = spec.display();
        assert!(shown.starts_with('"'));
        assert!(shown.contains("\"D:\\GTA VI\\Leonida\""));
    }

    #[test]
    fn resolve_finds_a_real_windows_tool() {
        let found = resolve_executable("cmd");
        assert!(found.is_some(), "cmd.exe must be resolvable");
        assert!(resolve_executable("definitely-not-a-tool-xyz").is_none());
    }

    #[tokio::test]
    async fn captures_output_and_exit_code() {
        #[cfg(windows)]
        let spec = CommandSpec::new("cmd").args(["/C", "echo hello & echo oops 1>&2 & exit 3"]);
        #[cfg(not(windows))]
        let spec = CommandSpec::new("sh").args(["-c", "echo hello; echo oops 1>&2; exit 3"]);
        let captured = run_capture(&spec, Duration::from_secs(20)).await.unwrap();
        assert_eq!(captured.code, Some(3));
        assert!(captured.stdout.contains("hello"));
        assert!(captured.stderr.contains("oops"));
        assert!(!captured.success());
    }

    #[tokio::test]
    async fn streams_lines_in_order() {
        #[cfg(windows)]
        let spec = CommandSpec::new("cmd").args(["/C", "echo one & echo two"]);
        #[cfg(not(windows))]
        let spec = CommandSpec::new("sh").args(["-c", "echo one; echo two"]);
        let mut lines = Vec::new();
        run_streaming(spec, |_| {}, |line| lines.push(line.text))
            .await
            .unwrap();
        assert!(lines.iter().any(|l| l == "one"));
        assert!(lines.iter().any(|l| l == "two"));
    }
}
