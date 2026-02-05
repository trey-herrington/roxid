// Shell Runner
// Executes script, bash, pwsh, and powershell steps

use crate::parser::models::{StepResult, StepStatus, Value};

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Shell types supported by the runner
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    /// Default shell (sh on Unix, cmd on Windows)
    Default,
    /// Bash shell
    Bash,
    /// PowerShell Core (cross-platform)
    Pwsh,
    /// Windows PowerShell (Windows only, falls back to pwsh)
    PowerShell,
}

impl Shell {
    /// Get the shell executable and arguments
    fn get_command(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Shell::Default => {
                if cfg!(target_os = "windows") {
                    ("cmd", &["/C"])
                } else {
                    ("sh", &["-c"])
                }
            }
            Shell::Bash => ("bash", &["-c"]),
            Shell::Pwsh => ("pwsh", &["-NoLogo", "-NoProfile", "-Command"]),
            Shell::PowerShell => {
                if cfg!(target_os = "windows") {
                    ("powershell.exe", &["-NoLogo", "-NoProfile", "-Command"])
                } else {
                    // Fall back to pwsh on non-Windows
                    ("pwsh", &["-NoLogo", "-NoProfile", "-Command"])
                }
            }
        }
    }
}

/// Configuration for shell execution
#[derive(Debug, Clone)]
pub struct ShellConfig {
    /// Working directory for the script
    pub working_dir: Option<String>,
    /// Fail if there's output to stderr
    pub fail_on_stderr: bool,
    /// Error action preference (for PowerShell)
    pub error_action_preference: Option<String>,
    /// Timeout in seconds (None = no timeout)
    pub timeout: Option<Duration>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            working_dir: None,
            fail_on_stderr: false,
            error_action_preference: None,
            timeout: None,
        }
    }
}

/// Output collected during script execution
#[derive(Debug, Clone, Default)]
pub struct ShellOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code (if available)
    pub exit_code: Option<i32>,
    /// Outputs extracted from logging commands
    pub outputs: HashMap<String, String>,
    /// Variables set via logging commands
    pub variables: HashMap<String, Value>,
}

/// Callback for handling output lines in real-time
pub type OutputCallback = Box<dyn Fn(&str, bool) + Send + Sync>;

/// Shell runner for executing scripts
pub struct ShellRunner {
    /// Default shell to use
    default_shell: Shell,
}

impl ShellRunner {
    /// Create a new shell runner with the default shell
    pub fn new() -> Self {
        Self {
            default_shell: Shell::Default,
        }
    }

    /// Create a shell runner with a specific default shell
    pub fn with_default_shell(shell: Shell) -> Self {
        Self {
            default_shell: shell,
        }
    }

    /// Execute a script using the default shell
    pub async fn run_script(
        &self,
        script: &str,
        env: &HashMap<String, String>,
        working_dir: &Path,
        config: &ShellConfig,
    ) -> ShellOutput {
        self.run_with_shell(self.default_shell, script, env, working_dir, config)
            .await
    }

    /// Execute a bash script
    pub async fn run_bash(
        &self,
        script: &str,
        env: &HashMap<String, String>,
        working_dir: &Path,
        config: &ShellConfig,
    ) -> ShellOutput {
        self.run_with_shell(Shell::Bash, script, env, working_dir, config)
            .await
    }

    /// Execute a PowerShell Core (pwsh) script
    pub async fn run_pwsh(
        &self,
        script: &str,
        env: &HashMap<String, String>,
        working_dir: &Path,
        config: &ShellConfig,
    ) -> ShellOutput {
        // Wrap script with error action preference if specified
        let script = if let Some(pref) = &config.error_action_preference {
            format!("$ErrorActionPreference = '{}'\n{}", pref, script)
        } else {
            script.to_string()
        };

        self.run_with_shell(Shell::Pwsh, &script, env, working_dir, config)
            .await
    }

    /// Execute a Windows PowerShell script
    pub async fn run_powershell(
        &self,
        script: &str,
        env: &HashMap<String, String>,
        working_dir: &Path,
        config: &ShellConfig,
    ) -> ShellOutput {
        // Wrap script with error action preference if specified
        let script = if let Some(pref) = &config.error_action_preference {
            format!("$ErrorActionPreference = '{}'\n{}", pref, script)
        } else {
            script.to_string()
        };

        self.run_with_shell(Shell::PowerShell, &script, env, working_dir, config)
            .await
    }

    /// Execute a script with a specific shell
    async fn run_with_shell(
        &self,
        shell: Shell,
        script: &str,
        env: &HashMap<String, String>,
        working_dir: &Path,
        config: &ShellConfig,
    ) -> ShellOutput {
        let (shell_cmd, shell_args) = shell.get_command();

        // Determine working directory
        let work_dir = config
            .working_dir
            .as_ref()
            .map(Path::new)
            .unwrap_or(working_dir);

        let mut cmd = Command::new(shell_cmd);
        cmd.args(shell_args);
        cmd.arg(script);
        cmd.current_dir(work_dir);
        cmd.envs(env);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn the process
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return ShellOutput {
                    stdout: String::new(),
                    stderr: format!("Failed to spawn shell process '{}': {}", shell_cmd, e),
                    exit_code: None,
                    outputs: HashMap::new(),
                    variables: HashMap::new(),
                };
            }
        };

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        // Read output streams concurrently
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let stdout_handle = tokio::spawn(async move {
            let mut lines = stdout_reader.lines();
            let mut output = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(&line);
            }
            output
        });

        let stderr_handle = tokio::spawn(async move {
            let mut lines = stderr_reader.lines();
            let mut output = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(&line);
            }
            output
        });

        // Wait for completion with optional timeout
        let wait_result = if let Some(timeout) = config.timeout {
            match tokio::time::timeout(timeout, child.wait()).await {
                Ok(result) => result,
                Err(_) => {
                    // Timeout - kill the process
                    let _ = child.kill().await;
                    return ShellOutput {
                        stdout: stdout_handle.await.unwrap_or_default(),
                        stderr: format!("Process timed out after {:?}", timeout),
                        exit_code: None,
                        outputs: HashMap::new(),
                        variables: HashMap::new(),
                    };
                }
            }
        } else {
            child.wait().await
        };

        let exit_code = wait_result.ok().and_then(|s| s.code());
        let stdout = stdout_handle.await.unwrap_or_default();
        let stderr = stderr_handle.await.unwrap_or_default();

        // Parse logging commands from stdout
        let (outputs, variables) = parse_logging_commands(&stdout);

        ShellOutput {
            stdout,
            stderr,
            exit_code,
            outputs,
            variables,
        }
    }

    /// Execute a script with real-time output streaming
    pub async fn run_script_streaming(
        &self,
        script: &str,
        env: &HashMap<String, String>,
        working_dir: &Path,
        config: &ShellConfig,
        on_output: OutputCallback,
    ) -> ShellOutput {
        self.run_with_shell_streaming(
            self.default_shell,
            script,
            env,
            working_dir,
            config,
            on_output,
        )
        .await
    }

    /// Execute a script with real-time output streaming using a specific shell
    async fn run_with_shell_streaming(
        &self,
        shell: Shell,
        script: &str,
        env: &HashMap<String, String>,
        working_dir: &Path,
        config: &ShellConfig,
        on_output: OutputCallback,
    ) -> ShellOutput {
        let (shell_cmd, shell_args) = shell.get_command();

        let work_dir = config
            .working_dir
            .as_ref()
            .map(Path::new)
            .unwrap_or(working_dir);

        let mut cmd = Command::new(shell_cmd);
        cmd.args(shell_args);
        cmd.arg(script);
        cmd.current_dir(work_dir);
        cmd.envs(env);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return ShellOutput {
                    stdout: String::new(),
                    stderr: format!("Failed to spawn shell process '{}': {}", shell_cmd, e),
                    exit_code: None,
                    outputs: HashMap::new(),
                    variables: HashMap::new(),
                };
            }
        };

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let on_output = std::sync::Arc::new(on_output);
        let on_output_stdout = on_output.clone();
        let on_output_stderr = on_output;

        // Stream stdout
        let stdout_handle = tokio::spawn(async move {
            let mut lines = stdout_reader.lines();
            let mut output = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                on_output_stdout(&line, false);
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(&line);
            }
            output
        });

        // Stream stderr
        let stderr_handle = tokio::spawn(async move {
            let mut lines = stderr_reader.lines();
            let mut output = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                on_output_stderr(&line, true);
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(&line);
            }
            output
        });

        let wait_result = if let Some(timeout) = config.timeout {
            match tokio::time::timeout(timeout, child.wait()).await {
                Ok(result) => result,
                Err(_) => {
                    let _ = child.kill().await;
                    return ShellOutput {
                        stdout: stdout_handle.await.unwrap_or_default(),
                        stderr: format!("Process timed out after {:?}", timeout),
                        exit_code: None,
                        outputs: HashMap::new(),
                        variables: HashMap::new(),
                    };
                }
            }
        } else {
            child.wait().await
        };

        let exit_code = wait_result.ok().and_then(|s| s.code());
        let stdout = stdout_handle.await.unwrap_or_default();
        let stderr = stderr_handle.await.unwrap_or_default();

        let (outputs, variables) = parse_logging_commands(&stdout);

        ShellOutput {
            stdout,
            stderr,
            exit_code,
            outputs,
            variables,
        }
    }

    /// Convert shell output to a step result
    pub fn to_step_result(
        &self,
        output: ShellOutput,
        step_name: Option<String>,
        display_name: Option<String>,
        fail_on_stderr: bool,
        duration: Duration,
    ) -> StepResult {
        let status = if output.exit_code.map(|c| c != 0).unwrap_or(true) {
            StepStatus::Failed
        } else if fail_on_stderr && !output.stderr.is_empty() {
            StepStatus::Failed
        } else {
            StepStatus::Succeeded
        };

        StepResult {
            step_name,
            display_name,
            status,
            output: output.stdout,
            error: if output.stderr.is_empty() {
                None
            } else {
                Some(output.stderr)
            },
            duration,
            exit_code: output.exit_code,
            outputs: output.outputs,
        }
    }
}

impl Default for ShellRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse Azure DevOps logging commands from output
fn parse_logging_commands(output: &str) -> (HashMap<String, String>, HashMap<String, Value>) {
    let mut outputs = HashMap::new();
    let mut variables = HashMap::new();

    for line in output.lines() {
        // ##vso[task.setvariable variable=name;isoutput=true;issecret=false]value
        if let Some(rest) = line.strip_prefix("##vso[task.setvariable") {
            if let Some((props, value)) = rest.split_once(']') {
                let mut var_name = None;
                let mut is_output = false;
                let mut is_secret = false;

                for prop in props.split(';') {
                    let prop = prop.trim();
                    if let Some(name) = prop.strip_prefix("variable=") {
                        var_name = Some(name.to_string());
                    } else if prop == "isoutput=true" || prop == "isOutput=true" {
                        is_output = true;
                    } else if prop == "issecret=true" || prop == "isSecret=true" {
                        is_secret = true;
                    }
                }

                if let Some(name) = var_name {
                    if is_output {
                        outputs.insert(name.clone(), value.to_string());
                    }
                    if !is_secret {
                        variables.insert(name, Value::String(value.to_string()));
                    }
                }
            }
        }
        // ##vso[task.setVariable variable=name]value (alternate format)
        else if let Some(rest) = line.strip_prefix("##vso[task.setVariable") {
            if let Some((props, value)) = rest.split_once(']') {
                let mut var_name = None;
                let mut is_output = false;
                let mut is_secret = false;

                for prop in props.split(';') {
                    let prop = prop.trim();
                    if let Some(name) = prop.strip_prefix("variable=") {
                        var_name = Some(name.to_string());
                    } else if prop == "isoutput=true" || prop == "isOutput=true" {
                        is_output = true;
                    } else if prop == "issecret=true" || prop == "isSecret=true" {
                        is_secret = true;
                    }
                }

                if let Some(name) = var_name {
                    if is_output {
                        outputs.insert(name.clone(), value.to_string());
                    }
                    if !is_secret {
                        variables.insert(name, Value::String(value.to_string()));
                    }
                }
            }
        }
        // ##vso[task.prependpath]path
        else if let Some(rest) = line.strip_prefix("##vso[task.prependpath]") {
            // Store prepend path requests
            let existing = variables
                .entry("_PREPEND_PATH".to_string())
                .or_insert_with(|| Value::Array(vec![]));
            if let Value::Array(arr) = existing {
                arr.push(Value::String(rest.to_string()));
            }
        }
        // ##vso[task.uploadfile]path
        else if let Some(rest) = line.strip_prefix("##vso[task.uploadfile]") {
            let existing = variables
                .entry("_UPLOAD_FILES".to_string())
                .or_insert_with(|| Value::Array(vec![]));
            if let Value::Array(arr) = existing {
                arr.push(Value::String(rest.to_string()));
            }
        }
        // ##vso[artifact.upload containerfolder=folder;artifactname=name]path
        // Skip for now - artifact handling
        // ##vso[build.addbuildtag]tag
        else if let Some(rest) = line.strip_prefix("##vso[build.addbuildtag]") {
            let existing = variables
                .entry("_BUILD_TAGS".to_string())
                .or_insert_with(|| Value::Array(vec![]));
            if let Value::Array(arr) = existing {
                arr.push(Value::String(rest.to_string()));
            }
        }
        // ##vso[task.complete result=Succeeded;]message
        else if let Some(rest) = line.strip_prefix("##vso[task.complete") {
            if let Some((props, _message)) = rest.split_once(']') {
                for prop in props.split(';') {
                    let prop = prop.trim();
                    if let Some(result) = prop.strip_prefix("result=") {
                        variables.insert("_TASK_RESULT".to_string(), Value::String(result.to_string()));
                    }
                }
            }
        }
    }

    (outputs, variables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_runner_echo() {
        let runner = ShellRunner::new();
        let env = HashMap::new();
        let working_dir = std::env::current_dir().unwrap();
        let config = ShellConfig::default();

        let output = runner
            .run_script("echo hello", &env, &working_dir, &config)
            .await;

        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout.contains("hello"));
        assert!(output.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_shell_runner_with_env() {
        let runner = ShellRunner::new();
        let mut env = HashMap::new();
        env.insert("MY_VAR".to_string(), "test_value".to_string());
        let working_dir = std::env::current_dir().unwrap();
        let config = ShellConfig::default();

        let script = if cfg!(target_os = "windows") {
            "echo %MY_VAR%"
        } else {
            "echo $MY_VAR"
        };

        let output = runner.run_script(script, &env, &working_dir, &config).await;

        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout.contains("test_value"));
    }

    #[tokio::test]
    async fn test_shell_runner_bash() {
        let runner = ShellRunner::new();
        let env = HashMap::new();
        let working_dir = std::env::current_dir().unwrap();
        let config = ShellConfig::default();

        let output = runner
            .run_bash("echo 'bash test'", &env, &working_dir, &config)
            .await;

        // This test might fail if bash is not installed
        if output.exit_code == Some(0) {
            assert!(output.stdout.contains("bash test"));
        }
    }

    #[tokio::test]
    async fn test_shell_runner_exit_code() {
        let runner = ShellRunner::new();
        let env = HashMap::new();
        let working_dir = std::env::current_dir().unwrap();
        let config = ShellConfig::default();

        let output = runner
            .run_script("exit 42", &env, &working_dir, &config)
            .await;

        assert_eq!(output.exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_shell_runner_stderr() {
        let runner = ShellRunner::new();
        let env = HashMap::new();
        let working_dir = std::env::current_dir().unwrap();
        let config = ShellConfig::default();

        let output = runner
            .run_script("echo error >&2", &env, &working_dir, &config)
            .await;

        assert_eq!(output.exit_code, Some(0));
        assert!(output.stderr.contains("error"));
    }

    #[test]
    fn test_parse_logging_commands_setvariable() {
        let output = r#"
Starting build
##vso[task.setvariable variable=version]1.0.0
##vso[task.setvariable variable=output;isoutput=true]result_value
Build complete
"#;

        let (outputs, variables) = parse_logging_commands(output);

        assert_eq!(
            variables.get("version"),
            Some(&Value::String("1.0.0".to_string()))
        );
        assert_eq!(outputs.get("output"), Some(&"result_value".to_string()));
        assert_eq!(
            variables.get("output"),
            Some(&Value::String("result_value".to_string()))
        );
    }

    #[test]
    fn test_parse_logging_commands_secret() {
        let output = "##vso[task.setvariable variable=password;issecret=true]secretvalue";

        let (outputs, variables) = parse_logging_commands(output);

        // Secrets should not be stored in variables
        assert!(!variables.contains_key("password"));
        assert!(!outputs.contains_key("password"));
    }

    #[test]
    fn test_parse_logging_commands_build_tag() {
        let output = r#"
##vso[build.addbuildtag]release
##vso[build.addbuildtag]v1.0
"#;

        let (_outputs, variables) = parse_logging_commands(output);

        let tags = variables.get("_BUILD_TAGS").unwrap();
        if let Value::Array(arr) = tags {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], Value::String("release".to_string()));
            assert_eq!(arr[1], Value::String("v1.0".to_string()));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_to_step_result_success() {
        let runner = ShellRunner::new();
        let output = ShellOutput {
            stdout: "Success".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            outputs: HashMap::new(),
            variables: HashMap::new(),
        };

        let result = runner.to_step_result(
            output,
            Some("test_step".to_string()),
            Some("Test Step".to_string()),
            false,
            Duration::from_secs(1),
        );

        assert_eq!(result.status, StepStatus::Succeeded);
        assert_eq!(result.output, "Success");
        assert!(result.error.is_none());
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn test_to_step_result_fail_on_stderr() {
        let runner = ShellRunner::new();
        let output = ShellOutput {
            stdout: "Output".to_string(),
            stderr: "Warning message".to_string(),
            exit_code: Some(0),
            outputs: HashMap::new(),
            variables: HashMap::new(),
        };

        let result = runner.to_step_result(output, None, None, true, Duration::from_secs(1));

        assert_eq!(result.status, StepStatus::Failed);
        assert_eq!(result.error, Some("Warning message".to_string()));
    }
}
