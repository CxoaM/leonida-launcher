use serde::{Deserialize, Serialize};

/// A user-facing error. Technical detail is kept separate so the UI can show a
/// short sentence and reveal the raw output only when the user asks for it.
#[derive(Debug, Clone)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        if !detail.trim().is_empty() {
            self.detail = Some(detail);
        }
        self
    }

    pub fn io(context: impl Into<String>, err: std::io::Error) -> Self {
        Self {
            code: "io_error".into(),
            message: format!("{} failed", context.into()),
            detail: Some(err.to_string()),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Repr<'a> {
            code: &'a str,
            message: &'a str,
            detail: Option<&'a str>,
        }
        Repr {
            code: &self.code,
            message: &self.message,
            detail: self.detail.as_deref(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AppError {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Repr {
            code: String,
            message: String,
            detail: Option<String>,
        }
        let repr = Repr::deserialize(deserializer)?;
        Ok(AppError {
            code: repr.code,
            message: repr.message,
            detail: repr.detail,
        })
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError {
            code: "io_error".into(),
            message: err.to_string(),
            detail: None,
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError {
            code: "invalid_json".into(),
            message: "Stored data is unreadable".into(),
            detail: Some(err.to_string()),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Turn a failed command into an error whose detail carries the captured output.
pub fn command_failed(
    context: impl Into<String>,
    code: Option<i32>,
    stderr: &str,
    stdout: &str,
) -> AppError {
    let message = match code {
        Some(c) => format!("{} (exit code {})", context.into(), c),
        None => context.into(),
    };
    let mut detail = String::new();
    if !stderr.trim().is_empty() {
        detail.push_str(stderr.trim());
    }
    if !stdout.trim().is_empty() {
        if !detail.is_empty() {
            detail.push_str("\n\n");
        }
        detail.push_str(stdout.trim());
    }
    AppError::new("command_failed", message).with_detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_without_detail_when_empty() {
        let err = AppError::new("x", "Something failed").with_detail("   ");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "x");
        assert_eq!(json["message"], "Something failed");
        assert!(json["detail"].is_null());
    }

    #[test]
    fn command_failure_keeps_output() {
        let err = command_failed("git fetch failed", Some(128), "fatal: no network", "");
        assert!(err.message.contains("128"));
        assert_eq!(err.detail.as_deref(), Some("fatal: no network"));
    }
}
