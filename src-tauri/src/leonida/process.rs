use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_LINES: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Idle,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Idle => "idle",
            JobStatus::Running => "running",
            JobStatus::Cancelling => "cancelling",
            JobStatus::Succeeded => "succeeded",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub kind: String,
    pub status: String,
    pub id: Option<String>,
    pub command: Option<String>,
    pub stages: Vec<String>,
    pub stage_index: usize,
    pub stage_count: usize,
    pub current_stage: Option<String>,
    pub percent: Option<f64>,
    pub eta: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error: Option<String>,
    pub lines: Vec<String>,
    pub pid: Option<u32>,
}

impl JobSnapshot {
    pub fn is_running(&self) -> bool {
        matches!(self.status.as_str(), "running" | "cancelling")
    }
}

#[derive(Debug)]
struct Inner {
    kind: String,
    status: JobStatus,
    id: Option<String>,
    command: Option<String>,
    stages: Vec<String>,
    stage_index: usize,
    current_stage: Option<String>,
    percent: Option<f64>,
    eta: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    error: Option<String>,
    lines: VecDeque<String>,
    pid: Option<u32>,
}

/// One supervised long-running process group: either a Leonida build or an
/// installation. Kept in shared state so the UI can re-attach after a page switch
/// and the cancel command can reach the real PID.
pub struct JobRuntime {
    inner: Mutex<Inner>,
}

impl JobRuntime {
    pub fn new(kind: &str) -> Self {
        Self {
            inner: Mutex::new(Inner {
                kind: kind.to_string(),
                status: JobStatus::Idle,
                id: None,
                command: None,
                stages: Vec::new(),
                stage_index: 0,
                current_stage: None,
                percent: None,
                eta: None,
                started_at: None,
                finished_at: None,
                error: None,
                lines: VecDeque::new(),
                pid: None,
            }),
        }
    }

    pub fn kind(&self) -> String {
        self.inner.lock().unwrap().kind.clone()
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self.inner.lock().unwrap().status,
            JobStatus::Running | JobStatus::Cancelling
        )
    }

    pub fn try_begin(&self, id: &str, command: &str, stages: Vec<String>) -> AppResult<()> {
        let mut inner = self.inner.lock().unwrap();
        if matches!(inner.status, JobStatus::Running | JobStatus::Cancelling) {
            return Err(AppError::new(
                "busy",
                format!("A {} is already running", inner.kind),
            ));
        }
        inner.status = JobStatus::Running;
        inner.id = Some(id.to_string());
        inner.command = Some(command.to_string());
        inner.stages = stages;
        inner.stage_index = 0;
        inner.current_stage = None;
        inner.percent = None;
        inner.eta = None;
        inner.started_at = Some(timestamp());
        inner.finished_at = None;
        inner.error = None;
        inner.lines.clear();
        inner.pid = None;
        Ok(())
    }

    pub fn set_pid(&self, pid: Option<u32>) {
        self.inner.lock().unwrap().pid = pid;
    }

    pub fn set_stage(&self, stage: &str) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(index) = inner.stages.iter().position(|s| s == stage) {
            inner.stage_index = index;
        }
        inner.current_stage = Some(stage.to_string());
        inner.percent = None;
        inner.eta = None;
    }

    pub fn progress(&self, percent: Option<f64>, eta: Option<String>) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(p) = percent {
            inner.percent = Some(p);
        }
        if let Some(e) = eta {
            inner.eta = Some(e);
        }
    }

    pub fn push_line(&self, line: impl Into<String>) {
        let mut inner = self.inner.lock().unwrap();
        inner.lines.push_back(line.into());
        while inner.lines.len() > MAX_LINES {
            inner.lines.pop_front();
        }
    }

    pub fn note(&self, line: impl Into<String>) {
        self.push_line(line);
    }

    /// Returns the PID that must be killed, and marks the job as cancelling so a
    /// second click does not spawn a second kill.
    pub fn request_cancel(&self) -> Option<u32> {
        let mut inner = self.inner.lock().unwrap();
        if inner.status == JobStatus::Running {
            inner.status = JobStatus::Cancelling;
            inner
                .lines
                .push_back("Cancellation requested by the user.".to_string());
        }
        inner.pid
    }

    pub fn finish(&self, status: JobStatus, error: Option<String>) {
        let mut inner = self.inner.lock().unwrap();
        inner.status = status;
        inner.finished_at = Some(timestamp());
        inner.pid = None;
        if let Some(err) = error {
            inner.error = Some(err.clone());
            inner.lines.push_back(err);
        }
    }

    pub fn fail(&self, error: String) {
        self.finish(JobStatus::Failed, Some(error));
    }

    pub fn snapshot(&self) -> JobSnapshot {
        let inner = self.inner.lock().unwrap();
        JobSnapshot {
            kind: inner.kind.clone(),
            status: inner.status.as_str().to_string(),
            id: inner.id.clone(),
            command: inner.command.clone(),
            stages: inner.stages.clone(),
            stage_index: inner.stage_index,
            stage_count: inner.stages.len(),
            current_stage: inner.current_stage.clone(),
            percent: inner.percent,
            eta: inner.eta.clone(),
            started_at: inner.started_at.clone(),
            finished_at: inner.finished_at.clone(),
            error: inner.error.clone(),
            lines: inner.lines.iter().cloned().collect(),
            pid: inner.pid,
        }
    }
}

impl Default for JobRuntime {
    fn default() -> Self {
        Self::new("job")
    }
}

pub fn timestamp() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_two_jobs_at_once_and_reports_progress() {
        let runtime = JobRuntime::new("build");
        runtime
            .try_begin(
                "run-1",
                "python -I make_game.py",
                vec!["check".into(), "shaders".into()],
            )
            .unwrap();
        assert!(runtime.is_running());
        assert!(runtime.try_begin("run-2", "x", vec![]).is_err());
        runtime.set_pid(Some(4242));
        runtime.set_stage("shaders");
        runtime.progress(Some(42.5), Some("1h20m".into()));
        runtime.push_line("shaders: 42.5%");
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.stage_index, 1);
        assert_eq!(snapshot.percent, Some(42.5));
        assert_eq!(snapshot.lines.len(), 1);
        assert_eq!(runtime.request_cancel(), Some(4242));
        assert_eq!(runtime.snapshot().status, "cancelling");
        runtime.finish(JobStatus::Cancelled, None);
        assert!(!runtime.is_running());
        assert!(runtime.try_begin("run-3", "y", vec![]).is_ok());
    }

    #[test]
    fn line_buffer_is_bounded() {
        let runtime = JobRuntime::new("build");
        runtime.try_begin("r", "c", vec![]).unwrap();
        for i in 0..MAX_LINES + 50 {
            runtime.push_line(format!("line {i}"));
        }
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.lines.len(), MAX_LINES);
        assert!(snapshot.lines.last().unwrap().contains("line 649"));
    }

    #[test]
    fn a_new_job_clears_the_previous_one() {
        let runtime = JobRuntime::new("install");
        runtime.try_begin("a", "cmd", vec![]).unwrap();
        runtime.push_line("old");
        runtime.finish(JobStatus::Failed, Some("boom".into()));
        runtime.try_begin("b", "cmd2", vec![]).unwrap();
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.status, "running");
        assert!(snapshot.lines.is_empty());
        assert!(snapshot.error.is_none());
    }
}
