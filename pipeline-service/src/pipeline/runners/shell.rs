use crate::pipeline::executor::{ExecutionEvent, ProgressSender};
use crate::pipeline::models::{ExecutionContext, Step, StepAction, StepResult, StepStatus};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct ShellRunner {
    context: ExecutionContext,
}

impl ShellRunner {
    pub fn new(context: ExecutionContext) -> Self {
        Self { context }
    }

    pub async fn run(&self, step: &Step, progress_tx: Option<&ProgressSender>) -> StepResult {
        let (command, args) = self.prepare_command(step);

        let mut cmd = Command::new(command);
        cmd.args(args)
            .current_dir(&self.context.working_dir)
            .envs(&self.context.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return StepResult {
                    step_name: step.name.clone(),
                    status: StepStatus::Failed,
                    output: String::new(),
                    error: Some(format!("Failed to spawn command: {}", e)),
                    duration: std::time::Duration::from_secs(0),
                    exit_code: None,
                };
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let step_name = step.name.clone();
        let progress_tx_clone = progress_tx.cloned();

        let stdout_task = tokio::spawn(async move {
            let mut lines = Vec::new();
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                if let Some(tx) = &progress_tx_clone {
                    let _ = tx.send(ExecutionEvent::StepOutput {
                        step_name: step_name.clone(),
                        output: line.clone(),
                    });
                }
                lines.push(line);
            }
            lines.join("\n")
        });

        let stderr_task = tokio::spawn(async move {
            let mut lines = Vec::new();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                lines.push(line);
            }
            lines.join("\n")
        });

        let output = stdout_task.await.unwrap_or_default();
        let mut error_output = stderr_task.await.unwrap_or_default();

        let status = child.wait().await;

        let (exit_code, step_status) = match status {
            Ok(exit_status) => {
                let code = exit_status.code();
                let status = if exit_status.success() {
                    StepStatus::Success
                } else {
                    StepStatus::Failed
                };
                (code, status)
            }
            Err(e) => {
                error_output = format!("Process error: {}", e);
                (None, StepStatus::Failed)
            }
        };

        StepResult {
            step_name: step.name.clone(),
            status: step_status,
            output,
            error: if error_output.is_empty() {
                None
            } else {
                Some(error_output)
            },
            duration: std::time::Duration::from_secs(0),
            exit_code,
        }
    }

    fn prepare_command(&self, step: &Step) -> (String, Vec<String>) {
        match &step.action {
            StepAction::Command(cmd) => {
                if cfg!(target_os = "windows") {
                    ("cmd".to_string(), vec!["/C".to_string(), cmd.clone()])
                } else {
                    ("sh".to_string(), vec!["-c".to_string(), cmd.clone()])
                }
            }
            StepAction::Shell { shell, script } => {
                let shell_cmd = shell.clone().unwrap_or_else(|| {
                    if cfg!(target_os = "windows") {
                        "cmd".to_string()
                    } else {
                        "sh".to_string()
                    }
                });

                if cfg!(target_os = "windows") {
                    (shell_cmd, vec!["/C".to_string(), script.clone()])
                } else {
                    (shell_cmd, vec!["-c".to_string(), script.clone()])
                }
            }
        }
    }
}
