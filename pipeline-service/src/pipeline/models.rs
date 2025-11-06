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
    // Legacy: direct steps (for backward compatibility)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<Step>,
    // New: stages-based structure
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub stage: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub condition: Option<String>,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub strategy: Option<Strategy>,
    #[serde(default)]
    pub pool: Option<Pool>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    #[serde(default)]
    pub matrix: Option<HashMap<String, HashMap<String, String>>>,
    #[serde(default)]
    pub max_parallel: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub vm_image: Option<String>,
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

#[derive(Debug, Clone)]
pub struct JobResult {
    pub job_name: String,
    pub status: JobStatus,
    pub steps: Vec<StepResult>,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage_name: String,
    pub status: StageStatus,
    pub jobs: Vec<JobResult>,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
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
    pub stage_name: Option<String>,
    pub job_name: Option<String>,
}

impl ExecutionContext {
    pub fn new(pipeline_name: String, working_dir: String) -> Self {
        Self {
            pipeline_name,
            env: HashMap::new(),
            working_dir,
            stage_name: None,
            job_name: None,
        }
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn with_stage(mut self, stage_name: String) -> Self {
        self.stage_name = Some(stage_name);
        self
    }

    pub fn with_job(mut self, job_name: String) -> Self {
        self.job_name = Some(job_name);
        self
    }
}

impl Pipeline {
    /// Check if this is a legacy pipeline (direct steps) or new format (stages)
    pub fn is_legacy(&self) -> bool {
        !self.steps.is_empty() && self.stages.is_empty()
    }

    /// Convert legacy pipeline to stage-based format
    pub fn to_stages_format(self) -> Self {
        if !self.is_legacy() {
            return self;
        }

        // Convert direct steps to a single stage with a single job
        let job = Job {
            job: "default".to_string(),
            display_name: Some("Default Job".to_string()),
            depends_on: vec![],
            condition: None,
            strategy: None,
            pool: None,
            env: HashMap::new(),
            steps: self.steps,
        };

        let stage = Stage {
            stage: "default".to_string(),
            display_name: Some("Default Stage".to_string()),
            depends_on: vec![],
            condition: None,
            jobs: vec![job],
        };

        Pipeline {
            name: self.name,
            description: self.description,
            env: self.env,
            steps: vec![],
            stages: vec![stage],
        }
    }
}
