// Execution Events
// Progress reporting and event types for pipeline execution

use crate::parser::models::{JobStatus, StageStatus, StepStatus};

use std::time::Duration;
use tokio::sync::mpsc;

/// Sender for execution progress events
pub type ProgressSender = mpsc::UnboundedSender<ExecutionEvent>;

/// Receiver for execution progress events
pub type ProgressReceiver = mpsc::UnboundedReceiver<ExecutionEvent>;

/// Create a new progress channel
pub fn progress_channel() -> (ProgressSender, ProgressReceiver) {
    mpsc::unbounded_channel()
}

/// Events emitted during pipeline execution
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// Pipeline execution started
    PipelineStarted {
        pipeline_name: String,
        total_stages: usize,
    },

    /// Pipeline execution completed
    PipelineCompleted {
        pipeline_name: String,
        success: bool,
        duration: Duration,
    },

    /// Stage execution started
    StageStarted {
        stage_name: String,
        display_name: Option<String>,
        total_jobs: usize,
    },

    /// Stage execution completed
    StageCompleted {
        stage_name: String,
        status: StageStatus,
        duration: Duration,
    },

    /// Stage was skipped (condition evaluated to false)
    StageSkipped { stage_name: String, reason: String },

    /// Job execution started
    JobStarted {
        stage_name: String,
        job_name: String,
        display_name: Option<String>,
        matrix_instance: Option<String>,
        total_steps: usize,
    },

    /// Job execution completed
    JobCompleted {
        stage_name: String,
        job_name: String,
        matrix_instance: Option<String>,
        status: JobStatus,
        duration: Duration,
    },

    /// Job was skipped (condition evaluated to false)
    JobSkipped {
        stage_name: String,
        job_name: String,
        reason: String,
    },

    /// Step execution started
    StepStarted {
        stage_name: String,
        job_name: String,
        step_name: Option<String>,
        display_name: Option<String>,
        step_index: usize,
    },

    /// Step output (stdout/stderr)
    StepOutput {
        stage_name: String,
        job_name: String,
        step_name: Option<String>,
        step_index: usize,
        output: String,
        is_error: bool,
    },

    /// Step execution completed
    StepCompleted {
        stage_name: String,
        job_name: String,
        step_name: Option<String>,
        step_index: usize,
        status: StepStatus,
        duration: Duration,
        exit_code: Option<i32>,
    },

    /// Step was skipped (condition evaluated to false or disabled)
    StepSkipped {
        stage_name: String,
        job_name: String,
        step_name: Option<String>,
        step_index: usize,
        reason: String,
    },

    /// Variable was set during execution
    VariableSet {
        stage_name: String,
        job_name: String,
        name: String,
        value: String,
        is_output: bool,
        is_secret: bool,
    },

    /// Log message (info, warning, error)
    Log {
        level: LogLevel,
        message: String,
        stage_name: Option<String>,
        job_name: Option<String>,
    },

    /// Execution error occurred
    Error {
        message: String,
        stage_name: Option<String>,
        job_name: Option<String>,
        step_index: Option<usize>,
    },
}

/// Log level for log events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl ExecutionEvent {
    /// Create a pipeline started event
    pub fn pipeline_started(name: impl Into<String>, total_stages: usize) -> Self {
        Self::PipelineStarted {
            pipeline_name: name.into(),
            total_stages,
        }
    }

    /// Create a pipeline completed event
    pub fn pipeline_completed(name: impl Into<String>, success: bool, duration: Duration) -> Self {
        Self::PipelineCompleted {
            pipeline_name: name.into(),
            success,
            duration,
        }
    }

    /// Create a stage started event
    pub fn stage_started(
        name: impl Into<String>,
        display_name: Option<String>,
        total_jobs: usize,
    ) -> Self {
        Self::StageStarted {
            stage_name: name.into(),
            display_name,
            total_jobs,
        }
    }

    /// Create a stage completed event
    pub fn stage_completed(
        name: impl Into<String>,
        status: StageStatus,
        duration: Duration,
    ) -> Self {
        Self::StageCompleted {
            stage_name: name.into(),
            status,
            duration,
        }
    }

    /// Create a job started event
    pub fn job_started(
        stage_name: impl Into<String>,
        job_name: impl Into<String>,
        display_name: Option<String>,
        matrix_instance: Option<String>,
        total_steps: usize,
    ) -> Self {
        Self::JobStarted {
            stage_name: stage_name.into(),
            job_name: job_name.into(),
            display_name,
            matrix_instance,
            total_steps,
        }
    }

    /// Create a job completed event
    pub fn job_completed(
        stage_name: impl Into<String>,
        job_name: impl Into<String>,
        matrix_instance: Option<String>,
        status: JobStatus,
        duration: Duration,
    ) -> Self {
        Self::JobCompleted {
            stage_name: stage_name.into(),
            job_name: job_name.into(),
            matrix_instance,
            status,
            duration,
        }
    }

    /// Create a step started event
    pub fn step_started(
        stage_name: impl Into<String>,
        job_name: impl Into<String>,
        step_name: Option<String>,
        display_name: Option<String>,
        step_index: usize,
    ) -> Self {
        Self::StepStarted {
            stage_name: stage_name.into(),
            job_name: job_name.into(),
            step_name,
            display_name,
            step_index,
        }
    }

    /// Create a step output event
    pub fn step_output(
        stage_name: impl Into<String>,
        job_name: impl Into<String>,
        step_name: Option<String>,
        step_index: usize,
        output: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self::StepOutput {
            stage_name: stage_name.into(),
            job_name: job_name.into(),
            step_name,
            step_index,
            output: output.into(),
            is_error,
        }
    }

    /// Create a step completed event
    pub fn step_completed(
        stage_name: impl Into<String>,
        job_name: impl Into<String>,
        step_name: Option<String>,
        step_index: usize,
        status: StepStatus,
        duration: Duration,
        exit_code: Option<i32>,
    ) -> Self {
        Self::StepCompleted {
            stage_name: stage_name.into(),
            job_name: job_name.into(),
            step_name,
            step_index,
            status,
            duration,
            exit_code,
        }
    }

    /// Create an info log event
    pub fn info(
        message: impl Into<String>,
        stage_name: Option<String>,
        job_name: Option<String>,
    ) -> Self {
        Self::Log {
            level: LogLevel::Info,
            message: message.into(),
            stage_name,
            job_name,
        }
    }

    /// Create a warning log event
    pub fn warning(
        message: impl Into<String>,
        stage_name: Option<String>,
        job_name: Option<String>,
    ) -> Self {
        Self::Log {
            level: LogLevel::Warning,
            message: message.into(),
            stage_name,
            job_name,
        }
    }

    /// Create an error log event
    pub fn error(
        message: impl Into<String>,
        stage_name: Option<String>,
        job_name: Option<String>,
    ) -> Self {
        Self::Log {
            level: LogLevel::Error,
            message: message.into(),
            stage_name,
            job_name,
        }
    }

    /// Create an execution error event
    pub fn execution_error(
        message: impl Into<String>,
        stage_name: Option<String>,
        job_name: Option<String>,
        step_index: Option<usize>,
    ) -> Self {
        Self::Error {
            message: message.into(),
            stage_name,
            job_name,
            step_index,
        }
    }
}

/// Helper trait for sending events, ignoring errors (fire-and-forget)
pub trait EventSender {
    fn send_event(&self, event: ExecutionEvent);
}

impl EventSender for ProgressSender {
    fn send_event(&self, event: ExecutionEvent) {
        let _ = self.send(event);
    }
}

impl EventSender for Option<ProgressSender> {
    fn send_event(&self, event: ExecutionEvent) {
        if let Some(sender) = self {
            let _ = sender.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_channel() {
        let (tx, mut rx) = progress_channel();

        tx.send_event(ExecutionEvent::pipeline_started("test", 2));
        tx.send_event(ExecutionEvent::stage_started("Build", None, 1));

        let event1 = rx.recv().await.unwrap();
        assert!(matches!(event1, ExecutionEvent::PipelineStarted { .. }));

        let event2 = rx.recv().await.unwrap();
        assert!(matches!(event2, ExecutionEvent::StageStarted { .. }));
    }

    #[test]
    fn test_event_construction() {
        let event = ExecutionEvent::job_completed(
            "Build",
            "Compile",
            Some("linux".to_string()),
            JobStatus::Succeeded,
            Duration::from_secs(30),
        );

        if let ExecutionEvent::JobCompleted {
            stage_name,
            job_name,
            matrix_instance,
            status,
            duration,
        } = event
        {
            assert_eq!(stage_name, "Build");
            assert_eq!(job_name, "Compile");
            assert_eq!(matrix_instance, Some("linux".to_string()));
            assert_eq!(status, JobStatus::Succeeded);
            assert_eq!(duration, Duration::from_secs(30));
        } else {
            panic!("wrong event type");
        }
    }

    #[test]
    fn test_optional_sender() {
        let sender: Option<ProgressSender> = None;
        // Should not panic
        sender.send_event(ExecutionEvent::info("test", None, None));
    }
}
