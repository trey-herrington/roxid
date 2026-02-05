// Runners Module
// Provides step execution runners for different step types

pub mod container;
pub mod shell;
pub mod task;

// Re-export key types
pub use container::ContainerRunner;
pub use shell::ShellRunner;
pub use task::TaskRunner;

use crate::parser::models::{Step, StepResult};

use std::collections::HashMap;
use std::path::Path;

/// Trait for step runners
#[async_trait::async_trait]
pub trait Runner: Send + Sync {
    /// Execute a step and return the result
    async fn execute(
        &self,
        step: &Step,
        env: &HashMap<String, String>,
        working_dir: &Path,
    ) -> StepResult;
}

/// Runner registry for managing available runners
pub struct RunnerRegistry {
    shell: ShellRunner,
    task: Option<TaskRunner>,
    container: Option<ContainerRunner>,
}

impl RunnerRegistry {
    /// Create a new runner registry with default shell runner
    pub fn new() -> Self {
        Self {
            shell: ShellRunner::new(),
            task: None,
            container: None,
        }
    }

    /// Enable task runner with the specified cache directory
    pub fn with_task_runner(mut self, cache_dir: impl AsRef<Path>) -> Self {
        self.task = Some(TaskRunner::new(cache_dir.as_ref().to_path_buf()));
        self
    }

    /// Enable container runner
    pub fn with_container_runner(mut self) -> Self {
        self.container = Some(ContainerRunner::new());
        self
    }

    /// Get the shell runner
    pub fn shell(&self) -> &ShellRunner {
        &self.shell
    }

    /// Get the task runner (if enabled)
    pub fn task(&self) -> Option<&TaskRunner> {
        self.task.as_ref()
    }

    /// Get the container runner (if enabled)
    pub fn container(&self) -> Option<&ContainerRunner> {
        self.container.as_ref()
    }
}

impl Default for RunnerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
