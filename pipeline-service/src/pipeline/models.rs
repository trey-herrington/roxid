use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    #[serde(flatten)]
    pub action: StepAction,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub continue_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepAction {
    Command(String),
    Shell {
        shell: Option<String>,
        script: String,
    },
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_name: String,
    pub status: StepStatus,
    pub output: String,
    pub error: Option<String>,
    pub duration: Duration,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub pipeline_name: String,
    pub env: HashMap<String, String>,
    pub working_dir: String,
}

impl ExecutionContext {
    pub fn new(pipeline_name: String, working_dir: String) -> Self {
        Self {
            pipeline_name,
            env: HashMap::new(),
            working_dir,
        }
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }
}
