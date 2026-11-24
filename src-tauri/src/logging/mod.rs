use crate::error::AppResult;
use crate::paths::fs_path;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

pub const MAX_BUFFER_LINES: usize = 4000;
const MAX_LINE_CHARS: usize = 8000;
const MAX_LOG_FILES: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Launcher,
    Updater,
    Git,
    Dependencies,
    Build,
    Diagnostics,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Launcher => "Launcher",
            Category::Updater => "Updater",
            Category::Git => "Git",
            Category::Dependencies => "Dependencies",
            Category::Build => "Build",
            Category::Diagnostics => "Diagnostics",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "launcher" => Some(Category::Launcher),
            "updater" => Some(Category::Updater),
            "git" => Some(Category::Git),
            "dependencies" | "deps" => Some(Category::Dependencies),
            "build" => Some(Category::Build),
            "diagnostics" => Some(Category::Diagnostics),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub timestamp: String,
    pub category: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileInfo {
    pub name: String,
    pub size_bytes: u64,
    pub modified: String,
}

pub struct LogManager {
    dir: Mutex<PathBuf>,
    writer: Mutex<Option<BufWriter<File>>>,
    buffer: Mutex<VecDeque<LogLine>>,
    app: Mutex<Option<AppHandle>>,
    verbose: AtomicBool,
    retention_days: Mutex<u32>,
}

impl LogManager {
    pub fn new() -> Self {
        Self {
            dir: Mutex::new(PathBuf::new()),
            writer: Mutex::new(None),
            buffer: Mutex::new(VecDeque::new()),
            app: Mutex::new(None),
            verbose: AtomicBool::new(false),
            retention_days: Mutex::new(14),
        }
    }

    pub fn init(&self, dir: &Path, retention_days: u32, app: AppHandle) -> AppResult<()> {
        std::fs::create_dir_all(fs_path(dir))
            .map_err(|e| crate::error::AppError::io(format!("Creating {}", dir.display()), e))?;
        *self.dir.lock().unwrap() = dir.to_path_buf();
        *self.retention_days.lock().unwrap() = retention_days.max(1);
        *self.app.lock().unwrap() = Some(app);
        self.rotate();
        self.open_session_file()?;
        Ok(())
    }

    pub fn set_verbose(&self, verbose: bool) {
        self.verbose.store(verbose, Ordering::Relaxed);
    }

    pub fn is_verbose(&self) -> bool {
        self.verbose.load(Ordering::Relaxed)
    }

    fn session_path(dir: &Path) -> PathBuf {
        dir.join(format!(
            "launcher-{}.log",
            chrono::Local::now().format("%Y-%m-%d")
        ))
    }

    fn open_session_file(&self) -> AppResult<()> {
        let dir = self.dir.lock().unwrap().clone();
        if dir.as_os_str().is_empty() {
            return Ok(());
        }
        let path = fs_path(&Self::session_path(&dir));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| {
                crate::error::AppError::io(format!("Opening log file {}", path.display()), e)
            })?;
        *self.writer.lock().unwrap() = Some(BufWriter::new(file));
        Ok(())
    }

    pub fn log(&self, category: Category, level: Level, message: impl Into<String>) {
        if level == Level::Debug && !self.is_verbose() {
            return;
        }
        let message = redact(&truncate(&message.into(), MAX_LINE_CHARS));
        let line = LogLine {
            timestamp: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            category: category.as_str().to_string(),
            level: level.as_str().to_string(),
            message,
        };

        {
            let mut buffer = self.buffer.lock().unwrap();
            buffer.push_back(line.clone());
            while buffer.len() > MAX_BUFFER_LINES {
                buffer.pop_front();
            }
        }

        {
            let mut guard = self.writer.lock().unwrap();
            if let Some(writer) = guard.as_mut() {
                let _ = writeln!(
                    writer,
                    "{} [{}] {} {}",
                    line.timestamp, line.category, line.level, line.message
                );
                let _ = writer.flush();
            }
        }

        if let Some(app) = self.app.lock().unwrap().as_ref() {
            let _ = app.emit("log://line", &line);
        }
    }

    pub fn info(&self, category: Category, message: impl Into<String>) {
        self.log(category, Level::Info, message);
    }

    pub fn warn(&self, category: Category, message: impl Into<String>) {
        self.log(category, Level::Warn, message);
    }

    pub fn error(&self, category: Category, message: impl Into<String>) {
        self.log(category, Level::Error, message);
    }

    pub fn debug(&self, category: Category, message: impl Into<String>) {
        self.log(category, Level::Debug, message);
    }

    pub fn snapshot(&self, limit: usize) -> Vec<LogLine> {
        let buffer = self.buffer.lock().unwrap();
        let skip = buffer.len().saturating_sub(limit);
        buffer.iter().skip(skip).cloned().collect()
    }

    pub fn list_files(&self) -> Vec<LogFileInfo> {
        let dir = self.dir.lock().unwrap().clone();
        let mut files = Vec::new();
        let Ok(entries) = std::fs::read_dir(fs_path(&dir)) else {
            return files;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("log") {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = metadata
                .modified()
                .ok()
                .map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                })
                .unwrap_or_default();
            files.push(LogFileInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                modified,
            });
        }
        files.sort_by(|a, b| b.name.cmp(&a.name));
        files
    }

    /// Log names come from the UI; only a bare file name inside the log folder is
    /// ever read, which keeps traversal out of the picture.
    pub fn read_file(&self, name: &str, max_lines: usize) -> AppResult<String> {
        if !is_safe_file_name(name) {
            return Err(crate::error::AppError::new(
                "invalid_log_name",
                "That log file name is not allowed",
            ));
        }
        let dir = self.dir.lock().unwrap().clone();
        let path = dir.join(name);
        let raw = std::fs::read_to_string(fs_path(&path))
            .map_err(|e| crate::error::AppError::io(format!("Reading {}", name), e))?;
        let lines: Vec<&str> = raw.lines().collect();
        let start = lines.len().saturating_sub(max_lines.max(1));
        Ok(lines[start..].join("\n"))
    }

    pub fn clear(&self, keep_days: u32) {
        let dir = self.dir.lock().unwrap().clone();
        let cutoff = chrono::Local::now() - chrono::Duration::days(keep_days as i64);
        let Ok(entries) = std::fs::read_dir(fs_path(&dir)) else {
            return;
        };
        let today = Self::session_path(&dir);
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("log") || path == today {
                continue;
            }
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .map(|t| chrono::DateTime::<chrono::Local>::from(t) < cutoff)
                .unwrap_or(false);
            if old {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    pub fn rotate(&self) {
        let retention = *self.retention_days.lock().unwrap();
        self.clear(retention);
        let dir = self.dir.lock().unwrap().clone();
        let mut logs = self.list_files();
        if logs.len() <= MAX_LOG_FILES {
            return;
        }
        logs.sort_by(|a, b| a.name.cmp(&b.name));
        let excess = logs.len() - MAX_LOG_FILES;
        for file in logs.into_iter().take(excess) {
            let _ = std::fs::remove_file(fs_path(&dir.join(&file.name)));
        }
    }
}

impl Default for LogManager {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars).collect();
    out.push_str(" … [truncated]");
    out
}

fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() < 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !name.contains("..")
}

fn patterns() -> &'static Vec<(Regex, &'static str)> {
    static PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            (
                Regex::new(r"(?i)(gh[pousr]_[A-Za-z0-9]{16,}|github_pat_[A-Za-z0-9_]{20,})").unwrap(),
                "<redacted-token>",
            ),
            (
                Regex::new(
                    r#"(?i)((?:access[_-]?token|refresh[_-]?token|api[_-]?key|secret|password)\s*[=:]\s*)("[^"]*"|[^\s,;]+)"#,
                )
                .unwrap(),
                "${1}<redacted>",
            ),
            (
                Regex::new(r"(?i)(Bearer\s+)[A-Za-z0-9._~+/=-]{12,}").unwrap(),
                "${1}<redacted>",
            ),
            (
                Regex::new(r"(?i)(https?://)[^/\s:@]+:[^/\s@]+@").unwrap(),
                "${1}<redacted>@",
            ),
            (
                Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----").unwrap(),
                "<redacted-private-key>",
            ),
        ]
    })
}

/// Secrets must never reach a log file, even if a tool prints them.
pub fn redact(input: &str) -> String {
    let mut out = input.to_string();
    for (regex, replacement) in patterns() {
        out = regex.replace_all(&out, *replacement).to_string();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_tokens_and_bearer_headers() {
        assert!(!redact("using ghp_abcdefghijklmnopqrstuvwxyz0123").contains("ghp_"));
        let line = redact("Authorization: Bearer abcdefghijklmnopqrstuvwxyz");
        assert!(!line.contains("abcdefghijklmnopqrstuvwxyz"));
        let key = redact("password=hunter2 and api_key: zzz");
        assert!(!key.contains("hunter2"));
        assert!(!key.contains("zzz"));
    }

    #[test]
    fn leaves_normal_text_alone() {
        let text = "shaders: 42% translated in 3m";
        assert_eq!(redact(text), text);
    }

    #[test]
    fn rejects_traversal_names() {
        assert!(!is_safe_file_name("../settings.json"));
        assert!(!is_safe_file_name(".."));
        assert!(is_safe_file_name("launcher-2026-11-24.log"));
    }

    #[test]
    fn truncates_long_lines_only() {
        assert_eq!(truncate("short", 10), "short");
        assert!(truncate(&"x".repeat(20), 10).contains("truncated"));
    }
}
