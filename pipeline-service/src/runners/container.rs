// Container Runner
// Executes jobs inside Docker containers

use crate::parser::models::{ContainerRef, ContainerSpec, Job, JobResult, JobStatus, Step, StepResult, StepStatus};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors that can occur with container execution
#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("Docker is not available: {0}")]
    DockerNotAvailable(String),

    #[error("Failed to pull image: {0}")]
    PullFailed(String),

    #[error("Failed to create container: {0}")]
    CreateFailed(String),

    #[error("Failed to start container: {0}")]
    StartFailed(String),

    #[error("Container execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Failed to stop container: {0}")]
    StopFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Configuration for container execution
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Docker socket path (default: /var/run/docker.sock on Unix)
    pub docker_socket: Option<PathBuf>,
    /// Whether to pull images before running
    pub pull_policy: ImagePullPolicy,
    /// Default timeout for container operations
    pub timeout: Duration,
    /// Whether to remove containers after execution
    pub auto_remove: bool,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            docker_socket: None,
            pull_policy: ImagePullPolicy::IfNotPresent,
            timeout: Duration::from_secs(3600),
            auto_remove: true,
        }
    }
}

/// Image pull policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePullPolicy {
    /// Always pull the image
    Always,
    /// Pull only if not present locally
    IfNotPresent,
    /// Never pull (must be present locally)
    Never,
}

/// Handle to a running container
#[derive(Debug)]
pub struct ContainerHandle {
    /// Container ID
    pub id: String,
    /// Container name
    pub name: String,
    /// Image used
    pub image: String,
}

/// Handle to service containers
#[derive(Debug)]
pub struct ServiceHandles {
    /// Service name to container handle mapping
    pub services: HashMap<String, ContainerHandle>,
}

/// Container runner for Docker-based execution
pub struct ContainerRunner {
    config: ContainerConfig,
}

impl ContainerRunner {
    /// Create a new container runner with default configuration
    pub fn new() -> Self {
        Self {
            config: ContainerConfig::default(),
        }
    }

    /// Create a container runner with custom configuration
    pub fn with_config(config: ContainerConfig) -> Self {
        Self { config }
    }

    /// Check if Docker is available
    pub async fn is_available(&self) -> bool {
        // Try to run `docker version`
        let output = tokio::process::Command::new("docker")
            .arg("version")
            .arg("--format")
            .arg("{{.Server.Version}}")
            .output()
            .await;

        output.map(|o| o.status.success()).unwrap_or(false)
    }

    /// Run a job inside a container
    pub async fn run_job_in_container(
        &self,
        job: &Job,
        container: &ContainerRef,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<JobResult, ContainerError> {
        let start = Instant::now();
        let job_name = job.identifier().unwrap_or("job").to_string();

        // Parse container spec
        let container_spec = self.parse_container_ref(container)?;

        // Pull image if needed
        self.pull_image_if_needed(&container_spec.image).await?;

        // Create and start the container
        let container_handle = self
            .create_container(&job_name, &container_spec, env, working_dir)
            .await?;

        // Execute steps inside the container
        let mut step_results = Vec::new();
        let mut job_status = JobStatus::Succeeded;

        for step in &job.steps {
            let result = self
                .run_step_in_container(&container_handle, step, env, working_dir)
                .await;

            if result.status == StepStatus::Failed {
                job_status = JobStatus::Failed;
            } else if result.status == StepStatus::SucceededWithIssues
                && job_status == JobStatus::Succeeded
            {
                job_status = JobStatus::SucceededWithIssues;
            }

            step_results.push(result);

            if job_status == JobStatus::Failed && !job.continue_on_error {
                break;
            }
        }

        // Clean up container
        self.stop_container(&container_handle).await?;

        Ok(JobResult {
            job_name,
            display_name: job.display_name.clone(),
            status: job_status,
            steps: step_results,
            duration: start.elapsed(),
            outputs: HashMap::new(),
        })
    }

    /// Start service containers for a job
    pub async fn start_service_containers(
        &self,
        services: &HashMap<String, ContainerRef>,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<ServiceHandles, ContainerError> {
        let mut handles = HashMap::new();

        for (service_name, container_ref) in services {
            let container_spec = self.parse_container_ref(container_ref)?;

            // Pull image if needed
            self.pull_image_if_needed(&container_spec.image).await?;

            // Create service container
            let handle = self
                .create_service_container(service_name, &container_spec, env, working_dir)
                .await?;

            handles.insert(service_name.clone(), handle);
        }

        Ok(ServiceHandles { services: handles })
    }

    /// Stop service containers
    pub async fn stop_service_containers(&self, handles: ServiceHandles) -> Result<(), ContainerError> {
        for (_, handle) in handles.services {
            self.stop_container(&handle).await?;
        }
        Ok(())
    }

    /// Parse a container reference into a spec
    fn parse_container_ref(&self, container: &ContainerRef) -> Result<ContainerSpec, ContainerError> {
        match container {
            ContainerRef::Image(image) => Ok(ContainerSpec {
                image: image.clone(),
                endpoint: None,
                env: HashMap::new(),
                ports: Vec::new(),
                volumes: Vec::new(),
                options: None,
                map_docker_socket: None,
                mount_read_only: None,
            }),
            ContainerRef::Spec(spec) => Ok(spec.clone()),
        }
    }

    /// Pull an image if needed based on pull policy
    async fn pull_image_if_needed(&self, image: &str) -> Result<(), ContainerError> {
        match self.config.pull_policy {
            ImagePullPolicy::Never => Ok(()),
            ImagePullPolicy::Always => self.pull_image(image).await,
            ImagePullPolicy::IfNotPresent => {
                // Check if image exists locally
                let output = tokio::process::Command::new("docker")
                    .args(["image", "inspect", image])
                    .output()
                    .await
                    .map_err(|e| ContainerError::DockerNotAvailable(e.to_string()))?;

                if !output.status.success() {
                    self.pull_image(image).await
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Pull a Docker image
    async fn pull_image(&self, image: &str) -> Result<(), ContainerError> {
        let output = tokio::process::Command::new("docker")
            .args(["pull", image])
            .output()
            .await
            .map_err(|e| ContainerError::DockerNotAvailable(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::PullFailed(format!(
                "Failed to pull {}: {}",
                image, stderr
            )));
        }

        Ok(())
    }

    /// Create a container for job execution
    async fn create_container(
        &self,
        name: &str,
        spec: &ContainerSpec,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> Result<ContainerHandle, ContainerError> {
        let container_name = format!("roxid-{}-{}", name, uuid_v4_simple());

        let mut args = vec![
            "create".to_string(),
            "--name".to_string(),
            container_name.clone(),
            "-w".to_string(),
            "/workspace".to_string(),
            "-v".to_string(),
            format!("{}:/workspace", working_dir.display()),
        ];

        // Add environment variables
        for (key, value) in env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Add container-specific env
        for (key, value) in &spec.env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Add volumes
        for volume in &spec.volumes {
            args.push("-v".to_string());
            args.push(volume.clone());
        }

        // Add ports
        for port in &spec.ports {
            args.push("-p".to_string());
            args.push(port.clone());
        }

        // Add Docker socket if requested
        if spec.map_docker_socket.unwrap_or(false) {
            args.push("-v".to_string());
            args.push("/var/run/docker.sock:/var/run/docker.sock".to_string());
        }

        // Add any additional options
        if let Some(options) = &spec.options {
            // Parse options string (space-separated Docker flags)
            for opt in options.split_whitespace() {
                args.push(opt.to_string());
            }
        }

        // Add the image
        args.push(spec.image.clone());

        // Keep container running with tail
        args.push("tail".to_string());
        args.push("-f".to_string());
        args.push("/dev/null".to_string());

        let output = tokio::process::Command::new("docker")
            .args(&args)
            .output()
            .await
            .map_err(|e| ContainerError::DockerNotAvailable(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::CreateFailed(stderr.to_string()));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Start the container
        let start_output = tokio::process::Command::new("docker")
            .args(["start", &container_name])
            .output()
            .await
            .map_err(|e| ContainerError::DockerNotAvailable(e.to_string()))?;

        if !start_output.status.success() {
            let stderr = String::from_utf8_lossy(&start_output.stderr);
            return Err(ContainerError::StartFailed(stderr.to_string()));
        }

        Ok(ContainerHandle {
            id: container_id,
            name: container_name,
            image: spec.image.clone(),
        })
    }

    /// Create a service container
    async fn create_service_container(
        &self,
        service_name: &str,
        spec: &ContainerSpec,
        env: &HashMap<String, String>,
        _working_dir: &Path,
    ) -> Result<ContainerHandle, ContainerError> {
        let container_name = format!("roxid-svc-{}-{}", service_name, uuid_v4_simple());

        let mut args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            container_name.clone(),
        ];

        // Add environment variables
        for (key, value) in env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        for (key, value) in &spec.env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Add ports
        for port in &spec.ports {
            args.push("-p".to_string());
            args.push(port.clone());
        }

        // Add volumes
        for volume in &spec.volumes {
            args.push("-v".to_string());
            args.push(volume.clone());
        }

        // Add the image
        args.push(spec.image.clone());

        let output = tokio::process::Command::new("docker")
            .args(&args)
            .output()
            .await
            .map_err(|e| ContainerError::DockerNotAvailable(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::CreateFailed(stderr.to_string()));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(ContainerHandle {
            id: container_id,
            name: container_name,
            image: spec.image.clone(),
        })
    }

    /// Run a step inside a container
    async fn run_step_in_container(
        &self,
        container: &ContainerHandle,
        step: &Step,
        _env: &HashMap<String, String>,
        _working_dir: &Path,
    ) -> StepResult {
        let start = Instant::now();
        let step_name = step.name.clone();

        // For now, we only support script steps in containers
        let script = match &step.action {
            crate::parser::models::StepAction::Script(s) => &s.script,
            crate::parser::models::StepAction::Bash(s) => &s.bash,
            crate::parser::models::StepAction::Pwsh(s) => &s.pwsh,
            crate::parser::models::StepAction::PowerShell(s) => &s.powershell,
            _ => {
                return StepResult {
                    step_name,
                    display_name: step.display_name.clone(),
                    status: StepStatus::Skipped,
                    output: "Step type not supported in container".to_string(),
                    error: None,
                    duration: start.elapsed(),
                    exit_code: None,
                    outputs: HashMap::new(),
                };
            }
        };

        // Execute the script in the container
        let output = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-w",
                "/workspace",
                &container.name,
                "sh",
                "-c",
                script,
            ])
            .output()
            .await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let status = if output.status.success() {
                    StepStatus::Succeeded
                } else {
                    StepStatus::Failed
                };

                StepResult {
                    step_name,
                    display_name: step.display_name.clone(),
                    status,
                    output: stdout,
                    error: if stderr.is_empty() { None } else { Some(stderr) },
                    duration: start.elapsed(),
                    exit_code: output.status.code(),
                    outputs: HashMap::new(),
                }
            }
            Err(e) => StepResult {
                step_name,
                display_name: step.display_name.clone(),
                status: StepStatus::Failed,
                output: String::new(),
                error: Some(format!("Failed to execute in container: {}", e)),
                duration: start.elapsed(),
                exit_code: None,
                outputs: HashMap::new(),
            },
        }
    }

    /// Stop and remove a container
    async fn stop_container(&self, handle: &ContainerHandle) -> Result<(), ContainerError> {
        // Stop the container
        let _ = tokio::process::Command::new("docker")
            .args(["stop", &handle.name])
            .output()
            .await;

        // Remove the container if auto_remove is enabled
        if self.config.auto_remove {
            let _ = tokio::process::Command::new("docker")
                .args(["rm", "-f", &handle.name])
                .output()
                .await;
        }

        Ok(())
    }
}

impl Default for ContainerRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a simple UUID-like string (8 chars)
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = duration.as_nanos();
    format!("{:08x}", (nanos as u32) ^ (std::process::id() as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_container_ref_image() {
        let runner = ContainerRunner::new();
        let container = ContainerRef::Image("ubuntu:22.04".to_string());
        let spec = runner.parse_container_ref(&container).unwrap();

        assert_eq!(spec.image, "ubuntu:22.04");
        assert!(spec.env.is_empty());
    }

    #[test]
    fn test_parse_container_ref_spec() {
        let runner = ContainerRunner::new();
        let mut env = HashMap::new();
        env.insert("MY_VAR".to_string(), "value".to_string());

        let container = ContainerRef::Spec(ContainerSpec {
            image: "node:18".to_string(),
            endpoint: None,
            env,
            ports: vec!["3000:3000".to_string()],
            volumes: vec!["/data:/data".to_string()],
            options: None,
            map_docker_socket: Some(true),
            mount_read_only: None,
        });

        let spec = runner.parse_container_ref(&container).unwrap();

        assert_eq!(spec.image, "node:18");
        assert_eq!(spec.env.get("MY_VAR"), Some(&"value".to_string()));
        assert!(spec.map_docker_socket.unwrap_or(false));
    }

    #[test]
    fn test_uuid_v4_simple() {
        let id1 = uuid_v4_simple();
        let id2 = uuid_v4_simple();

        assert_eq!(id1.len(), 8);
        // IDs generated in quick succession might be the same,
        // but they should be valid hex strings
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(id2.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_docker_availability_check() {
        let runner = ContainerRunner::new();
        // This test just verifies the check doesn't panic
        let _ = runner.is_available().await;
    }
}
