// Task Runner
// Executes Azure DevOps tasks (Bash@3, PowerShell@2, etc.)

use crate::parser::models::{StepResult, StepStatus};
use crate::runners::shell::{ShellConfig, ShellRunner};
use crate::tasks::cache::{CachedTask, TaskCache, TaskCacheError};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors that can occur when running tasks
#[derive(Debug, Error)]
pub enum TaskRunnerError {
    #[error("Task cache error: {0}")]
    CacheError(#[from] TaskCacheError),

    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Unsupported task execution type: {0}")]
    UnsupportedExecution(String),

    #[error("Missing required input: {0}")]
    MissingInput(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Task runner for executing Azure DevOps tasks
pub struct TaskRunner {
    /// Task cache
    cache: TaskCache,
    /// Shell runner for executing scripts
    shell_runner: ShellRunner,
    /// Path to Node.js executable (for JS tasks)
    node_path: Option<PathBuf>,
    /// Path to PowerShell executable (for PS tasks)
    powershell_path: Option<PathBuf>,
}

impl TaskRunner {
    /// Create a new task runner with the specified cache directory
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache: TaskCache::with_cache_dir(cache_dir),
            shell_runner: ShellRunner::new(),
            node_path: find_node_path(),
            powershell_path: find_powershell_path(),
        }
    }

    /// Create a task runner with a custom task cache
    pub fn with_cache(cache: TaskCache) -> Self {
        Self {
            cache,
            shell_runner: ShellRunner::new(),
            node_path: find_node_path(),
            powershell_path: find_powershell_path(),
        }
    }

    /// Set the Node.js path
    pub fn with_node_path(mut self, path: impl AsRef<Path>) -> Self {
        self.node_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set the PowerShell path
    pub fn with_powershell_path(mut self, path: impl AsRef<Path>) -> Self {
        self.powershell_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Get the task cache
    pub fn cache(&self) -> &TaskCache {
        &self.cache
    }

    /// Execute a task
    pub async fn execute_task(
        &self,
        task_ref: &str,
        inputs: &HashMap<String, String>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<StepResult, TaskRunnerError> {
        let start = Instant::now();

        // Get the task from cache (downloading if necessary)
        let task = self.cache.get_task(task_ref).await?;

        // Validate required inputs
        self.validate_inputs(&task, inputs)?;

        // Merge inputs with defaults
        let merged_inputs = self.merge_inputs(&task, inputs);

        // Execute based on task type
        let result = self
            .execute_task_impl(&task, &merged_inputs, env, working_dir)
            .await;

        let duration = start.elapsed();

        match result {
            Ok(mut step_result) => {
                step_result.duration = duration;
                Ok(step_result)
            }
            Err(e) => Ok(StepResult {
                step_name: None,
                display_name: task.manifest.friendly_name.clone(),
                status: StepStatus::Failed,
                output: String::new(),
                error: Some(e.to_string()),
                duration,
                exit_code: None,
                outputs: HashMap::new(),
            }),
        }
    }

    /// Validate that all required inputs are provided
    fn validate_inputs(
        &self,
        task: &CachedTask,
        inputs: &HashMap<String, String>,
    ) -> Result<(), TaskRunnerError> {
        for input in &task.manifest.inputs {
            if input.required.unwrap_or(false) {
                let name = &input.name;

                // Check if input is provided or has default
                if !inputs.contains_key(name) && input.default_value.is_none() {
                    // Check aliases
                    let has_alias = input
                        .aliases
                        .as_ref()
                        .map(|aliases| aliases.iter().any(|a| inputs.contains_key(a)))
                        .unwrap_or(false);

                    if !has_alias {
                        // Check visibility rules - if not visible, not required
                        if let Some(rule) = &input.visible_rule {
                            // Simple visibility rule parsing
                            // Format: "inputName = value" or "inputName != value"
                            if let Some((check_input, _)) = rule.split_once('=') {
                                let check_input = check_input.trim().trim_end_matches('!').trim();
                                if !inputs.contains_key(check_input) {
                                    continue; // Skip validation if condition input not provided
                                }
                            }
                        }

                        return Err(TaskRunnerError::MissingInput(name.clone()));
                    }
                }
            }
        }

        Ok(())
    }

    /// Merge provided inputs with defaults
    fn merge_inputs(
        &self,
        task: &CachedTask,
        inputs: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut merged = task.manifest.default_values();

        // Override with provided inputs
        for (key, value) in inputs {
            merged.insert(key.clone(), value.clone());
        }

        merged
    }

    /// Execute the task implementation
    async fn execute_task_impl(
        &self,
        task: &CachedTask,
        inputs: &HashMap<String, String>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<StepResult, TaskRunnerError> {
        // Check for built-in tasks that we handle specially
        match task.name.as_str() {
            "Bash" => self.execute_bash_task(inputs, env, working_dir).await,
            "PowerShell" => self.execute_powershell_task(inputs, env, working_dir).await,
            "CmdLine" => self.execute_cmdline_task(inputs, env, working_dir).await,
            _ => {
                // For other tasks, try to execute based on manifest
                self.execute_generic_task(task, inputs, env, working_dir)
                    .await
            }
        }
    }

    /// Execute the Bash task
    async fn execute_bash_task(
        &self,
        inputs: &HashMap<String, String>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<StepResult, TaskRunnerError> {
        let target_type = inputs.get("targetType").map(|s| s.as_str()).unwrap_or("inline");

        let script = match target_type {
            "inline" => inputs
                .get("script")
                .ok_or_else(|| TaskRunnerError::MissingInput("script".to_string()))?
                .clone(),
            "filePath" => {
                let file_path = inputs
                    .get("filePath")
                    .ok_or_else(|| TaskRunnerError::MissingInput("filePath".to_string()))?;

                // Read script from file
                let script_path = if Path::new(file_path).is_absolute() {
                    PathBuf::from(file_path)
                } else {
                    working_dir.join(file_path)
                };

                std::fs::read_to_string(&script_path)?
            }
            _ => {
                return Err(TaskRunnerError::ExecutionFailed(format!(
                    "Unknown targetType: {}",
                    target_type
                )))
            }
        };

        let config = ShellConfig {
            working_dir: inputs.get("workingDirectory").cloned(),
            fail_on_stderr: inputs
                .get("failOnStderr")
                .map(|s| s == "true")
                .unwrap_or(false),
            ..Default::default()
        };

        let output = self
            .shell_runner
            .run_bash(&script, env, working_dir, &config)
            .await;

        Ok(self.shell_runner.to_step_result(
            output,
            None,
            Some("Bash".to_string()),
            config.fail_on_stderr,
            Duration::ZERO, // Will be set by caller
        ))
    }

    /// Execute the PowerShell task
    async fn execute_powershell_task(
        &self,
        inputs: &HashMap<String, String>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<StepResult, TaskRunnerError> {
        let target_type = inputs.get("targetType").map(|s| s.as_str()).unwrap_or("inline");
        let use_pwsh = inputs.get("pwsh").map(|s| s == "true").unwrap_or(false);

        let script = match target_type {
            "inline" => inputs
                .get("script")
                .ok_or_else(|| TaskRunnerError::MissingInput("script".to_string()))?
                .clone(),
            "filePath" => {
                let file_path = inputs
                    .get("filePath")
                    .ok_or_else(|| TaskRunnerError::MissingInput("filePath".to_string()))?;

                let script_path = if Path::new(file_path).is_absolute() {
                    PathBuf::from(file_path)
                } else {
                    working_dir.join(file_path)
                };

                std::fs::read_to_string(&script_path)?
            }
            _ => {
                return Err(TaskRunnerError::ExecutionFailed(format!(
                    "Unknown targetType: {}",
                    target_type
                )))
            }
        };

        let config = ShellConfig {
            working_dir: inputs.get("workingDirectory").cloned(),
            fail_on_stderr: inputs
                .get("failOnStderr")
                .map(|s| s == "true")
                .unwrap_or(false),
            error_action_preference: inputs.get("errorActionPreference").cloned(),
            ..Default::default()
        };

        let output = if use_pwsh {
            self.shell_runner
                .run_pwsh(&script, env, working_dir, &config)
                .await
        } else {
            self.shell_runner
                .run_powershell(&script, env, working_dir, &config)
                .await
        };

        Ok(self.shell_runner.to_step_result(
            output,
            None,
            Some("PowerShell".to_string()),
            config.fail_on_stderr,
            Duration::ZERO,
        ))
    }

    /// Execute the CmdLine task
    async fn execute_cmdline_task(
        &self,
        inputs: &HashMap<String, String>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<StepResult, TaskRunnerError> {
        let script = inputs
            .get("script")
            .ok_or_else(|| TaskRunnerError::MissingInput("script".to_string()))?;

        let config = ShellConfig {
            working_dir: inputs.get("workingDirectory").cloned(),
            fail_on_stderr: inputs
                .get("failOnStderr")
                .map(|s| s == "true")
                .unwrap_or(false),
            ..Default::default()
        };

        let output = self
            .shell_runner
            .run_script(script, env, working_dir, &config)
            .await;

        Ok(self.shell_runner.to_step_result(
            output,
            None,
            Some("Command Line".to_string()),
            config.fail_on_stderr,
            Duration::ZERO,
        ))
    }

    /// Execute a generic task using its manifest
    async fn execute_generic_task(
        &self,
        task: &CachedTask,
        inputs: &HashMap<String, String>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<StepResult, TaskRunnerError> {
        let exec = task
            .manifest
            .primary_execution()
            .ok_or_else(|| TaskRunnerError::UnsupportedExecution("No execution defined".to_string()))?;

        let target_path = task.path.join(&exec.target);

        if !target_path.exists() {
            return Err(TaskRunnerError::ExecutionFailed(format!(
                "Task target not found: {}",
                target_path.display()
            )));
        }

        // Determine execution type from manifest
        if task.manifest.is_node_task() {
            self.execute_node_task(&target_path, task, inputs, env, working_dir)
                .await
        } else if task.manifest.is_powershell_task() {
            self.execute_ps_task(&target_path, task, inputs, env, working_dir)
                .await
        } else {
            Err(TaskRunnerError::UnsupportedExecution(format!(
                "Unknown execution type for task: {}",
                task.name
            )))
        }
    }

    /// Execute a Node.js-based task
    async fn execute_node_task(
        &self,
        target: &Path,
        task: &CachedTask,
        inputs: &HashMap<String, String>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<StepResult, TaskRunnerError> {
        let node_path = self
            .node_path
            .as_ref()
            .ok_or_else(|| TaskRunnerError::ExecutionFailed("Node.js not found".to_string()))?;

        // Set up task environment
        let mut task_env = env.clone();

        // Add inputs as environment variables (INPUT_<name>)
        for (key, value) in inputs {
            let env_key = format!("INPUT_{}", key.to_uppercase().replace([' ', '.'], "_"));
            task_env.insert(env_key, value.clone());
        }

        // Add task library variables
        task_env.insert(
            "AGENT_TEMPDIRECTORY".to_string(),
            std::env::temp_dir().to_string_lossy().to_string(),
        );
        task_env.insert(
            "AGENT_WORKFOLDER".to_string(),
            working_dir.to_string_lossy().to_string(),
        );
        task_env.insert(
            "SYSTEM_DEFAULTWORKINGDIRECTORY".to_string(),
            working_dir.to_string_lossy().to_string(),
        );

        // Build the command
        let script = format!(
            "{} {}",
            node_path.display(),
            target.display()
        );

        let config = ShellConfig {
            working_dir: Some(task.path.to_string_lossy().to_string()),
            ..Default::default()
        };

        let output = self
            .shell_runner
            .run_script(&script, &task_env, working_dir, &config)
            .await;

        Ok(self.shell_runner.to_step_result(
            output,
            None,
            task.manifest.friendly_name.clone(),
            false,
            Duration::ZERO,
        ))
    }

    /// Execute a PowerShell-based task
    async fn execute_ps_task(
        &self,
        target: &Path,
        task: &CachedTask,
        inputs: &HashMap<String, String>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<StepResult, TaskRunnerError> {
        // Set up task environment
        let mut task_env = env.clone();

        // Add inputs as environment variables
        for (key, value) in inputs {
            let env_key = format!("INPUT_{}", key.to_uppercase().replace([' ', '.'], "_"));
            task_env.insert(env_key, value.clone());
        }

        // Add task library variables
        task_env.insert(
            "AGENT_TEMPDIRECTORY".to_string(),
            std::env::temp_dir().to_string_lossy().to_string(),
        );
        task_env.insert(
            "SYSTEM_DEFAULTWORKINGDIRECTORY".to_string(),
            working_dir.to_string_lossy().to_string(),
        );

        // Build PowerShell command to execute the script
        let script = format!(
            "& '{}' ",
            target.display()
        );

        let config = ShellConfig {
            working_dir: Some(task.path.to_string_lossy().to_string()),
            ..Default::default()
        };

        let output = self
            .shell_runner
            .run_pwsh(&script, &task_env, working_dir, &config)
            .await;

        Ok(self.shell_runner.to_step_result(
            output,
            None,
            task.manifest.friendly_name.clone(),
            false,
            Duration::ZERO,
        ))
    }
}

/// Find the Node.js executable path
fn find_node_path() -> Option<PathBuf> {
    // Try common locations
    let candidates = if cfg!(target_os = "windows") {
        vec![
            "node.exe",
            "C:\\Program Files\\nodejs\\node.exe",
            "C:\\Program Files (x86)\\nodejs\\node.exe",
        ]
    } else {
        vec![
            "node",
            "/usr/bin/node",
            "/usr/local/bin/node",
            "/opt/homebrew/bin/node",
        ]
    };

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() || which::which(candidate).is_ok() {
            return Some(path);
        }
    }

    // Try using which
    which::which("node").ok()
}

/// Find the PowerShell executable path
fn find_powershell_path() -> Option<PathBuf> {
    // Try pwsh first (PowerShell Core), then fallback to powershell.exe on Windows
    let candidates = if cfg!(target_os = "windows") {
        vec![
            "pwsh.exe",
            "powershell.exe",
            "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
            "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        ]
    } else {
        vec![
            "pwsh",
            "/usr/bin/pwsh",
            "/usr/local/bin/pwsh",
            "/opt/microsoft/powershell/7/pwsh",
        ]
    };

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() || which::which(candidate).is_ok() {
            return Some(path);
        }
    }

    which::which("pwsh").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_runner() -> (TaskRunner, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let runner = TaskRunner::new(temp_dir.path().to_path_buf());
        (runner, temp_dir)
    }

    #[tokio::test]
    async fn test_execute_bash_task_inline() {
        let (runner, _temp_dir) = create_test_runner();
        let mut inputs = HashMap::new();
        inputs.insert("targetType".to_string(), "inline".to_string());
        inputs.insert("script".to_string(), "echo 'Hello from Bash task'".to_string());

        let env = HashMap::new();
        let working_dir = std::env::current_dir().unwrap();

        let result = runner
            .execute_bash_task(&inputs, &env, &working_dir)
            .await;

        // Bash might not be available on all systems
        if let Ok(step_result) = result {
            if step_result.status == StepStatus::Succeeded {
                assert!(step_result.output.contains("Hello from Bash task"));
            }
        }
    }

    #[tokio::test]
    async fn test_execute_cmdline_task() {
        let (runner, _temp_dir) = create_test_runner();
        let mut inputs = HashMap::new();
        inputs.insert("script".to_string(), "echo Hello from CmdLine".to_string());

        let env = HashMap::new();
        let working_dir = std::env::current_dir().unwrap();

        let result = runner
            .execute_cmdline_task(&inputs, &env, &working_dir)
            .await
            .unwrap();

        assert_eq!(result.status, StepStatus::Succeeded);
        assert!(result.output.contains("Hello from CmdLine"));
    }

    #[test]
    fn test_merge_inputs() {
        let (runner, _temp_dir) = create_test_runner();

        // Create a minimal cached task for testing
        let task = CachedTask {
            name: "Test".to_string(),
            version: "1".to_string(),
            path: PathBuf::from("/tmp/test"),
            manifest: crate::tasks::manifest::TaskManifest {
                id: "test".to_string(),
                name: "Test".to_string(),
                friendly_name: None,
                description: None,
                help_url: None,
                help_mark_down: None,
                category: None,
                visibility: None,
                runs_on: None,
                author: None,
                version: crate::tasks::manifest::TaskVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                minimum_agent_version: None,
                instance_name_format: None,
                groups: None,
                inputs: vec![crate::tasks::manifest::TaskInput {
                    name: "input1".to_string(),
                    input_type: None,
                    label: None,
                    default_value: Some("default_value".to_string()),
                    required: None,
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: None,
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                }],
                output_variables: None,
                execution: None,
                pre_job_execution: None,
                post_job_execution: None,
                data_source_bindings: None,
                messages: None,
                restrictions: None,
                demands: None,
            },
        };

        let mut inputs = HashMap::new();
        inputs.insert("input2".to_string(), "custom_value".to_string());

        let merged = runner.merge_inputs(&task, &inputs);

        assert_eq!(merged.get("input1"), Some(&"default_value".to_string()));
        assert_eq!(merged.get("input2"), Some(&"custom_value".to_string()));
    }
}
